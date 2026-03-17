use crate::config::mark_default_apps_prompt_pending;
use crate::core::auth::{hash_password, load_users, save_users, AuthMethod, UserRecord};

pub fn user_auth_method_label(auth_method: &AuthMethod) -> &'static str {
    match auth_method {
        AuthMethod::Password => "Password",
        AuthMethod::NoPassword => "No Password",
        AuthMethod::HackingMinigame => "Hacking Minigame",
    }
}

pub fn sorted_user_records() -> Vec<(String, UserRecord)> {
    let mut users: Vec<(String, UserRecord)> = load_users().into_iter().collect();
    users.sort_by(|a, b| a.0.cmp(&b.0));
    users
}

pub fn sorted_usernames() -> Vec<String> {
    sorted_user_records()
        .into_iter()
        .map(|(name, _)| name)
        .collect()
}

pub fn user_exists(username: &str) -> bool {
    load_users().contains_key(username)
}

pub fn create_user(
    username: &str,
    auth_method: AuthMethod,
    password: Option<&str>,
) -> Result<String, String> {
    let mut db = load_users();
    let status = create_user_in_db(&mut db, username, auth_method, password)?;
    save_users(&db);
    Ok(status)
}

pub fn update_user_auth_method(
    username: &str,
    auth_method: AuthMethod,
    password: Option<&str>,
) -> Result<String, String> {
    let mut db = load_users();
    let status = update_user_auth_method_in_db(&mut db, username, auth_method, password)?;
    save_users(&db);
    Ok(status)
}

pub fn delete_user(username: &str) -> Result<String, String> {
    let mut db = load_users();
    if db.remove(username).is_none() {
        return Err(format!("Unknown user '{username}'."));
    }
    save_users(&db);
    Ok(format!("User '{username}' deleted."))
}

pub fn toggle_user_admin(username: &str) -> Result<String, String> {
    let mut db = load_users();
    let status = toggle_user_admin_in_db(&mut db, username)?;
    save_users(&db);
    Ok(status)
}

fn create_user_in_db(
    db: &mut std::collections::HashMap<String, UserRecord>,
    username: &str,
    auth_method: AuthMethod,
    password: Option<&str>,
) -> Result<String, String> {
    let username = username.trim();
    if username.is_empty() {
        return Err("Username cannot be empty.".to_string());
    }

    if db.contains_key(username) {
        return Err("User already exists.".to_string());
    }

    db.insert(
        username.to_string(),
        build_user_record(auth_method, password)?,
    );
    mark_default_apps_prompt_pending(username);
    Ok(format!("User '{username}' created."))
}

fn update_user_auth_method_in_db(
    db: &mut std::collections::HashMap<String, UserRecord>,
    username: &str,
    auth_method: AuthMethod,
    password: Option<&str>,
) -> Result<String, String> {
    let Some(record) = db.get_mut(username) else {
        return Err(format!("Unknown user '{username}'."));
    };

    let updated = build_user_record(auth_method, password)?;
    record.password_hash = updated.password_hash;
    record.auth_method = updated.auth_method;
    Ok(format!("Auth method updated for '{username}'."))
}

fn toggle_user_admin_in_db(
    db: &mut std::collections::HashMap<String, UserRecord>,
    username: &str,
) -> Result<String, String> {
    let Some(record) = db.get_mut(username) else {
        return Err(format!("Unknown user '{username}'."));
    };
    record.is_admin = !record.is_admin;
    let label = if record.is_admin {
        "granted"
    } else {
        "revoked"
    };
    Ok(format!("Admin {label} for '{username}'."))
}

fn build_user_record(
    auth_method: AuthMethod,
    password: Option<&str>,
) -> Result<UserRecord, String> {
    let password_hash = match auth_method {
        AuthMethod::Password => {
            let Some(password) = password.filter(|password| !password.is_empty()) else {
                return Err("Password cannot be empty.".to_string());
            };
            hash_password(password)
        }
        AuthMethod::NoPassword | AuthMethod::HackingMinigame => String::new(),
    };

    Ok(UserRecord {
        password_hash,
        is_admin: false,
        auth_method,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn create_user_rejects_duplicate_names() {
        let mut users = HashMap::new();
        users.insert(
            "alice".to_string(),
            UserRecord {
                password_hash: String::new(),
                is_admin: false,
                auth_method: AuthMethod::NoPassword,
            },
        );

        let err = create_user_in_db(&mut users, "alice", AuthMethod::NoPassword, None)
            .expect_err("duplicate user");
        assert_eq!(err, "User already exists.");
    }

    #[test]
    fn update_user_auth_method_switches_to_password_login() {
        let mut users = HashMap::new();
        users.insert(
            "alice".to_string(),
            UserRecord {
                password_hash: String::new(),
                is_admin: false,
                auth_method: AuthMethod::NoPassword,
            },
        );

        let status = update_user_auth_method_in_db(
            &mut users,
            "alice",
            AuthMethod::Password,
            Some("secret"),
        )
        .expect("update");
        assert_eq!(status, "Auth method updated for 'alice'.");

        let record = users.get("alice").expect("alice record");
        assert_eq!(record.auth_method, AuthMethod::Password);
        assert_eq!(record.password_hash, hash_password("secret"));
    }

    #[test]
    fn toggle_user_admin_reports_new_state() {
        let mut users = HashMap::new();
        users.insert(
            "alice".to_string(),
            UserRecord {
                password_hash: String::new(),
                is_admin: false,
                auth_method: AuthMethod::NoPassword,
            },
        );

        let status = toggle_user_admin_in_db(&mut users, "alice").expect("toggle admin");
        assert_eq!(status, "Admin granted for 'alice'.");
        assert!(users.get("alice").is_some_and(|record| record.is_admin));
    }
}
