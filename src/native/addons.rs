use crate::platform::{
    AddonEntrypoint, AddonKind, AddonManifest, AddonRegistry, FileAssociation,
};

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
        base_app_manifest("shell.document-browser", "Document Browser", "document-browser")
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
        base_app_manifest("shell.programs", "Programs", "programs")
            .with_capability("app-catalog"),
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

#[cfg(test)]
mod tests {
    use super::first_party_addon_registry;
    use crate::platform::CapabilityId;

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
}
