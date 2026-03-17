use super::desktop_app::{DesktopMenuAction, DesktopMenuItem, DesktopMenuSection};
use std::path::PathBuf;
pub use robcos_native_editor_app::{
    EditorCommand, EditorTextAlign, EditorTextCommand, EditorWindow, EDITOR_APP_TITLE,
};

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
