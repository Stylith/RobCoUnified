use crate::config::{base_dir, load_json, mark_default_apps_prompt_pending, save_json, users_dir};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum AuthMethod {
    #[default]
    Password,
    NoPassword,
    HackingMinigame,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UserRecord {
    pub password_hash: String,
    pub is_admin: bool,
    #[serde(default)]
    pub auth_method: AuthMethod,
}

pub type UsersDb = HashMap<String, UserRecord>;

fn users_db_path() -> PathBuf {
    users_dir().join("users.json")
}

fn session_file() -> PathBuf {
    base_dir().join(".session")
}

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

pub fn write_session(username: &str) {
    let _ = std::fs::write(session_file(), username);
}

pub fn read_session() -> Option<String> {
    std::fs::read_to_string(session_file())
        .ok()
        .map(|s| s.trim().to_string())
}

pub fn clear_session() {
    let _ = std::fs::remove_file(session_file());
}

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
