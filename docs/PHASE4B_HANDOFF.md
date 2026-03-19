# Phase 4b Handoff — Theme Polish + Egui Removal

**Written:** 2026-03-19
**Status:** Phases 3a–3h complete. Phase 4 in progress (theme infrastructure landed, widget-level styling incomplete).
**Binary:** `cargo run -p robcos-native-shell --bin robcos-iced`
**Egui binary:** Being removed in current uncommitted changes — do not restore it.

---

## What Is Working Right Now

- **All desktop app windows** have real views:
  - `FileManager` — scrollable list, single-click selects, second-click opens, search bar, ↑ Up toolbar
  - `Editor` — `iced::widget::text_editor`, monospace, dirty flag in status bar
  - `Settings` — full panel nav with sub-panels, theme picker, toggles, sliders
  - `Applications` — section-based program launcher
  - `Installer` — package list with available/installed status
  - `NukeCodes` — word-grid puzzle UI
  - `PtyApp` — PTY canvas renderer via `PtyCanvas` + `CommittedFrame` (real bash/zsh session)
- **Terminal mode** fully wired (login, all screens, keyboard nav, back, logout)
- **RetroTheme infrastructure** in `src/native/retro_iced_theme.rs`:
  - `retro_theme()` — returns `Theme::custom()` with RetroColors palette
  - `window_background()`, `panel_background()`, `bordered_panel()`, `overlay_panel()`, `separator()` — container helpers
  - `terminal_text_input()` — text input styled with retro palette
  - `theme()` method on `RobcoShell` now returns `retro_theme()` (not `Theme::Dark`)
- **Settings handlers** fully wired (theme, open mode, window mode, sound, volume, hints, bootup, menu visibility)
- **FileManager** single-click-to-select + second-click-to-open via `Message::FileManagerRowPressed`
- **Egui binary being removed** — `main.rs` deleted, `[[bin]] robcos-native` removed from Cargo.toml

---

## State of Uncommitted Changes

There are uncommitted working-tree changes (~780 added lines in shell.rs + Cargo.toml + message.rs + mod.rs). These are **Phase 4 work in progress** and should be committed before continuing. The changes are clean — `cargo check` passes with zero errors. Commit them as:

```
Phase 4a: retro theme infrastructure and settings handlers

- Add retro_iced_theme.rs with Theme::custom() palette and container/input helpers
- Wire theme() to return retro_theme() instead of Theme::Dark
- Replace per-widget style closures with retro_iced_theme helpers in terminal views
- Add full settings message handlers (theme, open mode, sound, volume, hints)
- Add FileManagerRowPressed for click-to-select, FileManagerSearchChanged
- Remove robcos-native egui binary (delete main.rs, remove [[bin]])
- Add new Message variants for settings changes
```

---

## Why the UI Still Looks "Wrong"

**This is expected at the current stage.** The root cause:

`Theme::custom()` in iced 0.13 sets the `Palette` (background, text, primary, success, danger) which propagates to *some* widget styles automatically. But iced's built-in widgets each have their own `Catalog` trait implementations — `Theme::Custom` gets "best effort" styling, not pixel-perfect retro.

**What looks correct:**
- Overall window background (black)
- Text color (green) where explicit `.color(fg)` is used
- Desktop WM chrome (custom-drawn via fill_quad — bypasses theme entirely)
- Terminal mode views (all explicit color calls)

**What looks wrong (and why):**
1. **Buttons** — iced's built-in button style for `Theme::Custom` applies palette.primary as background on hover. This gives green-tinted hover states, not the flat retro look.
2. **`text_editor` widget** — has its own internal gray/white styling; won't fully match the retro palette without a custom Catalog impl.
3. **Scrollbars** — use palette.primary (green) which may appear too vivid.
4. **14 remaining `.style(move |_t| ...)` closures** — not yet migrated to theme helpers; these are scattered through the start menu, taskbar, file manager, and spotlight views.

---

## Phase 4b Tasks — Fix the Widget Styling

### Priority 1: Migrate remaining 14 old-style closures

Find them:
```bash
grep -n "\.style(move |" src/native/shell.rs
```

Current locations (as of this handoff):
- Lines ~2113, 2149, 2220, 2293 — start menu buttons
- Lines ~2390, 2404 — taskbar buttons
- Lines ~3606, 3619, 3655, 3686 — file manager row buttons
- Lines ~3747, 3792, 3925, 3963 — spotlight + other

For each, either:
a. Replace with an existing `retro_iced_theme::*` helper if one fits
b. Add a new helper to `retro_iced_theme.rs` then use it

### Priority 2: Fix button styling

Add to `retro_iced_theme.rs`:

```rust
use iced::widget::button;

pub fn retro_button(_theme: &Theme, status: button::Status) -> button::Style {
    let p = current_retro_colors();
    let bg = match status {
        button::Status::Hovered | button::Status::Pressed => p.selected_bg.to_iced(),
        _ => p.bg.to_iced(),
    };
    let text = match status {
        button::Status::Hovered | button::Status::Pressed => p.selected_fg.to_iced(),
        _ => p.fg.to_iced(),
    };
    button::Style {
        background: Some(iced::Background::Color(bg)),
        text_color: text,
        border: iced::Border { color: p.dim.to_iced(), width: 1.0, radius: 0.0.into() },
        ..Default::default()
    }
}

pub fn retro_button_flat(_theme: &Theme, status: button::Status) -> button::Style {
    let p = current_retro_colors();
    let bg = match status {
        button::Status::Hovered | button::Status::Pressed => p.selected_bg.to_iced(),
        _ => p.bg.to_iced(),
    };
    button::Style {
        background: Some(iced::Background::Color(bg)),
        text_color: p.fg.to_iced(),
        border: iced::Border { color: iced::Color::TRANSPARENT, width: 0.0, radius: 0.0.into() },
        ..Default::default()
    }
}
```

Use `retro_button` for bordered buttons (toolbar, dialogs), `retro_button_flat` for list rows and icon buttons.

### Priority 3: Fix text_editor styling

`iced::widget::text_editor` in 0.13 uses `text_editor::Catalog`. To style it, use a closure:

```rust
text_editor(&self.editor_content)
    .on_action(Message::TextEditorAction)
    .font(Font::MONOSPACE)
    .size(13.0)
    .style(|theme, status| {
        let p = current_retro_colors();
        iced::widget::text_editor::Style {
            background: p.bg.to_iced_bg(),
            border: iced::Border {
                color: p.dim.to_iced(),
                width: 1.0,
                radius: 0.0.into(),
            },
            icon: p.fg.to_iced(),
            placeholder: p.dim.to_iced(),
            value: p.fg.to_iced(),
            selection: p.selection_bg.to_iced(),
        }
    })
    .height(Length::Fill)
```

Check `RetroColors` for a `selection_bg` field — if absent, use `selected_bg`.

### Priority 4: Fix scrollbar styling

`scrollable::Style` can be set per-scrollable. Add to `retro_iced_theme.rs`:

```rust
pub fn retro_scrollable(_theme: &Theme, status: scrollable::Status) -> scrollable::Style {
    let p = current_retro_colors();
    let rail_color = p.bg.to_iced();
    let scroller_color = match status {
        scrollable::Status::Dragged { .. } | scrollable::Status::Hovered { .. } => p.fg.to_iced(),
        _ => p.dim.to_iced(),
    };
    scrollable::Style {
        container: container::Style::default(),
        vertical_rail: scrollable::Rail {
            background: Some(iced::Background::Color(rail_color)),
            border: iced::Border::default(),
            scroller: scrollable::Scroller {
                color: scroller_color,
                border: iced::Border { radius: 0.0.into(), ..Default::default() },
            },
        },
        horizontal_rail: scrollable::Rail { /* same */ ..Default::default() },
        gap: None,
    }
}
```

---

## Phase 5 — Cleanup (after Phase 4b)

The egui binary is already being removed in the uncommitted changes. After those are committed:

1. Delete `src/native/app.rs` — the 13k-line egui app. Verify nothing in the iced binary references it first:
   ```bash
   grep -r "native::app\|super::app" src/native/ crates/native-shell/src/
   ```
2. Remove `pub mod app;` from `src/native/mod.rs`
3. Remove egui/eframe from workspace `Cargo.toml` if no other crate uses them:
   ```bash
   grep -r "egui\|eframe" crates/ src/ --include="*.toml" | grep -v "native-shell"
   ```
4. Remove ratatui if unused (the PTY canvas switched to `CommittedFrame` which may not need ratatui directly):
   ```bash
   grep -r "ratatui" src/ crates/ --include="*.rs" | grep -v "test"
   ```
5. Run `cargo test` — must not regress below 20 failing tests.

---

## RetroColors Fields Reference

For use when writing theme helpers — key fields on `RetroColors`:

```
palette.fg       — primary text (green)
palette.bg       — background (black)
palette.dim      — dimmed text / borders (dark green)
palette.panel    — slightly lighter bg for panels
palette.selected_bg — selection background
palette.selected_fg — selection text
palette.err      — error red (if present)
```

Check `src/native/retro_theme.rs` for the full struct.

---

## Build Commands

```bash
cargo check -p robcos-native-shell --bin robcos-iced   # fast
cargo build -p robcos-native-shell --bin robcos-iced   # full build
cargo run   -p robcos-native-shell --bin robcos-iced   # run
```

---

## Commit Convention

No `Co-Authored-By` lines. Format: `Phase Nx: short description\n\n- bullets`.
