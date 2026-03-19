# Phase 4 Handoff — RetroTheme + PTY Terminal

**Written:** 2026-03-19
**Status:** Phases 3a–3h complete (with one 3h gap: PTY). Phase 4 is next.
**Binary:** `cargo run -p robcos-native-shell --bin robcos-iced`
**Egui binary:** `cargo run -p robcos-native-shell --bin robcos-native` — still untouched.

---

## What Is Working Right Now (post-3h)

- **Terminal mode** fully functional: login, password prompt, main menu, all sub-screen navigation, logout, Return to Desktop.
- **Desktop mode**: drag/resize inner windows, taskbar, top bar, start menu, spotlight search, desktop icon grid.
- **Hosted app windows** — real views for all DesktopWindow variants:
  - `FileManager` → scrollable directory list + `↑ Up` toolbar + path display. Commits `FileManagerCommand` on activate.
  - `Editor` → `iced::widget::text_editor` with monospace font, live keystrokes via `Message::TextEditorAction`, dirty indicator in status bar.
  - `Settings` → scrollable list of settings tiles from `desktop_settings_home_rows()`.
  - `NukeCodes`, `Applications`, `Installer`, `PtyApp`, `DonkeyKong`, `TerminalMode` → "Coming soon" placeholder.
- **Window titles** reflect real app state (path + `*` suffix when editor is dirty, cwd for file manager).
- **Both binaries compile cleanly.** Zero errors, ~7 pre-existing warnings.

---

## Phase 3h Gap — PTY Terminal App (still outstanding)

The PTY terminal (`DesktopWindow::PtyApp`) shows a "Coming soon" placeholder. Complete this **before or alongside** Phase 4.

See the full PTY spec in `docs/PHASE3H_HANDOFF.md` (§ PTY Terminal App). Summary:

### Fields to add to `RobcoShell` (in `shell.rs`)

```rust
// In the "Hosted app state" block:
pub pty_master: Option<tokio::process::ChildStdin>,  // or OwnedFd
pub vt_parser: vt100::Parser,
pub pty_title: String,
```

`vt100` is already a workspace dependency. Check with:
```bash
grep "vt100" Cargo.toml crates/*/Cargo.toml
```

### PTY launch — in `update()` for `Message::OpenWindow(DesktopWindow::PtyApp)`

```rust
// After opening the window, spawn a pty:
use std::os::unix::io::AsRawFd;
let pty = portable_pty::native_pty_system().openpty(Default::default()).unwrap();
// OR use nix::pty::openpty / tokio::process with stdin/stdout piped
```

Check whether `portable-pty` or `nix` is already available:
```bash
grep -r "portable.pty\|^nix\b" Cargo.toml crates/*/Cargo.toml
```

If neither is present, add `portable-pty` to the workspace.

### PTY output subscription — in `subscription()`

```rust
if self.pty_master.is_some() {
    let pty_sub = iced::subscription::channel(
        "robcos-pty",
        256,
        |mut tx| async move {
            loop {
                // async read from master fd
                let _ = tx.send(Message::PtyOutput(bytes)).await;
            }
        },
    );
    subs.push(pty_sub);
}
```

### PTY view — canvas widget

Create `src/native/terminal_canvas.rs` using `iced::widget::canvas`. Full code in `docs/PHASE3H_HANDOFF.md` §PTY view.

Then in `view_window_placeholder` match arm for `DesktopWindow::PtyApp`, replace with:

```rust
DesktopWindow::PtyApp => self.view_pty_terminal(),
```

```rust
fn view_pty_terminal(&self) -> Element<'_, Message> {
    use iced::widget::canvas;
    canvas(PtyCanvas { screen: self.vt_parser.screen(), palette: current_retro_colors() })
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}
```

### PTY keyboard input

When active window is `PtyApp`, intercept keyboard events in `subscription()` and route to `Message::PtyInput(Vec<u8>)`. In `update()`:

```rust
Message::PtyInput(bytes) => {
    if let Some(ref mut stdin) = self.pty_master {
        // write bytes to pty master
    }
}
Message::PtyOutput(bytes) => {
    self.vt_parser.process(&bytes);
}
Message::PtyTitleChanged(title) => {
    self.pty_title = title;
}
```

---

## Phase 4 — RetroTheme (custom iced theme)

**Goal:** Replace `Theme::Dark` with a real custom iced theme that automatically applies `RetroColors` to every widget style — eliminating the hundreds of per-widget `.style(move |_t| ...)` closures currently scattered through `shell.rs`.

### Current situation

`shell.rs`'s `theme()` returns `Theme::Dark`:
```rust
pub fn theme(&self) -> Theme {
    Theme::Dark
}
```

Every widget that needs retro colors overrides it inline, e.g.:
```rust
container(...).style(move |_t| container::Style {
    background: Some(iced::Background::Color(bg)),
    ..Default::default()
})
```

### Target

Implement `iced::theme::Custom` (or the `Catalog` trait) so every widget picks up RetroColors by default, and the inline style overrides are no longer needed.

### How to implement in iced 0.13

iced 0.13 uses the `Catalog` trait for theming. Each widget has a `Catalog` impl that maps from a style enum to a concrete `Style` struct.

**Step 1 — Create `src/native/retro_iced_theme.rs`**

```rust
use iced::{Color, Theme};
use super::retro_theme::current_retro_colors;

/// Returns an `iced::Theme` configured with RetroColors.
/// Pass this as the application theme.
pub fn retro_theme() -> Theme {
    let p = current_retro_colors();
    Theme::custom(
        "RobCoOS".to_string(),
        iced::theme::Palette {
            background: p.bg.to_iced(),
            text: p.fg.to_iced(),
            primary: p.fg.to_iced(),
            success: p.fg.to_iced(),
            danger: p.err.to_iced(),   // check RetroColors for an error color field
        },
    )
}
```

**Step 2 — Update `theme()` in `shell.rs`**

```rust
pub fn theme(&self) -> Theme {
    super::retro_iced_theme::retro_theme()
}
```

**Step 3 — Remove per-widget style closures**

Once the theme propagates automatically, container backgrounds, text colors, button styles, etc. no longer need per-call `.style()` overrides. Remove them file by file, checking `cargo check` after each pass.

**Step 4 — Implement widget-level Catalog extensions (if needed)**

If `Theme::custom()` doesn't fully cover a widget (e.g. `button` needs a custom hover color), implement the relevant `Catalog` trait:

```rust
// In retro_iced_theme.rs:
impl iced::widget::button::Catalog for RetroTheme {
    type Class<'a> = iced::widget::button::StyleFn<'a, RetroTheme>;

    fn style(&self, class: &Self::Class<'_>, status: iced::widget::button::Status)
        -> iced::widget::button::Style
    {
        let p = current_retro_colors();
        iced::widget::button::Style {
            background: Some(iced::Background::Color(p.bg.to_iced())),
            text_color: p.fg.to_iced(),
            border: iced::Border { color: p.dim.to_iced(), width: 1.0, radius: 2.0.into() },
            ..Default::default()
        }
    }
}
```

**Note:** Full custom theme implementation (implementing all Catalog traits) is involved. Start with `Theme::custom()` palette and verify how much it handles automatically before going deeper.

---

## Phase 5 — Cleanup (after Phase 4)

- Remove `robcos-native` egui binary:
  - Delete `crates/native-shell/src/main.rs` (egui entry point)
  - Remove the `[[bin]]` for `robcos-native` from `crates/native-shell/Cargo.toml`
  - Remove egui/eframe dependencies from workspace `Cargo.toml` if nothing else uses them
- Remove ratatui dependency if unused
- Delete dead code: `src/native/app.rs` once no binary references it
- Run `cargo test` — the 20 pre-existing failures should stay at 20 (don't fix unrelated tests)

---

## FileManager View — Known Limitations (polish after PTY)

The current `view_file_manager()` is functional but simplified compared to the egui version:

1. **No click-to-select before open** — clicking any row immediately dispatches `OpenSelected`. Fix: add `FileManagerCommand::SelectPath(PathBuf)` or use `Message::FileManagerSelectRow(PathBuf)` to first update `file_manager.selected`, then a second click or Enter opens it.

2. **No sidebar tree panel** — the egui version shows a tree on the left. Deferred.

3. **No search bar** — `file_manager.search_query` is unused in the view. Add a `text_input` toolbar item wired to `FileManagerCommand` (search variant needs adding to the crate or handle via a local message).

4. **No multi-selection** — shift/ctrl click not wired.

---

## Settings View — Known Limitations

The current `view_settings_app()` shows read-only tile labels. To make it interactive:

1. Wire tile clicks to `Message::SettingsPanelChanged(NativeSettingsPanel)` to navigate into sub-panels.
2. Each sub-panel needs its own view (sliders, toggles, text inputs) — port from `src/native/settings_screen.rs`.

---

## Build Commands

```bash
cargo check -p robcos-native-shell --bin robcos-iced   # fast check
cargo build -p robcos-native-shell --bin robcos-iced   # full build
cargo run   -p robcos-native-shell --bin robcos-iced   # run
cargo check -p robcos-native-shell --bin robcos-native # verify egui still compiles
```

---

## Commit Convention

No `Co-Authored-By` lines. Format: `Phase Nx: short description\n\n- bullet list`.

---

## What Stays Unchanged

- `src/native/app.rs` — egui app, do not touch
- `crates/native-shell/src/main.rs` — egui entry point, do not touch
- All pre-existing test failures (20) — do not regress further
