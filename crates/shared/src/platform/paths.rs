use super::profile::InstallProfile;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum LogicalRoot {
    CoreRoot,
    SystemAddonsRoot,
    UserRoot,
    UserAddonsRoot,
    CacheRoot,
    RuntimeRoot,
}

pub trait PlatformPaths {
    fn install_profile(&self) -> InstallProfile;
    fn logical_root(&self, root: LogicalRoot) -> &Path;

    fn core_root(&self) -> &Path {
        self.logical_root(LogicalRoot::CoreRoot)
    }

    fn system_addons_root(&self) -> &Path {
        self.logical_root(LogicalRoot::SystemAddonsRoot)
    }

    fn user_root(&self) -> &Path {
        self.logical_root(LogicalRoot::UserRoot)
    }

    fn user_addons_root(&self) -> &Path {
        self.logical_root(LogicalRoot::UserAddonsRoot)
    }

    fn cache_root(&self) -> &Path {
        self.logical_root(LogicalRoot::CacheRoot)
    }

    fn runtime_root(&self) -> &Path {
        self.logical_root(LogicalRoot::RuntimeRoot)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlatformPathEnvironment {
    pub home_dir: PathBuf,
    pub data_dir: PathBuf,
    pub data_local_dir: PathBuf,
    pub cache_dir: PathBuf,
    pub runtime_dir: Option<PathBuf>,
    pub temp_dir: PathBuf,
    pub portable_root: Option<PathBuf>,
}

impl PlatformPathEnvironment {
    pub fn detect() -> Self {
        let home_dir = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        let data_dir = dirs::data_dir().unwrap_or_else(|| home_dir.join(".local").join("share"));
        let data_local_dir = dirs::data_local_dir().unwrap_or_else(|| data_dir.clone());
        let cache_dir = dirs::cache_dir().unwrap_or_else(|| home_dir.join(".cache"));
        let runtime_dir = std::env::var_os("XDG_RUNTIME_DIR").map(PathBuf::from);
        let temp_dir = std::env::temp_dir();
        let portable_root = std::env::var_os("PORTABLE_DEV_ROOT").map(PathBuf::from);

        Self {
            home_dir,
            data_dir,
            data_local_dir,
            cache_dir,
            runtime_dir,
            temp_dir,
            portable_root,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedPlatformPaths {
    install_profile: InstallProfile,
    product_dir: String,
    core_root: PathBuf,
    system_addons_root: PathBuf,
    user_root: PathBuf,
    user_addons_root: PathBuf,
    cache_root: PathBuf,
    runtime_root: PathBuf,
}

impl ResolvedPlatformPaths {
    pub fn detect(product_dir: impl Into<String>, profile: InstallProfile) -> Self {
        Self::from_environment(product_dir, profile, PlatformPathEnvironment::detect())
    }

    pub fn from_environment(
        product_dir: impl Into<String>,
        profile: InstallProfile,
        env: PlatformPathEnvironment,
    ) -> Self {
        let product_dir = normalize_product_dir(product_dir.into());
        match profile {
            InstallProfile::LinuxDesktop => Self::linux_desktop(product_dir, env),
            InstallProfile::WindowsLauncher => Self::windows_launcher(product_dir, env),
            InstallProfile::MacLauncher => Self::mac_launcher(product_dir, env),
            InstallProfile::PortableDev => Self::portable_dev(product_dir, env),
        }
    }

    pub fn product_dir(&self) -> &str {
        &self.product_dir
    }

    pub fn ensure_runtime_dirs(&self) -> Result<()> {
        std::fs::create_dir_all(&self.user_root)?;
        std::fs::create_dir_all(&self.user_addons_root)?;
        std::fs::create_dir_all(&self.cache_root)?;
        std::fs::create_dir_all(&self.runtime_root)?;

        if matches!(self.install_profile, InstallProfile::PortableDev) {
            std::fs::create_dir_all(&self.core_root)?;
            std::fs::create_dir_all(&self.system_addons_root)?;
        }

        Ok(())
    }

    fn linux_desktop(product_dir: String, env: PlatformPathEnvironment) -> Self {
        let user_base = env.data_local_dir.join(&product_dir);
        Self {
            install_profile: InstallProfile::LinuxDesktop,
            product_dir: product_dir.clone(),
            core_root: PathBuf::from("/usr/share").join(&product_dir),
            system_addons_root: PathBuf::from("/usr/share")
                .join(&product_dir)
                .join("addons"),
            user_root: user_base.join("user"),
            user_addons_root: user_base.join("addons"),
            cache_root: env.cache_dir.join(&product_dir),
            runtime_root: env
                .runtime_dir
                .unwrap_or_else(|| env.temp_dir.join("app-shell-runtime"))
                .join(&product_dir),
        }
    }

    fn windows_launcher(product_dir: String, env: PlatformPathEnvironment) -> Self {
        let roaming_base = env.data_dir.join(&product_dir);
        let local_base = env.data_local_dir.join(&product_dir);
        Self {
            install_profile: InstallProfile::WindowsLauncher,
            product_dir,
            core_root: local_base.join("core"),
            system_addons_root: local_base.join("system-addons"),
            user_root: roaming_base.join("user"),
            user_addons_root: roaming_base.join("addons"),
            cache_root: local_base.join("cache"),
            runtime_root: local_base.join("runtime"),
        }
    }

    fn mac_launcher(product_dir: String, env: PlatformPathEnvironment) -> Self {
        let app_support_base = env.data_local_dir.join(&product_dir);
        Self {
            install_profile: InstallProfile::MacLauncher,
            product_dir: product_dir.clone(),
            core_root: app_support_base.join("core"),
            system_addons_root: app_support_base.join("system-addons"),
            user_root: app_support_base.join("user"),
            user_addons_root: app_support_base.join("addons"),
            cache_root: env.cache_dir.join(&product_dir),
            runtime_root: env.temp_dir.join(format!("{product_dir}.runtime")),
        }
    }

    fn portable_dev(product_dir: String, env: PlatformPathEnvironment) -> Self {
        let portable_root = env.portable_root.unwrap_or_else(|| {
            std::env::current_dir()
                .unwrap_or_else(|_| PathBuf::from("."))
                .join(".portable")
        });
        let base = portable_root.join(&product_dir);
        Self {
            install_profile: InstallProfile::PortableDev,
            product_dir,
            core_root: base.join("core"),
            system_addons_root: base.join("system-addons"),
            user_root: base.join("user"),
            user_addons_root: base.join("addons"),
            cache_root: base.join("cache"),
            runtime_root: base.join("runtime"),
        }
    }
}

impl PlatformPaths for ResolvedPlatformPaths {
    fn install_profile(&self) -> InstallProfile {
        self.install_profile
    }

    fn logical_root(&self, root: LogicalRoot) -> &Path {
        match root {
            LogicalRoot::CoreRoot => &self.core_root,
            LogicalRoot::SystemAddonsRoot => &self.system_addons_root,
            LogicalRoot::UserRoot => &self.user_root,
            LogicalRoot::UserAddonsRoot => &self.user_addons_root,
            LogicalRoot::CacheRoot => &self.cache_root,
            LogicalRoot::RuntimeRoot => &self.runtime_root,
        }
    }
}

fn normalize_product_dir(product_dir: String) -> String {
    let trimmed = product_dir.trim();
    if trimmed.is_empty() {
        "app-shell".to_string()
    } else {
        trimmed.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::{InstallProfile, PlatformPathEnvironment, PlatformPaths, ResolvedPlatformPaths};
    use std::path::PathBuf;

    #[test]
    fn linux_desktop_profile_uses_system_and_xdg_roots() {
        let env = PlatformPathEnvironment {
            home_dir: PathBuf::from("/home/alice"),
            data_dir: PathBuf::from("/home/alice/.local/share"),
            data_local_dir: PathBuf::from("/home/alice/.local/share"),
            cache_dir: PathBuf::from("/home/alice/.cache"),
            runtime_dir: Some(PathBuf::from("/run/user/1000")),
            temp_dir: PathBuf::from("/tmp"),
            portable_root: None,
        };

        let paths =
            ResolvedPlatformPaths::from_environment("nucleon", InstallProfile::LinuxDesktop, env);

        assert_eq!(paths.core_root(), PathBuf::from("/usr/share/nucleon"));
        assert_eq!(
            paths.system_addons_root(),
            PathBuf::from("/usr/share/nucleon/addons")
        );
        assert_eq!(
            paths.user_root(),
            PathBuf::from("/home/alice/.local/share/nucleon/user")
        );
        assert_eq!(
            paths.user_addons_root(),
            PathBuf::from("/home/alice/.local/share/nucleon/addons")
        );
        assert_eq!(
            paths.cache_root(),
            PathBuf::from("/home/alice/.cache/nucleon")
        );
        assert_eq!(
            paths.runtime_root(),
            PathBuf::from("/run/user/1000/nucleon")
        );
    }

    #[test]
    fn portable_dev_profile_stays_under_portable_root() {
        let env = PlatformPathEnvironment {
            home_dir: PathBuf::from("/home/alice"),
            data_dir: PathBuf::from("/home/alice/.local/share"),
            data_local_dir: PathBuf::from("/home/alice/.local/share"),
            cache_dir: PathBuf::from("/home/alice/.cache"),
            runtime_dir: None,
            temp_dir: PathBuf::from("/tmp"),
            portable_root: Some(PathBuf::from("/tmp/dev-layout")),
        };

        let paths =
            ResolvedPlatformPaths::from_environment("nucleon", InstallProfile::PortableDev, env);

        assert_eq!(
            paths.core_root(),
            PathBuf::from("/tmp/dev-layout/nucleon/core")
        );
        assert_eq!(
            paths.system_addons_root(),
            PathBuf::from("/tmp/dev-layout/nucleon/system-addons")
        );
        assert_eq!(
            paths.user_root(),
            PathBuf::from("/tmp/dev-layout/nucleon/user")
        );
        assert_eq!(
            paths.user_addons_root(),
            PathBuf::from("/tmp/dev-layout/nucleon/addons")
        );
        assert_eq!(
            paths.cache_root(),
            PathBuf::from("/tmp/dev-layout/nucleon/cache")
        );
        assert_eq!(
            paths.runtime_root(),
            PathBuf::from("/tmp/dev-layout/nucleon/runtime")
        );
    }
}
