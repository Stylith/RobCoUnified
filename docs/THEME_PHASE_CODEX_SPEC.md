# Nucleon Theme/Shell Composition — Codex Implementation Spec

> This document is the single source of truth for implementing the theme/shell composition system.
> Each phase is a self-contained unit of work. Do not skip ahead. Do not invent abstractions not described here.
> After each phase, `cargo check -p robcos` and `cargo check -p robcos-native-shell` must pass.
> The application must look and behave identically to the pre-refactor state at every phase boundary.

---

## Codebase orientation

### Workspace layout

Root `Cargo.toml` workspace members (all crates):
```
., crates/legacy-shell, crates/native-services, crates/native-shell, crates/shared,
crates/native-editor-app, crates/native-file-manager-app, crates/native-terminal-app,
crates/native-installer-app, crates/native-settings-app, crates/native-programs-app,
crates/native-default-apps-app, crates/native-connections-app, crates/native-edit-menus-app,
crates/native-document-browser-app, crates/native-about-app, crates/hosted-addon-contract,
crates/wasm-addon-sdk
```

### Key source files and their roles

| File | Role | Approx lines |
|------|------|--------------|
| `src/native/app.rs` | Main `RobcoNativeApp` struct (lines 483-574), Default impl (639+), appearance sync | ~850 |
| `src/native/app/frame_runtime.rs` | Main frame loop `update_native_shell_frame()` at line 286 | ~422 |
| `src/native/app/desktop_window_presenters.rs` | `draw_file_manager()` (38-191), `draw_editor()` (192-502), `draw_settings()` (503-end) | ~1555 |
| `src/native/app/settings_panels.rs` | Settings sub-panels: display effects (35-190), default apps (191-281), connections (282-405), CLI profiles (406-540), edit menus (541-650), user mgmt (651+) | ~1025 |
| `src/native/app/desktop_menu_bar.rs` | Top menu bar rendering (`draw_top_bar`, menu chrome) | ~400 |
| `src/native/app/desktop_taskbar.rs` | Bottom taskbar rendering (`draw_desktop_taskbar`) | ~250 |
| `src/native/app/desktop_start_menu.rs` | Start menu rendering (`draw_start_panel`) | ~600 |
| `src/native/app/desktop_spotlight.rs` | Spotlight/search overlay (`draw_spotlight`) | ~300 |
| `src/native/app/desktop_window_mgmt.rs` | Window tiling, z-order, rect tracking, header actions | ~500 |
| `src/native/app/desktop_surface.rs` | Desktop icon grid, drag-drop, wallpaper | ~600 |
| `src/native/app/desktop_runtime.rs` | Standalone window prepare/update functions | 277 |
| `src/native/desktop_app.rs` | `DesktopComponentBinding`, `DesktopComponentSpec`, `DESKTOP_COMPONENT_BINDINGS` array (7 entries), menu types, launch types | ~953 |
| `src/native/retro_ui.rs` | `RetroPalette`, `palette_for_theme_color()`, `current_palette()`, `RetroScreen`, `configure_visuals()` | 439 |
| `src/native/mod.rs` | Module declarations and re-exports | ~72 |
| `crates/native-shell/Cargo.toml` | Binary definitions (nucleon-native, nucleon-files, nucleon-settings, nucleon-text, nucleon-applications, nucleon-installer) | |

### Module path convention

All files in `src/native/app/` are sub-modules of `src/native/app.rs`. They use `super::super::` to reach `src/native/` siblings, and `super::RobcoNativeApp` to reach the app struct. Every function in these files is an `impl RobcoNativeApp` method with visibility `pub(super)` or `pub(crate)`.

### Current component binding system

In `src/native/desktop_app.rs`, lines 152-286:

```rust
#[derive(Clone, Copy)]
pub struct DesktopComponentBinding {
    pub spec: DesktopComponentSpec,
    pub is_open: fn(&RobcoNativeApp) -> bool,
    pub set_open: fn(&mut RobcoNativeApp, bool),
    pub draw: fn(&mut RobcoNativeApp, &Context),
    pub on_open: Option<fn(&mut RobcoNativeApp, bool)>,
    pub on_closed: Option<fn(&mut RobcoNativeApp)>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DesktopComponentSpec {
    pub window: DesktopWindow,
    pub hosted_app: DesktopHostedApp,
    pub id_salt: &'static str,
    pub default_size: [f32; 2],
    pub show_in_taskbar: bool,
    pub show_in_window_menu: bool,
    title_kind: DesktopTitleKind,
}

const DESKTOP_COMPONENT_BINDINGS: [DesktopComponentBinding; 7] = [ ... ];
```

The 7 components are: FileManager, Editor, Settings, Applications, Installer, TerminalMode, PtyApp.

### Frame loop structure (frame_runtime.rs:286-421)

The desktop branch of `update_native_shell_frame()` does:
```
self.draw_top_bar(ctx);           // desktop_menu_bar.rs
self.draw_desktop_taskbar(ctx);   // desktop_taskbar.rs
self.draw_desktop(ctx);           // desktop_surface.rs (icon grid + wallpaper)
// then overlays:
self.draw_desktop_windows(ctx);   // iterates DESKTOP_COMPONENT_BINDINGS, calls draw
self.draw_start_panel(ctx);       // desktop_start_menu.rs
self.draw_start_menu_rename_window(ctx);
self.draw_spotlight(ctx);         // desktop_spotlight.rs
```

---

# PHASE 0: Prep (no behavioral changes)

## Phase 0a: Create `nucleon-tweaks` app crate

### Goal
Extract appearance settings into a new standalone app. After this, the Settings window's "Appearance" tile opens the new tweaks app instead of navigating to an internal panel.

### Step 1: Create crate `crates/native-tweaks-app`

Create directory `crates/native-tweaks-app/src/`.

**`crates/native-tweaks-app/Cargo.toml`:**
```toml
[package]
name = "robcos-native-tweaks-app"
version = "0.4.4"
edition = "2021"
license = "GPL-3.0-only"

[dependencies]
robcos = { path = "../.." }
eframe = { version = "0.29", default-features = false }
```

Copy the version number from the root `Cargo.toml`.

**`crates/native-tweaks-app/src/lib.rs`:**

This crate re-exports types/functions that the main app needs. For Phase 0a, it is a thin shell — the actual appearance UI code stays in the main crate for now. The crate exists so the binary and standalone launcher can be wired up.

Define:
```rust
pub const TWEAKS_APP_TITLE: &str = "Tweaks";
```

That is ALL the lib.rs needs for Phase 0a. The actual UI extraction happens later.

### Step 2: Add workspace member

In root `Cargo.toml`, add `"crates/native-tweaks-app"` to both `members` and `default-members` arrays.

### Step 3: Add binary entries

In `crates/native-shell/Cargo.toml`, add dependency:
```toml
robcos-native-tweaks-app = { path = "../native-tweaks-app" }
```

Add binary entries after the installer entries:
```toml
[[bin]]
name = "robcos-tweaks"
path = "src/tweaks_main.rs"

[[bin]]
name = "nucleon-tweaks"
path = "src/nucleon_tweaks_main.rs"
```

### Step 4: Create binary entry points

Follow the exact same pattern as the existing standalone apps (e.g. `src/settings_main.rs` / `src/nucleon_settings_main.rs`).

Create `crates/native-shell/src/tweaks_main.rs` and `crates/native-shell/src/nucleon_tweaks_main.rs`. These should follow the same pattern as `settings_main.rs` and `nucleon_settings_main.rs` respectively. Read those files first and replicate the pattern.

### Step 5: Create standalone launcher module

Create `src/native/tweaks_standalone.rs` following the exact pattern of `src/native/settings_standalone.rs`. Read `settings_standalone.rs` first and replicate:
- Define `RobcoNativeTweaksApp` struct
- Implement `eframe::App` for it
- The `update()` method calls `self.inner.update_standalone_tweaks_window(ctx)`
- Export the struct from `src/native/mod.rs`

### Step 6: Add module declaration

In `src/native/mod.rs`, add:
```rust
mod tweaks_standalone;
```
And add the appropriate `pub use` re-export following the pattern of the other standalone apps.

### Step 7: Add `DesktopWindow::Tweaks` variant

In `src/native/desktop_app.rs`:
1. Add `Tweaks` to the `DesktopWindow` enum (in `crates/native-services/src/shared_types.rs` — check where this enum is actually defined, it's re-exported via `pub use super::shared_types::DesktopWindow`)
2. Add `Tweaks` to the `DesktopHostedApp` enum
3. Add a new `DesktopComponentBinding` entry to `DESKTOP_COMPONENT_BINDINGS` (bump array size to 8):
```rust
DesktopComponentBinding {
    spec: DesktopComponentSpec {
        window: DesktopWindow::Tweaks,
        hosted_app: DesktopHostedApp::Tweaks,
        id_salt: "native_tweaks",
        default_size: [820.0, 560.0],
        show_in_taskbar: true,
        show_in_window_menu: true,
        title_kind: DesktopTitleKind::Static("Tweaks"),
    },
    is_open: RobcoNativeApp::desktop_component_tweaks_is_open,
    set_open: RobcoNativeApp::desktop_component_tweaks_set_open,
    draw: RobcoNativeApp::desktop_component_tweaks_draw,
    on_open: None,
    on_closed: None,
}
```

### Step 8: Add tweaks state to RobcoNativeApp

In `src/native/app.rs`, add to `RobcoNativeApp` struct:
```rust
tweaks_open: bool,
```

Add the component bridge methods that `DesktopComponentBinding` references. These follow the exact same pattern as the existing `desktop_component_settings_*` methods. Find where those are defined and replicate for tweaks.

### Step 9: Wire Settings -> Tweaks navigation

In the Settings home tile grid (`draw_settings()` in `desktop_window_presenters.rs`, around line 558-596), when the user clicks the "Appearance" tile:
- Instead of setting `next_panel = Some(NativeSettingsPanel::Appearance)`, open the Tweaks window.
- Call the equivalent of `self.desktop_component_tweaks_set_open(true)` and close settings or keep it open — match UX preference.

Specifically, in the `SettingsHomeTileAction::OpenPanel(panel)` match arm, add a check:
```rust
if panel == NativeSettingsPanel::Appearance {
    // Open tweaks window instead
    self.open_tweaks_from_settings();
} else {
    next_panel = Some(panel);
}
```

Define `open_tweaks_from_settings()` to:
1. Set `self.tweaks_open = true`
2. Call `self.prime_desktop_window_defaults(DesktopWindow::Tweaks)`
3. Set `self.desktop_active_window = Some(WindowInstanceId::primary(DesktopWindow::Tweaks))`

### Step 10: Create `draw_tweaks()` presenter

Create `src/native/app/tweaks_presenter.rs` (new file). Add module declaration in `src/native/app.rs` (look at how other sub-modules are declared — they use `mod module_name;` inside the `app.rs` file or in a `mod.rs`; check the actual pattern).

This file contains `impl RobcoNativeApp` with:
```rust
pub(super) fn draw_tweaks(&mut self, ctx: &Context) { ... }
```

For Phase 0a, `draw_tweaks()` should render the exact same Appearance panel UI that currently lives inside `draw_settings()`. This means:
- Copy the `NativeSettingsPanel::Appearance` match arm from `desktop_window_presenters.rs` (starts at approximately line 646)
- Wrap it in an `egui::Window` with the same window management pattern as `draw_settings()` (header, open/close, resize tracking)
- Use default size `[820.0, 560.0]`

The appearance tab content includes 5 tabs: Background (tab 0), Display (tab 1), Colors (tab 2), Icons (tab 3), Terminal (tab 4). All of this content moves into `draw_tweaks()`.

The `draw_settings_display_effects_panel()` function from `settings_panels.rs` is called from within the Display tab — it should now be callable from the tweaks presenter too.

### Step 11: Remove Appearance from Settings

In `desktop_window_presenters.rs`, remove the `NativeSettingsPanel::Appearance` match arm from `draw_settings()`. The Appearance tile stays in the Settings home grid but now opens Tweaks instead of navigating internally.

### Step 12: Add standalone prepare/update methods

In `src/native/app/desktop_runtime.rs`, add:
```rust
pub(crate) fn prepare_standalone_tweaks_window(&mut self, session_username: Option<String>) {
    self.prepare_standalone_window_shell(session_username, true);
    self.prime_desktop_window_defaults(DesktopWindow::Tweaks);
    self.tweaks_open = true;
    self.desktop_active_window = Some(WindowInstanceId::primary(DesktopWindow::Tweaks));
}

pub(crate) fn update_standalone_tweaks_window(&mut self, ctx: &Context) {
    self.process_background_results(ctx);
    self.maybe_sync_settings_from_disk(ctx);
    self.sync_native_appearance(ctx);
    self.sync_native_display_effects();
    self.draw_tweaks(ctx);
    if !self.tweaks_open {
        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
    }
    ctx.request_repaint_after(Duration::from_millis(500));
}
```

### Verification
- `cargo check -p robcos`
- `cargo check -p robcos-native-shell`
- Run the app, open Settings, click Appearance — should open Tweaks window
- All other settings panels still work

---

## Phase 0b: Split window presenters into per-window modules

### Goal
Break `desktop_window_presenters.rs` into separate files per window type. Pure file moves, no logic changes.

### Step 1: Create new files

Create these files under `src/native/app/`:

**`presenter_file_manager.rs`** — contains `draw_file_manager()` method (currently lines 38-191 of `desktop_window_presenters.rs`)

**`presenter_editor.rs`** — contains `draw_editor()` method (currently lines 192-502)

**`presenter_settings.rs`** — contains `draw_settings()` method (currently lines 503-end, minus the Appearance arm which was removed in Phase 0a)

### Step 2: Move code

For each new file:
1. Copy the relevant `impl RobcoNativeApp` block containing the function
2. Copy the `use` imports that the moved function needs (trace each one — only copy what's actually used)
3. Keep the `use super::RobcoNativeApp;` pattern (since these are sub-modules of `app`)
4. Keep visibility as `pub(super)`

The import style in these files follows the existing convention: `use super::super::` to reach `src/native/` siblings.

### Step 3: Update desktop_window_presenters.rs

After moving, `desktop_window_presenters.rs` should either:
- Be deleted entirely (if all functions have been moved)
- Or contain only shared helper functions used by multiple presenters

If there are shared helpers (like `desktop_window_frame()`, `desktop_default_window_size()`, etc.), check if they're defined in this file or elsewhere. They're likely on `impl RobcoNativeApp` in `desktop_window_mgmt.rs` — verify before assuming.

### Step 4: Add module declarations

In the file that declares sub-modules of `app` (check how existing files like `desktop_runtime.rs` are declared — look at `src/native/app.rs` for `mod` statements), add:
```rust
mod presenter_file_manager;
mod presenter_editor;
mod presenter_settings;
```

Remove `mod desktop_window_presenters;` if it existed (check if it's declared in `app.rs` or if `desktop_window_presenters.rs` is used differently).

Actually — IMPORTANT: Check how `desktop_window_presenters.rs` is currently included. Look for `mod desktop_window_presenters` in `src/native/app.rs`. All files in `src/native/app/` are declared as modules there. Replace:
```rust
mod desktop_window_presenters;
```
with:
```rust
mod presenter_file_manager;
mod presenter_editor;
mod presenter_settings;
```

### Step 5: Verify nothing changed

Every call site that invoked `self.draw_file_manager(ctx)`, `self.draw_editor(ctx)`, `self.draw_settings(ctx)` should still work — they call methods on `RobcoNativeApp`, not functions in a specific module. The methods are found by Rust's impl resolution regardless of which file they're in.

### Verification
- `cargo check -p robcos`
- `cargo check -p robcos-native-shell`
- Run the app, open each window type — identical behavior

---

## Phase 0c: Introduce ManagedWindow trait

### Goal
Create an abstraction layer for windows that doesn't assume all windows are nucleon apps. Currently, windows are tracked as `HashMap<WindowInstanceId, DesktopWindowState>`. We introduce a `ManagedWindow` trait that wraps this.

### Step 1: Find DesktopWindowState

First, locate the `DesktopWindowState` struct definition. Search for `struct DesktopWindowState` in the codebase. It's likely in `desktop_window_mgmt.rs` or `desktop_app.rs`. Read it fully.

### Step 2: Define ManagedWindow types

In `src/native/desktop_app.rs` (or a new file `src/native/managed_window.rs` if cleaner), add:

```rust
/// Source of a managed window — nucleon-native app or (future) external window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowSource {
    /// A built-in nucleon app window.
    NucleonApp(DesktopHostedApp),
    // Future: External(ExternalWindowId) for X11/Wayland windows
}

/// A window managed by the shell. Wraps the existing DesktopWindowState
/// with source metadata for future WM compatibility.
pub struct ManagedWindow {
    pub id: WindowInstanceId,
    pub source: WindowSource,
    pub title: String,
    pub state: DesktopWindowState,
}
```

### Step 3: Do NOT refactor the HashMap yet

For Phase 0c, we only define the types. We do NOT change `desktop_window_states: HashMap<WindowInstanceId, DesktopWindowState>` to use `ManagedWindow`. That refactor happens in Phase 1 when the slot system needs it.

The point of Phase 0c is to have the types ready and reviewed. The actual migration is Phase 1.

### Step 4: Add WindowSource derivation

Add a helper to derive `WindowSource` from existing data:
```rust
impl WindowSource {
    pub fn from_desktop_window(window: DesktopWindow) -> Self {
        let hosted = hosted_app_for_window_kind(window);
        WindowSource::NucleonApp(hosted)
    }
}
```

Use the existing `hosted_app_for_window()` or equivalent to map `DesktopWindow` -> `DesktopHostedApp`.

### Verification
- `cargo check -p robcos`
- No behavioral changes

---

# PHASE 1: Core data models and slot system

## Phase 1a: Define theme data models

### Goal
Define the data structures for the theme system. No wiring yet.

### Location
Create `crates/shared/src/theme.rs` and add `pub mod theme;` to `crates/shared/src/lib.rs`.

### Types to define

```rust
use serde::{Deserialize, Serialize};

// ── Color System ──────────────────────────────────────────────

/// Preset monochrome colors — matches existing palette enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MonochromePreset {
    Green,
    White,
    Amber,
    Blue,
    LightBlue,
    Custom,
}

/// Color mode for the shell.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ColorStyle {
    /// Single-hue CRT monochrome. All rendered content is tinted.
    Monochrome {
        preset: MonochromePreset,
        /// Only used when preset == Custom
        custom_rgb: Option<[u8; 3]>,
    },
    /// Multi-color UI with semantic design tokens.
    FullColor {
        /// ID of the color theme (e.g. "nucleon-dark", "xp-blue")
        theme_id: String,
    },
}

/// A named color token for FullColor themes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ColorToken {
    BgPrimary,
    BgSecondary,
    FgPrimary,
    FgSecondary,
    FgDim,
    Accent,
    AccentHover,
    AccentActive,
    PanelBg,
    PanelBorder,
    WindowChrome,
    WindowChromeFocused,
    Selection,
    SelectionFg,
    Border,
    Separator,
    StatusBar,
    Error,
    Warning,
    Success,
}

/// A full-color theme definition: maps tokens to colors.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FullColorTheme {
    pub id: String,
    pub name: String,
    pub tokens: std::collections::HashMap<ColorToken, [u8; 4]>, // RGBA
}

// ── Shell Style ────────────────────────────────────────────────

/// Visual identity of the shell chrome.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShellStyle {
    pub id: String,
    pub name: String,
    /// Border radius for windows and panels (0.0 = sharp corners)
    pub border_radius: f32,
    /// Window title bar height
    pub title_bar_height: f32,
    /// Separator thickness
    pub separator_thickness: f32,
    /// Whether to show window drop shadows
    pub window_shadow: bool,
}

// ── Layout Profile ─────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PanelPosition {
    Top,
    Bottom,
    Hidden,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DockPosition {
    Bottom,
    Left,
    Right,
    Hidden,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LauncherStyle {
    StartMenu,
    Overlay,
    Hidden,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WindowHeaderStyle {
    Standard,
    Compact,
    Hidden,
}

/// Structural layout rules.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutProfile {
    pub id: String,
    pub name: String,
    pub panel_position: PanelPosition,
    pub panel_height: f32,
    pub dock_position: DockPosition,
    pub dock_size: f32,
    pub launcher_style: LauncherStyle,
    pub window_header_style: WindowHeaderStyle,
}

// ── Asset Pack ─────────────────────────────────────────────────

/// Reference to an asset pack (icons, cursors, wallpapers).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetPackRef {
    pub id: String,
    pub name: String,
    /// Path relative to the theme pack root or addon directory
    pub path: String,
}

// ── Theme Pack (top-level) ─────────────────────────────────────

/// A complete theme pack combining all visual/structural configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemePack {
    pub id: String,
    pub name: String,
    pub description: String,
    pub version: String,
    pub shell_style: ShellStyle,
    pub layout_profile: LayoutProfile,
    pub color_style: ColorStyle,
    pub asset_pack: Option<AssetPackRef>,
}
```

### Classic theme constructor

Add a function that returns the Classic theme matching the current hardcoded values:

```rust
impl ThemePack {
    /// The built-in "Classic" theme matching the pre-refactor UI.
    pub fn classic() -> Self {
        ThemePack {
            id: "classic".to_string(),
            name: "Classic".to_string(),
            description: "The original Nucleon terminal aesthetic".to_string(),
            version: "1.0.0".to_string(),
            shell_style: ShellStyle {
                id: "classic".to_string(),
                name: "Classic".to_string(),
                border_radius: 0.0,       // egui::Rounding::ZERO
                title_bar_height: 28.0,
                separator_thickness: 2.0,
                window_shadow: false,      // Shadow::NONE
            },
            layout_profile: LayoutProfile {
                id: "classic".to_string(),
                name: "Classic".to_string(),
                panel_position: PanelPosition::Top,
                panel_height: 32.0,  // measure from draw_top_bar
                dock_position: DockPosition::Bottom,
                dock_size: 32.0,     // measure from draw_desktop_taskbar
                launcher_style: LauncherStyle::StartMenu,
                window_header_style: WindowHeaderStyle::Standard,
            },
            color_style: ColorStyle::Monochrome {
                preset: MonochromePreset::Green,
                custom_rgb: None,
            },
            asset_pack: None, // uses built-in assets/
        }
    }
}
```

NOTE: The exact `panel_height` and `dock_size` values must be verified by reading `draw_top_bar()` in `desktop_menu_bar.rs` and `draw_desktop_taskbar()` in `desktop_taskbar.rs`. Look for `TopBottomPanel::top(...).exact_height(...)` or similar sizing calls. Use whatever values are hardcoded there.

### Verification
- `cargo check -p robcos-shared`
- No behavioral changes (data models only)

---

## Phase 1b: Build the slot registry

### Goal
Replace direct method calls in the frame loop with a dynamic slot registry that dispatches to renderers.

### Important constraint
The Classic slot renderers must call the EXACT same code that currently exists. We are wrapping, not rewriting.

### Step 1: Define ShellSlot and SlotRenderer

Create `src/native/shell_slots.rs`:

```rust
use eframe::egui::Context;

/// Named locations in the shell where components render.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ShellSlot {
    Panel,       // top menu bar
    Dock,        // bottom taskbar
    Launcher,    // start menu
    Spotlight,   // search overlay
    Desktop,     // desktop icon surface
}

/// Contextual data passed to slot renderers.
/// Start minimal — expand as needed.
pub struct SlotContext<'a> {
    pub ctx: &'a Context,
}

/// Actions that a slot renderer can emit.
pub enum SlotAction {
    // Placeholder — will be populated as needed
    None,
}

/// Trait for rendering into a shell slot.
pub trait SlotRenderer {
    fn slot(&self) -> ShellSlot;
    fn render(&self, app: &mut super::app::RobcoNativeApp, slot_ctx: &SlotContext) -> Vec<SlotAction>;
}
```

NOTE: The `app: &mut RobcoNativeApp` parameter is intentionally kept for Phase 1. The ShellState boundary (Phase 1c) will replace this with a constrained view. Don't try to do both at once.

### Step 2: Create Classic renderers (thin wrappers)

Create `src/native/shell_slots/classic_panel.rs`:
```rust
use super::{ShellSlot, SlotAction, SlotContext, SlotRenderer};
use crate::native::app::RobcoNativeApp;

pub struct ClassicPanelRenderer;

impl SlotRenderer for ClassicPanelRenderer {
    fn slot(&self) -> ShellSlot { ShellSlot::Panel }

    fn render(&self, app: &mut RobcoNativeApp, slot_ctx: &SlotContext) -> Vec<SlotAction> {
        app.draw_top_bar(slot_ctx.ctx);
        vec![]
    }
}
```

Create the same pattern for:
- `classic_dock.rs` — calls `app.draw_desktop_taskbar(slot_ctx.ctx)`
- `classic_launcher.rs` — calls `app.draw_start_panel(slot_ctx.ctx)`
- `classic_spotlight.rs` — calls `app.draw_spotlight(slot_ctx.ctx)`
- `classic_desktop.rs` — calls `app.draw_desktop(slot_ctx.ctx)`

### Step 3: Create SlotRegistry

In `src/native/shell_slots.rs`, add:

```rust
pub struct SlotRegistry {
    renderers: Vec<Box<dyn SlotRenderer>>,
}

impl SlotRegistry {
    pub fn classic() -> Self {
        SlotRegistry {
            renderers: vec![
                Box::new(classic_panel::ClassicPanelRenderer),
                Box::new(classic_dock::ClassicDockRenderer),
                Box::new(classic_launcher::ClassicLauncherRenderer),
                Box::new(classic_spotlight::ClassicSpotlightRenderer),
                Box::new(classic_desktop::ClassicDesktopRenderer),
            ],
        }
    }

    pub fn render_slot(&self, slot: ShellSlot, app: &mut RobcoNativeApp, ctx: &Context) -> Vec<SlotAction> {
        let slot_ctx = SlotContext { ctx };
        let mut actions = Vec::new();
        for renderer in &self.renderers {
            if renderer.slot() == slot {
                actions.extend(renderer.render(app, &slot_ctx));
            }
        }
        actions
    }
}
```

### Step 4: Wire into frame loop

In `src/native/app.rs`, add to `RobcoNativeApp`:
```rust
slot_registry: super::shell_slots::SlotRegistry,
```

In the `Default` impl, initialize:
```rust
slot_registry: super::shell_slots::SlotRegistry::classic(),
```

### Step 5: Update frame_runtime.rs

In `update_native_shell_frame()` (line ~386-402), replace the direct calls:

**Before:**
```rust
if self.desktop_mode_open {
    // ... escape/tiling/keyboard handling stays ...
    self.draw_top_bar(ctx);
    self.draw_desktop_taskbar(ctx);
    self.draw_desktop(ctx);
} else {
    self.draw_terminal_runtime(ctx);
}
if self.desktop_mode_open {
    self.draw_desktop_windows(ctx);
    self.draw_start_panel(ctx);
    self.draw_start_menu_rename_window(ctx);
    self.draw_spotlight(ctx);
}
```

**After:**
```rust
if self.desktop_mode_open {
    // ... escape/tiling/keyboard handling stays UNCHANGED ...
    // NOTE: We need to temporarily take ownership of the registry
    // because render_slot needs &mut self. Use a take/put pattern:
    let registry = std::mem::replace(
        &mut self.slot_registry,
        super::shell_slots::SlotRegistry::classic(), // placeholder
    );
    registry.render_slot(ShellSlot::Panel, self, ctx);
    registry.render_slot(ShellSlot::Dock, self, ctx);
    registry.render_slot(ShellSlot::Desktop, self, ctx);
    self.slot_registry = registry;
} else {
    self.draw_terminal_runtime(ctx);
}
if self.desktop_mode_open {
    self.draw_desktop_windows(ctx);
    let registry = std::mem::replace(
        &mut self.slot_registry,
        super::shell_slots::SlotRegistry::classic(),
    );
    registry.render_slot(ShellSlot::Launcher, self, ctx);
    self.slot_registry = registry;
    self.draw_start_menu_rename_window(ctx);
    let registry = std::mem::replace(
        &mut self.slot_registry,
        super::shell_slots::SlotRegistry::classic(),
    );
    registry.render_slot(ShellSlot::Spotlight, self, ctx);
    self.slot_registry = registry;
}
```

NOTE: The `std::mem::replace` pattern is ugly but necessary because `SlotRenderer::render` takes `&mut RobcoNativeApp` while `slot_registry` is a field of `RobcoNativeApp`. An alternative is to use `Option<SlotRegistry>` and `.take()` / re-assign. Choose whichever compiles cleanly. The CRITICAL thing is that the same functions get called in the same order.

### Step 6: Module declarations

In `src/native/mod.rs`, add:
```rust
mod shell_slots;
```

Make `shell_slots` a directory module: `src/native/shell_slots/mod.rs` with:
```rust
mod classic_panel;
mod classic_dock;
mod classic_launcher;
mod classic_spotlight;
mod classic_desktop;

// ... all the type definitions from Step 1 ...
// ... the SlotRegistry from Step 3 ...
```

### Verification
- `cargo check -p robcos`
- Run the app — desktop mode must look and behave identically
- The slot renderers are just wrappers around the same methods

---

## Phase 1c: ShellState boundary

### Goal
Define a constrained state view that slot renderers receive instead of `&mut RobcoNativeApp`.

### THIS PHASE IS DEFERRED
Phase 1c is intentionally left as a design placeholder. The `&mut RobcoNativeApp` approach from Phase 1b works for v1. Phase 1c should only be implemented when we actually have third-party slot renderers that need sandboxing. Do not implement this phase yet.

---

# PHASE 2: Theme engine and color system

## Phase 2a: Refactor retro_ui.rs to use ColorStyle

### Goal
Make `retro_ui.rs` palette generation driven by `ColorStyle` from the theme data model.

### Step 1: Map existing presets to MonochromePreset

The existing theme system uses `ratatui::style::Color` values. The mapping is:
- Green -> `Color::Rgb(111, 255, 84)` (from `color32_from_theme`)
- White -> `Color::Rgb(240, 240, 240)`
- Amber -> `Color::Rgb(255, 191, 74)` (the Yellow mapping)
- Blue -> `Color::Rgb(105, 180, 255)`
- LightBlue -> `Color::Rgb(110, 235, 255)` (the Cyan mapping)
- Custom -> `Color::Rgb(r, g, b)`

Find where these theme names/colors are defined in `crates/shared/src/config.rs` (look for `THEMES`, `CUSTOM_THEME_NAME`, `current_theme_color`, `theme_color_for_settings`). Those are the connection points.

### Step 2: Add ColorStyle -> RetroPalette conversion

In `src/native/retro_ui.rs`, add:
```rust
use crate::theme::ColorStyle;

pub fn palette_for_color_style(style: &ColorStyle) -> RetroPalette {
    match style {
        ColorStyle::Monochrome { preset, custom_rgb } => {
            let color = monochrome_preset_to_color(*preset, *custom_rgb);
            palette_for_theme_color(color)
        }
        ColorStyle::FullColor { theme_id } => {
            // Phase 2 stub — load FullColorTheme by ID and convert
            // For now, fall back to green monochrome
            palette_for_theme_color(Color::Rgb(111, 255, 84))
        }
    }
}

fn monochrome_preset_to_color(preset: MonochromePreset, custom_rgb: Option<[u8; 3]>) -> Color {
    match preset {
        MonochromePreset::Green => Color::Rgb(111, 255, 84),
        MonochromePreset::White => Color::Rgb(240, 240, 240),
        MonochromePreset::Amber => Color::Rgb(255, 191, 74),
        MonochromePreset::Blue => Color::Rgb(105, 180, 255),
        MonochromePreset::LightBlue => Color::Rgb(110, 235, 255),
        MonochromePreset::Custom => {
            let [r, g, b] = custom_rgb.unwrap_or([111, 255, 84]);
            Color::Rgb(r, g, b)
        }
    }
}
```

NOTE: Verify the exact RGB values by reading `color32_from_theme()` in `retro_ui.rs` and the theme name-to-color mapping in `config.rs`. The values above are from the current code but MUST be double-checked.

### Step 3: Do NOT remove the old path yet

Keep `current_palette()` and `palette_for_theme_color()` working as-is. The new `palette_for_color_style()` is an alternative entry point. The switch to using it exclusively happens when Settings/Tweaks is updated to store `ColorStyle` instead of the old theme name string.

### Verification
- `cargo check -p robcos`
- No behavioral changes (new function exists but isn't wired in yet)

---

## Phase 2b: Monochrome tinting pipeline

### Goal
Make monochrome mode tint ALL rendered content (images, icons, app content).

### Approach
The CRT shader in `vendor/egui-wgpu` already post-processes the entire frame. Monochrome tinting should be applied as a shader pass that desaturates the frame and applies the hue tint.

### Step 1: Understand the current CRT shader

Read `vendor/egui-wgpu/src/` and find the CRT effect shader. It's likely a WGSL or GLSL shader applied as a post-processing pass. The shader already receives parameters like curvature, scanlines, glow, etc.

### Step 2: Add monochrome uniform

Add a uniform to the CRT shader:
```
monochrome_enabled: u32,   // 0 or 1
monochrome_tint: vec3<f32>, // RGB tint color, normalized 0-1
```

### Step 3: Add shader logic

After all other CRT effects, if `monochrome_enabled == 1`:
```wgsl
// Convert to luminance
let lum = dot(color.rgb, vec3<f32>(0.299, 0.587, 0.114));
// Apply tint
color = vec4<f32>(monochrome_tint * lum, color.a);
```

This desaturates the entire frame to grayscale, then tints it with the monochrome color. Every pixel — including images, icons, colored text, everything — goes through this. The result looks like a genuine single-color CRT monitor.

### Step 4: Wire the uniform

Find where CRT effect parameters are set from Rust (look for where `sync_native_display_effects()` sends values to the shader). Add `monochrome_enabled` and `monochrome_tint` to the same pipeline.

When `ColorStyle::Monochrome` is active, set `monochrome_enabled = 1` and `monochrome_tint` to the RGB color from the preset. When `ColorStyle::FullColor` is active, set `monochrome_enabled = 0`.

### Verification
- `cargo check -p robcos`
- Run app in monochrome mode — everything should be tinted including any loaded images/icons
- Switching colors should change the tint

---

## Phase 2c: ThemePack loading via .ndpkg

### Goal
Theme packs can be discovered, installed, and loaded using the existing addon pipeline.

### Step 1: Extend addon manifest

In `crates/shared/src/platform/` (find the addon manifest types), add a new addon kind:
```rust
pub enum AddonKind {
    // ... existing variants ...
    Theme,
}
```

Or if there's an existing field for addon type/category, add "theme" as a valid value.

### Step 2: Theme pack manifest format

A theme `.ndpkg` contains:
```
manifest.json          # standard addon manifest with kind: "theme"
theme.json             # ThemePack serialized as JSON
shell_style.json       # optional override
layout_profile.json    # optional override
color_theme.json       # optional FullColorTheme (for FullColor themes)
assets/                # optional AssetPack directory
  icons/
  cursors/
  wallpapers/
```

### Step 3: Theme discovery

Add a function in the platform layer to discover installed themes:
```rust
pub fn installed_theme_packs() -> Vec<ThemePack> { ... }
```

This uses the existing addon discovery infrastructure. Filter addons by `AddonKind::Theme`, read `theme.json` from each, deserialize.

### Step 4: Theme application

Add a function to apply a `ThemePack`:
```rust
pub fn apply_theme_pack(theme: &ThemePack) { ... }
```

This:
1. Updates the active `ColorStyle`
2. Updates the active `ShellStyle`
3. Updates the active `LayoutProfile`
4. If the theme has an asset pack, updates the active asset paths
5. Persists the selection to settings

### Verification
- `cargo check -p robcos`
- The Classic theme (built-in) works as default
- Theme pack installation through existing installer UI

---

# PHASE 3: Layout profiles (data-driven layout)

## Overview

Make the shell layout driven by `LayoutProfile` data instead of hardcoded values.
Use Approach A: parameterize the existing egui panel calls. Do NOT replace egui's panel
system with manual rect computation. The architecture allows switching to a full rect
interpreter later without changing the data model.

After this phase:
- Classic layout profile produces pixel-identical output to pre-refactor
- A built-in "Minimal" test layout proves the system works (panel at bottom, no dock)
- The tweaks app has a layout picker to switch between them at runtime

### Current hardcoded values (verified from source)

| Slot | egui call | Size | File:Line |
|------|-----------|------|-----------|
| Panel (top bar) | `TopBottomPanel::top("native_top_bar").exact_height(30.0)` | 30px | `desktop_menu_bar.rs:266-267` |
| Dock (taskbar) | `TopBottomPanel::bottom("native_desktop_taskbar").exact_height(32.0)` | 32px | `desktop_taskbar.rs:49-50` |
| Desktop | `egui::CentralPanel::default()` | fills rest | `desktop_surface.rs:1474` |

### Current slot bridge methods (app.rs:556-574)

```rust
impl RobcoNativeApp {
    pub(super) fn render_classic_panel_slot(&mut self, ctx: &Context) {
        self.draw_top_bar(ctx);
    }
    pub(super) fn render_classic_dock_slot(&mut self, ctx: &Context) {
        self.draw_desktop_taskbar(ctx);
    }
    pub(super) fn render_classic_launcher_slot(&mut self, ctx: &Context) {
        self.draw_start_panel(ctx);
    }
    pub(super) fn render_classic_spotlight_slot(&mut self, ctx: &Context) {
        self.draw_spotlight(ctx);
    }
    pub(super) fn render_classic_desktop_slot(&mut self, ctx: &Context) {
        self.draw_desktop(ctx);
    }
}
```

---

## Step 1: Add active_layout to RobcoNativeApp

In `src/native/app.rs`, add a field next to `slot_registry`:

```rust
slot_registry: super::shell_slots::SlotRegistry,
active_layout: LayoutProfile,
```

Import `LayoutProfile` from `crate::theme::LayoutProfile`.

In the `Default` impl, initialize:
```rust
active_layout: crate::theme::ThemePack::classic().layout_profile,
```

Also add it to `ParkedSessionState` so layout persists across session switches. Follow the
same pattern as `slot_registry` — if `slot_registry` is NOT in `ParkedSessionState`, then
`active_layout` should not be either (layout is global, not per-session). Check and match.

---

## Step 2: Add LayoutProfile to SlotContext

In `src/native/shell_slots/mod.rs`, update `SlotContext`:

```rust
use crate::theme::LayoutProfile;

pub struct SlotContext<'a> {
    pub ctx: &'a Context,
    pub layout: &'a LayoutProfile,
}
```

Update `SlotRegistry::render_slot()` to accept and pass the layout:

```rust
pub fn render_slot(
    &self,
    slot: ShellSlot,
    app: &mut RobcoNativeApp,
    ctx: &Context,
    layout: &LayoutProfile,
) -> Vec<SlotAction> {
    let slot_ctx = SlotContext { ctx, layout };
    let mut actions = Vec::new();
    for renderer in &self.renderers {
        if renderer.slot() == slot {
            actions.extend(renderer.render(app, &slot_ctx));
        }
    }
    actions
}
```

---

## Step 3: Update frame_runtime.rs to pass layout

In `update_native_shell_frame()`, the slot dispatch section (around lines 406-429).

The `active_layout` field must be read BEFORE the `std::mem::replace` on `slot_registry`,
because after the replace we lose access to `self`. Clone it before the swap:

```rust
if self.desktop_mode_open {
    // ... escape/tiling/keyboard handling stays UNCHANGED ...
    let layout = self.active_layout.clone();
    let registry = std::mem::replace(
        &mut self.slot_registry,
        super::super::shell_slots::SlotRegistry::classic(),
    );
    registry.render_slot(ShellSlot::Panel, self, ctx, &layout);
    registry.render_slot(ShellSlot::Dock, self, ctx, &layout);
    registry.render_slot(ShellSlot::Desktop, self, ctx, &layout);
    self.slot_registry = registry;
} else {
    self.draw_terminal_runtime(ctx);
}
if self.desktop_mode_open {
    self.draw_desktop_windows(ctx);
    let layout = self.active_layout.clone();
    let registry = std::mem::replace(
        &mut self.slot_registry,
        super::super::shell_slots::SlotRegistry::classic(),
    );
    registry.render_slot(ShellSlot::Launcher, self, ctx, &layout);
    self.slot_registry = registry;
    self.draw_start_menu_rename_window(ctx);
    let layout = self.active_layout.clone();
    let registry = std::mem::replace(
        &mut self.slot_registry,
        super::super::shell_slots::SlotRegistry::classic(),
    );
    registry.render_slot(ShellSlot::Spotlight, self, ctx, &layout);
    self.slot_registry = registry;
}
```

NOTE: `LayoutProfile` must derive `Clone`. It already does (check `theme.rs` — yes it has
`#[derive(Debug, Clone, Serialize, Deserialize)]`). Good.

NOTE: To avoid cloning 3 times per frame, an alternative is to store `active_layout` as
`Arc<LayoutProfile>` or extract it before the entire desktop block. Use whichever is simpler.
Clone is fine for a small struct — don't over-optimize.

---

## Step 4: Update classic renderers to read layout

Update each classic renderer to respect layout parameters. The renderers call bridge methods
on `RobcoNativeApp`. The bridge methods need to accept layout parameters.

### 4a: Panel renderer

In `src/native/shell_slots/classic_panel.rs`:
```rust
fn render(&self, app: &mut RobcoNativeApp, slot_ctx: &SlotContext) -> Vec<SlotAction> {
    app.render_classic_panel_slot(slot_ctx.ctx, &slot_ctx.layout);
    vec![]
}
```

### 4b: Dock renderer

In `src/native/shell_slots/classic_dock.rs`:
```rust
fn render(&self, app: &mut RobcoNativeApp, slot_ctx: &SlotContext) -> Vec<SlotAction> {
    app.render_classic_dock_slot(slot_ctx.ctx, &slot_ctx.layout);
    vec![]
}
```

### 4c: Launcher renderer

In `src/native/shell_slots/classic_launcher.rs`:
```rust
fn render(&self, app: &mut RobcoNativeApp, slot_ctx: &SlotContext) -> Vec<SlotAction> {
    if slot_ctx.layout.launcher_style == LauncherStyle::Hidden {
        return vec![];
    }
    app.render_classic_launcher_slot(slot_ctx.ctx);
    vec![]
}
```

Import `LauncherStyle` from `crate::theme`.

### 4d: Spotlight and Desktop renderers

These are unaffected by layout profile. Keep them as-is:
```rust
fn render(&self, app: &mut RobcoNativeApp, slot_ctx: &SlotContext) -> Vec<SlotAction> {
    app.render_classic_spotlight_slot(slot_ctx.ctx);
    vec![]
}
```
Desktop surface always uses `CentralPanel` which fills remaining space — no layout changes needed.

---

## Step 5: Update bridge methods to accept layout and parameterize panel calls

In `src/native/app.rs`, update the bridge methods:

```rust
impl RobcoNativeApp {
    pub(super) fn render_classic_panel_slot(&mut self, ctx: &Context, layout: &LayoutProfile) {
        match layout.panel_position {
            PanelPosition::Hidden => {}
            _ => self.draw_top_bar(ctx, layout.panel_position, layout.panel_height),
        }
    }

    pub(super) fn render_classic_dock_slot(&mut self, ctx: &Context, layout: &LayoutProfile) {
        match layout.dock_position {
            DockPosition::Hidden => {}
            _ => self.draw_desktop_taskbar(ctx, layout.dock_position, layout.dock_size),
        }
    }

    // Launcher, Spotlight, Desktop — unchanged signatures
    pub(super) fn render_classic_launcher_slot(&mut self, ctx: &Context) {
        self.draw_start_panel(ctx);
    }
    pub(super) fn render_classic_spotlight_slot(&mut self, ctx: &Context) {
        self.draw_spotlight(ctx);
    }
    pub(super) fn render_classic_desktop_slot(&mut self, ctx: &Context) {
        self.draw_desktop(ctx);
    }
}
```

Import `LayoutProfile`, `PanelPosition`, `DockPosition` from `crate::theme`.

---

## Step 6: Parameterize draw_top_bar

In `src/native/app/desktop_menu_bar.rs`, change the signature of `draw_top_bar`:

**Before (line 259):**
```rust
pub(super) fn draw_top_bar(&mut self, ctx: &Context) {
```

**After:**
```rust
pub(super) fn draw_top_bar(
    &mut self,
    ctx: &Context,
    position: crate::theme::PanelPosition,
    height: f32,
) {
```

Then change the panel creation (lines 266-268):

**Before:**
```rust
TopBottomPanel::top("native_top_bar")
    .exact_height(30.0)
    .show_separator_line(false)
```

**After:**
```rust
use crate::theme::PanelPosition;

let panel = match position {
    PanelPosition::Top => TopBottomPanel::top("native_top_bar"),
    PanelPosition::Bottom => TopBottomPanel::bottom("native_top_bar"),
    PanelPosition::Hidden => return, // should not reach here, but guard
};
panel
    .exact_height(height)
    .show_separator_line(false)
```

The rest of the `draw_top_bar` body is UNCHANGED. Only the panel construction changes.

---

## Step 7: Parameterize draw_desktop_taskbar

In `src/native/app/desktop_taskbar.rs`, change the signature of `draw_desktop_taskbar`:

**Before (line 47):**
```rust
pub(super) fn draw_desktop_taskbar(&mut self, ctx: &Context) {
```

**After:**
```rust
pub(super) fn draw_desktop_taskbar(
    &mut self,
    ctx: &Context,
    position: crate::theme::DockPosition,
    size: f32,
) {
```

Then change the panel creation (lines 49-51):

**Before:**
```rust
TopBottomPanel::bottom("native_desktop_taskbar")
    .exact_height(32.0)
    .show_separator_line(false)
```

**After:**
```rust
use crate::theme::DockPosition;

let panel = match position {
    DockPosition::Bottom => TopBottomPanel::bottom("native_desktop_taskbar"),
    DockPosition::Top => TopBottomPanel::top("native_desktop_taskbar"),
    // Left/Right require SidePanel — DEFER for now, fall back to bottom
    DockPosition::Left | DockPosition::Right => {
        TopBottomPanel::bottom("native_desktop_taskbar")
    }
    DockPosition::Hidden => return, // guard
};
panel
    .exact_height(size)
    .show_separator_line(false)
```

NOTE: `DockPosition` enum does NOT have a `Top` variant (check `theme.rs` — it has Bottom,
Left, Right, Hidden). If you need Top for the test theme, add it. Actually — re-read the
test theme: Minimal has `dock_position: Hidden`. So we don't need `DockPosition::Top` for
the test theme. Leave the enum as-is. The match arm for Left/Right falls back to Bottom.

CORRECTION: Remove the `DockPosition::Top` match arm since the enum doesn't have it:
```rust
let panel = match position {
    DockPosition::Bottom => TopBottomPanel::bottom("native_desktop_taskbar"),
    // Left/Right require SidePanel — DEFER, fall back to bottom
    DockPosition::Left | DockPosition::Right => {
        TopBottomPanel::bottom("native_desktop_taskbar")
    }
    DockPosition::Hidden => return,
};
```

The rest of the `draw_desktop_taskbar` body is UNCHANGED. Only the panel construction changes.

---

## Step 8: Add "Minimal" test layout

In `crates/shared/src/theme.rs`, add a constructor for the Minimal layout:

```rust
impl LayoutProfile {
    /// Built-in "Minimal" layout for testing: panel at bottom, no dock.
    pub fn minimal() -> Self {
        LayoutProfile {
            id: "minimal".to_string(),
            name: "Minimal".to_string(),
            panel_position: PanelPosition::Bottom,
            panel_height: 26.0,
            dock_position: DockPosition::Hidden,
            dock_size: 32.0,
            launcher_style: LauncherStyle::StartMenu,
            window_header_style: WindowHeaderStyle::Standard,
        }
    }
}
```

Also add a `classic()` constructor on `LayoutProfile` itself for convenience:
```rust
impl LayoutProfile {
    pub fn classic() -> Self {
        LayoutProfile {
            id: "classic".to_string(),
            name: "Classic".to_string(),
            panel_position: PanelPosition::Top,
            panel_height: 30.0,
            dock_position: DockPosition::Bottom,
            dock_size: 32.0,
            launcher_style: LauncherStyle::StartMenu,
            window_header_style: WindowHeaderStyle::Standard,
        }
    }
}
```

Update `ThemePack::classic()` to use `LayoutProfile::classic()` instead of inline construction.

---

## Step 9: Add built-in layout list

In `crates/shared/src/theme.rs`, add:

```rust
impl LayoutProfile {
    /// All built-in layout profiles.
    pub fn builtin_layouts() -> Vec<LayoutProfile> {
        vec![
            LayoutProfile::classic(),
            LayoutProfile::minimal(),
        ]
    }
}
```

---

## Step 10: Wire layout switching into nucleon-tweaks

In `src/native/app/tweaks_presenter.rs`, add a layout picker section to the tweaks UI.
This should appear as a new section or tab.

Add a "Layout" section. It shows a list of available layouts (from `LayoutProfile::builtin_layouts()`).
When the user clicks a layout, update `self.active_layout` to the selected profile.

The UI should be simple:
```
Layout
  ( ) Classic — Panel at top, taskbar at bottom
  (*) Minimal — Panel at bottom, no taskbar
```

Use the existing retro radio button / choice button pattern from the settings UI.
Look at how `draw_top_bar` or the settings panels use `Self::retro_choice_button()`.

To detect the current selection, compare `self.active_layout.id` against each profile's `id`.

When a layout is selected:
```rust
self.active_layout = selected_profile;
```

No persistence to disk yet — that's Phase 5. For now, layout selection is runtime-only
(resets on restart). This is fine for testing.

---

## Step 11: Handle start menu position when panel is at bottom

When the panel (menu bar) moves to the bottom, the start menu (in `draw_start_panel`) needs
to know — it currently positions itself relative to the bottom-left of the screen as an
overlay. When the panel is at the bottom, the start menu should appear ABOVE the panel
rather than anchored to the bottom.

Read `src/native/app/desktop_start_menu.rs` and find how `draw_start_panel` positions
itself. It likely uses `egui::Area` with an anchor like `Align2::LEFT_BOTTOM` or a fixed
position near the taskbar.

If the start menu position depends on the taskbar being at the bottom:
- When `dock_position == Hidden` and `panel_position == Bottom`, the start menu should
  anchor above the bottom panel
- When dock is visible at bottom (Classic), behavior stays the same

This may require passing `active_layout` (or just the relevant position info) to
`draw_start_panel`. Read the function first, understand its positioning, and make the
minimal change needed.

If `draw_start_panel` positions relative to `self.desktop_start_button_rect` (which is set
in `draw_desktop_taskbar`), then when the taskbar is hidden, `desktop_start_button_rect`
will be `None`. Handle this case: if the start button rect is None (no taskbar), position
the start menu at the bottom-left of the screen, offset by the panel height if the panel is
at the bottom.

---

## Verification

- `cargo check -p robcos`
- `cargo check -p robcos-native-shell`
- Run the app with Classic layout — pixel-identical to pre-phase-3
- Open tweaks, switch to Minimal:
  - Top bar disappears from top, appears at bottom (smaller, 26px)
  - Taskbar disappears entirely
  - Desktop surface fills the extra space
  - Start menu still works (accessible from the panel's app menu or keyboard)
  - All desktop windows still work (open, move, resize, tile)
- Switch back to Classic — everything returns to normal

---

# PHASE 3b: Surface-specific theming and terminal slots

## Overview

Desktop mode and terminal mode are distinct shell surfaces. They must NOT share one live
theme/color/layout state object.

After this phase:
- Desktop mode has its own active color style and layout state
- Terminal mode has its own active color style and layout state
- Desktop and terminal may select different theme packs at the same time
- Tweaks has 2 top-level surface tabs: `Desktop` and `Terminal`
- Desktop and terminal surface changes apply independently at runtime
- Terminal mode has a slot registry parallel to the desktop slot registry
- The existing desktop slot system remains desktop-only; do NOT over-generalize both systems
  into one abstraction in this phase

This phase MUST be completed before the Full Color phase below. The current Phase 4 text still
contains single-surface assumptions; those assumptions are superseded by this phase.

Do NOT include sound/theme-pack audio work here. Sound profile swapping is a separate future phase.

Theme-pack rule for this phase:
- the installed theme-pack catalog may stay shared
- selection/application is surface-specific
- Desktop may point at one `ThemePack` while Terminal points at a different `ThemePack`
- choosing a theme pack for one surface must NOT implicitly retheme the other surface

---

## Step 1: Add terminal layout data model

In `crates/shared/src/theme.rs`, add terminal layout types. Keep them separate from
`LayoutProfile`; do NOT force terminal mode into the desktop layout schema.

Add:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TerminalStatusBarPosition {
    Bottom,
    Hidden,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminalLayoutProfile {
    pub id: String,
    pub name: String,
    pub status_bar_position: TerminalStatusBarPosition,
    pub status_bar_height: f32,
}
```

Also add built-ins:

```rust
impl TerminalLayoutProfile {
    pub fn classic() -> Self {
        TerminalLayoutProfile {
            id: "classic-terminal".to_string(),
            name: "Classic Terminal".to_string(),
            status_bar_position: TerminalStatusBarPosition::Bottom,
            status_bar_height: 28.0, // verify via retro_footer_height()
        }
    }

    pub fn minimal() -> Self {
        TerminalLayoutProfile {
            id: "minimal-terminal".to_string(),
            name: "Minimal Terminal".to_string(),
            status_bar_position: TerminalStatusBarPosition::Hidden,
            status_bar_height: 28.0,
        }
    }

    pub fn builtin_layouts() -> Vec<TerminalLayoutProfile> {
        vec![
            TerminalLayoutProfile::classic(),
            TerminalLayoutProfile::minimal(),
        ]
    }
}
```

NOTE: verify the classic status bar height by reading `retro_footer_height()`. The value above
is a placeholder until that read is done.

---

## Step 2: Split live theme state by surface

In `src/native/app.rs`, replace the single-surface runtime ownership model with explicit
surface-specific state.

Add these fields:

```rust
desktop_active_layout: LayoutProfile,
terminal_active_layout: TerminalLayoutProfile,
desktop_active_theme_pack_id: Option<String>,
terminal_active_theme_pack_id: Option<String>,
desktop_active_color_style: ColorStyle,
terminal_active_color_style: ColorStyle,
terminal_slot_registry: super::terminal_slots::TerminalSlotRegistry,
tweaks_surface_tab: u8,   // 0 = Desktop, 1 = Terminal
desktop_tweaks_tab: u8,   // existing desktop tweaks tabs
terminal_tweaks_tab: u8,  // 0 = Colors, 1 = Layout, 2 = Terminal
```

Import `ColorStyle` and `TerminalLayoutProfile` from `crate::theme`.

IMPORTANT:
- `desktop_active_layout` replaces the meaning of the current `active_layout`
- desktop and terminal color state must be separate from day 1 of this phase
- desktop and terminal selected theme-pack IDs must be separate from day 1 of this phase
- do NOT add one combined `SurfaceThemeState` struct in this phase
- keep the ownership flat on `RobcoNativeApp`

Defaults:

```rust
desktop_active_layout: crate::theme::LayoutProfile::classic(),
terminal_active_layout: crate::theme::TerminalLayoutProfile::classic(),
desktop_active_theme_pack_id: None,
terminal_active_theme_pack_id: None,
desktop_active_color_style: crate::theme::ColorStyle::Monochrome {
    preset: crate::theme::MonochromePreset::Green,
    custom_rgb: None,
},
terminal_active_color_style: crate::theme::ColorStyle::Monochrome {
    preset: crate::theme::MonochromePreset::Green,
    custom_rgb: None,
},
terminal_slot_registry: super::terminal_slots::TerminalSlotRegistry::classic(),
tweaks_surface_tab: 0,
desktop_tweaks_tab: 0,
terminal_tweaks_tab: 0,
```

Match the same global-vs-session persistence decision used for `active_layout` in Phase 3:
- desktop/terminal active layouts and color styles are global app state
- desktop/terminal selected theme-pack IDs are global app state
- do NOT park them in `ParkedSessionState`

Theme-pack selection rule:
- theme-pack discovery/inventory may remain one shared catalog
- the selected pack ID is tracked per surface
- applying a pack to Desktop derives Desktop runtime state only
- applying a pack to Terminal derives Terminal runtime state only

---

## Step 3: Add terminal slot registry

Create a new module directory:

```text
src/native/terminal_slots/
```

Add `mod terminal_slots;` to `src/native/mod.rs`.

Create `src/native/terminal_slots/mod.rs` with:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TerminalSlot {
    StatusBar,
    Screen,
    Overlay,
}

pub struct TerminalSlotContext<'a> {
    pub ctx: &'a Context,
    pub layout: &'a TerminalLayoutProfile,
}

pub trait TerminalSlotRenderer {
    fn slot(&self) -> TerminalSlot;
    fn render(
        &self,
        app: &mut RobcoNativeApp,
        slot_ctx: &TerminalSlotContext,
    ) -> Vec<SlotAction>;
}

pub struct TerminalSlotRegistry {
    renderers: Vec<Box<dyn TerminalSlotRenderer>>,
}
```

Mirror the desktop slot registry pattern exactly. Do NOT abstract both registries into one
generic slot type in this phase.

Add classic renderers:

```text
src/native/terminal_slots/classic_status_bar.rs
src/native/terminal_slots/classic_screen.rs
src/native/terminal_slots/classic_overlay.rs
```

Slots:
- `StatusBar` -> terminal status panel
- `Screen` -> the current `TerminalScreen` dispatcher body
- `Overlay` -> prompt overlay / command layer overlays that belong to terminal mode

The login screen, hacking screen, locked screen, and flash screens stay outside the terminal
slot system for this phase. Only the steady-state terminal runtime (`draw_terminal_runtime`) is
slotized.

---

## Step 4: Add terminal slot bridge methods

In `src/native/app.rs`, add bridge methods matching the desktop pattern:

```rust
pub(super) fn render_classic_terminal_status_slot(
    &mut self,
    ctx: &Context,
    layout: &TerminalLayoutProfile,
) { ... }

pub(super) fn render_classic_terminal_screen_slot(&mut self, ctx: &Context) { ... }

pub(super) fn render_classic_terminal_overlay_slot(&mut self, ctx: &Context) { ... }
```

The screen-slot bridge contains the current `match self.terminal_nav.screen { ... }` dispatch
that is currently inside `draw_terminal_runtime()`.

The overlay-slot bridge contains:
- terminal prompt overlay
- command layer overlays that belong to terminal mode

Do NOT move desktop overlays into this system.

---

## Step 5: Parameterize terminal status bar by terminal layout

Read `draw_terminal_status_bar()` in `src/native/app/terminal_screens.rs`. It currently uses:

```rust
TopBottomPanel::bottom("native_terminal_status_bar")
    .exact_height(retro_footer_height())
```

Change the signature to:

```rust
pub(super) fn draw_terminal_status_bar(
    &self,
    ctx: &Context,
    position: crate::theme::TerminalStatusBarPosition,
    height: f32,
) {
```

Then parameterize the panel construction:

```rust
use crate::theme::TerminalStatusBarPosition;

let panel = match position {
    TerminalStatusBarPosition::Bottom => TopBottomPanel::bottom("native_terminal_status_bar"),
    TerminalStatusBarPosition::Hidden => return,
};
panel
    .resizable(false)
    .exact_height(height)
```

The rest of the function body is unchanged.

This is the terminal equivalent of the desktop Phase 3 parameterization work.

---

## Step 6: Slotize draw_terminal_runtime

In `src/native/app/frame_runtime.rs`, keep the login / hacking / flash branches unchanged.

Inside the normal terminal branch, replace the direct terminal rendering sequence with terminal
slot dispatch.

Current shape:

```rust
self.draw_terminal_status_bar(ctx);
match self.terminal_nav.screen { ... }
self.draw_file_manager(ctx);
self.draw_editor(ctx);
self.draw_settings(ctx);
self.draw_applications(ctx);
self.draw_terminal_mode(ctx);
```

Target shape:

```rust
let terminal_layout = self.terminal_active_layout.clone();
let registry = std::mem::replace(
    &mut self.terminal_slot_registry,
    super::super::terminal_slots::TerminalSlotRegistry::classic(),
);
registry.render_slot(TerminalSlot::StatusBar, self, ctx, &terminal_layout);
registry.render_slot(TerminalSlot::Screen, self, ctx, &terminal_layout);
registry.render_slot(TerminalSlot::Overlay, self, ctx, &terminal_layout);
self.terminal_slot_registry = registry;

// Existing standalone/editor/file-manager windows remain after slot dispatch
self.draw_file_manager(ctx);
self.draw_editor(ctx);
self.draw_settings(ctx);
self.draw_applications(ctx);
self.draw_terminal_mode(ctx);
```

Do NOT slotize the login flow in this phase.

---

## Step 7: Split the palette pipeline by surface

The single global palette path is no longer correct once desktop and terminal can have
different active color styles.

In `src/native/retro_ui.rs`, add:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ShellSurfaceKind {
    Desktop,
    Terminal,
}

pub fn current_palette_for_surface(surface: ShellSurfaceKind) -> RetroPalette { ... }
pub fn set_active_color_style(surface: ShellSurfaceKind, style: ColorStyle) { ... }
```

Implementation rule:
- desktop palette reads `desktop_active_color_style`
- terminal palette reads `terminal_active_color_style`
- do NOT keep one global `ACTIVE_COLOR_STYLE`

Because `retro_ui.rs` is not a child module of `RobcoNativeApp`, the simplest implementation is
still a small global state store, but it must be keyed by surface:

```rust
Desktop -> Option<ColorStyle>
Terminal -> Option<ColorStyle>
```

Do NOT use one unscoped `ACTIVE_COLOR_STYLE`.

Update renderers:
- desktop-mode UI paths use `current_palette_for_surface(ShellSurfaceKind::Desktop)`
- terminal-mode UI paths use `current_palette_for_surface(ShellSurfaceKind::Terminal)`

Do NOT leave terminal renderers on `current_palette()` if `current_palette()` still resolves to
desktop state.

`current_palette()` may remain temporarily as a desktop-only compatibility helper during the
migration, but after this phase it must not be used by terminal-mode renderers.

---

## Step 8: Split appearance sync by surface

`sync_native_appearance()` currently assumes one desktop-oriented appearance state.

Refactor this into:

```rust
fn sync_desktop_appearance(&mut self, ctx: &Context) { ... }
fn sync_terminal_appearance(&mut self) { ... }
```

Rules:
- desktop appearance sync updates desktop egui visuals and desktop color-style activation
- terminal appearance sync updates terminal color-style activation only
- terminal mode does not call desktop layout sync

Do NOT reintroduce one shared "appearance key" for both surfaces.

---

## Step 9: Split Tweaks into Desktop and Terminal surface tabs

`Tweaks` is now a surface switcher first, not just a flat tab row.

At the top of `draw_tweaks()` in `src/native/app/tweaks_presenter.rs`, add:

```text
[ Desktop ] [ Terminal ]
```

Use `self.tweaks_surface_tab` to track selection.

### Desktop surface

Desktop keeps the existing controls, organized under the existing categories:
- Background
- Display
- Colors
- Icons
- Layout

These controls mutate ONLY:
- `desktop_active_theme_pack_id`
- `desktop_active_color_style`
- `desktop_active_layout`
- existing desktop-specific settings draft fields

### Terminal surface

Terminal gets its own sub-tabs:
- Colors
- Layout
- Terminal

These mutate ONLY:
- `terminal_active_theme_pack_id`
- `terminal_active_color_style`
- `terminal_active_layout`
- existing terminal/PTY settings fields (the current last tweaks tab moves here)

Do NOT share one tab index across both surfaces. Keep separate:
- `desktop_tweaks_tab`
- `terminal_tweaks_tab`

The current single `appearance_tab` is no longer sufficient.

---

## Step 10: Add terminal layout and color pickers

In the Terminal surface of Tweaks:

### Colors tab
- theme-pack picker for terminal surface
- Monochrome / Full Color mode toggle
- terminal-specific color theme picker
- terminal-specific monochrome preset/custom RGB picker

### Layout tab
- list `TerminalLayoutProfile::builtin_layouts()`
- selecting a layout updates only `self.terminal_active_layout`

Example:

```text
Terminal Layout
  (*) Classic Terminal — bottom status bar
  ( ) Minimal Terminal — no status bar
```

Do NOT write terminal layout selection into `desktop_active_layout`.

Desktop surface should also expose a desktop-only theme-pack picker. The two surface pickers
read from the same installed pack catalog, but they update different selected IDs.

---

## Step 11: Update Phase 4 assumption

Phase 4 below must be read with this correction:
- every reference to one `active_color_style` becomes 2 states:
  - `desktop_active_color_style`
  - `terminal_active_color_style`
- every reference to one `set_active_color_style(...)` becomes a surface-scoped call:
  - `set_active_color_style(ShellSurfaceKind::Desktop, ...)`
  - `set_active_color_style(ShellSurfaceKind::Terminal, ...)`

Desktop and terminal may choose different color modes:
- Desktop: Full Color
- Terminal: Monochrome

This is valid and expected.

---

## Verification

- `cargo check -p robcos`
- `cargo check -p robcos-native-shell`
- Run the app in Desktop Mode:
  - change desktop layout/theme in Tweaks Desktop tab
  - terminal mode appearance does NOT change
- Switch to Terminal Mode:
  - change terminal layout/theme in Tweaks Terminal tab
  - desktop appearance does NOT change
- Terminal status bar visibility follows terminal layout only
- Desktop panel/taskbar/layout follow desktop layout only
- Tweaks surface tabs do not leak state between each other

---

# PHASE 4: Full Color mode

## NOTE

Phase 4 assumes Phase 3b is complete.

Any instruction below that refers to a single global active color style is superseded by
Phase 3b. Full Color must be implemented per surface, not globally.

## Overview

Implement the `ColorStyle::FullColor` path so users can switch between Monochrome (single-hue
CRT tint) and Full Color (multi-color UI with semantic tokens).

**Key design decision:** Do NOT touch the 554 existing `palette.fg` / `palette.selected_bg`
call sites. Instead, populate `RetroPalette` differently depending on mode. In Monochrome
mode, all fields are derived from one color (existing logic). In FullColor mode, fields are
populated from the `FullColorTheme` token map. Call sites are unaware of which mode produced
the palette.

After this phase:
- User can toggle between Monochrome and FullColor in the tweaks app
- 2 built-in FullColor themes: "Nucleon Dark", "Nucleon Light"
- Monochrome CRT shader tint is disabled when FullColor is active
- All 554 palette consumption sites work unchanged

### Current palette system (retro_ui.rs)

The `RetroPalette` struct has 9 fields consumed everywhere:
```rust
pub struct RetroPalette {
    pub fg: Color32,          // primary foreground
    pub dim: Color32,         // dimmed/secondary text
    pub bg: Color32,          // primary background
    pub panel: Color32,       // panel/sidebar background
    pub selected_bg: Color32, // selection background (also used for top bar fill)
    pub selected_fg: Color32, // selection text color
    pub hovered_bg: Color32,  // hover state background
    pub active_bg: Color32,   // pressed/active state background
    pub selection_bg: Color32, // text selection highlight
}
```

These map naturally to `ColorToken` values:
| RetroPalette field | ColorToken |
|-------------------|------------|
| `fg` | `FgPrimary` |
| `dim` | `FgDim` |
| `bg` | `BgPrimary` |
| `panel` | `PanelBg` |
| `selected_bg` | `Selection` |
| `selected_fg` | `SelectionFg` |
| `hovered_bg` | `AccentHover` |
| `active_bg` | `AccentActive` |
| `selection_bg` | `Selection` (same as selected_bg) |

### Current flow

1. Settings stores a theme name string (e.g. "Green", "Amber")
2. `config.rs` resolves theme name → `ratatui::style::Color`
3. `retro_ui::palette_for_theme_color(color)` → `RetroPalette` (all derived from one color)
4. `apply_visuals_with_palette(ctx, palette)` → sets egui visuals
5. Components call `current_palette()` and use the fields

### Target flow for FullColor

1. Settings stores a `ColorStyle` (either Monochrome or FullColor with theme_id)
2. `retro_ui::palette_for_color_style(style)` → `RetroPalette`
   - Monochrome path: existing `palette_for_theme_color` logic
   - FullColor path: new `palette_from_full_color_theme` that maps tokens → fields
3. `apply_visuals_with_palette` works exactly as before
4. Components call `current_palette()` — completely unchanged

---

## Step 1: Define built-in FullColor themes

In `crates/shared/src/theme.rs`, add constructors for 3 built-in themes:

```rust
impl FullColorTheme {
    /// All built-in full-color themes.
    pub fn builtin_themes() -> Vec<FullColorTheme> {
        vec![
            FullColorTheme::nucleon_dark(),
            FullColorTheme::nucleon_light(),
        ]
    }

    pub fn nucleon_dark() -> Self {
        let mut tokens = std::collections::HashMap::new();
        // Deep charcoal workspace with teal accent (sci-fi terminal feel)
        // Backgrounds: near-black charcoal, not pure black
        tokens.insert(ColorToken::BgPrimary,           [18, 18, 24, 255]);   // #121218
        tokens.insert(ColorToken::BgSecondary,         [30, 30, 38, 255]);   // #1E1E26
        // Text: cool-toned whites and grays (not warm)
        tokens.insert(ColorToken::FgPrimary,           [212, 212, 216, 255]); // #D4D4D8 zinc-200
        tokens.insert(ColorToken::FgSecondary,         [161, 161, 170, 255]); // #A1A1AA zinc-400
        tokens.insert(ColorToken::FgDim,               [99, 99, 112, 255]);   // #636370 zinc-500
        // Accent: teal (nods to CRT phosphor glow without being literal green)
        tokens.insert(ColorToken::Accent,              [45, 212, 191, 255]);  // #2DD4BF teal-400
        tokens.insert(ColorToken::AccentHover,         [20, 184, 166, 255]);  // #14B8A6 teal-500
        tokens.insert(ColorToken::AccentActive,        [13, 148, 136, 255]);  // #0D9488 teal-600
        // Panels/chrome: subtle elevation over background
        tokens.insert(ColorToken::PanelBg,             [24, 24, 27, 255]);    // #18181B zinc-900
        tokens.insert(ColorToken::PanelBorder,         [63, 63, 70, 255]);    // #3F3F46 zinc-700
        tokens.insert(ColorToken::WindowChrome,        [39, 39, 42, 255]);    // #27272A zinc-800
        tokens.insert(ColorToken::WindowChromeFocused, [52, 52, 56, 255]);    // #343438
        // Selection: teal accent as selection highlight
        tokens.insert(ColorToken::Selection,           [45, 212, 191, 255]);  // #2DD4BF
        tokens.insert(ColorToken::SelectionFg,         [0, 0, 0, 255]);       // black on bright teal
        tokens.insert(ColorToken::Border,              [63, 63, 70, 255]);    // #3F3F46 zinc-700
        tokens.insert(ColorToken::Separator,           [52, 52, 56, 255]);    // #343438
        tokens.insert(ColorToken::StatusBar,           [24, 24, 27, 255]);    // #18181B
        // Semantic
        tokens.insert(ColorToken::Error,               [248, 113, 113, 255]); // #F87171 red-400
        tokens.insert(ColorToken::Warning,             [251, 191, 36, 255]);  // #FBBF24 amber-400
        tokens.insert(ColorToken::Success,             [52, 211, 153, 255]);  // #34D399 emerald-400
        FullColorTheme {
            id: "nucleon-dark".to_string(),
            name: "Nucleon Dark".to_string(),
            tokens,
        }
    }

    pub fn nucleon_light() -> Self {
        let mut tokens = std::collections::HashMap::new();
        // Clean warm-white workspace with slate blue accent
        // Backgrounds: warm off-white (stone family, not sterile)
        tokens.insert(ColorToken::BgPrimary,           [250, 250, 249, 255]); // #FAFAF9 stone-50
        tokens.insert(ColorToken::BgSecondary,         [245, 245, 244, 255]); // #F5F5F4 stone-100
        // Text: warm near-black and grays
        tokens.insert(ColorToken::FgPrimary,           [28, 25, 23, 255]);    // #1C1917 stone-900
        tokens.insert(ColorToken::FgSecondary,         [87, 83, 78, 255]);    // #57534E stone-600
        tokens.insert(ColorToken::FgDim,               [168, 162, 158, 255]); // #A8A29E stone-400
        // Accent: slate blue (understated, professional)
        tokens.insert(ColorToken::Accent,              [71, 85, 105, 255]);   // #475569 slate-600
        tokens.insert(ColorToken::AccentHover,         [51, 65, 85, 255]);    // #334155 slate-700
        tokens.insert(ColorToken::AccentActive,        [30, 41, 59, 255]);    // #1E293B slate-800
        // Panels/chrome: light stone grays
        tokens.insert(ColorToken::PanelBg,             [231, 229, 228, 255]); // #E7E5E4 stone-200
        tokens.insert(ColorToken::PanelBorder,         [214, 211, 209, 255]); // #D6D3D1 stone-300
        tokens.insert(ColorToken::WindowChrome,        [231, 229, 228, 255]); // #E7E5E4 stone-200
        tokens.insert(ColorToken::WindowChromeFocused, [214, 211, 209, 255]); // #D6D3D1 stone-300
        // Selection: slate accent on white text
        tokens.insert(ColorToken::Selection,           [71, 85, 105, 255]);   // #475569 slate-600
        tokens.insert(ColorToken::SelectionFg,         [255, 255, 255, 255]); // white on dark slate
        tokens.insert(ColorToken::Border,              [214, 211, 209, 255]); // #D6D3D1 stone-300
        tokens.insert(ColorToken::Separator,           [214, 211, 209, 255]); // #D6D3D1 stone-300
        tokens.insert(ColorToken::StatusBar,           [231, 229, 228, 255]); // #E7E5E4 stone-200
        // Semantic (darker variants for light bg)
        tokens.insert(ColorToken::Error,               [220, 38, 38, 255]);   // #DC2626 red-600
        tokens.insert(ColorToken::Warning,             [202, 138, 4, 255]);   // #CA8A04 yellow-600
        tokens.insert(ColorToken::Success,             [22, 163, 74, 255]);   // #16A34A green-600
        FullColorTheme {
            id: "nucleon-light".to_string(),
            name: "Nucleon Light".to_string(),
            tokens,
        }
    }

    /// Look up a built-in theme by ID. Returns None if not found.
    pub fn builtin_by_id(id: &str) -> Option<FullColorTheme> {
        Self::builtin_themes().into_iter().find(|t| t.id == id)
    }
}
```

---

## Step 2: Add palette_from_full_color_theme to retro_ui.rs

In `src/native/retro_ui.rs`, add a function that builds `RetroPalette` from a
`FullColorTheme` by mapping tokens to palette fields:

```rust
use crate::theme::{ColorToken, FullColorTheme};

fn color32_from_token(theme: &FullColorTheme, token: ColorToken, fallback: Color32) -> Color32 {
    theme.tokens.get(&token)
        .map(|&[r, g, b, a]| Color32::from_rgba_unmultiplied(r, g, b, a))
        .unwrap_or(fallback)
}

fn palette_from_full_color_theme(theme: &FullColorTheme) -> RetroPalette {
    let fg = color32_from_token(theme, ColorToken::FgPrimary, Color32::from_rgb(220, 220, 220));
    let bg = color32_from_token(theme, ColorToken::BgPrimary, Color32::from_rgb(18, 18, 18));
    RetroPalette {
        fg,
        dim: color32_from_token(theme, ColorToken::FgDim, scale(fg, 0.52)),
        bg,
        panel: color32_from_token(theme, ColorToken::PanelBg, scale(fg, 0.06)),
        selected_bg: color32_from_token(theme, ColorToken::Selection, fg),
        selected_fg: color32_from_token(theme, ColorToken::SelectionFg, bg),
        hovered_bg: color32_from_token(theme, ColorToken::AccentHover, scale(fg, 0.18)),
        active_bg: color32_from_token(theme, ColorToken::AccentActive, scale(fg, 0.26)),
        selection_bg: color32_from_token(theme, ColorToken::Selection, scale(fg, 0.26)),
    }
}
```

Then update the existing `palette_for_color_style` stub to actually use it:

**Before:**
```rust
pub fn palette_for_color_style(style: &ColorStyle) -> RetroPalette {
    match style {
        ColorStyle::Monochrome { preset, custom_rgb } => {
            let color = monochrome_preset_to_color(*preset, *custom_rgb);
            palette_for_theme_color(color)
        }
        ColorStyle::FullColor { theme_id: _ } => palette_for_theme_color(Color::Rgb(111, 255, 84)),
    }
}
```

**After:**
```rust
pub fn palette_for_color_style(style: &ColorStyle) -> RetroPalette {
    match style {
        ColorStyle::Monochrome { preset, custom_rgb } => {
            let color = monochrome_preset_to_color(*preset, *custom_rgb);
            palette_for_theme_color(color)
        }
        ColorStyle::FullColor { theme_id } => {
            match FullColorTheme::builtin_by_id(theme_id) {
                Some(theme) => palette_from_full_color_theme(&theme),
                None => palette_for_theme_color(Color::Rgb(111, 255, 84)), // fallback
            }
        }
    }
}
```

---

## Step 3: Store active ColorStyle on RobcoNativeApp

In `src/native/app.rs`, add a field:

```rust
active_color_style: ColorStyle,
```

Import `ColorStyle` from `crate::theme`.

In the `Default` impl:
```rust
active_color_style: crate::theme::ColorStyle::Monochrome {
    preset: crate::theme::MonochromePreset::Green,
    custom_rgb: None,
},
```

This initializes to the same default as today. The active_color_style is the source of truth
for the current color mode.

---

## Step 4: Wire active ColorStyle into the palette pipeline

This is the critical wiring step. Currently `current_palette()` reads the theme color from
the old settings path (`config::current_theme_color()`). We need to make it also work with
`ColorStyle::FullColor`.

There are two approaches:
- **(a)** Replace `current_palette()` calls with a method on `RobcoNativeApp` that uses `active_color_style`
- **(b)** Set a global `ColorStyle` that `current_palette()` reads

Approach (b) is simpler and matches the existing pattern (there's already a global
`PALETTE_CACHE` mutex). But approach (a) is cleaner long-term.

**Use approach (b) for now** — add a global active style that `current_palette()` checks:

In `src/native/retro_ui.rs`, add:

```rust
static ACTIVE_COLOR_STYLE: Mutex<Option<ColorStyle>> = Mutex::new(None);

/// Set the active color style. Called during appearance sync.
pub fn set_active_color_style(style: ColorStyle) {
    if let Ok(mut guard) = ACTIVE_COLOR_STYLE.lock() {
        *guard = Some(style);
    }
    // Invalidate palette cache
    if let Ok(mut guard) = PALETTE_CACHE.lock() {
        *guard = None;
    }
}
```

Then update `current_palette()`:

**Before:**
```rust
pub fn current_palette() -> RetroPalette {
    let color = current_theme_color();
    if let Ok(mut guard) = PALETTE_CACHE.lock() {
        if let Some(cache) = *guard {
            if cache.color == color {
                return cache.palette;
            }
        }
        let palette = palette_for_theme_color(color);
        *guard = Some(PaletteCache { color, palette });
        return palette;
    }
    palette_for_theme_color(color)
}
```

**After:**
```rust
pub fn current_palette() -> RetroPalette {
    // Check if a FullColor style is active
    if let Ok(guard) = ACTIVE_COLOR_STYLE.lock() {
        if let Some(ColorStyle::FullColor { ref theme_id }) = *guard {
            // FullColor mode — bypass the old theme color path entirely
            return match FullColorTheme::builtin_by_id(theme_id) {
                Some(theme) => palette_from_full_color_theme(&theme),
                None => palette_for_theme_color(Color::Rgb(111, 255, 84)),
            };
        }
    }
    // Monochrome mode — existing path via config theme color
    let color = current_theme_color();
    if let Ok(mut guard) = PALETTE_CACHE.lock() {
        if let Some(cache) = *guard {
            if cache.color == color {
                return cache.palette;
            }
        }
        let palette = palette_for_theme_color(color);
        *guard = Some(PaletteCache { color, palette });
        return palette;
    }
    palette_for_theme_color(color)
}
```

NOTE: The FullColor path does NOT use `PaletteCache` (which caches by `ratatui::Color`).
For FullColor, consider adding a separate cache keyed by theme_id. Or just skip caching for
now — `palette_from_full_color_theme` is cheap (HashMap lookups). Don't over-optimize.

---

## Step 5: Call set_active_color_style during appearance sync

Find `sync_native_appearance()` in the codebase. This is where the app syncs visual settings
each frame. Read it fully.

When the user's `active_color_style` changes (either through the tweaks UI or settings), call:

```rust
retro_ui::set_active_color_style(self.active_color_style.clone());
```

This should be called during `sync_native_appearance()`. Read the function first to find the
right insertion point — it already calls `configure_visuals(ctx)` which calls
`current_palette()`.

---

## Step 6: Disable monochrome shader tint in FullColor mode

Find where CRT shader uniforms are set (look for where `monochrome_enabled` / `monochrome_tint`
are written — this was added in Phase 2b). Read the function.

When `active_color_style` is `FullColor`:
- Set `monochrome_enabled = 0`
- Set `monochrome_tint = [0.0, 0.0, 0.0]`

When `active_color_style` is `Monochrome`:
- Set `monochrome_enabled = 1`
- Set `monochrome_tint` to the normalized RGB of the preset color

This ensures FullColor themes display their actual multi-color palette without any CRT tinting.

---

## Step 7: Add color mode UI to nucleon-tweaks

In `src/native/app/tweaks_presenter.rs`, update the Colors tab (tab index 2) to show:

### Mode toggle
```
Color Mode
  (*) Monochrome    ( ) Full Color
```

### When Monochrome is selected
Show the existing color preset picker (Green, Amber, White, Blue, LightBlue, Custom RGB).
This is the current UI — keep it exactly as-is.

### When Full Color is selected
Show a theme picker:
```
Color Theme
  ( ) Nucleon Dark — Dark background, teal accents
  (*) Nucleon Light — Light background, slate blue accents
```

Use `FullColorTheme::builtin_themes()` to populate the list.
Compare `theme.id` against the current `theme_id` in `active_color_style` to determine selection.

When the user selects a mode or theme:
```rust
// Switch to Monochrome
self.active_color_style = ColorStyle::Monochrome {
    preset: current_preset,
    custom_rgb: current_custom,
};

// Switch to FullColor
self.active_color_style = ColorStyle::FullColor {
    theme_id: selected_theme.id.clone(),
};
```

Then call `set_active_color_style(self.active_color_style.clone())` to apply immediately.

---

## Step 8: Handle top bar and taskbar styling in FullColor mode

The top bar (`draw_top_bar` in `desktop_menu_bar.rs`) paints its background with
`palette.selected_bg` and uses `Color32::BLACK` for text. This works in Monochrome because
`selected_bg` is the bright theme color. In FullColor mode, `selected_bg` (mapped from
`Selection` token) might be a dark color, making black text invisible.

Check if the existing `draw_top_bar` uses hardcoded `Color32::BLACK`:
- Line 37: `RichText::new(app_menu_name).strong().color(Color32::BLACK)`
- Line 280: `RichText::new(batt).color(Color32::BLACK)`
- Line 283: `RichText::new(now).color(Color32::BLACK)`
- Lines 314-336: multiple `Color32::BLACK` references in button styling

These hardcoded `Color32::BLACK` values need to become `palette.selected_fg` so they adapt
to the FullColor theme. This is the ONE place where existing code needs updating for
FullColor compatibility.

**Change all hardcoded `Color32::BLACK` in `draw_top_bar` to `palette.selected_fg`.**

Same check for `draw_desktop_taskbar` in `desktop_taskbar.rs`:
- Line 65: `Color32::BLACK` for "[Start]" text
- Line 75: `Color32::BLACK` for "|" separator

**Change these to `palette.selected_fg` as well.**

Do NOT change `Color32::BLACK` references in OTHER files — only in the top bar and taskbar
where the background is explicitly `palette.selected_bg`. Other files use `Color32::BLACK`
for different reasons (e.g., true black backgrounds) and those should stay.

---

## Verification

- `cargo check -p robcos`
- `cargo check -p robcos-native-shell`
- Run app in Monochrome Green — pixel-identical to before
- Open tweaks Colors tab, switch to Full Color → Nucleon Dark:
  - Background changes from black to dark gray
  - Text changes from green to light gray
  - Accents are green (not monochrome-derived)
  - CRT tint is OFF — colors display as actual RGB values
  - Top bar and taskbar text is readable (not black-on-dark)
  - All windows, menus, dialogs render correctly
- Switch to Nucleon Light:
  - Light background, dark text, blue accents
- Switch back to Monochrome — CRT tint re-enables, single-hue look returns

Anything still wrong in these areas remains Phase 4 work and should be fixed before starting a new phase:

- built-in `Nucleon Dark` / `Nucleon Light` token balance and readability
- CRT/full-color interaction bugs
- menu, taskbar, start-menu, spotlight, window chrome, PTY, or Tweaks contrast issues
- terminal `Settings -> Appearance` routing and terminal-native Tweaks behavior
- per-surface persistence for theme-pack/color-style/layout state

Do not defer those items to Phase 5 or Phase 6. Those later phases are for new systems, not cleanup of the built-in Full Color implementation.

---

# PHASE 5: Asset packs and shell style

### DEFERRED — Spec will be written after Phase 4 is implemented

---

# PHASE 6: nucleon-core-themes repo and packaging

### DEFERRED — Spec will be written after Phase 4 is implemented

---

# Rules for Codex

1. **Do not invent abstractions.** Only create what is described in this spec.
2. **Do not rename existing functions/types** unless the spec says to.
3. **Do not add comments** to code you didn't write.
4. **Do not add error handling** beyond what exists in the code you're wrapping.
5. **Do not add tests** unless the spec says to. (Tests will be a separate task.)
6. **Preserve exact import style.** Use `super::super::` not `crate::native::` for imports in `src/native/app/` files.
7. **Preserve exact visibility.** Functions in `src/native/app/` files use `pub(super)`. Functions in `src/native/` files use `pub(crate)` or `pub` depending on existing patterns.
8. **One phase at a time.** Complete Phase 0a, verify it compiles, then Phase 0b, etc.
9. **Read before writing.** Every file you modify must be read first to understand the full context.
10. **The Classic theme is the correctness test.** If the app looks different after your changes, something is wrong.
