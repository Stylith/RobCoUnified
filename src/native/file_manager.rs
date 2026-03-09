use crate::config::{get_settings, FileManagerSortMode};
use std::cmp::Ordering;
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
        self.tree_selected = Some(path);
        self.sync_active_tab_path();
        self.ensure_selection_valid();
    }

    pub fn select(&mut self, path: Option<PathBuf>) {
        self.selected = path;
        self.ensure_selection_valid();
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
        let root = PathBuf::from("/");
        let mut items = vec![FileTreeItem {
            line: "Folders".to_string(),
            path: None,
        }];
        items.push(FileTreeItem {
            line: "* /".to_string(),
            path: Some(root.clone()),
        });

        let rel = self.cwd.strip_prefix(&root).unwrap_or(&self.cwd);
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

        let mut running = root.clone();
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
}
