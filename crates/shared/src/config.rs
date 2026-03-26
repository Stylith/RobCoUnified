use crate::platform::{
    AddonRepositoryIndex, AddonStateOverrides, InstallProfile, PlatformPaths,
    ResolvedPlatformPaths, RuntimeEnvironment, StatePathLayout,
};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{OnceLock, RwLock};

// ── Paths ─────────────────────────────────────────────────────────────────────

pub fn runtime_environment() -> RuntimeEnvironment {
    static RUNTIME_ENVIRONMENT: OnceLock<RuntimeEnvironment> = OnceLock::new();
    RUNTIME_ENVIRONMENT
        .get_or_init(RuntimeEnvironment::detect)
        .clone()
}

pub fn install_profile() -> InstallProfile {
    runtime_environment().install_profile()
}

pub fn platform_paths() -> ResolvedPlatformPaths {
    runtime_environment().paths().clone()
}

pub fn state_root_dir() -> PathBuf {
    let dir = runtime_environment().state_layout().root().to_path_buf();
    let _ = std::fs::create_dir_all(&dir);
    dir
}

pub fn core_root_dir() -> PathBuf {
    platform_paths().core_root().to_path_buf()
}

pub fn system_addons_root_dir() -> PathBuf {
    platform_paths().system_addons_root().to_path_buf()
}

pub fn logical_user_root_dir() -> PathBuf {
    let dir = platform_paths().user_root().to_path_buf();
    let _ = std::fs::create_dir_all(&dir);
    dir
}

pub fn user_addons_root_dir() -> PathBuf {
    let dir = platform_paths().user_addons_root().to_path_buf();
    let _ = std::fs::create_dir_all(&dir);
    dir
}

pub fn cache_root_dir() -> PathBuf {
    let dir = platform_paths().cache_root().to_path_buf();
    let _ = std::fs::create_dir_all(&dir);
    dir
}

pub fn bundled_addon_repository_index_file() -> PathBuf {
    core_root_dir().join("addon-repository-index.json")
}

pub fn cached_addon_repository_index_file() -> PathBuf {
    cache_root_dir().join("addon-repository-index.json")
}

pub fn addon_downloads_cache_dir() -> PathBuf {
    let dir = cache_root_dir().join("addon-downloads");
    let _ = std::fs::create_dir_all(&dir);
    dir
}

pub const ADDON_REPOSITORY_INDEX_URL_ENV: &str = "NUCLEON_ADDON_REPOSITORY_INDEX_URL";
pub const LEGACY_ADDON_REPOSITORY_INDEX_URL_ENV: &str = "ROBCOS_ADDON_REPOSITORY_INDEX_URL";
pub const DEFAULT_ADDON_REPOSITORY_INDEX_URL: &str =
    "https://raw.githubusercontent.com/Stylith/nucleon-desktop-addons/main/index.json";

pub fn addon_repository_index_url() -> String {
    first_non_empty_env_value(&[
        ADDON_REPOSITORY_INDEX_URL_ENV,
        LEGACY_ADDON_REPOSITORY_INDEX_URL_ENV,
    ])
    .unwrap_or_else(|| DEFAULT_ADDON_REPOSITORY_INDEX_URL.to_string())
}

/// Spawn a background thread to refresh the cached index from the remote
/// addon repository.  Safe to call multiple times — skips if cache is fresh.
pub fn spawn_addon_repository_index_refresh() {
    std::thread::spawn(|| {
        let cached_path = cached_addon_repository_index_file();

        // Skip if the cached file was modified less than 10 minutes ago.
        if let Ok(metadata) = std::fs::metadata(&cached_path) {
            if let Ok(modified) = metadata.modified() {
                if modified.elapsed().unwrap_or_default() < std::time::Duration::from_secs(600) {
                    return;
                }
            }
        }

        let _ = std::fs::create_dir_all(cache_root_dir());
        let _ = Command::new("curl")
            .arg("-L")
            .arg("--fail")
            .arg("--silent")
            .arg("--max-time")
            .arg("15")
            .arg("-o")
            .arg(&cached_path)
            .arg(addon_repository_index_url())
            .status();
    });
}

pub fn load_addon_repository_index() -> Result<Option<(AddonRepositoryIndex, PathBuf)>> {
    for path in [
        cached_addon_repository_index_file(),
        bundled_addon_repository_index_file(),
    ] {
        if !path.exists() {
            continue;
        }
        let raw = std::fs::read_to_string(&path)
            .with_context(|| format!("failed to read addon repository index '{}'", path.display()))?;
        let index = serde_json::from_str(&raw).with_context(|| {
            format!(
                "failed to parse addon repository index '{}'",
                path.display()
            )
        })?;
        return Ok(Some((index, path)));
    }
    Ok(None)
}

pub const BIN_DIR_ENV: &str = "NUCLEON_BIN_DIR";
pub const LEGACY_BIN_DIR_ENV: &str = "ROBCOS_BIN_DIR";

pub fn bundled_bin_dir() -> PathBuf {
    first_non_empty_env_value(&[BIN_DIR_ENV, LEGACY_BIN_DIR_ENV])
        .map(PathBuf::from)
        .unwrap_or_else(|| core_root_dir().join("bin"))
}

pub fn bundled_binary_path(binary_name: impl AsRef<Path>) -> PathBuf {
    bundled_bin_dir().join(binary_name)
}

pub fn runtime_root_dir() -> PathBuf {
    let dir = runtime_environment().runtime_layout().root().to_path_buf();
    let _ = std::fs::create_dir_all(&dir);
    dir
}

// Legacy compatibility alias for older state-path callers.
pub fn base_dir() -> PathBuf {
    static BASE_DIR: OnceLock<PathBuf> = OnceLock::new();
    BASE_DIR.get_or_init(detect_base_dir).clone()
}

fn detect_base_dir() -> PathBuf {
    if let Some(path) = std::env::var_os("ROBCOS_BASE_DIR") {
        let dir = PathBuf::from(path);
        let _ = std::fs::create_dir_all(&dir);
        return dir;
    }

    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(bundle_dir) = macos_app_bundle_dir(&exe_path) {
            if let Some(app_support_dir) = macos_app_support_dir() {
                let dir = app_support_dir.join("RobCoOS");
                let _ = std::fs::create_dir_all(&dir);
                migrate_bundle_runtime_data_if_needed(&dir, &exe_path, &bundle_dir);
                return dir;
            }
        }

        if let Some(parent) = exe_path.parent() {
            return parent.to_path_buf();
        }
    }

    PathBuf::from(".")
}

fn first_non_empty_env_value(names: &[&str]) -> Option<String> {
    names.iter().find_map(|name| {
        std::env::var(name)
            .ok()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
    })
}

fn macos_app_bundle_dir(exe_path: &Path) -> Option<PathBuf> {
    let macos_dir = exe_path.parent()?;
    if macos_dir.file_name()? != "MacOS" {
        return None;
    }

    let contents_dir = macos_dir.parent()?;
    if contents_dir.file_name()? != "Contents" {
        return None;
    }

    let app_dir = contents_dir.parent()?;
    (app_dir.extension()? == "app").then(|| app_dir.to_path_buf())
}

fn macos_app_support_dir() -> Option<PathBuf> {
    dirs::data_local_dir()
        .or_else(|| dirs::home_dir().map(|home| home.join("Library").join("Application Support")))
}

fn has_runtime_state(dir: &Path) -> bool {
    let layout = StatePathLayout::new(dir.to_path_buf());
    [
        layout.global_settings_file(),
        layout.about_file(),
        layout.session_file(),
        layout.installed_package_descriptions_file(),
        layout.users_db_file(),
        layout.journal_entries_dir(),
    ]
    .iter()
    .any(|path| path.exists())
}

fn legacy_runtime_dirs(exe_path: &Path, bundle_dir: &Path) -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Some(macos_dir) = exe_path.parent() {
        dirs.push(macos_dir.to_path_buf());
    }
    if let Some(bundle_parent) = bundle_dir.parent() {
        let bundle_parent = bundle_parent.to_path_buf();
        if !dirs.contains(&bundle_parent) {
            dirs.push(bundle_parent);
        }
    }
    dirs
}

fn migrate_bundle_runtime_data_if_needed(target_dir: &Path, exe_path: &Path, bundle_dir: &Path) {
    for legacy_dir in legacy_runtime_dirs(exe_path, bundle_dir) {
        merge_runtime_state_from(target_dir, &legacy_dir);
    }
}

fn merge_runtime_state_from(target_dir: &Path, legacy_dir: &Path) {
    if !has_runtime_state(legacy_dir) {
        return;
    }
    let target_layout = StatePathLayout::new(target_dir.to_path_buf());
    let legacy_layout = StatePathLayout::new(legacy_dir.to_path_buf());
    for (from, to) in [
        (
            legacy_layout.global_settings_file(),
            target_layout.global_settings_file(),
        ),
        (legacy_layout.about_file(), target_layout.about_file()),
        (legacy_layout.session_file(), target_layout.session_file()),
        (
            legacy_layout.installed_package_descriptions_file(),
            target_layout.installed_package_descriptions_file(),
        ),
        (legacy_layout.users_dir(), target_layout.users_dir()),
        (
            legacy_layout.journal_entries_dir(),
            target_layout.journal_entries_dir(),
        ),
    ] {
        merge_path_if_missing(&from, &to);
    }
}

fn merge_users_db_if_needed(from: &Path, to: &Path) {
    let Ok(source_raw) = std::fs::read_to_string(from) else {
        return;
    };
    let Ok(source) =
        serde_json::from_str::<serde_json::Map<String, serde_json::Value>>(&source_raw)
    else {
        return;
    };
    if source.is_empty() {
        return;
    }

    if !to.exists() {
        if let Some(parent) = to.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::copy(from, to);
        return;
    }

    let Ok(target_raw) = std::fs::read_to_string(to) else {
        return;
    };
    let Ok(mut target) =
        serde_json::from_str::<serde_json::Map<String, serde_json::Value>>(&target_raw)
    else {
        return;
    };

    let target_is_bootstrap_admin = target.len() == 1 && target.contains_key("admin");
    for (username, record) in source {
        if target_is_bootstrap_admin || !target.contains_key(&username) {
            target.insert(username, record);
        }
    }
    if let Ok(raw) = serde_json::to_string_pretty(&target) {
        let _ = std::fs::write(to, raw);
    }
}

fn merge_path_if_missing(from: &Path, to: &Path) {
    if !from.exists() {
        return;
    }

    if from.is_dir() {
        let _ = std::fs::create_dir_all(to);
        if let Ok(entries) = std::fs::read_dir(from) {
            for entry in entries.flatten() {
                let src = entry.path();
                let dst = to.join(entry.file_name());
                merge_path_if_missing(&src, &dst);
            }
        }
    } else {
        if from.file_name().and_then(|name| name.to_str()) == Some("users.json") {
            merge_users_db_if_needed(from, to);
            return;
        }
        if to.exists() {
            return;
        }
        if let Some(parent) = to.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::copy(from, to);
    }
}

#[cfg(test)]
fn compat_state_path_with_roots(
    relative: &Path,
    target_root: &Path,
    legacy_root: &Path,
) -> PathBuf {
    let target = target_root.join(relative);
    let legacy = legacy_root.join(relative);
    if target != legacy {
        merge_path_if_missing(&legacy, &target);
    }
    target
}

fn compat_state_path_for_target(relative: &Path, target: PathBuf) -> PathBuf {
    let legacy = base_dir().join(relative);
    if target != legacy {
        merge_path_if_missing(&legacy, &target);
    }
    target
}

pub fn home_dir_fallback() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
}

pub fn documents_root_dir() -> PathBuf {
    dirs::document_dir().unwrap_or_else(home_dir_fallback)
}

fn legacy_diagnostics_path() -> PathBuf {
    if let Some(home) = std::env::var_os("HOME") {
        return PathBuf::from(home)
            .join(".local")
            .join("share")
            .join("robcos")
            .join("diagnostics.log");
    }
    std::env::temp_dir().join("robcos_diagnostics.log")
}

fn word_processor_documents_dir_with_roots(
    username: &str,
    target_root: &Path,
    legacy_documents_root: &Path,
) -> PathBuf {
    let dir = target_root
        .join("users")
        .join(username)
        .join("documents")
        .join("word-processor");
    let legacy_dir = legacy_documents_root
        .join("ROBCO Word Processor")
        .join(username);
    if dir != legacy_dir {
        merge_path_if_missing(&legacy_dir, &dir);
    }
    let _ = std::fs::create_dir_all(&dir);
    dir
}

pub fn users_dir() -> PathBuf {
    let dir = compat_state_path_for_target(
        Path::new("users"),
        runtime_environment().state_layout().users_dir(),
    );
    let _ = std::fs::create_dir_all(&dir);
    dir
}

pub fn users_db_file() -> PathBuf {
    compat_state_path_for_target(
        &Path::new("users").join("users.json"),
        runtime_environment().state_layout().users_db_file(),
    )
}

pub fn user_dir(username: &str) -> PathBuf {
    let d = runtime_environment().state_layout().user_dir(username);
    let _ = std::fs::create_dir_all(&d);
    d
}

pub fn desktop_dir_for_username(username: &str) -> PathBuf {
    let d = runtime_environment()
        .state_layout()
        .desktop_dir_for_username(username);
    let _ = std::fs::create_dir_all(&d);
    d
}

pub fn desktop_dir() -> PathBuf {
    if let Some(username) = get_current_user() {
        desktop_dir_for_username(&username)
    } else {
        let dir = compat_state_path_for_target(
            Path::new("Desktop"),
            runtime_environment().state_layout().shared_desktop_dir(),
        );
        let _ = std::fs::create_dir_all(&dir);
        dir
    }
}

pub fn word_processor_documents_dir(username: &str) -> PathBuf {
    word_processor_documents_dir_with_roots(username, &state_root_dir(), &documents_root_dir())
}

pub fn journal_entries_dir() -> PathBuf {
    let dir = compat_state_path_for_target(
        Path::new("journal_entries"),
        runtime_environment().state_layout().journal_entries_dir(),
    );
    let _ = std::fs::create_dir_all(&dir);
    dir
}

pub fn diagnostics_log_file() -> PathBuf {
    let target = runtime_environment().state_layout().diagnostics_log_file();
    let legacy = legacy_diagnostics_path();
    if target != legacy {
        merge_path_if_missing(&legacy, &target);
    }
    target
}

pub fn pty_key_debug_log_file() -> PathBuf {
    runtime_environment()
        .runtime_layout()
        .pty_key_debug_log_file()
}

pub fn ipc_socket_file() -> PathBuf {
    runtime_environment().runtime_layout().ipc_socket_file()
}

pub fn file_manager_trash_dir() -> PathBuf {
    if let Some(username) = get_current_user() {
        let dir = runtime_environment()
            .state_layout()
            .file_manager_trash_dir_for_username(&username);
        let _ = std::fs::create_dir_all(&dir);
        dir
    } else {
        let dir = compat_state_path_for_target(
            Path::new(".fm_trash"),
            runtime_environment()
                .state_layout()
                .shared_file_manager_trash_dir(),
        );
        let _ = std::fs::create_dir_all(&dir);
        dir
    }
}

pub fn native_shell_snapshot_file(username: &str) -> PathBuf {
    runtime_environment()
        .state_layout()
        .native_shell_snapshot_file(username)
}

fn default_apps_prompt_marker(username: &str) -> PathBuf {
    runtime_environment()
        .state_layout()
        .default_apps_prompt_marker(username)
}

pub fn mark_default_apps_prompt_pending(username: &str) {
    let _ = std::fs::write(default_apps_prompt_marker(username), b"1");
}

pub fn take_default_apps_prompt_pending(username: &str) -> bool {
    let marker = default_apps_prompt_marker(username);
    if marker.exists() {
        let _ = std::fs::remove_file(marker);
        return true;
    }
    false
}

pub fn global_settings_file() -> PathBuf {
    compat_state_path_for_target(
        Path::new("settings.json"),
        runtime_environment().state_layout().global_settings_file(),
    )
}
pub fn about_file() -> PathBuf {
    compat_state_path_for_target(
        Path::new("about.json"),
        runtime_environment().state_layout().about_file(),
    )
}
pub fn session_state_file() -> PathBuf {
    compat_state_path_for_target(
        Path::new(".session"),
        runtime_environment().state_layout().session_file(),
    )
}
pub fn installed_package_descriptions_file() -> PathBuf {
    compat_state_path_for_target(
        Path::new("installed_package_descriptions.json"),
        runtime_environment()
            .state_layout()
            .installed_package_descriptions_file(),
    )
}

pub fn load_installed_package_descriptions<T: for<'de> Deserialize<'de> + Default>() -> T {
    load_json(&installed_package_descriptions_file())
}

pub fn save_installed_package_descriptions<T: Serialize>(descriptions: &T) {
    let _ = save_json(&installed_package_descriptions_file(), descriptions);
}

pub fn addon_state_overrides_file() -> PathBuf {
    compat_state_path_for_target(
        Path::new("addon_state.json"),
        runtime_environment()
            .state_layout()
            .addon_state_overrides_file(),
    )
}

pub fn load_addon_state_overrides() -> AddonStateOverrides {
    load_json(&addon_state_overrides_file())
}

pub fn save_addon_state_overrides(overrides: &AddonStateOverrides) {
    let path = addon_state_overrides_file();
    if overrides.is_empty() {
        let _ = std::fs::remove_file(path);
    } else {
        let _ = save_json(&path, overrides);
    }
}

pub const ALLOWED_EXTENSIONS: &[&str] = &[".pdf", ".epub", ".txt", ".mobi", ".azw3"];

pub fn is_allowed_extension(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| {
            ALLOWED_EXTENSIONS
                .iter()
                .any(|a| a.trim_start_matches('.') == e)
        })
        .unwrap_or(false)
}

// ── JSON helpers ──────────────────────────────────────────────────────────────

pub fn load_json<T: for<'de> Deserialize<'de> + Default>(path: &Path) -> T {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn save_json<T: Serialize>(path: &Path, data: &T) -> Result<()> {
    let json = serde_json::to_string_pretty(data)?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating parent dirs for {}", path.display()))?;
    }
    std::fs::write(path, json).with_context(|| format!("writing {}", path.display()))
}

// ── User-aware file helpers ───────────────────────────────────────────────────

pub fn user_settings_file(username: &str) -> PathBuf {
    runtime_environment()
        .state_layout()
        .user_settings_file(username)
}

pub fn current_settings_file() -> PathBuf {
    if let Some(username) = get_current_user() {
        runtime_environment()
            .state_layout()
            .user_settings_file(&username)
    } else {
        global_settings_file()
    }
}

fn current_apps_catalog_file() -> PathBuf {
    if let Some(username) = get_current_user() {
        runtime_environment()
            .state_layout()
            .user_apps_catalog_file(&username)
    } else {
        compat_state_path_for_target(
            Path::new("apps.json"),
            runtime_environment()
                .state_layout()
                .shared_apps_catalog_file(),
        )
    }
}

fn current_games_catalog_file() -> PathBuf {
    if let Some(username) = get_current_user() {
        runtime_environment()
            .state_layout()
            .user_games_catalog_file(&username)
    } else {
        compat_state_path_for_target(
            Path::new("games.json"),
            runtime_environment()
                .state_layout()
                .shared_games_catalog_file(),
        )
    }
}

fn current_networks_catalog_file() -> PathBuf {
    if let Some(username) = get_current_user() {
        runtime_environment()
            .state_layout()
            .user_networks_catalog_file(&username)
    } else {
        compat_state_path_for_target(
            Path::new("networks.json"),
            runtime_environment()
                .state_layout()
                .shared_networks_catalog_file(),
        )
    }
}

fn current_documents_catalog_file() -> PathBuf {
    if let Some(username) = get_current_user() {
        runtime_environment()
            .state_layout()
            .user_documents_catalog_file(&username)
    } else {
        compat_state_path_for_target(
            Path::new("documents.json"),
            runtime_environment()
                .state_layout()
                .shared_documents_catalog_file(),
        )
    }
}

pub fn load_apps() -> serde_json::Map<String, serde_json::Value> {
    load_json(&current_apps_catalog_file())
}
pub fn save_apps(d: &serde_json::Map<String, serde_json::Value>) {
    let _ = save_json(&current_apps_catalog_file(), d);
}

pub fn load_games() -> serde_json::Map<String, serde_json::Value> {
    load_json(&current_games_catalog_file())
}
pub fn save_games(d: &serde_json::Map<String, serde_json::Value>) {
    let _ = save_json(&current_games_catalog_file(), d);
}

pub fn load_networks() -> serde_json::Map<String, serde_json::Value> {
    load_json(&current_networks_catalog_file())
}
pub fn save_networks(d: &serde_json::Map<String, serde_json::Value>) {
    let _ = save_json(&current_networks_catalog_file(), d);
}

pub fn load_categories() -> serde_json::Map<String, serde_json::Value> {
    load_json(&current_documents_catalog_file())
}
pub fn save_categories(d: &serde_json::Map<String, serde_json::Value>) {
    let _ = save_json(&current_documents_catalog_file(), d);
}

pub fn load_about() -> AboutConfig {
    load_json(&about_file())
}

pub fn load_settings() -> Settings {
    if let Some(u) = get_current_user() {
        let f = user_settings_file(&u);
        if f.exists() {
            let mut s: Settings = load_json(&f);
            apply_legacy_settings_migrations(&mut s);
            return s;
        }
    }
    let mut s: Settings = load_json(&global_settings_file());
    apply_legacy_settings_migrations(&mut s);
    s
}
pub fn save_settings(d: &Settings) {
    if let Some(u) = get_current_user() {
        let _ = save_json(&user_settings_file(&u), d);
    } else {
        let _ = save_json(&global_settings_file(), d);
    }
}

// ── Settings ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum CliColorMode {
    #[default]
    ThemeLock,
    PaletteMap,
    Color,
    Monochrome,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum CliAcsMode {
    Ascii,
    #[default]
    Unicode,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum OpenMode {
    #[default]
    Terminal,
    Desktop,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum NativeStartupWindowMode {
    #[default]
    Windowed,
    #[serde(rename = "maximized_window")]
    Maximized,
    #[serde(
        rename = "borderless_fullscreen",
        alias = "maximized",
        alias = "desktop"
    )]
    BorderlessFullscreen,
    Fullscreen,
}

impl NativeStartupWindowMode {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Windowed => "Windowed",
            Self::Maximized => "Maximized",
            Self::BorderlessFullscreen => "Borderless Fullscreen",
            Self::Fullscreen => "Fullscreen",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum DefaultAppMenuSource {
    #[default]
    Applications,
    Games,
    Network,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DefaultAppBinding {
    Builtin {
        id: String,
    },
    MenuEntry {
        source: DefaultAppMenuSource,
        name: String,
    },
    CustomArgv {
        argv: Vec<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DefaultAppsSettings {
    #[serde(default = "default_default_app_text_code")]
    pub text_code: DefaultAppBinding,
    #[serde(default = "default_default_app_ebook")]
    pub ebook: DefaultAppBinding,
}

fn default_default_app_text_code() -> DefaultAppBinding {
    DefaultAppBinding::Builtin {
        id: "robco_terminal_writer".to_string(),
    }
}

fn default_default_app_ebook() -> DefaultAppBinding {
    DefaultAppBinding::Builtin {
        id: "robco_terminal_writer".to_string(),
    }
}

impl Default for DefaultAppsSettings {
    fn default() -> Self {
        Self {
            text_code: default_default_app_text_code(),
            ebook: default_default_app_ebook(),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum ConnectionKind {
    #[default]
    Network,
    Bluetooth,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct SavedConnection {
    pub name: String,
    #[serde(default)]
    pub detail: String,
    #[serde(default)]
    pub last_connected_unix: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ConnectionsSettings {
    #[serde(default)]
    pub network: Vec<SavedConnection>,
    #[serde(default)]
    pub bluetooth: Vec<SavedConnection>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum WallpaperSizeMode {
    DefaultSize,
    #[default]
    FitToScreen,
    Centered,
    Tile,
    Stretch,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum DesktopIconStyle {
    Dos,
    #[default]
    Win95,
    Minimal,
    NoIcons,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum FileManagerViewMode {
    #[default]
    Grid,
    List,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum FileManagerSortMode {
    #[default]
    Name,
    Type,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum FileManagerTextOpenMode {
    Editor,
    #[default]
    Viewer,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DesktopFileManagerSettings {
    #[serde(default)]
    pub show_hidden_files: bool,
    #[serde(default = "default_file_manager_tree_panel")]
    pub show_tree_panel: bool,
    #[serde(default)]
    pub view_mode: FileManagerViewMode,
    #[serde(default)]
    pub sort_mode: FileManagerSortMode,
    #[serde(default = "default_file_manager_dirs_first")]
    pub directories_first: bool,
    #[serde(default)]
    pub text_open_mode: FileManagerTextOpenMode,
    #[serde(default)]
    pub open_with_by_extension: BTreeMap<String, Vec<String>>,
    #[serde(default)]
    pub open_with_default_by_extension: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DesktopSessionSettings {
    #[serde(default)]
    pub reopen_last_file_manager: bool,
    #[serde(default)]
    pub file_manager_tabs: Vec<String>,
    #[serde(default)]
    pub active_file_manager_tab: usize,
    #[serde(default)]
    pub recent_folders: Vec<String>,
}

const fn default_file_manager_dirs_first() -> bool {
    true
}

const fn default_file_manager_tree_panel() -> bool {
    true
}

impl Default for DesktopFileManagerSettings {
    fn default() -> Self {
        Self {
            show_hidden_files: false,
            show_tree_panel: true,
            view_mode: FileManagerViewMode::Grid,
            sort_mode: FileManagerSortMode::Name,
            directories_first: true,
            text_open_mode: FileManagerTextOpenMode::Viewer,
            open_with_by_extension: BTreeMap::new(),
            open_with_default_by_extension: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DesktopIconPosition {
    pub x: i32,
    pub y: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DesktopIconPositionsSettings {
    #[serde(default)]
    pub my_computer: Option<DesktopIconPosition>,
    #[serde(default)]
    pub trash: Option<DesktopIconPosition>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum DesktopIconSortMode {
    #[default]
    Custom,
    ByName,
    ByType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DesktopShortcut {
    pub label: String,
    pub app_name: String,
    #[serde(default)]
    pub pos_x: Option<f32>,
    #[serde(default)]
    pub pos_y: Option<f32>,
    #[serde(default)]
    pub launch_command: Option<String>,
    #[serde(default)]
    pub icon_path: Option<String>,
    #[serde(default = "default_shortcut_kind")]
    pub shortcut_kind: String,
}

fn default_shortcut_kind() -> String {
    "app".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DesktopPtyProfileSettings {
    pub min_w: u16,
    pub min_h: u16,
    #[serde(default)]
    pub preferred_w: Option<u16>,
    #[serde(default)]
    pub preferred_h: Option<u16>,
    #[serde(default = "default_profile_mouse_passthrough")]
    pub mouse_passthrough: bool,
    #[serde(default = "default_profile_open_fullscreen")]
    pub open_fullscreen: bool,
    #[serde(default = "default_profile_live_resize")]
    pub live_resize: bool,
}

const fn default_profile_mouse_passthrough() -> bool {
    true
}

const fn default_profile_open_fullscreen() -> bool {
    false
}

const fn default_profile_live_resize() -> bool {
    true
}

impl Default for DesktopPtyProfileSettings {
    fn default() -> Self {
        Self {
            min_w: 34,
            min_h: 12,
            preferred_w: Some(96),
            preferred_h: Some(32),
            mouse_passthrough: true,
            open_fullscreen: false,
            live_resize: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DesktopCliProfiles {
    #[serde(default)]
    pub default: DesktopPtyProfileSettings,
    #[serde(default = "default_calcurse_profile")]
    pub calcurse: DesktopPtyProfileSettings,
    #[serde(default = "default_spotify_profile")]
    pub spotify_player: DesktopPtyProfileSettings,
    #[serde(default = "default_ranger_profile")]
    pub ranger: DesktopPtyProfileSettings,
    #[serde(default = "default_reddit_profile")]
    pub reddit: DesktopPtyProfileSettings,
    #[serde(default)]
    pub custom: BTreeMap<String, DesktopPtyProfileSettings>,
}

fn default_calcurse_profile() -> DesktopPtyProfileSettings {
    DesktopPtyProfileSettings {
        min_w: 72,
        min_h: 20,
        preferred_w: Some(108),
        preferred_h: Some(34),
        mouse_passthrough: false,
        open_fullscreen: false,
        live_resize: true,
    }
}

fn default_spotify_profile() -> DesktopPtyProfileSettings {
    DesktopPtyProfileSettings {
        min_w: 66,
        min_h: 18,
        preferred_w: Some(118),
        preferred_h: Some(34),
        mouse_passthrough: true,
        open_fullscreen: false,
        live_resize: true,
    }
}

fn default_ranger_profile() -> DesktopPtyProfileSettings {
    DesktopPtyProfileSettings {
        min_w: 60,
        min_h: 16,
        preferred_w: Some(108),
        preferred_h: Some(32),
        mouse_passthrough: true,
        open_fullscreen: false,
        live_resize: true,
    }
}

fn default_reddit_profile() -> DesktopPtyProfileSettings {
    DesktopPtyProfileSettings {
        min_w: 72,
        min_h: 20,
        preferred_w: Some(112),
        preferred_h: Some(34),
        mouse_passthrough: true,
        open_fullscreen: false,
        live_resize: true,
    }
}

impl Default for DesktopCliProfiles {
    fn default() -> Self {
        Self {
            default: DesktopPtyProfileSettings::default(),
            calcurse: default_calcurse_profile(),
            spotify_player: default_spotify_profile(),
            ranger: default_ranger_profile(),
            reddit: default_reddit_profile(),
            custom: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuiltinMenuVisibilitySettings {
    #[serde(default = "default_true")]
    pub text_editor: bool,
}

fn default_true() -> bool {
    true
}

fn default_navigation_hints() -> bool {
    true
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum HackingDifficulty {
    Easy,
    #[default]
    Normal,
    Hard,
}

fn default_hacking_difficulty() -> HackingDifficulty {
    HackingDifficulty::Normal
}

pub fn hacking_difficulty_label(difficulty: HackingDifficulty) -> &'static str {
    match difficulty {
        HackingDifficulty::Easy => "Easy",
        HackingDifficulty::Normal => "Normal",
        HackingDifficulty::Hard => "Hard",
    }
}

pub fn cycle_hacking_difficulty(current: HackingDifficulty, forward: bool) -> HackingDifficulty {
    match (current, forward) {
        (HackingDifficulty::Easy, true) => HackingDifficulty::Normal,
        (HackingDifficulty::Normal, true) => HackingDifficulty::Hard,
        (HackingDifficulty::Hard, true) => HackingDifficulty::Easy,
        (HackingDifficulty::Easy, false) => HackingDifficulty::Hard,
        (HackingDifficulty::Normal, false) => HackingDifficulty::Easy,
        (HackingDifficulty::Hard, false) => HackingDifficulty::Normal,
    }
}

impl Default for BuiltinMenuVisibilitySettings {
    fn default() -> Self {
        Self {
            text_editor: true,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum CrtPreset {
    Off,
    Subtle,
    #[default]
    RobCoStandard,
    WornTerminal,
    ExtremeRetro,
    Custom,
}

impl CrtPreset {
    pub fn label(self) -> &'static str {
        match self {
            Self::Off => "Off",
            Self::Subtle => "Subtle",
            Self::RobCoStandard => "RobCo Standard",
            Self::WornTerminal => "Worn Terminal",
            Self::ExtremeRetro => "Extreme Retro",
            Self::Custom => "Custom",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct DisplayEffectsSettings {
    #[serde(default = "default_display_effects_enabled")]
    pub enabled: bool,
    #[serde(default)]
    pub preset: CrtPreset,
    #[serde(default = "default_crt_curvature")]
    pub curvature: f32,
    #[serde(default = "default_crt_scanlines")]
    pub scanlines: f32,
    #[serde(default = "default_crt_glow")]
    pub glow: f32,
    #[serde(default = "default_crt_bloom")]
    pub bloom: f32,
    #[serde(default = "default_crt_vignette")]
    pub vignette: f32,
    #[serde(default = "default_crt_noise")]
    pub noise: f32,
    #[serde(default = "default_crt_flicker")]
    pub flicker: f32,
    #[serde(default = "default_crt_jitter")]
    pub jitter: f32,
    #[serde(default = "default_crt_burn_in")]
    pub burn_in: f32,
    #[serde(default = "default_crt_glow_line")]
    pub glow_line: f32,
    #[serde(default = "default_crt_glow_line_speed")]
    pub glow_line_speed: f32,
    #[serde(default = "default_crt_brightness")]
    pub brightness: f32,
    #[serde(default = "default_crt_contrast")]
    pub contrast: f32,
    #[serde(default = "default_crt_phosphor_softness")]
    pub phosphor_softness: f32,
}

impl DisplayEffectsSettings {
    pub fn from_preset(preset: CrtPreset) -> Self {
        match preset {
            CrtPreset::Off => Self {
                enabled: false,
                preset,
                curvature: 0.0,
                scanlines: 0.0,
                glow: 0.0,
                bloom: 0.0,
                vignette: 0.0,
                noise: 0.0,
                flicker: 0.0,
                jitter: 0.0,
                burn_in: 0.0,
                glow_line: 0.0,
                glow_line_speed: default_crt_glow_line_speed(),
                brightness: 1.0,
                contrast: 1.0,
                phosphor_softness: 0.0,
            },
            CrtPreset::Subtle => Self {
                enabled: true,
                preset,
                curvature: 0.025,
                scanlines: 0.2,
                glow: 0.18,
                bloom: 0.08,
                vignette: 0.12,
                noise: 0.01,
                flicker: 0.015,
                jitter: 0.003,
                burn_in: 0.04,
                glow_line: 0.06,
                glow_line_speed: 0.5,
                brightness: 1.0,
                contrast: 1.05,
                phosphor_softness: 0.08,
            },
            CrtPreset::RobCoStandard => Self {
                enabled: true,
                preset,
                curvature: default_crt_curvature(),
                scanlines: default_crt_scanlines(),
                glow: default_crt_glow(),
                bloom: default_crt_bloom(),
                vignette: default_crt_vignette(),
                noise: default_crt_noise(),
                flicker: default_crt_flicker(),
                jitter: default_crt_jitter(),
                burn_in: default_crt_burn_in(),
                glow_line: default_crt_glow_line(),
                glow_line_speed: default_crt_glow_line_speed(),
                brightness: default_crt_brightness(),
                contrast: default_crt_contrast(),
                phosphor_softness: default_crt_phosphor_softness(),
            },
            CrtPreset::WornTerminal => Self {
                enabled: true,
                preset,
                curvature: 0.08,
                scanlines: 0.45,
                glow: 0.45,
                bloom: 0.34,
                vignette: 0.35,
                noise: 0.1,
                flicker: 0.05,
                jitter: 0.024,
                burn_in: 0.26,
                glow_line: 0.18,
                glow_line_speed: 0.62,
                brightness: 0.98,
                contrast: 1.14,
                phosphor_softness: 0.28,
            },
            CrtPreset::ExtremeRetro => Self {
                enabled: true,
                preset,
                curvature: 0.16,
                scanlines: 0.8,
                glow: 0.9,
                bloom: 0.82,
                vignette: 0.65,
                noise: 0.2,
                flicker: 0.12,
                jitter: 0.065,
                burn_in: 0.58,
                glow_line: 0.42,
                glow_line_speed: 0.9,
                brightness: 0.94,
                contrast: 1.2,
                phosphor_softness: 0.45,
            },
            CrtPreset::Custom => {
                let mut settings = Self::from_preset(CrtPreset::RobCoStandard);
                settings.preset = CrtPreset::Custom;
                settings
            }
        }
    }

    pub fn apply_preset(&mut self, preset: CrtPreset) {
        *self = Self::from_preset(preset);
    }

    pub fn mark_custom(&mut self) {
        self.preset = CrtPreset::Custom;
    }

    pub fn needs_animation(&self) -> bool {
        self.enabled
            && (self.noise > 0.0
                || self.flicker > 0.0
                || self.jitter > 0.0
                || self.glow_line > 0.0
                || self.burn_in > 0.0)
    }
}

impl Default for DisplayEffectsSettings {
    fn default() -> Self {
        Self::from_preset(CrtPreset::RobCoStandard)
    }
}

fn default_display_effects_enabled() -> bool {
    true
}

fn default_crt_curvature() -> f32 {
    0.06
}

fn default_crt_scanlines() -> f32 {
    0.28
}

fn default_crt_glow() -> f32 {
    0.22
}

fn default_crt_bloom() -> f32 {
    0.18
}

fn default_crt_vignette() -> f32 {
    0.18
}

fn default_crt_noise() -> f32 {
    0.03
}

fn default_crt_flicker() -> f32 {
    0.012
}

fn default_crt_jitter() -> f32 {
    0.01
}

fn default_crt_burn_in() -> f32 {
    0.08
}

fn default_crt_glow_line() -> f32 {
    0.12
}

fn default_crt_glow_line_speed() -> f32 {
    0.72
}

fn default_crt_brightness() -> f32 {
    1.0
}

fn default_crt_contrast() -> f32 {
    1.08
}

fn default_crt_phosphor_softness() -> f32 {
    0.12
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub sound: bool,
    #[serde(default = "default_system_sound_volume")]
    pub system_sound_volume: u8,
    pub bootup: bool,
    pub theme: String,
    #[serde(default = "default_custom_theme_rgb")]
    pub custom_theme_rgb: [u8; 3],
    #[serde(default)]
    pub cli_styled_render: bool,
    #[serde(default)]
    pub cli_color_mode: CliColorMode,
    #[serde(default)]
    pub cli_acs_mode: CliAcsMode,
    #[serde(default)]
    pub default_open_mode: OpenMode,
    #[serde(default)]
    pub native_startup_window_mode: NativeStartupWindowMode,
    #[serde(default = "default_navigation_hints")]
    pub show_navigation_hints: bool,
    #[serde(default = "default_hacking_difficulty")]
    pub hacking_difficulty: HackingDifficulty,
    #[serde(default)]
    // Legacy field migrated into `builtin_menu_visibility`.
    pub hide_builtin_apps_in_menus: bool,
    #[serde(default)]
    pub builtin_menu_visibility: BuiltinMenuVisibilitySettings,
    #[serde(default)]
    pub default_apps: DefaultAppsSettings,
    #[serde(default)]
    pub connections: ConnectionsSettings,
    #[serde(default)]
    pub desktop_cli_profiles: DesktopCliProfiles,
    #[serde(default)]
    pub pty_shell_preferred: BTreeMap<String, bool>,
    #[serde(default = "default_desktop_wallpaper")]
    pub desktop_wallpaper: String,
    #[serde(default = "default_desktop_show_cursor")]
    pub desktop_show_cursor: bool,
    #[serde(default = "default_desktop_cursor_scale")]
    pub desktop_cursor_scale: f32,
    #[serde(default)]
    pub desktop_icon_style: DesktopIconStyle,
    #[serde(default)]
    pub desktop_wallpaper_size_mode: WallpaperSizeMode,
    #[serde(default)]
    pub desktop_file_manager: DesktopFileManagerSettings,
    #[serde(default)]
    pub desktop_session: DesktopSessionSettings,
    #[serde(default)]
    pub display_effects: DisplayEffectsSettings,
    #[serde(default)]
    pub desktop_icon_positions: DesktopIconPositionsSettings,
    #[serde(default)]
    pub desktop_wallpapers_custom: BTreeMap<String, Vec<String>>,
    #[serde(default = "default_native_ui_scale")]
    pub native_ui_scale: f32,
    #[serde(default)]
    pub desktop_shortcuts: Vec<DesktopShortcut>,
    #[serde(default)]
    pub desktop_icon_sort: DesktopIconSortMode,
    #[serde(default)]
    pub desktop_snap_to_grid: bool,
    #[serde(default)]
    pub desktop_icon_custom_positions: BTreeMap<String, [f32; 2]>,
    #[serde(default)]
    pub desktop_hidden_builtin_icons: BTreeSet<String>,
    #[serde(default)]
    pub editor_recent_files: Vec<String>,
    #[serde(default = "default_native_terminal_ui_highlighting")]
    pub native_terminal_ui_highlighting: bool,
}

const fn default_native_terminal_ui_highlighting() -> bool {
    true
}

fn default_desktop_wallpaper() -> String {
    "RobCo".to_string()
}

const fn default_desktop_show_cursor() -> bool {
    true
}

fn default_desktop_cursor_scale() -> f32 {
    1.0
}

const fn default_system_sound_volume() -> u8 {
    100
}

fn default_native_ui_scale() -> f32 {
    1.0
}

fn default_custom_theme_rgb() -> [u8; 3] {
    [0, 255, 0]
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            sound: true,
            system_sound_volume: default_system_sound_volume(),
            bootup: true,
            theme: "Green (Default)".into(),
            custom_theme_rgb: default_custom_theme_rgb(),
            cli_styled_render: false,
            cli_color_mode: CliColorMode::ThemeLock,
            cli_acs_mode: CliAcsMode::Unicode,
            default_open_mode: OpenMode::Terminal,
            native_startup_window_mode: NativeStartupWindowMode::Windowed,
            show_navigation_hints: default_navigation_hints(),
            hacking_difficulty: default_hacking_difficulty(),
            hide_builtin_apps_in_menus: false,
            builtin_menu_visibility: BuiltinMenuVisibilitySettings::default(),
            default_apps: DefaultAppsSettings::default(),
            connections: ConnectionsSettings::default(),
            desktop_cli_profiles: DesktopCliProfiles::default(),
            pty_shell_preferred: BTreeMap::new(),
            desktop_wallpaper: default_desktop_wallpaper(),
            desktop_show_cursor: default_desktop_show_cursor(),
            desktop_cursor_scale: default_desktop_cursor_scale(),
            desktop_icon_style: DesktopIconStyle::Win95,
            desktop_wallpaper_size_mode: WallpaperSizeMode::FitToScreen,
            desktop_file_manager: DesktopFileManagerSettings::default(),
            desktop_session: DesktopSessionSettings::default(),
            display_effects: DisplayEffectsSettings::default(),
            desktop_icon_positions: DesktopIconPositionsSettings::default(),
            desktop_wallpapers_custom: BTreeMap::new(),
            native_ui_scale: default_native_ui_scale(),
            desktop_shortcuts: Vec::new(),
            desktop_icon_sort: DesktopIconSortMode::Custom,
            desktop_snap_to_grid: false,
            desktop_icon_custom_positions: BTreeMap::new(),
            desktop_hidden_builtin_icons: BTreeSet::new(),
            editor_recent_files: Vec::new(),
            native_terminal_ui_highlighting: default_native_terminal_ui_highlighting(),
        }
    }
}

fn apply_legacy_settings_migrations(settings: &mut Settings) {
    settings.system_sound_volume = settings.system_sound_volume.clamp(0, 100);
    settings.desktop_cursor_scale = settings.desktop_cursor_scale.clamp(0.5, 2.5);
    if settings.hide_builtin_apps_in_menus {
        settings.builtin_menu_visibility.text_editor = false;
        settings.hide_builtin_apps_in_menus = false;
    }
    settings.display_effects.curvature = settings.display_effects.curvature.clamp(0.0, 0.2);
    settings.display_effects.scanlines = settings.display_effects.scanlines.clamp(0.0, 1.0);
    settings.display_effects.glow = settings.display_effects.glow.clamp(0.0, 1.5);
    settings.display_effects.bloom = settings.display_effects.bloom.clamp(0.0, 1.5);
    settings.display_effects.vignette = settings.display_effects.vignette.clamp(0.0, 1.0);
    settings.display_effects.noise = settings.display_effects.noise.clamp(0.0, 0.35);
    settings.display_effects.flicker = settings.display_effects.flicker.clamp(0.0, 0.3);
    settings.display_effects.jitter = settings.display_effects.jitter.clamp(0.0, 0.12);
    settings.display_effects.burn_in = settings.display_effects.burn_in.clamp(0.0, 1.0);
    settings.display_effects.glow_line = settings.display_effects.glow_line.clamp(0.0, 1.0);
    settings.display_effects.glow_line_speed =
        settings.display_effects.glow_line_speed.clamp(0.2, 2.0);
    settings.display_effects.brightness = settings.display_effects.brightness.clamp(0.5, 1.4);
    settings.display_effects.contrast = settings.display_effects.contrast.clamp(0.7, 1.5);
    settings.display_effects.phosphor_softness =
        settings.display_effects.phosphor_softness.clamp(0.0, 1.0);
}

// ── About config ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AboutConfig {
    pub ascii: Vec<String>,
    pub fields: Vec<String>,
}

// ── Global mutable state ──────────────────────────────────────────────────────

static CURRENT_USER: OnceLock<RwLock<Option<String>>> = OnceLock::new();
static APP_SETTINGS: OnceLock<RwLock<Settings>> = OnceLock::new();

fn user_lock() -> &'static RwLock<Option<String>> {
    CURRENT_USER.get_or_init(|| RwLock::new(None))
}
fn settings_lock() -> &'static RwLock<Settings> {
    APP_SETTINGS.get_or_init(|| RwLock::new(Settings::default()))
}

pub fn get_current_user() -> Option<String> {
    user_lock().read().ok()?.clone()
}

pub fn set_current_user(username: Option<&str>) {
    if let Ok(mut guard) = user_lock().write() {
        *guard = username.map(str::to_string);
    }
    if username.is_some() {
        reload_settings();
    }
}

pub fn get_settings() -> Settings {
    settings_lock()
        .read()
        .map(|g| g.clone())
        .unwrap_or_default()
}

pub fn with_settings<R>(f: impl FnOnce(&Settings) -> R) -> R {
    if let Ok(guard) = settings_lock().read() {
        return f(&guard);
    }
    let default = Settings::default();
    f(&default)
}

pub fn reload_settings() {
    let s = load_settings();
    if let Ok(mut guard) = settings_lock().write() {
        *guard = s;
    }
}

pub fn update_settings<F: FnOnce(&mut Settings)>(f: F) {
    if let Ok(mut guard) = settings_lock().write() {
        f(&mut guard);
    }
}

pub fn persist_settings() {
    let s = get_settings();
    save_settings(&s);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Mutex, OnceLock};

    struct TempDirGuard {
        path: PathBuf,
    }

    fn env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    impl TempDirGuard {
        fn new(prefix: &str) -> Self {
            let unique = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("test clock")
                .as_nanos();
            let path = std::env::temp_dir().join(format!(
                "robcos_shared_config_{prefix}_{}_{}",
                std::process::id(),
                unique
            ));
            std::fs::create_dir_all(&path).expect("create temp dir");
            Self { path }
        }
    }

    impl Drop for TempDirGuard {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }
    use serde_json::{json, Value};
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn unique_temp_dir(label: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("robcos-{label}-{unique}"));
        fs::create_dir_all(&dir).expect("create temp dir");
        dir
    }

    #[test]
    fn app_bundle_path_uses_app_support_dir() {
        let exe = PathBuf::from("/Applications/RobCoOS.app/Contents/MacOS/robcos");
        let bundle_dir = macos_app_bundle_dir(&exe).expect("bundle dir");
        assert_eq!(bundle_dir, PathBuf::from("/Applications/RobCoOS.app"));

        let resolved = macos_app_support_dir()
            .unwrap_or_else(|| PathBuf::from("/tmp"))
            .join("RobCoOS");
        assert!(resolved.ends_with("RobCoOS"));
    }

    #[test]
    fn non_bundle_path_uses_executable_parent() {
        let exe = PathBuf::from("/tmp/robcos/bin/robcos");
        assert!(macos_app_bundle_dir(&exe).is_none());
        assert_eq!(
            exe.parent().expect("exe parent"),
            Path::new("/tmp/robcos/bin")
        );
    }

    #[test]
    fn merge_users_db_prefers_legacy_over_bootstrap_admin() {
        let dir = unique_temp_dir("users-merge");
        let source = dir.join("source-users.json");
        let target = dir.join("target-users.json");

        fs::write(
            &source,
            serde_json::to_string_pretty(&json!({
                "admin": { "password_hash": "legacy", "is_admin": true, "auth_method": "password" },
                "adi": { "password_hash": "user", "is_admin": false, "auth_method": "password" }
            }))
            .expect("source json"),
        )
        .expect("write source");
        fs::write(
            &target,
            serde_json::to_string_pretty(&json!({
                "admin": { "password_hash": "bootstrap", "is_admin": true, "auth_method": "password" }
            }))
            .expect("target json"),
        )
        .expect("write target");

        merge_users_db_if_needed(&source, &target);

        let merged: serde_json::Map<String, Value> =
            serde_json::from_str(&fs::read_to_string(&target).expect("read merged"))
                .expect("decode merged");
        assert_eq!(merged["admin"]["password_hash"].as_str(), Some("legacy"));
        assert!(merged.contains_key("adi"));

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn bundle_migration_merges_old_macos_runtime_state_into_target_dir() {
        let dir = unique_temp_dir("bundle-migrate");
        let bundle_dir = dir.join("RobCoOS.app");
        let macos_dir = bundle_dir.join("Contents").join("MacOS");
        let exe = macos_dir.join("robcos");
        let target = dir.join("Application Support").join("RobCoOS");

        fs::create_dir_all(macos_dir.join("users").join("admin")).expect("create legacy dirs");
        fs::create_dir_all(target.join("users")).expect("create target dirs");
        fs::write(&exe, b"").expect("write exe placeholder");
        fs::write(
            macos_dir.join("users").join("users.json"),
            serde_json::to_string_pretty(&json!({
                "admin": { "password_hash": "legacy", "is_admin": true, "auth_method": "password" },
                "adi": { "password_hash": "user", "is_admin": false, "auth_method": "password" }
            }))
            .expect("legacy users json"),
        )
        .expect("write legacy users");
        fs::write(
            macos_dir.join("users").join("admin").join("apps.json"),
            serde_json::to_string_pretty(&json!({
                "Firefox": ["open", "-a", "Firefox"]
            }))
            .expect("legacy apps json"),
        )
        .expect("write legacy apps");
        fs::write(
            target.join("users").join("users.json"),
            serde_json::to_string_pretty(&json!({
                "admin": { "password_hash": "bootstrap", "is_admin": true, "auth_method": "password" }
            }))
            .expect("target users json"),
        )
        .expect("write target users");

        migrate_bundle_runtime_data_if_needed(&target, &exe, &bundle_dir);

        let merged_users: serde_json::Map<String, Value> = serde_json::from_str(
            &fs::read_to_string(target.join("users").join("users.json"))
                .expect("read merged users"),
        )
        .expect("decode merged users");
        assert_eq!(
            merged_users["admin"]["password_hash"].as_str(),
            Some("legacy")
        );
        assert!(merged_users.contains_key("adi"));
        assert!(target
            .join("users")
            .join("admin")
            .join("apps.json")
            .exists());

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn legacy_hide_builtin_apps_flag_migrates_to_visibility() {
        let mut settings = Settings::default();
        settings.hide_builtin_apps_in_menus = true;
        settings.builtin_menu_visibility.text_editor = true;

        apply_legacy_settings_migrations(&mut settings);

        assert!(!settings.hide_builtin_apps_in_menus);
        assert!(!settings.builtin_menu_visibility.text_editor);
    }

    #[test]
    fn desktop_file_manager_new_default_open_with_field_defaults_when_missing() {
        let mut value = serde_json::to_value(Settings::default()).expect("serialize settings");
        let fm = value
            .get_mut("desktop_file_manager")
            .and_then(serde_json::Value::as_object_mut)
            .expect("desktop_file_manager object");
        fm.remove("open_with_default_by_extension");

        let decoded: Settings = serde_json::from_value(value).expect("decode settings");
        assert!(decoded
            .desktop_file_manager
            .open_with_default_by_extension
            .is_empty());
    }

    #[test]
    fn show_navigation_hints_defaults_when_missing() {
        let mut value = serde_json::to_value(Settings::default()).expect("serialize settings");
        let obj = value.as_object_mut().expect("settings object");
        obj.remove("show_navigation_hints");

        let decoded: Settings = serde_json::from_value(value).expect("decode settings");
        assert!(decoded.show_navigation_hints);
    }

    #[test]
    fn native_startup_window_mode_defaults_when_missing() {
        let mut value = serde_json::to_value(Settings::default()).expect("serialize settings");
        let obj = value.as_object_mut().expect("settings object");
        obj.remove("native_startup_window_mode");

        let decoded: Settings = serde_json::from_value(value).expect("decode settings");
        assert_eq!(
            decoded.native_startup_window_mode,
            NativeStartupWindowMode::Windowed
        );
    }

    #[test]
    fn native_startup_window_mode_decodes_legacy_maximized_as_borderless_fullscreen() {
        let mut value = serde_json::to_value(Settings::default()).expect("serialize settings");
        let obj = value.as_object_mut().expect("settings object");
        obj.insert(
            "native_startup_window_mode".to_string(),
            serde_json::Value::String("maximized".to_string()),
        );

        let decoded: Settings = serde_json::from_value(value).expect("decode settings");
        assert_eq!(
            decoded.native_startup_window_mode,
            NativeStartupWindowMode::BorderlessFullscreen
        );
    }

    #[test]
    fn hacking_difficulty_defaults_when_missing() {
        let mut value = serde_json::to_value(Settings::default()).expect("serialize settings");
        let obj = value.as_object_mut().expect("settings object");
        obj.remove("hacking_difficulty");

        let decoded: Settings = serde_json::from_value(value).expect("decode settings");
        assert_eq!(decoded.hacking_difficulty, HackingDifficulty::Normal);
    }

    #[test]
    fn native_ui_scale_defaults_when_missing() {
        let mut value = serde_json::to_value(Settings::default()).expect("serialize settings");
        let obj = value.as_object_mut().expect("settings object");
        obj.remove("native_ui_scale");

        let decoded: Settings = serde_json::from_value(value).expect("decode settings");
        assert!((decoded.native_ui_scale - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn pty_shell_preferred_defaults_when_missing() {
        let mut value = serde_json::to_value(Settings::default()).expect("serialize settings");
        let obj = value.as_object_mut().expect("settings object");
        obj.remove("pty_shell_preferred");

        let decoded: Settings = serde_json::from_value(value).expect("decode settings");
        assert!(decoded.pty_shell_preferred.is_empty());
    }

    #[test]
    fn custom_theme_rgb_defaults_when_missing() {
        let mut value = serde_json::to_value(Settings::default()).expect("serialize settings");
        let obj = value.as_object_mut().expect("settings object");
        obj.remove("custom_theme_rgb");

        let decoded: Settings = serde_json::from_value(value).expect("decode settings");
        assert_eq!(decoded.custom_theme_rgb, [0, 255, 0]);
    }

    #[test]
    fn display_effects_new_fields_default_when_missing() {
        let mut value = serde_json::to_value(Settings::default()).expect("serialize settings");
        let display_effects = value
            .get_mut("display_effects")
            .and_then(serde_json::Value::as_object_mut)
            .expect("display_effects object");
        display_effects.remove("bloom");
        display_effects.remove("jitter");
        display_effects.remove("burn_in");
        display_effects.remove("glow_line");
        display_effects.remove("glow_line_speed");

        let decoded: Settings = serde_json::from_value(value).expect("decode settings");
        assert!((decoded.display_effects.bloom - default_crt_bloom()).abs() < f32::EPSILON);
        assert!((decoded.display_effects.jitter - default_crt_jitter()).abs() < f32::EPSILON);
        assert!((decoded.display_effects.burn_in - default_crt_burn_in()).abs() < f32::EPSILON);
        assert!((decoded.display_effects.glow_line - default_crt_glow_line()).abs() < f32::EPSILON);
        assert!(
            (decoded.display_effects.glow_line_speed - default_crt_glow_line_speed()).abs()
                < f32::EPSILON
        );
    }

    #[test]
    fn desktop_dir_for_username_lives_under_user_dir() {
        let desktop = desktop_dir_for_username("adi");
        assert!(desktop.ends_with(Path::new("users").join("adi").join("Desktop")));
    }

    #[test]
    fn word_processor_documents_dir_lives_under_user_dir() {
        let dir = word_processor_documents_dir("adi");
        assert!(dir.ends_with(
            Path::new("users")
                .join("adi")
                .join("documents")
                .join("word-processor")
        ));
    }

    #[test]
    fn word_processor_documents_dir_with_roots_migrates_legacy_documents_forward() {
        let temp = TempDirGuard::new("word_processor_documents");
        let target_root = temp.path.join("target");
        let legacy_documents_root = temp.path.join("legacy-documents");
        let legacy_dir = legacy_documents_root
            .join("ROBCO Word Processor")
            .join("adi");
        std::fs::create_dir_all(&legacy_dir).expect("create legacy documents dir");
        std::fs::write(legacy_dir.join("notes.txt"), "hello").expect("write legacy doc");

        let dir =
            word_processor_documents_dir_with_roots("adi", &target_root, &legacy_documents_root);

        assert_eq!(
            dir,
            target_root
                .join("users")
                .join("adi")
                .join("documents")
                .join("word-processor")
        );
        assert_eq!(
            std::fs::read_to_string(dir.join("notes.txt")).expect("read migrated doc"),
            "hello"
        );
    }

    #[test]
    fn user_settings_file_lives_under_user_dir() {
        let settings = user_settings_file("adi");
        assert!(settings.ends_with(Path::new("users").join("adi").join("settings.json")));
    }

    #[test]
    fn native_shell_snapshot_file_lives_under_user_dir() {
        let snapshot = native_shell_snapshot_file("adi");
        assert!(snapshot.ends_with(Path::new("users").join("adi").join("native_shell.json")));
    }

    #[test]
    fn file_manager_trash_dir_uses_user_root_when_current_user_is_set() {
        let previous = get_current_user();
        set_current_user(Some("adi"));

        let trash = file_manager_trash_dir();

        set_current_user(previous.as_deref());
        assert!(trash.ends_with(Path::new("users").join("adi").join(".fm_trash")));
    }

    #[test]
    fn diagnostics_log_file_lives_under_state_root() {
        let diagnostics = diagnostics_log_file();
        assert_eq!(diagnostics, state_root_dir().join("diagnostics.log"));
    }

    #[test]
    fn pty_key_debug_log_file_lives_under_runtime_root() {
        let key_log = pty_key_debug_log_file();
        assert_eq!(key_log, runtime_root_dir().join("robcos_keys.log"));
    }

    #[test]
    fn bundled_binary_path_lives_under_bundled_bin_dir() {
        let binary = bundled_binary_path("robcos-editor");
        assert_eq!(binary, bundled_bin_dir().join("robcos-editor"));
    }

    #[test]
    fn addon_repository_index_url_prefers_nucleon_env_and_falls_back_to_legacy() {
        let _guard = env_lock().lock().unwrap();
        unsafe {
            std::env::remove_var(ADDON_REPOSITORY_INDEX_URL_ENV);
            std::env::remove_var(LEGACY_ADDON_REPOSITORY_INDEX_URL_ENV);
            std::env::set_var(
                LEGACY_ADDON_REPOSITORY_INDEX_URL_ENV,
                "https://legacy.example.invalid/index.json",
            );
        }
        assert_eq!(
            addon_repository_index_url(),
            "https://legacy.example.invalid/index.json"
        );
        unsafe {
            std::env::set_var(
                ADDON_REPOSITORY_INDEX_URL_ENV,
                "https://nucleon.example.invalid/index.json",
            );
        }
        assert_eq!(
            addon_repository_index_url(),
            "https://nucleon.example.invalid/index.json"
        );
        unsafe {
            std::env::remove_var(ADDON_REPOSITORY_INDEX_URL_ENV);
            std::env::remove_var(LEGACY_ADDON_REPOSITORY_INDEX_URL_ENV);
        }
    }

    #[test]
    fn ipc_socket_file_lives_under_runtime_root() {
        let socket = ipc_socket_file();
        assert_eq!(socket, runtime_root_dir().join("shell.sock"));
    }

    #[test]
    fn current_settings_file_falls_back_to_global_when_no_user() {
        set_current_user(None);
        assert_eq!(current_settings_file(), global_settings_file());
    }

    #[test]
    fn compat_state_path_with_roots_copies_legacy_file_forward() {
        let temp = TempDirGuard::new("compat_state_file");
        let target_root = temp.path.join("target");
        let legacy_root = temp.path.join("legacy");
        std::fs::create_dir_all(&legacy_root).expect("create legacy root");
        std::fs::write(
            legacy_root.join("settings.json"),
            "{\"theme\":\"Green (Default)\"}",
        )
        .expect("write legacy settings");

        let target =
            compat_state_path_with_roots(Path::new("settings.json"), &target_root, &legacy_root);

        assert_eq!(
            std::fs::read_to_string(target).expect("read migrated settings"),
            "{\"theme\":\"Green (Default)\"}"
        );
    }

    #[test]
    fn compat_state_path_with_roots_copies_legacy_directories_forward() {
        let temp = TempDirGuard::new("compat_state_dir");
        let target_root = temp.path.join("target");
        let legacy_root = temp.path.join("legacy");
        let legacy_users = legacy_root.join("users").join("alice");
        std::fs::create_dir_all(&legacy_users).expect("create legacy users dir");
        std::fs::write(legacy_users.join("profile.json"), "{\"ok\":true}")
            .expect("write legacy user file");

        let target = compat_state_path_with_roots(Path::new("users"), &target_root, &legacy_root);

        assert_eq!(
            std::fs::read_to_string(target.join("alice").join("profile.json"))
                .expect("read migrated user file"),
            "{\"ok\":true}"
        );
    }
}

// ── Themes ────────────────────────────────────────────────────────────────────

use ratatui::style::Color;

pub const CUSTOM_THEME_NAME: &str = "Custom";

pub const THEMES: &[(&str, Color)] = &[
    ("Green (Default)", Color::Green),
    ("White", Color::White),
    ("Amber", Color::Yellow),
    ("Blue", Color::Blue),
    ("Red", Color::Red),
    ("Purple", Color::Magenta),
    ("Light Blue", Color::Cyan),
    (CUSTOM_THEME_NAME, Color::Green),
];

pub fn theme_color(name: &str) -> Color {
    if name == CUSTOM_THEME_NAME {
        let [r, g, b] = default_custom_theme_rgb();
        return Color::Rgb(r, g, b);
    }
    THEMES
        .iter()
        .find(|(n, _)| *n == name)
        .map(|(_, c)| *c)
        .unwrap_or(Color::Green)
}

pub fn theme_color_for_settings(settings: &Settings) -> Color {
    if settings.theme == CUSTOM_THEME_NAME {
        let [r, g, b] = settings.custom_theme_rgb;
        return Color::Rgb(r, g, b);
    }
    theme_color(&settings.theme)
}

pub fn current_theme_color() -> Color {
    with_settings(theme_color_for_settings)
}

// ── Header ────────────────────────────────────────────────────────────────────

pub const HEADER_LINES: &[&str] = &[
    "ROBCO INDUSTRIES UNIFIED OPERATING SYSTEM",
    "COPYRIGHT 2075-2077 ROBCO INDUSTRIES",
    "-SERVER 1-",
];
