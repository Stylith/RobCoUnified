use std::path::PathBuf;

pub const EDITOR_APP_TITLE: &str = "ROBCO Word Processor";
pub const NEW_DESKTOP_DOCUMENT_STATUS: &str = "New document. Save to choose where it goes.";
pub const NEW_DOCUMENT_STATUS: &str = "New document.";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditorTextAlign {
    Left,
    Center,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditorCommand {
    Save,
    SaveAs,
    NewDocument,
    OpenFind,
    OpenFindReplace,
    CloseFind,
    ToggleWordWrap,
    IncreaseFontSize,
    DecreaseFontSize,
    ResetFontSize,
    SetTextAlign(EditorTextAlign),
    ToggleLineNumbers,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditorTextCommand {
    Undo,
    Redo,
    Cut,
    Copy,
    Paste,
    SelectAll,
}

#[derive(Debug, Clone)]
pub struct EditorUiState {
    pub show_line_numbers: bool,
    pub find_open: bool,
    pub find_replace_visible: bool,
    pub find_query: String,
    pub replace_query: String,
    pub find_occurrence: usize,
    pub text_align: EditorTextAlign,
}

impl Default for EditorUiState {
    fn default() -> Self {
        Self {
            show_line_numbers: false,
            find_open: false,
            find_replace_visible: false,
            find_query: String::new(),
            replace_query: String::new(),
            find_occurrence: 0,
            text_align: EditorTextAlign::Left,
        }
    }
}

impl EditorUiState {
    pub fn open_find(&mut self) {
        self.find_open = true;
        self.find_replace_visible = false;
        self.find_occurrence = 0;
    }

    pub fn open_find_replace(&mut self) {
        self.find_open = true;
        self.find_replace_visible = true;
        self.find_occurrence = 0;
    }

    pub fn close_find(&mut self) {
        self.find_open = false;
    }

    pub fn reset_search(&mut self) {
        self.find_open = false;
        self.find_replace_visible = false;
        self.find_query.clear();
        self.replace_query.clear();
        self.find_occurrence = 0;
    }

    pub fn toggle_line_numbers(&mut self) {
        self.show_line_numbers = !self.show_line_numbers;
    }

    pub fn set_text_align(&mut self, alignment: EditorTextAlign) {
        self.text_align = alignment;
    }
}

#[derive(Debug, Clone)]
pub struct EditorWindow {
    pub open: bool,
    pub path: Option<PathBuf>,
    pub text: String,
    pub dirty: bool,
    pub status: String,
    pub word_wrap: bool,
    pub font_size: f32,
    pub ui: EditorUiState,
    pub save_as_input: Option<String>,
}

impl Default for EditorWindow {
    fn default() -> Self {
        Self {
            open: false,
            path: None,
            text: String::new(),
            dirty: false,
            status: String::new(),
            word_wrap: true,
            font_size: 16.0,
            ui: EditorUiState::default(),
            save_as_input: None,
        }
    }
}

impl EditorWindow {
    pub fn reset_for_desktop_new_document(&mut self) {
        self.path = None;
        self.text.clear();
        self.dirty = false;
        self.status = NEW_DESKTOP_DOCUMENT_STATUS.to_string();
    }

    pub fn prepare_new_document_at(&mut self, path: PathBuf) {
        self.path = Some(path);
        self.text.clear();
        self.dirty = false;
        self.status = NEW_DOCUMENT_STATUS.to_string();
    }

    #[cfg(test)]
    pub fn reset_closed_state(&mut self) {
        self.open = false;
        self.path = None;
        self.text.clear();
        self.dirty = false;
        self.status.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn desktop_new_document_resets_editor_buffer_without_closing() {
        let mut editor = EditorWindow {
            open: true,
            path: Some(PathBuf::from("/tmp/doc.txt")),
            text: "hello".to_string(),
            dirty: true,
            status: "dirty".to_string(),
            word_wrap: false,
            font_size: 22.0,
            ..EditorWindow::default()
        };

        editor.reset_for_desktop_new_document();

        assert!(editor.open);
        assert_eq!(editor.path, None);
        assert!(editor.text.is_empty());
        assert!(!editor.dirty);
        assert_eq!(editor.status, NEW_DESKTOP_DOCUMENT_STATUS);
        assert!(!editor.word_wrap);
        assert_eq!(editor.font_size, 22.0);
        assert!(!editor.ui.find_open);
        assert!(editor.save_as_input.is_none());
    }

    #[test]
    fn closed_state_clears_editor_session_data() {
        let mut editor = EditorWindow {
            open: true,
            path: Some(PathBuf::from("/tmp/doc.txt")),
            text: "hello".to_string(),
            dirty: true,
            status: "dirty".to_string(),
            word_wrap: true,
            font_size: 16.0,
            ui: EditorUiState {
                find_open: true,
                ..EditorUiState::default()
            },
            save_as_input: Some("doc.txt".to_string()),
        };

        editor.reset_closed_state();

        assert!(!editor.open);
        assert_eq!(editor.path, None);
        assert!(editor.text.is_empty());
        assert!(!editor.dirty);
        assert!(editor.status.is_empty());
        assert!(editor.ui.find_open);
        assert_eq!(editor.save_as_input, Some("doc.txt".to_string()));
    }

    #[test]
    fn ui_state_reset_search_clears_and_closes_find_overlay() {
        let mut ui = EditorUiState {
            show_line_numbers: true,
            find_open: true,
            find_replace_visible: true,
            find_query: "find".to_string(),
            replace_query: "replace".to_string(),
            find_occurrence: 5,
            text_align: EditorTextAlign::Right,
        };

        ui.reset_search();

        assert!(ui.show_line_numbers);
        assert!(!ui.find_open);
        assert!(!ui.find_replace_visible);
        assert!(ui.find_query.is_empty());
        assert!(ui.replace_query.is_empty());
        assert_eq!(ui.find_occurrence, 0);
        assert_eq!(ui.text_align, EditorTextAlign::Right);
    }
}
