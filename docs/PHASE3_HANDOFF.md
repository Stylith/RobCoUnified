# Phase 3 Handoff — iced Shell Subsystem Progress

**Written:** 2026-03-19
**Status:** Phases 3a–3f complete. Phases 3g (terminal), 3h (hosted apps) remain.
**Next binary:** `cargo run -p robcos-native-shell --bin robcos-iced`
**Untouched:** `robcos-native` (egui binary) is completely unchanged throughout.

---

## What Is Working Right Now

Run `cargo run -p robcos-native-shell --bin robcos-iced` and you get:

1. **Two demo windows** (FileManager, Editor) open on launch — drag, resize (right/bottom/corner edges), minimize, maximize, close all work
2. **Top menu bar** — active app name (filled button), File/Edit/View/Window/Help entries, clock (HH:MM, refreshed every 30s)
3. **Taskbar** — [Start] button, one button per open window with active/minimized visual states
4. **Start menu** — opens when [Start] clicked; root column with separators; right panel shows submenu or leaf pane on hover
5. **Spotlight** — Cmd+Space opens centered modal; text input, 4 tabs, scrollable results; Escape or backdrop click closes it
6. **Desktop surface icons** — builtin icons (FileManager, Editor, Installer, Settings, NukeCodes, Terminal) in a left column; single-click selects, double-click opens the target window

Everything compiles cleanly. `cargo check -p robcos-native-shell --bin robcos-iced` emits zero errors, ~11 pre-existing warnings (visibility mismatches from the old egui code, not regressions).

---

## File Map — iced Shell Code

All new iced code lives in `src/native/`. The egui code in `src/native/app.rs` is untouched.

| File | Purpose |
|------|---------|
| `src/native/shell.rs` | Top-level `RobcoShell` struct, `update()`, `view()` and all view helpers |
| `src/native/message.rs` | Central `Message` enum (~45 variants), helper enums |
| `src/native/retro_theme.rs` | Framework-agnostic `RetroColor` / `RetroColors` palette |
| `src/native/desktop_wm_widget.rs` | Custom iced `Widget` — inner window manager |
| `crates/native-shell/src/iced_main.rs` | Binary entry point for `robcos-iced` |
| `docs/ICED_MIGRATION_PLAN.md` | Full 5-phase plan |
| `docs/PHASE2_HANDOFF.md` | Phase 2 details |
| `docs/PHASE3A_HANDOFF.md` | Phase 3a (window manager widget) details |

---

## Architecture: How the View Composes

```
RobcoShell::view()
  ├── if spotlight.open  → stack![shell_ui, view_spotlight()]
  ├── elif start_menu.open → stack![shell_ui, view_start_menu()]
  └── else → shell_ui

shell_ui = column![
  view_top_bar()       // 28px — app name + menu buttons + clock
  view_desktop()       // Fill — desktop workspace
  view_taskbar()       // 32px — [Start] + window buttons
]

view_desktop()
  └── stack![
        view_surface_icons()    // black bg + left-column icons (bottom layer)
        DesktopWindowHost::new(wm_children)  // floating windows (top layer)
      ]
```

---

## RobcoShell Struct Fields (shell.rs)

```rust
pub struct RobcoShell {
    pub windows: WindowManager,       // z-order, drag/resize, open/close
    pub spotlight: SpotlightState,    // search overlay state
    pub start_menu: StartMenuState,   // start menu panel state
    pub surface: DesktopSurfaceState, // desktop icons, selected icon
    pub desktop_mode: bool,           // true=desktop, false=terminal
    pub session_username: Option<String>,
    pub session_is_admin: bool,
    pub file_manager: NativeFileManagerState,
    pub editor: EditorWindow,
    pub settings_panel: Option<NativeSettingsPanel>,
    pub settings: Settings,
    pub shell_status: String,
    pub clock: String,                // "HH:MM" refreshed on Tick
}
```

---

## Subscriptions (shell.rs)

```rust
pub fn subscription(&self) -> Subscription<Message> {
    use iced::keyboard::{self, key::Named, Key, Modifiers};
    let tick = iced::time::every(Duration::from_secs(30)).map(Message::Tick);
    let hotkeys = keyboard::on_key_press(|key, mods| match key {
        Key::Named(Named::Space) if mods.contains(Modifiers::COMMAND) => Some(Message::OpenSpotlight),
        Key::Named(Named::Escape) => Some(Message::CloseSpotlight),
        _ => None,
    });
    Subscription::batch([tick, hotkeys])
}
```

Phase 3g will add a PTY subscription here.

---

## DesktopWindowHost Widget (desktop_wm_widget.rs)

Custom `iced::advanced::Widget` implementing the inner window manager.

**Key types:**
```rust
pub struct DesktopWindowHost<'a> {
    children: Vec<WindowChild<'a>>,  // front-to-back z-order
}

pub struct WindowChild<'a> {
    pub id: DesktopWindow,
    pub rect: WindowRect,
    pub title: String,
    pub lifecycle: WindowLifecycle,
    pub is_active: bool,
    pub resizable: bool,
    pub content: Element<'a, Message>,
}

struct WmState {           // stored in Tree::State across frames
    drag: Option<DragState>,
    resize: Option<ResizeState>,
}
```

**Constants:** `TITLE_BAR_HEIGHT=28`, `BORDER_WIDTH=2`, `BUTTON_WIDTH=28`, `RESIZE_HANDLE=8`

**Event flow:**
- Left-click title bar → drag (publishes `WindowMoved` on each mouse move)
- Left-click resize handle → resize (publishes `WindowResized` on each mouse move)
- Left-click chrome button → `WindowHeaderButtonClicked { button: Close/Min/Max/Restore }`
- Any click → `FocusWindow` (brings to front)
- Content zone clicks forwarded to child widget

---

## Message Variants Added in Phase 3a

```rust
// In message.rs — added to the Window management section:
WindowMoved { window: DesktopWindow, x: f32, y: f32 },
WindowResized { window: DesktopWindow, w: f32, h: f32 },
```

Both are handled in `update()`: mutate `windows.get_mut(window).rect`.

---

## Cargo.toml Change

`iced` in workspace dependencies now has `"advanced"` feature:
```toml
iced = { version = "0.13", features = ["advanced", "canvas", "image", "tokio"] }
```

---

## Known Visual Issues (to fix in Phase 4 or sooner)

1. **Chrome buttons hard to see** — `_` `+` `X` labels blend into title bar because both use `active_bg`. Fix: give buttons explicit `hovered_bg` default color that contrasts with the title bar.

2. **Demo windows start at y=40** — overlaps the top menu bar (height=28). Fix: change initial `y` in `RobcoShell::new()` from 40→36 or add top-bar height offset.

3. **No double-click on title bar to maximize** — requires tracking two clicks within a time window. Use `WmState` to store `last_click: Option<(DesktopWindow, Instant)>` and compare timestamps.

4. **Start menu right panel uses placeholder "Loading…"** for leaf items — needs to call the search service to populate Applications / Documents / Network / Games entries.

---

## Phase 3g — Terminal Mode (next priority)

Terminal mode is a PTY-backed canvas that renders a VT100 cell grid. In the egui version this already works via `pty_screen.rs`. For iced, the approach is:

### Architecture

```
Message::PtyInput(bytes) → write to PTY stdin
Message::PtyOutput(bytes) → feed to vt100::Parser → refresh terminal state
Message::PtyExited → show exit message, await keypress to return to desktop
Message::DesktopModeToggled → flip self.desktop_mode
```

### Subscription

```rust
// In shell.rs subscription(), alongside tick + hotkeys:
if self.desktop_mode == false {
    // Stream PTY stdout bytes as Message::PtyOutput
    iced::subscription::channel(PTY_SUB_ID, 256, |mut tx| async move {
        // read from pty_master fd → tx.send(Message::PtyOutput(bytes)).await
    })
}
```

### View

When `!self.desktop_mode`:
- Replace the entire `view()` with a full-screen Canvas widget
- Canvas renders the VT100 cell buffer (monospace grid, fg/bg per cell)
- Keyboard events → `Message::PtyInput(bytes)`

### PTY State

Add to `RobcoShell`:
```rust
pub pty: Option<RobcosPty>,   // existing type from pty_screen.rs or create new
pub vt_parser: vt100::Parser,
```

The `vt100` crate is already a workspace dependency. `RobcosPty` wraps a `pty::fork()` + master fd.

### Canvas Widget for Terminal

Create `src/native/terminal_canvas.rs`:
```rust
struct TerminalCanvas<'a> {
    parser: &'a vt100::Parser,
    palette: RetroColors,
}

impl canvas::Program<Message> for TerminalCanvas<'_> {
    type State = ();
    fn draw(&self, _state, renderer, _theme, bounds, _cursor) -> Vec<Geometry> {
        // Iterate vt100 screen cells → fill_quad for bg, fill_text for char
    }
    fn update(&self, _state, event, _bounds, _cursor) -> (Status, Option<Message>) {
        // keyboard::Event → Message::PtyInput(bytes)
    }
}
```

---

## Phase 3h — Hosted Apps (after terminal)

Replace placeholder content in windows with real app views.

### For each window kind, implement `DesktopApp` trait

```rust
// In a new file, e.g. src/native/apps/file_manager_app_iced.rs
struct FileManagerDesktopApp {
    state: NativeFileManagerState,
}

impl DesktopApp for FileManagerDesktopApp {
    fn window_id(&self) -> DesktopWindow { DesktopWindow::FileManager }
    fn title(&self) -> &str { "File Manager" }
    fn default_size(&self) -> (f32, f32) { (700.0, 500.0) }
    fn update(&mut self, msg: &Message) -> Vec<Message> { ... }
    fn view(&self) -> Element<'_, Message> { ... } // port file_manager_app.rs view logic
}
```

Then in `view_desktop()`, replace the placeholder `column![text(id), text("placeholder")]` with the actual `app.view()` call.

### RobcoShell holds apps

```rust
// Add to RobcoShell:
pub apps: HashMap<DesktopWindow, Box<dyn DesktopApp>>,
```

Populate in `new()`. Route messages in `update()` by forwarding to the active app.

---

## Build Commands

```bash
# Check (fast, no linking):
cargo check -p robcos-native-shell --bin robcos-iced

# Build:
cargo build -p robcos-native-shell --bin robcos-iced

# Run:
cargo run -p robcos-native-shell --bin robcos-iced

# Verify egui binary still works:
cargo check -p robcos-native-shell --bin robcos-native

# All tests (should pass — 20 pre-existing failures are in tests unrelated to iced):
cargo test
```

---

## Commit Convention

No `Co-Authored-By` lines. Commit messages follow: `Phase Nx: short description\n\nbullet list of what changed`.

---

## What NOT to touch

- `src/native/app.rs` — the egui app, completely off-limits
- `crates/native-shell/src/main.rs` — the egui entry point
- Any file not in `src/native/` or `crates/native-shell/src/iced_main.rs`
- The 20 pre-existing test failures — do not regress further

---

## Phase 4 Preview (after Phase 3 is complete)

Phase 4 is RetroTheme polish — replacing `Theme::Dark` with a real custom iced theme that applies `RetroColors` to all widget styles. Currently `shell.rs theme()` returns `Theme::Dark`. Phase 4 implements `iced::theme::Custom` or the `Catalog` trait.

Phase 5 is cleanup — removing the egui binary, `ratatui` dependency, dead code.
