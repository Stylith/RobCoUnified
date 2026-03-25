use super::desktop_app::DesktopWindow;
use super::menu::TerminalScreen;
use super::NativeSettingsPanel;
use crate::config;
#[cfg(test)]
use crate::platform::CapabilityId;
use crate::platform::{
    build_layered_addon_registry, discover_addon_manifests, AddonEntrypoint, AddonId, AddonKind,
    AddonManifest, AddonManifestDiscovery, AddonRegistry, AddonScope, AddonStateOverrides,
    DiscoveredAddonManifest, FileAssociation, InstallProfile,
};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NativeDesktopRoute {
    OpenWindow(DesktopWindow),
    OpenNukeCodes,
    OpenSettingsPanel(Option<NativeSettingsPanel>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NativeTerminalRoute {
    OpenScreen(TerminalScreen),
    OpenEmbeddedTerminalShell,
    OpenDocumentBrowser,
    OpenEditor,
    OpenNukeCodes,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct FirstPartyAddonRuntime {
    pub addon_id: &'static str,
    pub desktop_route: Option<NativeDesktopRoute>,
    pub terminal_route: Option<NativeTerminalRoute>,
}

pub fn first_party_addon_manifests() -> Vec<AddonManifest> {
    vec![
        base_app_manifest("shell.settings", "Settings", "settings")
            .essential()
            .with_capability("settings-ui")
            .with_permission("settings.read")
            .with_permission("settings.write"),
        base_app_manifest("shell.file-manager", "File Manager", "file-manager")
            .essential()
            .with_capability("file-browser")
            .with_permission("filesystem.read")
            .with_permission("filesystem.write"),
        base_app_manifest("shell.editor", "Editor", "editor")
            .essential()
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
        .essential()
        .with_capability("document-viewer")
        .with_permission("filesystem.read")
        .with_file_association(FileAssociation::new(
            "document-viewer",
            ["pdf", "epub", "mobi", "azw", "azw3", "rtf"],
        )),
        base_app_manifest("shell.terminal", "Terminal", "terminal")
            .essential()
            .with_capability("terminal-tool")
            .with_permission("terminal.spawn")
            .with_permission("terminal.execute"),
        base_app_manifest("shell.installer", "Installer", "installer")
            .essential()
            .with_capability("installer-ui")
            .with_permission("addons.manage"),
        base_app_manifest("shell.programs", "Programs", "programs")
            .essential()
            .with_capability("app-catalog"),
        base_app_manifest("shell.default-apps", "Default Apps", "default-apps")
            .essential()
            .with_capability("default-apps-ui")
            .with_permission("settings.write"),
        base_app_manifest("shell.connections", "Connections", "connections")
            .essential()
            .with_capability("connections-ui")
            .with_permission("connections.inspect"),
        base_app_manifest("shell.edit-menus", "Edit Menus", "edit-menus")
            .essential()
            .with_capability("edit-menus-ui")
            .with_permission("settings.write"),
        base_app_manifest("shell.about", "About", "about")
            .essential()
            .with_capability("about-ui"),
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

pub fn discovered_addon_manifest_catalog() -> AddonManifestDiscovery {
    discover_addon_manifests(&config::platform_paths())
}

pub fn installed_addon_manifest_registry() -> AddonRegistry {
    let discovery = discovered_addon_manifest_catalog();
    build_layered_addon_registry([first_party_addon_manifests(), discovery.into_manifests()])
        .expect("layered addon manifest catalog must remain internally consistent")
}

pub fn addon_state_overrides() -> AddonStateOverrides {
    config::load_addon_state_overrides()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstalledAddonRecord {
    pub manifest: AddonManifest,
    pub manifest_path: Option<PathBuf>,
    pub explicit_enabled: Option<bool>,
    pub effective_enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstalledAddonInventorySections {
    pub essential: Vec<InstalledAddonRecord>,
    pub optional: Vec<InstalledAddonRecord>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FirstPartyAddonDisabledReason {
    InstallProfile,
    AddonState,
}

pub fn effective_addon_enabled(manifest: &AddonManifest) -> bool {
    effective_addon_enabled_with_overrides(manifest, &addon_state_overrides())
}

pub fn installed_addon_inventory() -> Vec<InstalledAddonRecord> {
    installed_addon_inventory_with_overrides(
        first_party_addon_manifests(),
        discovered_addon_manifest_catalog(),
        &addon_state_overrides(),
    )
}

pub fn installed_addon_inventory_sections() -> InstalledAddonInventorySections {
    let mut essential = Vec::new();
    let mut optional = Vec::new();
    for record in installed_addon_inventory() {
        if record.manifest.essential {
            essential.push(record);
        } else {
            optional.push(record);
        }
    }
    InstalledAddonInventorySections {
        essential,
        optional,
    }
}

pub fn installed_enabled_addon_manifest_registry() -> AddonRegistry {
    installed_enabled_addon_manifest_registry_with_overrides(
        &installed_addon_manifest_registry(),
        &addon_state_overrides(),
    )
}

pub fn set_addon_enabled_override(addon_id: AddonId, enabled: Option<bool>) -> Result<(), String> {
    let registry = installed_addon_manifest_registry();
    let Some(manifest) = registry.manifest(&addon_id) else {
        return Err(format!("Unknown addon '{addon_id}'."));
    };
    if manifest.essential && enabled == Some(false) {
        return Err(format!(
            "Addon '{addon_id}' is essential and cannot be disabled."
        ));
    }
    let mut overrides = addon_state_overrides();
    if manifest.essential {
        overrides.set_enabled(addon_id, None);
    } else {
        overrides.set_enabled(addon_id, enabled);
    }
    config::save_addon_state_overrides(&overrides);
    Ok(())
}

pub fn remove_installed_addon(addon_id: AddonId) -> Result<String, String> {
    let record = installed_addon_inventory()
        .into_iter()
        .find(|record| record.manifest.id == addon_id)
        .ok_or_else(|| format!("Unknown addon '{addon_id}'."))?;

    let mut overrides = addon_state_overrides();
    remove_installed_addon_record(&record, &config::user_addons_root_dir(), &mut overrides)?;
    overrides.set_enabled(addon_id, None);
    config::save_addon_state_overrides(&overrides);
    Ok(format!("Removed {}.", record.manifest.display_name))
}

fn effective_addon_enabled_with_overrides(
    manifest: &AddonManifest,
    overrides: &AddonStateOverrides,
) -> bool {
    if manifest.essential {
        return true;
    }
    overrides
        .enabled_for(&manifest.id)
        .unwrap_or(manifest.enabled_by_default)
}

fn installed_enabled_addon_manifest_registry_with_overrides(
    registry: &AddonRegistry,
    overrides: &AddonStateOverrides,
) -> AddonRegistry {
    AddonRegistry::from_manifests(
        registry
            .iter()
            .filter(|manifest| effective_addon_enabled_with_overrides(manifest, overrides))
            .cloned(),
    )
    .expect("effective enabled addon catalog must remain internally consistent")
}

fn installed_addon_inventory_with_overrides(
    static_manifests: Vec<AddonManifest>,
    discovery: AddonManifestDiscovery,
    overrides: &AddonStateOverrides,
) -> Vec<InstalledAddonRecord> {
    let mut by_id = BTreeMap::new();

    for manifest in static_manifests {
        let explicit_enabled = overrides.enabled_for(&manifest.id);
        let effective_enabled = effective_addon_enabled_with_overrides(&manifest, overrides);
        by_id.insert(
            manifest.id.clone(),
            InstalledAddonRecord {
                manifest,
                manifest_path: None,
                explicit_enabled,
                effective_enabled,
            },
        );
    }

    for DiscoveredAddonManifest {
        manifest,
        manifest_path,
    } in discovery.manifests
    {
        let explicit_enabled = overrides.enabled_for(&manifest.id);
        let effective_enabled = effective_addon_enabled_with_overrides(&manifest, overrides);
        by_id.insert(
            manifest.id.clone(),
            InstalledAddonRecord {
                manifest,
                manifest_path: Some(manifest_path),
                explicit_enabled,
                effective_enabled,
            },
        );
    }

    let mut records = by_id.into_values().collect::<Vec<_>>();
    records.sort_by(|left, right| {
        left.manifest
            .display_name
            .to_ascii_lowercase()
            .cmp(&right.manifest.display_name.to_ascii_lowercase())
            .then_with(|| left.manifest.id.cmp(&right.manifest.id))
    });
    records
}

fn remove_installed_addon_record(
    record: &InstalledAddonRecord,
    user_addons_root: &Path,
    overrides: &mut AddonStateOverrides,
) -> Result<(), String> {
    let manifest_path = record
        .manifest_path
        .as_ref()
        .ok_or_else(|| format!("Addon '{}' cannot be removed.", record.manifest.id))?;
    if record.manifest.scope != AddonScope::User {
        return Err(format!(
            "Addon '{}' is not a user-scoped addon and cannot be removed.",
            record.manifest.id
        ));
    }

    let canonical_root = std::fs::canonicalize(user_addons_root)
        .map_err(|error| format!("Failed to resolve user addons root: {error}"))?;
    let canonical_manifest = std::fs::canonicalize(manifest_path)
        .map_err(|error| format!("Failed to resolve addon manifest path: {error}"))?;

    if !canonical_manifest.starts_with(&canonical_root) {
        return Err(format!(
            "Addon '{}' is outside the user addons root and cannot be removed.",
            record.manifest.id
        ));
    }

    std::fs::remove_file(&canonical_manifest)
        .map_err(|error| format!("Failed to remove addon manifest: {error}"))?;
    remove_empty_parent_dirs(canonical_manifest.parent(), &canonical_root);
    overrides.set_enabled(record.manifest.id.clone(), None);
    Ok(())
}

fn remove_empty_parent_dirs(mut dir: Option<&Path>, stop_at: &Path) {
    while let Some(current) = dir {
        if current == stop_at {
            break;
        }
        let is_empty = std::fs::read_dir(current)
            .ok()
            .and_then(|mut entries| entries.next().transpose().ok())
            .is_some_and(|entry| entry.is_none());
        if !is_empty {
            break;
        }
        if std::fs::remove_dir(current).is_err() {
            break;
        }
        dir = current.parent();
    }
}

pub(crate) fn first_party_addon_enabled(profile: InstallProfile, addon_id: &AddonId) -> bool {
    first_party_addon_runtime(addon_id).is_some()
        && first_party_addon_disabled_reason(profile, addon_id).is_none()
}

pub(crate) fn first_party_addon_registry_for_profile(profile: InstallProfile) -> AddonRegistry {
    first_party_addon_registry_for_profile_with_registry(
        profile,
        &installed_enabled_addon_manifest_registry(),
    )
}

#[cfg(test)]
pub(crate) fn first_party_capability_enabled(
    profile: InstallProfile,
    capability: &CapabilityId,
) -> bool {
    !first_party_addon_registry_for_profile(profile)
        .by_capability(capability)
        .is_empty()
}

#[cfg(test)]
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

pub(crate) fn first_party_addon_disabled_reason(
    profile: InstallProfile,
    addon_id: &AddonId,
) -> Option<FirstPartyAddonDisabledReason> {
    first_party_addon_disabled_reason_with_registry(
        profile,
        addon_id,
        &installed_enabled_addon_manifest_registry(),
    )
}

fn first_party_addon_disabled_reason_with_registry(
    profile: InstallProfile,
    addon_id: &AddonId,
    enabled_registry: &AddonRegistry,
) -> Option<FirstPartyAddonDisabledReason> {
    if profile_disables_addon(profile, addon_id) {
        Some(FirstPartyAddonDisabledReason::InstallProfile)
    } else if first_party_addon_runtime(addon_id).is_some()
        && enabled_registry.manifest(addon_id).is_none()
    {
        Some(FirstPartyAddonDisabledReason::AddonState)
    } else {
        None
    }
}

fn first_party_addon_registry_for_profile_with_registry(
    profile: InstallProfile,
    enabled_registry: &AddonRegistry,
) -> AddonRegistry {
    AddonRegistry::from_manifests(
        enabled_registry
            .iter()
            .filter(|manifest| {
                first_party_addon_runtime(&manifest.id).is_some()
                    && !profile_disables_addon(profile, &manifest.id)
            })
            .cloned(),
    )
    .expect("profile-filtered first-party addon catalog must remain internally consistent")
}

fn profile_disables_addon(profile: InstallProfile, addon_id: &AddonId) -> bool {
    matches!(profile, InstallProfile::MacLauncher) && addon_id.as_str() == "shell.connections"
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
        terminal_route: Some(NativeTerminalRoute::OpenScreen(TerminalScreen::Settings)),
    },
    FirstPartyAddonRuntime {
        addon_id: "shell.file-manager",
        desktop_route: Some(NativeDesktopRoute::OpenWindow(DesktopWindow::FileManager)),
        terminal_route: Some(NativeTerminalRoute::OpenDocumentBrowser),
    },
    FirstPartyAddonRuntime {
        addon_id: "shell.editor",
        desktop_route: Some(NativeDesktopRoute::OpenWindow(DesktopWindow::Editor)),
        terminal_route: Some(NativeTerminalRoute::OpenEditor),
    },
    FirstPartyAddonRuntime {
        addon_id: "shell.document-browser",
        desktop_route: None,
        terminal_route: None,
    },
    FirstPartyAddonRuntime {
        addon_id: "shell.terminal",
        desktop_route: Some(NativeDesktopRoute::OpenWindow(DesktopWindow::TerminalMode)),
        terminal_route: Some(NativeTerminalRoute::OpenEmbeddedTerminalShell),
    },
    FirstPartyAddonRuntime {
        addon_id: "shell.installer",
        desktop_route: Some(NativeDesktopRoute::OpenWindow(DesktopWindow::Installer)),
        terminal_route: Some(NativeTerminalRoute::OpenScreen(
            TerminalScreen::ProgramInstaller,
        )),
    },
    FirstPartyAddonRuntime {
        addon_id: "shell.programs",
        desktop_route: Some(NativeDesktopRoute::OpenWindow(DesktopWindow::Applications)),
        terminal_route: Some(NativeTerminalRoute::OpenScreen(
            TerminalScreen::Applications,
        )),
    },
    FirstPartyAddonRuntime {
        addon_id: "shell.default-apps",
        desktop_route: Some(NativeDesktopRoute::OpenSettingsPanel(Some(
            NativeSettingsPanel::DefaultApps,
        ))),
        terminal_route: Some(NativeTerminalRoute::OpenScreen(TerminalScreen::DefaultApps)),
    },
    FirstPartyAddonRuntime {
        addon_id: "shell.connections",
        desktop_route: Some(NativeDesktopRoute::OpenSettingsPanel(Some(
            NativeSettingsPanel::Connections,
        ))),
        terminal_route: Some(NativeTerminalRoute::OpenScreen(TerminalScreen::Connections)),
    },
    FirstPartyAddonRuntime {
        addon_id: "shell.edit-menus",
        desktop_route: Some(NativeDesktopRoute::OpenSettingsPanel(Some(
            NativeSettingsPanel::EditMenus,
        ))),
        terminal_route: Some(NativeTerminalRoute::OpenScreen(TerminalScreen::EditMenus)),
    },
    FirstPartyAddonRuntime {
        addon_id: "shell.about",
        desktop_route: Some(NativeDesktopRoute::OpenSettingsPanel(Some(
            NativeSettingsPanel::About,
        ))),
        terminal_route: Some(NativeTerminalRoute::OpenScreen(TerminalScreen::About)),
    },
    FirstPartyAddonRuntime {
        addon_id: "games.red-menace",
        desktop_route: None,
        terminal_route: None,
    },
    FirstPartyAddonRuntime {
        addon_id: "games.zeta-invaders",
        desktop_route: None,
        terminal_route: None,
    },
    FirstPartyAddonRuntime {
        addon_id: "tools.nuke-codes",
        desktop_route: Some(NativeDesktopRoute::OpenNukeCodes),
        terminal_route: Some(NativeTerminalRoute::OpenNukeCodes),
    },
];

#[cfg(test)]
mod tests {
    use super::{
        effective_addon_enabled_with_overrides, first_party_addon_disabled_reason_with_registry,
        first_party_addon_enabled, first_party_addon_registry,
        first_party_addon_registry_for_profile,
        first_party_addon_registry_for_profile_with_registry, first_party_addon_runtime,
        first_party_capability_enabled_str, installed_addon_inventory_sections,
        installed_addon_inventory_with_overrides,
        installed_enabled_addon_manifest_registry_with_overrides,
    };
    use crate::platform::{
        AddonEntrypoint, AddonId, AddonKind, AddonManifest, AddonManifestDiscovery, AddonScope,
        AddonStateOverrides, CapabilityId, DiscoveredAddonManifest, InstallProfile,
    };
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

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

    #[test]
    fn effective_addon_enabled_uses_override_when_present() {
        let manifest = first_party_addon_registry()
            .manifest(&AddonId::from("tools.nuke-codes"))
            .cloned()
            .expect("nuke codes manifest");
        let mut overrides = AddonStateOverrides::default();

        assert!(effective_addon_enabled_with_overrides(
            &manifest, &overrides
        ));

        overrides.set_enabled(AddonId::from("tools.nuke-codes"), Some(false));
        assert!(!effective_addon_enabled_with_overrides(
            &manifest, &overrides
        ));
    }

    #[test]
    fn installed_enabled_registry_filters_disabled_addons() {
        let registry = first_party_addon_registry();
        let mut overrides = AddonStateOverrides::default();
        overrides.set_enabled(AddonId::from("tools.nuke-codes"), Some(false));
        let registry =
            installed_enabled_addon_manifest_registry_with_overrides(&registry, &overrides);

        assert!(registry
            .manifest(&AddonId::from("shell.settings"))
            .is_some());
        assert!(registry
            .manifest(&AddonId::from("tools.nuke-codes"))
            .is_none());
    }

    #[test]
    fn installed_inventory_prefers_discovered_manifest_and_applies_override() {
        let static_manifest =
            manifest("tools.nuke-codes", "Static Nuke Codes", AddonScope::Bundled);
        let discovered_manifest = manifest("tools.nuke-codes", "User Nuke Codes", AddonScope::User);
        let mut overrides = AddonStateOverrides::default();
        overrides.set_enabled(AddonId::from("tools.nuke-codes"), Some(false));

        let records = installed_addon_inventory_with_overrides(
            vec![static_manifest],
            AddonManifestDiscovery {
                manifests: vec![DiscoveredAddonManifest {
                    manifest: discovered_manifest,
                    manifest_path: PathBuf::from("/tmp/addons/nuke-codes/manifest.json"),
                }],
                issues: Vec::new(),
            },
            &overrides,
        );

        assert_eq!(records.len(), 1);
        let record = &records[0];
        assert_eq!(record.manifest.display_name, "User Nuke Codes");
        assert_eq!(record.manifest.scope, AddonScope::User);
        assert_eq!(
            record.manifest_path.as_deref(),
            Some(PathBuf::from("/tmp/addons/nuke-codes/manifest.json").as_path())
        );
        assert_eq!(record.explicit_enabled, Some(false));
        assert!(!record.effective_enabled);
    }

    #[test]
    fn installed_inventory_is_sorted_by_display_name_then_id() {
        let records = installed_addon_inventory_with_overrides(
            vec![
                manifest("shell.zeta", "Zeta", AddonScope::Bundled),
                manifest("shell.alpha-b", "Alpha", AddonScope::Bundled),
                manifest("shell.alpha-a", "Alpha", AddonScope::Bundled),
            ],
            AddonManifestDiscovery::default(),
            &AddonStateOverrides::default(),
        );

        let ids = records
            .iter()
            .map(|record| record.manifest.id.as_str().to_string())
            .collect::<Vec<_>>();

        assert_eq!(ids, vec!["shell.alpha-a", "shell.alpha-b", "shell.zeta"]);
    }

    #[test]
    fn addon_state_disabled_addon_is_removed_from_profile_registry() {
        let mut overrides = AddonStateOverrides::default();
        overrides.set_enabled(AddonId::from("tools.nuke-codes"), Some(false));
        let enabled_registry = installed_enabled_addon_manifest_registry_with_overrides(
            &first_party_addon_registry(),
            &overrides,
        );

        let registry = first_party_addon_registry_for_profile_with_registry(
            InstallProfile::LinuxDesktop,
            &enabled_registry,
        );

        assert!(registry
            .manifest(&AddonId::from("shell.settings"))
            .is_some());
        assert!(registry
            .manifest(&AddonId::from("tools.nuke-codes"))
            .is_none());
    }

    #[test]
    fn addon_state_disabled_reason_is_reported_separately_from_profile_policy() {
        let mut overrides = AddonStateOverrides::default();
        overrides.set_enabled(AddonId::from("tools.nuke-codes"), Some(false));
        let enabled_registry = installed_enabled_addon_manifest_registry_with_overrides(
            &first_party_addon_registry(),
            &overrides,
        );

        assert_eq!(
            first_party_addon_disabled_reason_with_registry(
                InstallProfile::LinuxDesktop,
                &AddonId::from("tools.nuke-codes"),
                &enabled_registry,
            ),
            Some(super::FirstPartyAddonDisabledReason::AddonState)
        );
        assert_eq!(
            first_party_addon_disabled_reason_with_registry(
                InstallProfile::MacLauncher,
                &AddonId::from("shell.connections"),
                &enabled_registry,
            ),
            Some(super::FirstPartyAddonDisabledReason::InstallProfile)
        );
    }

    #[test]
    fn essential_addons_ignore_disabled_override() {
        let manifest = first_party_addon_registry()
            .manifest(&AddonId::from("shell.settings"))
            .cloned()
            .expect("settings manifest");
        let mut overrides = AddonStateOverrides::default();
        overrides.set_enabled(AddonId::from("shell.settings"), Some(false));

        assert!(manifest.essential);
        assert!(effective_addon_enabled_with_overrides(
            &manifest, &overrides
        ));
    }

    #[test]
    fn installed_inventory_sections_split_essential_and_optional_addons() {
        let sections = installed_addon_inventory_sections();

        assert!(sections
            .essential
            .iter()
            .any(|record| record.manifest.id.as_str() == "shell.settings"));
        assert!(sections
            .optional
            .iter()
            .any(|record| record.manifest.id.as_str() == "tools.nuke-codes"));
        assert!(sections
            .optional
            .iter()
            .all(|record| !record.manifest.essential));
    }

    #[test]
    fn user_scoped_manifest_removal_deletes_manifest_and_clears_override() {
        let root = temp_dir("user_scoped_manifest_removal_deletes_manifest_and_clears_override");
        let addon_dir = root.join("sample-addon");
        fs::create_dir_all(&addon_dir).unwrap();
        let manifest_path = addon_dir.join("manifest.json");
        fs::write(&manifest_path, "{}").unwrap();
        let mut overrides = AddonStateOverrides::default();
        overrides.set_enabled(AddonId::from("addons.sample"), Some(false));
        let record = super::InstalledAddonRecord {
            manifest: manifest("addons.sample", "Sample Addon", AddonScope::User),
            manifest_path: Some(manifest_path.clone()),
            explicit_enabled: Some(false),
            effective_enabled: false,
        };

        super::remove_installed_addon_record(&record, &root, &mut overrides).unwrap();

        assert!(!manifest_path.exists());
        assert!(!addon_dir.exists());
        assert_eq!(overrides.enabled_for(&AddonId::from("addons.sample")), None);
    }

    #[test]
    fn non_user_or_static_addons_are_not_removable() {
        let root = temp_dir("non_user_or_static_addons_are_not_removable");
        let mut overrides = AddonStateOverrides::default();

        let bundled_record = super::InstalledAddonRecord {
            manifest: manifest("shell.settings", "Settings", AddonScope::Bundled).essential(),
            manifest_path: None,
            explicit_enabled: None,
            effective_enabled: true,
        };
        assert!(
            super::remove_installed_addon_record(&bundled_record, &root, &mut overrides).is_err()
        );

        let system_path = root.join("system-addon.json");
        fs::write(&system_path, "{}").unwrap();
        let system_record = super::InstalledAddonRecord {
            manifest: manifest("addons.system", "System Addon", AddonScope::System),
            manifest_path: Some(system_path),
            explicit_enabled: None,
            effective_enabled: true,
        };
        assert!(
            super::remove_installed_addon_record(&system_record, &root, &mut overrides).is_err()
        );
    }

    fn manifest(id: &str, display_name: &str, scope: AddonScope) -> AddonManifest {
        AddonManifest::new(
            id,
            display_name,
            "0.1.0",
            AddonKind::App,
            AddonEntrypoint::StaticRoute {
                route: id.to_string(),
            },
        )
        .with_scope(scope)
    }

    fn temp_dir(label: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("robcos-addon-tests-{label}-{unique}"));
        fs::create_dir_all(&dir).unwrap();
        dir
    }
}
