use super::addons::{AddonManifest, AddonScope};
use super::paths::PlatformPaths;
use super::registry::{AddonRegistry, RegistryError};
use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

const DIRECTORY_MANIFEST_FILES: [&str; 2] = ["addon.json", "manifest.json"];
const ROOT_BUNDLED_ADDONS_DIR: &str = "bundled-addons";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddonManifestRoot {
    pub scope: AddonScope,
    pub path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveredAddonManifest {
    pub manifest: AddonManifest,
    pub manifest_path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddonManifestLoadIssue {
    pub scope: AddonScope,
    pub manifest_path: PathBuf,
    pub detail: String,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct AddonManifestDiscovery {
    pub manifests: Vec<DiscoveredAddonManifest>,
    pub issues: Vec<AddonManifestLoadIssue>,
}

impl AddonManifestDiscovery {
    pub fn into_manifests(self) -> Vec<AddonManifest> {
        self.manifests
            .into_iter()
            .map(|discovered| discovered.manifest)
            .collect()
    }

    pub fn is_clean(&self) -> bool {
        self.issues.is_empty()
    }
}

pub fn addon_manifest_roots(paths: &impl PlatformPaths) -> [AddonManifestRoot; 3] {
    [
        AddonManifestRoot {
            scope: AddonScope::Bundled,
            path: paths.core_root().join(ROOT_BUNDLED_ADDONS_DIR),
        },
        AddonManifestRoot {
            scope: AddonScope::System,
            path: paths.system_addons_root().to_path_buf(),
        },
        AddonManifestRoot {
            scope: AddonScope::User,
            path: paths.user_addons_root().to_path_buf(),
        },
    ]
}

pub fn discover_addon_manifests(paths: &impl PlatformPaths) -> AddonManifestDiscovery {
    let mut discovery = AddonManifestDiscovery::default();
    for root in addon_manifest_roots(paths) {
        discover_addon_manifests_in_root(&root, &mut discovery);
    }
    discovery
}

pub fn addon_manifest_path(source: &Path) -> Option<PathBuf> {
    if source.is_file() && is_json_file(source) {
        return Some(source.to_path_buf());
    }

    if source.is_dir() {
        return directory_manifest_file(source);
    }

    None
}

pub fn build_layered_addon_registry<I, J>(layers: I) -> Result<AddonRegistry, RegistryError>
where
    I: IntoIterator<Item = J>,
    J: IntoIterator<Item = AddonManifest>,
{
    let mut manifests = BTreeMap::new();
    for layer in layers {
        for manifest in layer {
            manifests.insert(manifest.id.clone(), manifest);
        }
    }
    AddonRegistry::from_manifests(manifests.into_values())
}

fn discover_addon_manifests_in_root(
    root: &AddonManifestRoot,
    discovery: &mut AddonManifestDiscovery,
) {
    let Ok(entries) = read_root_entries(&root.path, root.scope, discovery) else {
        return;
    };

    for entry_path in entries {
        if entry_path.is_file() && is_json_file(&entry_path) {
            load_manifest_file(&entry_path, root.scope, discovery);
            continue;
        }

        if entry_path.is_dir() {
            if let Some(manifest_path) = directory_manifest_file(&entry_path) {
                load_manifest_file(&manifest_path, root.scope, discovery);
            }
        }
    }
}

fn read_root_entries(
    root: &Path,
    scope: AddonScope,
    discovery: &mut AddonManifestDiscovery,
) -> Result<Vec<PathBuf>, ()> {
    let entries = match fs::read_dir(root) {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Err(()),
        Err(error) => {
            discovery.issues.push(AddonManifestLoadIssue {
                scope,
                manifest_path: root.to_path_buf(),
                detail: format!("failed to read addon root: {error}"),
            });
            return Err(());
        }
    };

    let mut paths = entries
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .collect::<Vec<_>>();
    paths.sort();
    Ok(paths)
}

fn directory_manifest_file(dir: &Path) -> Option<PathBuf> {
    DIRECTORY_MANIFEST_FILES
        .iter()
        .map(|name| dir.join(name))
        .find(|candidate| candidate.is_file())
}

fn is_json_file(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("json"))
}

fn load_manifest_file(
    manifest_path: &Path,
    scope: AddonScope,
    discovery: &mut AddonManifestDiscovery,
) {
    let raw = match fs::read_to_string(manifest_path) {
        Ok(raw) => raw,
        Err(error) => {
            discovery.issues.push(AddonManifestLoadIssue {
                scope,
                manifest_path: manifest_path.to_path_buf(),
                detail: format!("failed to read addon manifest: {error}"),
            });
            return;
        }
    };

    match serde_json::from_str::<AddonManifest>(&raw) {
        Ok(manifest) => discovery.manifests.push(DiscoveredAddonManifest {
            manifest: manifest.with_scope(scope),
            manifest_path: manifest_path.to_path_buf(),
        }),
        Err(error) => discovery.issues.push(AddonManifestLoadIssue {
            scope,
            manifest_path: manifest_path.to_path_buf(),
            detail: format!("failed to parse addon manifest: {error}"),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::{addon_manifest_roots, build_layered_addon_registry, discover_addon_manifests};
    use crate::platform::{
        AddonEntrypoint, AddonKind, AddonManifest, AddonScope, InstallProfile,
        PlatformPathEnvironment, ResolvedPlatformPaths,
    };
    use std::fs;
    use std::path::Path;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn discovery_reads_scoped_manifest_roots_and_applies_scope() {
        let paths = temp_portable_paths("discovery_reads_scoped_manifest_roots_and_applies_scope");
        let roots = addon_manifest_roots(&paths);
        fs::create_dir_all(&roots[0].path).unwrap();
        fs::create_dir_all(roots[1].path.join("system-addon")).unwrap();
        fs::create_dir_all(roots[2].path.join("user-addon")).unwrap();

        write_manifest(
            &roots[0].path.join("bundled-settings.json"),
            manifest("shell.settings", "Bundled Settings").with_scope(AddonScope::User),
        );
        write_manifest(
            &roots[1].path.join("system-addon").join("addon.json"),
            manifest("shell.installer", "System Installer"),
        );
        write_manifest(
            &roots[2].path.join("user-addon").join("manifest.json"),
            manifest("shell.editor", "User Editor"),
        );

        let discovery = discover_addon_manifests(&paths);

        assert!(discovery.is_clean());
        assert_eq!(discovery.manifests.len(), 3);
        assert_eq!(
            discovery
                .manifests
                .iter()
                .find(|manifest| manifest.manifest.id.as_str() == "shell.settings")
                .unwrap()
                .manifest
                .scope,
            AddonScope::Bundled
        );
        assert_eq!(
            discovery
                .manifests
                .iter()
                .find(|manifest| manifest.manifest.id.as_str() == "shell.installer")
                .unwrap()
                .manifest
                .scope,
            AddonScope::System
        );
        assert_eq!(
            discovery
                .manifests
                .iter()
                .find(|manifest| manifest.manifest.id.as_str() == "shell.editor")
                .unwrap()
                .manifest
                .scope,
            AddonScope::User
        );
    }

    #[test]
    fn discovery_records_invalid_json_without_stopping_other_manifests() {
        let paths =
            temp_portable_paths("discovery_records_invalid_json_without_stopping_other_manifests");
        let roots = addon_manifest_roots(&paths);
        fs::create_dir_all(&roots[2].path).unwrap();

        fs::write(roots[2].path.join("broken.json"), "{ not-json ").unwrap();
        write_manifest(
            &roots[2].path.join("editor.json"),
            manifest("shell.editor", "User Editor"),
        );

        let discovery = discover_addon_manifests(&paths);

        assert_eq!(discovery.manifests.len(), 1);
        assert_eq!(discovery.issues.len(), 1);
        assert_eq!(discovery.issues[0].scope, AddonScope::User);
        assert!(discovery.issues[0]
            .detail
            .contains("failed to parse addon manifest"));
    }

    #[test]
    fn layered_registry_prefers_later_manifest_layers() {
        let static_manifest = manifest("shell.editor", "Static Editor");
        let bundled_manifest = manifest("shell.editor", "Bundled Editor");
        let user_manifest = manifest("shell.editor", "User Editor").with_scope(AddonScope::User);
        let extra_manifest = manifest("shell.terminal", "Terminal");

        let registry = build_layered_addon_registry([
            vec![static_manifest],
            vec![bundled_manifest],
            vec![user_manifest, extra_manifest],
        ])
        .unwrap();

        assert_eq!(registry.len(), 2);
        assert_eq!(
            registry
                .manifest(&"shell.editor".into())
                .unwrap()
                .display_name,
            "User Editor"
        );
    }

    fn manifest(id: &str, display_name: &str) -> AddonManifest {
        AddonManifest::new(
            id,
            display_name,
            "0.1.0",
            AddonKind::App,
            AddonEntrypoint::StaticRoute {
                route: id.to_string(),
            },
        )
    }

    fn write_manifest(path: &Path, manifest: AddonManifest) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, serde_json::to_string_pretty(&manifest).unwrap()).unwrap();
    }

    fn temp_portable_paths(test_name: &str) -> ResolvedPlatformPaths {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root =
            std::env::temp_dir().join(format!("nucleon-addon-catalog-{test_name}-{unique}"));
        fs::create_dir_all(&root).unwrap();
        ResolvedPlatformPaths::from_environment(
            "nucleon",
            InstallProfile::PortableDev,
            PlatformPathEnvironment {
                home_dir: root.join("home"),
                data_dir: root.join("data"),
                data_local_dir: root.join("data-local"),
                cache_dir: root.join("cache"),
                runtime_dir: Some(root.join("runtime")),
                temp_dir: root.join("tmp"),
                portable_root: Some(root),
            },
        )
    }
}
