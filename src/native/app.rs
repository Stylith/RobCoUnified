use super::data::{
    app_names, authenticate, current_settings, home_dir_fallback, read_shell_snapshot,
    read_text_file, save_settings, save_text_file, word_processor_dir, write_shell_snapshot,
};
use super::file_manager::{FileManagerAction, NativeFileManagerState};
use super::retro_ui::{
    configure_visuals, current_palette, RetroScreen,
};
use super::terminal::{launch_plan, launch_terminal_mode};
use crate::config::{OpenMode, Settings, HEADER_LINES, THEMES};
use crate::core::auth::{load_users, save_users, UserRecord};
use chrono::Local;
use eframe::egui::{
    self, Align2, Color32, Context, FontData, FontDefinitions, FontFamily, FontId, Id, Key,
    RichText, TextEdit, TextStyle, TopBottomPanel,
};
use serde::{Deserialize, Serialize};
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SettingsChoiceKind {
    Theme,
    DefaultOpenMode,
}

#[derive(Debug, Clone, Copy)]
struct SettingsChoiceOverlay {
    kind: SettingsChoiceKind,
    selected: usize,
}

#[derive(Debug, Clone)]
enum UserManagementMode {
    Root,
    CreateAuthMethod { username: String },
    DeleteUser,
    ResetPassword,
    ChangeAuthSelectUser,
    ChangeAuthChoose { username: String },
    ToggleAdmin,
}

#[derive(Debug, Clone)]
enum FlashAction {
    ExitApp,
    FinishLogout,
    FinishLogin {
        username: String,
        user: UserRecord,
    },
}

#[derive(Debug, Clone)]
struct TerminalFlash {
    message: String,
    until: Instant,
    action: FlashAction,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TerminalPromptKind {
    Input,
    Password,
    Confirm,
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
enum TerminalPromptAction {
    LoginPassword,
    CreateUsername,
    CreatePassword { username: String },
    CreatePasswordConfirm { username: String, first_password: String },
    ResetPassword { username: String },
    ResetPasswordConfirm { username: String, first_password: String },
    ChangeAuthPassword { username: String },
    ChangeAuthPasswordConfirm { username: String, first_password: String },
    ConfirmDeleteUser { username: String },
    ConfirmToggleAdmin { username: String },
    Noop,
}

#[derive(Debug, Clone)]
struct TerminalPrompt {
    kind: TerminalPromptKind,
    title: String,
    prompt: String,
    buffer: String,
    confirm_yes: bool,
    action: TerminalPromptAction,
}

#[derive(Debug, Clone)]
enum LoginMenuRow {
    User(String),
    Separator,
    Exit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TerminalScreen {
    MainMenu,
    Applications,
    Documents,
    DocumentBrowser,
    Settings,
    UserManagement,
}

#[derive(Debug, Clone, Copy)]
enum MainMenuAction {
    Applications,
    Documents,
    Network,
    Games,
    ProgramInstaller,
    Terminal,
    DesktopMode,
    Settings,
    Logout,
}

#[derive(Debug, Clone, Copy)]
struct MainMenuEntry {
    label: &'static str,
    action: Option<MainMenuAction>,
}

const MAIN_MENU_ENTRIES: &[MainMenuEntry] = &[
    MainMenuEntry {
        label: "Applications",
        action: Some(MainMenuAction::Applications),
    },
    MainMenuEntry {
        label: "Documents",
        action: Some(MainMenuAction::Documents),
    },
    MainMenuEntry {
        label: "Network",
        action: Some(MainMenuAction::Network),
    },
    MainMenuEntry {
        label: "Games",
        action: Some(MainMenuAction::Games),
    },
    MainMenuEntry {
        label: "Program Installer",
        action: Some(MainMenuAction::ProgramInstaller),
    },
    MainMenuEntry {
        label: "Terminal",
        action: Some(MainMenuAction::Terminal),
    },
    MainMenuEntry {
        label: "Desktop Mode",
        action: Some(MainMenuAction::DesktopMode),
    },
    MainMenuEntry {
        label: "---",
        action: None,
    },
    MainMenuEntry {
        label: "Settings",
        action: Some(MainMenuAction::Settings),
    },
    MainMenuEntry {
        label: "Logout",
        action: Some(MainMenuAction::Logout),
    },
];

const DOCUMENT_MENU_ITEMS: &[&str] = &["New Document", "Open Documents", "Back"];
const STATIC_APP_MENU_ITEMS: &[&str] = &["ROBCO Word Processor"];
const TERMINAL_SCREEN_COLS: usize = 92;
const TERMINAL_SCREEN_ROWS: usize = 34;
const TERMINAL_CONTENT_COL: usize = 3;
const TERMINAL_HEADER_START_ROW: usize = 0;
const TERMINAL_SEPARATOR_TOP_ROW: usize = 3;
const TERMINAL_TITLE_ROW: usize = 4;
const TERMINAL_SEPARATOR_BOTTOM_ROW: usize = 5;
const TERMINAL_SUBTITLE_ROW: usize = 7;
const TERMINAL_MENU_START_ROW: usize = 9;
const TERMINAL_STATUS_ROW: usize = 28;
const TERMINAL_STATUS_ROW_ALT: usize = 30;

fn selectable_menu_count() -> usize {
    MAIN_MENU_ENTRIES.iter().filter(|entry| entry.action.is_some()).count()
}

const NATIVE_UI_SCALE_OPTIONS: &[f32] = &[0.85, 1.0, 1.2, 1.4, 1.7, 2.0, 2.3, 2.6];

fn entry_for_selectable_idx(idx: usize) -> MainMenuEntry {
    MAIN_MENU_ENTRIES
        .iter()
        .copied()
        .filter(|entry| entry.action.is_some())
        .nth(idx)
        .unwrap_or(MAIN_MENU_ENTRIES[0])
}

fn retro_footer_height() -> f32 {
    26.0
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
            .insert("retro".into(), FontData::from_owned(bytes).into());
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
    let scale = crate::config::get_settings().native_ui_scale.clamp(0.75, 2.6);
    style.text_styles = [
        (
            TextStyle::Heading,
            FontId::new(28.0 * scale, FontFamily::Monospace),
        ),
        (TextStyle::Body, FontId::new(22.0 * scale, FontFamily::Monospace)),
        (
            TextStyle::Monospace,
            FontId::new(22.0 * scale, FontFamily::Monospace),
        ),
        (TextStyle::Button, FontId::new(22.0 * scale, FontFamily::Monospace)),
        (TextStyle::Small, FontId::new(18.0 * scale, FontFamily::Monospace)),
    ]
    .into();
    ctx.set_style(style);
}

pub struct RobcoNativeApp {
    login: LoginState,
    login_mode: LoginScreenMode,
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
    terminal_settings_idx: usize,
    terminal_browser_idx: usize,
    terminal_user_management_idx: usize,
    terminal_user_management_mode: UserManagementMode,
    terminal_settings_choice: Option<SettingsChoiceOverlay>,
    terminal_prompt: Option<TerminalPrompt>,
    terminal_flash: Option<TerminalFlash>,
    shell_status: String,
}

impl Default for RobcoNativeApp {
    fn default() -> Self {
        Self {
            login: LoginState::default(),
            login_mode: LoginScreenMode::SelectUser,
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
                draft: current_settings(),
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
            terminal_settings_idx: 0,
            terminal_browser_idx: 0,
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
    fn restore_for_user(&mut self, username: &str, user: &UserRecord) {
        crate::config::reload_settings();
        let snapshot: NativeShellSnapshot = read_shell_snapshot(username);
        self.session = Some(SessionState {
            username: username.to_string(),
            is_admin: user.is_admin,
        });
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
        self.terminal_settings_idx = 0;
        self.terminal_browser_idx = 0;
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
        self.login.password.clear();
        self.login.error.clear();
        self.terminal_prompt = None;
        self.queue_terminal_flash(
            "Logging in...",
            700,
            FlashAction::FinishLogin { username, user },
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

    fn login_menu_rows(&self) -> Vec<LoginMenuRow> {
        let mut rows: Vec<LoginMenuRow> = self
            .login_usernames()
            .into_iter()
            .map(LoginMenuRow::User)
            .collect();
        rows.push(LoginMenuRow::Separator);
        rows.push(LoginMenuRow::Exit);
        rows
    }

    fn queue_terminal_flash(&mut self, message: impl Into<String>, ms: u64, action: FlashAction) {
        self.terminal_flash = Some(TerminalFlash {
            message: message.into(),
            until: Instant::now() + Duration::from_millis(ms),
            action,
        });
    }

    fn begin_logout(&mut self) {
        self.persist_snapshot();
        self.terminal_prompt = None;
        self.queue_terminal_flash("Logging out...", 800, FlashAction::FinishLogout);
    }

    fn finish_logout(&mut self) {
        crate::config::reload_settings();
        self.session = None;
        self.login_mode = LoginScreenMode::SelectUser;
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

    fn update_user_record<F: FnOnce(&mut UserRecord)>(&mut self, username: &str, f: F, status: String) {
        let mut db = load_users();
        if let Some(record) = db.get_mut(username) {
            f(record);
            save_users(&db);
            self.shell_status = status;
        } else {
            self.shell_status = format!("Unknown user '{username}'.");
        }
    }

    fn user_management_root_items(&self) -> Vec<String> {
        vec![
            "Create User".to_string(),
            "Delete User".to_string(),
            "Reset Password".to_string(),
            "Change Auth Method".to_string(),
            "Toggle Admin".to_string(),
            "---".to_string(),
            "Back".to_string(),
        ]
    }

    fn auth_method_items(&self) -> Vec<String> {
        vec![
            "Password             — classic password login".to_string(),
            "No Password          — log in without a password".to_string(),
            "Hacking Minigame     — must hack in to log in".to_string(),
            "---".to_string(),
            "Back".to_string(),
        ]
    }

    fn auth_method_from_label(label: &str) -> Option<crate::core::auth::AuthMethod> {
        if label.starts_with("Password") {
            Some(crate::core::auth::AuthMethod::Password)
        } else if label.starts_with("No Password") {
            Some(crate::core::auth::AuthMethod::NoPassword)
        } else if label.starts_with("Hacking") {
            Some(crate::core::auth::AuthMethod::HackingMinigame)
        } else {
            None
        }
    }

    fn user_list_items(&self, include_current: bool) -> Vec<String> {
        let current = self.session.as_ref().map(|s| s.username.as_str());
        let mut users: Vec<String> = load_users()
            .keys()
            .filter(|u| include_current || Some(u.as_str()) != current)
            .cloned()
            .collect();
        users.sort();
        users.push("Back".to_string());
        users
    }

    fn activate_login_selection(&mut self) {
        self.login.error.clear();
        let usernames = self.login_usernames();
        let idx = self.login.selected_idx.min(usernames.len());
        if idx == usernames.len() {
            self.queue_terminal_flash("Exiting...", 800, FlashAction::ExitApp);
            return;
        }
        let Some(selected) = usernames.get(idx).cloned() else {
            return;
        };

        let db = load_users();
        let Some(record) = db.get(&selected).cloned() else {
            self.login.error = "Unknown user.".to_string();
            return;
        };
        self.login.selected_username = selected.clone();
        match record.auth_method {
            crate::core::auth::AuthMethod::NoPassword => match authenticate(&selected, "") {
                Ok(user) => self.queue_login(selected, user),
                Err(err) => self.login.error = err.to_string(),
            },
            crate::core::auth::AuthMethod::Password => {
                self.login.password.clear();
                self.login_mode = LoginScreenMode::SelectUser;
                self.open_password_prompt(
                    "Password Prompt",
                    format!("Password for {}", self.login.selected_username),
                );
            }
            crate::core::auth::AuthMethod::HackingMinigame => {
                self.login.error =
                    "Hacking login is not implemented in the native rewrite yet.".to_string();
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

    fn handle_main_menu_action(&mut self, action: MainMenuAction) {
        match action {
            MainMenuAction::Applications => {
                self.terminal_screen = TerminalScreen::Applications;
                self.terminal_apps_idx = 0;
                self.shell_status.clear();
            }
            MainMenuAction::Documents => {
                self.terminal_screen = TerminalScreen::Documents;
                self.terminal_documents_idx = 0;
                self.shell_status.clear();
            }
            MainMenuAction::Network => {
                self.shell_status = "Network menu is not rewritten yet.".to_string();
            }
            MainMenuAction::Games => {
                self.shell_status = "Games menu is not rewritten yet.".to_string();
            }
            MainMenuAction::ProgramInstaller => {
                self.shell_status = "Program Installer is not rewritten yet.".to_string();
            }
            MainMenuAction::Terminal => {
                self.terminal_mode.open = true;
                self.shell_status.clear();
            }
            MainMenuAction::DesktopMode => {
                self.desktop_mode_open = true;
                self.shell_status = "Entered Desktop Mode.".to_string();
            }
            MainMenuAction::Settings => {
                self.settings.draft = current_settings();
                self.terminal_screen = TerminalScreen::Settings;
                self.terminal_settings_idx = 0;
                self.shell_status.clear();
            }
            MainMenuAction::Logout => self.begin_logout(),
        }
    }

    fn terminal_app_items(&self) -> Vec<String> {
        let mut items: Vec<String> = STATIC_APP_MENU_ITEMS.iter().map(|s| (*s).to_string()).collect();
        items.extend(app_names());
        items.push("Back".to_string());
        items
    }

    fn persist_native_settings(&mut self) {
        save_settings(self.settings.draft.clone());
        crate::config::reload_settings();
        self.settings.draft = current_settings();
        self.shell_status = "Settings saved.".to_string();
    }

    fn open_settings_choice(&mut self, kind: SettingsChoiceKind) {
        let selected = match kind {
            SettingsChoiceKind::Theme => THEMES
                .iter()
                .position(|(name, _)| *name == self.settings.draft.theme)
                .unwrap_or(0),
            SettingsChoiceKind::DefaultOpenMode => match self.settings.draft.default_open_mode {
                OpenMode::Terminal => 0,
                OpenMode::Desktop => 1,
            },
        };
        self.terminal_settings_choice = Some(SettingsChoiceOverlay { kind, selected });
    }

    fn settings_choice_items(&self, kind: SettingsChoiceKind) -> Vec<String> {
        match kind {
            SettingsChoiceKind::Theme => THEMES.iter().map(|(name, _)| (*name).to_string()).collect(),
            SettingsChoiceKind::DefaultOpenMode => vec!["Terminal".to_string(), "Desktop".to_string()],
        }
    }

    fn apply_settings_choice(&mut self, kind: SettingsChoiceKind, selected: usize) {
        match kind {
            SettingsChoiceKind::Theme => {
                if let Some((name, _)) = THEMES.get(selected) {
                    self.settings.draft.theme = (*name).to_string();
                }
            }
            SettingsChoiceKind::DefaultOpenMode => {
                self.settings.draft.default_open_mode = if selected == 0 {
                    OpenMode::Terminal
                } else {
                    OpenMode::Desktop
                };
            }
        }
        self.persist_native_settings();
    }

    fn terminal_settings_rows(&self) -> Vec<String> {
        let mut rows = vec![
            format!(
                "Sound: {} [toggle]",
                if self.settings.draft.sound { "ON" } else { "OFF" }
            ),
            format!(
                "Bootup: {} [toggle]",
                if self.settings.draft.bootup { "ON" } else { "OFF" }
            ),
            format!(
                "Navigation Hints: {} [toggle]",
                if self.settings.draft.show_navigation_hints {
                    "ON"
                } else {
                    "OFF"
                }
            ),
            format!("Theme: {} [choose]", self.settings.draft.theme),
            format!(
                "Interface Size: {}% [adjust]",
                (self.settings.draft.native_ui_scale * 100.0).round() as i32
            ),
            format!(
                "Default Open Mode: {} [choose]",
                match self.settings.draft.default_open_mode {
                    OpenMode::Terminal => "Terminal",
                    OpenMode::Desktop => "Desktop",
                }
            ),
        ];
        if self.session.as_ref().is_some_and(|s| s.is_admin) {
            rows.push("User Management".to_string());
        }
        rows.push("Back".to_string());
        rows
    }

    fn open_documents_browser(&mut self) {
        if let Some(session) = &self.session {
            self.file_manager.set_cwd(word_processor_dir(&session.username));
            self.file_manager.selected = None;
            self.terminal_browser_idx = 0;
            self.terminal_screen = TerminalScreen::DocumentBrowser;
        }
    }

    fn document_browser_rows(&self) -> Vec<(String, Option<PathBuf>)> {
        let mut rows = Vec::new();
        rows.push(("../".to_string(), None));
        for row in self.file_manager.rows() {
            let label = if row.is_dir {
                format!("[DIR] {}", row.label)
            } else {
                row.label.clone()
            };
            rows.push((label, Some(row.path.clone())));
        }
        if rows.is_empty() {
            rows.push(("(empty)".to_string(), None));
        }
        rows
    }

    fn activate_document_browser(&mut self) {
        let rows = self.document_browser_rows();
        let idx = self.terminal_browser_idx.min(rows.len().saturating_sub(1));
        if idx == 0 {
            self.file_manager.up();
            self.terminal_browser_idx = 0;
            return;
        }
        if let Some((_, Some(path))) = rows.get(idx) {
            self.file_manager.select(Some(path.clone()));
            self.activate_file_manager_selection();
        }
    }

    fn handle_terminal_back(&mut self) {
        if self.terminal_settings_choice.is_some() {
            self.terminal_settings_choice = None;
            return;
        }
        match self.terminal_screen {
            TerminalScreen::MainMenu => {}
            TerminalScreen::Applications
            | TerminalScreen::Documents
            | TerminalScreen::Settings
            | TerminalScreen::UserManagement => {
                self.terminal_screen = TerminalScreen::MainMenu;
                self.shell_status.clear();
            }
            TerminalScreen::DocumentBrowser => {
                self.terminal_screen = TerminalScreen::Documents;
                self.shell_status.clear();
            }
        }
    }

    fn step_interface_size(&mut self, delta: isize) {
        let current_idx = NATIVE_UI_SCALE_OPTIONS
            .iter()
            .position(|v| (*v - self.settings.draft.native_ui_scale).abs() < 0.001)
            .unwrap_or(1);
        let next_idx = if delta < 0 {
            current_idx.saturating_sub(delta.unsigned_abs())
        } else {
            (current_idx + delta as usize).min(NATIVE_UI_SCALE_OPTIONS.len().saturating_sub(1))
        };
        if next_idx != current_idx {
            self.settings.draft.native_ui_scale = NATIVE_UI_SCALE_OPTIONS[next_idx];
            self.persist_native_settings();
        }
    }

    fn interface_size_slider_text(&self, width: usize) -> String {
        let width = width.max(4);
        let idx = NATIVE_UI_SCALE_OPTIONS
            .iter()
            .position(|v| (*v - self.settings.draft.native_ui_scale).abs() < 0.001)
            .unwrap_or(1);
        let max = NATIVE_UI_SCALE_OPTIONS.len().saturating_sub(1).max(1);
        let fill = ((idx * (width - 1)) + (max / 2)) / max;
        let mut chars = vec!['-'; width];
        for ch in chars.iter_mut().take(fill) {
            *ch = '=';
        }
        chars[fill.min(width - 1)] = '|';
        format!("[{}]", chars.into_iter().collect::<String>())
    }

    fn handle_terminal_prompt_input(&mut self, ctx: &Context) {
        let Some(mut prompt) = self.terminal_prompt.clone() else {
            return;
        };
        match prompt.kind {
            TerminalPromptKind::Input | TerminalPromptKind::Password => {
                if ctx.input(|i| i.key_pressed(Key::Escape) || i.key_pressed(Key::Tab)) {
                    self.terminal_prompt = None;
                    self.login.password.clear();
                    self.login.error.clear();
                    return;
                }
                if ctx.input(|i| i.key_pressed(Key::Backspace)) {
                    prompt.buffer.pop();
                }
                let events = ctx.input(|i| i.events.clone());
                for event in events {
                    if let egui::Event::Text(text) = event {
                        for ch in text.chars() {
                            if !ch.is_control() {
                                prompt.buffer.push(ch);
                            }
                        }
                    }
                }
                if ctx.input(|i| i.key_pressed(Key::Enter)) {
                    match prompt.action {
                        TerminalPromptAction::LoginPassword => {
                            self.login.password = prompt.buffer.clone();
                            self.do_login();
                            if self.session.is_none() && self.terminal_flash.is_none() {
                                prompt.buffer.clear();
                                self.terminal_prompt = Some(prompt);
                            }
                            return;
                        }
                        TerminalPromptAction::CreateUsername => {
                            let username = prompt.buffer.trim().to_string();
                            if username.is_empty() {
                                self.shell_status = "Username cannot be empty.".to_string();
                                self.terminal_prompt = None;
                                return;
                            }
                            let db = load_users();
                            if db.contains_key(&username) {
                                self.shell_status = "User already exists.".to_string();
                                self.terminal_prompt = None;
                                return;
                            }
                            self.terminal_user_management_mode =
                                UserManagementMode::CreateAuthMethod { username };
                            self.terminal_user_management_idx = 0;
                            self.terminal_prompt = None;
                            return;
                        }
                        TerminalPromptAction::CreatePassword { username } => {
                            let first_password = prompt.buffer.clone();
                            if first_password.is_empty() {
                                self.shell_status = "Password cannot be empty.".to_string();
                                self.terminal_prompt = None;
                                return;
                            }
                            self.open_password_prompt_with_action(
                                "Confirm Password",
                                format!("Re-enter password for {username}"),
                                TerminalPromptAction::CreatePasswordConfirm {
                                    username,
                                    first_password,
                                },
                            );
                            return;
                        }
                        TerminalPromptAction::CreatePasswordConfirm {
                            username,
                            first_password,
                        } => {
                            if prompt.buffer != first_password {
                                self.shell_status = "Passwords do not match.".to_string();
                                self.terminal_prompt = None;
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
                            self.terminal_prompt = None;
                            return;
                        }
                        TerminalPromptAction::ResetPassword { username } => {
                            let first_password = prompt.buffer.clone();
                            if first_password.is_empty() {
                                self.shell_status = "Password cannot be empty.".to_string();
                                self.terminal_prompt = None;
                                return;
                            }
                            self.open_password_prompt_with_action(
                                "Confirm Password",
                                format!("Re-enter password for {username}"),
                                TerminalPromptAction::ResetPasswordConfirm {
                                    username,
                                    first_password,
                                },
                            );
                            return;
                        }
                        TerminalPromptAction::ResetPasswordConfirm {
                            username,
                            first_password,
                        } => {
                            if prompt.buffer != first_password {
                                self.shell_status = "Passwords do not match.".to_string();
                                self.terminal_prompt = None;
                                return;
                            }
                            self.update_user_record(
                                &username,
                                |record| {
                                    record.password_hash =
                                        crate::core::auth::hash_password(&first_password);
                                    record.auth_method = crate::core::auth::AuthMethod::Password;
                                },
                                "Password updated.".to_string(),
                            );
                            self.terminal_user_management_mode = UserManagementMode::Root;
                            self.terminal_user_management_idx = 0;
                            self.terminal_prompt = None;
                            return;
                        }
                        TerminalPromptAction::ChangeAuthPassword { username } => {
                            let first_password = prompt.buffer.clone();
                            if first_password.is_empty() {
                                self.shell_status = "Password cannot be empty.".to_string();
                                self.terminal_prompt = None;
                                return;
                            }
                            self.open_password_prompt_with_action(
                                "Confirm Password",
                                format!("Re-enter password for {username}"),
                                TerminalPromptAction::ChangeAuthPasswordConfirm {
                                    username,
                                    first_password,
                                },
                            );
                            return;
                        }
                        TerminalPromptAction::ChangeAuthPasswordConfirm {
                            username,
                            first_password,
                        } => {
                            if prompt.buffer != first_password {
                                self.shell_status = "Passwords do not match.".to_string();
                                self.terminal_prompt = None;
                                return;
                            }
                            self.update_user_record(
                                &username,
                                |record| {
                                    record.password_hash =
                                        crate::core::auth::hash_password(&first_password);
                                    record.auth_method = crate::core::auth::AuthMethod::Password;
                                },
                                format!("Auth method updated for '{username}'."),
                            );
                            self.terminal_user_management_mode = UserManagementMode::Root;
                            self.terminal_user_management_idx = 0;
                            self.terminal_prompt = None;
                            return;
                        }
                        TerminalPromptAction::Noop => {
                            self.terminal_prompt = None;
                            return;
                        }
                        TerminalPromptAction::ConfirmDeleteUser { .. }
                        | TerminalPromptAction::ConfirmToggleAdmin { .. } => {
                            self.terminal_prompt = None;
                            return;
                        }
                    }
                }
            }
            TerminalPromptKind::Confirm => {
                if ctx.input(|i| i.key_pressed(Key::Escape) || i.key_pressed(Key::Tab)) {
                    self.terminal_prompt = None;
                    return;
                }
                if ctx.input(|i| i.key_pressed(Key::ArrowLeft)) {
                    prompt.confirm_yes = true;
                }
                if ctx.input(|i| i.key_pressed(Key::ArrowRight)) {
                    prompt.confirm_yes = false;
                }
                if ctx.input(|i| i.key_pressed(Key::Enter)) {
                    if prompt.confirm_yes {
                        match prompt.action {
                            TerminalPromptAction::ConfirmDeleteUser { username } => {
                                let mut db = load_users();
                                db.remove(&username);
                                save_users(&db);
                                self.shell_status = format!("User '{username}' deleted.");
                            }
                            TerminalPromptAction::ConfirmToggleAdmin { username } => {
                                let mut db = load_users();
                                if let Some(record) = db.get_mut(&username) {
                                    record.is_admin = !record.is_admin;
                                    let label = if record.is_admin { "granted" } else { "revoked" };
                                    save_users(&db);
                                    self.shell_status =
                                        format!("Admin {label} for '{username}'.");
                                }
                            }
                            _ => {}
                        }
                    }
                    self.terminal_user_management_mode = UserManagementMode::Root;
                    self.terminal_user_management_idx = 0;
                    self.terminal_prompt = None;
                    return;
                }
            }
        }
        self.terminal_prompt = Some(prompt);
    }

    fn draw_terminal_prompt_overlay(&self, ui: &mut egui::Ui, screen: &RetroScreen) {
        let Some(prompt) = &self.terminal_prompt else {
            return;
        };
        let palette = current_palette();
        let painter = ui.painter_at(screen.rect);
        screen.boxed_panel(&painter, &palette, 23, 12, 46, 8);
        screen.text(&painter, 26, 13, &prompt.title, palette.fg);
        screen.text(&painter, 26, 15, &prompt.prompt, palette.fg);
        match prompt.kind {
            TerminalPromptKind::Input => {
                let line = format!("> {}_", prompt.buffer);
                screen.text(&painter, 26, 17, &line, palette.fg);
                screen.text(&painter, 26, 19, "Enter apply | Esc/Tab cancel", palette.dim);
            }
            TerminalPromptKind::Password => {
                let masked = format!("> {}_", "*".repeat(prompt.buffer.chars().count()));
                screen.text(&painter, 26, 17, &masked, palette.fg);
                screen.text(
                    &painter,
                    26,
                    19,
                    "Enter log in | Esc/Tab back | Backspace delete",
                    palette.dim,
                );
            }
            TerminalPromptKind::Confirm => {
                let yes = if prompt.confirm_yes { "[Yes]" } else { " Yes " };
                let no = if prompt.confirm_yes { " No " } else { "[No]" };
                screen.text(&painter, 26, 17, &format!("{yes}   {no}"), palette.fg);
                screen.text(&painter, 26, 19, "Left/Right choose | Enter apply", palette.dim);
            }
        }
    }

    fn draw_login(&mut self, ctx: &Context) {
        let rows = self.login_menu_rows();
        if self.terminal_prompt.is_some() {
            self.handle_terminal_prompt_input(ctx);
        } else if self.login_mode == LoginScreenMode::SelectUser {
            if ctx.input(|i| i.key_pressed(Key::ArrowUp)) {
                self.login.selected_idx = self.login.selected_idx.saturating_sub(1);
            }
            if ctx.input(|i| i.key_pressed(Key::ArrowDown)) {
                self.login.selected_idx =
                    (self.login.selected_idx + 1).min(rows.len().saturating_sub(2));
            }
            if ctx.input(|i| i.key_pressed(Key::Enter)) {
                self.activate_login_selection();
            }
        }

        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(current_palette().bg).inner_margin(0.0))
            .show(ctx, |ui| {
            let palette = current_palette();
            let (screen, _) = RetroScreen::new(ui, TERMINAL_SCREEN_COLS, TERMINAL_SCREEN_ROWS);
            let painter = ui.painter_at(screen.rect);
            screen.paint_bg(&painter, palette.bg);
            for (idx, line) in HEADER_LINES.iter().enumerate() {
                screen.centered_text(&painter, TERMINAL_HEADER_START_ROW + idx, line, palette.fg, true);
            }
            screen.separator(&painter, TERMINAL_SEPARATOR_TOP_ROW, &palette);
            screen.centered_text(&painter, TERMINAL_TITLE_ROW, "ROBCO TERMLINK - Select User", palette.fg, true);
            screen.separator(&painter, TERMINAL_SEPARATOR_BOTTOM_ROW, &palette);
            screen.text(&painter, TERMINAL_CONTENT_COL, TERMINAL_SUBTITLE_ROW, "Welcome. Please select a user.", palette.fg);
            if !self.login.error.is_empty() {
                screen.text(&painter, TERMINAL_CONTENT_COL, TERMINAL_STATUS_ROW, &self.login.error, Color32::LIGHT_RED);
            }

            let mut row = TERMINAL_MENU_START_ROW;
            let mut selectable_idx = 0usize;
            for entry in &rows {
                match entry {
                    LoginMenuRow::Separator => {
                        screen.text(
                            &painter,
                            TERMINAL_CONTENT_COL + 4,
                            row,
                            "---",
                            palette.dim,
                        );
                    }
                    LoginMenuRow::User(user) => {
                        let selected = self.login_mode == LoginScreenMode::SelectUser
                            && selectable_idx == self.login.selected_idx;
                        let text = if selected {
                            format!("  > {user}")
                        } else {
                            format!("    {user}")
                        };
                        let response = screen.selectable_row(
                            ui,
                            &painter,
                            &palette,
                            TERMINAL_CONTENT_COL,
                            row,
                            &text,
                            selected,
                        );
                        if response.clicked() {
                            self.login.selected_idx = selectable_idx;
                            self.activate_login_selection();
                        }
                        selectable_idx += 1;
                    }
                    LoginMenuRow::Exit => {
                        let selected = self.login_mode == LoginScreenMode::SelectUser
                            && selectable_idx == self.login.selected_idx;
                        let text = if selected {
                            "  > Exit".to_string()
                        } else {
                            "    Exit".to_string()
                        };
                        let response = screen.selectable_row(
                            ui,
                            &painter,
                            &palette,
                            TERMINAL_CONTENT_COL,
                            row,
                            &text,
                            selected,
                        );
                        if response.clicked() {
                            self.login.selected_idx = selectable_idx;
                            self.activate_login_selection();
                        }
                        selectable_idx += 1;
                    }
                }
                row += 1;
            }

            self.draw_terminal_prompt_overlay(ui, &screen);
        });
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
        if ctx.input(|i| i.key_pressed(Key::ArrowUp)) {
            self.main_menu_idx = self.main_menu_idx.saturating_sub(1);
        }
        if ctx.input(|i| i.key_pressed(Key::ArrowDown)) {
            self.main_menu_idx = (self.main_menu_idx + 1).min(selectable_menu_count() - 1);
        }
        if ctx.input(|i| i.key_pressed(Key::Enter)) {
            if let Some(action) = entry_for_selectable_idx(self.main_menu_idx).action {
                self.handle_main_menu_action(action);
            }
        }

        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(current_palette().bg).inner_margin(0.0))
            .show(ctx, |ui| {
            let palette = current_palette();
            let (screen, _) = RetroScreen::new(ui, TERMINAL_SCREEN_COLS, TERMINAL_SCREEN_ROWS);
            let painter = ui.painter_at(screen.rect);
            screen.paint_bg(&painter, palette.bg);
            for (idx, line) in HEADER_LINES.iter().enumerate() {
                screen.centered_text(&painter, TERMINAL_HEADER_START_ROW + idx, line, palette.fg, true);
            }
            screen.separator(&painter, TERMINAL_SEPARATOR_TOP_ROW, &palette);
            screen.centered_text(&painter, TERMINAL_TITLE_ROW, "Main Menu", palette.fg, true);
            screen.separator(&painter, TERMINAL_SEPARATOR_BOTTOM_ROW, &palette);
            screen.underlined_text(
                &painter,
                TERMINAL_CONTENT_COL,
                TERMINAL_SUBTITLE_ROW,
                &format!("RobcOS v{}", env!("CARGO_PKG_VERSION")),
                palette.fg,
            );

            let mut visible_row = TERMINAL_MENU_START_ROW;
            let mut selectable_idx = 0usize;
            for entry in MAIN_MENU_ENTRIES {
                if entry.action.is_none() {
                    screen.text(
                        &painter,
                        TERMINAL_CONTENT_COL + 4,
                        visible_row,
                        entry.label,
                        palette.dim,
                    );
                    visible_row += 1;
                    continue;
                }
                let selected = selectable_idx == self.main_menu_idx;
                let text = if selected {
                    format!("  > {}", entry.label)
                } else {
                    format!("    {}", entry.label)
                };
                let response =
                    screen.selectable_row(
                        ui,
                        &painter,
                        &palette,
                        TERMINAL_CONTENT_COL,
                        visible_row,
                        &text,
                        selected,
                    );
                if response.clicked() {
                    self.main_menu_idx = selectable_idx;
                    if let Some(action) = entry.action {
                        self.handle_main_menu_action(action);
                    }
                }
                visible_row += 1;
                selectable_idx += 1;
            }

            if !self.shell_status.is_empty() {
                screen.text(&painter, TERMINAL_CONTENT_COL, TERMINAL_STATUS_ROW, &self.shell_status, palette.dim);
            }
        });
    }

    fn draw_terminal_menu_screen(
        &mut self,
        ctx: &Context,
        title: &str,
        subtitle: Option<&str>,
        items: &[String],
        selected_idx: &mut usize,
    ) -> Option<usize> {
        let selectable_rows: Vec<usize> = items
            .iter()
            .enumerate()
            .filter_map(|(idx, item)| if item == "---" { None } else { Some(idx) })
            .collect();
        if selectable_rows.is_empty() {
            return None;
        }
        *selected_idx = (*selected_idx).min(selectable_rows.len().saturating_sub(1));
        if ctx.input(|i| i.key_pressed(Key::ArrowUp)) {
            *selected_idx = selected_idx.saturating_sub(1);
        }
        if ctx.input(|i| i.key_pressed(Key::ArrowDown)) {
            *selected_idx = (*selected_idx + 1).min(selectable_rows.len().saturating_sub(1));
        }

        let mut activated = None;
        if ctx.input(|i| i.key_pressed(Key::Enter)) {
            activated = selectable_rows.get(*selected_idx).copied();
        }

        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(current_palette().bg).inner_margin(0.0))
            .show(ctx, |ui| {
            let palette = current_palette();
            let (screen, _) = RetroScreen::new(ui, TERMINAL_SCREEN_COLS, TERMINAL_SCREEN_ROWS);
            let painter = ui.painter_at(screen.rect);
            screen.paint_bg(&painter, palette.bg);
            for (idx, line) in HEADER_LINES.iter().enumerate() {
                screen.centered_text(&painter, TERMINAL_HEADER_START_ROW + idx, line, palette.fg, true);
            }
            screen.separator(&painter, TERMINAL_SEPARATOR_TOP_ROW, &palette);
            screen.centered_text(&painter, TERMINAL_TITLE_ROW, title, palette.fg, true);
            screen.separator(&painter, TERMINAL_SEPARATOR_BOTTOM_ROW, &palette);
            if let Some(sub) = subtitle {
                screen.underlined_text(&painter, TERMINAL_CONTENT_COL, TERMINAL_SUBTITLE_ROW, sub, palette.fg);
            }
            let mut row = TERMINAL_MENU_START_ROW;
            for (idx, item) in items.iter().enumerate() {
                if item == "---" {
                    screen.text(
                        &painter,
                        TERMINAL_CONTENT_COL + 4,
                        row,
                        "---",
                        palette.dim,
                    );
                    row += 1;
                    continue;
                }
                let selected = selectable_rows.get(*selected_idx).copied() == Some(idx);
                let text = if selected {
                    format!("  > {item}")
                } else {
                    format!("    {item}")
                };
                let response = screen.selectable_row(ui, &painter, &palette, TERMINAL_CONTENT_COL, row, &text, selected);
                if response.clicked() {
                    if let Some(sel_idx) = selectable_rows.iter().position(|raw| *raw == idx) {
                        *selected_idx = sel_idx;
                    }
                    activated = Some(idx);
                }
                row += 1;
            }
            if !self.shell_status.is_empty() {
                screen.text(&painter, TERMINAL_CONTENT_COL, TERMINAL_STATUS_ROW, &self.shell_status, palette.dim);
            }
        });

        activated
    }

    fn draw_terminal_applications(&mut self, ctx: &Context) {
        let items = self.terminal_app_items();
        let mut selected = self.terminal_apps_idx.min(items.len().saturating_sub(1));
        let activated =
            self.draw_terminal_menu_screen(ctx, "Applications", Some("Built-in and configured apps"), &items, &mut selected);
        self.terminal_apps_idx = selected;
        if let Some(idx) = activated {
            let label = &items[idx];
            if label == "ROBCO Word Processor" {
                self.editor.open = true;
                if self.editor.path.is_none() {
                    self.new_document();
                }
                self.shell_status = "Opened ROBCO Word Processor.".to_string();
            } else if label == "Back" {
                self.terminal_screen = TerminalScreen::MainMenu;
                self.shell_status.clear();
            } else {
                self.shell_status = format!("External app launch for '{label}' is pending rewrite.");
            }
        }
    }

    fn draw_terminal_documents(&mut self, ctx: &Context) {
        let items: Vec<String> = DOCUMENT_MENU_ITEMS.iter().map(|s| (*s).to_string()).collect();
        let mut selected = self.terminal_documents_idx.min(items.len().saturating_sub(1));
        let activated =
            self.draw_terminal_menu_screen(ctx, "Documents", Some("ROBCO Word Processor"), &items, &mut selected);
        self.terminal_documents_idx = selected;
        if let Some(idx) = activated {
            match items[idx].as_str() {
                "New Document" => {
                    self.new_document();
                    self.shell_status = "New document created.".to_string();
                }
                "Open Documents" => self.open_documents_browser(),
                "Back" => {
                    self.terminal_screen = TerminalScreen::MainMenu;
                    self.shell_status.clear();
                }
                _ => {}
            }
        }
    }

    fn draw_terminal_document_browser(&mut self, ctx: &Context) {
        let rows = self.document_browser_rows();
        if ctx.input(|i| i.key_pressed(Key::ArrowUp)) {
            self.terminal_browser_idx = self.terminal_browser_idx.saturating_sub(1);
        }
        if ctx.input(|i| i.key_pressed(Key::ArrowDown)) {
            self.terminal_browser_idx =
                (self.terminal_browser_idx + 1).min(rows.len().saturating_sub(1));
        }
        if ctx.input(|i| i.key_pressed(Key::Enter)) {
            self.activate_document_browser();
        }

        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(current_palette().bg).inner_margin(0.0))
            .show(ctx, |ui| {
            let palette = current_palette();
            let (screen, _) = RetroScreen::new(ui, TERMINAL_SCREEN_COLS, TERMINAL_SCREEN_ROWS);
            let painter = ui.painter_at(screen.rect);
            screen.paint_bg(&painter, palette.bg);
            for (idx, line) in HEADER_LINES.iter().enumerate() {
                screen.centered_text(&painter, TERMINAL_HEADER_START_ROW + idx, line, palette.fg, true);
            }
            screen.separator(&painter, TERMINAL_SEPARATOR_TOP_ROW, &palette);
            screen.centered_text(&painter, TERMINAL_TITLE_ROW, "Open Documents", palette.fg, true);
            screen.separator(&painter, TERMINAL_SEPARATOR_BOTTOM_ROW, &palette);
            screen.underlined_text(
                &painter,
                TERMINAL_CONTENT_COL,
                TERMINAL_SUBTITLE_ROW,
                &self.file_manager.cwd.display().to_string(),
                palette.fg,
            );
            let mut row = TERMINAL_MENU_START_ROW;
            for (idx, (label, path)) in rows.iter().enumerate() {
                let selected = idx == self.terminal_browser_idx;
                let text = if selected {
                    format!("  > {label}")
                } else {
                    format!("    {label}")
                };
                let response = screen.selectable_row(ui, &painter, &palette, TERMINAL_CONTENT_COL, row, &text, selected);
                if response.clicked() {
                    self.terminal_browser_idx = idx;
                    if idx == 0 {
                        self.file_manager.up();
                        self.terminal_browser_idx = 0;
                    } else if let Some(path) = path {
                        self.file_manager.select(Some(path.clone()));
                        self.activate_file_manager_selection();
                    }
                }
                row += 1;
            }
            let hint = "Enter open | Tab back | Up/Down move";
            screen.text(&painter, TERMINAL_CONTENT_COL, TERMINAL_STATUS_ROW, hint, palette.dim);
            if !self.shell_status.is_empty() {
                screen.text(&painter, TERMINAL_CONTENT_COL, TERMINAL_STATUS_ROW_ALT, &self.shell_status, palette.dim);
            }
        });
    }

    fn draw_terminal_settings(&mut self, ctx: &Context) {
        let items = self.terminal_settings_rows();
        self.terminal_settings_idx = self
            .terminal_settings_idx
            .min(items.len().saturating_sub(1));

        if let Some(mut overlay) = self.terminal_settings_choice {
            let choice_items = self.settings_choice_items(overlay.kind);
            if ctx.input(|i| i.key_pressed(Key::ArrowUp)) {
                overlay.selected = overlay.selected.saturating_sub(1);
            }
            if ctx.input(|i| i.key_pressed(Key::ArrowDown)) {
                overlay.selected = (overlay.selected + 1).min(choice_items.len().saturating_sub(1));
            }
            if ctx.input(|i| i.key_pressed(Key::Escape) || i.key_pressed(Key::Tab)) {
                self.terminal_settings_choice = None;
            } else if ctx.input(|i| i.key_pressed(Key::Enter)) {
                self.apply_settings_choice(overlay.kind, overlay.selected);
                self.terminal_settings_choice = None;
            } else {
                self.terminal_settings_choice = Some(overlay);
            }
        } else {
            if ctx.input(|i| i.key_pressed(Key::ArrowUp)) {
                self.terminal_settings_idx = self.terminal_settings_idx.saturating_sub(1);
            }
            if ctx.input(|i| i.key_pressed(Key::ArrowDown)) {
                self.terminal_settings_idx =
                    (self.terminal_settings_idx + 1).min(items.len().saturating_sub(1));
            }
            if self.terminal_settings_idx == 4 {
                if ctx.input(|i| i.key_pressed(Key::ArrowLeft)) {
                    self.step_interface_size(-1);
                }
                if ctx.input(|i| i.key_pressed(Key::ArrowRight)) {
                    self.step_interface_size(1);
                }
            }
            if ctx.input(|i| i.key_pressed(Key::Enter)) {
                match self.terminal_settings_idx {
                    0 => {
                        self.settings.draft.sound = !self.settings.draft.sound;
                        self.persist_native_settings();
                    }
                    1 => {
                        self.settings.draft.bootup = !self.settings.draft.bootup;
                        self.persist_native_settings();
                    }
                    2 => {
                        self.settings.draft.show_navigation_hints =
                            !self.settings.draft.show_navigation_hints;
                        self.persist_native_settings();
                    }
                    3 => self.open_settings_choice(SettingsChoiceKind::Theme),
                    4 => {}
                    5 => self.open_settings_choice(SettingsChoiceKind::DefaultOpenMode),
                    6 => {
                        self.terminal_screen = TerminalScreen::MainMenu;
                        self.shell_status.clear();
                    }
                    _ => {}
                }
            }
        }

        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(current_palette().bg).inner_margin(0.0))
            .show(ctx, |ui| {
                let palette = current_palette();
                let (screen, _) = RetroScreen::new(ui, TERMINAL_SCREEN_COLS, TERMINAL_SCREEN_ROWS);
                let painter = ui.painter_at(screen.rect);
                screen.paint_bg(&painter, palette.bg);
                for (idx, line) in HEADER_LINES.iter().enumerate() {
                    screen.centered_text(
                        &painter,
                        TERMINAL_HEADER_START_ROW + idx,
                        line,
                        palette.fg,
                        true,
                    );
                }
                screen.separator(&painter, TERMINAL_SEPARATOR_TOP_ROW, &palette);
                screen.centered_text(&painter, TERMINAL_TITLE_ROW, "Settings", palette.fg, true);
                screen.separator(&painter, TERMINAL_SEPARATOR_BOTTOM_ROW, &palette);
                screen.underlined_text(
                    &painter,
                    TERMINAL_CONTENT_COL,
                    TERMINAL_SUBTITLE_ROW,
                    "Native terminal-style settings",
                    palette.fg,
                );

                let choice_items = self
                    .terminal_settings_choice
                    .map(|overlay| self.settings_choice_items(overlay.kind));
                let mut row = TERMINAL_MENU_START_ROW;
                for (idx, item) in items.iter().enumerate() {
                    let selected = idx == self.terminal_settings_idx;
                    let text = if selected {
                        format!("  > {item}")
                    } else {
                        format!("    {item}")
                    };
                    let response = screen.selectable_row(
                        ui,
                        &painter,
                        &palette,
                        TERMINAL_CONTENT_COL,
                        row,
                        &text,
                        selected,
                    );
                    if response.clicked() {
                        self.terminal_settings_idx = idx;
                        if self.terminal_settings_choice.is_some() {
                            self.terminal_settings_choice = None;
                        } else {
                            match idx {
                                0 => {
                                    self.settings.draft.sound = !self.settings.draft.sound;
                                    self.persist_native_settings();
                                }
                                1 => {
                                    self.settings.draft.bootup = !self.settings.draft.bootup;
                                    self.persist_native_settings();
                                }
                                2 => {
                                    self.settings.draft.show_navigation_hints =
                                        !self.settings.draft.show_navigation_hints;
                                    self.persist_native_settings();
                                }
                                3 => self.open_settings_choice(SettingsChoiceKind::Theme),
                                4 => {}
                                5 => self.open_settings_choice(SettingsChoiceKind::DefaultOpenMode),
                                6 if self.session.as_ref().is_some_and(|s| s.is_admin) => {
                                    self.terminal_screen = TerminalScreen::UserManagement;
                                    self.terminal_user_management_mode = UserManagementMode::Root;
                                    self.terminal_user_management_idx = 0;
                                    self.shell_status.clear();
                                }
                                _ if item == "Back" => {
                                    self.terminal_screen = TerminalScreen::MainMenu;
                                    self.shell_status.clear();
                                }
                                _ => {}
                            }
                        }
                    }
                    row += 1;

                    if selected {
                        if let (Some(overlay), Some(choice_items)) =
                            (self.terminal_settings_choice, choice_items.as_ref())
                        {
                            for (choice_idx, choice) in choice_items.iter().enumerate() {
                                let choice_selected = choice_idx == overlay.selected;
                                let choice_text = if choice_selected {
                                    format!("      > {choice}")
                                } else {
                                    format!("        {choice}")
                                };
                                let response = screen.selectable_row(
                                    ui,
                                    &painter,
                                    &palette,
                                    TERMINAL_CONTENT_COL,
                                    row,
                                    &choice_text,
                                    choice_selected,
                                );
                                if response.clicked() {
                                    self.terminal_settings_choice = None;
                                    self.apply_settings_choice(overlay.kind, choice_idx);
                                    return;
                                }
                                row += 1;
                            }
                            screen.text(
                                &painter,
                                TERMINAL_CONTENT_COL + 4,
                                row,
                                "Enter apply | Esc/Tab close",
                                palette.dim,
                            );
                            row += 1;
                        }
                        if idx == 4 {
                            let slider = format!(
                                "        {}  Left/Right adjust",
                                self.interface_size_slider_text(18)
                            );
                            screen.text(
                                &painter,
                                TERMINAL_CONTENT_COL,
                                row,
                                &slider,
                                palette.dim,
                            );
                            row += 1;
                        }
                    }
                }

                if !self.shell_status.is_empty() {
                    screen.text(
                        &painter,
                        TERMINAL_CONTENT_COL,
                        TERMINAL_STATUS_ROW,
                        &self.shell_status,
                        palette.dim,
                    );
                }
            });
    }

    fn draw_terminal_user_management(&mut self, ctx: &Context) {
        let mode = self.terminal_user_management_mode.clone();
        let (title, subtitle, items) = match &mode {
            UserManagementMode::Root => (
                "User Management",
                None,
                self.user_management_root_items(),
            ),
            UserManagementMode::CreateAuthMethod { username } => (
                "Choose Authentication Method",
                Some(format!("Create user '{username}'")),
                self.auth_method_items(),
            ),
            UserManagementMode::DeleteUser => (
                "Delete User",
                None,
                self.user_list_items(true),
            ),
            UserManagementMode::ResetPassword => (
                "Reset Password",
                None,
                self.user_list_items(true),
            ),
            UserManagementMode::ChangeAuthSelectUser => (
                "Change Auth Method — Select User",
                None,
                self.user_list_items(true),
            ),
            UserManagementMode::ChangeAuthChoose { username } => (
                "Choose Authentication Method",
                Some(format!("Change auth for '{username}'")),
                self.auth_method_items(),
            ),
            UserManagementMode::ToggleAdmin => (
                "Toggle Admin",
                None,
                self.user_list_items(false),
            ),
        };
        let mut selected = self
            .terminal_user_management_idx
            .min(items.iter().filter(|i| i.as_str() != "---").count().saturating_sub(1));
        let refs: Vec<String> = items;
        let activated =
            self.draw_terminal_menu_screen(ctx, title, subtitle.as_deref(), &refs, &mut selected);
        self.terminal_user_management_idx = selected;
        if let Some(idx) = activated {
            let selected_label = refs[idx].clone();
            match &mode {
                UserManagementMode::Root => match selected_label.as_str() {
                    "Create User" => self.open_input_prompt(
                        "Create User",
                        "New username:",
                        TerminalPromptAction::CreateUsername,
                    ),
                    "Delete User" => {
                        self.terminal_user_management_mode = UserManagementMode::DeleteUser;
                        self.terminal_user_management_idx = 0;
                    }
                    "Reset Password" => {
                        self.terminal_user_management_mode = UserManagementMode::ResetPassword;
                        self.terminal_user_management_idx = 0;
                    }
                    "Change Auth Method" => {
                        self.terminal_user_management_mode =
                            UserManagementMode::ChangeAuthSelectUser;
                        self.terminal_user_management_idx = 0;
                    }
                    "Toggle Admin" => {
                        self.terminal_user_management_mode = UserManagementMode::ToggleAdmin;
                        self.terminal_user_management_idx = 0;
                    }
                    "Back" => {
                        self.terminal_screen = TerminalScreen::Settings;
                        self.terminal_user_management_idx = 0;
                    }
                    _ => {}
                },
                UserManagementMode::CreateAuthMethod { username } => {
                    if selected_label == "Back" {
                        self.terminal_user_management_mode = UserManagementMode::Root;
                        self.terminal_user_management_idx = 0;
                    } else if let Some(method) = Self::auth_method_from_label(&selected_label) {
                        match method {
                            crate::core::auth::AuthMethod::Password => {
                                self.open_password_prompt_with_action(
                                    "Create User",
                                    format!("Password for {username}"),
                                    TerminalPromptAction::CreatePassword {
                                        username: username.clone(),
                                    },
                                );
                            }
                            crate::core::auth::AuthMethod::NoPassword => {
                                self.save_user_and_status(
                                    username,
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
                                self.shell_status =
                                    "Hacking auth user creation is pending native rewrite.".to_string();
                                self.terminal_user_management_mode = UserManagementMode::Root;
                                self.terminal_user_management_idx = 0;
                            }
                        }
                    }
                }
                UserManagementMode::DeleteUser => {
                    if selected_label == "Back" {
                        self.terminal_user_management_mode = UserManagementMode::Root;
                        self.terminal_user_management_idx = 0;
                    } else if self
                        .session
                        .as_ref()
                        .is_some_and(|s| s.username == selected_label)
                    {
                        self.shell_status = "Cannot delete yourself.".to_string();
                    } else {
                        self.open_confirm_prompt(
                            "Delete User",
                            format!("Delete user '{selected_label}'?"),
                            TerminalPromptAction::ConfirmDeleteUser {
                                username: selected_label,
                            },
                        );
                    }
                }
                UserManagementMode::ResetPassword => {
                    if selected_label == "Back" {
                        self.terminal_user_management_mode = UserManagementMode::Root;
                        self.terminal_user_management_idx = 0;
                    } else {
                        self.open_password_prompt_with_action(
                            "Reset Password",
                            format!("New password for '{selected_label}'"),
                            TerminalPromptAction::ResetPassword {
                                username: selected_label,
                            },
                        );
                    }
                }
                UserManagementMode::ChangeAuthSelectUser => {
                    if selected_label == "Back" {
                        self.terminal_user_management_mode = UserManagementMode::Root;
                        self.terminal_user_management_idx = 0;
                    } else {
                        self.terminal_user_management_mode =
                            UserManagementMode::ChangeAuthChoose {
                                username: selected_label,
                            };
                        self.terminal_user_management_idx = 0;
                    }
                }
                UserManagementMode::ChangeAuthChoose { username } => {
                    if selected_label == "Back" {
                        self.terminal_user_management_mode =
                            UserManagementMode::ChangeAuthSelectUser;
                        self.terminal_user_management_idx = 0;
                    } else if let Some(method) = Self::auth_method_from_label(&selected_label) {
                        match method {
                            crate::core::auth::AuthMethod::Password => {
                                self.open_password_prompt_with_action(
                                    "Change Auth Method",
                                    format!("New password for '{username}'"),
                                    TerminalPromptAction::ChangeAuthPassword {
                                        username: username.clone(),
                                    },
                                );
                            }
                            crate::core::auth::AuthMethod::NoPassword => {
                                self.update_user_record(
                                    username,
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
                                self.shell_status =
                                    "Hacking auth is pending native rewrite.".to_string();
                                self.terminal_user_management_mode = UserManagementMode::Root;
                                self.terminal_user_management_idx = 0;
                            }
                        }
                    }
                }
                UserManagementMode::ToggleAdmin => {
                    if selected_label == "Back" {
                        self.terminal_user_management_mode = UserManagementMode::Root;
                        self.terminal_user_management_idx = 0;
                    } else {
                        self.open_confirm_prompt(
                            "Toggle Admin",
                            format!("Toggle admin for '{selected_label}'?"),
                            TerminalPromptAction::ConfirmToggleAdmin {
                                username: selected_label,
                            },
                        );
                    }
                }
            }
        }
    }

    fn draw_terminal_footer(&self, ctx: &Context) {
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
            .frame(egui::Frame::none().fill(current_palette().bg).inner_margin(0.0))
            .show(ctx, |ui| {
                let palette = current_palette();
                let (screen, _) = RetroScreen::new(ui, TERMINAL_SCREEN_COLS, 1);
                let painter = ui.painter_at(screen.rect);
                screen.footer_bar(&painter, &palette, &left, &center, "44%");
            });
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
                            self.file_manager.set_cwd(word_processor_dir(&session.username));
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
        let title = self
            .editor
            .path
            .as_ref()
            .and_then(|p| p.file_name())
            .and_then(|p| p.to_str())
            .unwrap_or("ROBCO Word Processor")
            .to_string();
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
        if ctx.input(|i| i.key_pressed(Key::S) && i.modifiers.command) {
            self.save_editor();
        }
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
                changed |= ui.checkbox(&mut self.settings.draft.sound, "Sound").changed();
                changed |= ui.checkbox(&mut self.settings.draft.bootup, "Bootup").changed();
                changed |= ui.checkbox(
                    &mut self.settings.draft.show_navigation_hints,
                    "Show navigation hints",
                ).changed();
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
                    ui.label("Interface Size");
                    let mut scale_idx = NATIVE_UI_SCALE_OPTIONS
                        .iter()
                        .position(|v| (*v - self.settings.draft.native_ui_scale).abs() < 0.001)
                        .unwrap_or(1);
                    egui::ComboBox::from_id_salt("native_settings_scale")
                        .selected_text(format!(
                            "{}%",
                            (self.settings.draft.native_ui_scale * 100.0).round() as i32
                        ))
                        .show_ui(ui, |ui| {
                            for (idx, value) in NATIVE_UI_SCALE_OPTIONS.iter().enumerate() {
                                if ui
                                    .selectable_value(
                                        &mut scale_idx,
                                        idx,
                                        format!("{}%", (*value * 100.0).round() as i32),
                                    )
                                    .changed()
                                {
                                    self.settings.draft.native_ui_scale = *value;
                                    changed = true;
                                }
                            }
                        });
                });
                ui.horizontal(|ui| {
                    ui.label("Default Open Mode");
                    changed |= ui.selectable_value(
                        &mut self.settings.draft.default_open_mode,
                        OpenMode::Terminal,
                        "Terminal",
                    ).changed();
                    changed |= ui.selectable_value(
                        &mut self.settings.draft.default_open_mode,
                        OpenMode::Desktop,
                        "Desktop",
                    ).changed();
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
                    self.applications.status =
                        "Nuke Codes UI is not rewritten yet.".to_string();
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
                ui.small("The native shell launches the existing `robcos` TUI in your system terminal.");
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

    fn draw_terminal_flash(&self, ctx: &Context, message: &str) {
        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(current_palette().bg).inner_margin(0.0))
            .show(ctx, |ui| {
                let palette = current_palette();
                let (screen, _) = RetroScreen::new(ui, TERMINAL_SCREEN_COLS, TERMINAL_SCREEN_ROWS);
                let painter = ui.painter_at(screen.rect);
                screen.paint_bg(&painter, palette.bg);
                for (idx, line) in HEADER_LINES.iter().enumerate() {
                    screen.centered_text(
                        &painter,
                        TERMINAL_HEADER_START_ROW + idx,
                        line,
                        palette.fg,
                        true,
                    );
                }
                screen.separator(&painter, TERMINAL_SEPARATOR_TOP_ROW, &palette);
                screen.separator(&painter, TERMINAL_SEPARATOR_BOTTOM_ROW, &palette);
                screen.text(&painter, TERMINAL_CONTENT_COL, TERMINAL_STATUS_ROW, message, palette.fg);
            });
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
                }
            } else {
                ctx.request_repaint_after(flash.until.saturating_duration_since(Instant::now()));
                self.draw_terminal_flash(ctx, &flash.message);
                self.draw_terminal_footer(ctx);
                return;
            }
        }

        if self.session.is_none() {
            self.draw_login(ctx);
            return;
        }

        if !self.desktop_mode_open && ctx.input(|i| i.key_pressed(Key::Escape) || i.key_pressed(Key::Tab)) {
            self.handle_terminal_back();
        }

        if self.desktop_mode_open {
            self.draw_top_bar(ctx);
            self.draw_start_panel(ctx);
            self.draw_desktop(ctx);
        } else {
            match self.terminal_screen {
                TerminalScreen::MainMenu => self.draw_terminal_main_menu(ctx),
                TerminalScreen::Applications => self.draw_terminal_applications(ctx),
                TerminalScreen::Documents => self.draw_terminal_documents(ctx),
                TerminalScreen::DocumentBrowser => self.draw_terminal_document_browser(ctx),
                TerminalScreen::Settings => self.draw_terminal_settings(ctx),
                TerminalScreen::UserManagement => self.draw_terminal_user_management(ctx),
            }
        }
        self.draw_terminal_footer(ctx);
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
