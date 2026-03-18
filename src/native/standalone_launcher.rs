use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;

pub const ROBCOS_NATIVE_STANDALONE_USER_ENV: &str = "ROBCOS_NATIVE_STANDALONE_USER";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StandaloneNativeApp {
    FileManager,
    Settings,
    Editor,
    Applications,
    NukeCodes,
    Installer,
}

impl StandaloneNativeApp {
    pub const fn binary_stem(self) -> &'static str {
        match self {
            Self::FileManager => "robcos-file-manager",
            Self::Settings => "robcos-settings",
            Self::Editor => "robcos-editor",
            Self::Applications => "robcos-applications",
            Self::NukeCodes => "robcos-nuke-codes",
            Self::Installer => "robcos-installer",
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::FileManager => "file manager",
            Self::Settings => "settings",
            Self::Editor => "editor",
            Self::Applications => "applications",
            Self::NukeCodes => "nuke codes",
            Self::Installer => "installer",
        }
    }
}

pub fn launch_standalone_app(
    app: StandaloneNativeApp,
    args: &[OsString],
    session_username: Option<&str>,
) -> Result<(), String> {
    let binary = sibling_binary_path(app)?;
    let mut command = Command::new(binary);
    command.args(args);
    if let Some(username) = session_username.filter(|username| !username.is_empty()) {
        command.env(ROBCOS_NATIVE_STANDALONE_USER_ENV, username);
    }
    command
        .spawn()
        .map(|_| ())
        .map_err(|err| format!("Could not open {}: {err}", app.label()))
}

fn sibling_binary_path(app: StandaloneNativeApp) -> Result<PathBuf, String> {
    let current_exe = std::env::current_exe()
        .map_err(|err| format!("Could not resolve current executable: {err}"))?;
    let Some(dir) = current_exe.parent() else {
        return Err("Could not resolve application directory.".to_string());
    };
    let candidate = dir.join(sibling_binary_file_name(app.binary_stem(), &current_exe));
    if candidate.is_file() {
        Ok(candidate)
    } else {
        Err(format!(
            "Could not find the {} app beside `{}`.",
            app.label(),
            current_exe.display()
        ))
    }
}

fn sibling_binary_file_name(binary_stem: &str, _current_exe: &Path) -> OsString {
    #[cfg(target_os = "windows")]
    {
        if _current_exe
            .extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| ext.eq_ignore_ascii_case("exe"))
        {
            return OsString::from(format!("{binary_stem}.exe"));
        }
    }

    OsString::from(binary_stem)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sibling_binary_file_name_matches_current_platform_convention() {
        let current_exe = if cfg!(target_os = "windows") {
            PathBuf::from("/tmp/robcos-native.exe")
        } else {
            PathBuf::from("/tmp/robcos-native")
        };

        let name = sibling_binary_file_name("robcos-settings", &current_exe);

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
        assert_eq!(StandaloneNativeApp::Applications.label(), "applications");
        assert_eq!(StandaloneNativeApp::NukeCodes.label(), "nuke codes");
        assert_eq!(StandaloneNativeApp::Installer.label(), "installer");
    }
}
