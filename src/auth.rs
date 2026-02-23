use anyhow::Result;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::PathBuf;

use crate::config::{base_dir, users_dir, load_json, save_json};
use crate::ui::{Term, run_menu, input_prompt, flash_message, confirm, MenuResult};

// ── User record ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UserRecord {
    pub password_hash: String,
    pub is_admin: bool,
}

fn users_db_path() -> PathBuf { users_dir().join("users.json") }
pub type UsersDb = HashMap<String, UserRecord>;

pub fn load_users() -> UsersDb { load_json(&users_db_path()) }
pub fn save_users(db: &UsersDb) { let _ = save_json(&users_db_path(), db); }

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
    load_users().get(username).map(|r| r.is_admin).unwrap_or(false)
}

// ── Session file ──────────────────────────────────────────────────────────────

fn session_file() -> PathBuf { base_dir().join(".session") }

pub fn write_session(username: &str) {
    let _ = std::fs::write(session_file(), username);
}

#[allow(dead_code)]
pub fn read_session() -> Option<String> {
    std::fs::read_to_string(session_file()).ok().map(|s| s.trim().to_string())
}

pub fn clear_session() {
    let _ = std::fs::remove_file(session_file());
}

// ── Bootstrap: create default admin if no users exist ────────────────────────

pub fn ensure_default_admin() {
    let mut db = load_users();
    if db.is_empty() {
        db.insert("admin".to_string(), UserRecord {
            password_hash: hash_password("admin"),
            is_admin: true,
        });
        save_users(&db);
    }
}

// ── Login screen ─────────────────────────────────────────────────────────────

/// Returns the logged-in username, or `None` if the user chose Exit.
pub fn login_screen(terminal: &mut Term) -> Result<Option<String>> {
    loop {
        let result = run_menu(
            terminal,
            "ROBCO TERMLINK — Login",
            &["Login", "---", "Exit"],
            Some("Welcome. Please authenticate."),
        )?;

        match result {
            MenuResult::Selected(s) if s == "Exit" => return Ok(None),
            MenuResult::Selected(s) if s == "Login" => {
                let username = match input_prompt(terminal, "Enter username:")? {
                    Some(u) if !u.is_empty() => u,
                    _ => { flash_message(terminal, "Username cannot be empty.", 800)?; continue; }
                };
                let password = match input_prompt(terminal, "Enter password:")? {
                    Some(p) => p,
                    _ => continue,
                };

                let db = load_users();
                if let Some(record) = db.get(&username) {
                    if record.password_hash == hash_password(&password) {
                        write_session(&username);
                        return Ok(Some(username));
                    }
                }
                flash_message(terminal, "Access denied. Invalid credentials.", 1200)?;
            }
            MenuResult::Back | MenuResult::Selected(_) => continue,
        }
    }
}

// ── User management (admin only) ──────────────────────────────────────────────

pub fn user_management_menu(terminal: &mut Term, current_user: &str) -> Result<()> {
    loop {
        let result = run_menu(
            terminal,
            "User Management",
            &["Create User", "Delete User", "Reset Password", "Toggle Admin", "---", "Back"],
            None,
        )?;
        match result {
            MenuResult::Back => break,
            MenuResult::Selected(s) => match s.as_str() {
                "Create User" => create_user_dialog(terminal)?,
                "Delete User"  => delete_user_dialog(terminal, current_user)?,
                "Reset Password" => reset_password_dialog(terminal)?,
                "Toggle Admin" => toggle_admin_dialog(terminal, current_user)?,
                _ => {}
            }
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
    let pw = match input_prompt(terminal, "Password:")? {
        Some(p) if !p.is_empty() => p,
        _ => return Ok(()),
    };
    db.insert(username.clone(), UserRecord {
        password_hash: hash_password(&pw),
        is_admin: false,
    });
    save_users(&db);
    // Create user directory
    let _ = std::fs::create_dir_all(users_dir().join(&username));
    flash_message(terminal, &format!("User '{username}' created."), 800)
}

fn delete_user_dialog(terminal: &mut Term, current_user: &str) -> Result<()> {
    let db = load_users();
    let users: Vec<&str> = db.keys().map(String::as_str).collect();
    let mut opts: Vec<&str> = users.clone();
    opts.push("Back");
    let result = run_menu(terminal, "Delete User", &opts, None)?;
    if let MenuResult::Selected(u) = result {
        if u == current_user {
            flash_message(terminal, "Cannot delete yourself.", 800)?;
        } else if confirm(terminal, &format!("Delete user '{u}'?"))? {
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
    let users: Vec<String> = db.keys().cloned().collect();
    let opts_str: Vec<String> = users.iter().cloned().chain(["Back".to_string()]).collect();
    let opts: Vec<&str> = opts_str.iter().map(String::as_str).collect();
    if let MenuResult::Selected(u) = run_menu(terminal, "Reset Password", &opts, None)? {
        if u != "Back" {
            if let Some(pw) = input_prompt(terminal, &format!("New password for '{u}':"))? {
                let mut db = load_users();
                if let Some(r) = db.get_mut(&u) {
                    r.password_hash = hash_password(&pw);
                    save_users(&db);
                    flash_message(terminal, "Password updated.", 800)?;
                }
            }
        }
    }
    Ok(())
}

fn toggle_admin_dialog(terminal: &mut Term, current_user: &str) -> Result<()> {
    let db = load_users();
    let opts_str: Vec<String> = db.keys().filter(|u| *u != current_user)
        .cloned().chain(["Back".to_string()]).collect();
    let opts: Vec<&str> = opts_str.iter().map(String::as_str).collect();
    if let MenuResult::Selected(u) = run_menu(terminal, "Toggle Admin", &opts, None)? {
        if u != "Back" {
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
