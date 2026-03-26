use super::desktop_app::DesktopWindow;
use super::menu::TerminalScreen;
use super::NativeSettingsPanel;
use crate::config;
#[cfg(test)]
use crate::platform::CapabilityId;
use crate::platform::{
    addon_manifest_path, build_layered_addon_registry, discover_addon_manifests, AddonEntrypoint,
    AddonArtifact, AddonId, AddonKind, AddonManifest, AddonManifestDiscovery,
    AddonManifestLoadIssue, AddonRegistry, AddonRepositoryIndex, AddonRelease, AddonScope,
    AddonStateOverrides, DiscoveredAddonManifest, FileAssociation, HostedAddonProtocol,
    InstallProfile,
};
use flate2::read::GzDecoder;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::ffi::OsStr;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};
use tar::Archive as TarArchive;
use zip::ZipArchive;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NativeDesktopRoute {
    OpenWindow(DesktopWindow),
    OpenSettingsPanel(Option<NativeSettingsPanel>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NativeTerminalRoute {
    OpenScreen(TerminalScreen),
    OpenEmbeddedTerminalShell,
    OpenDocumentBrowser,
    OpenEditor,
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
    pub issues: Vec<AddonManifestLoadIssue>,
    pub repository_available: Vec<RepositoryAddonRecord>,
    pub repository_source: Option<PathBuf>,
    pub repository_issue: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepositoryAddonRecord {
    pub manifest: AddonManifest,
    pub release: Option<AddonRelease>,
    pub repository_source: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstalledHostedAddonProcess {
    pub addon_id: AddonId,
    pub protocol: HostedAddonProtocol,
    pub executable_path: PathBuf,
    pub bundle_dir: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstalledWasmAddonModule {
    pub addon_id: AddonId,
    pub protocol: HostedAddonProtocol,
    pub module_path: PathBuf,
    pub bundle_dir: PathBuf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepositoryAddonAction {
    Install,
    Update,
    Reinstall,
}

impl RepositoryAddonAction {
    pub fn label(self) -> &'static str {
        match self {
            Self::Install => "Install",
            Self::Update => "Update",
            Self::Reinstall => "Reinstall",
        }
    }
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

pub fn installed_addon_bundle_dir(addon_id: &AddonId) -> Option<PathBuf> {
    installed_addon_inventory()
        .into_iter()
        .find(|record| &record.manifest.id == addon_id)
        .and_then(|record| record.manifest_path)
        .and_then(|manifest_path| manifest_path.parent().map(Path::to_path_buf))
}

pub fn installed_addon_bundle_path(
    addon_id: &AddonId,
    relative_path: impl AsRef<Path>,
) -> Option<PathBuf> {
    installed_addon_bundle_dir(addon_id).map(|dir| dir.join(relative_path))
}

pub fn installed_hosted_addon_process(addon_id: &AddonId) -> Option<InstalledHostedAddonProcess> {
    let record = installed_addon_inventory()
        .into_iter()
        .find(|record| &record.manifest.id == addon_id)?;
    installed_hosted_addon_process_from_record(record)
}

pub fn installed_wasm_addon_module(addon_id: &AddonId) -> Option<InstalledWasmAddonModule> {
    let record = installed_addon_inventory()
        .into_iter()
        .find(|record| &record.manifest.id == addon_id)?;
    installed_wasm_addon_module_from_record(record)
}

pub fn installed_wasm_addon_module_by_display_name(
    display_name: &str,
) -> Option<InstalledWasmAddonModule> {
    installed_addon_inventory()
        .into_iter()
        .find(|record| {
            record.effective_enabled
                && record.manifest.display_name == display_name
                && matches!(record.manifest.entrypoint, AddonEntrypoint::WasmModule { .. })
        })
        .and_then(installed_wasm_addon_module_from_record)
}

pub fn installed_hosted_game_names() -> Vec<String> {
    hosted_game_names_from_registry(&installed_enabled_addon_manifest_registry())
}

pub fn installed_hosted_application_names() -> Vec<String> {
    hosted_application_names_from_registry(&installed_enabled_addon_manifest_registry())
}

pub fn is_installed_hosted_game(name: &str) -> bool {
    installed_hosted_game_names()
        .iter()
        .any(|candidate| candidate == name)
}

fn hosted_game_names_from_registry(registry: &AddonRegistry) -> Vec<String> {
    let mut names = registry
        .iter()
        .filter(|manifest| manifest.kind == AddonKind::Game)
        .filter(|manifest| is_hosted_addon_entrypoint(&manifest.entrypoint))
        .map(|manifest| manifest.display_name.clone())
        .collect::<Vec<_>>();
    names.sort();
    names
}

fn hosted_application_names_from_registry(registry: &AddonRegistry) -> Vec<String> {
    let mut names = registry
        .iter()
        .filter(|manifest| manifest.kind == AddonKind::App)
        .filter(|manifest| !manifest.essential)
        .filter(|manifest| is_hosted_addon_entrypoint(&manifest.entrypoint))
        .map(|manifest| manifest.display_name.clone())
        .collect::<Vec<_>>();
    names.sort();
    names
}

fn is_hosted_addon_entrypoint(entrypoint: &AddonEntrypoint) -> bool {
    matches!(
        entrypoint,
        AddonEntrypoint::WasmModule { .. } | AddonEntrypoint::HostedProcess { .. }
    )
}

fn installed_hosted_addon_process_from_record(
    record: InstalledAddonRecord,
) -> Option<InstalledHostedAddonProcess> {
    let AddonEntrypoint::HostedProcess {
        executable,
        protocol,
    } = &record.manifest.entrypoint
    else {
        return None;
    };
    let bundle_dir = record
        .manifest_path
        .as_ref()
        .and_then(|manifest_path| manifest_path.parent().map(Path::to_path_buf))?;
    Some(InstalledHostedAddonProcess {
        addon_id: record.manifest.id,
        protocol: *protocol,
        executable_path: bundle_dir.join(executable),
        bundle_dir,
    })
}

fn installed_wasm_addon_module_from_record(
    record: InstalledAddonRecord,
) -> Option<InstalledWasmAddonModule> {
    let AddonEntrypoint::WasmModule { module, protocol } = &record.manifest.entrypoint else {
        return None;
    };
    let bundle_dir = record
        .manifest_path
        .as_ref()
        .and_then(|manifest_path| manifest_path.parent().map(Path::to_path_buf))?;
    Some(InstalledWasmAddonModule {
        addon_id: record.manifest.id,
        protocol: *protocol,
        module_path: bundle_dir.join(module),
        bundle_dir,
    })
}

pub fn installed_addon_inventory_sections() -> InstalledAddonInventorySections {
    let discovery = discovered_addon_manifest_catalog();
    installed_addon_inventory_sections_with_overrides(
        first_party_addon_manifests(),
        discovery,
        &addon_state_overrides(),
    )
}

fn installed_addon_inventory_sections_with_overrides(
    static_manifests: Vec<AddonManifest>,
    discovery: AddonManifestDiscovery,
    overrides: &AddonStateOverrides,
) -> InstalledAddonInventorySections {
    let mut essential = Vec::new();
    let mut optional = Vec::new();
    let issues = discovery.issues.clone();
    let records = installed_addon_inventory_with_overrides(static_manifests, discovery, overrides);
    for record in &records {
        if record.manifest.essential {
            essential.push(record.clone());
        } else {
            optional.push(record.clone());
        }
    }
    let installed_ids = records
        .iter()
        .map(|record| record.manifest.id.clone())
        .collect::<Vec<_>>();
    let (repository_available, repository_source, repository_issue) =
        repository_addon_inventory(&installed_ids, config::install_profile());
    InstalledAddonInventorySections {
        essential,
        optional,
        issues,
        repository_available,
        repository_source,
        repository_issue,
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

pub fn install_user_addon(source_path: impl AsRef<Path>) -> Result<String, String> {
    let mut overrides = addon_state_overrides();
    let message = install_user_addon_at_path(
        source_path.as_ref(),
        &config::user_addons_root_dir(),
        &mut overrides,
    )?;
    config::save_addon_state_overrides(&overrides);
    Ok(message)
}

pub fn install_repository_addon(addon_id: AddonId) -> Result<String, String> {
    let (index, source_path) = config::load_addon_repository_index()
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "No addon repository index is available.".to_string())?;
    install_repository_addon_from_index(
        &index,
        &source_path,
        &addon_id,
        config::install_profile(),
        &config::addon_downloads_cache_dir(),
        &config::user_addons_root_dir(),
    )
}

pub fn repository_sync_action_for_manifest(
    manifest: &AddonManifest,
) -> Result<Option<RepositoryAddonAction>, String> {
    let Some(record) = repository_addon_for_id(&manifest.id)? else {
        return Ok(None);
    };
    let action = if manifest.version == repository_release_version(&record) {
        RepositoryAddonAction::Reinstall
    } else {
        RepositoryAddonAction::Update
    };
    Ok(Some(action))
}

pub fn repository_addon_for_id(addon_id: &AddonId) -> Result<Option<RepositoryAddonRecord>, String> {
    let Some((index, source_path)) = config::load_addon_repository_index()
        .map_err(|error| error.to_string())?
    else {
        return Ok(None);
    };
    Ok(repository_addon_record_from_index(
        &index,
        addon_id,
        config::install_profile(),
        &source_path,
    ))
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

fn install_user_addon_at_path(
    source_path: &Path,
    user_addons_root: &Path,
    overrides: &mut AddonStateOverrides,
) -> Result<String, String> {
    let prepared = prepare_addon_install_source(source_path)?;
    let manifest_path = addon_manifest_path(prepared.install_path()).ok_or_else(|| {
        format!(
            "No addon manifest found at '{}'.",
            source_path.display()
        )
    })?;
    let manifest = load_addon_manifest(&manifest_path)?;
    if first_party_addon_registry().manifest(&manifest.id).is_some() {
        return Err(format!(
            "Addon '{}' conflicts with a built-in first-party addon id and cannot be installed here.",
            manifest.id
        ));
    }
    let install_dir_name = install_dir_name_for_addon(&manifest)?;

    fs::create_dir_all(user_addons_root)
        .map_err(|error| format!("Failed to create user addons root: {error}"))?;
    let canonical_root = fs::canonicalize(user_addons_root)
        .map_err(|error| format!("Failed to resolve user addons root: {error}"))?;
    let canonical_source = fs::canonicalize(prepared.install_path())
        .map_err(|error| format!("Failed to resolve addon source: {error}"))?;
    if canonical_source.starts_with(&canonical_root) {
        return Err(format!(
            "Addon source '{}' is already inside the user addons root.",
            source_path.display()
        ));
    }

    let target_dir = canonical_root.join(install_dir_name);
    if target_dir.exists() {
        fs::remove_dir_all(&target_dir)
            .map_err(|error| format!("Failed to replace existing addon install: {error}"))?;
    }
    fs::create_dir_all(&target_dir)
        .map_err(|error| format!("Failed to create addon install directory: {error}"))?;

    if canonical_source.is_dir() {
        copy_dir_contents(&canonical_source, &target_dir)?;
    } else {
        fs::copy(&canonical_source, target_dir.join("manifest.json"))
            .map_err(|error| format!("Failed to copy addon manifest: {error}"))?;
    }

    overrides.set_enabled(manifest.id.clone(), None);
    Ok(format!("Installed {}.", manifest.display_name))
}

struct PreparedAddonInstallSource {
    install_path: PathBuf,
    cleanup_dir: Option<PathBuf>,
}

impl PreparedAddonInstallSource {
    fn direct(path: &Path) -> Self {
        Self {
            install_path: path.to_path_buf(),
            cleanup_dir: None,
        }
    }

    fn extracted(install_path: PathBuf, cleanup_dir: PathBuf) -> Self {
        Self {
            install_path,
            cleanup_dir: Some(cleanup_dir),
        }
    }

    fn install_path(&self) -> &Path {
        &self.install_path
    }
}

impl Drop for PreparedAddonInstallSource {
    fn drop(&mut self) {
        if let Some(dir) = self.cleanup_dir.take() {
            let _ = fs::remove_dir_all(dir);
        }
    }
}

fn prepare_addon_install_source(source_path: &Path) -> Result<PreparedAddonInstallSource, String> {
    if !is_supported_addon_archive(source_path) {
        return Ok(PreparedAddonInstallSource::direct(source_path));
    }

    let staging_root = addon_archive_temp_dir("manual-install")?;
    extract_addon_archive(source_path, &staging_root)?;
    let install_path = resolve_extracted_addon_root(&staging_root).ok_or_else(|| {
        format!(
            "No addon manifest found after extracting archive '{}'.",
            source_path.display()
        )
    })?;
    Ok(PreparedAddonInstallSource::extracted(install_path, staging_root))
}

fn is_supported_addon_archive(path: &Path) -> bool {
    let name = path
        .file_name()
        .and_then(OsStr::to_str)
        .unwrap_or_default()
        .to_ascii_lowercase();
    name.ends_with(".zip")
        || name.ends_with(".ndpkg")
        || name.ends_with(".tar")
        || name.ends_with(".tar.gz")
        || name.ends_with(".tgz")
}

fn addon_archive_temp_dir(label: &str) -> Result<PathBuf, String> {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| format!("Failed to create archive temp directory timestamp: {error}"))?
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("robcos-addon-archive-{label}-{unique}"));
    fs::create_dir_all(&dir)
        .map_err(|error| format!("Failed to create addon archive temp directory: {error}"))?;
    Ok(dir)
}

fn resolve_extracted_addon_root(staging_root: &Path) -> Option<PathBuf> {
    if addon_manifest_path(staging_root).is_some() {
        return Some(staging_root.to_path_buf());
    }

    let entries = fs::read_dir(staging_root).ok()?;
    let mut dirs = entries
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| path.is_dir())
        .collect::<Vec<_>>();
    if dirs.len() != 1 {
        return None;
    }
    let only_dir = dirs.pop()?;
    addon_manifest_path(&only_dir).map(|_| only_dir)
}

fn extract_addon_archive(source_path: &Path, destination_dir: &Path) -> Result<(), String> {
    let file_name = source_path
        .file_name()
        .and_then(OsStr::to_str)
        .unwrap_or_default()
        .to_ascii_lowercase();
    if file_name.ends_with(".zip") || file_name.ends_with(".ndpkg") {
        return extract_zip_archive(source_path, destination_dir);
    }
    if file_name.ends_with(".tar.gz") || file_name.ends_with(".tgz") {
        return extract_tar_archive(source_path, destination_dir, true);
    }
    if file_name.ends_with(".tar") {
        return extract_tar_archive(source_path, destination_dir, false);
    }
    Err(format!(
        "Unsupported addon archive format '{}'.",
        source_path.display()
    ))
}

fn extract_zip_archive(source_path: &Path, destination_dir: &Path) -> Result<(), String> {
    let file = fs::File::open(source_path)
        .map_err(|error| format!("Failed to open addon archive '{}': {error}", source_path.display()))?;
    let mut archive = ZipArchive::new(file)
        .map_err(|error| format!("Failed to read addon zip archive '{}': {error}", source_path.display()))?;
    for idx in 0..archive.len() {
        let mut entry = archive
            .by_index(idx)
            .map_err(|error| format!("Failed to read addon zip entry: {error}"))?;
        let Some(relative_path) = entry.enclosed_name() else {
            return Err("Addon zip archive contains an invalid path.".to_string());
        };
        let output_path = destination_dir.join(&relative_path);
        if entry.is_dir() {
            fs::create_dir_all(&output_path)
                .map_err(|error| format!("Failed to create extracted addon directory: {error}"))?;
            continue;
        }
        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| format!("Failed to create extracted addon directory: {error}"))?;
        }
        let mut output = fs::File::create(&output_path)
            .map_err(|error| format!("Failed to create extracted addon file: {error}"))?;
        std::io::copy(&mut entry, &mut output)
            .map_err(|error| format!("Failed to extract addon zip entry: {error}"))?;
    }
    Ok(())
}

fn extract_tar_archive(
    source_path: &Path,
    destination_dir: &Path,
    gzip: bool,
) -> Result<(), String> {
    let file = fs::File::open(source_path)
        .map_err(|error| format!("Failed to open addon archive '{}': {error}", source_path.display()))?;
    if gzip {
        let decoder = GzDecoder::new(file);
        let mut archive = TarArchive::new(decoder);
        unpack_tar_archive(&mut archive, destination_dir)
    } else {
        let mut archive = TarArchive::new(file);
        unpack_tar_archive(&mut archive, destination_dir)
    }
}

fn unpack_tar_archive<R: Read>(
    archive: &mut TarArchive<R>,
    destination_dir: &Path,
) -> Result<(), String> {
    let entries = archive
        .entries()
        .map_err(|error| format!("Failed to read addon tar archive entries: {error}"))?;
    for entry in entries {
        let mut entry =
            entry.map_err(|error| format!("Failed to read addon tar archive entry: {error}"))?;
        if !entry.header().entry_type().is_file() && !entry.header().entry_type().is_dir() {
            continue;
        }
        let relative_path = sanitize_archive_relative_path(&entry.path().map_err(|error| {
            format!("Failed to read addon tar archive path: {error}")
        })?)?;
        let output_path = destination_dir.join(relative_path);
        if entry.header().entry_type().is_dir() {
            fs::create_dir_all(&output_path)
                .map_err(|error| format!("Failed to create extracted addon directory: {error}"))?;
            continue;
        }
        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| format!("Failed to create extracted addon directory: {error}"))?;
        }
        let mut output = fs::File::create(&output_path)
            .map_err(|error| format!("Failed to create extracted addon file: {error}"))?;
        std::io::copy(&mut entry, &mut output)
            .map_err(|error| format!("Failed to extract addon tar entry: {error}"))?;
    }
    Ok(())
}

fn sanitize_archive_relative_path(path: &Path) -> Result<PathBuf, String> {
    use std::path::Component;

    let mut sanitized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Normal(part) => sanitized.push(part),
            Component::CurDir => {}
            Component::RootDir | Component::ParentDir | Component::Prefix(_) => {
                return Err("Addon archive contains an invalid path.".to_string())
            }
        }
    }
    if sanitized.as_os_str().is_empty() {
        return Err("Addon archive contains an empty path.".to_string());
    }
    Ok(sanitized)
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

    if let Some(parent_dir) = canonical_manifest.parent() {
        if parent_dir != canonical_root && parent_dir.starts_with(&canonical_root) {
            fs::remove_dir_all(parent_dir)
                .map_err(|error| format!("Failed to remove addon directory: {error}"))?;
        } else {
            fs::remove_file(&canonical_manifest)
                .map_err(|error| format!("Failed to remove addon manifest: {error}"))?;
            remove_empty_parent_dirs(canonical_manifest.parent(), &canonical_root);
        }
    } else {
        fs::remove_file(&canonical_manifest)
            .map_err(|error| format!("Failed to remove addon manifest: {error}"))?;
    }
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

fn copy_dir_contents(source_dir: &Path, target_dir: &Path) -> Result<(), String> {
    for entry in
        fs::read_dir(source_dir).map_err(|error| format!("Failed to read addon source: {error}"))?
    {
        let entry = entry.map_err(|error| format!("Failed to read addon source entry: {error}"))?;
        let source_path = entry.path();
        let target_path = target_dir.join(entry.file_name());
        let file_type = entry
            .file_type()
            .map_err(|error| format!("Failed to inspect addon source entry: {error}"))?;
        if file_type.is_dir() {
            fs::create_dir_all(&target_path)
                .map_err(|error| format!("Failed to create addon target directory: {error}"))?;
            copy_dir_contents(&source_path, &target_path)?;
        } else if file_type.is_file() {
            fs::copy(&source_path, &target_path)
                .map_err(|error| format!("Failed to copy addon file: {error}"))?;
        }
    }
    Ok(())
}

fn load_addon_manifest(manifest_path: &Path) -> Result<AddonManifest, String> {
    let raw = fs::read_to_string(manifest_path)
        .map_err(|error| format!("Failed to read addon manifest: {error}"))?;
    serde_json::from_str(&raw).map_err(|error| format!("Failed to parse addon manifest: {error}"))
}

fn install_repository_addon_from_index(
    index: &AddonRepositoryIndex,
    repository_source: &Path,
    addon_id: &AddonId,
    profile: InstallProfile,
    downloads_root: &Path,
    user_addons_root: &Path,
) -> Result<String, String> {
    let package = index
        .addon(addon_id)
        .ok_or_else(|| format!("Addon '{}' is not available in the repository feed.", addon_id))?;
    if package.manifest.essential {
        return Err(format!(
            "Addon '{}' is essential and cannot be installed from the repository feed.",
            package.manifest.id
        ));
    }
    let release = package
        .release(&package.manifest.version)
        .or_else(|| package.releases.first())
        .ok_or_else(|| format!("Addon '{}' has no repository release metadata.", addon_id))?;
    let artifact = release
        .artifact_for_profile(profile)
        .ok_or_else(|| format!("Addon '{}' has no artifact for {}.", addon_id, profile_name(profile)))?;

    let format = artifact.format.as_deref().unwrap_or("manifest-json");
    if !matches!(
        format,
        "manifest-json"
            | "manifest"
            | "json"
            | "addon-dir"
            | "directory"
            | "zip"
            | "addon-zip"
            | "ndpkg"
            | "tar"
            | "tar-gz"
            | "tgz"
    ) {
        return Err(format!(
            "Addon '{}' uses unsupported repository artifact format '{}'.",
            addon_id, format
        ));
    }

    let staging_dir = downloads_root.join(addon_id.as_str());
    if staging_dir.exists() {
        fs::remove_dir_all(&staging_dir)
            .map_err(|error| format!("Failed to reset addon download cache: {error}"))?;
    }
    fs::create_dir_all(&staging_dir)
        .map_err(|error| format!("Failed to create addon download cache: {error}"))?;
    let staged_source = stage_repository_artifact(index, repository_source, artifact, &staging_dir)?;
    verify_repository_artifact_checksum(&staged_source, &artifact.sha256)?;

    let mut overrides = addon_state_overrides();
    let message = install_user_addon_at_path(&staged_source, user_addons_root, &mut overrides)?;
    config::save_addon_state_overrides(&overrides);
    Ok(message)
}

fn stage_repository_artifact(
    index: &AddonRepositoryIndex,
    repository_source: &Path,
    artifact: &AddonArtifact,
    staging_dir: &Path,
) -> Result<PathBuf, String> {
    let format = artifact.format.as_deref().unwrap_or("manifest-json");
    if let Some(path) = file_url_path(&artifact.url) {
        return stage_repository_source_path(&path, format, staging_dir);
    }

    let resolved_url = resolve_repository_url(index, repository_source, &artifact.url);
    if looks_like_http_url(&resolved_url) {
        if matches!(format, "addon-dir" | "directory") {
            return Err(
                "Directory-form repository artifacts are not supported over HTTP yet.".to_string(),
            );
        }
        let destination = staged_repository_file_destination(staging_dir, format);
        let status = Command::new("curl")
            .arg("-L")
            .arg("--fail")
            .arg("-o")
            .arg(&destination)
            .arg(&resolved_url)
            .status()
            .map_err(|error| format!("Failed to launch curl: {error}"))?;
        if status.success() {
            return Ok(destination);
        }
        return Err(format!(
            "curl failed while downloading repository artifact (exit {}).",
            status
        ));
    }

    let source_path = resolve_repository_file_path(index, repository_source, &artifact.url);
    stage_repository_source_path(&source_path, format, staging_dir)
}

fn stage_repository_source_path(
    source_path: &Path,
    format: &str,
    staging_dir: &Path,
) -> Result<PathBuf, String> {
    match format {
        "addon-dir" | "directory" => {
            if !source_path.is_dir() {
                return Err(format!(
                    "Repository artifact '{}' is not a directory bundle.",
                    source_path.display()
                ));
            }
            let staged_dir = staging_dir.join("bundle");
            fs::create_dir_all(&staged_dir)
                .map_err(|error| format!("Failed to create staged addon bundle: {error}"))?;
            copy_dir_contents(source_path, &staged_dir)?;
            Ok(staged_dir)
        }
        "zip" | "addon-zip" | "ndpkg" | "tar" | "tar-gz" | "tgz" => {
            if !source_path.is_file() {
                return Err(format!(
                    "Repository artifact '{}' is not an archive file.",
                    source_path.display()
                ));
            }
            let destination = staged_repository_file_destination(staging_dir, format);
            fs::copy(source_path, &destination).map_err(|error| {
                format!(
                    "Failed to copy repository archive artifact '{}': {error}",
                    source_path.display()
                )
            })?;
            Ok(destination)
        }
        _ => {
            if !source_path.is_file() {
                return Err(format!(
                    "Repository artifact '{}' is not a manifest file.",
                    source_path.display()
                ));
            }
            let destination = staging_dir.join("manifest.json");
            fs::copy(source_path, &destination).map_err(|error| {
                format!(
                    "Failed to copy repository artifact '{}': {error}",
                    source_path.display()
                )
            })?;
            Ok(destination)
        }
    }
}

fn staged_repository_file_destination(staging_dir: &Path, format: &str) -> PathBuf {
    let file_name = match format {
        "zip" | "addon-zip" => "addon.zip",
        "ndpkg" => "addon.ndpkg",
        "tar" => "addon.tar",
        "tar-gz" | "tgz" => "addon.tar.gz",
        _ => "manifest.json",
    };
    staging_dir.join(file_name)
}

fn resolve_repository_file_path(
    index: &AddonRepositoryIndex,
    repository_source: &Path,
    url: &str,
) -> PathBuf {
    if let Some(path) = file_url_path(url) {
        return path;
    }
    let candidate = PathBuf::from(url);
    if candidate.is_absolute() {
        return candidate;
    }
    if let Some(base_url) = index.base_url.as_deref() {
        let base_path = PathBuf::from(base_url);
        if base_path.is_absolute() {
            return base_path.join(url);
        }
    }
    repository_source
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join(url)
}

fn resolve_repository_url(index: &AddonRepositoryIndex, repository_source: &Path, url: &str) -> String {
    if looks_like_http_url(url) {
        return url.to_string();
    }
    if let Some(base_url) = index.base_url.as_deref() {
        if looks_like_http_url(base_url) {
            return format!("{}/{}", base_url.trim_end_matches('/'), url.trim_start_matches('/'));
        }
    }
    resolve_repository_file_path(index, repository_source, url)
        .display()
        .to_string()
}

fn verify_repository_artifact_checksum(path: &Path, expected_hex: &str) -> Result<(), String> {
    let actual = if path.is_dir() {
        directory_sha256(path)?
    } else {
        file_sha256(path)?
    };
    if actual.eq_ignore_ascii_case(expected_hex) {
        Ok(())
    } else {
        Err(format!(
            "Downloaded addon artifact checksum mismatch (expected {}, got {}).",
            expected_hex, actual
        ))
    }
}

fn file_sha256(path: &Path) -> Result<String, String> {
    let mut file =
        fs::File::open(path).map_err(|error| format!("Failed to open downloaded artifact: {error}"))?;
    let mut hasher = Sha256::new();
    let mut buf = [0_u8; 8192];
    loop {
        let read = file
            .read(&mut buf)
            .map_err(|error| format!("Failed to read downloaded artifact: {error}"))?;
        if read == 0 {
            break;
        }
        hasher.update(&buf[..read]);
    }
    Ok(hex::encode(hasher.finalize()))
}

fn directory_sha256(root: &Path) -> Result<String, String> {
    let mut files = Vec::new();
    collect_directory_files(root, root, &mut files)?;
    files.sort_by(|left, right| left.0.cmp(&right.0));

    let mut hasher = Sha256::new();
    for (relative, absolute) in files {
        hasher.update(relative.as_bytes());
        hasher.update([0]);
        hasher.update(file_sha256(&absolute)?);
        hasher.update([0xff]);
    }
    Ok(hex::encode(hasher.finalize()))
}

fn collect_directory_files(
    root: &Path,
    dir: &Path,
    files: &mut Vec<(String, PathBuf)>,
) -> Result<(), String> {
    for entry in
        fs::read_dir(dir).map_err(|error| format!("Failed to read addon directory bundle: {error}"))?
    {
        let entry =
            entry.map_err(|error| format!("Failed to read addon directory entry: {error}"))?;
        let path = entry.path();
        let file_type = entry
            .file_type()
            .map_err(|error| format!("Failed to inspect addon directory entry: {error}"))?;
        if file_type.is_dir() {
            collect_directory_files(root, &path, files)?;
        } else if file_type.is_file() {
            let relative = path
                .strip_prefix(root)
                .map_err(|error| format!("Failed to resolve addon bundle path: {error}"))?
                .to_string_lossy()
                .replace('\\', "/");
            files.push((relative, path));
        }
    }
    Ok(())
}

fn looks_like_http_url(url: &str) -> bool {
    url.starts_with("http://") || url.starts_with("https://")
}

fn file_url_path(url: &str) -> Option<PathBuf> {
    url.strip_prefix("file://").map(PathBuf::from)
}

fn profile_name(profile: InstallProfile) -> &'static str {
    match profile {
        InstallProfile::LinuxDesktop => "linux-desktop",
        InstallProfile::WindowsLauncher => "windows-launcher",
        InstallProfile::MacLauncher => "mac-launcher",
        InstallProfile::PortableDev => "portable-dev",
    }
}

fn repository_addon_inventory(
    installed_ids: &[AddonId],
    profile: InstallProfile,
) -> (Vec<RepositoryAddonRecord>, Option<PathBuf>, Option<String>) {
    let Some((index, source_path)) = load_addon_repository_index_result() else {
        return (Vec::new(), None, None);
    };
    match index {
        Ok(index) => {
            let mut records =
                repository_addon_inventory_from_index(&index, installed_ids, profile, &source_path);
            records.sort_by(|left, right| {
                left.manifest
                    .display_name
                    .to_ascii_lowercase()
                    .cmp(&right.manifest.display_name.to_ascii_lowercase())
                    .then_with(|| left.manifest.id.cmp(&right.manifest.id))
            });
            (records, Some(source_path), None)
        }
        Err(err) => (Vec::new(), Some(source_path), Some(err)),
    }
}

fn load_addon_repository_index_result() -> Option<(Result<AddonRepositoryIndex, String>, PathBuf)> {
    match config::load_addon_repository_index() {
        Ok(Some((index, path))) => Some((Ok(index), path)),
        Ok(None) => None,
        Err(error) => {
            let source_path = config::cached_addon_repository_index_file();
            let fallback_path = config::bundled_addon_repository_index_file();
            let path = if source_path.exists() {
                source_path
            } else {
                fallback_path
            };
            Some((Err(error.to_string()), path))
        }
    }
}

fn repository_addon_inventory_from_index(
    index: &AddonRepositoryIndex,
    installed_ids: &[AddonId],
    profile: InstallProfile,
    source_path: &Path,
) -> Vec<RepositoryAddonRecord> {
    index
        .addons
        .iter()
        .filter(|package| !package.manifest.essential)
        .filter(|package| !installed_ids.iter().any(|id| *id == package.manifest.id))
        .filter_map(|package| {
            let release = package
                .release(&package.manifest.version)
                .cloned()
                .or_else(|| package.releases.first().cloned());
            let has_artifact = release
                .as_ref()
                .and_then(|release| release.artifact_for_profile(profile))
                .is_some();
            has_artifact.then_some(RepositoryAddonRecord {
                manifest: package.manifest.clone(),
                release,
                repository_source: source_path.to_path_buf(),
            })
        })
        .collect()
}

fn repository_addon_record_from_index(
    index: &AddonRepositoryIndex,
    addon_id: &AddonId,
    profile: InstallProfile,
    source_path: &Path,
) -> Option<RepositoryAddonRecord> {
    let package = index.addon(addon_id)?;
    if package.manifest.essential {
        return None;
    }
    let release = package
        .release(&package.manifest.version)
        .cloned()
        .or_else(|| package.releases.first().cloned())?;
    release.artifact_for_profile(profile)?;
    Some(RepositoryAddonRecord {
        manifest: package.manifest.clone(),
        release: Some(release),
        repository_source: source_path.to_path_buf(),
    })
}

fn repository_release_version(record: &RepositoryAddonRecord) -> &str {
    record
        .release
        .as_ref()
        .map(|release| release.version.as_str())
        .unwrap_or(record.manifest.version.as_str())
}

fn install_dir_name_for_addon(manifest: &AddonManifest) -> Result<&str, String> {
    use std::path::Component;

    let addon_id = manifest.id.as_str();
    let mut components = Path::new(addon_id).components();
    match (components.next(), components.next()) {
        (Some(Component::Normal(_)), None) => Ok(addon_id),
        _ => Err(format!(
            "Addon '{}' has an invalid id for installation.",
            manifest.id
        )),
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

const FIRST_PARTY_ADDON_RUNTIMES: [FirstPartyAddonRuntime; 11] = [
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
];

#[cfg(test)]
mod tests {
    use super::{
        effective_addon_enabled_with_overrides, first_party_addon_disabled_reason_with_registry,
        first_party_addon_enabled, first_party_addon_registry,
        first_party_addon_registry_for_profile,
        first_party_addon_registry_for_profile_with_registry, first_party_addon_runtime,
        first_party_capability_enabled_str, hosted_game_names_from_registry,
        installed_hosted_addon_process_from_record, installed_wasm_addon_module_from_record,
        installed_addon_inventory_sections_with_overrides, installed_addon_inventory_with_overrides,
        repository_addon_inventory_from_index,
        installed_enabled_addon_manifest_registry_with_overrides,
    };
    use crate::platform::{
        AddonArtifact, AddonEntrypoint, AddonId, AddonKind, AddonManifest, AddonRegistry,
        AddonManifestDiscovery, AddonManifestLoadIssue, AddonRelease, AddonRepositoryIndex,
        AddonScope, AddonStateOverrides, CapabilityId, DiscoveredAddonManifest,
        HostedAddonProtocol, InstallProfile, IndexedAddonPackage,
    };
    use sha2::{Digest, Sha256};
    use std::fs;
    use std::io::Read;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};
    use zip::write::SimpleFileOptions;

    const TOOL_ADDON_ID: &str = "tools.reference-tool";
    const TOOL_ADDON_NAME: &str = "Reference Tool";
    const TOOL_STATIC_NAME: &str = "Static Reference Tool";
    const TOOL_USER_NAME: &str = "User Reference Tool";
    const TOOL_MANIFEST_PATH: &str = "/tmp/addons/reference-tool/manifest.json";
    const TOOL_ARTIFACT_STEM: &str = "reference-tool";
    const GAME_ADDON_A_ID: &str = "games.arcade-alpha";
    const GAME_ADDON_A_NAME: &str = "Arcade Alpha";
    const GAME_ADDON_B_ID: &str = "games.arcade-beta";
    const GAME_ADDON_B_NAME: &str = "Arcade Beta";

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
        let manifest = manifest(TOOL_ADDON_ID, TOOL_ADDON_NAME, AddonScope::User);
        let mut overrides = AddonStateOverrides::default();

        assert!(effective_addon_enabled_with_overrides(
            &manifest, &overrides
        ));

        overrides.set_enabled(AddonId::from(TOOL_ADDON_ID), Some(false));
        assert!(!effective_addon_enabled_with_overrides(
            &manifest, &overrides
        ));
    }

    #[test]
    fn installed_enabled_registry_filters_disabled_addons() {
        let registry = AddonRegistry::from_manifests([
            first_party_addon_registry()
                .manifest(&AddonId::from("shell.settings"))
                .cloned()
                .expect("settings manifest"),
            manifest(TOOL_ADDON_ID, TOOL_ADDON_NAME, AddonScope::User),
        ])
        .unwrap();
        let mut overrides = AddonStateOverrides::default();
        overrides.set_enabled(AddonId::from(TOOL_ADDON_ID), Some(false));
        let registry =
            installed_enabled_addon_manifest_registry_with_overrides(&registry, &overrides);

        assert!(registry
            .manifest(&AddonId::from("shell.settings"))
            .is_some());
        assert!(registry
            .manifest(&AddonId::from(TOOL_ADDON_ID))
            .is_none());
    }

    #[test]
    fn installed_inventory_prefers_discovered_manifest_and_applies_override() {
        let static_manifest = manifest(TOOL_ADDON_ID, TOOL_STATIC_NAME, AddonScope::Bundled);
        let discovered_manifest = manifest(TOOL_ADDON_ID, TOOL_USER_NAME, AddonScope::User);
        let mut overrides = AddonStateOverrides::default();
        overrides.set_enabled(AddonId::from(TOOL_ADDON_ID), Some(false));

        let records = installed_addon_inventory_with_overrides(
            vec![static_manifest],
            AddonManifestDiscovery {
                manifests: vec![DiscoveredAddonManifest {
                    manifest: discovered_manifest,
                    manifest_path: PathBuf::from(TOOL_MANIFEST_PATH),
                }],
                issues: Vec::new(),
            },
            &overrides,
        );

        assert_eq!(records.len(), 1);
        let record = &records[0];
        assert_eq!(record.manifest.display_name, TOOL_USER_NAME);
        assert_eq!(record.manifest.scope, AddonScope::User);
        assert_eq!(
            record.manifest_path.as_deref(),
            Some(PathBuf::from(TOOL_MANIFEST_PATH).as_path())
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
        overrides.set_enabled(AddonId::from(TOOL_ADDON_ID), Some(false));
        let registry = AddonRegistry::from_manifests([
            first_party_addon_registry()
                .manifest(&AddonId::from("shell.settings"))
                .cloned()
                .expect("settings manifest"),
            manifest(TOOL_ADDON_ID, TOOL_ADDON_NAME, AddonScope::User),
        ])
        .unwrap();
        let enabled_registry = installed_enabled_addon_manifest_registry_with_overrides(
            &registry,
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
            .manifest(&AddonId::from(TOOL_ADDON_ID))
            .is_none());
    }

    #[test]
    fn addon_state_disabled_reason_is_reported_separately_from_profile_policy() {
        // Use shell.connections as the AddonState test case: it is essential
        // (so we create a non-essential clone), mark it disabled via override,
        // and confirm the disabled reason is AddonState on LinuxDesktop but
        // InstallProfile on MacLauncher.
        let connections = first_party_addon_registry()
            .manifest(&AddonId::from("shell.connections"))
            .cloned()
            .expect("connections manifest");
        // Build a non-essential version so the override actually takes effect.
        let disableable = AddonManifest::new(
            "shell.connections",
            "Connections",
            env!("CARGO_PKG_VERSION"),
            AddonKind::App,
            AddonEntrypoint::StaticRoute {
                route: "connections".to_string(),
            },
        )
        .with_scope(connections.scope)
        .with_capability("connections-ui");
        let mut overrides = AddonStateOverrides::default();
        overrides.set_enabled(AddonId::from("shell.connections"), Some(false));
        let registry = AddonRegistry::from_manifests([disableable]).unwrap();
        let enabled_registry = installed_enabled_addon_manifest_registry_with_overrides(
            &registry,
            &overrides,
        );

        assert_eq!(
            first_party_addon_disabled_reason_with_registry(
                InstallProfile::LinuxDesktop,
                &AddonId::from("shell.connections"),
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
        let sections = installed_addon_inventory_sections_with_overrides(
            vec![manifest("shell.settings", "Settings", AddonScope::Bundled).essential()],
            AddonManifestDiscovery {
                manifests: vec![DiscoveredAddonManifest {
                    manifest: manifest(TOOL_ADDON_ID, TOOL_ADDON_NAME, AddonScope::User),
                    manifest_path: PathBuf::from(TOOL_MANIFEST_PATH),
                }],
                issues: Vec::new(),
            },
            &AddonStateOverrides::default(),
        );

        assert!(sections
            .essential
            .iter()
            .any(|record| record.manifest.id.as_str() == "shell.settings"));
        assert!(sections
            .optional
            .iter()
            .any(|record| record.manifest.id.as_str() == TOOL_ADDON_ID));
        assert!(sections
            .optional
            .iter()
            .all(|record| !record.manifest.essential));
    }

    #[test]
    fn default_installed_inventory_does_not_seed_optional_addons() {
        let sections = installed_addon_inventory_sections_with_overrides(
            vec![manifest("shell.settings", "Settings", AddonScope::Bundled).essential()],
            AddonManifestDiscovery::default(),
            &AddonStateOverrides::default(),
        );

        assert!(sections.optional.is_empty());
    }

    #[test]
    fn hosted_game_names_only_include_installed_runtime_known_games() {
        let registry = AddonRegistry::from_manifests([
            AddonManifest::new(
                GAME_ADDON_A_ID,
                GAME_ADDON_A_NAME,
                "0.1.0",
                AddonKind::Game,
                AddonEntrypoint::WasmModule {
                    module: "addon.wasm".to_string(),
                    protocol: HostedAddonProtocol::ShellSurfaceV1,
                },
            )
            .with_scope(AddonScope::User)
            .with_capability("game-launcher"),
            manifest("tools.feed-sample", "Feed Sample", AddonScope::User),
        ])
        .unwrap();

        assert_eq!(
            hosted_game_names_from_registry(&registry),
            vec![GAME_ADDON_A_NAME]
        );
    }

    #[test]
    fn installed_hosted_addon_process_resolves_bundle_relative_executable() {
        let root = temp_dir("installed_hosted_addon_process_resolves_bundle_relative_executable");
        let addon_dir = root.join(GAME_ADDON_A_ID);
        fs::create_dir_all(addon_dir.join("bin")).unwrap();
        let manifest_path = addon_dir.join("manifest.json");
        let manifest = AddonManifest::new(
            GAME_ADDON_A_ID,
            GAME_ADDON_A_NAME,
            "0.1.0",
            AddonKind::Game,
            AddonEntrypoint::HostedProcess {
                executable: "bin/arcade-alpha-addon".to_string(),
                protocol: HostedAddonProtocol::ShellSurfaceV1,
            },
        )
        .with_scope(AddonScope::User);
        fs::write(&manifest_path, serde_json::to_string(&manifest).unwrap()).unwrap();

        let process = installed_hosted_addon_process_from_record(super::InstalledAddonRecord {
            manifest,
            manifest_path: Some(manifest_path.clone()),
            explicit_enabled: None,
            effective_enabled: true,
        })
        .unwrap();

        assert_eq!(process.addon_id.as_str(), GAME_ADDON_A_ID);
        assert_eq!(process.protocol, HostedAddonProtocol::ShellSurfaceV1);
        assert_eq!(
            process.executable_path,
            addon_dir.join("bin").join("arcade-alpha-addon")
        );
    }

    #[test]
    fn installed_wasm_addon_module_resolves_bundle_relative_module() {
        let root = temp_dir("installed_wasm_addon_module_resolves_bundle_relative_module");
        let addon_dir = root.join(GAME_ADDON_B_ID);
        fs::create_dir_all(addon_dir.join("wasm")).unwrap();
        let manifest_path = addon_dir.join("manifest.json");
        let manifest = AddonManifest::new(
            GAME_ADDON_B_ID,
            GAME_ADDON_B_NAME,
            "0.1.0",
            AddonKind::Game,
            AddonEntrypoint::WasmModule {
                module: "wasm/arcade-beta.wasm".to_string(),
                protocol: HostedAddonProtocol::ShellSurfaceV1,
            },
        )
        .with_scope(AddonScope::User);
        fs::write(&manifest_path, serde_json::to_string(&manifest).unwrap()).unwrap();

        let module = installed_wasm_addon_module_from_record(super::InstalledAddonRecord {
            manifest,
            manifest_path: Some(manifest_path.clone()),
            explicit_enabled: None,
            effective_enabled: true,
        })
        .unwrap();

        assert_eq!(module.addon_id.as_str(), GAME_ADDON_B_ID);
        assert_eq!(module.protocol, HostedAddonProtocol::ShellSurfaceV1);
        assert_eq!(
            module.module_path,
            addon_dir.join("wasm").join("arcade-beta.wasm")
        );
    }

    #[test]
    fn installed_inventory_sections_preserve_manifest_load_issues() {
        let sections = installed_addon_inventory_sections_with_overrides(
            vec![manifest("shell.settings", "Settings", AddonScope::Bundled).essential()],
            AddonManifestDiscovery {
                manifests: vec![DiscoveredAddonManifest {
                    manifest: manifest("tools.sample", "Sample Tool", AddonScope::User),
                    manifest_path: PathBuf::from("/tmp/addons/sample/manifest.json"),
                }],
                issues: vec![AddonManifestLoadIssue {
                    scope: AddonScope::User,
                    manifest_path: PathBuf::from("/tmp/addons/broken.json"),
                    detail: "failed to parse addon manifest: expected value".to_string(),
                }],
            },
            &AddonStateOverrides::default(),
        );

        assert_eq!(sections.issues.len(), 1);
        assert_eq!(sections.issues[0].scope, AddonScope::User);
        assert_eq!(sections.optional.len(), 1);
    }

    #[test]
    fn inventory_sections_include_uninstalled_optional_repository_addons() {
        let sections = installed_addon_inventory_sections_with_overrides(
            vec![manifest("shell.settings", "Settings", AddonScope::Bundled).essential()],
            AddonManifestDiscovery::default(),
            &AddonStateOverrides::default(),
        );

        let available = repository_addon_inventory_from_index(
            &sample_repository_index(),
            &sections
                .essential
                .iter()
                .chain(sections.optional.iter())
                .map(|record| record.manifest.id.clone())
                .collect::<Vec<_>>(),
            InstallProfile::LinuxDesktop,
            &PathBuf::from("/tmp/addon-repository-index.json"),
        );

        assert_eq!(available.len(), 1);
        assert_eq!(available[0].manifest.id.as_str(), "tools.feed-sample");
    }

    #[test]
    fn user_scoped_manifest_removal_deletes_manifest_and_clears_override() {
        let root = temp_dir("user_scoped_manifest_removal_deletes_manifest_and_clears_override");
        let addon_dir = root.join("sample-addon");
        fs::create_dir_all(&addon_dir).unwrap();
        let manifest_path = addon_dir.join("manifest.json");
        fs::write(&manifest_path, "{}").unwrap();
        fs::write(addon_dir.join("icon.txt"), "icon").unwrap();
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
    fn install_user_addon_from_manifest_file_creates_named_user_dir_and_clears_override() {
        let source_root =
            temp_dir("install_user_addon_from_manifest_file_creates_named_user_dir_and_clears");
        let install_root = temp_dir("install_user_addon_manifest_file_install_root");
        let manifest_path = source_root.join("sample-addon.json");
        fs::write(
            &manifest_path,
            serde_json::to_string(&manifest("addons.sample", "Sample Addon", AddonScope::User))
                .unwrap(),
        )
        .unwrap();
        let mut overrides = AddonStateOverrides::default();
        overrides.set_enabled(AddonId::from("addons.sample"), Some(false));

        let message =
            super::install_user_addon_at_path(&manifest_path, &install_root, &mut overrides)
                .unwrap();

        assert_eq!(message, "Installed Sample Addon.");
        assert_eq!(overrides.enabled_for(&AddonId::from("addons.sample")), None);
        assert_eq!(
            fs::read_to_string(install_root.join("addons.sample").join("manifest.json")).unwrap(),
            fs::read_to_string(manifest_path).unwrap()
        );
    }

    #[test]
    fn install_user_addon_from_directory_copies_directory_contents() {
        let source_root = temp_dir("install_user_addon_from_directory_copies_contents_source");
        let install_root = temp_dir("install_user_addon_from_directory_copies_contents_install");
        let addon_dir = source_root.join("sample-addon");
        fs::create_dir_all(addon_dir.join("assets")).unwrap();
        fs::write(
            addon_dir.join("manifest.json"),
            serde_json::to_string(&manifest("addons.sample", "Sample Addon", AddonScope::User))
                .unwrap(),
        )
        .unwrap();
        fs::write(addon_dir.join("assets").join("icon.txt"), "icon").unwrap();

        super::install_user_addon_at_path(
            &addon_dir,
            &install_root,
            &mut AddonStateOverrides::default(),
        )
        .unwrap();

        assert!(install_root.join("addons.sample").join("manifest.json").exists());
        assert_eq!(
            fs::read_to_string(
                install_root
                    .join("addons.sample")
                    .join("assets")
                    .join("icon.txt")
            )
            .unwrap(),
            "icon"
        );
    }

    #[test]
    fn install_user_addon_from_zip_archive_copies_directory_contents() {
        let source_root = temp_dir("install_user_addon_from_zip_archive_source");
        let install_root = temp_dir("install_user_addon_from_zip_archive_install");
        let archive_path = source_root.join("sample-addon.zip");
        write_zip_archive(
            &archive_path,
            &[
                (
                    "sample-addon/manifest.json",
                    serde_json::to_string(&manifest("addons.sample", "Sample Addon", AddonScope::User))
                        .unwrap(),
                ),
                ("sample-addon/assets/icon.txt", "icon".to_string()),
            ],
        );

        super::install_user_addon_at_path(
            &archive_path,
            &install_root,
            &mut AddonStateOverrides::default(),
        )
        .unwrap();

        assert!(install_root.join("addons.sample").join("manifest.json").exists());
        assert_eq!(
            fs::read_to_string(
                install_root
                    .join("addons.sample")
                    .join("assets")
                    .join("icon.txt")
            )
            .unwrap(),
            "icon"
        );
    }

    #[test]
    fn install_user_addon_rejects_sources_inside_user_root() {
        let install_root = temp_dir("install_user_addon_rejects_sources_inside_user_root");
        let addon_dir = install_root.join("existing-addon");
        fs::create_dir_all(&addon_dir).unwrap();
        let manifest_path = addon_dir.join("manifest.json");
        fs::write(
            &manifest_path,
            serde_json::to_string(&manifest("addons.sample", "Sample Addon", AddonScope::User))
                .unwrap(),
        )
        .unwrap();

        let err = super::install_user_addon_at_path(
            &manifest_path,
            &install_root,
            &mut AddonStateOverrides::default(),
        )
        .unwrap_err();

        assert!(err.contains("already inside the user addons root"));
    }

    #[test]
    fn install_user_addon_rejects_first_party_addon_ids() {
        let source_root = temp_dir("install_user_addon_rejects_first_party_addon_ids_source");
        let install_root = temp_dir("install_user_addon_rejects_first_party_addon_ids_install");
        let manifest_path = source_root.join("settings.json");
        fs::write(
            &manifest_path,
            serde_json::to_string(&manifest("shell.settings", "Settings Override", AddonScope::User))
                .unwrap(),
        )
        .unwrap();

        let err = super::install_user_addon_at_path(
            &manifest_path,
            &install_root,
            &mut AddonStateOverrides::default(),
        )
        .unwrap_err();

        assert!(err.contains("conflicts with a built-in first-party addon id"));
    }

    #[test]
    fn install_repository_addon_from_index_installs_relative_manifest_artifact() {
        let repository_root =
            temp_dir("install_repository_addon_from_index_installs_relative_manifest_artifact");
        let install_root = temp_dir("install_repository_addon_from_index_install_root");
        let downloads_root = temp_dir("install_repository_addon_from_index_download_root");
        let artifact_path = repository_root.join("feed-sample.json");
        let artifact_manifest = manifest("tools.feed-sample", "Feed Sample", AddonScope::User);
        fs::write(&artifact_path, serde_json::to_string(&artifact_manifest).unwrap()).unwrap();

        let message = super::install_repository_addon_from_index(
            &AddonRepositoryIndex {
                schema_version: 1,
                generated_at: None,
                base_url: None,
                addons: vec![IndexedAddonPackage {
                    manifest: artifact_manifest.clone(),
                    releases: vec![AddonRelease {
                        version: artifact_manifest.version.clone(),
                        channel: Some("stable".to_string()),
                        artifacts: vec![AddonArtifact {
                            install_profile: Some(InstallProfile::LinuxDesktop),
                            url: "feed-sample.json".to_string(),
                            sha256: file_sha256(&artifact_path),
                            signature_url: None,
                            size_bytes: None,
                            format: Some("manifest-json".to_string()),
                        }],
                    }],
                }],
            },
            &repository_root.join("addon-repository-index.json"),
            &AddonId::from("tools.feed-sample"),
            InstallProfile::LinuxDesktop,
            &downloads_root,
            &install_root,
        )
        .unwrap();

        assert_eq!(message, "Installed Feed Sample.");
        assert!(
            install_root
                .join("tools.feed-sample")
                .join("manifest.json")
                .exists()
        );
    }

    #[test]
    fn install_repository_addon_allows_runtime_known_optional_addon_id() {
        let repository_root =
            temp_dir("install_repository_addon_allows_runtime_known_optional_addon_id_repo");
        let install_root =
            temp_dir("install_repository_addon_allows_runtime_known_optional_addon_id_install");
        let downloads_root =
            temp_dir("install_repository_addon_allows_runtime_known_optional_addon_id_download");
        let artifact_path = repository_root.join(format!("{TOOL_ARTIFACT_STEM}.json"));
        let artifact_manifest = manifest(TOOL_ADDON_ID, TOOL_ADDON_NAME, AddonScope::User)
            .with_capability("code-reference");
        fs::write(&artifact_path, serde_json::to_string(&artifact_manifest).unwrap()).unwrap();

        let message = super::install_repository_addon_from_index(
            &AddonRepositoryIndex {
                schema_version: 1,
                generated_at: None,
                base_url: None,
                addons: vec![IndexedAddonPackage {
                    manifest: artifact_manifest.clone(),
                    releases: vec![AddonRelease {
                        version: artifact_manifest.version.clone(),
                        channel: Some("stable".to_string()),
                        artifacts: vec![AddonArtifact {
                            install_profile: Some(InstallProfile::LinuxDesktop),
                            url: format!("{TOOL_ARTIFACT_STEM}.json"),
                            sha256: file_sha256(&artifact_path),
                            signature_url: None,
                            size_bytes: None,
                            format: Some("manifest-json".to_string()),
                        }],
                    }],
                }],
            },
            &repository_root.join("addon-repository-index.json"),
            &AddonId::from(TOOL_ADDON_ID),
            InstallProfile::LinuxDesktop,
            &downloads_root,
            &install_root,
        )
        .unwrap();

        assert_eq!(message, format!("Installed {TOOL_ADDON_NAME}."));
        assert!(install_root.join(TOOL_ADDON_ID).join("manifest.json").exists());
    }

    #[test]
    fn install_repository_addon_from_index_installs_directory_bundle() {
        let repository_root =
            temp_dir("install_repository_addon_from_index_installs_directory_bundle_repo");
        let install_root =
            temp_dir("install_repository_addon_from_index_installs_directory_bundle_install");
        let downloads_root =
            temp_dir("install_repository_addon_from_index_installs_directory_bundle_download");
        let bundle_dir = repository_root.join(TOOL_ARTIFACT_STEM);
        fs::create_dir_all(bundle_dir.join("assets")).unwrap();
        let artifact_manifest = manifest(TOOL_ADDON_ID, TOOL_ADDON_NAME, AddonScope::User)
            .with_capability("code-reference");
        fs::write(
            bundle_dir.join("manifest.json"),
            serde_json::to_string(&artifact_manifest).unwrap(),
        )
        .unwrap();
        fs::write(bundle_dir.join("assets").join("help.txt"), "launch codes").unwrap();

        let message = super::install_repository_addon_from_index(
            &AddonRepositoryIndex {
                schema_version: 1,
                generated_at: None,
                base_url: None,
                addons: vec![IndexedAddonPackage {
                    manifest: artifact_manifest.clone(),
                    releases: vec![AddonRelease {
                        version: artifact_manifest.version.clone(),
                        channel: Some("stable".to_string()),
                        artifacts: vec![AddonArtifact {
                            install_profile: Some(InstallProfile::LinuxDesktop),
                            url: TOOL_ARTIFACT_STEM.to_string(),
                            sha256: directory_sha256_for_test(&bundle_dir),
                            signature_url: None,
                            size_bytes: None,
                            format: Some("addon-dir".to_string()),
                        }],
                    }],
                }],
            },
            &repository_root.join("addon-repository-index.json"),
            &AddonId::from(TOOL_ADDON_ID),
            InstallProfile::LinuxDesktop,
            &downloads_root,
            &install_root,
        )
        .unwrap();

        assert_eq!(message, format!("Installed {TOOL_ADDON_NAME}."));
        assert!(install_root.join(TOOL_ADDON_ID).join("manifest.json").exists());
        assert_eq!(
            fs::read_to_string(install_root.join(TOOL_ADDON_ID).join("assets").join("help.txt"))
                .unwrap(),
            "launch codes"
        );
    }

    #[test]
    fn install_repository_addon_from_index_rejects_checksum_mismatch() {
        let repository_root =
            temp_dir("install_repository_addon_from_index_rejects_checksum_mismatch_repo");
        let install_root = temp_dir("install_repository_addon_from_index_rejects_checksum_mismatch_install");
        let downloads_root = temp_dir("install_repository_addon_from_index_rejects_checksum_mismatch_download");
        let artifact_path = repository_root.join("feed-sample.json");
        let artifact_manifest = manifest("tools.feed-sample", "Feed Sample", AddonScope::User);
        fs::write(&artifact_path, serde_json::to_string(&artifact_manifest).unwrap()).unwrap();

        let err = super::install_repository_addon_from_index(
            &AddonRepositoryIndex {
                schema_version: 1,
                generated_at: None,
                base_url: None,
                addons: vec![IndexedAddonPackage {
                    manifest: artifact_manifest.clone(),
                    releases: vec![AddonRelease {
                        version: artifact_manifest.version.clone(),
                        channel: Some("stable".to_string()),
                        artifacts: vec![AddonArtifact {
                            install_profile: Some(InstallProfile::LinuxDesktop),
                            url: artifact_path.display().to_string(),
                            sha256: "deadbeef".to_string(),
                            signature_url: None,
                            size_bytes: None,
                            format: Some("manifest-json".to_string()),
                        }],
                    }],
                }],
            },
            &repository_root.join("addon-repository-index.json"),
            &AddonId::from("tools.feed-sample"),
            InstallProfile::LinuxDesktop,
            &downloads_root,
            &install_root,
        )
        .unwrap_err();

        assert!(err.contains("checksum mismatch"));
    }

    #[test]
    fn install_repository_addon_from_index_installs_zip_bundle() {
        let repository_root =
            temp_dir("install_repository_addon_from_index_installs_zip_bundle_repo");
        let install_root = temp_dir("install_repository_addon_from_index_installs_zip_bundle_install");
        let downloads_root =
            temp_dir("install_repository_addon_from_index_installs_zip_bundle_download");
        let archive_path = repository_root.join(format!("{TOOL_ARTIFACT_STEM}.zip"));
        let artifact_manifest = manifest(TOOL_ADDON_ID, TOOL_ADDON_NAME, AddonScope::User)
            .with_capability("code-reference");
        write_zip_archive(
            &archive_path,
            &[
                (
                    "reference-tool/manifest.json",
                    serde_json::to_string(&artifact_manifest).unwrap(),
                ),
                ("reference-tool/assets/help.txt", "launch codes".to_string()),
            ],
        );

        let message = super::install_repository_addon_from_index(
            &AddonRepositoryIndex {
                schema_version: 1,
                generated_at: None,
                base_url: None,
                addons: vec![IndexedAddonPackage {
                    manifest: artifact_manifest.clone(),
                    releases: vec![AddonRelease {
                        version: artifact_manifest.version.clone(),
                        channel: Some("stable".to_string()),
                        artifacts: vec![AddonArtifact {
                            install_profile: Some(InstallProfile::LinuxDesktop),
                            url: format!("{TOOL_ARTIFACT_STEM}.zip"),
                            sha256: file_sha256(&archive_path),
                            signature_url: None,
                            size_bytes: None,
                            format: Some("zip".to_string()),
                        }],
                    }],
                }],
            },
            &repository_root.join("addon-repository-index.json"),
            &AddonId::from(TOOL_ADDON_ID),
            InstallProfile::LinuxDesktop,
            &downloads_root,
            &install_root,
        )
        .unwrap();

        assert_eq!(message, format!("Installed {TOOL_ADDON_NAME}."));
        assert!(install_root.join(TOOL_ADDON_ID).join("manifest.json").exists());
        assert_eq!(
            fs::read_to_string(install_root.join(TOOL_ADDON_ID).join("assets").join("help.txt"))
                .unwrap(),
            "launch codes"
        );
    }

    #[test]
    fn install_repository_addon_from_index_installs_ndpkg_bundle() {
        let repository_root =
            temp_dir("install_repository_addon_from_index_installs_ndpkg_bundle_repo");
        let install_root =
            temp_dir("install_repository_addon_from_index_installs_ndpkg_bundle_install");
        let downloads_root =
            temp_dir("install_repository_addon_from_index_installs_ndpkg_bundle_download");
        let archive_path = repository_root.join(format!("{TOOL_ARTIFACT_STEM}.ndpkg"));
        let artifact_manifest = manifest(TOOL_ADDON_ID, TOOL_ADDON_NAME, AddonScope::User)
            .with_capability("code-reference");
        write_zip_archive(
            &archive_path,
            &[
                (
                    "reference-tool/manifest.json",
                    serde_json::to_string(&artifact_manifest).unwrap(),
                ),
                ("reference-tool/assets/help.txt", "launch codes".to_string()),
            ],
        );

        let message = super::install_repository_addon_from_index(
            &AddonRepositoryIndex {
                schema_version: 1,
                generated_at: None,
                base_url: None,
                addons: vec![IndexedAddonPackage {
                    manifest: artifact_manifest.clone(),
                    releases: vec![AddonRelease {
                        version: artifact_manifest.version.clone(),
                        channel: Some("stable".to_string()),
                        artifacts: vec![AddonArtifact {
                            install_profile: Some(InstallProfile::LinuxDesktop),
                            url: format!("{TOOL_ARTIFACT_STEM}.ndpkg"),
                            sha256: file_sha256(&archive_path),
                            signature_url: None,
                            size_bytes: None,
                            format: Some("ndpkg".to_string()),
                        }],
                    }],
                }],
            },
            &repository_root.join("addon-repository-index.json"),
            &AddonId::from(TOOL_ADDON_ID),
            InstallProfile::LinuxDesktop,
            &downloads_root,
            &install_root,
        )
        .unwrap();

        assert_eq!(message, format!("Installed {TOOL_ADDON_NAME}."));
        assert!(install_root.join(TOOL_ADDON_ID).join("manifest.json").exists());
        assert_eq!(
            fs::read_to_string(install_root.join(TOOL_ADDON_ID).join("assets").join("help.txt"))
                .unwrap(),
            "launch codes"
        );
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

    fn sample_repository_index() -> AddonRepositoryIndex {
        AddonRepositoryIndex {
            schema_version: 1,
            generated_at: None,
            base_url: Some("https://example.invalid/addons/".to_string()),
            addons: vec![
                IndexedAddonPackage {
                    manifest: manifest("tools.feed-sample", "Feed Sample", AddonScope::User),
                    releases: vec![AddonRelease {
                        version: "1.2.3".to_string(),
                        channel: Some("stable".to_string()),
                        artifacts: vec![AddonArtifact {
                            install_profile: Some(InstallProfile::LinuxDesktop),
                            url: "https://example.invalid/addons/feed-sample-linux.zip"
                                .to_string(),
                            sha256: "abc123".to_string(),
                            signature_url: None,
                            size_bytes: Some(42),
                            format: Some("zip".to_string()),
                        }],
                    }],
                },
                IndexedAddonPackage {
                    manifest: manifest("shell.settings", "Settings", AddonScope::User).essential(),
                    releases: vec![AddonRelease {
                        version: "1.0.0".to_string(),
                        channel: Some("stable".to_string()),
                        artifacts: vec![AddonArtifact {
                            install_profile: Some(InstallProfile::LinuxDesktop),
                            url: "https://example.invalid/addons/settings-linux.zip".to_string(),
                            sha256: "def456".to_string(),
                            signature_url: None,
                            size_bytes: Some(24),
                            format: Some("zip".to_string()),
                        }],
                    }],
                },
            ],
        }
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

    fn file_sha256(path: &Path) -> String {
        let mut file = fs::File::open(path).unwrap();
        let mut hasher = Sha256::new();
        let mut buf = [0_u8; 8192];
        loop {
            let read = file.read(&mut buf).unwrap();
            if read == 0 {
                break;
            }
            hasher.update(&buf[..read]);
        }
        hex::encode(hasher.finalize())
    }

    fn directory_sha256_for_test(path: &Path) -> String {
        super::directory_sha256(path).unwrap()
    }

    fn write_zip_archive(path: &Path, entries: &[(&str, String)]) {
        let file = fs::File::create(path).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        let options = SimpleFileOptions::default();
        for (name, contents) in entries {
            zip.start_file(name, options).unwrap();
            use std::io::Write;
            zip.write_all(contents.as_bytes()).unwrap();
        }
        zip.finish().unwrap();
    }
}
