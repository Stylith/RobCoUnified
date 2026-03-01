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

fn default_apps_prompt_marker(username: &str) -> PathBuf {
    user_dir(username).join(".default_apps_prompt")
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
    DefaultAppBinding::CustomArgv {
        argv: vec!["epy".to_string()],
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
pub struct BuiltinMenuVisibilitySettings {
    #[serde(default = "default_true")]
    pub nuke_codes: bool,
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
            nuke_codes: true,
            text_editor: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub sound: bool,
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
    #[serde(default = "default_desktop_wallpaper")]
    pub desktop_wallpaper: String,
    #[serde(default)]
    pub desktop_show_cursor: bool,
    #[serde(default)]
    pub desktop_icon_style: DesktopIconStyle,
    #[serde(default)]
    pub desktop_wallpaper_size_mode: WallpaperSizeMode,
    #[serde(default)]
    pub desktop_file_manager: DesktopFileManagerSettings,
    #[serde(default)]
    pub desktop_session: DesktopSessionSettings,
    #[serde(default)]
    pub desktop_icon_positions: DesktopIconPositionsSettings,
    #[serde(default)]
    pub desktop_wallpapers_custom: BTreeMap<String, Vec<String>>,
    #[serde(default = "default_native_ui_scale")]
    pub native_ui_scale: f32,
}

fn default_desktop_wallpaper() -> String {
    "RobCo".to_string()
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
            bootup: true,
            theme: "Green (Default)".into(),
            custom_theme_rgb: default_custom_theme_rgb(),
            cli_styled_render: false,
            cli_color_mode: CliColorMode::ThemeLock,
            cli_acs_mode: CliAcsMode::Unicode,
            default_open_mode: OpenMode::Terminal,
            show_navigation_hints: default_navigation_hints(),
            hacking_difficulty: default_hacking_difficulty(),
            hide_builtin_apps_in_menus: false,
            builtin_menu_visibility: BuiltinMenuVisibilitySettings::default(),
            default_apps: DefaultAppsSettings::default(),
            connections: ConnectionsSettings::default(),
            desktop_cli_profiles: DesktopCliProfiles::default(),
            desktop_wallpaper: default_desktop_wallpaper(),
            desktop_show_cursor: false,
            desktop_icon_style: DesktopIconStyle::Win95,
            desktop_wallpaper_size_mode: WallpaperSizeMode::FitToScreen,
            desktop_file_manager: DesktopFileManagerSettings::default(),
            desktop_session: DesktopSessionSettings::default(),
            desktop_icon_positions: DesktopIconPositionsSettings::default(),
            desktop_wallpapers_custom: BTreeMap::new(),
            native_ui_scale: default_native_ui_scale(),
        }
    }
}

fn apply_legacy_settings_migrations(settings: &mut Settings) {
    if settings.hide_builtin_apps_in_menus {
        settings.builtin_menu_visibility.nuke_codes = false;
        settings.builtin_menu_visibility.text_editor = false;
        settings.hide_builtin_apps_in_menus = false;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_hide_builtin_apps_flag_migrates_to_visibility() {
        let mut settings = Settings::default();
        settings.hide_builtin_apps_in_menus = true;
        settings.builtin_menu_visibility.nuke_codes = true;
        settings.builtin_menu_visibility.text_editor = true;

        apply_legacy_settings_migrations(&mut settings);

        assert!(!settings.hide_builtin_apps_in_menus);
        assert!(!settings.builtin_menu_visibility.nuke_codes);
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
    fn custom_theme_rgb_defaults_when_missing() {
        let mut value = serde_json::to_value(Settings::default()).expect("serialize settings");
        let obj = value.as_object_mut().expect("settings object");
        obj.remove("custom_theme_rgb");

        let decoded: Settings = serde_json::from_value(value).expect("decode settings");
        assert_eq!(decoded.custom_theme_rgb, [0, 255, 0]);
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
    let settings = get_settings();
    theme_color_for_settings(&settings)
}

// ── Header ────────────────────────────────────────────────────────────────────

pub const HEADER_LINES: &[&str] = &[
    "ROBCO INDUSTRIES UNIFIED OPERATING SYSTEM",
    "COPYRIGHT 2075-2077 ROBCO INDUSTRIES",
    "-SERVER 1-",
];
