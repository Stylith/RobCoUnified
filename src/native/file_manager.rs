use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileEntryRow {
    pub path: PathBuf,
    pub label: String,
    pub is_dir: bool,
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
}

impl NativeFileManagerState {
    pub fn new(cwd: PathBuf) -> Self {
        Self {
            open: false,
            cwd,
            selected: None,
        }
    }

    pub fn up(&mut self) {
        if let Some(parent) = self.cwd.parent() {
            self.cwd = parent.to_path_buf();
            self.selected = None;
        }
    }

    pub fn set_cwd(&mut self, path: PathBuf) {
        self.cwd = path;
        self.selected = None;
    }

    pub fn select(&mut self, path: Option<PathBuf>) {
        self.selected = path;
    }

    pub fn rows(&self) -> Vec<FileEntryRow> {
        let mut rows = Vec::new();
        if let Some(parent) = self.cwd.parent() {
            rows.push(FileEntryRow {
                path: parent.to_path_buf(),
                label: "..".to_string(),
                is_dir: true,
            });
        }
        let read_dir = match std::fs::read_dir(&self.cwd) {
            Ok(rd) => rd,
            Err(_) => return rows,
        };
        let mut dirs = Vec::new();
        let mut files = Vec::new();
        for entry in read_dir.flatten() {
            let path = entry.path();
            let label = entry.file_name().to_string_lossy().to_string();
            let is_dir = path.is_dir();
            let row = FileEntryRow {
                path,
                label,
                is_dir,
            };
            if is_dir {
                dirs.push(row);
            } else {
                files.push(row);
            }
        }
        dirs.sort_by(|a, b| a.label.to_lowercase().cmp(&b.label.to_lowercase()));
        files.sort_by(|a, b| a.label.to_lowercase().cmp(&b.label.to_lowercase()));
        rows.extend(dirs);
        rows.extend(files);
        rows
    }

    pub fn activate_selected(&mut self) -> FileManagerAction {
        let Some(path) = self.selected.clone() else {
            return FileManagerAction::None;
        };
        if path.is_dir() {
            self.cwd = path;
            self.selected = None;
            FileManagerAction::ChangedDir
        } else {
            FileManagerAction::OpenFile(path)
        }
    }
}
