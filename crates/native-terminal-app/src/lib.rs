pub use robcos_native_services::desktop_default_apps_service::DefaultAppSlot;
pub use robcos_native_services::shared_types::TerminalScreen;
use robcos_shared::core::auth::AuthMethod;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsChoiceKind {
    Theme,
    DefaultOpenMode,
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
        TerminalScreen::DonkeyKong
        | TerminalScreen::NukeCodes
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
        TerminalScreen::DonkeyKong => TerminalBackAction::NavigateTo {
            screen: TerminalScreen::Games,
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
        let action = resolve_login_selection(0, &["admin".to_string()], |_| {
            Ok(AuthMethod::Password)
        });
        assert_eq!(
            action,
            LoginSelectionAction::PromptPassword {
                username: "admin".to_string()
            }
        );
    }

    #[test]
    fn login_selection_returns_exit_for_exit_row() {
        let action = resolve_login_selection(1, &["admin".to_string()], |_| {
            Ok(AuthMethod::Password)
        });
        assert_eq!(action, LoginSelectionAction::Exit);
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
        assert_eq!(defaults.nuke_codes_return_screen, TerminalScreen::Applications);
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
