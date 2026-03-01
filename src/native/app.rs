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
    draw_installer_screen, InstallerEvent, InstallerPackageAction, TerminalInstallerState,
};
use super::menu::{
    draw_terminal_menu_screen, login_menu_rows_from_users, SettingsChoiceOverlay, TerminalScreen,
    UserManagementMode,
};
use super::programs_screen::{draw_programs_menu, resolve_program_command, ProgramMenuEvent};
use super::prompt::{
    draw_terminal_flash, draw_terminal_prompt_overlay, FlashAction, TerminalFlash, TerminalPrompt,
    TerminalPromptAction, TerminalPromptKind,
};
use super::prompt_flow::{handle_prompt_input, PromptOutcome};
use super::pty_screen::{
    draw_embedded_pty, spawn_embedded_pty_with_options, NativePtyState, PtyScreenEvent,
};
use super::retro_ui::{configure_visuals, current_palette, RetroScreen};
use super::settings_screen::{run_terminal_settings_screen, TerminalSettingsEvent};
use super::shell_actions::{
    resolve_login_selection, resolve_main_menu_action, LoginSelectionAction,
    MainMenuSelectionAction,
};
use super::shell_screen::{draw_login_screen, draw_main_menu_screen};
use super::terminal::{launch_plan, launch_terminal_mode};
use super::user_management::{
    handle_selection as handle_user_management_selection,
    screen_for_mode as user_management_screen_for_mode, UserManagementAction,
};
use crate::config::ConnectionKind;
use crate::config::{
    cycle_hacking_difficulty, get_settings, load_apps, load_categories, load_games, load_networks,
    persist_settings, save_apps, save_categories, save_games, save_networks, set_current_user,
    update_settings, OpenMode, Settings, THEMES,
};
use crate::connections::{connect_connection, network_requires_password, DiscoveredConnection};
use crate::core::auth::{load_users, read_session, save_users, UserRecord};
use crate::core::hacking::HackingGame;
use crate::default_apps::{parse_custom_command_line, set_binding_for_slot, DefaultAppSlot};
use chrono::Local;
use eframe::egui::{
    self, Align2, Color32, Context, FontData, FontDefinitions, FontFamily, FontId, Id, Key,
    RichText, TextEdit, TextStyle, TopBottomPanel,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::PathBuf;
use std::time::{Duration, Instant};

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

#[derive(Debug)]
struct EditorWindow {
    open: bool,
    path: Option<PathBuf>,
    text: String,
    dirty: bool,
    status: String,
}

#[derive(Debug)]
struct SettingsWindow {
    open: bool,
    draft: Settings,
    status: String,
}

#[derive(Debug, Default)]
struct ApplicationsWindow {
    open: bool,
    status: String,
}

#[derive(Debug, Default)]
struct TerminalModeWindow {
    open: bool,
    status: String,
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
    terminal_mode: TerminalModeWindow,
    start_open: bool,
    desktop_mode_open: bool,
    main_menu_idx: usize,
    terminal_screen: TerminalScreen,
    terminal_apps_idx: usize,
    terminal_documents_idx: usize,
    terminal_logs_idx: usize,
    terminal_network_idx: usize,
    terminal_games_idx: usize,
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
    shell_status: String,
}

impl Default for RobcoNativeApp {
    fn default() -> Self {
        // Keep pre-login terminal rendering consistent with the most recent user session.
        if let Some(last_user) = read_session() {
            if load_users().contains_key(&last_user) {
                set_current_user(Some(&last_user));
            }
        }
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
            },
            applications: ApplicationsWindow::default(),
            terminal_mode: TerminalModeWindow::default(),
            start_open: true,
            desktop_mode_open: false,
            main_menu_idx: 0,
            terminal_screen: TerminalScreen::MainMenu,
            terminal_apps_idx: 0,
            terminal_documents_idx: 0,
            terminal_logs_idx: 0,
            terminal_network_idx: 0,
            terminal_games_idx: 0,
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
            terminal_flash: None,
            shell_status: String::new(),
        }
    }
}

impl RobcoNativeApp {
    fn terminal_layout(&self) -> TerminalLayout {
        terminal_layout_for_scale(self.settings.draft.native_ui_scale)
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
        self.terminal_mode.status.clear();
        self.start_open = true;
        self.desktop_mode_open = false;
        self.main_menu_idx = 0;
        self.terminal_screen = TerminalScreen::MainMenu;
        self.terminal_apps_idx = 0;
        self.terminal_documents_idx = 0;
        self.terminal_logs_idx = 0;
        self.terminal_network_idx = 0;
        self.terminal_games_idx = 0;
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
        self.terminal_flash = None;
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

    fn queue_login(&mut self, username: String, user: UserRecord) {
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
            self.login.error = "Select a user.".to_string();
            return;
        }
        match authenticate(&username, &self.login.password) {
            Ok(user) => self.queue_login(username, user),
            Err(err) => self.login.error = err.to_string(),
        }
    }

    fn login_usernames(&self) -> Vec<String> {
        let mut usernames: Vec<String> = load_users().keys().cloned().collect();
        usernames.sort();
        usernames
    }

    fn queue_terminal_flash(&mut self, message: impl Into<String>, ms: u64, action: FlashAction) {
        self.terminal_flash = Some(TerminalFlash {
            message: message.into(),
            until: Instant::now() + Duration::from_millis(ms),
            action,
        });
    }

    fn begin_logout(&mut self) {
        if let Some(flash) = self.terminal_flash.as_ref() {
            if matches!(&flash.action, FlashAction::FinishLogout) {
                return;
            }
        }
        self.persist_snapshot();
        self.terminal_prompt = None;
        self.terminal_pty = None;
        self.terminal_screen = TerminalScreen::MainMenu;
        self.desktop_mode_open = false;
        self.queue_terminal_flash("Logging out...", 800, FlashAction::FinishLogout);
    }

    fn finish_logout(&mut self) {
        crate::config::reload_settings();
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
        self.applications.open = false;
        self.terminal_mode.open = false;
        self.desktop_mode_open = false;
        self.terminal_screen = TerminalScreen::MainMenu;
        self.terminal_apps_idx = 0;
        self.terminal_documents_idx = 0;
        self.terminal_logs_idx = 0;
        self.terminal_network_idx = 0;
        self.terminal_games_idx = 0;
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
        self.shell_status.clear();
    }

    fn open_password_prompt(&mut self, title: impl Into<String>, prompt: impl Into<String>) {
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
        self.terminal_prompt = Some(TerminalPrompt {
            kind: TerminalPromptKind::Confirm,
            title: title.into(),
            prompt: prompt.into(),
            buffer: String::new(),
            confirm_yes: true,
            action,
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
        let mut options = crate::pty::PtyLaunchOptions::default();
        options
            .env
            .push(("ROBCOS_PTY_RENDER".into(), "plain".into()));
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
                self.terminal_screen = TerminalScreen::PtyApp;
                self.shell_status = format!("Opened {title} in PTY.");
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
                ("ROBCOS_PTY_RENDER".into(), "plain".into()),
            ],
            top_bar: Some("ROBCO MAINTENANCE TERMLINK".into()),
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
                self.terminal_screen = TerminalScreen::PtyApp;
                self.shell_status = "Opened terminal shell in PTY.".to_string();
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
                self.editor.open = true;
                self.editor.status = "Opened document.".to_string();
            }
            Err(err) => {
                self.editor.status = format!("Open failed: {err}");
                self.editor.open = true;
            }
        }
    }

    fn activate_file_manager_selection(&mut self) {
        match self.file_manager.activate_selected() {
            FileManagerAction::None | FileManagerAction::ChangedDir => {}
            FileManagerAction::OpenFile(path) => self.open_path_in_editor(path),
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
        self.editor.open = true;
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
        self.terminal_screen = TerminalScreen::DocumentBrowser;
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
        self.editor.open = true;
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
                self.login.selected_username = username.clone();
                match authenticate(&username, "") {
                    Ok(user) => self.queue_login(username, user),
                    Err(err) => self.login.error = err.to_string(),
                }
            }
            LoginSelectionAction::StartHacking { username } => {
                self.login.selected_username = username.clone();
                self.queue_hacking_start(username);
            }
            LoginSelectionAction::ShowError(error) => {
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
                self.terminal_screen = screen;
                match screen {
                    TerminalScreen::Applications => self.terminal_apps_idx = selected_idx,
                    TerminalScreen::Documents => self.terminal_documents_idx = selected_idx,
                    TerminalScreen::Logs => self.terminal_logs_idx = selected_idx,
                    TerminalScreen::Network => self.terminal_network_idx = selected_idx,
                    TerminalScreen::Games => self.terminal_games_idx = selected_idx,
                    TerminalScreen::ProgramInstaller => {
                        self.terminal_installer.root_idx = selected_idx
                    }
                    TerminalScreen::Settings => self.terminal_settings_idx = selected_idx,
                    TerminalScreen::EditMenus => {}
                    TerminalScreen::Connections => {
                        self.terminal_connections.root_idx = selected_idx
                    }
                    TerminalScreen::DefaultApps => self.terminal_default_apps_idx = selected_idx,
                    TerminalScreen::About => {}
                    TerminalScreen::UserManagement => {
                        self.terminal_user_management_idx = selected_idx
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
                self.desktop_mode_open = true;
                self.shell_status = "Entered Desktop Mode.".to_string();
            }
            MainMenuSelectionAction::RefreshSettingsAndOpen => {
                self.settings.draft = current_settings();
                self.terminal_screen = TerminalScreen::Settings;
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
            self.terminal_settings_choice = None;
            return;
        }
        if self.terminal_default_app_slot.is_some() {
            self.terminal_default_app_slot = None;
            return;
        }
        if matches!(self.terminal_screen, TerminalScreen::Connections)
            && !self.terminal_connections.back()
        {
            self.shell_status.clear();
            return;
        }
        if matches!(self.terminal_screen, TerminalScreen::ProgramInstaller)
            && !self.terminal_installer.back()
        {
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
                self.terminal_screen = TerminalScreen::MainMenu;
                self.shell_status.clear();
            }
            TerminalScreen::Logs => {
                self.terminal_screen = TerminalScreen::Documents;
                self.shell_status.clear();
            }
            TerminalScreen::PtyApp => {
                if let Some(mut pty) = self.terminal_pty.take() {
                    pty.session.terminate();
                    self.terminal_screen = pty.return_screen;
                    self.shell_status = format!("Closed {}.", pty.title);
                } else {
                    self.terminal_screen = TerminalScreen::MainMenu;
                    self.shell_status.clear();
                }
            }
            TerminalScreen::ProgramInstaller => {
                self.terminal_screen = TerminalScreen::MainMenu;
                self.shell_status.clear();
                self.terminal_installer.reset();
            }
            TerminalScreen::Connections
            | TerminalScreen::DefaultApps
            | TerminalScreen::About
            | TerminalScreen::EditMenus => {
                self.terminal_screen = TerminalScreen::Settings;
                self.shell_status.clear();
            }
            TerminalScreen::DocumentBrowser => {
                self.terminal_screen = self.terminal_browser_return;
                self.shell_status.clear();
            }
        }
    }

    fn handle_terminal_prompt_input(&mut self, ctx: &Context) {
        let Some(prompt) = self.terminal_prompt.clone() else {
            return;
        };
        match handle_prompt_input(ctx, prompt) {
            PromptOutcome::Cancel => {
                self.terminal_prompt = None;
                self.login.password.clear();
                self.login.error.clear();
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
                self.terminal_user_management_mode =
                    UserManagementMode::CreateAuthMethod { username };
                self.terminal_user_management_idx = 0;
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
                self.terminal_user_management_mode = UserManagementMode::Root;
                self.terminal_user_management_idx = 0;
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
                self.terminal_user_management_mode = UserManagementMode::Root;
                self.terminal_user_management_idx = 0;
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
                self.terminal_user_management_mode = UserManagementMode::Root;
                self.terminal_user_management_idx = 0;
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
                self.terminal_user_management_mode = UserManagementMode::Root;
                self.terminal_user_management_idx = 0;
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
                self.terminal_user_management_mode = UserManagementMode::Root;
                self.terminal_user_management_idx = 0;
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
                            self.login.error = "Unknown user.".to_string();
                            self.login_mode = LoginScreenMode::SelectUser;
                            self.login_hacking = None;
                        }
                    }
                    HackingScreenEvent::LockedOut => {
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
        TopBottomPanel::top("native_top_bar").show(ctx, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.heading("RobCoOS Native");
                if let Some(session) = &self.session {
                    ui.separator();
                    ui.label(format!("User: {}", session.username));
                    if session.is_admin {
                        ui.label(RichText::new("ADMIN").strong());
                    }
                }
                ui.separator();
                if ui.button("Start").clicked() {
                    self.start_open = !self.start_open;
                }
                if ui.button("File Manager").clicked() {
                    self.file_manager.open = true;
                }
                if ui.button("Word Processor").clicked() {
                    self.editor.open = true;
                    if self.editor.path.is_none() {
                        self.new_document();
                    }
                }
                if ui.button("Settings").clicked() {
                    self.settings.open = true;
                }
                if ui.button("Apps").clicked() {
                    self.applications.open = true;
                }
                if ui.button("Terminal Mode").clicked() {
                    self.terminal_mode.open = true;
                }
                if ui.button("Log Out").clicked() {
                    self.begin_logout();
                }
            });
        });
    }

    fn draw_start_panel(&mut self, ctx: &Context) {
        if !self.start_open {
            return;
        }
        egui::SidePanel::left("native_start_panel")
            .default_width(220.0)
            .show(ctx, |ui| {
                ui.heading("Start");
                ui.separator();
                if ui.button("New Document").clicked() {
                    self.new_document();
                }
                if ui.button("Open File Manager").clicked() {
                    self.file_manager.open = true;
                }
                if ui.button("Open Settings").clicked() {
                    self.settings.open = true;
                }
                if ui.button("Open Applications").clicked() {
                    self.applications.open = true;
                }
                if ui.button("Launch Terminal Mode").clicked() {
                    self.terminal_mode.open = true;
                }
                if ui.button("Return To Terminal Menu").clicked() {
                    self.desktop_mode_open = false;
                }
                ui.separator();
                ui.label("Rewrite target:");
                ui.small("Native shell replacing the terminal-owned desktop.");
                ui.small("Terminal mode stays supported as a first-class product mode.");
            });
    }

    fn draw_desktop(&mut self, ctx: &Context) {
        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(current_palette().bg).inner_margin(0.0))
            .show(ctx, |ui| {
            ui.heading("Desktop Shell");
            ui.label("This is the new native workbench, not the old TUI desktop.");
            ui.add_space(8.0);
            ui.columns(3, |cols| {
                cols[0].group(|ui| {
                    ui.label(RichText::new("Desktop").strong());
                    ui.small("Native top bar, start panel, and app windows.");
                });
                cols[1].group(|ui| {
                    ui.label(RichText::new("Core").strong());
                    ui.small("Shared users, settings, and document storage paths.");
                });
                cols[2].group(|ui| {
                    ui.label(RichText::new("Dual Mode").strong());
                    ui.small(
                        "Terminal mode is preserved. Desktop mode is what this native shell is replacing.",
                    );
                    if ui.button("Launch Terminal Mode").clicked() {
                        self.terminal_mode.open = true;
                    }
                });
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
                self.shell_status = "Nuke Codes native app is pending rewrite.".to_string();
            } else if label == "Back" {
                self.terminal_screen = TerminalScreen::MainMenu;
                self.shell_status.clear();
            } else {
                let apps = load_apps();
                match resolve_program_command(label, &apps) {
                    Ok(cmd) => self.open_embedded_pty(label, &cmd, TerminalScreen::Applications),
                    Err(err) => self.shell_status = err,
                }
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
                    self.terminal_screen = TerminalScreen::Logs;
                    self.terminal_logs_idx = 0;
                    self.shell_status.clear();
                }
                "Back" => {
                    self.terminal_screen = TerminalScreen::MainMenu;
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
                    self.terminal_screen = TerminalScreen::Documents;
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
                self.terminal_screen = TerminalScreen::MainMenu;
                self.shell_status.clear();
            }
            TerminalSettingsEvent::OpenConnections => {
                self.terminal_screen = TerminalScreen::Connections;
                self.terminal_connections.reset();
                self.shell_status.clear();
            }
            TerminalSettingsEvent::OpenEditMenus => {
                self.terminal_screen = TerminalScreen::EditMenus;
                self.terminal_edit_menus.reset();
                self.shell_status.clear();
            }
            TerminalSettingsEvent::OpenDefaultApps => {
                self.terminal_screen = TerminalScreen::DefaultApps;
                self.terminal_default_apps_idx = 0;
                self.terminal_default_app_choice_idx = 0;
                self.terminal_default_app_slot = None;
                self.shell_status.clear();
            }
            TerminalSettingsEvent::OpenAbout => {
                self.terminal_screen = TerminalScreen::About;
                self.shell_status.clear();
            }
            TerminalSettingsEvent::EnterUserManagement => {
                self.terminal_screen = TerminalScreen::UserManagement;
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
                self.terminal_screen = TerminalScreen::Settings;
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
                self.terminal_screen = TerminalScreen::Settings;
                self.shell_status.clear();
            }
            ConnectionsEvent::OpenNetworkGroups => {
                self.terminal_connections.view =
                    super::connections_screen::ConnectionsView::NetworkGroups;
                self.shell_status.clear();
            }
            ConnectionsEvent::OpenBluetooth => {
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
                    self.terminal_screen = TerminalScreen::Settings;
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
                self.terminal_screen = TerminalScreen::Settings;
                self.shell_status.clear();
            }
            DefaultAppsEvent::OpenSlot(slot) => {
                self.terminal_default_app_slot = Some(slot);
                self.terminal_default_app_choice_idx = 0;
            }
            DefaultAppsEvent::CloseSlotPicker => {
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
            self.terminal_screen = TerminalScreen::Settings;
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
                self.terminal_screen = TerminalScreen::MainMenu;
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
                self.terminal_screen = TerminalScreen::MainMenu;
                self.shell_status.clear();
            }
            ProgramMenuEvent::Launch(name) => match resolve_program_command(&name, &games) {
                Ok(cmd) => self.open_embedded_pty(&name, &cmd, TerminalScreen::Games),
                Err(err) => self.shell_status = err,
            },
        }
    }

    fn draw_terminal_pty(&mut self, ctx: &Context) {
        let layout = self.terminal_layout();
        let Some(state) = self.terminal_pty.as_mut() else {
            self.terminal_screen = TerminalScreen::MainMenu;
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
                    self.terminal_screen = pty.return_screen;
                    self.shell_status = format!("{} exited.", pty.title);
                } else {
                    self.terminal_screen = TerminalScreen::MainMenu;
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
                self.terminal_screen = TerminalScreen::MainMenu;
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
            InstallerEvent::LaunchCommand { argv, status } => {
                self.open_embedded_pty(
                    "Program Installer",
                    &argv,
                    TerminalScreen::ProgramInstaller,
                );
                self.shell_status = status;
            }
            InstallerEvent::Status(status) => {
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
                    self.terminal_user_management_mode = mode;
                    self.terminal_user_management_idx = selected_idx;
                }
                UserManagementAction::BackToSettings => {
                    self.terminal_screen = TerminalScreen::Settings;
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
                        self.terminal_user_management_mode = UserManagementMode::Root;
                        self.terminal_user_management_idx = 0;
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
                        self.terminal_user_management_mode = UserManagementMode::Root;
                        self.terminal_user_management_idx = 0;
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
                    self.terminal_user_management_mode = UserManagementMode::Root;
                    self.terminal_user_management_idx = 0;
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
                        self.terminal_user_management_mode = UserManagementMode::Root;
                        self.terminal_user_management_idx = 0;
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
                        self.terminal_user_management_mode = UserManagementMode::Root;
                        self.terminal_user_management_idx = 0;
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
                    self.terminal_user_management_mode = UserManagementMode::Root;
                    self.terminal_user_management_idx = 0;
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

    fn draw_terminal_footer(&self, ctx: &Context) {
        let layout = self.terminal_layout();
        let now = Local::now();
        let left = now.format("%a %Y-%m-%d %I:%M%p").to_string();
        let mode = if self.desktop_mode_open {
            "desktop"
        } else {
            "terminal"
        };
        let center = if let Some(session) = &self.session {
            format!("[{} | {}]", session.username, mode)
        } else {
            "[*]".to_string()
        };
        TopBottomPanel::bottom("native_terminal_footer")
            .resizable(false)
            .exact_height(retro_footer_height())
            .show_separator_line(false)
            .frame(
                egui::Frame::none()
                    .fill(current_palette().bg)
                    .inner_margin(0.0),
            )
            .show(ctx, |ui| {
                let palette = current_palette();
                let (screen, _) = RetroScreen::new(ui, layout.cols, 1);
                let painter = ui.painter_at(screen.rect);
                screen.footer_bar(&painter, &palette, &left, &center, "44%");
            });
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

    fn draw_file_manager(&mut self, ctx: &Context) {
        if !self.file_manager.open {
            return;
        }
        let mut open = self.file_manager.open;
        egui::Window::new("File Manager")
            .id(Id::new("native_file_manager"))
            .open(&mut open)
            .default_size([700.0, 480.0])
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    if ui.button("Up").clicked() {
                        self.file_manager.up();
                    }
                    if ui.button("Word Processor Home").clicked() {
                        if let Some(session) = &self.session {
                            self.file_manager
                                .set_cwd(word_processor_dir(&session.username));
                        }
                    }
                    ui.label(self.file_manager.cwd.display().to_string());
                });
                ui.separator();
                egui::ScrollArea::vertical().show(ui, |ui| {
                    for row in self.file_manager.rows() {
                        let selected = self.file_manager.selected.as_ref() == Some(&row.path);
                        let label = if row.is_dir {
                            format!("[DIR] {}", row.label)
                        } else {
                            row.label.clone()
                        };
                        let response = ui.selectable_label(selected, label);
                        if response.clicked() {
                            self.file_manager.select(Some(row.path.clone()));
                        }
                        if response.double_clicked() {
                            self.file_manager.select(Some(row.path.clone()));
                            self.activate_file_manager_selection();
                        }
                    }
                });
                ui.separator();
                ui.horizontal(|ui| {
                    if ui.button("Open").clicked() {
                        self.activate_file_manager_selection();
                    }
                    if ui.button("New Document").clicked() {
                        self.new_document();
                    }
                });
            });
        self.file_manager.open = open;
    }

    fn draw_editor(&mut self, ctx: &Context) {
        if !self.editor.open {
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
                self.editor.open = false;
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
                            self.file_manager.open = true;
                        }
                        if ui.button("Close").clicked() {
                            self.editor.open = false;
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
        egui::Window::new(title)
            .id(Id::new("native_word_processor"))
            .open(&mut open)
            .default_size([820.0, 560.0])
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    if ui.button("New").clicked() {
                        self.new_document();
                    }
                    if ui.button("Save").clicked() {
                        self.save_editor();
                    }
                    if ui.button("Open File Manager").clicked() {
                        self.file_manager.open = true;
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
                let response = ui.add_sized(ui.available_size(), edit);
                if response.changed() {
                    self.editor.dirty = true;
                }
                if !self.editor.status.is_empty() {
                    ui.separator();
                    ui.small(&self.editor.status);
                }
            });
        self.editor.open = open;
    }

    fn draw_settings(&mut self, ctx: &Context) {
        if !self.settings.open {
            return;
        }
        let mut open = self.settings.open;
        egui::Window::new("Settings")
            .id(Id::new("native_settings"))
            .open(&mut open)
            .default_size([500.0, 360.0])
            .show(ctx, |ui| {
                let mut changed = false;
                changed |= ui
                    .checkbox(&mut self.settings.draft.sound, "Sound")
                    .changed();
                changed |= ui
                    .checkbox(&mut self.settings.draft.bootup, "Bootup")
                    .changed();
                changed |= ui
                    .checkbox(
                        &mut self.settings.draft.show_navigation_hints,
                        "Show navigation hints",
                    )
                    .changed();
                ui.horizontal(|ui| {
                    ui.label("Theme");
                    let mut current_idx = THEMES
                        .iter()
                        .position(|(name, _)| *name == self.settings.draft.theme)
                        .unwrap_or(0);
                    egui::ComboBox::from_id_salt("native_settings_theme")
                        .selected_text(THEMES[current_idx].0)
                        .show_ui(ui, |ui| {
                            for (idx, (name, _)) in THEMES.iter().enumerate() {
                                if ui.selectable_value(&mut current_idx, idx, *name).changed() {
                                    self.settings.draft.theme = (*name).to_string();
                                    changed = true;
                                }
                            }
                        });
                });
                ui.horizontal(|ui| {
                    ui.label("Default Open Mode");
                    changed |= ui
                        .selectable_value(
                            &mut self.settings.draft.default_open_mode,
                            OpenMode::Terminal,
                            "Terminal",
                        )
                        .changed();
                    changed |= ui
                        .selectable_value(
                            &mut self.settings.draft.default_open_mode,
                            OpenMode::Desktop,
                            "Desktop",
                        )
                        .changed();
                });
                ui.separator();
                if changed {
                    save_settings(self.settings.draft.clone());
                    self.settings.status = "Settings saved.".to_string();
                }
                if !self.settings.status.is_empty() {
                    ui.small(&self.settings.status);
                }
            });
        self.settings.open = open;
    }

    fn draw_applications(&mut self, ctx: &Context) {
        if !self.applications.open {
            return;
        }
        let mut open = self.applications.open;
        egui::Window::new("Applications")
            .id(Id::new("native_applications"))
            .open(&mut open)
            .default_size([420.0, 380.0])
            .show(ctx, |ui| {
                ui.heading("Built-in");
                if ui.button("ROBCO Word Processor").clicked() {
                    self.editor.open = true;
                    if self.editor.path.is_none() {
                        self.new_document();
                    }
                }
                if ui.button("Nuke Codes").clicked() {
                    self.applications.status = "Nuke Codes UI is not rewritten yet.".to_string();
                }
                ui.separator();
                ui.heading("Configured Apps");
                for name in app_names() {
                    if ui.button(&name).clicked() {
                        self.applications.status =
                            format!("External app launch for '{name}' is pending rewrite.");
                    }
                }
                if !self.applications.status.is_empty() {
                    ui.separator();
                    ui.small(&self.applications.status);
                }
            });
        self.applications.open = open;
    }

    fn draw_terminal_mode(&mut self, ctx: &Context) {
        if !self.terminal_mode.open {
            return;
        }
        let plan = launch_plan();
        let mut open = self.terminal_mode.open;
        egui::Window::new("Terminal Mode")
            .id(Id::new("native_terminal_mode"))
            .open(&mut open)
            .default_size([480.0, 220.0])
            .show(ctx, |ui| {
                ui.label("Terminal mode stays a first-class product mode.");
                ui.small(
                    "The native shell launches the existing `robcos` TUI in your system terminal.",
                );
                ui.separator();
                ui.label(format!("Launch path: {}", plan.display));
                ui.monospace(format!("{} {}", plan.program, plan.args.join(" ")));
                if ui.button("Open Terminal Mode").clicked() {
                    match launch_terminal_mode() {
                        Ok(used) => {
                            self.terminal_mode.status =
                                format!("Launched terminal mode via {}.", used.display);
                        }
                        Err(err) => {
                            self.terminal_mode.status = format!("Launch failed: {err}");
                        }
                    }
                }
                if !self.terminal_mode.status.is_empty() {
                    ui.separator();
                    ui.small(&self.terminal_mode.status);
                }
            });
        self.terminal_mode.open = open;
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
                    FlashAction::ExitApp => {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                    FlashAction::FinishLogout => self.finish_logout(),
                    FlashAction::FinishLogin { username, user } => {
                        self.restore_for_user(&username, &user);
                    }
                    FlashAction::StartHacking { username } => {
                        self.login_mode = LoginScreenMode::Hacking;
                        self.login_hacking = Some(LoginHackingState {
                            username,
                            game: HackingGame::new(
                                crate::config::get_settings().hacking_difficulty,
                            ),
                        });
                    }
                }
            } else {
                ctx.request_repaint_after(flash.until.saturating_duration_since(Instant::now()));
                let layout = self.terminal_layout();
                if self.session.is_some() {
                    self.draw_terminal_footer(ctx);
                } else {
                    self.draw_terminal_footer_spacer(ctx);
                }
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
                return;
            }
        }

        if self.session.is_none() {
            self.draw_terminal_footer_spacer(ctx);
            self.draw_login(ctx);
            return;
        }

        if !self.desktop_mode_open
            && !matches!(self.terminal_screen, TerminalScreen::PtyApp)
            && !self.editor.open
            && ctx.input(|i| i.key_pressed(Key::Escape) || i.key_pressed(Key::Tab))
        {
            self.handle_terminal_back();
        }

        self.draw_terminal_footer(ctx);

        if self.desktop_mode_open {
            self.draw_top_bar(ctx);
            self.draw_start_panel(ctx);
            self.draw_desktop(ctx);
        } else {
            if self.terminal_prompt.is_some() {
                self.handle_terminal_prompt_input(ctx);
            }
            match self.terminal_screen {
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
            self.draw_terminal_prompt_overlay_global(ctx);
        }
        self.draw_file_manager(ctx);
        self.draw_editor(ctx);
        self.draw_settings(ctx);
        self.draw_applications(ctx);
        self.draw_terminal_mode(ctx);

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
