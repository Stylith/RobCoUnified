# RobCoUnified — Full Session Handoff for Claude Code

This document supersedes `LLM_HANDOFF.md` with current state as of March 2026.
Read `LLM_HANDOFF.md` first for base project context, then this document for everything
that happened since.

---

## Project in One Paragraph

RobCoUnified (`robcos-native`) is a Rust eframe/egui 0.29.1 native desktop app with a
retro Fallout-terminal aesthetic. Green phosphor monochrome theme system (`current_palette().fg`
drives all chrome). It's a dual-binary project: `robcos` is a ratatui TUI reference
implementation, `robcos-native` is the native GUI. All active work is on the native binary.
The UI is built entirely in `src/native/app.rs` (~7800 lines). Config/settings persist via
`src/config.rs` and `get_settings()` / `persist_native_settings()`.

---

## Repository

- **Repo:** github.com/Stylith/RobCoUnified (public)
- **Local:** `/home/stylith/RobCoUnified`
- **Version:** 0.3.2
- **Key file:** `src/native/app.rs` — contains all window draw functions, state machine,
  settings panels, file manager, editor, etc.
- **Supporting files:** `src/native/file_manager.rs`, `src/config.rs`, `src/native/retro_ui.rs`

---

## What the Native App Looks Like

- Fullscreen retro grid renderer on login/terminal screens
- Desktop mode: floating windows (Win95-style), taskbar at bottom, top menu bar, start menu
- Windows: File Manager ("My Computer"), Word Processor (Editor), Settings, Applications,
  Nuke Codes, PTY Terminal
- Single color palette — user picks a theme color, everything derives from it

---

## Sessions 1–4 Summary (from `journal.txt`)

### Origin
Project started as Python curses app, converted to Rust over many sessions. The TUI binary
(`robcos`) is stable and not touched. All work is on `robcos-native`.

### Session 1 (2026-03-08)
- Fixed sound system: fire-and-forget afplay, AtomicUsize ACTIVE counter, OnceLock temp files
- Fixed PTY: portable-pty + vt100, PtySession struct, suspend/resume across session switches
- Fixed session system: single-process cooperative multitasking, Alt+[1-9] switching

### Session 2 (2026-03-09 early)
- Reverted repo after prior AI introduced regressions
- Fixed compile error: `TerminalSettingsPanel` removed from `settings_screen.rs` but still
  referenced in `app.rs`

### Session 3 (2026-03-09)
- PTY window: removed `ROBCO MAINTENANCE TERMLINK` header in desktop mode
- PTY font size/cell sizing: cols clamp `(40, 220)`, rows clamp `(20, 60)`
- Default PTY window size: `egui::vec2(960.0, 600.0)`

### Session 4 (2026-03-09)
- Settings window auto-growth bug: root cause was `NativeOptions::default()` has
  `persist_window: true` — egui saves window sizes to disk and restores them, beating
  `fixed_size` constraints
- Fix: Settings window is `resizable(false)` + unconditional `fixed_size(default_size)`
- Large app.rs refactor by user: settings redesigned with single-column layout, tile grid
  nav, `nav_history: Vec<NativeSettingsPanel>`, OK/Cancel/Apply buttons

---

## This Session — Complete Bug Log

### Bug 1: Window Auto-Growth on Hover (ALL resizable windows)

**Symptom:** File manager (and other windows) expand vertically when mouse hovers over them.
Becomes fullscreen height after a few seconds of hovering.

**Three failed attempts before fix:**

1. **Attempt 1** — Added `actively_resizing` detection, only saved rect when resizing.
   Result: resize broke entirely. Corner click triggered one frame of no-constraint, which
   inflated the window and that size was saved.

2. **Attempt 2** — Moved `ui.available_height()` call to after chrome widgets were rendered.
   Result: still grew. `ui.available_height()` inside a resizable window returns parent
   panel height regardless of where you call it — it feeds back into egui's Resize widget.

3. **Attempt 3** — Used `prev_window_h` (stored size from last frame) minus hardcoded
   `CHROME_OVERHEAD = 158.0`. Result: didn't grow, but couldn't resize smaller — the fixed
   `allocate_exact_size` blocked the Resize widget from shrinking.

**Root cause:** `allocate_exact_size` inside a resizable window asks egui's Resize widget
for a specific size. The Resize widget interprets this as "content needs this much space"
and expands the window. Any call to `available_height/width` → `allocate_exact_size` is a
feedback loop. There is no safe way to use `allocate_exact_size` for layout inside a
resizable window.

**Actual fix:** Replaced the entire body layout with `show_inside` panels:
- `TopBottomPanel::top` for header + tabs + search + path
- `TopBottomPanel::bottom` for status bar + buttons
- `SidePanel::left` for the folder tree (resizable by user)
- `CentralPanel::default` for the file list

`show_inside` panels carve from the window's current rect without participating in the
Resize widget's content-measurement pass. Window size is stable regardless of content,
hover state, or scroll position.

**Also removed** from all other resizable windows (Editor, Applications, NukeCodes, PtyApp):
- `ui.set_min_size(ui.available_size_before_wrap())` — same feedback loop, more aggressive
- `fixed_size(restore_size)` → replaced with `default_size(restore_size)` for restore branch

**Files changed:** `src/native/app.rs` — `draw_file_manager`, `draw_editor`,
`draw_applications`, `draw_nuke_codes_window`, `draw_desktop_pty_window`

### Bug 2: Highlighted File Row Shows Blank Text

**Symptom:** Selecting or hovering a file row shows the green highlight fill but text
disappears (invisible against the fill).

**Root cause:** `retro_file_manager_button` was using `ui.painter()` which returns a
painter clipped to the entire panel content area. After the `show_inside` panel rewrite,
the `CentralPanel` applies its own background visuals in a separate draw pass that could
land on top of the button fill in certain interaction frames.

**Fix:** Changed `ui.painter()` → `ui.painter_at(rect)`. This creates a painter clipped
specifically to the button rect — nothing outside can paint inside that boundary.

```rust
// Before
ui.painter().rect_filled(rect, 0.0, fill);
ui.painter().text(...);

// After
let painter = ui.painter_at(rect);
painter.rect_filled(rect, 0.0, fill);
painter.text(...);
```

---

## Features Added This Session

### File Manager — Major Rewrite

The old `src/native/file_manager.rs` was a 90-line stub. The app.rs was calling ~10 methods
that didn't exist yet. A full spec was written: `FILE_MANAGER_REWRITE.md`.

**What was added to `file_manager.rs`:**
- `FileManagerTab` struct (each tab has own cwd, selection, search, tree state, cached rows)
- `TreeItem` struct (for folder tree sidebar)
- Multi-tab support: `switch_to_tab`, `open_tab_here`, `close_active_tab`, `tab_title`
- Search: `update_search_query`, `rebuild_cache_with_settings`
- Tree panel: `tree_items()`, `open_selected_tree_path`
- Hidden files, sort mode (Name/Type), dirs-first all respected in `read_dir_rows`
- `ensure_selection_valid` — clears selection when it no longer appears in filtered rows
- `icon()` method on `FileEntryRow` — ASCII icons by extension `[DIR]`, `[TXT]`, `[COD]`, etc.

**What was added to `draw_file_manager` in app.rs:**
- Tab bar with active tab indicator
- Ctrl+F search bar
- Tree/List/Grid toggle buttons
- Path label
- Folder tree sidebar (SidePanel, user-resizable)
- List view and Grid view with proper scroll areas
- Status bar: item count, view mode, tree toggle, Home/Up/Open/New Document buttons

### Spec Docs Written (in `/outputs/` and repo root)

| File | Contents |
|---|---|
| `ASSET_CACHE_IMPL.md` | Full implementation spec for desktop icons (SVG→white texture+tint) and wallpaper loading. Covers `AssetCache` struct, `resvg`/`usvg` pipeline, all 5 `WallpaperSizeMode` variants, lazy init pattern. |
| `CONTEXT_MENU_IMPL.md` | Right-click context menu spec. `ContextMenuAction` enum, deferred dispatch pattern, 4 menu variants (file manager, desktop icon, desktop empty, generic), styling with `apply_top_dropdown_menu_style`, egui 0.29 `response.context_menu()` API. |
| `FILE_MANAGER_REWRITE.md` | Full `file_manager.rs` rewrite spec with all structs, methods, settings integration pattern. |

---

## Current State

### What Works
- File manager opens, shows files, navigates directories
- Tab bar, search, tree panel, list/grid view all functional
- Window resize works correctly — drag to resize, no auto-growth
- All 5 desktop windows stable (no hover-expansion on any of them)
- Selection highlight shows black text correctly

### Known Remaining Work

**Not yet implemented (placeholder stubs exist):**
- Context menus (spec written in `CONTEXT_MENU_IMPL.md`, not coded yet)
- Desktop icons with SVG (spec in `ASSET_CACHE_IMPL.md`, not coded yet)
- Wallpaper support (spec in `ASSET_CACHE_IMPL.md`, not coded yet)
- File operations: Rename, Cut, Paste, Delete, Duplicate, Properties
- Open With picker
- Right-click menus on desktop
- Address bar / breadcrumb navigation in file manager
- Keyboard navigation in file manager (arrow keys, Enter, Backspace)
- File details in status bar (size, modified date)

**Visual regressions from LLM_HANDOFF.md still outstanding:**
- Top menu bar should be plain retro text, not boxed cells
- Dropdown menus: single-panel with retro border, reverse highlight
- Start menu: Win95 proportions, not oversized panels
- `[Start]` taskbar button must stay theme-colored

---

## Architecture Notes for Claude Code

### Window Pattern (resizable)
```rust
// CORRECT pattern for resizable windows that don't auto-grow:
window.show(ctx, |ui| {
    TopBottomPanel::top(id).show_inside(ui, |ui| { /* chrome */ });
    TopBottomPanel::bottom(id).show_inside(ui, |ui| { /* footer */ });
    SidePanel::left(id).show_inside(ui, |ui| { /* sidebar */ });  // optional
    CentralPanel::default().show_inside(ui, |ui| { /* content */ });
});

// WRONG — causes feedback loop with Resize widget:
window.show(ctx, |ui| {
    let h = ui.available_height();  // returns parent panel height, not window height
    ui.allocate_exact_size(vec2(w, h), Sense::hover());  // tells Resize widget to grow
});
```

### Window Pattern (non-resizable, e.g. Settings)
```rust
// Settings is non-resizable — always use unconditional fixed_size:
window.resizable(false).fixed_size(default_size)
// Never call fixed_size on resizable windows
```

### Rect Save Pattern
```rust
// After window.show(), always save full rect unconditionally:
if !maximized {
    if let Some(rect) = shown_rect {
        self.note_desktop_window_rect(DesktopWindow::FileManager, rect);
    }
}
// note_desktop_window_rect saves both pos and size
// note_desktop_window_pos saves pos only (now dead code, can delete)
```

### Settings Access from file_manager.rs
`file_manager.rs` can't call `get_settings()` directly — it's in a different module.
Pass `&get_settings().desktop_file_manager` explicitly to any method that needs settings.

### Theme Color
```rust
let palette = current_palette();
palette.fg  // theme color (green, amber, etc) — use for all chrome
palette.bg  // background (black)
palette.panel // panel background
Color32::BLACK  // text on highlighted/selected items
```

### Key Existing Methods
- `open_desktop_window(DesktopWindow::X)` — opens a window
- `navigate_desktop_settings_to(NativeSettingsPanel::X)` — jump to settings panel
- `activate_file_manager_selection()` — open selected file/dir
- `apply_top_dropdown_menu_style(ui)` — retro style for menus/popups
- `retro_file_manager_button(ui, label, size, active, stroked)` — themed file row button
- `draw_desktop_window_header(ui, title, maximized)` — returns `DesktopHeaderAction`

---

## How to Resume

1. `cd /home/stylith/RobCoUnified && cargo check` to confirm clean state
2. Read `LLM_HANDOFF.md` for base architecture
3. Read this file for current state
4. Read the spec docs (`ASSET_CACHE_IMPL.md`, `CONTEXT_MENU_IMPL.md`) before implementing
   those features — they have all the egui API details and integration points worked out
5. Run `cargo run --bin robcos-native` to see current state

---

## Files to Know

```
src/native/app.rs           Main file — 7800 lines, all window draw functions
src/native/file_manager.rs  File manager state (recently rewritten)
src/native/retro_ui.rs      RetroScreen grid renderer
src/config.rs               All settings structs, persistence
src/native/settings_screen.rs  Settings panel renderer
LLM_HANDOFF.md              Base project context (read this too)
ASSET_CACHE_IMPL.md         Spec: desktop icons + wallpaper
CONTEXT_MENU_IMPL.md        Spec: right-click context menus
FILE_MANAGER_REWRITE.md     Spec: file_manager.rs full rewrite
```
