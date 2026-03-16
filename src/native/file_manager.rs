use crate::config::{get_settings, FileManagerSortMode};
use std::cmp::Ordering;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

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

#[derive(Debug, Clone)]
pub enum FileManagerAction {
    None,
    ChangedDir,
    OpenFile(PathBuf),
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
        }
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

    pub fn rows(&self) -> Vec<FileEntryRow> {
        let q = self.search_query.trim().to_ascii_lowercase();
        let rows = Self::read_rows(&self.cwd);
        if q.is_empty() {
            rows
        } else {
            rows.into_iter()
                .filter(|row| row.label.to_ascii_lowercase().contains(&q))
                .collect()
        }
    }

    pub fn tree_items(&self) -> Vec<FileTreeItem> {
        let show_hidden = get_settings().desktop_file_manager.show_hidden_files;
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

    pub fn close_active_tab(&mut self) -> bool {
        if self.tabs.len() <= 1 {
            return false;
        }
        self.tabs.remove(self.active_tab);
        if self.active_tab >= self.tabs.len() {
            self.active_tab = self.tabs.len().saturating_sub(1);
        }
        self.cwd = self.tabs[self.active_tab].clone();
        self.selected = None;
        self.tree_selected = Some(self.cwd.clone());
        true
    }

    pub fn switch_to_tab(&mut self, idx: usize) -> bool {
        if idx >= self.tabs.len() {
            return false;
        }
        self.active_tab = idx;
        self.cwd = self.tabs[idx].clone();
        self.selected = None;
        self.selected_paths.clear();
        self.tree_selected = Some(self.cwd.clone());
        true
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

    fn read_rows(path: &Path) -> Vec<FileEntryRow> {
        let settings = get_settings().desktop_file_manager;
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
                "robco_native_file_manager_{prefix}_{}_{}",
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
    fn tree_items_include_drive_and_folder_sections() {
        let fm = NativeFileManagerState::new(std::env::temp_dir());
        let items = fm.tree_items();

        assert_eq!(items.first().map(|item| item.line.as_str()), Some("Drives"));
        assert!(items.iter().any(|item| item.line == "Folders"));
        assert!(items.iter().any(|item| item.path.is_some()));
    }
}
