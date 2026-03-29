use serde::{Deserialize, Deserializer, Serialize};
use std::collections::HashMap;

fn default_theme_pack_version() -> String {
    "1.0.0".to_string()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MonochromePreset {
    Green,
    White,
    Amber,
    Blue,
    LightBlue,
    Custom,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ColorStyle {
    Monochrome {
        preset: MonochromePreset,
        custom_rgb: Option<[u8; 3]>,
    },
    FullColor {
        theme_id: String,
    },
}

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

impl ColorToken {
    pub fn all() -> &'static [ColorToken] {
        &[
            ColorToken::BgPrimary,
            ColorToken::BgSecondary,
            ColorToken::FgPrimary,
            ColorToken::FgSecondary,
            ColorToken::FgDim,
            ColorToken::Accent,
            ColorToken::AccentHover,
            ColorToken::AccentActive,
            ColorToken::PanelBg,
            ColorToken::PanelBorder,
            ColorToken::WindowChrome,
            ColorToken::WindowChromeFocused,
            ColorToken::Selection,
            ColorToken::SelectionFg,
            ColorToken::Border,
            ColorToken::Separator,
            ColorToken::StatusBar,
            ColorToken::Error,
            ColorToken::Warning,
            ColorToken::Success,
        ]
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FullColorTheme {
    pub id: String,
    pub name: String,
    pub tokens: std::collections::HashMap<ColorToken, [u8; 4]>,
}

impl FullColorTheme {
    pub fn builtin_themes() -> Vec<FullColorTheme> {
        vec![Self::nucleon_dark(), Self::nucleon_light()]
    }

    pub fn nucleon_dark() -> Self {
        let mut tokens = std::collections::HashMap::new();
        tokens.insert(ColorToken::BgPrimary, [18, 18, 24, 255]);
        tokens.insert(ColorToken::BgSecondary, [30, 30, 38, 255]);
        tokens.insert(ColorToken::FgPrimary, [212, 212, 216, 255]);
        tokens.insert(ColorToken::FgSecondary, [161, 161, 170, 255]);
        tokens.insert(ColorToken::FgDim, [99, 99, 112, 255]);
        tokens.insert(ColorToken::Accent, [45, 212, 191, 255]);
        tokens.insert(ColorToken::AccentHover, [20, 184, 166, 255]);
        tokens.insert(ColorToken::AccentActive, [13, 148, 136, 255]);
        tokens.insert(ColorToken::PanelBg, [24, 24, 27, 255]);
        tokens.insert(ColorToken::PanelBorder, [63, 63, 70, 255]);
        tokens.insert(ColorToken::WindowChrome, [39, 39, 42, 255]);
        tokens.insert(ColorToken::WindowChromeFocused, [52, 52, 56, 255]);
        tokens.insert(ColorToken::Selection, [45, 212, 191, 255]);
        tokens.insert(ColorToken::SelectionFg, [0, 0, 0, 255]);
        tokens.insert(ColorToken::Border, [63, 63, 70, 255]);
        tokens.insert(ColorToken::Separator, [52, 52, 56, 255]);
        tokens.insert(ColorToken::StatusBar, [24, 24, 27, 255]);
        tokens.insert(ColorToken::Error, [248, 113, 113, 255]);
        tokens.insert(ColorToken::Warning, [251, 191, 36, 255]);
        tokens.insert(ColorToken::Success, [52, 211, 153, 255]);
        FullColorTheme {
            id: "nucleon-dark".to_string(),
            name: "Nucleon Dark".to_string(),
            tokens,
        }
    }

    pub fn nucleon_light() -> Self {
        let mut tokens = std::collections::HashMap::new();
        tokens.insert(ColorToken::BgPrimary, [250, 250, 249, 255]);
        tokens.insert(ColorToken::BgSecondary, [245, 245, 244, 255]);
        tokens.insert(ColorToken::FgPrimary, [28, 25, 23, 255]);
        tokens.insert(ColorToken::FgSecondary, [87, 83, 78, 255]);
        tokens.insert(ColorToken::FgDim, [120, 113, 108, 255]);
        tokens.insert(ColorToken::Accent, [100, 116, 139, 255]);
        tokens.insert(ColorToken::AccentHover, [148, 163, 184, 255]);
        tokens.insert(ColorToken::AccentActive, [71, 85, 105, 255]);
        tokens.insert(ColorToken::PanelBg, [231, 229, 228, 255]);
        tokens.insert(ColorToken::PanelBorder, [214, 211, 209, 255]);
        tokens.insert(ColorToken::WindowChrome, [231, 229, 228, 255]);
        tokens.insert(ColorToken::WindowChromeFocused, [214, 211, 209, 255]);
        tokens.insert(ColorToken::Selection, [148, 163, 184, 255]);
        tokens.insert(ColorToken::SelectionFg, [28, 25, 23, 255]);
        tokens.insert(ColorToken::Border, [214, 211, 209, 255]);
        tokens.insert(ColorToken::Separator, [214, 211, 209, 255]);
        tokens.insert(ColorToken::StatusBar, [231, 229, 228, 255]);
        tokens.insert(ColorToken::Error, [220, 38, 38, 255]);
        tokens.insert(ColorToken::Warning, [202, 138, 4, 255]);
        tokens.insert(ColorToken::Success, [22, 163, 74, 255]);
        FullColorTheme {
            id: "nucleon-light".to_string(),
            name: "Nucleon Light".to_string(),
            tokens,
        }
    }

    pub fn builtin_by_id(id: &str) -> Option<FullColorTheme> {
        Self::builtin_themes()
            .into_iter()
            .find(|theme| theme.id == id)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum FillStyle {
    None,
    Solid {
        color: ThemeColor,
    },
    LinearGradient {
        stops: Vec<GradientStop>,
        angle: f32,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GradientStop {
    pub position: f32,
    pub color: ThemeColor,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ThemeColor {
    Palette(PaletteRef),
    Rgba([u8; 4]),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PaletteRef {
    Fg,
    Dim,
    Bg,
    Panel,
    SelectedBg,
    SelectedFg,
    HoveredBg,
    ActiveBg,
    SelectionBg,
    WindowChrome,
    WindowChromeFocused,
    BarBg,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BorderStyle {
    pub width: f32,
    pub color: ThemeColor,
    #[serde(default)]
    pub highlight: Option<ThemeColor>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ShadowStyle {
    pub offset_x: f32,
    pub offset_y: f32,
    pub blur: f32,
    pub color: ThemeColor,
}

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
    #[serde(default)]
    pub text_color: Option<ThemeColor>,
}

fn default_fill_none() -> FillStyle {
    FillStyle::None
}

impl ElementStyle {
    pub fn default_window_frame() -> Self {
        Self {
            fill: FillStyle::Solid {
                color: ThemeColor::Palette(PaletteRef::Bg),
            },
            border: Some(BorderStyle {
                width: 2.0,
                color: ThemeColor::Palette(PaletteRef::Fg),
                highlight: None,
            }),
            shadow: None,
            rounding: 0.0,
            text_color: None,
        }
    }

    pub fn default_title_bar() -> Self {
        Self {
            fill: FillStyle::Solid {
                color: ThemeColor::Palette(PaletteRef::WindowChromeFocused),
            },
            border: None,
            shadow: None,
            rounding: 0.0,
            text_color: Some(ThemeColor::Palette(PaletteRef::SelectedFg)),
        }
    }

    pub fn default_title_bar_unfocused() -> Self {
        Self {
            fill: FillStyle::Solid {
                color: ThemeColor::Palette(PaletteRef::WindowChrome),
            },
            border: None,
            shadow: None,
            rounding: 0.0,
            text_color: Some(ThemeColor::Palette(PaletteRef::SelectedFg)),
        }
    }

    pub fn default_bar() -> Self {
        Self {
            fill: FillStyle::Solid {
                color: ThemeColor::Palette(PaletteRef::BarBg),
            },
            border: None,
            shadow: None,
            rounding: 0.0,
            text_color: Some(ThemeColor::Palette(PaletteRef::SelectedFg)),
        }
    }

    pub fn default_menu_dropdown() -> Self {
        Self {
            fill: FillStyle::Solid {
                color: ThemeColor::Palette(PaletteRef::Bg),
            },
            border: Some(BorderStyle {
                width: 2.0,
                color: ThemeColor::Palette(PaletteRef::Fg),
                highlight: None,
            }),
            shadow: None,
            rounding: 0.0,
            text_color: None,
        }
    }

    pub fn default_start_menu() -> Self {
        Self {
            fill: FillStyle::Solid {
                color: ThemeColor::Palette(PaletteRef::Panel),
            },
            border: Some(BorderStyle {
                width: 2.0,
                color: ThemeColor::Palette(PaletteRef::Fg),
                highlight: None,
            }),
            shadow: None,
            rounding: 0.0,
            text_color: None,
        }
    }

    pub fn default_panel() -> Self {
        Self {
            fill: FillStyle::Solid {
                color: ThemeColor::Palette(PaletteRef::Panel),
            },
            border: None,
            shadow: None,
            rounding: 0.0,
            text_color: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DesktopStyle {
    pub id: String,
    pub name: String,
    pub title_bar_height: f32,
    pub separator_thickness: f32,
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
    #[serde(default = "ElementStyle::default_start_menu")]
    pub start_menu: ElementStyle,
    #[serde(default)]
    pub start_button: Option<ElementStyle>,
    #[serde(default = "ElementStyle::default_panel")]
    pub panel: ElementStyle,
    #[serde(default)]
    pub scrollbar: Option<ElementStyle>,
}

impl Default for DesktopStyle {
    fn default() -> Self {
        Self::flat()
    }
}

impl DesktopStyle {
    pub fn flat() -> Self {
        DesktopStyle {
            id: "flat".to_string(),
            name: "Flat".to_string(),
            title_bar_height: 28.0,
            separator_thickness: 2.0,
            window_frame: ElementStyle {
                fill: FillStyle::Solid {
                    color: ThemeColor::Palette(PaletteRef::Bg),
                },
                border: Some(BorderStyle {
                    width: 2.0,
                    color: ThemeColor::Palette(PaletteRef::Fg),
                    highlight: None,
                }),
                shadow: None,
                rounding: 0.0,
                text_color: None,
            },
            title_bar: ElementStyle {
                fill: FillStyle::Solid {
                    color: ThemeColor::Palette(PaletteRef::WindowChromeFocused),
                },
                border: None,
                shadow: None,
                rounding: 0.0,
                text_color: Some(ThemeColor::Palette(PaletteRef::SelectedFg)),
            },
            title_bar_unfocused: ElementStyle {
                fill: FillStyle::Solid {
                    color: ThemeColor::Palette(PaletteRef::WindowChrome),
                },
                border: None,
                shadow: None,
                rounding: 0.0,
                text_color: Some(ThemeColor::Palette(PaletteRef::SelectedFg)),
            },
            menu_bar: ElementStyle {
                fill: FillStyle::Solid {
                    color: ThemeColor::Palette(PaletteRef::BarBg),
                },
                border: None,
                shadow: None,
                rounding: 0.0,
                text_color: Some(ThemeColor::Palette(PaletteRef::SelectedFg)),
            },
            taskbar: ElementStyle {
                fill: FillStyle::Solid {
                    color: ThemeColor::Palette(PaletteRef::BarBg),
                },
                border: None,
                shadow: None,
                rounding: 0.0,
                text_color: Some(ThemeColor::Palette(PaletteRef::SelectedFg)),
            },
            menu_dropdown: ElementStyle {
                fill: FillStyle::Solid {
                    color: ThemeColor::Palette(PaletteRef::Bg),
                },
                border: Some(BorderStyle {
                    width: 2.0,
                    color: ThemeColor::Palette(PaletteRef::Fg),
                    highlight: None,
                }),
                shadow: None,
                rounding: 0.0,
                text_color: None,
            },
            start_menu: ElementStyle {
                fill: FillStyle::Solid {
                    color: ThemeColor::Palette(PaletteRef::Panel),
                },
                border: Some(BorderStyle {
                    width: 2.0,
                    color: ThemeColor::Palette(PaletteRef::Fg),
                    highlight: None,
                }),
                shadow: None,
                rounding: 0.0,
                text_color: None,
            },
            start_button: None,
            panel: ElementStyle {
                fill: FillStyle::Solid {
                    color: ThemeColor::Palette(PaletteRef::Panel),
                },
                border: None,
                shadow: None,
                rounding: 0.0,
                text_color: None,
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
                fill: FillStyle::Solid {
                    color: ThemeColor::Palette(PaletteRef::Bg),
                },
                border: Some(BorderStyle {
                    width: 1.0,
                    color: ThemeColor::Palette(PaletteRef::Dim),
                    highlight: None,
                }),
                shadow: Some(ShadowStyle {
                    offset_x: 0.0,
                    offset_y: 2.0,
                    blur: 8.0,
                    color: ThemeColor::Rgba([0, 0, 0, 80]),
                }),
                rounding: 6.0,
                text_color: None,
            },
            title_bar: ElementStyle {
                fill: FillStyle::Solid {
                    color: ThemeColor::Palette(PaletteRef::WindowChromeFocused),
                },
                border: None,
                shadow: None,
                rounding: 6.0,
                text_color: Some(ThemeColor::Palette(PaletteRef::SelectedFg)),
            },
            title_bar_unfocused: ElementStyle {
                fill: FillStyle::Solid {
                    color: ThemeColor::Palette(PaletteRef::WindowChrome),
                },
                border: None,
                shadow: None,
                rounding: 6.0,
                text_color: Some(ThemeColor::Palette(PaletteRef::SelectedFg)),
            },
            menu_bar: ElementStyle {
                fill: FillStyle::Solid {
                    color: ThemeColor::Palette(PaletteRef::BarBg),
                },
                border: None,
                shadow: None,
                rounding: 0.0,
                text_color: Some(ThemeColor::Palette(PaletteRef::SelectedFg)),
            },
            taskbar: ElementStyle {
                fill: FillStyle::Solid {
                    color: ThemeColor::Palette(PaletteRef::BarBg),
                },
                border: None,
                shadow: None,
                rounding: 0.0,
                text_color: Some(ThemeColor::Palette(PaletteRef::SelectedFg)),
            },
            menu_dropdown: ElementStyle {
                fill: FillStyle::Solid {
                    color: ThemeColor::Palette(PaletteRef::Bg),
                },
                border: Some(BorderStyle {
                    width: 1.0,
                    color: ThemeColor::Palette(PaletteRef::Dim),
                    highlight: None,
                }),
                shadow: Some(ShadowStyle {
                    offset_x: 0.0,
                    offset_y: 2.0,
                    blur: 6.0,
                    color: ThemeColor::Rgba([0, 0, 0, 60]),
                }),
                rounding: 4.0,
                text_color: None,
            },
            start_menu: ElementStyle {
                fill: FillStyle::Solid {
                    color: ThemeColor::Palette(PaletteRef::Panel),
                },
                border: Some(BorderStyle {
                    width: 1.0,
                    color: ThemeColor::Palette(PaletteRef::Dim),
                    highlight: None,
                }),
                shadow: Some(ShadowStyle {
                    offset_x: 0.0,
                    offset_y: 4.0,
                    blur: 12.0,
                    color: ThemeColor::Rgba([0, 0, 0, 80]),
                }),
                rounding: 4.0,
                text_color: None,
            },
            start_button: None,
            panel: ElementStyle {
                fill: FillStyle::Solid {
                    color: ThemeColor::Palette(PaletteRef::Panel),
                },
                border: None,
                shadow: None,
                rounding: 4.0,
                text_color: None,
            },
            scrollbar: None,
        }
    }

    pub fn builtin_desktop_styles() -> Vec<DesktopStyle> {
        vec![Self::flat(), Self::modern()]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PanelType {
    MenuBar,
    Taskbar,
    Disabled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LauncherStyle {
    StartMenu,
    Overlay,
    Hidden,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TerminalStatusBarPosition {
    Bottom,
    Hidden,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WindowHeaderStyle {
    Standard,
    Compact,
    Hidden,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct LayoutProfile {
    pub id: String,
    pub name: String,
    pub top_panel: PanelType,
    pub top_panel_height: f32,
    pub bottom_panel: PanelType,
    pub bottom_panel_height: f32,
    pub launcher_style: LauncherStyle,
    pub window_header_style: WindowHeaderStyle,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TerminalLayoutProfile {
    pub id: String,
    pub name: String,
    pub status_bar_position: TerminalStatusBarPosition,
    pub status_bar_height: f32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct SoundPack {
    /// Path to the sound pack root directory (relative to theme bundle).
    /// Contains optional WAV files: login.wav, logout.wav, error.wav,
    /// navigate.wav, keypress.wav, boot_01.wav through boot_05.wav
    pub path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssetPack {
    pub id: String,
    pub name: String,
    /// Path to the asset pack root directory (relative to theme bundle).
    /// Contains optional subdirectories: icons_mono/, icons_color/, cursors/
    pub path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CursorSprite {
    pub width: usize,
    pub height: usize,
    pub hotspot_x: usize,
    pub hotspot_y: usize,
    /// ASCII sprite mask: '#' = fill, 'O' = outline, '.' = highlight, ' ' = transparent
    pub mask: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerminalBranding {
    #[serde(default, alias = "lines")]
    pub header_lines: Vec<String>,
}

impl Default for TerminalBranding {
    fn default() -> Self {
        Self::none()
    }
}

impl TerminalBranding {
    pub fn none() -> Self {
        TerminalBranding {
            header_lines: vec![],
        }
    }
}

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

/// A single option that a terminal theme exposes to the user.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThemeOptionDef {
    /// Machine-readable key (for example, "bracket_menus").
    pub key: String,
    /// Human-readable label shown in Tweaks.
    pub label: String,
    /// Help text shown below the option.
    #[serde(default)]
    pub description: String,
    /// What kind of control this option is.
    pub kind: ThemeOptionKind,
}

/// The type and constraints of a theme option.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ThemeOptionKind {
    Bool { default: bool },
    Choice { choices: Vec<String>, default: String },
    Int { min: i32, max: i32, default: i32 },
    Float { min: f32, max: f32, default: f32 },
}

/// A concrete value for a theme option.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ThemeOptionValue {
    Bool(bool),
    String(String),
    Int(i32),
    Float(f32),
}

/// Which renderer a terminal theme uses.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum TerminalRenderer {
    /// Built-in renderer shipped with the core. ID must match a registered renderer.
    Builtin { id: String },
    /// WASM module that owns screen rendering (Phase 16).
    Wasm { module: String },
}

/// A terminal theme that controls the entire terminal UI composition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
    /// Options this theme exposes to the user.
    #[serde(default)]
    pub options_schema: Vec<ThemeOptionDef>,
    /// Default values for all options. Keys must match `options_schema`.
    #[serde(default)]
    pub default_options: HashMap<String, ThemeOptionValue>,
}

/// Manifest for a terminal theme installable from the themes repo.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TerminalThemeManifest {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default = "default_theme_pack_version")]
    pub version: String,
    pub theme: TerminalTheme,
}

impl TerminalTheme {
    pub fn classic() -> Self {
        Self {
            id: "classic".to_string(),
            name: "Classic".to_string(),
            description: "The original Nucleon terminal interface.".to_string(),
            version: "1.0.0".to_string(),
            renderer: TerminalRenderer::Builtin {
                id: "classic".to_string(),
            },
            font: None,
            options_schema: vec![
                ThemeOptionDef {
                    key: "separator_char".to_string(),
                    label: "Separator Character".to_string(),
                    description: "Character used for horizontal separators.".to_string(),
                    kind: ThemeOptionKind::Choice {
                        choices: vec![
                            "=".to_string(),
                            "-".to_string(),
                            "*".to_string(),
                            "#".to_string(),
                        ],
                        default: "=".to_string(),
                    },
                },
                ThemeOptionDef {
                    key: "show_separators".to_string(),
                    label: "Show Separators".to_string(),
                    description: "Display horizontal separator lines.".to_string(),
                    kind: ThemeOptionKind::Bool { default: true },
                },
                ThemeOptionDef {
                    key: "menu_alignment".to_string(),
                    label: "Menu Alignment".to_string(),
                    description: "How menu items are aligned on screen.".to_string(),
                    kind: ThemeOptionKind::Choice {
                        choices: vec!["Left".to_string(), "Center".to_string()],
                        default: "Left".to_string(),
                    },
                },
                ThemeOptionDef {
                    key: "selection_style".to_string(),
                    label: "Selection Style".to_string(),
                    description: "How the selected menu item is highlighted.".to_string(),
                    kind: ThemeOptionKind::Choice {
                        choices: vec!["Full Row".to_string(), "Text Only".to_string()],
                        default: "Full Row".to_string(),
                    },
                },
                ThemeOptionDef {
                    key: "selection_marker".to_string(),
                    label: "Selection Marker".to_string(),
                    description: "Text prefix for the selected item.".to_string(),
                    kind: ThemeOptionKind::Choice {
                        choices: vec![
                            "> ".to_string(),
                            ">> ".to_string(),
                            "* ".to_string(),
                            "".to_string(),
                        ],
                        default: "> ".to_string(),
                    },
                },
                ThemeOptionDef {
                    key: "header_visible".to_string(),
                    label: "Show Header".to_string(),
                    description: "Display the branding header at the top.".to_string(),
                    kind: ThemeOptionKind::Bool { default: false },
                },
                ThemeOptionDef {
                    key: "subtitle_underlined".to_string(),
                    label: "Underline Subtitle".to_string(),
                    description: "Draw an underline below the subtitle text.".to_string(),
                    kind: ThemeOptionKind::Bool { default: true },
                },
                ThemeOptionDef {
                    key: "content_margin".to_string(),
                    label: "Content Margin".to_string(),
                    description: "Columns of margin on each side of content.".to_string(),
                    kind: ThemeOptionKind::Int {
                        min: 0,
                        max: 10,
                        default: 3,
                    },
                },
            ],
            default_options: {
                let mut options = HashMap::new();
                options.insert(
                    "separator_char".to_string(),
                    ThemeOptionValue::String("=".to_string()),
                );
                options.insert(
                    "show_separators".to_string(),
                    ThemeOptionValue::Bool(true),
                );
                options.insert(
                    "menu_alignment".to_string(),
                    ThemeOptionValue::String("Left".to_string()),
                );
                options.insert(
                    "selection_style".to_string(),
                    ThemeOptionValue::String("Full Row".to_string()),
                );
                options.insert(
                    "selection_marker".to_string(),
                    ThemeOptionValue::String("> ".to_string()),
                );
                options.insert(
                    "header_visible".to_string(),
                    ThemeOptionValue::Bool(false),
                );
                options.insert(
                    "subtitle_underlined".to_string(),
                    ThemeOptionValue::Bool(true),
                );
                options.insert("content_margin".to_string(), ThemeOptionValue::Int(3));
                options
            },
        }
    }

    pub fn builtin_terminal_themes() -> Vec<Self> {
        vec![Self::classic()]
    }

    /// Get a bool option value, falling back to the schema default.
    pub fn get_bool(
        options: &HashMap<String, ThemeOptionValue>,
        key: &str,
        schema: &[ThemeOptionDef],
    ) -> bool {
        if let Some(ThemeOptionValue::Bool(value)) = options.get(key) {
            return *value;
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

    /// Get a string option value, falling back to the schema default.
    pub fn get_string(
        options: &HashMap<String, ThemeOptionValue>,
        key: &str,
        schema: &[ThemeOptionDef],
    ) -> String {
        if let Some(ThemeOptionValue::String(value)) = options.get(key) {
            return value.clone();
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

    /// Get an int option value, falling back to the schema default.
    pub fn get_int(
        options: &HashMap<String, ThemeOptionValue>,
        key: &str,
        schema: &[ThemeOptionDef],
    ) -> i32 {
        if let Some(ThemeOptionValue::Int(value)) = options.get(key) {
            return *value;
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

    /// Get a float option value, falling back to the schema default.
    pub fn get_float(
        options: &HashMap<String, ThemeOptionValue>,
        key: &str,
        schema: &[ThemeOptionDef],
    ) -> f32 {
        if let Some(ThemeOptionValue::Float(value)) = options.get(key) {
            return *value;
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DesktopStyleManifest {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default = "default_theme_pack_version")]
    pub version: String,
    pub style: DesktopStyle,
    #[serde(default)]
    pub font: Option<FontRef>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColorThemeManifest {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default = "default_theme_pack_version")]
    pub version: String,
    pub color_style: ColorStyle,
    #[serde(default)]
    pub full_color_theme: Option<FullColorTheme>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IconPackManifest {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default = "default_theme_pack_version")]
    pub version: String,
    pub asset_pack: AssetPack,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SoundPackManifest {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default = "default_theme_pack_version")]
    pub version: String,
    pub sound_pack: SoundPack,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CursorPackManifest {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default = "default_theme_pack_version")]
    pub version: String,
    pub cursor_pack: CursorPack,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum FontRef {
    Builtin { id: String },
    Bundled { file: String },
    Installed { font_pack_id: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FontPackManifest {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default = "default_theme_pack_version")]
    pub version: String,
    pub file: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum DesktopStyleRef {
    Inline(DesktopStyle),
    ById {
        #[serde(alias = "shell_style_id")]
        desktop_style_id: String,
    },
}

impl Default for DesktopStyleRef {
    fn default() -> Self {
        DesktopStyleRef::Inline(DesktopStyle::flat())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemePack {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default = "default_theme_pack_version")]
    pub version: String,
    #[serde(default)]
    #[serde(alias = "shell_style")]
    pub desktop_style: DesktopStyleRef,
    pub color_style: ColorStyle,
    #[serde(default)]
    pub full_color_theme: Option<FullColorTheme>,
    #[serde(default)]
    pub terminal_branding: TerminalBranding,
    #[serde(default)]
    pub terminal_decoration: TerminalDecoration,
    #[serde(default)]
    pub icon_pack_id: Option<String>,
    #[serde(default)]
    pub sound_pack_id: Option<String>,
    #[serde(default)]
    pub cursor_pack_id: Option<String>,
    #[serde(default)]
    pub font_pack_id: Option<String>,
    #[serde(default = "LayoutProfile::classic")]
    pub layout_profile: LayoutProfile,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
enum LegacyPanelPosition {
    Top,
    Bottom,
    Hidden,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
enum LegacyDockPosition {
    Bottom,
    Left,
    Right,
    Hidden,
}

#[derive(Deserialize)]
struct LayoutProfileCompat {
    id: String,
    name: String,
    #[serde(default)]
    top_panel: Option<PanelType>,
    #[serde(default)]
    top_panel_height: Option<f32>,
    #[serde(default)]
    bottom_panel: Option<PanelType>,
    #[serde(default)]
    bottom_panel_height: Option<f32>,
    #[serde(default)]
    launcher_style: Option<LauncherStyle>,
    #[serde(default)]
    window_header_style: Option<WindowHeaderStyle>,
    #[serde(default)]
    panel_position: Option<LegacyPanelPosition>,
    #[serde(default)]
    panel_height: Option<f32>,
    #[serde(default)]
    dock_position: Option<LegacyDockPosition>,
    #[serde(default)]
    dock_size: Option<f32>,
}

impl<'de> Deserialize<'de> for LayoutProfile {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let compat = LayoutProfileCompat::deserialize(deserializer)?;

        if compat.top_panel.is_none() && compat.bottom_panel.is_none() {
            match compat.id.as_str() {
                "classic" => return Ok(LayoutProfile::classic()),
                "minimal" => return Ok(LayoutProfile::minimal()),
                _ => {}
            }
        }

        let mut top_panel = compat.top_panel.unwrap_or(PanelType::Disabled);
        let mut top_panel_height = compat.top_panel_height.unwrap_or(30.0);
        let mut bottom_panel = compat.bottom_panel.unwrap_or(PanelType::Disabled);
        let mut bottom_panel_height = compat.bottom_panel_height.unwrap_or(32.0);

        if compat.top_panel.is_none() {
            match compat.panel_position.unwrap_or(LegacyPanelPosition::Hidden) {
                LegacyPanelPosition::Top => {
                    top_panel = PanelType::MenuBar;
                    top_panel_height = compat.panel_height.unwrap_or(30.0);
                }
                LegacyPanelPosition::Bottom => {
                    bottom_panel = PanelType::MenuBar;
                    bottom_panel_height = compat.panel_height.unwrap_or(30.0);
                }
                LegacyPanelPosition::Hidden => {}
            }
        }

        if compat.bottom_panel.is_none() {
            match compat.dock_position.unwrap_or(LegacyDockPosition::Hidden) {
                LegacyDockPosition::Bottom => {
                    if bottom_panel == PanelType::Disabled {
                        bottom_panel = PanelType::Taskbar;
                    }
                    bottom_panel_height = compat.dock_size.unwrap_or(bottom_panel_height);
                }
                LegacyDockPosition::Left | LegacyDockPosition::Right => {
                    if bottom_panel == PanelType::Disabled {
                        bottom_panel = PanelType::Taskbar;
                    }
                    bottom_panel_height = compat.dock_size.unwrap_or(bottom_panel_height);
                }
                LegacyDockPosition::Hidden => {}
            }
        }

        Ok(LayoutProfile {
            id: compat.id,
            name: compat.name,
            top_panel,
            top_panel_height,
            bottom_panel,
            bottom_panel_height,
            launcher_style: compat.launcher_style.unwrap_or(LauncherStyle::StartMenu),
            window_header_style: compat
                .window_header_style
                .unwrap_or(WindowHeaderStyle::Standard),
        })
    }
}

impl LayoutProfile {
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

    pub fn builtin_layouts() -> Vec<LayoutProfile> {
        vec![LayoutProfile::classic(), LayoutProfile::minimal()]
    }
}

impl TerminalLayoutProfile {
    pub fn classic() -> Self {
        TerminalLayoutProfile {
            id: "classic-terminal".to_string(),
            name: "Classic Terminal".to_string(),
            status_bar_position: TerminalStatusBarPosition::Bottom,
            status_bar_height: 31.0,
        }
    }

    pub fn minimal() -> Self {
        TerminalLayoutProfile {
            id: "minimal-terminal".to_string(),
            name: "Minimal Terminal".to_string(),
            status_bar_position: TerminalStatusBarPosition::Hidden,
            status_bar_height: 31.0,
        }
    }

    pub fn builtin_layouts() -> Vec<TerminalLayoutProfile> {
        vec![
            TerminalLayoutProfile::classic(),
            TerminalLayoutProfile::minimal(),
        ]
    }
}
