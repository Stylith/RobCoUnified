# Phase 3a Handoff — Inner Window Manager Widget

**Status:** Complete (2026-03-19) — compiles, builds, links cleanly
**Phase:** 3a of 5 (custom iced Widget for inner window management)

---

## What Phase 3a Does

Creates `src/native/desktop_wm_widget.rs` — a custom `iced::advanced::Widget` that
implements the inner window manager. This is the single hardest piece of the iced
migration: it renders desktop windows with title-bar chrome, handles mouse-driven
drag and resize, manages z-order focus, and forwards events to child content widgets.

**The existing `robcos-native` binary is completely untouched.**

---

## Changes Made in Phase 3a

### `Cargo.toml` (root workspace)
Added `"advanced"` to iced features (required for custom Widget trait):
```toml
iced = { version = "0.13", features = ["advanced", "canvas", "image", "tokio"] }
```

### `src/native/message.rs`
Added two new message variants:
```rust
WindowMoved { window: DesktopWindow, x: f32, y: f32 },
WindowResized { window: DesktopWindow, w: f32, h: f32 },
```

### `src/native/desktop_wm_widget.rs` (new)
Custom iced Widget — the core of Phase 3a. Key types:

| Type | Purpose |
|------|---------|
| `DesktopWindowHost<'a>` | The widget — holds `Vec<WindowChild<'a>>` in front-to-back z-order |
| `WindowChild<'a>` | One window: id, rect, title, lifecycle, is_active, resizable, content Element |
| `WmState` | Persistent interaction state (drag/resize in progress), stored in `Tree::State` |
| `DragState` | Tracks title-bar drag: window id, start cursor position, original rect |
| `ResizeState` | Tracks resize handle drag: window id, edge, start cursor, original rect |
| `HitInfo` / `HitZone` | Hit-test result identifying which window and which zone was hit |

Widget trait implementation:
- **`size()`** — `Fill × Fill` (takes all available space)
- **`layout()`** — Absolute positioning: each child content is positioned at its window's rect
- **`draw()`** — Back-to-front rendering: window border → title bar bg → title text → chrome buttons (_, +, X) with hover highlights → content (clipped to content area via `renderer.with_layer()`)
- **`on_event()`** — Front-to-back hit testing:
  - Left click on chrome buttons → publishes `WindowHeaderButtonClicked`
  - Left click on title bar → starts drag (tracked in `WmState`)
  - Left click on resize handles (right edge, bottom edge, corner) → starts resize
  - Mouse move during drag → publishes `WindowMoved` with new position
  - Mouse move during resize → publishes `WindowResized` with new size
  - Button release → finalizes drag/resize
  - Content zone clicks → forwarded to child widget
  - Any click → publishes `FocusWindow` for z-order management
- **`mouse_interaction()`** — Returns appropriate cursor: grab/grabbing for title bar, resize arrows for edges, pointer for buttons, delegates to child for content

Constants:
```rust
TITLE_BAR_HEIGHT = 28.0
BORDER_WIDTH = 2.0
BUTTON_WIDTH = 28.0
RESIZE_HANDLE = 8.0
```

### `src/native/shell.rs` (updated)
- Added `WindowMoved` and `WindowResized` handlers in `update()`
- Replaced Phase 2 placeholder center with real `DesktopWindowHost` widget in `view()`
- Each open, visible window gets a `WindowChild` with placeholder content
- Taskbar now shows buttons for each open window (click to focus/minimize toggle)
- Two demo windows (FileManager, Editor) opened in `new()` for testing

### `src/native/mod.rs`
Added `pub mod desktop_wm_widget;`

---

## Build & Run

```bash
# Build the iced binary
cargo build -p robcos-native-shell --bin robcos-iced

# Run it — you'll see two demo windows with drag/resize/minimize/maximize/close
cargo run -p robcos-native-shell --bin robcos-iced

# The old egui binary still works
cargo run -p robcos-native-shell --bin robcos-native
```

---

## Architecture Notes

### Event Flow
```
Mouse event → DesktopWindowHost::on_event()
  → hit_test() identifies window + zone
  → publishes Message variant via shell.publish()
  → iced runtime calls RobcoShell::update(message)
  → WindowManager state is mutated
  → iced calls RobcoShell::view() → new DesktopWindowHost with updated rects
```

### Z-Order
- `WindowManager::z_ordered()` returns front-to-back order
- `DesktopWindowHost.children` are in front-to-back order
- Drawing iterates back-to-front (last → first)
- Hit testing iterates front-to-back (first → last), returns first hit

### Tree State
`WmState` is stored in the iced Widget tree (`tree::State`). It persists across
frames and tracks ongoing drag/resize interactions. It is NOT part of `RobcoShell`
because it's transient frame-level interaction state, not application state.

---

## What Phase 3a Does NOT Do

- No real app content in windows — all windows show placeholder text (Phase 3h)
- No double-click title bar to maximize (easy to add)
- No window snapping or grid alignment
- No keyboard-driven window management (Alt+F4 etc.)
- No window decorations (shadow, glow effects) — Phase 4
- No start menu panel rendering (Phase 3d)
- No spotlight overlay (Phase 3e)
- No real menu bar (Phase 3c)
- No desktop surface/icons (Phase 3f)

---

## What Comes Next: Phase 3b — Taskbar

Phase 3b replaces the basic taskbar row with the full styled taskbar:
- Start button with proper retro styling
- System tray / clock area
- Window buttons with active/minimized visual states
- Tooltips on hover

Estimated: ~1 day of work.
