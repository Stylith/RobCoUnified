use anyhow::{anyhow, Result};
use nucleon_native_services::shared_file_manager_settings::{
    open_with_default_for_extension, open_with_history_for_extension,
};
use nucleon_shared::config::{
    file_manager_trash_dir, get_settings, DesktopFileManagerSettings, FileManagerSortMode,
    FileManagerViewMode,
};
use nucleon_shared::default_apps::parse_custom_command_line;
use nucleon_shared::launcher::command_exists;
use std::cell::RefCell;
use std::cmp::Ordering;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

pub const FILE_MANAGER_OPEN_WITH_NO_EXT_KEY: &str = "__no_ext__";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KnownAppEntry {
    pub label: String,
    pub command: String,
}

/// Returns known app associations for a given extension key.
/// Only includes apps whose command is found on the system.
pub fn known_apps_for_extension(ext_key: &str) -> Vec<KnownAppEntry> {
    let candidates: Vec<KnownAppEntry> = match ext_key {
        "txt" | "md" | "rs" | "toml" | "json" | "yaml" | "yml" => vec![
            KnownAppEntry {
                label: "Helix".into(),
                command: "hx".into(),
            },
            KnownAppEntry {
                label: "Neovim".into(),
                command: "nvim".into(),
            },
            KnownAppEntry {
                label: "Nano".into(),
                command: "nano".into(),
            },
            KnownAppEntry {
                label: "Emacs".into(),
                command: "emacs".into(),
            },
            KnownAppEntry {
                label: "Kakoune".into(),
                command: "kak".into(),
            },
            KnownAppEntry {
                label: "Micro".into(),
                command: "micro".into(),
            },
            KnownAppEntry {
                label: "Orbiton".into(),
                command: "o".into(),
            },
            KnownAppEntry {
                label: "mdv".into(),
                command: "mdv".into(),
            },
        ],
        "png" | "jpg" | "jpeg" | "gif" | "svg" | "webp" => vec![
            KnownAppEntry {
                label: "feh".into(),
                command: "feh".into(),
            },
            KnownAppEntry {
                label: "Preview".into(),
                command: "open -a Preview".into(),
            },
        ],
        "mp3" | "wav" | "flac" | "ogg" => vec![
            KnownAppEntry {
                label: "mpv".into(),
                command: "mpv".into(),
            },
            KnownAppEntry {
                label: "cmus".into(),
                command: "cmus".into(),
            },
            KnownAppEntry {
                label: "mpd".into(),
                command: "mpd".into(),
            },
            KnownAppEntry {
                label: "ncmpcpp".into(),
                command: "ncmpcpp".into(),
            },
            KnownAppEntry {
                label: "moc".into(),
                command: "moc".into(),
            },
            KnownAppEntry {
                label: "Musikcube".into(),
                command: "musikcube".into(),
            },
            KnownAppEntry {
                label: "mpvc".into(),
                command: "mpvc-tui".into(),
            },
        ],
        "mp4" | "mkv" | "mov" | "webm" => vec![
            KnownAppEntry {
                label: "mpv".into(),
                command: "mpv".into(),
            },
            KnownAppEntry {
                label: "VLC".into(),
                command: "vlc".into(),
            },
        ],
        "epub" | "mobi" | "azw" | "azw3" | "iba" | "rtf" => vec![
            KnownAppEntry {
                label: "epr".into(),
                command: "epr".into(),
            },
            KnownAppEntry {
                label: "epy".into(),
                command: "epy".into(),
            },
            KnownAppEntry {
                label: "hygg".into(),
                command: "hygg".into(),
            },
        ],
        "pdf" => vec![
            KnownAppEntry {
                label: "hygg".into(),
                command: "hygg".into(),
            },
            KnownAppEntry {
                label: "less".into(),
                command: "less".into(),
            },
            KnownAppEntry {
                label: "xdg-open".into(),
                command: "xdg-open".into(),
            },
            KnownAppEntry {
                label: "xreader".into(),
                command: "xreader".into(),
            },
            KnownAppEntry {
                label: "Ghostview".into(),
                command: "gv".into(),
            },
        ],
        _ => Vec::new(),
    };
    candidates
        .into_iter()
        .filter(|entry| {
            parse_custom_command_line(&entry.command)
                .and_then(|argv| argv.first().cloned())
                .is_some_and(|program| command_exists(&program))
        })
        .collect()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileEntryRow {
    pub path: PathBuf,
    pub label: String,
    pub is_dir: bool,
}

impl FileEntryRow {
    pub fn is_parent_dir(&self) -> bool {
        self.label == ".."
    }

    pub fn icon(&self) -> &'static str {
        if self.is_parent_dir() {
            return "[UP]";
        }
        if self.is_dir {
            return "[DIR]";
        }
        let ext = Path::new(&self.label)
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        match ext.as_str() {
            "txt" | "md" | "rs" | "json" | "toml" | "yaml" | "yml" => "[TXT]",
            "png" | "jpg" | "jpeg" | "gif" | "bmp" | "svg" | "webp" => "[IMG]",
            "zip" | "tar" | "gz" | "bz2" | "xz" | "7z" => "[ARC]",
            "mp3" | "wav" | "flac" | "ogg" => "[AUD]",
            "mp4" | "mkv" | "mov" | "webm" => "[VID]",
            "sh" | "exe" | "app" | "bat" | "cmd" => "[APP]",
            _ => "[FILE]",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileTreeItem {
    pub line: String,
    pub path: Option<PathBuf>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct FileManagerRowsCacheKey {
    show_hidden_files: bool,
    directories_first: bool,
    sort_mode: FileManagerSortMode,
}

#[derive(Debug, Clone)]
struct FileManagerRowsCache {
    cwd: PathBuf,
    query: String,
    key: FileManagerRowsCacheKey,
    rows: Vec<FileEntryRow>,
}

#[derive(Debug, Clone)]
struct FileManagerTreeCache {
    cwd: PathBuf,
    show_hidden_files: bool,
    items: Vec<FileTreeItem>,
}

#[derive(Debug, Clone, Default)]
struct FileManagerViewCache {
    rows: Option<FileManagerRowsCache>,
    tree: Option<FileManagerTreeCache>,
}

#[derive(Debug, Clone)]
pub enum FileManagerAction {
    None,
    ChangedDir,
    OpenFile(PathBuf),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileManagerCommand {
    OpenSelected,
    ClearSearch,
    NewFolder,
    NewTab,
    PreviousTab,
    NextTab,
    CloseTab,
    OpenHome,
    GoUp,
    Copy,
    Cut,
    Paste,
    Duplicate,
    Rename,
    Move,
    Delete,
    Undo,
    Redo,
    ToggleTreePanel,
    ToggleHiddenFiles,
    SetViewMode(FileManagerViewMode),
    SetSortMode(FileManagerSortMode),
}

#[derive(Debug, Clone)]
pub struct NativeFileManagerState {
    pub open: bool,
    pub cwd: PathBuf,
    pub selected: Option<PathBuf>,
    pub selected_paths: HashSet<PathBuf>,
    pub tabs: Vec<PathBuf>,
    pub active_tab: usize,
    pub tree_selected: Option<PathBuf>,
    pub search_query: String,
    view_cache: RefCell<FileManagerViewCache>,
}

impl NativeFileManagerState {
    pub fn new(cwd: PathBuf) -> Self {
        Self {
            open: false,
            cwd: cwd.clone(),
            selected: None,
            selected_paths: HashSet::new(),
            tabs: vec![cwd.clone()],
            active_tab: 0,
            tree_selected: Some(cwd),
            search_query: String::new(),
            view_cache: RefCell::new(FileManagerViewCache::default()),
        }
    }

    fn invalidate_view_cache(&mut self) {
        *self.view_cache.get_mut() = FileManagerViewCache::default();
    }

    pub fn refresh_contents(&mut self) {
        self.invalidate_view_cache();
        self.ensure_selection_valid();
    }

    fn sync_active_tab_path(&mut self) {
        if self.tabs.is_empty() {
            self.tabs.push(self.cwd.clone());
            self.active_tab = 0;
            return;
        }
        self.active_tab = self.active_tab.min(self.tabs.len().saturating_sub(1));
        self.tabs[self.active_tab] = self.cwd.clone();
    }

    pub fn up(&mut self) {
        if let Some(parent) = self.cwd.parent() {
            self.set_cwd(parent.to_path_buf());
        }
    }

    pub fn set_cwd(&mut self, path: PathBuf) {
        self.cwd = path.clone();
        self.invalidate_view_cache();
        self.selected = None;
        self.selected_paths.clear();
        self.tree_selected = Some(path);
        self.sync_active_tab_path();
        self.ensure_selection_valid();
    }

    pub fn select(&mut self, path: Option<PathBuf>) {
        self.selected = path;
        self.selected_paths.clear();
        self.ensure_selection_valid();
    }

    pub fn clear_multi_selection(&mut self) {
        self.selected_paths.clear();
    }

    pub fn toggle_selected_path(&mut self, path: &Path) {
        let Some(row) = self.rows().into_iter().find(|row| row.path == path) else {
            return;
        };
        self.selected = Some(row.path.clone());
        if row.is_parent_dir() {
            return;
        }
        if !self.selected_paths.insert(row.path.clone()) {
            self.selected_paths.remove(&row.path);
        }
    }

    pub fn is_path_selected(&self, path: &Path) -> bool {
        self.selected.as_deref() == Some(path) || self.selected_paths.contains(path)
    }

    pub fn selected_rows_for_action(&self) -> Vec<FileEntryRow> {
        let rows = self.rows();
        let mut selected_rows: Vec<FileEntryRow> = rows
            .iter()
            .filter(|row| !row.is_parent_dir() && self.selected_paths.contains(&row.path))
            .cloned()
            .collect();
        if selected_rows.is_empty() {
            if let Some(row) = rows
                .into_iter()
                .find(|row| self.selected.as_ref() == Some(&row.path))
            {
                if !row.is_parent_dir() {
                    selected_rows.push(row);
                }
            }
        }
        selected_rows
    }

    pub fn update_search_query(&mut self, query: String) {
        self.search_query = query;
        self.ensure_selection_valid();
    }

    pub fn clear_search(&mut self) {
        self.update_search_query(String::new());
    }

    pub fn rows(&self) -> Vec<FileEntryRow> {
        let settings = get_settings().desktop_file_manager;
        let q = self.search_query.trim().to_ascii_lowercase();
        let key = FileManagerRowsCacheKey {
            show_hidden_files: settings.show_hidden_files,
            directories_first: settings.directories_first,
            sort_mode: settings.sort_mode,
        };
        if let Some(cache) = self.view_cache.borrow().rows.as_ref() {
            if cache.cwd == self.cwd && cache.query == q && cache.key == key {
                return cache.rows.clone();
            }
        }
        let rows = Self::read_rows(&self.cwd, &settings);
        if q.is_empty() {
            self.view_cache.borrow_mut().rows = Some(FileManagerRowsCache {
                cwd: self.cwd.clone(),
                query: q,
                key,
                rows: rows.clone(),
            });
            rows
        } else {
            let filtered: Vec<FileEntryRow> = rows
                .into_iter()
                .filter(|row| row.label.to_ascii_lowercase().contains(&q))
                .collect();
            self.view_cache.borrow_mut().rows = Some(FileManagerRowsCache {
                cwd: self.cwd.clone(),
                query: q,
                key,
                rows: filtered.clone(),
            });
            filtered
        }
    }

    pub fn tree_items(&self) -> Vec<FileTreeItem> {
        let show_hidden = get_settings().desktop_file_manager.show_hidden_files;
        if let Some(cache) = self.view_cache.borrow().tree.as_ref() {
            if cache.cwd == self.cwd && cache.show_hidden_files == show_hidden {
                return cache.items.clone();
            }
        }
        let mut items = vec![FileTreeItem {
            line: "Drives".to_string(),
            path: None,
        }];
        let drives = Self::drive_roots();
        for drive in &drives {
            items.push(FileTreeItem {
                line: format!("* {}", Self::drive_label(drive)),
                path: Some(drive.clone()),
            });
        }
        items.push(FileTreeItem {
            line: "Folders".to_string(),
            path: None,
        });
        let current_drive = Self::current_drive_root_for_path(&self.cwd, &drives)
            .unwrap_or_else(|| PathBuf::from("/"));
        let rel = self.cwd.strip_prefix(&current_drive).unwrap_or(&self.cwd);
        let comps: Vec<String> = rel
            .components()
            .filter_map(|c| {
                let s = c.as_os_str().to_string_lossy().to_string();
                if s.is_empty() || s == "/" {
                    None
                } else {
                    Some(s)
                }
            })
            .collect();

        let mut running = current_drive.clone();
        for (depth, comp) in comps.iter().enumerate() {
            running = running.join(comp);
            items.push(FileTreeItem {
                line: format!("{}|- {}", "  ".repeat(depth + 1), comp),
                path: Some(running.clone()),
            });
        }

        let mut child_dirs: Vec<String> = std::fs::read_dir(&self.cwd)
            .ok()
            .into_iter()
            .flat_map(|iter| iter.flatten())
            .filter_map(|entry| {
                let path = entry.path();
                if !path.is_dir() {
                    return None;
                }
                let name = entry.file_name().to_string_lossy().to_string();
                if !show_hidden && name.starts_with('.') {
                    return None;
                }
                Some(name)
            })
            .collect();
        child_dirs.sort_by_key(|name| name.to_lowercase());
        let child_indent = "  ".repeat(comps.len() + 1);
        for name in child_dirs {
            items.push(FileTreeItem {
                line: format!("{child_indent}+- {name}"),
                path: Some(self.cwd.join(&name)),
            });
        }

        self.view_cache.borrow_mut().tree = Some(FileManagerTreeCache {
            cwd: self.cwd.clone(),
            show_hidden_files: show_hidden,
            items: items.clone(),
        });

        items
    }

    pub fn open_selected_tree_path(&mut self, path: PathBuf) {
        self.tree_selected = Some(path.clone());
        self.set_cwd(path);
    }

    pub fn open_tab_here(&mut self) {
        self.tabs.push(self.cwd.clone());
        self.active_tab = self.tabs.len().saturating_sub(1);
    }

    pub fn close_tab(&mut self, idx: usize) -> bool {
        if self.tabs.len() <= 1 || idx >= self.tabs.len() {
            return false;
        }
        self.tabs.remove(idx);
        if idx < self.active_tab {
            self.active_tab = self.active_tab.saturating_sub(1);
        } else if self.active_tab >= self.tabs.len() {
            self.active_tab = self.tabs.len().saturating_sub(1);
        }
        self.cwd = self.tabs[self.active_tab].clone();
        self.invalidate_view_cache();
        self.selected = None;
        self.selected_paths.clear();
        self.tree_selected = Some(self.cwd.clone());
        true
    }

    pub fn close_active_tab(&mut self) -> bool {
        self.close_tab(self.active_tab)
    }

    pub fn switch_to_tab(&mut self, idx: usize) -> bool {
        if idx >= self.tabs.len() {
            return false;
        }
        self.active_tab = idx;
        self.cwd = self.tabs[idx].clone();
        self.invalidate_view_cache();
        self.selected = None;
        self.selected_paths.clear();
        self.tree_selected = Some(self.cwd.clone());
        true
    }

    pub fn switch_to_previous_tab(&mut self) -> bool {
        if self.tabs.len() <= 1 {
            return false;
        }
        let idx = if self.active_tab == 0 {
            self.tabs.len().saturating_sub(1)
        } else {
            self.active_tab - 1
        };
        self.switch_to_tab(idx)
    }

    pub fn switch_to_next_tab(&mut self) -> bool {
        if self.tabs.len() <= 1 {
            return false;
        }
        let idx = (self.active_tab + 1) % self.tabs.len();
        self.switch_to_tab(idx)
    }

    pub fn tab_title(path: &Path) -> String {
        if path == Path::new("/") {
            return "/".to_string();
        }
        path.file_name()
            .and_then(|name| name.to_str())
            .map(|name| name.to_string())
            .unwrap_or_else(|| path.display().to_string())
    }

    pub fn selected_row(&self) -> Option<FileEntryRow> {
        let selected = self.selected.as_ref()?;
        self.rows().into_iter().find(|row| &row.path == selected)
    }

    pub fn ensure_selection_valid(&mut self) {
        let rows = self.rows();
        self.selected_paths.retain(|selected| {
            rows.iter()
                .any(|row| &row.path == selected && !row.is_parent_dir())
        });
        if rows.is_empty() {
            self.selected = None;
            return;
        }
        if self
            .selected
            .as_ref()
            .is_some_and(|selected| rows.iter().any(|row| &row.path == selected))
        {
            return;
        }
        self.selected = rows.first().map(|row| row.path.clone());
    }

    pub fn activate_selected(&mut self) -> FileManagerAction {
        let Some(path) = self.selected.clone() else {
            return FileManagerAction::None;
        };
        if path.is_dir() {
            self.set_cwd(path);
            FileManagerAction::ChangedDir
        } else {
            FileManagerAction::OpenFile(path)
        }
    }

    fn read_rows(path: &Path, settings: &DesktopFileManagerSettings) -> Vec<FileEntryRow> {
        let mut rows = Vec::new();
        if let Some(parent) = path.parent() {
            rows.push(FileEntryRow {
                path: parent.to_path_buf(),
                label: "..".to_string(),
                is_dir: true,
            });
        }
        let read_dir = match std::fs::read_dir(path) {
            Ok(rd) => rd,
            Err(_) => return rows,
        };
        for entry in read_dir.flatten() {
            let entry_path = entry.path();
            let label = entry.file_name().to_string_lossy().to_string();
            if !settings.show_hidden_files && label.starts_with('.') {
                continue;
            }
            rows.push(FileEntryRow {
                path: entry_path.clone(),
                label,
                is_dir: entry_path.is_dir(),
            });
        }

        rows.sort_by(|a, b| {
            if a.is_parent_dir() {
                return Ordering::Less;
            }
            if b.is_parent_dir() {
                return Ordering::Greater;
            }
            if settings.directories_first {
                match (a.is_dir, b.is_dir) {
                    (true, false) => return Ordering::Less,
                    (false, true) => return Ordering::Greater,
                    _ => {}
                }
            }
            match settings.sort_mode {
                FileManagerSortMode::Name => a.label.to_lowercase().cmp(&b.label.to_lowercase()),
                FileManagerSortMode::Type => {
                    let a_ext = Path::new(&a.label)
                        .extension()
                        .and_then(|s| s.to_str())
                        .unwrap_or("")
                        .to_ascii_lowercase();
                    let b_ext = Path::new(&b.label)
                        .extension()
                        .and_then(|s| s.to_str())
                        .unwrap_or("")
                        .to_ascii_lowercase();
                    a_ext
                        .cmp(&b_ext)
                        .then_with(|| a.label.to_lowercase().cmp(&b.label.to_lowercase()))
                }
            }
        });

        rows
    }

    pub fn drive_roots() -> Vec<PathBuf> {
        #[cfg(target_os = "windows")]
        {
            let mut drives = Vec::new();
            for letter in b'A'..=b'Z' {
                let path = PathBuf::from(format!("{}:\\", letter as char));
                if path.exists() {
                    drives.push(path);
                }
            }
            return drives;
        }

        #[cfg(not(target_os = "windows"))]
        {
            let mut drives = vec![PathBuf::from("/")];
            for mount_root in ["/Volumes", "/mnt", "/media"] {
                let mount_root = Path::new(mount_root);
                if let Ok(entries) = std::fs::read_dir(mount_root) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if path.is_dir() && !drives.iter().any(|existing| existing == &path) {
                            drives.push(path);
                        }
                    }
                }
            }
            drives.sort_by(|a, b| {
                let a_root = a == Path::new("/");
                let b_root = b == Path::new("/");
                match (a_root, b_root) {
                    (true, false) => Ordering::Less,
                    (false, true) => Ordering::Greater,
                    _ => a
                        .display()
                        .to_string()
                        .to_lowercase()
                        .cmp(&b.display().to_string().to_lowercase()),
                }
            });
            drives
        }
    }

    pub fn current_drive_root(&self) -> Option<PathBuf> {
        Self::current_drive_root_for_path(&self.cwd, &Self::drive_roots())
    }

    fn current_drive_root_for_path(path: &Path, drives: &[PathBuf]) -> Option<PathBuf> {
        drives
            .iter()
            .filter(|drive| path.starts_with(drive))
            .max_by_key(|drive| drive.components().count())
            .cloned()
    }

    pub fn drive_label(path: &Path) -> String {
        #[cfg(target_os = "windows")]
        {
            return path.display().to_string();
        }

        #[cfg(not(target_os = "windows"))]
        {
            if path == Path::new("/") {
                return "/".to_string();
            }
            path.file_name()
                .and_then(|name| name.to_str())
                .filter(|name| !name.is_empty())
                .map(|name| name.to_string())
                .unwrap_or_else(|| path.display().to_string())
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileManagerClipboardMode {
    Copy,
    Cut,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileManagerClipboardItem {
    pub paths: Vec<PathBuf>,
    pub mode: FileManagerClipboardMode,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeFileManagerDragPayload {
    pub paths: Vec<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileManagerEditOp {
    CopyCreated { src: PathBuf, dst: PathBuf },
    Moved { from: PathBuf, to: PathBuf },
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FileManagerEditRuntime {
    pub clipboard: Option<FileManagerClipboardItem>,
    pub undo_stack: Vec<FileManagerEditOp>,
    pub redo_stack: Vec<FileManagerEditOp>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenWithLaunchRequest {
    pub argv: Vec<String>,
    pub title: String,
    pub status_message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileManagerOpenWithState {
    pub ext_key: String,
    pub ext_label: String,
    pub saved_commands: Vec<String>,
    pub current_default: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileManagerPickMode {
    None,
    SaveAs,
    ShortcutIcon(usize),
    Wallpaper,
    ThemeImport,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileManagerSelectionActivation {
    ActivateSelection,
    FillSaveAsName(String),
    PickShortcutIcon { shortcut_idx: usize, path: PathBuf },
    PickWallpaper(PathBuf),
    PickThemeImport(PathBuf),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileManagerOpenTarget {
    NoOp,
    Launch(OpenWithLaunchRequest),
    OpenInEditor(PathBuf),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileManagerPickerCommit {
    SetShortcutIcon { shortcut_idx: usize, path: PathBuf },
    SetWallpaper(PathBuf),
    ImportTheme(PathBuf),
}

pub fn open_with_extension_key(path: &Path) -> String {
    path.extension()
        .and_then(|s| s.to_str())
        .map(|s| s.trim().to_ascii_lowercase())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| FILE_MANAGER_OPEN_WITH_NO_EXT_KEY.to_string())
}

pub fn open_with_extension_label(ext_key: &str) -> String {
    if ext_key == FILE_MANAGER_OPEN_WITH_NO_EXT_KEY {
        "(no extension)".to_string()
    } else {
        format!(".{ext_key}")
    }
}

pub fn open_with_command_title(program: &str) -> String {
    let name = Path::new(program)
        .file_name()
        .and_then(|s| s.to_str())
        .filter(|s| !s.is_empty())
        .unwrap_or(program);
    if name.eq_ignore_ascii_case("spotify_player") {
        "spotify".to_string()
    } else {
        name.to_string()
    }
}

pub fn open_with_state_for_path(
    path: &Path,
    fm: &DesktopFileManagerSettings,
) -> FileManagerOpenWithState {
    let ext_key = open_with_extension_key(path);
    FileManagerOpenWithState {
        ext_label: open_with_extension_label(&ext_key),
        saved_commands: open_with_history_for_extension(fm, &ext_key),
        current_default: open_with_default_for_extension(fm, &ext_key),
        ext_key,
    }
}

pub fn prepare_open_with_launch(path: &Path, command_line: &str) -> Result<OpenWithLaunchRequest> {
    let normalized = command_line.trim();
    let Some(mut argv) = parse_custom_command_line(normalized) else {
        return Err(anyhow!("Invalid command line: {normalized}"));
    };
    let program = argv.first().cloned().unwrap_or_default();
    if !program.is_empty() && !command_exists(&program) {
        return Err(anyhow!("Command `{program}` was not found in PATH."));
    }
    argv.push(path.display().to_string());
    Ok(OpenWithLaunchRequest {
        title: format!(
            "{} - {}",
            open_with_command_title(&argv[0]),
            path_display_name(path)
        ),
        status_message: format!("Opened {} in PTY", path_display_name(path)),
        argv,
    })
}

pub fn default_open_with_launch_for_path(
    path: &Path,
    fm: &DesktopFileManagerSettings,
) -> Option<Result<OpenWithLaunchRequest>> {
    let state = open_with_state_for_path(path, fm);
    state
        .current_default
        .as_deref()
        .map(|command| prepare_open_with_launch(path, command))
}

pub fn selection_activation_for_selected_path(
    selected_path: Option<PathBuf>,
    pick_mode: FileManagerPickMode,
) -> FileManagerSelectionActivation {
    let Some(selected_path) = selected_path else {
        return FileManagerSelectionActivation::ActivateSelection;
    };
    if !selected_path.is_file() {
        return FileManagerSelectionActivation::ActivateSelection;
    }
    match pick_mode {
        FileManagerPickMode::None => FileManagerSelectionActivation::ActivateSelection,
        FileManagerPickMode::SaveAs => selected_path
            .file_name()
            .and_then(|name| name.to_str())
            .map(|name| FileManagerSelectionActivation::FillSaveAsName(name.to_string()))
            .unwrap_or(FileManagerSelectionActivation::ActivateSelection),
        FileManagerPickMode::ShortcutIcon(shortcut_idx) => {
            FileManagerSelectionActivation::PickShortcutIcon {
                shortcut_idx,
                path: selected_path,
            }
        }
        FileManagerPickMode::Wallpaper => {
            FileManagerSelectionActivation::PickWallpaper(selected_path)
        }
        FileManagerPickMode::ThemeImport => {
            FileManagerSelectionActivation::PickThemeImport(selected_path)
        }
    }
}

pub fn open_target_for_file_manager_action(
    action: FileManagerAction,
    fm: &DesktopFileManagerSettings,
) -> Result<FileManagerOpenTarget, String> {
    match action {
        FileManagerAction::None | FileManagerAction::ChangedDir => Ok(FileManagerOpenTarget::NoOp),
        FileManagerAction::OpenFile(path) => {
            if let Some(result) = default_open_with_launch_for_path(&path, fm) {
                result
                    .map(FileManagerOpenTarget::Launch)
                    .map_err(|err| format!("Open failed: {err}"))
            } else {
                Ok(FileManagerOpenTarget::OpenInEditor(path))
            }
        }
    }
}

pub fn commit_picker_selection(
    selected_file: Option<FileEntryRow>,
    pick_mode: FileManagerPickMode,
) -> Result<FileManagerPickerCommit, String> {
    let selected_file = match pick_mode {
        FileManagerPickMode::ShortcutIcon(_) => {
            selected_file.ok_or_else(|| "Select an SVG file first.".to_string())?
        }
        FileManagerPickMode::Wallpaper => {
            selected_file.ok_or_else(|| "Select an image file first.".to_string())?
        }
        FileManagerPickMode::ThemeImport => {
            selected_file.ok_or_else(|| "Select a theme manifest or .ndpkg file first.".to_string())?
        }
        FileManagerPickMode::None | FileManagerPickMode::SaveAs => {
            return Err("No picker action is active.".to_string());
        }
    };

    match pick_mode {
        FileManagerPickMode::ShortcutIcon(shortcut_idx) => {
            Ok(FileManagerPickerCommit::SetShortcutIcon {
                shortcut_idx,
                path: selected_file.path,
            })
        }
        FileManagerPickMode::Wallpaper => {
            Ok(FileManagerPickerCommit::SetWallpaper(selected_file.path))
        }
        FileManagerPickMode::ThemeImport => {
            Ok(FileManagerPickerCommit::ImportTheme(selected_file.path))
        }
        FileManagerPickMode::None | FileManagerPickMode::SaveAs => {
            Err("No picker action is active.".to_string())
        }
    }
}

pub fn selected_file(mut entries: Vec<FileEntryRow>) -> Option<FileEntryRow> {
    if entries.len() != 1 {
        return None;
    }
    let entry = entries.pop()?;
    entry.path.is_file().then_some(entry)
}

impl FileManagerEditRuntime {
    pub fn has_clipboard(&self) -> bool {
        self.clipboard
            .as_ref()
            .is_some_and(|clipboard| !clipboard.paths.is_empty())
    }

    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    pub fn set_clipboard_from_entries(
        &mut self,
        entries: &[FileEntryRow],
        mode: FileManagerClipboardMode,
    ) -> Result<String> {
        if entries.is_empty() {
            return Err(anyhow!("Select a file or folder first."));
        }
        self.clipboard = Some(FileManagerClipboardItem {
            paths: entries.iter().map(|entry| entry.path.clone()).collect(),
            mode: mode.clone(),
        });
        let noun = if entries.len() == 1 {
            entries[0].label.clone()
        } else {
            format!("{} items", entries.len())
        };
        Ok(match mode {
            FileManagerClipboardMode::Copy => format!("Copied {noun}"),
            FileManagerClipboardMode::Cut => format!("Cut {noun}"),
        })
    }

    pub fn create_new_folder(
        &mut self,
        file_manager: &mut NativeFileManagerState,
    ) -> Result<String> {
        let dst = self.create_folder_in_dir(&file_manager.cwd, "New Folder")?;
        file_manager.invalidate_view_cache();
        file_manager.select(Some(dst.clone()));
        Ok(format!("Created {}", path_display_name(&dst)))
    }

    pub fn create_folder_in_dir(&mut self, target_dir: &Path, name: &str) -> Result<PathBuf> {
        let dst = unique_path_in_dir(target_dir, name);
        std::fs::create_dir_all(&dst)
            .map_err(|e| anyhow!("Failed creating {}: {e}", dst.display()))?;
        Ok(dst)
    }

    pub fn duplicate_selected(
        &mut self,
        file_manager: &mut NativeFileManagerState,
        entries: Vec<FileEntryRow>,
    ) -> Result<String> {
        if entries.is_empty() {
            return Err(anyhow!("Select a file or folder first."));
        }
        let mut created = Vec::new();
        for entry in entries {
            let Some(parent) = entry.path.parent() else {
                continue;
            };
            let name = path_display_name(&entry.path);
            let dst = unique_copy_path_in_dir(parent, &name, true);
            copy_path_recursive(&entry.path, &dst)?;
            self.record_edit_op(FileManagerEditOp::CopyCreated {
                src: entry.path,
                dst: dst.clone(),
            });
            created.push(dst);
        }
        let Some(last) = created.last().cloned() else {
            return Err(anyhow!("Cannot duplicate this selection."));
        };
        file_manager.invalidate_view_cache();
        file_manager.select(Some(last.clone()));
        if created.len() == 1 {
            Ok(format!("Duplicated as {}", path_display_name(&last)))
        } else {
            Ok(format!("Duplicated {} items", created.len()))
        }
    }

    pub fn rename_selected(
        &mut self,
        file_manager: &mut NativeFileManagerState,
        entry: FileEntryRow,
        new_name: String,
    ) -> Result<String> {
        let Some(parent) = entry.path.parent() else {
            return Err(anyhow!("Cannot rename this item."));
        };
        let name = new_name.trim();
        if name.is_empty() {
            return Err(anyhow!("Name cannot be empty."));
        }
        if name.contains('/') || name.contains('\\') {
            return Err(anyhow!("Name cannot contain path separators."));
        }
        if name == entry.label {
            return Ok("Name unchanged.".to_string());
        }
        let dst = parent.join(name);
        if dst.exists() {
            return Err(anyhow!("Destination already exists: {}", dst.display()));
        }
        move_path(&entry.path, &dst)?;
        self.record_edit_op(FileManagerEditOp::Moved {
            from: entry.path,
            to: dst.clone(),
        });
        file_manager.invalidate_view_cache();
        file_manager.select(Some(dst.clone()));
        Ok(format!("Renamed to {}", path_display_name(&dst)))
    }

    pub fn rename_entry(&mut self, entry: FileEntryRow, new_name: String) -> Result<PathBuf> {
        let Some(parent) = entry.path.parent() else {
            return Err(anyhow!("Cannot rename this item."));
        };
        let name = new_name.trim();
        if name.is_empty() {
            return Err(anyhow!("Name cannot be empty."));
        }
        if name.contains('/') || name.contains('\\') {
            return Err(anyhow!("Name cannot contain path separators."));
        }
        if name == entry.label {
            return Ok(entry.path);
        }
        let dst = parent.join(name);
        if dst.exists() {
            return Err(anyhow!("Destination already exists: {}", dst.display()));
        }
        move_path(&entry.path, &dst)?;
        self.record_edit_op(FileManagerEditOp::Moved {
            from: entry.path,
            to: dst.clone(),
        });
        Ok(dst)
    }

    pub fn move_selected(
        &mut self,
        file_manager: &mut NativeFileManagerState,
        entry: FileEntryRow,
        raw_destination: String,
    ) -> Result<String> {
        let mut dst = PathBuf::from(raw_destination.trim());
        if dst.as_os_str().is_empty() {
            return Err(anyhow!("Destination cannot be empty."));
        }
        if dst.is_relative() {
            dst = file_manager.cwd.join(dst);
        }
        if dst.exists() && dst.is_dir() {
            dst = dst.join(path_display_name(&entry.path));
        }
        if dst == entry.path {
            return Ok("Item already at destination.".to_string());
        }
        move_path(&entry.path, &dst)?;
        self.record_edit_op(FileManagerEditOp::Moved {
            from: entry.path.clone(),
            to: dst.clone(),
        });
        file_manager.invalidate_view_cache();
        if let Some(parent) = dst.parent() {
            file_manager.set_cwd(parent.to_path_buf());
        }
        file_manager.select(Some(dst.clone()));
        Ok(format!("Moved to {}", dst.display()))
    }

    pub fn move_paths_to_dir(
        &mut self,
        file_manager: &mut NativeFileManagerState,
        paths: Vec<PathBuf>,
        target_dir: &Path,
    ) -> Result<String> {
        if !target_dir.is_dir() {
            return Err(anyhow!("Destination folder does not exist."));
        }
        let mut seen = HashSet::new();
        let mut moved = Vec::new();
        let target_dir = target_dir.to_path_buf();
        for src in paths {
            if !seen.insert(src.clone()) || !src.exists() {
                continue;
            }
            if !can_move_path_to_dir(&src, &target_dir) {
                continue;
            }
            let source_name = path_display_name(&src);
            let mut dst = target_dir.join(&source_name);
            if dst.exists() {
                dst = unique_path_in_dir(&target_dir, &source_name);
            }
            if dst == src {
                continue;
            }
            move_path(&src, &dst)?;
            self.record_edit_op(FileManagerEditOp::Moved {
                from: src,
                to: dst.clone(),
            });
            moved.push(dst);
        }
        if moved.is_empty() {
            return Err(anyhow!("Nothing to move."));
        }
        file_manager.invalidate_view_cache();
        if target_dir == file_manager.cwd {
            if let Some(last) = moved.last().cloned() {
                file_manager.select(Some(last));
            }
        } else {
            file_manager.ensure_selection_valid();
        }
        if moved.len() == 1 {
            Ok(format!("Moved {}", path_display_name(&moved[0])))
        } else {
            Ok(format!("Moved {} items", moved.len()))
        }
    }

    pub fn drop_allowed(paths: &[PathBuf], target_dir: &Path) -> bool {
        paths
            .iter()
            .any(|src| src.exists() && can_move_path_to_dir(src, target_dir))
    }

    pub fn paste_clipboard(&mut self, file_manager: &mut NativeFileManagerState) -> Result<String> {
        let target_dir = file_manager.cwd.clone();
        let (changed, last_dst) = self.paste_clipboard_into_dir(&target_dir)?;
        file_manager.invalidate_view_cache();
        if let Some(dst) = last_dst {
            file_manager.select(Some(dst.clone()));
            if changed == 1 {
                Ok(format!("Pasted {}", path_display_name(&dst)))
            } else {
                Ok(format!("Pasted {changed} items"))
            }
        } else {
            Err(anyhow!("Clipboard source no longer exists."))
        }
    }

    pub fn paste_clipboard_into_dir(
        &mut self,
        target_dir: &Path,
    ) -> Result<(usize, Option<PathBuf>)> {
        let Some(clipboard) = self.clipboard.clone() else {
            return Err(anyhow!("Clipboard is empty."));
        };
        let target_dir = target_dir.to_path_buf();
        let mut changed = 0usize;
        let mut last_dst: Option<PathBuf> = None;

        match clipboard.mode {
            FileManagerClipboardMode::Copy => {
                for src in clipboard.paths {
                    if !src.exists() {
                        continue;
                    }
                    let source_name = path_display_name(&src);
                    let mut dst = target_dir.join(&source_name);
                    if dst.exists() {
                        dst = unique_copy_path_in_dir(&target_dir, &source_name, false);
                    }
                    copy_path_recursive(&src, &dst)?;
                    self.record_edit_op(FileManagerEditOp::CopyCreated {
                        src,
                        dst: dst.clone(),
                    });
                    changed += 1;
                    last_dst = Some(dst);
                }
            }
            FileManagerClipboardMode::Cut => {
                for src in clipboard.paths {
                    if !src.exists() {
                        continue;
                    }
                    let source_name = path_display_name(&src);
                    let source_parent = src.parent().map(Path::to_path_buf);
                    if source_parent.as_deref() == Some(target_dir.as_path()) {
                        continue;
                    }
                    let mut dst = target_dir.join(&source_name);
                    if dst.exists() {
                        dst = unique_path_in_dir(&target_dir, &source_name);
                    }
                    move_path(&src, &dst)?;
                    self.record_edit_op(FileManagerEditOp::Moved {
                        from: src,
                        to: dst.clone(),
                    });
                    changed += 1;
                    last_dst = Some(dst);
                }
                self.clipboard = None;
            }
        }

        if changed == 0 {
            Err(anyhow!("Clipboard source no longer exists."))
        } else {
            Ok((changed, last_dst))
        }
    }

    pub fn copy_paths_into_dir(
        &mut self,
        paths: Vec<PathBuf>,
        target_dir: &Path,
    ) -> Result<(usize, Option<PathBuf>)> {
        let mut changed = 0usize;
        let mut last_dst = None;
        for src in paths {
            if !src.exists() {
                continue;
            }
            let source_name = path_display_name(&src);
            let mut dst = target_dir.join(&source_name);
            if dst.exists() {
                dst = unique_copy_path_in_dir(target_dir, &source_name, false);
            }
            copy_path_recursive(&src, &dst)?;
            self.record_edit_op(FileManagerEditOp::CopyCreated {
                src,
                dst: dst.clone(),
            });
            changed += 1;
            last_dst = Some(dst);
        }
        if changed == 0 {
            Err(anyhow!("Nothing to import."))
        } else {
            Ok((changed, last_dst))
        }
    }

    pub fn delete_selected(
        &mut self,
        file_manager: &mut NativeFileManagerState,
        entries: Vec<FileEntryRow>,
    ) -> Result<String> {
        if entries.is_empty() {
            return Err(anyhow!("Select a file or folder first."));
        }
        let trash_dir = file_manager_trash_dir();
        let mut moved = 0usize;
        for entry in entries {
            let name = path_display_name(&entry.path);
            let trash_target = unique_path_in_dir(&trash_dir, &name);
            move_path(&entry.path, &trash_target)?;
            self.record_edit_op(FileManagerEditOp::Moved {
                from: entry.path,
                to: trash_target,
            });
            moved += 1;
        }
        file_manager.invalidate_view_cache();
        file_manager.ensure_selection_valid();
        if moved == 1 {
            Ok("Moved item to trash".to_string())
        } else {
            Ok(format!("Moved {moved} items to trash"))
        }
    }

    pub fn delete_entries(&mut self, entries: Vec<FileEntryRow>) -> Result<usize> {
        if entries.is_empty() {
            return Err(anyhow!("Select a file or folder first."));
        }
        let trash_dir = file_manager_trash_dir();
        let mut moved = 0usize;
        for entry in entries {
            let name = path_display_name(&entry.path);
            let trash_target = unique_path_in_dir(&trash_dir, &name);
            move_path(&entry.path, &trash_target)?;
            self.record_edit_op(FileManagerEditOp::Moved {
                from: entry.path,
                to: trash_target,
            });
            moved += 1;
        }
        Ok(moved)
    }

    pub fn undo(&mut self, file_manager: &mut NativeFileManagerState) -> Result<String> {
        let Some(op) = self.undo_stack.pop() else {
            return Err(anyhow!("Nothing to undo."));
        };
        apply_edit_op(&op, true)?;
        self.redo_stack.push(op);
        file_manager.invalidate_view_cache();
        file_manager.ensure_selection_valid();
        Ok("Undo complete".to_string())
    }

    pub fn redo(&mut self, file_manager: &mut NativeFileManagerState) -> Result<String> {
        let Some(op) = self.redo_stack.pop() else {
            return Err(anyhow!("Nothing to redo."));
        };
        apply_edit_op(&op, false)?;
        self.undo_stack.push(op);
        file_manager.invalidate_view_cache();
        file_manager.ensure_selection_valid();
        Ok("Redo complete".to_string())
    }

    fn record_edit_op(&mut self, op: FileManagerEditOp) {
        self.undo_stack.push(op);
        self.redo_stack.clear();
        if self.undo_stack.len() > 100 {
            let overflow = self.undo_stack.len().saturating_sub(100);
            self.undo_stack.drain(0..overflow);
        }
    }
}

fn split_file_name(name: &str) -> (&str, &str) {
    if let Some((stem, _ext)) = name.rsplit_once('.') {
        if !stem.is_empty() {
            return (stem, &name[stem.len()..]);
        }
    }
    (name, "")
}

fn path_display_name(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(|name| name.to_string())
        .unwrap_or_else(|| path.display().to_string())
}

fn unique_copy_path_in_dir(dir: &Path, original_name: &str, prefer_copy_suffix: bool) -> PathBuf {
    let direct = dir.join(original_name);
    if !prefer_copy_suffix && !direct.exists() {
        return direct;
    }
    let (stem, ext) = split_file_name(original_name);
    for index in 1..=9999usize {
        let candidate = if index == 1 {
            format!("{stem} copy{ext}")
        } else {
            format!("{stem} copy {index}{ext}")
        };
        let path = dir.join(candidate);
        if !path.exists() {
            return path;
        }
    }
    direct
}

fn unique_path_in_dir(dir: &Path, original_name: &str) -> PathBuf {
    let direct = dir.join(original_name);
    if !direct.exists() {
        return direct;
    }
    let (stem, ext) = split_file_name(original_name);
    for index in 1..=9999usize {
        let candidate = dir.join(format!("{stem} ({index}){ext}"));
        if !candidate.exists() {
            return candidate;
        }
    }
    direct
}

fn copy_path_recursive(src: &Path, dst: &Path) -> Result<()> {
    let meta =
        std::fs::metadata(src).map_err(|e| anyhow!("Failed reading {}: {e}", src.display()))?;
    if meta.is_dir() {
        std::fs::create_dir_all(dst)
            .map_err(|e| anyhow!("Failed creating {}: {e}", dst.display()))?;
        for item in
            std::fs::read_dir(src).map_err(|e| anyhow!("Failed listing {}: {e}", src.display()))?
        {
            let item = item.map_err(|e| anyhow!("Failed reading {} entry: {e}", src.display()))?;
            let child_src = item.path();
            let child_dst = dst.join(item.file_name());
            copy_path_recursive(&child_src, &child_dst)?;
        }
    } else {
        if let Some(parent) = dst.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| anyhow!("Failed creating {}: {e}", parent.display()))?;
        }
        std::fs::copy(src, dst)
            .map_err(|e| anyhow!("Failed copying {} -> {}: {e}", src.display(), dst.display()))?;
    }
    Ok(())
}

fn remove_path_recursive(path: &Path) -> Result<()> {
    if !path.exists() {
        return Ok(());
    }
    if path.is_dir() {
        std::fs::remove_dir_all(path)
            .map_err(|e| anyhow!("Failed deleting {}: {e}", path.display()))?;
    } else {
        std::fs::remove_file(path)
            .map_err(|e| anyhow!("Failed deleting {}: {e}", path.display()))?;
    }
    Ok(())
}

fn move_path(src: &Path, dst: &Path) -> Result<()> {
    if src == dst {
        return Ok(());
    }
    if dst.exists() {
        return Err(anyhow!("Destination already exists: {}", dst.display()));
    }
    if let Some(parent) = dst.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| anyhow!("Failed creating {}: {e}", parent.display()))?;
    }
    match std::fs::rename(src, dst) {
        Ok(_) => Ok(()),
        Err(_) => {
            copy_path_recursive(src, dst)?;
            remove_path_recursive(src)
        }
    }
}

fn apply_edit_op(op: &FileManagerEditOp, reverse: bool) -> Result<()> {
    match op {
        FileManagerEditOp::CopyCreated { src, dst } => {
            if reverse {
                remove_path_recursive(dst)
            } else {
                copy_path_recursive(src, dst)
            }
        }
        FileManagerEditOp::Moved { from, to } => {
            if reverse {
                move_path(to, from)
            } else {
                move_path(from, to)
            }
        }
    }
}

fn can_move_path_to_dir(src: &Path, target_dir: &Path) -> bool {
    if src == target_dir {
        return false;
    }
    if src.parent().is_some_and(|parent| parent == target_dir) {
        return false;
    }
    !(src.is_dir() && target_dir.starts_with(src))
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TempDirGuard {
        path: PathBuf,
    }

    impl TempDirGuard {
        fn new(prefix: &str) -> Self {
            let unique = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("test clock")
                .as_nanos();
            let path = std::env::temp_dir().join(format!(
                "nucleon_native_file_manager_app_{prefix}_{}_{}",
                std::process::id(),
                unique
            ));
            std::fs::create_dir_all(&path).expect("create temp test dir");
            Self { path }
        }
    }

    impl Drop for TempDirGuard {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }

    #[test]
    fn selected_rows_for_action_prefers_multi_selection() {
        let temp = TempDirGuard::new("selection");
        let a = temp.path.join("a.txt");
        let b = temp.path.join("b.txt");
        std::fs::write(&a, "a").expect("write a");
        std::fs::write(&b, "b").expect("write b");

        let mut fm = NativeFileManagerState::new(temp.path.clone());
        fm.ensure_selection_valid();
        fm.select(Some(a.clone()));
        fm.toggle_selected_path(&a);
        fm.toggle_selected_path(&b);

        let selected: Vec<PathBuf> = fm
            .selected_rows_for_action()
            .into_iter()
            .map(|row| row.path)
            .collect();

        assert_eq!(selected.len(), 2);
        assert!(selected.contains(&a));
        assert!(selected.contains(&b));
    }

    #[test]
    fn create_new_folder_uses_numbered_name_when_needed() {
        let temp = TempDirGuard::new("new_folder");
        std::fs::create_dir_all(temp.path.join("New Folder")).expect("seed existing folder");
        let mut fm = NativeFileManagerState::new(temp.path.clone());
        let mut runtime = FileManagerEditRuntime::default();

        let status = runtime
            .create_new_folder(&mut fm)
            .expect("new folder should be created");

        assert_eq!(status, "Created New Folder (1)");
        assert_eq!(fm.selected, Some(temp.path.join("New Folder (1)")));
    }

    #[test]
    fn create_new_folder_invalidates_cached_rows() {
        let temp = TempDirGuard::new("cache_invalidate");
        let mut fm = NativeFileManagerState::new(temp.path.clone());
        let mut runtime = FileManagerEditRuntime::default();

        let initial_labels: Vec<String> = fm.rows().into_iter().map(|row| row.label).collect();
        assert!(
            !initial_labels.iter().any(|label| label == "New Folder"),
            "new folder should not exist before creation"
        );

        runtime
            .create_new_folder(&mut fm)
            .expect("new folder should be created");

        let labels: Vec<String> = fm.rows().into_iter().map(|row| row.label).collect();
        assert!(
            labels.iter().any(|label| label == "New Folder"),
            "cached rows should refresh after filesystem mutation"
        );
    }

    #[test]
    fn paste_clipboard_into_dir_copies_items_for_desktop_targets() {
        let temp = TempDirGuard::new("desktop_paste");
        let src = temp.path.join("notes.txt");
        let desktop = temp.path.join("Desktop");
        std::fs::write(&src, "hello").expect("write source file");
        std::fs::create_dir_all(&desktop).expect("create desktop dir");

        let mut runtime = FileManagerEditRuntime::default();
        runtime.clipboard = Some(FileManagerClipboardItem {
            paths: vec![src.clone()],
            mode: FileManagerClipboardMode::Copy,
        });

        let (count, last_dst) = runtime
            .paste_clipboard_into_dir(&desktop)
            .expect("paste into desktop");

        assert_eq!(count, 1);
        assert_eq!(last_dst, Some(desktop.join("notes.txt")));
        assert!(src.exists());
        assert!(desktop.join("notes.txt").exists());
    }

    #[test]
    fn copy_paths_into_dir_recursively_imports_folders() {
        let temp = TempDirGuard::new("desktop_import");
        let src_dir = temp.path.join("Projects");
        let desktop = temp.path.join("Desktop");
        std::fs::create_dir_all(src_dir.join("nested")).expect("create source dirs");
        std::fs::create_dir_all(&desktop).expect("create desktop dir");
        std::fs::write(src_dir.join("nested").join("todo.txt"), "ship it").expect("write nested");

        let mut runtime = FileManagerEditRuntime::default();
        let (count, last_dst) = runtime
            .copy_paths_into_dir(vec![src_dir.clone()], &desktop)
            .expect("copy into desktop");

        assert_eq!(count, 1);
        assert_eq!(last_dst, Some(desktop.join("Projects")));
        assert!(desktop
            .join("Projects")
            .join("nested")
            .join("todo.txt")
            .exists());
        assert!(src_dir.join("nested").join("todo.txt").exists());
    }

    #[test]
    fn open_target_for_file_manager_action_prefers_default_open_with() {
        let mut settings = DesktopFileManagerSettings::default();
        nucleon_native_services::shared_file_manager_settings::set_open_with_default_in_settings(
            &mut settings,
            "txt",
            Some("echo"),
        );

        let target = open_target_for_file_manager_action(
            FileManagerAction::OpenFile(PathBuf::from("/tmp/demo.txt")),
            &settings,
        )
        .expect("open-with target should resolve");

        match target {
            FileManagerOpenTarget::Launch(launch) => {
                assert_eq!(
                    launch.argv.last().map(String::as_str),
                    Some("/tmp/demo.txt")
                );
            }
            other => panic!("unexpected target: {other:?}"),
        }
    }

    #[test]
    fn commit_picker_selection_builds_icon_and_wallpaper_results() {
        let icon_entry = FileEntryRow {
            path: PathBuf::from("/tmp/icon.svg"),
            label: "icon.svg".to_string(),
            is_dir: false,
        };
        let wallpaper_entry = FileEntryRow {
            path: PathBuf::from("/tmp/wallpaper.png"),
            label: "wallpaper.png".to_string(),
            is_dir: false,
        };

        assert_eq!(
            commit_picker_selection(Some(icon_entry), FileManagerPickMode::ShortcutIcon(2)),
            Ok(FileManagerPickerCommit::SetShortcutIcon {
                shortcut_idx: 2,
                path: PathBuf::from("/tmp/icon.svg"),
            })
        );
        assert_eq!(
            commit_picker_selection(Some(wallpaper_entry), FileManagerPickMode::Wallpaper),
            Ok(FileManagerPickerCommit::SetWallpaper(PathBuf::from(
                "/tmp/wallpaper.png"
            )))
        );
    }

    #[test]
    fn close_tab_keeps_current_path_when_removing_earlier_tab() {
        let root = PathBuf::from("/");
        let applications = PathBuf::from("/Applications");
        let users = PathBuf::from("/Users");
        let mut fm = NativeFileManagerState::new(root.clone());
        fm.tabs = vec![root, applications.clone(), users.clone()];
        fm.active_tab = 2;
        fm.cwd = users.clone();

        assert!(fm.close_tab(1));
        assert_eq!(fm.tabs, vec![PathBuf::from("/"), users.clone()]);
        assert_eq!(fm.active_tab, 1);
        assert_eq!(fm.cwd, users);
    }

    #[test]
    fn close_tab_selects_successor_when_active_tab_is_removed() {
        let root = PathBuf::from("/");
        let applications = PathBuf::from("/Applications");
        let users = PathBuf::from("/Users");
        let mut fm = NativeFileManagerState::new(root.clone());
        fm.tabs = vec![root, applications, users.clone()];
        fm.active_tab = 1;

        assert!(fm.close_tab(1));
        assert_eq!(fm.tabs, vec![PathBuf::from("/"), users.clone()]);
        assert_eq!(fm.active_tab, 1);
        assert_eq!(fm.cwd, users);
    }
}
