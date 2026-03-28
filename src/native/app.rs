use super::background::BackgroundTasks;
use super::command_layer::CommandLayerState;
use super::connections_screen::TerminalConnectionsState;
use super::data::home_dir_fallback;
#[cfg(test)]
use super::desktop_app::DesktopMenuAction;
#[cfg(test)]
use super::desktop_app::DesktopShellAction;
use super::desktop_app::{DesktopWindow, WindowInstanceId};
use super::desktop_connections_service::DiscoveredConnection;
#[cfg(test)]
use super::desktop_search_service::NativeSpotlightCategory;
use super::desktop_search_service::{NativeSpotlightResult, NativeStartLeafAction};
use super::desktop_session_service::restore_current_user_from_last_session;
use super::desktop_settings_service::load_settings_snapshot;
use super::desktop_surface_service::{DesktopIconGridLayout, DesktopSurfaceEntry};
use super::edit_menus_screen::{EditMenuTarget, TerminalEditMenusState};
#[cfg(test)]
use super::editor_app::EditorCommand;
use super::editor_app::{EditorWindow, EDITOR_APP_TITLE};
#[cfg(test)]
use super::file_manager::FileEntryRow;
use super::file_manager::{FileManagerCommand, NativeFileManagerState};
#[cfg(test)]
use super::file_manager_app::{
    FileManagerEditRuntime, FileManagerPickerCommit, NativeFileManagerDragPayload,
};
#[cfg(not(test))]
use super::file_manager_app::{FileManagerEditRuntime, NativeFileManagerDragPayload};
use super::file_manager_desktop::{self, FileManagerDesktopFooterAction};
use super::installer_screen::{DesktopInstallerState, TerminalInstallerState};
use super::menu::{
    terminal_runtime_defaults, TerminalLoginState, TerminalNavigationState, TerminalScreen,
    TerminalShellSurface,
};
#[cfg(test)]
use super::prompt::{
    FlashAction, TerminalFlash, TerminalPrompt, TerminalPromptAction, TerminalPromptKind,
};
#[cfg(not(test))]
use super::prompt::{TerminalFlash, TerminalPrompt};
use super::pty_screen::NativePtyState;
use super::retro_ui::{
    configure_visuals, configure_visuals_for_palette, current_palette, current_palette_for_surface,
    palette_for_color_style, palette_for_color_style_with_overrides, set_active_color_style,
    set_active_shell_style, set_active_terminal_decoration, set_active_terminal_wallpaper,
    RetroPalette, ShellSurfaceKind, FIXED_PTY_CELL_H, FIXED_PTY_CELL_W,
};
use super::terminal_open_with_picker;
use super::wasm_addon_runtime::WasmHostedAddonState;
use crate::config::SavedConnection;
#[cfg(test)]
use crate::config::{
    DesktopCursorThemeSelection, DesktopFileManagerSettings, DesktopIconSortMode,
    HackingDifficulty, OpenMode, Settings,
};
#[cfg(not(test))]
use crate::config::{
    DesktopCursorThemeSelection, DesktopFileManagerSettings, DesktopIconSortMode,
    HackingDifficulty, Settings,
};
use crate::core::auth::{AuthMethod, UserRecord};
use crate::session;
use crate::theme::{
    ColorStyle, ColorToken, CursorPack, LayoutProfile, MonochromePreset, PanelType, ShellStyle,
    TerminalBranding, TerminalDecoration, TerminalLayoutProfile,
};
use eframe::egui::{
    self, Align2, Color32, Context, FontData, FontDefinitions, FontFamily, FontId, Id, Key, Layout,
    RichText, TextEdit, TextStyle, TextureHandle,
};
use egui_wgpu::CrtEffects;
#[cfg(not(test))]
use nucleon_native_programs_app::{
    build_desktop_applications_sections, DesktopApplicationsSections,
};
#[cfg(test)]
use nucleon_native_programs_app::{
    build_desktop_applications_sections, DesktopApplicationsSections, DesktopProgramRequest,
};
use nucleon_native_settings_app::{
    build_desktop_settings_ui_defaults, desktop_settings_default_panel,
    desktop_settings_home_rows_with_visibility, desktop_settings_panel_enabled,
    DesktopSettingsVisibility, GuiCliProfileSlot, NativeSettingsPanel, SettingsHomeTile,
    TerminalSettingsPanel, TerminalSettingsVisibility,
};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::SystemTime;
use std::time::{Duration, Instant};

mod addon_policy;
mod asset_helpers;
mod command_layer_runtime;
mod desktop_component_host;
mod desktop_file_runtime;
mod desktop_installer_ui;
mod desktop_menu_bar;
mod desktop_runtime;
mod desktop_spotlight;
mod desktop_start_menu;
mod desktop_surface;
mod desktop_taskbar;
mod desktop_window_mgmt;
mod document_browser_runtime;
mod document_runtime;
mod edit_menu_runtime;
mod editor_runtime;
mod launch_registry;
mod launch_runtime;
mod presenter_applications;
mod presenter_editor;
mod presenter_file_manager;
mod presenter_pty;
mod presenter_settings;
mod presenter_terminal_mode;
mod prompt_runtime;
mod runtime_state;
mod session_management;
mod session_runtime;
mod software_cursor;
mod terminal_dispatch;
mod terminal_runtime;
mod terminal_screens;
mod tweaks_presenter;
mod ui_helpers;
use desktop_window_mgmt::DesktopHeaderAction;
#[cfg(test)]
use launch_registry::{resolve_terminal_launch_target, NativeTerminalLaunch};
mod file_manager_desktop_presenter;
mod frame_runtime;
mod settings_panels;
#[cfg(test)]
use super::editor_app::EditorTextAlign;
#[cfg(test)]
use crate::config::CUSTOM_THEME_NAME;
#[cfg(test)]
use desktop_start_menu::START_ROOT_ITEMS;
use desktop_start_menu::{StartLeaf, StartSubmenu};
#[cfg(test)]
use desktop_window_mgmt::DesktopWindowRectTracking;
pub(crate) use desktop_window_mgmt::DesktopWindowState;

#[derive(Debug, Clone)]
pub(super) struct SessionState {
    username: String,
    is_admin: bool,
}

#[derive(Debug, Clone)]
struct SettingsWindow {
    open: bool,
    draft: Settings,
    status: String,
    panel: NativeSettingsPanel,
    default_app_custom_text_code: String,
    default_app_custom_ebook: String,
    scanned_networks: Vec<DiscoveredConnection>,
    scanned_bluetooth: Vec<DiscoveredConnection>,
    connection_password: String,
    edit_target: EditMenuTarget,
    edit_name_input: String,
    edit_value_input: String,
    cli_profile_slot: GuiCliProfileSlot,
    user_selected: String,
    user_selected_loaded_for: String,
    user_create_username: String,
    user_create_auth: AuthMethod,
    user_create_password: String,
    user_create_password_confirm: String,
    user_edit_auth: AuthMethod,
    user_edit_password: String,
    user_edit_password_confirm: String,
    user_delete_confirm: String,
}

#[derive(Debug, Default, Clone)]
struct ApplicationsWindow {
    open: bool,
    status: String,
}

#[derive(Debug, Default, Clone)]
struct TerminalModeWindow {
    open: bool,
    status: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DesktopAppearanceKey {
    color_style: ColorStyle,
    overrides: Option<HashMap<ColorToken, [u8; 4]>>,
}

#[derive(Clone)]
struct CachedIcon {
    texture: TextureHandle,
    is_full_color: bool,
}

struct AssetCache {
    icon_settings: CachedIcon,
    icon_file_manager: CachedIcon,
    icon_terminal: CachedIcon,
    icon_applications: CachedIcon,
    icon_installer: CachedIcon,
    icon_editor: CachedIcon,
    icon_general: Option<CachedIcon>,
    icon_appearance: Option<CachedIcon>,
    icon_default_apps: Option<CachedIcon>,
    icon_connections: CachedIcon,
    icon_cli_profiles: Option<CachedIcon>,
    icon_edit_menus: Option<CachedIcon>,
    icon_user_management: Option<CachedIcon>,
    icon_about: Option<CachedIcon>,
    icon_folder: Option<CachedIcon>,
    icon_folder_open: Option<CachedIcon>,
    icon_file: Option<CachedIcon>,
    icon_text: Option<CachedIcon>,
    icon_image: Option<CachedIcon>,
    icon_audio: Option<CachedIcon>,
    icon_video: Option<CachedIcon>,
    icon_archive: Option<CachedIcon>,
    icon_app: Option<CachedIcon>,
    icon_shortcut_badge: Option<CachedIcon>,
    icon_gaming: Option<CachedIcon>,
    wallpaper: Option<TextureHandle>,
    wallpaper_loaded_for: String,
}

struct DesktopIconLayoutCache {
    layout: DesktopIconGridLayout,
    desktop_entry_keys: Arc<Vec<String>>,
    positions: Arc<HashMap<String, [f32; 2]>>,
}

struct DesktopSurfaceEntriesCache {
    dir: PathBuf,
    modified: Option<SystemTime>,
    entries: Arc<Vec<DesktopSurfaceEntry>>,
}

struct DesktopApplicationsSectionsCache {
    show_file_manager: bool,
    show_text_editor: bool,
    sections: Arc<DesktopApplicationsSections>,
}

// Note: old built-in optional-addon visibility toggles were removed — addons now drive visibility dynamically

struct SettingsHomeRowsCache {
    visibility: DesktopSettingsVisibility,
    rows: Arc<Vec<Vec<SettingsHomeTile>>>,
}

struct EditMenuEntriesCache {
    applications: Option<Arc<Vec<String>>>,
    documents: Option<Arc<Vec<String>>>,
    network: Option<Arc<Vec<String>>>,
    games: Option<Arc<Vec<String>>>,
}

#[derive(Debug, Clone)]
struct ShortcutPropertiesState {
    shortcut_idx: usize,
    name_draft: String,
    command_draft: String,
    icon_path_draft: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum DesktopIconSelection {
    Builtin(&'static str),
    Surface(String),
    Shortcut(usize),
}

#[derive(Debug, Clone)]
struct DesktopItemPropertiesState {
    path: PathBuf,
    name_draft: String,
    is_dir: bool,
}

#[derive(Debug, Clone)]
struct StartMenuRenameState {
    target: EditMenuTarget,
    original_name: String,
    name_input: String,
}

#[derive(Debug, Clone)]
pub(super) enum ContextMenuAction {
    Open,
    OpenWith,
    OpenWithCommand(String),
    Rename,
    Cut,
    Copy,
    Paste,
    Duplicate,
    Delete,
    Properties,
    PasteToDesktop,
    NewFolder,
    ChangeAppearance,
    OpenSettings,
    GenericCopy,
    GenericPaste,
    GenericSelectAll,
    CreateShortcut {
        label: String,
        action: NativeStartLeafAction,
    },
    RenameStartMenuEntry {
        target: EditMenuTarget,
        name: String,
    },
    RemoveStartMenuEntry {
        target: EditMenuTarget,
        name: String,
    },
    DeleteShortcut(usize),
    SortDesktopIcons(DesktopIconSortMode),
    ToggleSnapToGrid,
    LaunchShortcut(String),
    OpenShortcutProperties(usize),
    OpenDesktopItem(PathBuf),
    OpenDesktopItemWith(PathBuf),
    RenameDesktopItem(PathBuf),
    DeleteDesktopItem(PathBuf),
    OpenDesktopItemProperties(PathBuf),
}

pub(super) const BUILTIN_TEXT_EDITOR_APP: &str = EDITOR_APP_TITLE;

const TERMINAL_SCREEN_COLS: usize = 92;
const TERMINAL_SCREEN_ROWS: usize = 28;
const TERMINAL_CONTENT_COL: usize = 3;
const TERMINAL_STATUS_ROW: usize = 24;
const TERMINAL_STATUS_ROW_ALT: usize = 26;
pub(super) const SESSION_LEADER_WINDOW: Duration = Duration::from_millis(1200);

#[derive(Clone, Copy)]
struct TerminalLayout {
    cols: usize,
    rows: usize,
    content_col: usize,
    header_start_row: usize,
    separator_top_row: usize,
    title_row: usize,
    separator_bottom_row: usize,
    subtitle_row: usize,
    menu_start_row: usize,
    status_row: usize,
    status_row_alt: usize,
}

fn terminal_branding_from_theme_pack_id(theme_pack_id: Option<&str>) -> Option<TerminalBranding> {
    let theme_pack_id = theme_pack_id?;
    super::installed_theme_packs()
        .into_iter()
        .find(|theme| theme.id == theme_pack_id)
        .map(|theme| theme.terminal_branding)
}

fn terminal_branding_from_settings(settings: &Settings) -> TerminalBranding {
    if let Some(branding) = settings.terminal_branding.clone() {
        return branding;
    }
    let theme_pack_id = terminal_theme_pack_id_from_settings(settings);
    terminal_branding_from_theme_pack_id(theme_pack_id.as_deref())
        .unwrap_or_else(TerminalBranding::none)
}

fn terminal_branding_setting_value(branding: &TerminalBranding) -> Option<TerminalBranding> {
    (!branding.header_lines.is_empty()).then(|| branding.clone())
}

fn theme_bundle_dir_from_pack_id(theme_pack_id: &str) -> Option<PathBuf> {
    super::addons::installed_theme_bundle_dir(theme_pack_id)
}

fn resolve_theme_bundle_path(theme_pack_id: &str, relative_path: &str) -> Option<PathBuf> {
    let path = PathBuf::from(relative_path);
    if path.is_absolute() {
        return Some(path);
    }
    theme_bundle_dir_from_pack_id(theme_pack_id).map(|bundle_dir| bundle_dir.join(path))
}

fn load_cursor_pack_from_asset_root(asset_root: &Path) -> Option<CursorPack> {
    let cursor_path = asset_root.join("cursors").join("cursors.json");
    let raw = std::fs::read_to_string(cursor_path).ok()?;
    serde_json::from_str(&raw).ok()
}

fn desktop_asset_pack_path_from_theme_pack_id(theme_pack_id: Option<&str>) -> Option<PathBuf> {
    let Some(theme_pack_id) = theme_pack_id else {
        return None;
    };
    let Some(theme) = super::installed_theme_packs()
        .into_iter()
        .find(|theme| theme.id == theme_pack_id)
    else {
        return None;
    };
    let Some(asset_pack) = theme.asset_pack.as_ref() else {
        return None;
    };
    resolve_theme_bundle_path(theme_pack_id, &asset_pack.path)
}

fn desktop_sound_pack_path_from_theme_pack_id(theme_pack_id: Option<&str>) -> Option<PathBuf> {
    let Some(theme_pack_id) = theme_pack_id else {
        return None;
    };
    let Some(theme) = super::installed_theme_packs()
        .into_iter()
        .find(|theme| theme.id == theme_pack_id)
    else {
        return None;
    };
    let sound_pack_path = theme.sound_pack.path.as_deref()?;
    resolve_theme_bundle_path(theme_pack_id, sound_pack_path)
}

fn desktop_cursor_pack_from_theme_pack_id(theme_pack_id: Option<&str>) -> Option<CursorPack> {
    let Some(theme_pack_id) = theme_pack_id else {
        return None;
    };
    let Some(theme) = super::installed_theme_packs()
        .into_iter()
        .find(|theme| theme.id == theme_pack_id)
    else {
        return None;
    };
    let asset_pack_path = theme
        .asset_pack
        .as_ref()
        .and_then(|asset_pack| resolve_theme_bundle_path(theme_pack_id, &asset_pack.path));
    theme
        .cursor_pack
        .clone()
        .or_else(|| asset_pack_path.as_deref().and_then(load_cursor_pack_from_asset_root))
}

fn desktop_shell_style_from_theme_pack_id(theme_pack_id: Option<&str>) -> ShellStyle {
    super::installed_theme_packs()
        .into_iter()
        .find(|theme| Some(theme.id.as_str()) == theme_pack_id)
        .map(|theme| theme.shell_style)
        .unwrap_or_else(|| crate::theme::ThemePack::classic().shell_style)
}

fn theme_pack_color_overrides_from_theme_pack_id(
    theme_pack_id: Option<&str>,
) -> Option<HashMap<ColorToken, [u8; 4]>> {
    let theme_pack_id = theme_pack_id?;
    let bundle_dir = theme_bundle_dir_from_pack_id(theme_pack_id)?;
    let active_theme = super::installed_theme_packs()
        .into_iter()
        .find(|theme| theme.id == theme_pack_id)?;

    let mut candidates = vec![bundle_dir.join("colors").join("custom.json")];
    if let crate::theme::ColorStyle::FullColor { theme_id } = active_theme.color_style {
        candidates.push(bundle_dir.join("colors").join(format!("{theme_id}.json")));
        if theme_id == "nucleon-dark" {
            candidates.push(bundle_dir.join("colors").join("dark.json"));
        } else if theme_id == "nucleon-light" {
            candidates.push(bundle_dir.join("colors").join("light.json"));
        }
    }

    for colors_path in candidates {
        let Ok(raw) = std::fs::read_to_string(colors_path) else {
            continue;
        };
        let Ok(theme) = serde_json::from_str::<crate::theme::FullColorTheme>(&raw) else {
            continue;
        };
        return Some(theme.tokens);
    }

    None
}

#[cfg(test)]
fn desktop_asset_state_from_theme_pack_id(
    theme_pack_id: Option<&str>,
) -> (Option<PathBuf>, Option<CursorPack>) {
    (
        desktop_asset_pack_path_from_theme_pack_id(theme_pack_id),
        desktop_cursor_pack_from_theme_pack_id(theme_pack_id),
    )
}

fn terminal_decoration_from_theme_pack_id(
    theme_pack_id: Option<&str>,
) -> Option<TerminalDecoration> {
    let theme_pack_id = theme_pack_id?;
    super::installed_theme_packs()
        .into_iter()
        .find(|theme| theme.id == theme_pack_id)
        .map(|theme| theme.terminal_decoration)
}

fn terminal_decoration_from_settings(settings: &Settings) -> TerminalDecoration {
    let theme_pack_id = terminal_theme_pack_id_from_settings(settings);
    terminal_decoration_from_theme_pack_id(theme_pack_id.as_deref()).unwrap_or_default()
}

fn terminal_layout_with_branding(branding: &TerminalBranding) -> TerminalLayout {
    let header_lines = branding.header_lines.len();
    let separator_top_row = if header_lines == 0 { 0 } else { header_lines };
    let title_row = separator_top_row + 1;
    let separator_bottom_row = title_row + 1;
    let subtitle_row = separator_bottom_row + 2;
    let menu_start_row = subtitle_row + 2;
    TerminalLayout {
        cols: TERMINAL_SCREEN_COLS,
        rows: TERMINAL_SCREEN_ROWS,
        content_col: TERMINAL_CONTENT_COL,
        header_start_row: 0,
        separator_top_row,
        title_row,
        separator_bottom_row,
        subtitle_row,
        menu_start_row,
        status_row: TERMINAL_STATUS_ROW,
        status_row_alt: TERMINAL_STATUS_ROW_ALT,
    }
}

fn retro_footer_height() -> f32 {
    31.0
}

fn color_style_from_settings(settings: &Settings) -> ColorStyle {
    match settings.theme.as_str() {
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
        crate::config::CUSTOM_THEME_NAME => ColorStyle::Monochrome {
            preset: MonochromePreset::Custom,
            custom_rgb: Some(settings.custom_theme_rgb),
        },
        _ => ColorStyle::Monochrome {
            preset: MonochromePreset::Green,
            custom_rgb: None,
        },
    }
}

fn desktop_color_style_from_settings(settings: &Settings) -> ColorStyle {
    settings
        .desktop_color_style
        .clone()
        .unwrap_or_else(|| color_style_from_settings(settings))
}

fn terminal_color_style_from_settings(settings: &Settings) -> ColorStyle {
    settings
        .terminal_color_style
        .clone()
        .unwrap_or_else(|| color_style_from_settings(settings))
}

fn desktop_layout_from_settings(settings: &Settings) -> LayoutProfile {
    settings
        .desktop_layout_profile
        .clone()
        .unwrap_or_else(|| crate::theme::ThemePack::classic().layout_profile)
}

fn terminal_layout_from_settings(settings: &Settings) -> TerminalLayoutProfile {
    settings
        .terminal_layout_profile
        .clone()
        .unwrap_or_else(crate::theme::TerminalLayoutProfile::classic)
}

fn canonical_theme_pack_id(theme_pack_id: Option<String>) -> Option<String> {
    match theme_pack_id.as_deref() {
        Some("nucleon-dark") | Some("nucleon-light") => Some("nucleon".to_string()),
        _ => theme_pack_id,
    }
}

fn desktop_theme_pack_id_from_settings(settings: &Settings) -> Option<String> {
    canonical_theme_pack_id(
        settings
            .desktop_theme_pack_id
            .clone()
            .or_else(|| settings.active_theme_pack_id.clone()),
    )
}

fn canonical_desktop_cursor_theme_selection(
    selection: DesktopCursorThemeSelection,
) -> DesktopCursorThemeSelection {
    match selection {
        DesktopCursorThemeSelection::ThemePack { theme_pack_id } => {
            DesktopCursorThemeSelection::ThemePack {
                theme_pack_id: canonical_theme_pack_id(Some(theme_pack_id))
                    .unwrap_or_else(|| "nucleon".to_string()),
            }
        }
        other => other,
    }
}

fn desktop_cursor_theme_selection_from_settings(
    settings: &Settings,
) -> DesktopCursorThemeSelection {
    canonical_desktop_cursor_theme_selection(settings.desktop_cursor_theme_selection.clone())
}

fn desktop_cursor_theme_pack_id_for_selection<'a>(
    desktop_theme_pack_id: Option<&'a str>,
    selection: &'a DesktopCursorThemeSelection,
) -> Option<&'a str> {
    match selection {
        DesktopCursorThemeSelection::FollowTheme => desktop_theme_pack_id,
        DesktopCursorThemeSelection::Builtin => None,
        DesktopCursorThemeSelection::ThemePack { theme_pack_id } => Some(theme_pack_id.as_str()),
    }
}

fn terminal_theme_pack_id_from_settings(settings: &Settings) -> Option<String> {
    canonical_theme_pack_id(
        settings
            .terminal_theme_pack_id
            .clone()
            .or_else(|| settings.active_theme_pack_id.clone()),
    )
}

const RETRO_FONT_BYTES: &[u8] =
    include_bytes!("../../assets/fonts/FixedsysExcelsior301-Regular.ttf");

fn try_load_font_bytes() -> Option<Vec<u8>> {
    if !RETRO_FONT_BYTES.is_empty() {
        return Some(RETRO_FONT_BYTES.to_vec());
    }

    let mut candidates = vec![
        PathBuf::from("assets/fonts/FixedsysExcelsior301-Regular.ttf"),
        PathBuf::from("assets/fonts/Sysfixed.ttf"),
        PathBuf::from("assets/fonts/sysfixed.ttf"),
        PathBuf::from("Sysfixed.ttf"),
        PathBuf::from("sysfixed.ttf"),
    ];
    let home = home_dir_fallback();
    candidates.push(home.join("Library/Fonts/Sysfixed.ttf"));
    candidates.push(home.join("Library/Fonts/sysfixed.ttf"));
    candidates.push(PathBuf::from("/Library/Fonts/Sysfixed.ttf"));
    candidates.push(PathBuf::from("/Library/Fonts/sysfixed.ttf"));
    candidates.push(PathBuf::from("/System/Library/Fonts/Monaco.ttf"));

    for path in candidates {
        if let Ok(bytes) = std::fs::read(&path) {
            return Some(bytes);
        }
    }
    None
}

pub fn configure_native_context(ctx: &Context) {
    configure_native_fonts(ctx);
    apply_native_appearance(ctx);
    let settings = crate::config::get_settings();
    let color_style = desktop_color_style_from_settings(&settings);
    apply_native_display_effects_for_settings(&settings, &color_style);
}

fn configure_native_fonts(ctx: &Context) {
    let mut fonts = FontDefinitions::default();
    if let Some(bytes) = try_load_font_bytes() {
        fonts
            .font_data
            .insert("retro".into(), FontData::from_owned(bytes));
        fonts
            .families
            .entry(FontFamily::Monospace)
            .or_default()
            .insert(0, "retro".into());
        fonts
            .families
            .entry(FontFamily::Proportional)
            .or_default()
            .insert(0, "retro".into());
    }
    ctx.set_fonts(fonts);
}

pub fn apply_native_appearance(ctx: &Context) {
    configure_visuals(ctx);
    apply_native_text_style(ctx);
}

fn apply_native_appearance_for_color_style(
    ctx: &Context,
    style: &ColorStyle,
    overrides: Option<&HashMap<ColorToken, [u8; 4]>>,
) {
    configure_visuals_for_palette(
        ctx,
        palette_for_color_style_with_overrides(style, overrides),
    );
    apply_native_text_style(ctx);
}

fn crt_theme_tint_for_color_style(style: &ColorStyle) -> [f32; 3] {
    let [r, g, b, _] = palette_for_color_style(style).fg.to_array();
    [r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0]
}

fn apply_native_display_effects_for_settings(settings: &Settings, color_style: &ColorStyle) {
    let display_effects = &settings.display_effects;
    let (monochrome_enabled, monochrome_tint) = match color_style {
        ColorStyle::Monochrome { .. } => (1u32, crt_theme_tint_for_color_style(color_style)),
        ColorStyle::FullColor { .. } => (0u32, [0.0, 0.0, 0.0]),
    };
    if !display_effects.enabled && monochrome_enabled == 0 {
        egui_wgpu::set_crt_effects(None);
        egui_winit::set_crt_pointer_curve(None);
        return;
    }
    let theme_tint = if display_effects.enabled && monochrome_enabled != 0 {
        monochrome_tint
    } else {
        [0.0, 0.0, 0.0]
    };
    egui_winit::set_crt_pointer_curve(
        (display_effects.enabled && display_effects.curvature > 0.0)
            .then_some(display_effects.curvature),
    );
    egui_wgpu::set_crt_effects(Some(CrtEffects {
        curvature: if display_effects.enabled {
            display_effects.curvature
        } else {
            0.0
        },
        scanlines: if display_effects.enabled {
            display_effects.scanlines
        } else {
            0.0
        },
        glow: if display_effects.enabled {
            display_effects.glow
        } else {
            0.0
        },
        bloom: if display_effects.enabled {
            display_effects.bloom
        } else {
            0.0
        },
        vignette: if display_effects.enabled {
            display_effects.vignette
        } else {
            0.0
        },
        noise: if display_effects.enabled {
            display_effects.noise
        } else {
            0.0
        },
        flicker: if display_effects.enabled {
            display_effects.flicker
        } else {
            0.0
        },
        jitter: if display_effects.enabled {
            display_effects.jitter
        } else {
            0.0
        },
        burn_in: if display_effects.enabled {
            display_effects.burn_in
        } else {
            0.0
        },
        glow_line: if display_effects.enabled {
            display_effects.glow_line
        } else {
            0.0
        },
        glow_line_speed: if display_effects.enabled {
            display_effects.glow_line_speed
        } else {
            0.2
        },
        brightness: if display_effects.enabled {
            display_effects.brightness
        } else {
            1.0
        },
        contrast: if display_effects.enabled {
            display_effects.contrast
        } else {
            1.0
        },
        phosphor_softness: if display_effects.enabled {
            display_effects.phosphor_softness
        } else {
            0.0
        },
        theme_tint,
        monochrome_enabled,
        monochrome_tint,
    }));
}

fn apply_native_text_style(ctx: &Context) {
    let mut style = (*ctx.style()).clone();
    // Keep global egui zoom fixed. Terminal-mode sizing is handled in RetroScreen
    // to avoid feedback loops between zoom and cell/grid calculations.
    ctx.set_zoom_factor(1.0);
    style.text_styles = [
        (TextStyle::Heading, FontId::new(28.0, FontFamily::Monospace)),
        (TextStyle::Body, FontId::new(22.0, FontFamily::Monospace)),
        (
            TextStyle::Monospace,
            FontId::new(22.0, FontFamily::Monospace),
        ),
        (TextStyle::Button, FontId::new(22.0, FontFamily::Monospace)),
        (TextStyle::Small, FontId::new(18.0, FontFamily::Monospace)),
    ]
    .into();
    ctx.set_style(style);
}

impl NucleonNativeApp {
    pub(super) fn current_shell_surface_kind(&self) -> ShellSurfaceKind {
        if self.desktop_mode_open {
            ShellSurfaceKind::Desktop
        } else {
            ShellSurfaceKind::Terminal
        }
    }

    pub(super) fn current_shell_palette(&self) -> RetroPalette {
        current_palette_for_surface(self.current_shell_surface_kind())
    }

    pub(super) fn render_classic_panel_slot(&mut self, ctx: &Context, layout: &LayoutProfile) {
        match layout.top_panel {
            PanelType::MenuBar => self.draw_top_bar(ctx, true, layout.top_panel_height),
            PanelType::Taskbar => self.draw_desktop_taskbar(ctx, true, layout.top_panel_height),
            PanelType::Disabled => {}
        }
    }

    pub(super) fn render_classic_dock_slot(&mut self, ctx: &Context, layout: &LayoutProfile) {
        match layout.bottom_panel {
            PanelType::MenuBar => self.draw_top_bar(ctx, false, layout.bottom_panel_height),
            PanelType::Taskbar => self.draw_desktop_taskbar(ctx, false, layout.bottom_panel_height),
            PanelType::Disabled => {
                self.desktop_start_button_rect = None;
            }
        }
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

    pub(super) fn render_classic_terminal_status_slot(
        &mut self,
        ctx: &Context,
        layout: &TerminalLayoutProfile,
    ) {
        self.draw_terminal_status_bar(ctx, layout.status_bar_position, layout.status_bar_height);
    }

    pub(super) fn render_classic_terminal_screen_slot(&mut self, ctx: &Context) {
        if !self.editor.open {
            match self.terminal_nav.screen {
                TerminalScreen::MainMenu => self.draw_terminal_main_menu(ctx),
                TerminalScreen::Applications => self.draw_terminal_applications(ctx),
                TerminalScreen::Documents => self.draw_terminal_documents(ctx),
                TerminalScreen::Logs => self.draw_terminal_logs(ctx),
                TerminalScreen::Network => self.draw_terminal_network(ctx),
                TerminalScreen::Games => self.draw_terminal_games(ctx),
                TerminalScreen::PtyApp => self.draw_terminal_pty(ctx),
                TerminalScreen::ProgramInstaller => self.draw_terminal_program_installer(ctx),
                TerminalScreen::DocumentBrowser => self.draw_terminal_document_browser(ctx),
                TerminalScreen::Settings => self.draw_terminal_settings(ctx),
                TerminalScreen::EditMenus => self.draw_terminal_edit_menus(ctx),
                TerminalScreen::Connections => self.draw_terminal_connections(ctx),
                TerminalScreen::DefaultApps => self.draw_terminal_default_apps(ctx),
                TerminalScreen::About => self.draw_terminal_about(ctx),
                TerminalScreen::UserManagement => self.draw_terminal_user_management(ctx),
            }
        }
    }

    pub(super) fn render_classic_terminal_overlay_slot(&mut self, ctx: &Context) {
        self.draw_terminal_prompt_overlay_global(ctx);
    }
}

pub struct NucleonNativeApp {
    login: TerminalLoginState,
    session: Option<SessionState>,
    file_manager: NativeFileManagerState,
    editor: EditorWindow,
    settings: SettingsWindow,
    tweaks_open: bool,
    applications: ApplicationsWindow,
    desktop_installer: DesktopInstallerState,
    terminal_mode: TerminalModeWindow,
    desktop_window_states: HashMap<WindowInstanceId, DesktopWindowState>,
    desktop_active_window: Option<WindowInstanceId>,
    next_window_instance: u32,
    /// Secondary (non-primary) window instances for multi-instance support.
    pub(super) secondary_windows: Vec<SecondaryWindow>,
    /// Set during swap-and-draw to indicate which window instance is being drawn.
    /// Window management functions use this to resolve the correct instance.
    drawing_window_id: Option<WindowInstanceId>,
    desktop_start_button_rect: Option<egui::Rect>,
    start_root_panel_height: f32,
    start_open: bool,
    start_selected_root: usize,
    start_system_selected: usize,
    start_leaf_selected: usize,
    start_open_submenu: Option<StartSubmenu>,
    start_open_leaf: Option<StartLeaf>,
    desktop_mode_open: bool,
    slot_registry: super::shell_slots::SlotRegistry,
    desktop_active_layout: LayoutProfile,
    terminal_active_layout: TerminalLayoutProfile,
    desktop_active_theme_pack_id: Option<String>,
    desktop_active_cursor_theme_selection: DesktopCursorThemeSelection,
    terminal_active_theme_pack_id: Option<String>,
    pub(super) desktop_active_shell_style: ShellStyle,
    desktop_active_color_style: ColorStyle,
    terminal_active_color_style: ColorStyle,
    pub(super) active_sound_pack_path: Option<PathBuf>,
    pub(super) active_asset_pack_path: Option<PathBuf>,
    pub(super) active_cursor_pack: Option<CursorPack>,
    pub(super) desktop_color_overrides: Option<HashMap<ColorToken, [u8; 4]>>,
    pub(super) terminal_color_overrides: Option<HashMap<ColorToken, [u8; 4]>>,
    pub(super) terminal_branding: TerminalBranding,
    pub(super) terminal_decoration: TerminalDecoration,
    terminal_slot_registry: super::terminal_slots::TerminalSlotRegistry,
    terminal_nav: TerminalNavigationState,
    terminal_settings_panel: TerminalSettingsPanel,
    terminal_pty: Option<NativePtyState>,
    terminal_pty_surface: Option<TerminalShellSurface>,
    terminal_wasm_addon: Option<WasmHostedAddonState>,
    terminal_wasm_addon_return_screen: Option<TerminalScreen>,
    terminal_wasm_addon_last_frame_at: Option<Instant>,
    terminal_installer: TerminalInstallerState,
    terminal_edit_menus: TerminalEditMenusState,
    terminal_connections: TerminalConnectionsState,
    terminal_prompt: Option<TerminalPrompt>,
    terminal_flash: Option<TerminalFlash>,
    command_layer: CommandLayerState,
    terminal_open_with_picker: Option<terminal_open_with_picker::OpenWithPickerState>,
    session_leader_until: Option<Instant>,
    session_runtime: HashMap<usize, ParkedSessionState>,
    desktop_window_generation_seed: u64,
    file_manager_runtime: FileManagerEditRuntime,
    asset_cache: Option<AssetCache>,
    icon_cache_dirty: bool,
    context_menu_action: Option<ContextMenuAction>,
    shell_status: String,
    desktop_selected_icon: Option<DesktopIconSelection>,
    desktop_item_properties: Option<DesktopItemPropertiesState>,
    shortcut_properties: Option<ShortcutPropertiesState>,
    start_menu_rename: Option<StartMenuRenameState>,
    picking_icon_for_shortcut: Option<usize>,
    picking_wallpaper: bool,
    picking_terminal_wallpaper: bool,
    pub(super) picking_theme_import: bool,
    shortcut_icon_cache: HashMap<String, egui::TextureHandle>,
    shortcut_icon_missing: HashSet<String>,
    terminal_wallpaper_texture: Option<egui::TextureHandle>,
    terminal_wallpaper_loaded_for: String,
    file_manager_preview_texture: Option<egui::TextureHandle>,
    file_manager_preview_loaded_for: String,
    desktop_icon_layout_cache: Option<DesktopIconLayoutCache>,
    desktop_surface_entries_cache: Option<DesktopSurfaceEntriesCache>,
    settings_home_rows_cache_admin: Option<SettingsHomeRowsCache>,
    settings_home_rows_cache_standard: Option<SettingsHomeRowsCache>,
    desktop_applications_sections_cache: Option<DesktopApplicationsSectionsCache>,
    edit_menu_entries_cache: EditMenuEntriesCache,
    pending_settings_panel: Option<NativeSettingsPanel>,
    sorted_user_records_cache: Option<Arc<Vec<(String, UserRecord)>>>,
    sorted_usernames_cache: Option<Arc<Vec<String>>>,
    saved_network_connections_cache: Option<Arc<Vec<SavedConnection>>>,
    saved_bluetooth_connections_cache: Option<Arc<Vec<SavedConnection>>>,
    live_desktop_file_manager_settings: DesktopFileManagerSettings,
    live_hacking_difficulty: HackingDifficulty,
    last_desktop_appearance: Option<DesktopAppearanceKey>,
    last_settings_sync_check: Instant,
    last_settings_file_mtime: Option<SystemTime>,
    startup_profile_session_logged: bool,
    startup_profile_desktop_logged: bool,
    repaint_trace_last_pass: u64,
    tweaks_tab: u8,               // 0=Wallpaper, 1=Theme, 2=Effects, 3=Display
    tweaks_wallpaper_surface: u8, // 0=Desktop, 1=Terminal
    tweaks_theme_surface: u8,     // 0=Desktop, 1=Terminal
    tweaks_layout_overrides_open: bool,
    tweaks_customize_colors_open: bool,
    tweaks_editing_color_token: Option<usize>, // index into ColorToken::all()
    terminal_tweaks_active_section: u8, // 0=Wallpaper, 1=Theme, 2=Effects, 3=Display
    terminal_tweaks_open_dropdown: Option<tweaks_presenter::TerminalTweaksDropdown>,
    // Spotlight search
    spotlight_open: bool,
    spotlight_query: String,
    spotlight_tab: u8, // 0=All 1=Apps 2=Documents 3=Files
    spotlight_selected: usize,
    spotlight_results: Vec<NativeSpotlightResult>,
    spotlight_last_query: String,
    spotlight_last_tab: u8,
    // Background I/O task runner
    pub(super) background: BackgroundTasks,
    desktop_wasm_addon: Option<WasmHostedAddonState>,
    desktop_wasm_addon_last_frame_at: Option<Instant>,
    retained_wasm_addons: Vec<WasmHostedAddonState>,
    // IPC receiver for messages from standalone apps
    ipc: super::ipc::IpcReceiver,
}

pub(super) struct ParkedSessionState {
    file_manager: NativeFileManagerState,
    editor: EditorWindow,
    settings: SettingsWindow,
    tweaks_open: bool,
    applications: ApplicationsWindow,
    desktop_installer: DesktopInstallerState,
    terminal_mode: TerminalModeWindow,
    desktop_window_states: HashMap<WindowInstanceId, DesktopWindowState>,
    desktop_active_window: Option<WindowInstanceId>,
    desktop_mode_open: bool,
    start_root_panel_height: f32,
    start_open: bool,
    start_selected_root: usize,
    start_system_selected: usize,
    start_leaf_selected: usize,
    start_open_submenu: Option<StartSubmenu>,
    start_open_leaf: Option<StartLeaf>,
    terminal_nav: TerminalNavigationState,
    terminal_settings_panel: TerminalSettingsPanel,
    terminal_pty: Option<NativePtyState>,
    terminal_pty_surface: Option<TerminalShellSurface>,
    terminal_wasm_addon: Option<WasmHostedAddonState>,
    terminal_wasm_addon_return_screen: Option<TerminalScreen>,
    terminal_installer: TerminalInstallerState,
    terminal_edit_menus: TerminalEditMenusState,
    terminal_connections: TerminalConnectionsState,
    terminal_prompt: Option<TerminalPrompt>,
    terminal_flash: Option<TerminalFlash>,
    session_leader_until: Option<Instant>,
    desktop_window_generation_seed: u64,
    file_manager_runtime: FileManagerEditRuntime,
    shell_status: String,
    start_menu_rename: Option<StartMenuRenameState>,
    secondary_windows: Vec<SecondaryWindow>,
    desktop_wasm_addon: Option<WasmHostedAddonState>,
    tweaks_tab: u8,
    tweaks_wallpaper_surface: u8,
    tweaks_theme_surface: u8,
    tweaks_layout_overrides_open: bool,
    tweaks_customize_colors_open: bool,
    tweaks_editing_color_token: Option<usize>,
    terminal_tweaks_active_section: u8,
    terminal_tweaks_open_dropdown: Option<tweaks_presenter::TerminalTweaksDropdown>,
    desktop_color_overrides: Option<HashMap<ColorToken, [u8; 4]>>,
    terminal_color_overrides: Option<HashMap<ColorToken, [u8; 4]>>,
    desktop_active_shell_style: ShellStyle,
    terminal_decoration: TerminalDecoration,
    picking_terminal_wallpaper: bool,
    picking_theme_import: bool,
    active_sound_pack_path: Option<std::path::PathBuf>,
    active_asset_pack_path: Option<std::path::PathBuf>,
    active_cursor_pack: Option<CursorPack>,
}

/// App-specific state for a secondary (non-primary) window instance.
pub(super) enum SecondaryWindowApp {
    FileManager {
        state: NativeFileManagerState,
        runtime: FileManagerEditRuntime,
    },
    Editor(EditorWindow),
    Pty(Option<NativePtyState>),
    WasmAddon {
        state: Option<WasmHostedAddonState>,
        last_frame_at: Option<Instant>,
    },
}

/// A secondary window instance — a second (or third, etc.) copy of an app.
pub(super) struct SecondaryWindow {
    pub(super) id: WindowInstanceId,
    pub(super) app: SecondaryWindowApp,
}

impl SecondaryWindow {
    pub(super) fn is_open(&self) -> bool {
        match &self.app {
            SecondaryWindowApp::FileManager { state, .. } => state.open,
            SecondaryWindowApp::Editor(editor) => editor.open,
            SecondaryWindowApp::Pty(state) => state.is_some(),
            SecondaryWindowApp::WasmAddon { state, .. } => state.is_some(),
        }
    }
}

impl Default for NucleonNativeApp {
    fn default() -> Self {
        restore_current_user_from_last_session();
        session::clear_sessions();
        session::take_switch_request();
        let settings_draft = load_settings_snapshot();
        let live_desktop_file_manager_settings = settings_draft.desktop_file_manager.clone();
        let live_hacking_difficulty = settings_draft.hacking_difficulty;
        let settings_ui_defaults = build_desktop_settings_ui_defaults(&settings_draft, None);
        let terminal_defaults = terminal_runtime_defaults();
        let initial_desktop_color_style = desktop_color_style_from_settings(&settings_draft);
        let initial_terminal_color_style = terminal_color_style_from_settings(&settings_draft);
        let initial_desktop_theme_pack_id = desktop_theme_pack_id_from_settings(&settings_draft);
        let initial_desktop_cursor_theme_selection =
            desktop_cursor_theme_selection_from_settings(&settings_draft);
        let initial_terminal_theme_pack_id = terminal_theme_pack_id_from_settings(&settings_draft);
        let initial_desktop_shell_style =
            desktop_shell_style_from_theme_pack_id(initial_desktop_theme_pack_id.as_deref());
        let initial_desktop_layout = desktop_layout_from_settings(&settings_draft);
        let initial_terminal_layout = terminal_layout_from_settings(&settings_draft);
        let initial_terminal_branding = terminal_branding_from_settings(&settings_draft);
        let initial_terminal_decoration = terminal_decoration_from_settings(&settings_draft);
        let mut app = Self {
            login: TerminalLoginState::default(),
            session: None,
            file_manager: NativeFileManagerState::new(home_dir_fallback()),
            editor: EditorWindow::default(),
            settings: SettingsWindow {
                open: false,
                draft: settings_draft,
                status: String::new(),
                panel: settings_ui_defaults.panel,
                default_app_custom_text_code: settings_ui_defaults.default_app_custom_text_code,
                default_app_custom_ebook: settings_ui_defaults.default_app_custom_ebook,
                scanned_networks: Vec::new(),
                scanned_bluetooth: Vec::new(),
                connection_password: String::new(),
                edit_target: EditMenuTarget::Applications,
                edit_name_input: String::new(),
                edit_value_input: String::new(),
                cli_profile_slot: settings_ui_defaults.cli_profile_slot,
                user_selected: settings_ui_defaults.user_selected,
                user_selected_loaded_for: settings_ui_defaults.user_selected_loaded_for,
                user_create_username: String::new(),
                user_create_auth: settings_ui_defaults.user_create_auth,
                user_create_password: String::new(),
                user_create_password_confirm: String::new(),
                user_edit_auth: settings_ui_defaults.user_edit_auth,
                user_edit_password: String::new(),
                user_edit_password_confirm: String::new(),
                user_delete_confirm: String::new(),
            },
            tweaks_open: false,
            applications: ApplicationsWindow::default(),
            desktop_installer: DesktopInstallerState::default(),
            terminal_mode: TerminalModeWindow::default(),
            desktop_window_states: HashMap::new(),
            desktop_active_window: None,
            next_window_instance: 1,
            secondary_windows: Vec::new(),
            drawing_window_id: None,
            desktop_start_button_rect: None,
            start_root_panel_height: 260.0,
            start_open: true,
            start_selected_root: 0,
            start_system_selected: 0,
            start_leaf_selected: 0,
            start_open_submenu: None,
            start_open_leaf: None,
            desktop_mode_open: false,
            slot_registry: super::shell_slots::SlotRegistry::classic(),
            desktop_active_layout: initial_desktop_layout,
            terminal_active_layout: initial_terminal_layout,
            desktop_active_theme_pack_id: initial_desktop_theme_pack_id,
            desktop_active_cursor_theme_selection: initial_desktop_cursor_theme_selection,
            terminal_active_theme_pack_id: initial_terminal_theme_pack_id,
            desktop_active_shell_style: initial_desktop_shell_style,
            desktop_active_color_style: initial_desktop_color_style,
            terminal_active_color_style: initial_terminal_color_style,
            active_sound_pack_path: None,
            active_asset_pack_path: None,
            active_cursor_pack: None,
            desktop_color_overrides: None,
            terminal_color_overrides: None,
            terminal_branding: initial_terminal_branding,
            terminal_decoration: initial_terminal_decoration,
            terminal_slot_registry: super::terminal_slots::TerminalSlotRegistry::classic(),
            terminal_nav: terminal_defaults,
            terminal_settings_panel: TerminalSettingsPanel::Home,
            terminal_pty: None,
            terminal_pty_surface: None,
            terminal_wasm_addon: None,
            terminal_wasm_addon_return_screen: None,
            terminal_wasm_addon_last_frame_at: None,
            terminal_installer: TerminalInstallerState::default(),
            terminal_edit_menus: TerminalEditMenusState::default(),
            terminal_connections: TerminalConnectionsState::default(),
            terminal_prompt: None,
            terminal_flash: None,
            command_layer: CommandLayerState::default(),
            terminal_open_with_picker: None,
            session_leader_until: None,
            session_runtime: HashMap::new(),
            desktop_window_generation_seed: 1,
            file_manager_runtime: FileManagerEditRuntime::default(),
            asset_cache: None,
            icon_cache_dirty: true,
            context_menu_action: None,
            shell_status: String::new(),
            desktop_selected_icon: None,
            desktop_item_properties: None,
            shortcut_properties: None,
            start_menu_rename: None,
            picking_icon_for_shortcut: None,
            picking_wallpaper: false,
            picking_terminal_wallpaper: false,
            picking_theme_import: false,
            shortcut_icon_cache: HashMap::new(),
            shortcut_icon_missing: HashSet::new(),
            terminal_wallpaper_texture: None,
            terminal_wallpaper_loaded_for: String::new(),
            file_manager_preview_texture: None,
            file_manager_preview_loaded_for: String::new(),
            desktop_icon_layout_cache: None,
            desktop_surface_entries_cache: None,
            settings_home_rows_cache_admin: None,
            settings_home_rows_cache_standard: None,
            desktop_applications_sections_cache: None,
            edit_menu_entries_cache: EditMenuEntriesCache {
                applications: None,
                documents: None,
                network: None,
                games: None,
            },
            pending_settings_panel: None,
            sorted_user_records_cache: None,
            sorted_usernames_cache: None,
            saved_network_connections_cache: None,
            saved_bluetooth_connections_cache: None,
            live_desktop_file_manager_settings,
            live_hacking_difficulty,
            last_desktop_appearance: None,
            last_settings_sync_check: Instant::now(),
            last_settings_file_mtime: Self::current_settings_file_mtime(),
            startup_profile_session_logged: false,
            startup_profile_desktop_logged: false,
            repaint_trace_last_pass: 0,
            tweaks_tab: 0,
            tweaks_wallpaper_surface: 0,
            tweaks_theme_surface: 0,
            tweaks_layout_overrides_open: false,
            tweaks_customize_colors_open: false,
            tweaks_editing_color_token: None,
            terminal_tweaks_active_section: 1,
            terminal_tweaks_open_dropdown: None,
            spotlight_open: false,
            spotlight_query: String::new(),
            spotlight_tab: 0,
            spotlight_selected: 0,
            spotlight_results: Vec::new(),
            spotlight_last_query: String::new(),
            spotlight_last_tab: u8::MAX,
            background: BackgroundTasks::new(),
            desktop_wasm_addon: None,
            desktop_wasm_addon_last_frame_at: None,
            retained_wasm_addons: Vec::new(),
            ipc: super::ipc::start_listener(),
        };
        app.sync_active_sound_pack();
        app.sync_active_desktop_asset_pack();
        crate::config::spawn_addon_repository_index_refresh();
        app.maybe_apply_profile_autologin();
        app
    }
}

impl NucleonNativeApp {
    fn sync_active_sound_pack(&mut self) -> bool {
        let next_sound_pack_path =
            desktop_sound_pack_path_from_theme_pack_id(self.desktop_active_theme_pack_id.as_deref());
        let changed = self.active_sound_pack_path != next_sound_pack_path;
        if changed {
            self.active_sound_pack_path = next_sound_pack_path.clone();
            crate::sound::set_active_sound_pack(next_sound_pack_path);
        }
        changed
    }

    fn sync_active_desktop_asset_pack(&mut self) -> bool {
        let next_asset_pack_path =
            desktop_asset_pack_path_from_theme_pack_id(self.desktop_active_theme_pack_id.as_deref());
        let next_cursor_pack = desktop_cursor_pack_from_theme_pack_id(
            desktop_cursor_theme_pack_id_for_selection(
                self.desktop_active_theme_pack_id.as_deref(),
                &self.desktop_active_cursor_theme_selection,
            ),
        );
        let changed = self.active_asset_pack_path != next_asset_pack_path
            || self.active_cursor_pack != next_cursor_pack;
        self.active_asset_pack_path = next_asset_pack_path;
        self.active_cursor_pack = next_cursor_pack;
        changed
    }

    fn apply_surface_theme_state_from_settings(&mut self) {
        let previous_desktop_theme_pack_id = self.desktop_active_theme_pack_id.clone();
        let previous_desktop_color_style = self.desktop_active_color_style.clone();
        self.desktop_active_theme_pack_id =
            desktop_theme_pack_id_from_settings(&self.settings.draft);
        self.desktop_active_cursor_theme_selection =
            desktop_cursor_theme_selection_from_settings(&self.settings.draft);
        self.terminal_active_theme_pack_id =
            terminal_theme_pack_id_from_settings(&self.settings.draft);
        self.desktop_active_shell_style =
            desktop_shell_style_from_theme_pack_id(self.desktop_active_theme_pack_id.as_deref());
        self.desktop_active_color_style = desktop_color_style_from_settings(&self.settings.draft);
        self.terminal_active_color_style = terminal_color_style_from_settings(&self.settings.draft);
        self.desktop_active_layout = desktop_layout_from_settings(&self.settings.draft);
        self.terminal_active_layout = terminal_layout_from_settings(&self.settings.draft);
        self.terminal_branding = terminal_branding_from_settings(&self.settings.draft);
        self.terminal_decoration = terminal_decoration_from_settings(&self.settings.draft);
        self.desktop_color_overrides =
            theme_pack_color_overrides_from_theme_pack_id(self.desktop_active_theme_pack_id.as_deref());
        self.terminal_color_overrides =
            theme_pack_color_overrides_from_theme_pack_id(self.terminal_active_theme_pack_id.as_deref());
        self.tweaks_customize_colors_open = false;
        self.last_desktop_appearance = None;
        self.sync_active_sound_pack();
        let asset_state_changed = self.sync_active_desktop_asset_pack();
        if previous_desktop_theme_pack_id != self.desktop_active_theme_pack_id
            || previous_desktop_color_style != self.desktop_active_color_style
            || asset_state_changed
        {
            self.icon_cache_dirty = true;
        }
    }

    fn persist_surface_theme_state_to_settings(&mut self) {
        self.settings.draft.desktop_theme_pack_id = self.desktop_active_theme_pack_id.clone();
        self.settings.draft.desktop_cursor_theme_selection =
            self.desktop_active_cursor_theme_selection.clone();
        self.settings.draft.terminal_theme_pack_id = self.terminal_active_theme_pack_id.clone();
        self.settings.draft.desktop_color_style = Some(self.desktop_active_color_style.clone());
        self.settings.draft.terminal_color_style = Some(self.terminal_active_color_style.clone());
        self.settings.draft.desktop_layout_profile = Some(self.desktop_active_layout.clone());
        self.settings.draft.terminal_layout_profile = Some(self.terminal_active_layout.clone());
        self.settings.draft.terminal_branding =
            terminal_branding_setting_value(&self.terminal_branding);
        self.settings.draft.active_theme_pack_id = self.desktop_active_theme_pack_id.clone();

        if let ColorStyle::Monochrome { preset, custom_rgb } = &self.desktop_active_color_style {
            self.settings.draft.theme = match preset {
                MonochromePreset::Green => "Green (Default)",
                MonochromePreset::White => "White",
                MonochromePreset::Amber => "Amber",
                MonochromePreset::Blue => "Blue",
                MonochromePreset::LightBlue => "Light Blue",
                MonochromePreset::Custom => crate::config::CUSTOM_THEME_NAME,
            }
            .to_string();
            if let Some(custom_rgb) = custom_rgb {
                self.settings.draft.custom_theme_rgb = *custom_rgb;
            }
        }
    }

    fn sync_native_display_effects(&self) {
        let active_color_style = if self.desktop_mode_open {
            &self.desktop_active_color_style
        } else {
            &self.terminal_active_color_style
        };
        apply_native_display_effects_for_settings(&self.settings.draft, active_color_style);
    }

    fn sync_native_cursor_mode(&self) {
        let cursor_mode = if self.desktop_mode_open && self.settings.draft.desktop_show_cursor {
            egui_winit::AppCursorMode::Software
        } else {
            egui_winit::AppCursorMode::Hidden
        };
        egui_winit::set_app_cursor_mode(cursor_mode);
    }

    fn sync_desktop_appearance(&mut self, ctx: &Context) {
        let key = DesktopAppearanceKey {
            color_style: self.desktop_active_color_style.clone(),
            overrides: self.desktop_color_overrides.clone(),
        };
        set_active_shell_style(self.desktop_active_shell_style.clone());
        set_active_color_style(
            ShellSurfaceKind::Desktop,
            self.desktop_active_color_style.clone(),
            self.desktop_color_overrides.clone(),
        );
        if self.last_desktop_appearance.as_ref() == Some(&key) {
            return;
        }
        apply_native_appearance_for_color_style(
            ctx,
            &self.desktop_active_color_style,
            self.desktop_color_overrides.as_ref(),
        );
        self.last_desktop_appearance = Some(key);
    }

    fn sync_terminal_wallpaper(&mut self, ctx: &Context) {
        let wallpaper_path = self.settings.draft.terminal_wallpaper.as_str();
        let monochrome_wallpaper = matches!(
            self.terminal_active_color_style,
            crate::theme::ColorStyle::Monochrome { .. }
        );
        let cache_key = format!(
            "{}#{}",
            wallpaper_path,
            if monochrome_wallpaper {
                "monochrome"
            } else {
                "full-color"
            }
        );
        if self.terminal_wallpaper_loaded_for != cache_key {
            self.terminal_wallpaper_texture =
                Self::load_wallpaper_texture(ctx, wallpaper_path, monochrome_wallpaper);
            self.terminal_wallpaper_loaded_for = cache_key;
        }
        set_active_terminal_wallpaper(
            self.terminal_wallpaper_texture.as_ref(),
            self.settings.draft.terminal_wallpaper_size_mode,
            monochrome_wallpaper,
        );
    }

    fn sync_terminal_appearance(&mut self, ctx: &Context) {
        set_active_shell_style(self.desktop_active_shell_style.clone());
        set_active_color_style(
            ShellSurfaceKind::Terminal,
            self.terminal_active_color_style.clone(),
            self.terminal_color_overrides.clone(),
        );
        set_active_terminal_decoration(self.terminal_decoration.clone());
        self.sync_terminal_wallpaper(ctx);
    }

    fn native_pty_window_min_size(state: &NativePtyState) -> egui::Vec2 {
        let cols = state.desktop_cols_floor.unwrap_or(40) as f32;
        // Add one row for the desktop footer/status line that sits under the PTY grid.
        let rows = state.desktop_rows_floor.unwrap_or(20).saturating_add(1) as f32;
        egui::vec2(
            (cols * FIXED_PTY_CELL_W).max(640.0),
            (rows * FIXED_PTY_CELL_H).max(300.0),
        )
    }

    fn terminal_layout(&self) -> TerminalLayout {
        terminal_layout_with_branding(&self.terminal_branding)
    }

    pub(super) fn active_terminal_header_lines(&self) -> &[String] {
        &self.terminal_branding.header_lines
    }

    pub(super) fn next_embedded_game_dt(last_frame_at: &mut Option<Instant>) -> f32 {
        let now = Instant::now();
        let dt = last_frame_at
            .replace(now)
            .map(|previous| (now - previous).as_secs_f32())
            .unwrap_or(1.0 / 60.0);
        if !(0.0..=0.25).contains(&dt) {
            1.0 / 60.0
        } else {
            dt.min(1.0 / 20.0)
        }
    }
}

impl eframe::App for NucleonNativeApp {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        Color32::from_rgb(0, 0, 0).to_normalized_gamma_f32()
    }

    fn save(&mut self, _storage: &mut dyn eframe::Storage) {
        self.persist_snapshot();
    }

    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        self.update_native_shell_frame(ctx);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{DesktopShortcut, FileManagerSortMode, FileManagerViewMode};
    use crate::core::auth::{load_users, save_users, AuthMethod, UserRecord};
    use crate::native::desktop_app::DesktopLaunchPayload;
    use crate::native::file_manager_app::{FileManagerClipboardMode, OpenWithLaunchRequest};
    use crate::native::installer_screen::DesktopInstallerView;
    use std::collections::HashMap;
    use std::sync::{Mutex, OnceLock};

    fn session_test_guard() -> std::sync::MutexGuard<'static, ()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        match LOCK.get_or_init(|| Mutex::new(())).lock() {
            Ok(guard) => guard,
            Err(err) => err.into_inner(),
        }
    }

    struct UsersRestore {
        backup: HashMap<String, UserRecord>,
    }

    impl Drop for UsersRestore {
        fn drop(&mut self) {
            save_users(&self.backup);
        }
    }

    fn install_test_users(usernames: &[&str]) -> UsersRestore {
        let backup = load_users();
        let mut db = HashMap::new();
        for username in usernames {
            db.insert(
                (*username).to_string(),
                UserRecord {
                    password_hash: String::new(),
                    is_admin: *username == "u1",
                    auth_method: AuthMethod::NoPassword,
                },
            );
        }
        save_users(&db);
        UsersRestore { backup }
    }

    struct TempDirGuard {
        path: PathBuf,
    }

    impl TempDirGuard {
        fn new(prefix: &str) -> Self {
            let unique = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("test clock")
                .as_nanos();
            let path = std::env::temp_dir().join(format!(
                "nucleon_native_{prefix}_{}_{}",
                std::process::id(),
                unique
            ));
            std::fs::create_dir_all(&path).expect("create temp test dir");
            Self { path }
        }
    }

    impl Drop for TempDirGuard {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }

    struct SettingsRestore {
        backup: Settings,
    }

    impl SettingsRestore {
        fn capture() -> Self {
            Self {
                backup: crate::config::get_settings(),
            }
        }
    }

    impl Drop for SettingsRestore {
        fn drop(&mut self) {
            crate::config::update_settings(|settings| *settings = self.backup.clone());
            crate::config::save_settings(&self.backup);
        }
    }

    fn set_runtime_marker(app: &mut NucleonNativeApp, screen: TerminalScreen, idx: usize, tag: &str) {
        app.desktop_mode_open = false;
        app.start_open = true;
        app.start_selected_root = idx % START_ROOT_ITEMS.len();
        app.start_system_selected = idx % 2;
        app.start_leaf_selected = idx % 3;
        app.start_open_submenu = if idx % 2 == 0 {
            Some(StartSubmenu::System)
        } else {
            None
        };
        app.start_open_leaf = if idx % 2 == 1 {
            Some(StartLeaf::Applications)
        } else {
            None
        };
        app.terminal_nav.main_menu_idx = idx;
        app.terminal_nav.screen = screen;
        app.terminal_nav.settings_idx = idx;
        app.terminal_nav.user_management_idx = idx;
        app.file_manager.open = true;
        app.file_manager.cwd = PathBuf::from(format!("/tmp/{tag}"));
        app.file_manager.selected = Some(PathBuf::from(format!("/tmp/{tag}/selected.txt")));
        app.editor.open = true;
        app.editor.path = Some(PathBuf::from(format!("/tmp/{tag}/doc.txt")));
        app.editor.text = tag.to_string();
        app.editor.dirty = idx % 2 == 0;
        app.editor.status = format!("status-{tag}");
        app.editor.ui.show_line_numbers = idx % 2 == 0;
        app.editor.ui.find_open = idx % 2 == 1;
        app.editor.ui.find_replace_visible = idx % 3 == 0;
        app.editor.ui.find_query = format!("find-{tag}");
        app.editor.ui.replace_query = format!("replace-{tag}");
        app.editor.ui.find_occurrence = idx;
        app.editor.ui.text_align = match idx % 3 {
            1 => EditorTextAlign::Center,
            2 => EditorTextAlign::Right,
            _ => EditorTextAlign::Left,
        };
        app.settings.open = idx % 2 == 1;
        app.settings.status = format!("settings-{tag}");
        app.applications.open = idx % 2 == 0;
        app.applications.status = format!("apps-{tag}");
        app.terminal_mode.open = idx % 2 == 1;
        app.terminal_mode.status = format!("term-{tag}");
        app.shell_status = format!("shell-{tag}");
    }

    #[test]
    fn parked_session_runtime_restores_full_native_context() {
        let _guard = session_test_guard();
        session::clear_sessions();
        session::take_switch_request();

        let mut app = NucleonNativeApp::default();
        let idx = session::push_session("admin");
        session::set_active(idx);
        app.session = Some(SessionState {
            username: "admin".to_string(),
            is_admin: true,
        });

        app.file_manager.open = true;
        app.file_manager.cwd = PathBuf::from("/tmp");
        app.file_manager.selected = Some(PathBuf::from("/tmp/demo.txt"));
        app.editor.open = true;
        app.editor.path = Some(PathBuf::from("/tmp/doc.txt"));
        app.editor.text = "hello".to_string();
        app.editor.dirty = true;
        app.editor.status = "dirty".to_string();
        app.settings.open = true;
        app.settings.status = "saved".to_string();
        app.settings.draft.theme = CUSTOM_THEME_NAME.to_string();
        app.applications.open = true;
        app.applications.status = "apps".to_string();
        app.terminal_mode.open = true;
        app.terminal_mode.status = "term".to_string();
        app.desktop_mode_open = true;
        app.start_open = false;
        app.start_selected_root = 6;
        app.start_system_selected = 1;
        app.start_leaf_selected = 2;
        app.start_open_submenu = None;
        app.start_open_leaf = Some(StartLeaf::Games);
        app.terminal_nav.main_menu_idx = 3;
        app.terminal_nav.screen = TerminalScreen::Connections;
        app.terminal_nav.settings_idx = 2;
        app.terminal_nav.user_management_idx = 4;
        app.session_leader_until = Some(Instant::now() + Duration::from_millis(500));
        app.shell_status = "status".to_string();
        app.terminal_prompt = Some(TerminalPrompt {
            kind: TerminalPromptKind::Input,
            title: "t".to_string(),
            prompt: "p".to_string(),
            buffer: "buf".to_string(),
            confirm_yes: true,
            action: TerminalPromptAction::Noop,
        });
        app.terminal_flash = Some(TerminalFlash {
            message: "flash".to_string(),
            until: Instant::now() + Duration::from_millis(500),
            action: FlashAction::ExitApp,
            boxed: false,
        });

        app.park_active_session_runtime();

        app.file_manager.open = false;
        app.file_manager.cwd = PathBuf::from(".");
        app.file_manager.selected = None;
        app.editor.open = false;
        app.editor.path = None;
        app.editor.text.clear();
        app.editor.dirty = false;
        app.editor.status.clear();
        app.editor.ui = nucleon_native_editor_app::EditorUiState::default();
        app.settings.open = false;
        app.settings.status.clear();
        app.settings.draft.theme = "Green (Default)".to_string();
        app.applications.open = false;
        app.applications.status.clear();
        app.terminal_mode.open = false;
        app.terminal_mode.status.clear();
        app.desktop_mode_open = false;
        app.start_open = true;
        app.start_selected_root = 0;
        app.start_system_selected = 0;
        app.start_leaf_selected = 0;
        app.start_open_submenu = None;
        app.start_open_leaf = None;
        app.terminal_nav.main_menu_idx = 0;
        app.terminal_nav.screen = TerminalScreen::MainMenu;
        app.terminal_nav.settings_idx = 0;
        app.terminal_nav.user_management_idx = 0;
        app.session_leader_until = None;
        app.shell_status.clear();
        app.terminal_prompt = None;
        app.terminal_flash = None;

        assert!(app.restore_active_session_runtime_if_any());

        assert!(app.file_manager.open);
        assert_eq!(app.file_manager.cwd, PathBuf::from("/tmp"));
        assert_eq!(
            app.file_manager.selected,
            Some(PathBuf::from("/tmp/demo.txt"))
        );
        assert!(app.editor.open);
        assert_eq!(app.editor.path, Some(PathBuf::from("/tmp/doc.txt")));
        assert_eq!(app.editor.text, "hello");
        assert!(app.editor.dirty);
        assert_eq!(app.editor.status, "dirty");
        assert!(!app.editor.ui.show_line_numbers);
        assert!(!app.editor.ui.find_open);
        assert!(!app.editor.ui.find_replace_visible);
        assert!(app.editor.ui.find_query.is_empty());
        assert!(app.editor.ui.replace_query.is_empty());
        assert_eq!(app.editor.ui.find_occurrence, 0);
        assert_eq!(app.editor.ui.text_align, EditorTextAlign::Left);
        assert!(app.settings.open);
        assert_eq!(app.settings.status, "saved");
        assert_eq!(app.settings.draft.theme, CUSTOM_THEME_NAME);
        assert!(app.applications.open);
        assert_eq!(app.applications.status, "apps");
        assert!(app.terminal_mode.open);
        assert_eq!(app.terminal_mode.status, "term");
        assert!(app.desktop_mode_open);
        assert!(!app.start_open);
        assert_eq!(app.start_selected_root, 6);
        assert_eq!(app.start_system_selected, 1);
        assert_eq!(app.start_leaf_selected, 2);
        assert_eq!(app.start_open_submenu, None);
        assert_eq!(app.start_open_leaf, Some(StartLeaf::Games));
        assert_eq!(app.terminal_nav.main_menu_idx, 3);
        assert!(matches!(
            app.terminal_nav.screen,
            TerminalScreen::Connections
        ));
        assert_eq!(app.terminal_nav.settings_idx, 2);
        assert_eq!(app.terminal_nav.user_management_idx, 4);
        assert!(app.session_leader_until.is_some());
        assert_eq!(app.shell_status, "status");
        assert!(app.terminal_prompt.is_some());
        assert!(app.terminal_flash.is_some());
        assert!(!app.session_runtime.contains_key(&idx));
    }

    #[test]
    fn session_switch_restores_each_sessions_screen_context() {
        let _guard = session_test_guard();
        let _users = install_test_users(&["u1", "u2"]);
        session::clear_sessions();
        session::take_switch_request();

        let mut app = NucleonNativeApp::default();
        let s1 = session::push_session("u1");
        let s2 = session::push_session("u2");

        session::set_active(s1);
        assert!(app.sync_active_session_identity());
        set_runtime_marker(&mut app, TerminalScreen::Settings, 2, "u1-a");
        app.park_active_session_runtime();

        session::set_active(s2);
        assert!(app.sync_active_session_identity());
        set_runtime_marker(&mut app, TerminalScreen::Connections, 7, "u2-b");
        app.park_active_session_runtime();

        session::set_active(s1);
        assert!(app.sync_active_session_identity());
        assert!(app.restore_active_session_runtime_if_any());
        assert!(matches!(app.terminal_nav.screen, TerminalScreen::Settings));
        assert_eq!(app.terminal_nav.main_menu_idx, 2);
        assert_eq!(app.editor.text, "u1-a");
        assert_eq!(app.editor.ui.find_query, "find-u1-a");

        session::request_switch(s2);
        app.apply_pending_session_switch();
        assert_eq!(session::active_idx(), s2);
        assert!(matches!(
            app.terminal_nav.screen,
            TerminalScreen::Connections
        ));
        assert_eq!(app.terminal_nav.main_menu_idx, 7);
        assert_eq!(app.editor.text, "u2-b");
        assert_eq!(app.editor.ui.find_query, "find-u2-b");

        session::request_switch(s1);
        app.apply_pending_session_switch();
        assert_eq!(session::active_idx(), s1);
        assert!(matches!(app.terminal_nav.screen, TerminalScreen::Settings));
        assert_eq!(app.terminal_nav.main_menu_idx, 2);
        assert_eq!(app.editor.text, "u1-a");
        assert_eq!(app.editor.ui.find_query, "find-u1-a");
    }

    #[test]
    fn close_session_restores_previous_sessions_parked_runtime() {
        let _guard = session_test_guard();
        let _users = install_test_users(&["u1", "u2", "u3"]);
        session::clear_sessions();
        session::take_switch_request();

        let mut app = NucleonNativeApp::default();
        let s1 = session::push_session("u1");
        let s2 = session::push_session("u2");
        let s3 = session::push_session("u3");

        session::set_active(s1);
        assert!(app.sync_active_session_identity());
        set_runtime_marker(&mut app, TerminalScreen::Documents, 1, "u1");
        app.park_active_session_runtime();

        session::set_active(s2);
        assert!(app.sync_active_session_identity());
        set_runtime_marker(&mut app, TerminalScreen::ProgramInstaller, 5, "u2");
        app.park_active_session_runtime();

        session::set_active(s3);
        assert!(app.sync_active_session_identity());
        set_runtime_marker(&mut app, TerminalScreen::Games, 9, "u3");

        app.close_active_session_window();

        assert_eq!(session::session_count(), 2);
        assert_eq!(session::active_idx(), 1);
        assert_eq!(session::active_username().as_deref(), Some("u2"));
        assert!(matches!(
            app.terminal_nav.screen,
            TerminalScreen::ProgramInstaller
        ));
        assert_eq!(app.terminal_nav.main_menu_idx, 5);
        assert_eq!(app.editor.text, "u2");
        assert_eq!(app.editor.ui.find_query, "find-u2");
        assert_eq!(app.shell_status, "Closed session 3.");
    }

    fn terminal_submenu_screens() -> [TerminalScreen; 12] {
        [
            TerminalScreen::Applications,
            TerminalScreen::Documents,
            TerminalScreen::Network,
            TerminalScreen::Games,
            TerminalScreen::ProgramInstaller,
            TerminalScreen::Logs,
            TerminalScreen::DocumentBrowser,
            TerminalScreen::Settings,
            TerminalScreen::EditMenus,
            TerminalScreen::Connections,
            TerminalScreen::DefaultApps,
            TerminalScreen::UserManagement,
        ]
    }

    #[test]
    fn session_switch_restores_every_terminal_submenu_context() {
        let _guard = session_test_guard();
        let _users = install_test_users(&["u1", "u2"]);

        for (idx, screen) in terminal_submenu_screens().into_iter().enumerate() {
            session::clear_sessions();
            session::take_switch_request();

            let mut app = NucleonNativeApp::default();
            let s1 = session::push_session("u1");
            let s2 = session::push_session("u2");

            session::set_active(s1);
            assert!(app.sync_active_session_identity());
            set_runtime_marker(&mut app, screen, idx + 1, &format!("u1-{idx}"));
            app.park_active_session_runtime();

            session::set_active(s2);
            assert!(app.sync_active_session_identity());
            set_runtime_marker(
                &mut app,
                TerminalScreen::MainMenu,
                idx + 100,
                &format!("u2-{idx}"),
            );
            app.park_active_session_runtime();

            session::set_active(s1);
            assert!(app.sync_active_session_identity());
            assert!(app.restore_active_session_runtime_if_any());
            assert_eq!(app.terminal_nav.screen, screen);
            assert_eq!(app.editor.text, format!("u1-{idx}"));
            assert_eq!(app.editor.ui.find_query, format!("find-u1-{idx}"));

            session::request_switch(s2);
            app.apply_pending_session_switch();
            assert_eq!(session::active_idx(), s2);
            assert!(matches!(app.terminal_nav.screen, TerminalScreen::MainMenu));
            assert_eq!(app.editor.text, format!("u2-{idx}"));
            assert_eq!(app.editor.ui.find_query, format!("find-u2-{idx}"));

            session::request_switch(s1);
            app.apply_pending_session_switch();
            assert_eq!(session::active_idx(), s1);
            assert_eq!(app.terminal_nav.screen, screen);
            assert_eq!(app.editor.text, format!("u1-{idx}"));
            assert_eq!(app.editor.ui.find_query, format!("find-u1-{idx}"));
        }
    }

    #[test]
    fn editor_command_open_find_replace_sets_editor_search_mode() {
        let _guard = session_test_guard();

        let mut app = NucleonNativeApp::default();
        app.editor.ui.find_open = false;
        app.editor.ui.find_replace_visible = false;
        app.editor.ui.find_occurrence = 9;

        app.run_editor_command(EditorCommand::OpenFindReplace);

        assert!(app.editor.ui.find_open);
        assert!(app.editor.ui.find_replace_visible);
        assert_eq!(app.editor.ui.find_occurrence, 0);

        app.run_editor_command(EditorCommand::CloseFind);
        assert!(!app.editor.ui.find_open);
    }

    #[test]
    fn editor_commands_update_view_and_format_state() {
        let _guard = session_test_guard();

        let mut app = NucleonNativeApp::default();
        app.editor.word_wrap = true;
        app.editor.font_size = 16.0;
        app.editor.ui.show_line_numbers = false;
        app.editor.ui.text_align = EditorTextAlign::Left;

        app.run_editor_command(EditorCommand::ToggleWordWrap);
        app.run_editor_command(EditorCommand::IncreaseFontSize);
        app.run_editor_command(EditorCommand::SetTextAlign(EditorTextAlign::Right));
        app.run_editor_command(EditorCommand::ToggleLineNumbers);
        app.run_editor_command(EditorCommand::ResetFontSize);

        assert!(!app.editor.word_wrap);
        assert_eq!(app.editor.font_size, 16.0);
        assert_eq!(app.editor.ui.text_align, EditorTextAlign::Right);
        assert!(app.editor.ui.show_line_numbers);
    }

    #[test]
    fn file_manager_commands_update_navigation_and_clipboard_state() {
        let _guard = session_test_guard();
        let temp = TempDirGuard::new("file_manager_commands");
        let note = temp.path.join("note.txt");
        let other = temp.path.join("other");
        std::fs::write(&note, "note").expect("write note");
        std::fs::create_dir_all(&other).expect("create other directory");

        let mut app = NucleonNativeApp::default();
        app.file_manager = NativeFileManagerState::new(temp.path.clone());
        app.file_manager.tabs = vec![temp.path.clone(), other.clone()];
        app.file_manager.active_tab = 0;
        app.file_manager.cwd = temp.path.clone();
        app.file_manager.tree_selected = Some(temp.path.clone());
        app.file_manager.update_search_query("note".to_string());

        app.run_file_manager_command(FileManagerCommand::ClearSearch);
        assert!(app.file_manager.search_query.is_empty());

        app.run_file_manager_command(FileManagerCommand::NextTab);
        assert_eq!(app.file_manager.active_tab, 1);
        assert_eq!(app.file_manager.cwd, other);

        app.run_file_manager_command(FileManagerCommand::PreviousTab);
        assert_eq!(app.file_manager.active_tab, 0);
        assert_eq!(app.file_manager.cwd, temp.path);

        app.file_manager.select(Some(note.clone()));
        app.run_file_manager_command(FileManagerCommand::Copy);

        let clipboard = app
            .file_manager_runtime
            .clipboard
            .as_ref()
            .expect("copy puts selection on clipboard");
        assert_eq!(clipboard.paths, vec![note]);
        assert!(matches!(clipboard.mode, FileManagerClipboardMode::Copy));
        assert_eq!(app.shell_status, "Copied note.txt");
    }

    #[test]
    fn autologin_open_mode_honors_desktop_default() {
        let _guard = session_test_guard();

        let mut app = NucleonNativeApp::default();
        app.desktop_mode_open = false;
        app.start_open = true;
        app.settings.draft.default_open_mode = OpenMode::Desktop;

        app.apply_autologin_open_mode();

        assert!(app.desktop_mode_open);
        assert!(!app.start_open);

        app.desktop_mode_open = false;
        app.start_open = true;
        app.settings.draft.default_open_mode = OpenMode::Terminal;

        app.apply_autologin_open_mode();

        assert!(!app.desktop_mode_open);
        assert!(app.start_open);
    }

    #[test]
    fn file_manager_label_truncation_preserves_distinguishing_suffix() {
        assert_eq!(
            NucleonNativeApp::truncate_file_manager_label(
                "Screenshot 2026-03-18 at 17.37.56.png",
                16,
            ),
            "Screens...56.png"
        );
    }

    #[test]
    fn desktop_icon_label_lines_compact_long_file_names() {
        assert_eq!(
            NucleonNativeApp::desktop_icon_label_lines("Screenshot 2026-03-18 at 17.37.56.png"),
            vec!["Screenshot".to_string(), "2026-...56.png".to_string()]
        );
    }

    #[test]
    fn file_manager_commands_open_prompts_and_update_view_settings() {
        let _guard = session_test_guard();
        let _settings = SettingsRestore::capture();
        let temp = TempDirGuard::new("file_manager_command_settings");
        let note = temp.path.join("note.txt");
        std::fs::write(&note, "note").expect("write note");

        crate::config::update_settings(|settings| {
            settings.desktop_file_manager.show_tree_panel = false;
            settings.desktop_file_manager.show_hidden_files = false;
            settings.desktop_file_manager.view_mode = FileManagerViewMode::Grid;
            settings.desktop_file_manager.sort_mode = FileManagerSortMode::Name;
        });

        let mut app = NucleonNativeApp::default();
        app.file_manager.cwd = temp.path.clone();
        app.file_manager.select(Some(note.clone()));

        app.run_file_manager_command(FileManagerCommand::Rename);
        assert_eq!(
            app.terminal_prompt
                .as_ref()
                .map(|prompt| prompt.title.as_str()),
            Some("Rename")
        );

        app.terminal_prompt = None;
        app.run_file_manager_command(FileManagerCommand::Move);
        assert_eq!(
            app.terminal_prompt
                .as_ref()
                .map(|prompt| prompt.title.as_str()),
            Some("Move To")
        );

        app.run_file_manager_command(FileManagerCommand::ToggleTreePanel);
        app.run_file_manager_command(FileManagerCommand::ToggleHiddenFiles);
        app.run_file_manager_command(FileManagerCommand::SetViewMode(FileManagerViewMode::List));
        app.run_file_manager_command(FileManagerCommand::SetSortMode(FileManagerSortMode::Type));

        let settings = crate::config::get_settings();
        assert!(settings.desktop_file_manager.show_tree_panel);
        assert!(settings.desktop_file_manager.show_hidden_files);
        assert_eq!(
            settings.desktop_file_manager.view_mode,
            FileManagerViewMode::List
        );
        assert_eq!(
            settings.desktop_file_manager.sort_mode,
            FileManagerSortMode::Type
        );
    }

    #[test]
    fn terminal_document_browser_commands_use_highlighted_row_selection() {
        let _guard = session_test_guard();
        let temp = TempDirGuard::new("terminal_browser_delete");
        let keep = temp.path.join("alpha.txt");
        let target = temp.path.join("beta.txt");
        std::fs::write(&keep, "alpha").expect("write keep file");
        std::fs::write(&target, "beta").expect("write target file");

        let mut app = NucleonNativeApp::default();
        app.open_document_browser_at(temp.path.clone(), TerminalScreen::Documents);
        let rows = crate::native::document_browser::browser_rows(&app.file_manager);
        let target_idx = rows
            .iter()
            .position(|row| row.path.as_ref() == Some(&target))
            .expect("target row present");
        app.terminal_nav.browser_idx = target_idx;

        crate::native::document_browser::sync_browser_selection(
            &mut app.file_manager,
            app.terminal_nav.browser_idx,
        );
        app.run_file_manager_command(FileManagerCommand::Delete);

        assert!(keep.exists());
        assert!(!target.exists());
    }

    #[test]
    fn terminal_editor_blocks_logs_menu_space_shortcut() {
        let _guard = session_test_guard();

        let mut app = NucleonNativeApp::default();
        app.terminal_nav.screen = TerminalScreen::Logs;
        app.terminal_nav.logs_idx = 0;
        app.editor.open = true;
        app.editor.text = "existing log".to_string();
        app.editor.path = Some(PathBuf::from("/tmp/log.txt"));

        let ctx = Context::default();
        let raw_input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1024.0, 768.0),
            )),
            events: vec![egui::Event::Key {
                key: Key::Space,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            }],
            ..Default::default()
        };

        let _ = ctx.run(raw_input, |ctx| {
            app.draw_terminal_runtime(ctx);
        });

        assert!(app.editor.open);
        assert!(app.terminal_prompt.is_none());
        assert_eq!(app.terminal_nav.screen, TerminalScreen::Logs);
    }

    #[test]
    fn wallpaper_picker_commit_persists_before_settings_reset() {
        let _guard = session_test_guard();
        let _settings = SettingsRestore::capture();
        let wallpaper = PathBuf::from("/tmp/nucleon-wallpaper.png");
        let wallpaper_str = wallpaper.display().to_string();

        let mut app = NucleonNativeApp::default();
        app.file_manager.open = true;
        app.picking_wallpaper = true;

        app.apply_file_manager_picker_commit(FileManagerPickerCommit::SetWallpaper(
            wallpaper.clone(),
        ));

        for _ in 0..40 {
            if crate::config::get_settings().desktop_wallpaper == wallpaper_str {
                break;
            }
            std::thread::sleep(Duration::from_millis(5));
        }

        app.reset_desktop_settings_window();

        assert_eq!(app.settings.draft.desktop_wallpaper, wallpaper_str);
        assert_eq!(
            crate::config::get_settings().desktop_wallpaper,
            wallpaper_str
        );
        assert!(!app.picking_wallpaper);
        assert!(!app.file_manager.open);
    }

    #[test]
    fn svg_preview_texture_lazy_loads_uncached_svg_rows() {
        let _guard = session_test_guard();
        let temp = TempDirGuard::new("svg_preview_lazy_load");
        let svg_path = temp.path.join("icon.svg");
        std::fs::write(
            &svg_path,
            r##"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24"><rect width="24" height="24" fill="#ffffff"/></svg>"##,
        )
        .expect("write svg");

        let mut app = NucleonNativeApp::default();
        let row = FileEntryRow {
            path: svg_path.clone(),
            label: "icon.svg".to_string(),
            is_dir: false,
        };

        let preview = app.svg_preview_texture(&Context::default(), &row);

        assert!(preview.is_some());
        assert!(app
            .shortcut_icon_cache
            .contains_key(&svg_path.to_string_lossy().to_string()));
    }

    #[test]
    fn file_manager_preview_texture_downscales_large_wallpaper_images() {
        let _guard = session_test_guard();
        let temp = TempDirGuard::new("wallpaper_preview_texture");
        let png_path = temp.path.join("wallpaper.png");
        image::RgbaImage::from_pixel(640, 320, image::Rgba([255, 255, 255, 255]))
            .save(&png_path)
            .expect("write png");

        let row = FileEntryRow {
            path: png_path.clone(),
            label: "wallpaper.png".to_string(),
            is_dir: false,
        };

        let mut app = NucleonNativeApp::default();
        let texture = app
            .file_manager_preview_texture(&Context::default(), &row)
            .expect("preview texture");

        assert!(texture.size()[0] <= 192);
        assert!(texture.size()[1] <= 192);
        assert_eq!(
            app.file_manager_preview_loaded_for,
            format!("{}#monochrome", png_path.to_string_lossy())
        );
    }

    #[test]
    fn choosing_shortcut_icon_keeps_loaded_texture_cached() {
        let _guard = session_test_guard();
        let _settings = SettingsRestore::capture();
        let temp = TempDirGuard::new("shortcut_icon_cache_lifetime");
        let svg_path = temp.path.join("icon.svg");
        std::fs::write(
            &svg_path,
            r##"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24"><rect width="24" height="24" fill="#ffffff"/></svg>"##,
        )
        .expect("write svg");

        let mut app = NucleonNativeApp::default();
        app.settings.draft.desktop_shortcuts.push(DesktopShortcut {
            label: "Test".to_string(),
            app_name: "Test".to_string(),
            pos_x: None,
            pos_y: None,
            launch_command: None,
            icon_path: None,
            shortcut_kind: "app".to_string(),
        });
        app.file_manager.open = true;
        app.picking_icon_for_shortcut = Some(0);

        let cache_key = svg_path.to_string_lossy().to_string();
        let _texture = app
            .load_cached_shortcut_icon(&Context::default(), &cache_key, &svg_path, 32)
            .expect("cached svg texture");

        app.apply_file_manager_picker_commit(FileManagerPickerCommit::SetShortcutIcon {
            shortcut_idx: 0,
            path: svg_path.clone(),
        });

        assert!(app.shortcut_icon_cache.contains_key(&cache_key));
        assert_eq!(
            app.settings.draft.desktop_shortcuts[0].icon_path.as_deref(),
            Some(cache_key.as_str())
        );
        assert_eq!(app.picking_icon_for_shortcut, None);
        assert!(!app.file_manager.open);
    }

    #[test]
    fn closing_dirty_desktop_editor_opens_confirmation_prompt() {
        let _guard = session_test_guard();

        let mut app = NucleonNativeApp::default();
        app.desktop_mode_open = true;
        app.editor.open = true;
        app.editor.path = Some(PathBuf::from("/tmp/existing.txt"));
        app.editor.text = "keep me?".to_string();
        app.editor.dirty = true;
        app.editor.status = "Unsaved changes".to_string();
        app.editor.ui.find_open = true;
        app.editor.ui.find_replace_visible = true;
        app.editor.ui.find_query = "keep".to_string();
        app.editor.ui.replace_query = "drop".to_string();
        app.editor.ui.find_occurrence = 3;

        app.update_desktop_window_state(DesktopWindow::Editor, false);

        assert!(app.editor.open);
        assert_eq!(app.editor.path, Some(PathBuf::from("/tmp/existing.txt")));
        assert_eq!(app.editor.text, "keep me?");
        assert!(app.editor.dirty);
        assert_eq!(app.editor.status, "Unsaved changes");
        assert!(app.editor.close_confirmation_visible());
    }

    #[test]
    fn quitting_dirty_desktop_editor_resets_document_for_next_launch() {
        let _guard = session_test_guard();

        let mut app = NucleonNativeApp::default();
        app.desktop_mode_open = true;
        app.editor.open = true;
        app.editor.path = Some(PathBuf::from("/tmp/existing.txt"));
        app.editor.text = "keep me?".to_string();
        app.editor.dirty = true;
        app.editor.status = "Unsaved changes".to_string();
        app.editor.ui.find_open = true;
        app.editor.ui.find_replace_visible = true;
        app.editor.ui.find_query = "keep".to_string();
        app.editor.ui.replace_query = "drop".to_string();
        app.editor.ui.find_occurrence = 3;

        app.update_desktop_window_state(DesktopWindow::Editor, false);
        app.editor.cancel_close_confirmation();
        app.close_current_editor_window_unchecked();

        assert!(!app.editor.open);
        assert_eq!(app.editor.path, None);
        assert!(app.editor.text.is_empty());
        assert!(!app.editor.dirty);
        assert_eq!(
            app.editor.status,
            "New document. Save to choose where it goes."
        );
        assert!(!app.editor.ui.find_open);
        assert!(!app.editor.ui.find_replace_visible);
        assert!(app.editor.ui.find_query.is_empty());
        assert!(app.editor.ui.replace_query.is_empty());
        assert_eq!(app.editor.ui.find_occurrence, 0);
    }

    #[test]
    fn save_as_picker_numbers_duplicate_names() {
        let _guard = session_test_guard();
        let temp = TempDirGuard::new("save_as_collision");
        let existing = temp.path.join("document.txt");
        std::fs::write(&existing, "existing").expect("seed existing document");

        let mut app = NucleonNativeApp::default();
        app.desktop_mode_open = true;
        app.file_manager.cwd = temp.path.clone();
        app.file_manager.open = true;
        app.editor.text = "new content".to_string();
        app.editor.dirty = true;
        app.editor.save_as_input = Some("document.txt".to_string());

        app.complete_editor_save_as_from_picker();

        let saved = temp.path.join("document (1).txt");
        assert_eq!(app.editor.path, Some(saved.clone()));
        assert!(!app.editor.dirty);
        assert_eq!(
            app.editor.status,
            "Name already existed. Saved as document (1).txt."
        );
        assert_eq!(
            std::fs::read_to_string(&existing).expect("read existing document"),
            "existing"
        );
        assert_eq!(
            std::fs::read_to_string(&saved).expect("read numbered document"),
            "new content"
        );
        assert!(!app.file_manager.open);
        assert!(app.editor.open);
        assert!(app.editor.save_as_input.is_none());
    }

    #[test]
    fn save_as_picker_can_finish_pending_close_request() {
        let _guard = session_test_guard();
        let temp = TempDirGuard::new("save_as_then_close");

        let mut app = NucleonNativeApp::default();
        app.desktop_mode_open = true;
        app.file_manager.cwd = temp.path.clone();
        app.file_manager.open = true;
        app.editor.open = true;
        app.editor.text = "new content".to_string();
        app.editor.dirty = true;
        app.editor.save_as_input = Some("document.txt".to_string());
        app.editor.queue_close_after_save();

        app.complete_editor_save_as_from_picker();

        let saved = temp.path.join("document.txt");
        assert_eq!(
            std::fs::read_to_string(&saved).expect("read saved document"),
            "new content"
        );
        assert!(!app.editor.open);
        assert_eq!(app.editor.path, None);
        assert!(app.editor.text.is_empty());
        assert!(!app.editor.dirty);
        assert!(app.editor.close_confirm.is_none());
        assert!(!app.file_manager.open);
    }

    #[test]
    fn terminal_save_as_opens_path_prompt_instead_of_file_manager() {
        let _guard = session_test_guard();
        let temp = TempDirGuard::new("terminal_save_as_prompt");
        let existing = temp.path.join("draft.txt");

        let mut app = NucleonNativeApp::default();
        app.desktop_mode_open = false;
        app.editor.open = true;
        app.editor.path = Some(existing.clone());

        app.open_editor_save_as_picker();

        assert!(!app.file_manager.open);
        assert!(matches!(
            app.terminal_prompt.as_ref().map(|prompt| &prompt.action),
            Some(TerminalPromptAction::EditorSaveAsPath)
        ));
        assert_eq!(
            app.terminal_prompt
                .as_ref()
                .map(|prompt| prompt.buffer.as_str()),
            Some(existing.to_string_lossy().as_ref())
        );
    }

    #[test]
    fn terminal_save_as_prompt_saves_relative_to_current_document_directory() {
        let _guard = session_test_guard();
        let temp = TempDirGuard::new("terminal_save_as_relative");
        let current = temp.path.join("current.txt");
        let expected = temp.path.join("archive").join("copied.txt");

        let mut app = NucleonNativeApp::default();
        app.desktop_mode_open = false;
        app.editor.open = true;
        app.editor.path = Some(current);
        app.editor.text = "saved from prompt".to_string();
        app.editor.dirty = true;
        app.file_manager.open = true;

        assert!(app.save_editor_from_prompt_path("archive/copied.txt"));

        assert_eq!(
            std::fs::read_to_string(&expected).expect("read saved prompt target"),
            "saved from prompt"
        );
        assert_eq!(app.editor.path, Some(expected));
        assert!(!app.editor.dirty);
        assert!(app.file_manager.open);
    }

    #[test]
    fn terminal_save_without_path_opens_save_as_prompt() {
        let _guard = session_test_guard();

        let mut app = NucleonNativeApp::default();
        app.desktop_mode_open = false;
        app.editor.open = true;
        app.editor.text = "unsaved".to_string();
        app.editor.dirty = true;

        assert!(!app.save_editor_to_current_path());
        assert!(matches!(
            app.terminal_prompt.as_ref().map(|prompt| &prompt.action),
            Some(TerminalPromptAction::EditorSaveAsPath)
        ));
    }

    #[test]
    fn file_manager_new_folder_uses_numbered_suffix() {
        let _guard = session_test_guard();
        let temp = TempDirGuard::new("new_folder_collision");
        let existing = temp.path.join("New Folder");
        std::fs::create_dir_all(&existing).expect("seed existing folder");

        let mut app = NucleonNativeApp::default();
        app.file_manager.cwd = temp.path.clone();

        app.run_file_manager_command(FileManagerCommand::NewFolder);

        let created = temp.path.join("New Folder (1)");
        assert!(created.is_dir());
        assert_eq!(app.file_manager.selected, Some(created.clone()));
        assert_eq!(app.shell_status, "Created New Folder (1)");
    }

    #[test]
    fn desktop_shell_action_reveals_file_in_file_manager() {
        let _guard = session_test_guard();
        let temp = TempDirGuard::new("shell_action_reveal");
        let file_path = temp.path.join("demo.txt");
        std::fs::write(&file_path, "demo").expect("write temp file");

        let mut app = NucleonNativeApp::default();
        app.execute_desktop_shell_action(DesktopShellAction::LaunchByTargetWithPayload {
            target: launch_registry::file_manager_launch_target(),
            payload: DesktopLaunchPayload::RevealPath(file_path.clone()),
        });

        assert!(app.file_manager.open);
        assert_eq!(app.file_manager.cwd, temp.path);
        assert_eq!(app.file_manager.selected, Some(file_path));
    }

    #[test]
    fn desktop_surface_directory_open_uses_file_manager_launch_payload() {
        let _guard = session_test_guard();
        let temp = TempDirGuard::new("desktop_surface_dir");
        let folder = temp.path.join("docs");
        std::fs::create_dir_all(&folder).expect("create desktop folder");

        let mut app = NucleonNativeApp::default();
        app.open_desktop_surface_path(folder.clone());

        assert!(app.file_manager.open);
        assert!(app.desktop_window_is_open(DesktopWindow::FileManager));
        assert_eq!(app.file_manager.cwd, folder);
    }

    #[test]
    fn desktop_surface_text_file_open_uses_editor_launch_payload() {
        let _guard = session_test_guard();
        let temp = TempDirGuard::new("desktop_surface_editor");
        let file_path = temp.path.join("desktop-note.txt");
        std::fs::write(&file_path, "surface text").expect("write desktop note");

        let mut app = NucleonNativeApp::default();
        app.open_desktop_surface_path(file_path.clone());

        assert!(app.editor.open);
        assert!(app.desktop_window_is_open(DesktopWindow::Editor));
        assert_eq!(app.editor.path, Some(file_path));
        assert_eq!(app.editor.text, "surface text");
    }

    #[test]
    fn opening_start_menu_closes_spotlight() {
        let _guard = session_test_guard();

        let mut app = NucleonNativeApp::default();
        app.spotlight_open = true;
        app.spotlight_query = "demo".to_string();

        app.open_start_menu();

        assert!(app.start_open);
        assert!(!app.spotlight_open);
    }

    #[test]
    fn opening_desktop_window_closes_spotlight_in_desktop_mode() {
        let _guard = session_test_guard();

        let mut app = NucleonNativeApp::default();
        app.desktop_mode_open = true;
        app.spotlight_open = true;

        app.open_desktop_window(DesktopWindow::Editor);

        assert_eq!(
            app.desktop_active_window,
            Some(WindowInstanceId::primary(DesktopWindow::Editor))
        );
        assert!(!app.spotlight_open);
    }

    #[test]
    fn opening_second_desktop_pty_spawns_secondary_window() {
        let _guard = session_test_guard();

        let mut app = NucleonNativeApp::default();
        app.desktop_mode_open = true;
        let cmd = vec![
            "/bin/sh".to_string(),
            "-lc".to_string(),
            "sleep 30".to_string(),
        ];

        app.open_desktop_pty("Terminal", &cmd);
        assert!(app.terminal_pty.is_some());
        assert!(app.secondary_windows.is_empty());

        app.open_desktop_pty("Terminal", &cmd);

        assert!(app.terminal_pty.is_some());
        let secondary = app
            .secondary_windows
            .iter()
            .find(|window| window.id.kind == DesktopWindow::PtyApp)
            .expect("secondary pty window");
        assert_eq!(app.desktop_active_window, Some(secondary.id));
        assert!(matches!(
            &secondary.app,
            SecondaryWindowApp::Pty(Some(state)) if state.title == "Terminal"
        ));

        app.terminate_all_native_pty_children();
    }

    #[test]
    fn closing_drawn_secondary_pty_window_keeps_primary_open() {
        let _guard = session_test_guard();

        let mut app = NucleonNativeApp::default();
        app.desktop_mode_open = true;
        let cmd = vec![
            "/bin/sh".to_string(),
            "-lc".to_string(),
            "sleep 30".to_string(),
        ];

        app.open_desktop_pty("Terminal", &cmd);
        app.open_desktop_pty("ranger", &cmd);

        let secondary_id = app
            .secondary_windows
            .iter()
            .find(|window| window.id.kind == DesktopWindow::PtyApp)
            .map(|window| window.id)
            .expect("secondary pty window");

        let mut swapped_state = None;
        let mut swapped_surface = Some(TerminalShellSurface::Desktop);
        {
            let slot = app
                .desktop_pty_slot_mut(secondary_id)
                .expect("secondary pty slot");
            std::mem::swap(slot, &mut swapped_state);
        }
        std::mem::swap(&mut app.terminal_pty, &mut swapped_state);
        std::mem::swap(&mut app.terminal_pty_surface, &mut swapped_surface);
        app.drawing_window_id = Some(secondary_id);
        app.update_desktop_window_state(DesktopWindow::PtyApp, false);
        std::mem::swap(&mut app.terminal_pty_surface, &mut swapped_surface);
        std::mem::swap(&mut app.terminal_pty, &mut swapped_state);
        {
            let slot = app
                .desktop_pty_slot_mut(secondary_id)
                .expect("secondary pty slot after close");
            std::mem::swap(slot, &mut swapped_state);
        }
        app.drawing_window_id = None;

        assert!(app.terminal_pty.is_some());
        assert!(app.primary_desktop_pty_open());
        assert!(app.desktop_pty_state(secondary_id).is_none());

        app.terminate_all_native_pty_children();
    }

    #[test]
    fn reopening_settings_window_reprimes_component_state() {
        let _guard = session_test_guard();
        let mut app = NucleonNativeApp::default();
        app.settings.open = true;
        app.settings.panel = NativeSettingsPanel::Appearance;
        let state =
            app.desktop_window_state_mut(WindowInstanceId::primary(DesktopWindow::Settings));
        state.restore_pos = Some([24.0, 48.0]);
        state.restore_size = Some([640.0, 360.0]);
        state.apply_restore = true;
        state.maximized = true;
        state.minimized = true;
        state.user_resized = true;
        state.generation = 7;

        app.open_desktop_window(DesktopWindow::Settings);

        let state = app.desktop_window_state(WindowInstanceId::primary(DesktopWindow::Settings));
        assert!(app.settings.open);
        assert_eq!(app.settings.panel, desktop_settings_default_panel());
        assert_eq!(state.restore_pos, None);
        assert_eq!(state.restore_size, None);
        assert!(!state.apply_restore);
        assert!(!state.maximized);
        assert!(!state.minimized);
        assert!(!state.user_resized);
        assert_ne!(state.generation, 7);
    }

    #[test]
    fn change_appearance_context_menu_opens_settings_appearance_panel() {
        let _guard = session_test_guard();
        let mut app = NucleonNativeApp::default();
        app.context_menu_action = Some(ContextMenuAction::ChangeAppearance);

        app.dispatch_context_menu_action(&Context::default());

        assert!(app.tweaks_open);
        assert_eq!(
            app.desktop_active_window,
            Some(WindowInstanceId::primary(DesktopWindow::Tweaks))
        );
    }

    #[test]
    fn settings_launch_target_with_panel_payload_opens_requested_panel() {
        let _guard = session_test_guard();
        let mut app = NucleonNativeApp::default();

        app.execute_desktop_shell_action(DesktopShellAction::LaunchByTargetWithPayload {
            target: launch_registry::settings_launch_target(),
            payload: DesktopLaunchPayload::OpenSettingsPanel(NativeSettingsPanel::Connections),
        });

        assert!(app.settings.open);
        assert!(app.desktop_window_is_open(DesktopWindow::Settings));
        assert_eq!(app.settings.panel, NativeSettingsPanel::Connections);
    }

    #[test]
    fn settings_launch_target_opens_settings_window() {
        let _guard = session_test_guard();
        let mut app = NucleonNativeApp::default();

        app.launch_settings_via_registry();

        assert!(app.settings.open);
        assert!(app.desktop_window_is_open(DesktopWindow::Settings));
        assert_eq!(app.settings.panel, desktop_settings_default_panel());
    }

    #[test]
    fn terminal_default_apps_capability_maps_to_terminal_screen() {
        let target = launch_registry::default_apps_launch_target();

        assert_eq!(
            resolve_terminal_launch_target(&target),
            Some(NativeTerminalLaunch::OpenScreen(
                TerminalScreen::DefaultApps
            ))
        );
    }

    #[test]
    fn terminal_unknown_capability_is_not_routable() {
        let target = crate::platform::LaunchTarget::Capability {
            capability: crate::platform::CapabilityId::from("made-up-capability"),
        };

        assert_eq!(resolve_terminal_launch_target(&target), None);
    }

    #[test]
    fn disabled_settings_panel_coerces_to_home_panel() {
        let panel = NucleonNativeApp::coerce_desktop_settings_panel_for_visibility(
            NativeSettingsPanel::Connections,
            DesktopSettingsVisibility {
                default_apps: true,
                connections: false,
                edit_menus: true,
                about: true,
            },
        );

        assert_eq!(panel, desktop_settings_default_panel());
    }

    #[test]
    fn file_manager_launch_target_opens_file_manager_window() {
        let _guard = session_test_guard();
        let mut app = NucleonNativeApp::default();

        app.launch_file_manager_via_registry();

        assert!(app.file_manager.open);
        assert!(app.desktop_window_is_open(DesktopWindow::FileManager));
    }

    #[test]
    fn editor_launch_target_opens_editor_window() {
        let _guard = session_test_guard();
        let mut app = NucleonNativeApp::default();

        app.launch_editor_via_registry();

        assert!(app.editor.open);
        assert!(app.desktop_window_is_open(DesktopWindow::Editor));
    }

    #[test]
    fn terminal_launch_target_opens_terminal_window() {
        let _guard = session_test_guard();
        let mut app = NucleonNativeApp::default();

        app.execute_desktop_shell_action(DesktopShellAction::LaunchByTarget(
            launch_registry::terminal_launch_target(),
        ));

        assert!(app.terminal_mode.open);
        assert!(app.desktop_window_is_open(DesktopWindow::TerminalMode));
    }

    #[test]
    fn programs_launch_target_opens_applications_window() {
        let _guard = session_test_guard();
        let mut app = NucleonNativeApp::default();

        app.execute_desktop_shell_action(DesktopShellAction::LaunchByTarget(
            launch_registry::programs_launch_target(),
        ));

        assert!(app.applications.open);
        assert!(app.desktop_window_is_open(DesktopWindow::Applications));
    }

    #[test]
    fn connections_launch_target_opens_settings_connections_panel() {
        let _guard = session_test_guard();
        let mut app = NucleonNativeApp::default();

        app.execute_desktop_shell_action(DesktopShellAction::LaunchByTarget(
            launch_registry::connections_launch_target(),
        ));

        assert!(app.settings.open);
        assert!(app.desktop_window_is_open(DesktopWindow::Settings));
        assert_eq!(app.settings.panel, NativeSettingsPanel::Connections);
    }

    #[test]
    fn desktop_menu_open_settings_uses_registry_launch() {
        let _guard = session_test_guard();
        let mut app = NucleonNativeApp::default();

        app.apply_desktop_menu_action(&Context::default(), &DesktopMenuAction::OpenSettings);

        assert!(app.settings.open);
        assert!(app.desktop_window_is_open(DesktopWindow::Settings));
        assert_eq!(app.settings.panel, desktop_settings_default_panel());
    }

    #[test]
    fn desktop_menu_open_file_manager_uses_registry_launch() {
        let _guard = session_test_guard();
        let mut app = NucleonNativeApp::default();

        app.apply_desktop_menu_action(&Context::default(), &DesktopMenuAction::OpenFileManager);

        assert!(app.file_manager.open);
        assert!(app.desktop_window_is_open(DesktopWindow::FileManager));
    }

    #[test]
    fn start_menu_program_installer_uses_registry_launch() {
        let _guard = session_test_guard();
        let mut app = NucleonNativeApp::default();
        app.start_open = true;

        app.run_start_system_action(desktop_start_menu::StartSystemAction::ProgramInstaller);

        assert!(!app.start_open);
        assert!(app.desktop_installer.open);
        assert!(app.desktop_window_is_open(DesktopWindow::Installer));
    }

    #[test]
    fn start_menu_connections_uses_registry_launch() {
        let _guard = session_test_guard();
        let mut app = NucleonNativeApp::default();
        app.start_open = true;

        app.run_start_system_action(desktop_start_menu::StartSystemAction::Connections);

        assert!(!app.start_open);
        assert!(app.settings.open);
        assert!(app.desktop_window_is_open(DesktopWindow::Settings));
        assert_eq!(app.settings.panel, NativeSettingsPanel::Connections);
    }

    #[test]
    fn generic_context_menu_open_settings_uses_registry_launch() {
        let _guard = session_test_guard();
        let mut app = NucleonNativeApp::default();
        app.context_menu_action = Some(ContextMenuAction::OpenSettings);

        app.dispatch_context_menu_action(&Context::default());

        assert!(app.settings.open);
        assert!(app.desktop_window_is_open(DesktopWindow::Settings));
        assert_eq!(app.settings.panel, desktop_settings_default_panel());
    }

    #[test]
    fn desktop_program_request_open_file_manager_uses_registry_launch() {
        let _guard = session_test_guard();
        let mut app = NucleonNativeApp::default();

        app.apply_desktop_program_request(DesktopProgramRequest::OpenFileManager);

        assert!(app.file_manager.open);
        assert!(app.desktop_window_is_open(DesktopWindow::FileManager));
    }

    #[test]
    fn desktop_program_request_open_text_editor_uses_registry_launch() {
        let _guard = session_test_guard();
        let mut app = NucleonNativeApp::default();

        app.apply_desktop_program_request(DesktopProgramRequest::OpenTextEditor {
            close_window: true,
        });

        assert!(app.editor.open);
        assert!(app.desktop_window_is_open(DesktopWindow::Editor));
    }

    #[test]
    fn open_text_editor_action_uses_registry_launch() {
        let _guard = session_test_guard();
        let mut app = NucleonNativeApp::default();

        app.execute_desktop_shell_action(DesktopShellAction::LaunchByTarget(
            launch_registry::editor_launch_target(),
        ));

        assert!(app.editor.open);
        assert!(app.desktop_window_is_open(DesktopWindow::Editor));
    }

    #[test]
    fn spotlight_terminal_result_uses_registry_launch() {
        let _guard = session_test_guard();
        let mut app = NucleonNativeApp::default();
        app.spotlight_open = true;

        app.spotlight_activate_result(&NativeSpotlightResult {
            name: "Terminal".to_string(),
            category: NativeSpotlightCategory::System,
            path: None,
        });

        assert!(!app.spotlight_open);
        assert!(app.terminal_mode.open);
        assert!(app.desktop_window_is_open(DesktopWindow::TerminalMode));
    }

    #[test]
    fn launch_target_with_editor_path_payload_opens_requested_file() {
        let _guard = session_test_guard();
        let temp = TempDirGuard::new("editor_payload_open");
        let file_path = temp.path.join("notes.txt");
        std::fs::write(&file_path, "hello").expect("write temp editor file");

        let mut app = NucleonNativeApp::default();
        app.execute_desktop_shell_action(DesktopShellAction::LaunchByTargetWithPayload {
            target: launch_registry::editor_launch_target(),
            payload: DesktopLaunchPayload::OpenPath(file_path.clone()),
        });

        assert!(app.editor.open);
        assert!(app.desktop_window_is_open(DesktopWindow::Editor));
        assert_eq!(app.editor.path, Some(file_path));
        assert_eq!(app.editor.text, "hello");
    }

    #[test]
    fn terminal_document_open_reuses_embedded_editor_surface() {
        let _guard = session_test_guard();
        let temp = TempDirGuard::new("terminal_editor_policy");
        let first_path = temp.path.join("first.txt");
        let second_path = temp.path.join("second.txt");
        std::fs::write(&first_path, "first").expect("write first editor file");
        std::fs::write(&second_path, "second").expect("write second editor file");

        let mut app = NucleonNativeApp::default();
        app.desktop_mode_open = false;
        app.open_embedded_path_in_editor(first_path);

        app.open_file_with_default_app_or_editor(second_path.clone());

        assert!(app.editor.open);
        assert_eq!(app.editor.path, Some(second_path));
        assert_eq!(app.editor.text, "second");
        assert!(app
            .secondary_windows
            .iter()
            .all(|window| window.id.kind != DesktopWindow::Editor));
    }

    #[test]
    fn terminal_open_with_request_uses_embedded_pty_surface() {
        let _guard = session_test_guard();
        let mut app = NucleonNativeApp::default();
        app.desktop_mode_open = false;
        app.navigate_to_screen(TerminalScreen::DocumentBrowser);

        let status = app.launch_open_with_request(OpenWithLaunchRequest {
            argv: vec![
                "/bin/sh".to_string(),
                "-lc".to_string(),
                "sleep 30".to_string(),
            ],
            title: "Open With".to_string(),
            status_message: "Opened via terminal.".to_string(),
        });

        assert_eq!(status, "Opened via terminal.");
        assert!(app.terminal_pty.is_some());
        assert!(app.primary_embedded_pty_open());
        assert!(!app.primary_desktop_pty_open());
        assert_eq!(app.terminal_nav.screen, TerminalScreen::PtyApp);
        assert!(!app.desktop_mode_open);
        assert!(!app.desktop_component_pty_is_open());
        assert!(app
            .desktop_pty_state(WindowInstanceId::primary(DesktopWindow::PtyApp))
            .is_none());

        app.terminate_all_native_pty_children();
    }

    #[test]
    fn active_surface_command_launch_uses_embedded_pty_when_desktop_is_closed() {
        let _guard = session_test_guard();
        let mut app = NucleonNativeApp::default();
        app.desktop_mode_open = false;

        let argv = vec![
            "/bin/sh".to_string(),
            "-lc".to_string(),
            "sleep 30".to_string(),
        ];
        app.launch_shell_command_on_active_surface(
            "Surface Launch",
            &argv,
            TerminalScreen::DocumentBrowser,
        );

        assert!(app.primary_embedded_pty_open());
        assert!(!app.primary_desktop_pty_open());
        assert_eq!(app.terminal_nav.screen, TerminalScreen::PtyApp);

        app.terminate_all_native_pty_children();
    }

    #[test]
    fn active_surface_command_launch_uses_desktop_pty_when_desktop_is_open() {
        let _guard = session_test_guard();
        let mut app = NucleonNativeApp::default();
        app.desktop_mode_open = true;

        let argv = vec![
            "/bin/sh".to_string(),
            "-lc".to_string(),
            "sleep 30".to_string(),
        ];
        app.launch_shell_command_on_active_surface(
            "Surface Launch",
            &argv,
            TerminalScreen::DocumentBrowser,
        );

        assert!(app.primary_desktop_pty_open());
        assert!(!app.primary_embedded_pty_open());
        assert!(app.desktop_window_is_open(DesktopWindow::PtyApp));

        app.terminate_all_native_pty_children();
    }

    #[test]
    fn desktop_open_with_request_uses_desktop_pty_surface() {
        let _guard = session_test_guard();
        let mut app = NucleonNativeApp::default();
        app.desktop_mode_open = true;

        let status = app.launch_open_with_request(OpenWithLaunchRequest {
            argv: vec![
                "/bin/sh".to_string(),
                "-lc".to_string(),
                "sleep 30".to_string(),
            ],
            title: "Open With".to_string(),
            status_message: "Opened via desktop.".to_string(),
        });

        assert_eq!(status, "Opened via desktop.");
        assert!(app.terminal_pty.is_some());
        assert!(app.primary_desktop_pty_open());
        assert!(!app.primary_embedded_pty_open());
        assert!(app.desktop_window_is_open(DesktopWindow::PtyApp));

        app.terminate_all_native_pty_children();
    }

    #[test]
    fn start_system_terminal_action_opens_desktop_terminal_shell() {
        let _guard = session_test_guard();
        let mut app = NucleonNativeApp::default();

        app.run_start_system_action(desktop_start_menu::StartSystemAction::Terminal);

        assert!(app.terminal_pty.is_some());
        assert!(!app.terminal_mode.open);

        app.terminate_all_native_pty_children();
    }

    #[test]
    fn spotlight_hides_editor_result_when_builtin_visibility_is_disabled() {
        let _guard = session_test_guard();
        let mut app = NucleonNativeApp::default();
        app.settings.draft.builtin_menu_visibility.text_editor = false;
        app.spotlight_tab = 1;
        app.spotlight_query = "editor".to_string();

        app.spotlight_gather_results();

        assert!(!app
            .spotlight_results
            .iter()
            .any(|result| result.name == BUILTIN_TEXT_EDITOR_APP));
    }

    #[test]
    fn opening_closed_installer_window_clears_stale_restore_state() {
        let _guard = session_test_guard();
        let mut app = NucleonNativeApp::default();
        let state =
            app.desktop_window_state_mut(WindowInstanceId::primary(DesktopWindow::Installer));
        state.restore_pos = Some([12.0, 36.0]);
        state.restore_size = Some([800.0, 520.0]);
        state.apply_restore = true;
        state.maximized = true;
        state.minimized = true;
        state.user_resized = true;
        state.generation = 5;

        app.open_desktop_window(DesktopWindow::Installer);

        let state = app.desktop_window_state(WindowInstanceId::primary(DesktopWindow::Installer));
        assert!(app.desktop_installer.open);
        assert_eq!(state.restore_pos, None);
        assert_eq!(state.restore_size, None);
        assert!(!state.apply_restore);
        assert!(!state.maximized);
        assert!(!state.minimized);
        assert!(!state.user_resized);
        assert_ne!(state.generation, 5);
    }

    #[test]
    fn shared_desktop_window_host_tracks_position_only_and_handles_minimize() {
        let _guard = session_test_guard();
        let mut app = NucleonNativeApp::default();
        app.settings.open = true;
        let state =
            app.desktop_window_state_mut(WindowInstanceId::primary(DesktopWindow::Settings));
        state.restore_size = Some([760.0, 500.0]);
        let mut open = true;

        app.finish_desktop_window_host(
            &Context::default(),
            DesktopWindow::Settings,
            &mut open,
            false,
            Some(egui::Rect::from_min_size(
                egui::pos2(32.0, 48.0),
                egui::vec2(760.0, 500.0),
            )),
            false,
            DesktopWindowRectTracking::PositionOnly,
            DesktopHeaderAction::Minimize,
        );

        let state = app.desktop_window_state(WindowInstanceId::primary(DesktopWindow::Settings));
        assert!(open);
        assert_eq!(state.restore_pos, Some([32.0, 48.0]));
        assert_eq!(state.restore_size, Some([760.0, 500.0]));
        assert!(app.desktop_window_is_minimized(DesktopWindow::Settings));
    }

    #[test]
    fn settings_window_tracks_position_without_replaying_restore_size() {
        let _guard = session_test_guard();
        let mut app = NucleonNativeApp::default();
        app.settings.open = true;
        app.settings.panel = NativeSettingsPanel::General;
        let state =
            app.desktop_window_state_mut(WindowInstanceId::primary(DesktopWindow::Settings));
        state.restore_pos = Some([32.0, 48.0]);
        state.restore_size = Some([1600.0, 1200.0]);
        state.apply_restore = true;

        let ctx = Context::default();
        let raw_input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1280.0, 720.0),
            )),
            ..Default::default()
        };

        let _ = ctx.run(raw_input, |ctx| {
            app.draw_settings(ctx);
        });

        let state = app.desktop_window_state(WindowInstanceId::primary(DesktopWindow::Settings));
        assert_eq!(state.restore_size, Some([1600.0, 1200.0]));
        assert!(!state.apply_restore);
        assert_eq!(state.restore_pos, Some([32.0, 48.0]));
    }

    #[test]
    fn applications_window_does_not_grow_with_long_catalog_lists() {
        let _guard = session_test_guard();
        let mut app = NucleonNativeApp::default();
        app.desktop_mode_open = true;
        app.applications.open = true;
        let sections = DesktopApplicationsSections {
            builtins: (0..80)
                .map(|idx| nucleon_native_programs_app::DesktopProgramEntry {
                    label: format!("Builtin App {idx}"),
                    action: nucleon_native_programs_app::DesktopApplicationsAction::OpenFileManager,
                })
                .collect(),
            configured: (0..80)
                .map(|idx| nucleon_native_programs_app::DesktopProgramEntry {
                    label: format!("Configured App {idx}"),
                    action: nucleon_native_programs_app::DesktopApplicationsAction::OpenFileManager,
                })
                .collect(),
        };
        app.desktop_applications_sections_cache = Some(DesktopApplicationsSectionsCache {
            show_file_manager: true,
            show_text_editor: true,
            sections: Arc::new(sections),
        });

        let ctx = Context::default();
        let raw_input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1280.0, 720.0),
            )),
            ..Default::default()
        };

        let _ = ctx.run(raw_input, |ctx| {
            app.draw_applications(ctx);
        });

        let state =
            app.desktop_window_state(WindowInstanceId::primary(DesktopWindow::Applications));
        let default_size = NucleonNativeApp::desktop_default_window_size(DesktopWindow::Applications);
        let restore_size = state.restore_size.expect("applications restore size");
        assert!(restore_size[1] <= default_size.y + 1.0);
    }

    #[test]
    fn editor_window_does_not_grow_with_many_lines() {
        let _guard = session_test_guard();
        let mut app = NucleonNativeApp::default();
        app.desktop_mode_open = true;
        app.editor.open = true;
        app.editor.text = (0..200)
            .map(|idx| format!("line {idx}"))
            .collect::<Vec<_>>()
            .join("\n");

        let ctx = Context::default();
        let raw_input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1280.0, 720.0),
            )),
            ..Default::default()
        };

        let _ = ctx.run(raw_input, |ctx| {
            app.draw_editor(ctx);
        });

        let state = app.desktop_window_state(WindowInstanceId::primary(DesktopWindow::Editor));
        let default_size = NucleonNativeApp::desktop_default_window_size(DesktopWindow::Editor);
        let restore_size = state.restore_size.expect("editor restore size");
        assert!(restore_size[1] <= default_size.y + 1.0);
    }

    #[test]
    fn installer_window_tracks_position_without_replaying_restore_size() {
        let _guard = session_test_guard();
        let mut app = NucleonNativeApp::default();
        app.desktop_installer.open = true;
        app.desktop_installer.view = DesktopInstallerView::SearchResults;
        app.desktop_installer.search_query = "apps".to_string();
        app.desktop_installer.search_results = (0..80)
            .map(|idx| nucleon_native_installer_app::SearchResult {
                raw: format!("pkg-{idx}"),
                pkg: format!("pkg-{idx}"),
                description: Some("description".to_string()),
                installed: false,
            })
            .collect();
        let state =
            app.desktop_window_state_mut(WindowInstanceId::primary(DesktopWindow::Installer));
        state.restore_pos = Some([24.0, 48.0]);
        state.restore_size = Some([1200.0, 1600.0]);
        state.apply_restore = true;

        let ctx = Context::default();
        let raw_input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1280.0, 720.0),
            )),
            ..Default::default()
        };

        let _ = ctx.run(raw_input, |ctx| {
            app.draw_installer(ctx);
        });

        let state = app.desktop_window_state(WindowInstanceId::primary(DesktopWindow::Installer));
        assert_eq!(state.restore_size, Some([1200.0, 1600.0]));
        assert!(!state.apply_restore);
        assert_eq!(state.restore_pos, Some([24.0, 48.0]));
    }

    #[test]
    fn closing_desktop_overlays_clears_start_and_spotlight() {
        let _guard = session_test_guard();

        let mut app = NucleonNativeApp::default();
        app.start_open = true;
        app.start_open_submenu = Some(StartSubmenu::System);
        app.start_open_leaf = Some(StartLeaf::Applications);
        app.spotlight_open = true;

        app.close_desktop_overlays();

        assert!(!app.start_open);
        assert_eq!(app.start_open_submenu, None);
        assert_eq!(app.start_open_leaf, None);
        assert!(!app.spotlight_open);
    }

    #[test]
    fn activating_open_window_closes_desktop_overlays() {
        let _guard = session_test_guard();

        let mut app = NucleonNativeApp::default();
        app.start_open = true;
        app.spotlight_open = true;
        app.desktop_active_window = Some(WindowInstanceId::primary(DesktopWindow::FileManager));
        app.file_manager.open = true;

        app.apply_desktop_menu_action(
            &Context::default(),
            &DesktopMenuAction::ActivateDesktopWindow(WindowInstanceId::primary(
                DesktopWindow::FileManager,
            )),
        );

        assert_eq!(
            app.desktop_active_window,
            Some(WindowInstanceId::primary(DesktopWindow::FileManager))
        );
        assert!(!app.start_open);
        assert!(!app.spotlight_open);
    }

    #[test]
    fn activating_taskbar_for_active_window_minimizes_it() {
        let _guard = session_test_guard();

        let mut app = NucleonNativeApp::default();
        app.file_manager.open = true;
        app.desktop_active_window = Some(WindowInstanceId::primary(DesktopWindow::FileManager));

        app.apply_desktop_menu_action(
            &Context::default(),
            &DesktopMenuAction::ActivateTaskbarWindow(WindowInstanceId::primary(
                DesktopWindow::FileManager,
            )),
        );

        assert!(app.desktop_window_is_minimized(DesktopWindow::FileManager));
    }

    #[test]
    fn activating_taskbar_for_minimized_window_restores_it() {
        let _guard = session_test_guard();

        let mut app = NucleonNativeApp::default();
        app.file_manager.open = true;
        app.start_open = true;
        app.spotlight_open = true;
        app.set_desktop_window_minimized(DesktopWindow::FileManager, true);

        app.apply_desktop_menu_action(
            &Context::default(),
            &DesktopMenuAction::ActivateTaskbarWindow(WindowInstanceId::primary(
                DesktopWindow::FileManager,
            )),
        );

        assert!(!app.desktop_window_is_minimized(DesktopWindow::FileManager));
        assert_eq!(
            app.desktop_active_window,
            Some(WindowInstanceId::primary(DesktopWindow::FileManager))
        );
        assert!(!app.start_open);
        assert!(!app.spotlight_open);
    }

    #[test]
    fn next_active_window_prefers_topmost_visible_window() {
        let _guard = session_test_guard();

        let mut app = NucleonNativeApp::default();
        app.file_manager.open = true;
        app.settings.open = true;
        app.applications.open = true;

        assert_eq!(
            app.first_open_desktop_window(),
            Some(WindowInstanceId::primary(DesktopWindow::Applications))
        );
    }

    #[test]
    fn minimizing_active_window_activates_next_topmost_window() {
        let _guard = session_test_guard();

        let mut app = NucleonNativeApp::default();
        app.file_manager.open = true;
        app.settings.open = true;
        app.applications.open = true;
        app.desktop_active_window = Some(WindowInstanceId::primary(DesktopWindow::Applications));

        app.set_desktop_window_minimized(DesktopWindow::Applications, true);

        assert_eq!(
            app.desktop_active_window,
            Some(WindowInstanceId::primary(DesktopWindow::Settings))
        );
    }

    #[test]
    fn activating_start_root_selection_opens_current_panel() {
        let _guard = session_test_guard();

        let mut app = NucleonNativeApp::default();
        app.open_start_menu();
        app.start_selected_root = 0;

        app.activate_start_menu_selection();

        assert_eq!(app.start_open_leaf, Some(StartLeaf::Applications));
        assert_eq!(app.start_open_submenu, None);
    }

    #[test]
    fn start_menu_left_closes_open_panel_without_closing_menu() {
        let _guard = session_test_guard();

        let mut app = NucleonNativeApp::default();
        app.open_start_menu();
        app.set_start_panel_for_root(4);

        app.close_start_menu_panel();

        assert!(app.start_open);
        assert_eq!(app.start_open_submenu, None);
        assert_eq!(app.start_open_leaf, None);
    }

    #[test]
    fn start_menu_root_navigation_clamps_to_valid_bounds() {
        let _guard = session_test_guard();

        let mut app = NucleonNativeApp::default();
        app.open_start_menu();

        app.start_menu_move_root_selection(-1);
        assert_eq!(app.start_selected_root, 0);

        app.start_menu_move_root_selection(99);
        assert_eq!(app.start_selected_root, START_ROOT_ITEMS.len() - 1);
    }

    #[test]
    fn opening_spotlight_resets_to_all_tab() {
        let _guard = session_test_guard();

        let mut app = NucleonNativeApp::default();
        app.spotlight_tab = 3;
        app.spotlight_query = "demo".to_string();

        app.open_spotlight();

        assert!(app.spotlight_open);
        assert_eq!(app.spotlight_tab, 0);
        assert!(app.spotlight_query.is_empty());
    }

    #[test]
    fn spotlight_tab_navigation_clamps_between_bounds() {
        let _guard = session_test_guard();

        let mut app = NucleonNativeApp::default();
        app.set_spotlight_tab(2);
        assert_eq!(app.spotlight_tab, 2);

        app.move_spotlight_tab(1);
        assert_eq!(app.spotlight_tab, 3);

        app.move_spotlight_tab(1);
        assert_eq!(app.spotlight_tab, 3);

        app.move_spotlight_tab(-9);
        assert_eq!(app.spotlight_tab, 0);
    }

    #[test]
    fn spotlight_file_result_reveals_target_in_file_manager() {
        let _guard = session_test_guard();
        let temp = TempDirGuard::new("spotlight_reveal");
        let file_path = temp.path.join("demo.txt");
        std::fs::write(&file_path, "demo").expect("write temp file");

        let mut app = NucleonNativeApp::default();
        app.spotlight_open = true;
        app.spotlight_query = "demo".to_string();

        app.spotlight_activate_result(&NativeSpotlightResult {
            name: "demo.txt".to_string(),
            category: NativeSpotlightCategory::File,
            path: Some(file_path.clone()),
        });

        assert!(!app.spotlight_open);
        assert!(app.file_manager.open);
        assert_eq!(app.file_manager.cwd, temp.path);
        assert_eq!(app.file_manager.selected, Some(file_path));
    }

    #[test]
    fn phase_7_builtin_themes_keep_asset_state_empty() {
        let _guard = session_test_guard();

        let (classic_path, classic_cursor_pack) =
            desktop_asset_state_from_theme_pack_id(Some("classic"));
        let (nucleon_path, nucleon_cursor_pack) =
            desktop_asset_state_from_theme_pack_id(Some("nucleon"));

        assert!(classic_path.is_none());
        assert!(classic_cursor_pack.is_none());
        assert!(nucleon_path.is_none());
        assert!(nucleon_cursor_pack.is_none());
    }

    #[test]
    fn phase_7_cursor_pack_json_loads_from_asset_root() {
        let temp = TempDirGuard::new("cursor_pack_json");
        let cursors_dir = temp.path.join("cursors");
        std::fs::create_dir_all(&cursors_dir).expect("create cursors dir");
        std::fs::write(
            cursors_dir.join("cursors.json"),
            r##"{
                "arrow": {
                    "width": 2,
                    "height": 2,
                    "hotspot_x": 1,
                    "hotspot_y": 0,
                    "mask": "#.\nO "
                },
                "ibeam": null,
                "pointing_hand": null,
                "resize_horizontal": null,
                "resize_vertical": null,
                "resize_nwse": null,
                "resize_nesw": null,
                "move_cursor": null,
                "forbidden": null,
                "wait": null
            }"##,
        )
        .expect("write cursor json");

        let cursor_pack = load_cursor_pack_from_asset_root(temp.path.as_path())
            .expect("cursor pack from asset root");
        let arrow = cursor_pack.arrow.expect("arrow override");

        assert_eq!(arrow.width, 2);
        assert_eq!(arrow.height, 2);
        assert_eq!(arrow.hotspot_x, 1);
        assert_eq!(arrow.hotspot_y, 0);
        assert_eq!(arrow.mask, "#.\nO ");
    }

    #[test]
    fn phase_7_cursor_theme_selection_resolves_theme_pack_id() {
        assert_eq!(
            desktop_cursor_theme_pack_id_for_selection(
                Some("signal-forge"),
                &DesktopCursorThemeSelection::FollowTheme,
            ),
            Some("signal-forge")
        );
        assert_eq!(
            desktop_cursor_theme_pack_id_for_selection(
                Some("signal-forge"),
                &DesktopCursorThemeSelection::Builtin,
            ),
            None
        );
        assert_eq!(
            desktop_cursor_theme_pack_id_for_selection(
                Some("signal-forge"),
                &DesktopCursorThemeSelection::ThemePack {
                    theme_pack_id: "cursor-only".to_string(),
                },
            ),
            Some("cursor-only")
        );
    }

    #[test]
    fn phase_7_sample_theme_bundle_parses() {
        let bundle_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/phase7-sample-theme");
        let manifest_raw =
            std::fs::read_to_string(bundle_root.join("manifest.json")).expect("read sample manifest");
        let theme_raw =
            std::fs::read_to_string(bundle_root.join("theme.json")).expect("read sample theme");

        let manifest: crate::platform::AddonManifest =
            serde_json::from_str(&manifest_raw).expect("parse sample manifest");
        let theme: crate::theme::ThemePack =
            serde_json::from_str(&theme_raw).expect("parse sample theme");

        assert_eq!(manifest.kind, crate::platform::AddonKind::Theme);
        assert_eq!(manifest.id.as_str(), "themes.phase7-signal-forge");
        assert_eq!(theme.id, "phase7-signal-forge");
        assert!(theme.sound_pack.path.is_none());
        assert_eq!(theme.asset_pack.as_ref().map(|pack| pack.path.as_str()), Some("assets"));
        assert!(theme.cursor_pack.is_none());
        assert!(bundle_root.join("assets/cursors/cursors.json").exists());
    }
}
