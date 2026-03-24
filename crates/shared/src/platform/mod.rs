mod addons;
mod paths;
mod profile;
mod registry;
mod shell;

pub use addons::{
    AddonEntrypoint, AddonId, AddonKind, AddonManifest, AddonScope, AddonTrust, AppDefinition,
    CapabilityId, FileAssociation, PermissionId,
};
pub use paths::{LogicalRoot, PlatformPathEnvironment, PlatformPaths, ResolvedPlatformPaths};
pub use profile::{InstallProfile, IntegrationLevel};
pub use registry::{AddonRegistry, RegistryError};
pub use shell::{LaunchSurface, LaunchTarget, ShellAction, ShellEvent};
