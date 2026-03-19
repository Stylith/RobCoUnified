# RobCoOS: egui → iced Migration Plan

**Status:** Draft — not yet started
**Created:** 2026-03-19
**Goal:** Replace egui with iced, restructure state into sub-structs with message-passing, archive legacy shell, modernize asset pipeline. End state: a clean Elm-architecture desktop shell that's easier to reason about, extend, and test.

---

## Phase 0: Pre-Migration Cleanup (1-2 days)

Do this before touching iced. Reduces noise and avoids migrating dead code.

### 0a. Archive legacy shell
- Create `legacy` branch from `main`
- Delete `crates/legacy-shell/`, `src/legacy/`, and all legacy references from workspace
- Remove legacy from `Cargo.toml` workspace members
- ~22,700 lines gone. Build times improve immediately.

### 0b. Fix or delete the 20 failing tests
- Investigate root cause (likely shared mutable global state between tests — the `session_test_guard` mutex pattern suggests this)
- Fix the concurrency issue or make tests serial (`cargo test -- --test-threads=1` as stopgap)
- Delete any tests that test removed/obsolete behavior
- Goal: `cargo test` is green on `main` before migration begins

### 0c. Build-time SVG rasterization for built-in icons
- Extend `build.rs` to pre-render all built-in SVG icons (file manager, settings, terminal, etc.) to PNG at multiple sizes (16, 24, 32, 48, 64px)
- Embed as `include_bytes!()` — same pattern as Donkey Kong sprites
- Remove runtime `resvg`/`usvg` calls for built-in icons (keep for user-provided SVGs if needed)
- This is framework-agnostic and benefits the iced migration directly (iced uses `image` for textures)

### 0d. Add LRU bounds to caches
- Replace unbounded `HashMap` caches (`AssetCache`, shortcut icon cache, texture cache) with `lru::LruCache`
- Cap at reasonable limits (e.g., 256 entries for icons, 64 for textures)
- Add `lru` to workspace dependencies
- Framework-agnostic improvement that carries forward

---

## Phase 1: Architecture Redesign (2-3 days, no UI code yet)

Design the new state model and message system on paper/in code before wiring up any iced widgets. This is the most important phase — get it right and the UI work is mechanical.

### 1a. Define the Message enum

```rust
/// Top-level shell message — every user action and system event flows through here.
#[derive(Debug, Clone)]
pub enum Message {
    // Window management
    OpenWindow(DesktopWindow),
    CloseWindow(DesktopWindow),
    MinimizeWindow(DesktopWindow),
    MaximizeWindow(DesktopWindow),
    RestoreWindow(DesktopWindow),
    FocusWindow(DesktopWindow),
    WindowDragged(DesktopWindow, Point),
    WindowResized(DesktopWindow, Size),

    // Start menu
    ToggleStartMenu,
    StartMenuNavigate(StartMenuNav),
    StartMenuActivate,
    StartMenuRename(String, String),

    // Spotlight
    OpenSpotlight,
    CloseSpotlight,
    SpotlightQueryChanged(String),
    SpotlightTabChanged(u8),
    SpotlightNavigate(i32),  // delta
    SpotlightActivate,

    // Taskbar
    TaskbarWindowClicked(DesktopWindow),

    // Desktop surface
    DesktopIconClicked(DesktopIconId, bool),  // id, shift_held
    DesktopIconDoubleClicked(DesktopIconId),
    DesktopIconDragged(DesktopIconId, Point),
    DesktopIconDropped(DesktopIconId, Point),
    DesktopContextMenu(ContextMenuAction),
    WallpaperChanged(PathBuf),
    FilesDroppedOnDesktop(Vec<PathBuf>),

    // Menu bar
    MenuAction(DesktopMenuAction),

    // App hosting
    LaunchApp(String),
    LaunchStandaloneApp(StandaloneApp),
    ShellAction(DesktopShellAction),

    // Session
    Login(String, String),
    Logout,
    SwitchSession(String),

    // Terminal
    TerminalInput(Vec<u8>),
    TerminalOutput(Vec<u8>),
    TerminalResized(u16, u16),

    // System
    Tick(Instant),
    SettingsChanged(SettingsUpdate),
    PersistSnapshot,

    // Async results
    FileOperationComplete(FileOpResult),
    IconLoaded(IconId, ImageHandle),
}
```

### 1b. Define sub-state structs

```rust
pub struct RobcoShell {
    // Core subsystems — each owns its state, communicates via Message
    pub windows: WindowManager,
    pub taskbar: TaskbarState,
    pub start_menu: StartMenuState,
    pub spotlight: SpotlightState,
    pub surface: DesktopSurfaceState,
    pub menu_bar: MenuBarState,
    pub terminal: TerminalState,
    pub session: SessionManager,

    // App hosting
    pub apps: AppHostState,

    // Shared resources
    pub settings: ShellSettings,
    pub assets: AssetCache,
    pub palette: RetroPalette,
}
```

Each sub-struct gets:
- Its own file/module
- A `fn update(&mut self, msg: &Message) -> Vec<Message>` method (can emit follow-up messages)
- A `fn view(&self) -> Element<Message>` method (returns its iced widget tree)
- Clear ownership of its state — no reaching into siblings

### 1c. Define WindowManager properly

This is the trickiest part. iced doesn't have built-in window-in-window, so we need a proper window manager:

```rust
pub struct WindowManager {
    windows: Vec<ManagedWindow>,
    z_order: Vec<DesktopWindow>,  // front-to-back
    active: Option<DesktopWindow>,
    drag: Option<WindowDrag>,
    resize: Option<WindowResize>,
}

pub struct ManagedWindow {
    pub id: DesktopWindow,
    pub title: String,
    pub rect: Rect,
    pub state: WindowState,  // Normal, Minimized, Maximized
    pub restore_rect: Option<Rect>,
    pub min_size: Size,
    pub resizable: bool,
}
```

Window rendering strategy in iced:
- Use `iced::widget::Canvas` or a custom widget for the desktop surface
- Each inner window is drawn as a custom widget with:
  - Title bar (custom painted)
  - Content area (hosts the app's widget tree via `Element<Message>`)
  - Resize handles (hit-test regions)
- Z-ordering via draw order in the canvas
- Hit-testing for mouse events dispatched to the correct window

Alternative: use iced's `container` + `mouse_area` + absolute positioning via a custom layout. This might be simpler than full canvas rendering.

### 1d. Map existing desktop_app.rs component registry to iced

Current `DesktopComponentBinding` (closure-based) becomes trait-based:

```rust
pub trait DesktopApp {
    fn id(&self) -> DesktopWindow;
    fn title(&self) -> &str;
    fn default_size(&self) -> Size;
    fn min_size(&self) -> Size;
    fn update(&mut self, msg: &Message) -> Vec<Message>;
    fn view(&self) -> Element<Message>;
    fn menu_sections(&self) -> Vec<DesktopMenuSection>;
    fn show_in_taskbar(&self) -> bool { true }
}
```

Each app (file manager, editor, settings, etc.) implements this trait. The shell hosts a `Vec<Box<dyn DesktopApp>>`. This replaces the massive `draw_desktop_window_by_kind` match block.

---

## Phase 2: Scaffold iced Shell (3-5 days)

Build the minimum viable iced app that compiles and shows something.

### 2a. Add iced dependency, remove egui/eframe
- Add `iced = { version = "0.13", features = ["canvas", "image", "svg", "tokio"] }` to workspace deps
- Remove `eframe`, `egui` from workspace deps
- Update `crates/native-shell/Cargo.toml`
- The app crates (file-manager-app, editor-app, etc.) should NOT depend on iced — they're pure state/logic. Only the shell crate touches the UI framework.

### 2b. Implement shell skeleton

```rust
// src/native/shell.rs (replaces app.rs as the entry point)
use iced::{Application, Command, Element, Settings, Theme};

impl Application for RobcoShell {
    type Message = Message;
    type Theme = RetroTheme;
    type Executor = iced::executor::Default;
    type Flags = ShellFlags;

    fn new(flags: Self::Flags) -> (Self, Command<Message>) { ... }
    fn title(&self) -> String { "RobCoOS".into() }
    fn update(&mut self, message: Message) -> Command<Message> { ... }
    fn view(&self) -> Element<Message> { ... }
    fn theme(&self) -> RetroTheme { ... }
    fn subscription(&self) -> Subscription<Message> { ... }
}
```

### 2c. Implement RetroTheme

iced's theming works via the `StyleSheet` traits. Create a `RetroTheme` struct that wraps the palette:

```rust
pub struct RetroTheme {
    palette: RetroPalette,
}

impl button::StyleSheet for RetroTheme {
    type Style = ButtonStyle;
    fn active(&self, style: &Self::Style) -> button::Appearance {
        button::Appearance {
            background: Some(Background::Color(self.palette.panel)),
            border: Border { color: self.palette.fg, width: 2.0, radius: 0.0.into() },
            text_color: self.palette.fg,
            ..Default::default()
        }
    }
    // hovered, pressed, disabled...
}
// Repeat for container, text_input, scrollable, etc.
```

This is tedious but mechanical — the palette values already exist, you're just mapping them to iced's style traits.

### 2d. Implement desktop surface as Canvas widget

```rust
impl canvas::Program<Message> for DesktopSurface {
    type State = SurfaceInteraction;

    fn draw(&self, state: &Self::State, ...) -> Vec<Geometry> {
        // 1. Draw wallpaper
        // 2. Draw desktop icons
        // 3. Draw selection rectangle (if dragging)
    }

    fn update(&self, state: &mut Self::State, event: Event, ...) -> (Status, Option<Message>) {
        // Handle icon clicks, drags, context menu triggers
    }
}
```

### 2e. Implement inner window manager widget

Custom widget that:
1. Lays out child windows at absolute positions from `WindowManager` state
2. Draws window chrome (title bar, borders) around each child
3. Hit-tests mouse events and dispatches to correct window or to drag/resize handlers
4. Renders in z-order

This is the hardest custom widget. Expect 400-600 lines. But it replaces ~500 lines of egui window management code that was fighting the framework.

---

## Phase 3: Port Subsystems (1-2 weeks)

Migrate each subsystem one at a time. Order matters — start with the most self-contained.

### 3a. Taskbar (1 day)
- Simplest widget: horizontal row of buttons
- `view()` returns `Row<Message>` with styled buttons
- Clicking emits `Message::TaskbarWindowClicked(window)`

### 3b. Start menu (1-2 days)
- Overlay widget anchored to bottom-left
- Keyboard navigation via `subscription()` key events
- `update()` handles `StartMenuNavigate`, `StartMenuActivate`

### 3c. Menu bar (1-2 days)
- Top `Row` with dropdown menus
- iced has `pick_list` but custom dropdowns need a `mouse_area` + overlay pattern
- Per-app menus driven by `active_app.menu_sections()`

### 3d. Spotlight search (1 day)
- Centered overlay with text input + scrollable results list
- Text input emits `SpotlightQueryChanged(String)` on every keystroke
- Results computed in `update()`, rendered in `view()`

### 3e. Desktop icons (2-3 days)
- Part of the Canvas widget from 2d
- Icon positions stored in `SurfaceState`
- Drag-and-drop via Canvas event handling
- Context menus via overlay

### 3f. Window hosting — app content (3-5 days)
- Each `DesktopApp` impl provides its `view() -> Element<Message>`
- File manager, editor, settings each need their view ported
- This is where most of the egui → iced widget translation happens
- Strategy: port one app at a time, stub others with placeholder text

### 3g. Terminal / PTY (2-3 days)
- Canvas widget that renders the VT100 cell grid
- `Subscription` that reads PTY output asynchronously → `Message::TerminalOutput`
- Keyboard input → `Message::TerminalInput`
- This replaces the synchronous PTY polling loop

---

## Phase 4: Polish & Parity (1 week)

### 4a. Port remaining UI details
- Window drag/resize feel
- Desktop icon label wrapping
- Donkey Kong minigame (Canvas widget)
- Keyboard shortcuts (global subscription)
- Flash messages / notifications
- Save-as dialog, rename dialogs, prompts

### 4b. Standalone app binaries
- Each standalone binary becomes its own `iced::Application`
- Shares the `RetroTheme` and app state crate
- Standalone launcher stays the same (sibling binary resolution)

### 4c. Testing
- Unit tests on sub-state structs (no UI framework needed)
- Message-based integration tests: send `Message::OpenSpotlight`, assert state changed
- This is dramatically easier than testing egui because state changes are pure functions

### 4d. Performance
- Profile startup (target: <500ms)
- Measure idle CPU (iced should be near-zero when nothing changes)
- Verify memory with bounded caches
- Test on weak hardware if available

---

## Phase 5: Cleanup (2-3 days)

### 5a. Delete dead code
- Remove all egui-related code, imports, utilities
- Remove `eframe` from all Cargo.toml files
- Delete `src/native/retro_ui.rs` (egui-specific palette application) — replaced by `RetroTheme`
- Clean up unused `pub(super)` from the extraction we just did

### 5b. Reorganize modules
- `src/native/app.rs` → `src/native/shell.rs` (fresh start, should be <1000 lines)
- Sub-state modules: `src/native/window_manager.rs`, `src/native/taskbar.rs`, etc.
- Message definitions: `src/native/message.rs`
- Theme: `src/native/theme.rs`

### 5c. Update docs
- Rewrite `PROJECT_CONTEXT_FOR_LLM.md` to reflect new architecture
- Update build instructions
- Update `START_HERE_FOR_LLM.md`

---

## Dependency Changes

### Remove
- `eframe`
- `egui` (and `egui_extras` if present)

### Add
- `iced = "0.13"` with features: `canvas`, `image`, `svg`, `tokio`
- `lru = "0.12"` (for bounded caches)

### Remove (along with egui)
- `ratatui`, `crossterm` (only used by legacy shell, which is archived)

### Keep
- `portable-pty`, `vt100` (PTY hosting — terminal mode renders via iced canvas now)
- `serde`, `chrono`, `sysinfo`, `image`, `resvg`/`usvg` (build-time only for icons)
- All app crates unchanged (they don't touch UI framework)

---

## Risk Assessment

| Risk | Likelihood | Mitigation |
|------|-----------|------------|
| Window-in-window widget is harder than expected | Medium | Prototype it first in Phase 2. This is non-negotiable — it's the project's identity. Budget extra time if needed |
| iced canvas performance with many desktop icons | Low | Canvas is GPU-accelerated, icons are simple quads. Profile early |
| Retro theme styling is tedious | Certain | It's mechanical work, not hard. Budget 1-2 days for all style traits |
| PTY rendering in iced canvas is tricky | Medium | vt100 crate gives cell grid — drawing it is straightforward. Input handling needs care |
| Context menus are awkward in iced | Medium | Use overlay + mouse_area pattern. Several iced community examples exist |
| Migration takes longer than estimated | High | Phase it. Each phase produces a working (if incomplete) binary. Don't try to port everything at once |

---

## Resolved Decisions

1. **Terminal mode**: Terminal mode is already rendered via egui in the native shell (NOT ratatui). The ratatui/crossterm code lives only in the legacy shell which is being archived. Terminal mode ports from egui to iced — it becomes a Canvas widget drawing a cell grid. Same visuals, same feel. `ratatui` and `crossterm` are removed entirely after legacy archival.

2. **iced version**: Pin to iced 0.13 stable. Do not track master.

3. **Window model**: Windows-in-a-window (single OS window, inner window manager). This is the core identity of the project. Real OS windows are NOT an option. Long-term goal: implement X11 protocol support so the inner window manager can host real Linux GUI applications.

4. **Async runtime**: Yes, embrace tokio. iced's Subscription model is built around it. PTY reading becomes an async stream → `Message::TerminalOutput`. File operations run as background tasks. Timers become `tokio::time::interval` subscriptions.

## Phase 0 Delegation

Phase 0 is delegated to Codex AI. Full task spec is in `docs/PHASE0_CODEX_TASK.md`.
After Phase 0, Phases 1-5 are handled in conversation with Claude.

---

## Estimated Timeline

| Phase | Duration | Deliverable |
|-------|----------|-------------|
| Phase 0: Cleanup | 1-2 days | Clean main branch, no legacy, green tests, fast builds |
| Phase 1: Architecture | 2-3 days | Message enum, sub-state structs, trait definitions (compiles, no UI) |
| Phase 2: Scaffold | 3-5 days | Bare iced app with desktop surface, empty windows, theme |
| Phase 3: Port | 1-2 weeks | All subsystems ported, feature parity |
| Phase 4: Polish | 1 week | Full parity, tested, performant |
| Phase 5: Cleanup | 2-3 days | Dead code removed, docs updated |
| **Total** | **~4-6 weeks** | **Full migration complete** |

This can be parallelized if you have help — Phase 3 subsystems are independent of each other.
