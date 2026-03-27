use serde::{Deserialize, Serialize};

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminalLayoutProfile {
    pub id: String,
    pub name: String,
    pub status_bar_position: TerminalStatusBarPosition,
    pub status_bar_height: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetPackRef {
    pub id: String,
    pub name: String,
    pub path: String,
}

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
            asset_pack: None,
        }
    }
}
