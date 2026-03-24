use super::super::desktop_app::DesktopWindow;
use super::super::{first_party_addon_registry, NativeSettingsPanel};
use crate::platform::{AddonEntrypoint, CapabilityId, LaunchTarget};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum NativeDesktopLaunch {
    OpenWindow(DesktopWindow),
    OpenNukeCodes,
    OpenSettingsPanel(Option<NativeSettingsPanel>),
}

pub(super) fn file_manager_launch_target() -> LaunchTarget {
    LaunchTarget::Capability {
        capability: CapabilityId::from("file-browser"),
    }
}

pub(super) fn editor_launch_target() -> LaunchTarget {
    LaunchTarget::Capability {
        capability: CapabilityId::from("text-editor"),
    }
}

pub(super) fn nuke_codes_launch_target() -> LaunchTarget {
    LaunchTarget::Capability {
        capability: CapabilityId::from("code-reference"),
    }
}

pub(super) fn settings_launch_target() -> LaunchTarget {
    LaunchTarget::Capability {
        capability: CapabilityId::from("settings-ui"),
    }
}

pub(super) fn resolve_desktop_launch_target(target: &LaunchTarget) -> Option<NativeDesktopLaunch> {
    let registry = first_party_addon_registry();
    let manifest = match target {
        LaunchTarget::Addon { addon_id } => registry.manifest(addon_id),
        LaunchTarget::Capability { capability } => {
            registry.by_capability(capability).into_iter().next()
        }
    }?;

    match &manifest.entrypoint {
        AddonEntrypoint::StaticRoute { route } => resolve_static_route(route),
        AddonEntrypoint::StandaloneBinary { .. } => None,
    }
}

pub(super) fn unresolved_launch_target_status(target: &LaunchTarget) -> String {
    match target {
        LaunchTarget::Addon { addon_id } => {
            format!("Addon '{addon_id}' is not wired into the desktop launcher yet.")
        }
        LaunchTarget::Capability { capability } => {
            format!("Capability '{capability}' is not wired into the desktop launcher yet.")
        }
    }
}

fn resolve_static_route(route: &str) -> Option<NativeDesktopLaunch> {
    match route {
        "editor" => Some(NativeDesktopLaunch::OpenWindow(DesktopWindow::Editor)),
        "file-manager" => Some(NativeDesktopLaunch::OpenWindow(DesktopWindow::FileManager)),
        "nuke-codes" => Some(NativeDesktopLaunch::OpenNukeCodes),
        "settings" => Some(NativeDesktopLaunch::OpenSettingsPanel(None)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        editor_launch_target, file_manager_launch_target, nuke_codes_launch_target,
        resolve_desktop_launch_target,
        settings_launch_target,
        NativeDesktopLaunch,
    };
    use crate::platform::{AddonId, LaunchTarget};
    use crate::native::desktop_app::DesktopWindow;
    use crate::native::NativeSettingsPanel;

    #[test]
    fn file_manager_capability_resolves_to_file_manager_window() {
        assert_eq!(
            resolve_desktop_launch_target(&file_manager_launch_target()),
            Some(NativeDesktopLaunch::OpenWindow(DesktopWindow::FileManager))
        );
    }

    #[test]
    fn editor_capability_resolves_to_editor_window() {
        assert_eq!(
            resolve_desktop_launch_target(&editor_launch_target()),
            Some(NativeDesktopLaunch::OpenWindow(DesktopWindow::Editor))
        );
    }

    #[test]
    fn nuke_codes_capability_resolves_to_nuke_codes_route() {
        assert_eq!(
            resolve_desktop_launch_target(&nuke_codes_launch_target()),
            Some(NativeDesktopLaunch::OpenNukeCodes)
        );
    }

    #[test]
    fn settings_capability_resolves_to_settings_panel() {
        assert_eq!(
            resolve_desktop_launch_target(&settings_launch_target()),
            Some(NativeDesktopLaunch::OpenSettingsPanel(None))
        );
    }

    #[test]
    fn settings_addon_id_resolves_to_settings_panel() {
        let target = LaunchTarget::Addon {
            addon_id: AddonId::from("shell.settings"),
        };

        assert_eq!(
            resolve_desktop_launch_target(&target),
            Some(NativeDesktopLaunch::OpenSettingsPanel(None))
        );
    }

    #[test]
    fn unsupported_route_returns_none() {
        let target = LaunchTarget::Addon {
            addon_id: AddonId::from("shell.terminal"),
        };

        assert_eq!(resolve_desktop_launch_target(&target), None);
    }

    #[test]
    fn settings_resolution_stays_on_home_panel() {
        assert_eq!(
            resolve_desktop_launch_target(&settings_launch_target()),
            Some(NativeDesktopLaunch::OpenSettingsPanel(None))
        );
        assert_ne!(
            resolve_desktop_launch_target(&settings_launch_target()),
            Some(NativeDesktopLaunch::OpenSettingsPanel(Some(
                NativeSettingsPanel::Connections
            )))
        );
    }
}
