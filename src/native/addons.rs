use super::desktop_app::DesktopWindow;
use super::NativeSettingsPanel;
use crate::platform::{
    AddonEntrypoint, AddonId, AddonKind, AddonManifest, AddonRegistry, CapabilityId,
    FileAssociation, InstallProfile,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NativeDesktopRoute {
    OpenWindow(DesktopWindow),
    OpenNukeCodes,
    OpenSettingsPanel(Option<NativeSettingsPanel>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct FirstPartyAddonRuntime {
    pub addon_id: &'static str,
    pub desktop_route: Option<NativeDesktopRoute>,
}

pub fn first_party_addon_manifests() -> Vec<AddonManifest> {
    vec![
        base_app_manifest("shell.settings", "Settings", "settings")
            .with_capability("settings-ui")
            .with_permission("settings.read")
            .with_permission("settings.write"),
        base_app_manifest("shell.file-manager", "File Manager", "file-manager")
            .with_capability("file-browser")
            .with_permission("filesystem.read")
            .with_permission("filesystem.write"),
        base_app_manifest("shell.editor", "Editor", "editor")
            .with_capability("text-editor")
            .with_permission("filesystem.read")
            .with_permission("filesystem.write")
            .with_file_association(FileAssociation::new(
                "text-editor",
                ["txt", "md", "rs", "toml", "json", "yaml", "yml"],
            )),
        base_app_manifest(
            "shell.document-browser",
            "Document Browser",
            "document-browser",
        )
        .with_capability("document-viewer")
        .with_permission("filesystem.read")
        .with_file_association(FileAssociation::new(
            "document-viewer",
            ["pdf", "epub", "mobi", "azw", "azw3", "rtf"],
        )),
        base_app_manifest("shell.terminal", "Terminal", "terminal")
            .with_capability("terminal-tool")
            .with_permission("terminal.spawn")
            .with_permission("terminal.execute"),
        base_app_manifest("shell.installer", "Installer", "installer")
            .with_capability("installer-ui")
            .with_permission("addons.manage"),
        base_app_manifest("shell.programs", "Programs", "programs").with_capability("app-catalog"),
        base_app_manifest("shell.default-apps", "Default Apps", "default-apps")
            .with_capability("default-apps-ui")
            .with_permission("settings.write"),
        base_app_manifest("shell.connections", "Connections", "connections")
            .with_capability("connections-ui")
            .with_permission("connections.inspect"),
        base_app_manifest("shell.edit-menus", "Edit Menus", "edit-menus")
            .with_capability("edit-menus-ui")
            .with_permission("settings.write"),
        base_app_manifest("shell.about", "About", "about").with_capability("about-ui"),
        base_game_manifest("games.red-menace", "Red Menace", "red-menace")
            .with_capability("game-launcher"),
        base_game_manifest("games.zeta-invaders", "Zeta Invaders", "zeta-invaders")
            .with_capability("game-launcher"),
        base_app_manifest("tools.nuke-codes", "Nuke Codes", "nuke-codes")
            .with_capability("code-reference"),
    ]
}

pub fn first_party_addon_registry() -> AddonRegistry {
    AddonRegistry::from_manifests(first_party_addon_manifests())
        .expect("first-party addon catalog must remain internally consistent")
}

pub(crate) fn first_party_addon_enabled(profile: InstallProfile, addon_id: &AddonId) -> bool {
    match profile {
        InstallProfile::MacLauncher if addon_id.as_str() == "shell.connections" => false,
        _ => first_party_addon_runtime(addon_id).is_some(),
    }
}

pub(crate) fn first_party_addon_registry_for_profile(profile: InstallProfile) -> AddonRegistry {
    AddonRegistry::from_manifests(
        first_party_addon_manifests()
            .into_iter()
            .filter(|manifest| first_party_addon_enabled(profile, &manifest.id)),
    )
    .expect("profile-filtered first-party addon catalog must remain internally consistent")
}

pub(crate) fn first_party_capability_enabled(
    profile: InstallProfile,
    capability: &CapabilityId,
) -> bool {
    !first_party_addon_registry_for_profile(profile)
        .by_capability(capability)
        .is_empty()
}

pub(crate) fn first_party_capability_enabled_str(
    profile: InstallProfile,
    capability: &'static str,
) -> bool {
    first_party_capability_enabled(profile, &CapabilityId::from(capability))
}

pub(crate) fn first_party_addon_runtime(
    addon_id: &AddonId,
) -> Option<&'static FirstPartyAddonRuntime> {
    FIRST_PARTY_ADDON_RUNTIMES
        .iter()
        .find(|runtime| runtime.addon_id == addon_id.as_str())
}

fn base_app_manifest(id: &str, display_name: &str, route: &str) -> AddonManifest {
    AddonManifest::new(
        id,
        display_name,
        env!("CARGO_PKG_VERSION"),
        AddonKind::App,
        AddonEntrypoint::StaticRoute {
            route: route.to_string(),
        },
    )
}

fn base_game_manifest(id: &str, display_name: &str, route: &str) -> AddonManifest {
    AddonManifest::new(
        id,
        display_name,
        env!("CARGO_PKG_VERSION"),
        AddonKind::Game,
        AddonEntrypoint::StaticRoute {
            route: route.to_string(),
        },
    )
}

const FIRST_PARTY_ADDON_RUNTIMES: [FirstPartyAddonRuntime; 14] = [
    FirstPartyAddonRuntime {
        addon_id: "shell.settings",
        desktop_route: Some(NativeDesktopRoute::OpenSettingsPanel(None)),
    },
    FirstPartyAddonRuntime {
        addon_id: "shell.file-manager",
        desktop_route: Some(NativeDesktopRoute::OpenWindow(DesktopWindow::FileManager)),
    },
    FirstPartyAddonRuntime {
        addon_id: "shell.editor",
        desktop_route: Some(NativeDesktopRoute::OpenWindow(DesktopWindow::Editor)),
    },
    FirstPartyAddonRuntime {
        addon_id: "shell.document-browser",
        desktop_route: None,
    },
    FirstPartyAddonRuntime {
        addon_id: "shell.terminal",
        desktop_route: Some(NativeDesktopRoute::OpenWindow(DesktopWindow::TerminalMode)),
    },
    FirstPartyAddonRuntime {
        addon_id: "shell.installer",
        desktop_route: Some(NativeDesktopRoute::OpenWindow(DesktopWindow::Installer)),
    },
    FirstPartyAddonRuntime {
        addon_id: "shell.programs",
        desktop_route: Some(NativeDesktopRoute::OpenWindow(DesktopWindow::Applications)),
    },
    FirstPartyAddonRuntime {
        addon_id: "shell.default-apps",
        desktop_route: Some(NativeDesktopRoute::OpenSettingsPanel(Some(
            NativeSettingsPanel::DefaultApps,
        ))),
    },
    FirstPartyAddonRuntime {
        addon_id: "shell.connections",
        desktop_route: Some(NativeDesktopRoute::OpenSettingsPanel(Some(
            NativeSettingsPanel::Connections,
        ))),
    },
    FirstPartyAddonRuntime {
        addon_id: "shell.edit-menus",
        desktop_route: Some(NativeDesktopRoute::OpenSettingsPanel(Some(
            NativeSettingsPanel::EditMenus,
        ))),
    },
    FirstPartyAddonRuntime {
        addon_id: "shell.about",
        desktop_route: Some(NativeDesktopRoute::OpenSettingsPanel(Some(
            NativeSettingsPanel::About,
        ))),
    },
    FirstPartyAddonRuntime {
        addon_id: "games.red-menace",
        desktop_route: None,
    },
    FirstPartyAddonRuntime {
        addon_id: "games.zeta-invaders",
        desktop_route: None,
    },
    FirstPartyAddonRuntime {
        addon_id: "tools.nuke-codes",
        desktop_route: Some(NativeDesktopRoute::OpenNukeCodes),
    },
];

#[cfg(test)]
mod tests {
    use super::{
        first_party_addon_enabled, first_party_addon_registry,
        first_party_addon_registry_for_profile, first_party_addon_runtime,
        first_party_capability_enabled_str,
    };
    use crate::platform::{AddonId, CapabilityId, InstallProfile};

    #[test]
    fn first_party_registry_exposes_core_capabilities() {
        let registry = first_party_addon_registry();

        assert_eq!(
            registry
                .by_capability(&CapabilityId::from("settings-ui"))
                .len(),
            1
        );
        assert_eq!(
            registry
                .by_capability(&CapabilityId::from("file-browser"))
                .len(),
            1
        );
        assert_eq!(
            registry
                .by_capability(&CapabilityId::from("text-editor"))
                .len(),
            1
        );
    }

    #[test]
    fn first_party_runtime_registry_covers_manifest_catalog() {
        let registry = first_party_addon_registry();

        for manifest in registry.iter() {
            assert!(
                first_party_addon_runtime(&manifest.id).is_some(),
                "missing runtime entry for {}",
                manifest.id
            );
        }
    }

    #[test]
    fn first_party_runtime_registry_exposes_known_addon_ids() {
        assert!(first_party_addon_runtime(&AddonId::from("shell.editor")).is_some());
        assert!(first_party_addon_runtime(&AddonId::from("shell.installer")).is_some());
    }

    #[test]
    fn mac_launcher_policy_disables_connections_addon() {
        assert!(!first_party_addon_enabled(
            InstallProfile::MacLauncher,
            &AddonId::from("shell.connections")
        ));
        assert_eq!(
            first_party_addon_registry_for_profile(InstallProfile::MacLauncher)
                .by_capability(&CapabilityId::from("connections-ui"))
                .len(),
            0
        );
    }

    #[test]
    fn linux_desktop_policy_keeps_connections_addon_enabled() {
        assert!(first_party_addon_enabled(
            InstallProfile::LinuxDesktop,
            &AddonId::from("shell.connections")
        ));
        assert_eq!(
            first_party_addon_registry_for_profile(InstallProfile::LinuxDesktop)
                .by_capability(&CapabilityId::from("connections-ui"))
                .len(),
            1
        );
    }

    #[test]
    fn capability_helper_matches_profile_policy() {
        assert!(!first_party_capability_enabled_str(
            InstallProfile::MacLauncher,
            "connections-ui"
        ));
        assert!(first_party_capability_enabled_str(
            InstallProfile::LinuxDesktop,
            "connections-ui"
        ));
    }
}
