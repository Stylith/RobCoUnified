mod addons;
mod paths;
mod profile;
mod registry;
mod runtime;
mod shell;

pub use addons::{
    AddonEntrypoint, AddonId, AddonKind, AddonManifest, AddonScope, AddonTrust, AppDefinition,
    CapabilityId, FileAssociation, PermissionId,
};
pub use paths::{LogicalRoot, PlatformPathEnvironment, PlatformPaths, ResolvedPlatformPaths};
pub use profile::{InstallProfile, IntegrationLevel};
pub use registry::{AddonRegistry, RegistryError};
pub use runtime::{
    RuntimeEnvironment, BASE_DIR_ENV, DEFAULT_PRODUCT_DIR, INSTALL_PROFILE_ENV,
    LEGACY_BASE_DIR_ENV, LEGACY_INSTALL_PROFILE_ENV, LEGACY_PRODUCT_DIR_ENV, PRODUCT_DIR_ENV,
};
pub use shell::{LaunchSurface, LaunchTarget, ShellAction, ShellEvent};
