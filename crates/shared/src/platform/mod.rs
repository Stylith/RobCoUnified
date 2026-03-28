mod addons;
mod catalog;
mod hosted;
mod paths;
mod profile;
mod registry;
mod repository;
mod runtime;
mod shell;
mod state;

pub use addons::{
    AddonEntrypoint, AddonId, AddonKind, AddonManifest, AddonScope, AddonTrust, AppDefinition,
    CapabilityId, FileAssociation, PermissionId,
};
pub use catalog::{
    addon_manifest_path, addon_manifest_roots, build_layered_addon_registry,
    discover_addon_manifests, AddonManifestDiscovery, AddonManifestLoadIssue, AddonManifestRoot,
    DiscoveredAddonManifest,
};
pub use hosted::{
    HostedAddonFrame, HostedAddonInitRequest, HostedAddonProtocol, HostedAddonRequest,
    HostedAddonResponse, HostedAddonSize, HostedAddonSurface, HostedAddonUpdateRequest,
    HostedColor, HostedDrawCommand, HostedInputEvent, HostedPointerButton, HostedTextAlign,
};
pub use paths::{LogicalRoot, PlatformPathEnvironment, PlatformPaths, ResolvedPlatformPaths};
pub use profile::{InstallProfile, IntegrationLevel};
pub use registry::{AddonRegistry, RegistryError};
pub use repository::{AddonArtifact, AddonRelease, AddonRepositoryIndex, IndexedAddonPackage};
pub use runtime::{
    RuntimeEnvironment, RuntimePathLayout, StatePathLayout, BASE_DIR_ENV, DEFAULT_PRODUCT_DIR,
    INSTALL_PROFILE_ENV, PRODUCT_DIR_ENV,
};
pub use shell::{LaunchSurface, LaunchTarget, ShellAction, ShellEvent};
pub use state::{AddonStateOverride, AddonStateOverrides};
