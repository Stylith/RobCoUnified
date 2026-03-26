#![allow(dead_code)]

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;

pub const NUCLEON_NATIVE_STANDALONE_USER_ENV: &str = "NUCLEON_NATIVE_STANDALONE_USER";
pub const LEGACY_ROBCOS_NATIVE_STANDALONE_USER_ENV: &str = "ROBCOS_NATIVE_STANDALONE_USER";
pub const NUCLEON_NATIVE_IPC_SOCKET_ENV: &str = "NUCLEON_NATIVE_IPC_SOCKET";
pub const LEGACY_ROBCOS_NATIVE_IPC_SOCKET_ENV: &str = "ROBCOS_NATIVE_IPC_SOCKET";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StandaloneNativeApp {
    FileManager,
    Settings,
    Editor,
}

impl StandaloneNativeApp {
    pub const fn preferred_binary_stem(self) -> &'static str {
        match self {
            Self::FileManager => "nucleon-files",
            Self::Settings => "nucleon-settings",
            Self::Editor => "nucleon-text",
        }
    }

    pub const fn legacy_binary_stem(self) -> &'static str {
        match self {
            Self::FileManager => "robcos-file-manager",
            Self::Settings => "robcos-settings",
            Self::Editor => "robcos-editor",
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::FileManager => "file manager",
            Self::Settings => "settings",
            Self::Editor => "editor",
        }
    }
}

pub fn launch_standalone_app(
    app: StandaloneNativeApp,
    args: &[OsString],
    session_username: Option<&str>,
) -> Result<(), String> {
    let binary = resolve_binary_path(app)?;
    let mut command = Command::new(binary);
    command.args(args);
    if let Some(username) = session_username.filter(|username| !username.is_empty()) {
        command.env(NUCLEON_NATIVE_STANDALONE_USER_ENV, username);
        command.env(LEGACY_ROBCOS_NATIVE_STANDALONE_USER_ENV, username);
    }
    command.env(
        NUCLEON_NATIVE_IPC_SOCKET_ENV,
        super::ipc::socket_path().as_os_str(),
    );
    command.env(
        LEGACY_ROBCOS_NATIVE_IPC_SOCKET_ENV,
        super::ipc::socket_path().as_os_str(),
    );
    command
        .spawn()
        .map(|_| ())
        .map_err(|err| format!("Could not open {}: {err}", app.label()))
}

fn resolve_binary_path(app: StandaloneNativeApp) -> Result<PathBuf, String> {
    bundled_binary_path(app)
        .or_else(|| sibling_binary_path(app))
        .ok_or_else(|| {
            format!(
                "Could not find the {} app in `{}` or beside the current executable.",
                app.label(),
                crate::config::bundled_bin_dir().display()
            )
        })
}

fn bundled_binary_path(app: StandaloneNativeApp) -> Option<PathBuf> {
    [app.preferred_binary_stem(), app.legacy_binary_stem()]
        .into_iter()
        .map(|stem| crate::config::bundled_binary_path(platform_binary_file_name(stem)))
        .find(|candidate| candidate.is_file())
}

fn sibling_binary_path(app: StandaloneNativeApp) -> Option<PathBuf> {
    let current_exe = std::env::current_exe().ok()?;
    sibling_binary_dirs(&current_exe)
        .into_iter()
        .flat_map(|dir| {
            [app.preferred_binary_stem(), app.legacy_binary_stem()]
                .into_iter()
                .map(move |stem| dir.join(platform_binary_file_name(stem)))
        })
        .find(|candidate| candidate.is_file())
}

fn sibling_binary_dirs(current_exe: &Path) -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Some(parent) = current_exe.parent() {
        dirs.push(parent.to_path_buf());
        if parent.file_name().and_then(|name| name.to_str()) == Some("deps") {
            if let Some(grandparent) = parent.parent() {
                dirs.push(grandparent.to_path_buf());
            }
        }
    }
    dirs
}

fn platform_binary_file_name(binary_stem: &str) -> OsString {
    #[cfg(target_os = "windows")]
    {
        return OsString::from(format!("{binary_stem}.exe"));
    }

    OsString::from(binary_stem)
}

pub fn standalone_env_value() -> Option<String> {
    [NUCLEON_NATIVE_STANDALONE_USER_ENV, LEGACY_ROBCOS_NATIVE_STANDALONE_USER_ENV]
        .into_iter()
        .find_map(|name| {
            std::env::var(name)
                .ok()
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Mutex, OnceLock};

    fn env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    #[test]
    fn sibling_binary_file_name_matches_current_platform_convention() {
        let name = platform_binary_file_name("nucleon-settings");

        #[cfg(target_os = "windows")]
        assert_eq!(name, OsString::from("nucleon-settings.exe"));
        #[cfg(not(target_os = "windows"))]
        assert_eq!(name, OsString::from("nucleon-settings"));
    }

    #[test]
    fn standalone_apps_prefer_nucleon_binary_names() {
        assert_eq!(
            StandaloneNativeApp::FileManager.preferred_binary_stem(),
            "nucleon-files"
        );
        assert_eq!(
            StandaloneNativeApp::Settings.preferred_binary_stem(),
            "nucleon-settings"
        );
        assert_eq!(
            StandaloneNativeApp::Editor.preferred_binary_stem(),
            "nucleon-text"
        );
    }

    #[test]
    fn standalone_app_labels_are_user_facing() {
        assert_eq!(StandaloneNativeApp::FileManager.label(), "file manager");
        assert_eq!(StandaloneNativeApp::Settings.label(), "settings");
        assert_eq!(StandaloneNativeApp::Editor.label(), "editor");
    }

    #[test]
    fn standalone_env_value_prefers_nucleon_env_and_falls_back_to_legacy() {
        let _guard = env_lock().lock().unwrap();
        unsafe {
            std::env::remove_var(NUCLEON_NATIVE_STANDALONE_USER_ENV);
            std::env::remove_var(LEGACY_ROBCOS_NATIVE_STANDALONE_USER_ENV);
            std::env::set_var(LEGACY_ROBCOS_NATIVE_STANDALONE_USER_ENV, "legacy-user");
        }
        assert_eq!(standalone_env_value().as_deref(), Some("legacy-user"));
        unsafe {
            std::env::set_var(NUCLEON_NATIVE_STANDALONE_USER_ENV, "nucleon-user");
        }
        assert_eq!(standalone_env_value().as_deref(), Some("nucleon-user"));
        unsafe {
            std::env::remove_var(NUCLEON_NATIVE_STANDALONE_USER_ENV);
            std::env::remove_var(LEGACY_ROBCOS_NATIVE_STANDALONE_USER_ENV);
        }
    }
}
