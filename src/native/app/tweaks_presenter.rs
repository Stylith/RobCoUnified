use super::super::background::BackgroundResult;
use super::super::desktop_app::{DesktopWindow, WindowInstanceId};
use super::super::desktop_settings_service::persist_settings_draft;
use super::super::desktop_status_service::{
    clear_shell_status, saved_settings_status, settings_status,
};
use super::super::desktop_surface_service::{
    desktop_builtin_icons, set_builtin_icon_visible, set_desktop_icon_style,
    set_wallpaper_size_mode as set_desktop_wallpaper_size_mode, wallpaper_browser_start_dir,
};
use super::super::menu::TerminalScreen;
use super::super::retro_ui::{
    current_palette, current_palette_for_surface, set_active_color_style, set_active_desktop_style,
    terminal_menu_row_text, RetroPalette, RetroScreen, ShellSurfaceKind,
};
use super::super::{
    installed_color_themes, installed_cursor_packs, installed_desktop_styles,
    installed_font_packs, installed_icon_packs, installed_sound_packs, installed_terminal_themes,
};
use super::desktop_window_mgmt::{DesktopHeaderAction, DesktopWindowRectTracking};
use super::NucleonNativeApp;
use crate::config::{
    CliAcsMode, CliColorMode, CrtPreset, DesktopCursorThemeSelection, DesktopIconStyle,
    NativeStartupWindowMode, WallpaperSizeMode, CUSTOM_THEME_NAME,
};
use crate::theme::{
    ColorStyle, ColorThemeManifest, ColorToken, CursorPackManifest, DesktopStyle,
    DesktopStyleManifest, FontPackManifest, FullColorTheme, IconPackManifest, LauncherStyle,
    MonochromePreset, PanelType, SoundPackManifest, TerminalLayoutProfile, TerminalThemeManifest,
    ThemeOptionKind, ThemeOptionValue, ThemePack, WindowHeaderStyle,
};
use eframe::egui::color_picker::{color_edit_button_srgba, Alpha};
use eframe::egui::{self, Context, Key, RichText, TextEdit};
use std::path::Path;

const MONOCHROME_THEME_NAMES: &[&str] = &[
    "Green (Default)",
    "White",
    "Amber",
    "Blue",
    "Light Blue",
    CUSTOM_THEME_NAME,
];

const COLOR_PALETTE_COLUMNS: usize = 8;
const COLOR_PALETTE: &[[u8; 3]] = &[
    // Row 1: Grayscale
    [0, 0, 0],
    [68, 68, 68],
    [102, 102, 102],
    [136, 136, 136],
    [170, 170, 170],
    [204, 204, 204],
    [238, 238, 238],
    [255, 255, 255],
    // Row 2: Pure saturated
    [255, 0, 0],
    [255, 128, 0],
    [255, 255, 0],
    [0, 255, 0],
    [0, 255, 255],
    [0, 0, 255],
    [128, 0, 255],
    [255, 0, 255],
    // Row 3: Medium
    [192, 0, 0],
    [192, 96, 0],
    [192, 192, 0],
    [0, 192, 0],
    [0, 192, 192],
    [0, 0, 192],
    [96, 0, 192],
    [192, 0, 192],
    // Row 4: Dark
    [128, 0, 0],
    [128, 64, 0],
    [128, 128, 0],
    [0, 128, 0],
    [0, 128, 128],
    [0, 0, 128],
    [64, 0, 128],
    [128, 0, 128],
    // Row 5: Pastel
    [255, 128, 128],
    [255, 192, 128],
    [255, 255, 128],
    [128, 255, 128],
    [128, 255, 255],
    [128, 128, 255],
    [192, 128, 255],
    [255, 128, 255],
    // Row 6: Light
    [255, 192, 192],
    [255, 224, 192],
    [255, 255, 192],
    [192, 255, 192],
    [192, 255, 255],
    [192, 192, 255],
    [224, 192, 255],
    [255, 192, 255],
];

fn monochrome_theme_name_for_color_style(style: &ColorStyle) -> &'static str {
    match style {
        ColorStyle::Monochrome { preset, .. } => match preset {
            MonochromePreset::Green => "Green (Default)",
            MonochromePreset::White => "White",
            MonochromePreset::Amber => "Amber",
            MonochromePreset::Blue => "Blue",
            MonochromePreset::LightBlue => "Light Blue",
            MonochromePreset::Custom => CUSTOM_THEME_NAME,
        },
        ColorStyle::FullColor { .. } => "Green (Default)",
    }
}

fn full_color_theme_id_for_color_style(style: &ColorStyle) -> &str {
    match style {
        ColorStyle::FullColor { theme_id } => theme_id.as_str(),
        ColorStyle::Monochrome { .. } => "nucleon-dark",
    }
}

fn custom_rgb_for_color_style(style: &ColorStyle) -> [u8; 3] {
    match style {
        ColorStyle::Monochrome {
            preset: MonochromePreset::Custom,
            custom_rgb,
        } => custom_rgb.unwrap_or([0, 255, 0]),
        _ => [0, 255, 0],
    }
}

fn color_style_from_theme_name(name: &str, custom_rgb: [u8; 3]) -> ColorStyle {
    match name {
        "Green (Default)" => ColorStyle::Monochrome {
            preset: MonochromePreset::Green,
            custom_rgb: None,
        },
        "White" => ColorStyle::Monochrome {
            preset: MonochromePreset::White,
            custom_rgb: None,
        },
        "Amber" => ColorStyle::Monochrome {
            preset: MonochromePreset::Amber,
            custom_rgb: None,
        },
        "Blue" => ColorStyle::Monochrome {
            preset: MonochromePreset::Blue,
            custom_rgb: None,
        },
        "Light Blue" => ColorStyle::Monochrome {
            preset: MonochromePreset::LightBlue,
            custom_rgb: None,
        },
        CUSTOM_THEME_NAME => ColorStyle::Monochrome {
            preset: MonochromePreset::Custom,
            custom_rgb: Some(custom_rgb),
        },
        _ => ColorStyle::Monochrome {
            preset: MonochromePreset::Green,
            custom_rgb: None,
        },
    }
}

fn selected_theme_pack_name(selected_id: Option<&str>, theme_packs: &[ThemePack]) -> String {
    selected_id
        .and_then(|id| theme_packs.iter().find(|theme| theme.id == id))
        .map(|theme| theme.name.clone())
        .unwrap_or_else(|| "Custom".to_string())
}

fn theme_pack_name_by_id(theme_pack_id: &str, theme_packs: &[ThemePack]) -> String {
    theme_packs
        .iter()
        .find(|theme| theme.id == theme_pack_id)
        .map(|theme| theme.name.clone())
        .unwrap_or_else(|| theme_pack_id.to_string())
}

fn selected_desktop_style_name(
    selected_id: Option<&str>,
    current_style: &DesktopStyle,
    desktop_styles: &[DesktopStyleManifest],
) -> String {
    selected_id
        .and_then(|id| desktop_styles.iter().find(|manifest| manifest.id == id))
        .map(|manifest| manifest.name.clone())
        .unwrap_or_else(|| current_style.name.clone())
}

fn selected_color_theme_name(
    current_style: &ColorStyle,
    color_themes: &[ColorThemeManifest],
) -> String {
    color_themes
        .iter()
        .find(|manifest| manifest.color_style == *current_style)
        .map(|manifest| manifest.name.clone())
        .unwrap_or_else(|| match current_style {
            ColorStyle::Monochrome { .. } => {
                monochrome_theme_name_for_color_style(current_style).to_string()
            }
            ColorStyle::FullColor { theme_id } => full_color_theme_label(theme_id).to_string(),
        })
}

fn selected_manifest_name<T>(
    selected_id: Option<&str>,
    default_label: &str,
    manifests: &[T],
    id_of: fn(&T) -> &str,
    name_of: fn(&T) -> &str,
) -> String {
    selected_id
        .and_then(|id| manifests.iter().find(|manifest| id_of(manifest) == id))
        .map(|manifest| name_of(manifest).to_string())
        .unwrap_or_else(|| default_label.to_string())
}

fn selected_font_name(selected_id: Option<&str>, font_packs: &[FontPackManifest]) -> String {
    match selected_id {
        None => "Fixedsys (Default)".to_string(),
        Some(font_id) if font_id.starts_with("desktop-style:") => "Theme Default".to_string(),
        Some(font_id) if font_id.starts_with("terminal-theme:") => "Theme Default".to_string(),
        Some(font_id) => selected_manifest_name(
            Some(font_id),
            "Fixedsys (Default)",
            font_packs,
            |manifest: &FontPackManifest| manifest.id.as_str(),
            |manifest: &FontPackManifest| manifest.name.as_str(),
        ),
    }
}

fn selected_terminal_theme_name(
    current_theme: &str,
    terminal_themes: &[TerminalThemeManifest],
) -> String {
    terminal_themes
        .iter()
        .find(|manifest| manifest.id == current_theme)
        .map(|manifest| manifest.name.clone())
        .unwrap_or_else(|| current_theme.to_string())
}

fn theme_pack_has_cursor_assets(theme: &ThemePack) -> bool {
    theme.cursor_pack_id.is_some()
}

fn desktop_cursor_theme_options(theme_packs: &[ThemePack]) -> Vec<DesktopCursorThemeSelection> {
    let mut options = vec![
        DesktopCursorThemeSelection::FollowTheme,
        DesktopCursorThemeSelection::Builtin,
    ];
    options.extend(
        theme_packs
            .iter()
            .filter(|theme| theme_pack_has_cursor_assets(theme))
            .map(|theme| DesktopCursorThemeSelection::ThemePack {
                theme_pack_id: theme.id.clone(),
            }),
    );
    options
}

fn desktop_cursor_theme_selection_label(
    selection: &DesktopCursorThemeSelection,
    active_theme_pack_id: Option<&str>,
    theme_packs: &[ThemePack],
) -> String {
    match selection {
        DesktopCursorThemeSelection::FollowTheme => active_theme_pack_id
            .map(|theme_pack_id| {
                format!(
                    "Theme Default ({})",
                    theme_pack_name_by_id(theme_pack_id, theme_packs)
                )
            })
            .unwrap_or_else(|| "Theme Default".to_string()),
        DesktopCursorThemeSelection::Builtin => "Force Built-in".to_string(),
        DesktopCursorThemeSelection::ThemePack { theme_pack_id } => {
            theme_pack_name_by_id(theme_pack_id, theme_packs)
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TerminalTweaksRow {
    SectionSelector(u8),
    DropdownOption(TerminalTweaksDropdown, usize),
    TerminalWallpaperPicker,
    TerminalWallpaperMode,
    WindowMode,
    CrtEnabled,
    CrtPreset,
    CrtCurvature,
    CrtScanlines,
    CrtGlow,
    CrtBloom,
    CrtVignette,
    CrtNoise,
    CrtFlicker,
    CrtJitter,
    CrtBurnIn,
    CrtGlowLine,
    CrtGlowLineSpeed,
    CrtPhosphorSoftness,
    CrtBrightness,
    CrtContrast,
    TerminalTheme,
    TerminalColorMode,
    TerminalMonoTheme,
    TerminalCustomRed,
    TerminalCustomGreen,
    TerminalCustomBlue,
    TerminalFullColorTheme,
    TerminalFont,
    TerminalThemeOption(usize),
    TerminalLayout,
    TerminalStyledPty,
    TerminalPtyColorMode,
    TerminalBorderGlyphs,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum TerminalTweaksDropdown {
    TerminalWallpaperMode,
    WindowMode,
    CrtPreset,
    TerminalTheme,
    TerminalColorMode,
    TerminalMonoTheme,
    TerminalFullColorTheme,
    TerminalFont,
    TerminalThemeOptionChoice(usize),
    TerminalLayout,
    TerminalPtyColorMode,
    TerminalBorderGlyphs,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TerminalTweaksStep {
    Previous,
    Next,
    Activate,
}

impl TerminalTweaksStep {
    fn delta(self) -> i32 {
        match self {
            Self::Previous => -1,
            Self::Next | Self::Activate => 1,
        }
    }
}

#[derive(Debug, Default, Clone, Copy)]
struct TerminalTweaksMutation {
    persist_changed: bool,
    appearance_changed: bool,
    window_mode_changed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TerminalTweaksEntry {
    Header(&'static str),
    Row(TerminalTweaksRow),
}

fn adjust_f32(value: &mut f32, delta: i32, min: f32, max: f32, step: f32) -> bool {
    let next = (*value + delta as f32 * step).clamp(min, max);
    let next = (next * 1000.0).round() / 1000.0;
    if (*value - next).abs() < f32::EPSILON {
        false
    } else {
        *value = next;
        true
    }
}

fn adjust_u8(value: &mut u8, delta: i32) -> bool {
    let next = (*value as i32 + delta).clamp(0, 255) as u8;
    if *value == next {
        false
    } else {
        *value = next;
        true
    }
}

fn wallpaper_size_mode_label(mode: WallpaperSizeMode) -> &'static str {
    match mode {
        WallpaperSizeMode::DefaultSize => "Default Size",
        WallpaperSizeMode::FitToScreen => "Fit To Screen",
        WallpaperSizeMode::Centered => "Centered",
        WallpaperSizeMode::Tile => "Tile",
        WallpaperSizeMode::Stretch => "Stretch",
    }
}

fn panel_type_label(panel_type: PanelType) -> &'static str {
    match panel_type {
        PanelType::MenuBar => "Menu Bar",
        PanelType::Taskbar => "Taskbar",
        PanelType::Disabled => "Disabled",
    }
}

fn launcher_style_label(style: LauncherStyle) -> &'static str {
    match style {
        LauncherStyle::StartMenu => "Start Menu",
        LauncherStyle::Overlay => "Overlay",
        LauncherStyle::Hidden => "Hidden",
    }
}

fn window_header_label(style: WindowHeaderStyle) -> &'static str {
    match style {
        WindowHeaderStyle::Standard => "Standard",
        WindowHeaderStyle::Compact => "Compact",
        WindowHeaderStyle::Hidden => "Hidden",
    }
}

fn desktop_icon_style_label(style: DesktopIconStyle) -> &'static str {
    match style {
        DesktopIconStyle::Dos => "DOS",
        DesktopIconStyle::Win95 => "Win95",
        DesktopIconStyle::Minimal => "Minimal",
        DesktopIconStyle::NoIcons => "No Icons",
    }
}

fn cli_color_mode_label(mode: CliColorMode) -> &'static str {
    match mode {
        CliColorMode::ThemeLock => "Theme Lock",
        CliColorMode::PaletteMap => "Palette-map",
        CliColorMode::Color => "Color",
        CliColorMode::Monochrome => "Monochrome",
    }
}

fn cli_acs_mode_label(mode: CliAcsMode) -> &'static str {
    match mode {
        CliAcsMode::Ascii => "ASCII",
        CliAcsMode::Unicode => "Unicode Smooth",
    }
}

fn full_color_theme_label(theme_id: &str) -> &'static str {
    match theme_id {
        "nucleon-dark" => "Nucleon Dark",
        "nucleon-light" => "Nucleon Light",
        _ => "Unknown",
    }
}

fn wallpaper_display_name(path: &str) -> String {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return "None".to_string();
    }
    Path::new(trimmed)
        .file_name()
        .and_then(|name| name.to_str())
        .map(str::to_string)
        .unwrap_or_else(|| trimmed.to_string())
}

fn full_color_token_label(token: ColorToken) -> &'static str {
    match token {
        ColorToken::BgPrimary => "Background",
        ColorToken::BgSecondary => "Alt Background",
        ColorToken::FgPrimary => "Text",
        ColorToken::FgSecondary => "Alt Text",
        ColorToken::FgDim => "Dim Text",
        ColorToken::Accent => "Accent",
        ColorToken::AccentHover => "Accent Hover",
        ColorToken::AccentActive => "Accent Active",
        ColorToken::PanelBg => "Panel",
        ColorToken::PanelBorder => "Panel Border",
        ColorToken::WindowChrome => "Window Chrome",
        ColorToken::WindowChromeFocused => "Focused Chrome",
        ColorToken::Selection => "Selection",
        ColorToken::SelectionFg => "Selection Text",
        ColorToken::Border => "Border",
        ColorToken::Separator => "Separator",
        ColorToken::StatusBar => "Status Bar",
        ColorToken::Error => "Error",
        ColorToken::Warning => "Warning",
        ColorToken::Success => "Success",
    }
}

fn terminal_tweaks_section_name(section: u8) -> &'static str {
    match section {
        0 => "Wallpaper",
        1 => "Theme",
        2 => "Effects",
        _ => "Display",
    }
}

fn terminal_tweaks_indent(row: TerminalTweaksRow) -> usize {
    match row {
        TerminalTweaksRow::SectionSelector(_) => 0,
        TerminalTweaksRow::DropdownOption(_, _) => 4,
        _ => 2,
    }
}

impl NucleonNativeApp {
    pub(super) fn open_tweaks_from_settings(&mut self) {
        self.tweaks_open = true;
        self.prime_desktop_window_defaults(DesktopWindow::Tweaks);
        self.desktop_active_window = Some(WindowInstanceId::primary(DesktopWindow::Tweaks));
    }

    fn activate_desktop_surface_style(&mut self) {
        set_active_desktop_style(self.desktop_active_desktop_style.clone());
        set_active_color_style(
            ShellSurfaceKind::Desktop,
            self.desktop_active_color_style.clone(),
            self.desktop_color_overrides.clone(),
        );
    }

    fn activate_terminal_surface_style(&mut self) {
        set_active_color_style(
            ShellSurfaceKind::Terminal,
            self.terminal_active_color_style.clone(),
            self.terminal_color_overrides.clone(),
        );
    }

    fn clear_desktop_color_overrides(&mut self) {
        self.desktop_color_overrides = None;
        self.tweaks_customize_colors_open = false;
    }

    fn clear_terminal_color_overrides(&mut self) {
        self.terminal_color_overrides = None;
        self.tweaks_customize_colors_open = false;
    }

    fn clear_desktop_theme_pack_selection(&mut self) {
        if self.desktop_active_theme_pack_id.take().is_some() {
            self.icon_cache_dirty = true;
        }
    }

    fn set_desktop_cursor_theme_selection(
        &mut self,
        selection: DesktopCursorThemeSelection,
    ) -> bool {
        let selection = super::canonical_desktop_cursor_theme_selection(selection);
        if self.desktop_active_cursor_theme_selection == selection {
            return false;
        }
        self.desktop_active_cursor_theme_selection = selection;
        self.sync_active_desktop_asset_pack();
        true
    }

    fn apply_desktop_theme_pack_selection(&mut self, theme: &ThemePack) -> bool {
        let next_desktop_style_id =
            super::desktop_style_id_from_theme_pack_id(Some(theme.id.as_str()));
        let next_icon_pack_id = theme.icon_pack_id.clone();
        let next_sound_pack_id = theme.sound_pack_id.clone();
        let next_cursor_pack_id = theme.cursor_pack_id.clone();
        let next_font_pack_id = if self.settings.draft.desktop_font_id.is_some() {
            self.desktop_active_font_id.clone()
        } else {
            super::desktop_default_font_id(
                Some(theme.id.as_str()),
                next_desktop_style_id.as_deref(),
            )
        };
        let next_desktop_style = super::desktop_style_from_selection(
            Some(theme.id.as_str()),
            next_desktop_style_id.as_deref(),
        );
        let next_color_overrides =
            super::theme_pack_color_overrides_from_theme_pack_id(Some(theme.id.as_str()));
        let changed = self.desktop_active_theme_pack_id.as_deref() != Some(theme.id.as_str())
            || self.desktop_active_desktop_style_id != next_desktop_style_id
            || self.desktop_active_icon_pack_id != next_icon_pack_id
            || self.desktop_active_sound_pack_id != next_sound_pack_id
            || self.desktop_active_cursor_pack_id != next_cursor_pack_id
            || self.desktop_active_font_id != next_font_pack_id
            || self.desktop_active_desktop_style != next_desktop_style
            || self.desktop_active_color_style != theme.color_style
            || self.desktop_active_layout != theme.layout_profile
            || self.desktop_color_overrides != next_color_overrides;
        if !changed {
            return false;
        }
        self.desktop_active_theme_pack_id = Some(theme.id.clone());
        self.desktop_active_desktop_style_id = next_desktop_style_id;
        self.desktop_active_icon_pack_id = next_icon_pack_id;
        self.desktop_active_sound_pack_id = next_sound_pack_id;
        self.desktop_active_cursor_pack_id = next_cursor_pack_id;
        self.desktop_active_font_id = next_font_pack_id;
        self.desktop_active_desktop_style = next_desktop_style;
        self.desktop_active_color_style = theme.color_style.clone();
        self.desktop_active_layout = theme.layout_profile.clone();
        self.desktop_color_overrides = next_color_overrides;
        self.tweaks_customize_colors_open = false;
        self.sync_active_sound_pack();
        self.sync_active_desktop_asset_pack();
        self.icon_cache_dirty = true;
        self.activate_desktop_surface_style();
        true
    }

    fn set_desktop_style_id(&mut self, desktop_style_id: Option<String>) -> bool {
        if self.desktop_active_desktop_style_id == desktop_style_id {
            return false;
        }
        self.desktop_active_desktop_style_id = desktop_style_id;
        self.desktop_active_desktop_style = super::desktop_style_from_selection(
            self.desktop_active_theme_pack_id.as_deref(),
            self.desktop_active_desktop_style_id.as_deref(),
        );
        if self.settings.draft.desktop_font_id.is_none() {
            self.desktop_active_font_id = super::desktop_default_font_id(
                self.desktop_active_theme_pack_id.as_deref(),
                self.desktop_active_desktop_style_id.as_deref(),
            );
        }
        true
    }

    fn set_desktop_color_theme_selection(&mut self, theme: &ColorThemeManifest) -> bool {
        let next_overrides = theme
            .full_color_theme
            .as_ref()
            .map(|theme| theme.tokens.clone());
        if self.desktop_active_color_style == theme.color_style
            && self.desktop_color_overrides == next_overrides
        {
            return false;
        }
        self.desktop_active_color_style = theme.color_style.clone();
        self.desktop_color_overrides = next_overrides;
        self.tweaks_customize_colors_open = false;
        self.icon_cache_dirty = true;
        self.activate_desktop_surface_style();
        true
    }

    fn set_desktop_icon_pack_id(&mut self, icon_pack_id: Option<String>) -> bool {
        if self.desktop_active_icon_pack_id == icon_pack_id {
            return false;
        }
        self.desktop_active_icon_pack_id = icon_pack_id;
        self.sync_active_desktop_asset_pack();
        self.icon_cache_dirty = true;
        true
    }

    fn set_desktop_sound_pack_id(&mut self, sound_pack_id: Option<String>) -> bool {
        if self.desktop_active_sound_pack_id == sound_pack_id {
            return false;
        }
        self.desktop_active_sound_pack_id = sound_pack_id;
        self.sync_active_sound_pack();
        true
    }

    fn set_desktop_cursor_pack_id(&mut self, cursor_pack_id: Option<String>) -> bool {
        if self.desktop_active_cursor_pack_id == cursor_pack_id {
            return false;
        }
        self.desktop_active_cursor_pack_id = cursor_pack_id;
        self.sync_active_desktop_asset_pack();
        true
    }

    fn set_desktop_font_id(&mut self, font_id: Option<String>) -> bool {
        let font_id = font_id.or_else(|| {
            super::desktop_default_font_id(
                self.desktop_active_theme_pack_id.as_deref(),
                self.desktop_active_desktop_style_id.as_deref(),
            )
        });
        if self.desktop_active_font_id == font_id {
            return false;
        }
        self.desktop_active_font_id = font_id;
        true
    }

    fn apply_terminal_theme_pack_selection(&mut self, theme: &ThemePack) -> bool {
        let next_terminal_font_id = if self.settings.draft.terminal_font_id.is_some() {
            self.terminal_active_font_id.clone()
        } else {
            super::terminal_default_font_id(Some(theme.id.as_str()), Some("classic"))
        };
        let changed = self.terminal_active_theme_pack_id.as_deref() != Some(theme.id.as_str())
            || self.terminal_active_font_id != next_terminal_font_id
            || self.terminal_color_overrides.is_some();
        if !changed {
            return false;
        }
        self.terminal_active_theme_pack_id = Some(theme.id.clone());
        self.terminal_active_theme = crate::theme::TerminalTheme::classic();
        self.terminal_theme_options = self.terminal_active_theme.default_options.clone();
        self.terminal_active_font_id = next_terminal_font_id;
        super::apply_theme_pack_to_terminal_options(
            theme,
            &self.terminal_active_theme,
            &mut self.terminal_theme_options,
        );
        self.terminal_active_color_style = theme.color_style.clone();
        self.terminal_branding = theme.terminal_branding.clone();
        self.terminal_decoration = theme.terminal_decoration.clone();
        self.terminal_color_overrides =
            super::theme_pack_color_overrides_from_theme_pack_id(Some(theme.id.as_str()));
        self.tweaks_customize_colors_open = false;
        self.activate_terminal_surface_style();
        true
    }

    fn set_terminal_theme_selection(&mut self, manifest: &TerminalThemeManifest) -> bool {
        if self.terminal_active_theme.id == manifest.id {
            return false;
        }
        self.terminal_active_theme = manifest.theme.clone();
        self.terminal_theme_options = self.terminal_active_theme.default_options.clone();
        self.terminal_active_theme_pack_id = None;
        if self.settings.draft.terminal_font_id.is_none() {
            self.terminal_active_font_id =
                super::terminal_default_font_id(None, Some(manifest.id.as_str()));
        }
        self.activate_terminal_surface_style();
        true
    }

    fn set_terminal_font_id(&mut self, font_id: Option<String>) -> bool {
        let font_id = font_id.or_else(|| {
            super::terminal_default_font_id(
                self.terminal_active_theme_pack_id.as_deref(),
                Some(self.terminal_active_theme.id.as_str()),
            )
        });
        if self.terminal_active_font_id == font_id {
            return false;
        }
        self.terminal_active_font_id = font_id;
        true
    }

    fn terminal_theme_option_value(&self, index: usize) -> Option<ThemeOptionValue> {
        let def = self.terminal_active_theme.options_schema.get(index)?;
        Some(match &def.kind {
            ThemeOptionKind::Bool { .. } => ThemeOptionValue::Bool(
                crate::theme::TerminalTheme::get_bool(
                    &self.terminal_theme_options,
                    &def.key,
                    &self.terminal_active_theme.options_schema,
                ),
            ),
            ThemeOptionKind::Choice { .. } => ThemeOptionValue::String(
                crate::theme::TerminalTheme::get_string(
                    &self.terminal_theme_options,
                    &def.key,
                    &self.terminal_active_theme.options_schema,
                ),
            ),
            ThemeOptionKind::Int { .. } => ThemeOptionValue::Int(
                crate::theme::TerminalTheme::get_int(
                    &self.terminal_theme_options,
                    &def.key,
                    &self.terminal_active_theme.options_schema,
                ),
            ),
            ThemeOptionKind::Float { .. } => ThemeOptionValue::Float(
                crate::theme::TerminalTheme::get_float(
                    &self.terminal_theme_options,
                    &def.key,
                    &self.terminal_active_theme.options_schema,
                ),
            ),
        })
    }

    fn set_terminal_theme_option_value(&mut self, key: &str, value: ThemeOptionValue) -> bool {
        if self.terminal_theme_options.get(key) == Some(&value) {
            return false;
        }
        self.terminal_theme_options.insert(key.to_string(), value);
        self.terminal_active_theme_pack_id = None;
        self.activate_terminal_surface_style();
        true
    }

    fn cycle_terminal_theme_choice_option(&mut self, index: usize, delta: i32) -> bool {
        let Some(def) = self.terminal_active_theme.options_schema.get(index) else {
            return false;
        };
        let ThemeOptionKind::Choice { choices, .. } = &def.kind else {
            return false;
        };
        if choices.is_empty() {
            return false;
        }
        let current = crate::theme::TerminalTheme::get_string(
            &self.terminal_theme_options,
            &def.key,
            &self.terminal_active_theme.options_schema,
        );
        let current_index = choices.iter().position(|choice| *choice == current).unwrap_or(0);
        let next_index = (current_index as i32 + delta)
            .clamp(0, choices.len().saturating_sub(1) as i32) as usize;
        let key = def.key.clone();
        let choice = choices[next_index].clone();
        self.set_terminal_theme_option_value(
            &key,
            ThemeOptionValue::String(choice),
        )
    }

    fn set_desktop_color_style_selection(&mut self, style: ColorStyle) -> bool {
        if self.desktop_active_color_style == style && self.desktop_color_overrides.is_none() {
            return false;
        }
        self.desktop_active_color_style = style;
        self.clear_desktop_color_overrides();
        self.icon_cache_dirty = true;
        self.activate_desktop_surface_style();
        true
    }

    fn set_terminal_color_style_selection(&mut self, style: ColorStyle) -> bool {
        if self.terminal_active_color_style == style
            && self.terminal_active_theme_pack_id.is_none()
            && self.terminal_color_overrides.is_none()
        {
            return false;
        }
        self.terminal_active_color_style = style;
        self.terminal_active_theme_pack_id = None;
        self.clear_terminal_color_overrides();
        self.activate_terminal_surface_style();
        true
    }

    fn set_desktop_monochrome_custom_rgb(&mut self, rgb: [u8; 3]) -> bool {
        self.set_desktop_color_style_selection(ColorStyle::Monochrome {
            preset: MonochromePreset::Custom,
            custom_rgb: Some(rgb),
        })
    }

    fn set_terminal_monochrome_custom_rgb(&mut self, rgb: [u8; 3]) -> bool {
        self.set_terminal_color_style_selection(ColorStyle::Monochrome {
            preset: MonochromePreset::Custom,
            custom_rgb: Some(rgb),
        })
    }

    fn set_desktop_full_color_theme(&mut self, theme_id: &str) -> bool {
        self.set_desktop_color_style_selection(ColorStyle::FullColor {
            theme_id: theme_id.to_string(),
        })
    }

    fn set_terminal_full_color_theme(&mut self, theme_id: &str) -> bool {
        self.set_terminal_color_style_selection(ColorStyle::FullColor {
            theme_id: theme_id.to_string(),
        })
    }

    fn draw_surface_sub_tabs(ui: &mut egui::Ui, active_surface: &mut u8, palette: &RetroPalette) {
        ui.horizontal(|ui| {
            for (i, label) in ["Desktop", "Terminal"].iter().enumerate() {
                let active = *active_surface == i as u8;
                let button = ui.add(
                    egui::Button::new(
                        RichText::new(*label)
                            .color(if active {
                                palette.selected_fg
                            } else {
                                palette.fg
                            })
                            .strong(),
                    )
                    .stroke(egui::Stroke::new(
                        if active { 1.5 } else { 0.5 },
                        palette.dim,
                    ))
                    .fill(if active {
                        palette.selected_bg
                    } else {
                        palette.panel
                    }),
                );
                if button.clicked() {
                    *active_surface = i as u8;
                }
            }
        });
        ui.add_space(10.0);
    }

    fn terminal_tweaks_entries(
        &self,
        terminal_themes: &[TerminalThemeManifest],
        font_packs: &[FontPackManifest],
    ) -> Vec<TerminalTweaksEntry> {
        let active_section = self.terminal_tweaks_active_section.min(3);
        let mut entries = Vec::new();

        entries.push(TerminalTweaksEntry::Row(
            TerminalTweaksRow::SectionSelector(0),
        ));
        if active_section == 0 {
            entries.push(TerminalTweaksEntry::Header("  Terminal"));
            entries.extend([
                TerminalTweaksEntry::Row(TerminalTweaksRow::TerminalWallpaperPicker),
                TerminalTweaksEntry::Row(TerminalTweaksRow::TerminalWallpaperMode),
            ]);
        }

        entries.push(TerminalTweaksEntry::Row(
            TerminalTweaksRow::SectionSelector(1),
        ));
        if active_section == 1 {
            entries.push(TerminalTweaksEntry::Header("  Terminal"));
            entries.push(TerminalTweaksEntry::Row(TerminalTweaksRow::TerminalTheme));
            entries.push(TerminalTweaksEntry::Row(
                TerminalTweaksRow::TerminalColorMode,
            ));
            if matches!(
                self.terminal_active_color_style,
                ColorStyle::Monochrome { .. }
            ) {
                entries.push(TerminalTweaksEntry::Row(
                    TerminalTweaksRow::TerminalMonoTheme,
                ));
                if monochrome_theme_name_for_color_style(&self.terminal_active_color_style)
                    == CUSTOM_THEME_NAME
                {
                    entries.extend([
                        TerminalTweaksEntry::Row(TerminalTweaksRow::TerminalCustomRed),
                        TerminalTweaksEntry::Row(TerminalTweaksRow::TerminalCustomGreen),
                        TerminalTweaksEntry::Row(TerminalTweaksRow::TerminalCustomBlue),
                    ]);
                }
            } else {
                entries.push(TerminalTweaksEntry::Row(
                    TerminalTweaksRow::TerminalFullColorTheme,
                ));
            }
            entries.push(TerminalTweaksEntry::Row(TerminalTweaksRow::TerminalFont));
            if !terminal_themes.is_empty() {
                entries.push(TerminalTweaksEntry::Header("  Theme Options"));
                for (index, _) in self.terminal_active_theme.options_schema.iter().enumerate() {
                    entries.push(TerminalTweaksEntry::Row(
                        TerminalTweaksRow::TerminalThemeOption(index),
                    ));
                }
            }
        }

        entries.push(TerminalTweaksEntry::Row(
            TerminalTweaksRow::SectionSelector(2),
        ));
        if active_section == 2 {
            entries.extend([
                TerminalTweaksEntry::Row(TerminalTweaksRow::CrtEnabled),
                TerminalTweaksEntry::Row(TerminalTweaksRow::CrtPreset),
                TerminalTweaksEntry::Row(TerminalTweaksRow::CrtCurvature),
                TerminalTweaksEntry::Row(TerminalTweaksRow::CrtScanlines),
                TerminalTweaksEntry::Row(TerminalTweaksRow::CrtGlow),
                TerminalTweaksEntry::Row(TerminalTweaksRow::CrtBloom),
                TerminalTweaksEntry::Row(TerminalTweaksRow::CrtVignette),
                TerminalTweaksEntry::Row(TerminalTweaksRow::CrtNoise),
                TerminalTweaksEntry::Row(TerminalTweaksRow::CrtFlicker),
                TerminalTweaksEntry::Row(TerminalTweaksRow::CrtJitter),
                TerminalTweaksEntry::Row(TerminalTweaksRow::CrtBurnIn),
                TerminalTweaksEntry::Row(TerminalTweaksRow::CrtGlowLine),
                TerminalTweaksEntry::Row(TerminalTweaksRow::CrtGlowLineSpeed),
                TerminalTweaksEntry::Row(TerminalTweaksRow::CrtPhosphorSoftness),
                TerminalTweaksEntry::Row(TerminalTweaksRow::CrtBrightness),
                TerminalTweaksEntry::Row(TerminalTweaksRow::CrtContrast),
            ]);
        }

        entries.push(TerminalTweaksEntry::Row(
            TerminalTweaksRow::SectionSelector(3),
        ));
        if active_section == 3 {
            entries.extend([
                TerminalTweaksEntry::Row(TerminalTweaksRow::WindowMode),
                TerminalTweaksEntry::Row(TerminalTweaksRow::TerminalLayout),
                TerminalTweaksEntry::Row(TerminalTweaksRow::TerminalStyledPty),
                TerminalTweaksEntry::Row(TerminalTweaksRow::TerminalPtyColorMode),
                TerminalTweaksEntry::Row(TerminalTweaksRow::TerminalBorderGlyphs),
            ]);
        }

        self.inflate_terminal_tweaks_dropdown_entries(entries, terminal_themes, font_packs)
    }

    fn terminal_tweaks_selectable_indices(entries: &[TerminalTweaksEntry]) -> Vec<usize> {
        entries
            .iter()
            .enumerate()
            .filter_map(|(idx, entry)| match entry {
                TerminalTweaksEntry::Header(_) => None,
                TerminalTweaksEntry::Row(_) => Some(idx),
            })
            .collect()
    }

    fn terminal_tweaks_selected_row(
        entries: &[TerminalTweaksEntry],
        selected_idx: usize,
    ) -> Option<TerminalTweaksRow> {
        let selectable = Self::terminal_tweaks_selectable_indices(entries);
        selectable
            .get(selected_idx)
            .and_then(|entry_idx| match entries.get(*entry_idx) {
                Some(TerminalTweaksEntry::Row(row)) => Some(*row),
                _ => None,
            })
    }

    fn terminal_tweaks_viewport_start(
        entries: &[TerminalTweaksEntry],
        selectable_indices: &[usize],
        selected_idx: usize,
        visible_rows: usize,
    ) -> usize {
        if visible_rows == 0 || entries.len() <= visible_rows {
            return 0;
        }
        let selected_entry_idx = selectable_indices.get(selected_idx).copied().unwrap_or(0);
        let lead_rows = 3.min(visible_rows.saturating_sub(1));
        selected_entry_idx
            .saturating_sub(lead_rows)
            .min(entries.len().saturating_sub(visible_rows))
    }

    fn terminal_tweaks_dropdown_for_row(
        &self,
        row: TerminalTweaksRow,
    ) -> Option<TerminalTweaksDropdown> {
        match row {
            TerminalTweaksRow::TerminalWallpaperMode => {
                Some(TerminalTweaksDropdown::TerminalWallpaperMode)
            }
            TerminalTweaksRow::WindowMode => Some(TerminalTweaksDropdown::WindowMode),
            TerminalTweaksRow::CrtPreset => Some(TerminalTweaksDropdown::CrtPreset),
            TerminalTweaksRow::TerminalTheme => Some(TerminalTweaksDropdown::TerminalTheme),
            TerminalTweaksRow::TerminalColorMode => Some(TerminalTweaksDropdown::TerminalColorMode),
            TerminalTweaksRow::TerminalMonoTheme => Some(TerminalTweaksDropdown::TerminalMonoTheme),
            TerminalTweaksRow::TerminalFullColorTheme => {
                Some(TerminalTweaksDropdown::TerminalFullColorTheme)
            }
            TerminalTweaksRow::TerminalFont => Some(TerminalTweaksDropdown::TerminalFont),
            TerminalTweaksRow::TerminalThemeOption(index) => self
                .terminal_active_theme
                .options_schema
                .get(index)
                .and_then(|def| match &def.kind {
                    ThemeOptionKind::Choice { .. } => {
                        Some(TerminalTweaksDropdown::TerminalThemeOptionChoice(index))
                    }
                    _ => None,
                }),
            TerminalTweaksRow::TerminalLayout => Some(TerminalTweaksDropdown::TerminalLayout),
            TerminalTweaksRow::TerminalPtyColorMode => {
                Some(TerminalTweaksDropdown::TerminalPtyColorMode)
            }
            TerminalTweaksRow::TerminalBorderGlyphs => {
                Some(TerminalTweaksDropdown::TerminalBorderGlyphs)
            }
            _ => None,
        }
    }

    fn terminal_tweaks_dropdown_options(
        &self,
        dropdown: TerminalTweaksDropdown,
        terminal_themes: &[TerminalThemeManifest],
        font_packs: &[FontPackManifest],
    ) -> Vec<String> {
        match dropdown {
            TerminalTweaksDropdown::TerminalWallpaperMode => vec![
                wallpaper_size_mode_label(WallpaperSizeMode::DefaultSize).to_string(),
                wallpaper_size_mode_label(WallpaperSizeMode::FitToScreen).to_string(),
                wallpaper_size_mode_label(WallpaperSizeMode::Centered).to_string(),
                wallpaper_size_mode_label(WallpaperSizeMode::Tile).to_string(),
                wallpaper_size_mode_label(WallpaperSizeMode::Stretch).to_string(),
            ],
            TerminalTweaksDropdown::WindowMode => vec![
                NativeStartupWindowMode::Windowed.label().to_string(),
                NativeStartupWindowMode::Maximized.label().to_string(),
                NativeStartupWindowMode::BorderlessFullscreen
                    .label()
                    .to_string(),
                NativeStartupWindowMode::Fullscreen.label().to_string(),
            ],
            TerminalTweaksDropdown::CrtPreset => vec![
                CrtPreset::Off.label().to_string(),
                CrtPreset::Subtle.label().to_string(),
                CrtPreset::Classic.label().to_string(),
                CrtPreset::WornTerminal.label().to_string(),
                CrtPreset::ExtremeRetro.label().to_string(),
                CrtPreset::Custom.label().to_string(),
            ],
            TerminalTweaksDropdown::TerminalTheme => terminal_themes
                .iter()
                .cloned()
                .map(|manifest| manifest.name)
                .collect(),
            TerminalTweaksDropdown::TerminalColorMode => {
                vec!["Monochrome".to_string(), "Full Color".to_string()]
            }
            TerminalTweaksDropdown::TerminalMonoTheme => MONOCHROME_THEME_NAMES
                .iter()
                .map(|name| (*name).to_string())
                .collect(),
            TerminalTweaksDropdown::TerminalFullColorTheme => FullColorTheme::builtin_themes()
                .into_iter()
                .map(|theme| theme.name)
                .collect(),
            TerminalTweaksDropdown::TerminalFont => {
                let mut options = vec!["Fixedsys (Default)".to_string()];
                options.extend(font_packs.iter().map(|manifest| manifest.name.clone()));
                options
            }
            TerminalTweaksDropdown::TerminalThemeOptionChoice(index) => self
                .terminal_active_theme
                .options_schema
                .get(index)
                .and_then(|def| match &def.kind {
                    ThemeOptionKind::Choice { choices, .. } => Some(choices.clone()),
                    _ => None,
                })
                .unwrap_or_default(),
            TerminalTweaksDropdown::TerminalLayout => TerminalLayoutProfile::builtin_layouts()
                .into_iter()
                .map(|profile| profile.name)
                .collect(),
            TerminalTweaksDropdown::TerminalPtyColorMode => vec![
                cli_color_mode_label(CliColorMode::ThemeLock).to_string(),
                cli_color_mode_label(CliColorMode::PaletteMap).to_string(),
                cli_color_mode_label(CliColorMode::Color).to_string(),
                cli_color_mode_label(CliColorMode::Monochrome).to_string(),
            ],
            TerminalTweaksDropdown::TerminalBorderGlyphs => vec![
                cli_acs_mode_label(CliAcsMode::Ascii).to_string(),
                cli_acs_mode_label(CliAcsMode::Unicode).to_string(),
            ],
        }
    }

    fn terminal_tweaks_dropdown_selected_index(
        &self,
        dropdown: TerminalTweaksDropdown,
        terminal_themes: &[TerminalThemeManifest],
        font_packs: &[FontPackManifest],
    ) -> usize {
        match dropdown {
            TerminalTweaksDropdown::TerminalWallpaperMode => {
                match self.settings.draft.terminal_wallpaper_size_mode {
                    WallpaperSizeMode::DefaultSize => 0,
                    WallpaperSizeMode::FitToScreen => 1,
                    WallpaperSizeMode::Centered => 2,
                    WallpaperSizeMode::Tile => 3,
                    WallpaperSizeMode::Stretch => 4,
                }
            }
            TerminalTweaksDropdown::WindowMode => {
                match self.settings.draft.native_startup_window_mode {
                    NativeStartupWindowMode::Windowed => 0,
                    NativeStartupWindowMode::Maximized => 1,
                    NativeStartupWindowMode::BorderlessFullscreen => 2,
                    NativeStartupWindowMode::Fullscreen => 3,
                }
            }
            TerminalTweaksDropdown::CrtPreset => match self.settings.draft.display_effects.preset {
                CrtPreset::Off => 0,
                CrtPreset::Subtle => 1,
                CrtPreset::Classic => 2,
                CrtPreset::WornTerminal => 3,
                CrtPreset::ExtremeRetro => 4,
                CrtPreset::Custom => 5,
            },
            TerminalTweaksDropdown::TerminalTheme => terminal_themes
                    .iter()
                    .position(|manifest| manifest.id == self.terminal_active_theme.id)
                .unwrap_or(0),
            TerminalTweaksDropdown::TerminalColorMode => {
                if matches!(
                    self.terminal_active_color_style,
                    ColorStyle::Monochrome { .. }
                ) {
                    0
                } else {
                    1
                }
            }
            TerminalTweaksDropdown::TerminalMonoTheme => MONOCHROME_THEME_NAMES
                .iter()
                .position(|name| {
                    *name
                        == monochrome_theme_name_for_color_style(&self.terminal_active_color_style)
                })
                .unwrap_or(0),
            TerminalTweaksDropdown::TerminalFullColorTheme => FullColorTheme::builtin_themes()
                .iter()
                .position(|theme| {
                    theme.id
                        == full_color_theme_id_for_color_style(&self.terminal_active_color_style)
                })
                .unwrap_or(0),
            TerminalTweaksDropdown::TerminalFont => self
                .terminal_active_font_id
                .as_deref()
                .and_then(|font_id| {
                    font_packs
                        .iter()
                        .position(|manifest| manifest.id == font_id)
                        .map(|index| index + 1)
                })
                .unwrap_or(0),
            TerminalTweaksDropdown::TerminalThemeOptionChoice(index) => {
                let Some(def) = self.terminal_active_theme.options_schema.get(index) else {
                    return 0;
                };
                let ThemeOptionKind::Choice { choices, .. } = &def.kind else {
                    return 0;
                };
                let current = crate::theme::TerminalTheme::get_string(
                    &self.terminal_theme_options,
                    &def.key,
                    &self.terminal_active_theme.options_schema,
                );
                choices.iter().position(|choice| *choice == current).unwrap_or(0)
            }
            TerminalTweaksDropdown::TerminalLayout => TerminalLayoutProfile::builtin_layouts()
                .iter()
                .position(|profile| profile.id == self.terminal_active_layout.id)
                .unwrap_or(0),
            TerminalTweaksDropdown::TerminalPtyColorMode => {
                match self.settings.draft.cli_color_mode {
                    CliColorMode::ThemeLock => 0,
                    CliColorMode::PaletteMap => 1,
                    CliColorMode::Color => 2,
                    CliColorMode::Monochrome => 3,
                }
            }
            TerminalTweaksDropdown::TerminalBorderGlyphs => {
                match self.settings.draft.cli_acs_mode {
                    CliAcsMode::Ascii => 0,
                    CliAcsMode::Unicode => 1,
                }
            }
        }
    }

    fn append_terminal_tweaks_dropdown_options(
        &self,
        dropdown: TerminalTweaksDropdown,
        entries: &mut Vec<TerminalTweaksEntry>,
        terminal_themes: &[TerminalThemeManifest],
        font_packs: &[FontPackManifest],
    ) {
        for index in 0..self
            .terminal_tweaks_dropdown_options(dropdown, terminal_themes, font_packs)
            .len()
        {
            entries.push(TerminalTweaksEntry::Row(TerminalTweaksRow::DropdownOption(
                dropdown, index,
            )));
        }
    }

    fn inflate_terminal_tweaks_dropdown_entries(
        &self,
        entries: Vec<TerminalTweaksEntry>,
        terminal_themes: &[TerminalThemeManifest],
        font_packs: &[FontPackManifest],
    ) -> Vec<TerminalTweaksEntry> {
        let mut expanded = Vec::with_capacity(entries.len());
        for entry in entries {
            expanded.push(entry);
            if let TerminalTweaksEntry::Row(row) = entry {
                if let Some(dropdown) = self.terminal_tweaks_dropdown_for_row(row) {
                    if self.terminal_tweaks_open_dropdown == Some(dropdown) {
                        self.append_terminal_tweaks_dropdown_options(
                            dropdown,
                            &mut expanded,
                            terminal_themes,
                            font_packs,
                        );
                    }
                }
            }
        }
        expanded
    }

    fn terminal_tweaks_row_label(
        &self,
        row: TerminalTweaksRow,
        terminal_themes: &[TerminalThemeManifest],
        font_packs: &[FontPackManifest],
    ) -> String {
        match row {
            TerminalTweaksRow::SectionSelector(section) => {
                let marker = if self.terminal_tweaks_active_section == section {
                    "[-]"
                } else {
                    "[+]"
                };
                format!("{marker} {}", terminal_tweaks_section_name(section))
            }
            TerminalTweaksRow::DropdownOption(dropdown, index) => {
                let options =
                    self.terminal_tweaks_dropdown_options(dropdown, terminal_themes, font_packs);
                let label = options
                    .get(index)
                    .cloned()
                    .unwrap_or_else(|| "Option".to_string());
                if self
                    .terminal_tweaks_dropdown_selected_index(dropdown, terminal_themes, font_packs)
                    == index
                {
                    format!("{label} (current)")
                } else {
                    label
                }
            }
            TerminalTweaksRow::TerminalWallpaperPicker => format!(
                "Wallpaper File: {} [browse]",
                wallpaper_display_name(&self.settings.draft.terminal_wallpaper)
            ),
            TerminalTweaksRow::TerminalWallpaperMode => format!(
                "Wallpaper Mode: {}",
                wallpaper_size_mode_label(self.settings.draft.terminal_wallpaper_size_mode)
            ),
            TerminalTweaksRow::WindowMode => format!(
                "Window Mode: {}",
                self.settings.draft.native_startup_window_mode.label()
            ),
            TerminalTweaksRow::CrtEnabled => format!(
                "CRT Effects: {}",
                if self.settings.draft.display_effects.enabled {
                    "ON"
                } else {
                    "OFF"
                }
            ),
            TerminalTweaksRow::CrtPreset => format!(
                "CRT Preset: {}",
                self.settings.draft.display_effects.preset.label()
            ),
            TerminalTweaksRow::CrtCurvature => {
                format!(
                    "Curvature: {:.2}",
                    self.settings.draft.display_effects.curvature
                )
            }
            TerminalTweaksRow::CrtScanlines => {
                format!(
                    "Scanlines: {:.2}",
                    self.settings.draft.display_effects.scanlines
                )
            }
            TerminalTweaksRow::CrtGlow => {
                format!("Glow: {:.2}", self.settings.draft.display_effects.glow)
            }
            TerminalTweaksRow::CrtBloom => {
                format!(
                    "Text Bloom: {:.2}",
                    self.settings.draft.display_effects.bloom
                )
            }
            TerminalTweaksRow::CrtVignette => {
                format!(
                    "Vignette: {:.2}",
                    self.settings.draft.display_effects.vignette
                )
            }
            TerminalTweaksRow::CrtNoise => {
                format!("Noise: {:.2}", self.settings.draft.display_effects.noise)
            }
            TerminalTweaksRow::CrtFlicker => {
                format!(
                    "Flicker: {:.2}",
                    self.settings.draft.display_effects.flicker
                )
            }
            TerminalTweaksRow::CrtJitter => {
                format!("Jitter: {:.3}", self.settings.draft.display_effects.jitter)
            }
            TerminalTweaksRow::CrtBurnIn => {
                format!(
                    "Burn-In: {:.2}",
                    self.settings.draft.display_effects.burn_in
                )
            }
            TerminalTweaksRow::CrtGlowLine => {
                format!(
                    "Glow Line: {:.2}",
                    self.settings.draft.display_effects.glow_line
                )
            }
            TerminalTweaksRow::CrtGlowLineSpeed => format!(
                "Glow Line Speed: {:.2}",
                self.settings.draft.display_effects.glow_line_speed
            ),
            TerminalTweaksRow::CrtPhosphorSoftness => format!(
                "Phosphor Softness: {:.2}",
                self.settings.draft.display_effects.phosphor_softness
            ),
            TerminalTweaksRow::CrtBrightness => {
                format!(
                    "Brightness: {:.2}",
                    self.settings.draft.display_effects.brightness
                )
            }
            TerminalTweaksRow::CrtContrast => {
                format!(
                    "Contrast: {:.2}",
                    self.settings.draft.display_effects.contrast
                )
            }
            TerminalTweaksRow::TerminalTheme => format!(
                "Terminal: {}",
                selected_terminal_theme_name(&self.terminal_active_theme.id, terminal_themes)
            ),
            TerminalTweaksRow::TerminalColorMode => format!(
                "Color Mode: {}",
                if matches!(
                    self.terminal_active_color_style,
                    ColorStyle::Monochrome { .. }
                ) {
                    "Monochrome"
                } else {
                    "Full Color"
                }
            ),
            TerminalTweaksRow::TerminalMonoTheme => format!(
                "Monochrome Theme: {}",
                monochrome_theme_name_for_color_style(&self.terminal_active_color_style)
            ),
            TerminalTweaksRow::TerminalCustomRed => format!(
                "Custom Red: {}",
                custom_rgb_for_color_style(&self.terminal_active_color_style)[0]
            ),
            TerminalTweaksRow::TerminalCustomGreen => format!(
                "Custom Green: {}",
                custom_rgb_for_color_style(&self.terminal_active_color_style)[1]
            ),
            TerminalTweaksRow::TerminalCustomBlue => format!(
                "Custom Blue: {}",
                custom_rgb_for_color_style(&self.terminal_active_color_style)[2]
            ),
            TerminalTweaksRow::TerminalFullColorTheme => format!(
                "Full Color Theme: {}",
                full_color_theme_label(full_color_theme_id_for_color_style(
                    &self.terminal_active_color_style,
                ))
            ),
            TerminalTweaksRow::TerminalFont => format!(
                "Font: {}",
                selected_font_name(self.terminal_active_font_id.as_deref(), font_packs)
            ),
            TerminalTweaksRow::TerminalThemeOption(index) => {
                let Some(def) = self.terminal_active_theme.options_schema.get(index) else {
                    return "Theme Option".to_string();
                };
                match self.terminal_theme_option_value(index) {
                    Some(ThemeOptionValue::Bool(value)) => {
                        format!("{}: [{}]", def.label, if value { "ON" } else { "OFF" })
                    }
                    Some(ThemeOptionValue::String(value)) => {
                        format!("{}: [{}]", def.label, value)
                    }
                    Some(ThemeOptionValue::Int(value)) => format!("{}: {}", def.label, value),
                    Some(ThemeOptionValue::Float(value)) => {
                        format!("{}: {:.1}", def.label, value)
                    }
                    None => def.label.clone(),
                }
            }
            TerminalTweaksRow::TerminalLayout => {
                format!("Terminal Layout: {}", self.terminal_active_layout.name)
            }
            TerminalTweaksRow::TerminalStyledPty => format!(
                "Styled PTY Rendering: {}",
                if self.settings.draft.cli_styled_render {
                    "ON"
                } else {
                    "OFF"
                }
            ),
            TerminalTweaksRow::TerminalPtyColorMode => format!(
                "PTY Color Mode: {}",
                cli_color_mode_label(self.settings.draft.cli_color_mode)
            ),
            TerminalTweaksRow::TerminalBorderGlyphs => format!(
                "Border Glyphs: {}",
                cli_acs_mode_label(self.settings.draft.cli_acs_mode)
            ),
        }
    }

    fn terminal_tweaks_row_help(&self, row: TerminalTweaksRow) -> String {
        match row {
            TerminalTweaksRow::SectionSelector(section) => format!(
                "Enter opens {}. Only one section stays expanded at a time.",
                terminal_tweaks_section_name(section)
            ),
            TerminalTweaksRow::DropdownOption(_, _) => {
                "Applies this value and closes the option list.".to_string()
            }
            TerminalTweaksRow::TerminalWallpaperPicker => {
                "Enter opens the terminal wallpaper picker and returns here after selection."
                    .to_string()
            }
            TerminalTweaksRow::WindowMode => {
                "Enter opens the window-mode list. The selected mode persists across launches."
                    .to_string()
            }
            TerminalTweaksRow::CrtEnabled | TerminalTweaksRow::CrtPreset => {
                if matches!(row, TerminalTweaksRow::CrtEnabled) {
                    "CRT tweaks drive the same display-effects backend as desktop Tweaks."
                        .to_string()
                } else {
                    "Enter opens the CRT preset list.".to_string()
                }
            }
            TerminalTweaksRow::CrtCurvature
            | TerminalTweaksRow::CrtScanlines
            | TerminalTweaksRow::CrtGlow
            | TerminalTweaksRow::CrtBloom
            | TerminalTweaksRow::CrtVignette
            | TerminalTweaksRow::CrtNoise
            | TerminalTweaksRow::CrtFlicker
            | TerminalTweaksRow::CrtJitter
            | TerminalTweaksRow::CrtBurnIn
            | TerminalTweaksRow::CrtGlowLine
            | TerminalTweaksRow::CrtGlowLineSpeed
            | TerminalTweaksRow::CrtPhosphorSoftness
            | TerminalTweaksRow::CrtBrightness
            | TerminalTweaksRow::CrtContrast => {
                if self.settings.draft.display_effects.enabled {
                    "Left/Right tunes the active CRT profile and marks it Custom.".to_string()
                } else {
                    "Enable CRT effects first to tune these values.".to_string()
                }
            }
            TerminalTweaksRow::TerminalTheme => {
                "Enter opens the installed terminal-theme list.".to_string()
            }
            TerminalTweaksRow::TerminalColorMode => {
                "Enter opens the color-mode list for this surface.".to_string()
            }
            TerminalTweaksRow::TerminalMonoTheme | TerminalTweaksRow::TerminalFullColorTheme => {
                "Enter opens the available theme variants.".to_string()
            }
            TerminalTweaksRow::TerminalFont => {
                "Enter opens the installed font list for this surface.".to_string()
            }
            TerminalTweaksRow::TerminalCustomRed
            | TerminalTweaksRow::TerminalCustomGreen
            | TerminalTweaksRow::TerminalCustomBlue => {
                "Adjust the custom monochrome tint channel.".to_string()
            }
            TerminalTweaksRow::TerminalThemeOption(index) => self
                .terminal_active_theme
                .options_schema
                .get(index)
                .map(|def| {
                    if def.description.is_empty() {
                        "Adjust this terminal theme option.".to_string()
                    } else {
                        def.description.clone()
                    }
                })
                .unwrap_or_else(|| "Adjust this terminal theme option.".to_string()),
            TerminalTweaksRow::TerminalLayout => {
                "Enter opens the layout list for this surface.".to_string()
            }
            TerminalTweaksRow::TerminalStyledPty => {
                "Styled PTY rendering keeps ANSI decorations in terminal output.".to_string()
            }
            TerminalTweaksRow::TerminalPtyColorMode => {
                "Enter opens the PTY color-mode list.".to_string()
            }
            TerminalTweaksRow::TerminalBorderGlyphs => {
                "Enter opens the border glyph list.".to_string()
            }
            TerminalTweaksRow::TerminalWallpaperMode => {
                "Enter opens the wallpaper sizing list.".to_string()
            }
        }
    }

    fn apply_terminal_tweaks_dropdown_selection(
        &mut self,
        dropdown: TerminalTweaksDropdown,
        index: usize,
        terminal_themes: &[TerminalThemeManifest],
        font_packs: &[FontPackManifest],
    ) -> (TerminalTweaksMutation, bool) {
        let mut mutation = TerminalTweaksMutation::default();
        let mut changed = false;
        match dropdown {
            TerminalTweaksDropdown::TerminalWallpaperMode => {
                let options = [
                    WallpaperSizeMode::DefaultSize,
                    WallpaperSizeMode::FitToScreen,
                    WallpaperSizeMode::Centered,
                    WallpaperSizeMode::Tile,
                    WallpaperSizeMode::Stretch,
                ];
                if let Some(next) = options.get(index).copied() {
                    if next != self.settings.draft.terminal_wallpaper_size_mode {
                        self.settings.draft.terminal_wallpaper_size_mode = next;
                        mutation.persist_changed = true;
                        changed = true;
                    }
                }
            }
            TerminalTweaksDropdown::WindowMode => {
                let options = [
                    NativeStartupWindowMode::Windowed,
                    NativeStartupWindowMode::Maximized,
                    NativeStartupWindowMode::BorderlessFullscreen,
                    NativeStartupWindowMode::Fullscreen,
                ];
                if let Some(next) = options.get(index).copied() {
                    if next != self.settings.draft.native_startup_window_mode {
                        self.settings.draft.native_startup_window_mode = next;
                        mutation.persist_changed = true;
                        mutation.window_mode_changed = true;
                        changed = true;
                    }
                }
            }
            TerminalTweaksDropdown::CrtPreset => {
                let options = [
                    CrtPreset::Off,
                    CrtPreset::Subtle,
                    CrtPreset::Classic,
                    CrtPreset::WornTerminal,
                    CrtPreset::ExtremeRetro,
                    CrtPreset::Custom,
                ];
                if let Some(next) = options.get(index).copied() {
                    if next != self.settings.draft.display_effects.preset {
                        if next == CrtPreset::Custom {
                            self.settings.draft.display_effects.preset = next;
                        } else {
                            self.settings.draft.display_effects.apply_preset(next);
                        }
                        mutation.persist_changed = true;
                        changed = true;
                    }
                }
            }
            TerminalTweaksDropdown::TerminalTheme => {
                if let Some(manifest) = terminal_themes.get(index) {
                    if self.set_terminal_theme_selection(manifest) {
                        mutation.appearance_changed = true;
                        changed = true;
                    }
                }
            }
            TerminalTweaksDropdown::TerminalColorMode => {
                let next = match index {
                    0 => Some(ColorStyle::Monochrome {
                        preset: MonochromePreset::Green,
                        custom_rgb: None,
                    }),
                    1 => Some(ColorStyle::FullColor {
                        theme_id: "nucleon-dark".to_string(),
                    }),
                    _ => None,
                };
                if let Some(next_style) = next {
                    if self.set_terminal_color_style_selection(next_style) {
                        mutation.appearance_changed = true;
                        changed = true;
                    }
                }
            }
            TerminalTweaksDropdown::TerminalMonoTheme => {
                if let Some(next) = MONOCHROME_THEME_NAMES.get(index).copied() {
                    let next_style = color_style_from_theme_name(
                        next,
                        custom_rgb_for_color_style(&self.terminal_active_color_style),
                    );
                    if self.set_terminal_color_style_selection(next_style) {
                        mutation.appearance_changed = true;
                        changed = true;
                    }
                }
            }
            TerminalTweaksDropdown::TerminalFullColorTheme => {
                if let Some(theme) = FullColorTheme::builtin_themes().get(index) {
                    if self.set_terminal_full_color_theme(&theme.id) {
                        mutation.appearance_changed = true;
                        changed = true;
                    }
                }
            }
            TerminalTweaksDropdown::TerminalFont => {
                let next_font_id = if index == 0 {
                    None
                } else {
                    font_packs.get(index - 1).map(|manifest| manifest.id.clone())
                };
                if self.set_terminal_font_id(next_font_id) {
                    mutation.appearance_changed = true;
                    changed = true;
                }
            }
            TerminalTweaksDropdown::TerminalThemeOptionChoice(option_index) => {
                let Some(def) = self.terminal_active_theme.options_schema.get(option_index) else {
                    let action_taken = self.terminal_tweaks_open_dropdown.is_some();
                    self.terminal_tweaks_open_dropdown = None;
                    return (mutation, action_taken);
                };
                let ThemeOptionKind::Choice { choices, .. } = &def.kind else {
                    let action_taken = self.terminal_tweaks_open_dropdown.is_some();
                    self.terminal_tweaks_open_dropdown = None;
                    return (mutation, action_taken);
                };
                if let Some(choice) = choices.get(index) {
                    let key = def.key.clone();
                    let value = choice.clone();
                    if self.set_terminal_theme_option_value(
                        &key,
                        ThemeOptionValue::String(value),
                    ) {
                        mutation.appearance_changed = true;
                        changed = true;
                    }
                }
            }
            TerminalTweaksDropdown::TerminalLayout => {
                if let Some(profile) = TerminalLayoutProfile::builtin_layouts().get(index) {
                    if profile.id != self.terminal_active_layout.id {
                        self.terminal_active_layout = profile.clone();
                        mutation.appearance_changed = true;
                        changed = true;
                    }
                }
            }
            TerminalTweaksDropdown::TerminalPtyColorMode => {
                let options = [
                    CliColorMode::ThemeLock,
                    CliColorMode::PaletteMap,
                    CliColorMode::Color,
                    CliColorMode::Monochrome,
                ];
                if let Some(next) = options.get(index).copied() {
                    if next != self.settings.draft.cli_color_mode {
                        self.settings.draft.cli_color_mode = next;
                        mutation.persist_changed = true;
                        changed = true;
                    }
                }
            }
            TerminalTweaksDropdown::TerminalBorderGlyphs => {
                let options = [CliAcsMode::Ascii, CliAcsMode::Unicode];
                if let Some(next) = options.get(index).copied() {
                    if next != self.settings.draft.cli_acs_mode {
                        self.settings.draft.cli_acs_mode = next;
                        mutation.persist_changed = true;
                        changed = true;
                    }
                }
            }
        }
        let action_taken = changed || self.terminal_tweaks_open_dropdown.is_some();
        self.terminal_tweaks_open_dropdown = None;
        (mutation, action_taken)
    }

    fn apply_terminal_tweaks_step(
        &mut self,
        row: TerminalTweaksRow,
        step: TerminalTweaksStep,
        terminal_themes: &[TerminalThemeManifest],
        font_packs: &[FontPackManifest],
    ) -> (TerminalTweaksMutation, bool) {
        let mut mutation = TerminalTweaksMutation::default();
        let mut action_taken = false;
        let previously_open_dropdown = self.terminal_tweaks_open_dropdown;
        if !matches!(row, TerminalTweaksRow::DropdownOption(_, _)) {
            self.terminal_tweaks_open_dropdown = None;
        }
        match row {
            TerminalTweaksRow::SectionSelector(section) => {
                if matches!(step, TerminalTweaksStep::Activate)
                    && self.terminal_tweaks_active_section != section
                {
                    self.terminal_tweaks_active_section = section;
                    action_taken = true;
                }
            }
            TerminalTweaksRow::DropdownOption(dropdown, index) => {
                return self.apply_terminal_tweaks_dropdown_selection(
                    dropdown,
                    index,
                    terminal_themes,
                    font_packs,
                );
            }
            TerminalTweaksRow::TerminalWallpaperPicker => {
                if matches!(step, TerminalTweaksStep::Activate) {
                    let start =
                        wallpaper_browser_start_dir(&self.settings.draft.terminal_wallpaper);
                    self.picking_terminal_wallpaper = true;
                    self.open_document_browser_at(start, TerminalScreen::Settings);
                    self.apply_status_update(clear_shell_status());
                    action_taken = true;
                }
            }
            TerminalTweaksRow::TerminalWallpaperMode => {
                if matches!(step, TerminalTweaksStep::Activate) {
                    self.terminal_tweaks_open_dropdown = if previously_open_dropdown
                        == Some(TerminalTweaksDropdown::TerminalWallpaperMode)
                    {
                        None
                    } else {
                        Some(TerminalTweaksDropdown::TerminalWallpaperMode)
                    };
                    action_taken = true;
                }
            }
            TerminalTweaksRow::WindowMode => {
                if matches!(step, TerminalTweaksStep::Activate) {
                    self.terminal_tweaks_open_dropdown =
                        if previously_open_dropdown == Some(TerminalTweaksDropdown::WindowMode) {
                            None
                        } else {
                            Some(TerminalTweaksDropdown::WindowMode)
                        };
                    action_taken = true;
                }
            }
            TerminalTweaksRow::CrtEnabled => {
                self.settings.draft.display_effects.enabled =
                    !self.settings.draft.display_effects.enabled;
                if self.settings.draft.display_effects.enabled
                    && self.settings.draft.display_effects.preset == CrtPreset::Off
                {
                    self.settings
                        .draft
                        .display_effects
                        .apply_preset(CrtPreset::Classic);
                }
                mutation.persist_changed = true;
                action_taken = true;
            }
            TerminalTweaksRow::CrtPreset => {
                if matches!(step, TerminalTweaksStep::Activate) {
                    self.terminal_tweaks_open_dropdown =
                        if previously_open_dropdown == Some(TerminalTweaksDropdown::CrtPreset) {
                            None
                        } else {
                            Some(TerminalTweaksDropdown::CrtPreset)
                        };
                    action_taken = true;
                }
            }
            TerminalTweaksRow::CrtCurvature
            | TerminalTweaksRow::CrtScanlines
            | TerminalTweaksRow::CrtGlow
            | TerminalTweaksRow::CrtBloom
            | TerminalTweaksRow::CrtVignette
            | TerminalTweaksRow::CrtNoise
            | TerminalTweaksRow::CrtFlicker
            | TerminalTweaksRow::CrtJitter
            | TerminalTweaksRow::CrtBurnIn
            | TerminalTweaksRow::CrtGlowLine
            | TerminalTweaksRow::CrtGlowLineSpeed
            | TerminalTweaksRow::CrtPhosphorSoftness
            | TerminalTweaksRow::CrtBrightness
            | TerminalTweaksRow::CrtContrast => {
                if !self.settings.draft.display_effects.enabled {
                    return (mutation, false);
                }
                let changed = match row {
                    TerminalTweaksRow::CrtCurvature => adjust_f32(
                        &mut self.settings.draft.display_effects.curvature,
                        step.delta(),
                        0.0,
                        0.2,
                        0.01,
                    ),
                    TerminalTweaksRow::CrtScanlines => adjust_f32(
                        &mut self.settings.draft.display_effects.scanlines,
                        step.delta(),
                        0.0,
                        1.0,
                        0.05,
                    ),
                    TerminalTweaksRow::CrtGlow => adjust_f32(
                        &mut self.settings.draft.display_effects.glow,
                        step.delta(),
                        0.0,
                        1.5,
                        0.05,
                    ),
                    TerminalTweaksRow::CrtBloom => adjust_f32(
                        &mut self.settings.draft.display_effects.bloom,
                        step.delta(),
                        0.0,
                        1.5,
                        0.05,
                    ),
                    TerminalTweaksRow::CrtVignette => adjust_f32(
                        &mut self.settings.draft.display_effects.vignette,
                        step.delta(),
                        0.0,
                        1.0,
                        0.05,
                    ),
                    TerminalTweaksRow::CrtNoise => adjust_f32(
                        &mut self.settings.draft.display_effects.noise,
                        step.delta(),
                        0.0,
                        0.35,
                        0.01,
                    ),
                    TerminalTweaksRow::CrtFlicker => adjust_f32(
                        &mut self.settings.draft.display_effects.flicker,
                        step.delta(),
                        0.0,
                        0.3,
                        0.01,
                    ),
                    TerminalTweaksRow::CrtJitter => adjust_f32(
                        &mut self.settings.draft.display_effects.jitter,
                        step.delta(),
                        0.0,
                        0.12,
                        0.005,
                    ),
                    TerminalTweaksRow::CrtBurnIn => adjust_f32(
                        &mut self.settings.draft.display_effects.burn_in,
                        step.delta(),
                        0.0,
                        1.0,
                        0.05,
                    ),
                    TerminalTweaksRow::CrtGlowLine => adjust_f32(
                        &mut self.settings.draft.display_effects.glow_line,
                        step.delta(),
                        0.0,
                        1.0,
                        0.05,
                    ),
                    TerminalTweaksRow::CrtGlowLineSpeed => adjust_f32(
                        &mut self.settings.draft.display_effects.glow_line_speed,
                        step.delta(),
                        0.2,
                        2.0,
                        0.05,
                    ),
                    TerminalTweaksRow::CrtPhosphorSoftness => adjust_f32(
                        &mut self.settings.draft.display_effects.phosphor_softness,
                        step.delta(),
                        0.0,
                        1.0,
                        0.05,
                    ),
                    TerminalTweaksRow::CrtBrightness => adjust_f32(
                        &mut self.settings.draft.display_effects.brightness,
                        step.delta(),
                        0.7,
                        1.4,
                        0.05,
                    ),
                    TerminalTweaksRow::CrtContrast => adjust_f32(
                        &mut self.settings.draft.display_effects.contrast,
                        step.delta(),
                        0.8,
                        1.5,
                        0.05,
                    ),
                    _ => false,
                };
                if changed {
                    self.settings.draft.display_effects.mark_custom();
                    mutation.persist_changed = true;
                    action_taken = true;
                }
            }
            TerminalTweaksRow::TerminalTheme => {
                if matches!(step, TerminalTweaksStep::Activate) {
                    self.terminal_tweaks_open_dropdown = if previously_open_dropdown
                        == Some(TerminalTweaksDropdown::TerminalTheme)
                    {
                        None
                    } else {
                        Some(TerminalTweaksDropdown::TerminalTheme)
                    };
                    action_taken = true;
                }
            }
            TerminalTweaksRow::TerminalColorMode => {
                if matches!(step, TerminalTweaksStep::Activate) {
                    self.terminal_tweaks_open_dropdown = if previously_open_dropdown
                        == Some(TerminalTweaksDropdown::TerminalColorMode)
                    {
                        None
                    } else {
                        Some(TerminalTweaksDropdown::TerminalColorMode)
                    };
                    action_taken = true;
                }
            }
            TerminalTweaksRow::TerminalMonoTheme => {
                if matches!(step, TerminalTweaksStep::Activate) {
                    self.terminal_tweaks_open_dropdown = if previously_open_dropdown
                        == Some(TerminalTweaksDropdown::TerminalMonoTheme)
                    {
                        None
                    } else {
                        Some(TerminalTweaksDropdown::TerminalMonoTheme)
                    };
                    action_taken = true;
                }
            }
            TerminalTweaksRow::TerminalCustomRed
            | TerminalTweaksRow::TerminalCustomGreen
            | TerminalTweaksRow::TerminalCustomBlue => {
                let mut rgb = custom_rgb_for_color_style(&self.terminal_active_color_style);
                let changed = match row {
                    TerminalTweaksRow::TerminalCustomRed => adjust_u8(&mut rgb[0], step.delta()),
                    TerminalTweaksRow::TerminalCustomGreen => adjust_u8(&mut rgb[1], step.delta()),
                    TerminalTweaksRow::TerminalCustomBlue => adjust_u8(&mut rgb[2], step.delta()),
                    _ => false,
                };
                if changed && self.set_terminal_monochrome_custom_rgb(rgb) {
                    mutation.appearance_changed = true;
                    action_taken = true;
                }
            }
            TerminalTweaksRow::TerminalThemeOption(index) => {
                let Some(def) = self.terminal_active_theme.options_schema.get(index).cloned() else {
                    return (mutation, false);
                };
                match def.kind {
                    ThemeOptionKind::Bool { .. } => {
                        if matches!(step, TerminalTweaksStep::Activate) {
                            let current = matches!(
                                self.terminal_theme_option_value(index),
                                Some(ThemeOptionValue::Bool(true))
                            );
                            if self.set_terminal_theme_option_value(
                                &def.key,
                                ThemeOptionValue::Bool(!current),
                            ) {
                                mutation.appearance_changed = true;
                                action_taken = true;
                            }
                        }
                    }
                    ThemeOptionKind::Choice { .. } => {
                        if matches!(step, TerminalTweaksStep::Activate) {
                            let dropdown =
                                TerminalTweaksDropdown::TerminalThemeOptionChoice(index);
                            self.terminal_tweaks_open_dropdown =
                                if previously_open_dropdown == Some(dropdown) {
                                    None
                                } else {
                                    Some(dropdown)
                                };
                            action_taken = true;
                        } else if self.cycle_terminal_theme_choice_option(index, step.delta()) {
                            mutation.appearance_changed = true;
                            action_taken = true;
                        }
                    }
                    ThemeOptionKind::Int { min, max, .. } => {
                        if matches!(step, TerminalTweaksStep::Activate) {
                            return (mutation, false);
                        }
                        let current = match self.terminal_theme_option_value(index) {
                            Some(ThemeOptionValue::Int(value)) => value,
                            _ => 0,
                        };
                        let next = (current + step.delta()).clamp(min, max);
                        if next != current
                            && self.set_terminal_theme_option_value(
                                &def.key,
                                ThemeOptionValue::Int(next),
                            )
                        {
                            mutation.appearance_changed = true;
                            action_taken = true;
                        }
                    }
                    ThemeOptionKind::Float { min, max, .. } => {
                        if matches!(step, TerminalTweaksStep::Activate) {
                            return (mutation, false);
                        }
                        let current = match self.terminal_theme_option_value(index) {
                            Some(ThemeOptionValue::Float(value)) => value,
                            _ => 0.0,
                        };
                        let next = (current + step.delta() as f32 * 0.1).clamp(min, max);
                        if (next - current).abs() > f32::EPSILON
                            && self.set_terminal_theme_option_value(
                                &def.key,
                                ThemeOptionValue::Float((next * 10.0).round() / 10.0),
                            )
                        {
                            mutation.appearance_changed = true;
                            action_taken = true;
                        }
                    }
                }
            }
            TerminalTweaksRow::TerminalFullColorTheme => {
                if matches!(step, TerminalTweaksStep::Activate) {
                    self.terminal_tweaks_open_dropdown = if previously_open_dropdown
                        == Some(TerminalTweaksDropdown::TerminalFullColorTheme)
                    {
                        None
                    } else {
                        Some(TerminalTweaksDropdown::TerminalFullColorTheme)
                    };
                    action_taken = true;
                }
            }
            TerminalTweaksRow::TerminalFont => {
                if matches!(step, TerminalTweaksStep::Activate) {
                    self.terminal_tweaks_open_dropdown = if previously_open_dropdown
                        == Some(TerminalTweaksDropdown::TerminalFont)
                    {
                        None
                    } else {
                        Some(TerminalTweaksDropdown::TerminalFont)
                    };
                    action_taken = true;
                }
            }
            TerminalTweaksRow::TerminalLayout => {
                if matches!(step, TerminalTweaksStep::Activate) {
                    self.terminal_tweaks_open_dropdown = if previously_open_dropdown
                        == Some(TerminalTweaksDropdown::TerminalLayout)
                    {
                        None
                    } else {
                        Some(TerminalTweaksDropdown::TerminalLayout)
                    };
                    action_taken = true;
                }
            }
            TerminalTweaksRow::TerminalStyledPty => {
                self.settings.draft.cli_styled_render = !self.settings.draft.cli_styled_render;
                mutation.persist_changed = true;
                action_taken = true;
            }
            TerminalTweaksRow::TerminalPtyColorMode => {
                if matches!(step, TerminalTweaksStep::Activate) {
                    self.terminal_tweaks_open_dropdown = if previously_open_dropdown
                        == Some(TerminalTweaksDropdown::TerminalPtyColorMode)
                    {
                        None
                    } else {
                        Some(TerminalTweaksDropdown::TerminalPtyColorMode)
                    };
                    action_taken = true;
                }
            }
            TerminalTweaksRow::TerminalBorderGlyphs => {
                if matches!(step, TerminalTweaksStep::Activate) {
                    self.terminal_tweaks_open_dropdown = if previously_open_dropdown
                        == Some(TerminalTweaksDropdown::TerminalBorderGlyphs)
                    {
                        None
                    } else {
                        Some(TerminalTweaksDropdown::TerminalBorderGlyphs)
                    };
                    action_taken = true;
                }
            }
        }
        (mutation, action_taken)
    }

    fn export_full_color_theme(&mut self, terminal: bool) {
        let base_id = if terminal {
            full_color_theme_id_for_color_style(&self.terminal_active_color_style)
        } else {
            full_color_theme_id_for_color_style(&self.desktop_active_color_style)
        };
        let mut theme =
            FullColorTheme::builtin_by_id(base_id).unwrap_or_else(FullColorTheme::nucleon_dark);
        let overrides = if terminal {
            self.terminal_color_overrides.as_ref()
        } else {
            self.desktop_color_overrides.as_ref()
        };
        if let Some(overrides) = overrides {
            for (token, color) in overrides {
                theme.tokens.insert(*token, *color);
            }
        }
        theme.id = if terminal {
            format!("{base_id}-terminal-custom")
        } else {
            format!("{base_id}-custom")
        };
        theme.name = if terminal {
            format!("{} (Terminal Custom)", theme.name)
        } else {
            format!("{} (Custom)", theme.name)
        };
        let bundle_id = theme
            .name
            .to_ascii_lowercase()
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() {
                    c
                } else if c == ' ' || c == '-' || c == '_' {
                    '-'
                } else {
                    '_'
                }
            })
            .collect::<String>()
            .trim_matches('-')
            .to_string();
        let export_dir = crate::config::nucleon_data_dir()
            .join("exported_themes")
            .join(if bundle_id.is_empty() {
                "custom-theme".to_string()
            } else {
                bundle_id.clone()
            });
        if std::fs::create_dir_all(export_dir.join("colors")).is_err() {
            self.apply_status_update(settings_status("Failed to export theme."));
            return;
        }
        let manifest = crate::theme::ThemePack {
            id: if bundle_id.is_empty() {
                "custom-theme".to_string()
            } else {
                bundle_id
            },
            name: theme.name.clone(),
            description: "Exported from Tweaks".to_string(),
            version: "1.0.0".to_string(),
            desktop_style: crate::theme::DesktopStyleRef::Inline(
                self.desktop_active_desktop_style.clone(),
            ),
            color_style: if terminal {
                self.terminal_active_color_style.clone()
            } else {
                self.desktop_active_color_style.clone()
            },
            full_color_theme: Some(theme.clone()),
            terminal_branding: self.terminal_branding.clone(),
            terminal_decoration: self.terminal_decoration.clone(),
            icon_pack_id: None,
            sound_pack_id: None,
            cursor_pack_id: None,
            font_pack_id: None,
            layout_profile: self.desktop_active_layout.clone(),
        };
        let manifest_json = match serde_json::to_string_pretty(&manifest) {
            Ok(json) => json,
            Err(_) => {
                self.apply_status_update(settings_status("Failed to export theme."));
                return;
            }
        };
        let colors_json = match serde_json::to_string_pretty(&theme) {
            Ok(json) => json,
            Err(_) => {
                self.apply_status_update(settings_status("Failed to export theme."));
                return;
            }
        };
        if std::fs::write(export_dir.join("manifest.json"), manifest_json).is_err()
            || std::fs::write(export_dir.join("colors").join("custom.json"), colors_json).is_err()
        {
            self.apply_status_update(settings_status("Failed to export theme."));
            return;
        }
        self.apply_status_update(settings_status(format!(
            "Exported to {}",
            export_dir.display()
        )));
    }

    fn draw_full_color_customization_controls(
        &mut self,
        ui: &mut egui::Ui,
        terminal: bool,
    ) -> bool {
        ui.add_space(6.0);
        let toggle_label = if self.tweaks_customize_colors_open {
            "[-] Customize Colors"
        } else {
            "[+] Customize Colors"
        };
        if ui.button(toggle_label).clicked() {
            self.tweaks_customize_colors_open = !self.tweaks_customize_colors_open;
            if !self.tweaks_customize_colors_open {
                self.tweaks_editing_color_token = None;
            }
        }

        let mut appearance_changed = false;
        if self.tweaks_customize_colors_open {
            let palette = current_palette();
            let base_theme_id = if terminal {
                full_color_theme_id_for_color_style(&self.terminal_active_color_style)
            } else {
                full_color_theme_id_for_color_style(&self.desktop_active_color_style)
            }
            .to_string();
            let base_theme = FullColorTheme::builtin_by_id(&base_theme_id)
                .unwrap_or_else(FullColorTheme::nucleon_dark);
            let mut working_overrides = if terminal {
                self.terminal_color_overrides
                    .clone()
                    .unwrap_or_else(|| base_theme.tokens.clone())
            } else {
                self.desktop_color_overrides
                    .clone()
                    .unwrap_or_else(|| base_theme.tokens.clone())
            };

            let all_tokens = ColorToken::all();

            // Token list — clicking a swatch opens an inline picker right below it
            for (idx, token) in all_tokens.iter().enumerate() {
                let entry = working_overrides
                    .entry(*token)
                    .or_insert([128, 128, 128, 255]);
                let is_selected = self.tweaks_editing_color_token == Some(idx);
                let preview =
                    egui::Color32::from_rgba_unmultiplied(entry[0], entry[1], entry[2], entry[3]);

                ui.horizontal(|ui| {
                    let btn = ui.add(
                        egui::Button::new("")
                            .min_size(egui::vec2(22.0, 22.0))
                            .fill(preview)
                            .stroke(egui::Stroke::new(
                                if is_selected { 2.5 } else { 1.0 },
                                if is_selected {
                                    palette.selected_fg
                                } else {
                                    palette.dim
                                },
                            )),
                    );
                    if btn.clicked() {
                        self.tweaks_editing_color_token =
                            if is_selected { None } else { Some(idx) };
                    }
                    let label_text = if is_selected {
                        RichText::new(full_color_token_label(*token))
                            .color(palette.selected_fg)
                            .strong()
                    } else {
                        RichText::new(full_color_token_label(*token)).color(palette.fg)
                    };
                    ui.label(label_text);
                });

                // Inline color picker — appears directly under the selected token
                if is_selected {
                    let frame_stroke = egui::Stroke::new(1.0, palette.dim);
                    egui::Frame::none()
                        .inner_margin(egui::Margin::same(6.0))
                        .stroke(frame_stroke)
                        .show(ui, |ui| {
                            let saved_spacing = ui.spacing().item_spacing;
                            ui.spacing_mut().item_spacing = egui::vec2(2.0, 2.0);
                            egui::Grid::new(("color_palette_grid", terminal, idx))
                                .num_columns(COLOR_PALETTE_COLUMNS)
                                .show(ui, |ui| {
                                    for (index, swatch) in COLOR_PALETTE.iter().enumerate() {
                                        let swatch_color = egui::Color32::from_rgb(
                                            swatch[0], swatch[1], swatch[2],
                                        );
                                        if ui
                                            .add(
                                                egui::Button::new("")
                                                    .min_size(egui::vec2(20.0, 20.0))
                                                    .fill(swatch_color)
                                                    .stroke(egui::Stroke::new(0.5, palette.dim)),
                                            )
                                            .clicked()
                                        {
                                            let entry = working_overrides
                                                .entry(*token)
                                                .or_insert([128, 128, 128, 255]);
                                            *entry = [swatch[0], swatch[1], swatch[2], 255];
                                            appearance_changed = true;
                                        }
                                        if (index + 1) % COLOR_PALETTE_COLUMNS == 0 {
                                            ui.end_row();
                                        }
                                    }
                                });
                            ui.spacing_mut().item_spacing = saved_spacing;
                            ui.add_space(4.0);
                            let entry = working_overrides
                                .entry(*token)
                                .or_insert([128, 128, 128, 255]);
                            ui.horizontal(|ui| {
                                ui.label(RichText::new("Custom:").color(palette.fg));
                                let mut color = egui::Color32::from_rgba_unmultiplied(
                                    entry[0], entry[1], entry[2], entry[3],
                                );
                                if color_edit_button_srgba(ui, &mut color, Alpha::Opaque).changed()
                                {
                                    *entry = [color.r(), color.g(), color.b(), 255];
                                    appearance_changed = true;
                                }
                            });
                        });
                }
            }

            if appearance_changed {
                if terminal {
                    self.terminal_color_overrides = Some(working_overrides);
                } else {
                    self.desktop_color_overrides = Some(working_overrides);
                }
            }
        }

        if appearance_changed {
            if terminal {
                self.terminal_active_theme_pack_id = None;
                self.activate_terminal_surface_style();
            } else {
                self.activate_desktop_surface_style();
            }
        }

        let has_overrides = if terminal {
            self.terminal_color_overrides.is_some()
        } else {
            self.desktop_color_overrides.is_some()
        };
        if has_overrides {
            ui.add_space(8.0);
            if ui.button("Export Theme...").clicked() {
                self.export_full_color_theme(terminal);
            }
        }

        appearance_changed
    }

    fn draw_layout_override_controls(&mut self, ui: &mut egui::Ui) -> bool {
        let mut changed = false;
        if ui
            .button(if self.tweaks_layout_overrides_open {
                "[-] Layout Overrides"
            } else {
                "[+] Layout Overrides"
            })
            .clicked()
        {
            self.tweaks_layout_overrides_open = !self.tweaks_layout_overrides_open;
        }
        if !self.tweaks_layout_overrides_open {
            return false;
        }

        ui.add_space(8.0);
        ui.small(
            "Changing these values detaches the desktop surface from the selected theme pack and shows it as Custom.",
        );
        ui.add_space(8.0);

        ui.horizontal(|ui| {
            ui.label("Top");
            egui::ComboBox::from_id_salt("layout_top_panel")
                .selected_text(panel_type_label(self.desktop_active_layout.top_panel))
                .show_ui(ui, |ui| {
                    Self::apply_settings_control_style(ui);
                    for panel_type in [PanelType::MenuBar, PanelType::Taskbar, PanelType::Disabled]
                    {
                        if Self::retro_choice_button(
                            ui,
                            panel_type_label(panel_type),
                            self.desktop_active_layout.top_panel == panel_type,
                        )
                        .clicked()
                        {
                            self.desktop_active_layout.top_panel = panel_type;
                            self.clear_desktop_theme_pack_selection();
                            changed = true;
                            ui.close_menu();
                        }
                    }
                });
        });
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.label("Bottom");
            egui::ComboBox::from_id_salt("layout_bottom_panel")
                .selected_text(panel_type_label(self.desktop_active_layout.bottom_panel))
                .show_ui(ui, |ui| {
                    Self::apply_settings_control_style(ui);
                    for panel_type in [PanelType::Taskbar, PanelType::MenuBar, PanelType::Disabled]
                    {
                        if Self::retro_choice_button(
                            ui,
                            panel_type_label(panel_type),
                            self.desktop_active_layout.bottom_panel == panel_type,
                        )
                        .clicked()
                        {
                            self.desktop_active_layout.bottom_panel = panel_type;
                            self.clear_desktop_theme_pack_selection();
                            changed = true;
                            ui.close_menu();
                        }
                    }
                });
        });
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.label("Launcher");
            egui::ComboBox::from_id_salt("layout_launcher_style")
                .selected_text(launcher_style_label(
                    self.desktop_active_layout.launcher_style,
                ))
                .show_ui(ui, |ui| {
                    Self::apply_settings_control_style(ui);
                    for style in [
                        LauncherStyle::StartMenu,
                        LauncherStyle::Overlay,
                        LauncherStyle::Hidden,
                    ] {
                        if Self::retro_choice_button(
                            ui,
                            launcher_style_label(style),
                            self.desktop_active_layout.launcher_style == style,
                        )
                        .clicked()
                        {
                            self.desktop_active_layout.launcher_style = style;
                            self.clear_desktop_theme_pack_selection();
                            changed = true;
                            ui.close_menu();
                        }
                    }
                });
        });
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.label("Window Headers");
            egui::ComboBox::from_id_salt("layout_window_header_style")
                .selected_text(window_header_label(
                    self.desktop_active_layout.window_header_style,
                ))
                .show_ui(ui, |ui| {
                    Self::apply_settings_control_style(ui);
                    for style in [
                        WindowHeaderStyle::Standard,
                        WindowHeaderStyle::Compact,
                        WindowHeaderStyle::Hidden,
                    ] {
                        if Self::retro_choice_button(
                            ui,
                            window_header_label(style),
                            self.desktop_active_layout.window_header_style == style,
                        )
                        .clicked()
                        {
                            self.desktop_active_layout.window_header_style = style;
                            self.clear_desktop_theme_pack_selection();
                            changed = true;
                            ui.close_menu();
                        }
                    }
                });
        });

        changed
    }

    pub(super) fn draw_terminal_tweaks_screen(&mut self, ctx: &Context) {
        let layout = self.terminal_layout();
        let terminal_themes = installed_terminal_themes();
        let font_packs = installed_font_packs();
        let mut entries = self.terminal_tweaks_entries(&terminal_themes, &font_packs);
        let mut selectable_indices = Self::terminal_tweaks_selectable_indices(&entries);
        self.terminal_nav.settings_idx = self
            .terminal_nav
            .settings_idx
            .min(selectable_indices.len().saturating_sub(1));
        let mut mutation = TerminalTweaksMutation::default();

        if ctx.input(|i| i.key_pressed(Key::ArrowUp)) {
            let previous = self.terminal_nav.settings_idx;
            self.terminal_nav.settings_idx = self.terminal_nav.settings_idx.saturating_sub(1);
            if self.terminal_nav.settings_idx != previous {
                crate::sound::play_navigate();
            }
        }
        if ctx.input(|i| i.key_pressed(Key::ArrowDown)) {
            let previous = self.terminal_nav.settings_idx;
            self.terminal_nav.settings_idx = (self.terminal_nav.settings_idx + 1)
                .min(selectable_indices.len().saturating_sub(1));
            if self.terminal_nav.settings_idx != previous {
                crate::sound::play_navigate();
            }
        }

        let keyboard_step = if ctx.input(|i| i.key_pressed(Key::ArrowLeft)) {
            Some(TerminalTweaksStep::Previous)
        } else if ctx.input(|i| i.key_pressed(Key::ArrowRight)) {
            Some(TerminalTweaksStep::Next)
        } else if ctx.input(|i| i.key_pressed(Key::Enter) || i.key_pressed(Key::Space)) {
            Some(TerminalTweaksStep::Activate)
        } else {
            None
        };
        if let Some(step) = keyboard_step {
            if let Some(row) =
                Self::terminal_tweaks_selected_row(&entries, self.terminal_nav.settings_idx)
            {
                let (step_mutation, action_taken) =
                    self.apply_terminal_tweaks_step(row, step, &terminal_themes, &font_packs);
                if action_taken {
                    crate::sound::play_navigate();
                }
                mutation.persist_changed |= step_mutation.persist_changed;
                mutation.appearance_changed |= step_mutation.appearance_changed;
                mutation.window_mode_changed |= step_mutation.window_mode_changed;
                entries = self.terminal_tweaks_entries(&terminal_themes, &font_packs);
                selectable_indices = Self::terminal_tweaks_selectable_indices(&entries);
                self.terminal_nav.settings_idx = self
                    .terminal_nav
                    .settings_idx
                    .min(selectable_indices.len().saturating_sub(1));
            }
        }

        let mut clicked_row = None;

        egui::CentralPanel::default()
            .frame(
                egui::Frame::none()
                    .fill(current_palette_for_surface(ShellSurfaceKind::Terminal).bg)
                    .inner_margin(0.0),
            )
            .show(ctx, |ui| {
                let palette = current_palette_for_surface(ShellSurfaceKind::Terminal);
                let (screen, _) = RetroScreen::new(ui, layout.cols, layout.rows);
                let painter = ui.painter_at(screen.rect);
                screen.paint_terminal_background(&painter, &palette);
                for (idx, line) in self.active_terminal_header_lines().iter().enumerate() {
                    screen.centered_text(
                        &painter,
                        layout.header_start_row + idx,
                        line,
                        palette.fg,
                        true,
                    );
                }
                screen.themed_separator(
                    &painter,
                    layout.separator_top_row,
                    &palette,
                    &self.terminal_decoration,
                );
                screen.themed_title(
                    &painter,
                    layout.title_row,
                    "Tweaks",
                    &palette,
                    &self.terminal_decoration,
                );
                screen.themed_separator(
                    &painter,
                    layout.separator_bottom_row,
                    &palette,
                    &self.terminal_decoration,
                );
                screen.themed_subtitle(
                    &painter,
                    layout.content_col,
                    layout.subtitle_row,
                    "Wallpaper, Theme, Effects, and Display settings",
                    &palette,
                    &self.terminal_decoration,
                );

                let help_row = layout.status_row.saturating_sub(1);
                let visible_rows = help_row.saturating_sub(layout.menu_start_row);
                let viewport_start = Self::terminal_tweaks_viewport_start(
                    &entries,
                    &selectable_indices,
                    self.terminal_nav.settings_idx,
                    visible_rows,
                );
                let viewport_end = (viewport_start + visible_rows).min(entries.len());
                let mut draw_row = layout.menu_start_row;
                for entry_idx in viewport_start..viewport_end {
                    match entries[entry_idx] {
                        TerminalTweaksEntry::Header(label) => {
                            screen.text(&painter, layout.content_col, draw_row, label, palette.dim);
                        }
                        TerminalTweaksEntry::Row(row) => {
                            let selected = selectable_indices
                                .get(self.terminal_nav.settings_idx)
                                .copied()
                                == Some(entry_idx);
                            let text = terminal_menu_row_text(
                                &self.terminal_tweaks_row_label(
                                    row,
                                    &terminal_themes,
                                    &font_packs,
                                ),
                                selected,
                                2 + terminal_tweaks_indent(row),
                            );
                            let response = screen.selectable_row(
                                ui,
                                &painter,
                                &palette,
                                layout.content_col,
                                draw_row,
                                &text,
                                selected,
                            );
                            if response.clicked() {
                                if let Some(selectable_idx) = selectable_indices
                                    .iter()
                                    .position(|candidate| *candidate == entry_idx)
                                {
                                    self.terminal_nav.settings_idx = selectable_idx;
                                }
                                clicked_row = Some(row);
                            }
                        }
                    }
                    draw_row += 1;
                }

                if let Some(selected_row) =
                    Self::terminal_tweaks_selected_row(&entries, self.terminal_nav.settings_idx)
                {
                    screen.text(
                        &painter,
                        layout.content_col,
                        help_row,
                        &self.terminal_tweaks_row_help(selected_row),
                        palette.dim,
                    );
                }
                if !self.shell_status.is_empty() {
                    screen.text(
                        &painter,
                        layout.content_col,
                        layout.status_row,
                        &self.shell_status,
                        palette.dim,
                    );
                }
            });

        if let Some(row) = clicked_row {
            let (step_mutation, action_taken) =
                self.apply_terminal_tweaks_step(
                    row,
                    TerminalTweaksStep::Activate,
                    &terminal_themes,
                    &font_packs,
                );
            if action_taken {
                crate::sound::play_navigate();
            }
            mutation.persist_changed |= step_mutation.persist_changed;
            mutation.appearance_changed |= step_mutation.appearance_changed;
            mutation.window_mode_changed |= step_mutation.window_mode_changed;
        }

        if mutation.appearance_changed {
            self.sync_active_font(ctx);
            self.persist_surface_theme_state_to_settings();
            mutation.persist_changed = true;
        }
        if mutation.persist_changed {
            self.persist_native_settings();
            if mutation.window_mode_changed {
                self.apply_native_window_mode(ctx);
            }
        }
    }

    pub(super) fn draw_tweaks(&mut self, ctx: &Context) {
        if !self.tweaks_open || self.desktop_window_is_minimized(DesktopWindow::Tweaks) {
            return;
        }
        let wid = self.current_window_id(DesktopWindow::Tweaks);
        let mut open = self.tweaks_open;
        let maximized = self.desktop_window_is_maximized(DesktopWindow::Tweaks);
        let restore = self.take_desktop_window_restore_dims(DesktopWindow::Tweaks);
        let mut header_action = DesktopHeaderAction::None;
        let egui_id = self.desktop_window_egui_id(wid);
        let default_size = Self::desktop_default_window_size(DesktopWindow::Tweaks);
        let default_pos = self.active_desktop_default_window_pos(ctx, default_size);
        let mut window = egui::Window::new("Tweaks")
            .id(egui_id)
            .open(&mut open)
            .title_bar(false)
            .frame(self.desktop_window_frame())
            .resizable(false)
            .default_pos(default_pos)
            .fixed_size(default_size);
        if maximized {
            let rect = self.active_desktop_workspace_rect(ctx);
            window = window
                .movable(false)
                .fixed_pos(rect.min)
                .fixed_size(rect.size());
        } else if let Some((pos, _size)) = restore {
            let pos = self.active_desktop_clamp_window_pos(ctx, pos, default_size);
            window = window.current_pos(pos);
        }
        let shown = window.show(ctx, |ui| {
            Self::apply_settings_control_style(ui);
            header_action = Self::draw_desktop_window_header(
                ui,
                "Tweaks",
                maximized,
                self.desktop_active_window == Some(wid),
                &self.desktop_active_desktop_style,
            );
            let mut persist_changed = false;
            let mut window_mode_changed = false;
            let mut desktop_runtime_changed = false;
            let mut appearance_changed = false;
            let palette = current_palette();
            let desktop_styles = installed_desktop_styles();
            let color_themes = installed_color_themes();
            let font_packs = installed_font_packs();
            let icon_packs = installed_icon_packs();
            let sound_packs = installed_sound_packs();
            let cursor_packs = installed_cursor_packs();

            ui.add_space(4.0);
            ui.label(RichText::new("Tweaks").strong().size(28.0));
            ui.add_space(14.0);

            let body_max_height = ui.available_height().max(120.0);
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .max_height(body_max_height)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        for (i, label) in ["Wallpaper", "Theme", "Effects", "Display"]
                            .iter()
                            .enumerate()
                        {
                            let active = self.tweaks_tab == i as u8;
                            let color = if active {
                                palette.selected_fg
                            } else {
                                palette.fg
                            };
                            let btn = ui.add(
                                egui::Button::new(RichText::new(*label).color(color).strong())
                                .stroke(egui::Stroke::new(
                                    if active { 2.0 } else { 1.0 },
                                    palette.fg,
                                ))
                                .fill(if active {
                                    palette.selected_bg
                                } else {
                                    palette.panel
                                }),
                            );
                            if btn.clicked() {
                                self.tweaks_tab = i as u8;
                            }
                        }
                    });
                    ui.add_space(10.0);
                    Self::retro_separator_with_thickness(
                        ui,
                        self.desktop_active_desktop_style.separator_thickness,
                    );
                    ui.add_space(8.0);
                    match self.tweaks_tab {
                        0 => {
                            Self::draw_surface_sub_tabs(
                                ui,
                                &mut self.tweaks_wallpaper_surface,
                                &palette,
                            );
                            match self.tweaks_wallpaper_surface {
                                0 => {
                                    Self::settings_section(ui, "Desktop Wallpaper", |ui| {
                                        ui.label("Wallpaper Path");
                                        ui.horizontal(|ui| {
                                            let width = Self::responsive_input_width(
                                                ui, 0.72, 160.0, 400.0,
                                            );
                                            if ui
                                                .add(
                                                    TextEdit::singleline(
                                                        &mut self.settings.draft.desktop_wallpaper,
                                                    )
                                                    .desired_width(width)
                                                    .hint_text("/path/to/image.png"),
                                                )
                                                .changed()
                                            {
                                                persist_changed = true;
                                            }
                                            if ui.button("Browse...").clicked() {
                                                let start = wallpaper_browser_start_dir(
                                                    &self.settings.draft.desktop_wallpaper,
                                                );
                                                self.picking_wallpaper = true;
                                                self.open_embedded_file_manager_at(start);
                                            }
                                        });
                                        ui.add_space(8.0);
                                        ui.horizontal(|ui| {
                                            ui.label("Wallpaper Mode");
                                            let selected = wallpaper_size_mode_label(
                                                self.settings.draft.desktop_wallpaper_size_mode,
                                            );
                                            egui::ComboBox::from_id_salt(
                                                "native_settings_wallpaper_mode",
                                            )
                                            .selected_text(
                                                RichText::new(selected).color(palette.fg),
                                            )
                                            .show_ui(ui, |ui| {
                                                Self::apply_settings_control_style(ui);
                                                for (mode, label) in [
                                                    (
                                                        WallpaperSizeMode::DefaultSize,
                                                        "Default Size",
                                                    ),
                                                    (
                                                        WallpaperSizeMode::FitToScreen,
                                                        "Fit To Screen",
                                                    ),
                                                    (WallpaperSizeMode::Centered, "Centered"),
                                                    (WallpaperSizeMode::Tile, "Tile"),
                                                    (WallpaperSizeMode::Stretch, "Stretch"),
                                                ] {
                                                    if Self::retro_choice_button(
                                                        ui,
                                                        label,
                                                        self.settings
                                                            .draft
                                                            .desktop_wallpaper_size_mode
                                                            == mode,
                                                    )
                                                    .clicked()
                                                    {
                                                        set_desktop_wallpaper_size_mode(
                                                            &mut self.settings.draft,
                                                            mode,
                                                        );
                                                        persist_changed = true;
                                                        ui.close_menu();
                                                    }
                                                }
                                            });
                                        });
                                    });
                                }
                                _ => {
                                    Self::settings_section(ui, "Terminal Wallpaper", |ui| {
                                        ui.label("Wallpaper Path");
                                        ui.horizontal(|ui| {
                                            let width = Self::responsive_input_width(
                                                ui, 0.72, 160.0, 400.0,
                                            );
                                            if ui
                                                .add(
                                                    TextEdit::singleline(
                                                        &mut self.settings.draft.terminal_wallpaper,
                                                    )
                                                    .desired_width(width)
                                                    .hint_text("/path/to/image.png"),
                                                )
                                                .changed()
                                            {
                                                persist_changed = true;
                                            }
                                            if ui.button("Browse...").clicked() {
                                                let start = wallpaper_browser_start_dir(
                                                    &self.settings.draft.terminal_wallpaper,
                                                );
                                                self.picking_terminal_wallpaper = true;
                                                self.open_embedded_file_manager_at(start);
                                            }
                                        });
                                        ui.add_space(8.0);
                                        ui.horizontal(|ui| {
                                            ui.label("Wallpaper Mode");
                                            let selected = wallpaper_size_mode_label(
                                                self.settings.draft.terminal_wallpaper_size_mode,
                                            );
                                            egui::ComboBox::from_id_salt(
                                                "native_settings_terminal_wallpaper_mode",
                                            )
                                            .selected_text(
                                                RichText::new(selected).color(palette.fg),
                                            )
                                            .show_ui(ui, |ui| {
                                                Self::apply_settings_control_style(ui);
                                                for (mode, label) in [
                                                    (
                                                        WallpaperSizeMode::DefaultSize,
                                                        "Default Size",
                                                    ),
                                                    (
                                                        WallpaperSizeMode::FitToScreen,
                                                        "Fit To Screen",
                                                    ),
                                                    (WallpaperSizeMode::Centered, "Centered"),
                                                    (WallpaperSizeMode::Tile, "Tile"),
                                                    (WallpaperSizeMode::Stretch, "Stretch"),
                                                ] {
                                                    if Self::retro_choice_button(
                                                        ui,
                                                        label,
                                                        self.settings
                                                            .draft
                                                            .terminal_wallpaper_size_mode
                                                            == mode,
                                                    )
                                                    .clicked()
                                                    {
                                                        self.settings.draft
                                                            .terminal_wallpaper_size_mode = mode;
                                                        persist_changed = true;
                                                        ui.close_menu();
                                                    }
                                                }
                                            });
                                        });
                                    });
                                }
                            }
                        }
                        1 => {
                            self.tweaks_theme_surface = 0;
                            match self.tweaks_theme_surface {
                                0 => {
                                    Self::settings_section(ui, "Desktop Theme", |ui| {
                                        ui.horizontal(|ui| {
                                            ui.label("Desktop");
                                            egui::ComboBox::from_id_salt("native_desktop_style")
                                            .selected_text(
                                                RichText::new(selected_desktop_style_name(
                                                    self.desktop_active_desktop_style_id.as_deref(),
                                                    &self.desktop_active_desktop_style,
                                                    &desktop_styles,
                                                ))
                                                .color(palette.fg),
                                            )
                                            .show_ui(ui, |ui| {
                                                Self::apply_settings_control_style(ui);
                                                for manifest in &desktop_styles {
                                                    let selected = self
                                                        .desktop_active_desktop_style_id
                                                        .as_deref()
                                                        == Some(manifest.id.as_str());
                                                    if Self::retro_choice_button(
                                                        ui,
                                                        &manifest.name,
                                                        selected,
                                                    )
                                                    .clicked()
                                                    {
                                                        if self
                                                            .set_desktop_style_id(Some(manifest.id.clone()))
                                                        {
                                                            desktop_runtime_changed = true;
                                                            appearance_changed = true;
                                                        }
                                                        ui.close_menu();
                                                    }
                                                }
                                            });
                                        });
                                        ui.add_space(6.0);
                                        ui.horizontal(|ui| {
                                            ui.label("Colors");
                                            egui::ComboBox::from_id_salt(
                                                "native_desktop_color_theme_component",
                                            )
                                            .selected_text(
                                                RichText::new(selected_color_theme_name(
                                                    &self.desktop_active_color_style,
                                                    &color_themes,
                                                ))
                                                .color(palette.fg),
                                            )
                                            .show_ui(ui, |ui| {
                                                Self::apply_settings_control_style(ui);
                                                for manifest in &color_themes {
                                                    let selected = self.desktop_active_color_style
                                                        == manifest.color_style;
                                                    if Self::retro_choice_button(
                                                        ui,
                                                        &manifest.name,
                                                        selected,
                                                    )
                                                    .clicked()
                                                    {
                                                        if self
                                                            .set_desktop_color_theme_selection(
                                                                manifest,
                                                            )
                                                        {
                                                            desktop_runtime_changed = true;
                                                            appearance_changed = true;
                                                        }
                                                        ui.close_menu();
                                                    }
                                                }
                                            });
                                        });
                                        ui.add_space(8.0);
                                        let desktop_is_monochrome = matches!(
                                            self.desktop_active_color_style,
                                            ColorStyle::Monochrome { .. }
                                        );
                                        ui.horizontal(|ui| {
                                            if Self::retro_choice_button(
                                                ui,
                                                "Monochrome",
                                                desktop_is_monochrome,
                                            )
                                            .clicked()
                                                && !desktop_is_monochrome
                                            {
                                                if self.set_desktop_color_style_selection(
                                                    ColorStyle::Monochrome {
                                                        preset: MonochromePreset::Green,
                                                        custom_rgb: None,
                                                    },
                                                ) {
                                                    desktop_runtime_changed = true;
                                                    appearance_changed = true;
                                                }
                                            }
                                            if Self::retro_choice_button(
                                                ui,
                                                "Full Color",
                                                !desktop_is_monochrome,
                                            )
                                            .clicked()
                                                && desktop_is_monochrome
                                            {
                                                if self.set_desktop_full_color_theme(
                                                    "nucleon-dark",
                                                ) {
                                                    desktop_runtime_changed = true;
                                                    appearance_changed = true;
                                                }
                                            }
                                        });
                                        ui.add_space(6.0);
                                        if desktop_is_monochrome {
                                            ui.horizontal(|ui| {
                                                ui.label("Monochrome Theme");
                                                let mut current_idx = MONOCHROME_THEME_NAMES
                                                    .iter()
                                                    .position(|name| {
                                                        *name
                                                            == monochrome_theme_name_for_color_style(
                                                                &self.desktop_active_color_style,
                                                            )
                                                    })
                                                    .unwrap_or(0);
                                                egui::ComboBox::from_id_salt(
                                                    "native_desktop_theme",
                                                )
                                                .selected_text(
                                                    RichText::new(
                                                        MONOCHROME_THEME_NAMES[current_idx],
                                                    )
                                                    .color(palette.fg),
                                                )
                                                .show_ui(ui, |ui| {
                                                    Self::apply_settings_control_style(ui);
                                                    for (idx, name) in
                                                        MONOCHROME_THEME_NAMES.iter().enumerate()
                                                    {
                                                        if Self::retro_choice_button(
                                                            ui,
                                                            *name,
                                                            current_idx == idx,
                                                        )
                                                        .clicked()
                                                        {
                                                            current_idx = idx;
                                                            if self.set_desktop_color_style_selection(
                                                                color_style_from_theme_name(
                                                                    name,
                                                                    custom_rgb_for_color_style(
                                                                        &self.desktop_active_color_style,
                                                                    ),
                                                                ),
                                                            ) {
                                                                desktop_runtime_changed = true;
                                                                appearance_changed = true;
                                                            }
                                                            ui.close_menu();
                                                        }
                                                    }
                                                });
                                            });
                                            if monochrome_theme_name_for_color_style(
                                                &self.desktop_active_color_style,
                                            ) == CUSTOM_THEME_NAME
                                            {
                                                ui.add_space(6.0);
                                                let mut rgb = custom_rgb_for_color_style(
                                                    &self.desktop_active_color_style,
                                                );
                                                let preview_color = egui::Color32::from_rgb(
                                                    rgb[0], rgb[1], rgb[2],
                                                );
                                                ui.visuals_mut().selection.bg_fill = preview_color;
                                                ui.visuals_mut().widgets.inactive.bg_fill =
                                                    palette.dim;
                                                let mut changed_rgb = false;
                                                changed_rgb |= ui
                                                    .add(
                                                        egui::Slider::new(&mut rgb[0], 0..=255)
                                                            .text("Red"),
                                                    )
                                                    .changed();
                                                changed_rgb |= ui
                                                    .add(
                                                        egui::Slider::new(&mut rgb[1], 0..=255)
                                                            .text("Green"),
                                                    )
                                                    .changed();
                                                changed_rgb |= ui
                                                    .add(
                                                        egui::Slider::new(&mut rgb[2], 0..=255)
                                                            .text("Blue"),
                                                    )
                                                    .changed();
                                                if changed_rgb
                                                    && self.set_desktop_monochrome_custom_rgb(rgb)
                                                {
                                                    desktop_runtime_changed = true;
                                                    appearance_changed = true;
                                                }
                                            }
                                        } else {
                                            ui.label(
                                                RichText::new("Full Color Theme")
                                                    .color(palette.fg)
                                                    .strong(),
                                            );
                                            ui.add_space(4.0);
                                            let selected_theme_id = full_color_theme_id_for_color_style(
                                                &self.desktop_active_color_style,
                                            )
                                            .to_string();
                                            for theme in FullColorTheme::builtin_themes() {
                                                let selected =
                                                    selected_theme_id == theme.id.as_str();
                                                let label = match theme.id.as_str() {
                                                    "nucleon-dark" => {
                                                        "Nucleon Dark - Dark background, teal accents"
                                                    }
                                                    "nucleon-light" => {
                                                        "Nucleon Light - Light background, slate blue accents"
                                                    }
                                                    _ => theme.name.as_str(),
                                                };
                                                if Self::retro_choice_button(ui, label, selected)
                                                    .clicked()
                                                {
                                                    if self.set_desktop_full_color_theme(&theme.id)
                                                    {
                                                        desktop_runtime_changed = true;
                                                        appearance_changed = true;
                                                    }
                                                }
                                            }
                                            if self.draw_full_color_customization_controls(ui, false)
                                            {
                                                desktop_runtime_changed = true;
                                                appearance_changed = true;
                                            }
                                        }
                                        ui.add_space(6.0);
                                        ui.horizontal(|ui| {
                                            ui.label("Font");
                                            egui::ComboBox::from_id_salt(
                                                "native_desktop_font_pack_component",
                                            )
                                            .selected_text(
                                                RichText::new(selected_font_name(
                                                    self.desktop_active_font_id.as_deref(),
                                                    &font_packs,
                                                ))
                                                .color(palette.fg),
                                            )
                                            .show_ui(ui, |ui| {
                                                Self::apply_settings_control_style(ui);
                                                if Self::retro_choice_button(
                                                    ui,
                                                    "Fixedsys (Default)",
                                                    self.desktop_active_font_id
                                                        == super::desktop_default_font_id(
                                                            self.desktop_active_theme_pack_id
                                                                .as_deref(),
                                                            self.desktop_active_desktop_style_id
                                                                .as_deref(),
                                                        ),
                                                )
                                                .clicked()
                                                {
                                                    if self.set_desktop_font_id(None) {
                                                        appearance_changed = true;
                                                    }
                                                    ui.close_menu();
                                                }
                                                for manifest in &font_packs {
                                                    let selected = self
                                                        .desktop_active_font_id
                                                        .as_deref()
                                                        == Some(manifest.id.as_str());
                                                    if Self::retro_choice_button(
                                                        ui,
                                                        &manifest.name,
                                                        selected,
                                                    )
                                                    .clicked()
                                                    {
                                                        if self.set_desktop_font_id(Some(
                                                            manifest.id.clone(),
                                                        )) {
                                                            appearance_changed = true;
                                                        }
                                                        ui.close_menu();
                                                    }
                                                }
                                            });
                                        });
                                        ui.add_space(6.0);
                                        ui.horizontal(|ui| {
                                            ui.label("Icons");
                                            egui::ComboBox::from_id_salt(
                                                "native_desktop_icon_pack_component",
                                            )
                                            .selected_text(
                                                RichText::new(selected_manifest_name(
                                                    self.desktop_active_icon_pack_id.as_deref(),
                                                    "Default",
                                                    &icon_packs,
                                                    |manifest: &IconPackManifest| manifest.id.as_str(),
                                                    |manifest: &IconPackManifest| manifest.name.as_str(),
                                                ))
                                                .color(palette.fg),
                                            )
                                            .show_ui(ui, |ui| {
                                                Self::apply_settings_control_style(ui);
                                                if Self::retro_choice_button(
                                                    ui,
                                                    "Default",
                                                    self.desktop_active_icon_pack_id.is_none(),
                                                )
                                                .clicked()
                                                {
                                                    if self.set_desktop_icon_pack_id(None) {
                                                        desktop_runtime_changed = true;
                                                        appearance_changed = true;
                                                    }
                                                    ui.close_menu();
                                                }
                                                for manifest in &icon_packs {
                                                    let selected = self
                                                        .desktop_active_icon_pack_id
                                                        .as_deref()
                                                        == Some(manifest.id.as_str());
                                                    if Self::retro_choice_button(
                                                        ui,
                                                        &manifest.name,
                                                        selected,
                                                    )
                                                    .clicked()
                                                    {
                                                        if self.set_desktop_icon_pack_id(Some(
                                                            manifest.id.clone(),
                                                        )) {
                                                            desktop_runtime_changed = true;
                                                            appearance_changed = true;
                                                        }
                                                        ui.close_menu();
                                                    }
                                                }
                                            });
                                        });
                                        ui.add_space(6.0);
                                        ui.horizontal(|ui| {
                                            ui.label("Icon Style");
                                            let selected = desktop_icon_style_label(
                                                self.settings.draft.desktop_icon_style,
                                            );
                                            egui::ComboBox::from_id_salt(
                                                "native_settings_desktop_icons",
                                            )
                                            .selected_text(
                                                RichText::new(selected).color(palette.fg),
                                            )
                                            .show_ui(ui, |ui| {
                                                Self::apply_settings_control_style(ui);
                                                for (style, label) in [
                                                    (DesktopIconStyle::Dos, "DOS"),
                                                    (DesktopIconStyle::Win95, "Win95"),
                                                    (DesktopIconStyle::Minimal, "Minimal"),
                                                    (DesktopIconStyle::NoIcons, "No Icons"),
                                                ] {
                                                    if Self::retro_choice_button(
                                                        ui,
                                                        label,
                                                        self.settings.draft.desktop_icon_style
                                                            == style,
                                                    )
                                                    .clicked()
                                                    {
                                                        set_desktop_icon_style(
                                                            &mut self.settings.draft,
                                                            style,
                                                        );
                                                        persist_changed = true;
                                                        ui.close_menu();
                                                    }
                                                }
                                            });
                                        });
                                        ui.add_space(8.0);
                                        ui.label(
                                            RichText::new("Built-in Desktop Icons")
                                                .color(palette.fg)
                                                .strong(),
                                        );
                                        ui.add_space(4.0);
                                        for entry in desktop_builtin_icons() {
                                            let mut visible = !self
                                                .settings
                                                .draft
                                                .desktop_hidden_builtin_icons
                                                .contains(entry.key);
                                            if Self::retro_checkbox_row(
                                                ui,
                                                &mut visible,
                                                &format!("Show {}", entry.label),
                                            )
                                            .clicked()
                                            {
                                                set_builtin_icon_visible(
                                                    &mut self.settings.draft,
                                                    entry.key,
                                                    visible,
                                                );
                                                persist_changed = true;
                                            }
                                        }
                                        ui.add_space(6.0);
                                        ui.horizontal(|ui| {
                                            ui.label("Sounds");
                                            egui::ComboBox::from_id_salt(
                                                "native_desktop_sound_pack_component",
                                            )
                                            .selected_text(
                                                RichText::new(selected_manifest_name(
                                                    self.desktop_active_sound_pack_id.as_deref(),
                                                    "Default",
                                                    &sound_packs,
                                                    |manifest: &SoundPackManifest| manifest.id.as_str(),
                                                    |manifest: &SoundPackManifest| manifest.name.as_str(),
                                                ))
                                                .color(palette.fg),
                                            )
                                            .show_ui(ui, |ui| {
                                                Self::apply_settings_control_style(ui);
                                                if Self::retro_choice_button(
                                                    ui,
                                                    "Default",
                                                    self.desktop_active_sound_pack_id.is_none(),
                                                )
                                                .clicked()
                                                {
                                                    if self.set_desktop_sound_pack_id(None) {
                                                        desktop_runtime_changed = true;
                                                        appearance_changed = true;
                                                    }
                                                    ui.close_menu();
                                                }
                                                for manifest in &sound_packs {
                                                    let selected = self
                                                        .desktop_active_sound_pack_id
                                                        .as_deref()
                                                        == Some(manifest.id.as_str());
                                                    if Self::retro_choice_button(
                                                        ui,
                                                        &manifest.name,
                                                        selected,
                                                    )
                                                    .clicked()
                                                    {
                                                        if self.set_desktop_sound_pack_id(Some(
                                                            manifest.id.clone(),
                                                        )) {
                                                            desktop_runtime_changed = true;
                                                            appearance_changed = true;
                                                        }
                                                        ui.close_menu();
                                                    }
                                                }
                                            });
                                        });
                                        ui.add_space(6.0);
                                        ui.horizontal(|ui| {
                                            ui.label("Cursors");
                                            egui::ComboBox::from_id_salt(
                                                "native_desktop_cursor_pack_component",
                                            )
                                            .selected_text(
                                                RichText::new(selected_manifest_name(
                                                    self.desktop_active_cursor_pack_id.as_deref(),
                                                    "Default",
                                                    &cursor_packs,
                                                    |manifest: &CursorPackManifest| manifest.id.as_str(),
                                                    |manifest: &CursorPackManifest| manifest.name.as_str(),
                                                ))
                                                .color(palette.fg),
                                            )
                                            .show_ui(ui, |ui| {
                                                Self::apply_settings_control_style(ui);
                                                if Self::retro_choice_button(
                                                    ui,
                                                    "Default",
                                                    self.desktop_active_cursor_pack_id.is_none(),
                                                )
                                                .clicked()
                                                {
                                                    if self.set_desktop_cursor_pack_id(None) {
                                                        appearance_changed = true;
                                                    }
                                                    ui.close_menu();
                                                }
                                                for manifest in &cursor_packs {
                                                    let selected = self
                                                        .desktop_active_cursor_pack_id
                                                        .as_deref()
                                                        == Some(manifest.id.as_str());
                                                    if Self::retro_choice_button(
                                                        ui,
                                                        &manifest.name,
                                                        selected,
                                                    )
                                                    .clicked()
                                                    {
                                                        if self.set_desktop_cursor_pack_id(Some(
                                                            manifest.id.clone(),
                                                        )) {
                                                            appearance_changed = true;
                                                        }
                                                        ui.close_menu();
                                                    }
                                                }
                                            });
                                        });
                                        ui.add_space(6.0);
                                        if Self::retro_checkbox_row(
                                            ui,
                                            &mut self.settings.draft.desktop_show_cursor,
                                            "Show desktop cursor",
                                        )
                                        .clicked()
                                        {
                                            persist_changed = true;
                                        }
                                        if self.settings.draft.desktop_show_cursor {
                                            ui.add_space(6.0);
                                            ui.scope(|ui| {
                                                ui.visuals_mut().selection.bg_fill = palette.fg;
                                                ui.visuals_mut().widgets.inactive.bg_fill =
                                                    palette.dim;
                                                persist_changed |= ui
                                                    .add(
                                                        egui::Slider::new(
                                                            &mut self.settings.draft.desktop_cursor_scale,
                                                            0.5..=2.5,
                                                        )
                                                        .text("Cursor Scale"),
                                                    )
                                                    .changed();
                                            });
                                        }
                                    });
                                    ui.add_space(10.0);
                                    Self::settings_section(ui, "Layout Overrides", |ui| {
                                        if self.draw_layout_override_controls(ui) {
                                            desktop_runtime_changed = true;
                                            appearance_changed = true;
                                        }
                                    });
                                }
                                _ => {
                                    Self::settings_section(ui, "Color Mode", |ui| {
                                        let terminal_is_monochrome = matches!(
                                            self.terminal_active_color_style,
                                            ColorStyle::Monochrome { .. }
                                        );
                                        ui.horizontal(|ui| {
                                            if Self::retro_choice_button(
                                                ui,
                                                "Monochrome",
                                                terminal_is_monochrome,
                                            )
                                            .clicked()
                                                && !terminal_is_monochrome
                                            {
                                                if self.set_terminal_color_style_selection(
                                                    ColorStyle::Monochrome {
                                                        preset: MonochromePreset::Green,
                                                        custom_rgb: None,
                                                    },
                                                ) {
                                                    appearance_changed = true;
                                                }
                                            }
                                            if Self::retro_choice_button(
                                                ui,
                                                "Full Color",
                                                !terminal_is_monochrome,
                                            )
                                            .clicked()
                                                && terminal_is_monochrome
                                            {
                                                if self.set_terminal_full_color_theme(
                                                    "nucleon-dark",
                                                ) {
                                                    appearance_changed = true;
                                                }
                                            }
                                        });
                                    });
                                    ui.add_space(10.0);
                                    if matches!(
                                        self.terminal_active_color_style,
                                        ColorStyle::Monochrome { .. }
                                    ) {
                                        Self::settings_section(
                                            ui,
                                            "Terminal Monochrome Theme",
                                            |ui| {
                                                ui.horizontal(|ui| {
                                                    ui.label("Theme");
                                                    let mut current_idx = MONOCHROME_THEME_NAMES
                                                        .iter()
                                                        .position(|name| {
                                                            *name == monochrome_theme_name_for_color_style(
                                                                &self.terminal_active_color_style,
                                                            )
                                                        })
                                                        .unwrap_or(0);
                                                    egui::ComboBox::from_id_salt(
                                                        "native_terminal_theme",
                                                    )
                                                    .selected_text(
                                                        RichText::new(
                                                            MONOCHROME_THEME_NAMES[current_idx],
                                                        )
                                                        .color(palette.fg),
                                                    )
                                                    .show_ui(ui, |ui| {
                                                        Self::apply_settings_control_style(ui);
                                                        for (idx, name) in
                                                            MONOCHROME_THEME_NAMES
                                                                .iter()
                                                                .enumerate()
                                                        {
                                                            if Self::retro_choice_button(
                                                                ui,
                                                                *name,
                                                                current_idx == idx,
                                                            )
                                                            .clicked()
                                                            {
                                                                current_idx = idx;
                                                                if self.set_terminal_color_style_selection(
                                                                    color_style_from_theme_name(
                                                                        name,
                                                                        custom_rgb_for_color_style(
                                                                            &self.terminal_active_color_style,
                                                                        ),
                                                                    ),
                                                                ) {
                                                                    appearance_changed = true;
                                                                }
                                                                ui.close_menu();
                                                            }
                                                        }
                                                    });
                                                });
                                                if monochrome_theme_name_for_color_style(
                                                    &self.terminal_active_color_style,
                                                ) == CUSTOM_THEME_NAME
                                                {
                                                    let mut rgb = custom_rgb_for_color_style(
                                                        &self.terminal_active_color_style,
                                                    );
                                                    let preview_color = egui::Color32::from_rgb(
                                                        rgb[0], rgb[1], rgb[2],
                                                    );
                                                    ui.visuals_mut().selection.bg_fill =
                                                        preview_color;
                                                    ui.visuals_mut().widgets.inactive.bg_fill =
                                                        palette.dim;
                                                    let mut changed_rgb = false;
                                                    changed_rgb |= ui
                                                        .add(
                                                            egui::Slider::new(
                                                                &mut rgb[0],
                                                                0..=255,
                                                            )
                                                            .text("Red"),
                                                        )
                                                        .changed();
                                                    changed_rgb |= ui
                                                        .add(
                                                            egui::Slider::new(
                                                                &mut rgb[1],
                                                                0..=255,
                                                            )
                                                            .text("Green"),
                                                        )
                                                        .changed();
                                                    changed_rgb |= ui
                                                        .add(
                                                            egui::Slider::new(
                                                                &mut rgb[2],
                                                                0..=255,
                                                            )
                                                            .text("Blue"),
                                                        )
                                                        .changed();
                                                    if changed_rgb
                                                        && self
                                                            .set_terminal_monochrome_custom_rgb(rgb)
                                                    {
                                                        appearance_changed = true;
                                                    }
                                                }
                                            },
                                        );
                                    } else {
                                        Self::settings_section(
                                            ui,
                                            "Terminal Full Color Theme",
                                            |ui| {
                                                let selected_theme_id =
                                                    full_color_theme_id_for_color_style(
                                                        &self.terminal_active_color_style,
                                                    )
                                                    .to_string();
                                                for theme in FullColorTheme::builtin_themes() {
                                                    let selected =
                                                        selected_theme_id == theme.id.as_str();
                                                    let label = match theme.id.as_str() {
                                                        "nucleon-dark" => {
                                                            "Nucleon Dark - Dark background, teal accents"
                                                        }
                                                        "nucleon-light" => {
                                                            "Nucleon Light - Light background, slate blue accents"
                                                        }
                                                        _ => theme.name.as_str(),
                                                    };
                                                    if Self::retro_choice_button(
                                                        ui, label, selected,
                                                    )
                                                    .clicked()
                                                    {
                                                        if self.set_terminal_full_color_theme(
                                                            &theme.id,
                                                        ) {
                                                            appearance_changed = true;
                                                        }
                                                    }
                                                }
                                                if self.draw_full_color_customization_controls(ui, true)
                                                {
                                                    appearance_changed = true;
                                                }
                                            },
                                        );
                                    }
                                    ui.add_space(10.0);
                                    Self::settings_section(ui, "Terminal Layout", |ui| {
                                        for profile in TerminalLayoutProfile::builtin_layouts() {
                                            let selected =
                                                self.terminal_active_layout.id == profile.id;
                                            let description = match profile.id.as_str() {
                                                "classic-terminal" => {
                                                    "Classic Terminal - bottom status bar"
                                                }
                                                "minimal-terminal" => {
                                                    "Minimal Terminal - no status bar"
                                                }
                                                _ => profile.name.as_str(),
                                            };
                                            if Self::retro_choice_button(
                                                ui,
                                                description,
                                                selected,
                                            )
                                            .clicked()
                                            {
                                                self.terminal_active_layout = profile;
                                                appearance_changed = true;
                                            }
                                        }
                                    });
                                }
                            }
                        }
                        2 => {
                            Self::settings_section(ui, "Sound", |ui| {
                                ui.horizontal(|ui| {
                                    if Self::retro_checkbox_row(
                                        ui,
                                        &mut self.settings.draft.sound,
                                        "Enable sound",
                                    )
                                    .clicked()
                                    {
                                        persist_changed = true;
                                    }
                                    if ui.button("Preview ▶").clicked() {
                                        crate::sound::preview_navigate();
                                    }
                                });
                            });
                            ui.add_space(10.0);
                            persist_changed |= self.draw_settings_display_effects_panel(ui);
                        }
                        _ => {
                            Self::settings_section(ui, "Window Mode", |ui| {
                                ui.label("Window Mode");
                                ui.horizontal_wrapped(|ui| {
                                    for mode in [
                                        NativeStartupWindowMode::Windowed,
                                        NativeStartupWindowMode::Maximized,
                                        NativeStartupWindowMode::BorderlessFullscreen,
                                        NativeStartupWindowMode::Fullscreen,
                                    ] {
                                        if Self::retro_choice_button(
                                            ui,
                                            mode.label(),
                                            self.settings.draft.native_startup_window_mode == mode,
                                        )
                                        .clicked()
                                            && self.settings.draft.native_startup_window_mode != mode
                                        {
                                            self.settings.draft.native_startup_window_mode = mode;
                                            persist_changed = true;
                                            window_mode_changed = true;
                                        }
                                    }
                                });
                            });
                            ui.add_space(10.0);
                            Self::settings_section(ui, "PTY Rendering", |ui| {
                                if Self::retro_checkbox_row(
                                    ui,
                                    &mut self.settings.draft.cli_styled_render,
                                    "Styled PTY rendering",
                                )
                                .clicked()
                                {
                                    persist_changed = true;
                                }
                                ui.add_space(8.0);
                                ui.horizontal(|ui| {
                                    ui.label("PTY Color Mode");
                                    let selected =
                                        cli_color_mode_label(self.settings.draft.cli_color_mode);
                                    egui::ComboBox::from_id_salt("native_settings_cli_color")
                                        .selected_text(RichText::new(selected).color(palette.fg))
                                        .show_ui(ui, |ui| {
                                            Self::apply_settings_control_style(ui);
                                            for (mode, label) in [
                                                (CliColorMode::ThemeLock, "Theme Lock"),
                                                (CliColorMode::PaletteMap, "Palette-map"),
                                                (CliColorMode::Color, "Color"),
                                                (CliColorMode::Monochrome, "Monochrome"),
                                            ] {
                                                if Self::retro_choice_button(
                                                    ui,
                                                    label,
                                                    self.settings.draft.cli_color_mode == mode,
                                                )
                                                .clicked()
                                                    && self.settings.draft.cli_color_mode != mode
                                                {
                                                    self.settings.draft.cli_color_mode = mode;
                                                    persist_changed = true;
                                                    ui.close_menu();
                                                }
                                            }
                                        });
                                });
                                ui.add_space(8.0);
                                ui.horizontal(|ui| {
                                    ui.label("Border Glyphs");
                                    let selected =
                                        cli_acs_mode_label(self.settings.draft.cli_acs_mode);
                                    egui::ComboBox::from_id_salt("native_settings_cli_glyphs")
                                        .selected_text(RichText::new(selected).color(palette.fg))
                                        .show_ui(ui, |ui| {
                                            Self::apply_settings_control_style(ui);
                                            for (mode, label) in [
                                                (CliAcsMode::Ascii, "ASCII"),
                                                (CliAcsMode::Unicode, "Unicode Smooth"),
                                            ] {
                                                if Self::retro_choice_button(
                                                    ui,
                                                    label,
                                                    self.settings.draft.cli_acs_mode == mode,
                                                )
                                                .clicked()
                                                    && self.settings.draft.cli_acs_mode != mode
                                                {
                                                    self.settings.draft.cli_acs_mode = mode;
                                                    persist_changed = true;
                                                    ui.close_menu();
                                                }
                                            }
                                        });
                                });
                            });
                        }
                    }
                });

            if appearance_changed {
                self.sync_active_font(ctx);
                self.persist_surface_theme_state_to_settings();
                persist_changed = true;
            }
            Self::retro_separator_with_thickness(
                ui,
                self.desktop_active_desktop_style.separator_thickness,
            );
            if persist_changed {
                {
                    let draft = self.settings.draft.clone();
                    let tx = self.background.sender();
                    std::thread::spawn(move || {
                        persist_settings_draft(&draft);
                        let _ = tx.send(BackgroundResult::SettingsPersisted);
                    });
                }
                self.sync_runtime_settings_cache();
                self.invalidate_desktop_icon_layout_cache();
                self.invalidate_program_catalog_cache();
                self.invalidate_saved_connections_cache();
                self.refresh_settings_sync_marker();
                if window_mode_changed {
                    self.apply_native_window_mode(ctx);
                }
                self.apply_status_update(saved_settings_status());
            }
            if desktop_runtime_changed {
                self.invalidate_desktop_icon_layout_cache();
            }
            if !self.settings.status.is_empty() {
                ui.small(&self.settings.status);
            }
        });
        let shown_rect = shown.as_ref().map(|inner| inner.response.rect);
        let shown_contains_pointer = shown
            .as_ref()
            .is_some_and(|inner| inner.response.contains_pointer());
        self.finish_desktop_window_host(
            ctx,
            DesktopWindow::Tweaks,
            &mut open,
            maximized,
            shown_rect,
            shown_contains_pointer,
            DesktopWindowRectTracking::PositionOnly,
            header_action,
        );
    }
}
