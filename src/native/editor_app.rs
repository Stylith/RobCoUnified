use super::desktop_app::{DesktopMenuAction, DesktopMenuItem, DesktopMenuSection};
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

pub fn build_editor_menu_section(
    section: DesktopMenuSection,
    editor: &EditorWindow,
    recent_files: &[String],
) -> Vec<DesktopMenuItem> {
    match section {
        DesktopMenuSection::File => {
            let mut items = vec![
                DesktopMenuItem::Action {
                    label: "Save         Ctrl+S".to_string(),
                    action: DesktopMenuAction::EditorCommand(EditorCommand::Save),
                },
                DesktopMenuItem::Action {
                    label: "Save As…".to_string(),
                    action: DesktopMenuAction::EditorCommand(EditorCommand::SaveAs),
                },
                DesktopMenuItem::Separator,
                DesktopMenuItem::Action {
                    label: "New Document".to_string(),
                    action: DesktopMenuAction::EditorCommand(EditorCommand::NewDocument),
                },
                DesktopMenuItem::Action {
                    label: "Open File Manager".to_string(),
                    action: DesktopMenuAction::OpenFileManager,
                },
            ];
            if !recent_files.is_empty() {
                items.push(DesktopMenuItem::Submenu {
                    label: "Open Recent…".to_string(),
                    items: recent_files
                        .iter()
                        .map(|path_str| {
                            let label = PathBuf::from(path_str)
                                .file_name()
                                .and_then(|name| name.to_str())
                                .unwrap_or(path_str.as_str())
                                .to_string();
                            DesktopMenuItem::Action {
                                label,
                                action: DesktopMenuAction::OpenRecentEditorFile(PathBuf::from(
                                    path_str,
                                )),
                            }
                        })
                        .collect(),
                });
            }
            items.push(DesktopMenuItem::Separator);
            items
        }
        DesktopMenuSection::Edit => vec![
            DesktopMenuItem::Action {
                label: "Undo         Ctrl+Z".to_string(),
                action: DesktopMenuAction::EditorTextCommand(EditorTextCommand::Undo),
            },
            DesktopMenuItem::Action {
                label: "Redo         Ctrl+Y".to_string(),
                action: DesktopMenuAction::EditorTextCommand(EditorTextCommand::Redo),
            },
            DesktopMenuItem::Separator,
            DesktopMenuItem::Action {
                label: "Cut          Ctrl+X".to_string(),
                action: DesktopMenuAction::EditorTextCommand(EditorTextCommand::Cut),
            },
            DesktopMenuItem::Action {
                label: "Copy         Ctrl+C".to_string(),
                action: DesktopMenuAction::EditorTextCommand(EditorTextCommand::Copy),
            },
            DesktopMenuItem::Action {
                label: "Paste        Ctrl+V".to_string(),
                action: DesktopMenuAction::EditorTextCommand(EditorTextCommand::Paste),
            },
            DesktopMenuItem::Separator,
            DesktopMenuItem::Action {
                label: "Select All   Ctrl+A".to_string(),
                action: DesktopMenuAction::EditorTextCommand(EditorTextCommand::SelectAll),
            },
            DesktopMenuItem::Separator,
            DesktopMenuItem::Action {
                label: "Find          Ctrl+F".to_string(),
                action: DesktopMenuAction::EditorCommand(EditorCommand::OpenFind),
            },
            DesktopMenuItem::Action {
                label: "Find & Replace Ctrl+H".to_string(),
                action: DesktopMenuAction::EditorCommand(EditorCommand::OpenFindReplace),
            },
        ],
        DesktopMenuSection::Format => {
            let word_wrap_label = if editor.word_wrap {
                "[x] Word Wrap".to_string()
            } else {
                "[ ] Word Wrap".to_string()
            };
            let (marker_left, marker_center, marker_right) = match editor.ui.text_align {
                EditorTextAlign::Left => ("[x] ", "[ ] ", "[ ] "),
                EditorTextAlign::Center => ("[ ] ", "[x] ", "[ ] "),
                EditorTextAlign::Right => ("[ ] ", "[ ] ", "[x] "),
            };
            vec![
                DesktopMenuItem::Action {
                    label: word_wrap_label,
                    action: DesktopMenuAction::EditorCommand(EditorCommand::ToggleWordWrap),
                },
                DesktopMenuItem::Separator,
                DesktopMenuItem::Action {
                    label: "Font Larger  Ctrl++".to_string(),
                    action: DesktopMenuAction::EditorCommand(EditorCommand::IncreaseFontSize),
                },
                DesktopMenuItem::Action {
                    label: "Font Smaller Ctrl+-".to_string(),
                    action: DesktopMenuAction::EditorCommand(EditorCommand::DecreaseFontSize),
                },
                DesktopMenuItem::Action {
                    label: "Reset Font".to_string(),
                    action: DesktopMenuAction::EditorCommand(EditorCommand::ResetFontSize),
                },
                DesktopMenuItem::Separator,
                DesktopMenuItem::Submenu {
                    label: "Align Text ▶".to_string(),
                    items: vec![
                        DesktopMenuItem::Action {
                            label: format!("{marker_left}Left"),
                            action: DesktopMenuAction::EditorCommand(EditorCommand::SetTextAlign(
                                EditorTextAlign::Left,
                            )),
                        },
                        DesktopMenuItem::Action {
                            label: format!("{marker_center}Center"),
                            action: DesktopMenuAction::EditorCommand(EditorCommand::SetTextAlign(
                                EditorTextAlign::Center,
                            )),
                        },
                        DesktopMenuItem::Action {
                            label: format!("{marker_right}Right"),
                            action: DesktopMenuAction::EditorCommand(EditorCommand::SetTextAlign(
                                EditorTextAlign::Right,
                            )),
                        },
                    ],
                },
            ]
        }
        DesktopMenuSection::View => {
            let line_number_label = if editor.ui.show_line_numbers {
                "[x] Line Numbers".to_string()
            } else {
                "[ ] Line Numbers".to_string()
            };
            vec![
                DesktopMenuItem::Action {
                    label: line_number_label,
                    action: DesktopMenuAction::EditorCommand(EditorCommand::ToggleLineNumbers),
                },
                DesktopMenuItem::Separator,
            ]
        }
        DesktopMenuSection::Window | DesktopMenuSection::Help => Vec::new(),
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

    #[test]
    fn editor_file_menu_spec_includes_recent_submenu_when_files_exist() {
        let items = build_editor_menu_section(
            DesktopMenuSection::File,
            &EditorWindow::default(),
            &["/tmp/note.txt".to_string()],
        );

        assert!(items.iter().any(|item| matches!(
            item,
            DesktopMenuItem::Submenu { label, .. } if label == "Open Recent…"
        )));
    }

    #[test]
    fn editor_format_menu_spec_tracks_alignment_and_word_wrap() {
        let mut editor = EditorWindow::default();
        editor.word_wrap = false;
        editor.ui.text_align = EditorTextAlign::Center;

        let items = build_editor_menu_section(DesktopMenuSection::Format, &editor, &[]);

        assert!(matches!(
            items.first(),
            Some(DesktopMenuItem::Action { label, .. }) if label == "[ ] Word Wrap"
        ));
        assert!(items.iter().any(|item| matches!(
            item,
            DesktopMenuItem::Submenu { label, items }
                if label == "Align Text ▶"
                    && items.iter().any(|child| matches!(
                        child,
                        DesktopMenuItem::Action { label, .. } if label == "[x] Center"
                    ))
        )));
    }
}
