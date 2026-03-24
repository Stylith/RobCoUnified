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
    use super::{InstallProfile, PlatformPathEnvironment, PlatformPaths, RuntimeEnvironment};
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
}
