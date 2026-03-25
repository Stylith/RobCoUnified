#![allow(dead_code)]

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;

pub const ROBCOS_NATIVE_STANDALONE_USER_ENV: &str = "ROBCOS_NATIVE_STANDALONE_USER";
pub const ROBCOS_NATIVE_IPC_SOCKET_ENV: &str = "ROBCOS_NATIVE_IPC_SOCKET";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StandaloneNativeApp {
    FileManager,
    Settings,
    Editor,
}

impl StandaloneNativeApp {
    pub const fn binary_stem(self) -> &'static str {
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
        command.env(ROBCOS_NATIVE_STANDALONE_USER_ENV, username);
    }
    command.env(
        ROBCOS_NATIVE_IPC_SOCKET_ENV,
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
    let candidate =
        crate::config::bundled_binary_path(platform_binary_file_name(app.binary_stem()));
    candidate.is_file().then_some(candidate)
}

fn sibling_binary_path(app: StandaloneNativeApp) -> Option<PathBuf> {
    let current_exe = std::env::current_exe().ok()?;
    sibling_binary_dirs(&current_exe)
        .into_iter()
        .map(|dir| dir.join(platform_binary_file_name(app.binary_stem())))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sibling_binary_file_name_matches_current_platform_convention() {
        let name = platform_binary_file_name("robcos-settings");

        #[cfg(target_os = "windows")]
        assert_eq!(name, OsString::from("robcos-settings.exe"));
        #[cfg(not(target_os = "windows"))]
        assert_eq!(name, OsString::from("robcos-settings"));
    }

    #[test]
    fn standalone_app_labels_are_user_facing() {
        assert_eq!(StandaloneNativeApp::FileManager.label(), "file manager");
        assert_eq!(StandaloneNativeApp::Settings.label(), "settings");
        assert_eq!(StandaloneNativeApp::Editor.label(), "editor");
    }
}
