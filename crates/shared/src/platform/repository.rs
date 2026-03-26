use super::addons::{AddonId, AddonManifest};
use super::profile::InstallProfile;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AddonRepositoryIndex {
    #[serde(default = "repository_index_schema_version")]
    pub schema_version: u32,
    #[serde(default)]
    pub generated_at: Option<String>,
    #[serde(default)]
    pub base_url: Option<String>,
    #[serde(default)]
    pub addons: Vec<IndexedAddonPackage>,
}

impl Default for AddonRepositoryIndex {
    fn default() -> Self {
        Self {
            schema_version: repository_index_schema_version(),
            generated_at: None,
            base_url: None,
            addons: Vec::new(),
        }
    }
}

impl AddonRepositoryIndex {
    pub fn addon(&self, addon_id: &AddonId) -> Option<&IndexedAddonPackage> {
        self.addons.iter().find(|addon| addon.manifest.id == *addon_id)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndexedAddonPackage {
    pub manifest: AddonManifest,
    #[serde(default)]
    pub releases: Vec<AddonRelease>,
}

impl IndexedAddonPackage {
    pub fn release(&self, version: &str) -> Option<&AddonRelease> {
        self.releases
            .iter()
            .find(|release| release.version == version)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AddonRelease {
    pub version: String,
    #[serde(default)]
    pub channel: Option<String>,
    #[serde(default)]
    pub artifacts: Vec<AddonArtifact>,
}

impl AddonRelease {
    pub fn artifact_for_profile(&self, profile: InstallProfile) -> Option<&AddonArtifact> {
        self.artifacts
            .iter()
            .find(|artifact| artifact.install_profile == Some(profile))
            .or_else(|| {
                self.artifacts
                    .iter()
                    .find(|artifact| artifact.install_profile.is_none())
            })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AddonArtifact {
    #[serde(default)]
    pub install_profile: Option<InstallProfile>,
    pub url: String,
    pub sha256: String,
    #[serde(default)]
    pub signature_url: Option<String>,
    #[serde(default)]
    pub size_bytes: Option<u64>,
    /// Recognized values: `"manifest-json"`, `"addon-dir"`, `"directory"`,
    /// `"zip"`, `"addon-zip"`, `"ndpkg"`, `"tar"`, `"tar-gz"`, `"tgz"`.
    /// `.ndpkg` is internally treated as ZIP.
    #[serde(default)]
    pub format: Option<String>,
}

const fn repository_index_schema_version() -> u32 {
    1
}

#[cfg(test)]
mod tests {
    use super::{AddonArtifact, AddonRelease, AddonRepositoryIndex, IndexedAddonPackage};
    use crate::platform::{AddonEntrypoint, AddonId, AddonKind, AddonManifest, InstallProfile};

    #[test]
    fn release_prefers_exact_profile_artifact_then_falls_back_to_generic() {
        let release = AddonRelease {
            version: "1.0.0".to_string(),
            channel: Some("stable".to_string()),
            artifacts: vec![
                AddonArtifact {
                    install_profile: None,
                    url: "https://example.invalid/addons/example-generic.zip".to_string(),
                    sha256: "generic".to_string(),
                    signature_url: None,
                    size_bytes: Some(12),
                    format: Some("zip".to_string()),
                },
                AddonArtifact {
                    install_profile: Some(InstallProfile::LinuxDesktop),
                    url: "https://example.invalid/addons/example-linux.zip".to_string(),
                    sha256: "linux".to_string(),
                    signature_url: None,
                    size_bytes: Some(34),
                    format: Some("zip".to_string()),
                },
            ],
        };

        assert_eq!(
            release
                .artifact_for_profile(InstallProfile::LinuxDesktop)
                .unwrap()
                .sha256,
            "linux"
        );
        assert_eq!(
            release
                .artifact_for_profile(InstallProfile::MacLauncher)
                .unwrap()
                .sha256,
            "generic"
        );
    }

    #[test]
    fn repository_index_round_trips_with_manifest_and_release_metadata() {
        let index = AddonRepositoryIndex {
            schema_version: 1,
            generated_at: Some("2026-03-25T10:30:00Z".to_string()),
            base_url: Some("https://example.invalid/addons/".to_string()),
            addons: vec![IndexedAddonPackage {
                manifest: AddonManifest::new(
                    "tools.example",
                    "Example Tool",
                    "1.0.0",
                    AddonKind::App,
                    AddonEntrypoint::StaticRoute {
                        route: "example".to_string(),
                    },
                ),
                releases: vec![AddonRelease {
                    version: "1.0.0".to_string(),
                    channel: Some("stable".to_string()),
                    artifacts: vec![AddonArtifact {
                        install_profile: Some(InstallProfile::LinuxDesktop),
                        url: "https://example.invalid/addons/example-linux.zip".to_string(),
                        sha256: "deadbeef".to_string(),
                        signature_url: Some(
                            "https://example.invalid/addons/example-linux.sig".to_string(),
                        ),
                        size_bytes: Some(128),
                        format: Some("zip".to_string()),
                    }],
                }],
            }],
        };

        let encoded = serde_json::to_string(&index).unwrap();
        let decoded: AddonRepositoryIndex = serde_json::from_str(&encoded).unwrap();

        assert_eq!(decoded.schema_version, 1);
        assert_eq!(
            decoded
                .addon(&AddonId::from("tools.example"))
                .unwrap()
                .release("1.0.0")
                .unwrap()
                .artifact_for_profile(InstallProfile::LinuxDesktop)
                .unwrap()
                .format
                .as_deref(),
            Some("zip")
        );
    }

    #[test]
    fn ndpkg_repository_index_round_trips_with_expected_addon_structure() {
        let raw = r#"{
            "schema_version": 1,
            "base_url": "https://raw.githubusercontent.com/Stylith/nucleon-desktop-addons/main/",
            "addons": [
                {
                    "manifest": {
                        "id": "games.red-menace", "display_name": "Red Menace",
                        "version": "0.4.4", "kind": "game", "scope": "user", "trust": "first-party", "essential": false,
                        "entrypoint": { "kind": "static-route", "route": "red-menace" }
                    },
                    "releases": [{ "version": "0.4.4", "artifacts": [{
                        "url": "games/games.red-menace.ndpkg",
                        "sha256": "aabb", "format": "ndpkg"
                    }]}]
                },
                {
                    "manifest": {
                        "id": "games.zeta-invaders", "display_name": "Zeta Invaders",
                        "version": "0.4.4", "kind": "game", "scope": "user", "trust": "first-party", "essential": false,
                        "entrypoint": { "kind": "wasm-module", "module": "addon.wasm", "protocol": "shell-surface-v1" }
                    },
                    "releases": [{ "version": "0.4.4", "artifacts": [{
                        "url": "games/games.zeta-invaders.ndpkg",
                        "sha256": "ccdd", "format": "ndpkg"
                    }]}]
                },
                {
                    "manifest": {
                        "id": "tools.nuke-codes", "display_name": "Nuke Codes",
                        "version": "0.4.4", "kind": "app", "scope": "user", "trust": "first-party", "essential": false,
                        "entrypoint": { "kind": "wasm-module", "module": "addon.wasm", "protocol": "shell-surface-v1" }
                    },
                    "releases": [{ "version": "0.4.4", "artifacts": [{
                        "url": "tools/tools.nuke-codes.ndpkg",
                        "sha256": "eeff", "format": "ndpkg"
                    }]}]
                }
            ]
        }"#;

        let index: AddonRepositoryIndex = serde_json::from_str(raw).unwrap();

        let ids: Vec<_> = index
            .addons
            .iter()
            .map(|a| a.manifest.id.as_str().to_string())
            .collect();
        assert_eq!(
            ids,
            vec!["games.red-menace", "games.zeta-invaders", "tools.nuke-codes"]
        );
        assert!(index.addons.iter().all(|a| !a.manifest.essential));
        assert!(index.addons.iter().all(|a| a.releases.first().is_some()));
        assert_eq!(
            index.addons.iter().map(|a| a.releases[0].artifacts[0].format.as_deref()).collect::<Vec<_>>(),
            vec![Some("ndpkg"), Some("ndpkg"), Some("ndpkg")]
        );
    }
}
