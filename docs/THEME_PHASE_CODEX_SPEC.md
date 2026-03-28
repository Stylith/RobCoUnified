# Nucleon Theme/Shell Composition — Codex Implementation Spec

> This document is the single source of truth for implementing the theme/shell composition system.
> Each phase is a self-contained unit of work. Do not skip ahead. Do not invent abstractions not described here.
> After each phase, `cargo check -p nucleon` and `cargo check -p nucleon-native-shell` must pass.
> (After Phase 11, these become `cargo check -p nucleon` and `cargo check -p nucleon-native-shell`.)
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
| `src/native/app.rs` | Main `NucleonNativeApp` struct (lines 483-574), Default impl (639+), appearance sync | ~850 |
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

All files in `src/native/app/` are sub-modules of `src/native/app.rs`. They use `super::super::` to reach `src/native/` siblings, and `super::NucleonNativeApp` to reach the app struct. Every function in these files is an `impl NucleonNativeApp` method with visibility `pub(super)` or `pub(crate)`.

### Current component binding system

In `src/native/desktop_app.rs`, lines 152-286:

```rust
#[derive(Clone, Copy)]
pub struct DesktopComponentBinding {
    pub spec: DesktopComponentSpec,
    pub is_open: fn(&NucleonNativeApp) -> bool,
    pub set_open: fn(&mut NucleonNativeApp, bool),
    pub draw: fn(&mut NucleonNativeApp, &Context),
    pub on_open: Option<fn(&mut NucleonNativeApp, bool)>,
    pub on_closed: Option<fn(&mut NucleonNativeApp)>,
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
name = "nucleon-native-tweaks-app"
version = "0.4.4"
edition = "2021"
license = "GPL-3.0-only"

[dependencies]
nucleon = { path = "../.." }
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
nucleon-native-tweaks-app = { path = "../native-tweaks-app" }
```

Add binary entries after the installer entries:
```toml
[[bin]]
name = "nucleon-tweaks"
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
- Define `NucleonNativeTweaksApp` struct
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
    is_open: NucleonNativeApp::desktop_component_tweaks_is_open,
    set_open: NucleonNativeApp::desktop_component_tweaks_set_open,
    draw: NucleonNativeApp::desktop_component_tweaks_draw,
    on_open: None,
    on_closed: None,
}
```

### Step 8: Add tweaks state to NucleonNativeApp

In `src/native/app.rs`, add to `NucleonNativeApp` struct:
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

This file contains `impl NucleonNativeApp` with:
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
- `cargo check -p nucleon`
- `cargo check -p nucleon-native-shell`
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
1. Copy the relevant `impl NucleonNativeApp` block containing the function
2. Copy the `use` imports that the moved function needs (trace each one — only copy what's actually used)
3. Keep the `use super::NucleonNativeApp;` pattern (since these are sub-modules of `app`)
4. Keep visibility as `pub(super)`

The import style in these files follows the existing convention: `use super::super::` to reach `src/native/` siblings.

### Step 3: Update desktop_window_presenters.rs

After moving, `desktop_window_presenters.rs` should either:
- Be deleted entirely (if all functions have been moved)
- Or contain only shared helper functions used by multiple presenters

If there are shared helpers (like `desktop_window_frame()`, `desktop_default_window_size()`, etc.), check if they're defined in this file or elsewhere. They're likely on `impl NucleonNativeApp` in `desktop_window_mgmt.rs` — verify before assuming.

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

Every call site that invoked `self.draw_file_manager(ctx)`, `self.draw_editor(ctx)`, `self.draw_settings(ctx)` should still work — they call methods on `NucleonNativeApp`, not functions in a specific module. The methods are found by Rust's impl resolution regardless of which file they're in.

### Verification
- `cargo check -p nucleon`
- `cargo check -p nucleon-native-shell`
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
- `cargo check -p nucleon`
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
- `cargo check -p nucleon-shared`
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
    fn render(&self, app: &mut super::app::NucleonNativeApp, slot_ctx: &SlotContext) -> Vec<SlotAction>;
}
```

NOTE: The `app: &mut NucleonNativeApp` parameter is intentionally kept for Phase 1. The ShellState boundary (Phase 1c) will replace this with a constrained view. Don't try to do both at once.

### Step 2: Create Classic renderers (thin wrappers)

Create `src/native/shell_slots/classic_panel.rs`:
```rust
use super::{ShellSlot, SlotAction, SlotContext, SlotRenderer};
use crate::native::app::NucleonNativeApp;

pub struct ClassicPanelRenderer;

impl SlotRenderer for ClassicPanelRenderer {
    fn slot(&self) -> ShellSlot { ShellSlot::Panel }

    fn render(&self, app: &mut NucleonNativeApp, slot_ctx: &SlotContext) -> Vec<SlotAction> {
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

    pub fn render_slot(&self, slot: ShellSlot, app: &mut NucleonNativeApp, ctx: &Context) -> Vec<SlotAction> {
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

In `src/native/app.rs`, add to `NucleonNativeApp`:
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

NOTE: The `std::mem::replace` pattern is ugly but necessary because `SlotRenderer::render` takes `&mut NucleonNativeApp` while `slot_registry` is a field of `NucleonNativeApp`. An alternative is to use `Option<SlotRegistry>` and `.take()` / re-assign. Choose whichever compiles cleanly. The CRITICAL thing is that the same functions get called in the same order.

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
- `cargo check -p nucleon`
- Run the app — desktop mode must look and behave identically
- The slot renderers are just wrappers around the same methods

---

## Phase 1c: ShellState boundary

### Goal
Define a constrained state view that slot renderers receive instead of `&mut NucleonNativeApp`.

### THIS PHASE IS DEFERRED
Phase 1c is intentionally left as a design placeholder. The `&mut NucleonNativeApp` approach from Phase 1b works for v1. Phase 1c should only be implemented when we actually have third-party slot renderers that need sandboxing. Do not implement this phase yet.

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
- `cargo check -p nucleon`
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
- `cargo check -p nucleon`
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
- `cargo check -p nucleon`
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
impl NucleonNativeApp {
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

## Step 1: Add active_layout to NucleonNativeApp

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
    app: &mut NucleonNativeApp,
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
on `NucleonNativeApp`. The bridge methods need to accept layout parameters.

### 4a: Panel renderer

In `src/native/shell_slots/classic_panel.rs`:
```rust
fn render(&self, app: &mut NucleonNativeApp, slot_ctx: &SlotContext) -> Vec<SlotAction> {
    app.render_classic_panel_slot(slot_ctx.ctx, &slot_ctx.layout);
    vec![]
}
```

### 4b: Dock renderer

In `src/native/shell_slots/classic_dock.rs`:
```rust
fn render(&self, app: &mut NucleonNativeApp, slot_ctx: &SlotContext) -> Vec<SlotAction> {
    app.render_classic_dock_slot(slot_ctx.ctx, &slot_ctx.layout);
    vec![]
}
```

### 4c: Launcher renderer

In `src/native/shell_slots/classic_launcher.rs`:
```rust
fn render(&self, app: &mut NucleonNativeApp, slot_ctx: &SlotContext) -> Vec<SlotAction> {
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
fn render(&self, app: &mut NucleonNativeApp, slot_ctx: &SlotContext) -> Vec<SlotAction> {
    app.render_classic_spotlight_slot(slot_ctx.ctx);
    vec![]
}
```
Desktop surface always uses `CentralPanel` which fills remaining space — no layout changes needed.

---

## Step 5: Update bridge methods to accept layout and parameterize panel calls

In `src/native/app.rs`, update the bridge methods:

```rust
impl NucleonNativeApp {
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

- `cargo check -p nucleon`
- `cargo check -p nucleon-native-shell`
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
- keep the ownership flat on `NucleonNativeApp`

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
        app: &mut NucleonNativeApp,
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

Because `retro_ui.rs` is not a child module of `NucleonNativeApp`, the simplest implementation is
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

- `cargo check -p nucleon`
- `cargo check -p nucleon-native-shell`
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

## Step 3: Store active ColorStyle on NucleonNativeApp

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
- **(a)** Replace `current_palette()` calls with a method on `NucleonNativeApp` that uses `active_color_style`
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

- `cargo check -p nucleon`
- `cargo check -p nucleon-native-shell`
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

# PHASE 5: Terminal header branding slot

### Goal
The heritage terminal header becomes theme-controlled. By default (Classic theme)
it is **hidden**. A future heritage theme pack can re-enable it. When hidden,
terminal screen rows shift up to reclaim the space.

### Background

The header is rendered in 9 places in `src/native/` via this pattern:
```rust
for (idx, line) in HEADER_LINES.iter().enumerate() {
    screen.centered_text(&painter, header_start_row + idx, line, palette.fg, true);
}
```

Files that render the header directly:
- `src/native/shell_screen.rs` (2 call sites: lines 70, 206)
- `src/native/about_screen.rs` (line 41)
- `src/native/menu.rs` (line 98)
- `src/native/document_browser.rs` (line 103)
- `src/native/prompt.rs` (2 call sites: lines 201, 231)
- `src/native/settings_screen.rs` (line 95)
- `src/native/app/tweaks_presenter.rs` (line 2026)

There is also a TUI path in `crates/shared/src/ui.rs:323` (`render_header()`) called by
6 functions in `ui.rs` plus 3 sites in `src/legacy/`. The TUI path is out of scope for
this phase — it will be addressed when the legacy TUI is retired.

The header occupies rows 0-2 (3 rows). When hidden, every row index after it shifts
up by 3, gaining 3 extra rows of usable content space.

### Step 1: Add header configuration to ThemePack

In `crates/shared/src/theme.rs`, add a new struct and field:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminalBranding {
    pub header_lines: Vec<String>,
}

impl TerminalBranding {
    pub fn none() -> Self {
        TerminalBranding {
            header_lines: vec![],
        }
    }

    pub fn heritage() -> Self {
        TerminalBranding {
            header_lines: vec!["...".to_string(), "...".to_string(), "...".to_string()],
        }
    }
}
```

Add to `ThemePack`:
```rust
pub struct ThemePack {
    // ... existing fields ...
    pub terminal_branding: TerminalBranding,
}
```

Update `ThemePack::classic()` to use `TerminalBranding::none()`. The Classic theme has
no heritage branding — it is Nucleon's default identity.

### Step 2: Store active branding in NucleonNativeApp

In `src/native/app.rs`, add a field:
```rust
pub(super) terminal_branding: TerminalBranding,
```

Initialize it from the active theme pack in the constructor (default: `TerminalBranding::none()`).

When a theme pack is applied (in tweaks or via `apply_theme_pack()`), copy the pack's
`terminal_branding` into this field.

### Step 3: Compute terminal layout with dynamic header

The current `TerminalLayout` struct in `src/native/app.rs` (line 338) has hardcoded row
positions. The header occupies rows 0-2 (3 rows), with `separator_top_row: 3`.

Add a function that computes layout based on header line count:

```rust
fn terminal_layout_with_branding(branding: &TerminalBranding) -> TerminalLayout {
    let header_lines = branding.header_lines.len();
    // When header is empty, rows shift up by 3 (the old header size)
    // When header has N lines, separator starts at row N
    let header_start_row = 0;
    let separator_top_row = if header_lines == 0 { 0 } else { header_lines };
    let title_row = separator_top_row + 1;
    let separator_bottom_row = title_row + 1;
    let subtitle_row = separator_bottom_row + 2;
    let menu_start_row = subtitle_row + 2;
    // Keep status_row pinned to bottom
    let status_row = TERMINAL_SCREEN_ROWS - 4;
    let status_row_alt = TERMINAL_SCREEN_ROWS - 2;
    TerminalLayout {
        cols: TERMINAL_SCREEN_COLS,
        rows: TERMINAL_SCREEN_ROWS,
        content_col: TERMINAL_CONTENT_COL,
        header_start_row,
        separator_top_row,
        title_row,
        separator_bottom_row,
        subtitle_row,
        menu_start_row,
        status_row,
        status_row_alt,
    }
}
```

Replace the call to `terminal_layout_for_scale()` in `NucleonNativeApp::terminal_layout()`
so it calls `terminal_layout_with_branding(&self.terminal_branding)` instead.

**Important:** `terminal_layout_for_scale()` is currently called in one place. Find it
by searching for `terminal_layout_for_scale` — it should be in a method like
`fn terminal_layout(&self) -> TerminalLayout`. Replace that call.

### Step 4: Replace direct HEADER_LINES rendering with branding-aware rendering

Add a helper method to `NucleonNativeApp`:

```rust
pub(super) fn active_terminal_header_lines(&self) -> &[String] {
    &self.terminal_branding.header_lines
}
```

**However**, the 9 rendering call sites are NOT methods on `NucleonNativeApp` — they are
standalone functions that receive `header_start_row` as a parameter. They do NOT have
access to `self`. The header lines need to be passed as a parameter.

The cleanest approach: change `header_start_row: usize` parameter to a new type that
carries both the start row and the lines:

**Do NOT do that.** Too many call sites. Instead, use a simpler approach:

Add a new parameter `header_lines: &[&str]` to each rendering function that currently
uses `HEADER_LINES`. This is mechanical — each site already receives `header_start_row`.

For each of the 9 call sites listed above, change:
```rust
// BEFORE:
for (idx, line) in HEADER_LINES.iter().enumerate() {
    screen.centered_text(&painter, header_start_row + idx, line, palette.fg, true);
}

// AFTER:
for (idx, line) in header_lines.iter().enumerate() {
    screen.centered_text(&painter, header_start_row + idx, line, palette.fg, true);
}
```

At each **call site** (where the function is invoked), pass the header lines from the app:
```rust
// In src/native/app/terminal_screens.rs and frame_runtime.rs where these are called:
let header_lines: Vec<&str> = self.terminal_branding.header_lines
    .iter().map(|s| s.as_str()).collect();
// Pass &header_lines to the function
```

**Function signature changes** (add `header_lines: &[&str]` parameter):

1. `src/native/shell_screen.rs` — `draw_login_screen()` and `draw_shell_screen()`
   (or whatever the function names are — read the file to find them)
2. `src/native/about_screen.rs` — `draw_about_screen()`
3. `src/native/menu.rs` — the generic menu draw function
4. `src/native/document_browser.rs` — `draw_document_browser()`
5. `src/native/prompt.rs` — `draw_yes_no_prompt()` and `draw_prompt_with_input()`
6. `src/native/settings_screen.rs` — `draw_settings_screen()`

**For `src/native/app/tweaks_presenter.rs`** (the terminal tweaks screen), this is already
an `impl NucleonNativeApp` method, so it has `self` — replace `crate::config::HEADER_LINES`
with `self.terminal_branding.header_lines` directly.

### Step 5: Update call sites in frame_runtime.rs and terminal_screens.rs

These are the files that call the standalone rendering functions. Search for every
invocation of the functions modified in Step 4. At each call site:

1. Build the header lines slice:
   ```rust
   let header_lines: Vec<&str> = self.terminal_branding.header_lines
       .iter().map(|s| s.as_str()).collect();
   ```
2. Pass `&header_lines` as the new parameter.

**Critical:** There are many call sites in `terminal_screens.rs` (at least 15+) and
`frame_runtime.rs` (at least 3). To avoid repeating the `Vec<&str>` construction, build
it once at the top of the calling method and reuse it.

Alternatively, if multiple methods need it, you can add a helper:
```rust
fn header_line_refs(branding: &TerminalBranding) -> Vec<&str> {
    branding.header_lines.iter().map(|s| s.as_str()).collect()
}
```

### Step 6: Update installer_screen.rs

`src/native/installer_screen.rs` has many functions that receive `header_start_row` and
render the header. These use a different pattern — they call helper functions defined
locally that take `header_start_row`. Search for `HEADER_LINES` in installer_screen.rs.

If installer_screen.rs does NOT directly reference `HEADER_LINES`, it may delegate to one
of the functions already modified in Step 4. Read the file to confirm. If it has its own
header rendering, apply the same `header_lines: &[&str]` parameter change.

### Step 7: Remove HEADER_LINES import from modified files

After all sites use the passed-in `header_lines` parameter, remove `use crate::config::HEADER_LINES`
from the files that no longer reference the constant directly.

Do NOT remove the `HEADER_LINES` constant from `crates/shared/src/config.rs` — the TUI
path in `crates/shared/src/ui.rs` still uses it, and `TerminalBranding::heritage()` is the
new canonical source for the same strings.

### Step 8: Wire theme pack application to branding

When a theme pack is selected in the tweaks UI (both desktop egui and terminal retro
presenters), copy `theme.terminal_branding` to `self.terminal_branding`.

In `src/native/app/tweaks_presenter.rs`, find where `self.desktop_active_layout` is
updated from a theme pack (around line 2421). In the same block, add:
```rust
self.terminal_branding = theme.terminal_branding.clone();
```

Do the same in the terminal tweaks step handler if theme packs are applied there.

Also update `persist_surface_theme_state_to_settings()` (or wherever theme state is
persisted) to save/restore the branding. Add to `Settings` in `crates/shared/src/config.rs`:
```rust
pub terminal_branding: Option<TerminalBranding>,
```

Default to `None`. When `None`, use `TerminalBranding::none()`.

### Verification

- `cargo check -p nucleon`
- `cargo check -p nucleon-native-shell`
- Run app with Classic theme:
  - Terminal screens should have NO "ROBCO INDUSTRIES" header
  - All screens (login, main menu, settings, about, installer, etc.) should render correctly
  - Row content should start higher on screen (3 rows reclaimed)
  - The title and separator rows should still display correctly
- Verify no visual breakage in desktop mode
- If you manually set `TerminalBranding::heritage()` as the branding, the header should reappear
  in the old position with the old text

---

# PHASE 6 (REVISED): Tweaks UI restructure

> **This supersedes the initial Phase 6.** The previous implementation used
> ["Appearance", "Desktop", "Display", "Terminal"] tabs which mixed surface-specific
> and cross-surface concerns confusingly. This revision uses a **concern-first** model
> with Desktop/Terminal sub-tabs inside Wallpaper and Theme.

### Goal
Restructure the nucleon-tweaks app UI (both desktop egui and terminal retro-screen versions)
into four concern-oriented top-level tabs. Wallpaper and Theme tabs each contain Desktop and
Terminal sub-tabs so the user picks *what* to change first, then *which surface*. Also add
terminal wallpaper support, Full Color customization/export, per-position panel controls
(no Dock — it doesn't exist yet), and terminal screen decoration theming.

### What's already done (from initial Phase 6 — keep these)
- "Custom" naming (was "Manual") — `selected_theme_pack_name()` returns "Custom"
- Built-in ThemePacks: Classic, Nucleon Dark, Nucleon Light in `theme.rs`
- `installed_theme_packs()` uses `ThemePack::builtin_theme_packs()`
- `tweaks_tab: u8` and `tweaks_layout_overrides_open: bool` fields on `NucleonNativeApp`
- Basic tab rendering loop in `draw_tweaks()` and `terminal_tweaks_entries()`

### New UI structure

```
TWEAKS
├─ Wallpaper
│  ├─ [Desktop] [Terminal]  ← sub-tab selector
│  ├─ Desktop sub-tab:
│  │  ├─ Wallpaper Path: [path] [Browse...]
│  │  └─ Wallpaper Mode: [Default Size | Stretch | Fit | ...]
│  └─ Terminal sub-tab:
│     ├─ Wallpaper Path: [path] [Browse...]     ← NEW
│     └─ Wallpaper Mode: [Default Size | Stretch | Fit | ...]  ← NEW
│
├─ Theme
│  ├─ [Desktop] [Terminal]  ← sub-tab selector
│  ├─ Desktop sub-tab:
│  │  ├─ Theme Pack: [Classic | Nucleon Dark | Nucleon Light | ...]
│  │  │   ("Custom" shown as read-only when overridden)
│  │  ├─ Color Mode: [Monochrome | Full Color]
│  │  │   ├─ Monochrome: preset picker + custom RGB sliders
│  │  │   └─ Full Color: theme picker + [Customize...] + [Export Theme...]
│  │  ├─ Icons
│  │  │  ├─ Icon Style: [Dos | Win95 | Minimal | No Icons]
│  │  │  └─ Builtin icon toggles (per-icon show/hide)
│  │  ├─ Cursors
│  │  │  ├─ Show Cursor: [ON | OFF]
│  │  │  └─ Cursor Scale slider
│  │  └─ [+] Layout Overrides (collapsible, collapsed by default)
│  │     ├─ Top: [Menu Bar | Taskbar | Disabled]
│  │     ├─ Bottom: [Taskbar | Menu Bar | Disabled]
│  │     ├─ Launcher: [Start Menu | Overlay | Hidden]
│  │     └─ Window Headers: [Standard | Compact | Hidden]
│  └─ Terminal sub-tab:
│     ├─ Theme Pack: [Classic | ...] (independent of desktop theme)
│     ├─ Color Mode: [Monochrome | Full Color]
│     │   └─ (same sub-controls as Desktop)
│     └─ Terminal Layout: [Classic Terminal | Minimal Terminal]
│
├─ Effects
│  ├─ CRT Effects: [ON | OFF]
│  ├─ CRT Preset: [Classic | Retro | ...]
│  └─ CRT sliders (curvature, scanlines, glow, bloom, vignette, noise,
│     flicker, jitter, burn-in, glow-line, phosphor softness, brightness, contrast)
│
└─ Display
   ├─ Window Mode: [Windowed | Maximized | Fullscreen]
   └─ PTY Rendering
      ├─ Styled PTY: [ON | OFF]
      ├─ PTY Color Mode: [...]
      └─ PTY Border Glyphs: [...]
```

### Step 1: Data model — terminal wallpaper and panel type

**Terminal wallpaper fields** — in `crates/shared/src/config.rs`, add to the settings struct
next to the existing `desktop_wallpaper` fields:

```rust
#[serde(default)]
pub terminal_wallpaper: String,         // empty string = no wallpaper
#[serde(default)]
pub terminal_wallpaper_size_mode: WallpaperSizeMode,
```

Initialize both in the `Default` impl: `terminal_wallpaper: String::new()`,
`terminal_wallpaper_size_mode: WallpaperSizeMode::FitToScreen`.

**Replace DockPosition with per-position PanelType** — in `crates/shared/src/theme.rs`:

Remove the `PanelPosition` and `DockPosition` enums entirely. Replace with:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PanelType {
    MenuBar,
    Taskbar,
    Disabled,
}
```

Update `LayoutProfile` to use position-based fields:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutProfile {
    pub id: String,
    pub name: String,
    pub top_panel: PanelType,            // was: panel_position
    pub top_panel_height: f32,           // was: panel_height
    pub bottom_panel: PanelType,         // was: dock_position (conceptually)
    pub bottom_panel_height: f32,        // was: dock_size
    pub launcher_style: LauncherStyle,
    pub window_header_style: WindowHeaderStyle,
}
```

Update `LayoutProfile::classic()`:
```rust
pub fn classic() -> Self {
    LayoutProfile {
        id: "classic".to_string(),
        name: "Classic".to_string(),
        top_panel: PanelType::MenuBar,
        top_panel_height: 30.0,
        bottom_panel: PanelType::Taskbar,
        bottom_panel_height: 32.0,
        launcher_style: LauncherStyle::StartMenu,
        window_header_style: WindowHeaderStyle::Standard,
    }
}
```

Update `LayoutProfile::minimal()`:
```rust
pub fn minimal() -> Self {
    LayoutProfile {
        id: "minimal".to_string(),
        name: "Minimal".to_string(),
        top_panel: PanelType::Disabled,
        top_panel_height: 30.0,
        bottom_panel: PanelType::Taskbar,
        bottom_panel_height: 26.0,
        launcher_style: LauncherStyle::StartMenu,
        window_header_style: WindowHeaderStyle::Standard,
    }
}
```

**Cascade the LayoutProfile changes** — search for all usages of `panel_position`,
`panel_height`, `dock_position`, `dock_size`, `PanelPosition`, `DockPosition` across
the codebase and update them:

- `src/native/app.rs` — `desktop_active_layout` field usage
- `src/native/app/frame_runtime.rs` — top bar and taskbar rendering decisions.
  Replace `layout.panel_position != PanelPosition::Hidden` with
  `layout.top_panel != PanelType::Disabled`. Replace `layout.dock_position != DockPosition::Hidden`
  with `layout.bottom_panel != PanelType::Disabled`.
- `src/native/app/desktop_window_mgmt.rs` — `active_desktop_workspace_rect()` reads
  panel/dock heights. Update to read `top_panel_height` / `bottom_panel_height` and
  check `top_panel != PanelType::Disabled` / `bottom_panel != PanelType::Disabled`.
- `src/native/app/tweaks_presenter.rs` — layout override controls (Step 3 rewrites these)
- Any other import sites for the removed enums

### Step 2: App state changes

In `src/native/app.rs`, update the fields on `NucleonNativeApp`:

```rust
// Replace:
//   tweaks_tab: u8,  // 0=Appearance, 1=Desktop, 2=Display, 3=Terminal
// With:
tweaks_tab: u8,                    // 0=Wallpaper, 1=Theme, 2=Effects, 3=Display
tweaks_wallpaper_surface: u8,      // 0=Desktop, 1=Terminal (sub-tab within Wallpaper)
tweaks_theme_surface: u8,          // 0=Desktop, 1=Terminal (sub-tab within Theme)
tweaks_layout_overrides_open: bool, // already exists
```

Initialize `tweaks_wallpaper_surface: 0` and `tweaks_theme_surface: 0` in Default.

Also add fields for Full Color customization (Step 6):
```rust
pub(super) desktop_color_overrides: Option<std::collections::HashMap<crate::theme::ColorToken, [u8; 4]>>,
pub(super) terminal_color_overrides: Option<std::collections::HashMap<crate::theme::ColorToken, [u8; 4]>>,
pub(super) tweaks_customize_colors_open: bool,
```

Initialize all to `None`/`false`.

### Step 3: Restructure desktop egui tweaks tabs

In `draw_tweaks()` in `src/native/app/tweaks_presenter.rs`:

**Change tab labels** from `["Appearance", "Desktop", "Display", "Terminal"]` to
`["Wallpaper", "Theme", "Effects", "Display"]`.

The existing tab rendering loop stays the same shape — just change the label array and
the match arms.

**Helper: sub-tab selector** — add a reusable helper to render the Desktop/Terminal
sub-tab buttons:

```rust
fn draw_surface_sub_tabs(ui: &mut egui::Ui, active_surface: &mut u8, palette: &RetroPalette) {
    ui.horizontal(|ui| {
        for (i, label) in ["Desktop", "Terminal"].iter().enumerate() {
            let active = *active_surface == i as u8;
            let btn = ui.add(
                egui::Button::new(
                    RichText::new(*label)
                        .color(if active { palette.selected_fg } else { palette.fg })
                        .size(13.0),
                )
                .stroke(egui::Stroke::new(
                    if active { 1.5 } else { 0.5 },
                    palette.dim,
                ))
                .fill(if active { palette.selected_bg } else { palette.panel }),
            );
            if btn.clicked() {
                *active_surface = i as u8;
            }
        }
    });
    ui.add_space(6.0);
}
```

**Tab 0: Wallpaper** — render the surface sub-tabs, then show the appropriate wallpaper
controls:

```rust
0 => {
    Self::draw_surface_sub_tabs(ui, &mut self.tweaks_wallpaper_surface, &palette);
    match self.tweaks_wallpaper_surface {
        0 => {
            // Desktop wallpaper — MOVE existing wallpaper controls here
            // (wallpaper path, browse button, size mode combo)
            // These were in the old "Desktop" tab (index 1)
            Self::settings_section(ui, "Desktop Wallpaper", |ui| {
                // ... existing desktop wallpaper controls ...
            });
        }
        _ => {
            // Terminal wallpaper — NEW
            Self::settings_section(ui, "Terminal Wallpaper", |ui| {
                ui.label("Wallpaper Path");
                ui.horizontal(|ui| {
                    let w = ui.available_width() - 80.0;
                    let display = if self.settings.draft.terminal_wallpaper.is_empty() {
                        "None".to_string()
                    } else {
                        Path::new(&self.settings.draft.terminal_wallpaper)
                            .file_name()
                            .map(|n| n.to_string_lossy().to_string())
                            .unwrap_or_else(|| self.settings.draft.terminal_wallpaper.clone())
                    };
                    ui.add_sized([w, 22.0], egui::Label::new(
                        RichText::new(&display).color(palette.fg)
                    ));
                    if ui.button("Browse...").clicked() {
                        // Reuse the same file dialog pattern as desktop wallpaper
                        // Open native file dialog, set self.settings.draft.terminal_wallpaper
                    }
                });
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.label("Wallpaper Mode");
                    // ComboBox with WallpaperSizeMode options
                    // Same pattern as desktop wallpaper mode
                });
            });
        }
    }
}
```

**Tab 1: Theme** — render the surface sub-tabs, then show theme controls for that surface:

```rust
1 => {
    Self::draw_surface_sub_tabs(ui, &mut self.tweaks_theme_surface, &palette);
    match self.tweaks_theme_surface {
        0 => {
            // Desktop theme controls:
            // 1. Theme Pack combo — MOVE from old Appearance tab
            // 2. Color Mode selector — MOVE from old Appearance tab
            // 3. Monochrome/Full Color sub-controls — MOVE from old Appearance tab
            // 4. Customize Colors (Step 6) — Full Color only, collapsible
            // 5. Export Theme (Step 7) — visible when overrides exist
            // 6. Icons section — MOVE from old Desktop tab
            //    - Icon Style combo
            //    - Builtin icon toggles
            // 7. Cursors section — MOVE from old Desktop tab
            //    - Show Cursor toggle
            //    - Cursor Scale slider
            // 8. Layout Overrides (collapsible) — RESTRUCTURED from old Appearance tab
            //    See Step 4 for the new per-position controls
        }
        _ => {
            // Terminal theme controls:
            // 1. Terminal Theme Pack combo — MOVE from old Terminal tab
            // 2. Terminal Color Mode selector — MOVE from old Terminal tab
            // 3. Terminal Monochrome/Full Color sub-controls — MOVE from old Terminal tab
            // 4. Terminal Layout profile combo — MOVE from old Terminal tab
        }
    }
}
```

**Tab 2: Effects**:

```rust
2 => {
    // CRT Effects section — MOVE from old Display tab
    // All CRT controls: enable toggle, preset, curvature, scanlines, glow,
    // bloom, vignette, noise, flicker, jitter, burn-in, glow-line,
    // glow-line speed, phosphor softness, brightness, contrast
    // No sub-tabs — CRT is a global rendering concern
}
```

**Tab 3: Display**:

```rust
3 => {
    // Window Mode — MOVE from old Display tab
    Self::settings_section(ui, "Window Mode", |ui| {
        // ... existing window mode combo ...
    });
    ui.add_space(10.0);
    // PTY Rendering — MOVE from old Terminal tab
    Self::settings_section(ui, "PTY Rendering", |ui| {
        // Styled PTY toggle
        // PTY Color Mode
        // PTY Border Glyphs
    });
}
```

### Step 4: Layout override controls (per-position panel type)

Replace the old layout override controls (which referenced `DockPosition`) with
per-position `PanelType` dropdowns. These go inside Theme > Desktop > Layout Overrides.

```rust
if self.tweaks_layout_overrides_open {
    ui.horizontal(|ui| {
        ui.label("Top:");
        egui::ComboBox::from_id_salt("layout_top_panel")
            .selected_text(panel_type_label(self.desktop_active_layout.top_panel))
            .show_ui(ui, |ui| {
                for pt in &[PanelType::MenuBar, PanelType::Taskbar, PanelType::Disabled] {
                    if ui.selectable_label(
                        self.desktop_active_layout.top_panel == *pt,
                        panel_type_label(*pt),
                    ).clicked() {
                        self.desktop_active_layout.top_panel = *pt;
                        self.desktop_active_theme_pack_id = None;
                        desktop_runtime_changed = true;
                    }
                }
            });
    });
    ui.horizontal(|ui| {
        ui.label("Bottom:");
        egui::ComboBox::from_id_salt("layout_bottom_panel")
            .selected_text(panel_type_label(self.desktop_active_layout.bottom_panel))
            .show_ui(ui, |ui| {
                for pt in &[PanelType::Taskbar, PanelType::MenuBar, PanelType::Disabled] {
                    if ui.selectable_label(
                        self.desktop_active_layout.bottom_panel == *pt,
                        panel_type_label(*pt),
                    ).clicked() {
                        self.desktop_active_layout.bottom_panel = *pt;
                        self.desktop_active_theme_pack_id = None;
                        desktop_runtime_changed = true;
                    }
                }
            });
    });
    ui.horizontal(|ui| {
        ui.label("Launcher:");
        egui::ComboBox::from_id_salt("layout_launcher")
            .selected_text(launcher_style_label(self.desktop_active_layout.launcher_style))
            .show_ui(ui, |ui| {
                for ls in &[LauncherStyle::StartMenu, LauncherStyle::Overlay, LauncherStyle::Hidden] {
                    if ui.selectable_label(
                        self.desktop_active_layout.launcher_style == *ls,
                        launcher_style_label(*ls),
                    ).clicked() {
                        self.desktop_active_layout.launcher_style = *ls;
                        self.desktop_active_theme_pack_id = None;
                        desktop_runtime_changed = true;
                    }
                }
            });
    });
    ui.horizontal(|ui| {
        ui.label("Window Headers:");
        egui::ComboBox::from_id_salt("layout_window_header")
            .selected_text(window_header_label(self.desktop_active_layout.window_header_style))
            .show_ui(ui, |ui| {
                for wh in &[WindowHeaderStyle::Standard, WindowHeaderStyle::Compact, WindowHeaderStyle::Hidden] {
                    if ui.selectable_label(
                        self.desktop_active_layout.window_header_style == *wh,
                        window_header_label(*wh),
                    ).clicked() {
                        self.desktop_active_layout.window_header_style = *wh;
                        self.desktop_active_theme_pack_id = None;
                        desktop_runtime_changed = true;
                    }
                }
            });
    });
}
```

Add label helpers at the top of `tweaks_presenter.rs`:

```rust
fn panel_type_label(pt: PanelType) -> &'static str {
    match pt {
        PanelType::MenuBar => "Menu Bar",
        PanelType::Taskbar => "Taskbar",
        PanelType::Disabled => "Disabled",
    }
}

fn launcher_style_label(ls: LauncherStyle) -> &'static str {
    match ls {
        LauncherStyle::StartMenu => "Start Menu",
        LauncherStyle::Overlay => "Overlay",
        LauncherStyle::Hidden => "Hidden",
    }
}

fn window_header_label(wh: WindowHeaderStyle) -> &'static str {
    match wh {
        WindowHeaderStyle::Standard => "Standard",
        WindowHeaderStyle::Compact => "Compact",
        WindowHeaderStyle::Hidden => "Hidden",
    }
}
```

### Step 5: Restructure terminal retro-screen tweaks

The terminal retro-screen tweaks in `terminal_tweaks_entries()` currently uses flat headers
`["Appearance", "Desktop", "Display", "Terminal"]`. Change to match the new structure:

```rust
fn terminal_tweaks_entries(&self, theme_packs: &[ThemePack]) -> Vec<TerminalTweaksEntry> {
    let mut entries = vec![];

    // --- Wallpaper ---
    entries.push(TerminalTweaksEntry::Header("Wallpaper"));
    entries.push(TerminalTweaksEntry::Header("  Desktop"));
    entries.extend([
        TerminalTweaksEntry::Row(TerminalTweaksRow::WallpaperPicker),
        TerminalTweaksEntry::Row(TerminalTweaksRow::WallpaperMode),
    ]);
    entries.push(TerminalTweaksEntry::Header("  Terminal"));
    entries.extend([
        TerminalTweaksEntry::Row(TerminalTweaksRow::TerminalWallpaperPicker),  // NEW
        TerminalTweaksEntry::Row(TerminalTweaksRow::TerminalWallpaperMode),    // NEW
    ]);

    // --- Theme ---
    entries.push(TerminalTweaksEntry::Header("Theme"));
    entries.push(TerminalTweaksEntry::Header("  Desktop"));
    entries.extend([
        TerminalTweaksEntry::Row(TerminalTweaksRow::DesktopThemePack),
        TerminalTweaksEntry::Row(TerminalTweaksRow::DesktopColorMode),
    ]);
    // ... monochrome/full-color sub-rows (same logic as current) ...
    entries.extend([
        TerminalTweaksEntry::Row(TerminalTweaksRow::DesktopIconStyle),
    ]);
    for (index, _) in desktop_builtin_icons().iter().enumerate() {
        entries.push(TerminalTweaksEntry::Row(TerminalTweaksRow::DesktopBuiltinIcon(index)));
    }
    entries.push(TerminalTweaksEntry::Row(TerminalTweaksRow::DesktopShowCursor));
    if self.settings.draft.desktop_show_cursor {
        entries.push(TerminalTweaksEntry::Row(TerminalTweaksRow::DesktopCursorScale));
    }
    entries.push(TerminalTweaksEntry::Header("  Terminal"));
    entries.extend([
        TerminalTweaksEntry::Row(TerminalTweaksRow::TerminalThemePack),
        TerminalTweaksEntry::Row(TerminalTweaksRow::TerminalColorMode),
    ]);
    // ... terminal monochrome/full-color sub-rows ...
    entries.push(TerminalTweaksEntry::Row(TerminalTweaksRow::TerminalLayout));

    // --- Effects ---
    entries.push(TerminalTweaksEntry::Header("Effects"));
    entries.extend([
        TerminalTweaksEntry::Row(TerminalTweaksRow::CrtEnabled),
        TerminalTweaksEntry::Row(TerminalTweaksRow::CrtPreset),
        // ... all CRT slider rows ...
    ]);

    // --- Display ---
    entries.push(TerminalTweaksEntry::Header("Display"));
    entries.extend([
        TerminalTweaksEntry::Row(TerminalTweaksRow::WindowMode),
        TerminalTweaksEntry::Row(TerminalTweaksRow::TerminalStyledPty),
        TerminalTweaksEntry::Row(TerminalTweaksRow::TerminalPtyColorMode),
        TerminalTweaksEntry::Row(TerminalTweaksRow::TerminalBorderGlyphs),
    ]);

    self.inflate_terminal_tweaks_dropdown_entries(entries, theme_packs)
}
```

Add `TerminalTweaksRow::TerminalWallpaperPicker` and `TerminalTweaksRow::TerminalWallpaperMode`
to the enum. Implement their display text and step handlers following the same pattern as
`WallpaperPicker` / `WallpaperMode` but reading/writing `terminal_wallpaper` and
`terminal_wallpaper_size_mode` from `self.settings.draft`.

Note: Headers with leading spaces (e.g., `"  Desktop"`) render as indented sub-headers
in the terminal retro-screen. The existing header rendering in the terminal tweaks screen
already uses `screen.text()` with the header text — the leading spaces will indent naturally.

### Step 6: Full Color theme customization (Customize button)

**Not yet implemented.** This step adds per-token color editing for Full Color themes.

When the user is in Full Color mode under Theme > Desktop (or Theme > Terminal), show a
"Customize..." button below the theme picker. Clicking it expands a collapsible section
showing every `ColorToken` as a labeled color swatch + RGB editor.

**Data model** — already added in Step 2 (`desktop_color_overrides`,
`terminal_color_overrides`, `tweaks_customize_colors_open`).

**How overrides work:**
- When `desktop_color_overrides` is `Some`, the overrides are merged on top of the base
  `FullColorTheme` tokens before building the palette.
- Any override causes `desktop_active_theme_pack_id` to become `None` (shows "Custom").
- Selecting a different theme or theme pack clears the overrides back to `None`.

**Palette resolution with overrides** — in `retro_ui.rs`, add a new function:
```rust
pub fn palette_for_color_style_with_overrides(
    style: &ColorStyle,
    overrides: Option<&std::collections::HashMap<ColorToken, [u8; 4]>>,
) -> RetroPalette {
    match style {
        ColorStyle::FullColor { theme_id } => {
            let mut theme = FullColorTheme::builtin_by_id(theme_id)
                .unwrap_or_else(FullColorTheme::nucleon_dark);
            if let Some(overrides) = overrides {
                for (token, color) in overrides {
                    theme.tokens.insert(*token, *color);
                }
            }
            palette_from_full_color_theme(&theme)
        }
        ColorStyle::Monochrome { preset, custom_rgb } => {
            let color = monochrome_preset_to_color(*preset, *custom_rgb);
            palette_for_theme_color(color)
        }
    }
}
```

Wire this into `set_active_color_style()` — the desktop and terminal paths should pass
their respective overrides.

**UI** — inside the Theme > Desktop sub-tab (after the Full Color theme picker):

Use a small current-color box plus a compact Paint-style preset palette instead of raw RGB
editors or egui's popup HSV picker. Each `ColorToken` row should show:
- the token label
- a preview swatch for the current color
- no inline wall of tiles by default

Clicking the preview swatch opens a small chooser under that row with a larger preset tile
palette covering more hues. Clicking a tile applies that color immediately. Do not use a
hue square, value slider, hex input, or any transient HSV editor here.

**Note:** The monochrome Custom RGB sliders stay as DragValues. The tile swatch grid is
only for Full Color per-token customization.

```rust
if !desktop_is_monochrome {
    ui.add_space(6.0);
    let customize_label = if self.tweaks_customize_colors_open {
        "[-] Customize Colors"
    } else {
        "[+] Customize Colors"
    };
    if ui.button(customize_label).clicked() {
        self.tweaks_customize_colors_open = !self.tweaks_customize_colors_open;
    }
    if self.tweaks_customize_colors_open {
        let base_theme_id = full_color_theme_id_for_color_style(
            &self.desktop_active_color_style,
        );
        let base_theme = FullColorTheme::builtin_by_id(base_theme_id)
            .unwrap_or_else(FullColorTheme::nucleon_dark);
        let overrides = self.desktop_color_overrides
            .get_or_insert_with(|| base_theme.tokens.clone());

        for token in ColorToken::all() {
            let entry = overrides.entry(token).or_insert([128, 128, 128, 255]);
            ui.horizontal(|ui| {
                ui.label(format!("{:?}", token));
                let preview_response = preview_swatch_button(ui, *entry);
                if preview_response.clicked() {
                    // Toggle compact preset palette for this token
                }
                // Popup/anchored panel under this row only:
                // larger preset palette with more hues
            });
        }
    }
}
```

Add `ColorToken::all()` in `crates/shared/src/theme.rs`:
```rust
impl ColorToken {
    pub fn all() -> &'static [ColorToken] {
        &[
            ColorToken::BgPrimary, ColorToken::BgSecondary,
            ColorToken::FgPrimary, ColorToken::FgSecondary, ColorToken::FgDim,
            ColorToken::Accent, ColorToken::AccentHover, ColorToken::AccentActive,
            ColorToken::PanelBg, ColorToken::PanelBorder,
            ColorToken::WindowChrome, ColorToken::WindowChromeFocused,
            ColorToken::Selection, ColorToken::SelectionFg,
            ColorToken::Border, ColorToken::Separator,
            ColorToken::StatusBar,
            ColorToken::Error, ColorToken::Warning, ColorToken::Success,
        ]
    }
}
```

**For terminal retro-screen tweaks:** Show a read-only message instead of the customize UI:
`"Use Desktop Tweaks window for color customization"`. The customize/export controls are
desktop egui only.

### Step 7: Export Theme button

Below the Customize section (visible when `desktop_color_overrides.is_some()`), show an
"Export Theme..." button.

**What Export does:**
1. Builds a `FullColorTheme` from the current base theme + overrides.
2. Serializes it to pretty-printed JSON.
3. Writes to `<nucleon_data_dir>/exported_themes/<theme_name>.json`.
4. Shows a status message: "Theme exported to <path>".

```rust
if self.desktop_color_overrides.is_some() {
    ui.add_space(8.0);
    if ui.button("Export Theme...").clicked() {
        let base_id = full_color_theme_id_for_color_style(
            &self.desktop_active_color_style,
        );
        let mut theme = FullColorTheme::builtin_by_id(base_id)
            .unwrap_or_else(FullColorTheme::nucleon_dark);
        if let Some(overrides) = &self.desktop_color_overrides {
            for (token, color) in overrides {
                theme.tokens.insert(*token, *color);
            }
        }
        theme.id = format!("{}-custom", base_id);
        theme.name = format!("{} (Custom)", theme.name);
        let export_dir = crate::config::nucleon_data_dir().join("exported_themes");
        let _ = std::fs::create_dir_all(&export_dir);
        let path = export_dir.join(format!("{}.json", theme.id));
        match serde_json::to_string_pretty(&theme) {
            Ok(json) => {
                let _ = std::fs::write(&path, json);
                saved_settings_status(
                    &format!("Theme exported to {}", path.display()),
                );
            }
            Err(_) => {
                saved_settings_status("Failed to export theme");
            }
        }
    }
}
```

The exported JSON can later be placed inside a `.ndpkg` theme bundle's `color_theme.json`.

### Step 8: Terminal screen decoration theming

**Not yet implemented.** Every terminal screen renders the same decoration pattern:

```
[header lines]           ← themed via TerminalBranding (Phase 5)
==================       ← separator (top)
     Screen Title        ← title (centered, bold)
==================       ← separator (bottom)
  subtitle text          ← subtitle (left-aligned, sometimes underlined)
  menu content...
  status text            ← status row
```

Currently separator character (`=`), alignment (centered), title style (bold), and subtitle
style (underlined) are hardcoded. This step makes them theme-configurable.

**Data model** — in `crates/shared/src/theme.rs`, add:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TextAlignment {
    Left,
    Center,
    Right,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminalDecoration {
    pub separator_char: String,
    pub separator_alignment: TextAlignment,
    pub title_alignment: TextAlignment,
    pub title_bold: bool,
    pub subtitle_alignment: TextAlignment,
    pub subtitle_underlined: bool,
    pub show_separators: bool,
}

impl Default for TerminalDecoration {
    fn default() -> Self {
        TerminalDecoration {
            separator_char: "=".to_string(),
            separator_alignment: TextAlignment::Center,
            title_alignment: TextAlignment::Center,
            title_bold: true,
            subtitle_alignment: TextAlignment::Left,
            subtitle_underlined: true,
            show_separators: true,
        }
    }
}
```

Add to `ThemePack`:
```rust
pub terminal_decoration: TerminalDecoration,
```

Update all `ThemePack` constructors to include `terminal_decoration: TerminalDecoration::default()`.

**Store active decoration in NucleonNativeApp** — add:
```rust
pub(super) terminal_decoration: TerminalDecoration,
```

Initialize from active theme pack. When a theme pack is applied, copy its `terminal_decoration`.

**Add decoration-aware rendering helpers** — in `src/native/retro_ui.rs`, add methods
to `RetroScreen`:

```rust
pub fn themed_separator(
    &self,
    painter: &Painter,
    row: usize,
    palette: &RetroPalette,
    decoration: &TerminalDecoration,
) {
    if !decoration.show_separators { return; }
    let char_count = self.cols.saturating_sub(6).max(1);
    let text = decoration.separator_char.repeat(
        char_count / decoration.separator_char.len().max(1)
    );
    match decoration.separator_alignment {
        TextAlignment::Center => self.centered_text(painter, row, &text, palette.dim, false),
        TextAlignment::Left => self.text(painter, 3, row, &text, palette.dim),
        TextAlignment::Right => {
            let start_col = self.cols.saturating_sub(text.len() + 3);
            self.text(painter, start_col, row, &text, palette.dim);
        }
    }
}

pub fn themed_title(
    &self, painter: &Painter, row: usize, title: &str,
    palette: &RetroPalette, decoration: &TerminalDecoration,
) {
    match decoration.title_alignment {
        TextAlignment::Center => self.centered_text(painter, row, title, palette.fg, decoration.title_bold),
        TextAlignment::Left => self.text(painter, 3, row, title, palette.fg),
        TextAlignment::Right => {
            let start_col = self.cols.saturating_sub(title.len() + 3);
            self.text(painter, start_col, row, title, palette.fg);
        }
    }
}

pub fn themed_subtitle(
    &self, painter: &Painter, col: usize, row: usize, subtitle: &str,
    palette: &RetroPalette, decoration: &TerminalDecoration,
) {
    if decoration.subtitle_underlined {
        self.underlined_text(painter, col, row, subtitle, palette.fg);
    } else {
        self.text(painter, col, row, subtitle, palette.fg);
    }
}
```

**Update all terminal screen rendering functions** — add `decoration: &TerminalDecoration`
parameter and replace hardcoded calls:

```rust
// BEFORE:
screen.separator(&painter, separator_top_row, &palette);
screen.centered_text(&painter, title_row, "About", palette.fg, true);
screen.separator(&painter, separator_bottom_row, &palette);
screen.underlined_text(&painter, content_col, subtitle_row, "...", palette.fg);

// AFTER:
screen.themed_separator(&painter, separator_top_row, &palette, &decoration);
screen.themed_title(&painter, title_row, "About", &palette, &decoration);
screen.themed_separator(&painter, separator_bottom_row, &palette, &decoration);
screen.themed_subtitle(&painter, content_col, subtitle_row, "...", &palette, &decoration);
```

Pass `&self.terminal_decoration` from call sites in `terminal_screens.rs` and
`frame_runtime.rs`.

**Files to modify** (same list as Phase 5 header changes, plus `retro_ui.rs`):
- `src/native/retro_ui.rs` — add the three `themed_*` methods
- `src/native/menu.rs` — `draw_terminal_menu_screen()`
- `src/native/shell_screen.rs` — both draw functions
- `src/native/about_screen.rs`
- `src/native/document_browser.rs`
- `src/native/prompt.rs` — both prompt draw functions
- `src/native/settings_screen.rs`
- `src/native/installer_screen.rs`
- `src/native/app/tweaks_presenter.rs` — terminal tweaks screen
- `src/native/app/terminal_screens.rs` — all call sites
- `src/native/app/frame_runtime.rs` — all call sites
- `src/native/connections_screen.rs`
- `src/native/default_apps_screen.rs`
- `src/native/edit_menus_screen.rs`
- `src/native/programs_screen.rs`

The existing `separator()` method on `RetroScreen` must NOT be removed — it's the fallback
for code without access to a `TerminalDecoration`.

### Step 9: Wire terminal wallpaper rendering

The terminal wallpaper is rendered as a background image behind the `RetroScreen` character
grid. The implementation follows the same pattern as desktop wallpaper rendering in
`desktop_surface.rs`.

In the terminal rendering path (where `CentralPanel` is created with
`current_palette_for_surface(ShellSurfaceKind::Terminal).bg` as fill), add wallpaper
rendering before the RetroScreen character grid:

1. If `self.settings.draft.terminal_wallpaper` is non-empty, load and cache the wallpaper
   texture (reuse the same texture caching pattern as desktop wallpaper).
2. Paint the wallpaper image behind the character grid using the configured size mode.
3. When a terminal wallpaper is active, the `RetroScreen::paint_bg()` call should use a
   semi-transparent background (e.g., `Color32::from_black_alpha(180)`) instead of the
   opaque palette background, so the wallpaper shows through.

**Important:** This is a visual-only feature. The RetroScreen character positioning and
input handling remain unchanged. The wallpaper is purely decorative.

Add a `terminal_wallpaper_texture: Option<TextureHandle>` field to `NucleonNativeApp` for
caching. Invalidate when `terminal_wallpaper` path changes.

### Verification

- `cargo check -p nucleon`
- `cargo check -p nucleon-native-shell`
- Run app:
  - Tweaks window shows 4 tabs: **Wallpaper**, **Theme**, **Effects**, **Display**
  - Wallpaper tab has Desktop/Terminal sub-tabs
  - Theme tab has Desktop/Terminal sub-tabs
  - Effects tab shows CRT controls directly (no sub-tabs)
  - Display tab shows Window Mode + PTY settings
  - Desktop wallpaper controls are under Wallpaper > Desktop
  - Terminal wallpaper controls are under Wallpaper > Terminal (new feature)
  - Theme Pack, Color Mode, Icons, Cursors are under Theme > Desktop
  - Terminal Theme Pack, Color Mode, Layout are under Theme > Terminal
  - Layout Overrides show per-position PanelType dropdowns (Top, Bottom)
  - No "Dock" anywhere in the UI
  - In Full Color mode, "Customize Colors" expander shows all 20 tokens with a small current-color box
  - Clicking a token's color box opens a compact preset palette with more hues
  - "Export Theme..." writes JSON to `exported_themes/` directory
  - Selecting a different base theme clears overrides
  - Terminal retro-screen tweaks sections: Wallpaper (Desktop/Terminal), Theme (Desktop/Terminal), Effects, Display
  - All terminal screens render decorations identically to before (default TerminalDecoration
    matches the hardcoded values)

---

# PHASE 6 POLISH: Post-implementation fixes

> These fixes address bugs and UX issues found after the Phase 6 (REVISED) implementation.
> Complete all fixes before proceeding to Phase 7.

### Fix 1: Panel ID collision when same PanelType is used for top and bottom

**Problem:** When both `layout.top_panel` and `layout.bottom_panel` are set to the same
`PanelType` (e.g., both `MenuBar`), egui reports "first use / second use of widget" errors
because the panel ID strings are hardcoded without position differentiation.

In `src/native/app/desktop_menu_bar.rs` (around line 295), `draw_top_bar()` uses:
```rust
let panel = if top {
    TopBottomPanel::top("native_top_bar")
} else {
    TopBottomPanel::bottom("native_top_bar")  // same ID!
};
```

In `src/native/app/desktop_taskbar.rs` (around line 52), `draw_desktop_taskbar()` uses:
```rust
let panel = if top {
    TopBottomPanel::top("native_desktop_taskbar")
} else {
    TopBottomPanel::bottom("native_desktop_taskbar")  // same ID!
};
```

When both panels are MenuBar, `draw_top_bar()` is called twice with the same ID
`"native_top_bar"` — once as `TopBottomPanel::top()` and once as
`TopBottomPanel::bottom()`. egui sees this as a duplicate widget.

**Fix:** Make panel IDs position-aware. Change `draw_top_bar()`:
```rust
let panel_id = if top { "native_top_bar_top" } else { "native_top_bar_bottom" };
let panel = if top {
    TopBottomPanel::top(panel_id)
} else {
    TopBottomPanel::bottom(panel_id)
};
```

Change `draw_desktop_taskbar()`:
```rust
let panel_id = if top { "native_taskbar_top" } else { "native_taskbar_bottom" };
let panel = if top {
    TopBottomPanel::top(panel_id)
} else {
    TopBottomPanel::bottom(panel_id)
};
```

This allows the same component type to render at both positions without ID conflict.

### Fix 2: Desktop/Terminal sub-tab buttons too small

**Problem:** The Desktop/Terminal sub-tab buttons in `draw_surface_sub_tabs()` use
`RichText::new(*label).size(13.0)` without `.strong()`, making them visually too small
compared to the main tabs which use `.strong()` at default size.

**Fix:** In `draw_surface_sub_tabs()` (around line 430 of `tweaks_presenter.rs`), change:
```rust
// BEFORE:
RichText::new(*label)
    .color(if active { palette.selected_fg } else { palette.fg })
    .size(13.0),

// AFTER:
RichText::new(*label)
    .color(if active { palette.selected_fg } else { palette.fg })
    .strong(),
```

Remove the `.size(13.0)` and add `.strong()` to match the visual weight of the main tabs.
The sub-tabs should be the same height as the main tabs — the label text distinguishes them.

Also increase post-tab spacing from `6.0` to `10.0`:
```rust
// BEFORE:
ui.add_space(6.0);
// AFTER:
ui.add_space(10.0);
```

### Fix 3: Terminal retro-screen tweaks needs collapsible sections

**Problem:** The terminal retro-screen tweaks shows ALL sections expanded at once with
flat indented headers. This is overwhelming — all CRT sliders, all icon toggles, all
wallpaper and theme controls visible simultaneously in a single scrollable list.

**Fix:** Use collapsible top-level sections. Only one section is expanded at a time.
The user navigates by selecting a section header, which collapses the previous section
and expands the selected one.

Add a field to `NucleonNativeApp`:
```rust
pub(super) terminal_tweaks_active_section: u8,  // 0=Wallpaper, 1=Theme, 2=Effects, 3=Display
```
Initialize to `1` (Theme, since that's the most common reason to open Tweaks).

In `terminal_tweaks_entries()`, change the structure so that only the active section's
rows are included:

```rust
fn terminal_tweaks_entries(&self, theme_packs: &[ThemePack]) -> Vec<TerminalTweaksEntry> {
    let mut entries = vec![];

    // Section headers are always shown — they act as selectable menu items
    entries.push(TerminalTweaksEntry::Row(TerminalTweaksRow::SectionSelector(0)));  // Wallpaper
    if self.terminal_tweaks_active_section == 0 {
        entries.push(TerminalTweaksEntry::Header("  Desktop"));
        entries.extend([
            TerminalTweaksEntry::Row(TerminalTweaksRow::WallpaperPicker),
            TerminalTweaksEntry::Row(TerminalTweaksRow::WallpaperMode),
        ]);
        entries.push(TerminalTweaksEntry::Header("  Terminal"));
        entries.extend([
            TerminalTweaksEntry::Row(TerminalTweaksRow::TerminalWallpaperPicker),
            TerminalTweaksEntry::Row(TerminalTweaksRow::TerminalWallpaperMode),
        ]);
    }

    entries.push(TerminalTweaksEntry::Row(TerminalTweaksRow::SectionSelector(1)));  // Theme
    if self.terminal_tweaks_active_section == 1 {
        // ... Desktop and Terminal theme rows ...
    }

    entries.push(TerminalTweaksEntry::Row(TerminalTweaksRow::SectionSelector(2)));  // Effects
    if self.terminal_tweaks_active_section == 2 {
        // ... CRT rows ...
    }

    entries.push(TerminalTweaksEntry::Row(TerminalTweaksRow::SectionSelector(3)));  // Display
    if self.terminal_tweaks_active_section == 3 {
        // ... Window mode + PTY rows ...
    }

    self.inflate_terminal_tweaks_dropdown_entries(entries, theme_packs)
}
```

Add `TerminalTweaksRow::SectionSelector(u8)` to the enum. Its display text shows the
section name with an expand/collapse indicator:

```rust
TerminalTweaksRow::SectionSelector(idx) => {
    let name = match idx {
        0 => "Wallpaper",
        1 => "Theme",
        2 => "Effects",
        _ => "Display",
    };
    let marker = if self.terminal_tweaks_active_section == *idx { "[-]" } else { "[+]" };
    format!("{marker} {name}")
}
```

When a `SectionSelector` row is activated (Enter/Space), toggle the section:
```rust
TerminalTweaksRow::SectionSelector(idx) => {
    if self.terminal_tweaks_active_section == idx {
        // Already open — could close it, but for simplicity keep it open
        // (at least one section should always be visible)
    } else {
        self.terminal_tweaks_active_section = idx;
    }
}
```

### Fix 4: Merge "Nucleon Dark" and "Nucleon Light" into a single "Nucleon" ThemePack

**Problem:** "Nucleon Dark" and "Nucleon Light" are separate ThemePacks in the pack
dropdown. This is confusing because the user has to understand which pack is which before
selecting. Dark vs Light is a *color choice within a theme*, not a separate theme identity.

**Fix:** Merge them into one "Nucleon" ThemePack that defaults to the light color scheme.
The user then selects Dark or Light in the Full Color theme picker (the color mode controls
within Theme > Desktop or Theme > Terminal).

In `crates/shared/src/theme.rs`:

1. **Remove `ThemePack::nucleon_dark()` and `ThemePack::nucleon_light()`**. Replace with
   a single `ThemePack::nucleon()`:

```rust
pub fn nucleon() -> Self {
    ThemePack {
        id: "nucleon".to_string(),
        name: "Nucleon".to_string(),
        description: "Modern desktop shell with full-color theming".to_string(),
        version: "1.0.0".to_string(),
        shell_style: ShellStyle {
            id: "nucleon".to_string(),
            name: "Nucleon".to_string(),
            border_radius: 4.0,
            title_bar_height: 28.0,
            separator_thickness: 1.0,
            window_shadow: true,
        },
        layout_profile: LayoutProfile::classic(),
        color_style: ColorStyle::FullColor {
            theme_id: "nucleon-light".to_string(),
        },
        asset_pack: None,
        terminal_branding: TerminalBranding::none(),
        terminal_decoration: TerminalDecoration::default(),
    }
}
```

2. **Update `builtin_theme_packs()`:**
```rust
pub fn builtin_theme_packs() -> Vec<ThemePack> {
    vec![Self::classic(), Self::nucleon()]
}
```

3. **Keep both `FullColorTheme::nucleon_dark()` and `FullColorTheme::nucleon_light()`** —
   these are the color definitions, not theme packs. They remain available in the Full Color
   theme picker dropdown (the color controls inside Theme > Desktop/Terminal).

The flow is now:
- User selects ThemePack "Nucleon" → switches to Full Color mode with "Nucleon Light"
- In the Full Color theme picker, user can switch between "Nucleon Dark" and "Nucleon Light"
- The ThemePack dropdown shows: `Classic | Nucleon` (just two entries)

4. **Update any code that checks for theme pack IDs** `"nucleon-dark"` or `"nucleon-light"`:
   search for these strings in `tweaks_presenter.rs`, `addons.rs`, and `app.rs` and update
   to `"nucleon"` where appropriate.

### Verification

- `cargo check -p nucleon`
- `cargo check -p nucleon-native-shell`
- Run app:
  - Setting both Top and Bottom to "Menu Bar" renders two menu bars without errors
  - Setting both to "Taskbar" renders two taskbars without errors
  - Desktop/Terminal sub-tab buttons are visually the same height as main tabs
  - Terminal retro-screen tweaks shows collapsible sections — only one section expanded at a time
  - ThemePack dropdown shows: `Classic | Nucleon` (not three entries)
  - Selecting "Nucleon" switches to Full Color mode with "Nucleon Light" active
  - Full Color theme picker still offers both "Nucleon Dark" and "Nucleon Light"
  - No red "first use / second use" error text visible anywhere

---

# PHASE 7: Cursor and icon theming via AssetPack

### Goal
Theme packs can provide custom cursors and icon sets. The existing `AssetPackRef` on
`ThemePack` is wired into the rendering pipeline. Icon packs include two variants:
monochrome (white-channel, tinted at runtime) and full-color (RGBA, used as-is in
Full Color mode). Cursor packs provide alternative sprite definitions. The Classic
theme uses the existing built-in assets as defaults.

### Background: Current asset pipeline

**Cursors** (`src/native/app/software_cursor.rs`):
- Software-rendered from ASCII sprite masks (9 cursor types)
- Characters: `#` = fill, `O` = outline, `.` = highlight, space = transparent
- Colors derived from active palette: `fg` for fill, `dim` for shadow, blended white
  for highlight
- Cursor is painted via `draw_software_cursor()` called from `frame_runtime.rs:453`
- Cursor scale controlled by `settings.desktop_cursor_scale`

**Icons** (`src/native/app/ui_helpers.rs`, `desktop_surface.rs`, `asset_helpers.rs`):
- 96 SVG files in `src/Icons/` compiled via `include_bytes!()`
- Loaded by `load_svg_icon()` → parsed by usvg → rasterized by resvg → white-channel
  `ColorImage` → `TextureHandle`
- Painted via `paint_tinted_texture()` which tints white-channel images with palette `fg`
- File type icons selected by extension in `asset_helpers.rs`
- Desktop icons selected by `DesktopIconStyle` (Dos/Win95/Minimal/NoIcons)
- Built-in desktop icons defined in `desktop_surface_service.rs` (5 entries)

**AssetPackRef** (defined in `theme.rs` but currently unused):
```rust
pub struct AssetPackRef {
    pub id: String,
    pub name: String,
    pub path: String,
}
```

### Step 1: Expand AssetPack data model

In `crates/shared/src/theme.rs`, replace `AssetPackRef` with a richer model:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetPack {
    pub id: String,
    pub name: String,
    /// Path to the asset pack root directory (relative to theme bundle).
    /// Contains optional subdirectories: icons_mono/, icons_color/, cursors/
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CursorSprite {
    pub width: usize,
    pub height: usize,
    pub hotspot_x: usize,
    pub hotspot_y: usize,
    /// ASCII sprite mask: '#' = fill, 'O' = outline, '.' = highlight, ' ' = transparent
    pub mask: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CursorPack {
    pub arrow: Option<CursorSprite>,
    pub ibeam: Option<CursorSprite>,
    pub pointing_hand: Option<CursorSprite>,
    pub resize_horizontal: Option<CursorSprite>,
    pub resize_vertical: Option<CursorSprite>,
    pub resize_nwse: Option<CursorSprite>,
    pub resize_nesw: Option<CursorSprite>,
    pub move_cursor: Option<CursorSprite>,
    pub forbidden: Option<CursorSprite>,
    pub wait: Option<CursorSprite>,
}
```

Update `ThemePack`:
```rust
pub struct ThemePack {
    // ... existing fields ...
    pub asset_pack: Option<AssetPack>,    // was AssetPackRef
    pub cursor_pack: Option<CursorPack>,
}
```

`CursorPack` fields are all `Option` — if `None`, fall back to the built-in sprite for
that cursor type. This allows a theme to override just the arrow cursor, for example.

### Step 2: Icon pack directory convention

A theme pack's asset directory has this structure:
```
assets/
  icons_mono/         ← white-channel SVGs for monochrome mode (tinted at runtime)
    pixel--folder-solid.svg
    pixel--cog-solid.svg
    ...
  icons_color/        ← full-color SVGs for Full Color mode (rendered as-is)
    pixel--folder-solid.svg
    pixel--cog-solid.svg
    ...
  cursors/
    cursors.json      ← CursorPack serialized as JSON
```

**Fallback rules:**
1. If the active color mode is Monochrome, load from `icons_mono/`.
2. If the active color mode is Full Color, load from `icons_color/`.
3. If the needed directory is missing, fall back to the other one.
4. If the other one is also missing, fall back to built-in icons.
5. Individual missing icon files fall back to the built-in icon for that file type.

The icon file names must match the existing naming convention (`pixel--*.svg`) so the
lookup in `asset_helpers.rs` can resolve them by the same keys.

### Step 3: Add active asset state to NucleonNativeApp

In `src/native/app.rs`, add:
```rust
pub(super) active_asset_pack_path: Option<PathBuf>,
pub(super) active_cursor_pack: Option<CursorPack>,
```

When a theme pack with an asset pack is applied:
1. Resolve `asset_pack.path` relative to the theme bundle directory.
2. Store the resolved absolute path in `active_asset_pack_path`.
3. If `cursor_pack` is `Some`, store it in `active_cursor_pack`.
4. If `cursor_pack` is `None` but `assets/cursors/cursors.json` exists, load and parse it.

When the Classic theme is active (or any theme without an asset pack), both are `None`,
and all rendering falls back to built-ins.

### Step 4: Wire cursor pack into software_cursor.rs

In `src/native/app/software_cursor.rs`:

The current `sprite_for_cursor()` function returns a static `&CursorSpriteData` based on
the egui `CursorIcon`. Modify `draw_software_cursor()` to accept an optional `&CursorPack`
and check for theme overrides before falling back to built-ins.

Add a conversion function:
```rust
fn themed_sprite_for_cursor(
    icon: egui::CursorIcon,
    pack: Option<&CursorPack>,
) -> CursorSpriteData {
    if let Some(pack) = pack {
        let themed = match icon {
            egui::CursorIcon::Default => &pack.arrow,
            egui::CursorIcon::Text => &pack.ibeam,
            egui::CursorIcon::PointingHand => &pack.pointing_hand,
            egui::CursorIcon::ResizeHorizontal => &pack.resize_horizontal,
            egui::CursorIcon::ResizeVertical => &pack.resize_vertical,
            egui::CursorIcon::ResizeNwSe => &pack.resize_nwse,
            egui::CursorIcon::ResizeNeSw => &pack.resize_nesw,
            egui::CursorIcon::Move => &pack.move_cursor,
            egui::CursorIcon::NotAllowed => &pack.forbidden,
            egui::CursorIcon::Wait => &pack.wait,
            _ => &None,
        };
        if let Some(sprite) = themed {
            return CursorSpriteData {
                width: sprite.width,
                height: sprite.height,
                hotspot: (sprite.hotspot_x, sprite.hotspot_y),
                mask: &sprite.mask,
            };
        }
    }
    // Fall back to built-in
    sprite_for_cursor(icon)
}
```

**Note:** The `mask: &sprite.mask` borrow requires the CursorPack to outlive the sprite
data. Since `draw_software_cursor` is called once per frame and the pack lives on
`NucleonNativeApp`, this is fine — pass a reference from `self.active_cursor_pack`.

Update the call chain: `draw_software_cursor()` in `frame_runtime.rs` should pass
`self.active_cursor_pack.as_ref()` down.

### Step 5: Wire icon pack into asset loading

In `src/native/app/ui_helpers.rs` and `src/native/app/asset_helpers.rs`:

Add a function that resolves an icon path with theme fallback:

```rust
fn resolve_themed_icon_bytes(
    icon_name: &str,
    asset_pack_path: Option<&Path>,
    color_mode_is_full_color: bool,
) -> Option<Vec<u8>> {
    let pack_path = asset_pack_path?;
    // Try the preferred directory first
    let preferred_dir = if color_mode_is_full_color {
        "icons_color"
    } else {
        "icons_mono"
    };
    let preferred = pack_path.join(preferred_dir).join(icon_name);
    if preferred.exists() {
        return std::fs::read(&preferred).ok();
    }
    // Fall back to the other directory
    let fallback_dir = if color_mode_is_full_color {
        "icons_mono"
    } else {
        "icons_color"
    };
    let fallback = pack_path.join(fallback_dir).join(icon_name);
    if fallback.exists() {
        return std::fs::read(&fallback).ok();
    }
    None // Fall back to built-in
}
```

Modify `build_asset_cache()` in `desktop_surface.rs` to accept an optional asset pack
path. When building the cache:
1. Try `resolve_themed_icon_bytes()` for each icon.
2. If `Some`, use those bytes instead of the `include_bytes!()` built-in.
3. If `None`, use the built-in as before.

**Important for Full Color icons:** When loading from `icons_color/`, the SVG should be
loaded as a full-RGBA image, NOT converted to white-channel. Add a `load_svg_icon_color()`
variant that preserves original colors. The `paint_tinted_texture()` call should be skipped
for full-color icons — paint them with `Color32::WHITE` tint (no tinting = original colors).

Add a flag to track whether each cached texture is monochrome or full-color:
```rust
pub struct CachedIcon {
    pub texture: TextureHandle,
    pub is_full_color: bool,
}
```

When painting, check `is_full_color`:
- If true: paint with `Color32::WHITE` (preserves original colors)
- If false: paint with `palette.fg` (monochrome tinting, existing behavior)

### Step 6: Invalidate icon cache on theme change

When the active theme pack changes (or when color mode switches between Monochrome and
Full Color), the icon cache must be rebuilt because:
- Different icon set may be needed (mono vs color directory)
- Full color icons use different rendering path than mono icons

Add a `self.icon_cache_dirty: bool` flag. Set it to `true` when:
- `desktop_active_color_style` changes
- `active_asset_pack_path` changes
- A theme pack is applied

In the frame loop, when `icon_cache_dirty` is true, rebuild the asset cache and reset
the flag.

### Step 7: Update ThemePack constructors

Update `ThemePack::classic()`:
```rust
cursor_pack: None,  // uses built-in cursors
```

Update `ThemePack::nucleon_dark()` and `ThemePack::nucleon_light()`:
```rust
cursor_pack: None,  // uses built-in cursors (for now)
```

### Architecture notes for future terminal UI theming

The terminal retro-screen UI (character grid, RetroScreen, CentralPanel fill) is currently
rendered with hardcoded layout constants and palette colors. Future terminal theming will
want to customize:
- Screen margins and padding
- Grid cell dimensions (currently FIXED_PTY_CELL_W / FIXED_PTY_CELL_H)
- Title decoration style (separator characters, text alignment)
- Status bar format
- Menu rendering style (selection highlighting, indentation)

**To keep the architecture open for this:**
- The `TerminalLayout` struct already receives values from the theme system (via
  `terminal_layout_with_branding()` from Phase 5). Future terminal styling can extend
  this struct with additional fields.
- The `TerminalSlotRegistry` and `TerminalSlotRenderer` trait (from Phase 1b) already
  support swappable screen/overlay/status-bar renderers. A future "retro terminal theme"
  can provide custom implementations of these slots.
- Do NOT hardcode new constants in this phase. Any new layout or style values should flow
  through the existing theme/layout data structures.
- Do NOT create a `TerminalStyle` type yet — wait until the actual customization
  requirements are clear. The current `TerminalLayoutProfile` + `TerminalBranding` +
  palette system is sufficient scaffolding.

### Verification

- `cargo check -p nucleon`
- `cargo check -p nucleon-native-shell`
- Run app with Classic theme:
  - All icons render identically to before (built-in fallback)
  - Cursor renders identically to before (built-in fallback)
- If a test theme pack with `icons_mono/` directory is placed in addons:
  - Icons from the pack appear in monochrome mode
  - Switching to Full Color falls back to built-in if `icons_color/` is missing
- If a test theme pack with a `CursorPack` is loaded:
  - Overridden cursors use the theme sprite
  - Non-overridden cursors fall back to built-in

---

# PHASE 8: ShellStyle consumption

### Goal
Wire the existing `ShellStyle` fields into actual rendering. Currently `ShellStyle` is
defined on every `ThemePack` with values for `border_radius`, `title_bar_height`,
`separator_thickness`, and `window_shadow`, but nothing reads them. After this phase,
switching theme packs visibly changes window chrome geometry and shadow behavior.

### Background: Current hardcoded chrome

**Window frame** (`desktop_window_mgmt.rs:469`):
```rust
fn desktop_window_frame() -> egui::Frame {
    let palette = current_palette();
    egui::Frame::none()
        .fill(palette.bg)
        .stroke(egui::Stroke::new(1.0, palette.fg))
        .inner_margin(egui::Margin::same(1.0))
}
```
No rounding, no shadow, hardcoded 1px stroke. This is the single source of window frame
styling per the architecture constraints.

**Window header** (`desktop_window_mgmt.rs:1131`):
```rust
fn draw_desktop_window_header(...) {
    egui::Frame::none()
        .fill(palette.window_chrome_focused)
        .inner_margin(egui::Margin::symmetric(8.0, 4.0))
        ...
}
```
Header height is driven by `inner_margin` + content. No connection to `title_bar_height`.

**Shadow suppression** — 8+ sites set `Shadow::NONE` and `Rounding::ZERO`:
- `desktop_menu_bar.rs` (3 apply-style functions)
- `desktop_taskbar.rs` (`apply_desktop_panel_button_style`)
- `desktop_start_menu.rs` (`apply_start_menu_style`)
- `desktop_spotlight.rs` (spotlight frame)
- `ui_helpers.rs` (`apply_settings_control_style`)

**Separator thickness** — `retro_separator()` in `ui_helpers.rs` draws with hardcoded
`Stroke::new(2.0, palette.fg)`.

**Built-in ShellStyle values** (from `theme.rs`):
- Classic: `border_radius: 0.0, title_bar_height: 28.0, separator_thickness: 2.0, window_shadow: false`
- Nucleon: `border_radius: 4.0, title_bar_height: 28.0, separator_thickness: 1.0, window_shadow: true`

### Step 1: Add active ShellStyle to runtime state

In `src/native/app.rs`, add a field:
```rust
pub(super) desktop_active_shell_style: ShellStyle,
```

Initialize it from `ThemePack::classic().shell_style` in Default.

When a theme pack is applied via `apply_desktop_theme_pack_selection()`, also copy the
theme pack's `shell_style` into `desktop_active_shell_style`.

When `desktop_active_theme_pack_id` is set to `None` (custom overrides), keep the current
`desktop_active_shell_style` unchanged — the user's customization is color-only, not
chrome-geometry.

Add this field to `ParkedSessionState` and the park/restore cycle in `session_management.rs`.

### Step 2: Parameterize desktop_window_frame

Change `desktop_window_frame()` from a static method to an instance method so it can
read `self.desktop_active_shell_style`:

```rust
pub(super) fn desktop_window_frame(&self) -> egui::Frame {
    let palette = current_palette();
    let style = &self.desktop_active_shell_style;
    let rounding = egui::Rounding::same(style.border_radius);
    let shadow = if style.window_shadow {
        egui::epaint::Shadow {
            offset: egui::vec2(4.0, 4.0),
            blur: 8.0,
            spread: 0.0,
            color: Color32::from_black_alpha(80),
        }
    } else {
        egui::epaint::Shadow::NONE
    };
    egui::Frame::none()
        .fill(palette.bg)
        .stroke(egui::Stroke::new(style.separator_thickness.min(2.0), palette.fg))
        .inner_margin(egui::Margin::same(1.0))
        .rounding(rounding)
        .shadow(shadow)
}
```

Update all call sites from `Self::desktop_window_frame()` to `self.desktop_window_frame()`.
There are ~8 call sites in `tweaks_presenter.rs`, `desktop_window_presenters.rs`, and
`desktop_window_mgmt.rs`.

### Step 3: Parameterize window header height

In `draw_desktop_window_header()`, replace the hardcoded margin with a computed value
based on `title_bar_height`:

```rust
fn draw_desktop_window_header(
    ui: &mut egui::Ui,
    title: &str,
    maximized: bool,
    shell_style: &ShellStyle,
) -> DesktopHeaderAction {
    let vertical_pad = ((shell_style.title_bar_height - 20.0) / 2.0).max(2.0);
    egui::Frame::none()
        .fill(palette.window_chrome_focused)
        .inner_margin(egui::Margin::symmetric(8.0, vertical_pad))
        .rounding(egui::Rounding {
            nw: shell_style.border_radius,
            ne: shell_style.border_radius,
            sw: 0.0,
            se: 0.0,
        })
        ...
}
```

The top corners of the header get the theme's `border_radius`; the bottom corners stay
square so the header meets the window body cleanly.

Update all callers to pass `&self.desktop_active_shell_style`.

### Step 4: Parameterize separator thickness

In `ui_helpers.rs`, change `retro_separator()` to accept a thickness:

```rust
pub(super) fn retro_separator_with_thickness(ui: &mut egui::Ui, thickness: f32) {
    let palette = current_palette();
    let width = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(
        egui::vec2(width, thickness),
        egui::Sense::hover(),
    );
    ui.painter().rect_filled(rect, 0.0, palette.fg);
}
```

Keep the existing `retro_separator()` as a convenience that calls
`retro_separator_with_thickness(ui, 2.0)`. The Tweaks window and any theme-aware separators
should call the thickness variant with `self.desktop_active_shell_style.separator_thickness`.

### Step 5: Parameterize shadow in panel/menu style functions

The 8+ `apply_*_style()` functions hardcode `Shadow::NONE` and `Rounding::ZERO`. These
should read from the active shell style:

For window-like overlays (start menu, spotlight, dropdown menus):
- `window_shadow` / `popup_shadow` = shadow from shell style (same as Step 2 formula)
- `window_rounding` / `menu_rounding` = `border_radius` from shell style

For panel bars (taskbar, menu bar):
- These remain `Shadow::NONE` — bars are flush with the screen edge, not floating.
- Rounding remains `ZERO` for the same reason.

The distinction: **floating overlays** get the theme's shadow/rounding, **anchored bars** don't.

Functions to update:
- `apply_top_bar_menu_button_style()` — bars: no shadow, no rounding
- `apply_top_bar_dropdown_style()` — floating: gets shadow + rounding
- `apply_desktop_panel_button_style()` — bars: no shadow, no rounding
- `apply_start_menu_style()` (in `desktop_start_menu.rs`) — floating: gets shadow + rounding
- spotlight frame (in `desktop_spotlight.rs`) — floating: gets shadow + rounding
- `apply_settings_control_style()` (in `ui_helpers.rs`) — floating: gets shadow + rounding

Each of these functions needs access to the shell style. Pass it as a parameter or read
it from `self` (for instance methods) or from a thread-local (matching the existing
`current_palette()` pattern).

**Recommended approach:** Add a `current_shell_style() -> &'static ShellStyle` function
parallel to `current_palette()` in `retro_ui.rs`, backed by a thread-local. Set it in
`sync_desktop_appearance()` / `sync_terminal_appearance()` alongside the palette. This
avoids threading `&ShellStyle` through dozens of static style helper functions.

### Step 6: Persist active shell style

`persist_surface_theme_state_to_settings()` already persists theme pack ID. Since shell
style is determined by the theme pack (not independently overridden), no new settings
field is needed. When settings are loaded, the shell style is derived from the theme
pack's `shell_style` field.

In `apply_surface_theme_state_from_settings()`, after resolving the theme pack, also
set `desktop_active_shell_style`:
```rust
let pack = installed_theme_packs()
    .into_iter()
    .find(|p| Some(p.id.as_str()) == self.desktop_active_theme_pack_id.as_deref())
    .unwrap_or_else(ThemePack::classic);
self.desktop_active_shell_style = pack.shell_style.clone();
```

### Verification

- `cargo check -p nucleon -p nucleon-native-shell`
- Classic theme: all windows have 0px rounding, no shadow, 2px separators — identical
  to pre-phase behavior.
- Nucleon theme: windows have 4px rounded corners, subtle drop shadow, 1px separators.
- Start menu and spotlight overlays show rounding + shadow in Nucleon, square + no shadow
  in Classic.
- Taskbar and menu bar remain flat/square in both themes.
- Switch between Classic and Nucleon multiple times — no visual glitches or stale state.

---

# PHASE 9: Sound theming

### Goal
Make the sound system theme-aware. Theme packs can provide custom sound sets. The existing
`SoundPaths` structure in `crates/shared/src/sound.rs` uses hardcoded `include_bytes!()`
WAV files. After this phase, the active theme pack can override any or all sound events
with custom audio files, with per-event fallback to built-in.

### Background: Current sound system

**Location:** `crates/shared/src/sound.rs`

**Sound events** (6 types):
- `login` — played on successful login
- `logout` — played on logout
- `error` — played on error conditions
- `navigate` — played on menu navigation (with repeat gating: 80ms gap)
- `keypress` — played on key input (with repeat gating)
- `boot_keys` — 5 clips cycled during boot sequence (preprocessed PCM16)

**Playback pipeline:**
1. `SoundPaths` struct holds `SoundClip` values (name + embedded bytes)
2. `clip_path()` volume-scales the WAV, writes to a temp file, caches the path
3. `play_nonblocking()` spawns OS audio player (`aplay`/`paplay`/`afplay`/PowerShell)
4. `sound_enabled()` checks `Settings.sound`
5. `system_sound_volume()` reads `Settings.system_sound_volume` (0-100)

**Key constraint:** Sound clips are currently `include_bytes!()` at compile time. The
temp-file + cache system already exists because volume scaling rewrites the WAV. This
means theme sound files can use the same pipeline — just provide external file bytes
instead of embedded bytes.

### Step 1: Add SoundPack to theme data model

In `crates/shared/src/theme.rs`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SoundPack {
    /// Path to the sound pack root directory (relative to theme bundle).
    /// Contains optional WAV files: login.wav, logout.wav, error.wav,
    /// navigate.wav, keypress.wav, boot_01.wav through boot_05.wav
    pub path: Option<String>,
}
```

Update `ThemePack`:
```rust
pub struct ThemePack {
    // ... existing fields ...
    #[serde(default)]
    pub sound_pack: SoundPack,
}
```

All built-in themes default to `SoundPack { path: None }` (use built-in sounds).

### Step 2: Sound pack directory convention

A theme pack's sound directory:
```
sounds/
  login.wav
  logout.wav
  error.wav
  navigate.wav
  keypress.wav
  boot_01.wav
  boot_02.wav
  boot_03.wav
  boot_04.wav
  boot_05.wav
```

All files are optional. Missing files fall back to the built-in clip for that event.
Boot clips are numbered 01-05. If fewer than 5 are present, only the provided ones cycle.
If none are present, built-in boot clips are used.

WAV format requirement: PCM16 mono or stereo, any sample rate. The existing volume-scaling
pipeline handles format normalization.

### Step 3: Add active sound pack state

In `src/native/app.rs`:
```rust
pub(super) active_sound_pack_path: Option<PathBuf>,
```

When a theme pack with a `sound_pack.path` is applied, resolve the path relative to the
theme bundle directory and store it. When Classic theme is active (or any theme without
a sound pack), this is `None`.

### Step 4: Wire sound pack into the playback pipeline

In `crates/shared/src/sound.rs`, add a global for the active sound pack path:

```rust
static ACTIVE_SOUND_PACK: Mutex<Option<PathBuf>> = Mutex::new(None);

pub fn set_active_sound_pack(path: Option<PathBuf>) {
    if let Ok(mut guard) = ACTIVE_SOUND_PACK.lock() {
        *guard = path;
    }
    // Clear the clip cache so next play uses the new pack
    if let Ok(mut guard) = CLIP_CACHE.get_or_init(|| Mutex::new(HashMap::new())).lock() {
        guard.clear();
    }
}
```

Modify `clip_path()` to check the active sound pack first:

```rust
fn clip_path(clip: &SoundClip) -> PathBuf {
    // Check if active sound pack has an override for this clip
    if let Ok(guard) = ACTIVE_SOUND_PACK.lock() {
        if let Some(pack_path) = guard.as_ref() {
            let override_file = pack_path.join(format!("{}.wav", clip.name));
            if override_file.exists() {
                return volume_scaled_clip_path(&override_file, clip.name);
            }
        }
    }
    // Fall back to built-in
    volume_scaled_builtin_clip_path(clip)
}
```

Extract the existing volume-scaling logic into `volume_scaled_clip_path()` and
`volume_scaled_builtin_clip_path()` to avoid duplication. The only difference is the
byte source: file read vs embedded bytes.

### Step 5: Call set_active_sound_pack on theme change

In `tweaks_presenter.rs`, when `apply_desktop_theme_pack_selection()` succeeds:
```rust
let sound_path = theme.sound_pack.path.as_ref().map(|p| {
    // Resolve relative to theme bundle directory
    resolve_theme_bundle_path(&theme.id, p)
});
nucleon::sound::set_active_sound_pack(sound_path);
```

Also call `set_active_sound_pack(None)` in `apply_surface_theme_state_from_settings()`
when resetting to defaults.

### Step 6: Add sound preview in Tweaks

In the Effects tab of desktop Tweaks, add a "Preview" button next to the Sound toggle:
```
[Sound: On/Off]  [Preview ▶]
```

Clicking Preview plays the `navigate` sound so the user can hear the active sound pack
without navigating away from Tweaks.

### Verification

- `cargo check -p nucleon -p nucleon-native-shell`
- Classic theme: all sounds play identically to before (built-in fallback)
- If a test theme pack with `sounds/navigate.wav` is installed:
  - Navigate sound uses the theme clip
  - Login/logout/error/etc. fall back to built-in
- Theme switch clears sound cache — next sound event uses the new pack
- Volume scaling still works with theme-provided clips

---

# PHASE 10: Theme packaging, import, and distribution

### Goal
Establish the packaging format, repo structure, and import flow so community themes can
be created, shared, and installed. After this phase, users can export full themes from
Tweaks, import `.ndpkg` theme bundles, and a sample Heritage theme pack
demonstrates the full system.

### Step 1: Theme bundle directory structure

A theme bundle is a directory (or `.ndpkg` zip) with this layout:
```
my-theme/
  manifest.json          ← ThemePack serialized as JSON (required)
  colors/
    dark.json            ← FullColorTheme serialized as JSON (optional)
    light.json
  assets/
    icons_mono/          ← white-channel SVGs (optional)
    icons_color/         ← full-color SVGs (optional)
    cursors/
      cursors.json       ← CursorPack (optional)
  sounds/                ← WAV overrides (optional)
    login.wav
    navigate.wav
    ...
```

`manifest.json` is the only required file. Everything else is optional — missing
directories/files fall back to built-in.

### Step 2: Theme export from Tweaks

The existing "Export Theme..." button in Tweaks exports a `FullColorTheme` as a `.json`
file to `exported_themes/`. Extend this to export a full theme bundle:

When the user clicks "Export Theme...":
1. Create a directory `exported_themes/{theme-name}/`
2. Write `manifest.json` — a `ThemePack` with the current runtime state:
   - `color_style` from `desktop_active_color_style`
   - `shell_style` from `desktop_active_shell_style`
   - `layout_profile` from `desktop_active_layout`
   - `terminal_branding` from current terminal branding
   - `terminal_decoration` from current terminal decoration
3. Write `colors/custom.json` — the `FullColorTheme` with the user's token overrides
4. Set `shell_status` to confirm: `"Exported to exported_themes/{name}/"`

The export does NOT include asset or sound files — those come from installed packs.
The export captures the user's customized color/layout/style configuration as a reusable
theme pack.

### Step 3: Theme import in Tweaks

Add an "Import Theme..." button in the Theme tab of desktop Tweaks, below the theme
pack ComboBox:

```
Theme Pack: [Classic    ▼]
[Import Theme...]
```

When clicked:
1. Open the embedded file manager in browse mode (same pattern as wallpaper picker)
2. User selects a `manifest.json` file or a `.ndpkg` file
3. If `.ndpkg`: extract to `{state_dir}/themes/{bundle-id}/`
4. If `manifest.json`: copy the parent directory to `{state_dir}/themes/{bundle-id}/`
5. Parse `manifest.json` as `ThemePack`
6. Add to installed theme packs list
7. Optionally apply immediately
8. Set `shell_status` to confirm: `"Imported theme: {name}"`

### Step 4: Theme discovery from state directory

Currently `installed_theme_packs()` returns only built-in packs (Classic, Nucleon).
Extend it to also scan `{state_dir}/themes/` for user-installed theme bundles:

```rust
pub fn installed_theme_packs() -> Vec<ThemePack> {
    let mut packs = vec![ThemePack::classic(), ThemePack::nucleon()];
    if let Some(themes_dir) = themes_directory() {
        for entry in std::fs::read_dir(&themes_dir).into_iter().flatten() {
            if let Ok(entry) = entry {
                let manifest = entry.path().join("manifest.json");
                if manifest.exists() {
                    if let Ok(pack) = load_theme_pack_from_manifest(&manifest) {
                        // Deduplicate: skip if same id as a built-in
                        if !packs.iter().any(|p| p.id == pack.id) {
                            packs.push(pack);
                        }
                    }
                }
            }
        }
    }
    packs
}
```

### Step 5: .ndpkg integration

Theme `.ndpkg` files reuse the existing addon packaging pipeline:
- `AddonManifest.kind` gains a `"theme"` variant (or use a new field `addon_type`)
- The repository index can include theme packs alongside wasm addons
- The installer screen (`installer_screen.rs`) shows theme packs in a separate section
- Install extracts to `{state_dir}/themes/{id}/` instead of the addon directory

The existing `install_repository_addon()` in `platform/repository.rs` handles download
and extraction. Add a branch that routes theme-type addons to the themes directory.

### Step 6: Heritage theme pack (external only)

The Heritage theme pack is **not built-in**. It lives in the external
`nucleon-core-themes` repo (see Step 7) and is installed through the theme import
system or the addon repository. A `ThemePack::heritage()` constructor exists in
`theme.rs` for testing purposes, but it is NOT included in `builtin_theme_packs()`.

The Heritage pack's `manifest.json`:

```json
{
  "id": "heritage",
  "name": "Heritage",
  "shell_style": {
    "id": "heritage",
    "name": "Heritage",
    "border_radius": 0.0,
    "title_bar_height": 28.0,
    "separator_thickness": 2.0,
    "window_shadow": false
  },
  "layout_profile": { ... classic layout ... },
  "color_style": { "Monochrome": { "preset": "Green", "custom_rgb": null } },
  "terminal_branding": {
    "header_lines": ["...", "...", "..."]
  },
  "terminal_decoration": {
    "separator_char": "=",
    "separator_alignment": "Center",
    "title_alignment": "Center",
    "title_bold": true,
    "subtitle_alignment": "Left",
    "subtitle_underlined": true,
    "show_separators": true
  },
  "sound_pack": { "path": null },
  "asset_pack": null,
  "cursor_pack": null
}
```

This pack re-enables the heritage terminal header branding using existing
`TerminalBranding` fields. It doesn't need custom assets or sounds — it just
reconfigures the existing system. Users install it from nucleon-core-themes.

### Step 7: nucleon-core-themes repository structure

Create the separate `nucleon-core-themes` repo with this layout:
```
    nucleon-core-themes/
  README.md
  themes/
    heritage/
      manifest.json
    cyberpunk-neon/
      manifest.json
      colors/
        neon.json
    ...
  scripts/
    package.sh          ← bundles a theme dir into .ndpkg
  index.json            ← AddonRepositoryIndex for theme discovery
```

This repo is referenced by the addon repository system. Users install from it through
the existing Installer screen or by dropping `.ndpkg` files into the import flow.

### Verification

- `cargo check -p nucleon -p nucleon-native-shell`
- "Export Theme..." creates a valid theme bundle directory
- "Import Theme..." loads a theme bundle and adds it to the pack selector
- Importing the Heritage pack from nucleon-core-themes adds it to the pack selector
- Switching between Classic, Nucleon, and an imported theme works without state leakage
- Imported theme survives app restart (persisted pack ID resolves to installed bundle)
- `builtin_theme_packs()` returns only Classic and Nucleon (Heritage is external)

---

# PHASE 11: Full Nucleon rebrand

### Goal

This phase finishes the product-wide rename so the active application, crates,
binaries, UI strings, packaging metadata, paths, and documentation all use the
Nucleon identity. The only remaining legacy-brand strings in the codebase are
theme-content values inside `TerminalBranding::heritage()` and
`ThemePack::heritage()`.

### Completed scope

- Workspace packages were renamed in dependency order and imports were updated
  to the `nucleon_*` crate idents.
- Native app types were renamed to the `Nucleon*` forms.
- Legacy binary names and legacy environment-variable fallbacks were removed.
- Default UI branding now uses Nucleon strings, a neutral default wallpaper
  label, and the neutral default terminal header text.
- State paths now migrate once into the new Nucleon directory layout on first
  launch when an older install is detected.
- CI, packaging metadata, Linux desktop integration, and user-facing
  documentation were updated to the new product name.

### Verification

1. `cargo check` passes across the workspace.
2. `cargo test` passes across the workspace.
3. The code-surface grep for the legacy brand returns only the theme-content
   strings in `TerminalBranding::heritage()` and `ThemePack::heritage()`.
4. The packaging/config grep for the legacy brand returns zero hits.
5. Launching `nucleon-native` shows Nucleon branding on the default shell path.

---

# Architecture constraints: Window Manager compatibility

The following constraints protect future WM integration. All phases must respect them.

### DO NOT:

1. **Do not add new hardcoded panel/taskbar heights in workspace calculations.**
   `desktop_workspace_rect()` (static version, line ~439 of `desktop_window_mgmt.rs`)
   already hardcodes `TOP_BAR_H: 30.0` and `TASKBAR_H: 32.0`. This is legacy — the
   `active_desktop_workspace_rect()` (instance method) correctly reads from
   `desktop_active_layout`. Always use the instance method. Never add new static
   workspace calculations.

2. **Do not couple window content rendering to window chrome.**
   Window content (what a hosted app draws) and window decoration (title bar, borders,
   shadow, resize handles) must remain separable. Currently `draw_desktop_window_header()`
   is called inside each window's `.show()` closure — this is acceptable for now, but do
   not make it worse by mixing decoration logic with app content logic. If you add new
   window rendering code, keep header/chrome in a clearly separated block at the top of
   the closure.

3. **Do not bypass `DesktopWindowState` for position/size tracking.**
   All window rect tracking flows through `note_desktop_window_rect()` →
   `DesktopWindowState.restore_pos/restore_size`. A future WM will replace egui's
   built-in window dragging with its own rect management. Do not store window positions
   in ad-hoc fields outside of `DesktopWindowState`.

4. **Do not add new static window lists parallel to `DesktopComponentBinding`.**
   There is already a static `DESKTOP_COMPONENT_BINDINGS` array and a dynamic
   `secondary_windows` Vec. A future WM will unify these into a single window registry.
   Do not create additional window tracking structures.

5. **Do not add window decoration styles inline.**
   `Self::desktop_window_frame()` is the single source for window frame styling. When
   Phase 8 wires `ShellStyle` into rendering, it will modify this one function. Do not
   create per-window frame variations — if a window needs different styling, it should
   come from the active `ShellStyle`, not from a hardcoded override in the window's
   draw function.

### Current WM-adjacent infrastructure (do not remove):

- `ManagedWindow` struct — exists in `desktop_app.rs`, currently dead code. Will become
  the unified window record when the WM phase begins.
- `WindowSource` enum — distinguishes window origins. Will expand to include external
  windows (X11/Wayland surfaces) in the WM phase.
- `WindowInstanceId` — composite key (kind + instance number) that supports multi-instance
  windows. This is the right abstraction — do not flatten it.
- `SlotRegistry` + `SlotRenderer` trait — runtime dispatch for shell components. The WM
  will use this same pattern for dynamic window decorations.

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
