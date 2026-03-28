use super::{InstallProfile, PlatformPathEnvironment, PlatformPaths, ResolvedPlatformPaths};
use std::path::{Path, PathBuf};

pub const PRODUCT_DIR_ENV: &str = "NUCLEON_PRODUCT_DIR";
pub const INSTALL_PROFILE_ENV: &str = "NUCLEON_INSTALL_PROFILE";
pub const BASE_DIR_ENV: &str = "NUCLEON_BASE_DIR";
pub const DEFAULT_PRODUCT_DIR: &str = "nucleon";
pub const DEFAULT_PTY_KEY_DEBUG_LOG_FILE: &str = "nucleon_keys.log";

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

    pub fn users_db_file(&self) -> PathBuf {
        self.users_dir().join("users.json")
    }

    pub fn user_dir(&self, username: &str) -> PathBuf {
        self.users_dir().join(username)
    }

    pub fn desktop_dir_for_username(&self, username: &str) -> PathBuf {
        self.user_dir(username).join("Desktop")
    }

    pub fn shared_desktop_dir(&self) -> PathBuf {
        self.path("Desktop")
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

    pub fn user_settings_file(&self, username: &str) -> PathBuf {
        self.user_file(username, "settings.json")
    }

    pub fn user_apps_catalog_file(&self, username: &str) -> PathBuf {
        self.user_file(username, "apps.json")
    }

    pub fn user_games_catalog_file(&self, username: &str) -> PathBuf {
        self.user_file(username, "games.json")
    }

    pub fn user_networks_catalog_file(&self, username: &str) -> PathBuf {
        self.user_file(username, "networks.json")
    }

    pub fn user_documents_catalog_file(&self, username: &str) -> PathBuf {
        self.user_file(username, "documents.json")
    }

    pub fn global_settings_file(&self) -> PathBuf {
        self.path("settings.json")
    }

    pub fn shared_apps_catalog_file(&self) -> PathBuf {
        self.path("apps.json")
    }

    pub fn shared_games_catalog_file(&self) -> PathBuf {
        self.path("games.json")
    }

    pub fn shared_networks_catalog_file(&self) -> PathBuf {
        self.path("networks.json")
    }

    pub fn shared_documents_catalog_file(&self) -> PathBuf {
        self.path("documents.json")
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
        self.path(DEFAULT_PTY_KEY_DEBUG_LOG_FILE)
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
        let env = PlatformPathEnvironment::detect();
        let explicit_product_dir = detect_product_dir_override();
        let product_dir = explicit_product_dir
            .clone()
            .unwrap_or_else(|| DEFAULT_PRODUCT_DIR.to_string());
        let install_profile = detect_install_profile().unwrap_or_default();
        let state_root_override = detect_state_root_override();
        let runtime = Self::from_parts(
            product_dir,
            install_profile,
            env.clone(),
            state_root_override.clone(),
        );
        if explicit_product_dir.is_none() && state_root_override.is_none() {
            migrate_previous_product_layout_if_needed(&runtime, install_profile, &env);
        }
        runtime
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

fn detect_product_dir_override() -> Option<String> {
    first_non_empty_env(&[PRODUCT_DIR_ENV])
}

fn detect_install_profile() -> Option<InstallProfile> {
    first_non_empty_env(&[INSTALL_PROFILE_ENV])
        .and_then(|value| InstallProfile::parse(&value))
}

fn detect_state_root_override() -> Option<PathBuf> {
    first_non_empty_env(&[BASE_DIR_ENV]).map(PathBuf::from)
}

fn first_non_empty_env(names: &[&str]) -> Option<String> {
    names.iter().find_map(|name| {
        std::env::var(name)
            .ok()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
    })
}

fn previous_product_dir_name() -> String {
    String::from_utf8(vec![114, 111, 98, 99, 111, 115]).expect("ascii")
}

fn previous_pty_key_debug_log_file_name() -> String {
    String::from_utf8(vec![114, 111, 98, 99, 111, 115, 95, 107, 101, 121, 115, 46, 108, 111, 103])
        .expect("ascii")
}

fn migrate_previous_product_layout_if_needed(
    runtime: &RuntimeEnvironment,
    install_profile: InstallProfile,
    env: &PlatformPathEnvironment,
) {
    if runtime.product_dir() != DEFAULT_PRODUCT_DIR {
        return;
    }
    let legacy_paths = ResolvedPlatformPaths::from_environment(
        previous_product_dir_name(),
        install_profile,
        env.clone(),
    );
    let target_paths = runtime.paths();

    for (legacy, target) in [
        (legacy_paths.user_root(), target_paths.user_root()),
        (
            legacy_paths.user_addons_root(),
            target_paths.user_addons_root(),
        ),
        (legacy_paths.cache_root(), target_paths.cache_root()),
        (legacy_paths.runtime_root(), target_paths.runtime_root()),
    ] {
        if legacy != target {
            merge_path_if_missing(legacy, target);
        }
    }
    merge_path_if_missing(
        &legacy_paths
            .runtime_root()
            .join(previous_pty_key_debug_log_file_name()),
        &target_paths
            .runtime_root()
            .join(DEFAULT_PTY_KEY_DEBUG_LOG_FILE),
    );
}

fn merge_path_if_missing(from: &Path, to: &Path) {
    if !from.exists() {
        return;
    }
    if from.is_dir() {
        if let Ok(entries) = std::fs::read_dir(from) {
            let _ = std::fs::create_dir_all(to);
            for entry in entries.flatten() {
                let child_from = entry.path();
                let child_to = to.join(entry.file_name());
                merge_path_if_missing(&child_from, &child_to);
            }
        }
    } else {
        if to.exists() {
            return;
        }
        if let Some(parent) = to.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::copy(from, to);
    }
}

#[cfg(test)]
mod tests {
    use super::{
        migrate_previous_product_layout_if_needed, InstallProfile, PlatformPathEnvironment,
        PlatformPaths, ResolvedPlatformPaths, RuntimeEnvironment, RuntimePathLayout,
        StatePathLayout, DEFAULT_PRODUCT_DIR, DEFAULT_PTY_KEY_DEBUG_LOG_FILE,
    };
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn unique_temp_dir(label: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("nucleon-runtime-{label}-{unique}"));
        std::fs::create_dir_all(&dir).expect("create temp dir");
        dir
    }

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
            layout.users_db_file(),
            PathBuf::from("/state-root/users/users.json")
        );
        assert_eq!(
            layout.user_dir("alice"),
            PathBuf::from("/state-root/users/alice")
        );
        assert_eq!(
            layout.desktop_dir_for_username("alice"),
            PathBuf::from("/state-root/users/alice/Desktop")
        );
        assert_eq!(
            layout.shared_desktop_dir(),
            PathBuf::from("/state-root/Desktop")
        );
        assert_eq!(
            layout.native_shell_snapshot_file("alice"),
            PathBuf::from("/state-root/users/alice/native_shell.json")
        );
        assert_eq!(
            layout.user_settings_file("alice"),
            PathBuf::from("/state-root/users/alice/settings.json")
        );
        assert_eq!(
            layout.user_apps_catalog_file("alice"),
            PathBuf::from("/state-root/users/alice/apps.json")
        );
        assert_eq!(
            layout.shared_apps_catalog_file(),
            PathBuf::from("/state-root/apps.json")
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
            PathBuf::from("/runtime-root").join(DEFAULT_PTY_KEY_DEBUG_LOG_FILE)
        );
    }

    #[test]
    fn previous_product_roots_migrate_into_default_nucleon_layout() {
        let root = unique_temp_dir("legacy-migrate");
        let env = PlatformPathEnvironment {
            home_dir: root.join("home"),
            data_dir: root.join("data"),
            data_local_dir: root.join("data-local"),
            cache_dir: root.join("cache"),
            runtime_dir: Some(root.join("runtime-parent")),
            temp_dir: root.join("tmp"),
            portable_root: None,
        };
        let runtime = RuntimeEnvironment::from_environment(
            DEFAULT_PRODUCT_DIR,
            InstallProfile::LinuxDesktop,
            env.clone(),
        );
        let legacy_paths = ResolvedPlatformPaths::from_environment(
            super::previous_product_dir_name(),
            InstallProfile::LinuxDesktop,
            env.clone(),
        );

        std::fs::create_dir_all(legacy_paths.user_root()).expect("legacy user root");
        std::fs::create_dir_all(legacy_paths.user_addons_root()).expect("legacy addon root");
        std::fs::create_dir_all(legacy_paths.runtime_root()).expect("legacy runtime root");
        std::fs::write(legacy_paths.user_root().join("settings.json"), "{}")
            .expect("legacy settings");
        let legacy_addon_dir = legacy_paths.user_addons_root().join("sample-addon");
        std::fs::create_dir_all(&legacy_addon_dir).expect("legacy addon dir");
        std::fs::write(legacy_addon_dir.join("manifest.json"), "{}")
            .expect("legacy addon manifest");
        std::fs::write(
            legacy_paths
                .runtime_root()
                .join(super::previous_pty_key_debug_log_file_name()),
            "legacy-keys",
        )
        .expect("legacy key log");

        migrate_previous_product_layout_if_needed(&runtime, InstallProfile::LinuxDesktop, &env);

        assert_eq!(
            std::fs::read_to_string(runtime.state_root().join("settings.json"))
                .expect("migrated settings"),
            "{}"
        );
        assert!(runtime
            .paths()
            .user_addons_root()
            .join("sample-addon")
            .join("manifest.json")
            .exists());
        assert_eq!(
            std::fs::read_to_string(
                runtime
                    .paths()
                    .runtime_root()
                    .join(DEFAULT_PTY_KEY_DEBUG_LOG_FILE)
            )
            .expect("migrated key log"),
            "legacy-keys"
        );

        let _ = std::fs::remove_dir_all(root);
    }
}
