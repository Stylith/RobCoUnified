use std::ffi::OsString;
use std::path::{Path, PathBuf};
#[cfg(not(test))]
use std::process::Command;
#[cfg(test)]
use std::sync::{Mutex, OnceLock};

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
    #[cfg(not(test))]
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

#[cfg(test)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StandaloneLaunchRecord {
    pub app: StandaloneNativeApp,
    pub args: Vec<OsString>,
    pub session_username: Option<String>,
}

pub fn launch_standalone_app(
    app: StandaloneNativeApp,
    args: &[OsString],
    session_username: Option<&str>,
) -> Result<(), String> {
    #[cfg(test)]
    {
        record_test_launch(app, args, session_username);
        Ok(())
    }

    #[cfg(not(test))]
    {
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
}

#[cfg(test)]
fn launch_record_slot() -> &'static Mutex<Option<StandaloneLaunchRecord>> {
    static SLOT: OnceLock<Mutex<Option<StandaloneLaunchRecord>>> = OnceLock::new();
    SLOT.get_or_init(|| Mutex::new(None))
}

#[cfg(test)]
fn record_test_launch(
    app: StandaloneNativeApp,
    args: &[OsString],
    session_username: Option<&str>,
) {
    let mut slot = launch_record_slot()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    *slot = Some(StandaloneLaunchRecord {
        app,
        args: args.to_vec(),
        session_username: session_username.map(str::to_string),
    });
}

#[cfg(test)]
pub fn take_last_standalone_launch() -> Option<StandaloneLaunchRecord> {
    launch_record_slot()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .take()
}

#[cfg(not(test))]
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
