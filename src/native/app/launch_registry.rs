use super::super::desktop_app::DesktopWindow;
use super::super::menu::TerminalScreen;
use super::super::{
    first_party_addon_enabled, first_party_addon_registry, first_party_addon_registry_for_profile,
    first_party_addon_runtime, NativeDesktopRoute, NativeSettingsPanel, NativeTerminalRoute,
};
use crate::config::install_profile;
use crate::platform::{AddonId, CapabilityId, InstallProfile, LaunchTarget};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum NativeDesktopLaunch {
    OpenWindow(DesktopWindow),
    OpenNukeCodes,
    OpenSettingsPanel(Option<NativeSettingsPanel>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum NativeTerminalLaunch {
    OpenScreen(TerminalScreen),
    OpenEmbeddedTerminalShell,
    OpenDocumentBrowser,
    OpenEditor,
    OpenNukeCodes,
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

pub(super) fn terminal_launch_target() -> LaunchTarget {
    LaunchTarget::Capability {
        capability: CapabilityId::from("terminal-tool"),
    }
}

pub(super) fn installer_launch_target() -> LaunchTarget {
    LaunchTarget::Capability {
        capability: CapabilityId::from("installer-ui"),
    }
}

pub(super) fn programs_launch_target() -> LaunchTarget {
    LaunchTarget::Capability {
        capability: CapabilityId::from("app-catalog"),
    }
}

pub(super) fn default_apps_launch_target() -> LaunchTarget {
    LaunchTarget::Capability {
        capability: CapabilityId::from("default-apps-ui"),
    }
}

pub(super) fn connections_launch_target() -> LaunchTarget {
    LaunchTarget::Capability {
        capability: CapabilityId::from("connections-ui"),
    }
}

pub(super) fn edit_menus_launch_target() -> LaunchTarget {
    LaunchTarget::Capability {
        capability: CapabilityId::from("edit-menus-ui"),
    }
}

pub(super) fn about_launch_target() -> LaunchTarget {
    LaunchTarget::Capability {
        capability: CapabilityId::from("about-ui"),
    }
}

pub(super) fn resolve_desktop_launch_target(target: &LaunchTarget) -> Option<NativeDesktopLaunch> {
    resolve_desktop_launch_target_for_profile(target, install_profile())
}

pub(super) fn resolve_terminal_launch_target(
    target: &LaunchTarget,
) -> Option<NativeTerminalLaunch> {
    resolve_terminal_launch_target_for_profile(target, install_profile())
}

pub(super) fn resolve_desktop_launch_target_for_profile(
    target: &LaunchTarget,
    profile: InstallProfile,
) -> Option<NativeDesktopLaunch> {
    let registry = first_party_addon_registry_for_profile(profile);
    let addon_id = resolve_target_addon_id(target, &registry)?;

    first_party_addon_runtime(&addon_id)
        .and_then(|runtime| runtime.desktop_route)
        .map(resolve_runtime_route)
}

pub(super) fn resolve_terminal_launch_target_for_profile(
    target: &LaunchTarget,
    profile: InstallProfile,
) -> Option<NativeTerminalLaunch> {
    let registry = first_party_addon_registry_for_profile(profile);
    let addon_id = resolve_target_addon_id(target, &registry)?;

    first_party_addon_runtime(&addon_id)
        .and_then(|runtime| runtime.terminal_route)
        .map(resolve_terminal_runtime_route)
}

pub(super) fn unresolved_launch_target_status(target: &LaunchTarget) -> String {
    unresolved_launch_target_status_for_profile(target, install_profile())
}

pub(super) fn unresolved_terminal_launch_target_status(target: &LaunchTarget) -> String {
    unresolved_terminal_launch_target_status_for_profile(target, install_profile())
}

pub(super) fn unresolved_launch_target_status_for_profile(
    target: &LaunchTarget,
    profile: InstallProfile,
) -> String {
    unresolved_launch_target_status_for_profile_with_surface(target, profile, "desktop")
}

pub(super) fn unresolved_terminal_launch_target_status_for_profile(
    target: &LaunchTarget,
    profile: InstallProfile,
) -> String {
    unresolved_launch_target_status_for_profile_with_surface(target, profile, "terminal")
}

fn unresolved_launch_target_status_for_profile_with_surface(
    target: &LaunchTarget,
    profile: InstallProfile,
    surface: &str,
) -> String {
    let registry = first_party_addon_registry();
    if let Some(addon_id) = resolve_target_addon_id(target, &registry) {
        if !first_party_addon_enabled(profile, &addon_id) {
            return format!(
                "Addon '{addon_id}' is not enabled for install profile {:?}.",
                profile
            );
        }
    }

    match target {
        LaunchTarget::Addon { addon_id } => {
            format!("Addon '{addon_id}' is not wired into the {surface} launcher yet.")
        }
        LaunchTarget::Capability { capability } => {
            format!("Capability '{capability}' is not wired into the {surface} launcher yet.")
        }
    }
}

fn resolve_target_addon_id(
    target: &LaunchTarget,
    registry: &crate::platform::AddonRegistry,
) -> Option<AddonId> {
    match target {
        LaunchTarget::Addon { addon_id } => registry
            .manifest(addon_id)
            .map(|manifest| manifest.id.clone()),
        LaunchTarget::Capability { capability } => registry
            .by_capability(capability)
            .into_iter()
            .next()
            .map(|manifest| manifest.id.clone()),
    }
}

fn resolve_runtime_route(route: NativeDesktopRoute) -> NativeDesktopLaunch {
    match route {
        NativeDesktopRoute::OpenWindow(window) => NativeDesktopLaunch::OpenWindow(window),
        NativeDesktopRoute::OpenNukeCodes => NativeDesktopLaunch::OpenNukeCodes,
        NativeDesktopRoute::OpenSettingsPanel(panel) => {
            NativeDesktopLaunch::OpenSettingsPanel(panel)
        }
    }
}

fn resolve_terminal_runtime_route(route: NativeTerminalRoute) -> NativeTerminalLaunch {
    match route {
        NativeTerminalRoute::OpenScreen(screen) => NativeTerminalLaunch::OpenScreen(screen),
        NativeTerminalRoute::OpenEmbeddedTerminalShell => {
            NativeTerminalLaunch::OpenEmbeddedTerminalShell
        }
        NativeTerminalRoute::OpenDocumentBrowser => NativeTerminalLaunch::OpenDocumentBrowser,
        NativeTerminalRoute::OpenEditor => NativeTerminalLaunch::OpenEditor,
        NativeTerminalRoute::OpenNukeCodes => NativeTerminalLaunch::OpenNukeCodes,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        about_launch_target, connections_launch_target, default_apps_launch_target,
        edit_menus_launch_target, editor_launch_target, file_manager_launch_target,
        installer_launch_target, nuke_codes_launch_target, programs_launch_target,
        resolve_desktop_launch_target, resolve_desktop_launch_target_for_profile,
        resolve_terminal_launch_target, resolve_terminal_launch_target_for_profile,
        settings_launch_target, terminal_launch_target,
        unresolved_launch_target_status_for_profile,
        unresolved_terminal_launch_target_status_for_profile, NativeDesktopLaunch,
        NativeTerminalLaunch,
    };
    use crate::native::desktop_app::DesktopWindow;
    use crate::native::menu::TerminalScreen;
    use crate::native::NativeSettingsPanel;
    use crate::platform::{AddonId, InstallProfile, LaunchTarget};

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
    fn installer_capability_resolves_to_installer_window() {
        assert_eq!(
            resolve_desktop_launch_target(&installer_launch_target()),
            Some(NativeDesktopLaunch::OpenWindow(DesktopWindow::Installer))
        );
    }

    #[test]
    fn terminal_capability_resolves_to_terminal_window() {
        assert_eq!(
            resolve_desktop_launch_target(&terminal_launch_target()),
            Some(NativeDesktopLaunch::OpenWindow(DesktopWindow::TerminalMode))
        );
    }

    #[test]
    fn programs_capability_resolves_to_applications_window() {
        assert_eq!(
            resolve_desktop_launch_target(&programs_launch_target()),
            Some(NativeDesktopLaunch::OpenWindow(DesktopWindow::Applications))
        );
    }

    #[test]
    fn default_apps_capability_resolves_to_settings_panel() {
        assert_eq!(
            resolve_desktop_launch_target(&default_apps_launch_target()),
            Some(NativeDesktopLaunch::OpenSettingsPanel(Some(
                NativeSettingsPanel::DefaultApps
            )))
        );
    }

    #[test]
    fn connections_capability_resolves_to_settings_panel() {
        assert_eq!(
            resolve_desktop_launch_target(&connections_launch_target()),
            Some(NativeDesktopLaunch::OpenSettingsPanel(Some(
                NativeSettingsPanel::Connections
            )))
        );
    }

    #[test]
    fn edit_menus_capability_resolves_to_settings_panel() {
        assert_eq!(
            resolve_desktop_launch_target(&edit_menus_launch_target()),
            Some(NativeDesktopLaunch::OpenSettingsPanel(Some(
                NativeSettingsPanel::EditMenus
            )))
        );
    }

    #[test]
    fn about_capability_resolves_to_settings_panel() {
        assert_eq!(
            resolve_desktop_launch_target(&about_launch_target()),
            Some(NativeDesktopLaunch::OpenSettingsPanel(Some(
                NativeSettingsPanel::About
            )))
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
    fn installer_addon_id_resolves_to_installer_window() {
        let target = LaunchTarget::Addon {
            addon_id: AddonId::from("shell.installer"),
        };

        assert_eq!(
            resolve_desktop_launch_target(&target),
            Some(NativeDesktopLaunch::OpenWindow(DesktopWindow::Installer))
        );
    }

    #[test]
    fn document_browser_addon_without_desktop_runtime_returns_none() {
        let target = LaunchTarget::Addon {
            addon_id: AddonId::from("shell.document-browser"),
        };

        assert_eq!(resolve_desktop_launch_target(&target), None);
    }

    #[test]
    fn terminal_settings_capability_resolves_to_settings_screen() {
        assert_eq!(
            resolve_terminal_launch_target(&settings_launch_target()),
            Some(NativeTerminalLaunch::OpenScreen(TerminalScreen::Settings))
        );
    }

    #[test]
    fn terminal_programs_capability_resolves_to_applications_screen() {
        assert_eq!(
            resolve_terminal_launch_target(&programs_launch_target()),
            Some(NativeTerminalLaunch::OpenScreen(
                TerminalScreen::Applications
            ))
        );
    }

    #[test]
    fn terminal_file_manager_capability_resolves_to_document_browser_route() {
        assert_eq!(
            resolve_terminal_launch_target(&file_manager_launch_target()),
            Some(NativeTerminalLaunch::OpenDocumentBrowser)
        );
    }

    #[test]
    fn terminal_tool_capability_resolves_to_terminal_shell_route() {
        assert_eq!(
            resolve_terminal_launch_target(&terminal_launch_target()),
            Some(NativeTerminalLaunch::OpenEmbeddedTerminalShell)
        );
    }

    #[test]
    fn terminal_document_browser_addon_without_runtime_returns_none() {
        let target = LaunchTarget::Addon {
            addon_id: AddonId::from("shell.document-browser"),
        };

        assert_eq!(resolve_terminal_launch_target(&target), None);
    }

    #[test]
    fn connections_capability_is_disabled_for_mac_launcher() {
        assert_eq!(
            resolve_desktop_launch_target_for_profile(
                &connections_launch_target(),
                InstallProfile::MacLauncher
            ),
            None
        );
    }

    #[test]
    fn terminal_connections_capability_is_disabled_for_mac_launcher() {
        assert_eq!(
            resolve_terminal_launch_target_for_profile(
                &connections_launch_target(),
                InstallProfile::MacLauncher
            ),
            None
        );
    }

    #[test]
    fn disabled_profile_status_reports_addon_policy() {
        assert_eq!(
            unresolved_launch_target_status_for_profile(
                &connections_launch_target(),
                InstallProfile::MacLauncher
            ),
            "Addon 'shell.connections' is not enabled for install profile MacLauncher."
        );
    }

    #[test]
    fn terminal_disabled_profile_status_reports_addon_policy() {
        assert_eq!(
            unresolved_terminal_launch_target_status_for_profile(
                &connections_launch_target(),
                InstallProfile::MacLauncher
            ),
            "Addon 'shell.connections' is not enabled for install profile MacLauncher."
        );
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
