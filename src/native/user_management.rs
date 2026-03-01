use super::menu::UserManagementMode;
use crate::config::{hacking_difficulty_label, HackingDifficulty};
use crate::core::auth::{load_users, AuthMethod};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UserManagementAction {
    None,
    OpenCreateUserPrompt,
    CycleHackingDifficulty,
    SetMode {
        mode: UserManagementMode,
        selected_idx: usize,
    },
    BackToSettings,
    CreateWithMethod {
        username: String,
        method: AuthMethod,
    },
    ApplyCreateHacking {
        username: String,
    },
    ConfirmDeleteUser {
        username: String,
    },
    OpenResetPassword {
        username: String,
    },
    ChangeAuthWithMethod {
        username: String,
        method: AuthMethod,
    },
    ApplyChangeAuthHacking {
        username: String,
    },
    ConfirmToggleAdmin {
        username: String,
    },
    Status(String),
}

pub struct UserManagementScreen {
    pub title: &'static str,
    pub subtitle: Option<String>,
    pub items: Vec<String>,
}

pub fn screen_for_mode(
    mode: &UserManagementMode,
    current_username: Option<&str>,
    hacking_difficulty: HackingDifficulty,
) -> UserManagementScreen {
    match mode {
        UserManagementMode::Root => UserManagementScreen {
            title: "User Management",
            subtitle: None,
            items: root_items(),
        },
        UserManagementMode::CreateAuthMethod { username } => UserManagementScreen {
            title: "Choose Authentication Method",
            subtitle: Some(format!("Create user '{username}'")),
            items: auth_method_items(),
        },
        UserManagementMode::CreateHackingDifficulty { username } => UserManagementScreen {
            title: "Hacking Difficulty",
            subtitle: Some(format!("Create user '{username}'")),
            items: hacking_difficulty_items(hacking_difficulty),
        },
        UserManagementMode::DeleteUser => UserManagementScreen {
            title: "Delete User",
            subtitle: None,
            items: user_list_items(current_username, true),
        },
        UserManagementMode::ResetPassword => UserManagementScreen {
            title: "Reset Password",
            subtitle: None,
            items: user_list_items(current_username, true),
        },
        UserManagementMode::ChangeAuthSelectUser => UserManagementScreen {
            title: "Change Auth Method — Select User",
            subtitle: None,
            items: user_list_items(current_username, true),
        },
        UserManagementMode::ChangeAuthChoose { username } => UserManagementScreen {
            title: "Choose Authentication Method",
            subtitle: Some(format!("Change auth for '{username}'")),
            items: auth_method_items(),
        },
        UserManagementMode::ChangeAuthHackingDifficulty { username } => UserManagementScreen {
            title: "Hacking Difficulty",
            subtitle: Some(format!("Change auth for '{username}'")),
            items: hacking_difficulty_items(hacking_difficulty),
        },
        UserManagementMode::ToggleAdmin => UserManagementScreen {
            title: "Toggle Admin",
            subtitle: None,
            items: user_list_items(current_username, false),
        },
    }
}

pub fn handle_selection(
    mode: &UserManagementMode,
    selected_label: &str,
    current_username: Option<&str>,
) -> UserManagementAction {
    match mode {
        UserManagementMode::Root => match selected_label {
            "Create User" => UserManagementAction::OpenCreateUserPrompt,
            "Delete User" => UserManagementAction::SetMode {
                mode: UserManagementMode::DeleteUser,
                selected_idx: 0,
            },
            "Reset Password" => UserManagementAction::SetMode {
                mode: UserManagementMode::ResetPassword,
                selected_idx: 0,
            },
            "Change Auth Method" => UserManagementAction::SetMode {
                mode: UserManagementMode::ChangeAuthSelectUser,
                selected_idx: 0,
            },
            "Toggle Admin" => UserManagementAction::SetMode {
                mode: UserManagementMode::ToggleAdmin,
                selected_idx: 0,
            },
            "Back" => UserManagementAction::BackToSettings,
            _ => UserManagementAction::None,
        },
        UserManagementMode::CreateAuthMethod { username } => {
            if selected_label == "Back" {
                UserManagementAction::SetMode {
                    mode: UserManagementMode::Root,
                    selected_idx: 0,
                }
            } else if selected_label.starts_with("Hacking") {
                UserManagementAction::SetMode {
                    mode: UserManagementMode::CreateHackingDifficulty {
                        username: username.clone(),
                    },
                    selected_idx: 0,
                }
            } else if let Some(method) = auth_method_from_label(selected_label) {
                UserManagementAction::CreateWithMethod {
                    username: username.clone(),
                    method,
                }
            } else {
                UserManagementAction::None
            }
        }
        UserManagementMode::CreateHackingDifficulty { username } => {
            if is_hacking_difficulty_label(selected_label) {
                UserManagementAction::CycleHackingDifficulty
            } else if selected_label == "Apply" {
                UserManagementAction::ApplyCreateHacking {
                    username: username.clone(),
                }
            } else if selected_label == "Back" {
                UserManagementAction::SetMode {
                    mode: UserManagementMode::CreateAuthMethod {
                        username: username.clone(),
                    },
                    selected_idx: 0,
                }
            } else {
                UserManagementAction::None
            }
        }
        UserManagementMode::DeleteUser => {
            if selected_label == "Back" {
                UserManagementAction::SetMode {
                    mode: UserManagementMode::Root,
                    selected_idx: 0,
                }
            } else if current_username.is_some_and(|username| username == selected_label) {
                UserManagementAction::Status("Cannot delete yourself.".to_string())
            } else {
                UserManagementAction::ConfirmDeleteUser {
                    username: selected_label.to_string(),
                }
            }
        }
        UserManagementMode::ResetPassword => {
            if selected_label == "Back" {
                UserManagementAction::SetMode {
                    mode: UserManagementMode::Root,
                    selected_idx: 0,
                }
            } else {
                UserManagementAction::OpenResetPassword {
                    username: selected_label.to_string(),
                }
            }
        }
        UserManagementMode::ChangeAuthSelectUser => {
            if selected_label == "Back" {
                UserManagementAction::SetMode {
                    mode: UserManagementMode::Root,
                    selected_idx: 0,
                }
            } else {
                UserManagementAction::SetMode {
                    mode: UserManagementMode::ChangeAuthChoose {
                        username: selected_label.to_string(),
                    },
                    selected_idx: 0,
                }
            }
        }
        UserManagementMode::ChangeAuthChoose { username } => {
            if selected_label == "Back" {
                UserManagementAction::SetMode {
                    mode: UserManagementMode::ChangeAuthSelectUser,
                    selected_idx: 0,
                }
            } else if selected_label.starts_with("Hacking") {
                UserManagementAction::SetMode {
                    mode: UserManagementMode::ChangeAuthHackingDifficulty {
                        username: username.clone(),
                    },
                    selected_idx: 0,
                }
            } else if let Some(method) = auth_method_from_label(selected_label) {
                UserManagementAction::ChangeAuthWithMethod {
                    username: username.clone(),
                    method,
                }
            } else {
                UserManagementAction::None
            }
        }
        UserManagementMode::ChangeAuthHackingDifficulty { username } => {
            if is_hacking_difficulty_label(selected_label) {
                UserManagementAction::CycleHackingDifficulty
            } else if selected_label == "Apply" {
                UserManagementAction::ApplyChangeAuthHacking {
                    username: username.clone(),
                }
            } else if selected_label == "Back" {
                UserManagementAction::SetMode {
                    mode: UserManagementMode::ChangeAuthChoose {
                        username: username.clone(),
                    },
                    selected_idx: 0,
                }
            } else {
                UserManagementAction::None
            }
        }
        UserManagementMode::ToggleAdmin => {
            if selected_label == "Back" {
                UserManagementAction::SetMode {
                    mode: UserManagementMode::Root,
                    selected_idx: 0,
                }
            } else {
                UserManagementAction::ConfirmToggleAdmin {
                    username: selected_label.to_string(),
                }
            }
        }
    }
}

fn root_items() -> Vec<String> {
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

fn auth_method_items() -> Vec<String> {
    vec![
        "Password             — classic password login".to_string(),
        "No Password          — log in without a password".to_string(),
        "Hacking Minigame     — must hack in to log in".to_string(),
        "---".to_string(),
        "Back".to_string(),
    ]
}

fn hacking_difficulty_items(difficulty: HackingDifficulty) -> Vec<String> {
    vec![
        format!(
            "Difficulty: {} [cycle]",
            hacking_difficulty_label(difficulty)
        ),
        "Apply".to_string(),
        "---".to_string(),
        "Back".to_string(),
    ]
}

fn auth_method_from_label(label: &str) -> Option<AuthMethod> {
    if label.starts_with("Password") {
        Some(AuthMethod::Password)
    } else if label.starts_with("No Password") {
        Some(AuthMethod::NoPassword)
    } else if label.starts_with("Hacking") {
        Some(AuthMethod::HackingMinigame)
    } else {
        None
    }
}

fn is_hacking_difficulty_label(label: &str) -> bool {
    label.starts_with("Difficulty:")
}

fn user_list_items(current_username: Option<&str>, include_current: bool) -> Vec<String> {
    let mut users: Vec<String> = load_users()
        .keys()
        .filter(|u| include_current || Some(u.as_str()) != current_username)
        .cloned()
        .collect();
    users.sort();
    users.push("Back".to_string());
    users
}

#[cfg(test)]
mod tests {
    use super::super::menu::UserManagementMode;
    use super::*;
    use crate::config::HackingDifficulty;

    #[test]
    fn create_hacking_selection_routes_to_difficulty_mode() {
        let mode = UserManagementMode::CreateAuthMethod {
            username: "alice".to_string(),
        };
        let action = handle_selection(
            &mode,
            "Hacking Minigame     — must hack in to log in",
            Some("admin"),
        );
        assert!(matches!(
            action,
            UserManagementAction::SetMode {
                mode: UserManagementMode::CreateHackingDifficulty { .. },
                selected_idx: 0
            }
        ));
    }

    #[test]
    fn create_hacking_difficulty_rows_map_to_expected_actions() {
        let mode = UserManagementMode::CreateHackingDifficulty {
            username: "alice".to_string(),
        };
        let cycle = handle_selection(&mode, "Difficulty: Normal [cycle]", Some("admin"));
        assert_eq!(cycle, UserManagementAction::CycleHackingDifficulty);

        let apply = handle_selection(&mode, "Apply", Some("admin"));
        assert!(matches!(
            apply,
            UserManagementAction::ApplyCreateHacking { username } if username == "alice"
        ));
    }

    #[test]
    fn screen_for_hacking_difficulty_uses_current_label() {
        let mode = UserManagementMode::ChangeAuthHackingDifficulty {
            username: "bob".to_string(),
        };
        let screen = screen_for_mode(&mode, Some("admin"), HackingDifficulty::Hard);
        assert_eq!(screen.title, "Hacking Difficulty");
        assert!(screen
            .items
            .first()
            .is_some_and(|item| item.contains("Hard [cycle]")));
    }
}
