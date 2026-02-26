use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{OnceLock, RwLock};

// ── Paths ─────────────────────────────────────────────────────────────────────

pub fn base_dir() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."))
}

pub fn users_dir() -> PathBuf {
    let d = base_dir().join("users");
    let _ = std::fs::create_dir_all(&d);
    d
}

pub fn user_dir(username: &str) -> PathBuf {
    let d = users_dir().join(username);
    let _ = std::fs::create_dir_all(&d);
    d
}

pub fn global_settings_file() -> PathBuf {
    base_dir().join("settings.json")
}
pub fn about_file() -> PathBuf {
    base_dir().join("about.json")
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
    std::fs::write(path, json).with_context(|| format!("writing {}", path.display()))
}

// ── User-aware file helpers ───────────────────────────────────────────────────

fn user_file(filename: &str) -> PathBuf {
    if let Some(u) = get_current_user() {
        user_dir(&u).join(filename)
    } else {
        base_dir().join(filename)
    }
}

pub fn load_apps() -> serde_json::Map<String, serde_json::Value> {
    load_json(&user_file("apps.json"))
}
pub fn save_apps(d: &serde_json::Map<String, serde_json::Value>) {
    let _ = save_json(&user_file("apps.json"), d);
}

pub fn load_games() -> serde_json::Map<String, serde_json::Value> {
    load_json(&user_file("games.json"))
}
pub fn save_games(d: &serde_json::Map<String, serde_json::Value>) {
    let _ = save_json(&user_file("games.json"), d);
}

pub fn load_networks() -> serde_json::Map<String, serde_json::Value> {
    load_json(&user_file("networks.json"))
}
pub fn save_networks(d: &serde_json::Map<String, serde_json::Value>) {
    let _ = save_json(&user_file("networks.json"), d);
}

pub fn load_categories() -> serde_json::Map<String, serde_json::Value> {
    load_json(&user_file("documents.json"))
}
pub fn save_categories(d: &serde_json::Map<String, serde_json::Value>) {
    let _ = save_json(&user_file("documents.json"), d);
}

pub fn load_about() -> AboutConfig {
    load_json(&about_file())
}

pub fn load_settings() -> Settings {
    if let Some(u) = get_current_user() {
        let f = user_dir(&u).join("settings.json");
        if f.exists() {
            return load_json(&f);
        }
    }
    load_json(&global_settings_file())
}
pub fn save_settings(d: &Settings) {
    if let Some(u) = get_current_user() {
        let _ = save_json(&user_dir(&u).join("settings.json"), d);
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
        }
    }
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
}

const fn default_profile_mouse_passthrough() -> bool {
    true
}

const fn default_profile_open_fullscreen() -> bool {
    false
}

impl Default for DesktopPtyProfileSettings {
    fn default() -> Self {
        Self {
            min_w: 34,
            min_h: 12,
            preferred_w: None,
            preferred_h: None,
            mouse_passthrough: true,
            open_fullscreen: false,
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
pub struct Settings {
    pub sound: bool,
    pub bootup: bool,
    pub theme: String,
    #[serde(default)]
    pub cli_styled_render: bool,
    #[serde(default)]
    pub cli_color_mode: CliColorMode,
    #[serde(default)]
    pub cli_acs_mode: CliAcsMode,
    #[serde(default)]
    pub default_open_mode: OpenMode,
    #[serde(default)]
    pub desktop_cli_profiles: DesktopCliProfiles,
    #[serde(default = "default_desktop_wallpaper")]
    pub desktop_wallpaper: String,
    #[serde(default)]
    pub desktop_wallpaper_size_mode: WallpaperSizeMode,
    #[serde(default)]
    pub desktop_file_manager: DesktopFileManagerSettings,
    #[serde(default)]
    pub desktop_wallpapers_custom: BTreeMap<String, Vec<String>>,
}

fn default_desktop_wallpaper() -> String {
    "RobCo".to_string()
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            sound: true,
            bootup: true,
            theme: "Green (Default)".into(),
            cli_styled_render: false,
            cli_color_mode: CliColorMode::ThemeLock,
            cli_acs_mode: CliAcsMode::Unicode,
            default_open_mode: OpenMode::Terminal,
            desktop_cli_profiles: DesktopCliProfiles::default(),
            desktop_wallpaper: default_desktop_wallpaper(),
            desktop_wallpaper_size_mode: WallpaperSizeMode::FitToScreen,
            desktop_file_manager: DesktopFileManagerSettings::default(),
            desktop_wallpapers_custom: BTreeMap::new(),
        }
    }
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

// ── Themes ────────────────────────────────────────────────────────────────────

use ratatui::style::Color;

pub const THEMES: &[(&str, Color)] = &[
    ("Green (Default)", Color::Green),
    ("White", Color::White),
    ("Amber", Color::Yellow),
    ("Blue", Color::Blue),
    ("Red", Color::Red),
    ("Purple", Color::Magenta),
    ("Light Blue", Color::Cyan),
];

pub fn theme_color(name: &str) -> Color {
    THEMES
        .iter()
        .find(|(n, _)| *n == name)
        .map(|(_, c)| *c)
        .unwrap_or(Color::Green)
}

pub fn current_theme_color() -> Color {
    theme_color(&get_settings().theme)
}

// ── Header ────────────────────────────────────────────────────────────────────

pub const HEADER_LINES: &[&str] = &[
    "ROBCO INDUSTRIES UNIFIED OPERATING SYSTEM",
    "COPYRIGHT 2075-2077 ROBCO INDUSTRIES",
    "-SERVER 1-",
];
