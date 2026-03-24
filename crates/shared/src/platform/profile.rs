use serde::{Deserialize, Serialize};
use std::str::FromStr;

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

    pub fn parse(raw: &str) -> Option<Self> {
        raw.parse().ok()
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

impl FromStr for InstallProfile {
    type Err = ();

    fn from_str(raw: &str) -> Result<Self, Self::Err> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "linux-desktop" | "linux" => Ok(Self::LinuxDesktop),
            "windows-launcher" | "windows" => Ok(Self::WindowsLauncher),
            "mac-launcher" | "macos-launcher" | "macos" | "mac" => Ok(Self::MacLauncher),
            "portable-dev" | "portable" => Ok(Self::PortableDev),
            _ => Err(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::InstallProfile;

    #[test]
    fn install_profile_parse_accepts_primary_and_short_names() {
        assert_eq!(
            InstallProfile::parse("linux-desktop"),
            Some(InstallProfile::LinuxDesktop)
        );
        assert_eq!(
            InstallProfile::parse("windows"),
            Some(InstallProfile::WindowsLauncher)
        );
        assert_eq!(
            InstallProfile::parse("mac"),
            Some(InstallProfile::MacLauncher)
        );
        assert_eq!(
            InstallProfile::parse("portable"),
            Some(InstallProfile::PortableDev)
        );
        assert_eq!(InstallProfile::parse("unknown"), None);
    }
}
