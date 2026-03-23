mod user_management;

pub use robcos_native_services::desktop_default_apps_service::DefaultAppSlot;
use robcos_native_services::desktop_settings_service::pty_force_render_mode;
use robcos_native_services::shared_types::FlashAction;
pub use robcos_native_services::shared_types::TerminalScreen;
use robcos_shared::config::HackingDifficulty;
use robcos_shared::core::auth::AuthMethod;
use robcos_shared::core::hacking::HackingGame;
pub use user_management::{
    handle_user_management_selection, plan_user_management_action, user_management_screen_for_mode,
    UserManagementAction, UserManagementExecutionPlan, UserManagementScreen,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsChoiceKind {
    Theme,
    DefaultOpenMode,
    WindowMode,
    CrtPreset,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SettingsChoiceOverlay {
    pub kind: SettingsChoiceKind,
    pub selected: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UserManagementMode {
    Root,
    CreateAuthMethod { username: String },
    CreateHackingDifficulty { username: String },
    DeleteUser,
    ResetPassword,
    ChangeAuthSelectUser,
    ChangeAuthChoose { username: String },
    ChangeAuthHackingDifficulty { username: String },
    ToggleAdmin,
}

#[derive(Debug, Clone)]
pub enum LoginMenuRow {
    User(String),
    Separator,
    Exit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MainMenuAction {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MainMenuEntry {
    pub label: &'static str,
    pub action: Option<MainMenuAction>,
}

pub const MAIN_MENU_ENTRIES: &[MainMenuEntry] = &[
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MainMenuSelectionAction {
    OpenScreen {
        screen: TerminalScreen,
        selected_idx: usize,
        clear_status: bool,
    },
    OpenTerminalMode,
    EnterDesktopMode,
    RefreshSettingsAndOpen,
    BeginLogout,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoginSelectionAction {
    Exit,
    PromptPassword { username: String },
    AuthenticateWithoutPassword { username: String },
    StartHacking { username: String },
    ShowError(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TerminalLoginSelectionPlan<User> {
    Exit,
    PromptPassword {
        username: String,
        prompt: TerminalPromptSpec,
    },
    Submit {
        action: TerminalLoginSubmitAction<User>,
        missing_username_is_select_user: bool,
    },
    StartHacking {
        username: String,
    },
    ShowError(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TerminalLoginPasswordPlan<User> {
    pub action: TerminalLoginSubmitAction<User>,
    pub reopen_prompt: Option<TerminalPromptSpec>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TerminalLoginSubmitAction<User> {
    MissingUsername,
    Authenticated { username: String, user: User },
    ShowError(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TerminalLoginScreenMode {
    #[default]
    SelectUser,
    Hacking,
    Locked,
}

#[derive(Debug)]
pub struct TerminalLoginHackingState {
    pub username: String,
    pub game: HackingGame,
}

#[derive(Debug, Default)]
pub struct TerminalLoginState {
    pub selected_idx: usize,
    pub selected_username: String,
    pub password: String,
    pub error: String,
    pub mode: TerminalLoginScreenMode,
    pub hacking: Option<TerminalLoginHackingState>,
}

impl TerminalLoginState {
    pub fn clear_password_and_error(&mut self) {
        self.password.clear();
        self.error.clear();
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }

    pub fn start_hacking(&mut self, username: String, difficulty: HackingDifficulty) {
        self.mode = TerminalLoginScreenMode::Hacking;
        self.hacking = Some(TerminalLoginHackingState {
            username,
            game: HackingGame::new(difficulty),
        });
    }

    pub fn show_user_selection(&mut self) {
        self.mode = TerminalLoginScreenMode::SelectUser;
        self.hacking = None;
    }

    pub fn show_locked(&mut self) {
        self.mode = TerminalLoginScreenMode::Locked;
        self.hacking = None;
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TerminalPromptSpec {
    pub title: String,
    pub prompt: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalShellSurface {
    Embedded,
    Desktop,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TerminalPtyLaunchPlan {
    pub title: String,
    pub argv: Vec<String>,
    pub env: Vec<(String, String)>,
    pub return_screen: TerminalScreen,
    pub force_render_mode: Option<bool>,
    pub replace_existing_pty: bool,
    pub use_fixed_terminal_metrics: bool,
    pub success_status: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TerminalEmbeddedPtyExitPlan {
    pub return_screen: TerminalScreen,
    pub status: String,
    pub boxed_flash_message: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TerminalDesktopPtyExitPlan {
    pub status: String,
    pub reopen_installer: bool,
    pub installer_notice_message: Option<String>,
    pub installer_notice_success: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TerminalFlashPtyLaunchPlan {
    pub launch: TerminalPtyLaunchPlan,
    pub status: String,
    pub completion_message: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TerminalFlashActionPlan {
    StartHacking {
        username: String,
        difficulty: HackingDifficulty,
    },
    LaunchPty(TerminalFlashPtyLaunchPlan),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalUserPasswordFlow {
    Create,
    Reset,
    ChangeAuth,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TerminalUserManagementPromptPlan {
    Status(String),
    SetMode {
        mode: UserManagementMode,
        selected_idx: usize,
        suppress_next_menu_submit: bool,
    },
    OpenPasswordConfirm {
        flow: TerminalUserPasswordFlow,
        username: String,
        first_password: String,
        prompt: TerminalPromptSpec,
    },
    ApplyPassword {
        flow: TerminalUserPasswordFlow,
        username: String,
        password: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalHackingUiEvent {
    Cancel,
    Success,
    LockedOut,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TerminalHackingPlan<User> {
    ShowUserSelection,
    ShowLocked,
    Submit {
        action: TerminalLoginSubmitAction<User>,
        fallback_to_user_selection_on_error: bool,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TerminalBackContext {
    pub screen: TerminalScreen,
    pub has_settings_choice: bool,
    pub has_default_app_slot: bool,
    pub connections_at_root: bool,
    pub installer_at_root: bool,
    pub has_embedded_pty: bool,
    pub pty_return_screen: TerminalScreen,
    pub nuke_codes_return_screen: TerminalScreen,
    pub browser_return_screen: TerminalScreen,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalBackAction {
    NoOp,
    ClearSettingsChoice,
    ClearDefaultAppSlot,
    UseConnectionsInnerBack,
    UseInstallerInnerBack,
    NavigateTo {
        screen: TerminalScreen,
        clear_status: bool,
        reset_installer: bool,
    },
    ClosePtyAndReturn {
        return_screen: TerminalScreen,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TerminalNavigationState {
    pub main_menu_idx: usize,
    pub screen: TerminalScreen,
    pub apps_idx: usize,
    pub documents_idx: usize,
    pub logs_idx: usize,
    pub network_idx: usize,
    pub games_idx: usize,
    pub nuke_codes_return_screen: TerminalScreen,
    pub settings_idx: usize,
    pub default_apps_idx: usize,
    pub default_app_choice_idx: usize,
    pub default_app_slot: Option<DefaultAppSlot>,
    pub browser_idx: usize,
    pub browser_return_screen: TerminalScreen,
    pub user_management_idx: usize,
    pub user_management_mode: UserManagementMode,
    pub settings_choice: Option<SettingsChoiceOverlay>,
    pub suppress_next_menu_submit: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalSelectionIndexTarget {
    None,
    MainMenu,
    Applications,
    Documents,
    Logs,
    Network,
    Games,
    ProgramInstallerRoot,
    Settings,
    ConnectionsRoot,
    DefaultApps,
    UserManagement,
    DocumentBrowser,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TerminalScreenOpenPlan {
    pub screen: TerminalScreen,
    pub index_target: TerminalSelectionIndexTarget,
    pub selected_idx: usize,
    pub reset_installer: bool,
    pub reset_connections: bool,
    pub clear_settings_choice: bool,
    pub clear_default_app_slot: bool,
    pub reset_user_management_to_root: bool,
    pub clear_status: bool,
}

pub fn selectable_menu_count() -> usize {
    MAIN_MENU_ENTRIES
        .iter()
        .filter(|entry| entry.action.is_some())
        .count()
}

pub fn entry_for_selectable_idx(idx: usize) -> MainMenuEntry {
    MAIN_MENU_ENTRIES
        .iter()
        .copied()
        .filter(|entry| entry.action.is_some())
        .nth(idx)
        .unwrap_or(MAIN_MENU_ENTRIES[0])
}

pub fn login_menu_rows_from_users(usernames: Vec<String>) -> Vec<LoginMenuRow> {
    let mut rows: Vec<LoginMenuRow> = usernames.into_iter().map(LoginMenuRow::User).collect();
    rows.push(LoginMenuRow::Separator);
    rows.push(LoginMenuRow::Exit);
    rows
}

pub fn resolve_login_selection<F>(
    selected_idx: usize,
    usernames: &[String],
    auth_method_for: F,
) -> LoginSelectionAction
where
    F: FnOnce(&str) -> Result<AuthMethod, String>,
{
    let idx = selected_idx.min(usernames.len());
    if idx == usernames.len() {
        return LoginSelectionAction::Exit;
    }
    let Some(selected) = usernames.get(idx).cloned() else {
        return LoginSelectionAction::ShowError("Unknown user.".to_string());
    };
    match auth_method_for(&selected) {
        Ok(AuthMethod::NoPassword) => {
            LoginSelectionAction::AuthenticateWithoutPassword { username: selected }
        }
        Ok(AuthMethod::Password) => LoginSelectionAction::PromptPassword { username: selected },
        Ok(AuthMethod::HackingMinigame) => {
            LoginSelectionAction::StartHacking { username: selected }
        }
        Err(error) => LoginSelectionAction::ShowError(error),
    }
}

pub fn resolve_login_submission<User, F>(
    username: &str,
    password: &str,
    authenticate: F,
) -> TerminalLoginSubmitAction<User>
where
    F: FnOnce(&str, &str) -> Result<User, String>,
{
    let username = username.trim().to_string();
    if username.is_empty() {
        return TerminalLoginSubmitAction::MissingUsername;
    }
    match authenticate(&username, password) {
        Ok(user) => TerminalLoginSubmitAction::Authenticated { username, user },
        Err(error) => TerminalLoginSubmitAction::ShowError(error),
    }
}

pub fn login_password_prompt(username: &str) -> TerminalPromptSpec {
    TerminalPromptSpec {
        title: "Password Prompt".to_string(),
        prompt: format!("Password for {}", username.trim()),
    }
}

fn confirm_password_prompt(username: &str) -> TerminalPromptSpec {
    TerminalPromptSpec {
        title: "Confirm Password".to_string(),
        prompt: format!("Re-enter password for {}", username.trim()),
    }
}

pub fn resolve_create_username_prompt(
    raw_username: &str,
    username_exists: bool,
) -> TerminalUserManagementPromptPlan {
    let username = raw_username.trim().to_string();
    if username.is_empty() {
        return TerminalUserManagementPromptPlan::Status("Username cannot be empty.".to_string());
    }
    if username_exists {
        return TerminalUserManagementPromptPlan::Status("User already exists.".to_string());
    }
    TerminalUserManagementPromptPlan::SetMode {
        mode: UserManagementMode::CreateAuthMethod { username },
        selected_idx: 0,
        suppress_next_menu_submit: true,
    }
}

pub fn resolve_user_password_first_prompt(
    flow: TerminalUserPasswordFlow,
    username: String,
    password: String,
) -> TerminalUserManagementPromptPlan {
    if password.is_empty() {
        return TerminalUserManagementPromptPlan::Status("Password cannot be empty.".to_string());
    }
    TerminalUserManagementPromptPlan::OpenPasswordConfirm {
        flow,
        prompt: confirm_password_prompt(&username),
        username,
        first_password: password,
    }
}

pub fn resolve_user_password_confirm_prompt(
    flow: TerminalUserPasswordFlow,
    username: String,
    first_password: String,
    confirmation: String,
) -> TerminalUserManagementPromptPlan {
    if confirmation != first_password {
        return TerminalUserManagementPromptPlan::Status("Passwords do not match.".to_string());
    }
    TerminalUserManagementPromptPlan::ApplyPassword {
        flow,
        username,
        password: first_password,
    }
}

pub fn resolve_login_password_submission<User, F>(
    username: &str,
    password: &str,
    has_active_session: bool,
    has_terminal_flash: bool,
    authenticate: F,
) -> TerminalLoginPasswordPlan<User>
where
    F: FnOnce(&str, &str) -> Result<User, String>,
{
    let action = resolve_login_submission(username, password, authenticate);
    let reopen_prompt =
        if should_reopen_login_password_prompt(has_active_session, has_terminal_flash) {
            Some(login_password_prompt(username))
        } else {
            None
        };
    TerminalLoginPasswordPlan {
        action,
        reopen_prompt,
    }
}

pub fn terminal_command_launch_plan(
    surface: TerminalShellSurface,
    title: &str,
    argv: &[String],
    return_screen: TerminalScreen,
    force_render_mode: Option<bool>,
) -> TerminalPtyLaunchPlan {
    let (replace_existing_pty, use_fixed_terminal_metrics, success_status) = match surface {
        TerminalShellSurface::Embedded => (false, true, format!("Opened {title} in PTY.")),
        TerminalShellSurface::Desktop => (true, false, format!("Opened {title} in PTY window.")),
    };
    TerminalPtyLaunchPlan {
        title: title.to_string(),
        argv: argv.to_vec(),
        env: Vec::new(),
        return_screen,
        force_render_mode,
        replace_existing_pty,
        use_fixed_terminal_metrics,
        success_status,
    }
}

pub fn resolve_flash_pty_launch(
    title: &str,
    argv: &[String],
    return_screen: TerminalScreen,
    status: &str,
    force_render_mode: Option<bool>,
    completion_message: Option<String>,
) -> TerminalFlashPtyLaunchPlan {
    TerminalFlashPtyLaunchPlan {
        launch: terminal_command_launch_plan(
            TerminalShellSurface::Embedded,
            title,
            argv,
            return_screen,
            force_render_mode,
        ),
        status: status.to_string(),
        completion_message,
    }
}

pub fn resolve_terminal_flash_action(
    action: &FlashAction,
    hacking_difficulty: HackingDifficulty,
) -> Option<TerminalFlashActionPlan> {
    match action {
        FlashAction::StartHacking { username } => Some(TerminalFlashActionPlan::StartHacking {
            username: username.clone(),
            difficulty: hacking_difficulty,
        }),
        FlashAction::LaunchPty {
            title,
            argv,
            return_screen,
            status,
            completion_message,
        } => Some(TerminalFlashActionPlan::LaunchPty(
            resolve_flash_pty_launch(
                title,
                argv,
                *return_screen,
                status,
                pty_force_render_mode(argv),
                completion_message.clone(),
            ),
        )),
        _ => None,
    }
}

pub fn resolve_embedded_pty_exit(
    title: &str,
    return_screen: TerminalScreen,
    completion_message: Option<&str>,
) -> TerminalEmbeddedPtyExitPlan {
    if matches!(return_screen, TerminalScreen::ProgramInstaller) {
        if let Some(message) = completion_message {
            return TerminalEmbeddedPtyExitPlan {
                return_screen,
                status: message.to_string(),
                boxed_flash_message: Some(message.to_string()),
            };
        }
    }
    TerminalEmbeddedPtyExitPlan {
        return_screen,
        status: format!("{title} exited."),
        boxed_flash_message: None,
    }
}

pub fn resolve_desktop_pty_exit(
    title: &str,
    completion_message: Option<&str>,
    success: bool,
    exit_code: Option<u32>,
) -> TerminalDesktopPtyExitPlan {
    let status = if success {
        completion_message
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| format!("{title} exited."))
    } else if let Some(code) = exit_code {
        format!("{title} failed with exit code {code}.")
    } else {
        format!("{title} failed.")
    };
    let reopen_installer = title == "Program Installer";
    TerminalDesktopPtyExitPlan {
        installer_notice_message: reopen_installer.then(|| status.clone()),
        installer_notice_success: success,
        reopen_installer,
        status,
    }
}

pub fn terminal_shell_launch_plan(
    surface: TerminalShellSurface,
    requested_shell: Option<&str>,
    bash_exists: bool,
) -> TerminalPtyLaunchPlan {
    let requested_shell = requested_shell
        .filter(|shell| !shell.is_empty())
        .unwrap_or_else(|| {
            if cfg!(target_os = "macos") {
                "/bin/zsh"
            } else if bash_exists {
                "/bin/bash"
            } else {
                "/bin/sh"
            }
        });
    let requested_shell_name = std::path::Path::new(requested_shell)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("");
    let shell = if requested_shell_name == "fish" {
        if bash_exists {
            "/bin/bash".to_string()
        } else {
            "/bin/sh".to_string()
        }
    } else {
        requested_shell.to_string()
    };
    let shell_name = std::path::Path::new(&shell)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("");
    let mut argv = vec![shell.clone()];
    if matches!(shell_name, "bash" | "zsh") {
        argv.push("-l".to_string());
    }

    let title = match surface {
        TerminalShellSurface::Embedded => "ROBCO MAINTENANCE TERMLINK",
        TerminalShellSurface::Desktop => "Terminal",
    };
    let mut plan =
        terminal_command_launch_plan(surface, title, &argv, TerminalScreen::MainMenu, Some(false));
    plan.success_status = match surface {
        TerminalShellSurface::Embedded => "Opened terminal shell in PTY.".to_string(),
        TerminalShellSurface::Desktop => "Opened terminal shell in PTY window.".to_string(),
    };
    plan
}

pub fn resolve_login_selection_plan<User, AuthMethodFor, AuthenticateWithoutPassword>(
    selected_idx: usize,
    usernames: &[String],
    auth_method_for: AuthMethodFor,
    authenticate_without_password: AuthenticateWithoutPassword,
) -> TerminalLoginSelectionPlan<User>
where
    AuthMethodFor: FnOnce(&str) -> Result<AuthMethod, String>,
    AuthenticateWithoutPassword: FnOnce(&str) -> Result<User, String>,
{
    match resolve_login_selection(selected_idx, usernames, auth_method_for) {
        LoginSelectionAction::Exit => TerminalLoginSelectionPlan::Exit,
        LoginSelectionAction::PromptPassword { username } => {
            TerminalLoginSelectionPlan::PromptPassword {
                prompt: login_password_prompt(&username),
                username,
            }
        }
        LoginSelectionAction::AuthenticateWithoutPassword { username } => {
            TerminalLoginSelectionPlan::Submit {
                action: match authenticate_without_password(&username) {
                    Ok(user) => TerminalLoginSubmitAction::Authenticated { username, user },
                    Err(error) => TerminalLoginSubmitAction::ShowError(error),
                },
                missing_username_is_select_user: false,
            }
        }
        LoginSelectionAction::StartHacking { username } => {
            TerminalLoginSelectionPlan::StartHacking { username }
        }
        LoginSelectionAction::ShowError(error) => TerminalLoginSelectionPlan::ShowError(error),
    }
}

pub fn resolve_hacking_success<User, F>(
    username: &str,
    user_for_username: F,
) -> TerminalLoginSubmitAction<User>
where
    F: FnOnce(&str) -> Option<User>,
{
    let username = username.trim().to_string();
    if username.is_empty() {
        return TerminalLoginSubmitAction::ShowError("Unknown user.".to_string());
    }
    match user_for_username(&username) {
        Some(user) => TerminalLoginSubmitAction::Authenticated { username, user },
        None => TerminalLoginSubmitAction::ShowError("Unknown user.".to_string()),
    }
}

pub fn resolve_hacking_screen_event<User, F>(
    username: &str,
    event: TerminalHackingUiEvent,
    user_for_username: F,
) -> TerminalHackingPlan<User>
where
    F: FnOnce(&str) -> Option<User>,
{
    match event {
        TerminalHackingUiEvent::Cancel => TerminalHackingPlan::ShowUserSelection,
        TerminalHackingUiEvent::LockedOut => TerminalHackingPlan::ShowLocked,
        TerminalHackingUiEvent::Success => TerminalHackingPlan::Submit {
            action: resolve_hacking_success(username, user_for_username),
            fallback_to_user_selection_on_error: true,
        },
    }
}

pub fn should_reopen_login_password_prompt(
    has_active_session: bool,
    has_terminal_flash: bool,
) -> bool {
    !has_active_session && !has_terminal_flash
}

pub fn terminal_runtime_defaults() -> TerminalNavigationState {
    TerminalNavigationState {
        main_menu_idx: 0,
        screen: TerminalScreen::MainMenu,
        apps_idx: 0,
        documents_idx: 0,
        logs_idx: 0,
        network_idx: 0,
        games_idx: 0,
        nuke_codes_return_screen: TerminalScreen::Applications,
        settings_idx: 0,
        default_apps_idx: 0,
        default_app_choice_idx: 0,
        default_app_slot: None,
        browser_idx: 0,
        browser_return_screen: TerminalScreen::Documents,
        user_management_idx: 0,
        user_management_mode: UserManagementMode::Root,
        settings_choice: None,
        suppress_next_menu_submit: false,
    }
}

pub fn terminal_screen_open_plan(
    screen: TerminalScreen,
    selected_idx: usize,
    clear_status: bool,
) -> TerminalScreenOpenPlan {
    match screen {
        TerminalScreen::Applications => TerminalScreenOpenPlan {
            screen,
            index_target: TerminalSelectionIndexTarget::Applications,
            selected_idx,
            reset_installer: false,
            reset_connections: false,
            clear_settings_choice: false,
            clear_default_app_slot: false,
            reset_user_management_to_root: false,
            clear_status,
        },
        TerminalScreen::Documents => TerminalScreenOpenPlan {
            screen,
            index_target: TerminalSelectionIndexTarget::Documents,
            selected_idx,
            reset_installer: false,
            reset_connections: false,
            clear_settings_choice: false,
            clear_default_app_slot: false,
            reset_user_management_to_root: false,
            clear_status,
        },
        TerminalScreen::Logs => TerminalScreenOpenPlan {
            screen,
            index_target: TerminalSelectionIndexTarget::Logs,
            selected_idx,
            reset_installer: false,
            reset_connections: false,
            clear_settings_choice: false,
            clear_default_app_slot: false,
            reset_user_management_to_root: false,
            clear_status,
        },
        TerminalScreen::Network => TerminalScreenOpenPlan {
            screen,
            index_target: TerminalSelectionIndexTarget::Network,
            selected_idx,
            reset_installer: false,
            reset_connections: false,
            clear_settings_choice: false,
            clear_default_app_slot: false,
            reset_user_management_to_root: false,
            clear_status,
        },
        TerminalScreen::Games => TerminalScreenOpenPlan {
            screen,
            index_target: TerminalSelectionIndexTarget::Games,
            selected_idx,
            reset_installer: false,
            reset_connections: false,
            clear_settings_choice: false,
            clear_default_app_slot: false,
            reset_user_management_to_root: false,
            clear_status,
        },
        TerminalScreen::ProgramInstaller => TerminalScreenOpenPlan {
            screen,
            index_target: TerminalSelectionIndexTarget::ProgramInstallerRoot,
            selected_idx,
            reset_installer: true,
            reset_connections: false,
            clear_settings_choice: false,
            clear_default_app_slot: false,
            reset_user_management_to_root: false,
            clear_status,
        },
        TerminalScreen::Settings => TerminalScreenOpenPlan {
            screen,
            index_target: TerminalSelectionIndexTarget::Settings,
            selected_idx,
            reset_installer: false,
            reset_connections: false,
            clear_settings_choice: true,
            clear_default_app_slot: false,
            reset_user_management_to_root: false,
            clear_status,
        },
        TerminalScreen::Connections => TerminalScreenOpenPlan {
            screen,
            index_target: TerminalSelectionIndexTarget::ConnectionsRoot,
            selected_idx,
            reset_installer: false,
            reset_connections: true,
            clear_settings_choice: false,
            clear_default_app_slot: false,
            reset_user_management_to_root: false,
            clear_status,
        },
        TerminalScreen::DefaultApps => TerminalScreenOpenPlan {
            screen,
            index_target: TerminalSelectionIndexTarget::DefaultApps,
            selected_idx,
            reset_installer: false,
            reset_connections: false,
            clear_settings_choice: false,
            clear_default_app_slot: false,
            reset_user_management_to_root: false,
            clear_status,
        },
        TerminalScreen::UserManagement => TerminalScreenOpenPlan {
            screen,
            index_target: TerminalSelectionIndexTarget::UserManagement,
            selected_idx,
            reset_installer: false,
            reset_connections: false,
            clear_settings_choice: false,
            clear_default_app_slot: false,
            reset_user_management_to_root: true,
            clear_status,
        },
        TerminalScreen::DocumentBrowser => TerminalScreenOpenPlan {
            screen,
            index_target: TerminalSelectionIndexTarget::DocumentBrowser,
            selected_idx,
            reset_installer: false,
            reset_connections: false,
            clear_settings_choice: false,
            clear_default_app_slot: false,
            reset_user_management_to_root: false,
            clear_status,
        },
        TerminalScreen::MainMenu => TerminalScreenOpenPlan {
            screen,
            index_target: TerminalSelectionIndexTarget::MainMenu,
            selected_idx,
            reset_installer: false,
            reset_connections: false,
            clear_settings_choice: false,
            clear_default_app_slot: false,
            reset_user_management_to_root: false,
            clear_status,
        },
        TerminalScreen::NukeCodes
        | TerminalScreen::EditMenus
        | TerminalScreen::About
        | TerminalScreen::PtyApp => TerminalScreenOpenPlan {
            screen,
            index_target: TerminalSelectionIndexTarget::None,
            selected_idx,
            reset_installer: false,
            reset_connections: false,
            clear_settings_choice: false,
            clear_default_app_slot: false,
            reset_user_management_to_root: false,
            clear_status,
        },
    }
}

pub fn terminal_settings_refresh_plan() -> TerminalScreenOpenPlan {
    let mut plan = terminal_screen_open_plan(TerminalScreen::Settings, 0, true);
    plan.reset_connections = true;
    plan.clear_default_app_slot = true;
    plan
}

pub fn resolve_main_menu_action(action: MainMenuAction) -> MainMenuSelectionAction {
    match action {
        MainMenuAction::Applications => MainMenuSelectionAction::OpenScreen {
            screen: TerminalScreen::Applications,
            selected_idx: 0,
            clear_status: true,
        },
        MainMenuAction::Documents => MainMenuSelectionAction::OpenScreen {
            screen: TerminalScreen::Documents,
            selected_idx: 0,
            clear_status: true,
        },
        MainMenuAction::Network => MainMenuSelectionAction::OpenScreen {
            screen: TerminalScreen::Network,
            selected_idx: 0,
            clear_status: true,
        },
        MainMenuAction::Games => MainMenuSelectionAction::OpenScreen {
            screen: TerminalScreen::Games,
            selected_idx: 0,
            clear_status: true,
        },
        MainMenuAction::ProgramInstaller => MainMenuSelectionAction::OpenScreen {
            screen: TerminalScreen::ProgramInstaller,
            selected_idx: 0,
            clear_status: true,
        },
        MainMenuAction::Terminal => MainMenuSelectionAction::OpenTerminalMode,
        MainMenuAction::DesktopMode => MainMenuSelectionAction::EnterDesktopMode,
        MainMenuAction::Settings => MainMenuSelectionAction::RefreshSettingsAndOpen,
        MainMenuAction::Logout => MainMenuSelectionAction::BeginLogout,
    }
}

pub fn resolve_terminal_back_action(context: TerminalBackContext) -> TerminalBackAction {
    if context.has_settings_choice {
        return TerminalBackAction::ClearSettingsChoice;
    }
    if context.has_default_app_slot {
        return TerminalBackAction::ClearDefaultAppSlot;
    }
    if matches!(context.screen, TerminalScreen::Connections) && !context.connections_at_root {
        return TerminalBackAction::UseConnectionsInnerBack;
    }
    if matches!(context.screen, TerminalScreen::ProgramInstaller) && !context.installer_at_root {
        return TerminalBackAction::UseInstallerInnerBack;
    }

    match context.screen {
        TerminalScreen::MainMenu => TerminalBackAction::NoOp,
        TerminalScreen::Applications
        | TerminalScreen::Documents
        | TerminalScreen::Network
        | TerminalScreen::Games
        | TerminalScreen::Settings
        | TerminalScreen::UserManagement => TerminalBackAction::NavigateTo {
            screen: TerminalScreen::MainMenu,
            clear_status: true,
            reset_installer: false,
        },
        TerminalScreen::Logs => TerminalBackAction::NavigateTo {
            screen: TerminalScreen::Documents,
            clear_status: true,
            reset_installer: false,
        },
        TerminalScreen::PtyApp => {
            if context.has_embedded_pty {
                TerminalBackAction::ClosePtyAndReturn {
                    return_screen: context.pty_return_screen,
                }
            } else {
                TerminalBackAction::NavigateTo {
                    screen: TerminalScreen::MainMenu,
                    clear_status: true,
                    reset_installer: false,
                }
            }
        }
        TerminalScreen::ProgramInstaller => TerminalBackAction::NavigateTo {
            screen: TerminalScreen::MainMenu,
            clear_status: true,
            reset_installer: true,
        },
        TerminalScreen::Connections
        | TerminalScreen::DefaultApps
        | TerminalScreen::About
        | TerminalScreen::EditMenus => TerminalBackAction::NavigateTo {
            screen: TerminalScreen::Settings,
            clear_status: true,
            reset_installer: false,
        },
        TerminalScreen::NukeCodes => TerminalBackAction::NavigateTo {
            screen: context.nuke_codes_return_screen,
            clear_status: true,
            reset_installer: false,
        },
        TerminalScreen::DocumentBrowser => TerminalBackAction::NavigateTo {
            screen: context.browser_return_screen,
            clear_status: true,
            reset_installer: false,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selectable_menu_entries_skip_separator() {
        assert_eq!(selectable_menu_count(), 9);
        assert_eq!(
            entry_for_selectable_idx(0).action,
            Some(MainMenuAction::Applications)
        );
        assert_eq!(
            entry_for_selectable_idx(8).action,
            Some(MainMenuAction::Logout)
        );
    }

    #[test]
    fn login_rows_append_separator_and_exit() {
        let rows = login_menu_rows_from_users(vec!["admin".to_string()]);
        assert!(matches!(rows[0], LoginMenuRow::User(_)));
        assert!(matches!(rows[1], LoginMenuRow::Separator));
        assert!(matches!(rows[2], LoginMenuRow::Exit));
    }

    #[test]
    fn login_selection_uses_password_prompt_when_auth_requires_it() {
        let action =
            resolve_login_selection(0, &["admin".to_string()], |_| Ok(AuthMethod::Password));
        assert_eq!(
            action,
            LoginSelectionAction::PromptPassword {
                username: "admin".to_string()
            }
        );
    }

    #[test]
    fn login_selection_returns_exit_for_exit_row() {
        let action =
            resolve_login_selection(1, &["admin".to_string()], |_| Ok(AuthMethod::Password));
        assert_eq!(action, LoginSelectionAction::Exit);
    }

    #[test]
    fn login_selection_plan_builds_password_prompt() {
        let plan = resolve_login_selection_plan(
            0,
            &["admin".to_string()],
            |_| Ok(AuthMethod::Password),
            |_| Ok(()),
        );
        assert_eq!(
            plan,
            TerminalLoginSelectionPlan::PromptPassword {
                username: "admin".to_string(),
                prompt: TerminalPromptSpec {
                    title: "Password Prompt".to_string(),
                    prompt: "Password for admin".to_string(),
                },
            }
        );
    }

    #[test]
    fn login_selection_plan_submits_no_password_users() {
        let plan = resolve_login_selection_plan(
            0,
            &["admin".to_string()],
            |_| Ok(AuthMethod::NoPassword),
            |_| Ok("admin-user".to_string()),
        );
        assert_eq!(
            plan,
            TerminalLoginSelectionPlan::Submit {
                action: TerminalLoginSubmitAction::Authenticated {
                    username: "admin".to_string(),
                    user: "admin-user".to_string(),
                },
                missing_username_is_select_user: false,
            }
        );
    }

    #[test]
    fn create_username_prompt_rejects_blank_username() {
        assert_eq!(
            resolve_create_username_prompt("   ", false),
            TerminalUserManagementPromptPlan::Status("Username cannot be empty.".to_string())
        );
    }

    #[test]
    fn create_username_prompt_enters_auth_selection_mode() {
        assert_eq!(
            resolve_create_username_prompt("admin", false),
            TerminalUserManagementPromptPlan::SetMode {
                mode: UserManagementMode::CreateAuthMethod {
                    username: "admin".to_string()
                },
                selected_idx: 0,
                suppress_next_menu_submit: true,
            }
        );
    }

    #[test]
    fn password_first_prompt_requires_non_empty_password() {
        assert_eq!(
            resolve_user_password_first_prompt(
                TerminalUserPasswordFlow::Create,
                "admin".to_string(),
                String::new()
            ),
            TerminalUserManagementPromptPlan::Status("Password cannot be empty.".to_string())
        );
    }

    #[test]
    fn password_first_prompt_opens_confirmation_prompt() {
        assert_eq!(
            resolve_user_password_first_prompt(
                TerminalUserPasswordFlow::Reset,
                "admin".to_string(),
                "pw".to_string()
            ),
            TerminalUserManagementPromptPlan::OpenPasswordConfirm {
                flow: TerminalUserPasswordFlow::Reset,
                username: "admin".to_string(),
                first_password: "pw".to_string(),
                prompt: TerminalPromptSpec {
                    title: "Confirm Password".to_string(),
                    prompt: "Re-enter password for admin".to_string(),
                },
            }
        );
    }

    #[test]
    fn password_confirm_prompt_rejects_mismatch() {
        assert_eq!(
            resolve_user_password_confirm_prompt(
                TerminalUserPasswordFlow::ChangeAuth,
                "admin".to_string(),
                "pw".to_string(),
                "nope".to_string()
            ),
            TerminalUserManagementPromptPlan::Status("Passwords do not match.".to_string())
        );
    }

    #[test]
    fn password_confirm_prompt_applies_password_on_match() {
        assert_eq!(
            resolve_user_password_confirm_prompt(
                TerminalUserPasswordFlow::Create,
                "admin".to_string(),
                "pw".to_string(),
                "pw".to_string()
            ),
            TerminalUserManagementPromptPlan::ApplyPassword {
                flow: TerminalUserPasswordFlow::Create,
                username: "admin".to_string(),
                password: "pw".to_string(),
            }
        );
    }

    #[test]
    fn embedded_terminal_shell_plan_uses_maintenance_title_and_fixed_metrics() {
        let plan =
            terminal_shell_launch_plan(TerminalShellSurface::Embedded, Some("/bin/zsh"), true);
        assert_eq!(plan.title, "ROBCO MAINTENANCE TERMLINK");
        assert_eq!(plan.argv, vec!["/bin/zsh".to_string(), "-l".to_string()]);
        assert!(plan.env.is_empty());
        assert!(plan.use_fixed_terminal_metrics);
        assert!(!plan.replace_existing_pty);
    }

    #[test]
    fn desktop_terminal_shell_plan_falls_back_from_fish_to_bash() {
        let plan =
            terminal_shell_launch_plan(TerminalShellSurface::Desktop, Some("/usr/bin/fish"), true);
        assert_eq!(plan.argv, vec!["/bin/bash".to_string(), "-l".to_string()]);
        assert_eq!(plan.title, "Terminal");
        assert!(plan.replace_existing_pty);
        assert!(!plan.use_fixed_terminal_metrics);
    }

    #[test]
    fn terminal_shell_plan_uses_platform_default_when_shell_is_missing() {
        let plan = terminal_shell_launch_plan(TerminalShellSurface::Desktop, None, true);
        if cfg!(target_os = "macos") {
            assert_eq!(plan.argv, vec!["/bin/zsh".to_string(), "-l".to_string()]);
        } else {
            assert_eq!(plan.argv, vec!["/bin/bash".to_string(), "-l".to_string()]);
        }
    }

    #[test]
    fn embedded_command_plan_uses_fixed_metrics_and_keeps_return_screen() {
        let plan = terminal_command_launch_plan(
            TerminalShellSurface::Embedded,
            "Program Installer",
            &["installer".to_string()],
            TerminalScreen::ProgramInstaller,
            Some(true),
        );
        assert_eq!(plan.title, "Program Installer");
        assert_eq!(plan.return_screen, TerminalScreen::ProgramInstaller);
        assert!(plan.use_fixed_terminal_metrics);
        assert_eq!(plan.force_render_mode, Some(true));
    }

    #[test]
    fn desktop_command_plan_replaces_existing_pty() {
        let plan = terminal_command_launch_plan(
            TerminalShellSurface::Desktop,
            "Terminal",
            &["sh".to_string()],
            TerminalScreen::MainMenu,
            None,
        );
        assert!(plan.replace_existing_pty);
        assert!(!plan.use_fixed_terminal_metrics);
    }

    #[test]
    fn flash_pty_launch_wraps_embedded_launch_plan_and_status() {
        let plan = resolve_flash_pty_launch(
            "Program Installer",
            &["pkg".to_string()],
            TerminalScreen::ProgramInstaller,
            "Running install...",
            Some(true),
            Some("Done.".to_string()),
        );
        assert_eq!(plan.status, "Running install...");
        assert_eq!(plan.completion_message, Some("Done.".to_string()));
        assert_eq!(plan.launch.title, "Program Installer");
        assert_eq!(plan.launch.return_screen, TerminalScreen::ProgramInstaller);
        assert_eq!(plan.launch.force_render_mode, Some(true));
        assert!(plan.launch.use_fixed_terminal_metrics);
    }

    #[test]
    fn terminal_flash_action_maps_start_hacking() {
        let plan = resolve_terminal_flash_action(
            &FlashAction::StartHacking {
                username: "admin".to_string(),
            },
            HackingDifficulty::Hard,
        );
        assert_eq!(
            plan,
            Some(TerminalFlashActionPlan::StartHacking {
                username: "admin".to_string(),
                difficulty: HackingDifficulty::Hard,
            })
        );
    }

    #[test]
    fn terminal_flash_action_maps_launch_pty() {
        let plan = resolve_terminal_flash_action(
            &FlashAction::LaunchPty {
                title: "Program Installer".to_string(),
                argv: vec!["pkg".to_string()],
                return_screen: TerminalScreen::ProgramInstaller,
                status: "Running install...".to_string(),
                completion_message: Some("Done.".to_string()),
            },
            HackingDifficulty::Easy,
        );
        match plan {
            Some(TerminalFlashActionPlan::LaunchPty(plan)) => {
                assert_eq!(plan.status, "Running install...");
                assert_eq!(plan.launch.title, "Program Installer");
                assert_eq!(plan.launch.return_screen, TerminalScreen::ProgramInstaller);
                assert_eq!(plan.completion_message, Some("Done.".to_string()));
            }
            other => panic!("unexpected flash action plan: {other:?}"),
        }
    }

    #[test]
    fn embedded_pty_exit_uses_boxed_message_for_program_installer() {
        let plan = resolve_embedded_pty_exit(
            "Program Installer",
            TerminalScreen::ProgramInstaller,
            Some("Install complete."),
        );
        assert_eq!(plan.return_screen, TerminalScreen::ProgramInstaller);
        assert_eq!(plan.status, "Install complete.");
        assert_eq!(
            plan.boxed_flash_message,
            Some("Install complete.".to_string())
        );
    }

    #[test]
    fn embedded_pty_exit_falls_back_to_title_status() {
        let plan = resolve_embedded_pty_exit("Top", TerminalScreen::Applications, None);
        assert_eq!(plan.return_screen, TerminalScreen::Applications);
        assert_eq!(plan.status, "Top exited.");
        assert_eq!(plan.boxed_flash_message, None);
    }

    #[test]
    fn desktop_pty_exit_reopens_program_installer_with_notice() {
        let plan =
            resolve_desktop_pty_exit("Program Installer", Some("Install complete."), true, None);
        assert_eq!(plan.status, "Install complete.");
        assert!(plan.reopen_installer);
        assert_eq!(
            plan.installer_notice_message,
            Some("Install complete.".to_string())
        );
        assert!(plan.installer_notice_success);
    }

    #[test]
    fn desktop_pty_exit_reports_failure_code() {
        let plan = resolve_desktop_pty_exit("Terminal", None, false, Some(3));
        assert_eq!(plan.status, "Terminal failed with exit code 3.");
        assert!(!plan.reopen_installer);
        assert_eq!(plan.installer_notice_message, None);
        assert!(!plan.installer_notice_success);
    }

    #[test]
    fn login_submission_rejects_blank_username() {
        let action: TerminalLoginSubmitAction<()> =
            resolve_login_submission("   ", "pw", |_, _| Ok(()));
        assert_eq!(action, TerminalLoginSubmitAction::MissingUsername);
    }

    #[test]
    fn login_state_defaults_to_select_user_mode() {
        let state = TerminalLoginState::default();
        assert_eq!(state.mode, TerminalLoginScreenMode::SelectUser);
        assert!(state.hacking.is_none());
    }

    #[test]
    fn login_submission_returns_authenticated_user() {
        let action = resolve_login_submission("admin", "pw", |username, password| {
            Ok(format!("{username}:{password}"))
        });
        assert_eq!(
            action,
            TerminalLoginSubmitAction::Authenticated {
                username: "admin".to_string(),
                user: "admin:pw".to_string()
            }
        );
    }

    #[test]
    fn hacking_success_resolves_to_authenticated_user() {
        let action = resolve_hacking_success("admin", |_| Some("admin-user".to_string()));
        assert_eq!(
            action,
            TerminalLoginSubmitAction::Authenticated {
                username: "admin".to_string(),
                user: "admin-user".to_string()
            }
        );
    }

    #[test]
    fn hacking_event_cancel_returns_user_selection_plan() {
        let plan = resolve_hacking_screen_event("admin", TerminalHackingUiEvent::Cancel, |_| {
            Some("admin-user".to_string())
        });
        assert_eq!(plan, TerminalHackingPlan::ShowUserSelection);
    }

    #[test]
    fn hacking_event_success_submits_and_falls_back_on_error() {
        let plan = resolve_hacking_screen_event("admin", TerminalHackingUiEvent::Success, |_| {
            Some("admin-user".to_string())
        });
        assert_eq!(
            plan,
            TerminalHackingPlan::Submit {
                action: TerminalLoginSubmitAction::Authenticated {
                    username: "admin".to_string(),
                    user: "admin-user".to_string(),
                },
                fallback_to_user_selection_on_error: true,
            }
        );
    }

    #[test]
    fn login_password_prompt_uses_username_in_prompt() {
        let prompt = login_password_prompt("admin");
        assert_eq!(prompt.title, "Password Prompt");
        assert_eq!(prompt.prompt, "Password for admin");
    }

    #[test]
    fn password_prompt_retry_only_happens_without_session_or_flash() {
        assert!(should_reopen_login_password_prompt(false, false));
        assert!(!should_reopen_login_password_prompt(true, false));
        assert!(!should_reopen_login_password_prompt(false, true));
    }

    #[test]
    fn login_password_submission_reopens_prompt_without_session_or_flash() {
        let plan: TerminalLoginPasswordPlan<()> =
            resolve_login_password_submission("admin", "pw", false, false, |_, _| {
                Err("Wrong password.".to_string())
            });
        assert_eq!(
            plan.action,
            TerminalLoginSubmitAction::ShowError("Wrong password.".to_string())
        );
        assert_eq!(
            plan.reopen_prompt,
            Some(TerminalPromptSpec {
                title: "Password Prompt".to_string(),
                prompt: "Password for admin".to_string(),
            })
        );
    }

    #[test]
    fn login_password_submission_skips_retry_when_session_exists() {
        let plan = resolve_login_password_submission("admin", "pw", true, false, |_, _| Ok(()));
        assert_eq!(
            plan.action,
            TerminalLoginSubmitAction::Authenticated {
                username: "admin".to_string(),
                user: (),
            }
        );
        assert_eq!(plan.reopen_prompt, None);
    }

    #[test]
    fn settings_menu_action_maps_to_terminal_settings() {
        assert_eq!(
            resolve_main_menu_action(MainMenuAction::Settings),
            MainMenuSelectionAction::RefreshSettingsAndOpen
        );
    }

    #[test]
    fn runtime_defaults_start_in_main_menu_and_root_user_management() {
        let defaults = terminal_runtime_defaults();
        assert_eq!(defaults.screen, TerminalScreen::MainMenu);
        assert_eq!(
            defaults.nuke_codes_return_screen,
            TerminalScreen::Applications
        );
        assert_eq!(defaults.browser_return_screen, TerminalScreen::Documents);
        assert_eq!(defaults.user_management_mode, UserManagementMode::Root);
        assert!(!defaults.suppress_next_menu_submit);
    }

    #[test]
    fn back_action_prefers_overlay_state_before_screen_navigation() {
        let action = resolve_terminal_back_action(TerminalBackContext {
            screen: TerminalScreen::Settings,
            has_settings_choice: true,
            has_default_app_slot: false,
            connections_at_root: true,
            installer_at_root: true,
            has_embedded_pty: false,
            pty_return_screen: TerminalScreen::MainMenu,
            nuke_codes_return_screen: TerminalScreen::Applications,
            browser_return_screen: TerminalScreen::Documents,
        });
        assert_eq!(action, TerminalBackAction::ClearSettingsChoice);
    }

    #[test]
    fn back_action_uses_inner_connections_back_before_leaving_screen() {
        let action = resolve_terminal_back_action(TerminalBackContext {
            screen: TerminalScreen::Connections,
            has_settings_choice: false,
            has_default_app_slot: false,
            connections_at_root: false,
            installer_at_root: true,
            has_embedded_pty: false,
            pty_return_screen: TerminalScreen::MainMenu,
            nuke_codes_return_screen: TerminalScreen::Applications,
            browser_return_screen: TerminalScreen::Documents,
        });
        assert_eq!(action, TerminalBackAction::UseConnectionsInnerBack);
    }

    #[test]
    fn back_action_routes_pty_to_return_screen() {
        let action = resolve_terminal_back_action(TerminalBackContext {
            screen: TerminalScreen::PtyApp,
            has_settings_choice: false,
            has_default_app_slot: false,
            connections_at_root: true,
            installer_at_root: true,
            has_embedded_pty: true,
            pty_return_screen: TerminalScreen::Network,
            nuke_codes_return_screen: TerminalScreen::Applications,
            browser_return_screen: TerminalScreen::Documents,
        });
        assert_eq!(
            action,
            TerminalBackAction::ClosePtyAndReturn {
                return_screen: TerminalScreen::Network
            }
        );
    }

    #[test]
    fn settings_refresh_plan_clears_related_terminal_settings_state() {
        let plan = terminal_settings_refresh_plan();
        assert_eq!(plan.screen, TerminalScreen::Settings);
        assert_eq!(plan.index_target, TerminalSelectionIndexTarget::Settings);
        assert_eq!(plan.selected_idx, 0);
        assert!(plan.reset_connections);
        assert!(plan.clear_settings_choice);
        assert!(plan.clear_default_app_slot);
        assert!(plan.clear_status);
    }

    #[test]
    fn user_management_open_plan_resets_mode_to_root() {
        let plan = terminal_screen_open_plan(TerminalScreen::UserManagement, 3, true);
        assert_eq!(
            plan.index_target,
            TerminalSelectionIndexTarget::UserManagement
        );
        assert_eq!(plan.selected_idx, 3);
        assert!(plan.reset_user_management_to_root);
    }
}
