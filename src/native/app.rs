use super::about_screen::draw_about_screen;
use super::connections_screen::{
    apply_search_query as apply_connection_search_query, draw_connections_screen, ConnectionsEvent,
    TerminalConnectionsState,
};
use super::data::{
    app_names, authenticate, bind_login_session, current_settings, home_dir_fallback, logs_dir,
    read_shell_snapshot, read_text_file, save_settings, save_text_file, word_processor_dir,
    write_shell_snapshot,
};
use super::default_apps_screen::{
    apply_custom_command as apply_default_app_custom_command, draw_default_apps_screen,
    DefaultAppsEvent,
};
use super::document_browser::{activate_browser_selection, draw_terminal_document_browser};
use super::edit_menus_screen::{
    draw_edit_menus_screen, EditMenuTarget, EditMenusEntries, EditMenusEvent,
    TerminalEditMenusState,
};
use super::file_manager::{FileManagerAction, NativeFileManagerState};
use super::hacking_screen::{draw_hacking_screen, draw_locked_screen, HackingScreenEvent};
use super::installer_screen::{
    add_package_to_menu, apply_filter as apply_installer_filter,
    apply_search_query as apply_installer_search_query, build_package_command,
    draw_installer_screen, settle_view_after_package_command, InstallerEvent,
    InstallerPackageAction, TerminalInstallerState,
};
use super::menu::{
    draw_terminal_menu_screen, login_menu_rows_from_users, SettingsChoiceOverlay, TerminalScreen,
    UserManagementMode,
};
use super::nuke_codes_screen::{
    draw_nuke_codes_screen, fetch_nuke_codes, NukeCodesEvent, NukeCodesView,
};
use super::programs_screen::{draw_programs_menu, resolve_program_command, ProgramMenuEvent};
use super::prompt::{
    draw_terminal_flash, draw_terminal_flash_boxed, draw_terminal_prompt_overlay, FlashAction,
    TerminalFlash, TerminalPrompt, TerminalPromptAction, TerminalPromptKind,
};
use super::prompt_flow::{handle_prompt_input, PromptOutcome};
use super::pty_screen::{
    draw_embedded_pty, draw_embedded_pty_in_ui, spawn_embedded_pty_with_options, NativePtyState,
    PtyScreenEvent,
};
use super::retro_ui::{configure_visuals, current_palette, RetroScreen};
use super::settings_screen::{
    run_terminal_settings_screen, TerminalSettingsEvent,
};
use super::shell_actions::{
    resolve_login_selection, resolve_main_menu_action, LoginSelectionAction,
    MainMenuSelectionAction,
};
use super::shell_screen::{draw_login_screen, draw_main_menu_screen};
use super::user_management::{
    handle_selection as handle_user_management_selection,
    screen_for_mode as user_management_screen_for_mode, UserManagementAction,
};
use anyhow::{anyhow, Result};
use crate::config::ConnectionKind;
use crate::config::{
    cycle_hacking_difficulty, get_settings, load_apps, load_categories, load_games, load_networks,
    persist_settings, save_apps, save_categories, save_games, save_networks, set_current_user,
    update_settings, CliAcsMode, CliColorMode, DefaultAppBinding, DesktopFileManagerSettings,
    DesktopIconSortMode, DesktopPtyProfileSettings, DesktopIconStyle, DesktopShortcut,
    FileManagerSortMode, FileManagerViewMode,
    OpenMode, Settings, WallpaperSizeMode, CUSTOM_THEME_NAME, THEMES,
};
use crate::connections::{
    connect_connection, discovered_row_label, forget_saved_connection, network_requires_password,
    refresh_discovered_connections, saved_connections, saved_row_label, DiscoveredConnection,
};
use crate::core::auth::{
    ensure_default_admin, load_users, read_session, save_users, AuthMethod, UserRecord,
};
use crate::core::hacking::HackingGame;
use crate::default_apps::{
    binding_label, parse_custom_command_line, set_binding_for_slot, slot_label, DefaultAppSlot,
};
use crate::session;
use chrono::Local;
use eframe::egui::{
    self, Align2, Color32, Context, FontData, FontDefinitions, FontFamily, FontId, Id, Key, Layout,
    Modifiers, RichText, TextEdit, TextStyle, TextureHandle, TopBottomPanel,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

const FILE_MANAGER_OPEN_WITH_HISTORY_LIMIT: usize = 8;
const FILE_MANAGER_OPEN_WITH_NO_EXT_KEY: &str = "__no_ext__";

#[derive(Debug, Clone, Serialize, Deserialize)]
struct NativeShellSnapshot {
    file_manager_dir: PathBuf,
    editor_path: Option<PathBuf>,
}

impl Default for NativeShellSnapshot {
    fn default() -> Self {
        Self {
            file_manager_dir: home_dir_fallback(),
            editor_path: None,
        }
    }
}

#[derive(Debug, Clone)]
struct SessionState {
    username: String,
    is_admin: bool,
}

#[derive(Debug, Default)]
struct LoginState {
    selected_idx: usize,
    selected_username: String,
    password: String,
    error: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LoginScreenMode {
    SelectUser,
    Hacking,
    Locked,
}

#[derive(Debug)]
struct LoginHackingState {
    username: String,
    game: HackingGame,
}

#[derive(Debug, Clone)]
struct EditorWindow {
    open: bool,
    path: Option<PathBuf>,
    text: String,
    dirty: bool,
    status: String,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NativeSettingsPanel {
    Home,
    General,
    Appearance,
    DefaultApps,
    Connections,
    ConnectionsNetwork,
    ConnectionsBluetooth,
    CliProfiles,
    EditMenus,
    UserManagement,
    UserManagementViewUsers,
    UserManagementCreateUser,
    UserManagementEditUsers,
    UserManagementEditCurrentUser,
    About,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GuiCliProfileSlot {
    Default,
    Calcurse,
    SpotifyPlayer,
    Ranger,
    Reddit,
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

struct AssetCache {
    icon_settings: TextureHandle,
    icon_file_manager: TextureHandle,
    icon_terminal: TextureHandle,
    icon_applications: TextureHandle,
    icon_nuke_codes: TextureHandle,
    icon_editor: TextureHandle,
    icon_general: TextureHandle,
    icon_appearance: TextureHandle,
    icon_default_apps: TextureHandle,
    icon_connections: TextureHandle,
    icon_cli_profiles: TextureHandle,
    icon_edit_menus: TextureHandle,
    icon_user_management: TextureHandle,
    icon_about: TextureHandle,
    icon_folder: TextureHandle,
    icon_folder_open: TextureHandle,
    icon_file: TextureHandle,
    icon_text: TextureHandle,
    icon_image: TextureHandle,
    icon_audio: TextureHandle,
    icon_video: TextureHandle,
    icon_archive: TextureHandle,
    icon_app: TextureHandle,
    icon_shortcut_badge: TextureHandle,
    wallpaper: Option<TextureHandle>,
    wallpaper_loaded_for: String,
}

#[derive(Debug, Clone)]
enum FileManagerClipboardMode {
    Copy,
    Cut,
}

#[derive(Debug, Clone)]
struct FileManagerClipboardItem {
    paths: Vec<PathBuf>,
    mode: FileManagerClipboardMode,
}

#[derive(Debug, Clone)]
enum FileManagerEditOp {
    CopyCreated { src: PathBuf, dst: PathBuf },
    Moved { from: PathBuf, to: PathBuf },
}

#[derive(Debug, Clone, Copy)]
enum FileManagerActionRequest {
    Copy,
    Cut,
    Paste,
    Duplicate,
    Delete,
    Undo,
    Redo,
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
    CreateShortcut(String),
    DeleteShortcut(usize),
    SortDesktopIcons(DesktopIconSortMode),
    ToggleSnapToGrid,
    LaunchShortcut(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum DesktopWindow {
    FileManager,
    Editor,
    Settings,
    Applications,
    NukeCodes,
    TerminalMode,
    PtyApp,
}

#[derive(Debug, Clone, Copy, Default)]
struct DesktopWindowState {
    minimized: bool,
    maximized: bool,
    restore_pos: Option<[f32; 2]>,
    restore_size: Option<[f32; 2]>,
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

#[derive(Debug, Clone)]
enum StartLeafAction {
    None,
    LaunchNukeCodes,
    OpenTextEditor,
    LaunchConfiguredApp(String),
    OpenDocumentCategory(PathBuf),
    LaunchNetworkProgram(String),
    LaunchGameProgram(String),
}

#[derive(Debug, Clone)]
struct StartLeafItem {
    label: String,
    action: StartLeafAction,
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
const BUILTIN_TEXT_EDITOR_APP: &str = "ROBCO Word Processor";
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

fn try_load_font_bytes() -> Option<Vec<u8>> {
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
    login: LoginState,
    login_mode: LoginScreenMode,
    login_hacking: Option<LoginHackingState>,
    session: Option<SessionState>,
    file_manager: NativeFileManagerState,
    editor: EditorWindow,
    settings: SettingsWindow,
    applications: ApplicationsWindow,
    desktop_nuke_codes_open: bool,
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
    main_menu_idx: usize,
    terminal_screen: TerminalScreen,
    terminal_apps_idx: usize,
    terminal_documents_idx: usize,
    terminal_logs_idx: usize,
    terminal_network_idx: usize,
    terminal_games_idx: usize,
    terminal_nuke_codes: NukeCodesView,
    terminal_nuke_codes_return: TerminalScreen,
    terminal_pty: Option<NativePtyState>,
    terminal_installer: TerminalInstallerState,
    terminal_settings_idx: usize,
    terminal_edit_menus: TerminalEditMenusState,
    terminal_connections: TerminalConnectionsState,
    terminal_default_apps_idx: usize,
    terminal_default_app_choice_idx: usize,
    terminal_default_app_slot: Option<DefaultAppSlot>,
    terminal_browser_idx: usize,
    terminal_browser_return: TerminalScreen,
    terminal_user_management_idx: usize,
    terminal_user_management_mode: UserManagementMode,
    terminal_settings_choice: Option<SettingsChoiceOverlay>,
    terminal_prompt: Option<TerminalPrompt>,
    suppress_next_menu_submit: bool,
    terminal_flash: Option<TerminalFlash>,
    session_leader_until: Option<Instant>,
    session_runtime: HashMap<usize, ParkedSessionState>,
    desktop_window_generation_seed: u64,
    file_clipboard: Option<FileManagerClipboardItem>,
    file_undo_stack: Vec<FileManagerEditOp>,
    file_redo_stack: Vec<FileManagerEditOp>,
    asset_cache: Option<AssetCache>,
    context_menu_action: Option<ContextMenuAction>,
    shell_status: String,
}

struct ParkedSessionState {
    file_manager: NativeFileManagerState,
    editor: EditorWindow,
    settings: SettingsWindow,
    applications: ApplicationsWindow,
    desktop_nuke_codes_open: bool,
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
    main_menu_idx: usize,
    terminal_screen: TerminalScreen,
    terminal_apps_idx: usize,
    terminal_documents_idx: usize,
    terminal_logs_idx: usize,
    terminal_network_idx: usize,
    terminal_games_idx: usize,
    terminal_nuke_codes: NukeCodesView,
    terminal_nuke_codes_return: TerminalScreen,
    terminal_pty: Option<NativePtyState>,
    terminal_installer: TerminalInstallerState,
    terminal_settings_idx: usize,
    terminal_edit_menus: TerminalEditMenusState,
    terminal_connections: TerminalConnectionsState,
    terminal_default_apps_idx: usize,
    terminal_default_app_choice_idx: usize,
    terminal_default_app_slot: Option<DefaultAppSlot>,
    terminal_browser_idx: usize,
    terminal_browser_return: TerminalScreen,
    terminal_user_management_idx: usize,
    terminal_user_management_mode: UserManagementMode,
    terminal_settings_choice: Option<SettingsChoiceOverlay>,
    terminal_prompt: Option<TerminalPrompt>,
    terminal_flash: Option<TerminalFlash>,
    session_leader_until: Option<Instant>,
    suppress_next_menu_submit: bool,
    desktop_window_generation_seed: u64,
    file_clipboard: Option<FileManagerClipboardItem>,
    file_undo_stack: Vec<FileManagerEditOp>,
    file_redo_stack: Vec<FileManagerEditOp>,
    shell_status: String,
}

impl Default for RobcoNativeApp {
    fn default() -> Self {
        ensure_default_admin();
        // Keep pre-login terminal rendering consistent with the most recent user session.
        if let Some(last_user) = read_session() {
            if load_users().contains_key(&last_user) {
                set_current_user(Some(&last_user));
            }
        }
        session::clear_sessions();
        session::take_switch_request();
        let settings_draft = current_settings();
        Self {
            login: LoginState::default(),
            login_mode: LoginScreenMode::SelectUser,
            login_hacking: None,
            session: None,
            file_manager: NativeFileManagerState::new(home_dir_fallback()),
            editor: EditorWindow {
                open: false,
                path: None,
                text: String::new(),
                dirty: false,
                status: String::new(),
            },
            settings: SettingsWindow {
                open: false,
                draft: settings_draft,
                status: String::new(),
                panel: NativeSettingsPanel::Home,
                default_app_custom_text_code: String::new(),
                default_app_custom_ebook: String::new(),
                scanned_networks: Vec::new(),
                scanned_bluetooth: Vec::new(),
                connection_password: String::new(),
                edit_target: EditMenuTarget::Applications,
                edit_name_input: String::new(),
                edit_value_input: String::new(),
                cli_profile_slot: GuiCliProfileSlot::Default,
                user_selected: String::new(),
                user_selected_loaded_for: String::new(),
                user_create_username: String::new(),
                user_create_auth: AuthMethod::Password,
                user_create_password: String::new(),
                user_create_password_confirm: String::new(),
                user_edit_auth: AuthMethod::Password,
                user_edit_password: String::new(),
                user_edit_password_confirm: String::new(),
                user_delete_confirm: String::new(),
            },
            applications: ApplicationsWindow::default(),
            desktop_nuke_codes_open: false,
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
            main_menu_idx: 0,
            terminal_screen: TerminalScreen::MainMenu,
            terminal_apps_idx: 0,
            terminal_documents_idx: 0,
            terminal_logs_idx: 0,
            terminal_network_idx: 0,
            terminal_games_idx: 0,
            terminal_nuke_codes: NukeCodesView::default(),
            terminal_nuke_codes_return: TerminalScreen::Applications,
            terminal_pty: None,
            terminal_installer: TerminalInstallerState::default(),
            terminal_settings_idx: 0,
            terminal_edit_menus: TerminalEditMenusState::default(),
            terminal_connections: TerminalConnectionsState::default(),
            terminal_default_apps_idx: 0,
            terminal_default_app_choice_idx: 0,
            terminal_default_app_slot: None,
            terminal_browser_idx: 0,
            terminal_browser_return: TerminalScreen::Documents,
            terminal_user_management_idx: 0,
            terminal_user_management_mode: UserManagementMode::Root,
            terminal_settings_choice: None,
            terminal_prompt: None,
            suppress_next_menu_submit: false,
            terminal_flash: None,
            session_leader_until: None,
            session_runtime: HashMap::new(),
            desktop_window_generation_seed: 1,
            file_clipboard: None,
            file_undo_stack: Vec::new(),
            file_redo_stack: Vec::new(),
            asset_cache: None,
            context_menu_action: None,
            shell_status: String::new(),
        }
    }
}

impl RobcoNativeApp {
    fn default_app_custom_value(binding: &DefaultAppBinding) -> String {
        match binding {
            DefaultAppBinding::CustomArgv { argv } => argv.join(" "),
            _ => String::new(),
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

        let mut pixmap = resvg::tiny_skia::Pixmap::new(width, height)
            .expect("zero-sized SVG icon");
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
            let luma = ((pixel[0] as u16 * 77 + pixel[1] as u16 * 150 + pixel[2] as u16 * 29)
                / 256) as u8;
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
            icon_general: Self::load_svg_icon(
                ctx,
                "icon_general",
                include_bytes!("../Icons/pixel--home-solid.svg"),
                Some(ICON_SIZE),
            ),
            icon_appearance: Self::load_svg_icon(
                ctx,
                "icon_appearance",
                include_bytes!("../Icons/pixel--image-solid.svg"),
                Some(ICON_SIZE),
            ),
            icon_default_apps: Self::load_svg_icon(
                ctx,
                "icon_default_apps",
                include_bytes!("../Icons/pixel--external-link-solid.svg"),
                Some(ICON_SIZE),
            ),
            icon_connections: Self::load_svg_icon(
                ctx,
                "icon_connections",
                include_bytes!("../Icons/pixel--globe.svg"),
                Some(ICON_SIZE),
            ),
            icon_cli_profiles: Self::load_svg_icon(
                ctx,
                "icon_cli_profiles",
                include_bytes!("../Icons/pixel--code-solid.svg"),
                Some(ICON_SIZE),
            ),
            icon_edit_menus: Self::load_svg_icon(
                ctx,
                "icon_edit_menus",
                include_bytes!("../Icons/pixel--bullet-list-solid.svg"),
                Some(ICON_SIZE),
            ),
            icon_user_management: Self::load_svg_icon(
                ctx,
                "icon_user_management",
                include_bytes!("../Icons/pixel--user-solid.svg"),
                Some(ICON_SIZE),
            ),
            icon_about: Self::load_svg_icon(
                ctx,
                "icon_about",
                include_bytes!("../Icons/pixel--info-circle-solid.svg"),
                Some(ICON_SIZE),
            ),
            icon_folder: Self::load_svg_icon(
                ctx,
                "icon_folder",
                include_bytes!("../Icons/pixel--folder-solid.svg"),
                Some(ICON_SIZE),
            ),
            icon_folder_open: Self::load_svg_icon(
                ctx,
                "icon_folder_open",
                include_bytes!("../Icons/pixel--folder-open-solid.svg"),
                Some(ICON_SIZE),
            ),
            icon_file: Self::load_svg_icon(
                ctx,
                "icon_file",
                include_bytes!("../Icons/pixel--clipboard-solid.svg"),
                Some(ICON_SIZE),
            ),
            icon_text: Self::load_svg_icon(
                ctx,
                "icon_text",
                include_bytes!("../Icons/pixel--newspaper-solid.svg"),
                Some(ICON_SIZE),
            ),
            icon_image: Self::load_svg_icon(
                ctx,
                "icon_image",
                include_bytes!("../Icons/pixel--image-solid.svg"),
                Some(ICON_SIZE),
            ),
            icon_audio: Self::load_svg_icon(
                ctx,
                "icon_audio",
                include_bytes!("../Icons/pixel--music-solid.svg"),
                Some(ICON_SIZE),
            ),
            icon_video: Self::load_svg_icon(
                ctx,
                "icon_video",
                include_bytes!("../Icons/pixel--media.svg"),
                Some(ICON_SIZE),
            ),
            icon_archive: Self::load_svg_icon(
                ctx,
                "icon_archive",
                include_bytes!("../Icons/pixel--save-solid.svg"),
                Some(ICON_SIZE),
            ),
            icon_app: Self::load_svg_icon(
                ctx,
                "icon_app",
                include_bytes!("../Icons/pixel--programming.svg"),
                Some(ICON_SIZE),
            ),
            icon_shortcut_badge: Self::load_svg_icon(
                ctx,
                "icon_shortcut_badge",
                include_bytes!("../Icons/pixel--external-link-solid.svg"),
                Some(16),
            ),
            wallpaper: None,
            wallpaper_loaded_for: String::new(),
        }
    }

    fn sync_wallpaper(&mut self, ctx: &Context) {
        let wallpaper_path = self.settings.draft.desktop_wallpaper.clone();
        if let Some(cache) = &mut self.asset_cache {
            if cache.wallpaper_loaded_for != wallpaper_path {
                cache.wallpaper = Self::load_wallpaper_texture(ctx, &wallpaper_path);
                cache.wallpaper_loaded_for = wallpaper_path;
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

    fn draw_wallpaper(&self, painter: &egui::Painter, screen: egui::Rect) -> bool {
        let Some(cache) = &self.asset_cache else {
            return false;
        };
        let Some(texture) = &cache.wallpaper else {
            return false;
        };

        let image_size = egui::vec2(texture.size()[0] as f32, texture.size()[1] as f32);
        let uv = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
        let tint = current_palette().fg;
        match self.settings.draft.desktop_wallpaper_size_mode {
            WallpaperSizeMode::FitToScreen | WallpaperSizeMode::Stretch => {
                painter.image(texture.id(), screen, uv, tint);
            }
            WallpaperSizeMode::Centered => {
                painter.rect_filled(screen, 0.0, current_palette().bg);
                let origin = screen.center() - image_size * 0.5;
                painter.image(
                    texture.id(),
                    egui::Rect::from_min_size(origin, image_size),
                    uv,
                    tint,
                );
            }
            WallpaperSizeMode::DefaultSize => {
                painter.rect_filled(screen, 0.0, current_palette().bg);
                painter.image(
                    texture.id(),
                    egui::Rect::from_min_size(screen.min, image_size),
                    uv,
                    tint,
                );
            }
            WallpaperSizeMode::Tile => {
                painter.rect_filled(screen, 0.0, current_palette().bg);
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
        self.settings.draft = current_settings();
        self.settings.status.clear();
        self.settings.panel = NativeSettingsPanel::Home;
        self.settings.default_app_custom_text_code =
            Self::default_app_custom_value(&self.settings.draft.default_apps.text_code);
        self.settings.default_app_custom_ebook =
            Self::default_app_custom_value(&self.settings.draft.default_apps.ebook);
        self.settings.scanned_networks.clear();
        self.settings.scanned_bluetooth.clear();
        self.settings.connection_password.clear();
        self.settings.edit_target = EditMenuTarget::Applications;
        self.settings.edit_name_input.clear();
        self.settings.edit_value_input.clear();
        self.settings.cli_profile_slot = GuiCliProfileSlot::Default;
        self.settings.user_create_username.clear();
        self.settings.user_create_auth = AuthMethod::Password;
        self.settings.user_create_password.clear();
        self.settings.user_create_password_confirm.clear();
        self.settings.user_edit_password.clear();
        self.settings.user_edit_password_confirm.clear();
        self.settings.user_delete_confirm.clear();
        let mut users: Vec<String> = load_users().keys().cloned().collect();
        users.sort();
        self.settings.user_selected = self
            .session
            .as_ref()
            .map(|s| s.username.clone())
            .filter(|name| users.iter().any(|user| user == name))
            .or_else(|| users.first().cloned())
            .unwrap_or_default();
        self.settings.user_selected_loaded_for.clear();
        self.settings.user_edit_auth = AuthMethod::Password;
    }

    fn gui_cli_profile_slot_label(slot: GuiCliProfileSlot) -> &'static str {
        match slot {
            GuiCliProfileSlot::Default => "Default",
            GuiCliProfileSlot::Calcurse => "Calcurse",
            GuiCliProfileSlot::SpotifyPlayer => "Spotify Player",
            GuiCliProfileSlot::Ranger => "Ranger",
            GuiCliProfileSlot::Reddit => "Reddit",
        }
    }

    fn gui_cli_profile_mut(
        profiles: &mut crate::config::DesktopCliProfiles,
        slot: GuiCliProfileSlot,
    ) -> &mut DesktopPtyProfileSettings {
        match slot {
            GuiCliProfileSlot::Default => &mut profiles.default,
            GuiCliProfileSlot::Calcurse => &mut profiles.calcurse,
            GuiCliProfileSlot::SpotifyPlayer => &mut profiles.spotify_player,
            GuiCliProfileSlot::Ranger => &mut profiles.ranger,
            GuiCliProfileSlot::Reddit => &mut profiles.reddit,
        }
    }

    fn apply_global_retro_menu_chrome(ctx: &Context) {
        let palette = current_palette();
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
        let count = session::session_count();
        if target < count || (target == count && count < session::MAX_SESSIONS) {
            session::request_switch(target);
            return true;
        }
        false
    }

    fn ensure_login_session_entry(&mut self, username: &str) {
        let existing = session::get_sessions()
            .iter()
            .position(|entry| entry.username == username);
        let idx = existing.unwrap_or_else(|| session::push_session(username));
        session::set_active(idx);
    }

    fn park_active_session_runtime(&mut self) {
        if self.session.is_none() || session::session_count() == 0 {
            return;
        }
        let idx = session::active_idx();
        let parked = ParkedSessionState {
            file_manager: self.file_manager.clone(),
            editor: self.editor.clone(),
            settings: self.settings.clone(),
            applications: self.applications.clone(),
            desktop_nuke_codes_open: self.desktop_nuke_codes_open,
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
            main_menu_idx: self.main_menu_idx,
            terminal_screen: self.terminal_screen,
            terminal_apps_idx: self.terminal_apps_idx,
            terminal_documents_idx: self.terminal_documents_idx,
            terminal_logs_idx: self.terminal_logs_idx,
            terminal_network_idx: self.terminal_network_idx,
            terminal_games_idx: self.terminal_games_idx,
            terminal_nuke_codes: self.terminal_nuke_codes.clone(),
            terminal_nuke_codes_return: self.terminal_nuke_codes_return,
            terminal_pty: self.terminal_pty.take(),
            terminal_installer: std::mem::take(&mut self.terminal_installer),
            terminal_settings_idx: self.terminal_settings_idx,
            terminal_edit_menus: std::mem::take(&mut self.terminal_edit_menus),
            terminal_connections: std::mem::take(&mut self.terminal_connections),
            terminal_default_apps_idx: self.terminal_default_apps_idx,
            terminal_default_app_choice_idx: self.terminal_default_app_choice_idx,
            terminal_default_app_slot: self.terminal_default_app_slot.take(),
            terminal_browser_idx: self.terminal_browser_idx,
            terminal_browser_return: self.terminal_browser_return,
            terminal_user_management_idx: self.terminal_user_management_idx,
            terminal_user_management_mode: self.terminal_user_management_mode.clone(),
            terminal_settings_choice: self.terminal_settings_choice.take(),
            terminal_prompt: self.terminal_prompt.take(),
            terminal_flash: self.terminal_flash.take(),
            session_leader_until: self.session_leader_until.take(),
            suppress_next_menu_submit: self.suppress_next_menu_submit,
            desktop_window_generation_seed: self.desktop_window_generation_seed,
            file_clipboard: self.file_clipboard.clone(),
            file_undo_stack: self.file_undo_stack.clone(),
            file_redo_stack: self.file_redo_stack.clone(),
            shell_status: std::mem::take(&mut self.shell_status),
        };
        self.session_runtime.insert(idx, parked);
    }

    fn sync_active_session_identity(&mut self) -> bool {
        let Some(username) = session::active_username() else {
            self.session = None;
            return false;
        };
        let users = load_users();
        let Some(user) = users.get(&username) else {
            self.session = None;
            self.shell_status = format!("Unknown user '{username}'.");
            return false;
        };
        bind_login_session(&username);
        self.session = Some(SessionState {
            username,
            is_admin: user.is_admin,
        });
        true
    }

    fn restore_active_session_runtime_if_any(&mut self) -> bool {
        let idx = session::active_idx();
        let Some(parked) = self.session_runtime.remove(&idx) else {
            return false;
        };
        self.file_manager = parked.file_manager;
        self.editor = parked.editor;
        self.settings = parked.settings;
        self.applications = parked.applications;
        self.desktop_nuke_codes_open = parked.desktop_nuke_codes_open;
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
        self.main_menu_idx = parked.main_menu_idx;
        self.terminal_screen = parked.terminal_screen;
        self.terminal_apps_idx = parked.terminal_apps_idx;
        self.terminal_documents_idx = parked.terminal_documents_idx;
        self.terminal_logs_idx = parked.terminal_logs_idx;
        self.terminal_network_idx = parked.terminal_network_idx;
        self.terminal_games_idx = parked.terminal_games_idx;
        self.terminal_nuke_codes = parked.terminal_nuke_codes;
        self.terminal_nuke_codes_return = parked.terminal_nuke_codes_return;
        self.terminal_pty = parked.terminal_pty;
        self.terminal_installer = parked.terminal_installer;
        self.terminal_settings_idx = parked.terminal_settings_idx;
        self.terminal_edit_menus = parked.terminal_edit_menus;
        self.terminal_connections = parked.terminal_connections;
        self.terminal_default_apps_idx = parked.terminal_default_apps_idx;
        self.terminal_default_app_choice_idx = parked.terminal_default_app_choice_idx;
        self.terminal_default_app_slot = parked.terminal_default_app_slot;
        self.terminal_browser_idx = parked.terminal_browser_idx;
        self.terminal_browser_return = parked.terminal_browser_return;
        self.terminal_user_management_idx = parked.terminal_user_management_idx;
        self.terminal_user_management_mode = parked.terminal_user_management_mode;
        self.terminal_settings_choice = parked.terminal_settings_choice;
        self.terminal_prompt = parked.terminal_prompt;
        self.terminal_flash = parked.terminal_flash;
        self.session_leader_until = parked.session_leader_until;
        self.suppress_next_menu_submit = parked.suppress_next_menu_submit;
        self.desktop_window_generation_seed = parked.desktop_window_generation_seed;
        self.file_clipboard = parked.file_clipboard;
        self.file_undo_stack = parked.file_undo_stack;
        self.file_redo_stack = parked.file_redo_stack;
        self.context_menu_action = None;
        self.shell_status = parked.shell_status;
        true
    }

    fn activate_session_user(&mut self, username: &str) {
        let users = load_users();
        if let Some(user) = users.get(username).cloned() {
            bind_login_session(username);
            self.restore_for_user(username, &user);
        } else {
            self.shell_status = format!("Unknown user '{username}'.");
        }
    }

    fn apply_pending_session_switch(&mut self) {
        let Some(target) = session::take_switch_request() else {
            return;
        };

        let count = session::session_count();
        if target < count {
            let current = session::active_idx();
            if target == current {
                return;
            }
            self.persist_snapshot();
            self.park_active_session_runtime();
            session::set_active(target);
            if !self.sync_active_session_identity() {
                return;
            }
            if self.restore_active_session_runtime_if_any() {
                return;
            }
            if let Some(username) = session::active_username() {
                self.activate_session_user(&username);
            }
            return;
        }

        if target == count && count < session::MAX_SESSIONS {
            if let Some(username) = session::active_username() {
                self.persist_snapshot();
                self.park_active_session_runtime();
                let idx = session::push_session_with_default_mode(&username, false);
                session::set_active(idx);
                self.activate_session_user(&username);
                self.shell_status = format!("Switched to session {}.", idx + 1);
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
        let count = session::session_count();
        if count == 0 {
            return;
        }
        if count <= 1 {
            self.shell_status = "Cannot close the last session.".to_string();
            return;
        }

        self.persist_snapshot();
        let closing_idx = session::active_idx();

        if let Some(mut pty) = self.terminal_pty.take() {
            pty.session.terminate();
        }
        if let Some(mut parked) = self.session_runtime.remove(&closing_idx) {
            if let Some(mut pty) = parked.terminal_pty.take() {
                pty.session.terminate();
            }
        }

        let Some(removed_idx) = session::close_active_session() else {
            return;
        };

        // Session indexes are contiguous; shift parked state keys down after removal.
        let mut remapped = HashMap::new();
        for (idx, parked) in self.session_runtime.drain() {
            let new_idx = if idx > removed_idx { idx - 1 } else { idx };
            remapped.insert(new_idx, parked);
        }
        self.session_runtime = remapped;

        if !self.sync_active_session_identity() {
            return;
        }
        if !self.restore_active_session_runtime_if_any() {
            if let Some(username) = session::active_username() {
                self.activate_session_user(&username);
            }
        }
        self.shell_status = format!("Closed session {}.", removed_idx + 1);
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

    fn desktop_window_is_open(&self, window: DesktopWindow) -> bool {
        match window {
            DesktopWindow::FileManager => self.file_manager.open,
            DesktopWindow::Editor => self.editor.open,
            DesktopWindow::Settings => self.settings.open,
            DesktopWindow::Applications => self.applications.open,
            DesktopWindow::NukeCodes => self.desktop_nuke_codes_open,
            DesktopWindow::TerminalMode => self.terminal_mode.open,
            DesktopWindow::PtyApp => self.terminal_pty.is_some(),
        }
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
        match window {
            DesktopWindow::FileManager => egui::vec2(700.0, 480.0),
            DesktopWindow::Editor => egui::vec2(820.0, 560.0),
            DesktopWindow::Settings => egui::vec2(760.0, 500.0),
            DesktopWindow::Applications => egui::vec2(700.0, 480.0),
            DesktopWindow::NukeCodes => egui::vec2(640.0, 420.0),
            DesktopWindow::TerminalMode => egui::vec2(720.0, 500.0),
            DesktopWindow::PtyApp => egui::vec2(960.0, 600.0),
        }
    }

    fn desktop_default_window_pos(
        ctx: &Context,
        size: egui::Vec2,
    ) -> egui::Pos2 {
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

    fn desktop_clamp_window_pos(
        ctx: &Context,
        pos: egui::Pos2,
        size: egui::Vec2,
    ) -> egui::Pos2 {
        let workspace = Self::desktop_workspace_rect(ctx);
        egui::pos2(
            pos.x.clamp(workspace.left(), (workspace.right() - size.x).max(workspace.left())),
            pos.y.clamp(workspace.top(), (workspace.bottom() - size.y).max(workspace.top())),
        )
    }

    fn prime_desktop_window_defaults(&mut self, window: DesktopWindow) {
        let generation = self.next_desktop_window_generation();
        let state = self.desktop_window_state_mut(window);
        state.restore_pos = None;
        state.restore_size = None;
        state.apply_restore = false;
        state.maximized = false;
        state.minimized = false;
        state.generation = generation;
    }

    fn set_desktop_window_open(&mut self, window: DesktopWindow, open: bool) {
        let was_open = self.desktop_window_is_open(window);
        match window {
            DesktopWindow::FileManager => self.file_manager.open = open,
            DesktopWindow::Editor => self.editor.open = open,
            DesktopWindow::Settings => self.settings.open = open,
            DesktopWindow::Applications => self.applications.open = open,
            DesktopWindow::NukeCodes => self.desktop_nuke_codes_open = open,
            DesktopWindow::TerminalMode => self.terminal_mode.open = open,
            DesktopWindow::PtyApp => {
                if !open {
                    if let Some(mut pty) = self.terminal_pty.take() {
                        pty.session.terminate();
                    }
                }
            }
        }
        if !open {
            self.desktop_window_states.remove(&window);
        } else if !was_open && self.desktop_window_is_open(window) {
            let generation = self.next_desktop_window_generation();
            let state = self.desktop_window_state_mut(window);
            state.minimized = false;
            state.maximized = false;
            state.generation = generation;
        } else {
            self.desktop_window_states.entry(window).or_default();
        }
    }

    fn first_open_desktop_window(&self) -> Option<DesktopWindow> {
        const ORDER: [DesktopWindow; 7] = [
            DesktopWindow::FileManager,
            DesktopWindow::Editor,
            DesktopWindow::Settings,
            DesktopWindow::Applications,
            DesktopWindow::NukeCodes,
            DesktopWindow::TerminalMode,
            DesktopWindow::PtyApp,
        ];
        ORDER
            .into_iter()
            .find(|window| self.desktop_window_is_open(*window) && !self.desktop_window_is_minimized(*window))
    }

    fn sync_desktop_active_window(&mut self) {
        if self
            .desktop_active_window
            .is_some_and(|window| {
                !self.desktop_window_is_open(window) || self.desktop_window_is_minimized(window)
            })
        {
            self.desktop_active_window = self.first_open_desktop_window();
            return;
        }
        if self.desktop_active_window.is_none() {
            self.desktop_active_window = self.first_open_desktop_window();
        }
    }

    fn open_desktop_window(&mut self, window: DesktopWindow) {
        if matches!(window, DesktopWindow::Settings) {
            self.reset_desktop_settings_window();
            self.prime_desktop_window_defaults(window);
        } else if !self.desktop_window_is_open(window)
            && matches!(window, DesktopWindow::TerminalMode | DesktopWindow::PtyApp)
        {
            self.prime_desktop_window_defaults(window);
        }
        self.set_desktop_window_open(window, true);
        self.set_desktop_window_minimized(window, false);
        self.desktop_active_window = Some(window);
        if self.desktop_mode_open {
            self.close_start_menu();
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
            self.desktop_active_window = Some(window);
        }
    }

    fn close_desktop_window(&mut self, window: DesktopWindow) {
        self.set_desktop_window_open(window, false);
        if self.desktop_active_window == Some(window) {
            self.desktop_active_window = self.first_open_desktop_window();
        }
    }

    fn update_desktop_window_state(&mut self, window: DesktopWindow, open: bool) {
        self.set_desktop_window_open(window, open);
        if !open && self.desktop_active_window == Some(window) {
            self.desktop_active_window = self.first_open_desktop_window();
        }
    }

    fn handle_desktop_taskbar_window_click(&mut self, window: DesktopWindow) {
        if !self.desktop_window_is_open(window) {
            self.open_desktop_window(window);
            return;
        }
        if self.desktop_window_is_minimized(window) {
            self.set_desktop_window_minimized(window, false);
            self.close_start_menu();
        } else {
            self.desktop_active_window = Some(window);
            self.close_start_menu();
        }
    }

    fn desktop_taskbar_label(&self, window: DesktopWindow) -> String {
        self.desktop_window_title(window)
    }

    fn desktop_window_title(&self, window: DesktopWindow) -> String {
        match window {
            DesktopWindow::FileManager => "My Computer".to_string(),
            DesktopWindow::Editor => "ROBCO Word Processor".to_string(),
            DesktopWindow::Settings => "Settings".to_string(),
            DesktopWindow::Applications => "Applications".to_string(),
            DesktopWindow::NukeCodes => "Nuke Codes".to_string(),
            DesktopWindow::TerminalMode => "Terminal".to_string(),
            DesktopWindow::PtyApp => self
                .terminal_pty
                .as_ref()
                .map(|pty| pty.title.clone())
                .unwrap_or_else(|| "PTY App".to_string()),
        }
    }

    fn desktop_app_menu_name(&self) -> String {
        self.desktop_active_window
            .map(|w| self.desktop_window_title(w))
            .unwrap_or_else(|| "Desktop".to_string())
    }

    fn file_manager_view_mode(&self) -> FileManagerViewMode {
        get_settings().desktop_file_manager.view_mode
    }

    fn file_manager_sort_mode(&self) -> FileManagerSortMode {
        get_settings().desktop_file_manager.sort_mode
    }

    fn toggle_file_manager_tree_panel(&mut self) {
        update_settings(|s| {
            s.desktop_file_manager.show_tree_panel = !s.desktop_file_manager.show_tree_panel;
        });
        persist_settings();
        self.file_manager.ensure_selection_valid();
    }

    fn toggle_file_manager_hidden_files(&mut self) {
        update_settings(|s| {
            s.desktop_file_manager.show_hidden_files = !s.desktop_file_manager.show_hidden_files;
        });
        persist_settings();
        self.file_manager.ensure_selection_valid();
    }

    fn set_file_manager_view_mode(&mut self, mode: FileManagerViewMode) {
        update_settings(|s| {
            s.desktop_file_manager.view_mode = mode;
        });
        persist_settings();
        self.file_manager.ensure_selection_valid();
    }

    fn set_file_manager_sort_mode(&mut self, mode: FileManagerSortMode) {
        update_settings(|s| {
            s.desktop_file_manager.sort_mode = mode;
        });
        persist_settings();
        self.file_manager.ensure_selection_valid();
    }

    fn file_manager_open_home(&mut self) {
        if let Some(session) = &self.session {
            self.file_manager.set_cwd(word_processor_dir(&session.username));
        } else {
            self.file_manager.set_cwd(home_dir_fallback());
        }
    }

    fn truncate_file_manager_label(text: &str, max_chars: usize) -> String {
        if text.chars().count() <= max_chars {
            return text.to_string();
        }
        if max_chars <= 3 {
            return ".".repeat(max_chars);
        }
        let mut out: String = text.chars().take(max_chars - 3).collect();
        out.push_str("...");
        out
    }

    fn settings_panel_texture(&self, panel: NativeSettingsPanel) -> Option<&TextureHandle> {
        let cache = self.asset_cache.as_ref()?;
        Some(match panel {
            NativeSettingsPanel::General => &cache.icon_general,
            NativeSettingsPanel::Appearance => &cache.icon_appearance,
            NativeSettingsPanel::DefaultApps => &cache.icon_default_apps,
            NativeSettingsPanel::Connections => &cache.icon_connections,
            NativeSettingsPanel::CliProfiles => &cache.icon_cli_profiles,
            NativeSettingsPanel::EditMenus => &cache.icon_edit_menus,
            NativeSettingsPanel::UserManagement => &cache.icon_user_management,
            NativeSettingsPanel::About => &cache.icon_about,
            _ => return None,
        })
    }

    fn file_manager_texture_for_row(&self, row: &super::file_manager::FileEntryRow) -> Option<&TextureHandle> {
        let cache = self.asset_cache.as_ref()?;
        if row.is_parent_dir() {
            return Some(&cache.icon_folder_open);
        }
        if row.is_dir {
            return Some(&cache.icon_folder);
        }
        let extension = row
            .path
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        Some(match extension.as_str() {
            "txt" | "md" | "log" | "toml" | "yaml" | "yml" | "json" | "cfg" | "ini"
            | "conf" | "ron" | "rs" | "py" | "js" | "ts" | "c" | "cpp" | "h" | "hpp"
            | "sh" | "bash" | "fish" | "lua" | "rb" => &cache.icon_text,
            "png" | "jpg" | "jpeg" | "gif" | "bmp" | "webp" | "svg" | "ico" => &cache.icon_image,
            "mp3" | "wav" | "ogg" | "flac" | "aac" | "m4a" => &cache.icon_audio,
            "mp4" | "mkv" | "avi" | "mov" | "webm" => &cache.icon_video,
            "zip" | "tar" | "gz" | "bz2" | "xz" | "7z" | "rar" => &cache.icon_archive,
            "exe" | "bin" | "appimage" | "dmg" | "deb" | "rpm" | "app" | "bat" | "cmd" => {
                &cache.icon_app
            }
            _ => &cache.icon_file,
        })
    }

    fn file_manager_selected_entry(&self) -> Option<super::file_manager::FileEntryRow> {
        self.file_manager.selected_row()
    }

    fn split_file_name(name: &str) -> (&str, &str) {
        if let Some((stem, _ext)) = name.rsplit_once('.') {
            if !stem.is_empty() {
                return (stem, &name[stem.len()..]);
            }
        }
        (name, "")
    }

    fn path_display_name(path: &Path) -> String {
        path.file_name()
            .and_then(|name| name.to_str())
            .map(|name| name.to_string())
            .unwrap_or_else(|| path.display().to_string())
    }

    fn open_with_extension_key(path: &Path) -> String {
        path.extension()
            .and_then(|s| s.to_str())
            .map(|s| s.trim().to_ascii_lowercase())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| FILE_MANAGER_OPEN_WITH_NO_EXT_KEY.to_string())
    }

    fn open_with_extension_label(ext_key: &str) -> String {
        if ext_key == FILE_MANAGER_OPEN_WITH_NO_EXT_KEY {
            "(no extension)".to_string()
        } else {
            format!(".{ext_key}")
        }
    }

    fn push_open_with_history(history: &mut Vec<String>, command: &str) {
        let normalized = command.trim();
        if normalized.is_empty() {
            return;
        }
        history.retain(|entry| entry.trim() != normalized);
        history.insert(0, normalized.to_string());
        if history.len() > FILE_MANAGER_OPEN_WITH_HISTORY_LIMIT {
            history.truncate(FILE_MANAGER_OPEN_WITH_HISTORY_LIMIT);
        }
    }

    fn open_with_history_for_extension(&self, ext_key: &str) -> Vec<String> {
        get_settings()
            .desktop_file_manager
            .open_with_by_extension
            .get(ext_key)
            .cloned()
            .unwrap_or_default()
    }

    fn open_with_default_for_extension(&self, ext_key: &str) -> Option<String> {
        get_settings()
            .desktop_file_manager
            .open_with_default_by_extension
            .get(ext_key)
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
    }

    fn sync_open_with_settings_to_draft(&mut self) {
        let live = get_settings();
        self.settings.draft.desktop_file_manager.open_with_by_extension =
            live.desktop_file_manager.open_with_by_extension.clone();
        self.settings.draft.desktop_file_manager.open_with_default_by_extension =
            live.desktop_file_manager.open_with_default_by_extension.clone();
    }

    fn set_open_with_default_in_settings(
        fm: &mut DesktopFileManagerSettings,
        ext_key: &str,
        command: Option<&str>,
    ) {
        match command.map(str::trim).filter(|value| !value.is_empty()) {
            Some(normalized) => {
                let history = fm
                    .open_with_by_extension
                    .entry(ext_key.to_string())
                    .or_default();
                Self::push_open_with_history(history, normalized);
                fm.open_with_default_by_extension
                    .insert(ext_key.to_string(), normalized.to_string());
            }
            None => {
                fm.open_with_default_by_extension.remove(ext_key);
            }
        }
    }

    fn record_open_with_command(&mut self, ext_key: &str, command: &str) {
        let normalized = command.trim();
        if normalized.is_empty() {
            return;
        }
        update_settings(|settings| {
            let history = settings
                .desktop_file_manager
                .open_with_by_extension
                .entry(ext_key.to_string())
                .or_default();
            Self::push_open_with_history(history, normalized);
        });
        persist_settings();
        self.sync_open_with_settings_to_draft();
    }

    fn set_open_with_default_command(&mut self, ext_key: &str, command: Option<&str>) {
        update_settings(|settings| {
            Self::set_open_with_default_in_settings(
                &mut settings.desktop_file_manager,
                ext_key,
                command,
            );
        });
        persist_settings();
        self.sync_open_with_settings_to_draft();
    }

    fn replace_open_with_command_in_settings(
        fm: &mut DesktopFileManagerSettings,
        ext_key: &str,
        old_normalized: &str,
        new_normalized: &str,
    ) {
        let was_default = fm
            .open_with_default_by_extension
            .get(ext_key)
            .is_some_and(|current| current.trim() == old_normalized);

        let remove_bucket = {
            let history = fm
                .open_with_by_extension
                .entry(ext_key.to_string())
                .or_default();
            history.retain(|entry| entry.trim() != old_normalized);
            Self::push_open_with_history(history, new_normalized);
            history.is_empty()
        };
        if remove_bucket {
            fm.open_with_by_extension.remove(ext_key);
        }

        if was_default {
            fm.open_with_default_by_extension
                .insert(ext_key.to_string(), new_normalized.to_string());
        }
    }

    fn replace_open_with_command(
        &mut self,
        ext_key: &str,
        old_command: &str,
        new_command: &str,
    ) {
        let old_normalized = old_command.trim();
        let new_normalized = new_command.trim();
        if old_normalized.is_empty() || new_normalized.is_empty() {
            return;
        }
        update_settings(|settings| {
            Self::replace_open_with_command_in_settings(
                &mut settings.desktop_file_manager,
                ext_key,
                old_normalized,
                new_normalized,
            );
        });
        persist_settings();
        self.sync_open_with_settings_to_draft();
    }

    fn remove_open_with_command_in_settings(
        fm: &mut DesktopFileManagerSettings,
        ext_key: &str,
        normalized: &str,
    ) {
        let mut remove_bucket = false;
        if let Some(history) = fm.open_with_by_extension.get_mut(ext_key) {
            history.retain(|entry| entry.trim() != normalized);
            remove_bucket = history.is_empty();
        }
        if remove_bucket {
            fm.open_with_by_extension.remove(ext_key);
        }
        if fm
            .open_with_default_by_extension
            .get(ext_key)
            .is_some_and(|current| current.trim() == normalized)
        {
            fm.open_with_default_by_extension.remove(ext_key);
        }
    }

    fn remove_open_with_command(&mut self, ext_key: &str, command: &str) {
        let normalized = command.trim();
        if normalized.is_empty() {
            return;
        }
        update_settings(|settings| {
            Self::remove_open_with_command_in_settings(
                &mut settings.desktop_file_manager,
                ext_key,
                normalized,
            );
        });
        persist_settings();
        self.sync_open_with_settings_to_draft();
    }

    fn open_with_command_title(program: &str) -> String {
        let name = Path::new(program)
            .file_name()
            .and_then(|s| s.to_str())
            .filter(|s| !s.is_empty())
            .unwrap_or(program);
        if name.eq_ignore_ascii_case("spotify_player") {
            "spotify".to_string()
        } else {
            name.to_string()
        }
    }

    fn launch_open_with_command(&mut self, path: &Path, command_line: &str) -> Result<String> {
        let normalized = command_line.trim();
        let Some(mut cmd) = parse_custom_command_line(normalized) else {
            return Err(anyhow!("Invalid command line: {normalized}"));
        };
        let program = cmd.first().cloned().unwrap_or_default();
        if !program.is_empty() && !crate::launcher::command_exists(&program) {
            return Err(anyhow!("Command `{program}` was not found in PATH."));
        }
        cmd.push(path.display().to_string());
        let title = format!(
            "{} - {}",
            Self::open_with_command_title(&cmd[0]),
            Self::path_display_name(path)
        );
        self.open_desktop_pty(&title, &cmd);
        Ok(format!("Opened {} in PTY", Self::path_display_name(path)))
    }

    fn file_manager_selected_file(&self) -> Option<super::file_manager::FileEntryRow> {
        let entry = self.file_manager_selected_entry()?;
        if entry.path.is_file() {
            Some(entry)
        } else {
            None
        }
    }

    fn unique_copy_path_in_dir(dir: &Path, original_name: &str, prefer_copy_suffix: bool) -> PathBuf {
        let direct = dir.join(original_name);
        if !prefer_copy_suffix && !direct.exists() {
            return direct;
        }
        let (stem, ext) = Self::split_file_name(original_name);
        for index in 1..=9999usize {
            let candidate = if index == 1 {
                format!("{stem} copy{ext}")
            } else {
                format!("{stem} copy {index}{ext}")
            };
            let path = dir.join(candidate);
            if !path.exists() {
                return path;
            }
        }
        direct
    }

    fn unique_path_in_dir(dir: &Path, original_name: &str) -> PathBuf {
        let direct = dir.join(original_name);
        if !direct.exists() {
            return direct;
        }
        for index in 1..=9999usize {
            let candidate = dir.join(format!("{original_name}.{index}"));
            if !candidate.exists() {
                return candidate;
            }
        }
        direct
    }

    fn copy_path_recursive(src: &Path, dst: &Path) -> Result<()> {
        let meta = std::fs::metadata(src)
            .map_err(|e| anyhow!("Failed reading {}: {e}", src.display()))?;
        if meta.is_dir() {
            std::fs::create_dir_all(dst)
                .map_err(|e| anyhow!("Failed creating {}: {e}", dst.display()))?;
            for item in std::fs::read_dir(src)
                .map_err(|e| anyhow!("Failed listing {}: {e}", src.display()))?
            {
                let item =
                    item.map_err(|e| anyhow!("Failed reading {} entry: {e}", src.display()))?;
                let child_src = item.path();
                let child_dst = dst.join(item.file_name());
                Self::copy_path_recursive(&child_src, &child_dst)?;
            }
        } else {
            if let Some(parent) = dst.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| anyhow!("Failed creating {}: {e}", parent.display()))?;
            }
            std::fs::copy(src, dst).map_err(|e| {
                anyhow!("Failed copying {} -> {}: {e}", src.display(), dst.display())
            })?;
        }
        Ok(())
    }

    fn remove_path_recursive(path: &Path) -> Result<()> {
        if !path.exists() {
            return Ok(());
        }
        if path.is_dir() {
            std::fs::remove_dir_all(path)
                .map_err(|e| anyhow!("Failed deleting {}: {e}", path.display()))?;
        } else {
            std::fs::remove_file(path)
                .map_err(|e| anyhow!("Failed deleting {}: {e}", path.display()))?;
        }
        Ok(())
    }

    fn move_path(src: &Path, dst: &Path) -> Result<()> {
        if src == dst {
            return Ok(());
        }
        if dst.exists() {
            return Err(anyhow!("Destination already exists: {}", dst.display()));
        }
        if let Some(parent) = dst.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| anyhow!("Failed creating {}: {e}", parent.display()))?;
        }
        match std::fs::rename(src, dst) {
            Ok(_) => Ok(()),
            Err(_) => {
                Self::copy_path_recursive(src, dst)?;
                Self::remove_path_recursive(src)
            }
        }
    }

    fn record_file_manager_edit_op(&mut self, op: FileManagerEditOp) {
        self.file_undo_stack.push(op);
        self.file_redo_stack.clear();
        if self.file_undo_stack.len() > 100 {
            let overflow = self.file_undo_stack.len().saturating_sub(100);
            self.file_undo_stack.drain(0..overflow);
        }
    }

    fn apply_file_manager_edit_op(op: &FileManagerEditOp, reverse: bool) -> Result<()> {
        match op {
            FileManagerEditOp::CopyCreated { src, dst } => {
                if reverse {
                    Self::remove_path_recursive(dst)
                } else {
                    Self::copy_path_recursive(src, dst)
                }
            }
            FileManagerEditOp::Moved { from, to } => {
                if reverse {
                    Self::move_path(to, from)
                } else {
                    Self::move_path(from, to)
                }
            }
        }
    }

    fn file_manager_set_clipboard_from_selected(
        &mut self,
        mode: FileManagerClipboardMode,
    ) -> Result<String> {
        let Some(entry) = self.file_manager_selected_entry() else {
            return Err(anyhow!("Select a file or folder first."));
        };
        self.file_clipboard = Some(FileManagerClipboardItem {
            paths: vec![entry.path.clone()],
            mode: mode.clone(),
        });
        Ok(match mode {
            FileManagerClipboardMode::Copy => format!("Copied {}", entry.label),
            FileManagerClipboardMode::Cut => format!("Cut {}", entry.label),
        })
    }

    fn file_manager_duplicate_selected(&mut self) -> Result<String> {
        let Some(entry) = self.file_manager_selected_entry() else {
            return Err(anyhow!("Select a file or folder first."));
        };
        let Some(parent) = entry.path.parent() else {
            return Err(anyhow!("Cannot duplicate this item."));
        };
        let name = Self::path_display_name(&entry.path);
        let dst = Self::unique_copy_path_in_dir(parent, &name, true);
        Self::copy_path_recursive(&entry.path, &dst)?;
        self.record_file_manager_edit_op(FileManagerEditOp::CopyCreated {
            src: entry.path,
            dst: dst.clone(),
        });
        self.file_manager.select(Some(dst.clone()));
        Ok(format!("Duplicated as {}", Self::path_display_name(&dst)))
    }

    fn file_manager_rename_selected(&mut self, new_name: String) -> Result<String> {
        let Some(entry) = self.file_manager_selected_entry() else {
            return Err(anyhow!("Select a file or folder first."));
        };
        let Some(parent) = entry.path.parent() else {
            return Err(anyhow!("Cannot rename this item."));
        };
        let name = new_name.trim();
        if name.is_empty() {
            return Err(anyhow!("Name cannot be empty."));
        }
        if name.contains('/') || name.contains('\\') {
            return Err(anyhow!("Name cannot contain path separators."));
        }
        if name == entry.label {
            return Ok("Name unchanged.".to_string());
        }
        let dst = parent.join(name);
        if dst.exists() {
            return Err(anyhow!("Destination already exists: {}", dst.display()));
        }
        Self::move_path(&entry.path, &dst)?;
        self.record_file_manager_edit_op(FileManagerEditOp::Moved {
            from: entry.path,
            to: dst.clone(),
        });
        self.file_manager.select(Some(dst.clone()));
        Ok(format!("Renamed to {}", Self::path_display_name(&dst)))
    }

    fn file_manager_move_selected(&mut self, raw_destination: String) -> Result<String> {
        let Some(entry) = self.file_manager_selected_entry() else {
            return Err(anyhow!("Select a file or folder first."));
        };
        let mut dst = PathBuf::from(raw_destination.trim());
        if dst.as_os_str().is_empty() {
            return Err(anyhow!("Destination cannot be empty."));
        }
        if dst.is_relative() {
            dst = self.file_manager.cwd.join(dst);
        }
        if dst.exists() && dst.is_dir() {
            dst = dst.join(Self::path_display_name(&entry.path));
        }
        if dst == entry.path {
            return Ok("Item already at destination.".to_string());
        }
        Self::move_path(&entry.path, &dst)?;
        self.record_file_manager_edit_op(FileManagerEditOp::Moved {
            from: entry.path.clone(),
            to: dst.clone(),
        });
        if let Some(parent) = dst.parent() {
            self.file_manager.set_cwd(parent.to_path_buf());
        }
        self.file_manager.select(Some(dst.clone()));
        Ok(format!("Moved to {}", dst.display()))
    }

    fn file_manager_paste_clipboard(&mut self) -> Result<String> {
        let Some(clip) = self.file_clipboard.clone() else {
            return Err(anyhow!("Clipboard is empty."));
        };
        let target_dir = self.file_manager.cwd.clone();
        let mut changed = 0usize;
        let mut last_dst: Option<PathBuf> = None;

        match clip.mode {
            FileManagerClipboardMode::Copy => {
                for src in clip.paths {
                    if !src.exists() {
                        continue;
                    }
                    let source_name = Self::path_display_name(&src);
                    let mut dst = target_dir.join(&source_name);
                    if dst.exists() {
                        dst = Self::unique_copy_path_in_dir(&target_dir, &source_name, false);
                    }
                    Self::copy_path_recursive(&src, &dst)?;
                    self.record_file_manager_edit_op(FileManagerEditOp::CopyCreated {
                        src,
                        dst: dst.clone(),
                    });
                    changed += 1;
                    last_dst = Some(dst);
                }
            }
            FileManagerClipboardMode::Cut => {
                for src in clip.paths {
                    if !src.exists() {
                        continue;
                    }
                    let source_name = Self::path_display_name(&src);
                    let source_parent = src.parent().map(Path::to_path_buf);
                    if source_parent.as_deref() == Some(target_dir.as_path()) {
                        continue;
                    }
                    let mut dst = target_dir.join(&source_name);
                    if dst.exists() {
                        dst = Self::unique_path_in_dir(&target_dir, &source_name);
                    }
                    Self::move_path(&src, &dst)?;
                    self.record_file_manager_edit_op(FileManagerEditOp::Moved {
                        from: src,
                        to: dst.clone(),
                    });
                    changed += 1;
                    last_dst = Some(dst);
                }
                self.file_clipboard = None;
            }
        }

        if changed == 0 {
            return Err(anyhow!("Clipboard source no longer exists."));
        }
        if let Some(dst) = last_dst {
            self.file_manager.select(Some(dst.clone()));
            if changed == 1 {
                Ok(format!("Pasted {}", Self::path_display_name(&dst)))
            } else {
                Ok(format!("Pasted {changed} items"))
            }
        } else {
            Err(anyhow!("Clipboard source no longer exists."))
        }
    }

    fn file_manager_delete_selected(&mut self) -> Result<String> {
        let Some(entry) = self.file_manager_selected_entry() else {
            return Err(anyhow!("Select a file or folder first."));
        };
        let trash_dir = crate::config::base_dir().join(".fm_trash");
        std::fs::create_dir_all(&trash_dir)
            .map_err(|e| anyhow!("Failed creating trash dir {}: {e}", trash_dir.display()))?;
        let name = Self::path_display_name(&entry.path);
        let trash_target = Self::unique_path_in_dir(&trash_dir, &name);
        Self::move_path(&entry.path, &trash_target)?;
        self.record_file_manager_edit_op(FileManagerEditOp::Moved {
            from: entry.path,
            to: trash_target,
        });
        self.file_manager.ensure_selection_valid();
        Ok(format!("Moved {name} to trash"))
    }

    fn file_manager_undo(&mut self) -> Result<String> {
        let Some(op) = self.file_undo_stack.pop() else {
            return Err(anyhow!("Nothing to undo."));
        };
        Self::apply_file_manager_edit_op(&op, true)?;
        self.file_redo_stack.push(op);
        self.file_manager.ensure_selection_valid();
        Ok("Undo complete".to_string())
    }

    fn file_manager_redo(&mut self) -> Result<String> {
        let Some(op) = self.file_redo_stack.pop() else {
            return Err(anyhow!("Nothing to redo."));
        };
        Self::apply_file_manager_edit_op(&op, false)?;
        self.file_undo_stack.push(op);
        self.file_manager.ensure_selection_valid();
        Ok("Redo complete".to_string())
    }

    fn run_file_manager_action(&mut self, action: FileManagerActionRequest) {
        let outcome = match action {
            FileManagerActionRequest::Copy => {
                self.file_manager_set_clipboard_from_selected(FileManagerClipboardMode::Copy)
            }
            FileManagerActionRequest::Cut => {
                self.file_manager_set_clipboard_from_selected(FileManagerClipboardMode::Cut)
            }
            FileManagerActionRequest::Paste => self.file_manager_paste_clipboard(),
            FileManagerActionRequest::Duplicate => self.file_manager_duplicate_selected(),
            FileManagerActionRequest::Delete => self.file_manager_delete_selected(),
            FileManagerActionRequest::Undo => self.file_manager_undo(),
            FileManagerActionRequest::Redo => self.file_manager_redo(),
        };
        self.shell_status = match outcome {
            Ok(message) => message,
            Err(err) => format!("File action failed: {err}"),
        };
    }

    fn dispatch_context_menu_action(&mut self, _ctx: &Context) {
        let Some(action) = self.context_menu_action.take() else {
            return;
        };
        match action {
            ContextMenuAction::Open => self.activate_file_manager_selection(),
            ContextMenuAction::OpenWith => {
                if let Some(entry) = self.file_manager_selected_file() {
                    let ext_key = Self::open_with_extension_key(&entry.path);
                    self.open_file_manager_open_with_new_command_prompt(entry.path, ext_key, false);
                } else {
                    self.shell_status = "Open With requires a file.".to_string();
                }
            }
            ContextMenuAction::Rename => self.open_file_manager_rename_prompt(),
            ContextMenuAction::Cut => self.run_file_manager_action(FileManagerActionRequest::Cut),
            ContextMenuAction::Copy => self.run_file_manager_action(FileManagerActionRequest::Copy),
            ContextMenuAction::Paste => self.run_file_manager_action(FileManagerActionRequest::Paste),
            ContextMenuAction::Duplicate => {
                self.run_file_manager_action(FileManagerActionRequest::Duplicate)
            }
            ContextMenuAction::Delete => {
                self.run_file_manager_action(FileManagerActionRequest::Delete)
            }
            ContextMenuAction::Properties => {
                self.shell_status = "Properties dialog is not implemented yet.".to_string();
            }
            ContextMenuAction::PasteToDesktop => {
                self.shell_status = "Desktop paste is not implemented yet.".to_string();
            }
            ContextMenuAction::NewFolder => {
                self.shell_status = "Desktop folder creation is not implemented yet.".to_string();
            }
            ContextMenuAction::ChangeAppearance => {
                self.open_desktop_window(DesktopWindow::Settings);
                self.settings.panel = NativeSettingsPanel::Appearance;
            }
            ContextMenuAction::OpenSettings => {
                self.open_desktop_window(DesktopWindow::Settings);
            }
            ContextMenuAction::GenericCopy => {}
            ContextMenuAction::GenericPaste => {}
            ContextMenuAction::GenericSelectAll => {}
            ContextMenuAction::CreateShortcut(app_name) => {
                let label = app_name.clone();
                self.settings.draft.desktop_shortcuts.push(DesktopShortcut {
                    label,
                    app_name,
                    pos_x: None,
                    pos_y: None,
                });
                self.persist_native_settings();
            }
            ContextMenuAction::DeleteShortcut(idx) => {
                if idx < self.settings.draft.desktop_shortcuts.len() {
                    self.settings.draft.desktop_shortcuts.remove(idx);
                    self.persist_native_settings();
                }
            }
            ContextMenuAction::SortDesktopIcons(mode) => {
                self.settings.draft.desktop_icon_sort = mode;
                match mode {
                    DesktopIconSortMode::ByName => {
                        self.settings.draft.desktop_shortcuts.sort_by(|a, b| a.label.cmp(&b.label));
                    }
                    DesktopIconSortMode::ByType => {
                        self.settings.draft.desktop_shortcuts.sort_by(|a, b| a.app_name.cmp(&b.app_name));
                    }
                    DesktopIconSortMode::Custom => {}
                }
                self.settings.draft.desktop_icon_custom_positions.clear();
                self.persist_native_settings();
            }
            ContextMenuAction::ToggleSnapToGrid => {
                self.settings.draft.desktop_snap_to_grid = !self.settings.draft.desktop_snap_to_grid;
                self.persist_native_settings();
            }
            ContextMenuAction::LaunchShortcut(name) => {
                self.run_start_leaf_action(StartLeafAction::LaunchConfiguredApp(name));
            }
        }
    }

    fn attach_file_manager_context_menu(
        action: &mut Option<ContextMenuAction>,
        response: &egui::Response,
        has_selection: bool,
        has_file_selection: bool,
        has_clipboard: bool,
    ) {
        response.context_menu(|ui| {
            Self::apply_context_menu_style(ui);
            ui.set_min_width(136.0);
            ui.set_max_width(180.0);

            let open = if has_selection {
                ui.button("Open")
            } else {
                Self::retro_disabled_button(ui, "Open")
            };
            if open.clicked() {
                *action = Some(ContextMenuAction::Open);
                ui.close_menu();
            }
            let open_with = if has_file_selection {
                ui.button("Open with...")
            } else {
                Self::retro_disabled_button(ui, "Open with...")
            };
            if open_with.clicked() {
                *action = Some(ContextMenuAction::OpenWith);
                ui.close_menu();
            }

            Self::retro_separator(ui);

            let rename = if has_selection {
                ui.button("Rename")
            } else {
                Self::retro_disabled_button(ui, "Rename")
            };
            if rename.clicked() {
                *action = Some(ContextMenuAction::Rename);
                ui.close_menu();
            }

            Self::retro_separator(ui);

            let cut = if has_selection {
                ui.button("Cut")
            } else {
                Self::retro_disabled_button(ui, "Cut")
            };
            if cut.clicked() {
                *action = Some(ContextMenuAction::Cut);
                ui.close_menu();
            }
            let copy = if has_selection {
                ui.button("Copy")
            } else {
                Self::retro_disabled_button(ui, "Copy")
            };
            if copy.clicked() {
                *action = Some(ContextMenuAction::Copy);
                ui.close_menu();
            }
            let paste = if has_clipboard {
                ui.button("Paste")
            } else {
                Self::retro_disabled_button(ui, "Paste")
            };
            if paste.clicked() {
                *action = Some(ContextMenuAction::Paste);
                ui.close_menu();
            }
            let duplicate = if has_selection {
                ui.button("Duplicate")
            } else {
                Self::retro_disabled_button(ui, "Duplicate")
            };
            if duplicate.clicked() {
                *action = Some(ContextMenuAction::Duplicate);
                ui.close_menu();
            }

            Self::retro_separator(ui);

            let delete = if has_selection {
                ui.button("Delete")
            } else {
                Self::retro_disabled_button(ui, "Delete")
            };
            if delete.clicked() {
                *action = Some(ContextMenuAction::Delete);
                ui.close_menu();
            }

            Self::retro_separator(ui);

            let properties = if has_selection {
                ui.button("Properties")
            } else {
                Self::retro_disabled_button(ui, "Properties")
            };
            if properties.clicked() {
                *action = Some(ContextMenuAction::Properties);
                ui.close_menu();
            }
        });
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
                let name_label = if sort_mode == DesktopIconSortMode::ByName { "✓ Sort by Name" } else { "  Sort by Name" };
                let type_label = if sort_mode == DesktopIconSortMode::ByType { "✓ Sort by Type" } else { "  Sort by Type" };
                if ui.button(name_label).clicked() {
                    *action = Some(ContextMenuAction::SortDesktopIcons(DesktopIconSortMode::ByName));
                    ui.close_menu();
                }
                if ui.button(type_label).clicked() {
                    *action = Some(ContextMenuAction::SortDesktopIcons(DesktopIconSortMode::ByType));
                    ui.close_menu();
                }
                Self::retro_separator(ui);
                let snap_label = if snap_to_grid { "✓ Snap to Grid" } else { "  Snap to Grid" };
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

    fn draw_desktop_icons(&mut self, ui: &mut egui::Ui) {
        let Some(cache) = self.asset_cache.as_ref() else {
            return;
        };
        // Clone all needed textures upfront so we release the cache borrow
        let tex_file_manager = cache.icon_file_manager.clone();
        let tex_editor = cache.icon_editor.clone();
        let tex_applications = cache.icon_applications.clone();
        let tex_settings = cache.icon_settings.clone();
        let tex_nuke_codes = cache.icon_nuke_codes.clone();
        let tex_terminal = cache.icon_terminal.clone();
        let tex_shortcut_badge = cache.icon_shortcut_badge.clone();

        let palette = current_palette();
        let style = self.settings.draft.desktop_icon_style;
        let snap = self.settings.draft.desktop_snap_to_grid;
        let workspace = Self::desktop_workspace_rect(ui.ctx());
        let (icon_size, label_height, item_height, column_width): (f32, f32, f32, f32) =
            match style {
            DesktopIconStyle::Minimal => (34.0, 0.0, 46.0, 48.0),
            DesktopIconStyle::Win95 | DesktopIconStyle::Dos => (48.0, 18.0, 74.0, 84.0),
            DesktopIconStyle::NoIcons => return,
        };

        let grid_w = column_width;
        let grid_h = item_height;

        let snap_pos = |pos: egui::Pos2| -> egui::Pos2 {
            if snap {
                egui::pos2(
                    (pos.x / grid_w).round() * grid_w,
                    (pos.y / grid_h).round() * grid_h,
                )
            } else {
                pos
            }
        };

        let builtin_entries: [(TextureHandle, &str, &str, &str); 6] = [
            (tex_file_manager, "builtin_0", "Files", "[DIR]"),
            (tex_editor, "builtin_1", "Documents", "[TXT]"),
            (tex_applications.clone(), "builtin_2", "Apps", "[APP]"),
            (tex_settings, "builtin_3", "Settings", "[CFG]"),
            (tex_nuke_codes, "builtin_4", "Nuke Codes", "[!]"),
            (tex_terminal, "builtin_5", "Terminal", "[_]"),
        ];

        let mut open_window: Option<DesktopWindow> = None;
        let open_windows: [DesktopWindow; 6] = [
            DesktopWindow::FileManager,
            DesktopWindow::Editor,
            DesktopWindow::Applications,
            DesktopWindow::Settings,
            DesktopWindow::NukeCodes,
            DesktopWindow::PtyApp,
        ];
        let mut shortcut_action: Option<ContextMenuAction> = None;
        let mut needs_persist = false;

        for (index, (texture, key, label, ascii)) in builtin_entries.iter().enumerate() {
            let stored_pos = self.settings.draft.desktop_icon_custom_positions
                .get(*key)
                .map(|&[x, y]| egui::pos2(x, y));
            let top_left = stored_pos.unwrap_or_else(|| egui::pos2(
                workspace.left() + 4.0,
                workspace.top() + 16.0 + index as f32 * item_height,
            ));

            let icon_rect = egui::Rect::from_min_size(
                top_left + egui::vec2((column_width - icon_size) * 0.5, 0.0),
                egui::vec2(icon_size, icon_size),
            );
            let label_rect = egui::Rect::from_min_size(
                top_left + egui::vec2(0.0, icon_size + 2.0),
                egui::vec2(column_width, label_height.max(16.0)),
            );
            let hit_rect = if label_height > 0.0 {
                egui::Rect::from_min_size(top_left, egui::vec2(column_width, icon_size + label_height + 2.0))
            } else {
                icon_rect
            };

            let response = ui.allocate_rect(hit_rect, egui::Sense::click_and_drag());

            match style {
                DesktopIconStyle::Dos => {
                    ui.painter().text(
                        icon_rect.center(),
                        Align2::CENTER_CENTER,
                        *ascii,
                        FontId::new(18.0, FontFamily::Monospace),
                        palette.fg,
                    );
                }
                DesktopIconStyle::Minimal | DesktopIconStyle::Win95 => {
                    Self::paint_tinted_texture(ui.painter(), texture, icon_rect, palette.fg);
                }
                DesktopIconStyle::NoIcons => {}
            }

            if label_height > 0.0 {
                ui.painter().text(
                    label_rect.center(),
                    Align2::CENTER_CENTER,
                    *label,
                    FontId::new(13.0, FontFamily::Monospace),
                    palette.fg,
                );
            }

            if response.dragged() {
                let new_pos = top_left + response.drag_delta();
                self.settings.draft.desktop_icon_custom_positions
                    .insert(key.to_string(), [new_pos.x, new_pos.y]);
            }
            if response.drag_stopped() {
                if snap {
                    if let Some(&[x, y]) = self.settings.draft.desktop_icon_custom_positions.get(*key) {
                        let snapped = snap_pos(egui::pos2(x, y));
                        self.settings.draft.desktop_icon_custom_positions
                            .insert(key.to_string(), [snapped.x, snapped.y]);
                    }
                }
                needs_persist = true;
            }

            if response.double_clicked() {
                open_window = Some(open_windows[index]);
            }
        }

        let shortcuts = self.settings.draft.desktop_shortcuts.clone();
        for (sidx, shortcut) in shortcuts.iter().enumerate() {
            let key = format!("shortcut_{}", sidx);
            let stored_pos = self.settings.draft.desktop_icon_custom_positions
                .get(&key)
                .map(|&[x, y]| egui::pos2(x, y));
            let default_shortcut_pos = egui::pos2(
                workspace.left() + 4.0 + column_width,
                workspace.top() + 16.0 + sidx as f32 * item_height,
            );
            let top_left = stored_pos.unwrap_or(default_shortcut_pos);

            let icon_rect = egui::Rect::from_min_size(
                top_left + egui::vec2((column_width - icon_size) * 0.5, 0.0),
                egui::vec2(icon_size, icon_size),
            );
            let label_rect = egui::Rect::from_min_size(
                top_left + egui::vec2(0.0, icon_size + 2.0),
                egui::vec2(column_width, label_height.max(16.0)),
            );
            let hit_rect = if label_height > 0.0 {
                egui::Rect::from_min_size(top_left, egui::vec2(column_width, icon_size + label_height + 2.0))
            } else {
                icon_rect
            };

            let response = ui.allocate_rect(hit_rect, egui::Sense::click_and_drag());

            match style {
                DesktopIconStyle::Dos => {
                    ui.painter().text(
                        icon_rect.center(),
                        Align2::CENTER_CENTER,
                        "[LNK]",
                        FontId::new(18.0, FontFamily::Monospace),
                        palette.fg,
                    );
                }
                DesktopIconStyle::Minimal | DesktopIconStyle::Win95 => {
                    Self::paint_tinted_texture(ui.painter(), &tex_applications, icon_rect, palette.fg);
                    let badge_size = (icon_size * 0.35).max(10.0);
                    let badge_rect = egui::Rect::from_min_size(
                        icon_rect.min + egui::vec2(0.0, icon_size - badge_size),
                        egui::vec2(badge_size, badge_size),
                    );
                    ui.painter().rect_filled(badge_rect, 0.0, Color32::BLACK);
                    Self::paint_tinted_texture(ui.painter(), &tex_shortcut_badge, badge_rect, Color32::WHITE);
                }
                DesktopIconStyle::NoIcons => {}
            }

            if label_height > 0.0 {
                ui.painter().text(
                    label_rect.center(),
                    Align2::CENTER_CENTER,
                    &shortcut.label,
                    FontId::new(13.0, FontFamily::Monospace),
                    palette.fg,
                );
            }

            if response.dragged() {
                let new_pos = top_left + response.drag_delta();
                self.settings.draft.desktop_icon_custom_positions
                    .insert(key.clone(), [new_pos.x, new_pos.y]);
            }
            if response.drag_stopped() {
                if snap {
                    if let Some(&[x, y]) = self.settings.draft.desktop_icon_custom_positions.get(&key) {
                        let snapped = snap_pos(egui::pos2(x, y));
                        self.settings.draft.desktop_icon_custom_positions
                            .insert(key.clone(), [snapped.x, snapped.y]);
                    }
                }
                needs_persist = true;
            }

            let app_name_for_menu = shortcut.app_name.clone();
            response.context_menu(|ui| {
                Self::apply_context_menu_style(ui);
                ui.set_min_width(136.0);
                ui.set_max_width(180.0);
                if ui.button("Open").clicked() {
                    shortcut_action = Some(ContextMenuAction::LaunchShortcut(app_name_for_menu.clone()));
                    ui.close_menu();
                }
                Self::retro_separator(ui);
                if ui.button("Delete Shortcut").clicked() {
                    shortcut_action = Some(ContextMenuAction::DeleteShortcut(sidx));
                    ui.close_menu();
                }
            });

            if response.double_clicked() {
                shortcut_action = Some(ContextMenuAction::LaunchShortcut(shortcut.app_name.clone()));
            }
        }

        if needs_persist {
            self.persist_native_settings();
        }

        if let Some(action) = shortcut_action {
            match action {
                ContextMenuAction::DeleteShortcut(idx) => {
                    if idx < self.settings.draft.desktop_shortcuts.len() {
                        self.settings.draft.desktop_shortcuts.remove(idx);
                        self.persist_native_settings();
                    }
                }
                _ => {
                    self.context_menu_action = Some(action);
                }
            }
        }

        if let Some(window) = open_window {
            self.open_desktop_window(window);
        }
    }

    fn open_start_menu(&mut self) {
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
                !matches!(action, StartSystemAction::Connections)
                    || !crate::connections::macos_connections_disabled()
            })
            .collect()
    }

    fn start_leaf_items(&self, leaf: StartLeaf) -> Vec<StartLeafItem> {
        match leaf {
            StartLeaf::Applications => {
                let mut items = Vec::new();
                if self.settings.draft.builtin_menu_visibility.nuke_codes {
                    items.push(StartLeafItem {
                        label: BUILTIN_NUKE_CODES_APP.to_string(),
                        action: StartLeafAction::LaunchNukeCodes,
                    });
                }
                if self.settings.draft.builtin_menu_visibility.text_editor {
                    items.push(StartLeafItem {
                        label: BUILTIN_TEXT_EDITOR_APP.to_string(),
                        action: StartLeafAction::OpenTextEditor,
                    });
                }
                for name in app_names() {
                    if name == BUILTIN_NUKE_CODES_APP || name == BUILTIN_TEXT_EDITOR_APP {
                        continue;
                    }
                    items.push(StartLeafItem {
                        label: name.clone(),
                        action: StartLeafAction::LaunchConfiguredApp(name),
                    });
                }
                if items.is_empty() {
                    items.push(StartLeafItem {
                        label: "(No applications)".to_string(),
                        action: StartLeafAction::None,
                    });
                }
                items
            }
            StartLeaf::Documents => {
                let mut items = Vec::new();
                if let Some(session) = &self.session {
                    items.push(StartLeafItem {
                        label: "My Documents".to_string(),
                        action: StartLeafAction::OpenDocumentCategory(word_processor_dir(
                            &session.username,
                        )),
                    });
                }
                let categories = load_categories();
                for key in Self::sorted_keys(&categories) {
                    if let Some(path) = categories.get(&key).and_then(|v| v.as_str()) {
                        items.push(StartLeafItem {
                            label: key,
                            action: StartLeafAction::OpenDocumentCategory(PathBuf::from(path)),
                        });
                    }
                }
                if items.is_empty() {
                    items.push(StartLeafItem {
                        label: "(No documents)".to_string(),
                        action: StartLeafAction::None,
                    });
                }
                items
            }
            StartLeaf::Network => {
                let mut items = Vec::new();
                for key in Self::sorted_keys(&load_networks()) {
                    items.push(StartLeafItem {
                        label: key.clone(),
                        action: StartLeafAction::LaunchNetworkProgram(key),
                    });
                }
                if items.is_empty() {
                    items.push(StartLeafItem {
                        label: "(No network apps)".to_string(),
                        action: StartLeafAction::None,
                    });
                }
                items
            }
            StartLeaf::Games => {
                let mut items = Vec::new();
                for key in Self::sorted_keys(&load_games()) {
                    items.push(StartLeafItem {
                        label: key.clone(),
                        action: StartLeafAction::LaunchGameProgram(key),
                    });
                }
                if items.is_empty() {
                    items.push(StartLeafItem {
                        label: "(No games installed)".to_string(),
                        action: StartLeafAction::None,
                    });
                }
                items
            }
        }
    }

    fn open_file_manager_at(&mut self, path: PathBuf) {
        if !path.is_dir() {
            self.shell_status = format!("Error: '{}' not found.", path.display());
            return;
        }
        self.file_manager.set_cwd(path);
        self.file_manager.selected = None;
        self.open_desktop_window(DesktopWindow::FileManager);
    }

    fn launch_named_program_from_map(
        &mut self,
        name: &str,
        map: &serde_json::Map<String, Value>,
        status_label: &str,
    ) {
        match resolve_program_command(name, map) {
            Ok(cmd) => self.open_desktop_pty(status_label, &cmd),
            Err(err) => self.shell_status = err,
        }
    }

    fn open_desktop_nuke_codes(&mut self) {
        if matches!(self.terminal_nuke_codes, NukeCodesView::Unloaded) {
            self.terminal_nuke_codes = fetch_nuke_codes();
        }
        self.open_desktop_window(DesktopWindow::NukeCodes);
    }

    fn run_start_root_action(&mut self, action: StartRootAction) {
        match action {
            StartRootAction::ReturnToTerminal => {
                self.close_start_menu();
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
        match action {
            StartSystemAction::ProgramInstaller => {
                self.close_start_menu();
                self.desktop_mode_open = false;
                self.terminal_installer.reset();
                self.navigate_to_screen(TerminalScreen::ProgramInstaller);
                self.shell_status.clear();
            }
            StartSystemAction::Terminal => self.open_desktop_terminal_shell(),
            StartSystemAction::FileManager => self.open_desktop_window(DesktopWindow::FileManager),
            StartSystemAction::Settings => self.open_desktop_window(DesktopWindow::Settings),
            StartSystemAction::Connections => {
                if crate::connections::macos_connections_disabled() {
                    self.shell_status =
                        crate::connections::macos_connections_disabled_hint().to_string();
                } else {
                    self.close_start_menu();
                    self.open_desktop_window(DesktopWindow::Settings);
                    self.settings.panel = NativeSettingsPanel::Connections;
                    self.shell_status.clear();
                }
            }
        }
    }

    fn run_start_leaf_action(&mut self, action: StartLeafAction) {
        match action {
            StartLeafAction::None => {}
            StartLeafAction::LaunchNukeCodes => {
                self.close_start_menu();
                self.open_desktop_nuke_codes();
            }
            StartLeafAction::OpenTextEditor => {
                if self.editor.path.is_none() {
                    self.new_document();
                } else {
                    self.open_desktop_window(DesktopWindow::Editor);
                }
            }
            StartLeafAction::LaunchConfiguredApp(name) => {
                let apps = load_apps();
                match resolve_program_command(&name, &apps) {
                    Ok(cmd) => self.open_desktop_pty(&name, &cmd),
                    Err(err) => self.shell_status = err,
                }
            }
            StartLeafAction::OpenDocumentCategory(path) => self.open_file_manager_at(path),
            StartLeafAction::LaunchNetworkProgram(name) => {
                self.launch_named_program_from_map(&name, &load_networks(), &name);
            }
            StartLeafAction::LaunchGameProgram(name) => {
                self.launch_named_program_from_map(&name, &load_games(), &name);
            }
        }
    }

    fn open_manual_file(&mut self, path: &str, status_label: &str) {
        let manual = PathBuf::from(path);
        match read_text_file(&manual) {
            Ok(text) => {
                self.editor.path = Some(manual);
                self.editor.text = text;
                self.editor.dirty = false;
                self.editor.status = format!("Opened {status_label}.");
                self.open_desktop_window(DesktopWindow::Editor);
            }
            Err(err) => {
                self.shell_status = format!("{status_label} unavailable: {err}");
            }
        }
    }

    fn draw_desktop_window_by_kind(&mut self, ctx: &Context, window: DesktopWindow) {
        match window {
            DesktopWindow::FileManager => self.draw_file_manager(ctx),
            DesktopWindow::Editor => self.draw_editor(ctx),
            DesktopWindow::Settings => self.draw_settings(ctx),
            DesktopWindow::Applications => self.draw_applications(ctx),
            DesktopWindow::NukeCodes => self.draw_nuke_codes_window(ctx),
            DesktopWindow::TerminalMode => self.draw_terminal_mode(ctx),
            DesktopWindow::PtyApp => self.draw_desktop_pty_window(ctx),
        }
    }

    fn draw_desktop_windows(&mut self, ctx: &Context) {
        self.sync_desktop_active_window();
        const ORDER: [DesktopWindow; 7] = [
            DesktopWindow::FileManager,
            DesktopWindow::Editor,
            DesktopWindow::Settings,
            DesktopWindow::Applications,
            DesktopWindow::NukeCodes,
            DesktopWindow::TerminalMode,
            DesktopWindow::PtyApp,
        ];
        let active = self.desktop_active_window;
        for window in ORDER {
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
        crate::config::reload_settings();
        let snapshot: NativeShellSnapshot = read_shell_snapshot(username);
        self.session = Some(SessionState {
            username: username.to_string(),
            is_admin: user.is_admin,
        });
        self.login_hacking = None;
        self.file_manager.cwd = if snapshot.file_manager_dir.exists() {
            snapshot.file_manager_dir
        } else {
            word_processor_dir(username)
        };
        self.file_manager.open = false;
        self.file_manager.selected = None;
        self.editor.open = false;
        self.editor.path = None;
        self.editor.text.clear();
        self.editor.dirty = false;
        self.editor.status.clear();
        self.settings.draft = current_settings();
        self.settings.status.clear();
        self.settings.panel = NativeSettingsPanel::Home;
        self.desktop_nuke_codes_open = false;
        self.terminal_mode.status.clear();
        let launch_default_desktop = matches!(self.settings.draft.default_open_mode, OpenMode::Desktop)
            && session::take_default_mode_pending_for_active();
        self.desktop_window_states.clear();
        self.desktop_active_window = None;
        self.start_open = !launch_default_desktop;
        self.start_selected_root = 0;
        self.start_system_selected = 0;
        self.start_leaf_selected = 0;
        self.start_open_submenu = None;
        self.start_open_leaf = None;
        self.desktop_mode_open = launch_default_desktop;
        self.main_menu_idx = 0;
        self.terminal_screen = TerminalScreen::MainMenu;
        self.terminal_apps_idx = 0;
        self.terminal_documents_idx = 0;
        self.terminal_logs_idx = 0;
        self.terminal_network_idx = 0;
        self.terminal_games_idx = 0;
        self.terminal_nuke_codes = NukeCodesView::default();
        self.terminal_nuke_codes_return = TerminalScreen::Applications;
        self.terminal_pty = None;
        self.terminal_installer.reset();
        self.terminal_settings_idx = 0;
        self.terminal_edit_menus.reset();
        self.terminal_connections.reset();
        self.terminal_default_apps_idx = 0;
        self.terminal_default_app_choice_idx = 0;
        self.terminal_default_app_slot = None;
        self.terminal_browser_idx = 0;
        self.terminal_browser_return = TerminalScreen::Documents;
        self.terminal_user_management_idx = 0;
        self.terminal_user_management_mode = UserManagementMode::Root;
        self.terminal_settings_choice = None;
        self.terminal_prompt = None;
        self.suppress_next_menu_submit = false;
        self.terminal_flash = None;
        self.session_leader_until = None;
        self.shell_status.clear();
    }

    fn persist_snapshot(&self) {
        if let Some(session) = &self.session {
            write_shell_snapshot(
                &session.username,
                &NativeShellSnapshot {
                    file_manager_dir: self.file_manager.cwd.clone(),
                    editor_path: self.editor.path.clone(),
                },
            );
        }
    }

    fn navigate_to_screen(&mut self, screen: TerminalScreen) {
        if self.terminal_screen != screen {
            crate::sound::play_navigate();
        }
        self.terminal_screen = screen;
    }

    fn set_user_management_mode(&mut self, mode: UserManagementMode, selected_idx: usize) {
        let changed = self.terminal_user_management_mode != mode
            || self.terminal_user_management_idx != selected_idx;
        if changed {
            crate::sound::play_navigate();
        }
        self.terminal_user_management_mode = mode;
        self.terminal_user_management_idx = selected_idx;
    }

    fn queue_login(&mut self, username: String, user: UserRecord) {
        crate::sound::play_login();
        bind_login_session(&username);
        self.login.password.clear();
        self.login.error.clear();
        self.terminal_prompt = None;
        self.queue_terminal_flash(
            "Logging in...",
            700,
            FlashAction::FinishLogin { username, user },
        );
    }

    fn queue_hacking_start(&mut self, username: String) {
        self.login.error.clear();
        self.terminal_prompt = None;
        self.queue_terminal_flash(
            "SECURITY OVERRIDE",
            1200,
            FlashAction::StartHacking { username },
        );
    }

    fn do_login(&mut self) {
        self.login.error.clear();
        let username = self.login.selected_username.trim().to_string();
        if username.is_empty() {
            crate::sound::play_error();
            self.login.error = "Select a user.".to_string();
            return;
        }
        match authenticate(&username, &self.login.password) {
            Ok(user) => self.queue_login(username, user),
            Err(err) => {
                crate::sound::play_error();
                self.login.error = err.to_string();
            }
        }
    }

    fn login_usernames(&self) -> Vec<String> {
        ensure_default_admin();
        let mut usernames: Vec<String> = load_users().keys().cloned().collect();
        usernames.sort();
        usernames
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

    fn begin_logout(&mut self) {
        if let Some(flash) = self.terminal_flash.as_ref() {
            if matches!(&flash.action, FlashAction::FinishLogout) {
                return;
            }
        }
        crate::sound::play_logout();
        self.persist_snapshot();
        self.terminate_all_native_pty_children();
        self.terminal_prompt = None;
        self.terminal_screen = TerminalScreen::MainMenu;
        self.close_start_menu();
        self.desktop_mode_open = false;
        self.desktop_nuke_codes_open = false;
        self.desktop_active_window = None;
        self.session_leader_until = None;
        self.queue_terminal_flash("Logging out...", 800, FlashAction::FinishLogout);
    }

    fn finish_logout(&mut self) {
        crate::config::reload_settings();
        self.terminate_all_native_pty_children();
        session::clear_sessions();
        session::take_switch_request();
        self.session_runtime.clear();
        self.session = None;
        self.login_mode = LoginScreenMode::SelectUser;
        self.login_hacking = None;
        self.login.selected_idx = 0;
        self.login.selected_username.clear();
        self.login.password.clear();
        self.login.error.clear();
        self.file_manager.open = false;
        self.editor.open = false;
        self.settings.open = false;
        self.settings.panel = NativeSettingsPanel::Home;
        self.applications.open = false;
        self.desktop_nuke_codes_open = false;
        self.terminal_mode.open = false;
        self.desktop_active_window = None;
        self.start_open = true;
        self.start_selected_root = 0;
        self.start_system_selected = 0;
        self.start_leaf_selected = 0;
        self.start_open_submenu = None;
        self.start_open_leaf = None;
        self.desktop_mode_open = false;
        self.terminal_screen = TerminalScreen::MainMenu;
        self.terminal_apps_idx = 0;
        self.terminal_documents_idx = 0;
        self.terminal_logs_idx = 0;
        self.terminal_network_idx = 0;
        self.terminal_games_idx = 0;
        self.terminal_nuke_codes = NukeCodesView::default();
        self.terminal_nuke_codes_return = TerminalScreen::Applications;
        self.terminal_settings_idx = 0;
        self.terminal_default_apps_idx = 0;
        self.terminal_connections.reset();
        self.terminal_edit_menus.reset();
        self.terminal_pty = None;
        self.terminal_installer.reset();
        self.terminal_default_app_choice_idx = 0;
        self.terminal_default_app_slot = None;
        self.terminal_browser_idx = 0;
        self.terminal_browser_return = TerminalScreen::Documents;
        self.terminal_user_management_idx = 0;
        self.terminal_user_management_mode = UserManagementMode::Root;
        self.terminal_settings_choice = None;
        self.terminal_prompt = None;
        self.terminal_flash = None;
        self.session_leader_until = None;
        self.shell_status.clear();
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

    fn open_file_manager_rename_prompt(&mut self) {
        let Some(entry) = self.file_manager_selected_entry() else {
            self.shell_status = "Select a file or folder first.".to_string();
            return;
        };
        self.terminal_prompt = Some(TerminalPrompt {
            kind: TerminalPromptKind::Input,
            title: "Rename".to_string(),
            prompt: format!("Rename {} to:", entry.label),
            buffer: entry.label,
            confirm_yes: true,
            action: TerminalPromptAction::FileManagerRename {
                path: entry.path,
            },
        });
    }

    fn open_file_manager_move_prompt(&mut self) {
        let Some(entry) = self.file_manager_selected_entry() else {
            self.shell_status = "Select a file or folder first.".to_string();
            return;
        };
        self.terminal_prompt = Some(TerminalPrompt {
            kind: TerminalPromptKind::Input,
            title: "Move To".to_string(),
            prompt: "Move to (dir or full path):".to_string(),
            buffer: String::new(),
            confirm_yes: true,
            action: TerminalPromptAction::FileManagerMoveTo {
                path: entry.path,
            },
        });
    }

    fn open_file_manager_open_with_new_command_prompt(
        &mut self,
        path: PathBuf,
        ext_key: String,
        make_default: bool,
    ) {
        let ext_label = Self::open_with_extension_label(&ext_key);
        self.terminal_prompt = Some(TerminalPrompt {
            kind: TerminalPromptKind::Input,
            title: format!("Open With {}", ext_label),
            prompt: if make_default {
                format!("Open with command for {} (saved as default):", ext_label)
            } else {
                format!("Open with command for {}:", ext_label)
            },
            buffer: String::new(),
            confirm_yes: true,
            action: TerminalPromptAction::FileManagerOpenWithNewCommand {
                path,
                ext_key,
                make_default,
            },
        });
    }

    fn open_file_manager_open_with_edit_command_prompt(
        &mut self,
        path: PathBuf,
        ext_key: String,
        previous: String,
    ) {
        let ext_label = Self::open_with_extension_label(&ext_key);
        self.terminal_prompt = Some(TerminalPrompt {
            kind: TerminalPromptKind::Input,
            title: format!("Open With {}", ext_label),
            prompt: format!("Edit saved command for {}:", ext_label),
            buffer: previous.clone(),
            confirm_yes: true,
            action: TerminalPromptAction::FileManagerOpenWithEditCommand {
                path,
                ext_key,
                previous,
            },
        });
    }

    fn save_user_and_status(&mut self, username: &str, user: UserRecord, status: String) {
        let mut db = load_users();
        db.insert(username.to_string(), user);
        save_users(&db);
        let _ = std::fs::create_dir_all(crate::config::users_dir().join(username));
        crate::config::mark_default_apps_prompt_pending(username);
        self.shell_status = status;
    }

    fn update_user_record<F: FnOnce(&mut UserRecord)>(
        &mut self,
        username: &str,
        f: F,
        status: String,
    ) {
        let mut db = load_users();
        if let Some(record) = db.get_mut(username) {
            f(record);
            save_users(&db);
            self.shell_status = status;
        } else {
            self.shell_status = format!("Unknown user '{username}'.");
        }
    }

    fn open_embedded_pty(&mut self, title: &str, cmd: &[String], return_screen: TerminalScreen) {
        let layout = self.terminal_layout();
        let options = crate::pty::PtyLaunchOptions::default();
        match spawn_embedded_pty_with_options(
            title,
            cmd,
            return_screen,
            layout.cols as u16,
            layout.rows.saturating_sub(1) as u16,
            options,
        ) {
            Ok(state) => {
                self.terminal_pty = Some(state);
                self.navigate_to_screen(TerminalScreen::PtyApp);
                self.shell_status = format!("Opened {title} in PTY.");
            }
            Err(err) => {
                self.shell_status = err;
            }
        }
    }

    fn open_desktop_pty(&mut self, title: &str, cmd: &[String]) {
        if let Some(mut previous) = self.terminal_pty.take() {
            previous.session.terminate();
        }
        let layout = self.terminal_layout();
        let options = crate::pty::PtyLaunchOptions {
            force_render_mode: Some(false),
            ..crate::pty::PtyLaunchOptions::default()
        };
        match spawn_embedded_pty_with_options(
            title,
            cmd,
            TerminalScreen::MainMenu,
            layout.cols as u16,
            layout.rows.saturating_sub(1) as u16,
            options,
        ) {
            Ok(state) => {
                self.terminal_pty = Some(state);
                self.open_desktop_window(DesktopWindow::PtyApp);
                self.desktop_window_state_mut(DesktopWindow::PtyApp).maximized = false;
                self.shell_status = format!("Opened {title} in PTY window.");
            }
            Err(err) => {
                self.shell_status = err;
            }
        }
    }

    fn open_embedded_terminal_shell(&mut self) {
        let layout = self.terminal_layout();
        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".to_string());
        let shell_name = std::path::Path::new(&shell)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("");
        let mut cmd = vec![shell.clone()];
        match shell_name {
            "bash" => {
                cmd.push("--noprofile".to_string());
                cmd.push("--norc".to_string());
            }
            "zsh" => {
                cmd.push("-f".to_string());
            }
            _ => {}
        }
        let options = crate::pty::PtyLaunchOptions {
            env: vec![
                ("PS1".into(), "> ".into()),
                ("PROMPT".into(), "> ".into()),
                ("ZDOTDIR".into(), "/dev/null".into()),
            ],
            top_bar: Some("ROBCO MAINTENANCE TERMLINK".into()),
            force_render_mode: Some(true),
        };
        match spawn_embedded_pty_with_options(
            "ROBCO MAINTENANCE TERMLINK",
            &cmd,
            TerminalScreen::MainMenu,
            layout.cols as u16,
            layout.rows.saturating_sub(1) as u16,
            options,
        ) {
            Ok(state) => {
                self.terminal_pty = Some(state);
                self.navigate_to_screen(TerminalScreen::PtyApp);
                self.shell_status = "Opened terminal shell in PTY.".to_string();
            }
            Err(err) => {
                self.shell_status = err;
            }
        }
    }

    fn open_desktop_terminal_shell(&mut self) {
        if let Some(mut previous) = self.terminal_pty.take() {
            previous.session.terminate();
        }
        let layout = self.terminal_layout();
        let requested_shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".to_string());
        let requested_shell_name = std::path::Path::new(&requested_shell)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("");
        // fish emits a terminal capability warning in the embedded desktop PTY
        // because this surface does not answer the DA query it expects.
        let shell = if requested_shell_name == "fish" {
            if std::path::Path::new("/bin/bash").exists() {
                "/bin/bash".to_string()
            } else {
                "/bin/sh".to_string()
            }
        } else {
            requested_shell
        };
        let shell_name = std::path::Path::new(&shell)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("");
        let mut cmd = vec![shell.clone()];
        match shell_name {
            "bash" => {
                cmd.push("--noprofile".to_string());
                cmd.push("--norc".to_string());
            }
            "zsh" => {
                cmd.push("-f".to_string());
            }
            _ => {}
        }
        let options = crate::pty::PtyLaunchOptions {
            env: vec![
                ("PS1".into(), "> ".into()),
                ("PROMPT".into(), "> ".into()),
                ("ZDOTDIR".into(), "/dev/null".into()),
            ],
            top_bar: None,
            force_render_mode: Some(false),
        };
        match spawn_embedded_pty_with_options(
            "Terminal",
            &cmd,
            TerminalScreen::MainMenu,
            layout.cols as u16,
            layout.rows.saturating_sub(1) as u16,
            options,
        ) {
            Ok(state) => {
                self.terminal_pty = Some(state);
                self.open_desktop_window(DesktopWindow::PtyApp);
                self.shell_status = "Opened terminal shell in PTY window.".to_string();
            }
            Err(err) => {
                self.shell_status = err;
            }
        }
    }

    fn open_path_in_editor(&mut self, path: PathBuf) {
        match read_text_file(&path) {
            Ok(text) => {
                self.editor.path = Some(path);
                self.editor.text = text;
                self.editor.dirty = false;
                self.open_desktop_window(DesktopWindow::Editor);
                self.editor.status = "Opened document.".to_string();
            }
            Err(err) => {
                self.editor.status = format!("Open failed: {err}");
                self.open_desktop_window(DesktopWindow::Editor);
            }
        }
    }

    fn activate_file_manager_selection(&mut self) {
        match self.file_manager.activate_selected() {
            FileManagerAction::None | FileManagerAction::ChangedDir => {}
            FileManagerAction::OpenFile(path) => {
                let ext_key = Self::open_with_extension_key(&path);
                if let Some(command_line) = self.open_with_default_for_extension(&ext_key) {
                    match self.launch_open_with_command(&path, &command_line) {
                        Ok(message) => self.shell_status = message,
                        Err(err) => self.shell_status = format!("Open failed: {err}"),
                    }
                } else {
                    self.open_path_in_editor(path);
                }
            }
        }
    }

    fn new_document(&mut self) {
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
        self.editor.path = Some(path);
        self.editor.text.clear();
        self.editor.dirty = false;
        self.open_desktop_window(DesktopWindow::Editor);
        self.editor.status = "New document.".to_string();
    }

    fn save_editor(&mut self) {
        let Some(path) = self.editor.path.clone() else {
            self.editor.status = "No document path set.".to_string();
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
    }

    fn terminal_app_items(&self) -> Vec<String> {
        let mut items: Vec<String> = Vec::new();
        if self.settings.draft.builtin_menu_visibility.nuke_codes {
            items.push(BUILTIN_NUKE_CODES_APP.to_string());
        }
        if self.settings.draft.builtin_menu_visibility.text_editor {
            items.push(BUILTIN_TEXT_EDITOR_APP.to_string());
        }
        items.extend(
            app_names()
                .into_iter()
                .filter(|name| name != BUILTIN_NUKE_CODES_APP && name != BUILTIN_TEXT_EDITOR_APP),
        );
        items.push("---".to_string());
        items.push("Back".to_string());
        items
    }

    fn sorted_keys(data: &serde_json::Map<String, Value>) -> Vec<String> {
        let mut names: Vec<String> = data.keys().cloned().collect();
        names.sort();
        names
    }

    fn edit_program_entries(&self, target: EditMenuTarget) -> Vec<String> {
        match target {
            EditMenuTarget::Applications => Self::sorted_keys(&load_apps()),
            EditMenuTarget::Documents => Self::sorted_keys(&load_categories()),
            EditMenuTarget::Network => Self::sorted_keys(&load_networks()),
            EditMenuTarget::Games => Self::sorted_keys(&load_games()),
        }
    }

    fn add_program_entry(&mut self, target: EditMenuTarget, name: String, command: String) {
        let Some(argv) = parse_custom_command_line(command.trim()) else {
            self.shell_status = "Error: invalid command line".to_string();
            return;
        };
        if argv.is_empty() {
            self.shell_status = "Error: invalid command line".to_string();
            return;
        }
        let json_argv = Value::Array(argv.into_iter().map(Value::String).collect());
        match target {
            EditMenuTarget::Applications => {
                let mut apps = load_apps();
                apps.insert(name.clone(), json_argv);
                save_apps(&apps);
            }
            EditMenuTarget::Documents => {
                self.shell_status = "Error: invalid target for command entry.".to_string();
                return;
            }
            EditMenuTarget::Network => {
                let mut network = load_networks();
                network.insert(name.clone(), json_argv);
                save_networks(&network);
            }
            EditMenuTarget::Games => {
                let mut games = load_games();
                games.insert(name.clone(), json_argv);
                save_games(&games);
            }
        }
        self.shell_status = format!("{name} added.");
    }

    fn delete_program_entry(&mut self, target: EditMenuTarget, name: &str) {
        match target {
            EditMenuTarget::Applications => {
                let mut apps = load_apps();
                apps.remove(name);
                save_apps(&apps);
            }
            EditMenuTarget::Documents => {
                self.delete_document_category(name);
                return;
            }
            EditMenuTarget::Network => {
                let mut network = load_networks();
                network.remove(name);
                save_networks(&network);
            }
            EditMenuTarget::Games => {
                let mut games = load_games();
                games.remove(name);
                save_games(&games);
            }
        }
        self.shell_status = format!("{name} deleted.");
    }

    fn expand_tilde(raw: &str) -> PathBuf {
        if let Some(rest) = raw.strip_prefix('~') {
            if let Some(home) = dirs::home_dir() {
                return PathBuf::from(format!("{}{}", home.display(), rest));
            }
        }
        PathBuf::from(raw)
    }

    fn add_document_category(&mut self, name: String, path_raw: String) {
        let expanded = Self::expand_tilde(path_raw.trim());
        if !expanded.is_dir() {
            self.shell_status = "Error: Invalid directory.".to_string();
            return;
        }
        let mut categories = load_categories();
        categories.insert(name, Value::String(expanded.to_string_lossy().to_string()));
        save_categories(&categories);
        self.shell_status = "Category added.".to_string();
    }

    fn delete_document_category(&mut self, name: &str) {
        let mut categories = load_categories();
        categories.remove(name);
        save_categories(&categories);
        self.shell_status = "Deleted.".to_string();
    }

    fn sorted_document_categories() -> Vec<String> {
        Self::sorted_keys(&load_categories())
    }

    fn open_document_browser_at(&mut self, dir: PathBuf, return_screen: TerminalScreen) {
        if !dir.is_dir() {
            self.shell_status = format!("Error: '{}' not found.", dir.display());
            return;
        }
        self.file_manager.set_cwd(dir);
        self.file_manager.selected = None;
        self.terminal_browser_idx = 0;
        self.terminal_browser_return = return_screen;
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
        save_settings(self.settings.draft.clone());
        crate::config::reload_settings();
        self.settings.draft = current_settings();
        self.shell_status = "Settings saved.".to_string();
    }

    fn apply_login_selection_action(&mut self, action: LoginSelectionAction) {
        self.login.error.clear();
        match action {
            LoginSelectionAction::Exit => {
                crate::sound::play_logout();
                self.queue_terminal_flash("Exiting...", 800, FlashAction::ExitApp);
            }
            LoginSelectionAction::PromptPassword { username } => {
                self.login.selected_username = username;
                self.login.password.clear();
                self.login_mode = LoginScreenMode::SelectUser;
                self.open_password_prompt(
                    "Password Prompt",
                    format!("Password for {}", self.login.selected_username),
                );
            }
            LoginSelectionAction::AuthenticateWithoutPassword { username } => {
                crate::sound::play_navigate();
                self.login.selected_username = username.clone();
                match authenticate(&username, "") {
                    Ok(user) => self.queue_login(username, user),
                    Err(err) => {
                        crate::sound::play_error();
                        self.login.error = err.to_string();
                    }
                }
            }
            LoginSelectionAction::StartHacking { username } => {
                crate::sound::play_navigate();
                self.login.selected_username = username.clone();
                self.queue_hacking_start(username);
            }
            LoginSelectionAction::ShowError(error) => {
                crate::sound::play_error();
                self.login.error = error;
            }
        }
    }

    fn apply_main_menu_selection_action(&mut self, action: MainMenuSelectionAction) {
        match action {
            MainMenuSelectionAction::OpenScreen {
                screen,
                selected_idx,
                clear_status,
            } => {
                self.navigate_to_screen(screen);
                match screen {
                    TerminalScreen::Applications => self.terminal_apps_idx = selected_idx,
                    TerminalScreen::Documents => self.terminal_documents_idx = selected_idx,
                    TerminalScreen::Logs => self.terminal_logs_idx = selected_idx,
                    TerminalScreen::Network => self.terminal_network_idx = selected_idx,
                    TerminalScreen::Games => self.terminal_games_idx = selected_idx,
                    TerminalScreen::NukeCodes => {}
                    TerminalScreen::ProgramInstaller => {
                        self.terminal_installer.reset();
                        self.terminal_installer.root_idx = selected_idx;
                    }
                    TerminalScreen::Settings => {
                        self.terminal_settings_idx = selected_idx;
                        self.terminal_settings_choice = None;
                    }
                    TerminalScreen::EditMenus => {}
                    TerminalScreen::Connections => {
                        self.terminal_connections.reset();
                        self.terminal_connections.root_idx = selected_idx;
                    }
                    TerminalScreen::DefaultApps => self.terminal_default_apps_idx = selected_idx,
                    TerminalScreen::About => {}
                    TerminalScreen::UserManagement => {
                        self.terminal_user_management_mode = UserManagementMode::Root;
                        self.terminal_user_management_idx = selected_idx;
                    }
                    TerminalScreen::DocumentBrowser => self.terminal_browser_idx = selected_idx,
                    TerminalScreen::MainMenu => self.main_menu_idx = selected_idx,
                    TerminalScreen::PtyApp => {}
                }
                if clear_status {
                    self.shell_status.clear();
                }
            }
            MainMenuSelectionAction::OpenTerminalMode => {
                self.open_embedded_terminal_shell();
            }
            MainMenuSelectionAction::EnterDesktopMode => {
                crate::sound::play_navigate();
                self.desktop_mode_open = true;
                self.close_start_menu();
                self.sync_desktop_active_window();
                self.shell_status = "Entered Desktop Mode.".to_string();
            }
            MainMenuSelectionAction::RefreshSettingsAndOpen => {
                self.settings.draft = current_settings();
                self.navigate_to_screen(TerminalScreen::Settings);
                self.terminal_settings_idx = 0;
                self.terminal_connections.reset();
                self.terminal_default_app_slot = None;
                self.shell_status.clear();
            }
            MainMenuSelectionAction::BeginLogout => self.begin_logout(),
        }
    }

    fn handle_terminal_back(&mut self) {
        if self.terminal_settings_choice.is_some() {
            crate::sound::play_navigate();
            self.terminal_settings_choice = None;
            return;
        }
        if self.terminal_default_app_slot.is_some() {
            crate::sound::play_navigate();
            self.terminal_default_app_slot = None;
            return;
        }
        if matches!(self.terminal_screen, TerminalScreen::Connections)
            && !self.terminal_connections.back()
        {
            crate::sound::play_navigate();
            self.shell_status.clear();
            return;
        }
        if matches!(self.terminal_screen, TerminalScreen::ProgramInstaller)
            && !self.terminal_installer.back()
        {
            crate::sound::play_navigate();
            self.shell_status.clear();
            return;
        }
        match self.terminal_screen {
            TerminalScreen::MainMenu => {}
            TerminalScreen::Applications
            | TerminalScreen::Documents
            | TerminalScreen::Network
            | TerminalScreen::Games
            | TerminalScreen::Settings
            | TerminalScreen::UserManagement => {
                self.navigate_to_screen(TerminalScreen::MainMenu);
                self.shell_status.clear();
            }
            TerminalScreen::Logs => {
                self.navigate_to_screen(TerminalScreen::Documents);
                self.shell_status.clear();
            }
            TerminalScreen::PtyApp => {
                if let Some(mut pty) = self.terminal_pty.take() {
                    pty.session.terminate();
                    self.navigate_to_screen(pty.return_screen);
                    self.shell_status = format!("Closed {}.", pty.title);
                } else {
                    self.navigate_to_screen(TerminalScreen::MainMenu);
                    self.shell_status.clear();
                }
            }
            TerminalScreen::ProgramInstaller => {
                self.navigate_to_screen(TerminalScreen::MainMenu);
                self.shell_status.clear();
                self.terminal_installer.reset();
            }
            TerminalScreen::Connections
            | TerminalScreen::DefaultApps
            | TerminalScreen::About
            | TerminalScreen::EditMenus => {
                self.navigate_to_screen(TerminalScreen::Settings);
                self.shell_status.clear();
            }
            TerminalScreen::NukeCodes => {
                self.navigate_to_screen(self.terminal_nuke_codes_return);
                self.shell_status.clear();
            }
            TerminalScreen::DocumentBrowser => {
                self.navigate_to_screen(self.terminal_browser_return);
                self.shell_status.clear();
            }
        }
    }

    fn handle_terminal_prompt_input(&mut self, ctx: &Context) {
        let Some(prompt) = self.terminal_prompt.clone() else {
            return;
        };
        let prompt_action = prompt.action.clone();
        match handle_prompt_input(ctx, prompt) {
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
                self.do_login();
                if self.session.is_none() && self.terminal_flash.is_none() {
                    self.open_password_prompt(
                        "Password Prompt",
                        format!("Password for {}", self.login.selected_username),
                    );
                }
            }
            PromptOutcome::CreateUsername(raw_username) => {
                let username = raw_username.trim().to_string();
                self.terminal_prompt = None;
                if username.is_empty() {
                    self.shell_status = "Username cannot be empty.".to_string();
                    return;
                }
                let db = load_users();
                if db.contains_key(&username) {
                    self.shell_status = "User already exists.".to_string();
                    return;
                }
                self.set_user_management_mode(UserManagementMode::CreateAuthMethod { username }, 0);
                self.suppress_next_menu_submit = true;
            }
            PromptOutcome::CreatePasswordFirst { username, password } => {
                self.terminal_prompt = None;
                if password.is_empty() {
                    self.shell_status = "Password cannot be empty.".to_string();
                    return;
                }
                self.open_password_prompt_with_action(
                    "Confirm Password",
                    format!("Re-enter password for {username}"),
                    TerminalPromptAction::CreatePasswordConfirm {
                        username,
                        first_password: password,
                    },
                );
            }
            PromptOutcome::CreatePasswordConfirm {
                username,
                first_password,
                confirmation,
            } => {
                self.terminal_prompt = None;
                if confirmation != first_password {
                    self.shell_status = "Passwords do not match.".to_string();
                    return;
                }
                self.save_user_and_status(
                    &username,
                    UserRecord {
                        password_hash: crate::core::auth::hash_password(&first_password),
                        is_admin: false,
                        auth_method: crate::core::auth::AuthMethod::Password,
                    },
                    format!("User '{username}' created."),
                );
                self.set_user_management_mode(UserManagementMode::Root, 0);
            }
            PromptOutcome::ResetPasswordFirst { username, password } => {
                self.terminal_prompt = None;
                if password.is_empty() {
                    self.shell_status = "Password cannot be empty.".to_string();
                    return;
                }
                self.open_password_prompt_with_action(
                    "Confirm Password",
                    format!("Re-enter password for {username}"),
                    TerminalPromptAction::ResetPasswordConfirm {
                        username,
                        first_password: password,
                    },
                );
            }
            PromptOutcome::ResetPasswordConfirm {
                username,
                first_password,
                confirmation,
            } => {
                self.terminal_prompt = None;
                if confirmation != first_password {
                    self.shell_status = "Passwords do not match.".to_string();
                    return;
                }
                self.update_user_record(
                    &username,
                    |record| {
                        record.password_hash = crate::core::auth::hash_password(&first_password);
                        record.auth_method = crate::core::auth::AuthMethod::Password;
                    },
                    "Password updated.".to_string(),
                );
                self.set_user_management_mode(UserManagementMode::Root, 0);
            }
            PromptOutcome::ChangeAuthPasswordFirst { username, password } => {
                self.terminal_prompt = None;
                if password.is_empty() {
                    self.shell_status = "Password cannot be empty.".to_string();
                    return;
                }
                self.open_password_prompt_with_action(
                    "Confirm Password",
                    format!("Re-enter password for {username}"),
                    TerminalPromptAction::ChangeAuthPasswordConfirm {
                        username,
                        first_password: password,
                    },
                );
            }
            PromptOutcome::ChangeAuthPasswordConfirm {
                username,
                first_password,
                confirmation,
            } => {
                self.terminal_prompt = None;
                if confirmation != first_password {
                    self.shell_status = "Passwords do not match.".to_string();
                    return;
                }
                self.update_user_record(
                    &username,
                    |record| {
                        record.password_hash = crate::core::auth::hash_password(&first_password);
                        record.auth_method = crate::core::auth::AuthMethod::Password;
                    },
                    format!("Auth method updated for '{username}'."),
                );
                self.set_user_management_mode(UserManagementMode::Root, 0);
            }
            PromptOutcome::ConfirmDeleteUser {
                username,
                confirmed,
            } => {
                self.terminal_prompt = None;
                if confirmed {
                    let mut db = load_users();
                    db.remove(&username);
                    save_users(&db);
                    self.shell_status = format!("User '{username}' deleted.");
                }
                self.set_user_management_mode(UserManagementMode::Root, 0);
            }
            PromptOutcome::ConfirmToggleAdmin {
                username,
                confirmed,
            } => {
                self.terminal_prompt = None;
                if confirmed {
                    let mut db = load_users();
                    if let Some(record) = db.get_mut(&username) {
                        record.is_admin = !record.is_admin;
                        let label = if record.is_admin {
                            "granted"
                        } else {
                            "revoked"
                        };
                        save_users(&db);
                        self.shell_status = format!("Admin {label} for '{username}'.");
                    }
                }
                self.set_user_management_mode(UserManagementMode::Root, 0);
            }
            PromptOutcome::EditMenuAddProgramName { target, name } => {
                self.terminal_prompt = None;
                let name = name.trim().to_string();
                if name.is_empty() {
                    self.shell_status = "Error: Invalid input.".to_string();
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
                    self.shell_status = "Error: Invalid input.".to_string();
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
                    self.shell_status = "Error: Invalid input.".to_string();
                    return;
                }
                self.add_document_category(name, path);
            }
            PromptOutcome::FileManagerRename { path, name } => {
                self.terminal_prompt = None;
                if self.file_manager.selected.as_ref() != Some(&path) {
                    self.file_manager.select(Some(path));
                }
                match self.file_manager_rename_selected(name) {
                    Ok(message) => self.shell_status = message,
                    Err(err) => self.shell_status = format!("File action failed: {err}"),
                }
            }
            PromptOutcome::FileManagerMoveTo { path, destination } => {
                self.terminal_prompt = None;
                if self.file_manager.selected.as_ref() != Some(&path) {
                    self.file_manager.select(Some(path));
                }
                match self.file_manager_move_selected(destination) {
                    Ok(message) => self.shell_status = message,
                    Err(err) => self.shell_status = format!("File action failed: {err}"),
                }
            }
            PromptOutcome::FileManagerOpenWithNewCommand {
                path,
                ext_key,
                make_default,
                command,
            } => {
                self.terminal_prompt = None;
                let command = command.trim().to_string();
                if command.is_empty() {
                    self.shell_status = "Open With canceled.".to_string();
                    return;
                }
                if parse_custom_command_line(&command).is_none() {
                    self.shell_status = "Error: invalid command line".to_string();
                    return;
                }
                match self.launch_open_with_command(&path, &command) {
                    Ok(message) => {
                        self.record_open_with_command(&ext_key, &command);
                        if make_default {
                            self.set_open_with_default_command(&ext_key, Some(command.as_str()));
                        }
                        self.shell_status = message;
                    }
                    Err(err) => self.shell_status = format!("Open failed: {err}"),
                }
            }
            PromptOutcome::FileManagerOpenWithEditCommand {
                path,
                ext_key,
                previous,
                command,
            } => {
                self.terminal_prompt = None;
                let command = command.trim().to_string();
                if command.is_empty() {
                    self.shell_status = "Edited command cannot be empty.".to_string();
                    return;
                }
                if parse_custom_command_line(&command).is_none() {
                    self.shell_status = "Error: invalid command line".to_string();
                    return;
                }
                self.replace_open_with_command(&ext_key, &previous, &command);
                if self.file_manager.selected.as_ref() != Some(&path) {
                    self.file_manager.select(Some(path));
                }
                self.shell_status = format!(
                    "Updated saved command for {}.",
                    Self::open_with_extension_label(&ext_key)
                );
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
                    self.shell_status = "Cancelled.".to_string();
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
                match apply_default_app_custom_command(slot, &raw) {
                    DefaultAppsEvent::SetBinding { slot, binding } => {
                        set_binding_for_slot(&mut self.settings.draft, slot, binding);
                        self.persist_native_settings();
                    }
                    DefaultAppsEvent::Status(status) => {
                        self.shell_status = status;
                    }
                    _ => {}
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
                self.apply_installer_event(event);
            }
            PromptOutcome::ConfirmInstallerAction {
                pkg,
                action,
                confirmed,
            } => {
                self.terminal_prompt = None;
                if confirmed {
                    let event = build_package_command(&self.terminal_installer, &pkg, action);
                    self.apply_installer_event(event);
                } else {
                    self.shell_status = "Cancelled.".to_string();
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
                self.apply_connections_event(event);
            }
            PromptOutcome::ConnectionPassword {
                kind,
                name,
                detail,
                password,
            } => {
                self.terminal_prompt = None;
                if matches!(kind, ConnectionKind::Network)
                    && network_requires_password(&detail)
                    && password.trim().is_empty()
                {
                    self.shell_status = "Cancelled.".to_string();
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
        match connect_connection(
            kind,
            &target.name,
            Some(target.detail.as_str()),
            password.as_deref(),
        ) {
            Ok(msg) => self.shell_status = msg,
            Err(err) => self.shell_status = err.to_string(),
        }
    }

    fn draw_login(&mut self, ctx: &Context) {
        let layout = self.terminal_layout();
        match self.login_mode {
            LoginScreenMode::SelectUser => {
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
                    let action = resolve_login_selection(self.login.selected_idx, &usernames);
                    self.apply_login_selection_action(action);
                }
            }
            LoginScreenMode::Hacking => {
                let Some(hacking) = self.login_hacking.as_mut() else {
                    crate::sound::play_navigate();
                    self.login_mode = LoginScreenMode::SelectUser;
                    return;
                };
                match draw_hacking_screen(
                    ctx,
                    &mut hacking.game,
                    layout.cols,
                    layout.rows,
                    layout.status_row,
                    layout.status_row_alt,
                ) {
                    HackingScreenEvent::None => {}
                    HackingScreenEvent::Cancel => {
                        self.login_mode = LoginScreenMode::SelectUser;
                        self.login_hacking = None;
                    }
                    HackingScreenEvent::Success => {
                        let username = hacking.username.clone();
                        let db = load_users();
                        if let Some(user) = db.get(&username).cloned() {
                            self.queue_login(username, user);
                        } else {
                            crate::sound::play_error();
                            self.login.error = "Unknown user.".to_string();
                            crate::sound::play_navigate();
                            self.login_mode = LoginScreenMode::SelectUser;
                            self.login_hacking = None;
                        }
                    }
                    HackingScreenEvent::LockedOut => {
                        crate::sound::play_navigate();
                        self.login_mode = LoginScreenMode::Locked;
                        self.login_hacking = None;
                    }
                    HackingScreenEvent::ExitLocked => {}
                }
            }
            LoginScreenMode::Locked => {
                if matches!(
                    draw_locked_screen(ctx, layout.cols, layout.rows, layout.status_row_alt),
                    HackingScreenEvent::ExitLocked
                ) {
                    self.login_mode = LoginScreenMode::SelectUser;
                    self.login_hacking = None;
                }
            }
        }
    }

    fn draw_top_bar(&mut self, ctx: &Context) {
        Self::apply_global_retro_menu_chrome(ctx);
        let app_menu_name = self.desktop_app_menu_name();
        TopBottomPanel::top("native_top_bar")
            .exact_height(30.0)
            .show_separator_line(false)
            .show(ctx, |ui| {
                let palette = current_palette();
                ui.painter()
                    .rect_filled(ui.max_rect(), 0.0, palette.selected_bg);
                ui.horizontal(|ui| {
                    Self::apply_top_bar_menu_button_style(ui);
                    ui.spacing_mut().item_spacing.x = 14.0;
                    ui.menu_button(
                        RichText::new(app_menu_name.clone())
                            .strong()
                            .color(Color32::BLACK),
                        |ui| {
                        Self::apply_top_dropdown_menu_style(ui);
                        if let Some(window) = self.desktop_active_window {
                            if ui.button("Close Focused").clicked() {
                                self.close_desktop_window(window);
                                ui.close_menu();
                            }
                        } else {
                            ui.label("No active app");
                        }
                        if ui.button("Minimize").clicked() {
                            if let Some(window) = self.desktop_active_window {
                                self.set_desktop_window_minimized(window, true);
                            }
                            ui.close_menu();
                        }
                    });
                    ui.add_space(10.0);
                    ui.menu_button("File", |ui| {
                        Self::apply_top_dropdown_menu_style(ui);
                        let file_manager_active =
                            self.desktop_active_window == Some(DesktopWindow::FileManager);
                        if file_manager_active {
                            let has_extra_tabs = self.file_manager.tabs.len() > 1;
                            let has_selection = self.file_manager.selected_row().is_some();
                            let has_file_selection = self.file_manager_selected_file().is_some();
                            if ui.button("New Tab").clicked() {
                                self.file_manager.open_tab_here();
                                ui.close_menu();
                            }
                            if has_extra_tabs {
                                if ui.button("Previous Tab").clicked() {
                                    let idx = if self.file_manager.active_tab == 0 {
                                        self.file_manager.tabs.len().saturating_sub(1)
                                    } else {
                                        self.file_manager.active_tab - 1
                                    };
                                    let _ = self.file_manager.switch_to_tab(idx);
                                    ui.close_menu();
                                }
                                if ui.button("Next Tab").clicked() {
                                    let idx = (self.file_manager.active_tab + 1)
                                        % self.file_manager.tabs.len().max(1);
                                    let _ = self.file_manager.switch_to_tab(idx);
                                    ui.close_menu();
                                }
                                if ui.button("Close Tab").clicked() {
                                    let _ = self.file_manager.close_active_tab();
                                    ui.close_menu();
                                }
                            }
                            if has_selection {
                                if ui.button("Open Selected").clicked() {
                                    self.activate_file_manager_selection();
                                    ui.close_menu();
                                }
                            }
                            if has_file_selection {
                                ui.menu_button("Open With", |ui| {
                                    Self::apply_top_dropdown_menu_style(ui);
                                    let Some(entry) = self.file_manager_selected_file() else {
                                        return;
                                    };
                                    let ext_key = Self::open_with_extension_key(&entry.path);
                                    let ext_label = Self::open_with_extension_label(&ext_key);
                                    let saved = self.open_with_history_for_extension(&ext_key);
                                    let current_default =
                                        self.open_with_default_for_extension(&ext_key);

                                    for command in &saved {
                                        let is_default =
                                            current_default.as_deref() == Some(command.as_str());
                                        let label = if is_default {
                                            format!("Use: {command} [default]")
                                        } else {
                                            format!("Use: {command}")
                                        };
                                        if ui.button(label).clicked() {
                                            match self.launch_open_with_command(&entry.path, command) {
                                                Ok(message) => {
                                                    self.record_open_with_command(&ext_key, command);
                                                    self.shell_status = message;
                                                }
                                                Err(err) => {
                                                    self.shell_status = format!("Open failed: {err}");
                                                }
                                            }
                                            ui.close_menu();
                                        }
                                    }
                                    if !saved.is_empty() {
                                        Self::retro_separator(ui);
                                    }
                                    if ui.button("New Command...").clicked() {
                                        self.open_file_manager_open_with_new_command_prompt(
                                            entry.path.clone(),
                                            ext_key.clone(),
                                            false,
                                        );
                                        ui.close_menu();
                                    }
                                    if ui
                                        .button(format!(
                                            "New Command + Always Use for {}",
                                            ext_label
                                        ))
                                        .clicked()
                                    {
                                        self.open_file_manager_open_with_new_command_prompt(
                                            entry.path.clone(),
                                            ext_key.clone(),
                                            true,
                                        );
                                        ui.close_menu();
                                    }
                                    if !saved.is_empty() {
                                        ui.menu_button("Edit", |ui| {
                                            Self::apply_top_dropdown_menu_style(ui);
                                            for command in &saved {
                                                let is_default =
                                                    current_default.as_deref() == Some(command.as_str());
                                                ui.menu_button(command, |ui| {
                                                    Self::apply_top_dropdown_menu_style(ui);
                                                    let default_label = if is_default {
                                                        "Stop Always Using"
                                                    } else {
                                                        "Always Use"
                                                    };
                                                    if ui.button(default_label).clicked() {
                                                        if is_default {
                                                            self.set_open_with_default_command(
                                                                &ext_key,
                                                                None,
                                                            );
                                                            self.shell_status = format!(
                                                                "Cleared always-use command for {}.",
                                                                ext_label
                                                            );
                                                        } else {
                                                            self.set_open_with_default_command(
                                                                &ext_key,
                                                                Some(command.as_str()),
                                                            );
                                                            self.shell_status = format!(
                                                                "Now always using {} for {}.",
                                                                command, ext_label
                                                            );
                                                        }
                                                        ui.close_menu();
                                                    }
                                                    if ui.button("Edit Saved").clicked() {
                                                        self.open_file_manager_open_with_edit_command_prompt(
                                                            entry.path.clone(),
                                                            ext_key.clone(),
                                                            command.clone(),
                                                        );
                                                        ui.close_menu();
                                                    }
                                                    if ui.button("Remove Saved").clicked() {
                                                        self.remove_open_with_command(&ext_key, command);
                                                        self.shell_status = format!(
                                                            "Removed saved command for {}.",
                                                            ext_label
                                                        );
                                                        ui.close_menu();
                                                    }
                                                });
                                            }
                                            if current_default.is_some() {
                                                Self::retro_separator(ui);
                                                if ui.button("Clear Always Use").clicked() {
                                                    self.set_open_with_default_command(&ext_key, None);
                                                    self.shell_status = format!(
                                                        "Cleared always-use command for {}.",
                                                        ext_label
                                                    );
                                                    ui.close_menu();
                                                }
                                            }
                                        });
                                    }
                                });
                            }
                            if ui.button("Home").clicked() {
                                self.file_manager_open_home();
                                ui.close_menu();
                            }
                            Self::retro_separator(ui);
                        }
                        if ui.button("Applications").clicked() {
                            self.open_desktop_window(DesktopWindow::Applications);
                            ui.close_menu();
                        }
                        if ui.button("Documents").clicked() {
                            if let Some(session) = &self.session {
                                self.open_file_manager_at(word_processor_dir(&session.username));
                            }
                            ui.close_menu();
                        }
                        if ui.button("Logs").clicked() {
                            self.open_file_manager_at(logs_dir());
                            ui.close_menu();
                        }
                        if ui.button("Network").clicked() {
                            self.open_start_menu();
                            self.set_start_panel_for_root(2);
                            ui.close_menu();
                        }
                        if ui.button("Games").clicked() {
                            self.open_start_menu();
                            self.set_start_panel_for_root(3);
                            ui.close_menu();
                        }
                        if ui.button("Program Installer").clicked() {
                            self.run_start_system_action(StartSystemAction::ProgramInstaller);
                            ui.close_menu();
                        }
                        if ui.button("Settings").clicked() {
                            self.open_desktop_window(DesktopWindow::Settings);
                            ui.close_menu();
                        }
                        Self::retro_separator(ui);
                        if ui.button("Open Start Menu").clicked() {
                            self.open_start_menu();
                            ui.close_menu();
                        }
                        if ui.button("My Computer").clicked() {
                            self.open_desktop_window(DesktopWindow::FileManager);
                            ui.close_menu();
                        }
                    });
                    ui.menu_button("Edit", |ui| {
                        Self::apply_top_dropdown_menu_style(ui);
                        if self.desktop_active_window == Some(DesktopWindow::Editor) {
                            if ui.button("Save").clicked() {
                                self.save_editor();
                                ui.close_menu();
                            }
                            if ui.button("New Document").clicked() {
                                self.new_document();
                                ui.close_menu();
                            }
                            if ui.button("Open File Manager").clicked() {
                                self.open_desktop_window(DesktopWindow::FileManager);
                                ui.close_menu();
                            }
                        } else if self.desktop_active_window == Some(DesktopWindow::FileManager) {
                            let has_selection = self.file_manager.selected_row().is_some();
                            let has_clipboard = self
                                .file_clipboard
                                .as_ref()
                                .is_some_and(|clip| !clip.paths.is_empty());
                            let has_history =
                                !self.file_undo_stack.is_empty() || !self.file_redo_stack.is_empty();
                            let paste_label = if let Some(clip) = &self.file_clipboard {
                                let mode = if matches!(clip.mode, FileManagerClipboardMode::Cut) {
                                    "Move"
                                } else {
                                    "Paste"
                                };
                                if clip.paths.len() == 1 {
                                    format!("{mode} {}", Self::path_display_name(&clip.paths[0]))
                                } else {
                                    format!("{mode} {} items", clip.paths.len())
                                }
                            } else {
                                "Paste".to_string()
                            };
                            if ui.button("Open Selected").clicked() {
                                self.activate_file_manager_selection();
                                ui.close_menu();
                            }
                            if ui.button("Clear Search").clicked() {
                                self.file_manager.update_search_query(String::new());
                                ui.close_menu();
                            }
                            if has_selection || has_clipboard || has_history {
                                Self::retro_separator(ui);
                            }
                            if has_selection {
                                if ui.button("Copy").clicked() {
                                    self.run_file_manager_action(FileManagerActionRequest::Copy);
                                    ui.close_menu();
                                }
                                if ui.button("Cut").clicked() {
                                    self.run_file_manager_action(FileManagerActionRequest::Cut);
                                    ui.close_menu();
                                }
                                if ui.button("Duplicate").clicked() {
                                    self.run_file_manager_action(FileManagerActionRequest::Duplicate);
                                    ui.close_menu();
                                }
                                if ui.button("Rename").clicked() {
                                    self.open_file_manager_rename_prompt();
                                    ui.close_menu();
                                }
                                if ui.button("Move To").clicked() {
                                    self.open_file_manager_move_prompt();
                                    ui.close_menu();
                                }
                                if ui.button("Delete").clicked() {
                                    self.run_file_manager_action(FileManagerActionRequest::Delete);
                                    ui.close_menu();
                                }
                            }
                            if has_clipboard && ui.button(paste_label).clicked() {
                                self.run_file_manager_action(FileManagerActionRequest::Paste);
                                ui.close_menu();
                            }
                            if !self.file_undo_stack.is_empty() && ui.button("Undo").clicked() {
                                self.run_file_manager_action(FileManagerActionRequest::Undo);
                                ui.close_menu();
                            }
                            if !self.file_redo_stack.is_empty() && ui.button("Redo").clicked() {
                                self.run_file_manager_action(FileManagerActionRequest::Redo);
                                ui.close_menu();
                            }
                            if has_selection || has_clipboard || has_history {
                                Self::retro_separator(ui);
                            }
                            if ui.button("New Document").clicked() {
                                self.new_document();
                                ui.close_menu();
                            }
                        } else {
                            let _ = Self::retro_disabled_button(ui, "No edit actions");
                        }
                    });
                    ui.menu_button("View", |ui| {
                        Self::apply_top_dropdown_menu_style(ui);
                        let file_manager_active =
                            self.desktop_active_window == Some(DesktopWindow::FileManager);
                        if file_manager_active {
                            let show_tree = get_settings().desktop_file_manager.show_tree_panel;
                            let show_hidden = get_settings().desktop_file_manager.show_hidden_files;
                            let view_mode = self.file_manager_view_mode();
                            let sort_mode = self.file_manager_sort_mode();
                            if ui
                                .button(if show_tree {
                                    "Hide Folder Tree"
                                } else {
                                    "Show Folder Tree"
                                })
                                .clicked()
                            {
                                self.toggle_file_manager_tree_panel();
                                ui.close_menu();
                            }
                            if ui
                                .button(if view_mode == FileManagerViewMode::Grid {
                                    "Grid View [Active]"
                                } else {
                                    "Grid View"
                                })
                                .clicked()
                            {
                                self.set_file_manager_view_mode(FileManagerViewMode::Grid);
                                ui.close_menu();
                            }
                            if ui
                                .button(if view_mode == FileManagerViewMode::List {
                                    "List View [Active]"
                                } else {
                                    "List View"
                                })
                                .clicked()
                            {
                                self.set_file_manager_view_mode(FileManagerViewMode::List);
                                ui.close_menu();
                            }
                            if ui
                                .button(if sort_mode == FileManagerSortMode::Name {
                                    "Sort By Name [Active]"
                                } else {
                                    "Sort By Name"
                                })
                                .clicked()
                            {
                                self.set_file_manager_sort_mode(FileManagerSortMode::Name);
                                ui.close_menu();
                            }
                            if ui
                                .button(if sort_mode == FileManagerSortMode::Type {
                                    "Sort By Type [Active]"
                                } else {
                                    "Sort By Type"
                                })
                                .clicked()
                            {
                                self.set_file_manager_sort_mode(FileManagerSortMode::Type);
                                ui.close_menu();
                            }
                            if ui
                                .button(if show_hidden {
                                    "Hide Hidden Files"
                                } else {
                                    "Show Hidden Files"
                                })
                                .clicked()
                            {
                                self.toggle_file_manager_hidden_files();
                                ui.close_menu();
                            }
                            Self::retro_separator(ui);
                        }
                        if ui.button("My Computer").clicked() {
                            self.open_desktop_window(DesktopWindow::FileManager);
                            ui.close_menu();
                        }
                        if ui.button("Toggle Start Menu").clicked() {
                            if self.start_open {
                                self.close_start_menu();
                            } else {
                                self.open_start_menu();
                            }
                            ui.close_menu();
                        }
                        if ui.button("Settings").clicked() {
                            self.open_desktop_window(DesktopWindow::Settings);
                            ui.close_menu();
                        }
                    });
                    ui.menu_button("Window", |ui| {
                        Self::apply_top_dropdown_menu_style(ui);
                        for window in [
                            DesktopWindow::FileManager,
                            DesktopWindow::Editor,
                            DesktopWindow::Settings,
                            DesktopWindow::Applications,
                            DesktopWindow::NukeCodes,
                            DesktopWindow::PtyApp,
                        ] {
                            let open = self.desktop_window_is_open(window);
                            let active = self.desktop_active_window == Some(window);
                            let marker = if active {
                                "active"
                            } else if open {
                                "open"
                            } else {
                                "closed"
                            };
                            let label = format!("{marker}: {}", self.desktop_window_title(window));
                            if ui.button(label).clicked() {
                                if window == DesktopWindow::Editor
                                    && !self.desktop_window_is_open(DesktopWindow::Editor)
                                    && self.editor.path.is_none()
                                {
                                    self.new_document();
                                } else if !open {
                                    self.open_desktop_window(window);
                                } else {
                                    self.desktop_active_window = Some(window);
                                    self.close_start_menu();
                                }
                                ui.close_menu();
                            }
                        }
                    });
                    ui.menu_button("Help", |ui| {
                        Self::apply_top_dropdown_menu_style(ui);
                        if ui.button("App Manual").clicked() {
                            self.open_manual_file("README.md", "App Manual");
                            ui.close_menu();
                        }
                        if ui.button("User Manual").clicked() {
                            self.open_manual_file("USER_MANUAL.md", "User Manual");
                            ui.close_menu();
                        }
                    });
                    ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                        let now = Local::now().format("%a %d %b %H:%M").to_string();
                        ui.label(RichText::new(now).color(Color32::BLACK));
                    });
                });
            });
    }

    fn draw_start_panel(&mut self, ctx: &Context) {
        if !self.start_open {
            return;
        }
        const ROOT_W: f32 = 230.0;
        const SUB_W: f32 = 190.0;
        const LEAF_W: f32 = 210.0;
        const ROW_H: f32 = 22.0;
        const TITLE_H: f32 = 42.0;
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
                                let response = ui.selectable_label(
                                    selected,
                                    RichText::new(format!(" {label}{suffix}")).color(if selected {
                                        Color32::BLACK
                                    } else {
                                        palette.fg
                                    }),
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
                branch_anchor_y = frame_response.response.rect.top() + TITLE_H;
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
                                    let response = ui.selectable_label(
                                        selected,
                                        RichText::new(format!(" {label}")).color(if selected {
                                            Color32::BLACK
                                        } else {
                                            palette.fg
                                        }),
                                    );
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
                                let response = ui.selectable_label(
                                    selected,
                                    RichText::new(format!(" {}", item.label)).color(if selected {
                                        Color32::BLACK
                                    } else {
                                        palette.fg
                                    }),
                                );
                                if response.hovered() {
                                    self.start_leaf_selected = idx;
                                }
                                if response.clicked() {
                                    self.run_start_leaf_action(item.action.clone());
                                }
                                if matches!(leaf, StartLeaf::Applications | StartLeaf::Games | StartLeaf::Network)
                                    && !matches!(item.action, StartLeafAction::None)
                                {
                                    let app_name = item.label.clone();
                                    response.context_menu(|ui| {
                                        Self::apply_context_menu_style(ui);
                                        ui.set_min_width(136.0);
                                        ui.set_max_width(180.0);
                                        if ui.button("Create Shortcut").clicked() {
                                            leaf_context_action = Some(ContextMenuAction::CreateShortcut(app_name.clone()));
                                            ui.close_menu();
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

    fn draw_desktop(&mut self, ctx: &Context) {
        if self.asset_cache.is_none() {
            self.asset_cache = Some(Self::build_asset_cache(ctx));
        }
        self.sync_wallpaper(ctx);
        egui::CentralPanel::default()
            .frame(
                egui::Frame::none()
                    .fill(current_palette().bg)
                    .inner_margin(0.0),
            )
            .show(ctx, |ui| {
                let rect = ui.max_rect();
                let response = ui.allocate_rect(rect, egui::Sense::click());
                if !self.draw_wallpaper(ui.painter(), rect) {
                    ui.painter().rect_filled(rect, 0.0, current_palette().bg);
                }
                if !matches!(self.settings.draft.desktop_icon_style, DesktopIconStyle::NoIcons) {
                    self.draw_desktop_icons(ui);
                }
                Self::attach_desktop_empty_context_menu(
                    &mut self.context_menu_action,
                    &response,
                    self.settings.draft.desktop_snap_to_grid,
                    self.settings.draft.desktop_icon_sort,
                );
                if response.clicked() {
                    self.close_start_menu();
                }
            });
    }

    fn draw_desktop_taskbar(&mut self, ctx: &Context) {
        const WINDOW_ORDER: [DesktopWindow; 6] = [
            DesktopWindow::FileManager,
            DesktopWindow::Editor,
            DesktopWindow::Settings,
            DesktopWindow::Applications,
            DesktopWindow::NukeCodes,
            DesktopWindow::PtyApp,
        ];
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
                    for window in WINDOW_ORDER {
                        if !self.desktop_window_is_open(window) {
                            continue;
                        }
                        let label = self.desktop_taskbar_label(window);
                        // Taskbar chrome renders the "active" look in the inverse branch.
                        let active = self.desktop_active_window != Some(window);
                        if Self::desktop_bar_button(ui, label, active, false).clicked() {
                            let opening_editor = window == DesktopWindow::Editor
                                && !self.desktop_window_is_open(DesktopWindow::Editor)
                                && self.editor.path.is_none();
                            if opening_editor {
                                self.new_document();
                            } else {
                                self.handle_desktop_taskbar_window_click(window);
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
            &mut self.main_menu_idx,
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
        let items = self.terminal_app_items();
        let mut selected = self.terminal_apps_idx.min(items.len().saturating_sub(1));
        let activated = draw_terminal_menu_screen(
            ctx,
            "Applications",
            Some("Built-in and configured apps"),
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
        self.terminal_apps_idx = selected;
        if let Some(idx) = activated {
            let label = &items[idx];
            if label == BUILTIN_TEXT_EDITOR_APP {
                self.editor.open = true;
                if self.editor.path.is_none() {
                    self.new_document();
                }
                self.shell_status = format!("Opened {BUILTIN_TEXT_EDITOR_APP}.");
            } else if label == BUILTIN_NUKE_CODES_APP {
                self.open_nuke_codes_screen(TerminalScreen::Applications);
            } else if label == "Back" {
                self.navigate_to_screen(TerminalScreen::MainMenu);
                self.shell_status.clear();
            } else {
                self.launch_configured_app_in_pty(label, TerminalScreen::Applications);
            }
        }
    }

    fn launch_configured_app_in_pty(&mut self, name: &str, return_screen: TerminalScreen) {
        let apps = load_apps();
        match resolve_program_command(name, &apps) {
            Ok(cmd) => self.open_embedded_pty(name, &cmd, return_screen),
            Err(err) => self.shell_status = err,
        }
    }

    fn open_nuke_codes_screen(&mut self, return_screen: TerminalScreen) {
        self.terminal_nuke_codes = fetch_nuke_codes();
        self.terminal_nuke_codes_return = return_screen;
        self.navigate_to_screen(TerminalScreen::NukeCodes);
        self.shell_status.clear();
    }

    fn draw_terminal_documents(&mut self, ctx: &Context) {
        let layout = self.terminal_layout();
        let mut items = vec!["Logs".to_string()];
        items.extend(Self::sorted_document_categories());
        items.push("---".to_string());
        items.push("Back".to_string());
        let mut selected = self
            .terminal_documents_idx
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
        self.terminal_documents_idx = selected;
        if let Some(idx) = activated {
            let selected = items[idx].as_str();
            match selected {
                "Logs" => {
                    self.navigate_to_screen(TerminalScreen::Logs);
                    self.terminal_logs_idx = 0;
                    self.shell_status.clear();
                }
                "Back" => {
                    self.navigate_to_screen(TerminalScreen::MainMenu);
                    self.shell_status.clear();
                }
                "---" => {}
                category => {
                    let categories = load_categories();
                    let Some(path_str) = categories.get(category).and_then(|v| v.as_str()) else {
                        self.shell_status = format!("Error: invalid category '{category}'.");
                        return;
                    };
                    self.open_document_browser_at(
                        PathBuf::from(path_str),
                        TerminalScreen::Documents,
                    );
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
        let mut selected = self.terminal_logs_idx.min(items.len().saturating_sub(1));
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
        self.terminal_logs_idx = selected;
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
                    self.shell_status.clear();
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
            &mut self.terminal_browser_idx,
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
            match activate_browser_selection(&mut self.file_manager, self.terminal_browser_idx) {
                FileManagerAction::None => {}
                FileManagerAction::ChangedDir => {
                    self.terminal_browser_idx = 0;
                }
                FileManagerAction::OpenFile(path) => {
                    self.file_manager.select(Some(path));
                    self.activate_file_manager_selection();
                }
            }
        }
    }

    fn draw_terminal_settings(&mut self, ctx: &Context) {
        let layout = self.terminal_layout();
        let event = run_terminal_settings_screen(
            ctx,
            &mut self.settings.draft,
            &mut self.terminal_settings_idx,
            &mut self.terminal_settings_choice,
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
            TerminalSettingsEvent::Persist => self.persist_native_settings(),
            TerminalSettingsEvent::Back => {
                self.navigate_to_screen(TerminalScreen::MainMenu);
                self.shell_status.clear();
            }
            TerminalSettingsEvent::OpenConnections => {
                self.navigate_to_screen(TerminalScreen::Connections);
                self.terminal_connections.reset();
                self.shell_status.clear();
            }
            TerminalSettingsEvent::OpenEditMenus => {
                self.navigate_to_screen(TerminalScreen::EditMenus);
                self.terminal_edit_menus.reset();
                self.shell_status.clear();
            }
            TerminalSettingsEvent::OpenDefaultApps => {
                self.navigate_to_screen(TerminalScreen::DefaultApps);
                self.terminal_default_apps_idx = 0;
                self.terminal_default_app_choice_idx = 0;
                self.terminal_default_app_slot = None;
                self.shell_status.clear();
            }
            TerminalSettingsEvent::OpenAbout => {
                self.navigate_to_screen(TerminalScreen::About);
                self.shell_status.clear();
            }
            TerminalSettingsEvent::EnterUserManagement => {
                self.navigate_to_screen(TerminalScreen::UserManagement);
                self.terminal_user_management_mode = UserManagementMode::Root;
                self.terminal_user_management_idx = 0;
                self.shell_status.clear();
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
            EditMenusEvent::None => {}
            EditMenusEvent::BackToSettings => {
                self.navigate_to_screen(TerminalScreen::Settings);
                self.terminal_settings_idx = 0;
                self.shell_status.clear();
            }
            EditMenusEvent::ToggleBuiltinNukeCodes => {
                self.settings.draft.builtin_menu_visibility.nuke_codes =
                    !self.settings.draft.builtin_menu_visibility.nuke_codes;
                self.persist_native_settings();
            }
            EditMenusEvent::ToggleBuiltinTextEditor => {
                self.settings.draft.builtin_menu_visibility.text_editor =
                    !self.settings.draft.builtin_menu_visibility.text_editor;
                self.persist_native_settings();
            }
            EditMenusEvent::PromptAddProgramName(target) => {
                self.open_input_prompt(
                    format!("Edit {}", target.title()),
                    format!("Enter {} display name:", target.singular()),
                    TerminalPromptAction::EditMenuAddProgramName { target },
                );
            }
            EditMenusEvent::PromptAddCategoryName => {
                self.open_input_prompt(
                    "Edit Documents",
                    "Enter category name:",
                    TerminalPromptAction::EditMenuAddCategoryName,
                );
            }
            EditMenusEvent::ConfirmDeleteProgram { target, name } => {
                self.open_confirm_prompt(
                    format!("Delete {}", target.singular()),
                    format!("Delete '{name}'?"),
                    TerminalPromptAction::ConfirmEditMenuDelete { target, name },
                );
            }
            EditMenusEvent::ConfirmDeleteCategory { name } => {
                self.open_confirm_prompt(
                    "Delete Category",
                    format!("Delete category '{name}'?"),
                    TerminalPromptAction::ConfirmEditMenuDelete {
                        target: EditMenuTarget::Documents,
                        name,
                    },
                );
            }
            EditMenusEvent::Status(status) => {
                self.shell_status = status;
            }
        }
    }

    fn apply_connections_event(&mut self, event: ConnectionsEvent) {
        match event {
            ConnectionsEvent::None => {}
            ConnectionsEvent::BackToSettings => {
                self.navigate_to_screen(TerminalScreen::Settings);
                self.terminal_settings_idx = 0;
                self.shell_status.clear();
            }
            ConnectionsEvent::OpenNetworkGroups => {
                crate::sound::play_navigate();
                self.terminal_connections.view =
                    super::connections_screen::ConnectionsView::NetworkGroups;
                self.shell_status.clear();
            }
            ConnectionsEvent::OpenBluetooth => {
                crate::sound::play_navigate();
                self.terminal_connections.view = super::connections_screen::ConnectionsView::Kind {
                    kind: ConnectionKind::Bluetooth,
                    group: None,
                };
                self.terminal_connections.kind_idx = 0;
                self.shell_status.clear();
            }
            ConnectionsEvent::OpenPromptSearch { kind, group } => {
                self.open_input_prompt(
                    "Connections",
                    "Search query:",
                    TerminalPromptAction::ConnectionSearch { kind, group },
                );
            }
            ConnectionsEvent::OpenPasswordPrompt { kind, target } => {
                self.open_password_prompt_with_action(
                    "Connections",
                    format!("Password for {} (blank cancels)", target.name),
                    TerminalPromptAction::ConnectionPassword {
                        kind,
                        name: target.name,
                        detail: target.detail,
                    },
                );
            }
            ConnectionsEvent::ConnectImmediate { kind, target } => {
                self.connect_target(kind, target, None);
            }
            ConnectionsEvent::Status(status) => {
                if status == crate::connections::macos_connections_disabled_hint() {
                    self.shell_status = status;
                    self.navigate_to_screen(TerminalScreen::Settings);
                } else {
                    self.shell_status = status;
                }
            }
        }
    }

    fn draw_terminal_connections(&mut self, ctx: &Context) {
        let layout = self.terminal_layout();
        let event = draw_connections_screen(
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
        self.apply_connections_event(event);
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
            &mut self.terminal_default_apps_idx,
            &mut self.terminal_default_app_choice_idx,
            &mut self.terminal_default_app_slot,
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
            DefaultAppsEvent::None => {}
            DefaultAppsEvent::Back => {
                self.navigate_to_screen(TerminalScreen::Settings);
                self.terminal_settings_idx = 0;
                self.shell_status.clear();
            }
            DefaultAppsEvent::OpenSlot(slot) => {
                crate::sound::play_navigate();
                self.terminal_default_app_slot = Some(slot);
                self.terminal_default_app_choice_idx = 0;
            }
            DefaultAppsEvent::CloseSlotPicker => {
                crate::sound::play_navigate();
                self.terminal_default_app_slot = None;
            }
            DefaultAppsEvent::SetBinding { slot, binding } => {
                set_binding_for_slot(&mut self.settings.draft, slot, binding);
                self.persist_native_settings();
                self.terminal_default_app_slot = None;
            }
            DefaultAppsEvent::PromptCustom(slot) => {
                self.open_input_prompt(
                    "Default Apps",
                    format!(
                        "{} command (example: epy):",
                        crate::default_apps::slot_label(slot)
                    ),
                    TerminalPromptAction::DefaultAppCustom { slot },
                );
            }
            DefaultAppsEvent::Status(status) => {
                self.shell_status = status;
            }
        }
    }

    fn draw_terminal_about(&mut self, ctx: &Context) {
        let layout = self.terminal_layout();
        if draw_about_screen(
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
            self.navigate_to_screen(TerminalScreen::Settings);
            self.terminal_settings_idx = 0;
            self.shell_status.clear();
        }
    }

    fn draw_terminal_network(&mut self, ctx: &Context) {
        let layout = self.terminal_layout();
        let networks = load_networks();
        let entries: Vec<String> = networks.keys().cloned().collect();
        let event = draw_programs_menu(
            ctx,
            "Network",
            Some("Select Network Program"),
            &entries,
            &mut self.terminal_network_idx,
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
            ProgramMenuEvent::None => {}
            ProgramMenuEvent::Back => {
                self.navigate_to_screen(TerminalScreen::MainMenu);
                self.shell_status.clear();
            }
            ProgramMenuEvent::Launch(name) => match resolve_program_command(&name, &networks) {
                Ok(cmd) => self.open_embedded_pty(&name, &cmd, TerminalScreen::Network),
                Err(err) => self.shell_status = err,
            },
        }
    }

    fn draw_terminal_games(&mut self, ctx: &Context) {
        let layout = self.terminal_layout();
        let games = load_games();
        let entries: Vec<String> = games.keys().cloned().collect();
        let event = draw_programs_menu(
            ctx,
            "Games",
            Some("Select Game"),
            &entries,
            &mut self.terminal_games_idx,
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
            ProgramMenuEvent::None => {}
            ProgramMenuEvent::Back => {
                self.navigate_to_screen(TerminalScreen::MainMenu);
                self.shell_status.clear();
            }
            ProgramMenuEvent::Launch(name) => match resolve_program_command(&name, &games) {
                Ok(cmd) => self.open_embedded_pty(&name, &cmd, TerminalScreen::Games),
                Err(err) => self.shell_status = err,
            },
        }
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
                self.navigate_to_screen(self.terminal_nuke_codes_return);
                self.shell_status.clear();
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
                    self.navigate_to_screen(pty.return_screen);
                    if matches!(pty.return_screen, TerminalScreen::ProgramInstaller) {
                        if let Some(msg) = pty.completion_message {
                            self.queue_terminal_flash_boxed(msg.clone(), 1600, FlashAction::Noop);
                            self.shell_status = msg;
                        } else {
                            self.shell_status = format!("{} exited.", pty.title);
                        }
                    } else {
                        self.shell_status = format!("{} exited.", pty.title);
                    }
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
                self.terminal_installer.reset();
                self.navigate_to_screen(TerminalScreen::MainMenu);
                self.shell_status.clear();
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
        let mode = self.terminal_user_management_mode.clone();
        let screen = user_management_screen_for_mode(
            &mode,
            self.session.as_ref().map(|s| s.username.as_str()),
            get_settings().hacking_difficulty,
        );
        let mut selected = self.terminal_user_management_idx.min(
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
        self.terminal_user_management_idx = selected;
        if let Some(idx) = activated {
            let selected_label = refs[idx].clone();
            match handle_user_management_selection(
                &mode,
                &selected_label,
                self.session.as_ref().map(|s| s.username.as_str()),
            ) {
                UserManagementAction::None => {}
                UserManagementAction::OpenCreateUserPrompt => self.open_input_prompt(
                    "Create User",
                    "New username:",
                    TerminalPromptAction::CreateUsername,
                ),
                UserManagementAction::CycleHackingDifficulty => {
                    update_settings(|s| {
                        s.hacking_difficulty = cycle_hacking_difficulty(s.hacking_difficulty, true);
                    });
                    persist_settings();
                    self.shell_status = "Settings saved.".to_string();
                }
                UserManagementAction::SetMode { mode, selected_idx } => {
                    self.set_user_management_mode(mode, selected_idx);
                }
                UserManagementAction::BackToSettings => {
                    self.navigate_to_screen(TerminalScreen::Settings);
                    self.terminal_settings_idx = 0;
                    self.terminal_user_management_idx = 0;
                }
                UserManagementAction::CreateWithMethod { username, method } => match method {
                    crate::core::auth::AuthMethod::Password => {
                        self.open_password_prompt_with_action(
                            "Create User",
                            format!("Password for {username}"),
                            TerminalPromptAction::CreatePassword { username },
                        );
                    }
                    crate::core::auth::AuthMethod::NoPassword => {
                        self.save_user_and_status(
                            &username,
                            UserRecord {
                                password_hash: String::new(),
                                is_admin: false,
                                auth_method: method,
                            },
                            format!("User '{username}' created."),
                        );
                        self.set_user_management_mode(UserManagementMode::Root, 0);
                    }
                    crate::core::auth::AuthMethod::HackingMinigame => {
                        self.save_user_and_status(
                            &username,
                            UserRecord {
                                password_hash: String::new(),
                                is_admin: false,
                                auth_method: crate::core::auth::AuthMethod::HackingMinigame,
                            },
                            format!("User '{username}' created."),
                        );
                        self.set_user_management_mode(UserManagementMode::Root, 0);
                    }
                },
                UserManagementAction::ApplyCreateHacking { username } => {
                    self.save_user_and_status(
                        &username,
                        UserRecord {
                            password_hash: String::new(),
                            is_admin: false,
                            auth_method: crate::core::auth::AuthMethod::HackingMinigame,
                        },
                        format!("User '{username}' created."),
                    );
                    self.set_user_management_mode(UserManagementMode::Root, 0);
                }
                UserManagementAction::ConfirmDeleteUser { username } => {
                    self.open_confirm_prompt(
                        "Delete User",
                        format!("Delete user '{username}'?"),
                        TerminalPromptAction::ConfirmDeleteUser { username },
                    );
                }
                UserManagementAction::OpenResetPassword { username } => {
                    self.open_password_prompt_with_action(
                        "Reset Password",
                        format!("New password for '{username}'"),
                        TerminalPromptAction::ResetPassword { username },
                    );
                }
                UserManagementAction::ChangeAuthWithMethod { username, method } => match method {
                    crate::core::auth::AuthMethod::Password => {
                        self.open_password_prompt_with_action(
                            "Change Auth Method",
                            format!("New password for '{username}'"),
                            TerminalPromptAction::ChangeAuthPassword { username },
                        );
                    }
                    crate::core::auth::AuthMethod::NoPassword => {
                        self.update_user_record(
                            &username,
                            |record| {
                                record.auth_method = crate::core::auth::AuthMethod::NoPassword;
                                record.password_hash.clear();
                            },
                            format!("Auth method updated for '{username}'."),
                        );
                        self.set_user_management_mode(UserManagementMode::Root, 0);
                    }
                    crate::core::auth::AuthMethod::HackingMinigame => {
                        self.update_user_record(
                            &username,
                            |record| {
                                record.auth_method = crate::core::auth::AuthMethod::HackingMinigame;
                                record.password_hash.clear();
                            },
                            format!("Auth method updated for '{username}'."),
                        );
                        self.set_user_management_mode(UserManagementMode::Root, 0);
                    }
                },
                UserManagementAction::ApplyChangeAuthHacking { username } => {
                    self.update_user_record(
                        &username,
                        |record| {
                            record.auth_method = crate::core::auth::AuthMethod::HackingMinigame;
                            record.password_hash.clear();
                        },
                        format!("Auth method updated for '{username}'."),
                    );
                    self.set_user_management_mode(UserManagementMode::Root, 0);
                }
                UserManagementAction::ConfirmToggleAdmin { username } => {
                    self.open_confirm_prompt(
                        "Toggle Admin",
                        format!("Toggle admin for '{username}'?"),
                        TerminalPromptAction::ConfirmToggleAdmin { username },
                    );
                }
                UserManagementAction::Status(status) => {
                    self.shell_status = status;
                }
            }
        }
    }

    fn draw_terminal_footer_spacer(&self, ctx: &Context) {
        TopBottomPanel::bottom("native_terminal_footer_spacer")
            .resizable(false)
            .exact_height(retro_footer_height())
            .show_separator_line(false)
            .frame(
                egui::Frame::none()
                    .fill(current_palette().bg)
                    .inner_margin(0.0),
            )
            .show(ctx, |_ui| {});
    }

    fn desktop_workspace_rect(ctx: &Context) -> egui::Rect {
        const TOP_BAR_H: f32 = 30.0;
        const TASKBAR_H: f32 = 32.0;
        let screen = ctx.screen_rect();
        let top = screen.top() + TOP_BAR_H;
        let bottom = (screen.bottom() - TASKBAR_H).max(top + 120.0);
        egui::Rect::from_min_max(egui::pos2(screen.left(), top), egui::pos2(screen.right(), bottom))
    }

    fn desktop_window_frame() -> egui::Frame {
        let palette = current_palette();
        egui::Frame::none()
            .fill(palette.bg)
            .stroke(egui::Stroke::new(1.0, palette.fg))
            .inner_margin(egui::Margin::ZERO)
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
        style.visuals.window_fill = palette.panel;
        style.visuals.window_stroke = stroke;
        style.visuals.window_rounding = egui::Rounding::ZERO;
        style.visuals.menu_rounding = egui::Rounding::ZERO;
        style.visuals.window_shadow = egui::epaint::Shadow::NONE;
        style.visuals.popup_shadow = egui::epaint::Shadow::NONE;
        style.visuals.override_text_color = None;
        style.spacing.item_spacing.y = 0.0;
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

    fn retro_file_manager_button(
        ui: &mut egui::Ui,
        label: impl Into<String>,
        desired: egui::Vec2,
        active: bool,
        stroked: bool,
    ) -> egui::Response {
        let palette = current_palette();
        let label = label.into();
        let (rect, response) = ui.allocate_exact_size(desired, egui::Sense::click());
        let hovered = response.hovered();
        let highlighted = active || hovered;
        let fill = if highlighted { palette.fg } else { Color32::TRANSPARENT };
        let text_color = if highlighted { Color32::BLACK } else { palette.fg };
        let stroke = if stroked {
            egui::Stroke::new(1.0, palette.fg)
        } else {
            egui::Stroke::NONE
        };
        // Use painter_at(rect) so clip is the button rect — prevents the panel's
        // own background paint from covering the fill or text in any frame ordering.
        let painter = ui.painter_at(rect);
        if highlighted {
            painter.rect_filled(rect, 0.0, fill);
        }
        if stroke != egui::Stroke::NONE {
            painter.rect_stroke(rect, 0.0, stroke);
        }
        painter.text(
            rect.left_top() + egui::vec2(8.0, 6.0),
            Align2::LEFT_TOP,
            label,
            FontId::new(18.0, FontFamily::Monospace),
            text_color,
        );
        response
    }

    fn retro_file_manager_item(
        ui: &mut egui::Ui,
        texture: Option<&TextureHandle>,
        fallback_icon: &str,
        label: &str,
        desired: egui::Vec2,
        active: bool,
        stroked: bool,
        grid_mode: bool,
    ) -> egui::Response {
        let palette = current_palette();
        let (rect, response) = ui.allocate_exact_size(desired, egui::Sense::click());
        let hovered = response.hovered();
        let highlighted = active || hovered;
        let fill = if highlighted { palette.fg } else { Color32::TRANSPARENT };
        let text_color = if highlighted { Color32::BLACK } else { palette.fg };
        let stroke = if stroked {
            egui::Stroke::new(1.0, palette.fg)
        } else {
            egui::Stroke::NONE
        };
        let painter = ui.painter_at(rect);
        if highlighted {
            painter.rect_filled(rect, 0.0, fill);
        }
        if stroke != egui::Stroke::NONE {
            painter.rect_stroke(rect, 0.0, stroke);
        }

        if grid_mode {
            let icon_rect = egui::Rect::from_center_size(
                egui::pos2(rect.center().x, rect.top() + 18.0),
                egui::vec2(24.0, 24.0),
            );
            if let Some(texture) = texture {
                Self::paint_tinted_texture(&painter, texture, icon_rect, text_color);
            } else {
                painter.text(
                    icon_rect.center(),
                    Align2::CENTER_CENTER,
                    fallback_icon,
                    FontId::new(16.0, FontFamily::Monospace),
                    text_color,
                );
            }
            painter.text(
                egui::pos2(rect.center().x, rect.bottom() - 14.0),
                Align2::CENTER_CENTER,
                label,
                FontId::new(16.0, FontFamily::Monospace),
                text_color,
            );
        } else {
            let icon_rect = egui::Rect::from_min_size(
                rect.left_top() + egui::vec2(6.0, 4.0),
                egui::vec2(18.0, 18.0),
            );
            if let Some(texture) = texture {
                Self::paint_tinted_texture(&painter, texture, icon_rect, text_color);
            } else {
                painter.text(
                    icon_rect.center(),
                    Align2::CENTER_CENTER,
                    fallback_icon,
                    FontId::new(14.0, FontFamily::Monospace),
                    text_color,
                );
            }
            painter.text(
                rect.left_top() + egui::vec2(30.0, 6.0),
                Align2::LEFT_TOP,
                label,
                FontId::new(18.0, FontFamily::Monospace),
                text_color,
            );
        }
        response
    }

    fn retro_full_width_button(ui: &mut egui::Ui, label: impl Into<String>) -> egui::Response {
        let palette = current_palette();
        ui.add_sized(
            [ui.available_width().max(160.0), 0.0],
            egui::Button::new(label.into()).stroke(egui::Stroke::new(2.0, palette.fg)),
        )
    }

    fn responsive_input_width(
        ui: &egui::Ui,
        fraction: f32,
        min: f32,
        max: f32,
    ) -> f32 {
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
        title: &str,
        maximized: bool,
    ) -> DesktopHeaderAction {
        let palette = current_palette();
        let mut action = DesktopHeaderAction::None;
        egui::Frame::none()
            .fill(palette.fg)
            .stroke(egui::Stroke::NONE)
            .inner_margin(egui::Margin::symmetric(8.0, 3.0))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(title).color(Color32::BLACK).strong());
                    ui.add_space(6.0);
                    ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
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

    fn draw_file_manager(&mut self, ctx: &Context) {
        if !self.file_manager.open || self.desktop_window_is_minimized(DesktopWindow::FileManager) {
            return;
        }
        let mut open = self.file_manager.open;
        let maximized = self.desktop_window_is_maximized(DesktopWindow::FileManager);
        let restore = self.take_desktop_window_restore_dims(DesktopWindow::FileManager);
        let mut header_action = DesktopHeaderAction::None;
        let generation = self.desktop_window_generation(DesktopWindow::FileManager);
        let default_size = Self::desktop_default_window_size(DesktopWindow::FileManager);
        let mut window = egui::Window::new("File Manager")
            .id(Id::new(("native_file_manager", generation)))
            .open(&mut open)
            .title_bar(false)
            .frame(Self::desktop_window_frame())
            .resizable(true)
            .min_size([400.0, 300.0])
            .default_size([default_size.x, default_size.y]);
        if maximized {
            let rect = Self::desktop_workspace_rect(ctx);
            window = window
                .movable(false)
                .resizable(false)
                .fixed_pos(rect.min)
                .fixed_size(rect.size());
        } else if let Some((pos, size)) = restore {
            // Unmaximize or first open with a saved size: generation was bumped so egui
            // has no memory for this ID — default_size sets the initial size correctly.
            window = window.current_pos(pos).default_size(size);
        }
        self.file_manager.ensure_selection_valid();
        let search_id = Id::new(("native_file_manager_search", generation));
        let shown = window.show(ctx, |ui| {
                Self::apply_settings_control_style(ui);

                // ── TOP PANEL: header + chrome ─────────────────────────────────────
                // show_inside panels carve from the window rect directly and never
                // feed back into the Resize widget — this is the correct egui pattern
                // for resizable windows that behave like Win95 explorer.
                egui::TopBottomPanel::top(Id::new(("fm_top", generation)))
                    .frame(egui::Frame::none())
                    .show_inside(ui, |ui| {
                        header_action = Self::draw_desktop_window_header(ui, "My Computer", maximized);

                        ui.horizontal_wrapped(|ui| {
                            ui.label(RichText::new("Tabs").strong());
                            for idx in 0..self.file_manager.tabs.len() {
                                let title = NativeFileManagerState::tab_title(&self.file_manager.tabs[idx]);
                                let title = Self::truncate_file_manager_label(&title, 12);
                                let active = idx == self.file_manager.active_tab;
                                let response = Self::retro_file_manager_button(
                                    ui,
                                    format!("[{}:{}{}]", idx + 1, title, if active { "*" } else { "" }),
                                    egui::vec2(132.0, 28.0),
                                    active,
                                    false,
                                );
                                if response.clicked() {
                                    let _ = self.file_manager.switch_to_tab(idx);
                                }
                            }
                            if ui.button("+").clicked() {
                                self.file_manager.open_tab_here();
                            }
                            let close_tab = if self.file_manager.tabs.len() > 1 {
                                ui.button("x")
                            } else {
                                Self::retro_disabled_button(ui, "x")
                            };
                            if close_tab.clicked() {
                                let _ = self.file_manager.close_active_tab();
                            }
                        });

                        let search_requested = self.desktop_active_window == Some(DesktopWindow::FileManager)
                            && ctx.input(|i| i.modifiers.ctrl && i.key_pressed(Key::F));
                        if search_requested {
                            ui.memory_mut(|mem| mem.request_focus(search_id));
                        }

                        let mut search_query = self.file_manager.search_query.clone();
                        ui.horizontal(|ui| {
                            let search_label = if search_query.is_empty() { "Search: (Ctrl+F)" } else { "Search:" };
                            ui.label(RichText::new(search_label).strong());
                            let search_width = (ui.available_width() - 240.0).max(180.0);
                            let search = ui.add_sized(
                                [search_width, 0.0],
                                TextEdit::singleline(&mut search_query)
                                    .id(search_id)
                                    .hint_text("filter files and folders"),
                            );
                            if search.changed() {
                                self.file_manager.update_search_query(search_query.clone());
                            }
                            ui.add_space(8.0);
                            if ui.selectable_label(get_settings().desktop_file_manager.show_tree_panel, "Tree").clicked() {
                                self.toggle_file_manager_tree_panel();
                            }
                            if ui.selectable_label(self.file_manager_view_mode() == FileManagerViewMode::List, "List").clicked() {
                                self.set_file_manager_view_mode(FileManagerViewMode::List);
                            }
                            if ui.selectable_label(self.file_manager_view_mode() == FileManagerViewMode::Grid, "Grid").clicked() {
                                self.set_file_manager_view_mode(FileManagerViewMode::Grid);
                            }
                        });

                        ui.label(RichText::new(format!("Path: {}", self.file_manager.cwd.display())).strong());
                        ui.separator();
                    });

                // ── BOTTOM PANEL: status bar + buttons ────────────────────────────
                let view_mode = self.file_manager_view_mode();
                let show_tree_panel = get_settings().desktop_file_manager.show_tree_panel;
                let rows = self.file_manager.rows();
                egui::TopBottomPanel::bottom(Id::new(("fm_bottom", generation)))
                    .frame(egui::Frame::none())
                    .exact_height(34.0)
                    .show_inside(ui, |ui| {
                        ui.painter().hline(
                            ui.max_rect().x_range(),
                            ui.max_rect().top() + 1.0,
                            egui::Stroke::new(1.0, current_palette().fg),
                        );
                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            ui.small(format!("{} item(s)", rows.len()));
                            ui.separator();
                            ui.small(match view_mode {
                                FileManagerViewMode::Grid => "Grid View",
                                FileManagerViewMode::List => "List View",
                            });
                            ui.separator();
                            ui.small(if show_tree_panel { "Tree On" } else { "Tree Off" });
                            ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                                if ui.button("New Document").clicked() { self.new_document(); }
                                if ui.button("Open").clicked() { self.activate_file_manager_selection(); }
                                if ui.button("Up").clicked() { self.file_manager.up(); }
                                if ui.button("Home").clicked() { self.file_manager_open_home(); }
                            });
                        });
                    });

                // ── LEFT PANEL: folder tree (optional) ────────────────────────────
                if show_tree_panel {
                    let tree_items = self.file_manager.tree_items();
                    egui::SidePanel::left(Id::new(("fm_tree", generation)))
                        .frame(egui::Frame::none())
                        .width_range(140.0..=280.0)
                        .default_width(200.0)
                        .show_inside(ui, |ui| {
                            ui.label(RichText::new("Folders").strong());
                            egui::ScrollArea::vertical()
                                .id_salt(("native_file_manager_tree", generation))
                                .show(ui, |ui| {
                                    for item in &tree_items {
                                        let selected = item.path.as_ref()
                                            .is_some_and(|p| Some(p) == self.file_manager.tree_selected.as_ref());
                                        let response = Self::retro_file_manager_button(
                                            ui,
                                            item.line.clone(),
                                            egui::vec2(ui.available_width(), 26.0),
                                            selected,
                                            false,
                                        );
                                        if response.clicked() {
                                            if let Some(path) = &item.path {
                                                self.file_manager.open_selected_tree_path(path.clone());
                                            }
                                        }
                                    }
                                });
                        });
                }

                // ── CENTRAL PANEL: file list ──────────────────────────────────────
                egui::CentralPanel::default()
                    .frame(egui::Frame::none())
                    .show_inside(ui, |ui| {
                        if rows.is_empty() {
                            ui.label("No files match the current search.");
                            return;
                        }
                        match view_mode {
                            FileManagerViewMode::List => {
                                egui::ScrollArea::vertical()
                                    .id_salt(("native_file_manager_list", generation))
                                    .show(ui, |ui| {
                                        for row in &rows {
                                            let selected = self.file_manager.selected.as_ref() == Some(&row.path);
                                            let response = Self::retro_file_manager_item(
                                                ui,
                                                self.file_manager_texture_for_row(row),
                                                row.icon(),
                                                &row.label,
                                                egui::vec2(ui.available_width(), 28.0),
                                                selected,
                                                false,
                                                false,
                                            );
                                            if response.secondary_clicked() {
                                                self.file_manager.select(Some(row.path.clone()));
                                            }
                                            if response.clicked() { self.file_manager.select(Some(row.path.clone())); }
                                            if response.double_clicked() {
                                                self.file_manager.select(Some(row.path.clone()));
                                                self.activate_file_manager_selection();
                                            }
                                            let has_clipboard = self
                                                .file_clipboard
                                                .as_ref()
                                                .is_some_and(|clip| !clip.paths.is_empty());
                                            Self::attach_file_manager_context_menu(
                                                &mut self.context_menu_action,
                                                &response,
                                                true,
                                                row.path.is_file(),
                                                has_clipboard,
                                            );
                                        }
                                        let background = ui.allocate_rect(
                                            ui.available_rect_before_wrap(),
                                            egui::Sense::click(),
                                        );
                                        let has_selection = self.file_manager.selected.is_some();
                                        let has_file_selection =
                                            self.file_manager_selected_file().is_some();
                                        let has_clipboard = self
                                            .file_clipboard
                                            .as_ref()
                                            .is_some_and(|clip| !clip.paths.is_empty());
                                        Self::attach_file_manager_context_menu(
                                            &mut self.context_menu_action,
                                            &background,
                                            has_selection,
                                            has_file_selection,
                                            has_clipboard,
                                        );
                                    });
                            }
                            FileManagerViewMode::Grid => {
                                let tile_width = 150.0;
                                let available_w = ui.available_width();
                                let cols = ((available_w / tile_width).floor() as usize).max(1);
                                egui::ScrollArea::vertical()
                                    .id_salt(("native_file_manager_grid", generation))
                                    .show(ui, |ui| {
                                        for chunk in rows.chunks(cols) {
                                            ui.allocate_ui_with_layout(
                                                egui::vec2(available_w, 64.0),
                                                Layout::left_to_right(egui::Align::Min),
                                                |ui| {
                                                    for row in chunk {
                                                    let selected = self.file_manager.selected.as_ref() == Some(&row.path);
                                                    let label = Self::truncate_file_manager_label(&row.label, 16);
                                                        let response = Self::retro_file_manager_item(
                                                            ui,
                                                            self.file_manager_texture_for_row(row),
                                                            row.icon(),
                                                            &label,
                                                            egui::vec2(tile_width - 8.0, 60.0),
                                                            selected,
                                                            true,
                                                            true,
                                                        );
                                                        if response.secondary_clicked() {
                                                            self.file_manager.select(Some(row.path.clone()));
                                                        }
                                                        if response.clicked() { self.file_manager.select(Some(row.path.clone())); }
                                                        if response.double_clicked() {
                                                            self.file_manager.select(Some(row.path.clone()));
                                                            self.activate_file_manager_selection();
                                                        }
                                                        let has_clipboard = self
                                                            .file_clipboard
                                                            .as_ref()
                                                            .is_some_and(|clip| !clip.paths.is_empty());
                                                        Self::attach_file_manager_context_menu(
                                                            &mut self.context_menu_action,
                                                            &response,
                                                            true,
                                                            row.path.is_file(),
                                                            has_clipboard,
                                                        );
                                                    }
                                                },
                                            );
                                        }
                                        let background = ui.allocate_rect(
                                            ui.available_rect_before_wrap(),
                                            egui::Sense::click(),
                                        );
                                        let has_selection = self.file_manager.selected.is_some();
                                        let has_file_selection =
                                            self.file_manager_selected_file().is_some();
                                        let has_clipboard = self
                                            .file_clipboard
                                            .as_ref()
                                            .is_some_and(|clip| !clip.paths.is_empty());
                                        Self::attach_file_manager_context_menu(
                                            &mut self.context_menu_action,
                                            &background,
                                            has_selection,
                                            has_file_selection,
                                            has_clipboard,
                                        );
                                    });
                            }
                        }
                    });
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
        if !maximized {
            // Always save the full rect. egui owns window sizing for resizable windows
            // and will not inflate it — only user drag changes it.
            if let Some(rect) = shown_rect {
                self.note_desktop_window_rect(DesktopWindow::FileManager, rect);
            }
        }
        match header_action {
            DesktopHeaderAction::None => {}
            DesktopHeaderAction::Close => open = false,
            DesktopHeaderAction::Minimize => self.set_desktop_window_minimized(DesktopWindow::FileManager, true),
            DesktopHeaderAction::ToggleMaximize => {
                self.toggle_desktop_window_maximized(DesktopWindow::FileManager, shown_rect)
            }
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
            self.save_editor();
        }
        let title = self
            .editor
            .path
            .as_ref()
            .and_then(|p| p.file_name())
            .and_then(|p| p.to_str())
            .unwrap_or("ROBCO Word Processor")
            .to_string();

        if !self.desktop_mode_open {
            if ctx.input(|i| {
                i.key_pressed(Key::Escape)
                    || i.key_pressed(Key::Tab)
                    || (i.modifiers.ctrl && i.key_pressed(Key::Q))
            }) {
                self.update_desktop_window_state(DesktopWindow::Editor, false);
                return;
            }
            egui::CentralPanel::default()
                .frame(
                    egui::Frame::none()
                        .fill(current_palette().bg)
                        .inner_margin(egui::Margin::same(8.0)),
                )
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(&title).strong());
                        ui.separator();
                        if ui.button("New").clicked() {
                            self.new_document();
                        }
                        if ui.button("Save").clicked() {
                            self.save_editor();
                        }
                        if ui.button("Open File Manager").clicked() {
                            self.open_desktop_window(DesktopWindow::FileManager);
                        }
                        if ui.button("Close").clicked() {
                            self.update_desktop_window_state(DesktopWindow::Editor, false);
                        }
                    });
                    if let Some(path) = &self.editor.path {
                        ui.small(path.display().to_string());
                    }
                    ui.separator();
                    let edit = TextEdit::multiline(&mut self.editor.text)
                        .desired_rows(28)
                        .lock_focus(true)
                        .code_editor();
                    let response = ui.add_sized(ui.available_size(), edit);
                    Self::attach_generic_context_menu(&mut self.context_menu_action, &response);
                    if response.changed() {
                        self.editor.dirty = true;
                    }
                    if !self.editor.status.is_empty() {
                        ui.separator();
                        ui.small(&self.editor.status);
                    }
                });
            return;
        }

        let mut open = self.editor.open;
        let maximized = self.desktop_window_is_maximized(DesktopWindow::Editor);
        let restore = self.take_desktop_window_restore_dims(DesktopWindow::Editor);
        let mut header_action = DesktopHeaderAction::None;
        let generation = self.desktop_window_generation(DesktopWindow::Editor);
        let mut window = egui::Window::new(&title)
            .id(Id::new(("native_word_processor", generation)))
            .open(&mut open)
            .title_bar(false)
            .frame(Self::desktop_window_frame())
            .resizable(true)
            .min_size([400.0, 300.0])
            .default_size([820.0, 560.0]);
        if maximized {
            let rect = Self::desktop_workspace_rect(ctx);
            window = window
                .movable(false)
                .resizable(false)
                .fixed_pos(rect.min)
                .fixed_size(rect.size());
        } else if let Some((pos, size)) = restore {
            window = window.current_pos(pos).default_size(size);
        }
        let shown = window.show(ctx, |ui| {
                Self::apply_settings_control_style(ui);
                header_action = Self::draw_desktop_window_header(ui, &title, maximized);
                ui.horizontal(|ui| {
                    if ui.button("New").clicked() {
                        self.new_document();
                    }
                    if ui.button("Save").clicked() {
                        self.save_editor();
                    }
                    if ui.button("Open File Manager").clicked() {
                        self.open_desktop_window(DesktopWindow::FileManager);
                    }
                    if let Some(path) = &self.editor.path {
                        ui.label(path.display().to_string());
                    }
                });
                ui.separator();
                let edit = TextEdit::multiline(&mut self.editor.text)
                    .desired_rows(24)
                    .lock_focus(true)
                    .code_editor();
                // Use available_size() here only for the text edit fill — this is fine
                // because TextEdit doesn't cause the window to grow beyond its current size.
                let response = ui.add_sized(ui.available_size(), edit);
                Self::attach_generic_context_menu(&mut self.context_menu_action, &response);
                if response.changed() {
                    self.editor.dirty = true;
                }
                if !self.editor.status.is_empty() {
                    ui.separator();
                    ui.small(&self.editor.status);
                }
            });
        let shown_rect = shown.as_ref().map(|inner| inner.response.rect);
        let shown_contains_pointer = shown
            .as_ref()
            .is_some_and(|inner| inner.response.contains_pointer());
        if let Some(inner) = shown.as_ref() {
            Self::attach_generic_context_menu(&mut self.context_menu_action, &inner.response);
        }
        self.maybe_activate_desktop_window_from_click(
            ctx,
            DesktopWindow::Editor,
            shown_contains_pointer,
        );
        if !maximized {
            if let Some(rect) = shown_rect {
                self.note_desktop_window_rect(DesktopWindow::Editor, rect);
            }
        }
        match header_action {
            DesktopHeaderAction::None => {}
            DesktopHeaderAction::Close => open = false,
            DesktopHeaderAction::Minimize => self.set_desktop_window_minimized(DesktopWindow::Editor, true),
            DesktopHeaderAction::ToggleMaximize => {
                self.toggle_desktop_window_maximized(DesktopWindow::Editor, shown_rect)
            }
        }
        self.update_desktop_window_state(DesktopWindow::Editor, open);
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
            let mut next_panel = None;

            let panel_title = match panel {
                NativeSettingsPanel::Home => "Settings",
                NativeSettingsPanel::General => "General",
                NativeSettingsPanel::Appearance => "Appearance",
                NativeSettingsPanel::DefaultApps => "Default Apps",
                NativeSettingsPanel::Connections => "Connections",
                NativeSettingsPanel::ConnectionsNetwork => "Network",
                NativeSettingsPanel::ConnectionsBluetooth => "Bluetooth",
                NativeSettingsPanel::CliProfiles => "CLI Profiles",
                NativeSettingsPanel::EditMenus => "Edit Menus",
                NativeSettingsPanel::UserManagement => "User Management",
                NativeSettingsPanel::UserManagementViewUsers => "View Users",
                NativeSettingsPanel::UserManagementCreateUser => "Create User",
                NativeSettingsPanel::UserManagementEditUsers => "Edit Users",
                NativeSettingsPanel::UserManagementEditCurrentUser => "Edit Current User",
                NativeSettingsPanel::About => "About",
            };

            ui.add_space(4.0);
            if matches!(panel, NativeSettingsPanel::Home) {
                ui.label(RichText::new("Settings").strong().size(28.0));
                ui.add_space(14.0);
            } else {
                ui.horizontal(|ui| {
                    if ui.button("Back").clicked() {
                        next_panel = Some(NativeSettingsPanel::Home);
                    }
                    ui.strong(panel_title);
                });
                ui.separator();
                ui.add_space(4.0);
            }

            match panel {
                NativeSettingsPanel::Home => {
                    let rows: [&[(NativeSettingsPanel, &str, &str, bool)]; 3] = [
                        &[
                            (NativeSettingsPanel::General, "General", "[*]", true),
                            (NativeSettingsPanel::Appearance, "Appearance", "[A]", true),
                            (NativeSettingsPanel::DefaultApps, "Default Apps", "[D]", true),
                            (NativeSettingsPanel::Connections, "Connections", "[C]", true),
                        ],
                        &[
                            (NativeSettingsPanel::CliProfiles, "CLI Profiles", "[=]", true),
                            (NativeSettingsPanel::EditMenus, "Edit Menus", "[M]", true),
                            (
                                NativeSettingsPanel::UserManagement,
                                "User Management",
                                "[U]",
                                is_admin,
                            ),
                            (NativeSettingsPanel::About, "About", "[i]", true),
                        ],
                        &[(NativeSettingsPanel::Home, "Close", "[X]", true)],
                    ];
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
                            for (panel, label, icon, enabled) in *row {
                                let response = Self::retro_settings_tile(
                                    ui,
                                    self.settings_panel_texture(*panel),
                                    icon,
                                    label,
                                    *enabled,
                                    egui::vec2(tile_w, tile_h),
                                    icon_font_size,
                                    label_font_size,
                                );
                                if response.clicked() {
                                    if *label == "Close" {
                                        close_requested = true;
                                    } else {
                                        next_panel = Some(*panel);
                                    }
                                }
                            }
                            for _ in row.len()..4 {
                                ui.add_space(tile_w);
                            }
                        });
                        ui.add_space(if row_idx == rows.len() - 1 { 0.0 } else { row_gap });
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
                                        left.small("Choose which interface opens first after login.");
                                    });

                                    Self::settings_section(right, "Options", |right| {
                                        if Self::retro_checkbox_row(
                                            right,
                                            &mut self.settings.draft.sound,
                                            "Enable sound",
                                        )
                                        .clicked()
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
                                Self::settings_two_columns(ui, |left, right| {
                                    Self::settings_section(left, "Theme", |left| {
                                        left.horizontal(|ui| {
                                            ui.label("Theme");
                                            let mut current_idx = THEMES
                                                .iter()
                                                .position(|(name, _)| *name == self.settings.draft.theme)
                                                .unwrap_or(0);
                                            egui::ComboBox::from_id_salt("native_settings_theme")
                                                .selected_text(
                                                    RichText::new(THEMES[current_idx].0)
                                                        .color(current_palette().fg),
                                                )
                                                .show_ui(ui, |ui| {
                                                    Self::apply_settings_control_style(ui);
                                                    for (idx, (name, _)) in THEMES.iter().enumerate() {
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
                                            changed |= left
                                                .add(
                                                    egui::Slider::new(&mut rgb[0], 0..=255)
                                                        .text("Custom Red"),
                                                )
                                                .changed();
                                            changed |= left
                                                .add(
                                                    egui::Slider::new(&mut rgb[1], 0..=255)
                                                        .text("Custom Green"),
                                                )
                                                .changed();
                                            changed |= left
                                                .add(
                                                    egui::Slider::new(&mut rgb[2], 0..=255)
                                                        .text("Custom Blue"),
                                                )
                                                .changed();
                                            if rgb != self.settings.draft.custom_theme_rgb {
                                                self.settings.draft.custom_theme_rgb = rgb;
                                            }
                                        }
                                    });

                                    Self::settings_section(left, "Desktop Surface", |left| {
                                        left.label("Wallpaper Path");
                                        let wallpaper_width =
                                            Self::responsive_input_width(left, 0.95, 220.0, 520.0);
                                        if left
                                            .add(
                                                TextEdit::singleline(
                                                    &mut self.settings.draft.desktop_wallpaper,
                                                )
                                                .desired_width(wallpaper_width)
                                                .hint_text("/path/to/image.png"),
                                            )
                                            .changed()
                                        {
                                            changed = true;
                                        }
                                        left.add_space(8.0);
                                        left.horizontal(|ui| {
                                            ui.label("Wallpaper Mode");
                                            let selected = match self
                                                .settings
                                                .draft
                                                .desktop_wallpaper_size_mode
                                            {
                                                WallpaperSizeMode::DefaultSize => "Default Size",
                                                WallpaperSizeMode::FitToScreen => "Fit To Screen",
                                                WallpaperSizeMode::Centered => "Centered",
                                                WallpaperSizeMode::Tile => "Tile",
                                                WallpaperSizeMode::Stretch => "Stretch",
                                            };
                                            egui::ComboBox::from_id_salt(
                                                "native_settings_wallpaper_mode",
                                            )
                                            .selected_text(
                                                RichText::new(selected).color(current_palette().fg),
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
                                                        self.settings.draft.desktop_wallpaper_size_mode =
                                                            mode;
                                                        changed = true;
                                                        ui.close_menu();
                                                    }
                                                }
                                            });
                                        });
                                        left.add_space(8.0);
                                        left.horizontal(|ui| {
                                            ui.label("Desktop Icons");
                                            let selected = match self.settings.draft.desktop_icon_style {
                                                DesktopIconStyle::Dos => "DOS",
                                                DesktopIconStyle::Win95 => "Win95",
                                                DesktopIconStyle::Minimal => "Minimal",
                                                DesktopIconStyle::NoIcons => "No Icons",
                                            };
                                            egui::ComboBox::from_id_salt(
                                                "native_settings_desktop_icons",
                                            )
                                            .selected_text(
                                                RichText::new(selected).color(current_palette().fg),
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
                                                        self.settings.draft.desktop_icon_style =
                                                            style;
                                                        changed = true;
                                                        ui.close_menu();
                                                    }
                                                }
                                            });
                                        });
                                        left.add_space(8.0);
                                        if Self::retro_checkbox_row(
                                            left,
                                            &mut self.settings.draft.desktop_show_cursor,
                                            "Show desktop cursor",
                                        )
                                        .clicked()
                                        {
                                            changed = true;
                                        }
                                    });

                                    Self::settings_section(right, "PTY Display", |right| {
                                        if Self::retro_checkbox_row(
                                            right,
                                            &mut self.settings.draft.cli_styled_render,
                                            "Styled PTY rendering",
                                        )
                                        .clicked()
                                        {
                                            changed = true;
                                        }
                                        right.horizontal(|ui| {
                                            ui.label("PTY Color Mode");
                                            let selected = match self.settings.draft.cli_color_mode {
                                                CliColorMode::ThemeLock => "Theme Lock",
                                                CliColorMode::PaletteMap => "Palette-map",
                                                CliColorMode::Color => "Color",
                                                CliColorMode::Monochrome => "Monochrome",
                                            };
                                            egui::ComboBox::from_id_salt("native_settings_cli_color")
                                                .selected_text(
                                                    RichText::new(selected)
                                                        .color(current_palette().fg),
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
                                                            self.settings.draft.cli_color_mode == mode,
                                                        )
                                                        .clicked()
                                                            && self.settings.draft.cli_color_mode != mode
                                                        {
                                                            self.settings.draft.cli_color_mode = mode;
                                                            changed = true;
                                                            ui.close_menu();
                                                        }
                                                    }
                                                });
                                        });
                                        if right
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
                                });
                            }
                            NativeSettingsPanel::DefaultApps => {
                                changed |= self.draw_settings_default_apps_panel(ui);
                            }
                            NativeSettingsPanel::Connections => {
                                ui.vertical(|ui| {
                                    if Self::retro_full_width_button(ui, "Network").clicked() {
                                        next_panel = Some(NativeSettingsPanel::ConnectionsNetwork);
                                    }
                                    if Self::retro_full_width_button(ui, "Bluetooth").clicked() {
                                        next_panel =
                                            Some(NativeSettingsPanel::ConnectionsBluetooth);
                                    }
                                });
                            }
                            NativeSettingsPanel::ConnectionsNetwork => {
                                self.draw_settings_connections_kind_panel(ui, ConnectionKind::Network);
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
                                        if Self::retro_full_width_button(ui, "View Users").clicked() {
                                            next_panel =
                                                Some(NativeSettingsPanel::UserManagementViewUsers);
                                        }
                                        if Self::retro_full_width_button(ui, "Create User").clicked() {
                                            next_panel =
                                                Some(NativeSettingsPanel::UserManagementCreateUser);
                                        }
                                        if Self::retro_full_width_button(ui, "Edit Users").clicked() {
                                            next_panel =
                                                Some(NativeSettingsPanel::UserManagementEditUsers);
                                        }
                                        if Self::retro_full_width_button(
                                            ui,
                                            "Edit Current User",
                                        )
                                        .clicked()
                                        {
                                            next_panel = Some(
                                                NativeSettingsPanel::UserManagementEditCurrentUser,
                                            );
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
                            }
                            NativeSettingsPanel::Home => {}
                        });
                }
            }

            if let Some(panel) = next_panel {
                self.settings.panel = panel;
                self.settings.status.clear();
            }
            ui.separator();
            if changed {
                save_settings(self.settings.draft.clone());
                self.settings.status = "Settings saved.".to_string();
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
        if let Some(inner) = shown.as_ref() {
            Self::attach_generic_context_menu(&mut self.context_menu_action, &inner.response);
        }
        self.maybe_activate_desktop_window_from_click(
            ctx,
            DesktopWindow::Settings,
            shown_contains_pointer,
        );
        if !maximized {
            // Only save position, not size — settings window is fixed-size.
            if let Some(pos) = shown_rect.map(|r| r.min) {
                let state = self.desktop_window_state_mut(DesktopWindow::Settings);
                state.restore_pos = Some([pos.x, pos.y]);
            }
        }
        match header_action {
            DesktopHeaderAction::None => {}
            DesktopHeaderAction::Close => open = false,
            DesktopHeaderAction::Minimize => self.set_desktop_window_minimized(DesktopWindow::Settings, true),
            DesktopHeaderAction::ToggleMaximize => {
                self.toggle_desktop_window_maximized(DesktopWindow::Settings, shown_rect)
            }
        }
        self.update_desktop_window_state(DesktopWindow::Settings, open);
    }

    fn draw_settings_default_apps_panel(&mut self, ui: &mut egui::Ui) -> bool {
        let mut changed = false;
        egui::ScrollArea::vertical().show(ui, |ui| {
            for slot in [DefaultAppSlot::TextCode, DefaultAppSlot::Ebook] {
                let current_label = match slot {
                    DefaultAppSlot::TextCode => {
                        binding_label(&self.settings.draft.default_apps.text_code)
                    }
                    DefaultAppSlot::Ebook => binding_label(&self.settings.draft.default_apps.ebook),
                };
                let custom_buffer = match slot {
                    DefaultAppSlot::TextCode => &mut self.settings.default_app_custom_text_code,
                    DefaultAppSlot::Ebook => &mut self.settings.default_app_custom_ebook,
                };

                ui.group(|ui| {
                    Self::settings_two_columns(ui, |left, right| {
                        Self::settings_section(
                            left,
                            &format!("Default App For {}", slot_label(slot)),
                            |left| {
                                left.label(format!("Currently selected: {current_label}"));
                                left.small(match slot {
                                    DefaultAppSlot::TextCode => {
                                        "Used when opening text documents and code files."
                                    }
                                    DefaultAppSlot::Ebook => {
                                        "Used when opening ebook and reader-oriented documents."
                                    }
                                });
                            },
                        );

                        Self::settings_section(right, "Selection", |right| {
                            let field_width =
                                Self::responsive_input_width(right, 0.85, 220.0, 620.0);
                            right.horizontal(|ui| {
                                ui.label("Chooser");
                                egui::ComboBox::from_id_salt(format!("native_default_app_slot_{slot:?}"))
                                    .selected_text(
                                        RichText::new(current_label.clone()).color(current_palette().fg),
                                    )
                                    .show_ui(ui, |ui| {
                                        Self::apply_settings_control_style(ui);
                                        for choice in crate::default_apps::default_app_choices(slot) {
                                            if let crate::default_apps::DefaultAppChoiceAction::Set(binding) =
                                                choice.action
                                            {
                                                let selected = match slot {
                                                    DefaultAppSlot::TextCode => {
                                                        self.settings.draft.default_apps.text_code == binding
                                                    }
                                                    DefaultAppSlot::Ebook => {
                                                        self.settings.draft.default_apps.ebook == binding
                                                    }
                                                };
                                                if Self::retro_choice_button(ui, choice.label, selected)
                                                    .clicked()
                                                {
                                                    set_binding_for_slot(
                                                        &mut self.settings.draft,
                                                        slot,
                                                        binding,
                                                    );
                                                    changed = true;
                                                    ui.close_menu();
                                                }
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
                            if Self::retro_full_width_button(right, "Apply Custom Command").clicked() {
                                match parse_custom_command_line(custom_buffer.trim()) {
                                    Some(argv) if !argv.is_empty() => {
                                        set_binding_for_slot(
                                            &mut self.settings.draft,
                                            slot,
                                            DefaultAppBinding::CustomArgv { argv },
                                        );
                                        changed = true;
                                    }
                                    _ => {
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

    fn draw_settings_connections_kind_panel(
        &mut self,
        ui: &mut egui::Ui,
        kind: ConnectionKind,
    ) {
        if crate::connections::macos_connections_disabled() {
            ui.small(crate::connections::macos_connections_disabled_hint());
            return;
        }

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
            *scanned_items = refresh_discovered_connections(kind);
            self.settings.status = format!("Found {} items.", scanned_items.len());
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
        Self::settings_two_columns(ui, |left, right| {
            Self::settings_section(left, saved_title, |left| {
                let saved = saved_connections(kind);
                if saved.is_empty() {
                    left.small("No saved items.");
                } else {
                    egui::ScrollArea::vertical()
                        .max_height((left.available_height() * 0.85).clamp(180.0, 420.0))
                        .show(left, |ui| {
                            for entry in saved {
                                ui.horizontal(|ui| {
                                    ui.label(saved_row_label(&entry));
                                    if ui.button("Forget").clicked()
                                        && forget_saved_connection(kind, &entry.name)
                                    {
                                        self.settings.draft = current_settings();
                                        self.settings.status = format!("Forgot '{}'.", entry.name);
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
                                    ui.label(discovered_row_label(&entry));
                                    if ui.button("Connect").clicked() {
                                        let password = if matches!(kind, ConnectionKind::Network)
                                            && network_requires_password(&entry.detail)
                                            && !self.settings.connection_password.trim().is_empty()
                                        {
                                            Some(self.settings.connection_password.clone())
                                        } else {
                                            None
                                        };
                                        match connect_connection(
                                            kind,
                                            &entry.name,
                                            Some(&entry.detail),
                                            password.as_deref(),
                                        ) {
                                            Ok(status) => {
                                                self.settings.status = status;
                                                self.settings.draft = current_settings();
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
    }

    fn draw_settings_cli_profiles_panel(&mut self, ui: &mut egui::Ui) -> bool {
        let mut changed = false;
        let custom_profile_count = self.settings.draft.desktop_cli_profiles.custom.len();
        let profile =
            Self::gui_cli_profile_mut(&mut self.settings.draft.desktop_cli_profiles, self.settings.cli_profile_slot);
        let mut min_w = profile.min_w;
        let mut min_h = profile.min_h;
        Self::settings_two_columns(ui, |left, right| {
            Self::settings_section(left, "Profile", |left| {
                left.horizontal(|ui| {
                    ui.label("Profile");
                    egui::ComboBox::from_id_salt("native_settings_cli_profile_slot")
                        .selected_text(
                            RichText::new(Self::gui_cli_profile_slot_label(
                                self.settings.cli_profile_slot,
                            ))
                            .color(current_palette().fg),
                        )
                        .show_ui(ui, |ui| {
                            Self::apply_settings_control_style(ui);
                            for slot in [
                                GuiCliProfileSlot::Default,
                                GuiCliProfileSlot::Calcurse,
                                GuiCliProfileSlot::SpotifyPlayer,
                                GuiCliProfileSlot::Ranger,
                                GuiCliProfileSlot::Reddit,
                            ] {
                                if Self::retro_choice_button(
                                    ui,
                                    Self::gui_cli_profile_slot_label(slot),
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
                    .add(egui::DragValue::new(&mut min_w).range(20..=240).prefix("Min W "))
                    .changed();
                changed |= left
                    .add(egui::DragValue::new(&mut min_h).range(10..=120).prefix("Min H "))
                    .changed();

                let mut use_pref_w = profile.preferred_w.is_some();
                if Self::retro_checkbox_row(left, &mut use_pref_w, "Use Preferred Width").clicked() {
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
                if Self::retro_checkbox_row(right, &mut use_pref_h, "Use Preferred Height").clicked()
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
                if Self::retro_checkbox_row(right, &mut profile.mouse_passthrough, "Mouse passthrough")
                    .clicked()
                {
                    changed = true;
                }
                if Self::retro_checkbox_row(right, &mut profile.open_fullscreen, "Open fullscreen")
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
                        "Show ROBCO Word Processor",
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
                        for name in self.edit_program_entries(self.settings.edit_target) {
                            ui.horizontal(|ui| {
                                ui.label(&name);
                                if ui.button("Delete").clicked() {
                                    self.delete_program_entry(self.settings.edit_target, &name);
                                    self.settings.status = self.shell_status.clone();
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
                let value_label = if matches!(self.settings.edit_target, EditMenuTarget::Documents) {
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
                        self.settings.status = "Error: Invalid input.".to_string();
                    } else {
                        match self.settings.edit_target {
                            EditMenuTarget::Documents => self.add_document_category(name, value),
                            target => self.add_program_entry(target, name, value),
                        }
                        self.settings.status = self.shell_status.clone();
                        if !self.settings.status.to_ascii_lowercase().starts_with("error") {
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
        let mut users: Vec<(String, UserRecord)> = load_users().into_iter().collect();
        users.sort_by(|a, b| a.0.cmp(&b.0));
        egui::ScrollArea::vertical().show(ui, |ui| {
            for (name, record) in users {
                ui.label(format!(
                    "{} | auth: {} | admin: {}",
                    name,
                    match record.auth_method {
                        AuthMethod::Password => "Password",
                        AuthMethod::NoPassword => "No Password",
                        AuthMethod::HackingMinigame => "Hacking Minigame",
                    },
                    if record.is_admin { "yes" } else { "no" }
                ));
            }
        });
    }

    fn draw_settings_user_create_panel(&mut self, ui: &mut egui::Ui) {
        let mut users: Vec<String> = load_users().keys().cloned().collect();
        users.sort();
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
                        .selected_text(RichText::new(match self.settings.user_create_auth {
                            AuthMethod::Password => "Password",
                            AuthMethod::NoPassword => "No Password",
                            AuthMethod::HackingMinigame => "Hacking Minigame",
                        }).color(current_palette().fg))
                        .show_ui(left, |ui| {
                            Self::apply_settings_control_style(ui);
                            for auth in [
                                AuthMethod::Password,
                                AuthMethod::NoPassword,
                                AuthMethod::HackingMinigame,
                            ] {
                                let label = match auth {
                                    AuthMethod::Password => "Password",
                                    AuthMethod::NoPassword => "No Password",
                                    AuthMethod::HackingMinigame => "Hacking Minigame",
                                };
                                if Self::retro_choice_button(
                                    ui,
                                    label,
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
                    self.settings.status = "Username cannot be empty.".to_string();
                } else if users.iter().any(|name| name == &username) {
                    self.settings.status = "User already exists.".to_string();
                } else {
                    match self.settings.user_create_auth {
                        AuthMethod::Password => {
                            if self.settings.user_create_password.is_empty() {
                                self.settings.status = "Password cannot be empty.".to_string();
                            } else if self.settings.user_create_password
                                != self.settings.user_create_password_confirm
                            {
                                self.settings.status = "Passwords do not match.".to_string();
                            } else {
                                self.save_user_and_status(
                                    &username,
                                    UserRecord {
                                        password_hash: crate::core::auth::hash_password(
                                            &self.settings.user_create_password,
                                        ),
                                        is_admin: false,
                                        auth_method: AuthMethod::Password,
                                    },
                                    format!("User '{username}' created."),
                                );
                                self.settings.status = self.shell_status.clone();
                                self.settings.user_create_username.clear();
                                self.settings.user_create_password.clear();
                                self.settings.user_create_password_confirm.clear();
                                self.settings.user_selected = username;
                                self.settings.user_selected_loaded_for.clear();
                            }
                        }
                        AuthMethod::NoPassword | AuthMethod::HackingMinigame => {
                            self.save_user_and_status(
                                &username,
                                UserRecord {
                                    password_hash: String::new(),
                                    is_admin: false,
                                    auth_method: self.settings.user_create_auth.clone(),
                                },
                                format!("User '{username}' created."),
                            );
                            self.settings.status = self.shell_status.clone();
                            self.settings.user_create_username.clear();
                            self.settings.user_selected = username;
                            self.settings.user_selected_loaded_for.clear();
                        }
                    }
                }
            });
        });
    }

    fn draw_settings_user_edit_panel(&mut self, ui: &mut egui::Ui, current_only: bool) {
        let current_username = self.session.as_ref().map(|s| s.username.clone());
        let mut users: Vec<(String, UserRecord)> = load_users().into_iter().collect();
        users.sort_by(|a, b| a.0.cmp(&b.0));
        let names: Vec<String> = users.iter().map(|(name, _)| name.clone()).collect();
        if names.is_empty() {
            ui.small("No users found.");
            return;
        }
        if current_only {
            self.settings.user_selected = current_username.clone().unwrap_or_default();
        } else if !names.iter().any(|name| name == &self.settings.user_selected) {
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
                    if current_only { "Edit Current User" } else { "Edit User" },
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
                                match record.auth_method {
                                    AuthMethod::Password => "Password",
                                    AuthMethod::NoPassword => "No Password",
                                    AuthMethod::HackingMinigame => "Hacking Minigame",
                                },
                                if record.is_admin { "yes" } else { "no" }
                            ));
                        }
                        left.add_space(8.0);
                        left.label("New Auth");
                        egui::ComboBox::from_id_salt("native_settings_user_edit_auth")
                            .selected_text(RichText::new(match self.settings.user_edit_auth {
                                AuthMethod::Password => "Password",
                                AuthMethod::NoPassword => "No Password",
                                AuthMethod::HackingMinigame => "Hacking Minigame",
                            }).color(current_palette().fg))
                            .show_ui(left, |ui| {
                                Self::apply_settings_control_style(ui);
                                for auth in [
                                    AuthMethod::Password,
                                    AuthMethod::NoPassword,
                                    AuthMethod::HackingMinigame,
                                ] {
                                    let label = match auth {
                                        AuthMethod::Password => "Password",
                                        AuthMethod::NoPassword => "No Password",
                                        AuthMethod::HackingMinigame => "Hacking Minigame",
                                    };
                                    if Self::retro_choice_button(
                                        ui,
                                        label,
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
                                self.settings.status = "Password cannot be empty.".to_string();
                            } else if self.settings.user_edit_password
                                != self.settings.user_edit_password_confirm
                            {
                                self.settings.status = "Passwords do not match.".to_string();
                            } else {
                                let password_hash =
                                    crate::core::auth::hash_password(&self.settings.user_edit_password);
                                self.update_user_record(
                                    &username,
                                    |record| {
                                        record.password_hash = password_hash;
                                        record.auth_method = AuthMethod::Password;
                                    },
                                    format!("Auth method updated for '{username}'."),
                                );
                                self.settings.status = self.shell_status.clone();
                                self.settings.user_edit_password.clear();
                                self.settings.user_edit_password_confirm.clear();
                                self.settings.user_selected_loaded_for.clear();
                            }
                        }
                        AuthMethod::NoPassword | AuthMethod::HackingMinigame => {
                            let new_method = self.settings.user_edit_auth.clone();
                            self.update_user_record(
                                &username,
                                |record| {
                                    record.password_hash.clear();
                                    record.auth_method = new_method;
                                },
                                format!("Auth method updated for '{username}'."),
                            );
                            self.settings.status = self.shell_status.clone();
                            self.settings.user_selected_loaded_for.clear();
                        }
                    }
                }

                if Self::retro_full_width_button(right, "Toggle Admin").clicked() {
                    if !current_only {
                        let username = self.settings.user_selected.clone();
                        let mut db = load_users();
                        if let Some(record) = db.get_mut(&username) {
                            record.is_admin = !record.is_admin;
                            let label = if record.is_admin { "granted" } else { "revoked" };
                            save_users(&db);
                            self.settings.status = format!("Admin {label} for '{username}'.");
                            self.settings.user_selected_loaded_for.clear();
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
                            let mut db = load_users();
                            db.remove(&username);
                            save_users(&db);
                            self.settings.status = format!("User '{username}' deleted.");
                            self.settings.user_delete_confirm.clear();
                            self.settings.user_selected_loaded_for.clear();
                        } else {
                            self.settings.user_delete_confirm = self.settings.user_selected.clone();
                            self.settings.status =
                                "Click Delete User again to confirm.".to_string();
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

    fn draw_applications(&mut self, ctx: &Context) {
        if !self.applications.open || self.desktop_window_is_minimized(DesktopWindow::Applications) {
            return;
        }
        let mut open = self.applications.open;
        let mut close_after_launch = false;
        let maximized = self.desktop_window_is_maximized(DesktopWindow::Applications);
        let restore = self.take_desktop_window_restore_dims(DesktopWindow::Applications);
        let mut header_action = DesktopHeaderAction::None;
        let generation = self.desktop_window_generation(DesktopWindow::Applications);
        let mut window = egui::Window::new("Applications")
            .id(Id::new(("native_applications", generation)))
            .open(&mut open)
            .title_bar(false)
            .frame(Self::desktop_window_frame())
            .resizable(true)
            .min_size([320.0, 250.0])
            .default_size([420.0, 380.0]);
        if maximized {
            let rect = Self::desktop_workspace_rect(ctx);
            window = window
                .movable(false)
                .resizable(false)
                .fixed_pos(rect.min)
                .fixed_size(rect.size());
        } else if let Some((pos, size)) = restore {
            window = window.current_pos(pos).default_size(size);
        }
        let shown = window.show(ctx, |ui| {
                Self::apply_settings_control_style(ui);
                header_action = Self::draw_desktop_window_header(ui, "Applications", maximized);
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.heading("Built-in");
                    if self.settings.draft.builtin_menu_visibility.text_editor
                        && Self::retro_full_width_button(ui, "ROBCO Word Processor").clicked()
                    {
                        if self.editor.path.is_none() {
                            self.new_document();
                        } else {
                            self.open_desktop_window(DesktopWindow::Editor);
                        }
                    }
                    if self.settings.draft.builtin_menu_visibility.nuke_codes
                        && Self::retro_full_width_button(ui, "Nuke Codes").clicked()
                    {
                        close_after_launch = true;
                        self.open_desktop_nuke_codes();
                    }
                    ui.separator();
                    ui.heading("Configured Apps");
                    for name in app_names() {
                        if Self::retro_full_width_button(ui, &name).clicked() {
                            close_after_launch = true;
                            let apps = load_apps();
                            match resolve_program_command(&name, &apps) {
                                Ok(cmd) => self.open_desktop_pty(&name, &cmd),
                                Err(err) => self.shell_status = err,
                            }
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
        if let Some(inner) = shown.as_ref() {
            Self::attach_generic_context_menu(&mut self.context_menu_action, &inner.response);
        }
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
        match header_action {
            DesktopHeaderAction::None => {}
            DesktopHeaderAction::Close => open = false,
            DesktopHeaderAction::Minimize => self.set_desktop_window_minimized(DesktopWindow::Applications, true),
            DesktopHeaderAction::ToggleMaximize => {
                self.toggle_desktop_window_maximized(DesktopWindow::Applications, shown_rect)
            }
        }
        self.update_desktop_window_state(DesktopWindow::Applications, open);
    }

    fn draw_nuke_codes_window(&mut self, ctx: &Context) {
        if !self.desktop_nuke_codes_open
            || self.desktop_window_is_minimized(DesktopWindow::NukeCodes)
        {
            return;
        }
        let mut open = self.desktop_nuke_codes_open;
        let maximized = self.desktop_window_is_maximized(DesktopWindow::NukeCodes);
        let restore = self.take_desktop_window_restore_dims(DesktopWindow::NukeCodes);
        let mut header_action = DesktopHeaderAction::None;
        let mut refresh = false;
        let generation = self.desktop_window_generation(DesktopWindow::NukeCodes);
        let mut window = egui::Window::new("Nuke Codes")
            .id(Id::new(("native_nuke_codes", generation)))
            .open(&mut open)
            .title_bar(false)
            .frame(Self::desktop_window_frame())
            .resizable(true)
            .min_size([300.0, 200.0])
            .default_size([480.0, 260.0]);
        if maximized {
            let rect = Self::desktop_workspace_rect(ctx);
            window = window
                .movable(false)
                .resizable(false)
                .fixed_pos(rect.min)
                .fixed_size(rect.size());
        } else if let Some((pos, size)) = restore {
            window = window.current_pos(pos).default_size(size);
        }
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
        if let Some(inner) = shown.as_ref() {
            Self::attach_generic_context_menu(&mut self.context_menu_action, &inner.response);
        }
        self.maybe_activate_desktop_window_from_click(
            ctx,
            DesktopWindow::NukeCodes,
            shown_contains_pointer,
        );
        if refresh {
            self.terminal_nuke_codes = fetch_nuke_codes();
        }
        if !maximized {
            if let Some(rect) = shown_rect {
                self.note_desktop_window_rect(DesktopWindow::NukeCodes, rect);
            }
        }
        match header_action {
            DesktopHeaderAction::None => {}
            DesktopHeaderAction::Close => open = false,
            DesktopHeaderAction::Minimize => {
                self.set_desktop_window_minimized(DesktopWindow::NukeCodes, true)
            }
            DesktopHeaderAction::ToggleMaximize => {
                self.toggle_desktop_window_maximized(DesktopWindow::NukeCodes, shown_rect)
            }
        }
        self.update_desktop_window_state(DesktopWindow::NukeCodes, open);
    }

    fn draw_desktop_pty_window(&mut self, ctx: &Context) {
        if self.desktop_window_is_minimized(DesktopWindow::PtyApp) {
            return;
        }
        let maximized = self.desktop_window_is_maximized(DesktopWindow::PtyApp);
        let restore = self.take_desktop_window_restore_dims(DesktopWindow::PtyApp);
        let generation = self.desktop_window_generation(DesktopWindow::PtyApp);
        let default_size = Self::desktop_default_window_size(DesktopWindow::PtyApp);
        let default_pos = Self::desktop_default_window_pos(ctx, default_size);
        let Some(state) = self.terminal_pty.as_mut() else {
            self.update_desktop_window_state(DesktopWindow::PtyApp, false);
            return;
        };
        let mut open = true;
        let mut header_action = DesktopHeaderAction::None;
        let title = state.title.clone();
        let mut event = PtyScreenEvent::None;
        let mut window = egui::Window::new(title.clone())
            .id(Id::new(("native_desktop_pty", generation)))
            .open(&mut open)
            .title_bar(false)
            .frame(Self::desktop_window_frame())
            .resizable(true)
            .default_pos(default_pos)
            .default_size(default_size);
        if maximized {
            let rect = Self::desktop_workspace_rect(ctx);
            window = window
                .movable(false)
                .resizable(false)
                .fixed_pos(rect.min)
                .fixed_size(rect.size());
        } else if let Some((pos, size)) = restore {
            let size = Self::desktop_clamp_window_size(ctx, size, egui::vec2(640.0, 420.0));
            let pos = Self::desktop_clamp_window_pos(ctx, pos, size);
            window = window.current_pos(pos).default_size(size);
        }
        let shown = window.show(ctx, |ui| {
                Self::apply_settings_control_style(ui);
                header_action = Self::draw_desktop_window_header(ui, &title, maximized);
                let available = ui.available_size();
                let cols = ((available.x / 12.0).floor() as usize).clamp(40, 220);
                let rows = ((available.y / 27.0).floor() as usize).clamp(20, 60);
                ui.allocate_ui_with_layout(available, Layout::top_down(egui::Align::Min), |ui| {
                    event = draw_embedded_pty_in_ui(ui, ctx, state, cols, rows);
                });
            });
        let shown_rect = shown.as_ref().map(|inner| inner.response.rect);
        let shown_contains_pointer = shown
            .as_ref()
            .is_some_and(|inner| inner.response.contains_pointer());
        if let Some(inner) = shown.as_ref() {
            Self::attach_generic_context_menu(&mut self.context_menu_action, &inner.response);
        }
        let completion_message = state.completion_message.clone();
        let title_for_exit = state.title.clone();
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

        match event {
            PtyScreenEvent::None => {}
            PtyScreenEvent::CloseRequested => open = false,
            PtyScreenEvent::ProcessExited => {
                open = false;
                if let Some(msg) = completion_message.as_ref() {
                    self.shell_status = msg.to_string();
                } else {
                    self.shell_status = format!("{title_for_exit} exited.");
                }
            }
        }

        match header_action {
            DesktopHeaderAction::None => {}
            DesktopHeaderAction::Close => open = false,
            DesktopHeaderAction::Minimize => self.set_desktop_window_minimized(DesktopWindow::PtyApp, true),
            DesktopHeaderAction::ToggleMaximize => {
                self.toggle_desktop_window_maximized(DesktopWindow::PtyApp, shown_rect)
            }
        }
        self.update_desktop_window_state(DesktopWindow::PtyApp, open);
    }

    fn draw_terminal_mode(&mut self, ctx: &Context) {
        if !self.terminal_mode.open || self.desktop_window_is_minimized(DesktopWindow::TerminalMode) {
            return;
        }
        let _ = ctx;
        self.terminal_mode.open = false;
        self.desktop_window_states.remove(&DesktopWindow::TerminalMode);
        self.open_desktop_terminal_shell();
    }

    fn handle_desktop_file_manager_shortcuts(&mut self, ctx: &Context) {
        if self.desktop_active_window != Some(DesktopWindow::FileManager) || self.terminal_prompt.is_some() {
            return;
        }
        if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(Key::C)) {
            self.run_file_manager_action(FileManagerActionRequest::Copy);
        } else if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(Key::X)) {
            self.run_file_manager_action(FileManagerActionRequest::Cut);
        } else if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(Key::V)) {
            self.run_file_manager_action(FileManagerActionRequest::Paste);
        } else if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(Key::D)) {
            self.run_file_manager_action(FileManagerActionRequest::Duplicate);
        } else if ctx.input(|i| i.key_pressed(Key::F2)) {
            self.open_file_manager_rename_prompt();
        } else if ctx.input(|i| i.modifiers.ctrl && i.modifiers.shift && i.key_pressed(Key::M)) {
            self.open_file_manager_move_prompt();
        } else if ctx.input(|i| i.key_pressed(Key::Delete)) {
            self.run_file_manager_action(FileManagerActionRequest::Delete);
        } else if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(Key::Z)) {
            self.run_file_manager_action(FileManagerActionRequest::Undo);
        } else if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(Key::Y)) {
            self.run_file_manager_action(FileManagerActionRequest::Redo);
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
        apply_native_appearance(ctx);

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
                    FlashAction::StartHacking { username } => {
                        crate::sound::play_navigate();
                        self.login_mode = LoginScreenMode::Hacking;
                        self.login_hacking = Some(LoginHackingState {
                            username,
                            game: HackingGame::new(
                                crate::config::get_settings().hacking_difficulty,
                            ),
                        });
                    }
                    FlashAction::LaunchPty {
                        title,
                        argv,
                        return_screen,
                        status,
                        completion_message,
                    } => {
                        self.open_embedded_pty(&title, &argv, return_screen);
                        if let Some(state) = self.terminal_pty.as_mut() {
                            state.completion_message = completion_message;
                        }
                        self.shell_status = status;
                    }
                }
            } else {
                ctx.request_repaint_after(flash.until.saturating_duration_since(Instant::now()));
                let layout = self.terminal_layout();
                self.draw_terminal_footer_spacer(ctx);
                let show_hacking_wait = self.session.is_none()
                    && matches!(self.login_mode, LoginScreenMode::Hacking)
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

        if self.session.is_none() {
            self.draw_terminal_footer_spacer(ctx);
            self.draw_login(ctx);
            return;
        }

        if self.desktop_mode_open {
            self.capture_session_switch_shortcuts(ctx);
            if session::has_switch_request() {
                self.apply_pending_session_switch();
            }
        }

        self.dispatch_context_menu_action(ctx);

        if !self.desktop_mode_open
            && !matches!(self.terminal_screen, TerminalScreen::PtyApp)
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
            self.handle_desktop_file_manager_shortcuts(ctx);
            self.draw_top_bar(ctx);
            self.draw_desktop_taskbar(ctx);
            self.draw_desktop(ctx);
        } else {
            self.draw_terminal_footer_spacer(ctx);
            if self.suppress_next_menu_submit {
                ctx.input_mut(|i| {
                    i.consume_key(egui::Modifiers::NONE, Key::Enter);
                    i.consume_key(egui::Modifiers::NONE, Key::Space);
                });
                self.suppress_next_menu_submit = false;
            }
            match self.terminal_screen {
                TerminalScreen::MainMenu => self.draw_terminal_main_menu(ctx),
                TerminalScreen::Applications => self.draw_terminal_applications(ctx),
                TerminalScreen::Documents => self.draw_terminal_documents(ctx),
                TerminalScreen::Logs => self.draw_terminal_logs(ctx),
                TerminalScreen::Network => self.draw_terminal_network(ctx),
                TerminalScreen::Games => self.draw_terminal_games(ctx),
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
        } else {
            self.draw_file_manager(ctx);
            self.draw_editor(ctx);
            self.draw_settings(ctx);
            self.draw_applications(ctx);
            self.draw_terminal_mode(ctx);
        }
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
    use crate::core::auth::{load_users, save_users, AuthMethod, UserRecord};
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
        app.main_menu_idx = idx;
        app.terminal_screen = screen;
        app.terminal_settings_idx = idx;
        app.terminal_user_management_idx = idx;
        app.file_manager.open = true;
        app.file_manager.cwd = PathBuf::from(format!("/tmp/{tag}"));
        app.file_manager.selected = Some(PathBuf::from(format!("/tmp/{tag}/selected.txt")));
        app.editor.open = true;
        app.editor.path = Some(PathBuf::from(format!("/tmp/{tag}/doc.txt")));
        app.editor.text = tag.to_string();
        app.editor.dirty = idx % 2 == 0;
        app.editor.status = format!("status-{tag}");
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
        app.main_menu_idx = 3;
        app.terminal_screen = TerminalScreen::Connections;
        app.terminal_settings_idx = 2;
        app.terminal_user_management_idx = 4;
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
        app.main_menu_idx = 0;
        app.terminal_screen = TerminalScreen::MainMenu;
        app.terminal_settings_idx = 0;
        app.terminal_user_management_idx = 0;
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
        assert_eq!(app.main_menu_idx, 3);
        assert!(matches!(app.terminal_screen, TerminalScreen::Connections));
        assert_eq!(app.terminal_settings_idx, 2);
        assert_eq!(app.terminal_user_management_idx, 4);
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
        assert!(matches!(app.terminal_screen, TerminalScreen::Settings));
        assert_eq!(app.main_menu_idx, 2);
        assert_eq!(app.editor.text, "u1-a");

        session::request_switch(s2);
        app.apply_pending_session_switch();
        assert_eq!(session::active_idx(), s2);
        assert!(matches!(app.terminal_screen, TerminalScreen::Connections));
        assert_eq!(app.main_menu_idx, 7);
        assert_eq!(app.editor.text, "u2-b");

        session::request_switch(s1);
        app.apply_pending_session_switch();
        assert_eq!(session::active_idx(), s1);
        assert!(matches!(app.terminal_screen, TerminalScreen::Settings));
        assert_eq!(app.main_menu_idx, 2);
        assert_eq!(app.editor.text, "u1-a");
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
            app.terminal_screen,
            TerminalScreen::ProgramInstaller
        ));
        assert_eq!(app.main_menu_idx, 5);
        assert_eq!(app.editor.text, "u2");
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
            assert_eq!(app.terminal_screen, screen);
            assert_eq!(app.editor.text, format!("u1-{idx}"));

            session::request_switch(s2);
            app.apply_pending_session_switch();
            assert_eq!(session::active_idx(), s2);
            assert!(matches!(app.terminal_screen, TerminalScreen::MainMenu));
            assert_eq!(app.editor.text, format!("u2-{idx}"));

            session::request_switch(s1);
            app.apply_pending_session_switch();
            assert_eq!(session::active_idx(), s1);
            assert_eq!(app.terminal_screen, screen);
            assert_eq!(app.editor.text, format!("u1-{idx}"));
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
        app.terminal_screen = TerminalScreen::NukeCodes;
        app.terminal_nuke_codes_return = TerminalScreen::Applications;
        app.terminal_nuke_codes =
            NukeCodesView::Data(crate::native::nuke_codes_screen::NukeCodesData {
                alpha: "11111111".to_string(),
                bravo: "22222222".to_string(),
                charlie: "33333333".to_string(),
                source: "Test Source".to_string(),
                fetched_at: "2026-03-01 06:00 PM".to_string(),
            });
        app.park_active_session_runtime();

        session::set_active(s2);
        assert!(app.sync_active_session_identity());
        app.terminal_screen = TerminalScreen::MainMenu;
        app.terminal_nuke_codes_return = TerminalScreen::MainMenu;
        app.terminal_nuke_codes = NukeCodesView::Error("offline".to_string());
        app.park_active_session_runtime();

        session::request_switch(s1);
        app.apply_pending_session_switch();
        assert_eq!(session::active_idx(), s1);
        assert_eq!(app.terminal_screen, TerminalScreen::NukeCodes);
        assert_eq!(app.terminal_nuke_codes_return, TerminalScreen::Applications);
        match &app.terminal_nuke_codes {
            NukeCodesView::Data(data) => {
                assert_eq!(data.alpha, "11111111");
                assert_eq!(data.bravo, "22222222");
                assert_eq!(data.charlie, "33333333");
            }
            other => panic!("expected NukeCodes data, got {other:?}"),
        }
    }
}
