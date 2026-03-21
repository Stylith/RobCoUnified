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

/// Strip the file extension and replace underscores with spaces for display.
fn prettify_file_label(name: &str) -> String {
    let without_ext = match name.rfind('.') {
        Some(dot) if dot > 0 => &name[..dot],
        _ => name,
    };
    without_ext.replace('_', " ")
}

/// Sort key that ignores a leading "The " (case-insensitive) so books like
/// "The Great Gatsby" sort under G rather than T.
fn sort_key(label: &str) -> String {
    let trimmed = label.trim();
    let stripped = trimmed
        .strip_prefix("The ")
        .or_else(|| trimmed.strip_prefix("the "))
        .unwrap_or(trimmed);
    stripped.to_ascii_lowercase()
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
            prettify_file_label(&row.label)
        };
        rows.push(DocumentBrowserRow {
            label,
            path: Some(row.path),
        });
    }
    // Sort non-directory entries alphabetically, ignoring leading "The ".
    // Keep the parent ".." entry at the top.
    rows.sort_by(|a, b| {
        let a_is_parent = a.label == "../";
        let b_is_parent = b.label == "../";
        if a_is_parent || b_is_parent {
            return if a_is_parent {
                std::cmp::Ordering::Less
            } else {
                std::cmp::Ordering::Greater
            };
        }
        let a_is_dir = a.label.starts_with("[DIR]");
        let b_is_dir = b.label.starts_with("[DIR]");
        match (a_is_dir, b_is_dir) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => sort_key(&a.label).cmp(&sort_key(&b.label)),
        }
    });
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
