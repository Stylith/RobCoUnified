use robcos_native_file_manager_app::{FileManagerAction, NativeFileManagerState};
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct DocumentBrowserRow {
    pub label: String,
    pub path: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TerminalDocumentBrowserRequest {
    None,
    ChangedDir,
    OpenFile(PathBuf),
}

pub fn browser_rows(file_manager: &NativeFileManagerState) -> Vec<DocumentBrowserRow> {
    let mut rows = Vec::new();
    for row in file_manager.rows() {
        let label = if row.is_dir {
            if row.label == ".." {
                "../".to_string()
            } else {
                format!("[DIR] {}", row.label)
            }
        } else {
            row.label
        };
        rows.push(DocumentBrowserRow {
            label,
            path: Some(row.path),
        });
    }
    if rows.is_empty() {
        rows.push(DocumentBrowserRow {
            label: "(empty)".to_string(),
            path: None,
        });
    }
    rows
}

pub fn activate_browser_selection(
    file_manager: &mut NativeFileManagerState,
    selected_idx: usize,
) -> TerminalDocumentBrowserRequest {
    let rows = browser_rows(file_manager);
    let idx = selected_idx.min(rows.len().saturating_sub(1));
    let Some(path) = rows.get(idx).and_then(|row| row.path.clone()) else {
        return TerminalDocumentBrowserRequest::None;
    };
    file_manager.select(Some(path));
    match file_manager.activate_selected() {
        FileManagerAction::None => TerminalDocumentBrowserRequest::None,
        FileManagerAction::ChangedDir => TerminalDocumentBrowserRequest::ChangedDir,
        FileManagerAction::OpenFile(path) => TerminalDocumentBrowserRequest::OpenFile(path),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn browser_rows_exposes_parent_entry_for_empty_directory() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time went backwards")
            .as_nanos();
        let dir =
            std::env::temp_dir().join(format!("robcos_native_document_browser_app_test_{unique}"));
        fs::create_dir_all(&dir).expect("create empty temp dir");
        let state = NativeFileManagerState::new(dir);
        let rows = browser_rows(&state);

        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].label, "../");
        assert!(rows[0].path.is_some());
    }
}
