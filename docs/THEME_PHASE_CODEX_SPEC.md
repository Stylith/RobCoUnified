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

### DEFERRED — Spec will be written after Phase 0-2 are implemented

Phase 3 makes shell structure data-driven using LayoutProfile. The slot registry already provides the dispatch mechanism (Phase 1b). Phase 3 adds:
- A layout interpreter that computes egui Rects from LayoutProfile
- Slot renderers receive their assigned Rect from the interpreter
- Classic layout profile produces identical layout to current hardcoded values

---

# PHASE 4: Asset packs and shell style

### DEFERRED — Spec will be written after Phase 0-2 are implemented

---

# PHASE 5: nucleon-core-themes repo and packaging

### DEFERRED — Spec will be written after Phase 0-2 are implemented

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
