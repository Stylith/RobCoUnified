use super::addons::{AddonId, AddonManifest, CapabilityId};
use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegistryError {
    DuplicateAddonId(AddonId),
}

impl fmt::Display for RegistryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateAddonId(addon_id) => {
                write!(f, "duplicate addon id registered: {addon_id}")
            }
        }
    }
}

impl Error for RegistryError {}

#[derive(Debug, Default, Clone)]
pub struct AddonRegistry {
    manifests: BTreeMap<AddonId, AddonManifest>,
    capability_index: BTreeMap<CapabilityId, Vec<AddonId>>,
    extension_index: BTreeMap<String, Vec<AddonId>>,
}

impl AddonRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_manifests<I>(manifests: I) -> Result<Self, RegistryError>
    where
        I: IntoIterator<Item = AddonManifest>,
    {
        let mut registry = Self::new();
        registry.register_many(manifests)?;
        Ok(registry)
    }

    pub fn register_many<I>(&mut self, manifests: I) -> Result<(), RegistryError>
    where
        I: IntoIterator<Item = AddonManifest>,
    {
        for manifest in manifests {
            self.register(manifest)?;
        }
        Ok(())
    }

    pub fn register(&mut self, manifest: AddonManifest) -> Result<(), RegistryError> {
        if self.manifests.contains_key(&manifest.id) {
            return Err(RegistryError::DuplicateAddonId(manifest.id));
        }

        let addon_id = manifest.id.clone();
        for capability in &manifest.capabilities {
            self.capability_index
                .entry(capability.clone())
                .or_default()
                .push(addon_id.clone());
        }
        for association in &manifest.file_associations {
            for extension in &association.extensions {
                self.extension_index
                    .entry(extension.clone())
                    .or_default()
                    .push(addon_id.clone());
            }
        }
        self.manifests.insert(addon_id, manifest);
        Ok(())
    }

    pub fn len(&self) -> usize {
        self.manifests.len()
    }

    pub fn is_empty(&self) -> bool {
        self.manifests.is_empty()
    }

    pub fn manifest(&self, addon_id: &AddonId) -> Option<&AddonManifest> {
        self.manifests.get(addon_id)
    }

    pub fn iter(&self) -> impl Iterator<Item = &AddonManifest> {
        self.manifests.values()
    }

    pub fn by_capability(&self, capability: &CapabilityId) -> Vec<&AddonManifest> {
        self.capability_index
            .get(capability)
            .into_iter()
            .flatten()
            .filter_map(|addon_id| self.manifests.get(addon_id))
            .collect()
    }

    pub fn by_file_extension(&self, extension: &str) -> Vec<&AddonManifest> {
        let normalized = extension
            .trim()
            .trim_start_matches('.')
            .to_ascii_lowercase();
        self.extension_index
            .get(&normalized)
            .into_iter()
            .flatten()
            .filter_map(|addon_id| self.manifests.get(addon_id))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::{AddonRegistry, CapabilityId, RegistryError};
    use crate::platform::{AddonEntrypoint, AddonKind, AddonManifest, FileAssociation};

    #[test]
    fn registry_rejects_duplicate_ids() {
        let manifest = AddonManifest::new(
            "shell.settings",
            "Settings",
            "0.1.0",
            AddonKind::App,
            AddonEntrypoint::StaticRoute {
                route: "settings".to_string(),
            },
        )
        .with_capability("settings-ui");

        let mut registry = AddonRegistry::new();
        registry.register(manifest.clone()).unwrap();
        let error = registry.register(manifest).unwrap_err();

        assert_eq!(
            error,
            RegistryError::DuplicateAddonId("shell.settings".into())
        );
    }

    #[test]
    fn registry_indexes_capabilities_and_extensions() {
        let editor = AddonManifest::new(
            "shell.editor",
            "Editor",
            "0.1.0",
            AddonKind::App,
            AddonEntrypoint::StaticRoute {
                route: "editor".to_string(),
            },
        )
        .with_capability("text-editor")
        .with_file_association(FileAssociation::new("text-editor", ["txt", ".md"]));
        let registry = AddonRegistry::from_manifests([editor]).unwrap();

        assert_eq!(
            registry
                .by_capability(&CapabilityId::from("text-editor"))
                .len(),
            1
        );
        assert_eq!(registry.by_file_extension("TXT").len(), 1);
    }
}
