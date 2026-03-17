use super::desktop_user_service::{sorted_usernames, user_exists};
use super::shared_types::FlashAction;
use crate::config::{load_json, save_json, set_current_user, user_dir, OpenMode};
use crate::core::auth::{
    ensure_default_admin, hash_password, load_users, read_session, write_session, AuthMethod,
    UserRecord,
};
use crate::session;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

fn authenticate(username: &str, password: &str) -> Result<UserRecord, &'static str> {
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
        AuthMethod::HackingMinigame => Err("Use the hacking minigame flow from the login menu."),
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

fn bind_login_session(username: &str) {
    set_current_user(Some(username));
    write_session(username);
}

fn home_dir_fallback() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
}

fn documents_dir() -> PathBuf {
    dirs::document_dir().unwrap_or_else(home_dir_fallback)
}

fn word_processor_dir(username: &str) -> PathBuf {
    let dir = documents_dir().join("ROBCO Word Processor").join(username);
    let _ = std::fs::create_dir_all(&dir);
    dir
}

fn write_shell_snapshot<T: Serialize>(username: &str, value: &T) {
    let path = user_dir(username).join("native_shell.json");
    let _ = save_json(&path, value);
}

fn read_shell_snapshot<T: for<'de> Deserialize<'de> + Default>(username: &str) -> T {
    let path = user_dir(username).join("native_shell.json");
    load_json(&path)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct NativeShellSnapshot {
    file_manager_dir: PathBuf,
    editor_path: Option<PathBuf>,
}

impl Default for NativeShellSnapshot {
    fn default() -> Self {
        Self {
            file_manager_dir: PathBuf::new(),
            editor_path: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeSessionIdentity {
    pub username: String,
    pub is_admin: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativePendingSessionSwitch {
    AlreadyActive,
    ActivateExisting { target: usize },
    OpenNew { username: String, new_index: usize },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeClosedSessionOutcome {
    pub removed_idx: usize,
    pub active_identity: Option<NativeSessionIdentity>,
}

#[derive(Debug, Clone)]
pub struct NativeSessionFlashPlan {
    pub message: String,
    pub duration_ms: u64,
    pub action: FlashAction,
    pub boxed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeSessionRestorePlan {
    pub identity: NativeSessionIdentity,
    pub file_manager_dir: PathBuf,
    pub launch_default_desktop: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeSessionTabs {
    pub active: usize,
    pub labels: Vec<String>,
}

pub fn restore_current_user_from_last_session() {
    ensure_default_admin();
    if let Some(last_user) = last_session_username() {
        set_current_user(Some(&last_user));
    }
}

pub fn last_session_username() -> Option<String> {
    read_session().filter(|username| user_exists(username))
}

pub fn login_usernames() -> Vec<String> {
    ensure_default_admin();
    sorted_usernames()
}

pub fn authenticate_login(username: &str, password: &str) -> Result<UserRecord, String> {
    authenticate(username, password).map_err(|err| err.to_string())
}

pub fn restore_session_plan(
    username: &str,
    user: &UserRecord,
    default_open_mode: OpenMode,
) -> NativeSessionRestorePlan {
    let snapshot: NativeShellSnapshot = read_shell_snapshot(username);
    let fallback_dir = word_processor_dir(username);
    let file_manager_dir = if snapshot.file_manager_dir.exists() {
        snapshot.file_manager_dir
    } else {
        fallback_dir
    };
    let launch_default_desktop = matches!(default_open_mode, OpenMode::Desktop)
        && session::take_default_mode_pending_for_active();

    NativeSessionRestorePlan {
        identity: NativeSessionIdentity {
            username: username.to_string(),
            is_admin: user.is_admin,
        },
        file_manager_dir,
        launch_default_desktop,
    }
}

pub fn persist_shell_snapshot(username: &str, file_manager_dir: &Path, editor_path: Option<&Path>) {
    write_shell_snapshot(
        username,
        &NativeShellSnapshot {
            file_manager_dir: file_manager_dir.to_path_buf(),
            editor_path: editor_path.map(Path::to_path_buf),
        },
    );
}

pub fn bind_login_identity(username: &str) {
    bind_login_session(username);
}

pub fn login_selection_auth_method(username: &str) -> Result<AuthMethod, String> {
    load_users()
        .get(username)
        .map(|record| record.auth_method.clone())
        .ok_or_else(|| "Unknown user.".to_string())
}

pub fn user_record(username: &str) -> Option<UserRecord> {
    load_users().get(username).cloned()
}

pub fn ensure_login_session_entry(username: &str) -> usize {
    let existing = session::get_sessions()
        .iter()
        .position(|entry| entry.username == username);
    let idx = existing.unwrap_or_else(|| session::push_session(username));
    session::set_active(idx);
    idx
}

pub fn request_session_switch(target: usize) -> bool {
    if !session_switch_target_is_valid(target) {
        return false;
    }
    session::request_switch(target);
    true
}

pub fn has_pending_session_switch() -> bool {
    session::has_switch_request()
}

pub fn session_count() -> usize {
    session::session_count()
}

pub fn active_session_index() -> Option<usize> {
    let count = session::session_count();
    if count == 0 {
        None
    } else {
        Some(session::active_idx())
    }
}

pub fn active_session_username() -> Option<String> {
    session::active_username()
}

pub fn session_tabs() -> NativeSessionTabs {
    let labels = session::get_sessions()
        .into_iter()
        .enumerate()
        .map(|(idx, _)| {
            format!(
                "[{}{}]",
                idx + 1,
                if idx == session::active_idx() {
                    "*"
                } else {
                    ""
                }
            )
        })
        .collect();
    NativeSessionTabs {
        active: session::active_idx(),
        labels,
    }
}

pub fn session_switch_target_is_valid(target: usize) -> bool {
    let count = session::session_count();
    target < count || (target == count && count < session::MAX_SESSIONS)
}

pub fn take_pending_session_switch() -> Option<NativePendingSessionSwitch> {
    let target = session::take_switch_request()?;
    let count = session::session_count();
    if target < count {
        let current = session::active_idx();
        if target == current {
            return Some(NativePendingSessionSwitch::AlreadyActive);
        }
        return Some(NativePendingSessionSwitch::ActivateExisting { target });
    }
    if target == count && count < session::MAX_SESSIONS {
        let username = session::active_username()?;
        return Some(NativePendingSessionSwitch::OpenNew {
            username,
            new_index: count,
        });
    }
    None
}

pub fn apply_session_switch(
    plan: &NativePendingSessionSwitch,
) -> Result<Option<NativeSessionIdentity>, String> {
    match plan {
        NativePendingSessionSwitch::AlreadyActive => Ok(None),
        NativePendingSessionSwitch::ActivateExisting { target } => {
            session::set_active(*target);
            active_session_identity()
        }
        NativePendingSessionSwitch::OpenNew { username, .. } => {
            let idx = session::push_session_with_default_mode(username, false);
            session::set_active(idx);
            active_session_identity()
        }
    }
}

pub fn close_active_session() -> Result<Option<NativeClosedSessionOutcome>, String> {
    let count = session::session_count();
    if count == 0 {
        return Ok(None);
    }
    if count <= 1 {
        return Err("Cannot close the last session.".to_string());
    }
    let Some(removed_idx) = session::close_active_session() else {
        return Ok(None);
    };
    Ok(Some(NativeClosedSessionOutcome {
        removed_idx,
        active_identity: active_session_identity()?,
    }))
}

pub fn clear_all_sessions() {
    session::clear_sessions();
    session::take_switch_request();
}

pub fn login_flash_plan(username: String, user: UserRecord) -> NativeSessionFlashPlan {
    NativeSessionFlashPlan {
        message: "Logging in...".to_string(),
        duration_ms: 700,
        action: FlashAction::FinishLogin { username, user },
        boxed: false,
    }
}

pub fn hacking_start_flash_plan(username: String) -> NativeSessionFlashPlan {
    NativeSessionFlashPlan {
        message: "SECURITY OVERRIDE".to_string(),
        duration_ms: 1200,
        action: FlashAction::StartHacking { username },
        boxed: false,
    }
}

pub fn logout_flash_plan(already_logging_out: bool) -> Option<NativeSessionFlashPlan> {
    if already_logging_out {
        return None;
    }
    Some(NativeSessionFlashPlan {
        message: "Logging out...".to_string(),
        duration_ms: 800,
        action: FlashAction::FinishLogout,
        boxed: false,
    })
}

pub fn session_identity_for_username(username: &str) -> Result<NativeSessionIdentity, String> {
    let users = load_users();
    let Some(user) = users.get(username) else {
        return Err(format!("Unknown user '{username}'."));
    };
    bind_login_session(username);
    Ok(NativeSessionIdentity {
        username: username.to_string(),
        is_admin: user.is_admin,
    })
}

pub fn active_session_identity() -> Result<Option<NativeSessionIdentity>, String> {
    let Some(username) = session::active_username() else {
        return Ok(None);
    };
    session_identity_for_username(&username).map(Some)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::auth::{save_users, UserRecord};
    use std::collections::HashMap;
    use std::sync::{Mutex, OnceLock};

    fn session_test_guard() -> std::sync::MutexGuard<'static, ()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
            .lock()
            .expect("desktop session service test lock")
    }

    struct UsersRestore {
        backup: HashMap<String, UserRecord>,
    }

    impl UsersRestore {
        fn capture() -> Self {
            Self {
                backup: load_users(),
            }
        }
    }

    impl Drop for UsersRestore {
        fn drop(&mut self) {
            save_users(&self.backup);
        }
    }

    #[test]
    fn login_selection_auth_method_reads_saved_method() {
        let _guard = session_test_guard();
        let _restore = UsersRestore::capture();
        let mut users = HashMap::new();
        users.insert(
            "alice".to_string(),
            UserRecord {
                password_hash: String::new(),
                is_admin: false,
                auth_method: AuthMethod::HackingMinigame,
            },
        );
        save_users(&users);

        let auth_method = login_selection_auth_method("alice").expect("auth method");
        assert_eq!(auth_method, AuthMethod::HackingMinigame);
    }

    #[test]
    fn ensure_login_session_entry_reuses_existing_session() {
        let _guard = session_test_guard();
        session::clear_sessions();

        let first = ensure_login_session_entry("alice");
        let second = ensure_login_session_entry("alice");

        assert_eq!(first, second);
        assert_eq!(session::session_count(), 1);
        assert_eq!(session::active_username().as_deref(), Some("alice"));
    }

    #[test]
    fn take_pending_session_switch_reports_new_session_request() {
        let _guard = session_test_guard();
        session::clear_sessions();
        ensure_login_session_entry("alice");
        session::request_switch(1);

        let switch = take_pending_session_switch().expect("pending switch");
        assert_eq!(
            switch,
            NativePendingSessionSwitch::OpenNew {
                username: "alice".to_string(),
                new_index: 1,
            }
        );
    }

    #[test]
    fn close_active_session_returns_previous_identity() {
        let _guard = session_test_guard();
        let _restore = UsersRestore::capture();
        let mut users = HashMap::new();
        users.insert(
            "alice".to_string(),
            UserRecord {
                password_hash: String::new(),
                is_admin: false,
                auth_method: AuthMethod::NoPassword,
            },
        );
        users.insert(
            "bob".to_string(),
            UserRecord {
                password_hash: String::new(),
                is_admin: true,
                auth_method: AuthMethod::NoPassword,
            },
        );
        save_users(&users);

        session::clear_sessions();
        session::push_session("alice");
        session::push_session("bob");
        session::set_active(1);

        let outcome = close_active_session()
            .expect("close session result")
            .expect("closed session");
        assert_eq!(outcome.removed_idx, 1);
        assert_eq!(
            outcome.active_identity,
            Some(NativeSessionIdentity {
                username: "alice".to_string(),
                is_admin: false,
            })
        );
    }

    #[test]
    fn login_flash_plan_queues_finish_login_action() {
        let plan = login_flash_plan(
            "alice".to_string(),
            UserRecord {
                password_hash: String::new(),
                is_admin: true,
                auth_method: AuthMethod::NoPassword,
            },
        );
        assert_eq!(plan.message, "Logging in...");
        assert_eq!(plan.duration_ms, 700);
        assert!(!plan.boxed);
        assert!(matches!(
            plan.action,
            FlashAction::FinishLogin { username, .. } if username == "alice"
        ));
    }

    #[test]
    fn logout_flash_plan_skips_when_already_logging_out() {
        assert!(logout_flash_plan(true).is_none());
        assert!(matches!(
            logout_flash_plan(false).map(|plan| plan.action),
            Some(FlashAction::FinishLogout)
        ));
    }

    #[test]
    fn restore_session_plan_falls_back_and_consumes_default_mode_once() {
        let _guard = session_test_guard();
        session::clear_sessions();
        session::take_switch_request();

        let username = "restore-plan-user";
        let missing_dir = PathBuf::from("/tmp/robco-native-restore-plan-missing");
        persist_shell_snapshot(username, &missing_dir, None);

        let idx = session::push_session_with_default_mode(username, true);
        session::set_active(idx);

        let user = UserRecord {
            password_hash: String::new(),
            is_admin: true,
            auth_method: AuthMethod::NoPassword,
        };

        let first = restore_session_plan(username, &user, OpenMode::Desktop);
        let second = restore_session_plan(username, &user, OpenMode::Desktop);

        assert_eq!(
            first,
            NativeSessionRestorePlan {
                identity: NativeSessionIdentity {
                    username: username.to_string(),
                    is_admin: true,
                },
                file_manager_dir: word_processor_dir(username),
                launch_default_desktop: true,
            }
        );
        assert!(!second.launch_default_desktop);
    }

    #[test]
    fn session_tabs_formats_active_marker() {
        let _guard = session_test_guard();
        session::clear_sessions();
        let first = session::push_session("alice");
        let _second = session::push_session("bob");
        session::set_active(first);

        assert_eq!(
            session_tabs(),
            NativeSessionTabs {
                active: 0,
                labels: vec!["[1*]".to_string(), "[2]".to_string()],
            }
        );
    }
}
