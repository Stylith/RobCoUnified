use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fmt;

macro_rules! string_id {
    ($name:ident) => {
        #[derive(
            Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
        )]
        #[serde(transparent)]
        pub struct $name(pub String);

        impl $name {
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl From<&str> for $name {
            fn from(value: &str) -> Self {
                Self(value.to_string())
            }
        }

        impl From<String> for $name {
            fn from(value: String) -> Self {
                Self(value)
            }
        }

        impl AsRef<str> for $name {
            fn as_ref(&self) -> &str {
                self.as_str()
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(self.as_str())
            }
        }
    };
}

string_id!(AddonId);
string_id!(CapabilityId);
string_id!(PermissionId);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AddonKind {
    App,
    Theme,
    ContentPack,
    Game,
    Service,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AddonScope {
    Bundled,
    System,
    User,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AddonTrust {
    FirstParty,
    ThirdParty,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "kind")]
pub enum AddonEntrypoint {
    StaticRoute { route: String },
    StandaloneBinary { binary: String, package: Option<String> },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileAssociation {
    pub capability: CapabilityId,
    #[serde(default)]
    pub extensions: Vec<String>,
    #[serde(default)]
    pub mime_types: Vec<String>,
    #[serde(default = "default_file_association_rank")]
    pub rank: u16,
}

impl FileAssociation {
    pub fn new<I, S>(capability: impl Into<CapabilityId>, extensions: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        Self {
            capability: capability.into(),
            extensions: extensions
                .into_iter()
                .map(|extension| normalize_extension(extension.as_ref()))
                .collect(),
            mime_types: Vec::new(),
            rank: default_file_association_rank(),
        }
    }

    pub fn matches_extension(&self, extension: &str) -> bool {
        let normalized = normalize_extension(extension);
        self.extensions.iter().any(|candidate| candidate == &normalized)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AddonManifest {
    pub id: AddonId,
    pub display_name: String,
    pub version: String,
    pub kind: AddonKind,
    pub scope: AddonScope,
    pub trust: AddonTrust,
    #[serde(default = "default_enabled_by_default")]
    pub enabled_by_default: bool,
    #[serde(default)]
    pub capabilities: BTreeSet<CapabilityId>,
    #[serde(default)]
    pub permissions: BTreeSet<PermissionId>,
    #[serde(default)]
    pub file_associations: Vec<FileAssociation>,
    pub entrypoint: AddonEntrypoint,
}

impl AddonManifest {
    pub fn new(
        id: impl Into<AddonId>,
        display_name: impl Into<String>,
        version: impl Into<String>,
        kind: AddonKind,
        entrypoint: AddonEntrypoint,
    ) -> Self {
        Self {
            id: id.into(),
            display_name: display_name.into(),
            version: version.into(),
            kind,
            scope: AddonScope::Bundled,
            trust: AddonTrust::FirstParty,
            enabled_by_default: default_enabled_by_default(),
            capabilities: BTreeSet::new(),
            permissions: BTreeSet::new(),
            file_associations: Vec::new(),
            entrypoint,
        }
    }

    pub fn with_capability(mut self, capability: impl Into<CapabilityId>) -> Self {
        self.capabilities.insert(capability.into());
        self
    }

    pub fn with_permission(mut self, permission: impl Into<PermissionId>) -> Self {
        self.permissions.insert(permission.into());
        self
    }

    pub fn with_file_association(mut self, association: FileAssociation) -> Self {
        self.file_associations.push(association);
        self
    }

    pub fn provides_capability(&self, capability: &CapabilityId) -> bool {
        self.capabilities.contains(capability)
    }
}

pub trait AppDefinition {
    fn manifest(&self) -> &AddonManifest;
}

const fn default_enabled_by_default() -> bool {
    true
}

const fn default_file_association_rank() -> u16 {
    100
}

fn normalize_extension(extension: &str) -> String {
    extension
        .trim()
        .trim_start_matches('.')
        .to_ascii_lowercase()
}
