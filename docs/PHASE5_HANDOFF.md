# Phase 5 Handoff — Egui Removal and Dead Code Cleanup

**Written:** 2026-03-19
**Status:** Phase 4b complete. Phase 5 is next.
**Binary:** `cargo run -p robcos-native-shell --bin robcos-iced`

---

## What Is Working Right Now (post-4b)

- Full iced desktop shell with all app windows, real content, retro styling
- Zero old-style `.style(move |_t| ...)` closures remaining in `shell.rs`
- Complete `retro_iced_theme.rs` button/scrollable/text_editor/container helpers
- `theme()` returns `retro_theme()` backed by `Theme::custom()` with RetroColors palette
- Both the `robcos-iced` binary and the 6 standalone egui binaries compile cleanly

---

## The Complexity — Why Phase 5 Is Not a Simple Delete

Three entangled layers need to be unwound in order:

### Layer 1 — `app.rs` has real dependents

`src/native/app.rs` (10,794 lines) is not just the old main shell — it also backs 6 modules that were extracted from it but never fully decoupled:

| Module | What it imports from app.rs |
|--------|-----------------------------|
| `desktop_start_menu.rs` | `RobcoNativeApp`, `ContextMenuAction` |
| `desktop_window_mgmt.rs` | `RobcoNativeApp` |
| `desktop_surface.rs` | `RobcoNativeApp`, `ContextMenuAction` + others |
| `desktop_menu_bar.rs` | `RobcoNativeApp` |
| `desktop_taskbar.rs` | `RobcoNativeApp` |
| `desktop_spotlight.rs` | `RobcoNativeApp` |

These modules contain `impl RobcoNativeApp` blocks — egui draw methods for the old shell. They are **not used by `robcos-iced`** but are compiled as part of the crate because `app.rs` is `pub mod app`.

### Layer 2 — 6 standalone egui binaries still live

The following binaries remain declared in `crates/native-shell/Cargo.toml`:

| Binary | Entry point | Purpose |
|--------|-------------|---------|
| `robcos-file-manager` | `file_manager_main.rs` | Standalone egui file manager |
| `robcos-editor` | `editor_main.rs` | Standalone egui editor |
| `robcos-settings` | `settings_main.rs` | Standalone egui settings |
| `robcos-applications` | `applications_main.rs` | Standalone egui app browser |
| `robcos-nuke-codes` | `nuke_codes_main.rs` | Standalone egui nuke codes |
| `robcos-installer` | `installer_main.rs` | Standalone egui installer |

Each of these imports `configure_native_context` and/or `RobcoNativeApp` from `app.rs`. They must be removed **before** `app.rs` can be deleted.

### Layer 3 — ratatui is baked into the PTY infrastructure

`crates/shared/src/pty.rs` uses ratatui to build `CommittedFrame` / `PtyStyledCell` — the types that `src/native/terminal_canvas.rs` (`PtyCanvas`) reads to render the PTY screen. Removing ratatui requires refactoring the PTY rendering pipeline, which is a separate project. **Defer ratatui removal to Phase 6.**

---

## Phase 5 — Execution Order

### Step 1 — Remove the 6 standalone egui binaries from Cargo.toml

In `crates/native-shell/Cargo.toml`, delete these 6 `[[bin]]` blocks:

```toml
# DELETE all of these:
[[bin]]
name = "robcos-file-manager"
path = "src/file_manager_main.rs"

[[bin]]
name = "robcos-settings"
path = "src/settings_main.rs"

[[bin]]
name = "robcos-editor"
path = "src/editor_main.rs"

[[bin]]
name = "robcos-applications"
path = "src/applications_main.rs"

[[bin]]
name = "robcos-nuke-codes"
path = "src/nuke_codes_main.rs"

[[bin]]
name = "robcos-installer"
path = "src/installer_main.rs"
```

Keep `[[bin]] robcos-iced`. Run `cargo check -p robcos-native-shell --bin robcos-iced` — should still compile.

Then delete the now-orphaned entry point files:

```bash
rm crates/native-shell/src/file_manager_main.rs
rm crates/native-shell/src/editor_main.rs
rm crates/native-shell/src/settings_main.rs
rm crates/native-shell/src/applications_main.rs
rm crates/native-shell/src/nuke_codes_main.rs
rm crates/native-shell/src/installer_main.rs
```

Run `cargo check` again to verify.

### Step 2 — Remove the 6 extracted egui modules that back app.rs

These modules contain `impl RobcoNativeApp` blocks that are only used by the egui binary. Delete them:

```bash
rm src/native/desktop_menu_bar.rs
rm src/native/desktop_taskbar.rs
rm src/native/desktop_spotlight.rs
rm src/native/desktop_surface.rs
rm src/native/desktop_start_menu.rs
rm src/native/desktop_window_mgmt.rs
```

**Before deleting each file**, check that nothing in the iced shell imports from them. The iced shell (`shell.rs`) has its own implementations of start menu, taskbar, spotlight, surface icons — it does NOT use these egui modules.

```bash
grep -rn "desktop_menu_bar\|desktop_taskbar\|desktop_spotlight\|desktop_surface\|desktop_window_mgmt" \
  src/native/shell.rs src/native/message.rs crates/native-shell/src/iced_main.rs
```

Should return zero results. If any hits appear, the iced shell has an unexpected dependency — investigate before deleting.

Also remove their `mod` declarations from `src/native/mod.rs`:

```rust
// DELETE these lines in mod.rs:
mod desktop_menu_bar;
mod desktop_taskbar;
mod desktop_spotlight;
mod desktop_surface;
mod desktop_start_menu;
mod desktop_window_mgmt;
```

Run `cargo check` after each deletion.

### Step 3 — Remove the remaining egui-only modules

Check which modules in `mod.rs` are exclusively used by `app.rs` and have no iced equivalents. Candidates (verify each with grep before deleting):

```
mod applications_standalone;   -- egui standalone app
mod editor_standalone;         -- egui standalone app
mod file_manager_standalone;   -- egui standalone app
mod installer_standalone;      -- egui standalone app
mod nuke_codes_standalone;     -- egui standalone app
mod settings_standalone;       -- egui standalone app
mod standalone_launcher;       -- egui launch helpers
mod hacking_screen;            -- egui-rendered screen
mod retro_ui;                  -- ratatui-based retro rendering (iced has terminal_canvas.rs)
mod pty_screen;                -- ratatui-based PTY view (replaced by terminal_canvas.rs)
mod shell_screen;              -- egui terminal-mode rendering
mod menu;                      -- egui menu helpers
mod prompt_flow;               -- egui prompt flow (iced has prompt.rs)
```

For each, grep for usage in iced code first:
```bash
grep -rn "module_name" src/native/shell.rs src/native/message.rs crates/native-shell/src/
```

Delete only those with zero hits in iced code.

### Step 4 — Remove `app.rs` itself

Once all its dependents are gone:

```bash
rm src/native/app.rs
```

Remove from `mod.rs`:
```rust
// DELETE:
pub mod app;
pub use app::{apply_native_appearance, configure_native_context, RobcoNativeApp};
pub use applications_standalone::RobcoNativeApplicationsApp;
pub use editor_standalone::RobcoNativeEditorApp;
pub use file_manager_standalone::RobcoNativeFileManagerApp;
pub use installer_standalone::RobcoNativeInstallerApp;
pub use nuke_codes_standalone::RobcoNativeNukeCodesApp;
pub use settings_standalone::{...};
pub use standalone_launcher::ROBCOS_NATIVE_STANDALONE_USER_ENV;
```

Run `cargo check`. If it compiles: proceed.

### Step 5 — Remove eframe/egui from Cargo.toml

Only after `app.rs` and all standalone modules are gone, verify nothing uses egui:

```bash
grep -rn "egui\|eframe" src/ crates/ --include="*.rs" | grep -v "test\|#"
```

If the output is empty, remove from both:

`Cargo.toml` (workspace):
```toml
# DELETE:
eframe = { version = "0.29", ... }
```

`crates/native-shell/Cargo.toml`:
```toml
# DELETE:
eframe = { version = "0.29", ... }
```

Run `cargo build -p robcos-native-shell --bin robcos-iced` to verify the build still works without egui.

### Step 6 — Run the test suite

```bash
cargo test 2>&1 | grep -E "^test result|FAILED" | tail -20
```

Must not regress below 20 pre-existing failures. If new failures appear, investigate before proceeding.

---

## What NOT to Touch in Phase 5

- `ratatui` — defer to Phase 6. It's embedded in `crates/shared/src/pty.rs` (`CommittedFrame`), which `terminal_canvas.rs` depends on. Removing it requires refactoring the PTY rendering pipeline.
- `src/native/prompt.rs` — used by the iced shell (terminal prompt overlays)
- `src/native/retro_theme.rs` — used by the iced shell
- `src/native/retro_iced_theme.rs` — used by the iced shell
- `src/native/terminal_canvas.rs` — used by the iced shell (PTY canvas widget)
- `src/native/desktop_wm_widget.rs` — used by the iced shell (inner WM widget)

---

## Phase 6 Preview (after Phase 5)

- Refactor PTY rendering: replace `ratatui::Frame` / `CommittedFrame` with a direct `vt100::Screen` or a minimal cell-grid struct, eliminating the ratatui dependency
- Move `crates/shared/src/pty.rs` rendering code to use only `vt100` + `PtyCanvas` directly
- Remove ratatui from `Cargo.toml` and `crates/shared/Cargo.toml`

---

## Build Commands

```bash
cargo check -p robcos-native-shell --bin robcos-iced   # fast — run after each deletion
cargo build -p robcos-native-shell --bin robcos-iced   # full build
cargo test                                              # must not regress
```

---

## Commit Convention

No `Co-Authored-By` lines. Format: `Phase 5: short description\n\n- bullet list`.

Recommended granular commits:
1. `Phase 5a: remove standalone egui binaries from Cargo.toml and delete entry points`
2. `Phase 5b: remove egui desktop shell modules (start_menu, taskbar, surface, etc.)`
3. `Phase 5c: delete app.rs and remaining egui-only modules`
4. `Phase 5d: remove eframe/egui from workspace dependencies`
