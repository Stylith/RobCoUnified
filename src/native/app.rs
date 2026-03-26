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
use super::wasm_addon_runtime::WasmHostedAddonState;
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
    configure_visuals, configure_visuals_for_settings, current_palette, palette_for_settings,
    FIXED_PTY_CELL_H, FIXED_PTY_CELL_W,
};
use super::terminal_open_with_picker;
use crate::config::SavedConnection;
#[cfg(test)]
use crate::config::{
    DesktopFileManagerSettings, DesktopIconSortMode, HackingDifficulty, OpenMode, Settings,
};
#[cfg(not(test))]
use crate::config::{DesktopFileManagerSettings, DesktopIconSortMode, HackingDifficulty, Settings};
use crate::core::auth::{AuthMethod, UserRecord};
use crate::session;
use eframe::egui::{
    self, Align2, Color32, Context, FontData, FontDefinitions, FontFamily, FontId, Id, Key, Layout,
    RichText, TextEdit, TextStyle, TextureHandle,
};
use egui_wgpu::CrtEffects;
#[cfg(not(test))]
use robcos_native_programs_app::{
    build_desktop_applications_sections, DesktopApplicationsSections,
};
#[cfg(test)]
use robcos_native_programs_app::{
    build_desktop_applications_sections, DesktopApplicationsSections, DesktopProgramRequest,
};
use robcos_native_settings_app::{
    build_desktop_settings_ui_defaults, desktop_settings_default_panel,
    desktop_settings_home_rows_with_visibility, desktop_settings_panel_enabled,
    DesktopSettingsVisibility, GuiCliProfileSlot, NativeSettingsPanel, SettingsHomeTile,
    TerminalSettingsPanel, TerminalSettingsVisibility,
};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
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
mod prompt_runtime;
mod runtime_state;
mod session_management;
mod session_runtime;
mod software_cursor;
mod terminal_dispatch;
mod terminal_runtime;
mod terminal_screens;
mod ui_helpers;
use desktop_window_mgmt::{DesktopHeaderAction, DesktopWindowState};
#[cfg(test)]
use launch_registry::{resolve_terminal_launch_target, NativeTerminalLaunch};
mod desktop_window_presenters;
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
struct NativeAppearanceKey {
    theme: String,
    custom_theme_rgb: [u8; 3],
}

struct AssetCache {
    icon_settings: TextureHandle,
    icon_file_manager: TextureHandle,
    icon_terminal: TextureHandle,
    icon_applications: TextureHandle,
    icon_installer: TextureHandle,
    icon_editor: TextureHandle,
    icon_general: Option<TextureHandle>,
    icon_appearance: Option<TextureHandle>,
    icon_default_apps: Option<TextureHandle>,
    icon_connections: TextureHandle,
    icon_cli_profiles: Option<TextureHandle>,
    icon_edit_menus: Option<TextureHandle>,
    icon_user_management: Option<TextureHandle>,
    icon_about: Option<TextureHandle>,
    icon_folder: Option<TextureHandle>,
    icon_folder_open: Option<TextureHandle>,
    icon_file: Option<TextureHandle>,
    icon_text: Option<TextureHandle>,
    icon_image: Option<TextureHandle>,
    icon_audio: Option<TextureHandle>,
    icon_video: Option<TextureHandle>,
    icon_archive: Option<TextureHandle>,
    icon_app: Option<TextureHandle>,
    icon_shortcut_badge: Option<TextureHandle>,
    icon_gaming: Option<TextureHandle>,
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

// Note: show_nuke_codes was removed — Nuke Codes is now a dynamic addon

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
const TERMINAL_HEADER_START_ROW: usize = 0;
const TERMINAL_SEPARATOR_TOP_ROW: usize = 3;
const TERMINAL_TITLE_ROW: usize = 4;
const TERMINAL_SEPARATOR_BOTTOM_ROW: usize = 5;
const TERMINAL_SUBTITLE_ROW: usize = 7;
const TERMINAL_MENU_START_ROW: usize = 9;
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

fn terminal_layout_for_scale(_scale: f32) -> TerminalLayout {
    TerminalLayout {
        cols: TERMINAL_SCREEN_COLS,
        rows: TERMINAL_SCREEN_ROWS,
        content_col: TERMINAL_CONTENT_COL,
        header_start_row: TERMINAL_HEADER_START_ROW,
        separator_top_row: TERMINAL_SEPARATOR_TOP_ROW,
        title_row: TERMINAL_TITLE_ROW,
        separator_bottom_row: TERMINAL_SEPARATOR_BOTTOM_ROW,
        subtitle_row: TERMINAL_SUBTITLE_ROW,
        menu_start_row: TERMINAL_MENU_START_ROW,
        status_row: TERMINAL_STATUS_ROW,
        status_row_alt: TERMINAL_STATUS_ROW_ALT,
    }
}

fn retro_footer_height() -> f32 {
    31.0
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
    apply_native_display_effects_for_settings(&crate::config::get_settings());
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

fn apply_native_appearance_for_settings(ctx: &Context, settings: &Settings) {
    configure_visuals_for_settings(ctx, settings);
    apply_native_text_style(ctx);
}

fn crt_theme_tint(settings: &Settings) -> [f32; 3] {
    let [r, g, b, _] = palette_for_settings(settings).fg.to_array();
    [r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0]
}

fn apply_native_display_effects_for_settings(settings: &Settings) {
    let display_effects = &settings.display_effects;
    if !display_effects.enabled {
        egui_wgpu::set_crt_effects(None);
        egui_winit::set_crt_pointer_curve(None);
        return;
    }
    let theme_tint = crt_theme_tint(settings);
    egui_winit::set_crt_pointer_curve(
        (display_effects.curvature > 0.0).then_some(display_effects.curvature),
    );
    egui_wgpu::set_crt_effects(Some(CrtEffects {
        curvature: display_effects.curvature,
        scanlines: display_effects.scanlines,
        glow: display_effects.glow,
        bloom: display_effects.bloom,
        vignette: display_effects.vignette,
        noise: display_effects.noise,
        flicker: display_effects.flicker,
        jitter: display_effects.jitter,
        burn_in: display_effects.burn_in,
        glow_line: display_effects.glow_line,
        glow_line_speed: display_effects.glow_line_speed,
        brightness: display_effects.brightness,
        contrast: display_effects.contrast,
        phosphor_softness: display_effects.phosphor_softness,
        theme_tint,
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

pub struct RobcoNativeApp {
    login: TerminalLoginState,
    session: Option<SessionState>,
    file_manager: NativeFileManagerState,
    editor: EditorWindow,
    settings: SettingsWindow,
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
    context_menu_action: Option<ContextMenuAction>,
    shell_status: String,
    desktop_selected_icon: Option<DesktopIconSelection>,
    desktop_item_properties: Option<DesktopItemPropertiesState>,
    shortcut_properties: Option<ShortcutPropertiesState>,
    start_menu_rename: Option<StartMenuRenameState>,
    picking_icon_for_shortcut: Option<usize>,
    picking_wallpaper: bool,
    shortcut_icon_cache: HashMap<String, egui::TextureHandle>,
    shortcut_icon_missing: HashSet<String>,
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
    last_native_appearance: Option<NativeAppearanceKey>,
    last_settings_sync_check: Instant,
    last_settings_file_mtime: Option<SystemTime>,
    startup_profile_session_logged: bool,
    startup_profile_desktop_logged: bool,
    repaint_trace_last_pass: u64,
    appearance_tab: u8, // 0=Background, 1=Display, 2=Colors, 3=Icons, 4=Terminal
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
    // IPC receiver for messages from standalone apps
    ipc: super::ipc::IpcReceiver,
}

pub(super) struct ParkedSessionState {
    file_manager: NativeFileManagerState,
    editor: EditorWindow,
    settings: SettingsWindow,
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
}

/// App-specific state for a secondary (non-primary) window instance.
pub(super) enum SecondaryWindowApp {
    FileManager {
        state: NativeFileManagerState,
        runtime: FileManagerEditRuntime,
    },
    Editor(EditorWindow),
    Pty(Option<NativePtyState>),
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
        }
    }
}

impl Default for RobcoNativeApp {
    fn default() -> Self {
        restore_current_user_from_last_session();
        session::clear_sessions();
        session::take_switch_request();
        let settings_draft = load_settings_snapshot();
        let live_desktop_file_manager_settings = settings_draft.desktop_file_manager.clone();
        let live_hacking_difficulty = settings_draft.hacking_difficulty;
        let settings_ui_defaults = build_desktop_settings_ui_defaults(&settings_draft, None);
        let terminal_defaults = terminal_runtime_defaults();
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
            context_menu_action: None,
            shell_status: String::new(),
            desktop_selected_icon: None,
            desktop_item_properties: None,
            shortcut_properties: None,
            start_menu_rename: None,
            picking_icon_for_shortcut: None,
            picking_wallpaper: false,
            shortcut_icon_cache: HashMap::new(),
            shortcut_icon_missing: HashSet::new(),
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
            last_native_appearance: None,
            last_settings_sync_check: Instant::now(),
            last_settings_file_mtime: Self::current_settings_file_mtime(),
            startup_profile_session_logged: false,
            startup_profile_desktop_logged: false,
            repaint_trace_last_pass: 0,
            appearance_tab: 0,
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
            ipc: super::ipc::start_listener(),
        };
        crate::config::spawn_addon_repository_index_refresh();
        app.maybe_apply_profile_autologin();
        app
    }
}

impl RobcoNativeApp {
    fn sync_native_display_effects(&self) {
        apply_native_display_effects_for_settings(&self.settings.draft);
    }

    fn sync_native_cursor_mode(&self) {
        let cursor_mode = if self.desktop_mode_open && self.settings.draft.desktop_show_cursor {
            egui_winit::AppCursorMode::Software
        } else {
            egui_winit::AppCursorMode::Hidden
        };
        egui_winit::set_app_cursor_mode(cursor_mode);
    }

    fn sync_native_appearance(&mut self, ctx: &Context) {
        let key = NativeAppearanceKey {
            theme: self.settings.draft.theme.clone(),
            custom_theme_rgb: self.settings.draft.custom_theme_rgb,
        };
        if self.last_native_appearance.as_ref() == Some(&key) {
            return;
        }
        apply_native_appearance_for_settings(ctx, &self.settings.draft);
        self.last_native_appearance = Some(key);
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
        terminal_layout_for_scale(self.settings.draft.native_ui_scale)
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

impl eframe::App for RobcoNativeApp {
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
        LOCK.get_or_init(|| Mutex::new(()))
            .lock()
            .expect("native app session test lock")
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
                "robco_native_{prefix}_{}_{}",
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

    fn set_runtime_marker(app: &mut RobcoNativeApp, screen: TerminalScreen, idx: usize, tag: &str) {
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

        let mut app = RobcoNativeApp::default();
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
        app.editor.ui = robcos_native_editor_app::EditorUiState::default();
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

        let mut app = RobcoNativeApp::default();
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

        let mut app = RobcoNativeApp::default();
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

            let mut app = RobcoNativeApp::default();
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

        let mut app = RobcoNativeApp::default();
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

        let mut app = RobcoNativeApp::default();
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

        let mut app = RobcoNativeApp::default();
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

        let mut app = RobcoNativeApp::default();
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
            RobcoNativeApp::truncate_file_manager_label(
                "Screenshot 2026-03-18 at 17.37.56.png",
                16,
            ),
            "Screens...56.png"
        );
    }

    #[test]
    fn desktop_icon_label_lines_compact_long_file_names() {
        assert_eq!(
            RobcoNativeApp::desktop_icon_label_lines("Screenshot 2026-03-18 at 17.37.56.png"),
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

        let mut app = RobcoNativeApp::default();
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

        let mut app = RobcoNativeApp::default();
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

        let mut app = RobcoNativeApp::default();
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
        let wallpaper = PathBuf::from("/tmp/robco-wallpaper.png");
        let wallpaper_str = wallpaper.display().to_string();

        let mut app = RobcoNativeApp::default();
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

        let mut app = RobcoNativeApp::default();
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

        let mut app = RobcoNativeApp::default();
        let texture = app
            .file_manager_preview_texture(&Context::default(), &row)
            .expect("preview texture");

        assert!(texture.size()[0] <= 192);
        assert!(texture.size()[1] <= 192);
        assert_eq!(
            app.file_manager_preview_loaded_for,
            png_path.to_string_lossy().to_string()
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

        let mut app = RobcoNativeApp::default();
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

        let mut app = RobcoNativeApp::default();
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

        let mut app = RobcoNativeApp::default();
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

        let mut app = RobcoNativeApp::default();
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

        let mut app = RobcoNativeApp::default();
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

        let mut app = RobcoNativeApp::default();
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

        let mut app = RobcoNativeApp::default();
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

        let mut app = RobcoNativeApp::default();
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

        let mut app = RobcoNativeApp::default();
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

        let mut app = RobcoNativeApp::default();
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

        let mut app = RobcoNativeApp::default();
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

        let mut app = RobcoNativeApp::default();
        app.open_desktop_surface_path(file_path.clone());

        assert!(app.editor.open);
        assert!(app.desktop_window_is_open(DesktopWindow::Editor));
        assert_eq!(app.editor.path, Some(file_path));
        assert_eq!(app.editor.text, "surface text");
    }

    #[test]
    fn opening_start_menu_closes_spotlight() {
        let _guard = session_test_guard();

        let mut app = RobcoNativeApp::default();
        app.spotlight_open = true;
        app.spotlight_query = "demo".to_string();

        app.open_start_menu();

        assert!(app.start_open);
        assert!(!app.spotlight_open);
    }

    #[test]
    fn opening_desktop_window_closes_spotlight_in_desktop_mode() {
        let _guard = session_test_guard();

        let mut app = RobcoNativeApp::default();
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

        let mut app = RobcoNativeApp::default();
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

        let mut app = RobcoNativeApp::default();
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
        let mut app = RobcoNativeApp::default();
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
        let mut app = RobcoNativeApp::default();
        app.context_menu_action = Some(ContextMenuAction::ChangeAppearance);

        app.dispatch_context_menu_action(&Context::default());

        assert!(app.settings.open);
        assert_eq!(app.settings.panel, NativeSettingsPanel::Appearance);
    }

    #[test]
    fn settings_launch_target_with_panel_payload_opens_requested_panel() {
        let mut app = RobcoNativeApp::default();

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
        let mut app = RobcoNativeApp::default();

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
        let panel = RobcoNativeApp::coerce_desktop_settings_panel_for_visibility(
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
        let mut app = RobcoNativeApp::default();

        app.launch_file_manager_via_registry();

        assert!(app.file_manager.open);
        assert!(app.desktop_window_is_open(DesktopWindow::FileManager));
    }

    #[test]
    fn editor_launch_target_opens_editor_window() {
        let mut app = RobcoNativeApp::default();

        app.launch_editor_via_registry();

        assert!(app.editor.open);
        assert!(app.desktop_window_is_open(DesktopWindow::Editor));
    }

    #[test]
    fn terminal_launch_target_opens_terminal_window() {
        let mut app = RobcoNativeApp::default();

        app.execute_desktop_shell_action(DesktopShellAction::LaunchByTarget(
            launch_registry::terminal_launch_target(),
        ));

        assert!(app.terminal_mode.open);
        assert!(app.desktop_window_is_open(DesktopWindow::TerminalMode));
    }

    #[test]
    fn programs_launch_target_opens_applications_window() {
        let mut app = RobcoNativeApp::default();

        app.execute_desktop_shell_action(DesktopShellAction::LaunchByTarget(
            launch_registry::programs_launch_target(),
        ));

        assert!(app.applications.open);
        assert!(app.desktop_window_is_open(DesktopWindow::Applications));
    }

    #[test]
    fn connections_launch_target_opens_settings_connections_panel() {
        let mut app = RobcoNativeApp::default();

        app.execute_desktop_shell_action(DesktopShellAction::LaunchByTarget(
            launch_registry::connections_launch_target(),
        ));

        assert!(app.settings.open);
        assert!(app.desktop_window_is_open(DesktopWindow::Settings));
        assert_eq!(app.settings.panel, NativeSettingsPanel::Connections);
    }

    #[test]
    fn desktop_menu_open_settings_uses_registry_launch() {
        let mut app = RobcoNativeApp::default();

        app.apply_desktop_menu_action(&Context::default(), &DesktopMenuAction::OpenSettings);

        assert!(app.settings.open);
        assert!(app.desktop_window_is_open(DesktopWindow::Settings));
        assert_eq!(app.settings.panel, desktop_settings_default_panel());
    }

    #[test]
    fn desktop_menu_open_file_manager_uses_registry_launch() {
        let mut app = RobcoNativeApp::default();

        app.apply_desktop_menu_action(&Context::default(), &DesktopMenuAction::OpenFileManager);

        assert!(app.file_manager.open);
        assert!(app.desktop_window_is_open(DesktopWindow::FileManager));
    }

    #[test]
    fn start_menu_program_installer_uses_registry_launch() {
        let mut app = RobcoNativeApp::default();
        app.start_open = true;

        app.run_start_system_action(desktop_start_menu::StartSystemAction::ProgramInstaller);

        assert!(!app.start_open);
        assert!(app.desktop_installer.open);
        assert!(app.desktop_window_is_open(DesktopWindow::Installer));
    }

    #[test]
    fn start_menu_connections_uses_registry_launch() {
        let mut app = RobcoNativeApp::default();
        app.start_open = true;

        app.run_start_system_action(desktop_start_menu::StartSystemAction::Connections);

        assert!(!app.start_open);
        assert!(app.settings.open);
        assert!(app.desktop_window_is_open(DesktopWindow::Settings));
        assert_eq!(app.settings.panel, NativeSettingsPanel::Connections);
    }

    #[test]
    fn generic_context_menu_open_settings_uses_registry_launch() {
        let mut app = RobcoNativeApp::default();
        app.context_menu_action = Some(ContextMenuAction::OpenSettings);

        app.dispatch_context_menu_action(&Context::default());

        assert!(app.settings.open);
        assert!(app.desktop_window_is_open(DesktopWindow::Settings));
        assert_eq!(app.settings.panel, desktop_settings_default_panel());
    }

    #[test]
    fn desktop_program_request_open_file_manager_uses_registry_launch() {
        let mut app = RobcoNativeApp::default();

        app.apply_desktop_program_request(DesktopProgramRequest::OpenFileManager);

        assert!(app.file_manager.open);
        assert!(app.desktop_window_is_open(DesktopWindow::FileManager));
    }

    #[test]
    fn desktop_program_request_open_text_editor_uses_registry_launch() {
        let mut app = RobcoNativeApp::default();

        app.apply_desktop_program_request(DesktopProgramRequest::OpenTextEditor {
            close_window: true,
        });

        assert!(app.editor.open);
        assert!(app.desktop_window_is_open(DesktopWindow::Editor));
    }

    #[test]
    fn open_text_editor_action_uses_registry_launch() {
        let mut app = RobcoNativeApp::default();

        app.execute_desktop_shell_action(DesktopShellAction::LaunchByTarget(
            launch_registry::editor_launch_target(),
        ));

        assert!(app.editor.open);
        assert!(app.desktop_window_is_open(DesktopWindow::Editor));
    }


    #[test]
    fn spotlight_terminal_result_uses_registry_launch() {
        let mut app = RobcoNativeApp::default();
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

        let mut app = RobcoNativeApp::default();
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

        let mut app = RobcoNativeApp::default();
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
        let mut app = RobcoNativeApp::default();
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
        let mut app = RobcoNativeApp::default();
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
        let mut app = RobcoNativeApp::default();
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
        let mut app = RobcoNativeApp::default();
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
        let mut app = RobcoNativeApp::default();

        app.run_start_system_action(desktop_start_menu::StartSystemAction::Terminal);

        assert!(app.terminal_pty.is_some());
        assert!(!app.terminal_mode.open);

        app.terminate_all_native_pty_children();
    }

    #[test]
    fn spotlight_hides_editor_result_when_builtin_visibility_is_disabled() {
        let mut app = RobcoNativeApp::default();
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
        let mut app = RobcoNativeApp::default();
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
        let mut app = RobcoNativeApp::default();
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
        let mut app = RobcoNativeApp::default();
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
        let mut app = RobcoNativeApp::default();
        app.desktop_mode_open = true;
        app.applications.open = true;
        let sections = DesktopApplicationsSections {
            builtins: (0..80)
                .map(|idx| robcos_native_programs_app::DesktopProgramEntry {
                    label: format!("Builtin App {idx}"),
                    action: robcos_native_programs_app::DesktopApplicationsAction::OpenFileManager,
                })
                .collect(),
            configured: (0..80)
                .map(|idx| robcos_native_programs_app::DesktopProgramEntry {
                    label: format!("Configured App {idx}"),
                    action: robcos_native_programs_app::DesktopApplicationsAction::OpenFileManager,
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
        let default_size = RobcoNativeApp::desktop_default_window_size(DesktopWindow::Applications);
        let restore_size = state.restore_size.expect("applications restore size");
        assert!(restore_size[1] <= default_size.y + 1.0);
    }

    #[test]
    fn editor_window_does_not_grow_with_many_lines() {
        let mut app = RobcoNativeApp::default();
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
        let default_size = RobcoNativeApp::desktop_default_window_size(DesktopWindow::Editor);
        let restore_size = state.restore_size.expect("editor restore size");
        assert!(restore_size[1] <= default_size.y + 1.0);
    }

    #[test]
    fn installer_window_tracks_position_without_replaying_restore_size() {
        let mut app = RobcoNativeApp::default();
        app.desktop_installer.open = true;
        app.desktop_installer.view = DesktopInstallerView::SearchResults;
        app.desktop_installer.search_query = "apps".to_string();
        app.desktop_installer.search_results = (0..80)
            .map(|idx| robcos_native_installer_app::SearchResult {
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

        let mut app = RobcoNativeApp::default();
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

        let mut app = RobcoNativeApp::default();
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

        let mut app = RobcoNativeApp::default();
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

        let mut app = RobcoNativeApp::default();
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

        let mut app = RobcoNativeApp::default();
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

        let mut app = RobcoNativeApp::default();
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

        let mut app = RobcoNativeApp::default();
        app.open_start_menu();
        app.start_selected_root = 0;

        app.activate_start_menu_selection();

        assert_eq!(app.start_open_leaf, Some(StartLeaf::Applications));
        assert_eq!(app.start_open_submenu, None);
    }

    #[test]
    fn start_menu_left_closes_open_panel_without_closing_menu() {
        let _guard = session_test_guard();

        let mut app = RobcoNativeApp::default();
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

        let mut app = RobcoNativeApp::default();
        app.open_start_menu();

        app.start_menu_move_root_selection(-1);
        assert_eq!(app.start_selected_root, 0);

        app.start_menu_move_root_selection(99);
        assert_eq!(app.start_selected_root, START_ROOT_ITEMS.len() - 1);
    }

    #[test]
    fn opening_spotlight_resets_to_all_tab() {
        let _guard = session_test_guard();

        let mut app = RobcoNativeApp::default();
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

        let mut app = RobcoNativeApp::default();
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

        let mut app = RobcoNativeApp::default();
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

}
