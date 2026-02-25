use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
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
