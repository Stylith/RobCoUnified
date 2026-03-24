use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum InstallProfile {
    LinuxDesktop,
    WindowsLauncher,
    MacLauncher,
    PortableDev,
}

impl InstallProfile {
    pub const fn default_for_target() -> Self {
        if cfg!(target_os = "linux") {
            Self::LinuxDesktop
        } else if cfg!(target_os = "windows") {
            Self::WindowsLauncher
        } else if cfg!(target_os = "macos") {
            Self::MacLauncher
        } else {
            Self::PortableDev
        }
    }

    pub const fn integration_level(self) -> IntegrationLevel {
        match self {
            Self::LinuxDesktop => IntegrationLevel::FullEnvironment,
            Self::WindowsLauncher | Self::MacLauncher => IntegrationLevel::Launcher,
            Self::PortableDev => IntegrationLevel::Portable,
        }
    }
}

impl Default for InstallProfile {
    fn default() -> Self {
        Self::default_for_target()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum IntegrationLevel {
    FullEnvironment,
    Launcher,
    Portable,
}
