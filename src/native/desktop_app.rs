use super::donkey_kong::BUILTIN_DONKEY_KONG_GAME;
use super::editor_app::{
    build_editor_menu_section, EditorCommand, EditorTextCommand, EditorWindow, EDITOR_APP_TITLE,
};
use super::file_manager::FileManagerCommand;
use super::file_manager::NativeFileManagerState;
use super::file_manager_app::{FileManagerEditRuntime, FileManagerPromptRequest};
use super::file_manager_menu::build_file_manager_menu_section;
use super::file_manager_desktop::FILE_MANAGER_APP_TITLE;
pub use super::shared_types::DesktopWindow;
use crate::config::DesktopFileManagerSettings;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DesktopHostedApp {
    Desktop,
    FileManager,
    Editor,
    Settings,
    Applications,
    Game,
    Utility,
    Terminal,
    Installer,
    PtyApp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DesktopMenuSection {
    File,
    Edit,
    Format,
    View,
    Window,
    Help,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DesktopMenuAction {
    EditorCommand(EditorCommand),
    EditorTextCommand(EditorTextCommand),
    OpenRecentEditorFile(PathBuf),
    FileManagerCommand(FileManagerCommand),
    OpenFileManagerPrompt(FileManagerPromptRequest),
    FileManagerLaunchOpenWithCommand {
        path: PathBuf,
        ext_key: String,
        command: String,
    },
    FileManagerSetOpenWithDefault {
        ext_key: String,
        command: Option<String>,
    },
    FileManagerRemoveOpenWithCommand {
        ext_key: String,
        command: String,
    },
    OpenFileManager,
    OpenApplications,
    OpenSettings,
    ToggleStartMenu,
    CloseActiveDesktopWindow,
    MinimizeActiveDesktopWindow,
    ActivateDesktopWindow(DesktopWindow),
    ActivateTaskbarWindow(DesktopWindow),
    OpenManual {
        path: &'static str,
        status_label: &'static str,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DesktopShellAction {
    OpenWindow(DesktopWindow),
    OpenTextEditor,
    OpenNukeCodes,
    OpenDesktopTerminalShell,
    OpenConnectionsSettings,
    LaunchConfiguredApp(String),
    OpenFileManagerAt(PathBuf),
    LaunchNetworkProgram(String),
    LaunchGameProgram(String),
    OpenPathInEditor(PathBuf),
    RevealPathInFileManager(PathBuf),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DesktopMenuItem {
    Action {
        label: String,
        action: DesktopMenuAction,
    },
    Disabled {
        label: String,
    },
    Label {
        label: String,
    },
    Separator,
    Submenu {
        label: String,
        items: Vec<DesktopMenuItem>,
    },
}

pub struct DesktopMenuBuildContext<'a> {
    pub editor: &'a EditorWindow,
    pub editor_recent_files: &'a [String],
    pub file_manager: &'a NativeFileManagerState,
    pub file_manager_runtime: &'a FileManagerEditRuntime,
    pub file_manager_settings: &'a DesktopFileManagerSettings,
}

pub struct DesktopWindowMenuEntry {
    pub window: DesktopWindow,
    pub open: bool,
    pub active: bool,
}

pub struct DesktopTaskbarEntry {
    pub window: DesktopWindow,
    pub label: String,
    pub inactive: bool,
}

impl DesktopHostedApp {
    pub fn menu_sections(self) -> &'static [DesktopMenuSection] {
        match self {
            DesktopHostedApp::Editor => &[
                DesktopMenuSection::File,
                DesktopMenuSection::Edit,
                DesktopMenuSection::Format,
                DesktopMenuSection::View,
                DesktopMenuSection::Window,
                DesktopMenuSection::Help,
            ],
            _ => &[
                DesktopMenuSection::File,
                DesktopMenuSection::Edit,
                DesktopMenuSection::View,
                DesktopMenuSection::Window,
                DesktopMenuSection::Help,
            ],
        }
    }
}

impl DesktopMenuSection {
    pub fn label(self) -> &'static str {
        match self {
            DesktopMenuSection::File => "File",
            DesktopMenuSection::Edit => "Edit",
            DesktopMenuSection::Format => "Format",
            DesktopMenuSection::View => "View",
            DesktopMenuSection::Window => "Window",
            DesktopMenuSection::Help => "Help",
        }
    }
}

pub fn build_shared_desktop_menu_section(section: DesktopMenuSection) -> Vec<DesktopMenuItem> {
    match section {
        DesktopMenuSection::File => vec![
            DesktopMenuItem::Action {
                label: "My Computer".to_string(),
                action: DesktopMenuAction::OpenFileManager,
            },
            DesktopMenuItem::Action {
                label: "Applications".to_string(),
                action: DesktopMenuAction::OpenApplications,
            },
            DesktopMenuItem::Action {
                label: "Settings".to_string(),
                action: DesktopMenuAction::OpenSettings,
            },
            DesktopMenuItem::Separator,
            DesktopMenuItem::Action {
                label: "Exit".to_string(),
                action: DesktopMenuAction::CloseActiveDesktopWindow,
            },
        ],
        DesktopMenuSection::View => vec![
            DesktopMenuItem::Action {
                label: "My Computer".to_string(),
                action: DesktopMenuAction::OpenFileManager,
            },
            DesktopMenuItem::Action {
                label: "Toggle Start Menu".to_string(),
                action: DesktopMenuAction::ToggleStartMenu,
            },
            DesktopMenuItem::Action {
                label: "Settings".to_string(),
                action: DesktopMenuAction::OpenSettings,
            },
        ],
        DesktopMenuSection::Edit | DesktopMenuSection::Format | DesktopMenuSection::Window => {
            Vec::new()
        }
        DesktopMenuSection::Help => build_help_menu_section(),
    }
}

pub fn build_app_control_menu(has_active_window: bool) -> Vec<DesktopMenuItem> {
    let mut items = Vec::new();
    if has_active_window {
        items.push(DesktopMenuItem::Action {
            label: "Close Focused".to_string(),
            action: DesktopMenuAction::CloseActiveDesktopWindow,
        });
        items.push(DesktopMenuItem::Action {
            label: "Minimize".to_string(),
            action: DesktopMenuAction::MinimizeActiveDesktopWindow,
        });
    } else {
        items.push(DesktopMenuItem::Label {
            label: "No active app".to_string(),
        });
        items.push(DesktopMenuItem::Disabled {
            label: "Minimize".to_string(),
        });
    }
    items
}

pub fn build_help_menu_section() -> Vec<DesktopMenuItem> {
    vec![
        DesktopMenuItem::Action {
            label: "App Manual".to_string(),
            action: DesktopMenuAction::OpenManual {
                path: "README.md",
                status_label: "App Manual",
            },
        },
        DesktopMenuItem::Action {
            label: "User Manual".to_string(),
            action: DesktopMenuAction::OpenManual {
                path: "USER_MANUAL.md",
                status_label: "User Manual",
            },
        },
    ]
}

pub fn build_window_menu_section(
    entries: &[DesktopWindowMenuEntry],
    pty_title: Option<&str>,
) -> Vec<DesktopMenuItem> {
    entries
        .iter()
        .map(|entry| {
            let marker = if entry.active {
                "active"
            } else if entry.open {
                "open"
            } else {
                "closed"
            };
            DesktopMenuItem::Action {
                label: format!(
                    "{marker}: {}",
                    desktop_window_title(entry.window, pty_title)
                ),
                action: DesktopMenuAction::ActivateDesktopWindow(entry.window),
            }
        })
        .collect()
}

pub fn taskbar_window_order() -> &'static [DesktopWindow] {
    &[
        DesktopWindow::FileManager,
        DesktopWindow::Editor,
        DesktopWindow::Settings,
        DesktopWindow::Applications,
        DesktopWindow::DonkeyKong,
        DesktopWindow::NukeCodes,
        DesktopWindow::Installer,
        DesktopWindow::PtyApp,
    ]
}

pub fn build_taskbar_entries(
    open_windows: &[DesktopWindow],
    active_window: Option<DesktopWindow>,
    pty_title: Option<&str>,
) -> Vec<DesktopTaskbarEntry> {
    taskbar_window_order()
        .iter()
        .copied()
        .filter(|window| open_windows.contains(window))
        .map(|window| DesktopTaskbarEntry {
            window,
            label: desktop_window_title(window, pty_title),
            // Taskbar chrome renders the "active" look in the inverse branch.
            inactive: active_window != Some(window),
        })
        .collect()
}

pub fn build_active_desktop_menu_section(
    app: DesktopHostedApp,
    section: DesktopMenuSection,
    context: &DesktopMenuBuildContext<'_>,
) -> Vec<DesktopMenuItem> {
    match app {
        DesktopHostedApp::Editor => build_editor_menu_section(
            section,
            context.editor,
            if section == DesktopMenuSection::File {
                context.editor_recent_files
            } else {
                &[]
            },
        ),
        DesktopHostedApp::FileManager => build_file_manager_menu_section(
            section,
            context.file_manager,
            context.file_manager_runtime,
            context.file_manager_settings,
        ),
        _ if section == DesktopMenuSection::Edit => vec![DesktopMenuItem::Disabled {
            label: "No edit actions".to_string(),
        }],
        _ => Vec::new(),
    }
}

pub fn hosted_app_for_window(window: Option<DesktopWindow>) -> DesktopHostedApp {
    match window {
        Some(DesktopWindow::FileManager) => DesktopHostedApp::FileManager,
        Some(DesktopWindow::Editor) => DesktopHostedApp::Editor,
        Some(DesktopWindow::Settings) => DesktopHostedApp::Settings,
        Some(DesktopWindow::Applications) => DesktopHostedApp::Applications,
        Some(DesktopWindow::DonkeyKong) => DesktopHostedApp::Game,
        Some(DesktopWindow::NukeCodes) => DesktopHostedApp::Utility,
        Some(DesktopWindow::TerminalMode) => DesktopHostedApp::Terminal,
        Some(DesktopWindow::PtyApp) => DesktopHostedApp::PtyApp,
        Some(DesktopWindow::Installer) => DesktopHostedApp::Installer,
        None => DesktopHostedApp::Desktop,
    }
}

pub fn desktop_window_title(window: DesktopWindow, pty_title: Option<&str>) -> String {
    match window {
        DesktopWindow::FileManager => FILE_MANAGER_APP_TITLE.to_string(),
        DesktopWindow::Editor => EDITOR_APP_TITLE.to_string(),
        DesktopWindow::Settings => "Settings".to_string(),
        DesktopWindow::Applications => "Applications".to_string(),
        DesktopWindow::DonkeyKong => BUILTIN_DONKEY_KONG_GAME.to_string(),
        DesktopWindow::NukeCodes => "Nuke Codes".to_string(),
        DesktopWindow::Installer => "Program Installer".to_string(),
        DesktopWindow::TerminalMode => "Terminal".to_string(),
        DesktopWindow::PtyApp => pty_title.unwrap_or("PTY App").to_string(),
    }
}

pub fn desktop_app_menu_name(
    active_window: Option<DesktopWindow>,
    pty_title: Option<&str>,
) -> String {
    active_window
        .map(|window| desktop_window_title(window, pty_title))
        .unwrap_or_else(|| "Desktop".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::DesktopFileManagerSettings;
    use crate::native::editor_app::EditorWindow;
    use crate::native::file_manager::NativeFileManagerState;
    use crate::native::file_manager_app::FileManagerEditRuntime;
    use std::path::PathBuf;

    #[test]
    fn editor_menu_profile_includes_format_menu() {
        assert!(DesktopHostedApp::Editor
            .menu_sections()
            .contains(&DesktopMenuSection::Format));
    }

    #[test]
    fn file_manager_menu_profile_omits_format_menu() {
        assert!(!DesktopHostedApp::FileManager
            .menu_sections()
            .contains(&DesktopMenuSection::Format));
    }

    #[test]
    fn desktop_menu_item_supports_nested_submenus() {
        let item = DesktopMenuItem::Submenu {
            label: "Recent".to_string(),
            items: vec![DesktopMenuItem::Action {
                label: "note.txt".to_string(),
                action: DesktopMenuAction::OpenRecentEditorFile(PathBuf::from("/tmp/note.txt")),
            }],
        };

        assert!(matches!(item, DesktopMenuItem::Submenu { .. }));
    }

    #[test]
    fn menu_section_labels_match_top_bar_titles() {
        assert_eq!(DesktopMenuSection::File.label(), "File");
        assert_eq!(DesktopMenuSection::Format.label(), "Format");
    }

    #[test]
    fn shared_file_menu_spec_keeps_core_desktop_entries() {
        let items = build_shared_desktop_menu_section(DesktopMenuSection::File);

        assert!(items.iter().any(|item| matches!(
            item,
            DesktopMenuItem::Action {
                label,
                action: DesktopMenuAction::OpenApplications,
            } if label == "Applications"
        )));
        assert!(items.iter().any(|item| matches!(
            item,
            DesktopMenuItem::Action {
                label,
                action: DesktopMenuAction::CloseActiveDesktopWindow,
            } if label == "Exit"
        )));
    }

    #[test]
    fn inactive_apps_get_generic_no_edit_actions_item() {
        let editor = EditorWindow::default();
        let file_manager = NativeFileManagerState::new(PathBuf::from("/"));
        let runtime = FileManagerEditRuntime::default();
        let file_manager_settings = DesktopFileManagerSettings::default();
        let context = DesktopMenuBuildContext {
            editor: &editor,
            editor_recent_files: &[],
            file_manager: &file_manager,
            file_manager_runtime: &runtime,
            file_manager_settings: &file_manager_settings,
        };

        let items = build_active_desktop_menu_section(
            DesktopHostedApp::Settings,
            DesktopMenuSection::Edit,
            &context,
        );

        assert!(items.iter().any(|item| matches!(
            item,
            DesktopMenuItem::Disabled { label } if label == "No edit actions"
        )));
    }

    #[test]
    fn window_metadata_routes_titles_and_hosted_apps() {
        assert_eq!(
            hosted_app_for_window(Some(DesktopWindow::Editor)),
            DesktopHostedApp::Editor
        );
        assert_eq!(
            desktop_window_title(DesktopWindow::FileManager, None),
            FILE_MANAGER_APP_TITLE
        );
        assert_eq!(
            desktop_window_title(DesktopWindow::PtyApp, Some("Shell")),
            "Shell"
        );
    }

    #[test]
    fn app_control_menu_reflects_focus_state() {
        let active = build_app_control_menu(true);
        let inactive = build_app_control_menu(false);

        assert!(active.iter().any(|item| matches!(
            item,
            DesktopMenuItem::Action {
                label,
                action: DesktopMenuAction::MinimizeActiveDesktopWindow,
            } if label == "Minimize"
        )));
        assert!(inactive.iter().any(|item| matches!(
            item,
            DesktopMenuItem::Label { label } if label == "No active app"
        )));
    }

    #[test]
    fn help_menu_spec_opens_manuals() {
        let items = build_help_menu_section();

        assert!(items.iter().any(|item| matches!(
            item,
            DesktopMenuItem::Action {
                label,
                action: DesktopMenuAction::OpenManual { path: "README.md", .. },
            } if label == "App Manual"
        )));
    }

    #[test]
    fn window_menu_spec_reflects_window_state_markers() {
        let items = build_window_menu_section(
            &[DesktopWindowMenuEntry {
                window: DesktopWindow::Editor,
                open: true,
                active: false,
            }],
            None,
        );

        assert!(items.iter().any(|item| matches!(
            item,
            DesktopMenuItem::Action {
                label,
                action: DesktopMenuAction::ActivateDesktopWindow(DesktopWindow::Editor),
            } if label.starts_with("open: ")
        )));
    }

    #[test]
    fn taskbar_entries_follow_window_order_and_active_marker() {
        let entries = build_taskbar_entries(
            &[
                DesktopWindow::Applications,
                DesktopWindow::Editor,
                DesktopWindow::FileManager,
            ],
            Some(DesktopWindow::Editor),
            None,
        );

        assert_eq!(entries[0].window, DesktopWindow::FileManager);
        assert_eq!(entries[1].window, DesktopWindow::Editor);
        assert!(entries[0].inactive);
        assert!(!entries[1].inactive);
    }

    #[test]
    fn shell_action_can_carry_path_and_program_launches() {
        let action = DesktopShellAction::RevealPathInFileManager(PathBuf::from("/tmp/demo.txt"));
        assert!(matches!(
            action,
            DesktopShellAction::RevealPathInFileManager(path) if path == PathBuf::from("/tmp/demo.txt")
        ));
    }
}
