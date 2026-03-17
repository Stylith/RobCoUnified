pub use robcos_native_services::shared_types::TerminalScreen;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsChoiceKind {
    Theme,
    DefaultOpenMode,
}

#[derive(Debug, Clone, Copy)]
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
    fn settings_menu_action_maps_to_terminal_settings() {
        assert_eq!(
            resolve_main_menu_action(MainMenuAction::Settings),
            MainMenuSelectionAction::RefreshSettingsAndOpen
        );
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
}
