use super::{InstallProfile, PlatformPathEnvironment, PlatformPaths, ResolvedPlatformPaths};
use std::path::{Path, PathBuf};

pub const PRODUCT_DIR_ENV: &str = "NUCLEON_PRODUCT_DIR";
pub const LEGACY_PRODUCT_DIR_ENV: &str = "ROBCOS_PRODUCT_DIR";
pub const INSTALL_PROFILE_ENV: &str = "NUCLEON_INSTALL_PROFILE";
pub const LEGACY_INSTALL_PROFILE_ENV: &str = "ROBCOS_INSTALL_PROFILE";
pub const BASE_DIR_ENV: &str = "NUCLEON_BASE_DIR";
pub const LEGACY_BASE_DIR_ENV: &str = "ROBCOS_BASE_DIR";
pub const DEFAULT_PRODUCT_DIR: &str = "robcos";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatePathLayout {
    root: PathBuf,
}

impl StatePathLayout {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn path(&self, relative: impl AsRef<Path>) -> PathBuf {
        self.root.join(relative)
    }

    pub fn users_dir(&self) -> PathBuf {
        self.path("users")
    }

    pub fn user_dir(&self, username: &str) -> PathBuf {
        self.users_dir().join(username)
    }

    pub fn desktop_dir_for_username(&self, username: &str) -> PathBuf {
        self.user_dir(username).join("Desktop")
    }

    pub fn user_file(&self, username: &str, filename: &str) -> PathBuf {
        self.user_dir(username).join(filename)
    }

    pub fn file_manager_trash_dir_for_username(&self, username: &str) -> PathBuf {
        self.user_dir(username).join(".fm_trash")
    }

    pub fn native_shell_snapshot_file(&self, username: &str) -> PathBuf {
        self.user_file(username, "native_shell.json")
    }

    pub fn default_apps_prompt_marker(&self, username: &str) -> PathBuf {
        self.user_file(username, ".default_apps_prompt")
    }

    pub fn global_settings_file(&self) -> PathBuf {
        self.path("settings.json")
    }

    pub fn about_file(&self) -> PathBuf {
        self.path("about.json")
    }

    pub fn session_file(&self) -> PathBuf {
        self.path(".session")
    }

    pub fn installed_package_descriptions_file(&self) -> PathBuf {
        self.path("installed_package_descriptions.json")
    }

    pub fn addon_state_overrides_file(&self) -> PathBuf {
        self.path("addon_state.json")
    }

    pub fn journal_entries_dir(&self) -> PathBuf {
        self.path("journal_entries")
    }

    pub fn diagnostics_log_file(&self) -> PathBuf {
        self.path("diagnostics.log")
    }

    pub fn shared_file_manager_trash_dir(&self) -> PathBuf {
        self.path(".fm_trash")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimePathLayout {
    root: PathBuf,
}

impl RuntimePathLayout {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn path(&self, relative: impl AsRef<Path>) -> PathBuf {
        self.root.join(relative)
    }

    pub fn ipc_socket_file(&self) -> PathBuf {
        self.path("shell.sock")
    }

    pub fn pty_key_debug_log_file(&self) -> PathBuf {
        self.path("robcos_keys.log")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeEnvironment {
    product_dir: String,
    install_profile: InstallProfile,
    paths: ResolvedPlatformPaths,
    state_root: PathBuf,
}

impl RuntimeEnvironment {
    pub fn detect() -> Self {
        let product_dir = detect_product_dir();
        let install_profile = detect_install_profile().unwrap_or_default();
        let state_root_override = detect_state_root_override();
        Self::from_parts(
            product_dir,
            install_profile,
            PlatformPathEnvironment::detect(),
            state_root_override,
        )
    }

    pub fn from_environment(
        product_dir: impl Into<String>,
        install_profile: InstallProfile,
        env: PlatformPathEnvironment,
    ) -> Self {
        Self::from_parts(product_dir, install_profile, env, None)
    }

    pub fn product_dir(&self) -> &str {
        &self.product_dir
    }

    pub fn install_profile(&self) -> InstallProfile {
        self.install_profile
    }

    pub fn paths(&self) -> &ResolvedPlatformPaths {
        &self.paths
    }

    pub fn state_root(&self) -> &Path {
        &self.state_root
    }

    pub fn state_layout(&self) -> StatePathLayout {
        StatePathLayout::new(self.state_root.clone())
    }

    pub fn runtime_layout(&self) -> RuntimePathLayout {
        RuntimePathLayout::new(self.paths.runtime_root().to_path_buf())
    }

    pub fn state_path(&self, relative: impl AsRef<Path>) -> PathBuf {
        self.state_root.join(relative)
    }

    fn from_parts(
        product_dir: impl Into<String>,
        install_profile: InstallProfile,
        env: PlatformPathEnvironment,
        state_root_override: Option<PathBuf>,
    ) -> Self {
        let paths = ResolvedPlatformPaths::from_environment(product_dir, install_profile, env);
        let product_dir = paths.product_dir().to_string();
        let state_root = state_root_override.unwrap_or_else(|| paths.user_root().to_path_buf());

        Self {
            product_dir,
            install_profile,
            paths,
            state_root,
        }
    }
}

fn detect_product_dir() -> String {
    first_non_empty_env(&[PRODUCT_DIR_ENV, LEGACY_PRODUCT_DIR_ENV])
        .unwrap_or_else(|| DEFAULT_PRODUCT_DIR.to_string())
}

fn detect_install_profile() -> Option<InstallProfile> {
    first_non_empty_env(&[INSTALL_PROFILE_ENV, LEGACY_INSTALL_PROFILE_ENV])
        .and_then(|value| InstallProfile::parse(&value))
}

fn detect_state_root_override() -> Option<PathBuf> {
    first_non_empty_env(&[BASE_DIR_ENV, LEGACY_BASE_DIR_ENV]).map(PathBuf::from)
}

fn first_non_empty_env(names: &[&str]) -> Option<String> {
    names.iter().find_map(|name| {
        std::env::var(name)
            .ok()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
    })
}

#[cfg(test)]
mod tests {
    use super::{
        InstallProfile, PlatformPathEnvironment, PlatformPaths, RuntimeEnvironment,
        RuntimePathLayout, StatePathLayout,
    };
    use std::path::PathBuf;

    #[test]
    fn runtime_environment_uses_logical_user_root_as_compat_state_root_by_default() {
        let env = PlatformPathEnvironment {
            home_dir: PathBuf::from("/home/alice"),
            data_dir: PathBuf::from("/home/alice/.local/share"),
            data_local_dir: PathBuf::from("/home/alice/.local/share"),
            cache_dir: PathBuf::from("/home/alice/.cache"),
            runtime_dir: Some(PathBuf::from("/run/user/1000")),
            temp_dir: PathBuf::from("/tmp"),
            portable_root: None,
        };

        let runtime =
            RuntimeEnvironment::from_environment("nucleon", InstallProfile::LinuxDesktop, env);

        assert_eq!(runtime.state_root(), runtime.paths().user_root());
        assert_eq!(runtime.product_dir(), "nucleon");
    }

    #[test]
    fn runtime_environment_keeps_explicit_state_root_override_separate_from_logical_roots() {
        let env = PlatformPathEnvironment {
            home_dir: PathBuf::from("/home/alice"),
            data_dir: PathBuf::from("/home/alice/.local/share"),
            data_local_dir: PathBuf::from("/home/alice/.local/share"),
            cache_dir: PathBuf::from("/home/alice/.cache"),
            runtime_dir: Some(PathBuf::from("/run/user/1000")),
            temp_dir: PathBuf::from("/tmp"),
            portable_root: None,
        };

        let runtime = RuntimeEnvironment::from_parts(
            "nucleon",
            InstallProfile::LinuxDesktop,
            env,
            Some(PathBuf::from("/tmp/custom-state")),
        );

        assert_eq!(runtime.state_root(), PathBuf::from("/tmp/custom-state"));
        assert_eq!(
            runtime.paths().user_root(),
            PathBuf::from("/home/alice/.local/share/nucleon/user")
        );
    }

    #[test]
    fn state_path_layout_builds_named_state_paths() {
        let layout = StatePathLayout::new(PathBuf::from("/state-root"));

        assert_eq!(layout.users_dir(), PathBuf::from("/state-root/users"));
        assert_eq!(
            layout.user_dir("alice"),
            PathBuf::from("/state-root/users/alice")
        );
        assert_eq!(
            layout.desktop_dir_for_username("alice"),
            PathBuf::from("/state-root/users/alice/Desktop")
        );
        assert_eq!(
            layout.native_shell_snapshot_file("alice"),
            PathBuf::from("/state-root/users/alice/native_shell.json")
        );
        assert_eq!(
            layout.installed_package_descriptions_file(),
            PathBuf::from("/state-root/installed_package_descriptions.json")
        );
    }

    #[test]
    fn runtime_path_layout_builds_named_runtime_paths() {
        let layout = RuntimePathLayout::new(PathBuf::from("/runtime-root"));

        assert_eq!(
            layout.ipc_socket_file(),
            PathBuf::from("/runtime-root/shell.sock")
        );
        assert_eq!(
            layout.pty_key_debug_log_file(),
            PathBuf::from("/runtime-root/robcos_keys.log")
        );
    }
}
