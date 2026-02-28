use anyhow::Result;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::PathBuf;

use crate::config::{
    base_dir, cycle_hacking_difficulty, get_settings, hacking_difficulty_label, load_json,
    mark_default_apps_prompt_pending, persist_settings, save_json, update_settings, users_dir,
};
use crate::ui::{
    confirm, flash_message, input_prompt, is_back_menu_label, password_prompt, run_menu,
    MenuResult, Term,
};

// ── Auth method ───────────────────────────────────────────────────────────────

/// How a user authenticates at the login screen.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum AuthMethod {
    /// Standard SHA-256 hashed password (default).
    #[default]
    Password,
    /// No authentication required — user logs in immediately.
    NoPassword,
    /// User must complete the hacking minigame to log in.
    HackingMinigame,
}

// ── User record ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UserRecord {
    pub password_hash: String,
    pub is_admin: bool,
    #[serde(default)]
    pub auth_method: AuthMethod,
}

fn users_db_path() -> PathBuf {
    users_dir().join("users.json")
}
pub type UsersDb = HashMap<String, UserRecord>;

pub fn load_users() -> UsersDb {
    load_json(&users_db_path())
}
pub fn save_users(db: &UsersDb) {
    let _ = save_json(&users_db_path(), db);
}

pub fn hash_password(pw: &str) -> String {
    let mut h = Sha256::new();
    h.update(pw.as_bytes());
    hex::encode(h.finalize())
}

#[allow(dead_code)]
pub fn verify_password(username: &str, pw: &str) -> bool {
    let db = load_users();
    db.get(username)
        .map(|r| r.password_hash == hash_password(pw))
        .unwrap_or(false)
}

pub fn is_admin(username: &str) -> bool {
    load_users()
        .get(username)
        .map(|r| r.is_admin)
        .unwrap_or(false)
}

// ── Session file ──────────────────────────────────────────────────────────────

fn session_file() -> PathBuf {
    base_dir().join(".session")
}

pub fn write_session(username: &str) {
    let _ = std::fs::write(session_file(), username);
}

#[allow(dead_code)]
pub fn read_session() -> Option<String> {
    std::fs::read_to_string(session_file())
        .ok()
        .map(|s| s.trim().to_string())
}

pub fn clear_session() {
    let _ = std::fs::remove_file(session_file());
}

// ── Bootstrap: create default admin if no users exist ────────────────────────

pub fn ensure_default_admin() {
    let mut db = load_users();
    if db.is_empty() {
        db.insert(
            "admin".to_string(),
            UserRecord {
                password_hash: hash_password("admin"),
                is_admin: true,
                auth_method: AuthMethod::Password,
            },
        );
        save_users(&db);
        mark_default_apps_prompt_pending("admin");
    }
}

// ── Login screen ─────────────────────────────────────────────────────────────

pub fn login_screen(terminal: &mut Term) -> Result<Option<String>> {
    loop {
        let db = load_users();
        let mut usernames: Vec<String> = db.keys().cloned().collect();
        usernames.sort();

        let mut opts: Vec<String> = usernames.clone();
        opts.push("---".to_string());
        opts.push("Exit".to_string());
        let opts_str: Vec<&str> = opts.iter().map(String::as_str).collect();

        let result = run_menu(
            terminal,
            "ROBCO TERMLINK — Select User",
            &opts_str,
            Some("Welcome. Please select a user."),
        )?;

        match result {
            MenuResult::Selected(s) if s == "Exit" => return Ok(None),
            MenuResult::Back => return Ok(None),

            MenuResult::Selected(username) if db.contains_key(&username) => {
                let record = db[&username].clone();
                let authenticated = match record.auth_method {
                    AuthMethod::NoPassword => true,

                    AuthMethod::Password => {
                        let mut pw_auth = false;
                        let mut pw_attempts = 3u8;
                        loop {
                            let pw = match password_prompt(
                                terminal,
                                &format!("Enter password ({pw_attempts} attempt(s) left):"),
                            )? {
                                Some(p) => p,
                                None => break,
                            };
                            if record.password_hash == hash_password(&pw) {
                                pw_auth = true;
                                break;
                            }
                            pw_attempts = pw_attempts.saturating_sub(1);
                            crate::sound::play_error();
                            if pw_attempts == 0 {
                                crate::hacking::draw_terminal_locked(terminal)?;
                                break;
                            }
                        }
                        pw_auth
                    }

                    AuthMethod::HackingMinigame => {
                        let success = crate::hacking::run_hacking(terminal)?;
                        if !success {
                            crate::sound::play_error();
                            crate::hacking::draw_terminal_locked(terminal)?;
                        }
                        success
                    }
                };

                if authenticated {
                    crate::sound::play_login();
                    write_session(&username);
                    return Ok(Some(username));
                }
            }

            _ => {}
        }
    }
}

// ── User management (admin only) ──────────────────────────────────────────────

pub fn user_management_menu(terminal: &mut Term, current_user: &str) -> Result<()> {
    loop {
        let result = run_menu(
            terminal,
            "User Management",
            &[
                "Create User",
                "Delete User",
                "Reset Password",
                "Change Auth Method",
                "Toggle Admin",
                "---",
                "Back",
            ],
            None,
        )?;
        match result {
            MenuResult::Back => break,
            MenuResult::Selected(s) => match s.as_str() {
                s if is_back_menu_label(s) => break,
                "Create User" => create_user_dialog(terminal)?,
                "Delete User" => delete_user_dialog(terminal, current_user)?,
                "Reset Password" => reset_password_dialog(terminal)?,
                "Change Auth Method" => change_auth_method_dialog(terminal)?,
                "Toggle Admin" => toggle_admin_dialog(terminal, current_user)?,
                _ => {}
            },
        }
    }
    Ok(())
}

fn create_user_dialog(terminal: &mut Term) -> Result<()> {
    let username = match input_prompt(terminal, "New username:")? {
        Some(u) if !u.is_empty() => u,
        _ => return Ok(()),
    };
    let mut db = load_users();
    if db.contains_key(&username) {
        flash_message(terminal, "User already exists.", 800)?;
        return Ok(());
    }

    let Some(auth_method) = choose_auth_method(terminal)? else {
        return Ok(());
    };

    let password_hash = match auth_method {
        AuthMethod::Password => loop {
            let pw = match password_prompt(terminal, "Password:")? {
                Some(p) if !p.is_empty() => p,
                _ => return Ok(()),
            };
            let pw2 = match password_prompt(terminal, "Re-enter password to confirm:")? {
                Some(p) => p,
                _ => return Ok(()),
            };
            if pw == pw2 {
                break hash_password(&pw);
            }
            flash_message(terminal, "Passwords do not match. Try again.", 1000)?;
        },
        _ => String::new(),
    };

    db.insert(
        username.clone(),
        UserRecord {
            password_hash,
            is_admin: false,
            auth_method,
        },
    );
    save_users(&db);
    let _ = std::fs::create_dir_all(users_dir().join(&username));
    mark_default_apps_prompt_pending(&username);
    flash_message(terminal, &format!("User '{username}' created."), 800)
}

fn delete_user_dialog(terminal: &mut Term, current_user: &str) -> Result<()> {
    let db = load_users();
    let mut opts_str: Vec<String> = db.keys().cloned().collect();
    opts_str.sort();
    opts_str.push("Back".to_string());
    let opts: Vec<&str> = opts_str.iter().map(String::as_str).collect();
    let result = run_menu(terminal, "Delete User", &opts, None)?;
    if let MenuResult::Selected(u) = result {
        if u == current_user {
            flash_message(terminal, "Cannot delete yourself.", 800)?;
        } else if !is_back_menu_label(&u) && confirm(terminal, &format!("Delete user '{u}'?"))? {
            let mut db = load_users();
            db.remove(&u);
            save_users(&db);
            flash_message(terminal, &format!("User '{u}' deleted."), 800)?;
        }
    }
    Ok(())
}

fn reset_password_dialog(terminal: &mut Term) -> Result<()> {
    let db = load_users();
    let mut opts_str: Vec<String> = db.keys().cloned().collect();
    opts_str.sort();
    opts_str.push("Back".to_string());
    let opts: Vec<&str> = opts_str.iter().map(String::as_str).collect();
    if let MenuResult::Selected(u) = run_menu(terminal, "Reset Password", &opts, None)? {
        if !is_back_menu_label(&u) {
            let pw = match password_prompt(terminal, &format!("New password for '{u}':"))? {
                Some(p) if !p.is_empty() => p,
                _ => return Ok(()),
            };
            let pw2 = match password_prompt(terminal, "Re-enter password to confirm:")? {
                Some(p) => p,
                None => return Ok(()),
            };
            if pw == pw2 {
                let mut db = load_users();
                if let Some(r) = db.get_mut(&u) {
                    r.password_hash = hash_password(&pw);
                    r.auth_method = AuthMethod::Password;
                    save_users(&db);
                    flash_message(terminal, "Password updated.", 800)?;
                }
            } else {
                flash_message(terminal, "Passwords do not match.", 1000)?;
            }
        }
    }
    Ok(())
}

fn change_auth_method_dialog(terminal: &mut Term) -> Result<()> {
    let db = load_users();
    let mut user_opts: Vec<String> = db.keys().cloned().collect();
    user_opts.sort();
    user_opts.push("Back".to_string());
    let user_refs: Vec<&str> = user_opts.iter().map(String::as_str).collect();

    let username = match run_menu(
        terminal,
        "Change Auth Method — Select User",
        &user_refs,
        None,
    )? {
        MenuResult::Selected(u) if !is_back_menu_label(&u) => u,
        _ => return Ok(()),
    };

    let Some(new_method) = choose_auth_method(terminal)? else {
        return Ok(());
    };

    let new_hash = match new_method {
        AuthMethod::Password => loop {
            let pw = match password_prompt(terminal, &format!("New password for '{username}':"))? {
                Some(p) if !p.is_empty() => p,
                _ => return Ok(()),
            };
            let pw2 = match password_prompt(terminal, "Re-enter password to confirm:")? {
                Some(p) => p,
                None => return Ok(()),
            };
            if pw == pw2 {
                break hash_password(&pw);
            }
            flash_message(terminal, "Passwords do not match. Try again.", 1000)?;
        },
        _ => String::new(),
    };

    let mut db = load_users();
    if let Some(r) = db.get_mut(&username) {
        r.auth_method = new_method;
        r.password_hash = new_hash;
        save_users(&db);
        flash_message(
            terminal,
            &format!("Auth method updated for '{username}'."),
            800,
        )?;
    }
    Ok(())
}

fn auth_method_choice_from_label(label: &str) -> Option<AuthMethod> {
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

fn choose_hacking_difficulty(terminal: &mut Term) -> Result<bool> {
    loop {
        let current = get_settings().hacking_difficulty;
        let rows = vec![
            format!("Difficulty: {} [cycle]", hacking_difficulty_label(current)),
            "Apply".to_string(),
            "---".to_string(),
            "Back".to_string(),
        ];
        let refs: Vec<&str> = rows.iter().map(String::as_str).collect();
        match run_menu(
            terminal,
            "Hacking Difficulty",
            &refs,
            Some("Only used for Hacking Minigame authentication."),
        )? {
            MenuResult::Back => return Ok(false),
            MenuResult::Selected(sel) if sel == rows[0] => {
                update_settings(|s| {
                    s.hacking_difficulty = cycle_hacking_difficulty(s.hacking_difficulty, true);
                });
                persist_settings();
            }
            MenuResult::Selected(sel) if sel == "Apply" => return Ok(true),
            MenuResult::Selected(sel) if is_back_menu_label(&sel) => return Ok(false),
            _ => {}
        }
    }
}

fn choose_auth_method(terminal: &mut Term) -> Result<Option<AuthMethod>> {
    let result = run_menu(
        terminal,
        "Choose Authentication Method",
        &[
            "Password             — classic password login",
            "No Password          — log in without a password",
            "Hacking Minigame     — must hack in to log in",
            "---",
            "Back",
        ],
        Some("Select how this user will authenticate at login."),
    )?;

    Ok(match result {
        MenuResult::Selected(s) => {
            let Some(method) = auth_method_choice_from_label(&s) else {
                return Ok(None);
            };
            if matches!(method, AuthMethod::HackingMinigame)
                && !choose_hacking_difficulty(terminal)?
            {
                None
            } else {
                Some(method)
            }
        }
        MenuResult::Back => None,
    })
}

fn toggle_admin_dialog(terminal: &mut Term, current_user: &str) -> Result<()> {
    let db = load_users();
    let mut opts_str: Vec<String> = db.keys().filter(|u| *u != current_user).cloned().collect();
    opts_str.sort();
    opts_str.push("Back".to_string());
    let opts: Vec<&str> = opts_str.iter().map(String::as_str).collect();
    if let MenuResult::Selected(u) = run_menu(terminal, "Toggle Admin", &opts, None)? {
        if !is_back_menu_label(&u) {
            let mut db = load_users();
            if let Some(r) = db.get_mut(&u) {
                r.is_admin = !r.is_admin;
                let label = if r.is_admin { "granted" } else { "revoked" };
                save_users(&db);
                flash_message(terminal, &format!("Admin {label} for '{u}'."), 800)?;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auth_method_choice_maps_known_labels() {
        assert_eq!(
            auth_method_choice_from_label("Password             — classic password login"),
            Some(AuthMethod::Password)
        );
        assert_eq!(
            auth_method_choice_from_label("No Password          — log in without a password"),
            Some(AuthMethod::NoPassword)
        );
        assert_eq!(
            auth_method_choice_from_label("Hacking Minigame     — must hack in to log in"),
            Some(AuthMethod::HackingMinigame)
        );
    }

    #[test]
    fn auth_method_choice_rejects_back() {
        assert_eq!(auth_method_choice_from_label("Back"), None);
        assert!(is_back_menu_label("Back"));
    }
}
