# Phase 3h Handoff — Hosted Desktop Apps + PTY Terminal

**Written:** 2026-03-19
**Status:** Phases 3a–3g complete. Phase 3h is next.
**Binary:** `cargo run -p robcos-native-shell --bin robcos-iced`
**Egui binary:** `cargo run -p robcos-native-shell --bin robcos-native` — still untouched.

---

## What Is Working Right Now (post-3g)

- **Terminal mode** fully functional: login screen, password prompt, main menu, all sub-screen navigation (Applications, Documents, Network, Games, Settings, sub-sub-screens), logout, Return to Desktop. Arrow keys + Enter + Esc all wired.
- **Desktop mode** (3a–3f): inner window manager widget (drag/resize/chrome), taskbar, top bar with clock, start menu, spotlight search overlay, desktop surface icon grid.
- **Both binaries compile cleanly.** `robcos-native` (egui) is untouched.

---

## What Phase 3h Must Do

Replace the **placeholder content** inside every desktop window with the real hosted app UI, and implement the **PTY terminal app** inside a desktop window.

Current placeholder (in `view_desktop()` → `view_surface_icons()` in `shell.rs`):
```rust
// Each WindowChild gets this content today:
column![
    text(format!("{:?}", id)).size(15).color(fg),
    text("Window content placeholder").size(11).color(dim),
]
```

Phase 3h replaces this with real app views per `DesktopWindow` variant.

---

## Architecture for Hosted Apps

### Step 1 — Add `view` to `DesktopApp` trait (`src/native/shell.rs`)

The `DesktopApp` trait currently has no `view()` method (it was deferred). Add it:

```rust
// In shell.rs, the DesktopApp trait block:
fn view<'a>(&'a self, shell: &'a RobcoShell) -> Element<'a, Message>;
```

Passing `&RobcoShell` gives each app access to palette, settings, and shared state without
storing duplicates.

### Step 2 — Implement per-app view helpers on RobcoShell

Rather than the trait approach, the simpler path (matching the egui pattern) is to add view
methods directly on `RobcoShell` and dispatch by `DesktopWindow` variant:

```rust
// In view_desktop(), replace the placeholder WindowChild content:
let content: Element<'_, Message> = match w.id {
    DesktopWindow::FileManager => self.view_file_manager(),
    DesktopWindow::Editor      => self.view_editor(),
    DesktopWindow::Settings    => self.view_settings(),
    DesktopWindow::Applications => self.view_applications_app(),
    DesktopWindow::Installer   => self.view_installer(),
    DesktopWindow::NukeCodes   => self.view_nuke_codes(),
    DesktopWindow::PtyApp      => self.view_pty_terminal(),
    _                          => placeholder_content(w.id, fg, dim, bg),
};
```

---

## Per-App Implementation Notes

### FileManager (`DesktopWindow::FileManager`)

**Existing state:** `self.file_manager: NativeFileManagerState`
**Existing egui logic:** `src/native/file_manager_app.rs` — port its view logic to iced widgets.

Key iced widgets needed:
- `column![]` for directory listing rows
- `button` for each file/folder entry with hover highlight
- `text_input` for the path bar
- `scrollable` wrapper around the file list

```rust
fn view_file_manager(&self) -> Element<'_, Message> {
    // Port from file_manager_app.rs draw_file_manager_panel()
    // Dispatch FileManagerCommand messages for nav/open/delete
}
```

`FileManagerCommand` is already in the Message enum as `Message::FileManagerCommand(FileManagerCommand)`.

### Editor (`DesktopWindow::Editor`)

**Existing state:** `self.editor: EditorWindow`
**Existing egui logic:** `src/native/editor_app.rs`

For iced, use `iced::widget::text_editor::Content` for the text buffer. This is a proper iced
text editor widget with syntax highlighting support via `iced::highlighter`.

```rust
fn view_editor(&self) -> Element<'_, Message> {
    use iced::widget::text_editor;
    text_editor(&self.editor_content)  // needs Content field on RobcoShell
        .on_action(Message::EditorCommand)
        .font(iced::Font::MONOSPACE)
        .into()
}
```

**Note:** `EditorWindow` (egui type) should be replaced with `iced::widget::text_editor::Content`.
Add `pub editor_content: iced::widget::text_editor::Content` to `RobcoShell` and remove the
egui `editor: EditorWindow` field.

### Settings (`DesktopWindow::Settings`)

**Existing state:** `self.settings_panel: Option<NativeSettingsPanel>`
**Existing egui logic:** `src/native/settings_screen.rs`

Port as a scrollable form with labeled sections and toggle/picker controls.

### Installer (`DesktopWindow::Installer`)

Standalone app in `crates/native-installer-app/`. Port its view to iced. Has its own state —
add `pub installer: NativeInstallerState` (or equivalent) to `RobcoShell`.

### NukeCodes (`DesktopWindow::NukeCodes`)

Standalone app in `crates/native-nuke-codes-app/`. Simple word-grid puzzle UI.

### Applications (`DesktopWindow::Applications`)

Lists installed/configured apps. Relatively simple list view.

---

## PTY Terminal App (`DesktopWindow::PtyApp`)

This is the actual bash/zsh terminal emulator running inside a desktop window.

### State to add to RobcoShell

```rust
pub pty_master: Option<std::os::fd::OwnedFd>,  // master side of PTY
pub vt_parser: vt100::Parser,                  // VT100 screen state
pub pty_title: String,                          // title from OSC sequences
```

The `vt100` crate is already a workspace dependency.

### PTY launch

```rust
// In update() for Message::OpenWindow(DesktopWindow::PtyApp):
fn launch_pty(&mut self) -> Task<Message> {
    use std::process::Command;
    // Use portable-pty crate (already a dep?) or nix pty::openpty
    // Fork child with $SHELL, get master fd
    // Store master fd in self.pty_master
    Task::none()
}
```

Check if `portable-pty` or `nix` is already in the dependency tree:
```bash
grep -r "portable.pty\|nix\b" Cargo.toml crates/*/Cargo.toml
```

### PTY output subscription

Add to `subscription()` in `shell.rs` when the PTY is open:

```rust
if self.pty_master.is_some() {
    let pty_sub = iced::subscription::channel(
        "robcos-pty",
        256,
        |mut tx| async move {
            // async read loop from master fd → tx.send(Message::PtyOutput(bytes))
            loop {
                let bytes = tokio::io::AsyncReadExt::read(&mut master_reader, &mut buf).await;
                let _ = tx.send(Message::PtyOutput(bytes.to_vec())).await;
            }
        },
    );
    subs.push(pty_sub);
}
```

### PTY view — canvas widget

Create `src/native/terminal_canvas.rs`:

```rust
use iced::widget::canvas::{self, Frame, Geometry};
use iced::{Color, Font, Point, Rectangle, Size};
use crate::native::retro_theme::RetroColors;

pub struct PtyCanvas<'a> {
    pub screen: &'a vt100::Screen,
    pub palette: RetroColors,
}

impl<'a> canvas::Program<super::message::Message> for PtyCanvas<'a> {
    type State = ();

    fn draw(&self, _state: &(), renderer: &iced::Renderer, _theme: &iced::Theme,
            bounds: Rectangle, _cursor: canvas::Cursor) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        let cols = self.screen.size().1 as usize;
        let rows = self.screen.size().0 as usize;
        let cell_w = bounds.width / cols as f32;
        let cell_h = bounds.height / rows as f32;

        for row in 0..rows {
            for col in 0..cols {
                let cell = self.screen.cell(row as u16, col as u16).unwrap_or_default();
                // Draw background quad, then character
                let bg = cell_bg_color(cell, &self.palette);
                let fg = cell_fg_color(cell, &self.palette);
                frame.fill_rectangle(
                    Point::new(col as f32 * cell_w, row as f32 * cell_h),
                    Size::new(cell_w, cell_h),
                    bg,
                );
                if let Some(ch) = cell.contents().chars().next() {
                    frame.fill_text(canvas::Text {
                        content: ch.to_string(),
                        position: Point::new(col as f32 * cell_w, row as f32 * cell_h),
                        color: fg,
                        size: (cell_h * 0.85).into(),
                        font: Font::MONOSPACE,
                        ..canvas::Text::default()
                    });
                }
            }
        }
        vec![frame.into_geometry()]
    }
}
```

Then in `view_pty_terminal()`:
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

In `on_event()` for the `DesktopWindowHost` widget when the active window is `PtyApp`,
keyboard events need to be converted to bytes and sent as `Message::PtyInput`. Add keyboard
handling to the PTY window's content widget, or intercept in the subscription's hotkeys.

---

## File Manager Prompt State

The existing `NativeFileManagerState` already carries rename/create/delete prompt state.
For iced, these prompts should use the `TerminalPrompt` overlay pattern already established
in terminal mode (Phase 3g). Reuse `view_terminal_prompt_overlay()` or create a similar
`view_desktop_prompt_overlay()`.

---

## Window Titles

Currently all windows show `format!("{:?}", id)` as title. Once real app views land, each
`DesktopApp` impl provides its proper title:
- FileManager: `"File Manager"` or current path
- Editor: `"Editor — filename.txt"` or `"Editor — Untitled"`
- Settings: `"Settings"`
- PtyApp: `self.pty_title` (updated via `Message::PtyTitleChanged`)

Update `WindowChild.title` construction in `view_desktop()` to call the right title.

---

## RobcoShell Fields to Add in 3h

```rust
// Replace egui-typed fields:
// REMOVE: pub editor: EditorWindow  (egui type)
// ADD:
pub editor_content: iced::widget::text_editor::Content,

// ADD for PTY:
pub pty_master: Option<tokio::process::ChildStdin>,  // or OwnedFd
pub vt_parser: vt100::Parser,
pub pty_title: String,

// ADD for installer (if porting it as a desktop app):
// pub installer: <whatever NativeInstallerApp state looks like>
```

---

## Implementation Order (recommended)

1. **FileManager** — most complex, most useful. Gives a real sense of the app-hosting pattern.
2. **Editor** — straightforward, good test of `iced::widget::text_editor`.
3. **Settings** — form-based, good test of input/toggle widgets.
4. **PTY terminal app** — separate canvas widget; do this after the simpler apps are working.
5. **NukeCodes / Applications / Installer** — lower priority, mostly mechanical ports.

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

No `Co-Authored-By` lines. Format: `Phase 3h: short description\n\n- bullet list`.

---

## What Stays Unchanged

- `src/native/app.rs` — egui app, do not touch
- `crates/native-shell/src/main.rs` — egui entry point, do not touch
- All pre-existing test failures (20) — do not regress further

---

## Phase 4 Preview (after 3h)

RetroTheme polish — replace `Theme::Dark` with a real custom iced theme that applies `RetroColors`
to every widget style automatically, eliminating the per-widget `.style(move |_t| ...)` closures
that are scattered everywhere. Implement `iced::theme::Custom` or the `Catalog` trait.

Phase 5: remove egui binary, ratatui dependency, dead code.
