use serde::{Deserialize, Deserializer, Serialize};

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShellStyle {
    pub id: String,
    pub name: String,
    pub border_radius: f32,
    pub title_bar_height: f32,
    pub separator_thickness: f32,
    pub window_shadow: bool,
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

#[derive(Debug, Clone, Serialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
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

    pub fn heritage() -> Self {
        TerminalBranding {
            header_lines: vec![
                "ROBCO INDUSTRIES UNIFIED OPERATING SYSTEM".to_string(),
                "COPYRIGHT 2075-2077 ROBCO INDUSTRIES".to_string(),
                "-SERVER 1-".to_string(),
            ],
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemePack {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default = "default_theme_pack_version")]
    pub version: String,
    pub shell_style: ShellStyle,
    pub layout_profile: LayoutProfile,
    pub color_style: ColorStyle,
    #[serde(default)]
    pub sound_pack: SoundPack,
    pub asset_pack: Option<AssetPack>,
    pub cursor_pack: Option<CursorPack>,
    #[serde(default)]
    pub terminal_branding: TerminalBranding,
    #[serde(default)]
    pub terminal_decoration: TerminalDecoration,
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

impl ThemePack {
    pub fn classic() -> Self {
        ThemePack {
            id: "classic".to_string(),
            name: "Classic".to_string(),
            description: "The original Nucleon terminal aesthetic".to_string(),
            version: "1.0.0".to_string(),
            shell_style: ShellStyle {
                id: "classic".to_string(),
                name: "Classic".to_string(),
                border_radius: 0.0,
                title_bar_height: 28.0,
                separator_thickness: 2.0,
                window_shadow: false,
            },
            layout_profile: LayoutProfile::classic(),
            color_style: ColorStyle::Monochrome {
                preset: MonochromePreset::Green,
                custom_rgb: None,
            },
            sound_pack: SoundPack::default(),
            asset_pack: None,
            cursor_pack: None,
            terminal_branding: TerminalBranding::none(),
            terminal_decoration: TerminalDecoration::default(),
        }
    }

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
            sound_pack: SoundPack::default(),
            asset_pack: None,
            cursor_pack: None,
            terminal_branding: TerminalBranding::none(),
            terminal_decoration: TerminalDecoration::default(),
        }
    }

    pub fn heritage() -> Self {
        ThemePack {
            id: "robco-heritage".to_string(),
            name: "RobCo Heritage".to_string(),
            description: "Restores the original RobCo terminal presentation.".to_string(),
            version: "1.0.0".to_string(),
            shell_style: ShellStyle {
                id: "robco".to_string(),
                name: "RobCo".to_string(),
                border_radius: 0.0,
                title_bar_height: 28.0,
                separator_thickness: 2.0,
                window_shadow: false,
            },
            layout_profile: LayoutProfile::classic(),
            color_style: ColorStyle::Monochrome {
                preset: MonochromePreset::Green,
                custom_rgb: None,
            },
            sound_pack: SoundPack::default(),
            asset_pack: None,
            cursor_pack: None,
            terminal_branding: TerminalBranding::heritage(),
            terminal_decoration: TerminalDecoration::default(),
        }
    }

    pub fn builtin_theme_packs() -> Vec<ThemePack> {
        vec![Self::classic(), Self::nucleon()]
    }
}
