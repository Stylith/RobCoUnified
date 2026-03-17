use super::desktop_session_service::login_selection_auth_method;
pub use robcos_native_terminal_app::{
    resolve_main_menu_action, resolve_terminal_back_action, MainMenuSelectionAction,
    TerminalBackAction, TerminalBackContext,
};
use crate::core::auth::AuthMethod;

#[derive(Debug, Clone)]
pub enum LoginSelectionAction {
    Exit,
    PromptPassword { username: String },
    AuthenticateWithoutPassword { username: String },
    StartHacking { username: String },
    ShowError(String),
}

pub fn resolve_login_selection(selected_idx: usize, usernames: &[String]) -> LoginSelectionAction {
    let idx = selected_idx.min(usernames.len());
    if idx == usernames.len() {
        return LoginSelectionAction::Exit;
    }
    let Some(selected) = usernames.get(idx).cloned() else {
        return LoginSelectionAction::ShowError("Unknown user.".to_string());
    };
    match login_selection_auth_method(&selected) {
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
