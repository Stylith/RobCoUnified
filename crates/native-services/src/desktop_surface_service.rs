use super::shared_types::DesktopWindow;
use crate::config::{
    DesktopIconSortMode, DesktopIconStyle, DesktopShortcut, Settings, WallpaperSizeMode,
};
use std::collections::{BTreeSet, HashMap};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DesktopBuiltinIconKind {
    FileManager,
    Editor,
    Installer,
    Settings,
    NukeCodes,
    Terminal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DesktopBuiltinIconEntry {
    pub kind: DesktopBuiltinIconKind,
    pub key: &'static str,
    pub label: &'static str,
    pub ascii: &'static str,
    pub target_window: Option<DesktopWindow>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DesktopIconGridLayout {
    pub left: f32,
    pub top: f32,
    pub height: f32,
    pub item_height: f32,
    pub column_width: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DesktopIconDragGrid {
    pub cell_w: f32,
    pub cell_h: f32,
    pub snap_to_grid: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DesktopSurfaceItemKind {
    Folder,
    TextFile,
    ImageFile,
    AudioFile,
    VideoFile,
    ArchiveFile,
    AppFile,
    File,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DesktopSurfaceEntry {
    pub key: String,
    pub path: PathBuf,
    pub label: String,
    pub kind: DesktopSurfaceItemKind,
}

impl DesktopSurfaceEntry {
    pub fn is_dir(&self) -> bool {
        self.kind == DesktopSurfaceItemKind::Folder
    }
}

const DESKTOP_BUILTIN_ICONS: [DesktopBuiltinIconEntry; 6] = [
    DesktopBuiltinIconEntry {
        kind: DesktopBuiltinIconKind::FileManager,
        key: "builtin_0",
        label: "Files",
        ascii: "[DIR]",
        target_window: Some(DesktopWindow::FileManager),
    },
    DesktopBuiltinIconEntry {
        kind: DesktopBuiltinIconKind::Editor,
        key: "builtin_1",
        label: "Documents",
        ascii: "[TXT]",
        target_window: Some(DesktopWindow::Editor),
    },
    DesktopBuiltinIconEntry {
        kind: DesktopBuiltinIconKind::Installer,
        key: "builtin_2",
        label: "Program Installer",
        ascii: "[PKG]",
        target_window: Some(DesktopWindow::Installer),
    },
    DesktopBuiltinIconEntry {
        kind: DesktopBuiltinIconKind::Settings,
        key: "builtin_3",
        label: "Settings",
        ascii: "[CFG]",
        target_window: Some(DesktopWindow::Settings),
    },
    DesktopBuiltinIconEntry {
        kind: DesktopBuiltinIconKind::NukeCodes,
        key: "builtin_4",
        label: "Nuke Codes",
        ascii: "[!]",
        target_window: Some(DesktopWindow::NukeCodes),
    },
    DesktopBuiltinIconEntry {
        kind: DesktopBuiltinIconKind::Terminal,
        key: "builtin_5",
        label: "Terminal",
        ascii: "[_]",
        target_window: None,
    },
];

pub fn desktop_builtin_icons() -> &'static [DesktopBuiltinIconEntry] {
    &DESKTOP_BUILTIN_ICONS
}

pub fn wallpaper_browser_start_dir(current_wallpaper: &str) -> PathBuf {
    let trimmed = current_wallpaper.trim();
    if !trimmed.is_empty() {
        let candidate = PathBuf::from(trimmed);
        if let Some(parent) = candidate
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
        {
            if candidate.extension().is_some() || candidate.is_file() {
                return parent.to_path_buf();
            }
        }
    }

    dirs::picture_dir()
        .or_else(dirs::home_dir)
        .unwrap_or_else(|| PathBuf::from("."))
}

pub fn set_wallpaper_path(settings: &mut Settings, path: &Path) {
    settings.desktop_wallpaper = path.to_string_lossy().to_string();
}

pub fn set_wallpaper_size_mode(settings: &mut Settings, mode: WallpaperSizeMode) {
    settings.desktop_wallpaper_size_mode = mode;
}

pub fn set_desktop_icon_style(settings: &mut Settings, style: DesktopIconStyle) {
    settings.desktop_icon_style = style;
}

pub fn set_builtin_icon_visible(settings: &mut Settings, key: &str, visible: bool) {
    if visible {
        settings.desktop_hidden_builtin_icons.remove(key);
    } else {
        settings
            .desktop_hidden_builtin_icons
            .insert(key.to_string());
    }
}

pub fn icon_position(
    settings: &Settings,
    key: &str,
    fallback: [f32; 2],
    default_positions: &HashMap<String, [f32; 2]>,
) -> [f32; 2] {
    settings
        .desktop_icon_custom_positions
        .get(key)
        .copied()
        .or_else(|| default_positions.get(key).copied())
        .unwrap_or(fallback)
}

pub fn update_dragged_icon_position(
    settings: &mut Settings,
    key: &str,
    top_left: [f32; 2],
    drag_delta: [f32; 2],
) {
    settings.desktop_icon_custom_positions.insert(
        key.to_string(),
        [top_left[0] + drag_delta[0], top_left[1] + drag_delta[1]],
    );
}

pub fn finalize_dragged_icon_position(
    settings: &mut Settings,
    key: &str,
    grid: DesktopIconDragGrid,
) -> bool {
    let Some([x, y]) = settings.desktop_icon_custom_positions.get(key).copied() else {
        return false;
    };
    let [x, y] = if grid.snap_to_grid {
        [
            (x / grid.cell_w).round() * grid.cell_w,
            (y / grid.cell_h).round() * grid.cell_h,
        ]
    } else {
        [x, y]
    };
    settings
        .desktop_icon_custom_positions
        .insert(key.to_string(), [x, y]);
    true
}

fn shortcut_type_rank(shortcut_kind: &str) -> usize {
    match shortcut_kind {
        "network" => 22,
        "game" => 23,
        "nuke_codes" => 24,
        "editor" => 25,
        _ => 21,
    }
}

fn desktop_surface_kind_for_path(path: &Path) -> DesktopSurfaceItemKind {
    if path.is_dir() {
        return DesktopSurfaceItemKind::Folder;
    }
    let extension = path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    match extension.as_str() {
        "txt" | "md" | "log" | "toml" | "yaml" | "yml" | "json" | "cfg" | "ini" | "conf"
        | "ron" | "rs" | "py" | "js" | "ts" | "c" | "cpp" | "h" | "hpp" | "sh" | "bash"
        | "fish" | "lua" | "rb" => DesktopSurfaceItemKind::TextFile,
        "png" | "jpg" | "jpeg" | "gif" | "bmp" | "webp" | "svg" | "ico" => {
            DesktopSurfaceItemKind::ImageFile
        }
        "mp3" | "wav" | "ogg" | "flac" | "aac" | "m4a" => DesktopSurfaceItemKind::AudioFile,
        "mp4" | "mkv" | "avi" | "mov" | "webm" => DesktopSurfaceItemKind::VideoFile,
        "zip" | "tar" | "gz" | "bz2" | "xz" | "7z" | "rar" => DesktopSurfaceItemKind::ArchiveFile,
        "exe" | "bin" | "appimage" | "dmg" | "deb" | "rpm" | "app" | "bat" | "cmd" => {
            DesktopSurfaceItemKind::AppFile
        }
        _ => DesktopSurfaceItemKind::File,
    }
}

fn desktop_surface_kind_rank(kind: DesktopSurfaceItemKind) -> usize {
    match kind {
        DesktopSurfaceItemKind::Folder => 1,
        DesktopSurfaceItemKind::TextFile => 2,
        DesktopSurfaceItemKind::ImageFile => 3,
        DesktopSurfaceItemKind::AudioFile => 4,
        DesktopSurfaceItemKind::VideoFile => 5,
        DesktopSurfaceItemKind::ArchiveFile => 6,
        DesktopSurfaceItemKind::AppFile => 7,
        DesktopSurfaceItemKind::File => 8,
    }
}

pub fn load_desktop_surface_entries(dir: &Path) -> Vec<DesktopSurfaceEntry> {
    let Ok(read_dir) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut entries = Vec::new();
    for entry in read_dir.flatten() {
        let path = entry.path();
        let label = entry.file_name().to_string_lossy().to_string();
        if label.starts_with('.') {
            continue;
        }
        entries.push(DesktopSurfaceEntry {
            key: format!("desktop_item:{label}"),
            path: path.clone(),
            label,
            kind: desktop_surface_kind_for_path(&path),
        });
    }
    entries.sort_by_key(|entry| entry.label.to_ascii_lowercase());
    entries
}

pub fn build_default_desktop_icon_positions(
    layout: DesktopIconGridLayout,
    sort_mode: DesktopIconSortMode,
    hidden_builtin_icons: &BTreeSet<String>,
    desktop_entries: &[DesktopSurfaceEntry],
    shortcuts: &[DesktopShortcut],
) -> HashMap<String, [f32; 2]> {
    if matches!(sort_mode, DesktopIconSortMode::Custom) {
        return HashMap::new();
    }

    let rows_per_column = (((layout.height - 16.0) / layout.item_height).floor() as usize).max(1);
    let mut ordered_icons: Vec<(String, String, usize, usize)> = Vec::new();

    for (idx, entry) in desktop_builtin_icons().iter().enumerate() {
        if !hidden_builtin_icons.contains(entry.key) {
            ordered_icons.push((entry.key.to_string(), entry.label.to_string(), 0, idx));
        }
    }

    let mut fallback = ordered_icons.len();
    for entry in desktop_entries {
        ordered_icons.push((
            entry.key.clone(),
            entry.label.clone(),
            desktop_surface_kind_rank(entry.kind),
            fallback,
        ));
        fallback += 1;
    }

    for (idx, shortcut) in shortcuts.iter().enumerate() {
        ordered_icons.push((
            format!("shortcut_{idx}"),
            shortcut.label.clone(),
            shortcut_type_rank(&shortcut.shortcut_kind),
            fallback + idx,
        ));
    }

    match sort_mode {
        DesktopIconSortMode::ByName => {
            ordered_icons
                .sort_by_key(|(_, label, _, fallback)| (label.to_ascii_lowercase(), *fallback));
        }
        DesktopIconSortMode::ByType => {
            ordered_icons.sort_by_key(|(_, label, type_rank, fallback)| {
                (*type_rank, label.to_ascii_lowercase(), *fallback)
            });
        }
        DesktopIconSortMode::Custom => {}
    }

    let mut positions = HashMap::new();
    for (idx, (key, _, _, _)) in ordered_icons.into_iter().enumerate() {
        let col = idx / rows_per_column;
        let row = idx % rows_per_column;
        positions.insert(
            key,
            [
                layout.left + 4.0 + col as f32 * layout.column_width,
                layout.top + 16.0 + row as f32 * layout.item_height,
            ],
        );
    }
    positions
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::DesktopIconSortMode;
    use std::fs;
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

    fn shortcut(label: &str, kind: &str) -> DesktopShortcut {
        DesktopShortcut {
            label: label.to_string(),
            app_name: label.to_string(),
            pos_x: None,
            pos_y: None,
            launch_command: None,
            icon_path: None,
            shortcut_kind: kind.to_string(),
        }
    }

    #[test]
    fn custom_sort_mode_returns_no_default_positions() {
        let positions = build_default_desktop_icon_positions(
            DesktopIconGridLayout {
                left: 0.0,
                top: 0.0,
                height: 400.0,
                item_height: 80.0,
                column_width: 100.0,
            },
            DesktopIconSortMode::Custom,
            &BTreeSet::new(),
            &[],
            &[],
        );

        assert!(positions.is_empty());
    }

    #[test]
    fn by_name_positions_sort_shortcuts_alphabetically() {
        let positions = build_default_desktop_icon_positions(
            DesktopIconGridLayout {
                left: 0.0,
                top: 0.0,
                height: 400.0,
                item_height: 80.0,
                column_width: 100.0,
            },
            DesktopIconSortMode::ByName,
            &BTreeSet::new(),
            &[],
            &[shortcut("Zulu", "app"), shortcut("Alpha", "app")],
        );

        let alpha = positions.get("shortcut_1").expect("alpha position");
        let zulu = positions.get("shortcut_0").expect("zulu position");
        assert!(alpha[1] <= zulu[1] || alpha[0] < zulu[0]);
    }

    #[test]
    fn desktop_surface_entries_skip_hidden_files_and_classify_folders() {
        let dir = unique_temp_dir("desktop-surface");
        fs::create_dir_all(dir.join("Projects")).expect("create folder");
        fs::write(dir.join("notes.txt"), "hello").expect("write file");
        fs::write(dir.join(".hidden"), "secret").expect("write hidden file");

        let entries = load_desktop_surface_entries(&dir);

        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].label, "notes.txt");
        assert_eq!(entries[0].kind, DesktopSurfaceItemKind::TextFile);
        assert_eq!(entries[1].label, "Projects");
        assert_eq!(entries[1].kind, DesktopSurfaceItemKind::Folder);

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn wallpaper_browser_start_dir_prefers_current_wallpaper_parent() {
        let dir = unique_temp_dir("wallpaper-dir");
        let wallpaper = dir.join("wallpaper.png");

        let start = wallpaper_browser_start_dir(&wallpaper.to_string_lossy());

        assert_eq!(start, dir);
        let _ = fs::remove_dir_all(start);
    }

    #[test]
    fn wallpaper_browser_start_dir_falls_back_for_builtin_wallpaper_names() {
        let start = wallpaper_browser_start_dir("RobCo");

        assert!(!start.as_os_str().is_empty());
    }
}
