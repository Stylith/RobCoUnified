use crate::config::{
    get_settings, load_apps, persist_settings, set_current_user, update_settings, Settings,
};
use crate::core::auth::{hash_password, load_users, write_session, AuthMethod, UserRecord};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub fn authenticate(username: &str, password: &str) -> Result<UserRecord, &'static str> {
    let db = load_users();
    let Some(record) = db.get(username) else {
        return Err("Unknown user.");
    };
    match record.auth_method {
        AuthMethod::NoPassword => {
            set_current_user(Some(username));
            write_session(username);
            Ok(record.clone())
        }
        AuthMethod::HackingMinigame => {
            Err("Hacking login is not implemented in the native rewrite yet.")
        }
        AuthMethod::Password => {
            if record.password_hash == hash_password(password) {
                set_current_user(Some(username));
                write_session(username);
                Ok(record.clone())
            } else {
                Err("Wrong password.")
            }
        }
    }
}

pub fn current_settings() -> Settings {
    get_settings()
}

pub fn save_settings(settings: Settings) {
    update_settings(|s| *s = settings);
    persist_settings();
}

pub fn app_names() -> Vec<String> {
    let mut names: Vec<String> = load_apps().keys().cloned().collect();
    names.sort();
    names
}

pub fn home_dir_fallback() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
}

pub fn documents_dir() -> PathBuf {
    dirs::document_dir().unwrap_or_else(home_dir_fallback)
}

pub fn word_processor_dir(username: &str) -> PathBuf {
    let dir = documents_dir().join("ROBCO Word Processor").join(username);
    let _ = std::fs::create_dir_all(&dir);
    dir
}

pub fn save_text_file(path: &PathBuf, text: &str) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, text)?;
    Ok(())
}

pub fn read_text_file(path: &PathBuf) -> anyhow::Result<String> {
    Ok(std::fs::read_to_string(path)?)
}

pub fn write_shell_snapshot<T: Serialize>(username: &str, value: &T) {
    let path = crate::config::user_dir(username).join("native_shell.json");
    let _ = crate::config::save_json(&path, value);
}

pub fn read_shell_snapshot<T: for<'de> Deserialize<'de> + Default>(username: &str) -> T {
    let path = crate::config::user_dir(username).join("native_shell.json");
    crate::config::load_json(&path)
}
