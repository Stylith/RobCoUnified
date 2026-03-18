use super::about_screen::{draw_about_screen, TerminalAboutRequest};
use super::connections_screen::{
    apply_search_query as apply_connection_search_query, draw_terminal_connections_screen,
    resolve_terminal_connections_request, TerminalConnectionsRequest, TerminalConnectionsState,
};
use super::data::{home_dir_fallback, logs_dir, save_text_file, word_processor_dir};
use super::default_apps_screen::{draw_default_apps_screen, TerminalDefaultAppsRequest};
use super::desktop_app::{
    build_active_desktop_menu_section, build_app_control_menu, build_shared_desktop_menu_section,
    build_taskbar_entries, build_window_menu_section, desktop_app_menu_name,
    desktop_component_binding, desktop_component_spec, desktop_components, hosted_app_for_window,
    DesktopHostedApp, DesktopMenuAction, DesktopMenuBuildContext, DesktopMenuItem,
    DesktopMenuSection, DesktopShellAction, DesktopWindow, DesktopWindowMenuEntry,
};
use super::desktop_connections_service::{
    connect_connection_and_refresh_settings, connection_requires_password,
    connections_macos_disabled, connections_macos_disabled_hint, discovered_connection_label,
    forget_saved_connection_and_refresh_settings, saved_connection_label,
    saved_connections_for_kind, scan_discovered_connections, DiscoveredConnection,
};
use super::desktop_default_apps_service::{
    apply_default_app_binding, binding_label_for_slot, default_app_slot_label,
    resolve_custom_default_app_binding, DefaultAppSlot,
};
use super::desktop_documents_service::{
    add_document_category as add_desktop_document_category,
    delete_document_category as delete_desktop_document_category, document_category_names,
    document_category_path, rename_document_category as rename_desktop_document_category,
};
use super::desktop_file_service::{
    load_text_document, open_directory_location, FileManagerLocation,
};
use super::desktop_launcher_service::{
    add_catalog_entry, catalog_names, delete_catalog_entry, parse_catalog_command_line,
    rename_catalog_entry, resolve_catalog_launch, ProgramCatalog,
};
use super::desktop_search_service::{
    gather_spotlight_results, spotlight_category_tag, start_application_entries,
    start_document_entries, start_game_entries, start_network_entries, NativeSpotlightCategory,
    NativeSpotlightResult, NativeStartLeafAction, NativeStartLeafEntry,
};
#[cfg(test)]
use super::desktop_session_service::active_session_identity;
use super::desktop_session_service::{
    active_session_index as active_native_session_index,
    active_session_username as active_native_session_username, apply_session_switch,
    authenticate_login, bind_login_identity, clear_all_sessions as clear_native_sessions,
    close_active_session as close_native_session,
    ensure_login_session_entry as ensure_native_login_session_entry, hacking_start_flash_plan,
    has_pending_session_switch as has_native_pending_session_switch, login_flash_plan,
    last_session_username, login_selection_auth_method, login_usernames as load_login_usernames,
    logout_flash_plan,
    persist_shell_snapshot as persist_native_shell_snapshot,
    request_session_switch as request_native_session_switch,
    restore_current_user_from_last_session,
    restore_session_plan as build_native_session_restore_plan,
    session_count as native_session_count, session_tabs as native_session_tabs,
    take_pending_session_switch as take_native_pending_session_switch,
    user_record as session_user_record, NativePendingSessionSwitch, NativeSessionFlashPlan,
};
use super::desktop_settings_service::{
    apply_file_manager_display_settings_update as apply_desktop_file_manager_display_settings_update,
    apply_file_manager_settings_update as apply_desktop_file_manager_settings_update,
    cycle_hacking_difficulty_in_settings, load_settings_snapshot, persist_settings_draft,
    pty_force_render_mode as desktop_pty_force_render_mode,
    pty_profile_for_command as desktop_pty_profile_for_command, reload_settings_snapshot,
};
use super::desktop_shortcuts_service::{
    create_shortcut_from_start_action, delete_shortcut as delete_desktop_shortcut,
    set_shortcut_icon as set_desktop_shortcut_icon,
    shortcut_launch_command as desktop_shortcut_launch_command, sort_shortcuts,
    toggle_snap_to_grid as toggle_desktop_snap_to_grid,
    update_shortcut_properties as update_desktop_shortcut_properties, ShortcutPropertiesUpdate,
};
use super::desktop_status_service::{
    cancelled_shell_status, clear_settings_status, clear_shell_status,
    invalid_input_settings_status, invalid_input_shell_status, mirror_shell_to_settings,
    saved_settings_status, saved_shell_status, settings_status, shell_status, NativeStatusUpdate,
    NativeStatusValue,
};
use super::desktop_surface_service::{
    build_default_desktop_icon_positions, desktop_builtin_icons, finalize_dragged_icon_position,
    icon_position, load_desktop_surface_entries, set_builtin_icon_visible, set_desktop_icon_style,
    set_wallpaper_path as set_desktop_wallpaper_path,
    set_wallpaper_size_mode as set_desktop_wallpaper_size_mode, update_dragged_icon_position,
    wallpaper_browser_start_dir, DesktopBuiltinIconKind, DesktopIconDragGrid,
    DesktopIconGridLayout, DesktopSurfaceEntry,
};
use super::desktop_user_service::{
    create_user as create_desktop_user, delete_user as delete_desktop_user, sorted_user_records,
    sorted_usernames, toggle_user_admin as toggle_desktop_user_admin, update_user_auth_method,
    user_auth_method_label, user_exists,
};
use super::document_browser::{
    activate_browser_selection, draw_terminal_document_browser, TerminalDocumentBrowserRequest,
};
use super::donkey_kong::{
    input_from_ctx as donkey_kong_input_from_ctx, DonkeyKongConfig, DonkeyKongGame,
    DonkeyKongTheme, BUILTIN_DONKEY_KONG_GAME,
};
use super::edit_menus_screen::{
    draw_edit_menus_screen, EditMenuTarget, EditMenusEntries, TerminalEditMenusRequest,
    TerminalEditMenusState,
};
use super::editor_app::{
    EditorCommand, EditorTextAlign, EditorTextCommand, EditorWindow, EDITOR_APP_TITLE,
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
    add_package_to_menu, apply_filter as apply_installer_filter,
    apply_search_query as apply_installer_search_query, available_runtime_tools,
    build_package_command, cached_package_description as installer_cached_package_description,
    draw_installer_screen, runtime_tool_action_for_selection, runtime_tool_actions,
    runtime_tool_description,
    runtime_tool_installed_cached as installer_runtime_tool_installed_cached, runtime_tool_pkg,
    runtime_tool_title, settle_view_after_package_command, DesktopInstallerConfirm,
    DesktopInstallerEvent, DesktopInstallerNotice, DesktopInstallerState, DesktopInstallerView,
    InstallerCategory, InstallerEvent, InstallerMenuTarget, InstallerPackageAction,
    TerminalInstallerState,
};
use super::menu::{
    draw_terminal_menu_screen, handle_user_management_selection, login_menu_rows_from_users,
    plan_user_management_action, resolve_create_username_prompt, resolve_desktop_pty_exit,
    resolve_embedded_pty_exit, resolve_hacking_screen_event, resolve_login_password_submission,
    resolve_login_selection_plan, resolve_main_menu_action, resolve_terminal_back_action,
    resolve_terminal_flash_action, resolve_user_password_confirm_prompt,
    resolve_user_password_first_prompt, terminal_command_launch_plan, terminal_runtime_defaults,
    terminal_screen_open_plan, terminal_settings_refresh_plan, terminal_shell_launch_plan,
    user_management_screen_for_mode, MainMenuSelectionAction, TerminalBackAction,
    TerminalBackContext, TerminalDesktopPtyExitPlan, TerminalEmbeddedPtyExitPlan,
    TerminalFlashActionPlan, TerminalFlashPtyLaunchPlan, TerminalHackingPlan,
    TerminalHackingUiEvent, TerminalLoginPasswordPlan, TerminalLoginScreenMode,
    TerminalLoginSelectionPlan, TerminalLoginState, TerminalLoginSubmitAction,
    TerminalNavigationState, TerminalPtyLaunchPlan, TerminalScreen, TerminalScreenOpenPlan,
    TerminalSelectionIndexTarget, TerminalShellSurface, TerminalUserManagementPromptPlan,
    TerminalUserPasswordFlow, UserManagementExecutionPlan, UserManagementMode,
};
use super::nuke_codes_screen::{
    draw_nuke_codes_screen, fetch_nuke_codes, NukeCodesEvent, NukeCodesView,
};
use super::programs_screen::draw_programs_menu;
use super::prompt::{
    draw_terminal_flash, draw_terminal_flash_boxed, draw_terminal_prompt_overlay, FlashAction,
    TerminalFlash, TerminalPrompt, TerminalPromptAction, TerminalPromptKind,
};
use super::prompt_flow::{handle_prompt_input, PromptOutcome};
use super::pty_screen::{
    draw_embedded_pty, draw_embedded_pty_in_ui_focused, handle_pty_input,
    spawn_embedded_pty_with_options, NativePtyState, PtyScreenEvent, TERMINAL_MODE_PTY_CELL_H,
    TERMINAL_MODE_PTY_CELL_W,
};
use super::retro_ui::{
    configure_visuals, configure_visuals_for_settings, current_palette, RetroPalette, RetroScreen,
    FIXED_PTY_CELL_H, FIXED_PTY_CELL_W,
};
use super::settings_screen::{run_terminal_settings_screen, TerminalSettingsEvent};
use super::settings_standalone::standalone_settings_panel_arg;
use super::shell_screen::{draw_login_screen, draw_main_menu_screen};
use super::standalone_launcher::{launch_standalone_app, StandaloneNativeApp};
use crate::config::{
    desktop_dir as robco_desktop_dir, get_current_user, global_settings_file, user_dir,
    CliAcsMode, CliColorMode, DesktopFileManagerSettings, DesktopIconSortMode, DesktopIconStyle,
    HackingDifficulty, NativeStartupWindowMode, OpenMode, Settings, WallpaperSizeMode,
    CUSTOM_THEME_NAME, THEMES,
};
use crate::config::{ConnectionKind, SavedConnection};
use crate::core::auth::{AuthMethod, UserRecord};
use crate::session;
use anyhow::Result;
use chrono::{Local, Timelike};
use eframe::egui::{
    self, Align2, Color32, Context, FontData, FontDefinitions, FontFamily, FontId, Id, Key, Layout,
    Modifiers, RichText, TextEdit, TextStyle, TextureHandle, TopBottomPanel,
};
use robcos_native_default_apps_app::{
    build_default_app_settings_choices, default_app_slot_description,
};
use robcos_native_file_manager_app::FileManagerAction;
use robcos_native_programs_app::{
    build_desktop_applications_sections, build_terminal_application_entries,
    build_terminal_game_entries, resolve_desktop_applications_request,
    resolve_desktop_games_request, resolve_terminal_applications_request,
    resolve_terminal_catalog_request, resolve_terminal_games_request, DesktopApplicationsSections,
    DesktopProgramRequest, TerminalProgramRequest,
};
use robcos_native_settings_app::{
    build_desktop_settings_ui_defaults, desktop_settings_back_target,
    desktop_settings_connections_nav_items, desktop_settings_default_panel,
    desktop_settings_home_rows, desktop_settings_user_management_nav_items, gui_cli_profile_mut,
    gui_cli_profile_slot_label, gui_cli_profile_slots, settings_panel_title, GuiCliProfileSlot,
    NativeSettingsPanel, SettingsHomeTile, SettingsHomeTileAction, TerminalSettingsPanel,
};
use std::collections::{HashMap, HashSet};
use std::ffi::OsString;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::SystemTime;
use std::time::{Duration, Instant};

mod file_manager_desktop_presenter;

#[derive(Debug, Clone)]
struct SessionState {
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
enum ContextMenuAction {
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

#[derive(Debug, Clone, Copy, Default)]
struct DesktopWindowState {
    minimized: bool,
    maximized: bool,
    restore_pos: Option<[f32; 2]>,
    restore_size: Option<[f32; 2]>,
    user_resized: bool,
    apply_restore: bool,
    generation: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DesktopHeaderAction {
    None,
    Minimize,
    ToggleMaximize,
    Close,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DesktopWindowRectTracking {
    FullRect,
    PositionOnly,
}

#[derive(Debug, Clone, Copy)]
struct ResizableDesktopWindowOptions {
    min_size: egui::Vec2,
    default_size: egui::Vec2,
    default_pos: Option<egui::Pos2>,
    clamp_restore: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StartSubmenu {
    System,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StartLeaf {
    Applications,
    Documents,
    Network,
    Games,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StartSystemAction {
    ProgramInstaller,
    Terminal,
    FileManager,
    Settings,
    Connections,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StartRootAction {
    ReturnToTerminal,
    Logout,
    Shutdown,
}

const START_ROOT_ITEMS: [&str; 8] = [
    "Applications",
    "Documents",
    "Network",
    "Games",
    "System",
    "Return To Terminal Mode",
    "Logout",
    "Shutdown",
];

const START_ROOT_VIS_ROWS: [Option<usize>; 9] = [
    Some(0),
    Some(1),
    Some(2),
    Some(3),
    Some(4),
    None,
    Some(5),
    Some(6),
    Some(7),
];

const START_SYSTEM_ITEMS: [(&str, StartSystemAction); 5] = [
    ("Program Installer", StartSystemAction::ProgramInstaller),
    ("Terminal", StartSystemAction::Terminal),
    ("File Manager", StartSystemAction::FileManager),
    ("Settings", StartSystemAction::Settings),
    ("Connections", StartSystemAction::Connections),
];

fn start_root_leaf_for_idx(idx: usize) -> Option<StartLeaf> {
    match idx {
        0 => Some(StartLeaf::Applications),
        1 => Some(StartLeaf::Documents),
        2 => Some(StartLeaf::Network),
        3 => Some(StartLeaf::Games),
        _ => None,
    }
}

fn start_root_submenu_for_idx(idx: usize) -> Option<StartSubmenu> {
    if idx == 4 {
        Some(StartSubmenu::System)
    } else {
        None
    }
}

fn start_root_action_for_idx(idx: usize) -> Option<StartRootAction> {
    match idx {
        5 => Some(StartRootAction::ReturnToTerminal),
        6 => Some(StartRootAction::Logout),
        7 => Some(StartRootAction::Shutdown),
        _ => None,
    }
}

const BUILTIN_NUKE_CODES_APP: &str = "Nuke Codes";
const BUILTIN_TEXT_EDITOR_APP: &str = EDITOR_APP_TITLE;

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
const SESSION_LEADER_WINDOW: Duration = Duration::from_millis(1200);

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
    desktop_window_states: HashMap<DesktopWindow, DesktopWindowState>,
    desktop_active_window: Option<DesktopWindow>,
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
}

struct ParkedSessionState {
    file_manager: NativeFileManagerState,
    editor: EditorWindow,
    settings: SettingsWindow,
    applications: ApplicationsWindow,
    donkey_kong_window: DonkeyKongWindow,
    donkey_kong: Option<DonkeyKongGame>,
    desktop_nuke_codes_open: bool,
    desktop_installer: DesktopInstallerState,
    terminal_mode: TerminalModeWindow,
    desktop_window_states: HashMap<DesktopWindow, DesktopWindowState>,
    desktop_active_window: Option<DesktopWindow>,
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

    fn invalidate_desktop_icon_layout_cache(&mut self) {
        self.desktop_icon_layout_cache = None;
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

    fn default_desktop_icon_positions(
        &mut self,
        layout: DesktopIconGridLayout,
        desktop_entries: &[DesktopSurfaceEntry],
    ) -> Arc<HashMap<String, [f32; 2]>> {
        let desktop_entry_keys = Arc::new(
            desktop_entries
                .iter()
                .map(|entry| entry.key.clone())
                .collect::<Vec<_>>(),
        );
        let needs_rebuild = self.desktop_icon_layout_cache.as_ref().is_none_or(|cache| {
            cache.layout != layout
                || cache.desktop_entry_keys.as_ref() != desktop_entry_keys.as_ref()
        });
        if needs_rebuild {
            let positions = Arc::new(build_default_desktop_icon_positions(
                layout,
                self.settings.draft.desktop_icon_sort,
                &self.settings.draft.desktop_hidden_builtin_icons,
                desktop_entries,
                &self.settings.draft.desktop_shortcuts,
            ));
            self.desktop_icon_layout_cache = Some(DesktopIconLayoutCache {
                layout,
                desktop_entry_keys,
                positions,
            });
        }
        self.desktop_icon_layout_cache
            .as_ref()
            .expect("desktop icon layout cache initialized")
            .positions
            .clone()
    }

    fn desktop_surface_entries(&mut self) -> Arc<Vec<DesktopSurfaceEntry>> {
        let dir = robco_desktop_dir();
        let modified = std::fs::metadata(&dir)
            .and_then(|meta| meta.modified())
            .ok();
        let needs_reload = self
            .desktop_surface_entries_cache
            .as_ref()
            .is_none_or(|cache| cache.dir != dir || cache.modified != modified);
        if needs_reload {
            let entries = Arc::new(load_desktop_surface_entries(&dir));
            self.desktop_surface_entries_cache = Some(DesktopSurfaceEntriesCache {
                dir,
                modified,
                entries,
            });
            self.invalidate_desktop_icon_layout_cache();
        }
        self.desktop_surface_entries_cache
            .as_ref()
            .expect("desktop surface cache initialized")
            .entries
            .clone()
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

    fn load_wallpaper_texture(ctx: &Context, path: &str) -> Option<TextureHandle> {
        if path.trim().is_empty() {
            return None;
        }
        let bytes = std::fs::read(path).ok()?;
        let image = image::load_from_memory(&bytes).ok()?.into_rgba8();
        let (width, height) = image.dimensions();
        let mut rgba = Vec::with_capacity((width * height * 4) as usize);
        for pixel in image.pixels() {
            let luma =
                ((pixel[0] as u16 * 77 + pixel[1] as u16 * 150 + pixel[2] as u16 * 29) / 256) as u8;
            rgba.extend_from_slice(&[luma, luma, luma, pixel[3]]);
        }
        let color_image =
            egui::ColorImage::from_rgba_unmultiplied([width as usize, height as usize], &rgba);
        Some(ctx.load_texture(
            "desktop_wallpaper",
            color_image,
            egui::TextureOptions::LINEAR,
        ))
    }

    fn build_asset_cache(ctx: &Context) -> AssetCache {
        const ICON_SIZE: u32 = 64;

        AssetCache {
            icon_settings: Self::load_svg_icon(
                ctx,
                "icon_settings",
                include_bytes!("../Icons/pixel--cog-solid.svg"),
                Some(ICON_SIZE),
            ),
            icon_file_manager: Self::load_svg_icon(
                ctx,
                "icon_file_manager",
                include_bytes!("../Icons/pixel--folder-solid.svg"),
                Some(ICON_SIZE),
            ),
            icon_terminal: Self::load_svg_icon(
                ctx,
                "icon_terminal",
                include_bytes!("../Icons/pixel--code-block-solid.svg"),
                Some(ICON_SIZE),
            ),
            icon_applications: Self::load_svg_icon(
                ctx,
                "icon_applications",
                include_bytes!("../Icons/pixel--grid.svg"),
                Some(ICON_SIZE),
            ),
            icon_installer: Self::load_svg_icon(
                ctx,
                "icon_installer",
                include_bytes!("../Icons/pixel--file-import-solid.svg"),
                Some(ICON_SIZE),
            ),
            icon_nuke_codes: Self::load_svg_icon(
                ctx,
                "icon_nuke_codes",
                include_bytes!("../Icons/pixel--exclamation-triangle-solid.svg"),
                Some(ICON_SIZE),
            ),
            icon_editor: Self::load_svg_icon(
                ctx,
                "icon_editor",
                include_bytes!("../Icons/pixel--pen-solid.svg"),
                Some(ICON_SIZE),
            ),
            icon_general: None,
            icon_appearance: None,
            icon_default_apps: None,
            icon_connections: Self::load_svg_icon(
                ctx,
                "icon_connections",
                include_bytes!("../Icons/pixel--globe.svg"),
                Some(ICON_SIZE),
            ),
            icon_cli_profiles: None,
            icon_edit_menus: None,
            icon_user_management: None,
            icon_about: None,
            icon_folder: None,
            icon_folder_open: None,
            icon_file: None,
            icon_text: None,
            icon_image: None,
            icon_audio: None,
            icon_video: None,
            icon_archive: None,
            icon_app: None,
            icon_shortcut_badge: None,
            icon_gaming: None,
            wallpaper: None,
            wallpaper_loaded_for: String::new(),
        }
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

    fn sync_wallpaper(&mut self, ctx: &Context) {
        let wallpaper_path = self.settings.draft.desktop_wallpaper.as_str();
        if let Some(cache) = &mut self.asset_cache {
            if cache.wallpaper_loaded_for != wallpaper_path {
                cache.wallpaper = Self::load_wallpaper_texture(ctx, wallpaper_path);
                cache.wallpaper_loaded_for.clear();
                cache.wallpaper_loaded_for.push_str(wallpaper_path);
            }
        }
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

    fn draw_wallpaper(
        &self,
        painter: &egui::Painter,
        screen: egui::Rect,
        palette: &RetroPalette,
    ) -> bool {
        let Some(cache) = &self.asset_cache else {
            return false;
        };
        let Some(texture) = &cache.wallpaper else {
            return false;
        };

        let image_size = egui::vec2(texture.size()[0] as f32, texture.size()[1] as f32);
        let uv = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
        let tint = palette.fg;
        match self.settings.draft.desktop_wallpaper_size_mode {
            WallpaperSizeMode::FitToScreen | WallpaperSizeMode::Stretch => {
                painter.image(texture.id(), screen, uv, tint);
            }
            WallpaperSizeMode::Centered => {
                painter.rect_filled(screen, 0.0, palette.bg);
                let origin = screen.center() - image_size * 0.5;
                painter.image(
                    texture.id(),
                    egui::Rect::from_min_size(origin, image_size),
                    uv,
                    tint,
                );
            }
            WallpaperSizeMode::DefaultSize => {
                painter.rect_filled(screen, 0.0, palette.bg);
                painter.image(
                    texture.id(),
                    egui::Rect::from_min_size(screen.min, image_size),
                    uv,
                    tint,
                );
            }
            WallpaperSizeMode::Tile => {
                painter.rect_filled(screen, 0.0, palette.bg);
                let mut y = screen.top();
                while y < screen.bottom() {
                    let mut x = screen.left();
                    while x < screen.right() {
                        painter.image(
                            texture.id(),
                            egui::Rect::from_min_size(egui::pos2(x, y), image_size),
                            uv,
                            tint,
                        );
                        x += image_size.x.max(1.0);
                    }
                    y += image_size.y.max(1.0);
                }
            }
        }
        true
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

    fn apply_global_retro_menu_chrome(ctx: &Context, palette: &RetroPalette) {
        let stroke = egui::Stroke::new(2.0, palette.fg);
        ctx.style_mut(|style| {
            style.visuals.window_stroke = stroke;
            style.visuals.window_rounding = egui::Rounding::ZERO;
            style.visuals.menu_rounding = egui::Rounding::ZERO;
            style.visuals.window_shadow = egui::epaint::Shadow::NONE;
            style.visuals.popup_shadow = egui::epaint::Shadow::NONE;
        });
    }

    fn session_idx_from_digit_key(key: Key) -> Option<usize> {
        match key {
            Key::Num1 => Some(0),
            Key::Num2 => Some(1),
            Key::Num3 => Some(2),
            Key::Num4 => Some(3),
            Key::Num5 => Some(4),
            Key::Num6 => Some(5),
            Key::Num7 => Some(6),
            Key::Num8 => Some(7),
            Key::Num9 => Some(8),
            _ => None,
        }
    }

    fn request_session_switch_if_valid(&mut self, target: usize) -> bool {
        request_native_session_switch(target)
    }

    fn ensure_login_session_entry(&mut self, username: &str) {
        ensure_native_login_session_entry(username);
    }

    fn park_active_session_runtime(&mut self) {
        if self.session.is_none() || native_session_count() == 0 {
            return;
        }
        let Some(idx) = active_native_session_index() else {
            return;
        };
        let parked = ParkedSessionState {
            file_manager: self.file_manager.clone(),
            editor: self.editor.clone(),
            settings: self.settings.clone(),
            applications: self.applications.clone(),
            donkey_kong_window: self.donkey_kong_window.clone(),
            donkey_kong: self.donkey_kong.clone(),
            desktop_nuke_codes_open: self.desktop_nuke_codes_open,
            desktop_installer: std::mem::take(&mut self.desktop_installer),
            terminal_mode: self.terminal_mode.clone(),
            desktop_window_states: self.desktop_window_states.clone(),
            desktop_active_window: self.desktop_active_window,
            desktop_mode_open: self.desktop_mode_open,
            start_root_panel_height: self.start_root_panel_height,
            start_open: self.start_open,
            start_selected_root: self.start_selected_root,
            start_system_selected: self.start_system_selected,
            start_leaf_selected: self.start_leaf_selected,
            start_open_submenu: self.start_open_submenu,
            start_open_leaf: self.start_open_leaf,
            terminal_nav: self.current_terminal_navigation_state(),
            terminal_settings_panel: self.terminal_settings_panel,
            terminal_nuke_codes: self.terminal_nuke_codes.clone(),
            terminal_pty: self.terminal_pty.take(),
            terminal_installer: std::mem::take(&mut self.terminal_installer),
            terminal_edit_menus: std::mem::take(&mut self.terminal_edit_menus),
            terminal_connections: std::mem::take(&mut self.terminal_connections),
            terminal_prompt: self.terminal_prompt.take(),
            terminal_flash: self.terminal_flash.take(),
            session_leader_until: self.session_leader_until.take(),
            desktop_window_generation_seed: self.desktop_window_generation_seed,
            file_manager_runtime: self.file_manager_runtime.clone(),
            shell_status: std::mem::take(&mut self.shell_status),
            start_menu_rename: self.start_menu_rename.take(),
        };
        self.session_runtime.insert(idx, parked);
    }

    #[cfg(test)]
    fn sync_active_session_identity(&mut self) -> bool {
        match active_session_identity() {
            Ok(Some(identity)) => {
                self.session = Some(SessionState {
                    username: identity.username,
                    is_admin: identity.is_admin,
                });
                true
            }
            Ok(None) => {
                self.session = None;
                false
            }
            Err(status) => {
                self.session = None;
                self.shell_status = status;
                false
            }
        }
    }

    fn restore_active_session_runtime_if_any(&mut self) -> bool {
        let Some(idx) = active_native_session_index() else {
            return false;
        };
        let Some(parked) = self.session_runtime.remove(&idx) else {
            return false;
        };
        self.file_manager = parked.file_manager;
        self.editor = parked.editor;
        self.settings = parked.settings;
        self.applications = parked.applications;
        self.donkey_kong_window = parked.donkey_kong_window;
        self.donkey_kong = parked.donkey_kong;
        self.desktop_nuke_codes_open = parked.desktop_nuke_codes_open;
        self.desktop_installer = parked.desktop_installer;
        self.terminal_mode = parked.terminal_mode;
        self.desktop_window_states = parked.desktop_window_states;
        self.desktop_active_window = parked.desktop_active_window;
        self.desktop_mode_open = parked.desktop_mode_open;
        self.start_root_panel_height = parked.start_root_panel_height;
        self.start_open = parked.start_open;
        self.start_selected_root = parked.start_selected_root;
        self.start_system_selected = parked.start_system_selected;
        self.start_leaf_selected = parked.start_leaf_selected;
        self.start_open_submenu = parked.start_open_submenu;
        self.start_open_leaf = parked.start_open_leaf;
        self.apply_terminal_navigation_state(parked.terminal_nav);
        self.terminal_settings_panel = parked.terminal_settings_panel;
        self.terminal_nuke_codes = parked.terminal_nuke_codes;
        self.terminal_pty = parked.terminal_pty;
        self.terminal_installer = parked.terminal_installer;
        self.terminal_edit_menus = parked.terminal_edit_menus;
        self.terminal_connections = parked.terminal_connections;
        self.terminal_prompt = parked.terminal_prompt;
        self.terminal_flash = parked.terminal_flash;
        self.session_leader_until = parked.session_leader_until;
        self.desktop_window_generation_seed = parked.desktop_window_generation_seed;
        self.file_manager_runtime = parked.file_manager_runtime;
        self.context_menu_action = None;
        self.shell_status = parked.shell_status;
        self.start_menu_rename = parked.start_menu_rename;
        true
    }

    fn apply_pending_session_switch(&mut self) {
        let Some(plan) = take_native_pending_session_switch() else {
            return;
        };

        if matches!(plan, NativePendingSessionSwitch::AlreadyActive) {
            return;
        }

        self.persist_snapshot();
        self.park_active_session_runtime();

        let new_session_status = match &plan {
            NativePendingSessionSwitch::OpenNew { new_index, .. } => {
                Some(format!("Switched to session {}.", new_index + 1))
            }
            _ => None,
        };

        match apply_session_switch(&plan) {
            Ok(Some(identity)) => {
                self.session = Some(SessionState {
                    username: identity.username.clone(),
                    is_admin: identity.is_admin,
                });
                if !self.restore_active_session_runtime_if_any() {
                    if let Some(user) = session_user_record(&identity.username) {
                        self.restore_for_user(&identity.username, &user);
                    } else {
                        self.shell_status = format!("Unknown user '{}'.", identity.username);
                        return;
                    }
                }
                if let Some(status) = new_session_status {
                    self.shell_status = status;
                }
            }
            Ok(None) => {}
            Err(status) => {
                self.shell_status = status;
            }
        }
    }

    fn terminate_all_native_pty_children(&mut self) {
        if let Some(mut pty) = self.terminal_pty.take() {
            pty.session.terminate();
        }
        for parked in self.session_runtime.values_mut() {
            if let Some(mut pty) = parked.terminal_pty.take() {
                pty.session.terminate();
            }
        }
    }

    fn close_active_session_window(&mut self) {
        self.persist_snapshot();
        let Some(closing_idx) = active_native_session_index() else {
            return;
        };

        let outcome = match close_native_session() {
            Ok(Some(outcome)) => outcome,
            Ok(None) => return,
            Err(status) => {
                self.shell_status = status;
                return;
            }
        };

        if let Some(mut pty) = self.terminal_pty.take() {
            pty.session.terminate();
        }
        if let Some(mut parked) = self.session_runtime.remove(&closing_idx) {
            if let Some(mut pty) = parked.terminal_pty.take() {
                pty.session.terminate();
            }
        }

        // Session indexes are contiguous; shift parked state keys down after removal.
        let mut remapped = HashMap::new();
        for (idx, parked) in self.session_runtime.drain() {
            let new_idx = if idx > outcome.removed_idx {
                idx - 1
            } else {
                idx
            };
            remapped.insert(new_idx, parked);
        }
        self.session_runtime = remapped;

        if let Some(identity) = outcome.active_identity {
            self.session = Some(SessionState {
                username: identity.username.clone(),
                is_admin: identity.is_admin,
            });
            if !self.restore_active_session_runtime_if_any() {
                if let Some(user) = session_user_record(&identity.username) {
                    self.restore_for_user(&identity.username, &user);
                } else {
                    self.shell_status = format!("Unknown user '{}'.", identity.username);
                    return;
                }
            }
        } else {
            self.session = None;
        }
        self.shell_status = format!("Closed session {}.", outcome.removed_idx + 1);
    }

    fn capture_session_switch_shortcuts(&mut self, ctx: &Context) {
        if self.session.is_none() {
            self.session_leader_until = None;
            return;
        }

        if self
            .session_leader_until
            .is_some_and(|deadline| Instant::now() > deadline)
        {
            self.session_leader_until = None;
        }

        let events = ctx.input(|i| i.events.clone());
        let mut consumed: Vec<(Modifiers, Key)> = Vec::new();
        let mut switch_target: Option<usize> = None;
        let mut close_active = false;
        let now = Instant::now();

        for event in events {
            let egui::Event::Key {
                key,
                pressed: true,
                modifiers,
                ..
            } = event
            else {
                continue;
            };

            if modifiers.ctrl && key == Key::Q {
                self.session_leader_until = Some(now + SESSION_LEADER_WINDOW);
                consumed.push((modifiers, key));
                continue;
            }

            if self.session_leader_until.is_some() {
                // Native session switching is intentionally strict:
                // only Ctrl+Q followed by plain 1..9 (switch) or W/X (close).
                let plain_follow = !modifiers.ctrl && !modifiers.alt && !modifiers.command;
                if plain_follow {
                    if let Some(idx) = Self::session_idx_from_digit_key(key) {
                        switch_target = Some(idx);
                        consumed.push((modifiers, key));
                        self.session_leader_until = None;
                        break;
                    }
                    if matches!(key, Key::W | Key::X) {
                        close_active = true;
                        consumed.push((modifiers, key));
                        self.session_leader_until = None;
                        break;
                    }
                }
                self.session_leader_until = None;
                continue;
            }
        }

        if !consumed.is_empty() {
            ctx.input_mut(|i| {
                for (mods, key) in &consumed {
                    i.consume_key(*mods, *key);
                }
            });
        }

        if close_active {
            self.close_active_session_window();
            return;
        }

        if let Some(target) = switch_target {
            self.request_session_switch_if_valid(target);
        }
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
        self.desktop_active_window = Some(DesktopWindow::Settings);
        self.apply_status_update(clear_settings_status());
    }

    pub(crate) fn update_standalone_settings_window(&mut self, ctx: &Context) {
        self.maybe_sync_settings_from_disk(ctx);
        self.sync_native_appearance(ctx);
        self.dispatch_context_menu_action(ctx);
        if self.terminal_prompt.is_some() {
            self.handle_terminal_prompt_input(ctx);
            self.consume_terminal_prompt_keys(ctx);
        }
        let file_manager_first =
            self.desktop_active_window != Some(DesktopWindow::FileManager) || !self.file_manager.open;
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
        self.desktop_active_window = Some(DesktopWindow::Editor);
    }

    pub(crate) fn update_standalone_editor_window(&mut self, ctx: &Context) {
        self.maybe_sync_settings_from_disk(ctx);
        self.sync_native_appearance(ctx);
        if self.terminal_prompt.is_some() {
            self.handle_terminal_prompt_input(ctx);
            self.consume_terminal_prompt_keys(ctx);
        }
        let file_manager_first =
            self.desktop_active_window != Some(DesktopWindow::FileManager) || !self.file_manager.open;
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
        self.desktop_active_window = Some(DesktopWindow::Applications);
    }

    pub(crate) fn update_standalone_applications_window(&mut self, ctx: &Context) {
        self.maybe_sync_settings_from_disk(ctx);
        self.sync_native_appearance(ctx);
        self.draw_applications(ctx);
        if !self.applications.open {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
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
        self.desktop_active_window = Some(DesktopWindow::NukeCodes);
    }

    pub(crate) fn update_standalone_nuke_codes_window(&mut self, ctx: &Context) {
        self.maybe_sync_settings_from_disk(ctx);
        self.sync_native_appearance(ctx);
        self.draw_nuke_codes_window(ctx);
        if !self.desktop_nuke_codes_open {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
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

    fn desktop_window_is_open(&self, window: DesktopWindow) -> bool {
        (desktop_component_binding(window).is_open)(self)
    }

    fn desktop_window_state(&self, window: DesktopWindow) -> DesktopWindowState {
        self.desktop_window_states
            .get(&window)
            .copied()
            .unwrap_or_default()
    }

    fn desktop_window_state_mut(&mut self, window: DesktopWindow) -> &mut DesktopWindowState {
        self.desktop_window_states.entry(window).or_default()
    }

    fn desktop_window_generation(&self, window: DesktopWindow) -> u64 {
        self.desktop_window_states
            .get(&window)
            .map(|state| state.generation)
            .unwrap_or(0)
    }

    fn desktop_window_egui_id(&self, window: DesktopWindow) -> egui::Id {
        let gen = self.desktop_window_generation(window);
        Id::new((desktop_component_spec(window).id_salt, gen))
    }

    fn next_desktop_window_generation(&mut self) -> u64 {
        let generation = self.desktop_window_generation_seed;
        self.desktop_window_generation_seed =
            self.desktop_window_generation_seed.wrapping_add(1).max(1);
        generation
    }

    fn desktop_window_is_minimized(&self, window: DesktopWindow) -> bool {
        self.desktop_window_is_open(window) && self.desktop_window_state(window).minimized
    }

    fn desktop_window_is_maximized(&self, window: DesktopWindow) -> bool {
        self.desktop_window_is_open(window) && self.desktop_window_state(window).maximized
    }

    fn set_desktop_window_minimized(&mut self, window: DesktopWindow, minimized: bool) {
        if !self.desktop_window_is_open(window) {
            return;
        }
        let state = self.desktop_window_state_mut(window);
        state.minimized = minimized;
        if minimized {
            if self.desktop_active_window == Some(window) {
                self.desktop_active_window = self.first_open_desktop_window();
            }
        } else {
            self.desktop_active_window = Some(window);
        }
    }

    fn take_desktop_window_restore_dims(
        &mut self,
        window: DesktopWindow,
    ) -> Option<(egui::Pos2, egui::Vec2)> {
        let state = self.desktop_window_state_mut(window);
        if state.maximized || !state.apply_restore {
            return None;
        }
        state.apply_restore = false;
        let pos = state.restore_pos?;
        let size = state.restore_size?;
        Some((egui::pos2(pos[0], pos[1]), egui::vec2(size[0], size[1])))
    }

    fn note_desktop_window_rect(&mut self, window: DesktopWindow, rect: egui::Rect) {
        let state = self.desktop_window_state_mut(window);
        state.restore_pos = Some([rect.min.x, rect.min.y]);
        let restore_size = Self::desktop_window_restore_size(rect);
        state.restore_size = Some([restore_size.x, restore_size.y]);
        state.apply_restore = false;
    }

    fn toggle_desktop_window_maximized(
        &mut self,
        window: DesktopWindow,
        current_rect: Option<egui::Rect>,
    ) {
        if !self.desktop_window_is_open(window) {
            return;
        }
        let generation = self.next_desktop_window_generation();
        let state = self.desktop_window_state_mut(window);
        if state.maximized {
            state.maximized = false;
            state.apply_restore = true;
            state.generation = generation;
        } else {
            if let Some(rect) = current_rect {
                state.restore_pos = Some([rect.min.x, rect.min.y]);
                let restore_size = Self::desktop_window_restore_size(rect);
                state.restore_size = Some([restore_size.x, restore_size.y]);
                state.user_resized = true;
            }
            state.maximized = true;
            state.apply_restore = false;
            state.generation = generation;
        }
        state.minimized = false;
        self.desktop_active_window = Some(window);
    }

    fn desktop_window_restore_size(rect: egui::Rect) -> egui::Vec2 {
        let margin = Self::desktop_window_frame().total_margin().sum();
        egui::vec2(
            (rect.width() - margin.x).max(160.0),
            (rect.height() - margin.y).max(120.0),
        )
    }

    fn desktop_default_window_size(window: DesktopWindow) -> egui::Vec2 {
        let [x, y] = desktop_component_spec(window).default_size;
        egui::vec2(x, y)
    }

    fn desktop_file_manager_window_min_size() -> egui::Vec2 {
        egui::vec2(760.0, 520.0)
    }

    fn desktop_default_window_pos(ctx: &Context, size: egui::Vec2) -> egui::Pos2 {
        let workspace = Self::desktop_workspace_rect(ctx);
        let x = workspace.left() + ((workspace.width() - size.x) * 0.5).max(24.0);
        let y = workspace.top() + ((workspace.height() - size.y) * 0.18).max(24.0);
        egui::pos2(x, y)
    }

    fn desktop_clamp_window_size(
        ctx: &Context,
        size: egui::Vec2,
        min_size: egui::Vec2,
    ) -> egui::Vec2 {
        let workspace = Self::desktop_workspace_rect(ctx);
        egui::vec2(
            size.x.clamp(min_size.x, workspace.width().max(min_size.x)),
            size.y.clamp(min_size.y, workspace.height().max(min_size.y)),
        )
    }

    fn desktop_clamp_window_pos(ctx: &Context, pos: egui::Pos2, size: egui::Vec2) -> egui::Pos2 {
        let workspace = Self::desktop_workspace_rect(ctx);
        egui::pos2(
            pos.x.clamp(
                workspace.left(),
                (workspace.right() - size.x).max(workspace.left()),
            ),
            pos.y.clamp(
                workspace.top(),
                (workspace.bottom() - size.y).max(workspace.top()),
            ),
        )
    }

    fn build_resizable_desktop_window<'open, Title>(
        &mut self,
        ctx: &Context,
        desktop_window: DesktopWindow,
        title: Title,
        open: &'open mut bool,
        options: ResizableDesktopWindowOptions,
    ) -> (egui::Window<'open>, bool)
    where
        Title: Into<egui::WidgetText>,
    {
        let maximized = self.desktop_window_is_maximized(desktop_window);
        let restore = self.take_desktop_window_restore_dims(desktop_window);
        let mut window = egui::Window::new(title)
            .id(self.desktop_window_egui_id(desktop_window))
            .open(open)
            .title_bar(false)
            .frame(Self::desktop_window_frame())
            .resizable(true)
            .min_size(options.min_size)
            .default_size(options.default_size);
        if let Some(default_pos) = options.default_pos {
            window = window.default_pos(default_pos);
        }
        if maximized {
            let rect = Self::desktop_workspace_rect(ctx);
            window = window
                .movable(false)
                .resizable(false)
                .fixed_pos(rect.min)
                .fixed_size(rect.size());
        } else if let Some((mut pos, mut size)) = restore {
            if options.clamp_restore {
                size = Self::desktop_clamp_window_size(ctx, size, options.min_size);
                pos = Self::desktop_clamp_window_pos(ctx, pos, size);
            }
            window = window.current_pos(pos).default_size(size);
        }
        (window, maximized)
    }

    fn finish_desktop_window_host(
        &mut self,
        ctx: &Context,
        desktop_window: DesktopWindow,
        open: &mut bool,
        maximized: bool,
        shown_rect: Option<egui::Rect>,
        shown_contains_pointer: bool,
        rect_tracking: DesktopWindowRectTracking,
        header_action: DesktopHeaderAction,
    ) {
        self.maybe_activate_desktop_window_from_click(ctx, desktop_window, shown_contains_pointer);
        if !maximized {
            match rect_tracking {
                DesktopWindowRectTracking::FullRect => {
                    if let Some(rect) = shown_rect {
                        self.note_desktop_window_rect(desktop_window, rect);
                    }
                }
                DesktopWindowRectTracking::PositionOnly => {
                    if let Some(pos) = shown_rect.map(|rect| rect.min) {
                        let state = self.desktop_window_state_mut(desktop_window);
                        state.restore_pos = Some([pos.x, pos.y]);
                    }
                }
            }
        }
        match header_action {
            DesktopHeaderAction::None => {}
            DesktopHeaderAction::Close => *open = false,
            DesktopHeaderAction::Minimize => {
                self.set_desktop_window_minimized(desktop_window, true);
            }
            DesktopHeaderAction::ToggleMaximize => {
                self.toggle_desktop_window_maximized(desktop_window, shown_rect);
            }
        }
        self.update_desktop_window_state(desktop_window, *open);
    }

    fn prime_desktop_window_defaults(&mut self, window: DesktopWindow) {
        let generation = self.next_desktop_window_generation();
        let state = self.desktop_window_state_mut(window);
        state.restore_pos = None;
        state.restore_size = None;
        state.user_resized = false;
        state.apply_restore = false;
        state.maximized = false;
        state.minimized = false;
        state.generation = generation;
    }

    fn set_desktop_window_open(&mut self, window: DesktopWindow, open: bool) {
        let was_open = self.desktop_window_is_open(window);
        (desktop_component_binding(window).set_open)(self, open);
        if !open {
            self.desktop_window_states.remove(&window);
        } else if !was_open && self.desktop_window_is_open(window) {
            let generation = self.next_desktop_window_generation();
            let state = self.desktop_window_state_mut(window);
            state.minimized = false;
            state.maximized = false;
            state.user_resized = false;
            state.generation = generation;
        } else {
            self.desktop_window_states.entry(window).or_default();
        }
    }

    fn first_open_desktop_window(&self) -> Option<DesktopWindow> {
        desktop_components()
            .iter()
            .rev()
            .map(|component| component.spec.window)
            .find(|window| {
                self.desktop_window_is_open(*window) && !self.desktop_window_is_minimized(*window)
            })
    }

    fn focus_desktop_window(&mut self, ctx: Option<&Context>, window: DesktopWindow) {
        self.desktop_active_window = Some(window);
        if let Some(ctx) = ctx {
            let layer_id =
                egui::LayerId::new(egui::Order::Middle, self.desktop_window_egui_id(window));
            ctx.move_to_top(layer_id);
        }
    }

    fn sync_desktop_active_window(&mut self) {
        if self.desktop_active_window.is_some_and(|window| {
            !self.desktop_window_is_open(window) || self.desktop_window_is_minimized(window)
        }) {
            self.desktop_active_window = self.first_open_desktop_window();
            return;
        }
        if self.desktop_active_window.is_none() {
            self.desktop_active_window = self.first_open_desktop_window();
        }
    }

    fn open_desktop_window(&mut self, window: DesktopWindow) {
        let was_open = self.desktop_window_is_open(window);
        if let Some(on_open) = desktop_component_binding(window).on_open {
            on_open(self, was_open);
        }
        self.set_desktop_window_open(window, true);
        self.set_desktop_window_minimized(window, false);
        self.desktop_active_window = Some(window);
        if self.desktop_mode_open {
            self.close_desktop_overlays();
        }
    }

    fn maybe_activate_desktop_window_from_click(
        &mut self,
        ctx: &Context,
        window: DesktopWindow,
        contains_pointer: bool,
    ) {
        let clicked_inside = ctx.input(|i| {
            (i.pointer.primary_clicked() || i.pointer.secondary_clicked()) && contains_pointer
        });
        if clicked_inside {
            self.focus_desktop_window(Some(ctx), window);
        }
    }

    fn handle_closed_desktop_window(&mut self, window: DesktopWindow) {
        if let Some(on_closed) = desktop_component_binding(window).on_closed {
            on_closed(self);
        }
    }

    fn close_desktop_window(&mut self, window: DesktopWindow) {
        let was_open = self.desktop_window_is_open(window);
        self.set_desktop_window_open(window, false);
        if was_open {
            self.handle_closed_desktop_window(window);
        }
        if self.desktop_active_window == Some(window) {
            self.desktop_active_window = self.first_open_desktop_window();
        }
    }

    fn update_desktop_window_state(&mut self, window: DesktopWindow, open: bool) {
        let was_open = self.desktop_window_is_open(window);
        self.set_desktop_window_open(window, open);
        if was_open && !open {
            self.handle_closed_desktop_window(window);
        }
        if !open && self.desktop_active_window == Some(window) {
            self.desktop_active_window = self.first_open_desktop_window();
        }
    }

    fn active_desktop_app(&self) -> DesktopHostedApp {
        hosted_app_for_window(self.desktop_active_window)
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

    fn load_cached_shortcut_icon(
        &mut self,
        ctx: &Context,
        cache_key: &str,
        path: &Path,
        size_px: u32,
    ) -> Option<TextureHandle> {
        if let Some(tex) = self.shortcut_icon_cache.get(cache_key) {
            return Some(tex.clone());
        }
        if self.shortcut_icon_missing.contains(cache_key) {
            return None;
        }
        let bytes = match std::fs::read(path) {
            Ok(bytes) => bytes,
            Err(_) => {
                self.shortcut_icon_missing.insert(cache_key.to_string());
                return None;
            }
        };
        let tex = Self::load_svg_icon(ctx, cache_key, &bytes, Some(size_px));
        self.shortcut_icon_cache
            .insert(cache_key.to_string(), tex.clone());
        Some(tex)
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

    fn import_paths_to_desktop(&mut self, paths: Vec<PathBuf>) {
        let desktop_dir = robco_desktop_dir();
        self.shell_status = match self
            .file_manager_runtime
            .copy_paths_into_dir(paths, &desktop_dir)
        {
            Ok((count, last_dst)) => {
                self.invalidate_desktop_surface_cache();
                if count == 1 {
                    format!(
                        "Imported {} to the desktop.",
                        last_dst
                            .as_ref()
                            .and_then(|path| path.file_name())
                            .and_then(|name| name.to_str())
                            .unwrap_or("item")
                    )
                } else {
                    format!("Imported {count} items to the desktop.")
                }
            }
            Err(err) => format!("Desktop import failed: {err}"),
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

    fn dispatch_context_menu_action(&mut self, _ctx: &Context) {
        let Some(action) = self.context_menu_action.take() else {
            return;
        };
        match action {
            ContextMenuAction::Open => {
                self.run_file_manager_command(FileManagerCommand::OpenSelected)
            }
            ContextMenuAction::OpenWith => {
                if let Some(entry) = self.file_manager_selected_file() {
                    let ext_key = file_manager_app::open_with_extension_key(&entry.path);
                    self.open_file_manager_prompt(FileManagerPromptRequest::open_with_new_command(
                        entry.path, ext_key, false,
                    ));
                } else {
                    self.shell_status = "Open With requires a file.".to_string();
                }
            }
            ContextMenuAction::Rename => self.run_file_manager_command(FileManagerCommand::Rename),
            ContextMenuAction::Cut => self.run_file_manager_command(FileManagerCommand::Cut),
            ContextMenuAction::Copy => self.run_file_manager_command(FileManagerCommand::Copy),
            ContextMenuAction::Paste => self.run_file_manager_command(FileManagerCommand::Paste),
            ContextMenuAction::Duplicate => {
                self.run_file_manager_command(FileManagerCommand::Duplicate)
            }
            ContextMenuAction::Delete => self.run_file_manager_command(FileManagerCommand::Delete),
            ContextMenuAction::Properties => {
                self.shell_status = "Properties dialog is not implemented yet.".to_string();
            }
            ContextMenuAction::PasteToDesktop => {
                self.paste_to_desktop();
            }
            ContextMenuAction::NewFolder => {
                self.create_desktop_folder();
            }
            ContextMenuAction::ChangeAppearance => {
                self.open_standalone_settings(Some(NativeSettingsPanel::Appearance));
            }
            ContextMenuAction::OpenSettings => {
                self.open_standalone_settings(None);
            }
            ContextMenuAction::GenericCopy => {}
            ContextMenuAction::GenericPaste => {}
            ContextMenuAction::GenericSelectAll => {}
            ContextMenuAction::CreateShortcut { label, action } => {
                create_shortcut_from_start_action(&mut self.settings.draft, label, &action);
                self.persist_native_settings();
            }
            ContextMenuAction::RenameStartMenuEntry { target, name } => {
                self.start_menu_rename = Some(StartMenuRenameState {
                    target,
                    original_name: name.clone(),
                    name_input: name,
                });
            }
            ContextMenuAction::RemoveStartMenuEntry { target, name } => {
                self.delete_program_entry(target, &name);
                self.close_start_menu();
            }
            ContextMenuAction::DeleteShortcut(idx) => {
                if delete_desktop_shortcut(&mut self.settings.draft, idx) {
                    self.persist_native_settings();
                }
            }
            ContextMenuAction::SortDesktopIcons(mode) => {
                sort_shortcuts(&mut self.settings.draft, mode);
                self.persist_native_settings();
            }
            ContextMenuAction::ToggleSnapToGrid => {
                toggle_desktop_snap_to_grid(&mut self.settings.draft);
                self.persist_native_settings();
            }
            ContextMenuAction::LaunchShortcut(name) => {
                let custom_cmd = desktop_shortcut_launch_command(&self.settings.draft, &name);
                if let Some(cmd) = custom_cmd {
                    let args: Vec<String> = cmd.split_whitespace().map(|s| s.to_string()).collect();
                    self.open_desktop_pty(&name, &args);
                } else {
                    self.run_start_leaf_action(NativeStartLeafAction::LaunchConfiguredApp(name));
                }
            }
            ContextMenuAction::OpenShortcutProperties(idx) => {
                if let Some(sc) = self.settings.draft.desktop_shortcuts.get(idx) {
                    self.shortcut_properties = Some(ShortcutPropertiesState {
                        shortcut_idx: idx,
                        name_draft: sc.label.clone(),
                        command_draft: sc
                            .launch_command
                            .clone()
                            .unwrap_or_else(|| sc.app_name.clone()),
                        icon_path_draft: sc.icon_path.clone(),
                    });
                }
            }
            ContextMenuAction::OpenDesktopItem(path) => {
                self.open_desktop_surface_path(path);
            }
            ContextMenuAction::OpenDesktopItemWith(path) => {
                self.open_desktop_surface_with_prompt(path);
            }
            ContextMenuAction::RenameDesktopItem(path) => {
                self.rename_desktop_item(path);
            }
            ContextMenuAction::DeleteDesktopItem(path) => {
                self.delete_desktop_item(path);
            }
            ContextMenuAction::OpenDesktopItemProperties(path) => {
                self.open_desktop_item_properties(path);
            }
        }
    }

    fn attach_desktop_empty_context_menu(
        action: &mut Option<ContextMenuAction>,
        response: &egui::Response,
        snap_to_grid: bool,
        sort_mode: DesktopIconSortMode,
    ) {
        response.context_menu(|ui| {
            Self::apply_context_menu_style(ui);
            ui.set_min_width(136.0);
            ui.set_max_width(180.0);

            ui.menu_button("View", |ui| {
                Self::apply_context_menu_style(ui);
                ui.set_min_width(140.0);
                ui.set_max_width(180.0);
                let name_label = if sort_mode == DesktopIconSortMode::ByName {
                    "✓ Sort by Name"
                } else {
                    "  Sort by Name"
                };
                let type_label = if sort_mode == DesktopIconSortMode::ByType {
                    "✓ Sort by Type"
                } else {
                    "  Sort by Type"
                };
                if ui.button(name_label).clicked() {
                    *action = Some(ContextMenuAction::SortDesktopIcons(
                        DesktopIconSortMode::ByName,
                    ));
                    ui.close_menu();
                }
                if ui.button(type_label).clicked() {
                    *action = Some(ContextMenuAction::SortDesktopIcons(
                        DesktopIconSortMode::ByType,
                    ));
                    ui.close_menu();
                }
                Self::retro_separator(ui);
                let snap_label = if snap_to_grid {
                    "✓ Snap to Grid"
                } else {
                    "  Snap to Grid"
                };
                if ui.button(snap_label).clicked() {
                    *action = Some(ContextMenuAction::ToggleSnapToGrid);
                    ui.close_menu();
                }
            });

            Self::retro_separator(ui);

            if ui.button("Paste").clicked() {
                *action = Some(ContextMenuAction::PasteToDesktop);
                ui.close_menu();
            }

            Self::retro_separator(ui);

            if ui.button("New Folder").clicked() {
                *action = Some(ContextMenuAction::NewFolder);
                ui.close_menu();
            }

            Self::retro_separator(ui);

            if ui.button("Change Appearance...").clicked() {
                *action = Some(ContextMenuAction::ChangeAppearance);
                ui.close_menu();
            }
            if ui.button("Settings...").clicked() {
                *action = Some(ContextMenuAction::OpenSettings);
                ui.close_menu();
            }
        });
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

    fn desktop_icon_label_lines(label: &str) -> Vec<String> {
        const MAX_LINE_CHARS: usize = 14;

        if label.chars().count() <= MAX_LINE_CHARS {
            return vec![label.to_string()];
        }
        let words: Vec<&str> = label.split_whitespace().collect();
        if words.len() < 2 {
            return vec![Self::truncate_file_manager_label(label, MAX_LINE_CHARS)];
        }

        let mut first_line = String::new();
        let mut split_idx = 0usize;
        for (idx, word) in words.iter().enumerate() {
            let candidate = if first_line.is_empty() {
                (*word).to_string()
            } else {
                format!("{first_line} {word}")
            };
            if candidate.chars().count() > MAX_LINE_CHARS {
                break;
            }
            first_line = candidate;
            split_idx = idx + 1;
        }
        if first_line.is_empty() {
            return vec![Self::truncate_file_manager_label(label, MAX_LINE_CHARS)];
        }
        if split_idx >= words.len() {
            return vec![first_line];
        }

        let second_line = words[split_idx..].join(" ");
        vec![
            first_line,
            Self::truncate_file_manager_label(&second_line, MAX_LINE_CHARS),
        ]
    }

    fn paint_desktop_icon_label(ui: &mut egui::Ui, rect: egui::Rect, label: &str, color: Color32) {
        let lines = Self::desktop_icon_label_lines(label);
        if lines.len() == 1 {
            ui.painter().text(
                rect.center(),
                Align2::CENTER_CENTER,
                &lines[0],
                FontId::new(13.0, FontFamily::Monospace),
                color,
            );
            return;
        }

        let line_height = 11.0;
        let total_height = line_height * lines.len() as f32;
        let start_y = rect.center().y - total_height * 0.5 + line_height * 0.5;
        for (idx, line) in lines.iter().enumerate() {
            ui.painter().text(
                egui::pos2(rect.center().x, start_y + idx as f32 * line_height),
                Align2::CENTER_CENTER,
                line,
                FontId::new(11.0, FontFamily::Monospace),
                color,
            );
        }
    }

    fn paint_desktop_icon_selection(
        ui: &mut egui::Ui,
        rect: egui::Rect,
        palette: RetroPalette,
        selected: bool,
        hovered: bool,
    ) {
        if !(selected || hovered) {
            return;
        }
        let fill = if selected {
            palette.selected_bg
        } else {
            palette.panel
        };
        let stroke = if selected { palette.fg } else { palette.dim };
        ui.painter().rect_filled(rect.expand(2.0), 0.0, fill);
        ui.painter()
            .rect_stroke(rect.expand(2.0), 0.0, egui::Stroke::new(1.0, stroke));
    }

    fn desktop_icon_foreground(palette: RetroPalette, selected: bool) -> Color32 {
        if selected {
            palette.bg
        } else {
            palette.fg
        }
    }

    fn draw_desktop_icons(&mut self, ui: &mut egui::Ui) {
        let (
            tex_file_manager,
            tex_editor,
            tex_installer,
            tex_settings,
            tex_nuke_codes,
            tex_terminal,
            tex_connections,
        ) = {
            let Some(cache) = self.asset_cache.as_ref() else {
                return;
            };
            (
                cache.icon_file_manager.clone(),
                cache.icon_editor.clone(),
                cache.icon_installer.clone(),
                cache.icon_settings.clone(),
                cache.icon_nuke_codes.clone(),
                cache.icon_terminal.clone(),
                cache.icon_connections.clone(),
            )
        };
        let tex_shortcut_badge = Self::ensure_cached_svg_icon(
            &mut self
                .asset_cache
                .as_mut()
                .expect("desktop asset cache")
                .icon_shortcut_badge,
            ui.ctx(),
            "icon_shortcut_badge",
            include_bytes!("../Icons/pixel--external-link-solid.svg"),
            Some(16),
        );
        let tex_app = Self::ensure_cached_svg_icon(
            &mut self
                .asset_cache
                .as_mut()
                .expect("desktop asset cache")
                .icon_app,
            ui.ctx(),
            "icon_app",
            include_bytes!("../Icons/pixel--programming.svg"),
            Some(64),
        );

        let palette = current_palette();
        let style = self.settings.draft.desktop_icon_style;
        let snap = self.settings.draft.desktop_snap_to_grid;
        let workspace = Self::desktop_workspace_rect(ui.ctx());
        let (icon_size, label_height, item_height, column_width): (f32, f32, f32, f32) = match style
        {
            DesktopIconStyle::Minimal => (34.0, 0.0, 46.0, 48.0),
            DesktopIconStyle::Win95 | DesktopIconStyle::Dos => (48.0, 28.0, 84.0, 100.0),
            DesktopIconStyle::NoIcons => return,
        };

        let drag_grid = DesktopIconDragGrid {
            cell_w: column_width,
            cell_h: item_height,
            snap_to_grid: snap,
        };

        let hidden_icons = self.settings.draft.desktop_hidden_builtin_icons.clone();
        let desktop_entries = self.desktop_surface_entries();
        let shortcuts = self.settings.draft.desktop_shortcuts.clone();
        let builtin_entries = desktop_builtin_icons();
        let default_positions = self.default_desktop_icon_positions(
            DesktopIconGridLayout {
                left: workspace.left(),
                top: workspace.top(),
                height: workspace.height(),
                item_height,
                column_width,
            },
            &desktop_entries,
        );
        let mut open_window: Option<DesktopWindow> = None;
        let mut open_terminal = false;
        let mut open_desktop_path: Option<PathBuf> = None;
        let mut desktop_action: Option<ContextMenuAction> = None;
        let mut needs_persist = false;

        for (index, entry) in builtin_entries.iter().enumerate() {
            if hidden_icons.contains(entry.key) {
                continue;
            }
            let texture = match entry.kind {
                DesktopBuiltinIconKind::FileManager => &tex_file_manager,
                DesktopBuiltinIconKind::Editor => &tex_editor,
                DesktopBuiltinIconKind::Installer => &tex_installer,
                DesktopBuiltinIconKind::Settings => &tex_settings,
                DesktopBuiltinIconKind::NukeCodes => &tex_nuke_codes,
                DesktopBuiltinIconKind::Terminal => &tex_terminal,
            };
            let top_left = {
                let [x, y] = icon_position(
                    &self.settings.draft,
                    entry.key,
                    [
                        workspace.left() + 4.0,
                        workspace.top() + 16.0 + index as f32 * item_height,
                    ],
                    &default_positions,
                );
                egui::pos2(x, y)
            };

            let icon_rect = egui::Rect::from_min_size(
                top_left + egui::vec2((column_width - icon_size) * 0.5, 0.0),
                egui::vec2(icon_size, icon_size),
            );
            let label_rect = egui::Rect::from_min_size(
                top_left + egui::vec2(0.0, icon_size + 2.0),
                egui::vec2(column_width, label_height.max(16.0)),
            );
            let hit_rect = if label_height > 0.0 {
                egui::Rect::from_min_size(
                    top_left,
                    egui::vec2(column_width, icon_size + label_height + 2.0),
                )
            } else {
                icon_rect
            };

            let response = ui.allocate_rect(hit_rect, egui::Sense::click_and_drag());
            let selected =
                self.desktop_selected_icon == Some(DesktopIconSelection::Builtin(entry.key));
            Self::paint_desktop_icon_selection(ui, hit_rect, palette, selected, response.hovered());
            let icon_fg = Self::desktop_icon_foreground(palette, selected);

            match style {
                DesktopIconStyle::Dos => {
                    ui.painter().text(
                        icon_rect.center(),
                        Align2::CENTER_CENTER,
                        entry.ascii,
                        FontId::new(18.0, FontFamily::Monospace),
                        icon_fg,
                    );
                }
                DesktopIconStyle::Minimal | DesktopIconStyle::Win95 => {
                    Self::paint_tinted_texture(ui.painter(), texture, icon_rect, icon_fg);
                }
                DesktopIconStyle::NoIcons => {}
            }

            if label_height > 0.0 {
                Self::paint_desktop_icon_label(ui, label_rect, entry.label, icon_fg);
            }

            if response.dragged() {
                update_dragged_icon_position(
                    &mut self.settings.draft,
                    entry.key,
                    [top_left.x, top_left.y],
                    [response.drag_delta().x, response.drag_delta().y],
                );
            }
            if response.drag_stopped() {
                needs_persist |=
                    finalize_dragged_icon_position(&mut self.settings.draft, entry.key, drag_grid);
            }

            if response.clicked() || response.secondary_clicked() {
                self.desktop_selected_icon = Some(DesktopIconSelection::Builtin(entry.key));
            }
            if response.double_clicked() {
                if let Some(window) = entry.target_window {
                    open_window = Some(window);
                } else {
                    open_terminal = true;
                }
            }
        }

        for (entry_idx, entry) in desktop_entries.iter().enumerate() {
            let entry_key = entry.key.clone();
            let entry_path = entry.path.clone();
            let entry_label = entry.label.clone();
            let entry_is_dir = entry.is_dir();
            let row = Self::desktop_entry_row(entry);
            let top_left = {
                let [x, y] = icon_position(
                    &self.settings.draft,
                    &entry_key,
                    [
                        workspace.left() + 4.0 + column_width,
                        workspace.top()
                            + 16.0
                            + (builtin_entries.len() + entry_idx) as f32 * item_height,
                    ],
                    &default_positions,
                );
                egui::pos2(x, y)
            };

            let icon_rect = egui::Rect::from_min_size(
                top_left + egui::vec2((column_width - icon_size) * 0.5, 0.0),
                egui::vec2(icon_size, icon_size),
            );
            let label_rect = egui::Rect::from_min_size(
                top_left + egui::vec2(0.0, icon_size + 2.0),
                egui::vec2(column_width, label_height.max(16.0)),
            );
            let hit_rect = if label_height > 0.0 {
                egui::Rect::from_min_size(
                    top_left,
                    egui::vec2(column_width, icon_size + label_height + 2.0),
                )
            } else {
                icon_rect
            };

            let response = ui.allocate_rect(hit_rect, egui::Sense::click_and_drag());
            let selected = self.desktop_selected_icon
                == Some(DesktopIconSelection::Surface(entry_key.clone()));
            Self::paint_desktop_icon_selection(ui, hit_rect, palette, selected, response.hovered());
            let icon_fg = Self::desktop_icon_foreground(palette, selected);
            response.dnd_set_drag_payload(NativeFileManagerDragPayload {
                paths: vec![entry_path.clone()],
            });
            let file_manager_drop_hover = entry_is_dir
                && response
                    .dnd_hover_payload::<NativeFileManagerDragPayload>()
                    .is_some_and(|payload| {
                        Self::file_manager_drop_allowed(&payload.paths, &entry_path)
                    });

            match style {
                DesktopIconStyle::Dos => {
                    ui.painter().text(
                        icon_rect.center(),
                        Align2::CENTER_CENTER,
                        row.icon(),
                        FontId::new(18.0, FontFamily::Monospace),
                        icon_fg,
                    );
                }
                DesktopIconStyle::Minimal | DesktopIconStyle::Win95 => {
                    if let Some(texture) = self.file_manager_texture_for_row(ui.ctx(), &row) {
                        Self::paint_tinted_texture(ui.painter(), &texture, icon_rect, icon_fg);
                    }
                }
                DesktopIconStyle::NoIcons => {}
            }

            if file_manager_drop_hover {
                ui.painter().rect_stroke(
                    hit_rect.expand(2.0),
                    0.0,
                    egui::Stroke::new(1.5, palette.fg),
                );
            }

            if label_height > 0.0 {
                Self::paint_desktop_icon_label(ui, label_rect, &entry_label, icon_fg);
            }

            if response.dragged() {
                update_dragged_icon_position(
                    &mut self.settings.draft,
                    &entry_key,
                    [top_left.x, top_left.y],
                    [response.drag_delta().x, response.drag_delta().y],
                );
            }
            if response.drag_stopped() {
                needs_persist |=
                    finalize_dragged_icon_position(&mut self.settings.draft, &entry_key, drag_grid);
            }

            if response.clicked() || response.secondary_clicked() {
                self.desktop_selected_icon = Some(DesktopIconSelection::Surface(entry_key.clone()));
            }

            response.context_menu(|ui| {
                Self::apply_context_menu_style(ui);
                ui.set_min_width(140.0);
                ui.set_max_width(190.0);
                if ui.button("Open").clicked() {
                    desktop_action = Some(ContextMenuAction::OpenDesktopItem(entry_path.clone()));
                    ui.close_menu();
                }
                if !entry_is_dir {
                    if ui.button("Open With...").clicked() {
                        desktop_action =
                            Some(ContextMenuAction::OpenDesktopItemWith(entry_path.clone()));
                        ui.close_menu();
                    }
                }
                Self::retro_separator(ui);
                if ui.button("Rename").clicked() {
                    desktop_action = Some(ContextMenuAction::RenameDesktopItem(entry_path.clone()));
                    ui.close_menu();
                }
                if ui.button("Properties").clicked() {
                    desktop_action = Some(ContextMenuAction::OpenDesktopItemProperties(
                        entry_path.clone(),
                    ));
                    ui.close_menu();
                }
                Self::retro_separator(ui);
                if ui.button("Delete").clicked() {
                    desktop_action = Some(ContextMenuAction::DeleteDesktopItem(entry_path.clone()));
                    ui.close_menu();
                }
            });

            if entry_is_dir {
                if let Some(payload) =
                    response.dnd_release_payload::<NativeFileManagerDragPayload>()
                {
                    if Self::file_manager_drop_allowed(&payload.paths, &entry_path) {
                        self.file_manager_handle_drop_to_dir(
                            payload.paths.clone(),
                            entry_path.clone(),
                        );
                    }
                }
            }

            if response.double_clicked() {
                open_desktop_path = Some(entry_path.clone());
            }
        }

        for (sidx, shortcut) in shortcuts.iter().enumerate() {
            let key = format!("shortcut_{}", sidx);
            let top_left = {
                let [x, y] = icon_position(
                    &self.settings.draft,
                    &key,
                    [
                        workspace.left() + 4.0 + column_width * 2.0,
                        workspace.top() + 16.0 + sidx as f32 * item_height,
                    ],
                    &default_positions,
                );
                egui::pos2(x, y)
            };

            let icon_rect = egui::Rect::from_min_size(
                top_left + egui::vec2((column_width - icon_size) * 0.5, 0.0),
                egui::vec2(icon_size, icon_size),
            );
            let label_rect = egui::Rect::from_min_size(
                top_left + egui::vec2(0.0, icon_size + 2.0),
                egui::vec2(column_width, label_height.max(16.0)),
            );
            let hit_rect = if label_height > 0.0 {
                egui::Rect::from_min_size(
                    top_left,
                    egui::vec2(column_width, icon_size + label_height + 2.0),
                )
            } else {
                icon_rect
            };

            let response = ui.allocate_rect(hit_rect, egui::Sense::click_and_drag());
            let selected = self.desktop_selected_icon == Some(DesktopIconSelection::Shortcut(sidx));
            Self::paint_desktop_icon_selection(ui, hit_rect, palette, selected, response.hovered());
            let icon_fg = Self::desktop_icon_foreground(palette, selected);

            match style {
                DesktopIconStyle::Dos => {
                    ui.painter().text(
                        icon_rect.center(),
                        Align2::CENTER_CENTER,
                        "[LNK]",
                        FontId::new(18.0, FontFamily::Monospace),
                        icon_fg,
                    );
                }
                DesktopIconStyle::Minimal | DesktopIconStyle::Win95 => {
                    // Try to use a custom icon texture if icon_path is set
                    let icon_path_clone = shortcut.icon_path.clone();
                    let icon_tex: Option<egui::TextureHandle> =
                        if let Some(ref path) = icon_path_clone {
                            self.load_cached_shortcut_icon(ui.ctx(), path, Path::new(path), 48)
                        } else {
                            None
                        };
                    if let Some(tex) = icon_tex {
                        Self::paint_tinted_texture(ui.painter(), &tex, icon_rect, icon_fg);
                    } else {
                        let kind_tex = match shortcut.shortcut_kind.as_str() {
                            "network" => &tex_connections,
                            "nuke_codes" => &tex_nuke_codes,
                            "editor" => &tex_editor,
                            _ => &tex_app,
                        };
                        Self::paint_tinted_texture(ui.painter(), kind_tex, icon_rect, icon_fg);
                    }
                    let badge_size = (icon_size * 0.35).max(10.0);
                    let badge_rect = egui::Rect::from_min_size(
                        icon_rect.min + egui::vec2(0.0, icon_size - badge_size),
                        egui::vec2(badge_size, badge_size),
                    );
                    let badge_bg = if selected {
                        palette.panel
                    } else {
                        Color32::BLACK
                    };
                    ui.painter().rect_filled(badge_rect, 0.0, badge_bg);
                    Self::paint_tinted_texture(
                        ui.painter(),
                        &tex_shortcut_badge,
                        badge_rect,
                        icon_fg,
                    );
                }
                DesktopIconStyle::NoIcons => {}
            }

            if label_height > 0.0 {
                Self::paint_desktop_icon_label(ui, label_rect, &shortcut.label, icon_fg);
            }

            if response.dragged() {
                update_dragged_icon_position(
                    &mut self.settings.draft,
                    &key,
                    [top_left.x, top_left.y],
                    [response.drag_delta().x, response.drag_delta().y],
                );
            }
            if response.drag_stopped() {
                needs_persist |=
                    finalize_dragged_icon_position(&mut self.settings.draft, &key, drag_grid);
            }

            if response.clicked() || response.secondary_clicked() {
                self.desktop_selected_icon = Some(DesktopIconSelection::Shortcut(sidx));
            }

            let app_name_for_menu = shortcut.app_name.clone();
            response.context_menu(|ui| {
                Self::apply_context_menu_style(ui);
                ui.set_min_width(136.0);
                ui.set_max_width(180.0);
                if ui.button("Open").clicked() {
                    desktop_action =
                        Some(ContextMenuAction::LaunchShortcut(app_name_for_menu.clone()));
                    ui.close_menu();
                }
                Self::retro_separator(ui);
                if ui.button("Properties").clicked() {
                    desktop_action = Some(ContextMenuAction::OpenShortcutProperties(sidx));
                    ui.close_menu();
                }
                Self::retro_separator(ui);
                if ui.button("Delete Shortcut").clicked() {
                    desktop_action = Some(ContextMenuAction::DeleteShortcut(sidx));
                    ui.close_menu();
                }
            });

            if response.double_clicked() {
                desktop_action = Some(ContextMenuAction::LaunchShortcut(shortcut.app_name.clone()));
            }
        }

        if needs_persist {
            self.persist_native_settings();
        }

        if let Some(action) = desktop_action {
            match action {
                ContextMenuAction::DeleteShortcut(idx) => {
                    if delete_desktop_shortcut(&mut self.settings.draft, idx) {
                        if self.desktop_selected_icon == Some(DesktopIconSelection::Shortcut(idx)) {
                            self.desktop_selected_icon = None;
                        }
                        self.persist_native_settings();
                    }
                }
                _ => {
                    self.context_menu_action = Some(action);
                }
            }
        }

        if open_terminal {
            self.open_desktop_terminal_shell();
        } else if let Some(path) = open_desktop_path {
            self.open_desktop_surface_path(path);
        } else if let Some(window) = open_window {
            self.open_desktop_window(window);
        }
    }

    fn draw_editor_save_as_window(&mut self, _ctx: &egui::Context) {}

    // ── SPOTLIGHT SEARCH ─────────────────────────────────────────────────

    fn spotlight_gather_results(&mut self) {
        let query = self.spotlight_query.to_lowercase();
        let tab = self.spotlight_tab;
        // Skip if query+tab haven't changed
        if query == self.spotlight_last_query && tab == self.spotlight_last_tab {
            return;
        }
        self.spotlight_last_query = query.clone();
        self.spotlight_last_tab = tab;
        let active_username = active_native_session_username();
        self.spotlight_results = gather_spotlight_results(
            &query,
            tab,
            active_username.as_deref(),
            BUILTIN_TEXT_EDITOR_APP,
            BUILTIN_NUKE_CODES_APP,
            BUILTIN_DONKEY_KONG_GAME,
        );
        self.spotlight_selected = 0;
    }

    fn spotlight_activate_result(&mut self, result: &NativeSpotlightResult) {
        self.close_spotlight();
        self.spotlight_query.clear();
        if let Some(action) = self.spotlight_action_for_result(result) {
            self.execute_desktop_shell_action(action);
        }
    }

    fn draw_spotlight(&mut self, ctx: &Context) {
        if !self.spotlight_open {
            return;
        }

        // Close on Escape
        if ctx.input(|i| i.key_pressed(Key::Escape)) {
            self.close_spotlight();
            return;
        }

        // Arrow key navigation
        let mut scroll_selected_into_view = false;
        if ctx.input(|i| i.key_pressed(Key::ArrowDown)) {
            if !self.spotlight_results.is_empty() {
                let next = (self.spotlight_selected + 1).min(self.spotlight_results.len() - 1);
                if next != self.spotlight_selected {
                    self.spotlight_selected = next;
                    scroll_selected_into_view = true;
                }
            }
        }
        if ctx.input(|i| i.key_pressed(Key::ArrowUp)) {
            let next = self.spotlight_selected.saturating_sub(1);
            if next != self.spotlight_selected {
                self.spotlight_selected = next;
                scroll_selected_into_view = true;
            }
        }
        if ctx.input(|i| i.key_pressed(Key::ArrowRight)) {
            self.move_spotlight_tab(1);
            scroll_selected_into_view = true;
        }
        if ctx.input(|i| i.key_pressed(Key::ArrowLeft)) {
            self.move_spotlight_tab(-1);
            scroll_selected_into_view = true;
        }
        if ctx.input(|i| i.key_pressed(Key::Tab) && !i.modifiers.shift) {
            self.move_spotlight_tab(1);
            scroll_selected_into_view = true;
            ctx.input_mut(|i| {
                i.consume_key(egui::Modifiers::NONE, Key::Tab);
            });
        }
        if ctx.input(|i| i.key_pressed(Key::Tab) && i.modifiers.shift) {
            self.move_spotlight_tab(-1);
            scroll_selected_into_view = true;
            ctx.input_mut(|i| {
                i.consume_key(
                    egui::Modifiers {
                        shift: true,
                        ..Default::default()
                    },
                    Key::Tab,
                );
            });
        }

        // Enter to activate
        let mut activate_idx: Option<usize> = None;
        if ctx.input(|i| i.key_pressed(Key::Enter)) && !self.spotlight_results.is_empty() {
            activate_idx = Some(self.spotlight_selected);
        }

        // Gather results
        let prev_query = self.spotlight_last_query.clone();
        let prev_tab = self.spotlight_last_tab;
        self.spotlight_gather_results();
        if self.spotlight_last_query != prev_query || self.spotlight_last_tab != prev_tab {
            scroll_selected_into_view = true;
        }

        let palette = current_palette();
        let screen = ctx.screen_rect();
        let box_width = 600.0_f32.min(screen.width() - 40.0);
        let box_height = 420.0_f32.min(screen.height() - 80.0);

        egui::Window::new("spotlight_window")
            .title_bar(false)
            .resizable(false)
            .collapsible(false)
            .fixed_size(egui::vec2(box_width, box_height))
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
            .order(egui::Order::Foreground)
            .frame(
                egui::Frame::none()
                    .fill(palette.bg)
                    .stroke(egui::Stroke::new(2.0, palette.fg))
                    .shadow(egui::epaint::Shadow::NONE)
                    .inner_margin(egui::Margin::same(12.0)),
            )
            .show(ctx, |ui| {
                let v = ui.visuals_mut();
                v.override_text_color = Some(palette.fg);
                v.extreme_bg_color = palette.bg;
                v.selection.bg_fill = palette.fg;
                v.selection.stroke = egui::Stroke::new(1.0, palette.fg);
                // noninteractive (labels, frames)
                v.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0, palette.fg);
                v.widgets.noninteractive.bg_fill = Color32::TRANSPARENT;
                v.widgets.noninteractive.weak_bg_fill = Color32::TRANSPARENT;
                v.widgets.noninteractive.bg_stroke = egui::Stroke::NONE;
                // inactive (buttons at rest)
                v.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, palette.fg);
                v.widgets.inactive.bg_fill = Color32::TRANSPARENT;
                v.widgets.inactive.weak_bg_fill = Color32::TRANSPARENT;
                v.widgets.inactive.bg_stroke = egui::Stroke::new(1.0, palette.fg);
                // hovered
                v.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, palette.fg);
                v.widgets.hovered.bg_fill = palette.panel;
                v.widgets.hovered.weak_bg_fill = palette.panel;
                v.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, palette.fg);
                v.widgets.hovered.expansion = 0.0;
                // active (pressed)
                v.widgets.active.fg_stroke = egui::Stroke::new(1.0, Color32::BLACK);
                v.widgets.active.bg_fill = palette.fg;
                v.widgets.active.weak_bg_fill = palette.fg;
                v.widgets.active.bg_stroke = egui::Stroke::new(1.0, palette.fg);

                // Search input
                let search_resp = ui.add(
                    TextEdit::singleline(&mut self.spotlight_query)
                        .desired_width(box_width - 48.0)
                        .hint_text("Search apps, documents, files…")
                        .font(egui::TextStyle::Body),
                );
                // Auto-focus
                if search_resp.gained_focus() || !search_resp.has_focus() {
                    search_resp.request_focus();
                }

                ui.add_space(6.0);

                // Tab buttons
                ui.horizontal(|ui| {
                    let tabs = ["All", "Apps", "Documents", "Files"];
                    for (i, label) in tabs.iter().enumerate() {
                        let selected = self.spotlight_tab == i as u8;
                        let text = if selected {
                            RichText::new(*label).color(Color32::BLACK).strong()
                        } else {
                            RichText::new(*label).color(palette.fg)
                        };
                        let btn = egui::Button::new(text);
                        let btn = if selected {
                            btn.fill(palette.fg)
                        } else {
                            btn.fill(palette.panel)
                        };
                        if ui.add(btn).clicked() {
                            self.set_spotlight_tab(i as u8);
                        }
                    }
                });

                ui.add_space(4.0);

                // Results
                let results_height = ui.available_height();
                egui::ScrollArea::vertical()
                    .max_height(results_height)
                    .auto_shrink(false)
                    .show(ui, |ui| {
                        if self.spotlight_results.is_empty() {
                            if self.spotlight_query.is_empty() {
                                ui.label(RichText::new("Type to search…").color(palette.dim));
                            } else {
                                ui.label(RichText::new("No results found.").color(palette.dim));
                            }
                        } else {
                            for (i, result) in self.spotlight_results.iter().enumerate() {
                                let selected = i == self.spotlight_selected;
                                let cat_label = spotlight_category_tag(&result.category);
                                let display = format!("[{cat_label}]  {}", result.name);
                                let text_color = if selected { Color32::BLACK } else { palette.fg };
                                let resp = ui.add(egui::SelectableLabel::new(
                                    selected,
                                    RichText::new(display).color(text_color),
                                ));
                                if resp.clicked() {
                                    activate_idx = Some(i);
                                }
                                if selected && scroll_selected_into_view {
                                    resp.scroll_to_me(None);
                                }
                            }
                        }
                    });
            });

        // Activate after UI is done (deferred to avoid borrow issues)
        if let Some(idx) = activate_idx {
            if idx < self.spotlight_results.len() {
                let result = self.spotlight_results[idx].clone();
                self.spotlight_activate_result(&result);
            }
        }
    }

    fn draw_shortcut_properties_window(&mut self, ctx: &egui::Context) {
        let Some(props) = self.shortcut_properties.clone() else {
            return;
        };
        let palette = current_palette();
        let props_idx = props.shortcut_idx;
        let mut name_draft = props.name_draft.clone();
        let mut command_draft = props.command_draft.clone();
        let icon_path_draft = props.icon_path_draft.clone();
        let mut action: Option<&'static str> = None;

        egui::Window::new("shortcut_properties_window")
            .title_bar(false)
            .resizable(false)
            .collapsible(false)
            .frame(Self::desktop_window_frame())
            .fixed_size(egui::vec2(360.0, 260.0))
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
            .show(ctx, |ui| {
                Self::apply_settings_control_style(ui);

                // Header
                let header_action =
                    Self::draw_desktop_window_header(ui, "Shortcut Properties", false);
                if matches!(header_action, DesktopHeaderAction::Close) {
                    action = Some("cancel");
                }

                ui.add_space(12.0);

                // Icon preview + shortcut label
                ui.horizontal(|ui| {
                    // Icon preview box
                    let icon_size = 48.0;
                    let (rect, _) = ui.allocate_exact_size(
                        egui::vec2(icon_size, icon_size),
                        egui::Sense::hover(),
                    );
                    // Draw current icon
                    let icon_tex: Option<egui::TextureHandle> = icon_path_draft
                        .as_ref()
                        .and_then(|p| self.load_cached_shortcut_icon(ctx, p, Path::new(p), 48));
                    if let Some(tex) = icon_tex {
                        Self::paint_tinted_texture(ui.painter(), &tex, rect, palette.fg);
                    } else if let Some(cache) = &self.asset_cache {
                        let icon = cache.icon_applications.clone();
                        Self::paint_tinted_texture(ui.painter(), &icon, rect, palette.fg);
                    }
                    ui.add_space(8.0);
                    ui.label(
                        RichText::new(&name_draft)
                            .strong()
                            .monospace()
                            .color(palette.fg),
                    );
                });

                ui.add_space(8.0);
                Self::retro_separator(ui);
                ui.add_space(8.0);

                // Name field
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Name:   ").monospace().color(palette.fg));
                    let name_edit = egui::TextEdit::singleline(&mut name_draft)
                        .font(egui::TextStyle::Monospace)
                        .desired_width(220.0);
                    ui.add(name_edit);
                });

                ui.add_space(6.0);

                // Target field
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Target: ").monospace().color(palette.fg));
                    let cmd_edit = egui::TextEdit::singleline(&mut command_draft)
                        .font(egui::TextStyle::Monospace)
                        .desired_width(220.0);
                    ui.add(cmd_edit);
                });

                ui.add_space(6.0);

                // Change Icon button
                ui.horizontal(|ui| {
                    ui.add_space(80.0);
                    if ui.button("Change Icon...").clicked() {
                        action = Some("change_icon");
                    }
                    if let Some(path) = &icon_path_draft {
                        let filename = std::path::Path::new(path)
                            .file_name()
                            .map(|n| n.to_string_lossy().to_string())
                            .unwrap_or_default();
                        ui.label(
                            RichText::new(filename)
                                .small()
                                .monospace()
                                .color(palette.dim),
                        );
                    }
                });

                ui.add_space(12.0);
                Self::retro_separator(ui);
                ui.add_space(8.0);

                // OK / Cancel
                ui.horizontal(|ui| {
                    ui.add_space(ui.available_width() / 2.0 - 70.0);
                    if ui.button("  OK  ").clicked() {
                        action = Some("ok");
                    }
                    ui.add_space(8.0);
                    if ui.button("Cancel").clicked() {
                        action = Some("cancel");
                    }
                });

                // Sync drafts back to state
                if let Some(props) = &mut self.shortcut_properties {
                    props.name_draft = name_draft;
                    props.command_draft = command_draft;
                }
            });

        // Handle deferred actions OUTSIDE the window closure (to avoid double-borrow)
        match action {
            Some("ok") => {
                if let Some(props) = &self.shortcut_properties {
                    let update = ShortcutPropertiesUpdate {
                        label: props.name_draft.clone(),
                        command_draft: props.command_draft.clone(),
                        icon_path: props.icon_path_draft.clone(),
                    };
                    update_desktop_shortcut_properties(
                        &mut self.settings.draft,
                        props_idx,
                        &update,
                    );
                }
                self.persist_native_settings();
                self.shortcut_properties = None;
            }
            Some("cancel") => {
                self.shortcut_properties = None;
            }
            Some("change_icon") => {
                let icons_dir =
                    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/Icons");
                self.picking_icon_for_shortcut = Some(props_idx);
                self.open_embedded_file_manager_at(icons_dir);
            }
            _ => {}
        }
    }

    fn draw_desktop_item_properties_window(&mut self, ctx: &egui::Context) {
        let Some(props) = self.desktop_item_properties.clone() else {
            return;
        };
        let palette = current_palette();
        let mut name_draft = props.name_draft.clone();
        let mut action: Option<&'static str> = None;
        let item_name = props
            .path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("item")
            .to_string();
        let item_type = if props.is_dir { "Folder" } else { "File" };
        let path_display = props.path.display().to_string();

        egui::Window::new("desktop_item_properties_window")
            .title_bar(false)
            .resizable(false)
            .collapsible(false)
            .frame(Self::desktop_window_frame())
            .fixed_size(egui::vec2(440.0, 250.0))
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
            .show(ctx, |ui| {
                Self::apply_settings_control_style(ui);
                let header_action =
                    Self::draw_desktop_window_header(ui, "Desktop Item Properties", false);
                if matches!(header_action, DesktopHeaderAction::Close) {
                    action = Some("cancel");
                }

                ui.add_space(12.0);
                ui.label(
                    RichText::new(&item_name)
                        .strong()
                        .monospace()
                        .color(palette.fg),
                );
                ui.add_space(6.0);
                ui.label(RichText::new(format!("Type: {item_type}")).color(palette.dim));
                ui.add_space(6.0);
                ui.label(RichText::new("Path:").color(palette.dim));
                ui.label(RichText::new(path_display).monospace().color(palette.fg));

                ui.add_space(12.0);
                Self::retro_separator(ui);
                ui.add_space(10.0);

                ui.horizontal(|ui| {
                    ui.label(RichText::new("Name:").monospace().color(palette.fg));
                    let edit = egui::TextEdit::singleline(&mut name_draft)
                        .font(egui::TextStyle::Monospace)
                        .desired_width(260.0);
                    ui.add(edit);
                });

                ui.add_space(16.0);
                ui.horizontal(|ui| {
                    if ui.button("Open").clicked() {
                        action = Some("open");
                    }
                    if !props.is_dir && ui.button("Open With...").clicked() {
                        action = Some("open_with");
                    }
                    if ui.button("Delete").clicked() {
                        action = Some("delete");
                    }
                });

                ui.add_space(12.0);
                Self::retro_separator(ui);
                ui.add_space(10.0);

                ui.horizontal(|ui| {
                    ui.add_space(ui.available_width() / 2.0 - 70.0);
                    if ui.button(" Save ").clicked() {
                        action = Some("save");
                    }
                    ui.add_space(8.0);
                    if ui.button("Cancel").clicked() {
                        action = Some("cancel");
                    }
                });
            });

        if let Some(props) = &mut self.desktop_item_properties {
            props.name_draft = name_draft.clone();
        }

        match action {
            Some("open") => {
                self.open_desktop_surface_path(props.path.clone());
                self.desktop_item_properties = None;
            }
            Some("open_with") => {
                self.open_desktop_surface_with_prompt(props.path.clone());
                self.desktop_item_properties = None;
            }
            Some("delete") => {
                self.delete_desktop_item(props.path.clone());
            }
            Some("save") => {
                let entry = FileEntryRow {
                    path: props.path.clone(),
                    label: item_name,
                    is_dir: props.is_dir,
                };
                self.shell_status = match self.file_manager_runtime.rename_entry(entry, name_draft)
                {
                    Ok(new_path) => {
                        self.desktop_selected_icon = Some(DesktopIconSelection::Surface(format!(
                            "desktop_item:{}",
                            new_path
                                .file_name()
                                .and_then(|name| name.to_str())
                                .unwrap_or("item")
                        )));
                        self.desktop_item_properties = None;
                        self.invalidate_desktop_surface_cache();
                        "Desktop item renamed.".to_string()
                    }
                    Err(err) => format!("Desktop rename failed: {err}"),
                };
            }
            Some("cancel") => {
                self.desktop_item_properties = None;
            }
            _ => {}
        }
    }

    fn open_start_menu(&mut self) {
        self.close_spotlight();
        self.start_open = true;
        self.start_selected_root = 0;
        self.start_system_selected = 0;
        self.start_leaf_selected = 0;
        self.start_open_submenu = None;
        self.start_open_leaf = None;
    }

    fn close_start_menu(&mut self) {
        self.start_open = false;
        self.start_open_submenu = None;
        self.start_open_leaf = None;
    }

    fn close_start_menu_panel(&mut self) {
        self.start_open_submenu = None;
        self.start_open_leaf = None;
        self.start_system_selected = 0;
        self.start_leaf_selected = 0;
    }

    fn open_spotlight(&mut self) {
        self.close_start_menu();
        self.spotlight_open = true;
        self.spotlight_tab = 0;
        self.spotlight_query.clear();
        self.spotlight_selected = 0;
        self.spotlight_results.clear();
        self.spotlight_last_query.clear();
        self.spotlight_last_tab = u8::MAX;
    }

    fn close_spotlight(&mut self) {
        self.spotlight_open = false;
    }

    fn set_spotlight_tab(&mut self, tab: u8) {
        let next = tab.min(3);
        if self.spotlight_tab == next {
            return;
        }
        self.spotlight_tab = next;
        self.spotlight_selected = 0;
        self.spotlight_last_tab = u8::MAX;
    }

    fn move_spotlight_tab(&mut self, delta: i8) {
        let current = self.spotlight_tab as i8;
        let next = (current + delta).clamp(0, 3) as u8;
        self.set_spotlight_tab(next);
    }

    fn close_desktop_overlays(&mut self) {
        self.close_start_menu();
        self.close_spotlight();
    }

    fn start_menu_open_current_panel(&mut self) {
        let idx = self.start_selected_root;
        if start_root_leaf_for_idx(idx).is_some() || start_root_submenu_for_idx(idx).is_some() {
            self.set_start_panel_for_root(idx);
        }
    }

    fn start_menu_move_root_selection(&mut self, delta: isize) {
        let max_idx = START_ROOT_ITEMS.len().saturating_sub(1) as isize;
        let next = (self.start_selected_root as isize + delta).clamp(0, max_idx) as usize;
        if next == self.start_selected_root {
            return;
        }
        if self.start_open_leaf.is_some() || self.start_open_submenu.is_some() {
            self.set_start_panel_for_root(next);
        } else {
            self.start_selected_root = next;
        }
    }

    fn start_menu_move_panel_selection(&mut self, delta: isize) {
        if let Some(StartSubmenu::System) = self.start_open_submenu {
            let items_len = self.start_system_items().len();
            if items_len > 0 {
                let max_idx = items_len.saturating_sub(1) as isize;
                self.start_system_selected =
                    (self.start_system_selected as isize + delta).clamp(0, max_idx) as usize;
            }
        } else if let Some(leaf) = self.start_open_leaf {
            let items_len = self.start_leaf_items(leaf).len();
            if items_len > 0 {
                let max_idx = items_len.saturating_sub(1) as isize;
                self.start_leaf_selected =
                    (self.start_leaf_selected as isize + delta).clamp(0, max_idx) as usize;
            }
        } else {
            self.start_menu_move_root_selection(delta);
        }
    }

    fn activate_start_menu_selection(&mut self) {
        if let Some(StartSubmenu::System) = self.start_open_submenu {
            let items = self.start_system_items();
            if let Some((_, action)) = items.get(self.start_system_selected) {
                self.run_start_system_action(*action);
            }
            return;
        }

        if let Some(leaf) = self.start_open_leaf {
            let items = self.start_leaf_items(leaf);
            if let Some(item) = items.get(self.start_leaf_selected) {
                self.run_start_leaf_action(item.action.clone());
            }
            return;
        }

        if let Some(action) = start_root_action_for_idx(self.start_selected_root) {
            self.run_start_root_action(action);
        } else {
            self.start_menu_open_current_panel();
        }
    }

    fn handle_start_menu_keyboard(&mut self, ctx: &Context) {
        if !self.start_open {
            return;
        }

        let mut handled = false;
        ctx.input_mut(|i| {
            if i.key_pressed(Key::ArrowUp) {
                self.start_menu_move_panel_selection(-1);
                i.consume_key(egui::Modifiers::NONE, Key::ArrowUp);
                handled = true;
            } else if i.key_pressed(Key::ArrowDown) {
                self.start_menu_move_panel_selection(1);
                i.consume_key(egui::Modifiers::NONE, Key::ArrowDown);
                handled = true;
            } else if i.key_pressed(Key::ArrowRight) {
                if self.start_open_leaf.is_none() && self.start_open_submenu.is_none() {
                    self.start_menu_open_current_panel();
                }
                i.consume_key(egui::Modifiers::NONE, Key::ArrowRight);
                handled = true;
            } else if i.key_pressed(Key::ArrowLeft) {
                if self.start_open_leaf.is_some() || self.start_open_submenu.is_some() {
                    self.close_start_menu_panel();
                } else {
                    self.close_start_menu();
                }
                i.consume_key(egui::Modifiers::NONE, Key::ArrowLeft);
                handled = true;
            } else if i.key_pressed(Key::Enter) {
                self.activate_start_menu_selection();
                i.consume_key(egui::Modifiers::NONE, Key::Enter);
                handled = true;
            }
        });

        if handled {
            self.close_spotlight();
        }
    }

    fn set_start_panel_for_root(&mut self, root_idx: usize) {
        self.start_selected_root = root_idx.min(START_ROOT_ITEMS.len().saturating_sub(1));
        self.start_open_leaf = start_root_leaf_for_idx(self.start_selected_root);
        self.start_open_submenu = start_root_submenu_for_idx(self.start_selected_root);
        self.start_leaf_selected = 0;
        self.start_system_selected = 0;
    }

    fn start_system_items(&self) -> Vec<(&'static str, StartSystemAction)> {
        START_SYSTEM_ITEMS
            .iter()
            .copied()
            .filter(|(_, action)| {
                !matches!(action, StartSystemAction::Connections) || !connections_macos_disabled()
            })
            .collect()
    }

    fn start_leaf_menu_target(action: &NativeStartLeafAction) -> Option<(EditMenuTarget, String)> {
        match action {
            NativeStartLeafAction::LaunchConfiguredApp(name) => {
                Some((EditMenuTarget::Applications, name.clone()))
            }
            NativeStartLeafAction::LaunchNetworkProgram(name) => {
                Some((EditMenuTarget::Network, name.clone()))
            }
            NativeStartLeafAction::LaunchGameProgram(name) if name != BUILTIN_DONKEY_KONG_GAME => {
                Some((EditMenuTarget::Games, name.clone()))
            }
            _ => None,
        }
    }

    fn start_leaf_items(&self, leaf: StartLeaf) -> Vec<NativeStartLeafEntry> {
        match leaf {
            StartLeaf::Applications => start_application_entries(
                self.settings.draft.builtin_menu_visibility.nuke_codes,
                self.settings.draft.builtin_menu_visibility.text_editor,
                BUILTIN_TEXT_EDITOR_APP,
                BUILTIN_NUKE_CODES_APP,
            ),
            StartLeaf::Documents => start_document_entries(
                self.session
                    .as_ref()
                    .map(|session| session.username.as_str()),
            ),
            StartLeaf::Network => start_network_entries(),
            StartLeaf::Games => start_game_entries(BUILTIN_DONKEY_KONG_GAME),
        }
    }

    fn open_file_manager_at(&mut self, path: PathBuf) {
        self.launch_standalone_file_manager(Some(path));
    }

    fn open_embedded_file_manager_at(&mut self, path: PathBuf) {
        match open_directory_location(path) {
            Ok(location) => self.apply_file_manager_location(location),
            Err(status) => self.shell_status = status,
        }
    }

    fn launch_standalone_file_manager(&mut self, path: Option<PathBuf>) {
        let start_path = path.unwrap_or_else(|| self.file_manager.cwd.clone());
        let current_user = get_current_user();
        let session_username = self
            .session
            .as_ref()
            .map(|session| session.username.as_str())
            .or(current_user.as_deref());
        let args = vec![start_path.into_os_string()];
        match launch_standalone_app(StandaloneNativeApp::FileManager, &args, session_username) {
            Ok(()) => self.apply_status_update(clear_shell_status()),
            Err(status) => self.shell_status = status,
        }
    }

    fn launch_standalone_editor(&mut self, path: Option<PathBuf>) {
        let current_user = get_current_user();
        let session_username = self
            .session
            .as_ref()
            .map(|session| session.username.as_str())
            .or(current_user.as_deref());
        let args = path
            .map(|path| vec![path.into_os_string()])
            .unwrap_or_default();
        match launch_standalone_app(StandaloneNativeApp::Editor, &args, session_username) {
            Ok(()) => self.apply_status_update(clear_shell_status()),
            Err(status) => self.shell_status = status,
        }
    }

    fn launch_standalone_applications(&mut self) {
        let current_user = get_current_user();
        let session_username = self
            .session
            .as_ref()
            .map(|session| session.username.as_str())
            .or(current_user.as_deref());
        match launch_standalone_app(StandaloneNativeApp::Applications, &[], session_username) {
            Ok(()) => self.apply_status_update(clear_shell_status()),
            Err(status) => self.shell_status = status,
        }
    }

    fn launch_standalone_nuke_codes(&mut self) {
        let current_user = get_current_user();
        let session_username = self
            .session
            .as_ref()
            .map(|session| session.username.as_str())
            .or(current_user.as_deref());
        match launch_standalone_app(StandaloneNativeApp::NukeCodes, &[], session_username) {
            Ok(()) => self.apply_status_update(clear_shell_status()),
            Err(status) => self.shell_status = status,
        }
    }

    fn open_standalone_settings(&mut self, panel: Option<NativeSettingsPanel>) {
        let current_user = get_current_user();
        let session_username = self
            .session
            .as_ref()
            .map(|session| session.username.as_str())
            .or(current_user.as_deref());
        let mut args = Vec::new();
        if let Some(panel) = panel {
            args.push(OsString::from(standalone_settings_panel_arg(panel)));
        }
        match launch_standalone_app(StandaloneNativeApp::Settings, &args, session_username) {
            Ok(()) => self.apply_status_update(clear_shell_status()),
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
        self.desktop_active_window = Some(DesktopWindow::FileManager);
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
            self.terminal_nuke_codes = fetch_nuke_codes();
        }
        self.open_desktop_window(DesktopWindow::NukeCodes);
    }

    fn execute_desktop_shell_action(&mut self, action: DesktopShellAction) {
        match action {
            DesktopShellAction::OpenWindow(window) => match window {
                DesktopWindow::FileManager => self.launch_standalone_file_manager(None),
                DesktopWindow::Settings => self.open_standalone_settings(None),
                DesktopWindow::Applications => self.launch_standalone_applications(),
                _ => self.open_desktop_window(window),
            },
            DesktopShellAction::OpenTextEditor => self.launch_standalone_editor(None),
            DesktopShellAction::OpenNukeCodes => self.launch_standalone_nuke_codes(),
            DesktopShellAction::OpenDesktopTerminalShell => self.open_desktop_terminal_shell(),
            DesktopShellAction::OpenConnectionsSettings => {
                if connections_macos_disabled() {
                    self.shell_status = connections_macos_disabled_hint().to_string();
                } else {
                    self.open_standalone_settings(Some(NativeSettingsPanel::Connections));
                }
            }
            DesktopShellAction::LaunchConfiguredApp(name) => {
                self.apply_desktop_program_request(DesktopProgramRequest::LaunchCatalog {
                    name,
                    catalog: ProgramCatalog::Applications,
                    close_window: true,
                });
            }
            DesktopShellAction::OpenFileManagerAt(path) => self.open_file_manager_at(path),
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
            DesktopShellAction::OpenPathInEditor(path) => self.launch_standalone_editor(Some(path)),
            DesktopShellAction::RevealPathInFileManager(path) => {
                self.launch_standalone_file_manager(Some(path));
            }
        }
    }

    fn run_start_root_action(&mut self, action: StartRootAction) {
        match action {
            StartRootAction::ReturnToTerminal => {
                self.close_start_menu();
                crate::sound::play_logout();
                self.desktop_mode_open = false;
            }
            StartRootAction::Logout => {
                self.close_start_menu();
                self.begin_logout();
            }
            StartRootAction::Shutdown => {
                self.close_start_menu();
                self.queue_terminal_flash("Shutting down...", 800, FlashAction::ExitApp);
            }
        }
    }

    fn run_start_system_action(&mut self, action: StartSystemAction) {
        self.close_start_menu();
        let action = match action {
            StartSystemAction::ProgramInstaller => {
                DesktopShellAction::OpenWindow(DesktopWindow::Installer)
            }
            StartSystemAction::Terminal => DesktopShellAction::OpenDesktopTerminalShell,
            StartSystemAction::FileManager => {
                DesktopShellAction::OpenWindow(DesktopWindow::FileManager)
            }
            StartSystemAction::Settings => DesktopShellAction::OpenWindow(DesktopWindow::Settings),
            StartSystemAction::Connections => DesktopShellAction::OpenConnectionsSettings,
        };
        self.execute_desktop_shell_action(action);
    }

    fn run_start_leaf_action(&mut self, action: NativeStartLeafAction) {
        let action = match action {
            NativeStartLeafAction::None => return,
            NativeStartLeafAction::LaunchNukeCodes => DesktopShellAction::OpenNukeCodes,
            NativeStartLeafAction::OpenTextEditor => DesktopShellAction::OpenTextEditor,
            NativeStartLeafAction::LaunchConfiguredApp(name) => {
                DesktopShellAction::LaunchConfiguredApp(name)
            }
            NativeStartLeafAction::OpenDocumentCategory(path) => {
                DesktopShellAction::OpenFileManagerAt(path)
            }
            NativeStartLeafAction::LaunchNetworkProgram(name) => {
                DesktopShellAction::LaunchNetworkProgram(name)
            }
            NativeStartLeafAction::LaunchGameProgram(name) => {
                DesktopShellAction::LaunchGameProgram(name)
            }
        };
        self.execute_desktop_shell_action(action);
    }

    fn spotlight_action_for_result(
        &self,
        result: &NativeSpotlightResult,
    ) -> Option<DesktopShellAction> {
        match &result.category {
            NativeSpotlightCategory::System => match result.name.as_str() {
                "File Manager" => Some(DesktopShellAction::OpenWindow(DesktopWindow::FileManager)),
                "Settings" => Some(DesktopShellAction::OpenWindow(DesktopWindow::Settings)),
                "Terminal" => Some(DesktopShellAction::OpenWindow(DesktopWindow::TerminalMode)),
                n if n == BUILTIN_TEXT_EDITOR_APP => Some(DesktopShellAction::OpenTextEditor),
                n if n == BUILTIN_NUKE_CODES_APP => {
                    Some(DesktopShellAction::OpenWindow(DesktopWindow::NukeCodes))
                }
                _ => None,
            },
            NativeSpotlightCategory::App => {
                Some(DesktopShellAction::LaunchConfiguredApp(result.name.clone()))
            }
            NativeSpotlightCategory::Game => {
                Some(DesktopShellAction::LaunchGameProgram(result.name.clone()))
            }
            NativeSpotlightCategory::Network => Some(DesktopShellAction::LaunchNetworkProgram(
                result.name.clone(),
            )),
            NativeSpotlightCategory::Document => result
                .path
                .clone()
                .map(DesktopShellAction::OpenPathInEditor),
            NativeSpotlightCategory::File => result
                .path
                .clone()
                .map(DesktopShellAction::RevealPathInFileManager),
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

    fn draw_desktop_window_by_kind(&mut self, ctx: &Context, window: DesktopWindow) {
        (desktop_component_binding(window).draw)(self, ctx);
    }

    fn draw_desktop_windows(&mut self, ctx: &Context) {
        self.sync_desktop_active_window();
        let active = self.desktop_active_window;
        for window in desktop_components()
            .iter()
            .map(|component| component.spec.window)
        {
            if Some(window) == active {
                continue;
            }
            if self.desktop_window_is_minimized(window) {
                continue;
            }
            self.draw_desktop_window_by_kind(ctx, window);
        }
        if let Some(window) = active {
            if !self.desktop_window_is_minimized(window) {
                self.draw_desktop_window_by_kind(ctx, window);
            }
        }
        self.sync_desktop_active_window();
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
                    let window = self.desktop_window_state_mut(DesktopWindow::PtyApp);
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
        self.launch_standalone_editor(Some(path));
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
        let settings = persist_settings_draft(&self.settings.draft);
        self.replace_settings_draft(settings);
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

    fn apply_terminal_login_selection_plan(
        &mut self,
        plan: TerminalLoginSelectionPlan<UserRecord>,
    ) {
        self.login.error.clear();
        match plan {
            TerminalLoginSelectionPlan::Exit => {
                crate::sound::play_logout();
                self.queue_terminal_flash("Exiting...", 800, FlashAction::ExitApp);
            }
            TerminalLoginSelectionPlan::PromptPassword { username, prompt } => {
                self.login.selected_username = username;
                self.login.clear_password_and_error();
                self.login.mode = TerminalLoginScreenMode::SelectUser;
                self.open_password_prompt(prompt.title, prompt.prompt);
            }
            TerminalLoginSelectionPlan::Submit {
                action,
                missing_username_is_select_user,
            } => {
                crate::sound::play_navigate();
                self.apply_terminal_login_submit_action(action, missing_username_is_select_user);
            }
            TerminalLoginSelectionPlan::StartHacking { username } => {
                crate::sound::play_navigate();
                self.login.selected_username = username.clone();
                self.login.error.clear();
                self.terminal_prompt = None;
                self.queue_session_flash_plan(hacking_start_flash_plan(username));
            }
            TerminalLoginSelectionPlan::ShowError(error) => {
                crate::sound::play_error();
                self.login.error = error;
            }
        }
    }

    fn apply_terminal_login_submit_action(
        &mut self,
        action: TerminalLoginSubmitAction<UserRecord>,
        missing_username_is_select_user: bool,
    ) {
        self.login.error.clear();
        match action {
            TerminalLoginSubmitAction::MissingUsername => {
                crate::sound::play_error();
                self.login.error = if missing_username_is_select_user {
                    "Select a user.".to_string()
                } else {
                    "Username cannot be empty.".to_string()
                };
            }
            TerminalLoginSubmitAction::Authenticated { username, user } => {
                crate::sound::play_login();
                bind_login_identity(&username);
                self.login.selected_username = username.clone();
                self.login.password.clear();
                self.login.error.clear();
                self.terminal_prompt = None;
                self.queue_session_flash_plan(login_flash_plan(username, user));
            }
            TerminalLoginSubmitAction::ShowError(error) => {
                crate::sound::play_error();
                self.login.error = error;
            }
        }
    }

    fn apply_terminal_hacking_plan(&mut self, plan: TerminalHackingPlan<UserRecord>) {
        match plan {
            TerminalHackingPlan::ShowUserSelection => {
                self.login.show_user_selection();
            }
            TerminalHackingPlan::ShowLocked => {
                crate::sound::play_navigate();
                self.login.show_locked();
            }
            TerminalHackingPlan::Submit {
                action,
                fallback_to_user_selection_on_error,
            } => {
                let unknown_user = fallback_to_user_selection_on_error
                    && matches!(action, TerminalLoginSubmitAction::ShowError(_));
                self.apply_terminal_login_submit_action(action, false);
                if unknown_user {
                    crate::sound::play_navigate();
                    self.login.show_user_selection();
                }
            }
        }
    }

    fn apply_main_menu_selection_action(&mut self, action: MainMenuSelectionAction) {
        match action {
            MainMenuSelectionAction::OpenScreen {
                screen,
                selected_idx,
                clear_status,
            } => self.apply_terminal_screen_open_plan(terminal_screen_open_plan(
                screen,
                selected_idx,
                clear_status,
            )),
            MainMenuSelectionAction::OpenTerminalMode => {
                self.open_embedded_terminal_shell();
            }
            MainMenuSelectionAction::EnterDesktopMode => {
                crate::sound::play_login();
                self.desktop_mode_open = true;
                self.close_start_menu();
                self.sync_desktop_active_window();
                self.shell_status = "Entered Desktop Mode.".to_string();
            }
            MainMenuSelectionAction::RefreshSettingsAndOpen => {
                let settings = reload_settings_snapshot();
                self.replace_settings_draft(settings);
                self.apply_terminal_screen_open_plan(terminal_settings_refresh_plan());
            }
            MainMenuSelectionAction::BeginLogout => self.begin_logout(),
        }
    }

    fn apply_terminal_screen_open_plan(&mut self, plan: TerminalScreenOpenPlan) {
        self.navigate_to_screen(plan.screen);
        if matches!(plan.screen, TerminalScreen::Settings) {
            self.terminal_settings_panel = TerminalSettingsPanel::Home;
        }
        if plan.reset_installer {
            self.terminal_installer.reset();
        }
        if plan.reset_connections {
            self.terminal_connections.reset();
        }
        if plan.clear_settings_choice {
            self.terminal_nav.settings_choice = None;
        }
        if plan.clear_default_app_slot {
            self.terminal_nav.default_app_slot = None;
        }
        if plan.reset_user_management_to_root {
            self.terminal_nav.user_management_mode = UserManagementMode::Root;
        }
        match plan.index_target {
            TerminalSelectionIndexTarget::None => {}
            TerminalSelectionIndexTarget::MainMenu => {
                self.terminal_nav.main_menu_idx = plan.selected_idx
            }
            TerminalSelectionIndexTarget::Applications => {
                self.terminal_nav.apps_idx = plan.selected_idx
            }
            TerminalSelectionIndexTarget::Documents => {
                self.terminal_nav.documents_idx = plan.selected_idx
            }
            TerminalSelectionIndexTarget::Logs => self.terminal_nav.logs_idx = plan.selected_idx,
            TerminalSelectionIndexTarget::Network => {
                self.terminal_nav.network_idx = plan.selected_idx
            }
            TerminalSelectionIndexTarget::Games => self.terminal_nav.games_idx = plan.selected_idx,
            TerminalSelectionIndexTarget::ProgramInstallerRoot => {
                self.terminal_installer.root_idx = plan.selected_idx;
            }
            TerminalSelectionIndexTarget::Settings => {
                self.terminal_nav.settings_idx = plan.selected_idx;
            }
            TerminalSelectionIndexTarget::ConnectionsRoot => {
                self.terminal_connections.root_idx = plan.selected_idx;
            }
            TerminalSelectionIndexTarget::DefaultApps => {
                self.terminal_nav.default_apps_idx = plan.selected_idx;
            }
            TerminalSelectionIndexTarget::UserManagement => {
                self.terminal_nav.user_management_idx = plan.selected_idx;
            }
            TerminalSelectionIndexTarget::DocumentBrowser => {
                self.terminal_nav.browser_idx = plan.selected_idx;
            }
        }
        if plan.clear_status {
            self.apply_status_update(clear_shell_status());
        }
    }

    fn handle_terminal_back(&mut self) {
        if matches!(self.terminal_nav.screen, TerminalScreen::Settings)
            && self.terminal_nav.settings_choice.is_none()
            && !matches!(self.terminal_settings_panel, TerminalSettingsPanel::Home)
        {
            crate::sound::play_navigate();
            self.terminal_settings_panel = TerminalSettingsPanel::Home;
            self.terminal_nav.settings_idx = 0;
            self.apply_status_update(clear_shell_status());
            return;
        }
        let action = resolve_terminal_back_action(TerminalBackContext {
            screen: self.terminal_nav.screen,
            has_settings_choice: self.terminal_nav.settings_choice.is_some(),
            has_default_app_slot: self.terminal_nav.default_app_slot.is_some(),
            connections_at_root: self.terminal_connections.is_at_root(),
            installer_at_root: self.terminal_installer.is_at_root(),
            has_embedded_pty: self.terminal_pty.is_some(),
            pty_return_screen: self
                .terminal_pty
                .as_ref()
                .map(|pty| pty.return_screen)
                .unwrap_or(TerminalScreen::MainMenu),
            nuke_codes_return_screen: self.terminal_nav.nuke_codes_return_screen,
            browser_return_screen: self.terminal_nav.browser_return_screen,
        });
        match action {
            TerminalBackAction::NoOp => {}
            TerminalBackAction::ClearSettingsChoice => {
                crate::sound::play_navigate();
                self.terminal_nav.settings_choice = None;
            }
            TerminalBackAction::ClearDefaultAppSlot => {
                crate::sound::play_navigate();
                self.terminal_nav.default_app_slot = None;
            }
            TerminalBackAction::UseConnectionsInnerBack => {
                crate::sound::play_navigate();
                let _ = self.terminal_connections.back();
                self.apply_status_update(clear_shell_status());
            }
            TerminalBackAction::UseInstallerInnerBack => {
                crate::sound::play_navigate();
                let _ = self.terminal_installer.back();
                self.apply_status_update(clear_shell_status());
            }
            TerminalBackAction::NavigateTo {
                screen,
                clear_status,
                reset_installer,
            } => {
                self.navigate_to_screen(screen);
                if reset_installer {
                    self.terminal_installer.reset();
                }
                if clear_status {
                    self.apply_status_update(clear_shell_status());
                }
            }
            TerminalBackAction::ClosePtyAndReturn { return_screen } => {
                if let Some(mut pty) = self.terminal_pty.take() {
                    pty.session.terminate();
                    self.navigate_to_screen(return_screen);
                    self.shell_status = format!("Closed {}.", pty.title);
                } else {
                    self.navigate_to_screen(TerminalScreen::MainMenu);
                    self.apply_status_update(clear_shell_status());
                }
            }
        }
    }

    fn handle_terminal_prompt_input(&mut self, ctx: &Context) {
        let Some(prompt) = self.terminal_prompt.clone() else {
            return;
        };
        let prompt_action = prompt.action.clone();
        let outcome = handle_prompt_input(ctx, prompt);
        if self.handle_file_manager_prompt_outcome(&outcome) {
            return;
        }
        match outcome {
            PromptOutcome::Cancel => {
                crate::sound::play_navigate();
                self.terminal_prompt = None;
                if matches!(prompt_action, TerminalPromptAction::LoginPassword) {
                    self.login.password.clear();
                    self.login.error.clear();
                }
            }
            PromptOutcome::Continue(prompt) => {
                self.terminal_prompt = Some(prompt);
            }
            PromptOutcome::LoginPassword(password) => {
                self.terminal_prompt = None;
                self.login.password = password;
                let plan = resolve_login_password_submission(
                    &self.login.selected_username,
                    &self.login.password,
                    self.session.is_some(),
                    self.terminal_flash.is_some(),
                    authenticate_login,
                );
                self.apply_terminal_login_password_plan(plan);
            }
            PromptOutcome::CreateUsername(raw_username) => {
                self.terminal_prompt = None;
                let exists = user_exists(raw_username.trim());
                let plan = resolve_create_username_prompt(&raw_username, exists);
                self.apply_terminal_user_management_prompt_plan(plan);
            }
            PromptOutcome::CreatePasswordFirst { username, password } => {
                self.terminal_prompt = None;
                let plan = resolve_user_password_first_prompt(
                    TerminalUserPasswordFlow::Create,
                    username,
                    password,
                );
                self.apply_terminal_user_management_prompt_plan(plan);
            }
            PromptOutcome::CreatePasswordConfirm {
                username,
                first_password,
                confirmation,
            } => {
                self.terminal_prompt = None;
                let plan = resolve_user_password_confirm_prompt(
                    TerminalUserPasswordFlow::Create,
                    username,
                    first_password,
                    confirmation,
                );
                self.apply_terminal_user_management_prompt_plan(plan);
            }
            PromptOutcome::ResetPasswordFirst { username, password } => {
                self.terminal_prompt = None;
                let plan = resolve_user_password_first_prompt(
                    TerminalUserPasswordFlow::Reset,
                    username,
                    password,
                );
                self.apply_terminal_user_management_prompt_plan(plan);
            }
            PromptOutcome::ResetPasswordConfirm {
                username,
                first_password,
                confirmation,
            } => {
                self.terminal_prompt = None;
                let plan = resolve_user_password_confirm_prompt(
                    TerminalUserPasswordFlow::Reset,
                    username,
                    first_password,
                    confirmation,
                );
                self.apply_terminal_user_management_prompt_plan(plan);
            }
            PromptOutcome::ChangeAuthPasswordFirst { username, password } => {
                self.terminal_prompt = None;
                let plan = resolve_user_password_first_prompt(
                    TerminalUserPasswordFlow::ChangeAuth,
                    username,
                    password,
                );
                self.apply_terminal_user_management_prompt_plan(plan);
            }
            PromptOutcome::ChangeAuthPasswordConfirm {
                username,
                first_password,
                confirmation,
            } => {
                self.terminal_prompt = None;
                let plan = resolve_user_password_confirm_prompt(
                    TerminalUserPasswordFlow::ChangeAuth,
                    username,
                    first_password,
                    confirmation,
                );
                self.apply_terminal_user_management_prompt_plan(plan);
            }
            PromptOutcome::ConfirmDeleteUser {
                username,
                confirmed,
            } => {
                self.terminal_prompt = None;
                if confirmed {
                    self.apply_shell_status_result(delete_desktop_user(&username));
                    self.invalidate_user_cache();
                }
                self.set_user_management_mode(UserManagementMode::Root, 0);
            }
            PromptOutcome::ConfirmToggleAdmin {
                username,
                confirmed,
            } => {
                self.terminal_prompt = None;
                if confirmed {
                    self.apply_shell_status_result(toggle_desktop_user_admin(&username));
                    self.invalidate_user_cache();
                }
                self.set_user_management_mode(UserManagementMode::Root, 0);
            }
            PromptOutcome::EditMenuAddProgramName { target, name } => {
                self.terminal_prompt = None;
                let name = name.trim().to_string();
                if name.is_empty() {
                    self.apply_status_update(invalid_input_shell_status());
                    return;
                }
                self.open_input_prompt(
                    format!("Edit {}", target.title()),
                    format!("Enter launch command for '{name}':"),
                    TerminalPromptAction::EditMenuAddProgramCommand { target, name },
                );
            }
            PromptOutcome::EditMenuAddProgramCommand {
                target,
                name,
                command,
            } => {
                self.terminal_prompt = None;
                self.add_program_entry(target, name, command);
            }
            PromptOutcome::EditMenuAddCategoryName(name) => {
                self.terminal_prompt = None;
                let name = name.trim().to_string();
                if name.is_empty() {
                    self.apply_status_update(invalid_input_shell_status());
                    return;
                }
                self.open_input_prompt(
                    "Edit Documents",
                    "Enter folder path:",
                    TerminalPromptAction::EditMenuAddCategoryPath { name },
                );
            }
            PromptOutcome::EditMenuAddCategoryPath { name, path } => {
                self.terminal_prompt = None;
                if path.trim().is_empty() {
                    self.apply_status_update(invalid_input_shell_status());
                    return;
                }
                self.add_document_category(name, path);
            }
            PromptOutcome::FileManagerRename { .. }
            | PromptOutcome::FileManagerMoveTo { .. }
            | PromptOutcome::FileManagerOpenWithNewCommand { .. }
            | PromptOutcome::FileManagerOpenWithEditCommand { .. } => {
                unreachable!("file manager prompt outcomes are handled before this match")
            }
            PromptOutcome::ConfirmEditMenuDelete {
                target,
                name,
                confirmed,
            } => {
                self.terminal_prompt = None;
                if confirmed {
                    self.delete_program_entry(target, &name);
                } else {
                    self.apply_status_update(cancelled_shell_status());
                }
            }
            PromptOutcome::NewLogName(name) => {
                self.terminal_prompt = None;
                self.create_or_open_log(&name);
            }
            PromptOutcome::Noop => {
                self.terminal_prompt = None;
            }
            PromptOutcome::DefaultAppCustom { slot, raw } => {
                self.terminal_prompt = None;
                match resolve_custom_default_app_binding(&raw) {
                    Ok(binding) => {
                        apply_default_app_binding(&mut self.settings.draft, slot, binding);
                        self.persist_native_settings();
                    }
                    Err(status) => {
                        self.shell_status = status;
                    }
                }
            }
            PromptOutcome::InstallerSearch(query) => {
                self.terminal_prompt = None;
                let event = apply_installer_search_query(&mut self.terminal_installer, &query);
                self.apply_installer_event(event);
            }
            PromptOutcome::InstallerFilter(filter) => {
                self.terminal_prompt = None;
                apply_installer_filter(&mut self.terminal_installer, &filter);
            }
            PromptOutcome::InstallerDisplayName {
                pkg,
                target,
                display_name,
            } => {
                self.terminal_prompt = None;
                let event =
                    add_package_to_menu(&mut self.terminal_installer, &pkg, target, &display_name);
                self.invalidate_program_catalog_cache();
                self.apply_installer_event(event);
            }
            PromptOutcome::ConfirmInstallerAction {
                pkg,
                action,
                confirmed,
            } => {
                self.terminal_prompt = None;
                if confirmed {
                    let event = build_package_command(&mut self.terminal_installer, &pkg, action);
                    self.apply_installer_event(event);
                } else {
                    self.apply_status_update(cancelled_shell_status());
                }
            }
            PromptOutcome::ConnectionSearch { kind, group, query } => {
                self.terminal_prompt = None;
                let event = apply_connection_search_query(
                    &mut self.terminal_connections,
                    kind,
                    group,
                    &query,
                );
                let request = resolve_terminal_connections_request(
                    &mut self.terminal_connections,
                    event,
                    connections_macos_disabled_hint(),
                );
                self.apply_terminal_connections_request(request);
            }
            PromptOutcome::ConnectionPassword {
                kind,
                name,
                detail,
                password,
            } => {
                self.terminal_prompt = None;
                if matches!(kind, ConnectionKind::Network)
                    && connection_requires_password(&detail)
                    && password.trim().is_empty()
                {
                    self.apply_status_update(cancelled_shell_status());
                    return;
                }
                let target = DiscoveredConnection { name, detail };
                self.connect_target(
                    kind,
                    target,
                    if password.trim().is_empty() {
                        None
                    } else {
                        Some(password)
                    },
                );
            }
        }
    }

    fn consume_terminal_prompt_keys(&self, ctx: &Context) {
        ctx.input_mut(|i| {
            for mods in [Modifiers::NONE, Modifiers::SHIFT] {
                i.consume_key(mods, Key::Enter);
                i.consume_key(mods, Key::Space);
                i.consume_key(mods, Key::Tab);
                i.consume_key(mods, Key::Escape);
                i.consume_key(mods, Key::ArrowUp);
                i.consume_key(mods, Key::ArrowDown);
                i.consume_key(mods, Key::ArrowLeft);
                i.consume_key(mods, Key::ArrowRight);
                i.consume_key(mods, Key::Backspace);
            }
        });
    }

    fn connect_target(
        &mut self,
        kind: ConnectionKind,
        target: DiscoveredConnection,
        password: Option<String>,
    ) {
        match connect_connection_and_refresh_settings(kind, &target, password.as_deref()) {
            Ok((settings, status)) => {
                self.replace_settings_draft(settings);
                self.shell_status = status;
            }
            Err(err) => self.shell_status = err.to_string(),
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

    fn draw_top_bar_app_menu(&mut self, ui: &mut egui::Ui, ctx: &Context, app_menu_name: &str) {
        let menu = ui.menu_button(
            RichText::new(app_menu_name).strong().color(Color32::BLACK),
            |ui| {
                Self::apply_top_dropdown_menu_style(ui);
                let items = build_app_control_menu(self.desktop_active_window.is_some());
                self.draw_desktop_menu_items(ui, ctx, &items);
            },
        );
        if menu.response.clicked() {
            self.close_desktop_overlays();
        }
    }

    fn active_editor_text_edit_id(&self) -> Id {
        let generation = self.desktop_window_generation(DesktopWindow::Editor);
        Id::new(("editor_text_edit", generation))
    }

    fn apply_desktop_menu_action(&mut self, ctx: &Context, action: &DesktopMenuAction) {
        match action {
            DesktopMenuAction::EditorCommand(command) => self.run_editor_command(*command),
            DesktopMenuAction::EditorTextCommand(command) => {
                self.run_editor_text_command(ctx, self.active_editor_text_edit_id(), *command);
            }
            DesktopMenuAction::OpenRecentEditorFile(path) => {
                self.open_path_in_editor(path.clone());
            }
            DesktopMenuAction::FileManagerCommand(command) => {
                self.run_file_manager_command(*command);
            }
            DesktopMenuAction::OpenFileManagerPrompt(request) => {
                self.open_file_manager_prompt(request.clone());
            }
            DesktopMenuAction::FileManagerLaunchOpenWithCommand {
                path,
                ext_key,
                command,
            } => match self.launch_open_with_command(path, command) {
                Ok(message) => {
                    self.apply_file_manager_settings_update(
                        FileManagerSettingsUpdate::RecordOpenWithCommand {
                            ext_key: ext_key.clone(),
                            command: command.clone(),
                        },
                    );
                    self.shell_status = message;
                }
                Err(err) => {
                    self.shell_status = format!("Open failed: {err}");
                }
            },
            DesktopMenuAction::FileManagerSetOpenWithDefault { ext_key, command } => {
                self.apply_file_manager_settings_update(
                    FileManagerSettingsUpdate::SetOpenWithDefaultCommand {
                        ext_key: ext_key.clone(),
                        command: command.clone(),
                    },
                );
                self.shell_status = if let Some(command) = command {
                    file_manager_app::open_with_set_default_status(command, ext_key)
                } else {
                    file_manager_app::open_with_cleared_default_status(ext_key)
                };
            }
            DesktopMenuAction::FileManagerRemoveOpenWithCommand { ext_key, command } => {
                self.apply_file_manager_settings_update(
                    FileManagerSettingsUpdate::RemoveOpenWithCommand {
                        ext_key: ext_key.clone(),
                        command: command.clone(),
                    },
                );
                self.shell_status = file_manager_app::open_with_removed_saved_status(ext_key);
            }
            DesktopMenuAction::OpenFileManager => {
                self.launch_standalone_file_manager(None);
            }
            DesktopMenuAction::OpenApplications => {
                self.launch_standalone_applications();
            }
            DesktopMenuAction::OpenSettings => {
                self.open_standalone_settings(None);
            }
            DesktopMenuAction::ToggleStartMenu => {
                if self.start_open {
                    self.close_start_menu();
                } else {
                    self.open_start_menu();
                }
            }
            DesktopMenuAction::CloseActiveDesktopWindow => {
                if let Some(window) = self.desktop_active_window {
                    self.close_desktop_window(window);
                }
            }
            DesktopMenuAction::MinimizeActiveDesktopWindow => {
                if let Some(window) = self.desktop_active_window {
                    self.set_desktop_window_minimized(window, true);
                }
            }
            DesktopMenuAction::ActivateDesktopWindow(window) => {
                if *window == DesktopWindow::Editor
                    && !self.desktop_window_is_open(DesktopWindow::Editor)
                    && self.editor.path.is_none()
                {
                    self.new_document();
                } else if !self.desktop_window_is_open(*window) {
                    self.open_desktop_window(*window);
                } else {
                    self.focus_desktop_window(Some(ctx), *window);
                    self.close_desktop_overlays();
                }
            }
            DesktopMenuAction::ActivateTaskbarWindow(window) => {
                if !self.desktop_window_is_open(*window) {
                    self.open_desktop_window(*window);
                } else if self.desktop_window_is_minimized(*window) {
                    self.set_desktop_window_minimized(*window, false);
                    self.close_desktop_overlays();
                } else if self.desktop_active_window == Some(*window) {
                    self.set_desktop_window_minimized(*window, true);
                    self.close_desktop_overlays();
                } else {
                    self.focus_desktop_window(Some(ctx), *window);
                    self.close_desktop_overlays();
                }
            }
            DesktopMenuAction::OpenManual { path, status_label } => {
                self.open_manual_file(path, status_label);
            }
        }
    }

    fn draw_desktop_menu_items(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &Context,
        items: &[DesktopMenuItem],
    ) {
        for item in items {
            match item {
                DesktopMenuItem::Action { label, action } => {
                    if ui.button(label).clicked() {
                        self.apply_desktop_menu_action(ctx, action);
                        ui.close_menu();
                    }
                }
                DesktopMenuItem::Disabled { label } => {
                    let _ = Self::retro_disabled_button(ui, label);
                }
                DesktopMenuItem::Label { label } => {
                    ui.label(RichText::new(label).small());
                }
                DesktopMenuItem::Separator => Self::retro_separator(ui),
                DesktopMenuItem::Submenu { label, items } => {
                    ui.menu_button(label, |ui| {
                        Self::apply_top_dropdown_menu_style(ui);
                        self.draw_desktop_menu_items(ui, ctx, items);
                    });
                }
            }
        }
    }

    fn draw_top_bar_standard_menu(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &Context,
        section: DesktopMenuSection,
    ) {
        let menu = ui.menu_button(section.label(), |ui| {
            Self::apply_top_dropdown_menu_style(ui);
            if section == DesktopMenuSection::Format {
                ui.set_min_width(160.0);
                ui.set_max_width(220.0);
            }
            let active_app = self.active_desktop_app();
            let menu_context = DesktopMenuBuildContext {
                editor: &self.editor,
                editor_recent_files: &self.settings.draft.editor_recent_files,
                file_manager: &self.file_manager,
                file_manager_runtime: &self.file_manager_runtime,
                file_manager_settings: &self.live_desktop_file_manager_settings,
            };
            let items = build_active_desktop_menu_section(active_app, section, &menu_context);
            if !items.is_empty() {
                self.draw_desktop_menu_items(ui, ctx, &items);
            }
            let shared_items = build_shared_desktop_menu_section(section);
            if !shared_items.is_empty() {
                self.draw_desktop_menu_items(ui, ctx, &shared_items);
            }
        });
        if menu.response.clicked() {
            self.close_desktop_overlays();
        }
    }

    fn draw_top_bar_window_menu(&mut self, ui: &mut egui::Ui, ctx: &Context) {
        let menu = ui.menu_button("Window", |ui| {
            Self::apply_top_dropdown_menu_style(ui);
            let entries: Vec<DesktopWindowMenuEntry> = desktop_components()
                .iter()
                .filter(|component| component.spec.show_in_window_menu)
                .map(|component| DesktopWindowMenuEntry {
                    window: component.spec.window,
                    open: self.desktop_window_is_open(component.spec.window),
                    active: self.desktop_active_window == Some(component.spec.window),
                })
                .collect();
            let items = build_window_menu_section(
                &entries,
                self.terminal_pty.as_ref().map(|pty| pty.title.as_str()),
            );
            self.draw_desktop_menu_items(ui, ctx, &items);
        });
        if menu.response.clicked() {
            self.close_desktop_overlays();
        }
    }

    fn draw_top_bar_help_menu(&mut self, ui: &mut egui::Ui, ctx: &Context) {
        let menu = ui.menu_button("Help", |ui| {
            Self::apply_top_dropdown_menu_style(ui);
            let items = build_shared_desktop_menu_section(DesktopMenuSection::Help);
            self.draw_desktop_menu_items(ui, ctx, &items);
        });
        if menu.response.clicked() {
            self.close_desktop_overlays();
        }
    }

    fn draw_top_bar_menu_section(
        &mut self,
        ctx: &Context,
        ui: &mut egui::Ui,
        section: DesktopMenuSection,
    ) {
        match section {
            DesktopMenuSection::File
            | DesktopMenuSection::Edit
            | DesktopMenuSection::Format
            | DesktopMenuSection::View => self.draw_top_bar_standard_menu(ui, ctx, section),
            DesktopMenuSection::Window => self.draw_top_bar_window_menu(ui, ctx),
            DesktopMenuSection::Help => self.draw_top_bar_help_menu(ui, ctx),
        }
    }

    fn draw_top_bar(&mut self, ctx: &Context) {
        let palette = current_palette();
        Self::apply_global_retro_menu_chrome(ctx, &palette);
        let app_menu_name = desktop_app_menu_name(
            self.desktop_active_window,
            self.terminal_pty.as_ref().map(|pty| pty.title.as_str()),
        );
        let active_app = self.active_desktop_app();
        TopBottomPanel::top("native_top_bar")
            .exact_height(30.0)
            .show_separator_line(false)
            .show(ctx, |ui| {
                ui.painter()
                    .rect_filled(ui.max_rect(), 0.0, palette.selected_bg);
                ui.horizontal(|ui| {
                    Self::apply_top_bar_menu_button_style(ui);
                    ui.spacing_mut().item_spacing.x = 14.0;
                    self.draw_top_bar_app_menu(ui, ctx, &app_menu_name);
                    ui.add_space(10.0);
                    for section in active_app.menu_sections() {
                        self.draw_top_bar_menu_section(ctx, ui, *section);
                    }
                    ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                        let batt = crate::status::battery_status_string();
                        if !batt.is_empty() {
                            ui.label(RichText::new(batt).color(Color32::BLACK));
                            ui.add_space(10.0);
                        }
                        let now = Local::now().format("%a %d %b %H:%M").to_string();
                        ui.label(RichText::new(now).color(Color32::BLACK));
                        ui.add_space(10.0);
                        if ui
                            .button(RichText::new("Search").color(Color32::BLACK))
                            .clicked()
                            || ctx.input(|i| i.key_pressed(Key::Space) && i.modifiers.command)
                        {
                            if self.spotlight_open {
                                self.close_spotlight();
                            } else {
                                self.open_spotlight();
                            }
                        }
                    });
                });
            });
    }

    fn draw_start_panel(&mut self, ctx: &Context) {
        if !self.start_open {
            return;
        }
        const ROOT_W: f32 = 270.0;
        const SUB_W: f32 = 250.0;
        const LEAF_W: f32 = 270.0;
        const ROW_H: f32 = 24.0;
        const PANEL_PAD_H: f32 = 16.0;
        const TASKBAR_H: f32 = 32.0;
        const ROOT_LEFT: f32 = 8.0;
        const EDGE_PAD: f32 = 8.0;

        let palette = current_palette();
        let screen = ctx.screen_rect();
        let taskbar_top = screen.bottom() - TASKBAR_H;
        let root_x = self
            .desktop_start_button_rect
            .map(|rect| rect.left().max(screen.left() + ROOT_LEFT))
            .unwrap_or(screen.left() + ROOT_LEFT);
        let root_y = (taskbar_top - self.start_root_panel_height).max(screen.top() + EDGE_PAD);
        let mut branch_anchor_y = screen.top() + EDGE_PAD;
        let mut branch_x = root_x + ROOT_W - 2.0;
        let mut root_rect: Option<egui::Rect> = None;

        egui::Area::new(Id::new("native_start_root_panel"))
            .fixed_pos([root_x, root_y])
            .interactable(true)
            .show(ctx, |ui| {
                let frame = egui::Frame::none()
                    .fill(palette.panel)
                    .stroke(egui::Stroke::new(2.0, palette.fg))
                    .inner_margin(egui::Margin::same(8.0));
                let frame_response = frame.show(ui, |ui| {
                    Self::apply_start_menu_highlight_style(ui);
                    ui.set_min_width(ROOT_W);
                    ui.set_max_width(ROOT_W);
                    ui.label(RichText::new("Start").strong().color(palette.fg));
                    Self::retro_separator(ui);

                    for row in START_ROOT_VIS_ROWS {
                        match row {
                            Some(idx) => {
                                let label = START_ROOT_ITEMS[idx];
                                let has_panel = start_root_leaf_for_idx(idx).is_some()
                                    || start_root_submenu_for_idx(idx).is_some();
                                let suffix = if has_panel { " >" } else { "" };
                                let selected = self.start_selected_root == idx;
                                let response = Self::start_menu_row(
                                    ui,
                                    &format!("{label}{suffix}"),
                                    selected,
                                    ROOT_W - 16.0,
                                );
                                if response.hovered() {
                                    self.set_start_panel_for_root(idx);
                                }
                                if response.clicked() {
                                    if let Some(action) = start_root_action_for_idx(idx) {
                                        self.run_start_root_action(action);
                                    } else if has_panel {
                                        self.set_start_panel_for_root(idx);
                                    }
                                }
                                if self.start_selected_root == idx {
                                    branch_anchor_y = response.rect.top() - 2.0;
                                }
                            }
                            None => {
                                Self::retro_separator(ui);
                            }
                        }
                    }
                });
                root_rect = Some(frame_response.response.rect);
                self.start_root_panel_height = frame_response.response.rect.height();
                branch_x = frame_response.response.rect.right() - 2.0;
            });
        let Some(root_rect) = root_rect else {
            return;
        };

        if let Some(submenu) = self.start_open_submenu {
            if submenu == StartSubmenu::System {
                let items = self.start_system_items();
                self.start_system_selected = self
                    .start_system_selected
                    .min(items.len().saturating_sub(1));
                let sub_h = PANEL_PAD_H + ROW_H * (items.len() as f32);
                let sub_y =
                    branch_anchor_y.clamp(screen.top() + EDGE_PAD, root_rect.bottom() - sub_h);
                egui::Area::new(Id::new("native_start_submenu_panel"))
                    .fixed_pos([branch_x, sub_y])
                    .interactable(true)
                    .show(ctx, |ui| {
                        egui::Frame::none()
                            .fill(palette.panel)
                            .stroke(egui::Stroke::new(2.0, palette.fg))
                            .inner_margin(egui::Margin::same(8.0))
                            .show(ui, |ui| {
                                Self::apply_start_menu_highlight_style(ui);
                                ui.set_min_width(SUB_W);
                                ui.set_max_width(SUB_W);
                                for (idx, (label, action)) in items.iter().enumerate() {
                                    let selected = self.start_system_selected == idx;
                                    let response =
                                        Self::start_menu_row(ui, label, selected, SUB_W - 16.0);
                                    if response.hovered() {
                                        self.start_system_selected = idx;
                                    }
                                    if response.clicked() {
                                        self.run_start_system_action(*action);
                                    }
                                }
                            });
                    });
            }
        } else if let Some(leaf) = self.start_open_leaf {
            let items = self.start_leaf_items(leaf);
            self.start_leaf_selected = self.start_leaf_selected.min(items.len().saturating_sub(1));
            let leaf_h = PANEL_PAD_H + ROW_H * (items.len() as f32);
            let leaf_y =
                branch_anchor_y.clamp(screen.top() + EDGE_PAD, root_rect.bottom() - leaf_h);
            let mut leaf_context_action: Option<ContextMenuAction> = None;
            egui::Area::new(Id::new("native_start_leaf_panel"))
                .fixed_pos([branch_x, leaf_y])
                .interactable(true)
                .show(ctx, |ui| {
                    egui::Frame::none()
                        .fill(palette.panel)
                        .stroke(egui::Stroke::new(2.0, palette.fg))
                        .inner_margin(egui::Margin::same(8.0))
                        .show(ui, |ui| {
                            Self::apply_start_menu_highlight_style(ui);
                            ui.set_min_width(LEAF_W);
                            ui.set_max_width(LEAF_W);
                            for (idx, item) in items.iter().enumerate() {
                                let selected = self.start_leaf_selected == idx;
                                let response =
                                    Self::start_menu_row(ui, &item.label, selected, LEAF_W - 16.0);
                                if response.hovered() {
                                    self.start_leaf_selected = idx;
                                }
                                if response.clicked() {
                                    self.run_start_leaf_action(item.action.clone());
                                }
                                if matches!(
                                    leaf,
                                    StartLeaf::Applications | StartLeaf::Games | StartLeaf::Network
                                ) && !matches!(item.action, NativeStartLeafAction::None)
                                {
                                    let item_label = item.label.clone();
                                    let item_action = item.action.clone();
                                    let removable_item = Self::start_leaf_menu_target(&item_action);
                                    response.context_menu(|ui| {
                                        Self::apply_context_menu_style(ui);
                                        ui.set_min_width(136.0);
                                        ui.set_max_width(180.0);
                                        if let Some((target, name)) = removable_item.as_ref() {
                                            if ui.button("Rename").clicked() {
                                                leaf_context_action =
                                                    Some(ContextMenuAction::RenameStartMenuEntry {
                                                        target: *target,
                                                        name: name.clone(),
                                                    });
                                                ui.close_menu();
                                            }
                                            Self::retro_separator(ui);
                                        }
                                        if ui.button("Create Shortcut").clicked() {
                                            leaf_context_action =
                                                Some(ContextMenuAction::CreateShortcut {
                                                    label: item_label.clone(),
                                                    action: item_action.clone(),
                                                });
                                            ui.close_menu();
                                        }
                                        if let Some((target, name)) = removable_item.as_ref() {
                                            Self::retro_separator(ui);
                                            if ui
                                                .button(format!("Remove from {}", target.title()))
                                                .clicked()
                                            {
                                                leaf_context_action =
                                                    Some(ContextMenuAction::RemoveStartMenuEntry {
                                                        target: *target,
                                                        name: name.clone(),
                                                    });
                                                ui.close_menu();
                                            }
                                        }
                                    });
                                }
                            }
                        });
                });
            if let Some(action) = leaf_context_action {
                self.context_menu_action = Some(action);
            }
        }
    }

    fn draw_start_menu_rename_window(&mut self, ctx: &Context) {
        let Some(rename) = self.start_menu_rename.clone() else {
            return;
        };

        let palette = current_palette();
        let mut close = false;
        let mut apply = false;
        let mut name_input = rename.name_input.clone();

        egui::Window::new("start_menu_rename_window")
            .title_bar(false)
            .collapsible(false)
            .resizable(false)
            .fixed_size(egui::vec2(320.0, 124.0))
            .anchor(Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
            .frame(
                egui::Frame::none()
                    .fill(palette.panel)
                    .stroke(egui::Stroke::new(2.0, palette.fg))
                    .inner_margin(egui::Margin::same(12.0)),
            )
            .show(ctx, |ui| {
                Self::apply_context_menu_style(ui);
                ui.label(
                    RichText::new(format!("Rename {}", rename.target.singular()))
                        .strong()
                        .color(palette.fg),
                );
                ui.add_space(8.0);
                ui.label(RichText::new(&rename.original_name).color(palette.dim));
                ui.add_space(6.0);
                let response = ui.add(
                    egui::TextEdit::singleline(&mut name_input)
                        .desired_width(f32::INFINITY)
                        .text_color(palette.fg)
                        .cursor_at_end(true),
                );
                if response.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)) {
                    apply = true;
                }
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    if ui.button("Rename").clicked() {
                        apply = true;
                    }
                    if ui.button("Cancel").clicked() {
                        close = true;
                    }
                });
            });

        if let Some(rename_state) = &mut self.start_menu_rename {
            rename_state.name_input = name_input;
        }
        if apply {
            if let Some(rename_state) = self.start_menu_rename.take() {
                self.rename_program_entry(
                    rename_state.target,
                    &rename_state.original_name,
                    &rename_state.name_input,
                );
            }
            self.close_start_menu();
        } else if close {
            self.start_menu_rename = None;
        }
    }

    fn draw_desktop(&mut self, ctx: &Context) {
        if self.asset_cache.is_none() {
            self.asset_cache = Some(Self::build_asset_cache(ctx));
        }
        self.sync_wallpaper(ctx);
        let palette = current_palette();
        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(palette.bg).inner_margin(0.0))
            .show(ctx, |ui| {
                let rect = ui.max_rect();
                let response = ui.allocate_rect(rect, egui::Sense::click());
                let desktop_dir = robco_desktop_dir();
                let file_manager_drop_hover = response
                    .dnd_hover_payload::<NativeFileManagerDragPayload>()
                    .is_some_and(|payload| {
                        Self::file_manager_drop_allowed(&payload.paths, &desktop_dir)
                    });
                if !self.draw_wallpaper(ui.painter(), rect, &palette) {
                    ui.painter().rect_filled(rect, 0.0, palette.bg);
                }
                if file_manager_drop_hover {
                    ui.painter().rect_stroke(
                        rect.shrink(6.0),
                        0.0,
                        egui::Stroke::new(2.0, palette.fg),
                    );
                }
                if !matches!(
                    self.settings.draft.desktop_icon_style,
                    DesktopIconStyle::NoIcons
                ) {
                    self.draw_desktop_icons(ui);
                }
                if let Some(payload) =
                    response.dnd_release_payload::<NativeFileManagerDragPayload>()
                {
                    if Self::file_manager_drop_allowed(&payload.paths, &desktop_dir) {
                        self.file_manager_handle_drop_to_dir(payload.paths.clone(), desktop_dir);
                    }
                }
                Self::attach_desktop_empty_context_menu(
                    &mut self.context_menu_action,
                    &response,
                    self.settings.draft.desktop_snap_to_grid,
                    self.settings.draft.desktop_icon_sort,
                );
                let dropped_paths: Vec<PathBuf> = ctx.input(|input| {
                    let hovered = input
                        .pointer
                        .hover_pos()
                        .is_some_and(|pos| rect.contains(pos));
                    if !hovered {
                        return Vec::new();
                    }
                    input
                        .raw
                        .dropped_files
                        .iter()
                        .filter_map(|file| file.path.clone())
                        .collect()
                });
                if !dropped_paths.is_empty() {
                    self.import_paths_to_desktop(dropped_paths);
                }
                if response.clicked() {
                    self.close_desktop_overlays();
                    self.desktop_selected_icon = None;
                }
            });
    }

    fn draw_desktop_taskbar(&mut self, ctx: &Context) {
        self.sync_desktop_active_window();
        TopBottomPanel::bottom("native_desktop_taskbar")
            .exact_height(32.0)
            .show_separator_line(false)
            .show(ctx, |ui| {
                let palette = current_palette();
                ui.painter()
                    .rect_filled(ui.max_rect(), 0.0, palette.selected_bg);

                ui.horizontal(|ui| {
                    Self::apply_desktop_panel_button_style(ui);
                    ui.spacing_mut().item_spacing.x = 8.0;
                    let start_response = ui.add(
                        egui::Label::new(
                            RichText::new("[Start]")
                                .strong()
                                .monospace()
                                .color(Color32::BLACK),
                        )
                        .sense(egui::Sense::click()),
                    );
                    self.desktop_start_button_rect = Some(start_response.rect);
                    if start_response.clicked() {
                        if self.start_open {
                            self.close_start_menu();
                        } else {
                            self.open_start_menu();
                        }
                    }
                    ui.label(RichText::new("|").monospace().color(Color32::BLACK));
                    ui.add_space(8.0);
                    let open_windows: Vec<DesktopWindow> = desktop_components()
                        .iter()
                        .filter(|component| component.spec.show_in_taskbar)
                        .map(|component| component.spec.window)
                        .filter(|window| self.desktop_window_is_open(*window))
                        .collect();
                    let entries = build_taskbar_entries(
                        &open_windows,
                        self.desktop_active_window,
                        self.terminal_pty.as_ref().map(|pty| pty.title.as_str()),
                    );
                    for entry in entries {
                        if Self::desktop_bar_button(ui, entry.label, entry.inactive, false)
                            .clicked()
                        {
                            self.apply_desktop_menu_action(
                                ctx,
                                &DesktopMenuAction::ActivateTaskbarWindow(entry.window),
                            );
                            if !self.desktop_window_is_minimized(entry.window) {
                                // Bring the window to the top of the egui layer stack.
                                let layer_id = egui::LayerId::new(
                                    egui::Order::Middle,
                                    self.desktop_window_egui_id(entry.window),
                                );
                                ctx.move_to_top(layer_id);
                            }
                        }
                    }
                });
            });
    }

    fn draw_terminal_main_menu(&mut self, ctx: &Context) {
        let layout = self.terminal_layout();
        let activated = draw_main_menu_screen(
            ctx,
            &mut self.terminal_nav.main_menu_idx,
            &self.shell_status,
            &format!("RobcOS v{}", env!("CARGO_PKG_VERSION")),
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
        if let Some(action) = activated {
            let action = resolve_main_menu_action(action);
            self.apply_main_menu_selection_action(action);
        }
    }

    fn draw_terminal_applications(&mut self, ctx: &Context) {
        let layout = self.terminal_layout();
        let entries = build_terminal_application_entries(
            self.settings.draft.builtin_menu_visibility.text_editor,
            self.settings.draft.builtin_menu_visibility.nuke_codes,
            &catalog_names(ProgramCatalog::Applications),
            BUILTIN_TEXT_EDITOR_APP,
            BUILTIN_NUKE_CODES_APP,
        );
        let event = draw_programs_menu(
            ctx,
            "Applications",
            Some("Built-in and configured apps"),
            &entries,
            &mut self.terminal_nav.apps_idx,
            &self.shell_status,
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
        let request = resolve_terminal_applications_request(
            event,
            BUILTIN_TEXT_EDITOR_APP,
            BUILTIN_NUKE_CODES_APP,
        );
        self.apply_terminal_program_request(request, TerminalScreen::Applications);
    }

    fn open_nuke_codes_screen(&mut self, return_screen: TerminalScreen) {
        self.terminal_nuke_codes = fetch_nuke_codes();
        self.terminal_nav.nuke_codes_return_screen = return_screen;
        self.navigate_to_screen(TerminalScreen::NukeCodes);
        self.apply_status_update(clear_shell_status());
    }

    fn apply_terminal_program_request(
        &mut self,
        request: TerminalProgramRequest,
        launch_return_screen: TerminalScreen,
    ) {
        match request {
            TerminalProgramRequest::None => {}
            TerminalProgramRequest::BackToMainMenu => {
                self.navigate_to_screen(TerminalScreen::MainMenu);
                self.apply_status_update(clear_shell_status());
            }
            TerminalProgramRequest::OpenTextEditor => {
                self.editor.open = true;
                if self.editor.path.is_none() {
                    self.new_document();
                }
                self.shell_status = format!("Opened {BUILTIN_TEXT_EDITOR_APP}.");
            }
            TerminalProgramRequest::OpenNukeCodes => {
                self.open_nuke_codes_screen(launch_return_screen);
            }
            TerminalProgramRequest::OpenBuiltinGame => {
                self.open_terminal_donkey_kong();
            }
            TerminalProgramRequest::LaunchCatalog { name, catalog } => {
                self.open_embedded_catalog_launch(&name, catalog, launch_return_screen);
            }
        }
    }

    fn apply_desktop_program_request(&mut self, request: DesktopProgramRequest) {
        match request {
            DesktopProgramRequest::OpenTextEditor { close_window: _ } => {
                self.launch_standalone_editor(None);
            }
            DesktopProgramRequest::OpenNukeCodes { close_window: _ } => {
                self.launch_standalone_nuke_codes();
            }
            DesktopProgramRequest::OpenBuiltinGame => {
                self.open_desktop_donkey_kong();
            }
            DesktopProgramRequest::LaunchCatalog { name, catalog, .. } => {
                self.open_desktop_catalog_launch(&name, catalog);
            }
        }
    }

    fn draw_terminal_documents(&mut self, ctx: &Context) {
        let layout = self.terminal_layout();
        let mut items = vec!["Logs".to_string()];
        items.extend(Self::sorted_document_categories());
        items.push("---".to_string());
        items.push("Back".to_string());
        let mut selected = self
            .terminal_nav
            .documents_idx
            .min(items.len().saturating_sub(1));
        let activated = draw_terminal_menu_screen(
            ctx,
            "Documents",
            Some("Select Document Type"),
            &items,
            &mut selected,
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
            &self.shell_status,
        );
        self.terminal_nav.documents_idx = selected;
        if let Some(idx) = activated {
            let selected = items[idx].as_str();
            match selected {
                "Logs" => {
                    self.navigate_to_screen(TerminalScreen::Logs);
                    self.terminal_nav.logs_idx = 0;
                    self.apply_status_update(clear_shell_status());
                }
                "Back" => {
                    self.navigate_to_screen(TerminalScreen::MainMenu);
                    self.apply_status_update(clear_shell_status());
                }
                "---" => {}
                category => {
                    let Some(path) = document_category_path(category) else {
                        self.shell_status = format!("Error: invalid category '{category}'.");
                        return;
                    };
                    self.open_document_browser_at(path, TerminalScreen::Documents);
                }
            }
        }
    }

    fn draw_terminal_logs(&mut self, ctx: &Context) {
        let layout = self.terminal_layout();
        let items = vec![
            "New Log".to_string(),
            "View Logs".to_string(),
            "---".to_string(),
            "Back".to_string(),
        ];
        let mut selected = self
            .terminal_nav
            .logs_idx
            .min(items.len().saturating_sub(1));
        let activated = draw_terminal_menu_screen(
            ctx,
            "Logs",
            None,
            &items,
            &mut selected,
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
            &self.shell_status,
        );
        self.terminal_nav.logs_idx = selected;
        if let Some(idx) = activated {
            match items[idx].as_str() {
                "New Log" => {
                    let default_stem = Local::now().format("%Y-%m-%d").to_string();
                    self.open_input_prompt(
                        "New Log",
                        format!("Document name (.txt default, blank for {default_stem}.txt):"),
                        TerminalPromptAction::NewLogName,
                    );
                }
                "View Logs" => self.open_log_view(),
                "Back" => {
                    self.navigate_to_screen(TerminalScreen::Documents);
                    self.apply_status_update(clear_shell_status());
                }
                _ => {}
            }
        }
    }

    fn draw_terminal_document_browser(&mut self, ctx: &Context) {
        let layout = self.terminal_layout();
        let activated = draw_terminal_document_browser(
            ctx,
            &self.file_manager,
            &mut self.terminal_nav.browser_idx,
            &self.shell_status,
            layout.cols,
            layout.rows,
            layout.header_start_row,
            layout.separator_top_row,
            layout.title_row,
            layout.separator_bottom_row,
            layout.subtitle_row,
            layout.menu_start_row,
            layout.status_row,
            layout.status_row_alt,
            layout.content_col,
        );
        if activated.is_some() {
            match activate_browser_selection(&mut self.file_manager, self.terminal_nav.browser_idx)
            {
                TerminalDocumentBrowserRequest::None => {}
                TerminalDocumentBrowserRequest::ChangedDir => {
                    self.terminal_nav.browser_idx = 0;
                }
                TerminalDocumentBrowserRequest::OpenFile(path) => {
                    self.file_manager.select(Some(path));
                    self.activate_file_manager_selection();
                }
            }
        }
    }

    fn draw_terminal_settings(&mut self, ctx: &Context) {
        let layout = self.terminal_layout();
        let previous_window_mode = self.settings.draft.native_startup_window_mode;
        let event = run_terminal_settings_screen(
            ctx,
            &mut self.settings.draft,
            &mut self.terminal_settings_panel,
            &mut self.terminal_nav.settings_idx,
            &mut self.terminal_nav.settings_choice,
            self.session.as_ref().is_some_and(|s| s.is_admin),
            &self.shell_status,
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
        match event {
            TerminalSettingsEvent::None => {}
            TerminalSettingsEvent::Persist => {
                self.persist_native_settings();
                if self.settings.draft.native_startup_window_mode != previous_window_mode {
                    self.apply_native_window_mode(ctx);
                }
            }
            TerminalSettingsEvent::OpenPanel(panel) => {
                self.terminal_settings_panel = panel;
                self.terminal_nav.settings_idx = 0;
                self.terminal_nav.settings_choice = None;
                self.apply_status_update(clear_shell_status());
            }
            TerminalSettingsEvent::Back => {
                if matches!(self.terminal_settings_panel, TerminalSettingsPanel::Home) {
                    self.apply_terminal_screen_open_plan(terminal_screen_open_plan(
                        TerminalScreen::MainMenu,
                        0,
                        true,
                    ));
                } else {
                    self.terminal_settings_panel = TerminalSettingsPanel::Home;
                    self.terminal_nav.settings_idx = 0;
                    self.terminal_nav.settings_choice = None;
                    self.apply_status_update(clear_shell_status());
                }
            }
            TerminalSettingsEvent::OpenConnections => {
                self.apply_terminal_screen_open_plan(terminal_screen_open_plan(
                    TerminalScreen::Connections,
                    0,
                    true,
                ));
            }
            TerminalSettingsEvent::OpenEditMenus => {
                self.navigate_to_screen(TerminalScreen::EditMenus);
                self.terminal_edit_menus.reset();
                self.apply_status_update(clear_shell_status());
            }
            TerminalSettingsEvent::OpenDefaultApps => {
                self.apply_terminal_screen_open_plan(terminal_screen_open_plan(
                    TerminalScreen::DefaultApps,
                    0,
                    true,
                ));
            }
            TerminalSettingsEvent::OpenAbout => {
                self.apply_terminal_screen_open_plan(terminal_screen_open_plan(
                    TerminalScreen::About,
                    0,
                    true,
                ));
            }
            TerminalSettingsEvent::EnterUserManagement => {
                self.apply_terminal_screen_open_plan(terminal_screen_open_plan(
                    TerminalScreen::UserManagement,
                    0,
                    true,
                ));
            }
        }
    }

    fn draw_terminal_edit_menus(&mut self, ctx: &Context) {
        let layout = self.terminal_layout();
        let applications = self.edit_program_entries(EditMenuTarget::Applications);
        let documents = self.edit_program_entries(EditMenuTarget::Documents);
        let network = self.edit_program_entries(EditMenuTarget::Network);
        let games = self.edit_program_entries(EditMenuTarget::Games);
        let event = draw_edit_menus_screen(
            ctx,
            &mut self.terminal_edit_menus,
            EditMenusEntries {
                applications: &applications,
                documents: &documents,
                network: &network,
                games: &games,
            },
            self.settings.draft.builtin_menu_visibility.nuke_codes,
            self.settings.draft.builtin_menu_visibility.text_editor,
            &self.shell_status,
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
        match event {
            TerminalEditMenusRequest::None => {}
            TerminalEditMenusRequest::BackToSettings => {
                self.apply_terminal_screen_open_plan(terminal_settings_refresh_plan());
            }
            TerminalEditMenusRequest::PersistToggleBuiltinNukeCodes => {
                self.settings.draft.builtin_menu_visibility.nuke_codes =
                    !self.settings.draft.builtin_menu_visibility.nuke_codes;
                self.persist_native_settings();
            }
            TerminalEditMenusRequest::PersistToggleBuiltinTextEditor => {
                self.settings.draft.builtin_menu_visibility.text_editor =
                    !self.settings.draft.builtin_menu_visibility.text_editor;
                self.persist_native_settings();
            }
            TerminalEditMenusRequest::OpenPromptAddProgramName {
                target,
                title,
                prompt,
            } => {
                self.open_input_prompt(
                    title,
                    prompt,
                    TerminalPromptAction::EditMenuAddProgramName { target },
                );
            }
            TerminalEditMenusRequest::OpenPromptAddCategoryName { title, prompt } => {
                self.open_input_prompt(
                    title,
                    prompt,
                    TerminalPromptAction::EditMenuAddCategoryName,
                );
            }
            TerminalEditMenusRequest::OpenConfirmDelete {
                target,
                title,
                prompt,
                name,
            } => {
                self.open_confirm_prompt(
                    title,
                    prompt,
                    TerminalPromptAction::ConfirmEditMenuDelete { target, name },
                );
            }
            TerminalEditMenusRequest::Status(status) => {
                self.shell_status = status;
            }
        }
    }

    fn apply_terminal_connections_request(&mut self, request: TerminalConnectionsRequest) {
        match request {
            TerminalConnectionsRequest::None => {}
            TerminalConnectionsRequest::BackToSettings => {
                self.apply_terminal_screen_open_plan(terminal_settings_refresh_plan());
            }
            TerminalConnectionsRequest::NavigateToView {
                view,
                clear_status,
                reset_kind_idx,
                reset_picker_idx,
            } => {
                crate::sound::play_navigate();
                self.terminal_connections.view = view;
                if reset_kind_idx {
                    self.terminal_connections.kind_idx = 0;
                }
                if reset_picker_idx {
                    self.terminal_connections.picker_idx = 0;
                }
                if clear_status {
                    self.apply_status_update(clear_shell_status());
                }
            }
            TerminalConnectionsRequest::OpenPromptSearch {
                kind,
                group,
                title,
                prompt,
            } => {
                self.open_input_prompt(
                    &title,
                    prompt,
                    TerminalPromptAction::ConnectionSearch { kind, group },
                );
            }
            TerminalConnectionsRequest::OpenPasswordPrompt {
                kind,
                target,
                title,
                prompt,
            } => {
                self.open_password_prompt_with_action(
                    &title,
                    prompt,
                    TerminalPromptAction::ConnectionPassword {
                        kind,
                        name: target.name,
                        detail: target.detail,
                    },
                );
            }
            TerminalConnectionsRequest::ConnectImmediate { kind, target } => {
                self.connect_target(kind, target, None);
            }
            TerminalConnectionsRequest::Status {
                status,
                back_to_settings,
            } => {
                self.shell_status = status;
                if back_to_settings {
                    self.apply_terminal_screen_open_plan(terminal_settings_refresh_plan());
                }
            }
        }
    }

    fn draw_terminal_connections(&mut self, ctx: &Context) {
        let layout = self.terminal_layout();
        let request = draw_terminal_connections_screen(
            ctx,
            &mut self.terminal_connections,
            &self.shell_status,
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
        self.apply_terminal_connections_request(request);
    }

    fn draw_terminal_prompt_overlay_global(&self, ctx: &Context) {
        let layout = self.terminal_layout();
        let Some(prompt) = self.terminal_prompt.as_ref() else {
            return;
        };
        let viewport = ctx.screen_rect();
        egui::Area::new(Id::new("native_terminal_prompt_overlay"))
            .order(egui::Order::Foreground)
            .fixed_pos(viewport.min)
            .show(ctx, |ui| {
                ui.set_min_size(viewport.size());
                let (screen, _) = RetroScreen::new(ui, layout.cols, layout.rows);
                draw_terminal_prompt_overlay(ui, &screen, prompt);
            });
    }

    fn draw_terminal_default_apps(&mut self, ctx: &Context) {
        let layout = self.terminal_layout();
        let event = draw_default_apps_screen(
            ctx,
            &self.settings.draft,
            &mut self.terminal_nav.default_apps_idx,
            &mut self.terminal_nav.default_app_choice_idx,
            &mut self.terminal_nav.default_app_slot,
            &self.shell_status,
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
        match event {
            TerminalDefaultAppsRequest::None => {}
            TerminalDefaultAppsRequest::BackToSettings => {
                self.apply_terminal_screen_open_plan(terminal_settings_refresh_plan());
            }
            TerminalDefaultAppsRequest::OpenSlot(slot) => {
                crate::sound::play_navigate();
                self.terminal_nav.default_app_slot = Some(slot);
                self.terminal_nav.default_app_choice_idx = 0;
            }
            TerminalDefaultAppsRequest::CloseSlotPicker => {
                crate::sound::play_navigate();
                self.terminal_nav.default_app_slot = None;
            }
            TerminalDefaultAppsRequest::ApplyBinding { slot, binding } => {
                apply_default_app_binding(&mut self.settings.draft, slot, binding);
                self.persist_native_settings();
                self.terminal_nav.default_app_slot = None;
            }
            TerminalDefaultAppsRequest::PromptCustom { slot, prompt_label } => {
                self.open_input_prompt(
                    "Default Apps",
                    format!("{prompt_label} command (example: epy):"),
                    TerminalPromptAction::DefaultAppCustom { slot },
                );
            }
        }
    }

    fn draw_terminal_about(&mut self, ctx: &Context) {
        let layout = self.terminal_layout();
        match draw_about_screen(
            ctx,
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
        ) {
            TerminalAboutRequest::None => {}
            TerminalAboutRequest::Back => {
                self.apply_terminal_screen_open_plan(terminal_settings_refresh_plan());
            }
        }
    }

    fn draw_terminal_network(&mut self, ctx: &Context) {
        let layout = self.terminal_layout();
        let entries = catalog_names(ProgramCatalog::Network);
        let event = draw_programs_menu(
            ctx,
            "Network",
            Some("Select Network Program"),
            &entries,
            &mut self.terminal_nav.network_idx,
            &self.shell_status,
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
        let request = resolve_terminal_catalog_request(event, ProgramCatalog::Network);
        self.apply_terminal_program_request(request, TerminalScreen::Network);
    }

    fn draw_terminal_games(&mut self, ctx: &Context) {
        let layout = self.terminal_layout();
        let entries = build_terminal_game_entries(
            &catalog_names(ProgramCatalog::Games),
            BUILTIN_DONKEY_KONG_GAME,
        );
        let event = draw_programs_menu(
            ctx,
            "Games",
            Some("Select Game"),
            &entries,
            &mut self.terminal_nav.games_idx,
            &self.shell_status,
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
        let request = resolve_terminal_games_request(event, BUILTIN_DONKEY_KONG_GAME);
        self.apply_terminal_program_request(request, TerminalScreen::Games);
    }

    fn draw_terminal_donkey_kong(&mut self, ctx: &Context) {
        ctx.request_repaint();
        let theme = self.current_donkey_kong_theme();
        let dt = ctx.input(|i| i.stable_dt).max(1.0 / 60.0);
        let input = donkey_kong_input_from_ctx(ctx);
        let game = self.ensure_donkey_kong_loaded(ctx);
        game.set_theme(theme);
        game.update(input, dt);

        egui::CentralPanel::default()
            .frame(
                egui::Frame::none()
                    .fill(current_palette().bg)
                    .inner_margin(egui::Margin::same(12.0)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(BUILTIN_DONKEY_KONG_GAME).strong());
                    ui.separator();
                    ui.small("Arrow keys / WASD move");
                    ui.separator();
                    ui.small("Space jump / restart");
                    ui.separator();
                    ui.small("Esc back");
                });
                ui.add_space(8.0);
                let game_rect = ui.available_rect_before_wrap();
                game.draw(ui, game_rect);
                ui.allocate_rect(game_rect, egui::Sense::hover());
            });
    }

    fn draw_terminal_nuke_codes(&mut self, ctx: &Context) {
        let layout = self.terminal_layout();
        match draw_nuke_codes_screen(
            ctx,
            &self.terminal_nuke_codes,
            layout.cols,
            layout.rows,
            layout.header_start_row,
            layout.separator_top_row,
            layout.title_row,
            layout.separator_bottom_row,
            layout.menu_start_row,
            layout.status_row,
            layout.content_col,
        ) {
            NukeCodesEvent::None => {}
            NukeCodesEvent::Refresh => {
                self.terminal_nuke_codes = fetch_nuke_codes();
            }
            NukeCodesEvent::Back => {
                self.navigate_to_screen(self.terminal_nav.nuke_codes_return_screen);
                self.apply_status_update(clear_shell_status());
            }
        }
    }

    fn draw_terminal_pty(&mut self, ctx: &Context) {
        let layout = self.terminal_layout();
        let Some(state) = self.terminal_pty.as_mut() else {
            self.navigate_to_screen(TerminalScreen::MainMenu);
            self.shell_status = "No embedded PTY session.".to_string();
            return;
        };
        let event = draw_embedded_pty(
            ctx,
            state,
            &self.shell_status,
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
        match event {
            PtyScreenEvent::None => {}
            PtyScreenEvent::CloseRequested => self.handle_terminal_back(),
            PtyScreenEvent::ProcessExited => {
                if let Some(pty) = self.terminal_pty.take() {
                    let plan = resolve_embedded_pty_exit(
                        &pty.title,
                        pty.return_screen,
                        pty.completion_message.as_deref(),
                    );
                    self.apply_terminal_embedded_pty_exit_plan(plan);
                } else {
                    self.navigate_to_screen(TerminalScreen::MainMenu);
                    self.shell_status = "PTY session exited.".to_string();
                }
            }
        }
    }

    fn apply_installer_event(&mut self, event: InstallerEvent) {
        match event {
            InstallerEvent::None => {}
            InstallerEvent::BackToMainMenu => {
                self.apply_terminal_screen_open_plan(terminal_screen_open_plan(
                    TerminalScreen::MainMenu,
                    0,
                    true,
                ));
            }
            InstallerEvent::OpenSearchPrompt => {
                self.open_input_prompt(
                    "Program Installer",
                    "Search packages:",
                    TerminalPromptAction::InstallerSearch,
                );
            }
            InstallerEvent::OpenFilterPrompt => {
                self.open_input_prompt(
                    "Installed Apps",
                    "Filter:",
                    TerminalPromptAction::InstallerFilter,
                );
            }
            InstallerEvent::OpenConfirmAction { pkg, action } => {
                let prompt = match action {
                    InstallerPackageAction::Install => format!("Install {pkg}?"),
                    InstallerPackageAction::Update => format!("Update {pkg}?"),
                    InstallerPackageAction::Reinstall => format!("Reinstall {pkg}?"),
                    InstallerPackageAction::Uninstall => format!("Uninstall {pkg}?"),
                };
                self.open_confirm_prompt(
                    "Program Installer",
                    prompt,
                    TerminalPromptAction::ConfirmInstallerAction { pkg, action },
                );
            }
            InstallerEvent::OpenDisplayNamePrompt { pkg, target } => {
                self.open_input_prompt(
                    "Add to Menu",
                    format!("Display name for '{pkg}':"),
                    TerminalPromptAction::InstallerDisplayName { pkg, target },
                );
            }
            InstallerEvent::LaunchCommand {
                argv,
                status,
                completion_message,
            } => {
                settle_view_after_package_command(&mut self.terminal_installer);
                self.queue_terminal_flash(
                    status.clone(),
                    700,
                    FlashAction::LaunchPty {
                        title: "Program Installer".to_string(),
                        argv,
                        return_screen: TerminalScreen::ProgramInstaller,
                        status: status.clone(),
                        completion_message,
                    },
                );
                self.shell_status = status;
            }
            InstallerEvent::Status(status) => {
                self.queue_terminal_flash(status.clone(), 650, FlashAction::Noop);
                self.shell_status = status;
            }
        }
    }

    fn draw_terminal_program_installer(&mut self, ctx: &Context) {
        let layout = self.terminal_layout();
        let event = draw_installer_screen(
            ctx,
            &mut self.terminal_installer,
            &self.shell_status,
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
        self.apply_installer_event(event);
    }

    fn draw_terminal_user_management(&mut self, ctx: &Context) {
        let layout = self.terminal_layout();
        let mode = self.terminal_nav.user_management_mode.clone();
        let screen = user_management_screen_for_mode(
            &mode,
            self.session.as_ref().map(|s| s.username.as_str()),
            self.live_hacking_difficulty,
        );
        let mut selected = self.terminal_nav.user_management_idx.min(
            screen
                .items
                .iter()
                .filter(|i| i.as_str() != "---")
                .count()
                .saturating_sub(1),
        );
        let refs = screen.items;
        let activated = draw_terminal_menu_screen(
            ctx,
            screen.title,
            screen.subtitle.as_deref(),
            &refs,
            &mut selected,
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
            &self.shell_status,
        );
        self.terminal_nav.user_management_idx = selected;
        if let Some(idx) = activated {
            let selected_label = refs[idx].clone();
            let action = handle_user_management_selection(
                &mode,
                &selected_label,
                self.session.as_ref().map(|s| s.username.as_str()),
            );
            match plan_user_management_action(action) {
                UserManagementExecutionPlan::None => {}
                UserManagementExecutionPlan::OpenCreateUserPrompt => self.open_input_prompt(
                    "Create User",
                    "New username:",
                    TerminalPromptAction::CreateUsername,
                ),
                UserManagementExecutionPlan::CycleHackingDifficulty => {
                    cycle_hacking_difficulty_in_settings(&mut self.settings.draft);
                    self.sync_runtime_settings_cache();
                    self.apply_status_update(saved_shell_status());
                }
                UserManagementExecutionPlan::SetMode { mode, selected_idx } => {
                    self.set_user_management_mode(mode, selected_idx);
                }
                UserManagementExecutionPlan::BackToSettings => {
                    self.apply_terminal_screen_open_plan(terminal_settings_refresh_plan());
                    self.terminal_nav.user_management_idx = 0;
                }
                UserManagementExecutionPlan::OpenCreatePasswordPrompt { username } => {
                    self.open_password_prompt_with_action(
                        "Create User",
                        format!("Password for {username}"),
                        TerminalPromptAction::CreatePassword { username },
                    );
                }
                UserManagementExecutionPlan::ApplyCreateUser { username, method } => {
                    self.apply_shell_status_result(create_desktop_user(&username, method, None));
                    self.invalidate_user_cache();
                    self.set_user_management_mode(UserManagementMode::Root, 0);
                }
                UserManagementExecutionPlan::OpenConfirmDeleteUser { username } => {
                    self.open_confirm_prompt(
                        "Delete User",
                        format!("Delete user '{username}'?"),
                        TerminalPromptAction::ConfirmDeleteUser { username },
                    );
                }
                UserManagementExecutionPlan::OpenResetPasswordPrompt { username } => {
                    self.open_password_prompt_with_action(
                        "Reset Password",
                        format!("New password for '{username}'"),
                        TerminalPromptAction::ResetPassword { username },
                    );
                }
                UserManagementExecutionPlan::OpenChangeAuthPasswordPrompt { username } => {
                    self.open_password_prompt_with_action(
                        "Change Auth Method",
                        format!("New password for '{username}'"),
                        TerminalPromptAction::ChangeAuthPassword { username },
                    );
                }
                UserManagementExecutionPlan::ApplyChangeAuthMethod { username, method } => {
                    self.apply_shell_status_result(update_user_auth_method(
                        &username, method, None,
                    ));
                    self.invalidate_user_cache();
                    self.set_user_management_mode(UserManagementMode::Root, 0);
                }
                UserManagementExecutionPlan::OpenConfirmToggleAdmin { username } => {
                    self.open_confirm_prompt(
                        "Toggle Admin",
                        format!("Toggle admin for '{username}'?"),
                        TerminalPromptAction::ConfirmToggleAdmin { username },
                    );
                }
                UserManagementExecutionPlan::Status(status) => {
                    self.shell_status = status;
                }
            }
        }
    }

    fn terminal_status_bar_repaint_interval(ctx: &Context) -> Duration {
        if !ctx.input(|i| i.focused) {
            return Duration::from_secs(300);
        }
        let now = Local::now();
        Duration::from_secs(u64::from((60 - now.second()).max(1)))
    }

    fn draw_terminal_status_bar(&self, ctx: &Context) {
        ctx.request_repaint_after(Self::terminal_status_bar_repaint_interval(ctx));
        let palette = current_palette();
        TopBottomPanel::bottom("native_terminal_status_bar")
            .resizable(false)
            .exact_height(retro_footer_height())
            .show_separator_line(false)
            .frame(
                egui::Frame::none()
                    .fill(palette.fg)
                    .inner_margin(egui::Margin::symmetric(6.0, 4.0)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    // Left: date/time
                    let now = Local::now().format("%a %Y-%m-%d %I:%M%p").to_string();
                    ui.label(RichText::new(now).color(Color32::BLACK).strong());

                    // Center: session tabs [1*] [2] [3]
                    let tabs = native_session_tabs();
                    if !tabs.labels.is_empty() {
                        let tabs = tabs.labels.join(" ");
                        // Approximate centering
                        let avail = ui.available_width();
                        let tab_width = tabs.len() as f32 * 8.0;
                        let spacing = ((avail - tab_width) / 2.0).max(8.0);
                        ui.add_space(spacing);
                        ui.label(RichText::new(tabs).color(Color32::BLACK).strong());
                    }

                    // Right: battery (if available)
                    ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                        let batt = crate::status::battery_status_string();
                        if !batt.is_empty() {
                            ui.label(RichText::new(batt).color(Color32::BLACK).strong());
                        }
                    });
                });
            });
    }

    fn desktop_workspace_rect(ctx: &Context) -> egui::Rect {
        const TOP_BAR_H: f32 = 30.0;
        const TASKBAR_H: f32 = 32.0;
        let screen = ctx.screen_rect();
        let top = screen.top() + TOP_BAR_H;
        let bottom = (screen.bottom() - TASKBAR_H).max(top + 120.0);
        egui::Rect::from_min_max(
            egui::pos2(screen.left(), top),
            egui::pos2(screen.right(), bottom),
        )
    }

    fn desktop_window_frame() -> egui::Frame {
        let palette = current_palette();
        egui::Frame::none()
            .fill(palette.bg)
            .stroke(egui::Stroke::new(1.0, palette.fg))
            .inner_margin(egui::Margin::same(1.0))
    }

    fn desktop_bar_button(
        ui: &mut egui::Ui,
        label: impl Into<String>,
        active: bool,
        bold: bool,
    ) -> egui::Response {
        let palette = current_palette();
        let label = label.into();
        let fill = if active { palette.fg } else { palette.panel };
        let text = if active {
            RichText::new(label.clone()).color(Color32::BLACK)
        } else {
            RichText::new(label.clone()).color(palette.fg)
        };
        let text = if bold { text.strong() } else { text };
        let response = ui.add(
            egui::Button::new(text)
                .fill(fill)
                .stroke(egui::Stroke::new(2.0, palette.fg)),
        );
        if active {
            let text = if bold {
                RichText::new(label).strong()
            } else {
                RichText::new(label)
            };
            let font = egui::TextStyle::Button.resolve(ui.style());
            ui.painter().text(
                response.rect.center(),
                egui::Align2::CENTER_CENTER,
                text.text(),
                font,
                Color32::BLACK,
            );
        }
        response
    }

    fn desktop_header_glyph_button(ui: &mut egui::Ui, label: &str) -> egui::Response {
        ui.add(
            egui::Button::new(RichText::new(label).color(Color32::BLACK).monospace())
                .frame(false)
                .fill(Color32::TRANSPARENT)
                .stroke(egui::Stroke::NONE)
                .min_size(egui::vec2(0.0, 0.0)),
        )
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

    fn apply_top_bar_menu_button_style(ui: &mut egui::Ui) {
        let palette = current_palette();
        let mut style = ui.style().as_ref().clone();
        // Popup/window fill must be set HERE on the parent UI — menu_button
        // reads these when creating the popup frame, before the inner closure runs.
        style.visuals.panel_fill = palette.bg;
        style.visuals.extreme_bg_color = palette.bg;
        style.visuals.window_fill = palette.bg;
        style.visuals.window_stroke = egui::Stroke::new(2.0, palette.fg);
        style.visuals.window_rounding = egui::Rounding::ZERO;
        style.visuals.menu_rounding = egui::Rounding::ZERO;
        style.visuals.window_shadow = egui::epaint::Shadow::NONE;
        style.visuals.popup_shadow = egui::epaint::Shadow::NONE;
        style.visuals.button_frame = false;
        style.visuals.override_text_color = Some(Color32::BLACK);
        style.visuals.widgets.noninteractive.bg_fill = Color32::TRANSPARENT;
        style.visuals.widgets.noninteractive.weak_bg_fill = Color32::TRANSPARENT;
        style.visuals.widgets.noninteractive.bg_stroke = egui::Stroke::NONE;
        style.visuals.widgets.noninteractive.fg_stroke.color = Color32::BLACK;
        style.visuals.widgets.noninteractive.rounding = egui::Rounding::ZERO;
        style.visuals.widgets.noninteractive.expansion = 0.0;
        style.visuals.widgets.inactive.bg_fill = Color32::TRANSPARENT;
        style.visuals.widgets.inactive.weak_bg_fill = Color32::TRANSPARENT;
        style.visuals.widgets.inactive.bg_stroke = egui::Stroke::NONE;
        style.visuals.widgets.inactive.fg_stroke.color = Color32::BLACK;
        style.visuals.widgets.inactive.rounding = egui::Rounding::ZERO;
        style.visuals.widgets.inactive.expansion = 0.0;
        for visuals in [
            &mut style.visuals.widgets.hovered,
            &mut style.visuals.widgets.active,
            &mut style.visuals.widgets.open,
        ] {
            visuals.bg_fill = palette.selected_bg;
            visuals.weak_bg_fill = palette.selected_bg;
            visuals.bg_stroke = egui::Stroke::NONE;
            visuals.fg_stroke.color = Color32::BLACK;
            visuals.rounding = egui::Rounding::ZERO;
            visuals.expansion = 0.0;
        }
        ui.set_style(style);
    }

    fn apply_top_dropdown_menu_style(ui: &mut egui::Ui) {
        let palette = current_palette();
        let mut style = ui.style().as_ref().clone();
        let stroke = egui::Stroke::new(2.0, palette.fg);
        style.visuals.button_frame = true;
        style.visuals.panel_fill = palette.bg;
        style.visuals.extreme_bg_color = palette.bg;
        style.visuals.window_fill = palette.bg;
        style.visuals.window_stroke = stroke;
        style.visuals.window_rounding = egui::Rounding::ZERO;
        style.visuals.menu_rounding = egui::Rounding::ZERO;
        style.visuals.window_shadow = egui::epaint::Shadow::NONE;
        style.visuals.popup_shadow = egui::epaint::Shadow::NONE;
        style.visuals.override_text_color = None;
        style.spacing.item_spacing.y = 0.0;
        style.visuals.widgets.noninteractive.bg_fill = palette.bg;
        style.visuals.widgets.noninteractive.weak_bg_fill = palette.bg;
        style.visuals.widgets.noninteractive.bg_stroke = egui::Stroke::NONE;
        style.visuals.widgets.noninteractive.fg_stroke.color = palette.fg;
        style.visuals.widgets.noninteractive.rounding = egui::Rounding::ZERO;
        style.visuals.widgets.noninteractive.expansion = 0.0;
        style.visuals.widgets.inactive.bg_fill = palette.bg;
        style.visuals.widgets.inactive.weak_bg_fill = palette.bg;
        style.visuals.widgets.inactive.bg_stroke = egui::Stroke::NONE;
        style.visuals.widgets.inactive.fg_stroke.color = palette.fg;
        style.visuals.widgets.inactive.rounding = egui::Rounding::ZERO;
        style.visuals.widgets.inactive.expansion = 0.0;
        for visuals in [
            &mut style.visuals.widgets.hovered,
            &mut style.visuals.widgets.active,
            &mut style.visuals.widgets.open,
        ] {
            visuals.bg_fill = palette.fg;
            visuals.weak_bg_fill = palette.fg;
            visuals.bg_stroke = egui::Stroke::NONE;
            visuals.fg_stroke.color = Color32::BLACK;
            visuals.rounding = egui::Rounding::ZERO;
            visuals.expansion = 0.0;
        }
        ui.set_style(style);
        ui.painter().rect_filled(ui.max_rect(), 0.0, palette.bg);
    }

    fn apply_context_menu_style(ui: &mut egui::Ui) {
        let palette = current_palette();
        let mut style = ui.style().as_ref().clone();
        let stroke = egui::Stroke::new(2.0, palette.fg);
        style.visuals.button_frame = true;
        style.visuals.window_fill = palette.panel;
        style.visuals.window_stroke = stroke;
        style.visuals.window_rounding = egui::Rounding::ZERO;
        style.visuals.menu_rounding = egui::Rounding::ZERO;
        style.visuals.window_shadow = egui::epaint::Shadow::NONE;
        style.visuals.popup_shadow = egui::epaint::Shadow::NONE;
        style.visuals.override_text_color = None;
        style.spacing.item_spacing = egui::vec2(0.0, 0.0);
        style.spacing.button_padding = egui::vec2(5.0, 2.0);
        style.spacing.menu_margin = egui::Margin::same(2.0);
        style.spacing.interact_size.y = 18.0;
        style.visuals.widgets.noninteractive.bg_fill = Color32::TRANSPARENT;
        style.visuals.widgets.noninteractive.weak_bg_fill = Color32::TRANSPARENT;
        style.visuals.widgets.noninteractive.bg_stroke = egui::Stroke::NONE;
        style.visuals.widgets.noninteractive.fg_stroke.color = palette.fg;
        style.visuals.widgets.noninteractive.rounding = egui::Rounding::ZERO;
        style.visuals.widgets.noninteractive.expansion = 0.0;
        style.visuals.widgets.inactive.bg_fill = Color32::TRANSPARENT;
        style.visuals.widgets.inactive.weak_bg_fill = Color32::TRANSPARENT;
        style.visuals.widgets.inactive.bg_stroke = egui::Stroke::NONE;
        style.visuals.widgets.inactive.fg_stroke.color = palette.fg;
        style.visuals.widgets.inactive.rounding = egui::Rounding::ZERO;
        style.visuals.widgets.inactive.expansion = 0.0;
        for visuals in [
            &mut style.visuals.widgets.hovered,
            &mut style.visuals.widgets.active,
            &mut style.visuals.widgets.open,
        ] {
            visuals.bg_fill = palette.fg;
            visuals.weak_bg_fill = palette.fg;
            visuals.bg_stroke = egui::Stroke::NONE;
            visuals.fg_stroke.color = Color32::BLACK;
            visuals.rounding = egui::Rounding::ZERO;
            visuals.expansion = 0.0;
        }
        ui.set_style(style);
    }

    fn apply_desktop_panel_button_style(ui: &mut egui::Ui) {
        let palette = current_palette();
        let mut style = ui.style().as_ref().clone();
        let stroke = egui::Stroke::new(2.0, palette.fg);
        style.visuals.override_text_color = None;
        style.visuals.window_stroke = stroke;
        style.visuals.window_rounding = egui::Rounding::ZERO;
        style.visuals.menu_rounding = egui::Rounding::ZERO;
        style.visuals.window_shadow = egui::epaint::Shadow::NONE;
        style.visuals.popup_shadow = egui::epaint::Shadow::NONE;
        style.visuals.selection.bg_fill = palette.panel;
        style.visuals.selection.stroke = stroke;
        style.visuals.widgets.noninteractive.bg_fill = palette.panel;
        style.visuals.widgets.noninteractive.weak_bg_fill = palette.panel;
        style.visuals.widgets.noninteractive.bg_stroke = stroke;
        style.visuals.widgets.noninteractive.fg_stroke = stroke;
        style.visuals.widgets.noninteractive.rounding = egui::Rounding::ZERO;
        style.visuals.widgets.noninteractive.expansion = 0.0;
        style.visuals.widgets.inactive.bg_fill = palette.panel;
        style.visuals.widgets.inactive.weak_bg_fill = palette.panel;
        style.visuals.widgets.inactive.bg_stroke = stroke;
        style.visuals.widgets.inactive.fg_stroke = stroke;
        style.visuals.widgets.inactive.rounding = egui::Rounding::ZERO;
        style.visuals.widgets.inactive.expansion = 0.0;
        for visuals in [
            &mut style.visuals.widgets.hovered,
            &mut style.visuals.widgets.active,
            &mut style.visuals.widgets.open,
        ] {
            visuals.bg_fill = palette.panel;
            visuals.weak_bg_fill = palette.panel;
            visuals.bg_stroke = stroke;
            visuals.fg_stroke = stroke;
            visuals.rounding = egui::Rounding::ZERO;
            visuals.expansion = 0.0;
        }
        ui.set_style(style);
    }

    fn apply_start_menu_highlight_style(ui: &mut egui::Ui) {
        let palette = current_palette();
        let mut style = ui.style().as_ref().clone();
        let stroke = egui::Stroke::new(2.0, palette.fg);
        style.visuals.window_stroke = stroke;
        style.visuals.window_rounding = egui::Rounding::ZERO;
        style.visuals.menu_rounding = egui::Rounding::ZERO;
        style.visuals.window_shadow = egui::epaint::Shadow::NONE;
        style.visuals.popup_shadow = egui::epaint::Shadow::NONE;
        style.visuals.selection.bg_fill = palette.fg;
        style.visuals.selection.stroke = stroke;
        style.visuals.widgets.noninteractive.bg_fill = palette.panel;
        style.visuals.widgets.noninteractive.weak_bg_fill = palette.panel;
        style.visuals.widgets.noninteractive.bg_stroke = stroke;
        style.visuals.widgets.noninteractive.fg_stroke = stroke;
        style.visuals.widgets.noninteractive.rounding = egui::Rounding::ZERO;
        style.visuals.widgets.noninteractive.expansion = 0.0;
        style.visuals.widgets.inactive.bg_fill = palette.panel;
        style.visuals.widgets.inactive.weak_bg_fill = palette.panel;
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

    fn start_menu_row(
        ui: &mut egui::Ui,
        label: &str,
        selected: bool,
        width: f32,
    ) -> egui::Response {
        let palette = current_palette();
        let (rect, response) =
            ui.allocate_exact_size(egui::vec2(width, 26.0), egui::Sense::click());
        let active = selected || response.hovered();
        let fill = if active { palette.fg } else { palette.panel };
        let text_color = if active { Color32::BLACK } else { palette.fg };
        ui.painter().rect_filled(rect, 0.0, fill);
        ui.painter().text(
            egui::pos2(rect.left() + 8.0, rect.center().y),
            Align2::LEFT_CENTER,
            label,
            FontId::new(20.0, FontFamily::Monospace),
            text_color,
        );
        response
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

    fn draw_desktop_window_header(
        ui: &mut egui::Ui,
        _title: &str,
        maximized: bool,
    ) -> DesktopHeaderAction {
        let palette = current_palette();
        let mut action = DesktopHeaderAction::None;
        // egui::Frame handles background fill + margin in a single allocation.
        // No manual allocate_exact_size/child_ui, so no "double use of widget".
        egui::Frame::none()
            .fill(palette.fg)
            .inner_margin(egui::Margin::symmetric(8.0, 4.0))
            .show(ui, |ui| {
                ui.set_min_height(20.0);
                ui.horizontal(|ui| {
                    ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add_space(8.0);
                        if Self::desktop_header_glyph_button(ui, "[X]").clicked() {
                            action = DesktopHeaderAction::Close;
                        }
                        if Self::desktop_header_glyph_button(
                            ui,
                            if maximized { "[R]" } else { "[+]" },
                        )
                        .clicked()
                        {
                            action = DesktopHeaderAction::ToggleMaximize;
                        }
                        if Self::desktop_header_glyph_button(ui, "[-]").clicked() {
                            action = DesktopHeaderAction::Minimize;
                        }
                    });
                });
            });
        ui.add_space(2.0);
        action
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

    fn draw_file_manager(&mut self, ctx: &Context) {
        if !self.file_manager.open || self.desktop_window_is_minimized(DesktopWindow::FileManager) {
            return;
        }
        let save_picker_mode = self.editor.save_as_input.is_some();
        let mut open = self.file_manager.open;
        let maximized = self.desktop_window_is_maximized(DesktopWindow::FileManager);
        let restore = self.take_desktop_window_restore_dims(DesktopWindow::FileManager);
        let mut header_action = DesktopHeaderAction::None;
        let generation = self.desktop_window_generation(DesktopWindow::FileManager);
        let default_size = Self::desktop_default_window_size(DesktopWindow::FileManager);
        let min_size = Self::desktop_file_manager_window_min_size();
        let save_picker_size = egui::vec2(860.0, 560.0);
        let mut window = egui::Window::new("File Manager")
            .id(Id::new(("native_file_manager", generation)))
            .open(&mut open)
            .title_bar(false)
            .frame(Self::desktop_window_frame())
            .resizable(true)
            .min_size(min_size)
            .default_size([default_size.x, default_size.y]);
        if save_picker_mode {
            window = window.resizable(false);
            if let Some((pos, _)) = restore {
                window = window.current_pos(pos).fixed_size(save_picker_size);
            } else {
                let pos = Self::desktop_default_window_pos(ctx, save_picker_size);
                window = window.current_pos(pos).fixed_size(save_picker_size);
            }
        } else if maximized {
            let rect = Self::desktop_workspace_rect(ctx);
            window = window
                .movable(false)
                .resizable(false)
                .fixed_pos(rect.min)
                .fixed_size(rect.size());
        } else if let Some((pos, size)) = restore {
            let size = Self::desktop_clamp_window_size(ctx, size, min_size);
            let pos = Self::desktop_clamp_window_pos(ctx, pos, size);
            // Unmaximize or first open with a saved size: generation was bumped so egui
            // has no memory for this ID — default_size sets the initial size correctly.
            window = window.current_pos(pos).default_size(size);
        }
        self.file_manager.ensure_selection_valid();
        let rows = self.file_manager.rows();
        let action_selection_paths: Vec<PathBuf> = self
            .file_manager_selected_entries()
            .into_iter()
            .map(|entry| entry.path)
            .collect();
        let has_editable_selection = !action_selection_paths.is_empty();
        let has_single_file_selection =
            action_selection_paths.len() == 1 && action_selection_paths[0].is_file();
        let has_clipboard = self.file_manager_runtime.has_clipboard();
        let desktop_model = file_manager_desktop::build_desktop_view_model(
            &self.file_manager,
            &self.live_desktop_file_manager_settings,
            &rows,
            self.file_manager_selection_count(),
            has_editable_selection,
            has_single_file_selection,
            has_clipboard,
            self.editor.save_as_input.clone(),
            self.picking_icon_for_shortcut,
            self.picking_wallpaper,
        );
        let footer_model = file_manager_desktop::build_footer_model(&desktop_model);

        self.preload_file_manager_svg_previews(ctx, &desktop_model.rows);

        let search_id = Id::new(("native_file_manager_search", generation));
        let shown = window.show(ctx, |ui| {
            Self::apply_settings_control_style(ui);
            self.draw_file_manager_top_panel(
                ctx,
                ui,
                generation,
                maximized,
                save_picker_mode,
                &desktop_model,
                &search_id,
                &mut header_action,
            );
            self.draw_file_manager_footer_panel(ui, generation, save_picker_mode, &footer_model);
            self.draw_file_manager_tree_panel(ui, generation, save_picker_mode, &desktop_model);
            self.draw_file_manager_content_panel(
                ctx,
                ui,
                generation,
                save_picker_mode,
                &desktop_model,
                &action_selection_paths,
                has_editable_selection,
                has_single_file_selection,
                has_clipboard,
            );
        });
        let shown_rect = shown.as_ref().map(|inner| inner.response.rect);
        let shown_contains_pointer = shown
            .as_ref()
            .is_some_and(|inner| inner.response.contains_pointer());
        self.maybe_activate_desktop_window_from_click(
            ctx,
            DesktopWindow::FileManager,
            shown_contains_pointer,
        );
        if !maximized && !save_picker_mode {
            // Always save the full rect. egui owns window sizing for resizable windows
            // and will not inflate it — only user drag changes it.
            if let Some(rect) = shown_rect {
                self.note_desktop_window_rect(DesktopWindow::FileManager, rect);
            }
        }
        match header_action {
            DesktopHeaderAction::None => {}
            DesktopHeaderAction::Close => open = false,
            DesktopHeaderAction::Minimize => {
                self.set_desktop_window_minimized(DesktopWindow::FileManager, true)
            }
            DesktopHeaderAction::ToggleMaximize => {
                self.toggle_desktop_window_maximized(DesktopWindow::FileManager, shown_rect)
            }
        }
        // If the inner closure forced file_manager.open to false (e.g. Choose Icon),
        // honour that — the local `open` bool was never updated inside the closure.
        if !self.file_manager.open {
            open = false;
        }
        // If the file manager was closed while in a pick mode, cancel the pick.
        if !open {
            self.editor.save_as_input = None;
            self.picking_icon_for_shortcut = None;
            self.picking_wallpaper = false;
        }
        self.update_desktop_window_state(DesktopWindow::FileManager, open);
    }

    fn draw_editor(&mut self, ctx: &Context) {
        if !self.editor.open {
            return;
        }
        if self.desktop_mode_open && self.desktop_window_is_minimized(DesktopWindow::Editor) {
            return;
        }
        if ctx.input(|i| i.key_pressed(Key::S) && i.modifiers.command) {
            self.run_editor_command(EditorCommand::Save);
        }
        if ctx.input(|i| i.key_pressed(Key::F) && i.modifiers.command) {
            self.run_editor_command(EditorCommand::OpenFind);
        }
        if ctx.input(|i| i.key_pressed(Key::H) && i.modifiers.command) {
            self.run_editor_command(EditorCommand::OpenFindReplace);
        }
        if ctx.input(|i| i.key_pressed(Key::Escape)) && self.editor.ui.find_open {
            self.run_editor_command(EditorCommand::CloseFind);
        }
        let title = self
            .editor
            .path
            .as_ref()
            .and_then(|p| p.file_name())
            .and_then(|p| p.to_str())
            .unwrap_or(EDITOR_APP_TITLE)
            .to_string();

        if !self.desktop_mode_open {
            // Keyboard shortcuts for terminal mode (no mouse needed)
            if ctx.input(|i| i.key_pressed(Key::Escape) || i.key_pressed(Key::Tab)) {
                self.update_desktop_window_state(DesktopWindow::Editor, false);
                return;
            }
            if ctx.input(|i| i.key_pressed(Key::N) && i.modifiers.command) {
                self.run_editor_command(EditorCommand::NewDocument);
            }
            let palette = current_palette();

            // Editor fills remaining space (status bar drawn globally)
            egui::CentralPanel::default()
                .frame(
                    egui::Frame::none()
                        .fill(palette.bg)
                        .inner_margin(egui::Margin::same(4.0)),
                )
                .show(ctx, |ui| {
                    // Header: title + hints
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(&title).color(palette.fg).strong());
                        ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(
                                RichText::new("Esc:Back  ^S:Save  ^N:New  ^F:Find")
                                    .color(palette.dim)
                                    .small(),
                            );
                        });
                    });
                    if let Some(path) = &self.editor.path {
                        ui.label(
                            RichText::new(path.display().to_string())
                                .color(palette.dim)
                                .small(),
                        );
                    }
                    if !self.editor.status.is_empty() {
                        ui.label(
                            RichText::new(&self.editor.status)
                                .color(palette.dim)
                                .small(),
                        );
                    }

                    // Block cursor in theme color
                    let char_width = 16.0 * 0.6;
                    ui.visuals_mut().text_cursor.stroke = egui::Stroke::new(char_width, palette.fg);
                    let edit = TextEdit::multiline(&mut self.editor.text)
                        .lock_focus(true)
                        .frame(false)
                        .font(egui::TextStyle::Monospace);
                    let response = ui.add_sized(ui.available_size(), edit);
                    if response.changed() {
                        self.editor.dirty = true;
                    }
                });
            return;
        }

        let mut open = self.editor.open;
        let mut header_action = DesktopHeaderAction::None;
        let (window, maximized) = self.build_resizable_desktop_window(
            ctx,
            DesktopWindow::Editor,
            &title,
            &mut open,
            ResizableDesktopWindowOptions {
                min_size: egui::vec2(400.0, 300.0),
                default_size: Self::desktop_default_window_size(DesktopWindow::Editor),
                default_pos: None,
                clamp_restore: false,
            },
        );
        let generation = self.desktop_window_generation(DesktopWindow::Editor);
        let text_edit_id = Id::new(("editor_text_edit", generation));
        let shown = window.show(ctx, |ui| {
            // ── HEADER ───────────────────────────────────────────────────────
            header_action = Self::draw_desktop_window_header(ui, &title, maximized);
            if let Some(path) = &self.editor.path {
                ui.small(path.display().to_string());
            }
            if !self.editor.status.is_empty() {
                ui.small(self.editor.status.clone());
            }

            // ── FIND/REPLACE BAR ─────────────────────────────────────────────
            if self.editor.ui.find_open {
                let palette = current_palette();
                egui::Frame::none()
                    .fill(palette.panel)
                    .inner_margin(egui::Margin::symmetric(4.0, 4.0))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("Find:").color(palette.dim));
                            ui.add_space(4.0);
                            let find_resp = ui.add(
                                TextEdit::singleline(&mut self.editor.ui.find_query)
                                    .desired_width(180.0)
                                    .hint_text("search text"),
                            );
                            if find_resp.lost_focus()
                                && ctx.input(|i| i.key_pressed(egui::Key::Enter))
                            {
                                self.editor_find_next(ctx, text_edit_id);
                            }
                            if ui.button("Find Next").clicked() {
                                self.editor_find_next(ctx, text_edit_id);
                            }
                            if self.editor.ui.find_replace_visible {
                                ui.separator();
                                ui.label(RichText::new("Replace:").color(palette.dim));
                                ui.add(
                                    TextEdit::singleline(&mut self.editor.ui.replace_query)
                                        .desired_width(180.0)
                                        .hint_text("replacement"),
                                );
                                if ui.button("Replace").clicked() {
                                    self.editor_replace_one(ctx, text_edit_id);
                                }
                                if ui.button("Replace All").clicked() {
                                    self.editor_replace_all();
                                }
                            }
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if ui.button("[X]").clicked() {
                                        self.run_editor_command(EditorCommand::CloseFind);
                                    }
                                },
                            );
                        });
                    });
            }

            // ── TEXT EDITOR AREA ─────────────────────────────────────────────
            let palette = current_palette();
            let char_width = self.editor.font_size * 0.6;
            ui.visuals_mut().text_cursor.stroke = egui::Stroke::new(char_width, palette.fg);
            if (self.editor.font_size - 16.0).abs() > 0.1 {
                ui.style_mut().text_styles.insert(
                    egui::TextStyle::Monospace,
                    egui::FontId::new(self.editor.font_size, egui::FontFamily::Monospace),
                );
            }
            let text_align = match self.editor.ui.text_align {
                EditorTextAlign::Center => egui::Align::Center,
                EditorTextAlign::Right => egui::Align::RIGHT,
                EditorTextAlign::Left => egui::Align::LEFT,
            };

            // Fill all remaining space with the TextEdit.
            let remaining = ui.available_size();
            let mut edit = TextEdit::multiline(&mut self.editor.text)
                .id(text_edit_id)
                .lock_focus(true)
                .frame(false)
                .font(egui::TextStyle::Monospace)
                .horizontal_align(text_align);
            if !self.editor.word_wrap {
                edit = edit.desired_width(f32::INFINITY);
            }
            let response = ui.add_sized(remaining, edit);
            Self::attach_generic_context_menu(&mut self.context_menu_action, &response);
            if response.changed() {
                self.editor.dirty = true;
            }
        });
        let shown_rect = shown.as_ref().map(|inner| inner.response.rect);
        let shown_contains_pointer = shown
            .as_ref()
            .is_some_and(|inner| inner.response.contains_pointer());
        self.finish_desktop_window_host(
            ctx,
            DesktopWindow::Editor,
            &mut open,
            maximized,
            shown_rect,
            shown_contains_pointer,
            DesktopWindowRectTracking::FullRect,
            header_action,
        );
    }

    fn draw_settings(&mut self, ctx: &Context) {
        if !self.settings.open || self.desktop_window_is_minimized(DesktopWindow::Settings) {
            return;
        }
        let mut open = self.settings.open;
        let maximized = self.desktop_window_is_maximized(DesktopWindow::Settings);
        let restore = self.take_desktop_window_restore_dims(DesktopWindow::Settings);
        let mut header_action = DesktopHeaderAction::None;
        let generation = self.desktop_window_generation(DesktopWindow::Settings);
        let default_size = Self::desktop_default_window_size(DesktopWindow::Settings);
        let default_pos = Self::desktop_default_window_pos(ctx, default_size);
        let mut window = egui::Window::new("Settings")
            .id(Id::new(("native_settings", generation)))
            .open(&mut open)
            .title_bar(false)
            .frame(Self::desktop_window_frame())
            .resizable(false)
            .default_pos(default_pos)
            .fixed_size(default_size);
        if maximized {
            let rect = Self::desktop_workspace_rect(ctx);
            window = window
                .movable(false)
                .fixed_pos(rect.min)
                .fixed_size(rect.size());
        } else if let Some((pos, _size)) = restore {
            // Restore pos after un-maximize, but keep fixed content size.
            let pos = Self::desktop_clamp_window_pos(ctx, pos, default_size);
            window = window.current_pos(pos);
        }
        let mut close_requested = false;
        let shown = window.show(ctx, |ui| {
            Self::apply_settings_control_style(ui);
            header_action = Self::draw_desktop_window_header(ui, "Settings", maximized);
            let is_admin = self.session.as_ref().is_some_and(|s| s.is_admin);
            let panel = self.settings.panel;
            let mut changed = false;
            let mut window_mode_changed = false;
            let mut next_panel = None;

            let panel_title = settings_panel_title(panel);

            ui.add_space(4.0);
            if matches!(panel, NativeSettingsPanel::Home) {
                ui.label(RichText::new("Settings").strong().size(28.0));
                ui.add_space(14.0);
            } else {
                ui.horizontal(|ui| {
                    if ui.button("Back").clicked() {
                        next_panel = Some(desktop_settings_back_target(panel));
                    }
                    ui.strong(panel_title);
                });
                ui.separator();
                ui.add_space(4.0);
            }

            match panel {
                NativeSettingsPanel::Home => {
                    let rows = self.settings_home_rows_for_session(is_admin);
                    let tile_w = 140.0;
                    let tile_h = 112.0;
                    let gap_x = 34.0;
                    let row_gap = 24.0;
                    let icon_font_size = 22.0;
                    let label_font_size = 22.0;

                    ui.add_space(6.0);

                    for (row_idx, row) in rows.iter().enumerate() {
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = gap_x;
                            for tile in row {
                                let panel_texture = match tile.action {
                                    SettingsHomeTileAction::OpenPanel(panel) => {
                                        self.settings_panel_texture(ctx, panel)
                                    }
                                    SettingsHomeTileAction::CloseWindow => None,
                                };
                                let response = Self::retro_settings_tile(
                                    ui,
                                    panel_texture.as_ref(),
                                    tile.icon,
                                    tile.label,
                                    tile.enabled,
                                    egui::vec2(tile_w, tile_h),
                                    icon_font_size,
                                    label_font_size,
                                );
                                if response.clicked() {
                                    match tile.action {
                                        SettingsHomeTileAction::CloseWindow => {
                                            close_requested = true;
                                        }
                                        SettingsHomeTileAction::OpenPanel(panel) => {
                                            next_panel = Some(panel);
                                        }
                                    }
                                }
                            }
                            for _ in row.len()..4 {
                                ui.add_space(tile_w);
                            }
                        });
                        ui.add_space(if row_idx == rows.len() - 1 {
                            0.0
                        } else {
                            row_gap
                        });
                    }
                    if !is_admin {
                        ui.small("User Management requires an admin session.");
                    }
                }
                _ => {
                    let body_max_height = ui.available_height().max(120.0);
                    egui::ScrollArea::vertical()
                        .auto_shrink([false, false])
                        .max_height(body_max_height)
                        .show(ui, |ui| match panel {
                            NativeSettingsPanel::General => {
                                Self::settings_two_columns(ui, |left, right| {
                                    Self::settings_section(left, "Startup", |left| {
                                        left.label("Default Open Mode");
                                        left.horizontal(|ui| {
                                            if Self::retro_choice_button(
                                                ui,
                                                "Terminal",
                                                self.settings.draft.default_open_mode
                                                    == OpenMode::Terminal,
                                            )
                                            .clicked()
                                                && self.settings.draft.default_open_mode
                                                    != OpenMode::Terminal
                                            {
                                                self.settings.draft.default_open_mode =
                                                    OpenMode::Terminal;
                                                changed = true;
                                            }
                                            if Self::retro_choice_button(
                                                ui,
                                                "Desktop",
                                                self.settings.draft.default_open_mode
                                                    == OpenMode::Desktop,
                                            )
                                            .clicked()
                                                && self.settings.draft.default_open_mode
                                                    != OpenMode::Desktop
                                            {
                                                self.settings.draft.default_open_mode =
                                                    OpenMode::Desktop;
                                                changed = true;
                                            }
                                        });
                                        left.add_space(8.0);
                                        left.small(
                                            "Choose which interface opens first after login.",
                                        );
                                    });

                                    Self::settings_section(right, "Options", |right| {
                                        let palette = current_palette();
                                        if Self::retro_checkbox_row(
                                            right,
                                            &mut self.settings.draft.sound,
                                            "Enable sound",
                                        )
                                        .clicked()
                                        {
                                            changed = true;
                                        }
                                        right.add_space(8.0);
                                        right.label("System sound volume");
                                        right.visuals_mut().selection.bg_fill = palette.fg;
                                        right.visuals_mut().widgets.inactive.bg_fill = palette.dim;
                                        if right
                                            .add(
                                                egui::Slider::new(
                                                    &mut self.settings.draft.system_sound_volume,
                                                    0..=100,
                                                )
                                                .suffix("%"),
                                            )
                                            .changed()
                                        {
                                            changed = true;
                                        }
                                        if Self::retro_checkbox_row(
                                            right,
                                            &mut self.settings.draft.bootup,
                                            "Play bootup on login",
                                        )
                                        .clicked()
                                        {
                                            changed = true;
                                        }
                                        if Self::retro_checkbox_row(
                                            right,
                                            &mut self.settings.draft.show_navigation_hints,
                                            "Show navigation hints",
                                        )
                                        .clicked()
                                        {
                                            changed = true;
                                        }
                                    });
                                });
                            }
                            NativeSettingsPanel::Appearance => {
                                let palette = current_palette();
                                // ── Tab bar ───────────────────────────────────────────────────
                                let tabs = ["Background", "Colors", "Icons", "Terminal"];
                                ui.horizontal(|ui| {
                                    for (i, label) in tabs.iter().enumerate() {
                                        let active = self.appearance_tab == i as u8;
                                        let color = if active { palette.fg } else { palette.dim };
                                        let btn = ui.add(
                                            egui::Button::new(
                                                RichText::new(*label).color(color).strong(),
                                            )
                                            .stroke(egui::Stroke::new(
                                                if active { 2.0 } else { 1.0 },
                                                color,
                                            ))
                                            .fill(if active { palette.panel } else { palette.bg }),
                                        );
                                        if btn.clicked() {
                                            self.appearance_tab = i as u8;
                                        }
                                    }
                                });
                                ui.add_space(10.0);
                                Self::retro_separator(ui);
                                ui.add_space(8.0);
                                // ── Tab content ───────────────────────────────────────────────
                                match self.appearance_tab {
                                    // ── Background ─────────────────────────────────────────────
                                    0 => {
                                        Self::settings_section(ui, "Window", |ui| {
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
                                                        self.settings.draft
                                                            .native_startup_window_mode
                                                            == mode,
                                                    )
                                                    .clicked()
                                                        && self.settings.draft
                                                            .native_startup_window_mode
                                                            != mode
                                                    {
                                                        self.settings.draft
                                                            .native_startup_window_mode = mode;
                                                        changed = true;
                                                        window_mode_changed = true;
                                                    }
                                                }
                                            });
                                            ui.add_space(8.0);
                                            ui.small(
                                                "Applies immediately and persists across launches. Windowed is the safest mode on older GPUs.",
                                            );
                                        });
                                        ui.add_space(10.0);
                                        Self::settings_section(ui, "Wallpaper", |ui| {
                                            ui.label("Wallpaper Path");
                                            ui.horizontal(|ui| {
                                                let w = Self::responsive_input_width(
                                                    ui, 0.72, 160.0, 400.0,
                                                );
                                                if ui
                                                    .add(
                                                        TextEdit::singleline(
                                                            &mut self
                                                                .settings
                                                                .draft
                                                                .desktop_wallpaper,
                                                        )
                                                        .desired_width(w)
                                                        .hint_text("/path/to/image.png"),
                                                    )
                                                    .changed()
                                                {
                                                    changed = true;
                                                }
                                                if ui.button("Browse…").clicked() {
                                                    let start = wallpaper_browser_start_dir();
                                                    self.picking_wallpaper = true;
                                                    self.open_embedded_file_manager_at(start);
                                                }
                                            });
                                            ui.add_space(8.0);
                                            ui.horizontal(|ui| {
                                                ui.label("Wallpaper Mode");
                                                let selected = match self
                                                    .settings
                                                    .draft
                                                    .desktop_wallpaper_size_mode
                                                {
                                                    WallpaperSizeMode::DefaultSize => {
                                                        "Default Size"
                                                    }
                                                    WallpaperSizeMode::FitToScreen => {
                                                        "Fit To Screen"
                                                    }
                                                    WallpaperSizeMode::Centered => "Centered",
                                                    WallpaperSizeMode::Tile => "Tile",
                                                    WallpaperSizeMode::Stretch => "Stretch",
                                                };
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
                                                            changed = true;
                                                            ui.close_menu();
                                                        }
                                                    }
                                                });
                                            });
                                        });
                                    }
                                    // ── Colors ─────────────────────────────────────────────────
                                    1 => {
                                        Self::settings_section(ui, "Theme Color", |ui| {
                                            ui.horizontal(|ui| {
                                                ui.label("Theme");
                                                let mut current_idx = THEMES
                                                    .iter()
                                                    .position(|(name, _)| {
                                                        *name == self.settings.draft.theme
                                                    })
                                                    .unwrap_or(0);
                                                egui::ComboBox::from_id_salt(
                                                    "native_settings_theme",
                                                )
                                                .selected_text(
                                                    RichText::new(THEMES[current_idx].0)
                                                        .color(palette.fg),
                                                )
                                                .show_ui(ui, |ui| {
                                                    Self::apply_settings_control_style(ui);
                                                    for (idx, (name, _)) in
                                                        THEMES.iter().enumerate()
                                                    {
                                                        if Self::retro_choice_button(
                                                            ui,
                                                            *name,
                                                            current_idx == idx,
                                                        )
                                                        .clicked()
                                                        {
                                                            current_idx = idx;
                                                            self.settings.draft.theme =
                                                                (*name).to_string();
                                                            changed = true;
                                                            ui.close_menu();
                                                        }
                                                    }
                                                });
                                            });
                                            if self.settings.draft.theme == CUSTOM_THEME_NAME {
                                                let mut rgb = self.settings.draft.custom_theme_rgb;
                                                let preview_color =
                                                    egui::Color32::from_rgb(rgb[0], rgb[1], rgb[2]);
                                                // Make slider rails visible: track in custom color,
                                                // unfilled portion in dim. Without this, the rail
                                                // is BLACK-on-BLACK (invisible) due to settings style.
                                                ui.visuals_mut().selection.bg_fill = preview_color;
                                                ui.visuals_mut().widgets.inactive.bg_fill =
                                                    palette.dim;
                                                changed |= ui
                                                    .add(
                                                        egui::Slider::new(&mut rgb[0], 0..=255)
                                                            .text("Red"),
                                                    )
                                                    .changed();
                                                changed |= ui
                                                    .add(
                                                        egui::Slider::new(&mut rgb[1], 0..=255)
                                                            .text("Green"),
                                                    )
                                                    .changed();
                                                changed |= ui
                                                    .add(
                                                        egui::Slider::new(&mut rgb[2], 0..=255)
                                                            .text("Blue"),
                                                    )
                                                    .changed();
                                                if rgb != self.settings.draft.custom_theme_rgb {
                                                    self.settings.draft.custom_theme_rgb = rgb;
                                                }
                                            }
                                        });
                                    }
                                    // ── Icons ──────────────────────────────────────────────────
                                    2 => {
                                        Self::settings_section(ui, "Desktop Icons", |ui| {
                                            ui.horizontal(|ui| {
                                                ui.label("Icon Style");
                                                let selected =
                                                    match self.settings.draft.desktop_icon_style {
                                                        DesktopIconStyle::Dos => "DOS",
                                                        DesktopIconStyle::Win95 => "Win95",
                                                        DesktopIconStyle::Minimal => "Minimal",
                                                        DesktopIconStyle::NoIcons => "No Icons",
                                                    };
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
                                                            changed = true;
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
                                                    changed = true;
                                                }
                                            }
                                            ui.add_space(8.0);
                                            if Self::retro_checkbox_row(
                                                ui,
                                                &mut self.settings.draft.desktop_show_cursor,
                                                "Show desktop cursor",
                                            )
                                            .clicked()
                                            {
                                                changed = true;
                                            }
                                        });
                                    }
                                    // ── Terminal ───────────────────────────────────────────────
                                    _ => {
                                        Self::settings_section(ui, "PTY Display", |ui| {
                                            if Self::retro_checkbox_row(
                                                ui,
                                                &mut self.settings.draft.cli_styled_render,
                                                "Styled PTY rendering",
                                            )
                                            .clicked()
                                            {
                                                changed = true;
                                            }
                                            ui.add_space(8.0);
                                            ui.horizontal(|ui| {
                                                ui.label("PTY Color Mode");
                                                let selected =
                                                    match self.settings.draft.cli_color_mode {
                                                        CliColorMode::ThemeLock => "Theme Lock",
                                                        CliColorMode::PaletteMap => "Palette-map",
                                                        CliColorMode::Color => "Color",
                                                        CliColorMode::Monochrome => "Monochrome",
                                                    };
                                                egui::ComboBox::from_id_salt(
                                                    "native_settings_cli_color",
                                                )
                                                .selected_text(
                                                    RichText::new(selected).color(palette.fg),
                                                )
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
                                                            self.settings.draft.cli_color_mode
                                                                == mode,
                                                        )
                                                        .clicked()
                                                            && self.settings.draft.cli_color_mode
                                                                != mode
                                                        {
                                                            self.settings.draft.cli_color_mode =
                                                                mode;
                                                            changed = true;
                                                            ui.close_menu();
                                                        }
                                                    }
                                                });
                                            });
                                            ui.add_space(8.0);
                                            if ui
                                                .button(match self.settings.draft.cli_acs_mode {
                                                    CliAcsMode::Ascii => "Border Glyphs: ASCII",
                                                    CliAcsMode::Unicode => {
                                                        "Border Glyphs: Unicode Smooth"
                                                    }
                                                })
                                                .clicked()
                                            {
                                                self.settings.draft.cli_acs_mode =
                                                    match self.settings.draft.cli_acs_mode {
                                                        CliAcsMode::Ascii => CliAcsMode::Unicode,
                                                        CliAcsMode::Unicode => CliAcsMode::Ascii,
                                                    };
                                                changed = true;
                                            }
                                        });
                                    }
                                }
                            }
                            NativeSettingsPanel::DefaultApps => {
                                changed |= self.draw_settings_default_apps_panel(ui);
                            }
                            NativeSettingsPanel::Connections => {
                                ui.vertical(|ui| {
                                    for item in desktop_settings_connections_nav_items() {
                                        if Self::retro_full_width_button(ui, item.label).clicked() {
                                            next_panel = Some(item.panel);
                                        }
                                    }
                                });
                            }
                            NativeSettingsPanel::ConnectionsNetwork => {
                                self.draw_settings_connections_kind_panel(
                                    ui,
                                    ConnectionKind::Network,
                                );
                            }
                            NativeSettingsPanel::ConnectionsBluetooth => {
                                self.draw_settings_connections_kind_panel(
                                    ui,
                                    ConnectionKind::Bluetooth,
                                );
                            }
                            NativeSettingsPanel::CliProfiles => {
                                changed |= self.draw_settings_cli_profiles_panel(ui);
                            }
                            NativeSettingsPanel::EditMenus => {
                                changed |= self.draw_settings_edit_menus_panel(ui);
                            }
                            NativeSettingsPanel::UserManagement => {
                                if is_admin {
                                    ui.vertical(|ui| {
                                        for item in desktop_settings_user_management_nav_items() {
                                            if Self::retro_full_width_button(ui, item.label)
                                                .clicked()
                                            {
                                                next_panel = Some(item.panel);
                                            }
                                        }
                                    });
                                } else {
                                    ui.small("User Management requires an admin session.");
                                }
                            }
                            NativeSettingsPanel::UserManagementViewUsers => {
                                if is_admin {
                                    self.draw_settings_user_view_panel(ui);
                                } else {
                                    ui.small("User Management requires an admin session.");
                                }
                            }
                            NativeSettingsPanel::UserManagementCreateUser => {
                                if is_admin {
                                    self.draw_settings_user_create_panel(ui);
                                } else {
                                    ui.small("User Management requires an admin session.");
                                }
                            }
                            NativeSettingsPanel::UserManagementEditUsers => {
                                if is_admin {
                                    self.draw_settings_user_edit_panel(ui, false);
                                } else {
                                    ui.small("User Management requires an admin session.");
                                }
                            }
                            NativeSettingsPanel::UserManagementEditCurrentUser => {
                                if is_admin {
                                    self.draw_settings_user_edit_panel(ui, true);
                                } else {
                                    ui.small("User Management requires an admin session.");
                                }
                            }
                            NativeSettingsPanel::About => {
                                ui.label(format!("Version: v{}", env!("CARGO_PKG_VERSION")));
                                ui.label(format!("Theme: {}", self.settings.draft.theme));
                                ui.label(format!(
                                    "Default Open Mode: {}",
                                    match self.settings.draft.default_open_mode {
                                        OpenMode::Terminal => "Terminal",
                                        OpenMode::Desktop => "Desktop",
                                    }
                                ));
                                ui.label(format!(
                                    "Window Mode: {}",
                                    self.settings.draft.native_startup_window_mode.label()
                                ));
                            }
                            NativeSettingsPanel::Home => {}
                        });
                }
            }

            if let Some(panel) = next_panel {
                self.settings.panel = panel;
                self.apply_status_update(clear_settings_status());
            }
            ui.separator();
            if changed {
                let settings = persist_settings_draft(&self.settings.draft);
                self.replace_settings_draft(settings);
                if window_mode_changed {
                    self.apply_native_window_mode(ctx);
                }
                self.apply_status_update(saved_settings_status());
            }
            if !self.settings.status.is_empty() {
                ui.small(&self.settings.status);
            }
        });
        if close_requested {
            open = false;
        }
        let shown_rect = shown.as_ref().map(|inner| inner.response.rect);
        let shown_contains_pointer = shown
            .as_ref()
            .is_some_and(|inner| inner.response.contains_pointer());
        self.finish_desktop_window_host(
            ctx,
            DesktopWindow::Settings,
            &mut open,
            maximized,
            shown_rect,
            shown_contains_pointer,
            DesktopWindowRectTracking::PositionOnly,
            header_action,
        );
    }

    fn draw_settings_default_apps_panel(&mut self, ui: &mut egui::Ui) -> bool {
        let mut changed = false;
        egui::ScrollArea::vertical().show(ui, |ui| {
            for slot in [DefaultAppSlot::TextCode, DefaultAppSlot::Ebook] {
                let current_label = binding_label_for_slot(&self.settings.draft, slot);
                let custom_buffer = match slot {
                    DefaultAppSlot::TextCode => &mut self.settings.default_app_custom_text_code,
                    DefaultAppSlot::Ebook => &mut self.settings.default_app_custom_ebook,
                };

                ui.group(|ui| {
                    Self::settings_two_columns(ui, |left, right| {
                        Self::settings_section(
                            left,
                            &format!("Default App For {}", default_app_slot_label(slot)),
                            |left| {
                                left.label(format!("Currently selected: {current_label}"));
                                left.small(default_app_slot_description(slot));
                            },
                        );

                        Self::settings_section(right, "Selection", |right| {
                            let field_width =
                                Self::responsive_input_width(right, 0.85, 220.0, 620.0);
                            right.horizontal(|ui| {
                                ui.label("Chooser");
                                egui::ComboBox::from_id_salt(format!(
                                    "native_default_app_slot_{slot:?}"
                                ))
                                .selected_text(
                                    RichText::new(current_label.clone())
                                        .color(current_palette().fg),
                                )
                                .show_ui(ui, |ui| {
                                    Self::apply_settings_control_style(ui);
                                    for choice in build_default_app_settings_choices(
                                        &self.settings.draft,
                                        slot,
                                    ) {
                                        if Self::retro_choice_button(
                                            ui,
                                            choice.label,
                                            choice.selected,
                                        )
                                        .clicked()
                                        {
                                            apply_default_app_binding(
                                                &mut self.settings.draft,
                                                slot,
                                                choice.binding,
                                            );
                                            changed = true;
                                            ui.close_menu();
                                        }
                                    }
                                });
                            });
                            right.add_space(6.0);
                            right.label("Custom Command");
                            right.add(
                                TextEdit::singleline(custom_buffer)
                                    .desired_width(field_width)
                                    .hint_text("epy"),
                            );
                            if Self::retro_full_width_button(right, "Apply Custom Command")
                                .clicked()
                            {
                                match resolve_custom_default_app_binding(custom_buffer.trim()) {
                                    Ok(binding) => {
                                        apply_default_app_binding(
                                            &mut self.settings.draft,
                                            slot,
                                            binding,
                                        );
                                        changed = true;
                                    }
                                    Err(_) => {
                                        self.settings.status =
                                            "Error: invalid command line".to_string();
                                    }
                                }
                            }
                        });
                    });
                });
                ui.add_space(10.0);
            }
        });
        changed
    }

    fn draw_settings_connections_kind_panel(&mut self, ui: &mut egui::Ui, kind: ConnectionKind) {
        if connections_macos_disabled() {
            ui.small(connections_macos_disabled_hint());
            return;
        }

        let saved_connections = self.saved_connections_cached(kind);

        let (scan_label, saved_title, discovered_title, scanned_items) = match kind {
            ConnectionKind::Network => (
                "Scan Networks",
                "Saved Networks",
                "Discovered Networks",
                &mut self.settings.scanned_networks,
            ),
            ConnectionKind::Bluetooth => (
                "Scan Bluetooth",
                "Saved Bluetooth",
                "Discovered Bluetooth",
                &mut self.settings.scanned_bluetooth,
            ),
        };

        if Self::retro_full_width_button(ui, scan_label).clicked() {
            let (discovered, status) = scan_discovered_connections(kind);
            *scanned_items = discovered;
            self.settings.status = status;
        }
        if matches!(kind, ConnectionKind::Network) {
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.label("Network Password");
                let field_width = Self::responsive_input_width(ui, 0.65, 220.0, 520.0);
                ui.add(
                    TextEdit::singleline(&mut self.settings.connection_password)
                        .desired_width(field_width)
                        .password(true),
                );
            });
            ui.small("Used only when connecting to secured networks.");
        }
        ui.add_space(8.0);
        let mut pending_settings: Option<Settings> = None;
        let mut pending_status: Option<String> = None;
        Self::settings_two_columns(ui, |left, right| {
            Self::settings_section(left, saved_title, |left| {
                if saved_connections.is_empty() {
                    left.small("No saved items.");
                } else {
                    egui::ScrollArea::vertical()
                        .max_height((left.available_height() * 0.85).clamp(180.0, 420.0))
                        .show(left, |ui| {
                            for entry in saved_connections.iter() {
                                ui.horizontal(|ui| {
                                    ui.label(saved_connection_label(entry));
                                    if ui.button("Forget").clicked() {
                                        if let Some((settings, status)) =
                                            forget_saved_connection_and_refresh_settings(
                                                kind,
                                                &entry.name,
                                            )
                                        {
                                            pending_settings = Some(settings);
                                            pending_status = Some(status);
                                        }
                                    }
                                });
                            }
                        });
                }
            });

            Self::settings_section(right, discovered_title, |right| {
                if scanned_items.is_empty() {
                    right.small("Run a scan to populate this list.");
                } else {
                    egui::ScrollArea::vertical()
                        .max_height((right.available_height() * 0.85).clamp(180.0, 420.0))
                        .show(right, |ui| {
                            for entry in scanned_items.clone() {
                                ui.horizontal(|ui| {
                                    ui.label(discovered_connection_label(&entry));
                                    if ui.button("Connect").clicked() {
                                        let password = if matches!(kind, ConnectionKind::Network)
                                            && connection_requires_password(&entry.detail)
                                            && !self.settings.connection_password.trim().is_empty()
                                        {
                                            Some(self.settings.connection_password.clone())
                                        } else {
                                            None
                                        };
                                        match connect_connection_and_refresh_settings(
                                            kind,
                                            &entry,
                                            password.as_deref(),
                                        ) {
                                            Ok((settings, status)) => {
                                                pending_settings = Some(settings);
                                                pending_status = Some(status);
                                            }
                                            Err(err) => {
                                                self.settings.status =
                                                    format!("Connect failed: {err}");
                                            }
                                        }
                                    }
                                });
                            }
                        });
                }
            });
        });
        if let Some(settings) = pending_settings {
            self.replace_settings_draft(settings);
        }
        if let Some(status) = pending_status {
            self.settings.status = status;
        }
    }

    fn draw_settings_cli_profiles_panel(&mut self, ui: &mut egui::Ui) -> bool {
        let mut changed = false;
        let custom_profile_count = self.settings.draft.desktop_cli_profiles.custom.len();
        let profile = gui_cli_profile_mut(
            &mut self.settings.draft.desktop_cli_profiles,
            self.settings.cli_profile_slot,
        );
        let mut min_w = profile.min_w;
        let mut min_h = profile.min_h;
        Self::settings_two_columns(ui, |left, right| {
            Self::settings_section(left, "Profile", |left| {
                left.horizontal(|ui| {
                    ui.label("Profile");
                    egui::ComboBox::from_id_salt("native_settings_cli_profile_slot")
                        .selected_text(
                            RichText::new(gui_cli_profile_slot_label(
                                self.settings.cli_profile_slot,
                            ))
                            .color(current_palette().fg),
                        )
                        .show_ui(ui, |ui| {
                            Self::apply_settings_control_style(ui);
                            for slot in gui_cli_profile_slots() {
                                if Self::retro_choice_button(
                                    ui,
                                    gui_cli_profile_slot_label(slot),
                                    self.settings.cli_profile_slot == slot,
                                )
                                .clicked()
                                {
                                    self.settings.cli_profile_slot = slot;
                                    ui.close_menu();
                                }
                            }
                        });
                });
                left.add_space(8.0);
                changed |= left
                    .add(
                        egui::DragValue::new(&mut min_w)
                            .range(20..=240)
                            .prefix("Min W "),
                    )
                    .changed();
                changed |= left
                    .add(
                        egui::DragValue::new(&mut min_h)
                            .range(10..=120)
                            .prefix("Min H "),
                    )
                    .changed();

                let mut use_pref_w = profile.preferred_w.is_some();
                if Self::retro_checkbox_row(left, &mut use_pref_w, "Use Preferred Width").clicked()
                {
                    profile.preferred_w = if use_pref_w {
                        Some(profile.min_w)
                    } else {
                        None
                    };
                    changed = true;
                }
                if let Some(preferred) = profile.preferred_w.as_mut() {
                    changed |= left
                        .add(
                            egui::DragValue::new(preferred)
                                .range(profile.min_w..=280)
                                .prefix("Preferred W "),
                        )
                        .changed();
                }
            });

            Self::settings_section(right, "Behavior", |right| {
                let mut use_pref_h = profile.preferred_h.is_some();
                if Self::retro_checkbox_row(right, &mut use_pref_h, "Use Preferred Height")
                    .clicked()
                {
                    profile.preferred_h = if use_pref_h {
                        Some(profile.min_h)
                    } else {
                        None
                    };
                    changed = true;
                }
                if let Some(preferred) = profile.preferred_h.as_mut() {
                    changed |= right
                        .add(
                            egui::DragValue::new(preferred)
                                .range(profile.min_h..=140)
                                .prefix("Preferred H "),
                        )
                        .changed();
                }
                if Self::retro_checkbox_row(
                    right,
                    &mut profile.mouse_passthrough,
                    "Mouse passthrough",
                )
                .clicked()
                {
                    changed = true;
                }
                if Self::retro_checkbox_row(right, &mut profile.open_fullscreen, "Open fullscreen")
                    .clicked()
                {
                    changed = true;
                }
                if Self::retro_checkbox_row(right, &mut profile.live_resize, "Live resize")
                    .clicked()
                {
                    changed = true;
                }
                right.add_space(8.0);
                right.small(format!(
                    "Custom profiles currently stored: {}",
                    custom_profile_count
                ));
            });
        });
        if min_w != profile.min_w {
            profile.min_w = min_w;
            if let Some(preferred) = profile.preferred_w.as_mut() {
                *preferred = (*preferred).max(profile.min_w);
            }
        }
        if min_h != profile.min_h {
            profile.min_h = min_h;
            if let Some(preferred) = profile.preferred_h.as_mut() {
                *preferred = (*preferred).max(profile.min_h);
            }
        }
        changed
    }

    fn draw_settings_edit_menus_panel(&mut self, ui: &mut egui::Ui) -> bool {
        let mut changed = false;
        ui.horizontal(|ui| {
            ui.label("Menu");
            egui::ComboBox::from_id_salt("native_settings_edit_target")
                .selected_text(
                    RichText::new(self.settings.edit_target.title()).color(current_palette().fg),
                )
                .show_ui(ui, |ui| {
                    Self::apply_settings_control_style(ui);
                    for target in [
                        EditMenuTarget::Applications,
                        EditMenuTarget::Documents,
                        EditMenuTarget::Network,
                        EditMenuTarget::Games,
                    ] {
                        if Self::retro_choice_button(
                            ui,
                            target.title(),
                            self.settings.edit_target == target,
                        )
                        .clicked()
                        {
                            self.settings.edit_target = target;
                            ui.close_menu();
                        }
                    }
                });
        });
        ui.add_space(8.0);

        Self::settings_two_columns(ui, |left, right| {
            Self::settings_section(left, "Current Entries", |left| {
                if matches!(self.settings.edit_target, EditMenuTarget::Applications) {
                    if Self::retro_checkbox_row(
                        left,
                        &mut self.settings.draft.builtin_menu_visibility.text_editor,
                        &format!("Show {EDITOR_APP_TITLE}"),
                    )
                    .clicked()
                    {
                        changed = true;
                    }
                    if Self::retro_checkbox_row(
                        left,
                        &mut self.settings.draft.builtin_menu_visibility.nuke_codes,
                        "Show Nuke Codes",
                    )
                    .clicked()
                    {
                        changed = true;
                    }
                }
                egui::ScrollArea::vertical()
                    .max_height((left.available_height() * 0.7).clamp(180.0, 380.0))
                    .show(left, |ui| {
                        let entries = self.edit_menu_entries_cached(self.settings.edit_target);
                        for name in entries.iter() {
                            ui.horizontal(|ui| {
                                ui.label(name.as_str());
                                if ui.button("Delete").clicked() {
                                    self.delete_program_entry(self.settings.edit_target, &name);
                                    self.apply_status_update(mirror_shell_to_settings(
                                        &self.shell_status,
                                    ));
                                }
                            });
                        }
                    });
            });

            Self::settings_section(right, "Add Entry", |right| {
                let name_width = Self::responsive_input_width(right, 0.9, 220.0, 420.0);
                let value_width = Self::responsive_input_width(right, 0.95, 320.0, 760.0);
                right.label("Name");
                right.add(
                    TextEdit::singleline(&mut self.settings.edit_name_input)
                        .desired_width(name_width),
                );
                right.add_space(6.0);
                let value_label = if matches!(self.settings.edit_target, EditMenuTarget::Documents)
                {
                    "Folder Path"
                } else {
                    "Command"
                };
                right.label(value_label);
                right.add(
                    TextEdit::singleline(&mut self.settings.edit_value_input)
                        .desired_width(value_width),
                );
                right.add_space(8.0);
                if Self::retro_full_width_button(right, "Add Entry").clicked() {
                    let name = self.settings.edit_name_input.trim().to_string();
                    let value = self.settings.edit_value_input.trim().to_string();
                    if name.is_empty() || value.is_empty() {
                        self.apply_status_update(invalid_input_settings_status());
                    } else {
                        match self.settings.edit_target {
                            EditMenuTarget::Documents => self.add_document_category(name, value),
                            target => self.add_program_entry(target, name, value),
                        }
                        self.apply_status_update(mirror_shell_to_settings(&self.shell_status));
                        if !self
                            .settings
                            .status
                            .to_ascii_lowercase()
                            .starts_with("error")
                        {
                            self.settings.edit_name_input.clear();
                            self.settings.edit_value_input.clear();
                        }
                    }
                }
            });
        });
        changed
    }

    fn draw_settings_user_view_panel(&mut self, ui: &mut egui::Ui) {
        let users = self.sorted_user_records_cached();
        egui::ScrollArea::vertical().show(ui, |ui| {
            for (name, record) in users.iter() {
                ui.label(format!(
                    "{} | auth: {} | admin: {}",
                    name,
                    user_auth_method_label(&record.auth_method),
                    if record.is_admin { "yes" } else { "no" }
                ));
            }
        });
    }

    fn draw_settings_user_create_panel(&mut self, ui: &mut egui::Ui) {
        let users = self.sorted_usernames_cached();
        ui.group(|ui| {
            Self::settings_two_columns(ui, |left, right| {
                let field_width = Self::responsive_input_width(left, 0.85, 180.0, 420.0);
                Self::settings_section(left, "Account", |left| {
                    left.label("Username");
                    left.add(
                        TextEdit::singleline(&mut self.settings.user_create_username)
                            .desired_width(field_width),
                    );
                    left.add_space(6.0);
                    left.label("Authentication");
                    egui::ComboBox::from_id_salt("native_settings_user_create_auth")
                        .selected_text(
                            RichText::new(user_auth_method_label(&self.settings.user_create_auth))
                                .color(current_palette().fg),
                        )
                        .show_ui(left, |ui| {
                            Self::apply_settings_control_style(ui);
                            for auth in [
                                AuthMethod::Password,
                                AuthMethod::NoPassword,
                                AuthMethod::HackingMinigame,
                            ] {
                                if Self::retro_choice_button(
                                    ui,
                                    user_auth_method_label(&auth),
                                    self.settings.user_create_auth == auth,
                                )
                                .clicked()
                                {
                                    self.settings.user_create_auth = auth;
                                    ui.close_menu();
                                }
                            }
                        });
                });

                let pw_width = Self::responsive_input_width(right, 0.85, 180.0, 420.0);
                let create_clicked = Self::settings_section(right, "Credentials", |right| {
                    if matches!(self.settings.user_create_auth, AuthMethod::Password) {
                        right.label("Password");
                        right.add(
                            TextEdit::singleline(&mut self.settings.user_create_password)
                                .desired_width(pw_width)
                                .password(true),
                        );
                        right.add_space(6.0);
                        right.label("Confirm");
                        right.add(
                            TextEdit::singleline(&mut self.settings.user_create_password_confirm)
                                .desired_width(pw_width)
                                .password(true),
                        );
                    } else {
                        right.small("No password fields required for this auth method.");
                    }
                    right.add_space(8.0);
                    Self::retro_full_width_button(right, "Create User").clicked()
                });
                if !create_clicked {
                    return;
                }
                let username = self.settings.user_create_username.trim().to_string();
                if username.is_empty() {
                    self.apply_status_update(settings_status("Username cannot be empty."));
                } else if users.iter().any(|name| name == &username) {
                    self.apply_status_update(settings_status("User already exists."));
                } else {
                    match self.settings.user_create_auth {
                        AuthMethod::Password => {
                            if self.settings.user_create_password.is_empty() {
                                self.apply_status_update(settings_status(
                                    "Password cannot be empty.",
                                ));
                            } else if self.settings.user_create_password
                                != self.settings.user_create_password_confirm
                            {
                                self.apply_status_update(settings_status(
                                    "Passwords do not match.",
                                ));
                            } else {
                                match create_desktop_user(
                                    &username,
                                    AuthMethod::Password,
                                    Some(&self.settings.user_create_password),
                                ) {
                                    Ok(status) => {
                                        self.invalidate_user_cache();
                                        self.apply_status_update(settings_status(status));
                                        self.settings.user_create_username.clear();
                                        self.settings.user_create_password.clear();
                                        self.settings.user_create_password_confirm.clear();
                                        self.settings.user_selected = username;
                                        self.settings.user_selected_loaded_for.clear();
                                    }
                                    Err(status) => {
                                        self.apply_status_update(settings_status(status));
                                    }
                                }
                            }
                        }
                        AuthMethod::NoPassword | AuthMethod::HackingMinigame => {
                            match create_desktop_user(
                                &username,
                                self.settings.user_create_auth.clone(),
                                None,
                            ) {
                                Ok(status) => {
                                    self.invalidate_user_cache();
                                    self.apply_status_update(settings_status(status));
                                    self.settings.user_create_username.clear();
                                    self.settings.user_selected = username;
                                    self.settings.user_selected_loaded_for.clear();
                                }
                                Err(status) => {
                                    self.apply_status_update(settings_status(status));
                                }
                            }
                        }
                    }
                }
            });
        });
    }

    fn draw_settings_user_edit_panel(&mut self, ui: &mut egui::Ui, current_only: bool) {
        let current_username = self.session.as_ref().map(|s| s.username.clone());
        let users = self.sorted_user_records_cached();
        let names: Vec<String> = users.iter().map(|(name, _)| name.clone()).collect();
        if names.is_empty() {
            ui.small("No users found.");
            return;
        }
        if current_only {
            self.settings.user_selected = current_username.clone().unwrap_or_default();
        } else if !names
            .iter()
            .any(|name| name == &self.settings.user_selected)
        {
            self.settings.user_selected = names[0].clone();
        }
        if self.settings.user_selected_loaded_for != self.settings.user_selected {
            if let Some((_, record)) = users
                .iter()
                .find(|(name, _)| name == &self.settings.user_selected)
            {
                self.settings.user_edit_auth = record.auth_method.clone();
                self.settings.user_edit_password.clear();
                self.settings.user_edit_password_confirm.clear();
                self.settings.user_selected_loaded_for = self.settings.user_selected.clone();
            }
        }

        ui.group(|ui| {
            Self::settings_two_columns(ui, |left, right| {
                let field_width = Self::responsive_input_width(left, 0.85, 180.0, 420.0);
                Self::settings_section(
                    left,
                    if current_only {
                        "Edit Current User"
                    } else {
                        "Edit User"
                    },
                    |left| {
                        left.label("User");
                        if current_only {
                            left.label(&self.settings.user_selected);
                        } else {
                            egui::ComboBox::from_id_salt("native_settings_user_selected")
                                .selected_text(
                                    RichText::new(self.settings.user_selected.clone())
                                        .color(current_palette().fg),
                                )
                                .show_ui(left, |ui| {
                                    Self::apply_settings_control_style(ui);
                                    for name in &names {
                                        if Self::retro_choice_button(
                                            ui,
                                            name,
                                            self.settings.user_selected == *name,
                                        )
                                        .clicked()
                                        {
                                            self.settings.user_selected = name.clone();
                                            ui.close_menu();
                                        }
                                    }
                                });
                        }
                        if let Some((_, record)) = users
                            .iter()
                            .find(|(name, _)| name == &self.settings.user_selected)
                        {
                            left.small(format!(
                                "Current auth: {} | Admin: {}",
                                user_auth_method_label(&record.auth_method),
                                if record.is_admin { "yes" } else { "no" }
                            ));
                        }
                        left.add_space(8.0);
                        left.label("New Auth");
                        egui::ComboBox::from_id_salt("native_settings_user_edit_auth")
                            .selected_text(
                                RichText::new(user_auth_method_label(
                                    &self.settings.user_edit_auth,
                                ))
                                .color(current_palette().fg),
                            )
                            .show_ui(left, |ui| {
                                Self::apply_settings_control_style(ui);
                                for auth in [
                                    AuthMethod::Password,
                                    AuthMethod::NoPassword,
                                    AuthMethod::HackingMinigame,
                                ] {
                                    if Self::retro_choice_button(
                                        ui,
                                        user_auth_method_label(&auth),
                                        self.settings.user_edit_auth == auth,
                                    )
                                    .clicked()
                                    {
                                        self.settings.user_edit_auth = auth;
                                        ui.close_menu();
                                    }
                                }
                            });
                    },
                );

                let apply_auth = Self::settings_section(right, "Actions", |right| {
                    if matches!(self.settings.user_edit_auth, AuthMethod::Password) {
                        right.label("Password");
                        right.add(
                            TextEdit::singleline(&mut self.settings.user_edit_password)
                                .desired_width(field_width)
                                .password(true),
                        );
                        right.add_space(6.0);
                        right.label("Confirm");
                        right.add(
                            TextEdit::singleline(&mut self.settings.user_edit_password_confirm)
                                .desired_width(field_width)
                                .password(true),
                        );
                        right.add_space(8.0);
                    }
                    Self::retro_full_width_button(right, "Apply Auth Method").clicked()
                });
                if apply_auth {
                    let username = self.settings.user_selected.clone();
                    match self.settings.user_edit_auth {
                        AuthMethod::Password => {
                            if self.settings.user_edit_password.is_empty() {
                                self.apply_status_update(settings_status(
                                    "Password cannot be empty.",
                                ));
                            } else if self.settings.user_edit_password
                                != self.settings.user_edit_password_confirm
                            {
                                self.apply_status_update(settings_status(
                                    "Passwords do not match.",
                                ));
                            } else {
                                match update_user_auth_method(
                                    &username,
                                    AuthMethod::Password,
                                    Some(&self.settings.user_edit_password),
                                ) {
                                    Ok(status) => {
                                        self.invalidate_user_cache();
                                        self.apply_status_update(settings_status(status));
                                        self.settings.user_edit_password.clear();
                                        self.settings.user_edit_password_confirm.clear();
                                        self.settings.user_selected_loaded_for.clear();
                                    }
                                    Err(status) => {
                                        self.apply_status_update(settings_status(status));
                                    }
                                }
                            }
                        }
                        AuthMethod::NoPassword | AuthMethod::HackingMinigame => {
                            match update_user_auth_method(
                                &username,
                                self.settings.user_edit_auth.clone(),
                                None,
                            ) {
                                Ok(status) => {
                                    self.invalidate_user_cache();
                                    self.apply_status_update(settings_status(status));
                                    self.settings.user_selected_loaded_for.clear();
                                }
                                Err(status) => {
                                    self.apply_status_update(settings_status(status));
                                }
                            }
                        }
                    }
                }

                if Self::retro_full_width_button(right, "Toggle Admin").clicked() {
                    if !current_only {
                        let username = self.settings.user_selected.clone();
                        match toggle_desktop_user_admin(&username) {
                            Ok(status) => {
                                self.invalidate_user_cache();
                                self.apply_status_update(settings_status(status));
                                self.settings.user_selected_loaded_for.clear();
                            }
                            Err(status) => {
                                self.apply_status_update(settings_status(status));
                            }
                        }
                    }
                }
                right.add_space(8.0);

                if !current_only {
                    let can_delete = current_username
                        .as_ref()
                        .is_none_or(|name| name != &self.settings.user_selected);
                    let delete_user = if can_delete {
                        right.button("Delete User")
                    } else {
                        Self::retro_disabled_button(right, "Delete User")
                    };
                    if delete_user.clicked() {
                        if self.settings.user_delete_confirm == self.settings.user_selected {
                            let username = self.settings.user_selected.clone();
                            match delete_desktop_user(&username) {
                                Ok(status) => {
                                    self.invalidate_user_cache();
                                    self.apply_status_update(settings_status(status));
                                    self.settings.user_delete_confirm.clear();
                                    self.settings.user_selected_loaded_for.clear();
                                }
                                Err(status) => {
                                    self.apply_status_update(settings_status(status));
                                }
                            }
                        } else {
                            self.settings.user_delete_confirm = self.settings.user_selected.clone();
                            self.apply_status_update(settings_status(
                                "Click Delete User again to confirm.",
                            ));
                        }
                    }
                    if current_username
                        .as_ref()
                        .is_some_and(|name| name == &self.settings.user_selected)
                    {
                        right.small("You cannot delete the current user.");
                    }
                }
            });
        });
    }

    // ─── Desktop Program Installer ─────────────────────────────────────────────

    fn draw_installer(&mut self, ctx: &Context) {
        if !self.desktop_installer.open
            || self.desktop_window_is_minimized(DesktopWindow::Installer)
        {
            return;
        }
        if self.desktop_installer.search_in_flight() {
            ctx.request_repaint_after(std::time::Duration::from_millis(50));
        }
        let _ = self.desktop_installer.poll_search();
        {
            let state = self.desktop_window_state(DesktopWindow::Installer);
            if state.maximized && (state.restore_pos.is_none() || state.restore_size.is_none()) {
                let state = self.desktop_window_state_mut(DesktopWindow::Installer);
                state.maximized = false;
            }
        }
        let mut open = self.desktop_installer.open;
        let maximized = self.desktop_window_is_maximized(DesktopWindow::Installer);
        let mut header_action = DesktopHeaderAction::None;
        let generation = self.desktop_window_generation(DesktopWindow::Installer);
        let default_size = Self::desktop_default_window_size(DesktopWindow::Installer);
        let default_pos = Self::desktop_default_window_pos(ctx, default_size);
        let workspace_rect = Self::desktop_workspace_rect(ctx);
        let live_pos = {
            let state = self.desktop_window_state(DesktopWindow::Installer);
            state.restore_pos.map(|pos| egui::pos2(pos[0], pos[1]))
        };
        let mut window = egui::Window::new("Program Installer")
            .id(Id::new(("native_installer", generation)))
            .open(&mut open)
            .title_bar(false)
            .frame(Self::desktop_window_frame())
            .resizable(false)
            .min_size([500.0, 400.0])
            .max_size(workspace_rect.size())
            .constrain_to(workspace_rect)
            .default_pos(default_pos)
            .default_size([default_size.x, default_size.y]);
        if maximized {
            window = window
                .movable(false)
                .resizable(false)
                .fixed_pos(workspace_rect.min)
                .fixed_size(workspace_rect.size());
        } else {
            window = window
                .current_pos(live_pos.unwrap_or(default_pos))
                .fixed_size(default_size);
        }

        let palette = current_palette();
        let mut deferred_back = false;
        let mut deferred_search = false;
        let mut deferred_load_installed = false;
        let mut deferred_open_installed_actions: Option<String> = None;
        let mut deferred_open_search_actions: Option<(String, bool)> = None;
        let mut deferred_confirm_setup: Option<(String, InstallerPackageAction)> = None;
        let mut deferred_confirm_yes = false;
        let mut deferred_confirm_no = false;
        let mut deferred_notice_close = false;
        let mut deferred_add_to_menu: Option<(String, InstallerMenuTarget)> = None;
        let mut deferred_open_add_to_menu: Option<String> = None;
        let mut deferred_open_runtime_tools = false;

        let view = self.desktop_installer.view.clone();
        let status = self.desktop_installer.status.clone();
        let has_confirm = self.desktop_installer.confirm_dialog.is_some();
        let notice = self.desktop_installer.notice.clone();
        let tex_apps = self
            .asset_cache
            .as_ref()
            .map(|c| c.icon_applications.clone());
        let tex_tools = self.asset_cache.as_ref().map(|c| c.icon_terminal.clone());
        let tex_network = self
            .asset_cache
            .as_ref()
            .map(|c| c.icon_connections.clone());
        let tex_games = self.installer_games_texture(ctx);

        let shown = window.show(ctx, |ui| {
            Self::apply_installer_widget_style(ui, palette);

            egui::TopBottomPanel::top(Id::new(("inst_top", generation)))
                .frame(egui::Frame::none())
                .show_inside(ui, |ui| {
                    header_action =
                        Self::draw_desktop_window_header(ui, "RobCo Program Installer", maximized);
                });

            egui::TopBottomPanel::bottom(Id::new(("inst_bottom", generation)))
                .frame(egui::Frame::none().inner_margin(egui::Margin::symmetric(8.0, 4.0)))
                .exact_height(28.0)
                .show_inside(ui, |ui| {
                    if !status.is_empty() {
                        ui.label(RichText::new(&status).color(palette.dim));
                    } else {
                        ui.allocate_space(egui::vec2(ui.available_width(), 0.0));
                    }
                });

            if has_confirm {
                egui::TopBottomPanel::bottom(Id::new(("inst_confirm", generation)))
                    .frame(
                        egui::Frame::none()
                            .fill(palette.panel)
                            .stroke(egui::Stroke::new(1.0, palette.fg))
                            .inner_margin(egui::Margin::same(12.0)),
                    )
                    .show_inside(ui, |ui| {
                        if let Some(ref confirm) = self.desktop_installer.confirm_dialog {
                            let action_label = match confirm.action {
                                InstallerPackageAction::Install => "Install",
                                InstallerPackageAction::Update => "Update",
                                InstallerPackageAction::Reinstall => "Reinstall",
                                InstallerPackageAction::Uninstall => "Uninstall",
                            };
                            ui.label(
                                RichText::new(format!("{} {}?", action_label, confirm.pkg))
                                    .color(palette.fg)
                                    .strong(),
                            );
                            ui.add_space(8.0);
                            ui.horizontal(|ui| {
                                if ui
                                    .button(RichText::new("[ Yes ]").color(palette.fg))
                                    .clicked()
                                {
                                    deferred_confirm_yes = true;
                                }
                                ui.add_space(12.0);
                                if ui
                                    .button(RichText::new("[ No ]").color(palette.fg))
                                    .clicked()
                                {
                                    deferred_confirm_no = true;
                                }
                            });
                        }
                    });
            }

            if let Some(notice) = notice.as_ref() {
                egui::Area::new(Id::new(("inst_notice", generation)))
                    .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
                    .order(egui::Order::Foreground)
                    .show(ctx, |ui| {
                        Self::apply_installer_widget_style(ui, palette);
                        egui::Frame::none()
                            .fill(palette.bg)
                            .stroke(egui::Stroke::new(2.0, palette.fg))
                            .inner_margin(egui::Margin::same(14.0))
                            .show(ui, |ui| {
                                ui.set_min_width(360.0);
                                ui.label(
                                    RichText::new(if notice.success {
                                        "Operation Complete"
                                    } else {
                                        "Operation Failed"
                                    })
                                    .color(palette.fg)
                                    .strong()
                                    .heading(),
                                );
                                ui.add_space(8.0);
                                ui.label(RichText::new(&notice.message).color(palette.fg));
                                ui.add_space(12.0);
                                if ui
                                    .button(RichText::new("[ OK ]").color(palette.fg))
                                    .clicked()
                                {
                                    deferred_notice_close = true;
                                }
                            });
                    });
            }

            egui::CentralPanel::default()
                .frame(egui::Frame::none().inner_margin(egui::Margin::same(16.0)))
                .show_inside(ui, |ui| match view {
                    DesktopInstallerView::Home => {
                        Self::draw_installer_home(
                            ui,
                            &mut self.desktop_installer,
                            palette,
                            &mut deferred_search,
                            &mut deferred_load_installed,
                            &mut deferred_open_runtime_tools,
                            [&tex_apps, &tex_tools, &tex_network, &tex_games],
                        );
                    }
                    DesktopInstallerView::SearchResults => {
                        Self::draw_installer_search_results(
                            ui,
                            &mut self.desktop_installer,
                            palette,
                            &mut deferred_back,
                            &mut deferred_open_search_actions,
                        );
                    }
                    DesktopInstallerView::Installed => {
                        Self::draw_installer_installed(
                            ui,
                            &mut self.desktop_installer,
                            palette,
                            &mut deferred_back,
                            &mut deferred_open_installed_actions,
                        );
                    }
                    DesktopInstallerView::PackageActions { ref pkg, installed } => {
                        let pkg = pkg.clone();
                        Self::draw_installer_package_actions(
                            ui,
                            &mut self.desktop_installer,
                            palette,
                            &pkg,
                            installed,
                            &mut deferred_back,
                            &mut deferred_confirm_setup,
                            &mut deferred_open_add_to_menu,
                        );
                    }
                    DesktopInstallerView::AddToMenu { ref pkg } => {
                        let pkg = pkg.clone();
                        Self::draw_installer_add_to_menu(
                            ui,
                            &mut self.desktop_installer,
                            palette,
                            &pkg,
                            &mut deferred_back,
                            &mut deferred_add_to_menu,
                        );
                    }
                    DesktopInstallerView::RuntimeTools => {
                        Self::draw_installer_runtime_tools(
                            ui,
                            &mut self.desktop_installer,
                            palette,
                            &mut deferred_back,
                            &mut deferred_confirm_setup,
                        );
                    }
                });
        });

        let shown_rect = shown.as_ref().map(|inner| inner.response.rect);
        let shown_contains_pointer = shown
            .as_ref()
            .is_some_and(|inner| inner.response.contains_pointer());
        if let Some(rect) = shown_rect {
            if !maximized {
                let state = self.desktop_window_state_mut(DesktopWindow::Installer);
                state.restore_pos = Some([rect.min.x, rect.min.y]);
                state.restore_size = Some([default_size.x, default_size.y]);
                state.user_resized = false;
                state.apply_restore = false;
            }
            self.maybe_activate_desktop_window_from_click(
                ctx,
                DesktopWindow::Installer,
                shown_contains_pointer,
            );
        }

        // Sync open state
        if !open {
            self.desktop_installer.open = false;
        }
        self.update_desktop_window_state(DesktopWindow::Installer, self.desktop_installer.open);

        // Handle header buttons
        match header_action {
            DesktopHeaderAction::Close => self.close_desktop_window(DesktopWindow::Installer),
            DesktopHeaderAction::Minimize => {
                self.set_desktop_window_minimized(DesktopWindow::Installer, true)
            }
            DesktopHeaderAction::ToggleMaximize => {
                self.toggle_desktop_window_maximized(DesktopWindow::Installer, shown_rect)
            }
            DesktopHeaderAction::None => {}
        }

        // Process deferred actions
        if deferred_back {
            self.desktop_installer.go_back();
        }
        if deferred_search {
            self.desktop_installer.do_search();
        }
        if deferred_load_installed {
            self.desktop_installer.load_installed();
        }
        if deferred_open_runtime_tools {
            self.desktop_installer.view = DesktopInstallerView::RuntimeTools;
        }
        if let Some(pkg) = deferred_open_installed_actions {
            self.desktop_installer.view = DesktopInstallerView::PackageActions {
                pkg,
                installed: true,
            };
        }
        if let Some((pkg, installed)) = deferred_open_search_actions {
            self.desktop_installer.view = DesktopInstallerView::PackageActions { pkg, installed };
        }
        if let Some((pkg, action)) = deferred_confirm_setup {
            self.desktop_installer.confirm_dialog = Some(DesktopInstallerConfirm { pkg, action });
        }
        if deferred_confirm_yes {
            let event = self.desktop_installer.confirm_action();
            if let DesktopInstallerEvent::LaunchCommand {
                argv,
                status,
                completion_message,
            } = event
            {
                self.desktop_installer.status = status.clone();
                self.open_desktop_pty("Program Installer", &argv);
                if let Some(pty) = self.terminal_pty.as_mut() {
                    pty.completion_message = completion_message;
                }
            }
        }
        if deferred_confirm_no {
            self.desktop_installer.confirm_dialog = None;
        }
        if deferred_notice_close {
            self.desktop_installer.notice = None;
        }
        if let Some(pkg) = deferred_open_add_to_menu {
            self.desktop_installer.display_name_input = pkg.clone();
            self.desktop_installer.view = DesktopInstallerView::AddToMenu { pkg };
        }
        if let Some((pkg, target)) = deferred_add_to_menu {
            self.desktop_installer.add_to_menu(&pkg, target);
            self.invalidate_program_catalog_cache();
        }
    }

    // ── Installer sub-views ─────────────────────────────────────────────────

    fn apply_installer_widget_style(ui: &mut egui::Ui, palette: super::retro_ui::RetroPalette) {
        ui.visuals_mut().window_fill = palette.bg;
        ui.visuals_mut().panel_fill = palette.bg;
        ui.visuals_mut().faint_bg_color = palette.bg;
        let widgets = &mut ui.visuals_mut().widgets;
        widgets.inactive.bg_fill = palette.bg;
        widgets.inactive.weak_bg_fill = palette.bg;
        widgets.inactive.bg_stroke = egui::Stroke::new(1.0, palette.fg);
        widgets.inactive.fg_stroke = egui::Stroke::new(1.0, palette.fg);
        widgets.hovered.bg_fill = palette.hovered_bg;
        widgets.hovered.weak_bg_fill = palette.hovered_bg;
        widgets.hovered.bg_stroke = egui::Stroke::new(1.0, palette.fg);
        widgets.hovered.fg_stroke = egui::Stroke::new(1.0, palette.fg);
        widgets.active.bg_fill = palette.active_bg;
        widgets.active.weak_bg_fill = palette.active_bg;
        widgets.active.bg_stroke = egui::Stroke::new(1.0, palette.fg);
        widgets.active.fg_stroke = egui::Stroke::new(1.0, palette.fg);
        widgets.open.bg_fill = palette.hovered_bg;
        widgets.open.weak_bg_fill = palette.hovered_bg;
        widgets.open.bg_stroke = egui::Stroke::new(1.0, palette.fg);
        widgets.open.fg_stroke = egui::Stroke::new(1.0, palette.fg);
        widgets.noninteractive.bg_fill = palette.bg;
        widgets.noninteractive.weak_bg_fill = palette.bg;
        widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0, palette.fg);
        widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0, palette.fg);
        ui.visuals_mut().extreme_bg_color = palette.panel;
        ui.visuals_mut().code_bg_color = palette.bg;
        ui.visuals_mut().window_shadow = egui::epaint::Shadow::NONE;
        ui.visuals_mut().popup_shadow = egui::epaint::Shadow::NONE;
        ui.visuals_mut().window_rounding = egui::Rounding::ZERO;
        ui.visuals_mut().menu_rounding = egui::Rounding::ZERO;
        ui.visuals_mut().selection.bg_fill = palette.selection_bg;
        ui.visuals_mut().selection.stroke = egui::Stroke::new(1.0, palette.fg);
        ui.visuals_mut().text_cursor.stroke = egui::Stroke::new(1.5, palette.fg);
        ui.visuals_mut().widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0, palette.dim);
        ui.visuals_mut().widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0, palette.fg);
    }

    fn apply_installer_dropdown_style(ui: &mut egui::Ui, palette: super::retro_ui::RetroPalette) {
        let mut style = ui.style().as_ref().clone();
        style.visuals.window_fill = palette.bg;
        style.visuals.panel_fill = palette.bg;
        style.visuals.window_stroke = egui::Stroke::new(1.0, palette.fg);
        style.visuals.window_rounding = egui::Rounding::ZERO;
        style.visuals.menu_rounding = egui::Rounding::ZERO;
        style.visuals.window_shadow = egui::epaint::Shadow::NONE;
        style.visuals.popup_shadow = egui::epaint::Shadow::NONE;
        style.visuals.widgets.noninteractive.bg_fill = palette.bg;
        style.visuals.widgets.noninteractive.weak_bg_fill = palette.bg;
        style.visuals.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0, palette.fg);
        style.visuals.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0, palette.fg);
        style.visuals.widgets.inactive.bg_fill = palette.bg;
        style.visuals.widgets.inactive.weak_bg_fill = palette.bg;
        style.visuals.widgets.inactive.bg_stroke = egui::Stroke::new(1.0, palette.fg);
        style.visuals.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, palette.fg);
        style.visuals.widgets.hovered.bg_fill = palette.hovered_bg;
        style.visuals.widgets.hovered.weak_bg_fill = palette.hovered_bg;
        style.visuals.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, palette.fg);
        style.visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, palette.fg);
        style.visuals.widgets.active.bg_fill = palette.active_bg;
        style.visuals.widgets.active.weak_bg_fill = palette.active_bg;
        style.visuals.widgets.active.bg_stroke = egui::Stroke::new(1.0, palette.fg);
        style.visuals.widgets.active.fg_stroke = egui::Stroke::new(1.0, palette.fg);
        style.visuals.widgets.open = style.visuals.widgets.hovered;
        ui.set_style(style);
    }

    fn installer_link_button(
        ui: &mut egui::Ui,
        text: RichText,
        palette: super::retro_ui::RetroPalette,
    ) -> egui::Response {
        ui.scope(|ui| {
            let mut style = ui.style().as_ref().clone();
            style.visuals.widgets.inactive.bg_fill = Color32::TRANSPARENT;
            style.visuals.widgets.inactive.weak_bg_fill = Color32::TRANSPARENT;
            style.visuals.widgets.inactive.bg_stroke = egui::Stroke::NONE;
            style.visuals.widgets.hovered.bg_fill = palette.hovered_bg;
            style.visuals.widgets.hovered.weak_bg_fill = palette.hovered_bg;
            style.visuals.widgets.hovered.bg_stroke = egui::Stroke::NONE;
            style.visuals.widgets.active.bg_fill = palette.active_bg;
            style.visuals.widgets.active.weak_bg_fill = palette.active_bg;
            style.visuals.widgets.active.bg_stroke = egui::Stroke::NONE;
            style.visuals.widgets.open = style.visuals.widgets.hovered;
            style.spacing.button_padding = egui::vec2(8.0, 4.0);
            ui.set_style(style);
            ui.add(egui::Button::new(text))
        })
        .inner
    }

    fn installer_description_preview(desc: &str, limit: usize) -> String {
        let trimmed = desc.trim();
        let count = trimmed.chars().count();
        if count <= limit {
            trimmed.to_string()
        } else {
            format!(
                "{}...",
                trimmed
                    .chars()
                    .take(limit.saturating_sub(3).max(1))
                    .collect::<String>()
                    .trim_end()
            )
        }
    }

    fn draw_installer_home(
        ui: &mut egui::Ui,
        state: &mut DesktopInstallerState,
        palette: super::retro_ui::RetroPalette,
        deferred_search: &mut bool,
        deferred_load_installed: &mut bool,
        deferred_open_runtime_tools: &mut bool,
        icons: [&Option<TextureHandle>; 4], // [apps, tools, network, games]
    ) {
        ui.vertical_centered(|ui| {
            ui.add_space(12.0);
            ui.label(
                RichText::new("RobCo Program Installer")
                    .color(palette.fg)
                    .heading()
                    .strong()
                    .underline(),
            );
            ui.add_space(16.0);

            // ── Search bar ──────────────────────────────────────────────
            let search_width = ui.available_width().min(500.0);
            ui.allocate_ui_with_layout(
                egui::vec2(search_width, 32.0),
                egui::Layout::left_to_right(egui::Align::Center),
                |ui| {
                    let search_field = ui.add_sized(
                        [search_width - 80.0, 28.0],
                        egui::TextEdit::singleline(&mut state.search_query)
                            .hint_text("Search packages...")
                            .text_color(palette.fg)
                            .frame(true),
                    );
                    if search_field.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)) {
                        *deferred_search = true;
                    }
                    if ui
                        .button(RichText::new("Search").color(palette.fg))
                        .clicked()
                    {
                        *deferred_search = true;
                    }
                },
            );

            ui.add_space(24.0);

            // ── Category cards with SVG icons ───────────────────────────
            let card_size = egui::vec2(130.0, 120.0);
            let icon_size = 48.0;
            let categories = [
                (InstallerCategory::Apps, 0usize),
                (InstallerCategory::Tools, 1),
                (InstallerCategory::Network, 2),
                (InstallerCategory::Games, 3),
            ];

            ui.horizontal(|ui| {
                let total_width = categories.len() as f32 * (card_size.x + 16.0) - 16.0;
                let avail = ui.available_width();
                if avail > total_width {
                    ui.add_space((avail - total_width) / 2.0);
                }

                for (cat, icon_idx) in &categories {
                    let (resp, painter) = ui.allocate_painter(card_size, egui::Sense::click());
                    let rect = resp.rect;
                    // Card border
                    painter.rect_stroke(rect, 0.0, egui::Stroke::new(1.0, palette.fg));
                    // Hover highlight
                    if resp.hovered() {
                        painter.rect_filled(rect, 0.0, palette.hovered_bg);
                    }
                    // SVG icon (tinted to theme color)
                    if let Some(tex) = icons[*icon_idx] {
                        let icon_rect = egui::Rect::from_center_size(
                            rect.center() - egui::vec2(0.0, 14.0),
                            egui::vec2(icon_size, icon_size),
                        );
                        Self::paint_tinted_texture(&painter, tex, icon_rect, palette.fg);
                    }
                    // Label
                    painter.text(
                        egui::pos2(rect.center().x, rect.bottom() - 18.0),
                        egui::Align2::CENTER_CENTER,
                        cat.label(),
                        egui::FontId::monospace(16.0),
                        palette.fg,
                    );

                    if resp.clicked() {
                        state.search_query = cat.label().to_lowercase();
                        *deferred_search = true;
                    }
                    ui.add_space(16.0);
                }
            });

            ui.add_space(24.0);

            // ── Installed apps button ───────────────────────────────────
            let installed_btn = Self::installer_link_button(
                ui,
                RichText::new("Installed apps").color(palette.fg).heading(),
                palette,
            );
            if installed_btn.clicked() {
                *deferred_load_installed = true;
            }

            ui.add_space(8.0);

            // ── Runtime tools link ──────────────────────────────────────
            let runtime_btn = Self::installer_link_button(
                ui,
                RichText::new("Runtime Tools").color(palette.dim),
                palette,
            );
            if runtime_btn.clicked() {
                *deferred_open_runtime_tools = true;
            }

            ui.add_space(8.0);

            // ── Package manager selector ─────────────────────────────────
            state.ensure_available_pms();
            if state.available_pms.len() > 1 {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Package Manager:").color(palette.dim).small());
                    let current_label = state.pm_label().to_string();
                    ui.scope(|ui| {
                        Self::apply_installer_dropdown_style(ui, palette);
                        egui::ComboBox::from_id_salt("pm_selector")
                            .selected_text(RichText::new(&current_label).color(palette.fg).small())
                            .show_ui(ui, |ui| {
                                Self::apply_installer_dropdown_style(ui, palette);
                                for (idx, pm) in state.available_pms.clone().iter().enumerate() {
                                    let selected = idx == state.selected_pm_idx;
                                    let text_color =
                                        if selected { Color32::BLACK } else { palette.fg };
                                    if ui
                                        .selectable_label(
                                            selected,
                                            RichText::new(pm.name()).color(text_color),
                                        )
                                        .clicked()
                                    {
                                        state.select_package_manager(idx);
                                    }
                                }
                            });
                    });
                });
            } else {
                ui.label(
                    RichText::new(format!("Package Manager: {}", state.pm_label()))
                        .color(palette.dim)
                        .small(),
                );
            }
        });
    }

    fn draw_installer_search_results(
        ui: &mut egui::Ui,
        state: &mut DesktopInstallerState,
        palette: super::retro_ui::RetroPalette,
        deferred_back: &mut bool,
        deferred_open_actions: &mut Option<(String, bool)>,
    ) {
        const HEADER_H: f32 = 28.0;
        const FOOTER_H: f32 = 40.0;
        const RESULTS_PER_PAGE: usize = 20;
        let total = state.search_results.len();
        let row_height = 58.0;
        let page_size = RESULTS_PER_PAGE.max(1);
        let total_pages = total.div_ceil(page_size).max(1);
        state.search_page = state.search_page.min(total_pages.saturating_sub(1));
        let start = state.search_page * page_size;
        let end = (start + page_size).min(total);
        egui::TopBottomPanel::top("inst_search_top")
            .frame(egui::Frame::none())
            .exact_height(HEADER_H)
            .show_inside(ui, |ui| {
                ui.horizontal(|ui| {
                    if ui
                        .button(RichText::new("< Back").color(palette.fg))
                        .clicked()
                    {
                        *deferred_back = true;
                    }
                    ui.add_space(8.0);
                    ui.label(
                        RichText::new(format!(
                            "Search Results: \"{}\"  ({} found)",
                            state.search_query,
                            state.search_results.len()
                        ))
                        .color(palette.fg)
                        .strong(),
                    );
                });
            });

        egui::TopBottomPanel::bottom("inst_search_bottom")
            .frame(egui::Frame::none())
            .exact_height(FOOTER_H)
            .show_inside(ui, |ui| {
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if state.search_page > 0
                        && ui
                            .button(RichText::new("< Prev").color(palette.fg))
                            .clicked()
                    {
                        state.search_page -= 1;
                    }
                    ui.add_space(8.0);
                    ui.label(
                        RichText::new(format!("Page {}/{}", state.search_page + 1, total_pages))
                            .color(palette.dim),
                    );
                    ui.add_space(8.0);
                    if state.search_page + 1 < total_pages
                        && ui
                            .button(RichText::new("Next >").color(palette.fg))
                            .clicked()
                    {
                        state.search_page += 1;
                    }
                });
            });

        let available = ui.available_rect_before_wrap();
        let body_size = egui::vec2(available.width().max(240.0), available.height().max(120.0));
        let body_rect = egui::Rect::from_min_size(available.min, body_size);
        ui.allocate_rect(body_rect, egui::Sense::hover());
        ui.scope_builder(egui::UiBuilder::new().max_rect(body_rect), |ui| {
            ui.set_min_size(body_size);
            ui.set_max_size(body_size);
            ui.style_mut().spacing.scroll = egui::style::ScrollStyle::solid();
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysVisible)
                .show(ui, |ui| {
                    let row_width = (ui.available_width() - 2.0).floor().max(220.0);
                    for idx in start..end {
                        let result = &state.search_results[idx];
                        let desc_preview = result
                            .description
                            .as_ref()
                            .cloned()
                            .or_else(|| installer_cached_package_description(state, &result.pkg))
                            .as_ref()
                            .map(|desc| Self::installer_description_preview(desc, 72));
                        let (_, row_rect) =
                            ui.allocate_space(egui::vec2(row_width, row_height - 4.0));
                        ui.scope_builder(egui::UiBuilder::new().max_rect(row_rect), |ui| {
                            let frame = egui::Frame::none()
                                .stroke(egui::Stroke::new(1.0, palette.fg))
                                .inner_margin(egui::Margin::same(2.0));
                            let content_width = (row_width - 4.0).max(80.0);
                            frame.show(ui, |ui| {
                                ui.set_min_width(content_width);
                                ui.set_max_width(content_width);
                                ui.set_min_height(row_height - 8.0);
                                let button_width = 112.0;
                                let text_width = (content_width - button_width - 24.0).max(140.0);
                                ui.horizontal(|ui| {
                                    ui.allocate_ui_with_layout(
                                        egui::vec2(text_width, 0.0),
                                        egui::Layout::left_to_right(egui::Align::Center),
                                        |ui| {
                                            let status_text = if result.installed {
                                                "[installed]"
                                            } else {
                                                "[get]"
                                            };
                                            let status_color = if result.installed {
                                                palette.dim
                                            } else {
                                                palette.fg
                                            };
                                            ui.label(
                                                RichText::new(status_text).color(status_color),
                                            );
                                            ui.add_space(6.0);
                                            ui.add_sized(
                                                [ui.available_width().max(80.0), 0.0],
                                                egui::Label::new(
                                                    RichText::new(&result.pkg)
                                                        .color(palette.fg)
                                                        .strong(),
                                                )
                                                .truncate(),
                                            );
                                        },
                                    );
                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            let btn_label = if result.installed {
                                                "Actions"
                                            } else {
                                                "Install"
                                            };
                                            if ui
                                                .add_sized(
                                                    [button_width, 24.0],
                                                    egui::Button::new(
                                                        RichText::new(format!("[ {btn_label} ]"))
                                                            .color(palette.fg),
                                                    ),
                                                )
                                                .clicked()
                                            {
                                                *deferred_open_actions =
                                                    Some((result.pkg.clone(), result.installed));
                                            }
                                        },
                                    );
                                });
                                ui.add_space(2.0);
                                let desc_text = desc_preview.unwrap_or_else(|| {
                                    if state.can_fetch_descriptions() {
                                        String::new()
                                    } else {
                                        "Description unavailable while offline.".to_string()
                                    }
                                });
                                if !desc_text.is_empty() {
                                    ui.add_sized(
                                        [(content_width - 8.0).max(80.0), 0.0],
                                        egui::Label::new(
                                            RichText::new(desc_text).color(palette.dim),
                                        )
                                        .truncate(),
                                    );
                                }
                            });
                        });
                    }
                });
        });
    }

    fn draw_installer_installed(
        ui: &mut egui::Ui,
        state: &mut DesktopInstallerState,
        palette: super::retro_ui::RetroPalette,
        deferred_back: &mut bool,
        deferred_open_actions: &mut Option<String>,
    ) {
        const HEADER_H: f32 = 28.0;
        const FOOTER_H: f32 = 40.0;
        const RESULTS_PER_PAGE: usize = 20;
        let filtered = state.filtered_installed();
        let total = filtered.len();
        let row_height = 58.0;
        let page_size = RESULTS_PER_PAGE.max(1);
        let total_pages = total.div_ceil(page_size).max(1);
        state.installed_page = state.installed_page.min(total_pages.saturating_sub(1));
        let start = state.installed_page * page_size;
        let end = (start + page_size).min(total);
        egui::TopBottomPanel::top("inst_installed_top")
            .frame(egui::Frame::none())
            .exact_height(HEADER_H)
            .show_inside(ui, |ui| {
                ui.horizontal(|ui| {
                    if ui
                        .button(RichText::new("< Back").color(palette.fg))
                        .clicked()
                    {
                        *deferred_back = true;
                    }
                    ui.add_space(8.0);
                    ui.label(RichText::new("Installed Apps").color(palette.fg).strong());
                    ui.add_space(16.0);
                    ui.label(RichText::new("Filter:").color(palette.dim));
                    ui.add_sized(
                        [200.0, 0.0],
                        egui::TextEdit::singleline(&mut state.installed_filter)
                            .hint_text("type to filter...")
                            .text_color(palette.fg),
                    );
                });
            });

        egui::TopBottomPanel::bottom("inst_installed_bottom")
            .frame(egui::Frame::none())
            .exact_height(FOOTER_H)
            .show_inside(ui, |ui| {
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if state.installed_page > 0
                        && ui
                            .button(RichText::new("< Prev").color(palette.fg))
                            .clicked()
                    {
                        state.installed_page -= 1;
                    }
                    ui.add_space(8.0);
                    ui.label(
                        RichText::new(format!(
                            "Page {}/{}  ({} packages)",
                            state.installed_page + 1,
                            total_pages,
                            total
                        ))
                        .color(palette.dim),
                    );
                    ui.add_space(8.0);
                    if state.installed_page + 1 < total_pages
                        && ui
                            .button(RichText::new("Next >").color(palette.fg))
                            .clicked()
                    {
                        state.installed_page += 1;
                    }
                });
            });

        let available = ui.available_rect_before_wrap();
        let body_size = egui::vec2(available.width().max(240.0), available.height().max(120.0));
        let body_rect = egui::Rect::from_min_size(available.min, body_size);
        ui.allocate_rect(body_rect, egui::Sense::hover());
        ui.scope_builder(egui::UiBuilder::new().max_rect(body_rect), |ui| {
            ui.set_min_size(body_size);
            ui.set_max_size(body_size);
            ui.style_mut().spacing.scroll = egui::style::ScrollStyle::solid();
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysVisible)
                .show(ui, |ui| {
                    let row_width = (ui.available_width() - 2.0).floor().max(220.0);
                    for idx in start..end {
                        let pkg = &filtered[idx];
                        let desc_preview = installer_cached_package_description(state, pkg)
                            .map(|desc| Self::installer_description_preview(&desc, 72));
                        let (_, row_rect) =
                            ui.allocate_space(egui::vec2(row_width, row_height - 4.0));
                        ui.scope_builder(egui::UiBuilder::new().max_rect(row_rect), |ui| {
                            let frame = egui::Frame::none()
                                .stroke(egui::Stroke::new(1.0, palette.fg))
                                .inner_margin(egui::Margin::same(2.0));
                            let content_width = (row_width - 4.0).max(80.0);
                            frame.show(ui, |ui| {
                                ui.set_min_width(content_width);
                                ui.set_max_width(content_width);
                                ui.set_min_height(row_height - 8.0);
                                let button_width = 112.0;
                                let text_width = (content_width - button_width - 24.0).max(140.0);
                                ui.horizontal(|ui| {
                                    ui.add_sized(
                                        [text_width, 0.0],
                                        egui::Label::new(
                                            RichText::new(pkg).color(palette.fg).strong(),
                                        )
                                        .truncate(),
                                    );
                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            if ui
                                                .add_sized(
                                                    [button_width, 24.0],
                                                    egui::Button::new(
                                                        RichText::new("[ Actions ]")
                                                            .color(palette.fg),
                                                    ),
                                                )
                                                .clicked()
                                            {
                                                *deferred_open_actions = Some(pkg.clone());
                                            }
                                        },
                                    );
                                });
                                ui.add_space(2.0);
                                let desc_text = desc_preview.unwrap_or_else(|| {
                                    if state.can_fetch_descriptions() {
                                        String::new()
                                    } else {
                                        "Description unavailable while offline.".to_string()
                                    }
                                });
                                if !desc_text.is_empty() {
                                    ui.add_sized(
                                        [(content_width - 8.0).max(80.0), 0.0],
                                        egui::Label::new(
                                            RichText::new(desc_text).color(palette.dim),
                                        )
                                        .truncate(),
                                    );
                                }
                            });
                        });
                    }
                });
        });
    }

    fn draw_installer_package_actions(
        ui: &mut egui::Ui,
        state: &mut DesktopInstallerState,
        palette: super::retro_ui::RetroPalette,
        pkg: &str,
        installed: bool,
        deferred_back: &mut bool,
        deferred_confirm: &mut Option<(String, InstallerPackageAction)>,
        deferred_open_add_to_menu: &mut Option<String>,
    ) {
        ui.horizontal(|ui| {
            if ui
                .button(RichText::new("< Back").color(palette.fg))
                .clicked()
            {
                *deferred_back = true;
            }
            ui.add_space(8.0);
            ui.label(RichText::new("App Details").color(palette.dim).strong());
        });
        ui.separator();
        ui.add_space(12.0);

        let description = state.fetch_package_description(pkg);
        let status_label = if installed { "Installed" } else { "Available" };

        egui::Frame::none()
            .fill(palette.panel)
            .stroke(egui::Stroke::new(1.0, palette.fg))
            .inner_margin(egui::Margin::same(18.0))
            .show(ui, |ui| {
                ui.vertical_centered(|ui| {
                    ui.label(RichText::new(pkg).color(palette.fg).heading().strong());
                    ui.add_space(4.0);
                    ui.label(
                        RichText::new(format!("{} via {}", status_label, state.pm_label()))
                            .color(palette.dim),
                    );
                });
                ui.add_space(14.0);
                ui.separator();
                ui.add_space(14.0);
                ui.label(RichText::new("Description").color(palette.fg).strong());
                ui.add_space(6.0);
                match description {
                    Some(desc) => {
                        ui.label(RichText::new(desc).color(palette.dim));
                    }
                    None => {
                        let message = if state.can_fetch_descriptions() {
                            "Description unavailable."
                        } else {
                            "Description unavailable while offline."
                        };
                        ui.label(RichText::new(message).color(palette.dim));
                    }
                }
            });

        ui.add_space(16.0);

        if installed {
            ui.horizontal_wrapped(|ui| {
                if ui
                    .button(RichText::new("[ Update ]").color(palette.fg))
                    .clicked()
                {
                    *deferred_confirm = Some((pkg.to_string(), InstallerPackageAction::Update));
                }
                ui.add_space(8.0);
                if ui
                    .button(RichText::new("[ Reinstall ]").color(palette.fg))
                    .clicked()
                {
                    *deferred_confirm = Some((pkg.to_string(), InstallerPackageAction::Reinstall));
                }
                ui.add_space(8.0);
                if ui
                    .button(RichText::new("[ Uninstall ]").color(palette.fg))
                    .clicked()
                {
                    *deferred_confirm = Some((pkg.to_string(), InstallerPackageAction::Uninstall));
                }
                ui.add_space(8.0);
                if ui
                    .button(RichText::new("[ Add to Menu ]").color(palette.fg))
                    .clicked()
                {
                    *deferred_open_add_to_menu = Some(pkg.to_string());
                }
            });
        } else if ui
            .button(RichText::new("[ Install ]").color(palette.fg))
            .clicked()
        {
            *deferred_confirm = Some((pkg.to_string(), InstallerPackageAction::Install));
        }
    }

    fn draw_installer_add_to_menu(
        ui: &mut egui::Ui,
        state: &mut DesktopInstallerState,
        palette: super::retro_ui::RetroPalette,
        pkg: &str,
        deferred_back: &mut bool,
        deferred_add: &mut Option<(String, InstallerMenuTarget)>,
    ) {
        ui.horizontal(|ui| {
            if ui
                .button(RichText::new("< Back").color(palette.fg))
                .clicked()
            {
                *deferred_back = true;
            }
            ui.add_space(8.0);
            ui.label(
                RichText::new(format!("Add \"{}\" to Menu", pkg))
                    .color(palette.fg)
                    .strong(),
            );
        });
        ui.separator();
        ui.add_space(12.0);

        ui.horizontal(|ui| {
            ui.label(RichText::new("Display Name:").color(palette.fg));
            ui.add_sized(
                [250.0, 0.0],
                egui::TextEdit::singleline(&mut state.display_name_input)
                    .hint_text(pkg)
                    .text_color(palette.fg),
            );
        });
        ui.add_space(16.0);

        ui.label(RichText::new("Choose target menu:").color(palette.fg));
        ui.add_space(8.0);

        ui.horizontal(|ui| {
            if ui
                .button(RichText::new("[ Applications ]").color(palette.fg))
                .clicked()
            {
                *deferred_add = Some((pkg.to_string(), InstallerMenuTarget::Applications));
            }
            ui.add_space(8.0);
            if ui
                .button(RichText::new("[ Games ]").color(palette.fg))
                .clicked()
            {
                *deferred_add = Some((pkg.to_string(), InstallerMenuTarget::Games));
            }
            ui.add_space(8.0);
            if ui
                .button(RichText::new("[ Network ]").color(palette.fg))
                .clicked()
            {
                *deferred_add = Some((pkg.to_string(), InstallerMenuTarget::Network));
            }
        });
    }

    fn draw_installer_runtime_tools(
        ui: &mut egui::Ui,
        state: &mut DesktopInstallerState,
        palette: super::retro_ui::RetroPalette,
        deferred_back: &mut bool,
        deferred_confirm: &mut Option<(String, InstallerPackageAction)>,
    ) {
        ui.horizontal(|ui| {
            if ui
                .button(RichText::new("< Back").color(palette.fg))
                .clicked()
            {
                *deferred_back = true;
            }
            ui.add_space(8.0);
            ui.label(RichText::new("Runtime Tools").color(palette.fg).strong());
        });
        ui.separator();
        ui.add_space(12.0);

        for (idx, tool) in available_runtime_tools().iter().copied().enumerate() {
            if idx > 0 {
                ui.add_space(12.0);
            }
            let installed = installer_runtime_tool_installed_cached(state, tool);
            let status = if installed {
                "[installed]"
            } else {
                "[not installed]"
            };
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(format!(
                        "{} {} — {}",
                        status,
                        runtime_tool_title(tool),
                        runtime_tool_description(tool)
                    ))
                    .color(palette.fg),
                );
            });
            ui.horizontal(|ui| {
                for (action_idx, action) in runtime_tool_actions(installed).iter().enumerate() {
                    let label = match action {
                        InstallerPackageAction::Install => "[ Install ]",
                        InstallerPackageAction::Update => "[ Update ]",
                        InstallerPackageAction::Reinstall => "[ Reinstall ]",
                        InstallerPackageAction::Uninstall => "[ Uninstall ]",
                    };
                    if ui.button(RichText::new(label).color(palette.fg)).clicked() {
                        *deferred_confirm =
                            runtime_tool_action_for_selection(installed, action_idx)
                                .map(|action| (runtime_tool_pkg(tool).to_string(), action));
                    }
                }
            });
        }
    }

    fn draw_applications(&mut self, ctx: &Context) {
        if !self.applications.open || self.desktop_window_is_minimized(DesktopWindow::Applications)
        {
            return;
        }
        let mut open = self.applications.open;
        let mut close_after_launch = false;
        let mut header_action = DesktopHeaderAction::None;
        let (window, maximized) = self.build_resizable_desktop_window(
            ctx,
            DesktopWindow::Applications,
            "Applications",
            &mut open,
            ResizableDesktopWindowOptions {
                min_size: egui::vec2(320.0, 250.0),
                default_size: Self::desktop_default_window_size(DesktopWindow::Applications),
                default_pos: None,
                clamp_restore: false,
            },
        );
        let shown = window.show(ctx, |ui| {
            Self::apply_settings_control_style(ui);
            header_action = Self::draw_desktop_window_header(ui, "Applications", maximized);
            let sections = self.desktop_applications_sections();
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.heading("Built-in");
                for entry in &sections.builtins {
                    if Self::retro_full_width_button(ui, entry.label.as_str()).clicked() {
                        let request = resolve_desktop_applications_request(&entry.action);
                        close_after_launch = matches!(
                            request,
                            DesktopProgramRequest::OpenNukeCodes { close_window: true }
                                | DesktopProgramRequest::LaunchCatalog {
                                    close_window: true,
                                    ..
                                }
                        );
                        self.apply_desktop_program_request(request);
                    }
                }
                ui.separator();
                ui.heading("Configured Apps");
                for entry in &sections.configured {
                    if Self::retro_full_width_button(ui, entry.label.as_str()).clicked() {
                        let request = resolve_desktop_applications_request(&entry.action);
                        close_after_launch = matches!(
                            request,
                            DesktopProgramRequest::OpenNukeCodes { close_window: true }
                                | DesktopProgramRequest::LaunchCatalog {
                                    close_window: true,
                                    ..
                                }
                        );
                        self.apply_desktop_program_request(request);
                    }
                }
                if !self.applications.status.is_empty() {
                    ui.separator();
                    ui.small(&self.applications.status);
                }
            });
        });
        let shown_rect = shown.as_ref().map(|inner| inner.response.rect);
        let shown_contains_pointer = shown
            .as_ref()
            .is_some_and(|inner| inner.response.contains_pointer());
        // Context menus are attached to specific content widgets inside the
        // window closure, not to the outer Area response (which causes
        // "double use of widget" ID collisions in egui 0.29).
        self.maybe_activate_desktop_window_from_click(
            ctx,
            DesktopWindow::Applications,
            shown_contains_pointer,
        );
        if !maximized {
            if let Some(rect) = shown_rect {
                self.note_desktop_window_rect(DesktopWindow::Applications, rect);
            }
        }
        if close_after_launch {
            open = false;
        }
        self.finish_desktop_window_host(
            ctx,
            DesktopWindow::Applications,
            &mut open,
            maximized,
            shown_rect,
            shown_contains_pointer,
            DesktopWindowRectTracking::FullRect,
            header_action,
        );
    }

    fn draw_desktop_donkey_kong(&mut self, ctx: &Context) {
        if !self.donkey_kong_window.open
            || self.desktop_window_is_minimized(DesktopWindow::DonkeyKong)
        {
            return;
        }
        ctx.request_repaint();
        let mut open = self.donkey_kong_window.open;
        let mut header_action = DesktopHeaderAction::None;
        let (window, maximized) = self.build_resizable_desktop_window(
            ctx,
            DesktopWindow::DonkeyKong,
            BUILTIN_DONKEY_KONG_GAME,
            &mut open,
            ResizableDesktopWindowOptions {
                min_size: egui::vec2(560.0, 500.0),
                default_size: Self::desktop_default_window_size(DesktopWindow::DonkeyKong),
                default_pos: None,
                clamp_restore: false,
            },
        );
        let theme = self.current_donkey_kong_theme();
        let dt = ctx.input(|i| i.stable_dt).max(1.0 / 60.0);
        let input = donkey_kong_input_from_ctx(ctx);
        let game = self.ensure_donkey_kong_loaded(ctx);
        game.set_theme(theme);
        game.update(input, dt);
        let shown = window.show(ctx, |ui| {
            Self::apply_settings_control_style(ui);
            header_action =
                Self::draw_desktop_window_header(ui, BUILTIN_DONKEY_KONG_GAME, maximized);
            ui.horizontal(|ui| {
                ui.small("Arrow keys / WASD move");
                ui.separator();
                ui.small("Space jump / restart");
                ui.separator();
                ui.small("Esc closes");
            });
            ui.separator();
            let game_rect = ui.available_rect_before_wrap();
            game.draw(ui, game_rect);
            ui.allocate_rect(game_rect, egui::Sense::hover());
        });
        let shown_rect = shown.as_ref().map(|inner| inner.response.rect);
        let shown_contains_pointer = shown
            .as_ref()
            .is_some_and(|inner| inner.response.contains_pointer());
        self.finish_desktop_window_host(
            ctx,
            DesktopWindow::DonkeyKong,
            &mut open,
            maximized,
            shown_rect,
            shown_contains_pointer,
            DesktopWindowRectTracking::FullRect,
            header_action,
        );
    }

    fn draw_nuke_codes_window(&mut self, ctx: &Context) {
        if !self.desktop_nuke_codes_open
            || self.desktop_window_is_minimized(DesktopWindow::NukeCodes)
        {
            return;
        }
        let mut open = self.desktop_nuke_codes_open;
        let mut header_action = DesktopHeaderAction::None;
        let mut refresh = false;
        let (window, maximized) = self.build_resizable_desktop_window(
            ctx,
            DesktopWindow::NukeCodes,
            "Nuke Codes",
            &mut open,
            ResizableDesktopWindowOptions {
                min_size: egui::vec2(300.0, 200.0),
                default_size: Self::desktop_default_window_size(DesktopWindow::NukeCodes),
                default_pos: None,
                clamp_restore: false,
            },
        );
        let shown = window.show(ctx, |ui| {
            Self::apply_settings_control_style(ui);
            header_action = Self::draw_desktop_window_header(ui, "Nuke Codes", maximized);
            if Self::retro_full_width_button(ui, "Refresh").clicked() {
                refresh = true;
            }
            ui.separator();
            ui.add_space(12.0);
            match &self.terminal_nuke_codes {
                NukeCodesView::Unloaded => {
                    ui.monospace("Codes are not loaded yet.");
                }
                NukeCodesView::Error(err) => {
                    ui.monospace("UNABLE TO FETCH LIVE CODES");
                    ui.small(format!("ERROR: {err}"));
                }
                NukeCodesView::Data(codes) => {
                    ui.monospace(format!("ALPHA   : {}", codes.alpha));
                    ui.monospace(format!("BRAVO   : {}", codes.bravo));
                    ui.monospace(format!("CHARLIE : {}", codes.charlie));
                    ui.add_space(6.0);
                    ui.small(format!("Source: {}", codes.source));
                    ui.small(format!("Fetched: {}", codes.fetched_at));
                }
            }
        });
        let shown_rect = shown.as_ref().map(|inner| inner.response.rect);
        let shown_contains_pointer = shown
            .as_ref()
            .is_some_and(|inner| inner.response.contains_pointer());
        // Context menus are attached to specific content widgets inside the
        // window closure, not to the outer Area response (which causes
        // "double use of widget" ID collisions in egui 0.29).
        self.maybe_activate_desktop_window_from_click(
            ctx,
            DesktopWindow::NukeCodes,
            shown_contains_pointer,
        );
        if refresh {
            self.terminal_nuke_codes = fetch_nuke_codes();
        }
        self.finish_desktop_window_host(
            ctx,
            DesktopWindow::NukeCodes,
            &mut open,
            maximized,
            shown_rect,
            shown_contains_pointer,
            DesktopWindowRectTracking::FullRect,
            header_action,
        );
    }

    fn draw_desktop_pty_window(&mut self, ctx: &Context) {
        if self.desktop_window_is_minimized(DesktopWindow::PtyApp) {
            return;
        }
        let default_size = Self::desktop_default_window_size(DesktopWindow::PtyApp);
        let default_pos = Self::desktop_default_window_pos(ctx, default_size);
        let pty_focused = self.desktop_active_window == Some(DesktopWindow::PtyApp);
        let Some(pty_state) = self.terminal_pty.as_ref() else {
            self.update_desktop_window_state(DesktopWindow::PtyApp, false);
            return;
        };
        let title = pty_state.title.clone();
        let min_size = Self::native_pty_window_min_size(pty_state);
        let mut open = true;
        let mut header_action = DesktopHeaderAction::None;
        let mut event = PtyScreenEvent::None;
        let (window, maximized) = self.build_resizable_desktop_window(
            ctx,
            DesktopWindow::PtyApp,
            title.clone(),
            &mut open,
            ResizableDesktopWindowOptions {
                min_size,
                default_size,
                default_pos: Some(default_pos),
                clamp_restore: true,
            },
        );
        let Some(state) = self.terminal_pty.as_mut() else {
            self.update_desktop_window_state(DesktopWindow::PtyApp, false);
            return;
        };
        let shown = window.show(ctx, |ui| {
            // NOTE: do NOT call apply_settings_control_style here — it changes
            // extreme_bg_color and margins, which destabilizes available_size()
            // causing resize oscillation (constant SIGWINCH) for ncurses apps.
            header_action = Self::draw_desktop_window_header(ui, &title, maximized);
            let available = ui.available_size();
            let cols_floor = state.desktop_cols_floor.unwrap_or(40) as usize;
            let rows_floor = state.desktop_rows_floor.unwrap_or(20).saturating_add(1) as usize;
            let (cols, rows) = if state.desktop_live_resize {
                (
                    ((available.x / FIXED_PTY_CELL_W).floor() as usize)
                        .max(cols_floor)
                        .clamp(40, 220),
                    ((available.y / FIXED_PTY_CELL_H).floor() as usize)
                        .max(rows_floor)
                        .clamp(20, 60),
                )
            } else {
                (cols_floor, rows_floor)
            };
            ui.allocate_ui_with_layout(available, Layout::top_down(egui::Align::Min), |ui| {
                event = draw_embedded_pty_in_ui_focused(ui, ctx, state, cols, rows, pty_focused);
            });
        });
        let shown_rect = shown.as_ref().map(|inner| inner.response.rect);
        let shown_contains_pointer = shown
            .as_ref()
            .is_some_and(|inner| inner.response.contains_pointer());
        // Context menus are attached to specific content widgets inside the
        // window closure, not to the outer Area response (which causes
        // "double use of widget" ID collisions in egui 0.29).
        let completion_message = state.completion_message.clone();
        let title_for_exit = state.title.clone();
        let mut desktop_exit_plan: Option<TerminalDesktopPtyExitPlan> = None;

        match event {
            PtyScreenEvent::None => {}
            PtyScreenEvent::CloseRequested => open = false,
            PtyScreenEvent::ProcessExited => {
                let exit_status = state.session.exit_status();
                let success = exit_status
                    .as_ref()
                    .map(|status| status.success())
                    .unwrap_or(true);
                let exit_code = exit_status.as_ref().map(|status| status.exit_code());
                open = false;
                desktop_exit_plan = Some(resolve_desktop_pty_exit(
                    &title_for_exit,
                    completion_message.as_deref(),
                    success,
                    exit_code,
                ));
            }
        }

        self.maybe_activate_desktop_window_from_click(
            ctx,
            DesktopWindow::PtyApp,
            shown_contains_pointer,
        );
        if !maximized {
            if let Some(rect) = shown_rect {
                self.note_desktop_window_rect(DesktopWindow::PtyApp, rect);
            }
        }
        if let Some(plan) = desktop_exit_plan {
            self.apply_terminal_desktop_pty_exit_plan(plan);
        }
        self.finish_desktop_window_host(
            ctx,
            DesktopWindow::PtyApp,
            &mut open,
            maximized,
            shown_rect,
            shown_contains_pointer,
            DesktopWindowRectTracking::FullRect,
            header_action,
        );
    }

    fn draw_terminal_mode(&mut self, ctx: &Context) {
        if !self.terminal_mode.open || self.desktop_window_is_minimized(DesktopWindow::TerminalMode)
        {
            return;
        }
        let _ = ctx;
        self.terminal_mode.open = false;
        self.desktop_window_states
            .remove(&DesktopWindow::TerminalMode);
        self.open_desktop_terminal_shell();
    }

    fn handle_desktop_file_manager_shortcuts(&mut self, ctx: &Context) {
        if self.desktop_active_window != Some(DesktopWindow::FileManager)
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
}

impl eframe::App for RobcoNativeApp {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        Color32::from_rgb(0, 0, 0).to_normalized_gamma_f32()
    }

    fn save(&mut self, _storage: &mut dyn eframe::Storage) {
        self.persist_snapshot();
    }

    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        // Process PTY keyboard input at the very top of the frame, before
        // any egui widgets render.  Widgets (TextEdit, menus, buttons) can
        // consume Event::Key and Event::Text from the events list during
        // their show() calls, leaving the PTY with zero events if it runs
        // after them.
        let mut early_pty_close = false;
        if self.desktop_mode_open && self.desktop_active_window == Some(DesktopWindow::PtyApp) {
            if let Some(state) = self.terminal_pty.as_mut() {
                if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(Key::Q)) {
                    early_pty_close = true;
                }
                if ctx.input(|i| i.modifiers.ctrl && i.modifiers.shift && i.key_pressed(Key::P)) {
                    state.show_perf_overlay = !state.show_perf_overlay;
                }
                handle_pty_input(ctx, &mut state.session);
                // Clear keyboard events so the later draw pass doesn't
                // double-process them.
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

        assert_eq!(app.desktop_active_window, Some(DesktopWindow::Editor));
        assert!(!app.spotlight_open);
    }

    #[test]
    fn reopening_settings_window_reprimes_component_state() {
        let mut app = RobcoNativeApp::default();
        app.settings.open = true;
        let state = app.desktop_window_state_mut(DesktopWindow::Settings);
        state.restore_pos = Some([24.0, 48.0]);
        state.restore_size = Some([640.0, 360.0]);
        state.apply_restore = true;
        state.maximized = true;
        state.minimized = true;
        state.user_resized = true;
        state.generation = 7;

        app.open_desktop_window(DesktopWindow::Settings);

        let state = app.desktop_window_state(DesktopWindow::Settings);
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
        let state = app.desktop_window_state_mut(DesktopWindow::Installer);
        state.restore_pos = Some([12.0, 36.0]);
        state.restore_size = Some([800.0, 520.0]);
        state.apply_restore = true;
        state.maximized = true;
        state.minimized = true;
        state.user_resized = true;
        state.generation = 5;

        app.open_desktop_window(DesktopWindow::Installer);

        let state = app.desktop_window_state(DesktopWindow::Installer);
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
        let state = app.desktop_window_state_mut(DesktopWindow::Settings);
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

        let state = app.desktop_window_state(DesktopWindow::Settings);
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
        app.desktop_active_window = Some(DesktopWindow::FileManager);
        app.file_manager.open = true;

        app.apply_desktop_menu_action(
            &Context::default(),
            &DesktopMenuAction::ActivateDesktopWindow(DesktopWindow::FileManager),
        );

        assert_eq!(app.desktop_active_window, Some(DesktopWindow::FileManager));
        assert!(!app.start_open);
        assert!(!app.spotlight_open);
    }

    #[test]
    fn activating_taskbar_for_active_window_minimizes_it() {
        let _guard = session_test_guard();

        let mut app = RobcoNativeApp::default();
        app.file_manager.open = true;
        app.desktop_active_window = Some(DesktopWindow::FileManager);

        app.apply_desktop_menu_action(
            &Context::default(),
            &DesktopMenuAction::ActivateTaskbarWindow(DesktopWindow::FileManager),
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
            &DesktopMenuAction::ActivateTaskbarWindow(DesktopWindow::FileManager),
        );

        assert!(!app.desktop_window_is_minimized(DesktopWindow::FileManager));
        assert_eq!(app.desktop_active_window, Some(DesktopWindow::FileManager));
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
            Some(DesktopWindow::Applications)
        );
    }

    #[test]
    fn minimizing_active_window_activates_next_topmost_window() {
        let _guard = session_test_guard();

        let mut app = RobcoNativeApp::default();
        app.file_manager.open = true;
        app.settings.open = true;
        app.applications.open = true;
        app.desktop_active_window = Some(DesktopWindow::Applications);

        app.set_desktop_window_minimized(DesktopWindow::Applications, true);

        assert_eq!(app.desktop_active_window, Some(DesktopWindow::Settings));
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
