use super::desktop_app::{DesktopMenuAction, DesktopMenuItem, DesktopMenuSection};
use super::editor_app::EditorCommand;
use super::file_manager::{FileManagerCommand, NativeFileManagerState};
use super::file_manager_app::FileManagerPromptRequest;
#[cfg(test)]
use super::file_manager_app::{
    record_open_with_command_in_settings, set_open_with_default_in_settings,
    FileManagerClipboardItem,
};
use crate::config::{DesktopFileManagerSettings, FileManagerSortMode, FileManagerViewMode};
use robcos_native_file_manager_app::{
    known_apps_for_extension, open_with_state_for_path, selected_file, FileManagerClipboardMode,
    FileManagerEditRuntime,
};
use std::path::Path;

fn build_file_manager_open_with_menu(
    path: &Path,
    fm: &DesktopFileManagerSettings,
) -> Vec<DesktopMenuItem> {
    let open_with = open_with_state_for_path(path, fm);
    let mut items = Vec::new();
    let known_apps = known_apps_for_extension(&open_with.ext_key);

    // Known apps at the top
    let mut known_commands: std::collections::HashSet<&str> = std::collections::HashSet::new();
    for app in &known_apps {
        known_commands.insert(&app.command);
        items.push(DesktopMenuItem::Action {
            label: app.label.clone(),
            action: DesktopMenuAction::FileManagerLaunchOpenWithCommand {
                path: path.to_path_buf(),
                ext_key: open_with.ext_key.clone(),
                command: app.command.clone(),
            },
        });
    }

    // Saved commands (skip duplicates with known apps)
    let has_saved = open_with
        .saved_commands
        .iter()
        .any(|c| !known_commands.contains(c.as_str()));
    if !known_apps.is_empty() && has_saved {
        items.push(DesktopMenuItem::Separator);
    }
    for command in &open_with.saved_commands {
        if known_commands.contains(command.as_str()) {
            continue;
        }
        let is_default = open_with.current_default.as_deref() == Some(command.as_str());
        let label = if is_default {
            format!("{command} [default]")
        } else {
            command.clone()
        };
        items.push(DesktopMenuItem::Action {
            label,
            action: DesktopMenuAction::FileManagerLaunchOpenWithCommand {
                path: path.to_path_buf(),
                ext_key: open_with.ext_key.clone(),
                command: command.clone(),
            },
        });
    }

    if !items.is_empty() {
        items.push(DesktopMenuItem::Separator);
    }

    items.push(DesktopMenuItem::Action {
        label: "Other...".to_string(),
        action: DesktopMenuAction::OpenFileManagerPrompt(
            FileManagerPromptRequest::open_with_new_command(
                path.to_path_buf(),
                open_with.ext_key.clone(),
                false,
            ),
        ),
    });
    items.push(DesktopMenuItem::Action {
        label: format!("Other + Always Use for {}", open_with.ext_label),
        action: DesktopMenuAction::OpenFileManagerPrompt(
            FileManagerPromptRequest::open_with_new_command(
                path.to_path_buf(),
                open_with.ext_key.clone(),
                true,
            ),
        ),
    });

    if !open_with.saved_commands.is_empty() {
        let mut edit_items = Vec::new();
        for command in &open_with.saved_commands {
            let is_default = open_with.current_default.as_deref() == Some(command.as_str());
            edit_items.push(DesktopMenuItem::Submenu {
                label: command.clone(),
                items: vec![
                    DesktopMenuItem::Action {
                        label: if is_default {
                            "Stop Always Using".to_string()
                        } else {
                            "Always Use".to_string()
                        },
                        action: DesktopMenuAction::FileManagerSetOpenWithDefault {
                            ext_key: open_with.ext_key.clone(),
                            command: (!is_default).then_some(command.clone()),
                        },
                    },
                    DesktopMenuItem::Action {
                        label: "Edit Saved".to_string(),
                        action: DesktopMenuAction::OpenFileManagerPrompt(
                            FileManagerPromptRequest::open_with_edit_command(
                                path.to_path_buf(),
                                open_with.ext_key.clone(),
                                command.clone(),
                            ),
                        ),
                    },
                    DesktopMenuItem::Action {
                        label: "Remove Saved".to_string(),
                        action: DesktopMenuAction::FileManagerRemoveOpenWithCommand {
                            ext_key: open_with.ext_key.clone(),
                            command: command.clone(),
                        },
                    },
                ],
            });
        }
        if open_with.current_default.is_some() {
            edit_items.push(DesktopMenuItem::Separator);
            edit_items.push(DesktopMenuItem::Action {
                label: "Clear Always Use".to_string(),
                action: DesktopMenuAction::FileManagerSetOpenWithDefault {
                    ext_key: open_with.ext_key.clone(),
                    command: None,
                },
            });
        }
        items.push(DesktopMenuItem::Submenu {
            label: "Edit".to_string(),
            items: edit_items,
        });
    }

    items
}

fn file_manager_paste_label(runtime: &FileManagerEditRuntime) -> String {
    if let Some(clip) = &runtime.clipboard {
        let mode = if matches!(clip.mode, FileManagerClipboardMode::Cut) {
            "Move"
        } else {
            "Paste"
        };
        if clip.paths.len() == 1 {
            format!("{mode} {}", path_display_name(&clip.paths[0]))
        } else {
            format!("{mode} {} items", clip.paths.len())
        }
    } else {
        "Paste".to_string()
    }
}

pub fn build_file_manager_menu_section(
    section: DesktopMenuSection,
    file_manager: &NativeFileManagerState,
    runtime: &FileManagerEditRuntime,
    fm: &DesktopFileManagerSettings,
) -> Vec<DesktopMenuItem> {
    let selected_entries = file_manager.selected_rows_for_action();
    let has_selection = !selected_entries.is_empty();
    let selection_count = selected_entries.len();
    let selected_file = selected_file(selected_entries.clone());
    let has_extra_tabs = file_manager.tabs.len() > 1;

    match section {
        DesktopMenuSection::File => {
            let mut items = vec![
                DesktopMenuItem::Action {
                    label: "New Folder   Ctrl+Shift+N".to_string(),
                    action: DesktopMenuAction::FileManagerCommand(FileManagerCommand::NewFolder),
                },
                DesktopMenuItem::Action {
                    label: "New Tab".to_string(),
                    action: DesktopMenuAction::FileManagerCommand(FileManagerCommand::NewTab),
                },
                DesktopMenuItem::Separator,
            ];
            if has_extra_tabs {
                items.extend([
                    DesktopMenuItem::Action {
                        label: "Previous Tab".to_string(),
                        action: DesktopMenuAction::FileManagerCommand(
                            FileManagerCommand::PreviousTab,
                        ),
                    },
                    DesktopMenuItem::Action {
                        label: "Next Tab".to_string(),
                        action: DesktopMenuAction::FileManagerCommand(FileManagerCommand::NextTab),
                    },
                    DesktopMenuItem::Action {
                        label: "Close Tab".to_string(),
                        action: DesktopMenuAction::FileManagerCommand(FileManagerCommand::CloseTab),
                    },
                ]);
            }
            if has_selection {
                items.push(DesktopMenuItem::Action {
                    label: "Open Selected".to_string(),
                    action: DesktopMenuAction::FileManagerCommand(FileManagerCommand::OpenSelected),
                });
            }
            if let Some(entry) = selected_file {
                items.push(DesktopMenuItem::Submenu {
                    label: "Open With".to_string(),
                    items: build_file_manager_open_with_menu(&entry.path, fm),
                });
            }
            items.push(DesktopMenuItem::Action {
                label: "Home".to_string(),
                action: DesktopMenuAction::FileManagerCommand(FileManagerCommand::OpenHome),
            });
            if has_selection {
                items.push(DesktopMenuItem::Separator);
                items.push(DesktopMenuItem::Label {
                    label: format!("{selection_count} selected"),
                });
            }
            items
        }
        DesktopMenuSection::Edit => {
            let has_clipboard = runtime.has_clipboard();
            let has_history = runtime.can_undo() || runtime.can_redo();
            let mut items = vec![
                DesktopMenuItem::Action {
                    label: "Open Selected".to_string(),
                    action: DesktopMenuAction::FileManagerCommand(FileManagerCommand::OpenSelected),
                },
                DesktopMenuItem::Action {
                    label: "Clear Search".to_string(),
                    action: DesktopMenuAction::FileManagerCommand(FileManagerCommand::ClearSearch),
                },
            ];
            if has_selection || has_clipboard || has_history {
                items.push(DesktopMenuItem::Separator);
            }
            if has_selection {
                items.extend([
                    DesktopMenuItem::Action {
                        label: "Copy".to_string(),
                        action: DesktopMenuAction::FileManagerCommand(FileManagerCommand::Copy),
                    },
                    DesktopMenuItem::Action {
                        label: "Cut".to_string(),
                        action: DesktopMenuAction::FileManagerCommand(FileManagerCommand::Cut),
                    },
                    DesktopMenuItem::Action {
                        label: "Duplicate".to_string(),
                        action: DesktopMenuAction::FileManagerCommand(
                            FileManagerCommand::Duplicate,
                        ),
                    },
                    DesktopMenuItem::Action {
                        label: "Rename".to_string(),
                        action: DesktopMenuAction::FileManagerCommand(FileManagerCommand::Rename),
                    },
                    DesktopMenuItem::Action {
                        label: "Move To".to_string(),
                        action: DesktopMenuAction::FileManagerCommand(FileManagerCommand::Move),
                    },
                    DesktopMenuItem::Action {
                        label: "Delete".to_string(),
                        action: DesktopMenuAction::FileManagerCommand(FileManagerCommand::Delete),
                    },
                ]);
            }
            if has_clipboard {
                items.push(DesktopMenuItem::Action {
                    label: file_manager_paste_label(runtime),
                    action: DesktopMenuAction::FileManagerCommand(FileManagerCommand::Paste),
                });
            }
            if runtime.can_undo() {
                items.push(DesktopMenuItem::Action {
                    label: "Undo".to_string(),
                    action: DesktopMenuAction::FileManagerCommand(FileManagerCommand::Undo),
                });
            }
            if runtime.can_redo() {
                items.push(DesktopMenuItem::Action {
                    label: "Redo".to_string(),
                    action: DesktopMenuAction::FileManagerCommand(FileManagerCommand::Redo),
                });
            }
            if has_selection || has_clipboard || has_history {
                items.push(DesktopMenuItem::Separator);
            }
            items.push(DesktopMenuItem::Action {
                label: "New Document".to_string(),
                action: DesktopMenuAction::EditorCommand(EditorCommand::NewDocument),
            });
            items
        }
        DesktopMenuSection::View => {
            vec![
                DesktopMenuItem::Action {
                    label: if fm.show_tree_panel {
                        "Hide Folder Tree".to_string()
                    } else {
                        "Show Folder Tree".to_string()
                    },
                    action: DesktopMenuAction::FileManagerCommand(
                        FileManagerCommand::ToggleTreePanel,
                    ),
                },
                DesktopMenuItem::Action {
                    label: if fm.view_mode == FileManagerViewMode::Grid {
                        "Grid View [Active]".to_string()
                    } else {
                        "Grid View".to_string()
                    },
                    action: DesktopMenuAction::FileManagerCommand(FileManagerCommand::SetViewMode(
                        FileManagerViewMode::Grid,
                    )),
                },
                DesktopMenuItem::Action {
                    label: if fm.view_mode == FileManagerViewMode::List {
                        "List View [Active]".to_string()
                    } else {
                        "List View".to_string()
                    },
                    action: DesktopMenuAction::FileManagerCommand(FileManagerCommand::SetViewMode(
                        FileManagerViewMode::List,
                    )),
                },
                DesktopMenuItem::Action {
                    label: if fm.sort_mode == FileManagerSortMode::Name {
                        "Sort By Name [Active]".to_string()
                    } else {
                        "Sort By Name".to_string()
                    },
                    action: DesktopMenuAction::FileManagerCommand(FileManagerCommand::SetSortMode(
                        FileManagerSortMode::Name,
                    )),
                },
                DesktopMenuItem::Action {
                    label: if fm.sort_mode == FileManagerSortMode::Type {
                        "Sort By Type [Active]".to_string()
                    } else {
                        "Sort By Type".to_string()
                    },
                    action: DesktopMenuAction::FileManagerCommand(FileManagerCommand::SetSortMode(
                        FileManagerSortMode::Type,
                    )),
                },
                DesktopMenuItem::Action {
                    label: if fm.show_hidden_files {
                        "Hide Hidden Files".to_string()
                    } else {
                        "Show Hidden Files".to_string()
                    },
                    action: DesktopMenuAction::FileManagerCommand(
                        FileManagerCommand::ToggleHiddenFiles,
                    ),
                },
                DesktopMenuItem::Separator,
            ]
        }
        DesktopMenuSection::Format | DesktopMenuSection::Window | DesktopMenuSection::Help => {
            Vec::new()
        }
    }
}

fn path_display_name(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(|name| name.to_string())
        .unwrap_or_else(|| path.display().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

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
                "robco_native_file_manager_menu_{prefix}_{}_{}",
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
    fn file_menu_spec_includes_open_with_submenu_for_selected_file() {
        let temp = TempDirGuard::new("file_menu_spec");
        let file_path = temp.path.join("demo.txt");
        std::fs::write(&file_path, "demo").expect("write temp file");
        let mut file_manager = NativeFileManagerState::new(temp.path.clone());
        file_manager.select(Some(file_path));

        let mut settings = DesktopFileManagerSettings::default();
        record_open_with_command_in_settings(&mut settings, "txt", "hx");
        set_open_with_default_in_settings(&mut settings, "txt", Some("hx"));

        let items = build_file_manager_menu_section(
            DesktopMenuSection::File,
            &file_manager,
            &FileManagerEditRuntime::default(),
            &settings,
        );

        let open_with_items = items
            .iter()
            .find_map(|item| match item {
                DesktopMenuItem::Submenu { label, items } if label == "Open With" => Some(items),
                _ => None,
            })
            .expect("open with submenu should exist");

        assert!(open_with_items.iter().any(|item| matches!(
            item,
            DesktopMenuItem::Action { label, action: DesktopMenuAction::FileManagerLaunchOpenWithCommand { .. } }
                if label == "hx [default]"
        )));
    }

    #[test]
    fn edit_menu_spec_uses_clipboard_aware_paste_label() {
        let temp = TempDirGuard::new("edit_menu_spec");
        let file_path = temp.path.join("demo.txt");
        std::fs::write(&file_path, "demo").expect("write temp file");
        let file_manager = NativeFileManagerState::new(temp.path.clone());
        let runtime = FileManagerEditRuntime {
            clipboard: Some(FileManagerClipboardItem {
                paths: vec![file_path],
                mode: FileManagerClipboardMode::Cut,
            }),
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
        };

        let items = build_file_manager_menu_section(
            DesktopMenuSection::Edit,
            &file_manager,
            &runtime,
            &DesktopFileManagerSettings::default(),
        );

        assert!(items.iter().any(|item| matches!(
            item,
            DesktopMenuItem::Action {
                label,
                action: DesktopMenuAction::FileManagerCommand(FileManagerCommand::Paste),
            } if label == "Move demo.txt"
        )));
    }
}
