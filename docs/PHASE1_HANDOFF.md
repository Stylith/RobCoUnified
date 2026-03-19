# Phase 1 Handoff — Architecture Scaffolding

**Status:** Complete (2026-03-19)
**Phase:** 1 of 5 (Architecture Redesign — no UI code)

---

## What Phase 1 Does

Creates the new architectural foundation alongside the existing egui code.
**Nothing existing is changed or removed.** Two new modules are added:

- `src/native/message.rs` — The central `Message` enum (the event bus)
- `src/native/shell.rs` — `RobcoShell` top-level struct + sub-state structs + `DesktopApp` trait

Both modules are declared in `src/native/mod.rs` but not yet used by the running app.
The existing `RobcoNativeApp` / egui code continues to compile and run unchanged.

---

## Files Created in Phase 1

### `src/native/message.rs` ✅ (compiles clean)
The complete `Message` enum. Every user action and system event in the new architecture
flows through here. Also defines small helper types:
- `NavDirection` — keyboard navigation direction
- `WindowHeaderButton` — close / minimize / maximize / restore
- `DesktopIconId` — identifies a desktop icon (builtin, surface entry, or shortcut)
- `FileOpResult` — result of an async file operation
- `ContextMenuAction` — all right-click context menu actions (mirrors `app::ContextMenuAction`,
  which will be deleted when Phase 3 removes app.rs)

### `src/native/shell.rs` ✅ (compiles clean)
Contains:
- `WindowRect` — egui-free window geometry (x, y, w, h)
- `WindowLifecycle` — Normal / Minimized / Maximized
- `ManagedWindow` — one inner desktop window's state
- `WindowManager` — all inner windows, z-order, active focus, drag/resize state
- `SpotlightState` — spotlight search query, results, tab, selection
- `StartMenuState` — root selection, submenu/leaf navigation, rename state
- `DesktopSurfaceState` — selected icon, context menu, wallpaper/icon picker state, drag
- `RobcoShell` — top-level struct with all sub-state + references to existing complex types
- `DesktopApp` trait — the per-app interface (update + menu sections; `view()` added in Phase 2)
- impl blocks: `WindowManager::new`, `is_open`, `focus`, `bring_to_front`; `SpotlightState::reset`

### `src/native/mod.rs` — modified ✅
Added `mod message;` and `mod shell;`.

---

## What Was Intentionally Left Out of Phase 1

- **No iced dependency** — iced is not added yet; `DesktopApp::view()` is TODO'd
- **No impl for RobcoShell** — constructor / update / view come in Phase 2
- **Existing app.rs untouched** — `RobcoNativeApp` continues to work
- **`ContextMenuAction` duplication** — both `app::ContextMenuAction` and `message::ContextMenuAction`
  exist temporarily. The one in `app.rs` is deleted in Phase 3.
- **Async/tokio** — not wired up yet; `Message::PtyOutput`, `Message::FileOperationCompleted`, etc.
  are defined but will be connected to tokio Subscriptions in Phase 2

---

## What Comes Next: Phase 2

**Goal:** Scaffold a bare iced `Application` that compiles and opens a window.

Steps:
1. Add `iced = { version = "0.13", features = ["canvas", "image", "svg", "tokio"] }` to workspace deps
2. Add iced dep to `crates/native-shell/Cargo.toml`
3. Implement `iced::Application` for `RobcoShell`:
   - `new()` — builds from `RobcoNativeApp::default()` data or loads fresh
   - `update()` — match on `Message`, dispatch to sub-state `update` methods
   - `view()` — returns a placeholder `Text::new("RobCoOS")` element initially
   - `subscription()` — PTY output stream, settings file watcher, tick timer
4. Implement `RetroTheme` (struct wrapping `RetroPalette`, implementing iced StyleSheet traits)
5. Implement the inner `WindowManager` custom widget (title bars, borders, hit-testing, z-order)
6. Implement `DesktopApp::view()` on the trait now that iced is available
7. Create a new `robcos-native-shell/src/main.rs` entry point that runs `RobcoShell::run()`
   alongside or replacing the existing eframe entry point

**The hardest part of Phase 2:** The inner window manager widget. It needs to:
- Draw window chrome (title bar, close/min/max buttons, border) via `canvas` or custom widget layout
- Handle mouse hit-testing for drag, resize, and button clicks
- Maintain z-order (front-to-back draw order)
- Forward mouse/keyboard events to the correct child window's `DesktopApp::view()` element

**Suggested prototyping approach:** Before wiring up real apps, implement the window manager
with a single dummy app that shows a colored rectangle. Get drag, resize, min/max/close working
before adding real content.

---

## Key Architectural Decisions (Already Made)

| Decision | Choice |
|----------|--------|
| iced version | 0.13 stable |
| Window model | Windows-in-a-window (single OS window, inner WM) |
| Async | tokio via iced Subscriptions |
| Terminal mode | Ported to iced Canvas (cell grid renderer) |
| ratatui/crossterm | Removed (only in legacy shell which is archived) |
| Legacy shell | To be archived to `legacy` branch (Phase 0 / Codex) |

---

## Type Reference (Quick Lookup)

All types referenced in message.rs / shell.rs and where they come from:

| Type | Source |
|------|--------|
| `DesktopWindow` | `crates/native-services` → `shared_types` |
| `DesktopMenuAction` | `src/native/desktop_app.rs` |
| `DesktopShellAction` | `src/native/desktop_app.rs` |
| `DesktopMenuSection` | `src/native/desktop_app.rs` |
| `StartSubmenu`, `StartLeaf` | `src/native/desktop_start_menu.rs` |
| `NativeSpotlightResult` | `crates/native-services` → `desktop_search_service` |
| `DesktopSurfaceEntry` | `crates/native-services` → `desktop_surface_service` |
| `NativeSettingsPanel` | `crates/native-settings-app` |
| `EditorCommand`, `EditorWindow` | `crates/native-editor-app` |
| `FileManagerCommand` | `crates/native-file-manager-app` |
| `FileManagerPromptRequest` | `src/native/file_manager_prompt.rs` |
| `NativePtyState` | `src/native/pty_screen.rs` |
| `Settings` | `crates/shared` → `config` |
| `DesktopIconSortMode` | `crates/shared` → `config` |

---

## Compile Verification

After Phase 1 is complete:
```
cargo check           # must show 0 errors
cargo test -p robcos --lib  # same pass/fail as before Phase 1
```
