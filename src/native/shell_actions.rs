use super::menu::{MainMenuAction, TerminalScreen};
use crate::core::auth::{load_users, AuthMethod};

#[derive(Debug, Clone)]
pub enum LoginSelectionAction {
    Exit,
    PromptPassword { username: String },
    AuthenticateWithoutPassword { username: String },
    StartHacking { username: String },
    ShowError(String),
}

#[derive(Debug, Clone)]
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

pub fn resolve_login_selection(selected_idx: usize, usernames: &[String]) -> LoginSelectionAction {
    let idx = selected_idx.min(usernames.len());
    if idx == usernames.len() {
        return LoginSelectionAction::Exit;
    }
    let Some(selected) = usernames.get(idx).cloned() else {
        return LoginSelectionAction::ShowError("Unknown user.".to_string());
    };
    let db = load_users();
    let Some(record) = db.get(&selected) else {
        return LoginSelectionAction::ShowError("Unknown user.".to_string());
    };
    match record.auth_method {
        AuthMethod::NoPassword => {
            LoginSelectionAction::AuthenticateWithoutPassword { username: selected }
        }
        AuthMethod::Password => LoginSelectionAction::PromptPassword { username: selected },
        AuthMethod::HackingMinigame => LoginSelectionAction::StartHacking { username: selected },
    }
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
