use super::background::{BackgroundResult, BackgroundTasks};
use super::connections_screen::TerminalConnectionsState;
use super::data::{home_dir_fallback, logs_dir, save_text_file, word_processor_dir};
use super::desktop_app::{DesktopShellAction, DesktopWindow, WindowInstanceId};
#[cfg(test)]
use super::desktop_app::DesktopMenuAction;
use super::desktop_connections_service::{
    connections_macos_disabled, connections_macos_disabled_hint,
    saved_connections_for_kind, DiscoveredConnection,
};
use super::desktop_documents_service::{
    add_document_category as add_desktop_document_category,
    delete_document_category as delete_desktop_document_category, document_category_names, rename_document_category as rename_desktop_document_category,
};
use super::desktop_file_service::{
    load_text_document, open_directory_location, reveal_path_location, FileManagerLocation,
};
use super::desktop_launcher_service::{
    add_catalog_entry, catalog_names, delete_catalog_entry, parse_catalog_command_line,
    rename_catalog_entry, resolve_catalog_launch, ProgramCatalog,
};
use super::desktop_search_service::{NativeSpotlightResult, NativeStartLeafAction};
#[cfg(test)]
use super::desktop_search_service::NativeSpotlightCategory;
use super::desktop_session_service::{
    authenticate_login, bind_login_identity, clear_all_sessions as clear_native_sessions,
    has_pending_session_switch as has_native_pending_session_switch,
    last_session_username, login_selection_auth_method, login_usernames as load_login_usernames,
    logout_flash_plan,
    persist_shell_snapshot as persist_native_shell_snapshot,
    restore_current_user_from_last_session,
    restore_session_plan as build_native_session_restore_plan,
    user_record as session_user_record, NativeSessionFlashPlan,
};
use super::desktop_settings_service::{
    apply_file_manager_display_settings_update as apply_desktop_file_manager_display_settings_update,
    apply_file_manager_settings_update as apply_desktop_file_manager_settings_update, load_settings_snapshot, persist_settings_draft,
    pty_force_render_mode as desktop_pty_force_render_mode,
    pty_profile_for_command as desktop_pty_profile_for_command, reload_settings_snapshot,
};
use super::desktop_shortcuts_service::{
    set_shortcut_icon as set_desktop_shortcut_icon,
};
use super::desktop_status_service::{
    clear_settings_status, clear_shell_status, saved_shell_status, shell_status, NativeStatusUpdate,
    NativeStatusValue,
};
use super::desktop_surface_service::{
    set_wallpaper_path as set_desktop_wallpaper_path, DesktopIconGridLayout, DesktopSurfaceEntry,
};
use super::desktop_user_service::{
    create_user as create_desktop_user, sorted_user_records,
    sorted_usernames, update_user_auth_method,
};
use super::donkey_kong::{
    DonkeyKongConfig, DonkeyKongGame,
    DonkeyKongTheme, BUILTIN_DONKEY_KONG_GAME,
};
use super::edit_menus_screen::{
    EditMenuTarget,
    TerminalEditMenusState,
};
use super::editor_app::{
    EditorCommand, EditorTextCommand, EditorWindow, EDITOR_APP_TITLE,
};
use super::file_manager::{FileEntryRow, FileManagerCommand, NativeFileManagerState};
use super::file_manager_app::{
    self, FileManagerCommandRequest, FileManagerDisplaySettingsUpdate, FileManagerEditRuntime,
    FileManagerOpenTarget, FileManagerPickMode, FileManagerPickerCommit, FileManagerPromptAction,
    FileManagerPromptRequest, FileManagerSelectionActivation, FileManagerSettingsUpdate,
    NativeFileManagerDragPayload, OpenWithLaunchRequest,
};
use super::file_manager_desktop::{
    self, FileManagerDesktopFooterAction, FileManagerDesktopFooterRequest,
};
use super::hacking_screen::{draw_hacking_screen, draw_locked_screen, HackingScreenEvent};
use super::installer_screen::{
    DesktopInstallerNotice, DesktopInstallerState,
    TerminalInstallerState,
};
use super::menu::{
    login_menu_rows_from_users, resolve_hacking_screen_event,
    resolve_login_selection_plan,
    resolve_terminal_flash_action, terminal_command_launch_plan, terminal_runtime_defaults, terminal_shell_launch_plan, TerminalDesktopPtyExitPlan, TerminalEmbeddedPtyExitPlan,
    TerminalFlashActionPlan, TerminalFlashPtyLaunchPlan,
    TerminalHackingUiEvent, TerminalLoginPasswordPlan, TerminalLoginScreenMode, TerminalLoginState,
    TerminalNavigationState, TerminalPtyLaunchPlan, TerminalScreen, TerminalShellSurface, TerminalUserManagementPromptPlan,
    TerminalUserPasswordFlow, UserManagementMode,
};
use super::nuke_codes_screen::{
    fetch_nuke_codes, NukeCodesView,
};
use super::prompt::{
    draw_terminal_flash, draw_terminal_flash_boxed, FlashAction,
    TerminalFlash, TerminalPrompt, TerminalPromptAction, TerminalPromptKind,
};
use super::prompt_flow::PromptOutcome;
use super::pty_screen::{
    handle_pty_input,
    spawn_embedded_pty_with_options, NativePtyState, TERMINAL_MODE_PTY_CELL_H,
    TERMINAL_MODE_PTY_CELL_W,
};
use super::retro_ui::{
    configure_visuals, configure_visuals_for_settings, current_palette,
    FIXED_PTY_CELL_H, FIXED_PTY_CELL_W,
};
use super::shell_screen::draw_login_screen;
use crate::config::{
    desktop_dir as robco_desktop_dir, get_current_user, global_settings_file, user_dir, DesktopFileManagerSettings, DesktopIconSortMode,
    HackingDifficulty, NativeStartupWindowMode, OpenMode, Settings,
};
use crate::config::{ConnectionKind, SavedConnection};
use crate::core::auth::{AuthMethod, UserRecord};
use crate::session;
use anyhow::Result;
use chrono::Local;
use eframe::egui::{
    self, Align2, Color32, Context, FontData, FontDefinitions, FontFamily, FontId, Id, Key, Layout, RichText, TextEdit, TextStyle, TextureHandle,
};
use robcos_native_file_manager_app::FileManagerAction;
use robcos_native_programs_app::{
    build_desktop_applications_sections,
    resolve_desktop_games_request, DesktopApplicationsSections,
    DesktopProgramRequest,
};
use robcos_native_settings_app::{
    build_desktop_settings_ui_defaults, desktop_settings_default_panel,
    desktop_settings_home_rows, GuiCliProfileSlot,
    NativeSettingsPanel, SettingsHomeTile, TerminalSettingsPanel,
};
use std::collections::{HashMap, HashSet};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::SystemTime;
use std::time::{Duration, Instant};

mod desktop_installer_ui;
mod desktop_menu_bar;
mod desktop_spotlight;
mod desktop_start_menu;
mod desktop_surface;
mod desktop_taskbar;
mod desktop_window_mgmt;
mod session_management;
mod terminal_dispatch;
mod terminal_screens;
use desktop_window_mgmt::{
    DesktopHeaderAction, DesktopWindowState,
};
mod desktop_window_presenters;
mod file_manager_desktop_presenter;
mod settings_panels;

use desktop_start_menu::{StartLeaf, StartSubmenu};
#[cfg(test)]
use desktop_start_menu::START_ROOT_ITEMS;
#[cfg(test)]
use super::editor_app::EditorTextAlign;
#[cfg(test)]
use crate::config::CUSTOM_THEME_NAME;
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

#[derive(Debug, Default, Clone)]
struct DonkeyKongWindow {
    open: bool,
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
    icon_nuke_codes: TextureHandle,
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
    show_text_editor: bool,
    show_nuke_codes: bool,
    sections: Arc<DesktopApplicationsSections>,
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


pub(super) const BUILTIN_NUKE_CODES_APP: &str = "Nuke Codes";
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
    if let Some(home) = dirs::home_dir() {
        candidates.push(home.join("Library/Fonts/Sysfixed.ttf"));
        candidates.push(home.join("Library/Fonts/sysfixed.ttf"));
    }
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
    donkey_kong_window: DonkeyKongWindow,
    donkey_kong: Option<DonkeyKongGame>,
    desktop_nuke_codes_open: bool,
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
    terminal_nuke_codes: NukeCodesView,
    terminal_pty: Option<NativePtyState>,
    terminal_installer: TerminalInstallerState,
    terminal_edit_menus: TerminalEditMenusState,
    terminal_connections: TerminalConnectionsState,
    terminal_prompt: Option<TerminalPrompt>,
    terminal_flash: Option<TerminalFlash>,
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
    desktop_icon_layout_cache: Option<DesktopIconLayoutCache>,
    desktop_surface_entries_cache: Option<DesktopSurfaceEntriesCache>,
    settings_home_rows_cache_admin: Option<Arc<Vec<Vec<SettingsHomeTile>>>>,
    settings_home_rows_cache_standard: Option<Arc<Vec<Vec<SettingsHomeTile>>>>,
    desktop_applications_sections_cache: Option<DesktopApplicationsSectionsCache>,
    edit_menu_entries_cache: EditMenuEntriesCache,
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
    appearance_tab: u8, // 0=Background, 1=Colors, 2=Icons, 3=Terminal
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
    // IPC receiver for messages from standalone apps
    ipc: super::ipc::IpcReceiver,
}

pub(super) struct ParkedSessionState {
    file_manager: NativeFileManagerState,
    editor: EditorWindow,
    settings: SettingsWindow,
    applications: ApplicationsWindow,
    donkey_kong_window: DonkeyKongWindow,
    donkey_kong: Option<DonkeyKongGame>,
    desktop_nuke_codes_open: bool,
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
    terminal_nuke_codes: NukeCodesView,
    terminal_pty: Option<NativePtyState>,
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
}

/// App-specific state for a secondary (non-primary) window instance.
#[derive(Clone)]
pub(super) enum SecondaryWindowApp {
    FileManager {
        state: NativeFileManagerState,
        runtime: FileManagerEditRuntime,
    },
    Editor(EditorWindow),
}

/// A secondary window instance — a second (or third, etc.) copy of an app.
#[derive(Clone)]
pub(super) struct SecondaryWindow {
    pub(super) id: WindowInstanceId,
    pub(super) app: SecondaryWindowApp,
}

impl SecondaryWindow {
    pub(super) fn is_open(&self) -> bool {
        match &self.app {
            SecondaryWindowApp::FileManager { state, .. } => state.open,
            SecondaryWindowApp::Editor(editor) => editor.open,
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
            donkey_kong_window: DonkeyKongWindow::default(),
            donkey_kong: None,
            desktop_nuke_codes_open: false,
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
            terminal_nuke_codes: NukeCodesView::default(),
            terminal_pty: None,
            terminal_installer: TerminalInstallerState::default(),
            terminal_edit_menus: TerminalEditMenusState::default(),
            terminal_connections: TerminalConnectionsState::default(),
            terminal_prompt: None,
            terminal_flash: None,
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
            ipc: super::ipc::start_listener(),
        };
        app.maybe_apply_profile_autologin();
        app
    }
}

impl RobcoNativeApp {
    fn apply_autologin_open_mode(&mut self) {
        if matches!(self.settings.draft.default_open_mode, OpenMode::Desktop) {
            self.desktop_mode_open = true;
            self.close_start_menu();
            self.sync_desktop_active_window();
        }
    }

    fn maybe_apply_profile_autologin(&mut self) {
        if self.session.is_some() {
            return;
        }
        let Some(username) = std::env::var("ROBCOS_AUTOLOGIN_USER")
            .ok()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
        else {
            return;
        };
        let Some(user) = session_user_record(&username) else {
            return;
        };
        if user.auth_method != AuthMethod::NoPassword {
            return;
        }
        bind_login_identity(&username);
        self.ensure_login_session_entry(&username);
        self.restore_for_user(&username, &user);
        self.apply_autologin_open_mode();
    }

    fn append_startup_profile_marker(marker: &str) {
        let Some(path) = std::env::var_os("ROBCOS_STARTUP_PROFILE_LOG") else {
            return;
        };
        let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
        else {
            return;
        };
        let timestamp_ms = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|duration| duration.as_millis())
            .unwrap_or(0);
        let _ = writeln!(file, "{timestamp_ms} {marker}");
    }

    fn maybe_write_startup_profile_markers(&mut self) {
        if !self.startup_profile_session_logged && self.session.is_some() {
            Self::append_startup_profile_marker("session_ready");
            self.startup_profile_session_logged = true;
        }
        if !self.startup_profile_desktop_logged && self.session.is_some() && self.desktop_mode_open
        {
            Self::append_startup_profile_marker("desktop_ready");
            self.startup_profile_desktop_logged = true;
        }
    }

    fn maybe_trace_repaint_causes(&mut self, ctx: &Context) {
        let Some(path) = std::env::var_os("ROBCOS_REPAINT_TRACE_LOG") else {
            return;
        };
        let pass = ctx.cumulative_pass_nr();
        if pass == 0 || pass == self.repaint_trace_last_pass {
            return;
        }
        self.repaint_trace_last_pass = pass;
        let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
        else {
            return;
        };
        let timestamp_ms = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|duration| duration.as_millis())
            .unwrap_or(0);
        let causes = ctx.repaint_causes();
        let cause_text = if causes.is_empty() {
            "none".to_string()
        } else {
            causes
                .into_iter()
                .map(|cause| cause.to_string())
                .collect::<Vec<_>>()
                .join(" | ")
        };
        let requested = ctx.has_requested_repaint();
        let input_summary = ctx.input(|input| {
            format!(
                "events={} pointer_delta=({:.2},{:.2}) motion={:?} latest_pos={:?}",
                input.events.len(),
                input.pointer.delta().x,
                input.pointer.delta().y,
                input.pointer.motion(),
                input.pointer.latest_pos(),
            )
        });
        let mode = if self.desktop_mode_open {
            "desktop"
        } else {
            "terminal"
        };
        let _ = writeln!(
            file,
            "{timestamp_ms} pass={pass} mode={mode} requested={requested} causes={cause_text} input={input_summary}"
        );
    }

    fn sync_runtime_settings_cache(&mut self) {
        self.live_desktop_file_manager_settings = self.settings.draft.desktop_file_manager.clone();
        self.live_hacking_difficulty = self.settings.draft.hacking_difficulty;
    }


    fn invalidate_desktop_surface_cache(&mut self) {
        self.desktop_surface_entries_cache = None;
        self.invalidate_desktop_icon_layout_cache();
        if self.file_manager.cwd == robco_desktop_dir() {
            self.file_manager.refresh_contents();
        }
    }

    fn invalidate_program_catalog_cache(&mut self) {
        self.desktop_applications_sections_cache = None;
    }

    fn invalidate_edit_menu_entries_cache(&mut self, target: EditMenuTarget) {
        match target {
            EditMenuTarget::Applications => self.edit_menu_entries_cache.applications = None,
            EditMenuTarget::Documents => self.edit_menu_entries_cache.documents = None,
            EditMenuTarget::Network => self.edit_menu_entries_cache.network = None,
            EditMenuTarget::Games => self.edit_menu_entries_cache.games = None,
        }
    }

    fn invalidate_user_cache(&mut self) {
        self.sorted_user_records_cache = None;
        self.sorted_usernames_cache = None;
    }

    fn invalidate_saved_connections_cache(&mut self) {
        self.saved_network_connections_cache = None;
        self.saved_bluetooth_connections_cache = None;
    }

    fn current_settings_file_path() -> PathBuf {
        if let Some(username) = get_current_user() {
            user_dir(&username).join("settings.json")
        } else {
            global_settings_file()
        }
    }

    fn current_settings_file_mtime() -> Option<SystemTime> {
        std::fs::metadata(Self::current_settings_file_path())
            .ok()
            .and_then(|metadata| metadata.modified().ok())
    }

    fn refresh_settings_sync_marker(&mut self) {
        self.last_settings_file_mtime = Self::current_settings_file_mtime();
        self.last_settings_sync_check = Instant::now();
    }

    fn replace_settings_draft(&mut self, draft: Settings) {
        self.settings.draft = draft;
        self.sync_runtime_settings_cache();
        self.invalidate_desktop_icon_layout_cache();
        self.invalidate_program_catalog_cache();
        self.invalidate_saved_connections_cache();
        self.refresh_settings_sync_marker();
    }

    fn process_background_results(&mut self, ctx: &Context) {
        let results = self.background.poll();
        if results.is_empty() {
            return;
        }
        for result in results {
            match result {
                BackgroundResult::NukeCodesFetched(view) => {
                    self.terminal_nuke_codes = view;
                }
                BackgroundResult::SettingsPersisted => {
                    super::ipc::notify_settings_changed();
                }
            }
        }
        ctx.request_repaint();
    }

    fn process_ipc_messages(&mut self, ctx: &Context) {
        let messages = self.ipc.poll();
        if messages.is_empty() {
            return;
        }
        for msg in messages {
            match msg {
                super::ipc::IpcMessage::SettingsChanged => {
                    let settings = reload_settings_snapshot();
                    self.replace_settings_draft(settings);
                }
                super::ipc::IpcMessage::OpenInEditor { path } => {
                    self.open_path_in_editor(std::path::PathBuf::from(path));
                }
                super::ipc::IpcMessage::RevealInFileManager { path } => {
                    self.open_file_manager_at(std::path::PathBuf::from(path));
                }
                super::ipc::IpcMessage::OpenSettings { panel } => {
                    let panel = panel.and_then(|p| {
                        super::settings_standalone::standalone_settings_panel_from_arg(&p)
                    });
                    if let Some(panel) = panel {
                        self.settings.panel = panel;
                    }
                    self.open_desktop_window(DesktopWindow::Settings);
                }
                super::ipc::IpcMessage::AppClosed { .. } | super::ipc::IpcMessage::Ping => {}
            }
        }
        ctx.request_repaint();
    }

    fn maybe_sync_settings_from_disk(&mut self, ctx: &Context) {
        const SETTINGS_SYNC_INTERVAL: Duration = Duration::from_millis(500);

        if self.settings.open || self.last_settings_sync_check.elapsed() < SETTINGS_SYNC_INTERVAL {
            return;
        }
        self.last_settings_sync_check = Instant::now();

        let current_mtime = Self::current_settings_file_mtime();
        if current_mtime == self.last_settings_file_mtime {
            return;
        }

        let previous_window_mode = self.settings.draft.native_startup_window_mode;
        let settings = reload_settings_snapshot();
        self.replace_settings_draft(settings);
        if self.settings.draft.native_startup_window_mode != previous_window_mode {
            self.apply_native_window_mode(ctx);
        }
    }

    fn saved_connections_cached(&mut self, kind: ConnectionKind) -> Arc<Vec<SavedConnection>> {
        let cache = match kind {
            ConnectionKind::Network => &mut self.saved_network_connections_cache,
            ConnectionKind::Bluetooth => &mut self.saved_bluetooth_connections_cache,
        };
        if cache.is_none() {
            *cache = Some(Arc::new(saved_connections_for_kind(kind)));
        }
        cache
            .as_ref()
            .expect("saved connections cache initialized")
            .clone()
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

    fn settings_home_rows_for_session(
        &mut self,
        is_admin: bool,
    ) -> Arc<Vec<Vec<SettingsHomeTile>>> {
        let cache = if is_admin {
            &mut self.settings_home_rows_cache_admin
        } else {
            &mut self.settings_home_rows_cache_standard
        };
        cache
            .get_or_insert_with(|| Arc::new(desktop_settings_home_rows(is_admin)))
            .clone()
    }

    fn desktop_applications_sections(&mut self) -> Arc<DesktopApplicationsSections> {
        let show_text_editor = self.settings.draft.builtin_menu_visibility.text_editor;
        let show_nuke_codes = self.settings.draft.builtin_menu_visibility.nuke_codes;
        let needs_rebuild = self
            .desktop_applications_sections_cache
            .as_ref()
            .is_none_or(|cache| {
                cache.show_text_editor != show_text_editor
                    || cache.show_nuke_codes != show_nuke_codes
            });
        if needs_rebuild {
            let configured_names = catalog_names(ProgramCatalog::Applications);
            let sections = Arc::new(build_desktop_applications_sections(
                show_text_editor,
                show_nuke_codes,
                &configured_names,
                BUILTIN_TEXT_EDITOR_APP,
                BUILTIN_NUKE_CODES_APP,
            ));
            self.desktop_applications_sections_cache = Some(DesktopApplicationsSectionsCache {
                show_text_editor,
                show_nuke_codes,
                sections,
            });
        }
        self.desktop_applications_sections_cache
            .as_ref()
            .expect("desktop applications sections cache initialized")
            .sections
            .clone()
    }

    fn edit_menu_entries_cached(&mut self, target: EditMenuTarget) -> Arc<Vec<String>> {
        let cached = match target {
            EditMenuTarget::Applications => self.edit_menu_entries_cache.applications.clone(),
            EditMenuTarget::Documents => self.edit_menu_entries_cache.documents.clone(),
            EditMenuTarget::Network => self.edit_menu_entries_cache.network.clone(),
            EditMenuTarget::Games => self.edit_menu_entries_cache.games.clone(),
        };
        if let Some(entries) = cached {
            return entries;
        }
        let entries = Arc::new(self.edit_program_entries(target));
        match target {
            EditMenuTarget::Applications => {
                self.edit_menu_entries_cache.applications = Some(entries.clone())
            }
            EditMenuTarget::Documents => {
                self.edit_menu_entries_cache.documents = Some(entries.clone())
            }
            EditMenuTarget::Network => self.edit_menu_entries_cache.network = Some(entries.clone()),
            EditMenuTarget::Games => self.edit_menu_entries_cache.games = Some(entries.clone()),
        }
        entries
    }

    fn sorted_user_records_cached(&mut self) -> Arc<Vec<(String, UserRecord)>> {
        self.sorted_user_records_cache
            .get_or_insert_with(|| Arc::new(sorted_user_records()))
            .clone()
    }

    fn sorted_usernames_cached(&mut self) -> Arc<Vec<String>> {
        self.sorted_usernames_cache
            .get_or_insert_with(|| Arc::new(sorted_usernames()))
            .clone()
    }

    fn apply_status_update(&mut self, update: NativeStatusUpdate) {
        if let Some(shell) = update.shell {
            match shell {
                NativeStatusValue::Set(message) => self.shell_status = message,
                NativeStatusValue::Clear => self.shell_status.clear(),
            }
        }
        if let Some(settings) = update.settings {
            match settings {
                NativeStatusValue::Set(message) => self.settings.status = message,
                NativeStatusValue::Clear => self.settings.status.clear(),
            }
        }
    }

    fn load_svg_icon(
        ctx: &Context,
        id: &str,
        svg_bytes: &[u8],
        size_px: Option<u32>,
    ) -> TextureHandle {
        let tree = usvg::Tree::from_data(svg_bytes, &usvg::Options::default())
            .expect("invalid SVG in src/Icons");
        let natural = tree.size().to_int_size();
        let target_size = size_px.unwrap_or(natural.width().max(natural.height()));
        let scale = target_size as f32 / natural.width().max(natural.height()) as f32;
        let width = (natural.width() as f32 * scale).round() as u32;
        let height = (natural.height() as f32 * scale).round() as u32;

        let mut pixmap = resvg::tiny_skia::Pixmap::new(width, height).expect("zero-sized SVG icon");
        resvg::render(
            &tree,
            resvg::tiny_skia::Transform::from_scale(scale, scale),
            &mut pixmap.as_mut(),
        );

        let mut rgba = Vec::with_capacity((width * height * 4) as usize);
        for pixel in pixmap.pixels() {
            rgba.extend_from_slice(&[255, 255, 255, pixel.alpha()]);
        }
        let image =
            egui::ColorImage::from_rgba_unmultiplied([width as usize, height as usize], &rgba);
        ctx.load_texture(id, image, egui::TextureOptions::LINEAR)
    }

    fn ensure_cached_svg_icon(
        slot: &mut Option<TextureHandle>,
        ctx: &Context,
        id: &str,
        svg_bytes: &[u8],
        size_px: Option<u32>,
    ) -> TextureHandle {
        slot.get_or_insert_with(|| Self::load_svg_icon(ctx, id, svg_bytes, size_px))
            .clone()
    }

    fn paint_tinted_texture(
        painter: &egui::Painter,
        texture: &TextureHandle,
        rect: egui::Rect,
        tint: Color32,
    ) {
        let uv = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
        painter.image(texture.id(), rect, uv, tint);
    }

    fn reset_desktop_settings_window(&mut self) {
        let draft = load_settings_snapshot();
        let defaults = build_desktop_settings_ui_defaults(
            &draft,
            self.session
                .as_ref()
                .map(|session| session.username.as_str()),
        );
        self.replace_settings_draft(draft);
        self.apply_status_update(clear_settings_status());
        self.settings.panel = defaults.panel;
        self.settings.default_app_custom_text_code = defaults.default_app_custom_text_code;
        self.settings.default_app_custom_ebook = defaults.default_app_custom_ebook;
        self.settings.scanned_networks.clear();
        self.settings.scanned_bluetooth.clear();
        self.settings.connection_password.clear();
        self.settings.edit_target = EditMenuTarget::Applications;
        self.settings.edit_name_input.clear();
        self.settings.edit_value_input.clear();
        self.settings.cli_profile_slot = defaults.cli_profile_slot;
        self.settings.user_create_username.clear();
        self.settings.user_create_auth = defaults.user_create_auth;
        self.settings.user_create_password.clear();
        self.settings.user_create_password_confirm.clear();
        self.settings.user_edit_password.clear();
        self.settings.user_edit_password_confirm.clear();
        self.settings.user_delete_confirm.clear();
        self.settings.user_selected = defaults.user_selected;
        self.settings.user_selected_loaded_for = defaults.user_selected_loaded_for;
        self.settings.user_edit_auth = defaults.user_edit_auth;
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

    pub(crate) fn desktop_component_file_manager_is_open(&self) -> bool {
        self.file_manager.open
    }

    pub(crate) fn desktop_component_file_manager_set_open(&mut self, open: bool) {
        self.file_manager.open = open;
    }

    pub(crate) fn desktop_component_file_manager_draw(&mut self, ctx: &Context) {
        self.draw_file_manager(ctx);
    }

    pub(crate) fn desktop_component_editor_is_open(&self) -> bool {
        self.editor.open
    }

    pub(crate) fn desktop_component_editor_set_open(&mut self, open: bool) {
        self.editor.open = open;
    }

    pub(crate) fn desktop_component_editor_draw(&mut self, ctx: &Context) {
        self.draw_editor(ctx);
    }

    pub(crate) fn desktop_component_editor_on_closed(&mut self) {
        if self.desktop_mode_open {
            self.editor.reset_for_desktop_new_document();
            self.editor.ui.reset_search();
        }
    }

    pub(crate) fn desktop_component_settings_is_open(&self) -> bool {
        self.settings.open
    }

    pub(crate) fn desktop_component_settings_set_open(&mut self, open: bool) {
        self.settings.open = open;
    }

    pub(crate) fn desktop_component_settings_draw(&mut self, ctx: &Context) {
        self.draw_settings(ctx);
    }

    pub(crate) fn desktop_component_settings_on_open(&mut self, _was_open: bool) {
        self.reset_desktop_settings_window();
        self.prime_desktop_window_defaults(DesktopWindow::Settings);
    }

    fn restore_standalone_session_identity(&mut self, session_username: Option<String>) {
        let session_username = session_username
            .and_then(|username| {
                let trimmed = username.trim().to_string();
                (!trimmed.is_empty()).then_some(trimmed)
            })
            .or_else(get_current_user)
            .or_else(last_session_username);
        if let Some(username) = session_username {
            if let Some(user) = session_user_record(&username) {
                bind_login_identity(&username);
                self.ensure_login_session_entry(&username);
                self.restore_for_user(&username, &user);
            }
        }
    }

    fn prepare_standalone_window_shell(
        &mut self,
        session_username: Option<String>,
        desktop_mode_open: bool,
    ) {
        self.restore_standalone_session_identity(session_username);
        self.desktop_window_states.clear();
        self.close_desktop_overlays();
        self.terminal_prompt = None;
        self.desktop_mode_open = desktop_mode_open;
        self.desktop_active_window = None;
        self.apply_status_update(clear_shell_status());
    }

    pub(crate) fn prepare_standalone_settings_window(
        &mut self,
        session_username: Option<String>,
        panel: Option<NativeSettingsPanel>,
    ) {
        self.prepare_standalone_window_shell(session_username, false);
        self.reset_desktop_settings_window();
        self.prime_desktop_window_defaults(DesktopWindow::Settings);
        self.settings.open = true;
        self.settings.panel = panel.unwrap_or_else(desktop_settings_default_panel);
        self.file_manager.open = false;
        self.picking_icon_for_shortcut = None;
        self.picking_wallpaper = false;
        self.desktop_active_window = Some(WindowInstanceId::primary(DesktopWindow::Settings));
        self.apply_status_update(clear_settings_status());
    }

    pub(crate) fn update_standalone_settings_window(&mut self, ctx: &Context) {
        self.process_background_results(ctx);
        self.maybe_sync_settings_from_disk(ctx);
        self.sync_native_appearance(ctx);
        self.dispatch_context_menu_action(ctx);
        if self.terminal_prompt.is_some() {
            self.handle_terminal_prompt_input(ctx);
            self.consume_terminal_prompt_keys(ctx);
        }
        let file_manager_first =
            self.active_window_kind() != Some(DesktopWindow::FileManager) || !self.file_manager.open;
        if file_manager_first {
            self.draw_file_manager(ctx);
            self.draw_settings(ctx);
        } else {
            self.draw_settings(ctx);
            self.draw_file_manager(ctx);
        }
        self.draw_terminal_prompt_overlay_global(ctx);
        if !self.settings.open && !self.file_manager.open && self.terminal_prompt.is_none() {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        ctx.request_repaint_after(Duration::from_millis(500));
    }

    pub(crate) fn prepare_standalone_editor_window(
        &mut self,
        session_username: Option<String>,
        start_path: Option<PathBuf>,
    ) {
        self.prepare_standalone_window_shell(session_username, true);
        self.file_manager.open = false;
        self.picking_icon_for_shortcut = None;
        self.picking_wallpaper = false;
        self.editor.reset_for_desktop_new_document();
        self.editor.status.clear();
        self.editor.ui.reset_search();
        self.prime_desktop_window_defaults(DesktopWindow::Editor);
        if let Some(path) = start_path {
            self.open_embedded_path_in_editor(path);
        } else {
            self.new_document();
        }
        self.desktop_active_window = Some(WindowInstanceId::primary(DesktopWindow::Editor));
    }

    pub(crate) fn update_standalone_editor_window(&mut self, ctx: &Context) {
        self.process_background_results(ctx);
        self.maybe_sync_settings_from_disk(ctx);
        self.sync_native_appearance(ctx);
        if self.terminal_prompt.is_some() {
            self.handle_terminal_prompt_input(ctx);
            self.consume_terminal_prompt_keys(ctx);
        }
        let file_manager_first =
            self.active_window_kind() != Some(DesktopWindow::FileManager) || !self.file_manager.open;
        if file_manager_first {
            self.draw_file_manager(ctx);
            self.draw_editor(ctx);
        } else {
            self.draw_editor(ctx);
            self.draw_file_manager(ctx);
        }
        self.draw_terminal_prompt_overlay_global(ctx);
        if !self.editor.open && !self.file_manager.open && self.terminal_prompt.is_none() {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        ctx.request_repaint_after(Duration::from_millis(500));
    }

    pub(crate) fn desktop_component_applications_is_open(&self) -> bool {
        self.applications.open
    }

    pub(crate) fn desktop_component_applications_set_open(&mut self, open: bool) {
        self.applications.open = open;
    }

    pub(crate) fn desktop_component_applications_draw(&mut self, ctx: &Context) {
        self.draw_applications(ctx);
    }

    pub(crate) fn prepare_standalone_applications_window(
        &mut self,
        session_username: Option<String>,
    ) {
        self.prepare_standalone_window_shell(session_username, true);
        self.applications.status.clear();
        self.prime_desktop_window_defaults(DesktopWindow::Applications);
        self.applications.open = true;
        self.desktop_active_window = Some(WindowInstanceId::primary(DesktopWindow::Applications));
    }

    pub(crate) fn update_standalone_applications_window(&mut self, ctx: &Context) {
        self.process_background_results(ctx);
        self.maybe_sync_settings_from_disk(ctx);
        self.sync_native_appearance(ctx);
        self.draw_applications(ctx);
        if !self.applications.open {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        ctx.request_repaint_after(Duration::from_millis(500));
    }

    pub(crate) fn desktop_component_donkey_kong_is_open(&self) -> bool {
        self.donkey_kong_window.open
    }

    pub(crate) fn desktop_component_donkey_kong_set_open(&mut self, open: bool) {
        self.donkey_kong_window.open = open;
    }

    pub(crate) fn desktop_component_donkey_kong_draw(&mut self, ctx: &Context) {
        self.draw_desktop_donkey_kong(ctx);
    }

    pub(crate) fn desktop_component_nuke_codes_is_open(&self) -> bool {
        self.desktop_nuke_codes_open
    }

    pub(crate) fn desktop_component_nuke_codes_set_open(&mut self, open: bool) {
        self.desktop_nuke_codes_open = open;
    }

    pub(crate) fn desktop_component_nuke_codes_draw(&mut self, ctx: &Context) {
        self.draw_nuke_codes_window(ctx);
    }

    pub(crate) fn prepare_standalone_nuke_codes_window(
        &mut self,
        session_username: Option<String>,
    ) {
        self.prepare_standalone_window_shell(session_username, true);
        self.prime_desktop_window_defaults(DesktopWindow::NukeCodes);
        self.open_desktop_nuke_codes();
        self.desktop_active_window = Some(WindowInstanceId::primary(DesktopWindow::NukeCodes));
    }

    pub(crate) fn update_standalone_nuke_codes_window(&mut self, ctx: &Context) {
        self.process_background_results(ctx);
        self.maybe_sync_settings_from_disk(ctx);
        self.sync_native_appearance(ctx);
        self.draw_nuke_codes_window(ctx);
        if !self.desktop_nuke_codes_open {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        ctx.request_repaint_after(Duration::from_millis(500));
    }

    pub(crate) fn desktop_component_installer_is_open(&self) -> bool {
        self.desktop_installer.open
    }

    pub(crate) fn desktop_component_installer_set_open(&mut self, open: bool) {
        self.desktop_installer.open = open;
    }

    pub(crate) fn desktop_component_installer_draw(&mut self, ctx: &Context) {
        self.draw_installer(ctx);
    }

    pub(crate) fn desktop_component_installer_on_open(&mut self, was_open: bool) {
        if !was_open {
            self.prime_desktop_window_defaults(DesktopWindow::Installer);
        }
    }

    pub(crate) fn prepare_standalone_installer_window(
        &mut self,
        session_username: Option<String>,
    ) {
        self.prepare_standalone_window_shell(session_username, true);
        self.prime_desktop_window_defaults(DesktopWindow::Installer);
        self.desktop_installer.open = true;
        self.desktop_active_window = Some(WindowInstanceId::primary(DesktopWindow::Installer));
    }

    pub(crate) fn update_standalone_installer_window(&mut self, ctx: &Context) {
        self.process_background_results(ctx);
        self.process_desktop_pty_input_early(ctx);
        self.maybe_sync_settings_from_disk(ctx);
        self.sync_native_appearance(ctx);
        let pty_last =
            self.active_window_kind() == Some(DesktopWindow::PtyApp) && self.terminal_pty.is_some();
        if pty_last {
            self.draw_installer(ctx);
            self.draw_desktop_pty_window(ctx);
        } else {
            self.draw_desktop_pty_window(ctx);
            self.draw_installer(ctx);
        }
        if !self.desktop_installer.open && self.terminal_pty.is_none() {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        ctx.request_repaint_after(Duration::from_millis(500));

    }

    pub(crate) fn desktop_component_terminal_mode_is_open(&self) -> bool {
        self.terminal_mode.open
    }

    pub(crate) fn desktop_component_terminal_mode_set_open(&mut self, open: bool) {
        self.terminal_mode.open = open;
    }

    pub(crate) fn desktop_component_terminal_mode_draw(&mut self, ctx: &Context) {
        self.draw_terminal_mode(ctx);
    }

    pub(crate) fn desktop_component_terminal_mode_on_open(&mut self, was_open: bool) {
        if !was_open {
            self.prime_desktop_window_defaults(DesktopWindow::TerminalMode);
        }
    }

    pub(crate) fn desktop_component_pty_is_open(&self) -> bool {
        self.terminal_pty.is_some()
    }

    pub(crate) fn desktop_component_pty_set_open(&mut self, open: bool) {
        if !open {
            if let Some(mut pty) = self.terminal_pty.take() {
                pty.session.terminate();
            }
        }
    }

    pub(crate) fn desktop_component_pty_draw(&mut self, ctx: &Context) {
        self.draw_desktop_pty_window(ctx);
    }

    pub(crate) fn desktop_component_pty_on_open(&mut self, was_open: bool) {
        if !was_open {
            self.prime_desktop_window_defaults(DesktopWindow::PtyApp);
        }
    }


    fn file_manager_home_path(&self) -> PathBuf {
        if let Some(session) = &self.session {
            word_processor_dir(&session.username)
        } else {
            home_dir_fallback()
        }
    }

    fn apply_file_manager_location(&mut self, location: FileManagerLocation) {
        self.file_manager.set_cwd(location.cwd);
        if let Some(selected) = location.selected {
            self.file_manager.select(Some(selected));
        }
        self.open_desktop_window(DesktopWindow::FileManager);
    }

    fn apply_file_manager_display_settings_update(
        &mut self,
        update: FileManagerDisplaySettingsUpdate,
    ) {
        apply_desktop_file_manager_display_settings_update(&mut self.settings.draft, update);
        self.sync_runtime_settings_cache();
        self.file_manager.ensure_selection_valid();
    }

    fn truncate_file_manager_label(text: &str, max_chars: usize) -> String {
        let total_chars = text.chars().count();
        if total_chars <= max_chars {
            return text.to_string();
        }
        if max_chars <= 3 {
            return ".".repeat(max_chars);
        }
        let suffix_budget = ((max_chars - 3) + 1) / 2;
        let mut suffix: String = text
            .chars()
            .skip(total_chars.saturating_sub(suffix_budget))
            .collect();
        if total_chars > suffix_budget && suffix.starts_with('.') {
            suffix.remove(0);
        }
        let prefix_budget = max_chars.saturating_sub(3 + suffix.chars().count());
        let prefix: String = text.chars().take(prefix_budget).collect();
        format!("{prefix}...{suffix}")
    }

    fn settings_panel_texture(
        &mut self,
        ctx: &Context,
        panel: NativeSettingsPanel,
    ) -> Option<TextureHandle> {
        let cache = self.asset_cache.as_mut()?;
        let texture = match panel {
            NativeSettingsPanel::General => cache.icon_general.get_or_insert_with(|| {
                Self::load_svg_icon(
                    ctx,
                    "icon_general",
                    include_bytes!("../Icons/pixel--home-solid.svg"),
                    Some(64),
                )
            }),
            NativeSettingsPanel::Appearance => cache.icon_appearance.get_or_insert_with(|| {
                Self::load_svg_icon(
                    ctx,
                    "icon_appearance",
                    include_bytes!("../Icons/pixel--image-solid.svg"),
                    Some(64),
                )
            }),
            NativeSettingsPanel::DefaultApps => cache.icon_default_apps.get_or_insert_with(|| {
                Self::load_svg_icon(
                    ctx,
                    "icon_default_apps",
                    include_bytes!("../Icons/pixel--external-link-solid.svg"),
                    Some(64),
                )
            }),
            NativeSettingsPanel::Connections => &mut cache.icon_connections,
            NativeSettingsPanel::CliProfiles => cache.icon_cli_profiles.get_or_insert_with(|| {
                Self::load_svg_icon(
                    ctx,
                    "icon_cli_profiles",
                    include_bytes!("../Icons/pixel--code-solid.svg"),
                    Some(64),
                )
            }),
            NativeSettingsPanel::EditMenus => cache.icon_edit_menus.get_or_insert_with(|| {
                Self::load_svg_icon(
                    ctx,
                    "icon_edit_menus",
                    include_bytes!("../Icons/pixel--bullet-list-solid.svg"),
                    Some(64),
                )
            }),
            NativeSettingsPanel::UserManagement => {
                cache.icon_user_management.get_or_insert_with(|| {
                    Self::load_svg_icon(
                        ctx,
                        "icon_user_management",
                        include_bytes!("../Icons/pixel--user-solid.svg"),
                        Some(64),
                    )
                })
            }
            NativeSettingsPanel::About => cache.icon_about.get_or_insert_with(|| {
                Self::load_svg_icon(
                    ctx,
                    "icon_about",
                    include_bytes!("../Icons/pixel--info-circle-solid.svg"),
                    Some(64),
                )
            }),
            _ => return None,
        };
        Some(texture.clone())
    }

    fn installer_games_texture(&mut self, ctx: &Context) -> Option<TextureHandle> {
        let cache = self.asset_cache.as_mut()?;
        Some(
            cache
                .icon_gaming
                .get_or_insert_with(|| {
                    Self::load_svg_icon(
                        ctx,
                        "icon_gaming",
                        include_bytes!("../Icons/pixel--gaming.svg"),
                        Some(64),
                    )
                })
                .clone(),
        )
    }

    fn file_manager_texture_for_row(
        &mut self,
        ctx: &Context,
        row: &super::file_manager::FileEntryRow,
    ) -> Option<TextureHandle> {
        let cache = self.asset_cache.as_mut()?;
        if row.is_parent_dir() {
            return Some(Self::ensure_cached_svg_icon(
                &mut cache.icon_folder_open,
                ctx,
                "icon_folder_open",
                include_bytes!("../Icons/pixel--folder-open-solid.svg"),
                Some(64),
            ));
        }
        if row.is_dir {
            return Some(Self::ensure_cached_svg_icon(
                &mut cache.icon_folder,
                ctx,
                "icon_folder",
                include_bytes!("../Icons/pixel--folder-solid.svg"),
                Some(64),
            ));
        }
        let extension = row
            .path
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        Some(match extension.as_str() {
            "txt" | "md" | "log" | "toml" | "yaml" | "yml" | "json" | "cfg" | "ini" | "conf"
            | "ron" | "rs" | "py" | "js" | "ts" | "c" | "cpp" | "h" | "hpp" | "sh" | "bash"
            | "fish" | "lua" | "rb" => Self::ensure_cached_svg_icon(
                &mut cache.icon_text,
                ctx,
                "icon_text",
                include_bytes!("../Icons/pixel--newspaper-solid.svg"),
                Some(64),
            ),
            "png" | "jpg" | "jpeg" | "gif" | "bmp" | "webp" | "svg" | "ico" => {
                Self::ensure_cached_svg_icon(
                    &mut cache.icon_image,
                    ctx,
                    "icon_image",
                    include_bytes!("../Icons/pixel--image-solid.svg"),
                    Some(64),
                )
            }
            "mp3" | "wav" | "ogg" | "flac" | "aac" | "m4a" => Self::ensure_cached_svg_icon(
                &mut cache.icon_audio,
                ctx,
                "icon_audio",
                include_bytes!("../Icons/pixel--music-solid.svg"),
                Some(64),
            ),
            "mp4" | "mkv" | "avi" | "mov" | "webm" => Self::ensure_cached_svg_icon(
                &mut cache.icon_video,
                ctx,
                "icon_video",
                include_bytes!("../Icons/pixel--media.svg"),
                Some(64),
            ),
            "zip" | "tar" | "gz" | "bz2" | "xz" | "7z" | "rar" => Self::ensure_cached_svg_icon(
                &mut cache.icon_archive,
                ctx,
                "icon_archive",
                include_bytes!("../Icons/pixel--save-solid.svg"),
                Some(64),
            ),
            "exe" | "bin" | "appimage" | "dmg" | "deb" | "rpm" | "app" | "bat" | "cmd" => {
                Self::ensure_cached_svg_icon(
                    &mut cache.icon_app,
                    ctx,
                    "icon_app",
                    include_bytes!("../Icons/pixel--programming.svg"),
                    Some(64),
                )
            }
            _ => Self::ensure_cached_svg_icon(
                &mut cache.icon_file,
                ctx,
                "icon_file",
                include_bytes!("../Icons/pixel--clipboard-solid.svg"),
                Some(64),
            ),
        })
    }

    fn file_manager_selected_entries(&self) -> Vec<super::file_manager::FileEntryRow> {
        self.file_manager.selected_rows_for_action()
    }

    fn file_manager_selection_count(&self) -> usize {
        self.file_manager_selected_entries().len()
    }

    fn file_manager_select_path(&mut self, path: PathBuf, ctrl_toggle: bool, allow_multi: bool) {
        if allow_multi && ctrl_toggle {
            self.file_manager.toggle_selected_path(&path);
        } else {
            self.file_manager.select(Some(path));
        }
    }

    /// Returns the SVG preview from cache if available, otherwise the default asset icon.
    /// Returns an owned TextureHandle (Arc clone) so callers don't borrow self across &mut calls.
    fn svg_preview_texture(
        &mut self,
        ctx: &Context,
        row: &super::file_manager::FileEntryRow,
    ) -> Option<TextureHandle> {
        let is_svg = row
            .path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.eq_ignore_ascii_case("svg"))
            .unwrap_or(false);
        if is_svg {
            let key = row.path.to_string_lossy().to_string();
            if let Some(tex) = self.shortcut_icon_cache.get(&key) {
                return Some(tex.clone());
            }
        }
        self.file_manager_texture_for_row(ctx, row)
    }

    fn split_file_name(name: &str) -> (&str, &str) {
        if let Some((stem, _ext)) = name.rsplit_once('.') {
            if !stem.is_empty() {
                return (stem, &name[stem.len()..]);
            }
        }
        (name, "")
    }

    fn apply_file_manager_settings_update(&mut self, update: FileManagerSettingsUpdate) {
        apply_desktop_file_manager_settings_update(&mut self.settings.draft, update);
        self.sync_runtime_settings_cache();
    }

    fn launch_open_with_command(&mut self, path: &Path, command_line: &str) -> Result<String> {
        let launch = file_manager_app::prepare_open_with_launch(path, command_line)?;
        Ok(self.launch_open_with_request(launch))
    }

    fn launch_open_with_request(&mut self, launch: OpenWithLaunchRequest) -> String {
        self.open_desktop_pty(&launch.title, &launch.argv);
        launch.status_message
    }

    fn handle_file_manager_prompt_outcome(&mut self, outcome: &PromptOutcome) -> bool {
        let Some(actions) = file_manager_app::apply_prompt_outcome(
            outcome,
            &mut self.file_manager,
            &mut self.file_manager_runtime,
        ) else {
            return false;
        };
        self.terminal_prompt = None;
        for action in actions {
            match action {
                FileManagerPromptAction::Launch(launch) => {
                    self.shell_status = self.launch_open_with_request(launch);
                }
                FileManagerPromptAction::ApplySettingsUpdate(update) => {
                    self.apply_file_manager_settings_update(update);
                }
                FileManagerPromptAction::ReportStatus(status) => {
                    self.shell_status = status;
                }
            }
        }
        true
    }

    fn file_manager_selected_file(&self) -> Option<super::file_manager::FileEntryRow> {
        file_manager_app::selected_file(self.file_manager_selected_entries())
    }

    fn unique_path_in_dir(dir: &Path, original_name: &str) -> PathBuf {
        let direct = dir.join(original_name);
        if !direct.exists() {
            return direct;
        }
        let (stem, ext) = Self::split_file_name(original_name);
        for index in 1..=9999usize {
            let candidate = dir.join(format!("{stem} ({index}){ext}"));
            if !candidate.exists() {
                return candidate;
            }
        }
        direct
    }

    fn file_manager_move_paths_to_dir(
        &mut self,
        paths: Vec<PathBuf>,
        target_dir: &Path,
    ) -> Result<String> {
        self.file_manager_runtime
            .move_paths_to_dir(&mut self.file_manager, paths, target_dir)
    }

    fn file_manager_drop_allowed(paths: &[PathBuf], target_dir: &Path) -> bool {
        FileManagerEditRuntime::drop_allowed(paths, target_dir)
    }

    fn file_manager_handle_drop_to_dir(&mut self, paths: Vec<PathBuf>, target_dir: PathBuf) {
        self.shell_status = match self.file_manager_move_paths_to_dir(paths, &target_dir) {
            Ok(message) => message,
            Err(err) => format!("File action failed: {err}"),
        };
        let desktop_dir = robco_desktop_dir();
        if target_dir == desktop_dir || target_dir.starts_with(&desktop_dir) {
            self.invalidate_desktop_surface_cache();
        }
    }

    fn desktop_entry_row(entry: &DesktopSurfaceEntry) -> FileEntryRow {
        FileEntryRow {
            path: entry.path.clone(),
            label: entry.label.clone(),
            is_dir: entry.is_dir(),
        }
    }

    fn create_desktop_folder(&mut self) {
        let desktop_dir = robco_desktop_dir();
        self.shell_status = match self
            .file_manager_runtime
            .create_folder_in_dir(&desktop_dir, "New Folder")
        {
            Ok(path) => {
                self.invalidate_desktop_surface_cache();
                format!(
                    "Created {} on the desktop.",
                    path.file_name()
                        .and_then(|name| name.to_str())
                        .unwrap_or("folder")
                )
            }
            Err(err) => format!("Desktop folder create failed: {err}"),
        };
    }

    fn paste_to_desktop(&mut self) {
        let desktop_dir = robco_desktop_dir();
        self.shell_status = match self
            .file_manager_runtime
            .paste_clipboard_into_dir(&desktop_dir)
        {
            Ok((count, last_dst)) => {
                self.invalidate_desktop_surface_cache();
                if count == 1 {
                    format!(
                        "Pasted {} onto the desktop.",
                        last_dst
                            .as_ref()
                            .and_then(|path| path.file_name())
                            .and_then(|name| name.to_str())
                            .unwrap_or("item")
                    )
                } else {
                    format!("Pasted {count} items onto the desktop.")
                }
            }
            Err(err) => format!("Desktop paste failed: {err}"),
        };
    }

    fn open_desktop_surface_path(&mut self, path: PathBuf) {
        if path.is_dir() {
            self.open_file_manager_at(path);
            return;
        }
        match file_manager_app::open_target_for_file_manager_action(
            FileManagerAction::OpenFile(path),
            &self.live_desktop_file_manager_settings,
        ) {
            Ok(FileManagerOpenTarget::NoOp) => {}
            Ok(FileManagerOpenTarget::Launch(launch)) => {
                self.shell_status = self.launch_open_with_request(launch);
            }
            Ok(FileManagerOpenTarget::OpenInEditor(path)) => self.open_path_in_editor(path),
            Err(status) => self.shell_status = status,
        }
    }

    fn open_desktop_item_properties(&mut self, path: PathBuf) {
        let name_draft = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("item")
            .to_string();
        self.desktop_selected_icon = Some(DesktopIconSelection::Surface(format!(
            "desktop_item:{name_draft}"
        )));
        self.desktop_item_properties = Some(DesktopItemPropertiesState {
            is_dir: path.is_dir(),
            path,
            name_draft,
        });
    }

    fn rename_desktop_item(&mut self, path: PathBuf) {
        self.open_desktop_item_properties(path);
    }

    fn delete_desktop_item(&mut self, path: PathBuf) {
        let label = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("item")
            .to_string();
        let row = FileEntryRow {
            path: path.clone(),
            label,
            is_dir: path.is_dir(),
        };
        self.shell_status = match self.file_manager_runtime.delete_entries(vec![row]) {
            Ok(count) => {
                self.desktop_item_properties = None;
                self.desktop_selected_icon = None;
                self.invalidate_desktop_surface_cache();
                if count == 1 {
                    "Moved desktop item to trash.".to_string()
                } else {
                    format!("Moved {count} desktop items to trash.")
                }
            }
            Err(err) => format!("Desktop delete failed: {err}"),
        };
    }

    fn open_desktop_surface_with_prompt(&mut self, path: PathBuf) {
        let ext_key = file_manager_app::open_with_extension_key(&path);
        self.open_file_manager_prompt(FileManagerPromptRequest::open_with_new_command(
            path, ext_key, false,
        ));
    }

    fn run_file_manager_command(&mut self, command: FileManagerCommand) {
        let home_path = self.file_manager_home_path();
        match file_manager_app::run_command(
            command,
            &mut self.file_manager,
            &mut self.file_manager_runtime,
            &home_path,
        ) {
            FileManagerCommandRequest::None => {}
            FileManagerCommandRequest::ActivateSelection => {
                self.activate_file_manager_selection();
            }
            FileManagerCommandRequest::OpenPrompt(request) => {
                self.open_file_manager_prompt(request);
            }
            FileManagerCommandRequest::ApplyDisplaySettings(update) => {
                self.apply_file_manager_display_settings_update(update);
            }
            FileManagerCommandRequest::ReportStatus(status) => {
                self.shell_status = status;
            }
        }
    }

    fn attach_generic_context_menu(
        action: &mut Option<ContextMenuAction>,
        response: &egui::Response,
    ) {
        response.context_menu(|ui| {
            Self::apply_context_menu_style(ui);
            ui.set_min_width(118.0);
            ui.set_max_width(160.0);

            if ui.button("Copy").clicked() {
                *action = Some(ContextMenuAction::GenericCopy);
                ui.close_menu();
            }
            if ui.button("Paste").clicked() {
                *action = Some(ContextMenuAction::GenericPaste);
                ui.close_menu();
            }

            Self::retro_separator(ui);

            if ui.button("Select All").clicked() {
                *action = Some(ContextMenuAction::GenericSelectAll);
                ui.close_menu();
            }
        });
    }

    fn draw_editor_save_as_window(&mut self, _ctx: &egui::Context) {}

    fn open_file_manager_at(&mut self, path: PathBuf) {
        if self.desktop_window_is_open(DesktopWindow::FileManager) {
            self.spawn_secondary_window(
                DesktopWindow::FileManager,
                SecondaryWindowApp::FileManager {
                    state: NativeFileManagerState::new(path),
                    runtime: FileManagerEditRuntime::default(),
                },
            );
        } else {
            self.open_embedded_file_manager_at(path);
            self.open_desktop_window(DesktopWindow::FileManager);
        }
    }

    fn open_embedded_file_manager_at(&mut self, path: PathBuf) {
        match open_directory_location(path) {
            Ok(location) => self.apply_file_manager_location(location),
            Err(status) => self.shell_status = status,
        }
    }

    fn default_editor_save_name(&self) -> String {
        self.editor
            .path
            .as_ref()
            .and_then(|path| path.file_name())
            .and_then(|name| name.to_str())
            .filter(|name| !name.is_empty())
            .unwrap_or("document.txt")
            .to_string()
    }

    fn open_editor_save_as_picker(&mut self) {
        let Some(session) = &self.session else {
            self.editor.status = "No active session.".to_string();
            return;
        };
        let start_dir = self
            .editor
            .path
            .as_ref()
            .and_then(|path| path.parent().map(Path::to_path_buf))
            .unwrap_or_else(|| word_processor_dir(&session.username));
        self.editor.save_as_input = Some(self.default_editor_save_name());
        self.open_embedded_file_manager_at(start_dir);
        if let Some(path) = self.editor.path.clone() {
            self.file_manager.select(Some(path));
        }
        self.editor.status =
            "Choose a folder in My Computer, enter a file name, then click Save Here.".to_string();
        self.desktop_active_window = Some(WindowInstanceId::primary(DesktopWindow::FileManager));
    }

    fn complete_editor_save_as_from_picker(&mut self) {
        let Some(name_draft) = self.editor.save_as_input.clone() else {
            return;
        };
        let file_name = name_draft.trim();
        if file_name.is_empty() {
            self.editor.status = "Enter a file name first.".to_string();
            return;
        }
        let name_path = Path::new(file_name);
        if name_path.file_name().and_then(|name| name.to_str()) != Some(file_name)
            || name_path.components().count() != 1
        {
            self.editor.status =
                "Enter a file name only. Use My Computer to choose the folder.".to_string();
            return;
        }
        let requested_target = self.file_manager.cwd.join(file_name);
        let target = Self::unique_path_in_dir(&self.file_manager.cwd, file_name);
        let renamed_to_avoid_collision = target != requested_target;
        match save_text_file(&target, &self.editor.text) {
            Ok(()) => {
                let label = target
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("document")
                    .to_string();
                self.editor.path = Some(target.clone());
                self.editor.dirty = false;
                self.editor.status = if renamed_to_avoid_collision {
                    format!("Name already existed. Saved as {label}.")
                } else {
                    format!("Saved {label}.")
                };
                self.push_editor_recent_file(&target);
                self.editor.save_as_input = None;
                self.file_manager.open = false;
                self.open_desktop_window(DesktopWindow::Editor);
            }
            Err(err) => {
                self.editor.status = format!("Save failed: {err}");
            }
        }
    }

    fn open_desktop_catalog_launch(&mut self, name: &str, catalog: ProgramCatalog) {
        match resolve_catalog_launch(name, catalog) {
            Ok(launch) => self.open_desktop_pty(&launch.title, &launch.argv),
            Err(err) => self.shell_status = err,
        }
    }

    fn open_embedded_catalog_launch(
        &mut self,
        name: &str,
        catalog: ProgramCatalog,
        return_screen: TerminalScreen,
    ) {
        match resolve_catalog_launch(name, catalog) {
            Ok(launch) => self.open_embedded_pty(&launch.title, &launch.argv, return_screen),
            Err(err) => self.shell_status = err,
        }
    }

    fn open_desktop_nuke_codes(&mut self) {
        if matches!(self.terminal_nuke_codes, NukeCodesView::Unloaded) {
            let tx = self.background.sender();
            std::thread::spawn(move || {
                let view = fetch_nuke_codes();
                let _ = tx.send(BackgroundResult::NukeCodesFetched(view));
            });
        }
        self.open_desktop_window(DesktopWindow::NukeCodes);
    }

    /// Open a desktop window if not already open, otherwise spawn a secondary
    /// embedded instance inside the shell.
    pub(super) fn open_or_spawn_desktop_window(&mut self, window: DesktopWindow) {
        if !self.desktop_window_is_open(window) {
            self.open_desktop_window(window);
            return;
        }
        // Already open — try to create a secondary embedded instance.
        let secondary_app = match window {
            DesktopWindow::FileManager => Some(SecondaryWindowApp::FileManager {
                state: NativeFileManagerState::new(home_dir_fallback()),
                runtime: FileManagerEditRuntime::default(),
            }),
            DesktopWindow::Editor => Some(SecondaryWindowApp::Editor(EditorWindow::default())),
            // Window types that don't support multi-instance: just focus existing.
            _ => None,
        };
        if let Some(app) = secondary_app {
            self.spawn_secondary_window(window, app);
        } else {
            self.open_desktop_window(window);
        }
    }

    fn spawn_secondary_window(&mut self, kind: DesktopWindow, app: SecondaryWindowApp) {
        let instance = self.next_window_instance;
        self.next_window_instance += 1;
        let id = WindowInstanceId { kind, instance };
        // Set up window state with a fresh generation.
        let generation = self.next_desktop_window_generation();
        let state = self.desktop_window_state_mut(id);
        state.minimized = false;
        state.maximized = false;
        state.generation = generation;
        let secondary = SecondaryWindow { id, app };
        self.secondary_windows.push(secondary);
        self.desktop_active_window = Some(id);
        if self.desktop_mode_open {
            self.close_desktop_overlays();
        }
    }

    fn execute_desktop_shell_action(&mut self, action: DesktopShellAction) {
        match action {
            DesktopShellAction::OpenWindow(window) => self.open_or_spawn_desktop_window(window),
            DesktopShellAction::OpenTextEditor => {
                self.open_or_spawn_desktop_window(DesktopWindow::Editor);
            }
            DesktopShellAction::OpenNukeCodes => {
                self.open_desktop_nuke_codes();
            }
            DesktopShellAction::OpenDesktopTerminalShell => self.open_desktop_terminal_shell(),
            DesktopShellAction::OpenConnectionsSettings => {
                if connections_macos_disabled() {
                    self.shell_status = connections_macos_disabled_hint().to_string();
                } else {
                    self.reset_desktop_settings_window();
                    self.settings.panel = NativeSettingsPanel::Connections;
                    self.open_desktop_window(DesktopWindow::Settings);
                }
            }
            DesktopShellAction::LaunchConfiguredApp(name) => {
                self.apply_desktop_program_request(DesktopProgramRequest::LaunchCatalog {
                    name,
                    catalog: ProgramCatalog::Applications,
                    close_window: true,
                });
            }
            DesktopShellAction::OpenFileManagerAt(path) => {
                self.open_file_manager_at(path);
            }
            DesktopShellAction::LaunchNetworkProgram(name) => {
                self.apply_desktop_program_request(DesktopProgramRequest::LaunchCatalog {
                    name,
                    catalog: ProgramCatalog::Network,
                    close_window: true,
                });
            }
            DesktopShellAction::LaunchGameProgram(name) => {
                let request = resolve_desktop_games_request(&name, BUILTIN_DONKEY_KONG_GAME);
                self.apply_desktop_program_request(request);
            }
            DesktopShellAction::OpenPathInEditor(path) => {
                self.open_path_in_editor(path);
            }
            DesktopShellAction::RevealPathInFileManager(path) => {
                if self.desktop_window_is_open(DesktopWindow::FileManager) {
                    // Spawn a new embedded instance at the parent directory.
                    let dir = path.parent().map(Path::to_path_buf).unwrap_or_else(home_dir_fallback);
                    self.spawn_secondary_window(
                        DesktopWindow::FileManager,
                        SecondaryWindowApp::FileManager {
                            state: NativeFileManagerState::new(dir),
                            runtime: FileManagerEditRuntime::default(),
                        },
                    );
                } else {
                    match reveal_path_location(path) {
                        Ok(location) => {
                            self.apply_file_manager_location(location);
                            self.open_desktop_window(DesktopWindow::FileManager);
                        }
                        Err(status) => self.shell_status = status,
                    }
                }
            }
        }
    }

    fn open_manual_file(&mut self, path: &str, status_label: &str) {
        let manual = PathBuf::from(path);
        match load_text_document(manual) {
            Ok(document) => {
                self.editor.path = Some(document.path);
                self.editor.text = document.text;
                self.editor.dirty = false;
                self.editor.status = format!("Opened {status_label}.");
                self.open_desktop_window(DesktopWindow::Editor);
            }
            Err(status) => {
                self.shell_status = format!("{status_label} unavailable: {status}");
            }
        }
    }

    fn restore_for_user(&mut self, username: &str, user: &UserRecord) {
        let settings = reload_settings_snapshot();
        let plan = build_native_session_restore_plan(username, user, settings.default_open_mode);
        self.session = Some(SessionState {
            username: plan.identity.username,
            is_admin: plan.identity.is_admin,
        });
        self.login.hacking = None;
        self.file_manager.cwd = plan.file_manager_dir;
        self.file_manager.open = false;
        self.file_manager.selected = None;
        self.editor = EditorWindow::default();
        self.replace_settings_draft(settings);
        self.apply_status_update(clear_settings_status());
        self.settings.panel = desktop_settings_default_panel();
        self.donkey_kong_window.open = false;
        self.donkey_kong = None;
        self.desktop_nuke_codes_open = false;
        self.desktop_installer = DesktopInstallerState::default();
        self.terminal_mode.status.clear();
        self.reset_shell_runtime_for_session(plan.launch_default_desktop);
        self.apply_status_update(clear_shell_status());
    }

    fn reset_shell_runtime_for_session(&mut self, launch_default_desktop: bool) {
        let terminal_defaults = terminal_runtime_defaults();
        self.desktop_window_states.clear();
        self.desktop_active_window = None;
        self.start_open = !launch_default_desktop;
        self.start_selected_root = 0;
        self.start_system_selected = 0;
        self.start_leaf_selected = 0;
        self.start_open_submenu = None;
        self.start_open_leaf = None;
        self.desktop_mode_open = launch_default_desktop;
        self.apply_terminal_navigation_state(terminal_defaults);
        self.terminal_nuke_codes = NukeCodesView::default();
        self.terminal_pty = None;
        self.terminal_installer.reset();
        self.terminal_edit_menus.reset();
        self.terminal_connections.reset();
        self.terminal_prompt = None;
        self.terminal_flash = None;
        self.session_leader_until = None;
    }

    fn current_terminal_navigation_state(&self) -> TerminalNavigationState {
        self.terminal_nav.clone()
    }

    fn apply_terminal_navigation_state(&mut self, state: TerminalNavigationState) {
        self.terminal_nav = state;
    }

    fn persist_snapshot(&self) {
        if let Some(session) = &self.session {
            persist_native_shell_snapshot(
                &session.username,
                &self.file_manager.cwd,
                self.editor.path.as_deref(),
            );
        }
    }

    fn navigate_to_screen(&mut self, screen: TerminalScreen) {
        if self.terminal_nav.screen != screen {
            crate::sound::play_navigate();
        }
        self.terminal_nav.screen = screen;
    }

    fn set_user_management_mode(&mut self, mode: UserManagementMode, selected_idx: usize) {
        let changed = self.terminal_nav.user_management_mode != mode
            || self.terminal_nav.user_management_idx != selected_idx;
        if changed {
            crate::sound::play_navigate();
        }
        self.terminal_nav.user_management_mode = mode;
        self.terminal_nav.user_management_idx = selected_idx;
    }

    fn apply_terminal_login_password_plan(&mut self, plan: TerminalLoginPasswordPlan<UserRecord>) {
        self.apply_terminal_login_submit_action(plan.action, true);
        if let Some(prompt) = plan.reopen_prompt {
            self.open_password_prompt(prompt.title, prompt.prompt);
        }
    }

    fn apply_terminal_user_management_prompt_plan(
        &mut self,
        plan: TerminalUserManagementPromptPlan,
    ) {
        match plan {
            TerminalUserManagementPromptPlan::Status(message) => {
                self.apply_status_update(shell_status(message));
            }
            TerminalUserManagementPromptPlan::SetMode {
                mode,
                selected_idx,
                suppress_next_menu_submit,
            } => {
                self.set_user_management_mode(mode, selected_idx);
                self.terminal_nav.suppress_next_menu_submit = suppress_next_menu_submit;
            }
            TerminalUserManagementPromptPlan::OpenPasswordConfirm {
                flow,
                username,
                first_password,
                prompt,
            } => {
                let action = match flow {
                    TerminalUserPasswordFlow::Create => {
                        TerminalPromptAction::CreatePasswordConfirm {
                            username,
                            first_password,
                        }
                    }
                    TerminalUserPasswordFlow::Reset => TerminalPromptAction::ResetPasswordConfirm {
                        username,
                        first_password,
                    },
                    TerminalUserPasswordFlow::ChangeAuth => {
                        TerminalPromptAction::ChangeAuthPasswordConfirm {
                            username,
                            first_password,
                        }
                    }
                };
                self.open_password_prompt_with_action(prompt.title, prompt.prompt, action);
            }
            TerminalUserManagementPromptPlan::ApplyPassword {
                flow,
                username,
                password,
            } => {
                match flow {
                    TerminalUserPasswordFlow::Create => {
                        self.apply_shell_status_result(create_desktop_user(
                            &username,
                            AuthMethod::Password,
                            Some(&password),
                        ));
                        self.invalidate_user_cache();
                    }
                    TerminalUserPasswordFlow::Reset => {
                        self.apply_shell_status_result(
                            update_user_auth_method(
                                &username,
                                AuthMethod::Password,
                                Some(&password),
                            )
                            .map(|_| "Password updated.".to_string()),
                        );
                        self.invalidate_user_cache();
                    }
                    TerminalUserPasswordFlow::ChangeAuth => {
                        self.apply_shell_status_result(update_user_auth_method(
                            &username,
                            AuthMethod::Password,
                            Some(&password),
                        ));
                        self.invalidate_user_cache();
                    }
                }
                self.set_user_management_mode(UserManagementMode::Root, 0);
            }
        }
    }

    fn login_usernames(&self) -> Vec<String> {
        load_login_usernames()
    }

    fn queue_terminal_flash(&mut self, message: impl Into<String>, ms: u64, action: FlashAction) {
        self.terminal_flash = Some(TerminalFlash {
            message: message.into(),
            until: Instant::now() + Duration::from_millis(ms),
            action,
            boxed: false,
        });
    }

    fn queue_terminal_flash_boxed(
        &mut self,
        message: impl Into<String>,
        ms: u64,
        action: FlashAction,
    ) {
        self.terminal_flash = Some(TerminalFlash {
            message: message.into(),
            until: Instant::now() + Duration::from_millis(ms),
            action,
            boxed: true,
        });
    }

    fn queue_session_flash_plan(&mut self, plan: NativeSessionFlashPlan) {
        self.terminal_flash = Some(TerminalFlash {
            message: plan.message,
            until: Instant::now() + Duration::from_millis(plan.duration_ms),
            action: plan.action,
            boxed: plan.boxed,
        });
    }

    fn begin_logout(&mut self) {
        let already_logging_out = self
            .terminal_flash
            .as_ref()
            .is_some_and(|flash| matches!(&flash.action, FlashAction::FinishLogout));
        let Some(plan) = logout_flash_plan(already_logging_out) else {
            return;
        };
        crate::sound::play_logout();
        self.persist_snapshot();
        self.terminate_all_native_pty_children();
        self.terminal_prompt = None;
        self.terminal_nav.screen = TerminalScreen::MainMenu;
        self.close_start_menu();
        self.desktop_mode_open = false;
        self.desktop_nuke_codes_open = false;
        self.desktop_active_window = None;
        self.session_leader_until = None;
        self.queue_session_flash_plan(plan);
    }

    fn finish_logout(&mut self) {
        let _ = reload_settings_snapshot();
        self.terminate_all_native_pty_children();
        clear_native_sessions();
        self.session_runtime.clear();
        self.session = None;
        self.login.reset();
        self.file_manager.open = false;
        self.editor.open = false;
        self.settings.open = false;
        self.settings.panel = desktop_settings_default_panel();
        self.applications.open = false;
        self.desktop_nuke_codes_open = false;
        self.terminal_mode.open = false;
        self.reset_shell_runtime_for_logout();
        self.apply_status_update(clear_shell_status());
    }

    fn reset_shell_runtime_for_logout(&mut self) {
        self.reset_shell_runtime_for_session(false);
        self.terminal_pty = None;
    }

    fn open_password_prompt(&mut self, title: impl Into<String>, prompt: impl Into<String>) {
        crate::sound::play_navigate();
        self.terminal_prompt = Some(TerminalPrompt {
            kind: TerminalPromptKind::Password,
            title: title.into(),
            prompt: prompt.into(),
            buffer: String::new(),
            confirm_yes: true,
            action: TerminalPromptAction::LoginPassword,
        });
    }

    fn open_input_prompt(
        &mut self,
        title: impl Into<String>,
        prompt: impl Into<String>,
        action: TerminalPromptAction,
    ) {
        crate::sound::play_navigate();
        self.terminal_prompt = Some(TerminalPrompt {
            kind: TerminalPromptKind::Input,
            title: title.into(),
            prompt: prompt.into(),
            buffer: String::new(),
            confirm_yes: true,
            action,
        });
    }

    fn open_password_prompt_with_action(
        &mut self,
        title: impl Into<String>,
        prompt: impl Into<String>,
        action: TerminalPromptAction,
    ) {
        crate::sound::play_navigate();
        self.terminal_prompt = Some(TerminalPrompt {
            kind: TerminalPromptKind::Password,
            title: title.into(),
            prompt: prompt.into(),
            buffer: String::new(),
            confirm_yes: true,
            action,
        });
    }

    fn open_confirm_prompt(
        &mut self,
        title: impl Into<String>,
        prompt: impl Into<String>,
        action: TerminalPromptAction,
    ) {
        crate::sound::play_navigate();
        self.terminal_prompt = Some(TerminalPrompt {
            kind: TerminalPromptKind::Confirm,
            title: title.into(),
            prompt: prompt.into(),
            buffer: String::new(),
            confirm_yes: true,
            action,
        });
    }

    fn open_file_manager_prompt(&mut self, request: FileManagerPromptRequest) {
        self.terminal_prompt = Some(request.to_terminal_prompt());
    }

    fn apply_shell_status_result(&mut self, result: Result<String, String>) {
        match result {
            Ok(status) | Err(status) => self.apply_status_update(shell_status(status)),
        }
    }

    fn apply_terminal_pty_launch_plan(
        &mut self,
        plan: TerminalPtyLaunchPlan,
        desktop_window: bool,
    ) {
        if plan.replace_existing_pty {
            if let Some(mut previous) = self.terminal_pty.take() {
                previous.session.terminate();
            }
        }
        let profile = desktop_pty_profile_for_command(&plan.argv);
        let pty_cols = profile
            .preferred_w
            .unwrap_or(96)
            .max(profile.min_w)
            .clamp(40, 160);
        let pty_rows = profile
            .preferred_h
            .unwrap_or(32)
            .max(profile.min_h)
            .clamp(10, 60);
        let options = crate::pty::PtyLaunchOptions {
            env: plan.env,
            top_bar: None,
            force_render_mode: plan.force_render_mode,
        };
        match spawn_embedded_pty_with_options(
            &plan.title,
            &plan.argv,
            plan.return_screen,
            pty_cols,
            pty_rows,
            options,
        ) {
            Ok(mut state) => {
                state.desktop_cols_floor = Some(pty_cols);
                state.desktop_rows_floor = Some(pty_rows);
                state.desktop_live_resize = profile.live_resize;
                if plan.use_fixed_terminal_metrics {
                    state.fixed_cell_w = Some(TERMINAL_MODE_PTY_CELL_W);
                    state.fixed_cell_h = Some(TERMINAL_MODE_PTY_CELL_H);
                    state.fixed_font_scale = Some(0.94);
                    state.fixed_font_width_divisor = Some(0.44);
                }
                self.terminal_pty = Some(state);
                if desktop_window {
                    self.open_desktop_window(DesktopWindow::PtyApp);
                    let window = self.desktop_window_state_mut(WindowInstanceId::primary(DesktopWindow::PtyApp));
                    window.maximized = profile.open_fullscreen;
                } else {
                    self.navigate_to_screen(TerminalScreen::PtyApp);
                }
                self.shell_status = plan.success_status;
            }
            Err(err) => {
                self.shell_status = err;
            }
        }
    }

    fn apply_terminal_flash_pty_launch_plan(&mut self, plan: TerminalFlashPtyLaunchPlan) {
        self.apply_terminal_pty_launch_plan(plan.launch, false);
        if let Some(state) = self.terminal_pty.as_mut() {
            state.completion_message = plan.completion_message;
        }
        self.shell_status = plan.status;
    }

    fn apply_terminal_flash_action_plan(&mut self, plan: TerminalFlashActionPlan) {
        match plan {
            TerminalFlashActionPlan::StartHacking {
                username,
                difficulty,
            } => {
                crate::sound::play_navigate();
                self.login.start_hacking(username, difficulty);
            }
            TerminalFlashActionPlan::LaunchPty(plan) => {
                self.apply_terminal_flash_pty_launch_plan(plan);
            }
        }
    }

    fn apply_terminal_embedded_pty_exit_plan(&mut self, plan: TerminalEmbeddedPtyExitPlan) {
        self.navigate_to_screen(plan.return_screen);
        if let Some(message) = plan.boxed_flash_message.clone() {
            self.queue_terminal_flash_boxed(message.clone(), 1600, FlashAction::Noop);
            self.shell_status = message;
        } else {
            self.shell_status = plan.status;
        }
    }

    fn apply_terminal_desktop_pty_exit_plan(&mut self, plan: TerminalDesktopPtyExitPlan) {
        self.shell_status = plan.status.clone();
        if let Some(message) = plan.installer_notice_message {
            self.desktop_installer.status = message.clone();
            self.desktop_installer.notice = Some(DesktopInstallerNotice {
                message,
                success: plan.installer_notice_success,
            });
        }
        if plan.reopen_installer {
            self.open_desktop_window(DesktopWindow::Installer);
        }
    }

    fn open_embedded_pty(&mut self, title: &str, cmd: &[String], return_screen: TerminalScreen) {
        let plan = terminal_command_launch_plan(
            TerminalShellSurface::Embedded,
            title,
            cmd,
            return_screen,
            desktop_pty_force_render_mode(cmd),
        );
        self.apply_terminal_pty_launch_plan(plan, false);
    }

    fn open_desktop_pty(&mut self, title: &str, cmd: &[String]) {
        let plan = terminal_command_launch_plan(
            TerminalShellSurface::Desktop,
            title,
            cmd,
            TerminalScreen::MainMenu,
            desktop_pty_force_render_mode(cmd),
        );
        self.apply_terminal_pty_launch_plan(plan, true);
    }

    fn open_embedded_terminal_shell(&mut self) {
        let requested_shell = std::env::var("SHELL").ok();
        let bash_exists = std::path::Path::new("/bin/bash").exists();
        let plan = terminal_shell_launch_plan(
            TerminalShellSurface::Embedded,
            requested_shell.as_deref(),
            bash_exists,
        );
        self.apply_terminal_pty_launch_plan(plan, false);
    }

    fn open_desktop_terminal_shell(&mut self) {
        let requested_shell = std::env::var("SHELL").ok();
        let bash_exists = std::path::Path::new("/bin/bash").exists();
        let plan = terminal_shell_launch_plan(
            TerminalShellSurface::Desktop,
            requested_shell.as_deref(),
            bash_exists,
        );
        self.apply_terminal_pty_launch_plan(plan, true);
    }

    fn open_path_in_editor(&mut self, path: PathBuf) {
        if self.desktop_window_is_open(DesktopWindow::Editor) {
            let mut editor = EditorWindow::default();
            if let Ok(document) = load_text_document(path.clone()) {
                editor.path = Some(document.path.clone());
                editor.text = document.text;
            }
            self.spawn_secondary_window(DesktopWindow::Editor, SecondaryWindowApp::Editor(editor));
        } else {
            self.open_embedded_path_in_editor(path);
        }
    }

    fn open_embedded_path_in_editor(&mut self, path: PathBuf) {
        match load_text_document(path.clone()) {
            Ok(document) => {
                self.editor.path = Some(document.path.clone());
                self.editor.text = document.text;
                self.editor.dirty = false;
                self.editor.status = "Opened document.".to_string();
                self.push_editor_recent_file(&document.path);
                self.open_desktop_window(DesktopWindow::Editor);
            }
            Err(status) => {
                self.editor.status = format!("Open failed: {status}");
                self.open_desktop_window(DesktopWindow::Editor);
            }
        }
    }

    fn activate_file_manager_selection(&mut self) {
        let settings = load_settings_snapshot();
        match file_manager_app::open_target_for_file_manager_action(
            self.file_manager.activate_selected(),
            &settings.desktop_file_manager,
        ) {
            Ok(FileManagerOpenTarget::NoOp) => {}
            Ok(FileManagerOpenTarget::Launch(launch)) => {
                self.shell_status = self.launch_open_with_request(launch);
            }
            Ok(FileManagerOpenTarget::OpenInEditor(path)) => self.open_path_in_editor(path),
            Err(status) => self.shell_status = status,
        }
    }

    /// Double-click handler for the file manager. In pick modes (icon/wallpaper
    /// selection), clicking a file triggers the pick action immediately. For
    /// directories or normal mode, falls through to the regular open logic.
    fn file_manager_activate_or_pick(&mut self) {
        let pick_mode = if self.editor.save_as_input.is_some() {
            FileManagerPickMode::SaveAs
        } else if let Some(pick_idx) = self.picking_icon_for_shortcut {
            FileManagerPickMode::ShortcutIcon(pick_idx)
        } else if self.picking_wallpaper {
            FileManagerPickMode::Wallpaper
        } else {
            FileManagerPickMode::None
        };
        match file_manager_app::selection_activation_for_selected_path(
            self.file_manager.selected.clone(),
            pick_mode,
        ) {
            FileManagerSelectionActivation::ActivateSelection => {}
            FileManagerSelectionActivation::FillSaveAsName(name) => {
                self.editor.save_as_input = Some(name);
                self.complete_editor_save_as_from_picker();
                return;
            }
            FileManagerSelectionActivation::PickShortcutIcon { shortcut_idx, path } => {
                self.apply_file_manager_picker_commit(FileManagerPickerCommit::SetShortcutIcon {
                    shortcut_idx,
                    path,
                });
                return;
            }
            FileManagerSelectionActivation::PickWallpaper(path) => {
                self.apply_file_manager_picker_commit(FileManagerPickerCommit::SetWallpaper(path));
                return;
            }
        }
        self.activate_file_manager_selection();
    }

    fn new_document(&mut self) {
        if self.desktop_mode_open {
            self.editor.reset_for_desktop_new_document();
            self.open_desktop_window(DesktopWindow::Editor);
            return;
        }
        let Some(session) = &self.session else {
            return;
        };
        let base = word_processor_dir(&session.username);
        let mut path = base.join("document.txt");
        let mut idx = 1usize;
        while path.exists() {
            path = base.join(format!("document-{idx}.txt"));
            idx += 1;
        }
        self.editor.prepare_new_document_at(path);
        self.open_desktop_window(DesktopWindow::Editor);
    }

    fn run_editor_command(&mut self, command: EditorCommand) {
        match command {
            EditorCommand::Save => self.save_editor(),
            EditorCommand::SaveAs => self.open_editor_save_as_picker(),
            EditorCommand::NewDocument => self.new_document(),
            EditorCommand::OpenFind => self.editor.ui.open_find(),
            EditorCommand::OpenFindReplace => self.editor.ui.open_find_replace(),
            EditorCommand::CloseFind => self.editor.ui.close_find(),
            EditorCommand::ToggleWordWrap => {
                self.editor.word_wrap = !self.editor.word_wrap;
            }
            EditorCommand::IncreaseFontSize => {
                self.editor.font_size = (self.editor.font_size + 2.0).min(32.0);
            }
            EditorCommand::DecreaseFontSize => {
                self.editor.font_size = (self.editor.font_size - 2.0).max(10.0);
            }
            EditorCommand::ResetFontSize => {
                self.editor.font_size = 16.0;
            }
            EditorCommand::SetTextAlign(alignment) => {
                self.editor.ui.set_text_align(alignment);
            }
            EditorCommand::ToggleLineNumbers => {
                self.editor.ui.toggle_line_numbers();
            }
        }
    }

    fn run_editor_text_command(
        &mut self,
        ctx: &Context,
        text_edit_id: Id,
        command: EditorTextCommand,
    ) {
        let key = match command {
            EditorTextCommand::Undo => egui::Key::Z,
            EditorTextCommand::Redo => egui::Key::Y,
            EditorTextCommand::Cut => egui::Key::X,
            EditorTextCommand::Copy => egui::Key::C,
            EditorTextCommand::Paste => egui::Key::V,
            EditorTextCommand::SelectAll => egui::Key::A,
        };
        ctx.memory_mut(|m| m.request_focus(text_edit_id));
        ctx.input_mut(|i| {
            i.events.push(egui::Event::Key {
                key,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::COMMAND,
            })
        });
    }

    fn editor_find_next(&mut self, ctx: &egui::Context, text_edit_id: egui::Id) {
        if self.editor.ui.find_query.is_empty() {
            return;
        }
        let text = self.editor.text.clone();
        let query = self.editor.ui.find_query.clone();
        // Collect all match byte positions
        let matches: Vec<usize> = text
            .char_indices()
            .filter_map(|(byte_idx, _)| {
                if text[byte_idx..].starts_with(query.as_str()) {
                    Some(byte_idx)
                } else {
                    None
                }
            })
            .collect();
        if matches.is_empty() {
            self.editor.status = format!("Not found: {}", query);
            return;
        }
        let idx = self.editor.ui.find_occurrence % matches.len();
        self.editor.ui.find_occurrence = idx + 1;
        let byte_start = matches[idx];
        let byte_end = byte_start + query.len();
        // Convert byte offsets to char counts
        let char_start = text[..byte_start].chars().count();
        let char_end = text[..byte_end].chars().count();
        // Set TextEdit cursor/selection
        let mut state = egui::text_edit::TextEditState::load(ctx, text_edit_id).unwrap_or_default();
        state
            .cursor
            .set_char_range(Some(egui::text::CCursorRange::two(
                egui::text::CCursor::new(char_start),
                egui::text::CCursor::new(char_end),
            )));
        state.store(ctx, text_edit_id);
        ctx.memory_mut(|m| m.request_focus(text_edit_id));
        self.editor.status = format!("Match {} of {}", idx + 1, matches.len());
    }

    fn editor_replace_one(&mut self, ctx: &egui::Context, text_edit_id: egui::Id) {
        if self.editor.ui.find_query.is_empty() {
            return;
        }
        let query = self.editor.ui.find_query.clone();
        let replacement = self.editor.ui.replace_query.clone();
        if let Some(pos) = self.editor.text.find(&query) {
            self.editor
                .text
                .replace_range(pos..pos + query.len(), &replacement);
            self.editor.dirty = true;
        }
        self.editor_find_next(ctx, text_edit_id);
    }

    fn editor_replace_all(&mut self) {
        if self.editor.ui.find_query.is_empty() {
            return;
        }
        let query = self.editor.ui.find_query.clone();
        let replacement = self.editor.ui.replace_query.clone();
        let count = self.editor.text.matches(query.as_str()).count();
        if count > 0 {
            self.editor.text = self.editor.text.replace(query.as_str(), &replacement);
            self.editor.dirty = true;
            self.editor.status = format!("Replaced {} occurrences.", count);
        } else {
            self.editor.status = format!("Not found: {}", query);
        }
    }

    fn push_editor_recent_file(&mut self, path: &std::path::Path) {
        if let Some(s) = path.to_str() {
            let s = s.to_string();
            self.settings.draft.editor_recent_files.retain(|p| p != &s);
            self.settings.draft.editor_recent_files.insert(0, s);
            self.settings.draft.editor_recent_files.truncate(10);
        }
    }

    fn save_editor(&mut self) {
        let Some(path) = self.editor.path.clone() else {
            if self.desktop_mode_open {
                self.open_editor_save_as_picker();
            } else {
                self.editor.status = "No document path set.".to_string();
            }
            return;
        };
        match save_text_file(&path, &self.editor.text) {
            Ok(()) => {
                self.editor.dirty = false;
                self.editor.status = format!(
                    "Saved {}.",
                    path.file_name()
                        .and_then(|name| name.to_str())
                        .unwrap_or("document")
                );
            }
            Err(err) => self.editor.status = format!("Save failed: {err}"),
        }
        self.push_editor_recent_file(&path);
    }

    fn current_donkey_kong_theme(&self) -> DonkeyKongTheme {
        let palette = current_palette();
        DonkeyKongTheme {
            primary: palette.fg,
            enemy: palette.selected_bg,
            ui: palette.fg,
            neutral: palette.dim,
        }
    }

    fn ensure_donkey_kong_loaded(&mut self, ctx: &Context) -> &mut DonkeyKongGame {
        let theme = self.current_donkey_kong_theme();
        let scale = self.settings.draft.native_ui_scale.max(1.0);
        self.donkey_kong.get_or_insert_with(|| {
            DonkeyKongGame::new(
                ctx,
                DonkeyKongConfig {
                    scale,
                    theme: theme.clone(),
                },
            )
        })
    }

    fn open_terminal_donkey_kong(&mut self) {
        self.navigate_to_screen(TerminalScreen::DonkeyKong);
        self.apply_status_update(clear_shell_status());
    }

    fn open_desktop_donkey_kong(&mut self) {
        self.open_desktop_window(DesktopWindow::DonkeyKong);
        self.apply_status_update(clear_shell_status());
    }

    fn edit_program_entries(&self, target: EditMenuTarget) -> Vec<String> {
        match target {
            EditMenuTarget::Applications => catalog_names(ProgramCatalog::Applications),
            EditMenuTarget::Documents => document_category_names(),
            EditMenuTarget::Network => catalog_names(ProgramCatalog::Network),
            EditMenuTarget::Games => catalog_names(ProgramCatalog::Games),
        }
    }

    fn program_catalog_for_edit_target(target: EditMenuTarget) -> Option<ProgramCatalog> {
        match target {
            EditMenuTarget::Applications => Some(ProgramCatalog::Applications),
            EditMenuTarget::Network => Some(ProgramCatalog::Network),
            EditMenuTarget::Games => Some(ProgramCatalog::Games),
            EditMenuTarget::Documents => None,
        }
    }

    fn add_program_entry(&mut self, target: EditMenuTarget, name: String, command: String) {
        let Ok(argv) = parse_catalog_command_line(command.trim()) else {
            self.shell_status = "Error: invalid command line".to_string();
            return;
        };
        match target {
            EditMenuTarget::Documents => {
                self.shell_status = "Error: invalid target for command entry.".to_string();
                return;
            }
            other => {
                let Some(catalog) = Self::program_catalog_for_edit_target(other) else {
                    self.shell_status = "Error: invalid target for command entry.".to_string();
                    return;
                };
                self.shell_status = add_catalog_entry(catalog, name, argv);
                self.invalidate_program_catalog_cache();
                self.invalidate_edit_menu_entries_cache(other);
            }
        }
    }

    fn delete_program_entry(&mut self, target: EditMenuTarget, name: &str) {
        match target {
            EditMenuTarget::Documents => {
                self.delete_document_category(name);
                return;
            }
            other => {
                let Some(catalog) = Self::program_catalog_for_edit_target(other) else {
                    return;
                };
                self.shell_status = delete_catalog_entry(catalog, name);
                self.invalidate_program_catalog_cache();
                self.invalidate_edit_menu_entries_cache(other);
            }
        }
    }

    fn rename_program_entry(&mut self, target: EditMenuTarget, old_name: &str, new_name: &str) {
        let new_name = new_name.trim();
        if new_name.is_empty() {
            self.shell_status = "Name cannot be empty.".to_string();
            return;
        }
        if new_name == old_name {
            self.shell_status = "Name unchanged.".to_string();
            return;
        }

        match target {
            EditMenuTarget::Documents => {
                match rename_desktop_document_category(old_name, new_name) {
                    Ok(status) => {
                        self.shell_status = status;
                        self.invalidate_edit_menu_entries_cache(EditMenuTarget::Documents);
                    }
                    Err(err) => self.shell_status = err,
                }
            }
            other => {
                let Some(catalog) = Self::program_catalog_for_edit_target(other) else {
                    return;
                };
                match rename_catalog_entry(catalog, old_name, new_name) {
                    Ok(status) => {
                        self.shell_status = status;
                        self.invalidate_program_catalog_cache();
                        self.invalidate_edit_menu_entries_cache(other);
                    }
                    Err(err) => self.shell_status = err,
                }
            }
        }
    }

    fn add_document_category(&mut self, name: String, path_raw: String) {
        match add_desktop_document_category(name, &path_raw) {
            Ok(status) => {
                self.shell_status = status;
                self.invalidate_edit_menu_entries_cache(EditMenuTarget::Documents);
            }
            Err(err) => self.shell_status = err,
        }
    }

    fn delete_document_category(&mut self, name: &str) {
        self.shell_status = delete_desktop_document_category(name);
        self.invalidate_edit_menu_entries_cache(EditMenuTarget::Documents);
    }

    fn sorted_document_categories() -> Vec<String> {
        document_category_names()
    }

    fn open_document_browser_at(&mut self, dir: PathBuf, return_screen: TerminalScreen) {
        if !dir.is_dir() {
            self.shell_status = format!("Error: '{}' not found.", dir.display());
            return;
        }
        self.file_manager.set_cwd(dir);
        self.file_manager.selected = None;
        self.terminal_nav.browser_idx = 0;
        self.terminal_nav.browser_return_screen = return_screen;
        self.navigate_to_screen(TerminalScreen::DocumentBrowser);
    }

    fn open_log_view(&mut self) {
        self.open_document_browser_at(logs_dir(), TerminalScreen::Logs);
    }

    fn normalize_new_file_name(raw: &str, default_stem: &str) -> Option<String> {
        let candidate = if raw.trim().is_empty() {
            default_stem.to_string()
        } else {
            raw.trim().to_string()
        };
        let mut normalized = String::new();
        let mut last_was_sep = false;
        for ch in candidate.chars() {
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.') {
                normalized.push(ch);
                last_was_sep = false;
            } else if ch.is_whitespace() && !normalized.is_empty() && !last_was_sep {
                normalized.push('_');
                last_was_sep = true;
            }
        }
        let normalized = normalized.trim_matches(['_', '.', ' ']).to_string();
        if normalized.is_empty() || normalized == "." || normalized == ".." {
            return None;
        }
        if std::path::Path::new(&normalized).extension().is_some() {
            Some(normalized)
        } else {
            Some(format!("{normalized}.txt"))
        }
    }

    fn create_or_open_log(&mut self, raw_name: &str) {
        let default_stem = Local::now().format("%Y-%m-%d").to_string();
        let Some(name) = Self::normalize_new_file_name(raw_name, &default_stem) else {
            self.shell_status = "Error: Invalid document name.".to_string();
            return;
        };
        let path = logs_dir().join(name);
        let existing = if path.exists() {
            std::fs::read_to_string(&path).unwrap_or_default()
        } else {
            String::new()
        };
        self.editor.path = Some(path);
        self.editor.text = existing;
        self.editor.dirty = false;
        self.open_desktop_window(DesktopWindow::Editor);
        self.editor.status = "Opened log.".to_string();
        self.shell_status = "Opened log editor.".to_string();
    }

    fn persist_native_settings(&mut self) {
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
        self.apply_status_update(saved_shell_status());
    }

    fn apply_native_window_mode(&self, ctx: &Context) {
        match self.settings.draft.native_startup_window_mode {
            NativeStartupWindowMode::Windowed => {
                ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(false));
                ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(false));
                ctx.send_viewport_cmd(egui::ViewportCommand::Decorations(true));
            }
            NativeStartupWindowMode::Maximized => {
                ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(false));
                ctx.send_viewport_cmd(egui::ViewportCommand::Decorations(true));
                ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(true));
            }
            NativeStartupWindowMode::BorderlessFullscreen => {
                ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(false));
                ctx.send_viewport_cmd(egui::ViewportCommand::Decorations(false));
                ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(true));
            }
            NativeStartupWindowMode::Fullscreen => {
                ctx.send_viewport_cmd(egui::ViewportCommand::Decorations(false));
                ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(false));
                ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(true));
            }
        }
    }

    fn draw_login(&mut self, ctx: &Context) {
        let layout = self.terminal_layout();
        match self.login.mode {
            TerminalLoginScreenMode::SelectUser => {
                let rows = login_menu_rows_from_users(self.login_usernames());
                if self.terminal_prompt.is_some() {
                    self.handle_terminal_prompt_input(ctx);
                }
                let activated = draw_login_screen(
                    ctx,
                    &rows,
                    &mut self.login.selected_idx,
                    &self.login.error,
                    self.terminal_prompt.as_ref(),
                    layout.cols,
                    layout.rows,
                    layout.header_start_row,
                    layout.separator_top_row,
                    layout.title_row,
                    layout.separator_bottom_row,
                    layout.subtitle_row,
                    layout.menu_start_row,
                    layout.status_row,
                    layout.content_col,
                );
                if activated {
                    let usernames = self.login_usernames();
                    let plan = resolve_login_selection_plan(
                        self.login.selected_idx,
                        &usernames,
                        login_selection_auth_method,
                        |username| authenticate_login(username, ""),
                    );
                    self.apply_terminal_login_selection_plan(plan);
                }
            }
            TerminalLoginScreenMode::Hacking => {
                let (username, event) = match self.login.hacking.as_mut() {
                    Some(hacking) => (
                        hacking.username.clone(),
                        draw_hacking_screen(
                            ctx,
                            &mut hacking.game,
                            layout.cols,
                            layout.rows,
                            layout.status_row,
                            layout.status_row_alt,
                        ),
                    ),
                    None => {
                        crate::sound::play_navigate();
                        self.login.mode = TerminalLoginScreenMode::SelectUser;
                        return;
                    }
                };
                match event {
                    HackingScreenEvent::None => {}
                    HackingScreenEvent::Cancel => {
                        self.apply_terminal_hacking_plan(resolve_hacking_screen_event(
                            &username,
                            TerminalHackingUiEvent::Cancel,
                            session_user_record,
                        ))
                    }
                    HackingScreenEvent::Success => {
                        self.apply_terminal_hacking_plan(resolve_hacking_screen_event(
                            &username,
                            TerminalHackingUiEvent::Success,
                            session_user_record,
                        ))
                    }
                    HackingScreenEvent::LockedOut => {
                        self.apply_terminal_hacking_plan(resolve_hacking_screen_event(
                            &username,
                            TerminalHackingUiEvent::LockedOut,
                            session_user_record,
                        ))
                    }
                    HackingScreenEvent::ExitLocked => {}
                }
            }
            TerminalLoginScreenMode::Locked => {
                if matches!(
                    draw_locked_screen(ctx, layout.cols, layout.rows, layout.status_row_alt),
                    HackingScreenEvent::ExitLocked
                ) {
                    self.login.show_user_selection();
                }
            }
        }
    }

    fn active_editor_text_edit_id(&self) -> Id {
        let generation = self.desktop_window_generation(DesktopWindow::Editor.into());
        Id::new(("editor_text_edit", generation))
    }


    fn retro_separator(ui: &mut egui::Ui) {
        let palette = current_palette();
        let desired = egui::vec2(ui.available_width().max(1.0), 2.0);
        let (rect, _) = ui.allocate_exact_size(desired, egui::Sense::hover());
        ui.painter().rect_filled(rect, 0.0, palette.fg);
        ui.add_space(2.0);
    }

    fn retro_disabled_button(ui: &mut egui::Ui, label: impl Into<String>) -> egui::Response {
        let palette = current_palette();
        ui.add(
            egui::Button::new(egui::RichText::new(label.into()).color(palette.dim))
                .sense(egui::Sense::hover()),
        )
    }

    fn apply_settings_control_style(ui: &mut egui::Ui) {
        let palette = current_palette();
        let mut style = ui.style().as_ref().clone();
        let stroke = egui::Stroke::new(2.0, palette.fg);
        style.visuals.override_text_color = None;
        style.visuals.window_fill = Color32::BLACK;
        style.visuals.panel_fill = Color32::BLACK;
        style.visuals.faint_bg_color = Color32::BLACK;
        style.visuals.extreme_bg_color = Color32::BLACK;
        style.visuals.code_bg_color = Color32::BLACK;
        style.visuals.window_stroke = stroke;
        style.visuals.window_rounding = egui::Rounding::ZERO;
        style.visuals.menu_rounding = egui::Rounding::ZERO;
        style.visuals.window_shadow = egui::epaint::Shadow::NONE;
        style.visuals.popup_shadow = egui::epaint::Shadow::NONE;
        style.visuals.selection.bg_fill = palette.fg;
        style.visuals.selection.stroke = stroke;
        style.visuals.hyperlink_color = palette.fg;
        style.visuals.text_cursor.stroke = stroke;
        style.visuals.widgets.noninteractive.bg_fill = Color32::BLACK;
        style.visuals.widgets.noninteractive.weak_bg_fill = Color32::BLACK;
        style.visuals.widgets.noninteractive.bg_stroke = stroke;
        style.visuals.widgets.noninteractive.fg_stroke = stroke;
        style.visuals.widgets.noninteractive.rounding = egui::Rounding::ZERO;
        style.visuals.widgets.noninteractive.expansion = 0.0;
        style.visuals.widgets.inactive.bg_fill = Color32::BLACK;
        style.visuals.widgets.inactive.weak_bg_fill = Color32::BLACK;
        style.visuals.widgets.inactive.bg_stroke = stroke;
        style.visuals.widgets.inactive.fg_stroke = stroke;
        style.visuals.widgets.inactive.rounding = egui::Rounding::ZERO;
        style.visuals.widgets.inactive.expansion = 0.0;
        style.visuals.widgets.hovered.bg_fill = palette.fg;
        style.visuals.widgets.hovered.weak_bg_fill = palette.fg;
        style.visuals.widgets.hovered.bg_stroke = stroke;
        style.visuals.widgets.hovered.fg_stroke.color = Color32::BLACK;
        style.visuals.widgets.hovered.rounding = egui::Rounding::ZERO;
        style.visuals.widgets.hovered.expansion = 0.0;
        style.visuals.widgets.active.bg_fill = palette.fg;
        style.visuals.widgets.active.weak_bg_fill = palette.fg;
        style.visuals.widgets.active.bg_stroke = stroke;
        style.visuals.widgets.active.fg_stroke.color = Color32::BLACK;
        style.visuals.widgets.active.rounding = egui::Rounding::ZERO;
        style.visuals.widgets.active.expansion = 0.0;
        style.visuals.widgets.open.bg_fill = palette.fg;
        style.visuals.widgets.open.weak_bg_fill = palette.fg;
        style.visuals.widgets.open.bg_stroke = stroke;
        style.visuals.widgets.open.fg_stroke.color = Color32::BLACK;
        style.visuals.widgets.open.rounding = egui::Rounding::ZERO;
        style.visuals.widgets.open.expansion = 0.0;
        ui.set_style(style);
    }

    fn retro_choice_button(
        ui: &mut egui::Ui,
        label: impl Into<String>,
        selected: bool,
    ) -> egui::Response {
        let palette = current_palette();
        let label = label.into();
        let button = if selected {
            egui::Button::new(label.clone())
                .fill(palette.fg)
                .stroke(egui::Stroke::new(2.0, palette.fg))
        } else {
            egui::Button::new(label.clone()).stroke(egui::Stroke::new(2.0, palette.fg))
        };
        let response = ui.add(button);
        if selected {
            let font = TextStyle::Button.resolve(ui.style());
            ui.painter().text(
                response.rect.center(),
                Align2::CENTER_CENTER,
                label,
                font,
                Color32::BLACK,
            );
        }
        response
    }

    fn retro_checkbox_row(ui: &mut egui::Ui, value: &mut bool, label: &str) -> egui::Response {
        let marker = if *value { "[x]" } else { "[ ]" };
        let response = ui.add(
            egui::Button::new(format!("{marker} {label}"))
                .stroke(egui::Stroke::new(2.0, current_palette().fg)),
        );
        if response.clicked() {
            *value = !*value;
        }
        response
    }

    fn retro_settings_tile(
        ui: &mut egui::Ui,
        texture: Option<&TextureHandle>,
        icon: &str,
        label: &str,
        enabled: bool,
        desired: egui::Vec2,
        icon_font_size: f32,
        label_font_size: f32,
    ) -> egui::Response {
        let palette = current_palette();
        let sense = if enabled {
            egui::Sense::click()
        } else {
            egui::Sense::hover()
        };
        let (rect, response) = ui.allocate_exact_size(desired, sense);
        let hovered = enabled && response.hovered();
        if hovered {
            ui.painter().rect_filled(rect, 0.0, palette.fg);
        }
        let text_color = if hovered { Color32::BLACK } else { palette.fg };
        if let Some(texture) = texture {
            let icon_side = (desired.y * 0.34).clamp(24.0, 40.0);
            let icon_rect = egui::Rect::from_center_size(
                egui::pos2(rect.center().x, rect.top() + desired.y * 0.34),
                egui::vec2(icon_side, icon_side),
            );
            Self::paint_tinted_texture(ui.painter(), texture, icon_rect, text_color);
        } else {
            ui.painter().text(
                rect.left_top() + egui::vec2(8.0, desired.y * 0.18),
                Align2::LEFT_TOP,
                icon,
                FontId::new(icon_font_size, FontFamily::Monospace),
                text_color,
            );
        }
        ui.painter().text(
            egui::pos2(rect.center().x, rect.top() + desired.y * 0.70),
            Align2::CENTER_CENTER,
            label,
            FontId::new(label_font_size, FontFamily::Monospace),
            text_color,
        );
        response
    }

    fn retro_full_width_button(ui: &mut egui::Ui, label: impl Into<String>) -> egui::Response {
        let palette = current_palette();
        ui.add_sized(
            [ui.available_width().max(160.0), 0.0],
            egui::Button::new(label.into()).stroke(egui::Stroke::new(2.0, palette.fg)),
        )
    }

    fn responsive_input_width(ui: &egui::Ui, fraction: f32, min: f32, max: f32) -> f32 {
        (ui.available_width() * fraction).clamp(min, max)
    }

    fn settings_two_columns<R>(
        ui: &mut egui::Ui,
        add_contents: impl FnOnce(&mut egui::Ui, &mut egui::Ui) -> R,
    ) -> R {
        let total_w = ui.available_width().min(860.0);
        let column_gap = 18.0;
        let column_w = ((total_w - column_gap) * 0.5).max(220.0);
        ui.columns(2, |columns| {
            let (left_slice, right_slice) = columns.split_at_mut(1);
            let left = &mut left_slice[0];
            let right = &mut right_slice[0];
            left.set_width(column_w);
            right.set_width(column_w);
            add_contents(left, right)
        })
    }

    fn settings_section<R>(
        ui: &mut egui::Ui,
        title: &str,
        add_contents: impl FnOnce(&mut egui::Ui) -> R,
    ) -> R {
        let palette = current_palette();
        egui::Frame::none()
            .fill(Color32::BLACK)
            .stroke(egui::Stroke::new(2.0, palette.fg))
            .inner_margin(egui::Margin::same(10.0))
            .show(ui, |ui| {
                ui.strong(title);
                ui.add_space(8.0);
                Self::retro_separator(ui);
                ui.add_space(8.0);
                add_contents(ui)
            })
            .inner
    }

    fn apply_file_manager_picker_commit(&mut self, commit: FileManagerPickerCommit) {
        match commit {
            FileManagerPickerCommit::SetShortcutIcon { shortcut_idx, path } => {
                if let Some(path_str) =
                    set_desktop_shortcut_icon(&mut self.settings.draft, shortcut_idx, &path)
                {
                    self.shortcut_icon_cache.remove(&path_str);
                    self.shortcut_icon_missing.remove(&path_str);
                    if let Some(props) = &mut self.shortcut_properties {
                        if props.shortcut_idx == shortcut_idx {
                            props.icon_path_draft = Some(path_str);
                        }
                    }
                }
                self.picking_icon_for_shortcut = None;
                self.file_manager.open = false;
                self.persist_native_settings();
            }
            FileManagerPickerCommit::SetWallpaper(path) => {
                set_desktop_wallpaper_path(&mut self.settings.draft, &path);
                self.picking_wallpaper = false;
                self.file_manager.open = false;
            }
        }
    }

    fn commit_file_manager_picker(&mut self, pick_mode: FileManagerPickMode) {
        match file_manager_app::commit_picker_selection(
            self.file_manager_selected_file(),
            pick_mode,
        ) {
            Ok(commit) => self.apply_file_manager_picker_commit(commit),
            Err(status) => self.shell_status = status,
        }
    }

    fn apply_file_manager_desktop_footer_action(&mut self, action: FileManagerDesktopFooterAction) {
        match file_manager_desktop::resolve_footer_action(action) {
            FileManagerDesktopFooterRequest::RunCommand(command) => {
                self.run_file_manager_command(command);
            }
            FileManagerDesktopFooterRequest::NewDocument => self.new_document(),
            FileManagerDesktopFooterRequest::CompleteSaveAs => {
                self.complete_editor_save_as_from_picker()
            }
            FileManagerDesktopFooterRequest::CancelSavePicker => {
                self.editor.save_as_input = None;
                self.editor.status = "Save canceled.".to_string();
                self.file_manager.open = false;
                self.open_desktop_window(DesktopWindow::Editor);
            }
            FileManagerDesktopFooterRequest::CommitIconPicker => {
                let pick_mode = self
                    .picking_icon_for_shortcut
                    .map(FileManagerPickMode::ShortcutIcon)
                    .unwrap_or(FileManagerPickMode::None);
                self.commit_file_manager_picker(pick_mode);
            }
            FileManagerDesktopFooterRequest::CancelIconPicker => {
                self.picking_icon_for_shortcut = None;
            }
            FileManagerDesktopFooterRequest::CommitWallpaperPicker => {
                self.commit_file_manager_picker(FileManagerPickMode::Wallpaper);
            }
            FileManagerDesktopFooterRequest::CancelWallpaperPicker => {
                self.picking_wallpaper = false;
            }
        }
    }


    fn handle_desktop_file_manager_shortcuts(&mut self, ctx: &Context) {
        if self.active_window_kind() != Some(DesktopWindow::FileManager)
            || self.terminal_prompt.is_some()
        {
            return;
        }
        if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(Key::C)) {
            self.run_file_manager_command(FileManagerCommand::Copy);
        } else if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(Key::X)) {
            self.run_file_manager_command(FileManagerCommand::Cut);
        } else if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(Key::V)) {
            self.run_file_manager_command(FileManagerCommand::Paste);
        } else if ctx.input(|i| i.modifiers.ctrl && i.modifiers.shift && i.key_pressed(Key::N)) {
            self.run_file_manager_command(FileManagerCommand::NewFolder);
        } else if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(Key::D)) {
            self.run_file_manager_command(FileManagerCommand::Duplicate);
        } else if ctx.input(|i| i.key_pressed(Key::F2)) {
            self.run_file_manager_command(FileManagerCommand::Rename);
        } else if ctx.input(|i| i.modifiers.ctrl && i.modifiers.shift && i.key_pressed(Key::M)) {
            self.run_file_manager_command(FileManagerCommand::Move);
        } else if ctx.input(|i| i.key_pressed(Key::Delete)) {
            self.run_file_manager_command(FileManagerCommand::Delete);
        } else if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(Key::Z)) {
            self.run_file_manager_command(FileManagerCommand::Undo);
        } else if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(Key::Y)) {
            self.run_file_manager_command(FileManagerCommand::Redo);
        }
    }

    fn process_desktop_pty_input_early(&mut self, ctx: &Context) {
        let mut early_pty_close = false;
        if self.desktop_mode_open && self.active_window_kind() == Some(DesktopWindow::PtyApp) {
            if let Some(state) = self.terminal_pty.as_mut() {
                if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(Key::Q)) {
                    early_pty_close = true;
                }
                if ctx.input(|i| i.modifiers.ctrl && i.modifiers.shift && i.key_pressed(Key::P)) {
                    state.show_perf_overlay = !state.show_perf_overlay;
                }
                handle_pty_input(ctx, &mut state.session);
                ctx.input_mut(|i| {
                    i.events.retain(|e| {
                        !matches!(
                            e,
                            egui::Event::Key { .. } | egui::Event::Text(_) | egui::Event::Paste(_)
                        )
                    });
                });
            }
        }
        if early_pty_close {
            if let Some(mut pty) = self.terminal_pty.take() {
                pty.session.terminate();
            }
            self.update_desktop_window_state(DesktopWindow::PtyApp, false);
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
        self.process_background_results(ctx);
        self.process_ipc_messages(ctx);

        // Process PTY keyboard input at the very top of the frame, before
        // any egui widgets render. Widgets can otherwise consume key events
        // before the PTY sees them.
        self.process_desktop_pty_input_early(ctx);
        self.maybe_sync_settings_from_disk(ctx);
        self.sync_native_appearance(ctx);

        if let Some(flash) = &self.terminal_flash {
            if Instant::now() >= flash.until {
                let action = flash.action.clone();
                self.terminal_flash = None;
                match action {
                    FlashAction::Noop => {}
                    FlashAction::ExitApp => {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                    FlashAction::FinishLogout => self.finish_logout(),
                    FlashAction::FinishLogin { username, user } => {
                        self.ensure_login_session_entry(&username);
                        self.restore_for_user(&username, &user);
                    }
                    _ => {
                        if let Some(plan) =
                            resolve_terminal_flash_action(&action, self.live_hacking_difficulty)
                        {
                            self.apply_terminal_flash_action_plan(plan);
                        }
                    }
                }
            } else {
                ctx.request_repaint_after(flash.until.saturating_duration_since(Instant::now()));
                let layout = self.terminal_layout();
                self.draw_terminal_status_bar(ctx);
                let show_hacking_wait = self.session.is_none()
                    && matches!(self.login.mode, TerminalLoginScreenMode::Hacking)
                    && matches!(&flash.action, FlashAction::FinishLogin { .. });
                if show_hacking_wait {
                    self.draw_login(ctx);
                    return;
                }
                if flash.boxed {
                    draw_terminal_flash_boxed(
                        ctx,
                        &flash.message,
                        layout.cols,
                        layout.rows,
                        layout.header_start_row,
                        layout.separator_top_row,
                        layout.separator_bottom_row,
                    );
                } else {
                    draw_terminal_flash(
                        ctx,
                        &flash.message,
                        layout.cols,
                        layout.rows,
                        layout.header_start_row,
                        layout.separator_top_row,
                        layout.separator_bottom_row,
                        layout.status_row,
                        layout.content_col,
                    );
                }
                return;
            }
        }

        self.maybe_write_startup_profile_markers();
        self.maybe_trace_repaint_causes(ctx);

        if self.session.is_none() {
            self.draw_terminal_status_bar(ctx);
            self.draw_login(ctx);
            return;
        }

        if !self.desktop_mode_open {
            self.capture_session_switch_shortcuts(ctx);
            if has_native_pending_session_switch() {
                self.apply_pending_session_switch();
            }
        }

        self.dispatch_context_menu_action(ctx);

        if !self.desktop_mode_open
            && !matches!(self.terminal_nav.screen, TerminalScreen::PtyApp)
            && !self.editor.open
            && ctx.input(|i| i.key_pressed(Key::Escape) || i.key_pressed(Key::Tab))
        {
            self.handle_terminal_back();
        }

        if self.terminal_prompt.is_some() {
            self.handle_terminal_prompt_input(ctx);
            self.consume_terminal_prompt_keys(ctx);
        }

        if self.desktop_mode_open {
            if ctx.input(|i| i.key_pressed(Key::Escape)) {
                let had_overlay = self.start_open || self.spotlight_open;
                if had_overlay {
                    self.close_desktop_overlays();
                }
            }
            self.handle_start_menu_keyboard(ctx);
            self.handle_desktop_file_manager_shortcuts(ctx);
            self.draw_top_bar(ctx);
            self.draw_desktop_taskbar(ctx);
            self.draw_desktop(ctx);
        } else {
            self.draw_terminal_status_bar(ctx);
            if self.terminal_nav.suppress_next_menu_submit {
                ctx.input_mut(|i| {
                    i.consume_key(egui::Modifiers::NONE, Key::Enter);
                    i.consume_key(egui::Modifiers::NONE, Key::Space);
                });
                self.terminal_nav.suppress_next_menu_submit = false;
            }
            match self.terminal_nav.screen {
                TerminalScreen::MainMenu => self.draw_terminal_main_menu(ctx),
                TerminalScreen::Applications => self.draw_terminal_applications(ctx),
                TerminalScreen::Documents => self.draw_terminal_documents(ctx),
                TerminalScreen::Logs => self.draw_terminal_logs(ctx),
                TerminalScreen::Network => self.draw_terminal_network(ctx),
                TerminalScreen::Games => self.draw_terminal_games(ctx),
                TerminalScreen::DonkeyKong => self.draw_terminal_donkey_kong(ctx),
                TerminalScreen::NukeCodes => self.draw_terminal_nuke_codes(ctx),
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
        if self.desktop_mode_open {
            self.draw_desktop_windows(ctx);
            self.draw_start_panel(ctx);
            self.draw_start_menu_rename_window(ctx);
            self.draw_spotlight(ctx);
        } else {
            self.draw_file_manager(ctx);
            self.draw_editor(ctx);
            self.draw_settings(ctx);
            self.draw_applications(ctx);
            self.draw_terminal_mode(ctx);
        }
        self.draw_shortcut_properties_window(ctx);
        self.draw_desktop_item_properties_window(ctx);
        self.draw_editor_save_as_window(ctx);
        self.draw_terminal_prompt_overlay_global(ctx);

        if ctx.input(|i| i.viewport().close_requested()) {
            self.persist_snapshot();
        }

        if self.session.is_some() && self.editor.open && self.editor.dirty {
            egui::Area::new(Id::new("native_unsaved_badge"))
                .anchor(Align2::RIGHT_BOTTOM, [-16.0, -16.0])
                .show(ctx, |ui| {
                    ui.label(RichText::new("Unsaved changes").color(Color32::LIGHT_RED));
                });
        }

        // Schedule an idle repaint so the clock, settings-sync, and IPC
        // polling still work when nothing else requests a faster repaint.
        // Active subsystems (PTY, games, flash animations) already call
        // ctx.request_repaint() or request_repaint_after() with shorter
        // intervals, which takes precedence over this.
        ctx.request_repaint_after(Duration::from_millis(500));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{FileManagerSortMode, FileManagerViewMode};
    use crate::core::auth::{load_users, save_users, AuthMethod, UserRecord};
    use crate::native::file_manager_app::FileManagerClipboardMode;
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

    fn terminal_submenu_screens() -> [TerminalScreen; 13] {
        [
            TerminalScreen::Applications,
            TerminalScreen::Documents,
            TerminalScreen::Network,
            TerminalScreen::Games,
            TerminalScreen::NukeCodes,
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
    fn nuke_codes_screen_state_restores_across_session_switch() {
        let _guard = session_test_guard();
        let _users = install_test_users(&["u1", "u2"]);
        session::clear_sessions();
        session::take_switch_request();

        let mut app = RobcoNativeApp::default();
        let s1 = session::push_session("u1");
        let s2 = session::push_session("u2");

        session::set_active(s1);
        assert!(app.sync_active_session_identity());
        app.terminal_nav.screen = TerminalScreen::NukeCodes;
        app.terminal_nav.nuke_codes_return_screen = TerminalScreen::Applications;
        app.terminal_nuke_codes =
            NukeCodesView::Data(robcos_native_nuke_codes_app::NukeCodesData {
                alpha: "11111111".to_string(),
                bravo: "22222222".to_string(),
                charlie: "33333333".to_string(),
                source: "Test Source".to_string(),
                fetched_at: "2026-03-01 06:00 PM".to_string(),
            });
        app.park_active_session_runtime();

        session::set_active(s2);
        assert!(app.sync_active_session_identity());
        app.terminal_nav.screen = TerminalScreen::MainMenu;
        app.terminal_nav.nuke_codes_return_screen = TerminalScreen::MainMenu;
        app.terminal_nuke_codes = NukeCodesView::Error("offline".to_string());
        app.park_active_session_runtime();

        session::request_switch(s1);
        app.apply_pending_session_switch();
        assert_eq!(session::active_idx(), s1);
        assert_eq!(app.terminal_nav.screen, TerminalScreen::NukeCodes);
        assert_eq!(
            app.terminal_nav.nuke_codes_return_screen,
            TerminalScreen::Applications
        );
        match &app.terminal_nuke_codes {
            NukeCodesView::Data(data) => {
                assert_eq!(data.alpha, "11111111");
                assert_eq!(data.bravo, "22222222");
                assert_eq!(data.charlie, "33333333");
            }
            other => panic!("expected NukeCodes data, got {other:?}"),
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
    fn closing_desktop_editor_resets_document_for_next_launch() {
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
        app.execute_desktop_shell_action(DesktopShellAction::RevealPathInFileManager(
            file_path.clone(),
        ));

        assert!(app.file_manager.open);
        assert_eq!(app.file_manager.cwd, temp.path);
        assert_eq!(app.file_manager.selected, Some(file_path));
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

        assert_eq!(app.desktop_active_window, Some(WindowInstanceId::primary(DesktopWindow::Editor)));
        assert!(!app.spotlight_open);
    }

    #[test]
    fn reopening_settings_window_reprimes_component_state() {
        let mut app = RobcoNativeApp::default();
        app.settings.open = true;
        let state = app.desktop_window_state_mut(WindowInstanceId::primary(DesktopWindow::Settings));
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
        assert_eq!(state.restore_pos, None);
        assert_eq!(state.restore_size, None);
        assert!(!state.apply_restore);
        assert!(!state.maximized);
        assert!(!state.minimized);
        assert!(!state.user_resized);
        assert_ne!(state.generation, 7);
    }

    #[test]
    fn opening_closed_installer_window_clears_stale_restore_state() {
        let mut app = RobcoNativeApp::default();
        let state = app.desktop_window_state_mut(WindowInstanceId::primary(DesktopWindow::Installer));
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
        let state = app.desktop_window_state_mut(WindowInstanceId::primary(DesktopWindow::Settings));
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
            &DesktopMenuAction::ActivateDesktopWindow(WindowInstanceId::primary(DesktopWindow::FileManager)),
        );

        assert_eq!(app.desktop_active_window, Some(WindowInstanceId::primary(DesktopWindow::FileManager)));
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
            &DesktopMenuAction::ActivateTaskbarWindow(WindowInstanceId::primary(DesktopWindow::FileManager)),
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
            &DesktopMenuAction::ActivateTaskbarWindow(WindowInstanceId::primary(DesktopWindow::FileManager)),
        );

        assert!(!app.desktop_window_is_minimized(DesktopWindow::FileManager));
        assert_eq!(app.desktop_active_window, Some(WindowInstanceId::primary(DesktopWindow::FileManager)));
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

        assert_eq!(app.desktop_active_window, Some(WindowInstanceId::primary(DesktopWindow::Settings)));
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
