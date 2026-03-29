# Nucleon Core — Codex Implementation Handoff

> Single source of truth for all remaining theme/shell/addon implementation work.
> Each phase is a self-contained unit. Do not skip ahead. Do not invent abstractions not described here.
> After each phase, `cargo check -p nucleon -p nucleon-native-shell` must pass.
> The application must look and behave identically to the pre-refactor state at every phase boundary.

---

## Repo snapshot

- Repo: `nucleon-core`
- Remote: `origin https://github.com/Stylith/nucleon-core.git`
- Branch: `WIP`
- Completed: Phases 0 through 12 (theme composition, per-surface split, sound theming, packaging, full rebrand, DesktopStyle expansion)
- Next: Phase 13 (theme component decomposition)

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

### Key source files

| File | Role |
|------|------|
| `src/native/app.rs` | Main `NucleonNativeApp` struct, Default impl, appearance sync |
| `src/native/app/frame_runtime.rs` | Main frame loop `update_native_shell_frame()` |
| `src/native/app/desktop_window_mgmt.rs` | `desktop_window_frame()`, `draw_desktop_window_header()`, tiling, z-order |
| `src/native/app/desktop_menu_bar.rs` | Top menu bar rendering (`draw_top_bar`) |
| `src/native/app/desktop_taskbar.rs` | Bottom taskbar rendering (`draw_desktop_taskbar`) |
| `src/native/app/desktop_start_menu.rs` | Start menu rendering (`draw_start_panel`) |
| `src/native/app/desktop_spotlight.rs` | Spotlight/search overlay |
| `src/native/app/desktop_surface.rs` | Desktop icon grid, wallpaper |
| `src/native/app/desktop_runtime.rs` | Standalone window prepare/update functions |
| `src/native/app/tweaks_presenter.rs` | Tweaks UI (Wallpaper, Theme, Effects, Display tabs) |
| `src/native/app/software_cursor.rs` | Software-rendered ASCII cursors |
| `src/native/app/session_management.rs` | ParkedSessionState, session switching |
| `src/native/desktop_app.rs` | `DesktopComponentBinding`, `DesktopComponentSpec`, `DESKTOP_COMPONENT_BINDINGS` |
| `src/native/retro_ui.rs` | `RetroPalette`, `current_palette()`, `current_desktop_style()`, `configure_visuals()` |
| `src/native/addons.rs` | Addon discovery, install, enable/disable, repository fetching |
| `src/native/installer_screen.rs` | Program installer + addon inventory UI |
| `src/native/settings_screen.rs` | Settings panels, home tile grid |
| `crates/shared/src/theme.rs` | Core theme data models (ThemePack, DesktopStyle, ColorStyle, etc.) |
| `crates/shared/src/config.rs` | Settings persistence, directory paths |
| `src/native/shell_slots/` | Desktop slot registry |
| `src/native/terminal_slots/` | Terminal slot registry |

### Module path convention

All files in `src/native/app/` are sub-modules of `src/native/app.rs`. They use `super::super::` to reach `src/native/` siblings, and `super::NucleonNativeApp` to reach the app struct. Every function in these files is an `impl NucleonNativeApp` method with visibility `pub(super)` or `pub(crate)`.

### Component binding system

In `src/native/desktop_app.rs`:
```rust
pub struct DesktopComponentBinding {
    pub spec: DesktopComponentSpec,
    pub is_open: fn(&NucleonNativeApp) -> bool,
    pub set_open: fn(&mut NucleonNativeApp, bool),
    pub draw: fn(&mut NucleonNativeApp, &Context),
    pub on_open: Option<fn(&mut NucleonNativeApp, bool)>,
    pub on_closed: Option<fn(&mut NucleonNativeApp)>,
}
```
Currently 8 components: FileManager, Editor, Settings, Applications, Installer, TerminalMode, PtyApp, Tweaks.

### Frame loop structure (frame_runtime.rs)

Desktop branch of `update_native_shell_frame()`:
```
self.draw_top_bar(ctx);           // desktop_menu_bar.rs
self.draw_desktop_taskbar(ctx);   // desktop_taskbar.rs
self.draw_desktop(ctx);           // desktop_surface.rs
self.draw_desktop_windows(ctx);   // iterates DESKTOP_COMPONENT_BINDINGS
self.draw_start_panel(ctx);       // desktop_start_menu.rs
self.draw_spotlight(ctx);         // desktop_spotlight.rs
```

### Thread-local desktop style access

- `current_desktop_style()` returns `DesktopStyle` clone from thread-local
- `set_active_desktop_style()` updates it each frame in `sync_desktop_appearance()`

---

## Completed work (Phases 0-11)

### Phase 0: Prep
- Tweaks standalone app crate (`nucleon-native-tweaks-app`)
- Window presenters split into per-window modules
- `WindowSource` and `ManagedWindow` WM seam types (scaffold only, not yet wired)

### Phase 1: Slot registries
- Desktop shell slots (`shell_slots/`)
- Terminal shell slots (`terminal_slots/`)
- Classic rendering dispatches through slots

### Phase 2: Color themes
- `ColorStyle` with Monochrome/FullColor modes
- CRT full-frame monochrome tinting in egui-wgpu shader
- `DesktopStyle::flat()` baseline (was `ShellStyle::classic()`)

### Phase 3 + 3b: Layout + surface split
- Data-driven `LayoutProfile` (classic, minimal)
- Desktop and Terminal have separate live layout/color/theme-pack state
- Surface-first Tweaks (Desktop/Terminal tabs)
- Terminal slot registry parallel to desktop

### Phase 4: Full-color themes
- `Nucleon Dark` and `Nucleon Light` built-in full-color themes
- Per-surface durable settings (theme-pack, color style, layout profile)
- Desktop Tweaks exposes all surface-independent controls

### Phase 5: Terminal branding
- Theme-controlled `TerminalBranding` (header lines)
- Default state has no RobCo header

### Phase 6: Tweaks polish
- Top-level tabs: Wallpaper, Theme, Effects, Display
- Desktop/Terminal sub-tabs in Wallpaper and Theme
- Terminal wallpaper support
- Terminal decoration (separator_char, alignment, underline)
- Color palette inline picker (Paint-style preset grid)
- `RetroPalette` separation: `window_chrome`, `window_chrome_focused`, `bar_bg`

### Phase 7: Asset theming
- `AssetPack` model (icons_mono/, icons_color/, cursors/)
- `CursorPack` with per-cursor-type sprite overrides
- Icon cache with theme fallback (themed → built-in)
- Full-color icon rendering path (no tinting)

### Phase 8: DesktopStyle consumption
- `DesktopStyle` with `id`, `name`, `border_radius`, `title_bar_height`, `separator_thickness`, `window_shadow`
- Desktop rendering reads desktop style for frame, header, menu chrome
- Desktop style persisted in settings, parked in sessions

### Phase 9: Sound theming
- `SoundPack` model with clip paths for all sound events
- `ACTIVE_SOUND_PACK` global
- Sound preview in Tweaks
- Session management preserves active sound pack

### Phase 10: Theme packaging
- `.ndpkg` theme pack export/import
- Theme discovery from `{state_dir}/theme_packs/`
- `nucleon-core-themes` external repo structure
- RobCo removed from codebase entirely (external-only, lives in themes repo)
- `builtin_theme_packs()` removed — no built-in packs, default state is Flat + Monochrome Green

### Phase 11: Full rebrand
- All crates renamed to `nucleon-*`
- All types renamed (`NucleonNativeApp`, etc.)
- UI strings updated, legacy env vars removed
- State directory migration
- `cargo check -p nucleon -p nucleon-native-shell` passes

### Current DesktopStyle (4 fields — to be expanded in Phase 12)
```rust
pub struct DesktopStyle {
    pub id: String,
    pub name: String,
    pub border_radius: f32,
    pub title_bar_height: f32,
    pub separator_thickness: f32,
    pub window_shadow: bool,
}
```

### Current rendering pipeline
- `desktop_window_frame()` reads `border_radius`, `separator_thickness`, `window_shadow`
- `draw_desktop_window_header()` reads `title_bar_height`, `border_radius`
- `draw_top_bar()` paints flat `palette.bar_bg`
- `draw_desktop_taskbar()` paints flat `palette.bar_bg`
- `draw_start_panel()` paints flat `palette.panel`
- `apply_global_retro_menu_chrome()` sets zero rounding, no shadow for menus
- All colors come from `RetroPalette` via `current_palette()`

### Current settings persistence fields
```
desktop_theme_pack_id, terminal_theme_pack_id,
desktop_color_style, terminal_color_style,
desktop_layout_profile, terminal_layout_profile,
terminal_branding, desktop_wallpaper, terminal_wallpaper,
desktop_wallpaper_size_mode, terminal_wallpaper_size_mode
```

### Current AddonKind enum
```rust
pub enum AddonKind { App, Theme, ContentPack, Game, Service }
```

### Current Settings home grid
Row 1: General, Appearance (opens Tweaks), Default Apps, Connections
Row 2: CLI Profiles, Edit Menus, User Management, About

### Current installer views
`InstallerView`: Root (Search, Installed Apps, Runtime Tools, Installed Addons, Package Manager) → AddonInventory → AddonActions

### Bug fixes already landed
- CRT uniform/shader layout mismatch
- Hosted addon texture deferred release
- Desktop hosted addon + PTY window coexistence
- Color palette picker persistence bug (IPC self-notification)
- RetroPalette field separation (window_chrome, bar_bg)

---

# PHASE 12: Expand DesktopStyle to element-level styling

### Goal

Replace the 4-field `DesktopStyle` with a rich element-level styling system that can
express flat panels (Win95), gradient title bars (XP), glossy bars (Aero/macOS), and
shadowed chrome — all using lightweight egui primitives.

---

### Step 1: Define element styling primitives

In `crates/shared/src/theme.rs`, add these types:

```rust
/// How an element's background is filled.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum FillStyle {
    /// No background fill — element is transparent.
    None,
    /// Single solid color.
    Solid {
        color: ThemeColor,
    },
    /// Multi-stop linear gradient. Minimum 2 stops.
    /// Rendered as vertex-colored triangle strips — N stops = 2*(N-1) triangles.
    LinearGradient {
        stops: Vec<GradientStop>,
        /// Angle in degrees. 0 = top-to-bottom, 90 = left-to-right.
        angle: f32,
    },
}

/// A single color stop in a gradient.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GradientStop {
    /// Position along the gradient axis. 0.0 = start edge, 1.0 = end edge.
    pub position: f32,
    pub color: ThemeColor,
}

/// A color that can reference a palette token or be an explicit RGBA value.
/// Palette references adapt to the active color theme. RGBA values are fixed.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ThemeColor {
    /// Reference a palette field by name. Resolved at render time.
    Palette(PaletteRef),
    /// Explicit RGBA value. Does NOT adapt to palette changes.
    Rgba([u8; 4]),
}

/// Which palette field to reference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PaletteRef {
    Fg, Dim, Bg, Panel, SelectedBg, SelectedFg,
    HoveredBg, ActiveBg, SelectionBg,
    WindowChrome, WindowChromeFocused, BarBg,
}

/// Border styling for an element.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BorderStyle {
    pub width: f32,
    pub color: ThemeColor,
    /// Optional second border line for ridge/groove effects (1px inside/outside primary).
    pub highlight: Option<ThemeColor>,
}

/// Shadow styling for an element.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ShadowStyle {
    pub offset_x: f32,
    pub offset_y: f32,
    pub blur: f32,
    pub color: ThemeColor,
}

/// Complete visual style for one shell element.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ElementStyle {
    #[serde(default = "default_fill_none")]
    pub fill: FillStyle,
    #[serde(default)]
    pub border: Option<BorderStyle>,
    #[serde(default)]
    pub shadow: Option<ShadowStyle>,
    #[serde(default)]
    pub rounding: f32,
    /// Override text color for this element. None = inherit from palette.
    #[serde(default)]
    pub text_color: Option<ThemeColor>,
}

fn default_fill_none() -> FillStyle { FillStyle::None }
```

**Design rationale:**
- `ThemeColor::Palette(PaletteRef::BarBg)` = "use whatever the active palette's `bar_bg` is." Keeps desktop styles adaptive.
- `ThemeColor::Rgba([0, 120, 215, 255])` = "always this exact blue." For things like XP-blue title bars that don't change with color theme.
- N-stop gradients allow highlight bands (e.g., XP Luna 3-stop: dark blue → light blue highlight → dark blue).

**Example: XP Luna title bar** (3-stop vertical gradient):
```json
{
  "fill": {
    "type": "LinearGradient",
    "stops": [
      { "position": 0.0,  "color": [0, 88, 238, 255] },
      { "position": 0.15, "color": [53, 145, 255, 255] },
      { "position": 1.0,  "color": [0, 62, 186, 255] }
    ],
    "angle": 0
  },
  "rounding": 4.0,
  "text_color": { "Rgba": [255, 255, 255, 255] }
}
```

**Example: Win95 flat panel with ridge border**:
```json
{
  "fill": { "type": "Solid", "color": [192, 192, 192, 255] },
  "border": { "width": 1.0, "color": [64, 64, 64, 255], "highlight": [255, 255, 255, 255] },
  "rounding": 0.0
}
```

---

### Step 2: Expand DesktopStyle with per-element styles

Replace the current `DesktopStyle` struct:

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DesktopStyle {
    pub id: String,
    pub name: String,

    // Dimensions
    pub title_bar_height: f32,
    pub separator_thickness: f32,

    // Per-element styles
    #[serde(default = "ElementStyle::default_window_frame")]
    pub window_frame: ElementStyle,
    #[serde(default = "ElementStyle::default_title_bar")]
    pub title_bar: ElementStyle,
    #[serde(default = "ElementStyle::default_title_bar_unfocused")]
    pub title_bar_unfocused: ElementStyle,
    #[serde(default = "ElementStyle::default_bar")]
    pub menu_bar: ElementStyle,
    #[serde(default = "ElementStyle::default_bar")]
    pub taskbar: ElementStyle,
    #[serde(default = "ElementStyle::default_menu_dropdown")]
    pub menu_dropdown: ElementStyle,
    #[serde(default = "ElementStyle::default_panel")]
    pub start_menu: ElementStyle,
    #[serde(default)]
    pub start_button: Option<ElementStyle>,
    #[serde(default = "ElementStyle::default_panel")]
    pub panel: ElementStyle,
    #[serde(default)]
    pub scrollbar: Option<ElementStyle>,
}
```

The old `border_radius` and `window_shadow` fields are removed. Their functionality is now `window_frame.rounding` and `window_frame.shadow`.

Add `DesktopStyle::flat()` (default) that produces the current flat green-border look,
and `DesktopStyle::modern()` that produces the rounded/shadowed Nucleon look:

```rust
impl Default for DesktopStyle {
    fn default() -> Self { Self::flat() }
}

impl DesktopStyle {
    pub fn flat() -> Self {
        DesktopStyle {
            id: "flat".to_string(),
            name: "Flat".to_string(),
            title_bar_height: 28.0,
            separator_thickness: 2.0,
            window_frame: ElementStyle {
                fill: FillStyle::Solid { color: ThemeColor::Palette(PaletteRef::Bg) },
                border: Some(BorderStyle {
                    width: 2.0, color: ThemeColor::Palette(PaletteRef::Fg), highlight: None,
                }),
                shadow: None, rounding: 0.0, text_color: None,
            },
            title_bar: ElementStyle {
                fill: FillStyle::Solid { color: ThemeColor::Palette(PaletteRef::WindowChromeFocused) },
                border: None, shadow: None, rounding: 0.0,
                text_color: Some(ThemeColor::Palette(PaletteRef::SelectedFg)),
            },
            title_bar_unfocused: ElementStyle {
                fill: FillStyle::Solid { color: ThemeColor::Palette(PaletteRef::WindowChrome) },
                border: None, shadow: None, rounding: 0.0,
                text_color: Some(ThemeColor::Palette(PaletteRef::SelectedFg)),
            },
            menu_bar: ElementStyle {
                fill: FillStyle::Solid { color: ThemeColor::Palette(PaletteRef::BarBg) },
                border: None, shadow: None, rounding: 0.0,
                text_color: Some(ThemeColor::Palette(PaletteRef::SelectedFg)),
            },
            taskbar: ElementStyle {
                fill: FillStyle::Solid { color: ThemeColor::Palette(PaletteRef::BarBg) },
                border: None, shadow: None, rounding: 0.0,
                text_color: Some(ThemeColor::Palette(PaletteRef::SelectedFg)),
            },
            menu_dropdown: ElementStyle {
                fill: FillStyle::Solid { color: ThemeColor::Palette(PaletteRef::Bg) },
                border: Some(BorderStyle {
                    width: 2.0, color: ThemeColor::Palette(PaletteRef::Fg), highlight: None,
                }),
                shadow: None, rounding: 0.0, text_color: None,
            },
            start_menu: ElementStyle {
                fill: FillStyle::Solid { color: ThemeColor::Palette(PaletteRef::Panel) },
                border: Some(BorderStyle {
                    width: 2.0, color: ThemeColor::Palette(PaletteRef::Fg), highlight: None,
                }),
                shadow: None, rounding: 0.0, text_color: None,
            },
            start_button: None,
            panel: ElementStyle {
                fill: FillStyle::Solid { color: ThemeColor::Palette(PaletteRef::Panel) },
                border: None, shadow: None, rounding: 0.0, text_color: None,
            },
            scrollbar: None,
        }
    }

    pub fn modern() -> Self {
        DesktopStyle {
            id: "modern".to_string(),
            name: "Modern".to_string(),
            title_bar_height: 28.0,
            separator_thickness: 1.0,
            window_frame: ElementStyle {
                fill: FillStyle::Solid { color: ThemeColor::Palette(PaletteRef::Bg) },
                border: Some(BorderStyle {
                    width: 1.0, color: ThemeColor::Palette(PaletteRef::Dim), highlight: None,
                }),
                shadow: Some(ShadowStyle {
                    offset_x: 0.0, offset_y: 2.0, blur: 8.0,
                    color: ThemeColor::Rgba([0, 0, 0, 80]),
                }),
                rounding: 6.0, text_color: None,
            },
            title_bar: ElementStyle {
                fill: FillStyle::Solid { color: ThemeColor::Palette(PaletteRef::WindowChromeFocused) },
                border: None, shadow: None, rounding: 6.0,
                text_color: Some(ThemeColor::Palette(PaletteRef::SelectedFg)),
            },
            title_bar_unfocused: ElementStyle {
                fill: FillStyle::Solid { color: ThemeColor::Palette(PaletteRef::WindowChrome) },
                border: None, shadow: None, rounding: 6.0,
                text_color: Some(ThemeColor::Palette(PaletteRef::SelectedFg)),
            },
            menu_bar: ElementStyle {
                fill: FillStyle::Solid { color: ThemeColor::Palette(PaletteRef::BarBg) },
                border: None, shadow: None, rounding: 0.0,
                text_color: Some(ThemeColor::Palette(PaletteRef::SelectedFg)),
            },
            taskbar: ElementStyle {
                fill: FillStyle::Solid { color: ThemeColor::Palette(PaletteRef::BarBg) },
                border: None, shadow: None, rounding: 0.0,
                text_color: Some(ThemeColor::Palette(PaletteRef::SelectedFg)),
            },
            menu_dropdown: ElementStyle {
                fill: FillStyle::Solid { color: ThemeColor::Palette(PaletteRef::Bg) },
                border: Some(BorderStyle {
                    width: 1.0, color: ThemeColor::Palette(PaletteRef::Dim), highlight: None,
                }),
                shadow: Some(ShadowStyle {
                    offset_x: 0.0, offset_y: 2.0, blur: 6.0,
                    color: ThemeColor::Rgba([0, 0, 0, 60]),
                }),
                rounding: 4.0, text_color: None,
            },
            start_menu: ElementStyle {
                fill: FillStyle::Solid { color: ThemeColor::Palette(PaletteRef::Panel) },
                border: Some(BorderStyle {
                    width: 1.0, color: ThemeColor::Palette(PaletteRef::Dim), highlight: None,
                }),
                shadow: Some(ShadowStyle {
                    offset_x: 0.0, offset_y: 4.0, blur: 12.0,
                    color: ThemeColor::Rgba([0, 0, 0, 80]),
                }),
                rounding: 4.0, text_color: None,
            },
            start_button: None,
            panel: ElementStyle {
                fill: FillStyle::Solid { color: ThemeColor::Palette(PaletteRef::Panel) },
                border: None, shadow: None, rounding: 4.0, text_color: None,
            },
            scrollbar: None,
        }
    }

    pub fn builtin_desktop_styles() -> Vec<DesktopStyle> {
        vec![Self::flat(), Self::modern()]
    }
}
```

---

### Step 3: Add rendering helpers

In `src/native/retro_ui.rs`, add:

```rust
use nucleon_shared::theme::{ElementStyle, FillStyle, ThemeColor, PaletteRef, ShadowStyle, BorderStyle};

/// Resolve a ThemeColor to a concrete Color32 using the given palette.
pub fn resolve_theme_color(color: &ThemeColor, palette: &RetroPalette) -> Color32 {
    match color {
        ThemeColor::Rgba(rgba) => Color32::from_rgba_premultiplied(rgba[0], rgba[1], rgba[2], rgba[3]),
        ThemeColor::Palette(r) => match r {
            PaletteRef::Fg => palette.fg,
            PaletteRef::Dim => palette.dim,
            PaletteRef::Bg => palette.bg,
            PaletteRef::Panel => palette.panel,
            PaletteRef::SelectedBg => palette.selected_bg,
            PaletteRef::SelectedFg => palette.selected_fg,
            PaletteRef::HoveredBg => palette.hovered_bg,
            PaletteRef::ActiveBg => palette.active_bg,
            PaletteRef::SelectionBg => palette.selection_bg,
            PaletteRef::WindowChrome => palette.window_chrome,
            PaletteRef::WindowChromeFocused => palette.window_chrome_focused,
            PaletteRef::BarBg => palette.bar_bg,
        },
    }
}

/// Build an egui::Frame from an ElementStyle (solid fills only — gradients painted separately).
pub fn frame_from_element_style(style: &ElementStyle, palette: &RetroPalette) -> egui::Frame {
    let mut frame = egui::Frame::none();
    if let FillStyle::Solid { color } = &style.fill {
        frame = frame.fill(resolve_theme_color(color, palette));
    }
    if let Some(border) = &style.border {
        frame = frame.stroke(egui::Stroke::new(
            border.width, resolve_theme_color(&border.color, palette),
        ));
    }
    frame = frame.rounding(style.rounding);
    if let Some(shadow) = &style.shadow {
        frame = frame.shadow(egui::Shadow {
            offset: egui::vec2(shadow.offset_x, shadow.offset_y),
            blur: shadow.blur, spread: 0.0,
            color: resolve_theme_color(&shadow.color, palette),
        });
    }
    frame
}

/// Paint an N-stop gradient fill as a vertex-colored triangle strip.
pub fn paint_gradient_fill(
    painter: &egui::Painter,
    rect: egui::Rect,
    style: &ElementStyle,
    palette: &RetroPalette,
) {
    let FillStyle::LinearGradient { stops, angle } = &style.fill else { return };
    if stops.len() < 2 { return }

    let mut mesh = egui::Mesh::default();
    let is_vertical = *angle == 0.0 || *angle == 180.0;
    let reversed = *angle == 180.0 || *angle == 270.0;

    for stop in stops {
        let t = if reversed { 1.0 - stop.position } else { stop.position };
        let color = resolve_theme_color(&stop.color, palette);
        if is_vertical {
            let y = rect.top() + t * rect.height();
            mesh.vertices.push(egui::epaint::Vertex { pos: egui::pos2(rect.left(), y), uv: egui::epaint::WHITE_UV, color });
            mesh.vertices.push(egui::epaint::Vertex { pos: egui::pos2(rect.right(), y), uv: egui::epaint::WHITE_UV, color });
        } else {
            let x = rect.left() + t * rect.width();
            mesh.vertices.push(egui::epaint::Vertex { pos: egui::pos2(x, rect.top()), uv: egui::epaint::WHITE_UV, color });
            mesh.vertices.push(egui::epaint::Vertex { pos: egui::pos2(x, rect.bottom()), uv: egui::epaint::WHITE_UV, color });
        }
    }
    for i in 0..(stops.len() - 1) {
        let base = (i * 2) as u32;
        mesh.indices.extend_from_slice(&[base, base + 1, base + 3, base, base + 3, base + 2]);
    }
    painter.add(egui::Shape::mesh(mesh));
}

/// Resolve the text color for an element, falling back to palette.fg.
pub fn element_text_color(style: &ElementStyle, palette: &RetroPalette) -> Color32 {
    style.text_color.as_ref()
        .map(|c| resolve_theme_color(c, palette))
        .unwrap_or(palette.fg)
}
```

Only vertical (0/180) and horizontal (90/270) gradients are supported. These cover all real OS chrome needs.

---

### Step 4: Update desktop_window_frame()

Rewrite to consume `ElementStyle`:

```rust
pub(super) fn desktop_window_frame(&self) -> egui::Frame {
    let palette = current_palette();
    let style = current_desktop_style();
    frame_from_element_style(&style.window_frame, &palette)
        .inner_margin(egui::Margin::same(1.0))
}
```

For gradient window frames: `frame_from_element_style` returns a transparent frame. The gradient is painted by calling `paint_gradient_fill()` on the allocated rect after the frame is shown.

---

### Step 5: Update draw_desktop_window_header()

Add `focused: bool` parameter. Use `title_bar` or `title_bar_unfocused` based on focus state:

```rust
let element = if focused { &desktop_style.title_bar } else { &desktop_style.title_bar_unfocused };
let frame = frame_from_element_style(element, &palette);
// Paint gradient if needed after layout
if matches!(element.fill, FillStyle::LinearGradient { .. }) {
    paint_gradient_fill(ui.painter(), ui.max_rect(), element, &palette);
}
```

Update all call sites to pass `focused` (use `desktop_active_window` to determine focus).

---

### Step 6: Update draw_top_bar() and draw_desktop_taskbar()

Replace flat `palette.bar_bg` rect fill with ElementStyle consumption:

```rust
let style = current_desktop_style();
let bar_rect = ui.max_rect();
if let FillStyle::LinearGradient { .. } = &style.menu_bar.fill {
    paint_gradient_fill(ui.painter(), bar_rect, &style.menu_bar, &palette);
} else {
    let fill_color = match &style.menu_bar.fill {
        FillStyle::Solid { color } => resolve_theme_color(color, &palette),
        _ => palette.bar_bg,
    };
    ui.painter().rect_filled(bar_rect, style.menu_bar.rounding, fill_color);
}
if let Some(border) = &style.menu_bar.border {
    ui.painter().rect_stroke(bar_rect, style.menu_bar.rounding,
        egui::Stroke::new(border.width, resolve_theme_color(&border.color, &palette)));
}
```

Same pattern for taskbar using `style.taskbar`.

---

### Step 7: Update apply_global_retro_menu_chrome()

```rust
pub(super) fn apply_global_retro_menu_chrome(ctx: &Context, palette: &RetroPalette) {
    let style = current_desktop_style();
    let frame = frame_from_element_style(&style.menu_dropdown, palette);
    ctx.style_mut(|s| {
        s.visuals.window_stroke = frame.stroke;
        s.visuals.window_rounding = egui::Rounding::same(style.menu_dropdown.rounding);
        s.visuals.menu_rounding = egui::Rounding::same(style.menu_dropdown.rounding);
        s.visuals.window_shadow = frame.shadow;
        s.visuals.popup_shadow = frame.shadow;
    });
}
```

---

### Step 8: Update start menu rendering

In `desktop_start_menu.rs`, replace `palette.panel` fill with:
```rust
let style = current_desktop_style();
let frame = frame_from_element_style(&style.start_menu, &palette);
```

Paint gradient after layout if applicable.

---

### Step 9: Migrate DesktopStyle constructors

Update all `DesktopStyle` constructors (classic, nucleon) to use the new struct.
RobCo constructors have been removed — RobCo is external-only.

---

### Step 10: Settings persistence migration

With `#[serde(default)]` on all new ElementStyle fields, old serialized DesktopStyles
deserialize with defaults (Classic look). No explicit migration code needed.
Remove `border_radius` and `window_shadow` from the serialized format. Ensure
`#[serde(deny_unknown_fields)]` is NOT present on DesktopStyle.

---

### Step 11: Update ParkedSessionState

`desktop_active_desktop_style` clones the whole struct. No modification needed beyond
verifying `session_management.rs` compiles.

---

### Verification

1. `cargo check -p nucleon -p nucleon-native-shell`
2. Flat desktop style produces pixel-identical output to pre-refactor state
3. Window frame: flat bg fill + green border + no shadow + no rounding
4. Title bar: solid `window_chrome_focused` fill
5. Menu bar / taskbar: solid `bar_bg` fill
6. Menu dropdowns: solid bg + green border + no rounding + no shadow
7. Start menu: solid panel fill + green border
8. Old settings files load without error (defaults fill gaps)
9. A test DesktopStyle with gradient title bar renders correctly

---

# PHASE 13: Theme component decomposition

### Goal

Decompose the monolithic `ThemePack` into independently installable theme components.
Each component type (shell, colors, icons, sounds, cursors) becomes a standalone unit.
A ThemePack ("Global Theme") bundles references to one of each.

---

### Step 1: Define component manifest types

In `crates/shared/src/theme.rs`, add:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DesktopStyleManifest {
    pub id: String,
    pub name: String,
    pub description: String,
    pub version: String,
    pub style: DesktopStyle,
    /// Preferred font for this desktop style. User can override in Tweaks.
    #[serde(default)]
    pub font: Option<FontRef>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColorThemeManifest {
    pub id: String,
    pub name: String,
    pub description: String,
    pub version: String,
    pub color_style: ColorStyle,
    pub full_color_theme: Option<FullColorTheme>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IconPackManifest {
    pub id: String,
    pub name: String,
    pub description: String,
    pub version: String,
    pub asset_pack: AssetPack,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SoundPackManifest {
    pub id: String,
    pub name: String,
    pub description: String,
    pub version: String,
    pub sound_pack: SoundPack,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CursorPackManifest {
    pub id: String,
    pub name: String,
    pub description: String,
    pub version: String,
    pub cursor_pack: CursorPack,
}

/// How a theme or manifest references a font.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum FontRef {
    /// Built-in font shipped with the binary (e.g., "fixedsys").
    Builtin { id: String },
    /// Font file bundled in this theme/pack's directory.
    Bundled { file: String },
    /// Installed font pack referenced by ID.
    Installed { font_pack_id: String },
}

/// A standalone font pack installable from the themes repo.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FontPackManifest {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default = "default_theme_pack_version")]
    pub version: String,
    /// Relative path to the .ttf or .otf file within the pack directory.
    pub file: String,
}
```

---

### Step 2: Update ThemePack to reference components by ID

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemePack {
    pub id: String,
    pub name: String,
    pub description: String,
    pub version: String,

    /// Desktop style: inline or reference to installed desktop style.
    #[serde(default)]
    pub desktop_style: DesktopStyleRef,

    pub color_style: ColorStyle,
    #[serde(default)]
    pub full_color_theme: Option<FullColorTheme>,

    pub terminal_branding: TerminalBranding,
    pub terminal_decoration: TerminalDecoration,

    /// Component ID references. Missing = use default.
    #[serde(default)]
    pub icon_pack_id: Option<String>,
    #[serde(default)]
    pub sound_pack_id: Option<String>,
    #[serde(default)]
    pub cursor_pack_id: Option<String>,
    #[serde(default)]
    pub font_pack_id: Option<String>,
    #[serde(default)]
    pub layout_profile: LayoutProfile,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum DesktopStyleRef {
    Inline(DesktopStyle),
    ById { desktop_style_id: String },
}

impl Default for DesktopStyleRef {
    fn default() -> Self { DesktopStyleRef::Inline(DesktopStyle::flat()) }
}
```

---

### Step 3: Component directory structure

```
{state_dir}/themes/
    packs/             ← ThemePack manifests
    desktop/    ← DesktopStyleManifest
    terminal/   ← TerminalThemeManifest (Phase 15)
    colors/            ← ColorThemeManifest
    icons/             ← IconPackManifest (with icons_mono/, icons_color/)
    sounds/            ← SoundPackManifest (with .wav files)
    cursors/           ← CursorPackManifest
    fonts/             ← FontPackManifest (with .ttf/.otf files)
```

Add directory accessors in `config.rs`:
```rust
pub fn theme_packs_directory() -> PathBuf { themes_directory().join("packs") }
pub fn desktop_styles_directory() -> PathBuf { themes_directory().join("desktop") }
pub fn terminal_themes_directory() -> PathBuf { themes_directory().join("terminal") }
pub fn color_themes_directory() -> PathBuf { themes_directory().join("colors") }
pub fn icon_packs_directory() -> PathBuf { themes_directory().join("icons") }
pub fn sound_packs_directory() -> PathBuf { themes_directory().join("sounds") }
pub fn cursor_packs_directory() -> PathBuf { themes_directory().join("cursors") }
pub fn font_packs_directory() -> PathBuf { themes_directory().join("fonts") }
```

---

### Step 4: Component discovery functions

In `src/native/addons.rs`, add:
```rust
pub fn installed_desktop_styles() -> Vec<DesktopStyleManifest> { ... }
pub fn installed_color_themes() -> Vec<ColorThemeManifest> { ... }
pub fn installed_icon_packs() -> Vec<IconPackManifest> { ... }
pub fn installed_sound_packs() -> Vec<SoundPackManifest> { ... }
pub fn installed_cursor_packs() -> Vec<CursorPackManifest> { ... }
pub fn installed_font_packs() -> Vec<FontPackManifest> { ... }
```

Each scans its directory for `manifest.json`, parses, returns list. Built-in components
(Flat/Modern built-in desktop styles, monochrome presets) are prepended.

---

### Step 5: Extend AddonKind

```rust
pub enum AddonKind {
    App,
    Theme,           // existing — now means "theme pack / global theme"
    DesktopTheme,      // NEW — desktop style
    TerminalTheme,   // NEW — terminal shell theme (Phase 15)
    ColorTheme,      // NEW
    IconPack,        // NEW
    SoundPack,       // NEW
    CursorPack,      // NEW
    FontPack,        // NEW
    ContentPack,
    Game,
    Service,
}
```

Update `install_repository_addon_from_index()` to route each kind to its subdirectory.

---

### Step 6: Update Tweaks UI — Desktop mode component selectors

Remove the "Theme Pack" / "Global Theme" selector from the built-in Tweaks. There are
no built-in theme packs — `ThemePack::classic()` and `ThemePack::nucleon()` are removed.
The default state is `DesktopStyle::flat()` + `ColorStyle::Monochrome { preset: Green }`.

Replace with individual component selectors in `tweaks_presenter.rs`:

**Desktop Tweaks (desktop mode only):**
```
Desktop:       [Flat       v]    ← DesktopStyle (builtin: Flat, Modern + installed)
Colors:        [Monochrome v]    ← ColorStyle (existing)
Font:          [Fixedsys   v]    ← FontPack (builtin + installed)
Icons:         [Default    v]    ← IconPack (builtin + installed)
Sounds:        [Default    v]    ← SoundPack (builtin + installed)
Cursors:       [Default    v]    ← CursorPack (builtin + installed)
```

**Terminal Tweaks (terminal mode only):**
```
Colors:        [Monochrome v]    ← ColorStyle (existing, independent from desktop)
Font:          [Fixedsys   v]    ← FontPack (independent from desktop)
```

Terminal theme selection is deferred to Phase 15 (TerminalTheme).

Global theme packs (from the Addons app / themes repo) set all selectors at once when
installed and applied. Individual selectors override pack selections.

Remove `ThemePack::classic()`, `ThemePack::nucleon()`, and `builtin_theme_packs()`.
The `ThemePack` type still exists for repo-distributed packs, but no built-ins ship.

Add runtime fields to `NucleonNativeApp`:
```rust
pub(super) desktop_active_desktop_style_id: Option<String>,
pub(super) desktop_active_icon_pack_id: Option<String>,
pub(super) desktop_active_sound_pack_id: Option<String>,
pub(super) desktop_active_cursor_pack_id: Option<String>,
pub(super) desktop_active_font_id: Option<String>,     // None = builtin "fixedsys"
pub(super) terminal_active_font_id: Option<String>,     // None = builtin "fixedsys"
pub(super) font_cache: HashMap<String, Vec<u8>>,        // font id → loaded bytes
pub(super) current_applied_font_id: Option<String>,     // tracks active font to avoid redundant set_fonts()
```

---

### Step 7: Pack installation decomposes into components

When a ThemePack is installed, if it bundles embedded component manifests:
```
robco/
    manifest.json           <- ThemePack
    shells/robco.json       <- embedded DesktopStyleManifest
    sounds/robco.json       <- embedded SoundPackManifest
```

The installer:
1. Installs pack manifest to `themes/packs/{id}/`
2. Scans for embedded component manifests in subdirectories
3. Installs each component to its respective directory
4. Pack's component ID references now resolve to installed components

---

### Verification

1. `cargo check -p nucleon -p nucleon-native-shell`
2. Flat desktop style is default and produces identical output
3. Individual component installation works
4. Pack installation decomposes and installs embedded components
5. Tweaks "Global Theme" applies all components at once
6. Individual component selectors override pack selections
7. Components survive app restart (IDs persisted in settings)
8. Font selector appears in both Desktop and Terminal Tweaks
9. Font packs install to `themes/fonts/` and appear in the Font dropdown

---

# PHASE 14: Addons app

### Goal

Create a standalone Addons app that replaces the addon management section in the
Program Installer. The app provides a browsable, categorized interface for discovering,
installing, and managing addons and theme components from the two external repositories.

### Design overview

**Architecture:** Standalone app like Tweaks — own crate, own window, own binary.
Opens from Settings or from the desktop taskbar/start menu.

**Layout:** Left sidebar with category list. Right panel shows items for the selected
category. Items are fetched from the repo index and merged with locally installed state.

**Two repositories:**
- `nucleon-core-addons` — apps, content packs, games, services, tools, extras
- `nucleon-core-themes` — theme packs, desktop styles, color themes, icon packs, sound packs, cursor packs, font packs

**Sidebar categories:**
```
Installed            ← all installed items (addons + themes), with "View All" suboption
Addons               ← nested submenu:
  > Apps             ← application addons from addons repo
  > Games            ← game addons from addons repo
Themes               ← nested submenu:
  > Packs            ← global theme packs (ThemePack)
  > Desktop    ← desktop styles (DesktopStyleManifest)
  > Terminal   ← terminal themes (TerminalThemeManifest — Phase 15)
  > Colors           ← color themes (ColorThemeManifest, shared)
  > Icons            ← icon packs (IconPackManifest, desktop)
  > Sounds           ← sound packs (SoundPackManifest, shared)
  > Cursors          ← cursor packs (CursorPackManifest, desktop)
  > Fonts            ← font packs (FontPackManifest, shared)
Tools                ← runtime tools, CLI extensions (future)
Extras               ← miscellaneous (future)
```

**Right panel:** For each category, lists all available items from the repo. Each item shows:
- Name, version, description
- Install / Update / Uninstall button
- Status indicator (installed, update available, not installed)

---

### Step 1: Create crate `crates/native-addons-app`

**`crates/native-addons-app/Cargo.toml`:**
```toml
[package]
name = "nucleon-native-addons-app"
version = "0.4.4"
edition = "2021"
license = "GPL-3.0-only"

[dependencies]
nucleon = { path = "../.." }
eframe = { version = "0.29", default-features = false }
```

Copy version from root `Cargo.toml`.

**`crates/native-addons-app/src/lib.rs`:**
```rust
pub const ADDONS_APP_TITLE: &str = "Addons";
```

---

### Step 2: Add workspace member + binary entries

In root `Cargo.toml`, add `"crates/native-addons-app"` to `members` and `default-members`.

In `crates/native-shell/Cargo.toml`, add dependency:
```toml
nucleon-native-addons-app = { path = "../native-addons-app" }
```

Add binary entries:
```toml
[[bin]]
name = "nucleon-addons"
path = "src/addons_main.rs"

[[bin]]
name = "nucleon-addons"
path = "src/nucleon_addons_main.rs"
```

Create `crates/native-shell/src/addons_main.rs` and `nucleon_addons_main.rs` following
the exact same pattern as `tweaks_main.rs` / `nucleon_tweaks_main.rs`.

---

### Step 3: Create standalone launcher module

Create `src/native/addons_standalone.rs` following the exact pattern of
`src/native/tweaks_standalone.rs`:
- Define `NucleonNativeAddonsApp` struct
- Implement `eframe::App` for it
- `update()` calls `self.inner.update_standalone_addons_window(ctx)`
- Export from `src/native/mod.rs`

---

### Step 4: Add DesktopWindow::Addons variant

In `desktop_app.rs` (or wherever `DesktopWindow` is defined):
1. Add `Addons` to `DesktopWindow` enum
2. Add `Addons` to `DesktopHostedApp` enum
3. Add new `DesktopComponentBinding` entry (bump array size):

```rust
DesktopComponentBinding {
    spec: DesktopComponentSpec {
        window: DesktopWindow::Addons,
        hosted_app: DesktopHostedApp::Addons,
        id_salt: "native_addons",
        default_size: [900.0, 600.0],
        show_in_taskbar: true,
        show_in_window_menu: true,
        title_kind: DesktopTitleKind::Static("Addons"),
    },
    is_open: NucleonNativeApp::desktop_component_addons_is_open,
    set_open: NucleonNativeApp::desktop_component_addons_set_open,
    draw: NucleonNativeApp::desktop_component_addons_draw,
    on_open: None,
    on_closed: None,
}
```

---

### Step 5: Add addons state to NucleonNativeApp

In `src/native/app.rs`:
```rust
pub(super) addons_open: bool,
pub(super) addons_sidebar_category: AddonsSidebarCategory,
pub(super) addons_addon_subcategory: AddonsAddonSubcategory,
pub(super) addons_theme_subcategory: AddonsThemeSubcategory,
pub(super) addons_repo_cache: Option<AddonsRepoCache>,
pub(super) addons_repo_fetch_in_progress: bool,
```

Define the sidebar enums:
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AddonsSidebarCategory {
    #[default]
    Installed,
    Addons,
    Themes,
    Tools,
    Extras,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AddonsAddonSubcategory {
    #[default]
    Apps,
    Games,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AddonsThemeSubcategory {
    #[default]
    Packs,
    Desktop,
    Terminal,
    Colors,
    Icons,
    Sounds,
    Cursors,
}
```

Define the repo cache:
```rust
pub struct AddonsRepoCache {
    pub addons_index: Vec<RepoAddonEntry>,
    pub themes_index: Vec<RepoThemeEntry>,
    pub fetched_at: std::time::Instant,
}

pub struct RepoAddonEntry {
    pub id: String,
    pub name: String,
    pub description: String,
    pub version: String,
    pub kind: AddonKind,
    pub installed: bool,
    pub update_available: bool,
}

pub struct RepoThemeEntry {
    pub id: String,
    pub name: String,
    pub description: String,
    pub version: String,
    pub kind: AddonKind,  // Theme, DesktopTheme, ColorTheme, IconPack, SoundPack, CursorPack
    pub installed: bool,
    pub update_available: bool,
}
```

---

### Step 6: Create addons_presenter.rs

Create `src/native/app/addons_presenter.rs`. Add module declaration in `src/native/app.rs`.

This file contains `impl NucleonNativeApp` with:
```rust
pub(super) fn draw_addons(&mut self, ctx: &Context) { ... }
```

**Layout structure:**
```rust
fn draw_addons(&mut self, ctx: &Context) {
    // Left sidebar (fixed width ~200px)
    egui::SidePanel::left("addons_sidebar").exact_width(200.0).show(ctx, |ui| {
        self.draw_addons_sidebar(ui);
    });

    // Right content panel
    egui::CentralPanel::default().show(ctx, |ui| {
        self.draw_addons_content(ui);
    });
}
```

**Sidebar rendering:**
```rust
fn draw_addons_sidebar(&mut self, ui: &mut egui::Ui) {
    let categories = [
        ("Installed", AddonsSidebarCategory::Installed),
        ("Addons", AddonsSidebarCategory::Addons),
        ("Themes", AddonsSidebarCategory::Themes),
        ("Tools", AddonsSidebarCategory::Tools),
        ("Extras", AddonsSidebarCategory::Extras),
    ];
    for (label, cat) in &categories {
        let selected = self.addons_sidebar_category == *cat;
        if ui.selectable_label(selected, *label).clicked() {
            self.addons_sidebar_category = *cat;
        }
        // Nested addon subcategories
        if *cat == AddonsSidebarCategory::Addons && selected {
            ui.indent("addon_sub", |ui| {
                let subcats = [
                    ("Apps", AddonsAddonSubcategory::Apps),
                    ("Games", AddonsAddonSubcategory::Games),
                ];
                for (label, sub) in &subcats {
                    let sub_selected = self.addons_addon_subcategory == *sub;
                    if ui.selectable_label(sub_selected, *label).clicked() {
                        self.addons_addon_subcategory = *sub;
                    }
                }
            });
        }
        // Nested theme subcategories
        if *cat == AddonsSidebarCategory::Themes && selected {
            ui.indent("theme_sub", |ui| {
                let subcats = [
                    ("Packs", AddonsThemeSubcategory::Packs),
                    ("Desktop", AddonsThemeSubcategory::Desktop),
                    ("Terminal", AddonsThemeSubcategory::Terminal),
                    ("Colors", AddonsThemeSubcategory::Colors),
                    ("Icons", AddonsThemeSubcategory::Icons),
                    ("Sounds", AddonsThemeSubcategory::Sounds),
                    ("Cursors", AddonsThemeSubcategory::Cursors),
                ];
                for (label, sub) in &subcats {
                    let sub_selected = self.addons_theme_subcategory == *sub;
                    if ui.selectable_label(sub_selected, *label).clicked() {
                        self.addons_theme_subcategory = *sub;
                    }
                }
            });
        }
    }
}
```

**Content panel rendering:**

Based on the active sidebar category, filter the repo cache and show matching items.
Each item renders as a row with: name, version, description (truncated), and an
action button (Install / Update / Uninstall).

The mapping from sidebar category to AddonKind filter:
```
Installed             → all kinds where installed == true
Addons/Apps           → App, ContentPack, Service
Addons/Games          → Game
Themes/Packs          → Theme (AddonKind::Theme)
Themes/Desktop  → DesktopTheme (AddonKind::DesktopTheme)
Themes/Terminal  → TerminalTheme (AddonKind::TerminalTheme — Phase 15)
Themes/Colors         → ColorTheme
Themes/Icons          → IconPack
Themes/Sounds         → SoundPack
Themes/Cursors        → CursorPack
Themes/Fonts          → FontPack
Tools                 → (future — show empty for now)
Extras                → (future — show empty for now)
```

---

### Step 7: Repo index fetching

On first open of the Addons app (or when the user clicks a refresh button), fetch
both repo indexes in a background thread:

```rust
fn fetch_addons_repo_indexes(&mut self) {
    self.addons_repo_fetch_in_progress = true;
    // Spawn background task to fetch:
    //   nucleon-core-addons/index.json  → Vec<RepoAddonEntry>
    //   nucleon-core-themes/index.json  → Vec<RepoThemeEntry>
    // On completion, merge with installed state and store in addons_repo_cache.
}
```

The index format follows the existing addon repo structure. Theme entries use the same
JSON format but include a `kind` field to distinguish Packs/Desktop/Terminal/Colors/Icons/Sounds/Cursors.

**Repo index schema (themes):**
```json
{
  "entries": [
    {
      "id": "robco",
      "name": "RobCo",
      "description": "The original RobCo Industries terminal presentation.",
      "version": "1.0.0",
      "kind": "Theme",
      "download_url": "...",
      "checksum": "..."
    },
    {
      "id": "aero-glass",
      "name": "Aero Glass",
      "description": "Translucent gradient desktop style",
      "version": "1.0.0",
      "kind": "DesktopTheme",
      "download_url": "...",
      "checksum": "..."
    }
  ]
}
```

---

### Step 8: Install/uninstall actions

When the user clicks Install on an item:
1. Determine the `AddonKind` from the entry
2. Route to the correct install function:
   - `AddonKind::Theme` → install pack to `themes/packs/`, decompose embedded components
   - `AddonKind::DesktopTheme` → install to `themes/shells/`
   - `AddonKind::ColorTheme` → install to `themes/colors/`
   - `AddonKind::IconPack` → install to `themes/icons/`
   - `AddonKind::SoundPack` → install to `themes/sounds/`
   - `AddonKind::CursorPack` → install to `themes/cursors/`
   - `AddonKind::FontPack` → install to `themes/fonts/`
   - Other kinds → install via existing `install_repository_addon()` path
3. Update the installed state in the repo cache
4. Refresh the content panel

Uninstall works symmetrically: remove the manifest directory and update cache.

---

### Step 9: Settings integration

Add "Addons" tile to the Settings home grid. In `settings_screen.rs`, add a new tile
in the home grid (after Appearance):

```
Row 1: General, Appearance (opens Tweaks), Addons (opens Addons), Default Apps
Row 2: Connections, CLI Profiles, Edit Menus, User Management
Row 3: About
```

When the "Addons" tile is clicked, open the Addons window (same pattern as Appearance
opening Tweaks):
```rust
if panel == NativeSettingsPanel::Addons {
    self.open_addons_from_settings();
} else {
    next_panel = Some(panel);
}
```

Add `Addons` variant to `NativeSettingsPanel` enum. Add the tile definition with
glyph `[+]` or `[A]`.

---

### Step 10: Remove addon management from Program Installer

In `src/native/installer_screen.rs`:
1. Remove `InstallerView::AddonInventory` variant
2. Remove `InstallerView::AddonActions` variant
3. Remove "Installed Addons" menu item from the Root view
4. Remove all addon-related UI code from the installer
5. Keep: Search, Installed Apps, Runtime Tools, Package Manager

The installer now focuses exclusively on system package manager integration and
local app management.

---

### Step 11: Standalone prepare/update methods

In `src/native/app/desktop_runtime.rs`:
```rust
pub(crate) fn prepare_standalone_addons_window(&mut self, session_username: Option<String>) {
    self.prepare_standalone_window_shell(session_username, true);
    self.prime_desktop_window_defaults(DesktopWindow::Addons);
    self.addons_open = true;
    self.desktop_active_window = Some(WindowInstanceId::primary(DesktopWindow::Addons));
}

pub(crate) fn update_standalone_addons_window(&mut self, ctx: &Context) {
    self.process_background_results(ctx);
    self.maybe_sync_settings_from_disk(ctx);
    self.sync_native_appearance(ctx);
    self.draw_addons(ctx);
    if !self.addons_open {
        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
    }
    ctx.request_repaint_after(Duration::from_millis(500));
}
```

---

### Step 12: Add component bridge methods

Add the `desktop_component_addons_*` methods that `DesktopComponentBinding` references.
Follow the exact same pattern as `desktop_component_tweaks_*`:

```rust
fn desktop_component_addons_is_open(app: &NucleonNativeApp) -> bool { app.addons_open }
fn desktop_component_addons_set_open(app: &mut NucleonNativeApp, open: bool) { app.addons_open = open; }
fn desktop_component_addons_draw(app: &mut NucleonNativeApp, ctx: &Context) { app.draw_addons(ctx); }
```

---

### Verification

1. `cargo check -p nucleon -p nucleon-native-shell`
2. Addons app opens from Settings "Addons" tile
3. Addons app opens from start menu / taskbar
4. Left sidebar navigates between categories
5. Themes category expands nested subcategories
6. Right panel lists items from repo (or shows "fetching..." / "no items")
7. Install/uninstall buttons work and update displayed state
8. Theme pack installation decomposes into components
9. Installed view shows all installed addons and theme components
10. Program Installer no longer shows addon management
11. Standalone binary (`nucleon-addons`) launches correctly

---

# PHASE 15: Terminal theme system + Tweaks surface split

### Goal

Introduce `TerminalTheme` — a theme type that controls the entire terminal UI composition,
layout, and behavior. Each terminal theme declares a renderer (built-in or WASM in Phase 16)
and a dynamic options schema. Refactor the current hardcoded terminal UI into the `classic`
built-in renderer. Split Tweaks so desktop mode only shows desktop controls and terminal
mode only shows terminal controls.

---

### Step 1: Define theme option types

In `crates/shared/src/theme.rs`, add:

```rust
/// A single option that a terminal theme exposes to the user.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeOptionDef {
    /// Machine-readable key (e.g., "bracket_menus").
    pub key: String,
    /// Human-readable label shown in Tweaks (e.g., "Menus Between Brackets").
    pub label: String,
    /// Help text shown below the option.
    #[serde(default)]
    pub description: String,
    /// What kind of control this option is.
    pub kind: ThemeOptionKind,
}

/// The type and constraints of a theme option.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ThemeOptionKind {
    Bool { default: bool },
    Choice { choices: Vec<String>, default: String },
    Int { min: i32, max: i32, default: i32 },
    Float { min: f32, max: f32, default: f32 },
}

/// A concrete value for a theme option.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum ThemeOptionValue {
    Bool(bool),
    String(String),
    Int(i32),
    Float(f32),
}
```

---

### Step 2: Define TerminalTheme

```rust
/// Which renderer a terminal theme uses.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum TerminalRenderer {
    /// Built-in renderer shipped with the core. ID must match a registered renderer.
    Builtin { id: String },
    /// WASM module that owns screen rendering (Phase 16).
    Wasm { module: String },
}

/// A terminal theme that controls the entire terminal UI composition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminalTheme {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default = "default_theme_pack_version")]
    pub version: String,

    /// Which renderer handles screen drawing.
    pub renderer: TerminalRenderer,

    /// Preferred font for this terminal theme. User can override in Tweaks.
    #[serde(default)]
    pub font: Option<FontRef>,

    /// Header branding lines displayed at the top of the terminal when
    /// the "header_visible" option is true. Empty = no header.
    /// Example: ["ROBCO INDUSTRIES UNIFIED OPERATING SYSTEM", "COPYRIGHT 2075-2077 ROBCO INDUSTRIES", "-SERVER 1-"]
    #[serde(default)]
    pub branding_lines: Vec<String>,

    /// Options this theme exposes to the user. Displayed in terminal Tweaks
    /// when this theme is active.
    #[serde(default)]
    pub options_schema: Vec<ThemeOptionDef>,

    /// Default values for all options. Keys must match options_schema keys.
    #[serde(default)]
    pub default_options: HashMap<String, ThemeOptionValue>,
}

/// Manifest for a terminal theme installable from the themes repo.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminalThemeManifest {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default = "default_theme_pack_version")]
    pub version: String,
    pub theme: TerminalTheme,
}
```

Add `use std::collections::HashMap;` at the top of the file if not already present.

---

### Step 3: Create the built-in terminal themes

Two built-in renderers exist: `"dashboard"` (default) and `"robco"` (legacy, used by
the external RobCo theme). The default theme is Dashboard — a multi-panel
terminal interface with a persistent navigation sidebar and home screen widgets.

```rust
impl TerminalTheme {
    /// The default terminal theme. Multi-panel layout with nav sidebar and widgets.
    pub fn dashboard() -> Self {
        TerminalTheme {
            id: "dashboard".to_string(),
            name: "Dashboard".to_string(),
            description: "Multi-panel terminal with navigation sidebar and system widgets.".to_string(),
            version: "1.0.0".to_string(),
            renderer: TerminalRenderer::Builtin { id: "dashboard".to_string() },
            font: None,
            branding_lines: vec![],
            options_schema: vec![
                ThemeOptionDef {
                    key: "show_nav_panel".to_string(),
                    label: "Show Navigation Panel".to_string(),
                    description: "Display the persistent navigation sidebar.".to_string(),
                    kind: ThemeOptionKind::Bool { default: true },
                },
                ThemeOptionDef {
                    key: "nav_width".to_string(),
                    label: "Navigation Width".to_string(),
                    description: "Width of the navigation panel in columns.".to_string(),
                    kind: ThemeOptionKind::Int { min: 14, max: 30, default: 20 },
                },
                ThemeOptionDef {
                    key: "show_system_status".to_string(),
                    label: "Show System Status".to_string(),
                    description: "Display CPU, memory, and disk usage on the home screen.".to_string(),
                    kind: ThemeOptionKind::Bool { default: true },
                },
                ThemeOptionDef {
                    key: "show_recent_files".to_string(),
                    label: "Show Recent Files".to_string(),
                    description: "Display recently accessed files on the home screen.".to_string(),
                    kind: ThemeOptionKind::Bool { default: true },
                },
                ThemeOptionDef {
                    key: "show_quick_actions".to_string(),
                    label: "Show Quick Actions".to_string(),
                    description: "Display quick-launch tiles on the home screen.".to_string(),
                    kind: ThemeOptionKind::Bool { default: true },
                },
                ThemeOptionDef {
                    key: "clock_format".to_string(),
                    label: "Clock Format".to_string(),
                    description: "Time display format in the header bar.".to_string(),
                    kind: ThemeOptionKind::Choice {
                        choices: vec!["24h".to_string(), "12h".to_string()],
                        default: "24h".to_string(),
                    },
                },
                ThemeOptionDef {
                    key: "status_shortcuts".to_string(),
                    label: "Show Status Shortcuts".to_string(),
                    description: "Display keyboard shortcut hints in the status bar.".to_string(),
                    kind: ThemeOptionKind::Bool { default: true },
                },
            ],
            default_options: {
                let mut m = HashMap::new();
                m.insert("show_nav_panel".to_string(), ThemeOptionValue::Bool(true));
                m.insert("nav_width".to_string(), ThemeOptionValue::Int(20));
                m.insert("show_system_status".to_string(), ThemeOptionValue::Bool(true));
                m.insert("show_recent_files".to_string(), ThemeOptionValue::Bool(true));
                m.insert("show_quick_actions".to_string(), ThemeOptionValue::Bool(true));
                m.insert("clock_format".to_string(), ThemeOptionValue::String("24h".to_string()));
                m.insert("status_shortcuts".to_string(), ThemeOptionValue::Bool(true));
                m
            },
        }
    }

    pub fn builtin_terminal_themes() -> Vec<TerminalTheme> {
        vec![Self::dashboard()]
    }
}
```

**Dashboard renderer layout (92×28 grid):**

```
Row 0:      Header bar — "NUCLEON OS" left, clock + date right
Row 1:      Horizontal separator (full width)
Rows 2-24:  Two-column layout:
              Col 0–(nav_width-1):  Navigation panel (screen list, always visible)
              Col nav_width:        Vertical divider character (│)
              Col (nav_width+1)–91: Content panel (active screen)
Row 25:     Horizontal separator (full width)
Rows 26-27: Status bar — shortcuts left, session info right
```

**Navigation panel behavior:**
- Lists all available terminal screens as selectable rows
- "Home" is the first item — renders dashboard widgets in the content panel
- Other items render the corresponding screen within the content panel bounds
- Bottom of the nav panel (pinned to rows 21–23): `Desktop` and `Logout` items,
  separated from the main list by a thin separator. `Desktop` switches to desktop
  mode. `Logout` triggers session logout.
- Active item marked with `▸`, others indented with space
- Up/Down arrows navigate, Enter selects (or Left to focus nav, Right to focus content)

**Home screen widgets (rendered in content panel when "Home" is selected):**
- **System Status** (2 rows): CPU, MEM, DSK as filled-block progress bars (`████░░░░`) with
  percentages. Two columns to save vertical space.
- **Quick Actions** (1 row): Bracketed labels `[Applications]  [Documents]  [Terminal]`
  that navigate to those screens when selected.
- **Recent Files** (4-6 rows): Dot-leader aligned file names with relative timestamps.
  Selecting a file opens it in the editor.

**Content bounds for screen rendering:**
Existing screen draw functions must accept rendering bounds instead of assuming full-width:

```rust
pub struct ContentBounds {
    pub col_start: usize,    // First usable column
    pub col_end: usize,      // Last usable column
    pub row_start: usize,    // First usable row
    pub row_end: usize,      // Last usable row
}

impl ContentBounds {
    /// Full-width bounds (used by robco renderer).
    pub fn full() -> Self {
        ContentBounds { col_start: 3, col_end: 89, row_start: 3, row_end: 24 }
    }
    /// Dashboard content panel bounds.
    pub fn dashboard(nav_width: usize) -> Self {
        ContentBounds { col_start: nav_width + 2, col_end: 90, row_start: 3, row_end: 24 }
    }
}
```

The robco renderer passes `ContentBounds::full()`. The dashboard renderer passes
`ContentBounds::dashboard(nav_width)`. Screen draw functions use `bounds.col_start`
instead of hardcoded `TERMINAL_CONTENT_COL`.

**The robco renderer still exists** — it is the refactored existing terminal code.
The external RobCo theme (from `nucleon-core-themes`) references it via
`TerminalRenderer::Builtin { id: "robco" }`. No built-in theme uses it directly.

---

### Step 4: Add terminal theme state to NucleonNativeApp

In `src/native/app.rs`, add:

```rust
pub(super) terminal_active_theme: TerminalTheme,
pub(super) terminal_theme_options: HashMap<String, ThemeOptionValue>,
```

Initialize in `Default`:
```rust
terminal_active_theme: TerminalTheme::dashboard(),
terminal_theme_options: TerminalTheme::dashboard().default_options.clone(),
```

Add to `ParkedSessionState`:
```rust
pub terminal_active_theme: TerminalTheme,
pub terminal_theme_options: HashMap<String, ThemeOptionValue>,
```

**Font fields** are already added in Phase 13 Step 6 (`desktop_active_font_id`,
`terminal_active_font_id`, `font_cache`, `current_applied_font_id`). When a
terminal theme is applied and it declares `font: Some(FontRef::...)`, set
`terminal_active_font_id` to the resolved font ID (unless the user has manually
overridden the font in Tweaks).

---

### Step 5: Add option value accessor helpers

In `crates/shared/src/theme.rs`, add convenience methods:

```rust
impl TerminalTheme {
    /// Get a bool option value, falling back to schema default if not in the options map.
    pub fn get_bool(options: &HashMap<String, ThemeOptionValue>, key: &str, schema: &[ThemeOptionDef]) -> bool {
        if let Some(ThemeOptionValue::Bool(v)) = options.get(key) {
            return *v;
        }
        for def in schema {
            if def.key == key {
                if let ThemeOptionKind::Bool { default } = &def.kind {
                    return *default;
                }
            }
        }
        false
    }

    /// Get a string option value (for Choice options).
    pub fn get_string(options: &HashMap<String, ThemeOptionValue>, key: &str, schema: &[ThemeOptionDef]) -> String {
        if let Some(ThemeOptionValue::String(v)) = options.get(key) {
            return v.clone();
        }
        for def in schema {
            if def.key == key {
                if let ThemeOptionKind::Choice { default, .. } = &def.kind {
                    return default.clone();
                }
            }
        }
        String::new()
    }

    /// Get an int option value.
    pub fn get_int(options: &HashMap<String, ThemeOptionValue>, key: &str, schema: &[ThemeOptionDef]) -> i32 {
        if let Some(ThemeOptionValue::Int(v)) = options.get(key) {
            return *v;
        }
        for def in schema {
            if def.key == key {
                if let ThemeOptionKind::Int { default, .. } = &def.kind {
                    return *default;
                }
            }
        }
        0
    }

    /// Get a float option value.
    pub fn get_float(options: &HashMap<String, ThemeOptionValue>, key: &str, schema: &[ThemeOptionDef]) -> f32 {
        if let Some(ThemeOptionValue::Float(v)) = options.get(key) {
            return *v;
        }
        for def in schema {
            if def.key == key {
                if let ThemeOptionKind::Float { default, .. } = &def.kind {
                    return *default;
                }
            }
        }
        0.0
    }
}
```

---

### Step 6: Refactor terminal rendering into the robco renderer

The current terminal rendering code becomes the `robco` built-in renderer. This
renderer is used by the external RobCo theme. The current hardcoded reads
from `TerminalDecoration` are replaced with theme option lookups.

**In `src/native/retro_ui.rs` — `selectable_row()`:**

The `native_terminal_ui_highlighting` setting check becomes a theme option read.
Add a thread-local for the active terminal theme options (same pattern as `current_desktop_style()`):

```rust
thread_local! {
    static ACTIVE_TERMINAL_OPTIONS: RefCell<HashMap<String, ThemeOptionValue>> = RefCell::new(HashMap::new());
    static ACTIVE_TERMINAL_SCHEMA: RefCell<Vec<ThemeOptionDef>> = RefCell::new(Vec::new());
}

pub fn set_active_terminal_theme(theme: &TerminalTheme, options: &HashMap<String, ThemeOptionValue>) {
    ACTIVE_TERMINAL_OPTIONS.with(|o| *o.borrow_mut() = options.clone());
    ACTIVE_TERMINAL_SCHEMA.with(|s| *s.borrow_mut() = theme.options_schema.clone());
}

pub fn terminal_option_bool(key: &str) -> bool {
    ACTIVE_TERMINAL_OPTIONS.with(|o| {
        ACTIVE_TERMINAL_SCHEMA.with(|s| {
            TerminalTheme::get_bool(&o.borrow(), key, &s.borrow())
        })
    })
}

pub fn terminal_option_string(key: &str) -> String {
    ACTIVE_TERMINAL_OPTIONS.with(|o| {
        ACTIVE_TERMINAL_SCHEMA.with(|s| {
            TerminalTheme::get_string(&o.borrow(), key, &s.borrow())
        })
    })
}

pub fn terminal_option_int(key: &str) -> i32 {
    ACTIVE_TERMINAL_OPTIONS.with(|o| {
        ACTIVE_TERMINAL_SCHEMA.with(|s| {
            TerminalTheme::get_int(&o.borrow(), key, &s.borrow())
        })
    })
}
```

Call `set_active_terminal_theme()` each frame from `sync_native_appearance()` in `app.rs`,
passing `self.terminal_active_theme` and `self.terminal_theme_options`.

**Replace hardcoded values in terminal screen functions:**

Where the code currently reads `decoration.separator_char`:
```rust
// Before:
let sep = &decoration.separator_char;
// After:
let sep = terminal_option_string("separator_char");
```

Where the code reads `decoration.show_separators`:
```rust
// Before:
if decoration.show_separators { ... }
// After:
if terminal_option_bool("show_separators") { ... }
```

Where the code reads `decoration.subtitle_underlined`:
```rust
// Before:
if decoration.subtitle_underlined { ... }
// After:
if terminal_option_bool("subtitle_underlined") { ... }
```

Where `selectable_row()` checks `native_terminal_ui_highlighting`:
```rust
// Before:
let extend_highlight = selected && get_settings().native_terminal_ui_highlighting;
// After:
let extend_highlight = selected && terminal_option_string("selection_style") == "Full Row";
```

Where the code uses the hardcoded `"> "` prefix for selected items:
```rust
// Before:
let prefix = if selected { "> " } else { "  " };
// After:
let marker = terminal_option_string("selection_marker");
let prefix = if selected { &marker } else { &" ".repeat(marker.len()) };
```

Where `content_col` is calculated:
```rust
// Before:
let content_col = 3; // hardcoded
// After:
let content_col = terminal_option_int("content_margin") as usize;
```

Where menu alignment is used:
```rust
// Before:
screen.text(content_col, row, label, palette.fg); // always left-aligned
// After:
let alignment = terminal_option_string("menu_alignment");
if alignment == "Center" {
    screen.centered_text(row, label, palette.fg, false);
} else {
    screen.text(content_col, row, label, palette.fg);
}
```

Where the header/branding is conditionally shown:
```rust
// Before:
if !terminal_branding.header_lines.is_empty() { ... }
// After:
if terminal_option_bool("header_visible") && !terminal_branding.header_lines.is_empty() { ... }
```

**Important:** Do NOT remove `TerminalDecoration` or `TerminalBranding` from the codebase yet.
They still exist on the `ThemePack` type for backwards compatibility. The robco renderer
reads from theme options instead; existing `TerminalDecoration` values are used to initialize
the classic theme's default options when a theme pack is applied.

**Add `ContentBounds` parameter to all screen draw functions:**

All terminal screen draw functions (e.g., `draw_terminal_main_menu()`,
`draw_terminal_applications()`, etc.) must accept a `&ContentBounds` parameter
that defines the usable column/row range. Replace all hardcoded `TERMINAL_CONTENT_COL`
references with `bounds.col_start`. The robco renderer passes
`ContentBounds::full()`; the dashboard renderer passes
`ContentBounds::dashboard(nav_width)`. This allows the same screen code to render
in either full-width or sidebar-constrained layouts.

---

### Step 7: Apply theme pack terminal settings to theme options

When a `ThemePack` is applied, translate its `TerminalDecoration` and `TerminalBranding`
into the classic theme's options:

```rust
fn apply_theme_pack_to_terminal_options(
    pack: &ThemePack,
    theme: &TerminalTheme,
    options: &mut HashMap<String, ThemeOptionValue>,
) {
    // Map TerminalDecoration fields to classic theme options
    options.insert("separator_char".to_string(),
        ThemeOptionValue::String(pack.terminal_decoration.separator_char.clone()));
    options.insert("show_separators".to_string(),
        ThemeOptionValue::Bool(pack.terminal_decoration.show_separators));
    options.insert("subtitle_underlined".to_string(),
        ThemeOptionValue::Bool(pack.terminal_decoration.subtitle_underlined));
    options.insert("header_visible".to_string(),
        ThemeOptionValue::Bool(!pack.terminal_branding.header_lines.is_empty()));
}
```

This bridges the old ThemePack data into the new options system without breaking existing packs.

---

### Step 7b: Implement the Dashboard renderer

Create the `dashboard` built-in renderer. This is the default terminal experience.

**Renderer dispatch:** In `draw_terminal_runtime()` (or the slot renderer), check
`terminal_active_theme.renderer`:
```rust
match &self.terminal_active_theme.renderer {
    TerminalRenderer::Builtin { id } if id == "dashboard" => {
        self.render_dashboard_terminal(ctx, &screen);
    }
    TerminalRenderer::Builtin { id } if id == "robco" => {
        self.render_robco_terminal(ctx, &screen);
    }
    _ => {
        self.render_dashboard_terminal(ctx, &screen); // fallback to default
    }
}
```

**`render_dashboard_terminal()`:** Draws the full dashboard layout on the 92×28 grid:

1. **Header bar (row 0):** `"NUCLEON OS"` left-aligned at col 2. Clock and date
   right-aligned. Use `terminal_option_string("clock_format")` for 12h/24h.

2. **Top separator (row 1):** Full-width separator using `═` or `─`.

3. **Navigation panel (rows 2–24, col 0–nav_width-1):**
   Draw `boxed_panel()` or direct character rendering. List all terminal screens:
   ```
   Home, Applications, Documents, Network, Games, Programs,
   Logs, Settings, Connections, Default Apps, About
   ```
   Active item rendered with `▸` prefix, others with `  ` (2-space indent).
   Use `selectable_row()` within the nav column bounds for click support.
   Track `self.dashboard_nav_index: usize` for keyboard navigation.

4. **Vertical divider (col nav_width, rows 2–24):** Draw `│` character in each row.

5. **Content panel (rows 2–24, col nav_width+1–91):**
   - If "Home" is selected: render dashboard home widgets (see below).
   - Otherwise: call the existing screen draw function with
     `ContentBounds::dashboard(nav_width)` to constrain rendering.

6. **Bottom separator (row 25):** Full-width separator.

7. **Status bar (rows 26–27):** If `terminal_option_bool("status_shortcuts")`:
   `"F1 Help  F2 Shell  F5 Refresh"` left-aligned. Session info right-aligned.

**Dashboard home widgets (rendered in content panel):**

```rust
fn render_dashboard_home(&mut self, screen: &RetroScreen, painter: &Painter, palette: &RetroPalette, bounds: &ContentBounds) {
    let col = bounds.col_start;
    let mut row = bounds.row_start;

    // System Status
    if terminal_option_bool("show_system_status") {
        screen.text(painter, col, row, "System", palette.fg);
        row += 1;
        screen.separator(painter, row, palette); // or themed_separator
        row += 1;
        // CPU bar: use sysinfo or mock data
        let cpu_bar = format_progress_bar("CPU", cpu_percent, 16);
        let mem_bar = format_progress_bar("MEM", mem_percent, 16);
        screen.text(painter, col, row, &cpu_bar, palette.fg);
        screen.text(painter, col + 35, row, &mem_bar, palette.fg);
        row += 1;
        let dsk_bar = format_progress_bar("DSK", disk_percent, 16);
        screen.text(painter, col, row, &dsk_bar, palette.fg);
        row += 2;
    }

    // Quick Actions
    if terminal_option_bool("show_quick_actions") {
        screen.text(painter, col, row, "Navigation", palette.fg);
        row += 1;
        screen.separator(painter, row, palette);
        row += 1;
        // Render bracketed action tiles as selectable items
        // [Applications]  [Documents]  [Terminal]  [Settings]
        row += 2;
    }

    // Recent Files
    if terminal_option_bool("show_recent_files") {
        screen.text(painter, col, row, "Recent Files", palette.fg);
        row += 1;
        screen.separator(painter, row, palette);
        row += 1;
        // List recent files with dot-leader alignment
        // file.txt ··············· 2m ago
    }
}
```

**Progress bar helper:**
```rust
fn format_progress_bar(label: &str, percent: u8, width: usize) -> String {
    let filled = (width as f32 * percent as f32 / 100.0) as usize;
    let empty = width - filled;
    format!("{}  {}{}  {}%",
        label,
        "█".repeat(filled),
        "░".repeat(empty),
        percent
    )
}
```

**System info:** For CPU/MEM/DSK percentages, use the `sysinfo` crate if available,
otherwise show placeholder values. This is cosmetic — exact values are not critical
for the theme to function.

**Navigation input handling:** Add keyboard handling in the dashboard renderer:
- Up/Down: move `dashboard_nav_index` through the screen list
- Enter: select the screen (sets `terminal_nav.screen`)
- Left: focus navigation panel (if content panel was focused)
- Right: focus content panel (if nav panel was focused)
- Tab: toggle focus between nav and content

Add `dashboard_nav_focused: bool` to track which panel has keyboard focus.

**App state additions:**
```rust
pub(super) dashboard_nav_index: usize,           // 0 = Home
pub(super) dashboard_nav_focused: bool,           // true = nav panel has focus
pub(super) dashboard_recent_files: Vec<(String, std::time::SystemTime)>,
```

---

### Step 8: Split Tweaks into desktop-only and terminal-only

**Desktop Tweaks (`tweaks_presenter.rs` — desktop mode):**

Remove all terminal-related controls. The Theme tab shows only desktop controls:
```
Desktop:   [Flat       v]    ← DesktopStyle selector (builtin + installed)
Colors:    [Monochrome v]    ← Desktop color style
Font:      [Fixedsys   v]    ← Desktop font (builtin + installed)
Icons:     [Default    v]    ← Icon pack selector
Sounds:    [Default    v]    ← Sound pack selector
Cursors:   [Default    v]    ← Cursor pack selector
```

Remove the Desktop/Terminal sub-tab switcher from the Theme tab. Desktop Tweaks
controls ONLY the desktop surface.

**Terminal Tweaks (`tweaks_presenter.rs` — terminal mode):**

Replace the current terminal Tweaks sections with:
```
Terminal:  [Dashboard  v]    ← TerminalTheme selector (builtin + installed)
Colors:    [Monochrome v]    ← Terminal color style (independent from desktop)
Font:      [Fixedsys   v]    ← Terminal font (independent from desktop, theme can set default)
─────────────────────────────
Theme Options:               ← dynamically generated from active theme's options_schema
  Separator Character  [= v]
  Show Separators      [ON]
  Menu Alignment       [Left v]
  Selection Style      [Full Row v]
  Selection Marker     [>  v]
  Show Header          [OFF]
  Underline Subtitle   [ON]
  Content Margin       [===-----] 3
```

The "Theme Options" section is generated dynamically from `terminal_active_theme.options_schema`.
When the user switches terminal themes, the options section completely changes to show
that theme's schema with that theme's default values.

---

### Step 9: Dynamic option controls in terminal Tweaks

For the terminal-native Tweaks (character grid UI), render each option based on its kind:

```rust
fn draw_terminal_theme_option(
    screen: &RetroScreen,
    row: usize,
    col: usize,
    def: &ThemeOptionDef,
    value: &ThemeOptionValue,
    selected: bool,
    palette: &RetroPalette,
) {
    match &def.kind {
        ThemeOptionKind::Bool { .. } => {
            let on = matches!(value, ThemeOptionValue::Bool(true));
            let label = format!("{}: [{}]", def.label, if on { "ON" } else { "OFF" });
            screen.selectable_row(col, row, &label, selected);
        }
        ThemeOptionKind::Choice { choices, .. } => {
            let current = match value {
                ThemeOptionValue::String(s) => s.as_str(),
                _ => "",
            };
            let label = format!("{}: [{}]", def.label, current);
            screen.selectable_row(col, row, &label, selected);
        }
        ThemeOptionKind::Int { min, max, .. } => {
            let v = match value { ThemeOptionValue::Int(i) => *i, _ => 0 };
            let label = format!("{}: {}", def.label, v);
            screen.selectable_row(col, row, &label, selected);
            // Left/Right arrow adjusts value within min..max
        }
        ThemeOptionKind::Float { min, max, .. } => {
            let v = match value { ThemeOptionValue::Float(f) => *f, _ => 0.0 };
            let label = format!("{}: {:.1}", def.label, v);
            screen.selectable_row(col, row, &label, selected);
        }
    }
}
```

**Input handling for options:**
- `Bool`: Enter/Space toggles the value
- `Choice`: Enter opens a choice overlay (same pattern as existing dropdown overlays), or Left/Right cycles through choices
- `Int`: Left/Right decrements/increments within min..max
- `Float`: Left/Right adjusts by 0.1 within min..max

For the desktop Tweaks (egui window), render each option using egui widgets:
- `Bool` → `ui.checkbox()`
- `Choice` → `egui::ComboBox`
- `Int` → `ui.add(egui::Slider::new(...))`
- `Float` → `ui.add(egui::Slider::new(...))`

---

### Step 10: Persist terminal theme selection and options

Add to `Settings` in `config.rs`:
```rust
#[serde(default)]
pub terminal_theme_id: Option<String>,
#[serde(default)]
pub terminal_theme_options: HashMap<String, ThemeOptionValue>,
#[serde(default)]
pub desktop_font_id: Option<String>,
#[serde(default)]
pub terminal_font_id: Option<String>,
```

On theme change or option change, persist to settings. On app startup, load the
persisted theme ID, look it up in `builtin_terminal_themes()` + installed themes,
and restore the options.

If the persisted theme ID is not found (theme was uninstalled), fall back to
`TerminalTheme::dashboard()` with default options.

---

### Step 11: Terminal theme discovery

In `src/native/addons.rs`, add:
```rust
pub fn installed_terminal_themes() -> Vec<TerminalThemeManifest> {
    // Scan terminal_themes_directory() for manifest.json files
    // Prepend built-in themes (Classic)
    // Return merged list
}
```

---

### Step 12: Remove `native_terminal_ui_highlighting` setting

The `native_terminal_ui_highlighting` boolean in `Settings` is now superseded by the
`selection_style` theme option. Remove the setting field. The classic theme's
"Selection Style" option replaces it.

Migrate: if `native_terminal_ui_highlighting` was `true` in persisted settings,
set `selection_style` to "Full Row" in the terminal theme options. If `false`,
set to "Text Only".

---

### Step 13: Font loading and switching

Refactor `configure_native_fonts()` in `app.rs` to support dynamic font switching.

**Built-in fonts:** The existing `FixedsysExcelsior301-Regular.ttf` is registered as
the built-in font with ID `"fixedsys"`. This remains the default for both surfaces.

**Font cache:** On startup, scan `font_packs_directory()` for installed font packs.
Load each font's bytes into `font_cache: HashMap<String, Vec<u8>>` keyed by pack ID.
Also load any theme-bundled fonts (from active DesktopStyle or TerminalTheme directories).

**Font switching per frame:** In `sync_native_appearance()`:
```rust
let needed_font_id = if self.is_terminal_mode() {
    self.terminal_active_font_id.clone()
} else {
    self.desktop_active_font_id.clone()
};
// None means builtin "fixedsys"
if needed_font_id != self.current_applied_font_id {
    self.apply_active_font(ctx, &needed_font_id);
    self.current_applied_font_id = needed_font_id;
}
```

**`apply_active_font()`:** Rebuilds `FontDefinitions` with the target font as
the primary for both `FontFamily::Monospace` and `FontFamily::Proportional`:
```rust
fn apply_active_font(&self, ctx: &Context, font_id: &Option<String>) {
    let mut fonts = FontDefinitions::default();
    let (name, bytes) = match font_id {
        None => ("fixedsys".to_string(), include_bytes!("../assets/fonts/FixedsysExcelsior301-Regular.ttf").to_vec()),
        Some(id) => {
            if let Some(cached) = self.font_cache.get(id) {
                (id.clone(), cached.clone())
            } else {
                // Fallback to builtin if font not found
                ("fixedsys".to_string(), include_bytes!("../assets/fonts/FixedsysExcelsior301-Regular.ttf").to_vec())
            }
        }
    };
    fonts.font_data.insert(name.clone(), FontData::from_owned(bytes));
    fonts.families.entry(FontFamily::Monospace)
        .or_default()
        .insert(0, name.clone());
    fonts.families.entry(FontFamily::Proportional)
        .or_default()
        .insert(0, name);
    ctx.set_fonts(fonts);
}
```

**Theme font application:** When a TerminalTheme with `font: Some(ref)` is applied,
resolve the FontRef to a font ID and set `terminal_active_font_id`. When a
DesktopStyleManifest with `font: Some(ref)` is applied, set `desktop_active_font_id`.
User overrides in Tweaks take precedence — only apply theme fonts if the user hasn't
manually selected a different font.

**Font selector in Tweaks:** Both Desktop and Terminal Tweaks show a Font dropdown
that lists: "Fixedsys (Default)" + all installed font packs by name. Selecting a font
updates `desktop_active_font_id` or `terminal_active_font_id` and persists to settings.

---

### Verification

1. `cargo check -p nucleon -p nucleon-native-shell`
2. Dashboard terminal theme renders the multi-panel layout: nav sidebar, content panel, header bar, status bar
3. Terminal Tweaks shows: theme selector, colors, font, dynamic options
4. Desktop Tweaks shows: desktop style, colors, font, icons, sounds, cursors — no terminal controls
5. Changing a theme option (e.g., separator char) immediately affects terminal rendering
6. Switching terminal themes changes the options section dynamically
7. Theme selection and option values survive app restart
8. Session switching preserves terminal theme state
9. The `header_visible` option works: OFF hides header, ON shows it (when branding lines exist)
10. The `selection_style` option works: "Full Row" extends highlight to edge, "Text Only" does not
11. The `menu_alignment` option works: "Center" centers menu items, "Left" left-aligns
12. The `content_margin` option adjusts the left margin of terminal content
13. Font selector shows builtin + installed fonts
14. Changing font in Desktop Tweaks only affects desktop mode rendering
15. Changing font in Terminal Tweaks only affects terminal mode rendering
16. Applying a terminal theme with a bundled font sets the terminal font
17. Font selection survives app restart

---

# Architecture constraints

### WM compatibility

The future Window Manager will own window decoration rendering. Everything in these
phases must be consumable by a WM without rewriting:

1. **DesktopStyle is pure data** — no rendering logic, no closures, no trait impls. Serializable struct that renderers consume.
2. **Decoration rendering stays in `desktop_window_frame()` and `draw_desktop_window_header()`** — these are the two functions the WM will replace. All new styling flows through them.
3. **No per-window style overrides** — all windows use the active DesktopStyle. The WM may add per-window overrides later.
4. **`ManagedWindow` is not modified** — dead code waiting for the WM phase.

### Performance

Desktop styles must be rendering-cheap:
- Gradients use vertex-colored meshes (2 triangles per rect) — zero extra GPU cost
- Shadows use `egui::Shadow` (built-in, already optimized)
- No texture sampling, no blur, no compositing, no shaders
- All painting is immediate-mode `egui::Painter` calls

### Desktop style rendering ownership

1. **All element fill rendering goes through `frame_from_element_style()` and `paint_gradient_fill()`.** No alternative rendering paths.
2. **ElementStyle is the WM's decoration contract.** The WM will read `title_bar`, `title_bar_unfocused`, and `window_frame` from the active DesktopStyle for external windows.
3. **ThemeColor::Palette references are resolved at render time.** Never cache resolved colors — the palette can change between frames.

### RobCo theme

RobCo is external-only. It lives in `nucleon-core-themes` repo as a terminal theme
at `terminal/robco/manifest.json`. All Heritage constructors (`ThemePack::heritage()`,
`TerminalBranding::heritage()`) have been removed from the codebase. The RobCo
theme is maintained exclusively in the external themes repository. It uses the
built-in `robco` renderer (`TerminalRenderer::Builtin { id: "robco" }`).

---

# Rules for Codex

1. After each step, run `cargo check -p nucleon -p nucleon-native-shell`. Fix all errors before proceeding.
2. Do not add features not described in this spec.
3. Do not rename types, fields, or functions unless the spec says to.
4. Do not refactor surrounding code while implementing a step.
5. When a step says "follow the pattern of X," read X first and replicate exactly.
6. When adding new files, add the `mod` declaration and any necessary `pub use` re-exports.
7. Test the Flat desktop style at every phase boundary — it must be pixel-identical.
8. Do not modify `ManagedWindow` or `WindowSource`. They are WM scaffolds.
9. Gradients: only vertical (0/180) and horizontal (90/270) angles. No arbitrary angles.
10. All `ThemeColor::Palette` references resolve at render time via `resolve_theme_color()`. Never cache.
11. Theme component directories: `themes/{type}/{id}/manifest.json`.
12. Pack installation must decompose embedded component manifests into their respective directories.
13. The Addons app sidebar uses `egui::SidePanel::left`. The content uses `egui::CentralPanel`.
14. Repo indexes are fetched in background threads. Never block the UI thread on network.
15. When in doubt, read the existing code pattern first. The codebase is consistent — replicate, don't innovate.
16. Terminal theme options are accessed via `terminal_option_bool()`, `terminal_option_string()`, `terminal_option_int()` thread-local helpers. Never read `TerminalDecoration` fields directly in rendering code — use theme options.
17. Desktop Tweaks must not show terminal controls. Terminal Tweaks must not show desktop controls. Each surface is fully independent.
18. Font selection is per-surface. Desktop and terminal each have their own active font ID. Switching between modes triggers a font swap via `ctx.set_fonts()`. Never call `set_fonts()` if the font hasn't changed.
19. Themes can declare a preferred font via `font: Option<FontRef>`. Theme fonts are applied as defaults — user overrides in Tweaks always take precedence.
