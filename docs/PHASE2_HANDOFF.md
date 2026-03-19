# Phase 2 Handoff — iced Scaffold

**Status:** Complete (2026-03-19) — binary builds and links cleanly
**Phase:** 2 of 5 (iced scaffold — window opens, basic layout, RetroTheme stub)

---

## What Phase 2 Does

Adds iced as a dependency and creates a new binary (`robcos-iced`) that opens
a window and renders a minimal retro-themed shell layout.

**The existing `robcos-native` binary is completely untouched.**

---

## Changes Made in Phase 2

### Cargo.toml (root workspace)
Added to `[workspace.dependencies]`:
```toml
iced = { version = "0.13", features = ["canvas", "image", "tokio"] }
```
Added to root `[dependencies]`:
```toml
iced = { workspace = true }
```

### `crates/native-shell/Cargo.toml`
Added `iced = { workspace = true }` to `[dependencies]`.
Added new binary:
```toml
[[bin]]
name = "robcos-iced"
path = "src/iced_main.rs"
```

### `src/native/mod.rs`
Changed the new Phase 1 modules from `pub(super)` to `pub` so the `native-shell` crate can access them:
```rust
pub mod message;
pub mod shell;
```

### `src/native/retro_theme.rs` (new)
Defines `RetroColor` — an egui/iced-independent color tuple — and `RetroColors`
(the full computed palette). Also defines the iced-side `ShellTheme` wrapper
that adapts `RetroColors` into iced widget styles via style closures.

In Phase 2 the theme is a stub using `iced::Theme::Dark` with manual
background fill to approximate the retro look. The real custom-theme
styling (all widget variants) comes in Phase 4.

### `src/native/shell.rs` (updated)
Added iced methods to `RobcoShell`:
- `fn new() -> (Self, iced::Task<Message>)` — constructs initial state, loads settings from disk
- `fn update(&mut self, msg: Message) -> iced::Task<Message>` — dispatches Message variants to sub-state handlers
- `fn view(&self) -> iced::Element<'_, Message>` — Phase 2 placeholder: top bar + center + taskbar
- `fn theme(&self) -> iced::Theme` — returns Dark for now; Phase 4 replaces with custom RetroTheme
- `fn subscription(&self) -> iced::Subscription<Message>` — empty for now; Phase 3 adds PTY + tick

Also declared the `retro_theme` module.

### `crates/native-shell/src/iced_main.rs` (new)
Entry point for the iced binary:
```rust
iced::application("RobCoOS", RobcoShell::update, RobcoShell::view)
    .theme(RobcoShell::theme)
    .subscription(RobcoShell::subscription)
    .run_with(RobcoShell::new)
```

---

## Build & Run

```bash
# Build the new iced binary
cargo build -p robcos-native-shell --bin robcos-iced

# Run it
cargo run -p robcos-native-shell --bin robcos-iced

# The old egui binary still works
cargo run -p robcos-native-shell --bin robcos-native
```

---

## What Phase 2 Does NOT Do

- No real window-in-window manager yet (Phase 3)
- No PTY / terminal rendering yet (Phase 3)
- No real app content in the windows (Phase 3)
- No custom RetroTheme styling — uses `iced::Theme::Dark` (Phase 4)
- No persistence / settings save on quit (Phase 3)
- No spotlight search wired up (Phase 3)
- No start menu wired up (Phase 3)

---

## What Comes Next: Phase 3 — Port Subsystems

Phase 3 ports one subsystem at a time into the iced binary. Recommended order:

### 3a. Window manager custom widget (~3-5 days) — HARDEST
Build the inner-window-manager iced widget that:
1. Draws window chrome (title bar, close/min/max buttons, border)
   - Use `canvas::Frame` for the chrome drawing
2. Hit-tests mouse events → dispatches to correct window
3. Maintains z-order (draw back-to-front)
4. Handles drag (title bar) and resize (handle corners/edges)
5. Forwards events to child window's view element

Start with ONE dummy app that shows a colored rectangle. Get drag/resize/
chrome working before adding real app content.

Architecture:
```rust
// Custom widget implementing iced's Widget trait
struct DesktopWindowHost<'a> {
    manager: &'a WindowManager,
    children: Vec<(DesktopWindow, Element<'a, Message>)>,
}
```

### 3b. Taskbar (1 day)
Replace placeholder with real taskbar:
- Row of `button` widgets for open windows
- [Start] button → `Message::StartButtonClicked`
- Styled with the green-on-black retro look

### 3c. Top menu bar (1-2 days)
Replace placeholder with real menu bar:
- App name button (bold)
- File / Edit / View / Window / Help dropdowns
- Use iced `overlay` or a custom dropdown widget

### 3d. Start menu (1-2 days)
Show the start menu panel anchored to the [Start] button rect.
Port the 3-column layout from `desktop_start_menu.rs`.

### 3e. Spotlight overlay (1 day)
Port `desktop_spotlight.rs` view logic into a floating overlay.

### 3f. Desktop surface (2-3 days)
Port `desktop_surface.rs` into a Canvas widget:
- Wallpaper background
- Desktop icons (drag-drop, selection, context menu)

### 3g. Terminal mode (2-3 days)
PTY canvas widget:
- Canvas that renders the VT100 cell buffer (from `vt100` crate)
- `Subscription` that streams PTY output → `Message::PtyOutput`
- Keyboard input → `Message::PtyInput`

### 3h. Hosted apps (1 week)
Port file manager, editor, settings, installer, etc. each as a
`DesktopApp` trait implementor with a real `view()`.
This is mostly mechanical translation of egui widgets → iced widgets.

---

## Key iced 0.13 API Notes

iced 0.13 uses `Task<Message>` (NOT `Command`).

Functional API entry point:
```rust
iced::application(title, update_fn, view_fn)
    .theme(theme_fn)
    .subscription(subscription_fn)
    .run_with(init_fn)  // init returns (State, Task<Message>)
```

Custom widget (for window manager):
```rust
impl<Message> Widget<Message, Theme, Renderer> for MyWidget {
    fn size(&self) -> Size<Length> { ... }
    fn layout(&self, tree, renderer, limits) -> layout::Node { ... }
    fn draw(&self, tree, renderer, theme, style, layout, cursor, viewport) { ... }
    fn on_event(&mut self, tree, event, layout, cursor, renderer, clipboard, shell, viewport) -> Status { ... }
    fn children(&self) -> Vec<Tree> { ... }
    fn diff(&self, tree: &mut Tree) { ... }
}
```

Canvas widget (for PTY/surface):
```rust
impl<Message> canvas::Program<Message> for MyCanvas {
    type State = MyInteractionState;
    fn draw(&self, state, renderer, theme, bounds, cursor) -> Vec<Geometry> { ... }
    fn update(&self, state, event, bounds, cursor) -> (Status, Option<Message>) { ... }
}
```

---

## Type Reference

New types in Phase 2:

| Type | File | Purpose |
|------|------|---------|
| `RetroColor` | `retro_theme.rs` | egui/iced-agnostic RGBA color |
| `RetroColors` | `retro_theme.rs` | Full computed palette (fg/bg/panel/dim/etc.) |

iced entry point: `crates/native-shell/src/iced_main.rs`
iced methods on RobcoShell: `src/native/shell.rs`
