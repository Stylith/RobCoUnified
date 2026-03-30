use super::app::NucleonNativeApp;
use super::editor_app::{
    build_editor_menu_section, EditorCommand, EditorTextCommand, EditorWindow, EDITOR_APP_TITLE,
};
use super::file_manager::FileManagerCommand;
use super::file_manager::NativeFileManagerState;
use super::file_manager_app::{FileManagerEditRuntime, FileManagerPromptRequest};
use super::file_manager_desktop::FILE_MANAGER_APP_TITLE;
use super::file_manager_menu::build_file_manager_menu_section;
pub use super::shared_types::{DesktopWindow, WindowInstanceId};
use crate::config::DesktopFileManagerSettings;
use crate::native::NativeSettingsPanel;
use crate::platform::LaunchTarget;
use eframe::egui::Context;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DesktopTitleKind {
    Static(&'static str),
    PtyFallback(&'static str),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DesktopHostedApp {
    Desktop,
    FileManager,
    Editor,
    Settings,
    Tweaks,
    Addons,
    Applications,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DesktopWindowTileAction {
    LeftHalf,
    RightHalf,
    TopHalf,
    BottomHalf,
    TopLeftQuarter,
    TopRightQuarter,
    BottomLeftQuarter,
    BottomRightQuarter,
    Center,
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
    OpenSettings,
    ToggleStartMenu,
    CloseActiveDesktopWindow,
    MinimizeActiveDesktopWindow,
    TileActiveDesktopWindow(DesktopWindowTileAction),
    ActivateDesktopWindow(WindowInstanceId),
    ActivateTaskbarWindow(WindowInstanceId),
    OpenManual {
        path: &'static str,
        status_label: &'static str,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DesktopLaunchPayload {
    OpenTerminalShell,
    OpenSettingsPanel(NativeSettingsPanel),
    OpenPath(PathBuf),
    RevealPath(PathBuf),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DesktopShellAction {
    LaunchByTarget(LaunchTarget),
    LaunchByTargetWithPayload {
        target: LaunchTarget,
        payload: DesktopLaunchPayload,
    },
    LaunchConfiguredApp(String),
    OpenFileManagerAt(PathBuf),
    LaunchNetworkProgram(String),
    LaunchGameProgram(String),
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
    pub id: WindowInstanceId,
    pub open: bool,
    pub active: bool,
}

pub struct DesktopTaskbarEntry {
    pub id: WindowInstanceId,
    pub label: String,
    pub inactive: bool,
}

#[derive(Clone, Copy)]
pub struct DesktopComponentBinding {
    pub spec: DesktopComponentSpec,
    pub is_open: fn(&NucleonNativeApp) -> bool,
    pub set_open: fn(&mut NucleonNativeApp, bool),
    pub draw: fn(&mut NucleonNativeApp, &Context),
    pub on_open: Option<fn(&mut NucleonNativeApp, bool)>,
    pub on_closed: Option<fn(&mut NucleonNativeApp)>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DesktopComponentSpec {
    pub window: DesktopWindow,
    pub hosted_app: DesktopHostedApp,
    pub id_salt: &'static str,
    pub default_size: [f32; 2],
    pub show_in_taskbar: bool,
    pub show_in_window_menu: bool,
    title_kind: DesktopTitleKind,
}

const DESKTOP_COMPONENT_BINDINGS: [DesktopComponentBinding; 9] = [
    DesktopComponentBinding {
        spec: DesktopComponentSpec {
            window: DesktopWindow::FileManager,
            hosted_app: DesktopHostedApp::FileManager,
            id_salt: "native_file_manager",
            default_size: [860.0, 560.0],
            show_in_taskbar: true,
            show_in_window_menu: true,
            title_kind: DesktopTitleKind::Static(FILE_MANAGER_APP_TITLE),
        },
        is_open: NucleonNativeApp::desktop_component_file_manager_is_open,
        set_open: NucleonNativeApp::desktop_component_file_manager_set_open,
        draw: NucleonNativeApp::desktop_component_file_manager_draw,
        on_open: None,
        on_closed: None,
    },
    DesktopComponentBinding {
        spec: DesktopComponentSpec {
            window: DesktopWindow::Editor,
            hosted_app: DesktopHostedApp::Editor,
            id_salt: "native_word_processor",
            default_size: [820.0, 560.0],
            show_in_taskbar: true,
            show_in_window_menu: true,
            title_kind: DesktopTitleKind::Static(EDITOR_APP_TITLE),
        },
        is_open: NucleonNativeApp::desktop_component_editor_is_open,
        set_open: NucleonNativeApp::desktop_component_editor_set_open,
        draw: NucleonNativeApp::desktop_component_editor_draw,
        on_open: None,
        on_closed: Some(NucleonNativeApp::desktop_component_editor_on_closed),
    },
    DesktopComponentBinding {
        spec: DesktopComponentSpec {
            window: DesktopWindow::Settings,
            hosted_app: DesktopHostedApp::Settings,
            id_salt: "native_settings",
            default_size: [760.0, 500.0],
            show_in_taskbar: true,
            show_in_window_menu: true,
            title_kind: DesktopTitleKind::Static("Settings"),
        },
        is_open: NucleonNativeApp::desktop_component_settings_is_open,
        set_open: NucleonNativeApp::desktop_component_settings_set_open,
        draw: NucleonNativeApp::desktop_component_settings_draw,
        on_open: Some(NucleonNativeApp::desktop_component_settings_on_open),
        on_closed: None,
    },
    DesktopComponentBinding {
        spec: DesktopComponentSpec {
            window: DesktopWindow::Tweaks,
            hosted_app: DesktopHostedApp::Tweaks,
            id_salt: "native_tweaks",
            default_size: [820.0, 560.0],
            show_in_taskbar: true,
            show_in_window_menu: true,
            title_kind: DesktopTitleKind::Static("Tweaks"),
        },
        is_open: NucleonNativeApp::desktop_component_tweaks_is_open,
        set_open: NucleonNativeApp::desktop_component_tweaks_set_open,
        draw: NucleonNativeApp::desktop_component_tweaks_draw,
        on_open: None,
        on_closed: None,
    },
    DesktopComponentBinding {
        spec: DesktopComponentSpec {
            window: DesktopWindow::Addons,
            hosted_app: DesktopHostedApp::Addons,
            id_salt: "native_addons",
            default_size: [900.0, 600.0],
            show_in_taskbar: true,
            show_in_window_menu: true,
            title_kind: DesktopTitleKind::Static("Addons"),
        },
        is_open: NucleonNativeApp::desktop_component_addons_is_open,
        set_open: NucleonNativeApp::desktop_component_addons_set_open,
        draw: NucleonNativeApp::desktop_component_addons_draw,
        on_open: None,
        on_closed: None,
    },
    DesktopComponentBinding {
        spec: DesktopComponentSpec {
            window: DesktopWindow::Applications,
            hosted_app: DesktopHostedApp::Applications,
            id_salt: "native_applications",
            default_size: [700.0, 480.0],
            show_in_taskbar: true,
            show_in_window_menu: true,
            title_kind: DesktopTitleKind::Static("Applications"),
        },
        is_open: NucleonNativeApp::desktop_component_applications_is_open,
        set_open: NucleonNativeApp::desktop_component_applications_set_open,
        draw: NucleonNativeApp::desktop_component_applications_draw,
        on_open: None,
        on_closed: None,
    },
    DesktopComponentBinding {
        spec: DesktopComponentSpec {
            window: DesktopWindow::Installer,
            hosted_app: DesktopHostedApp::Installer,
            id_salt: "native_installer",
            default_size: [800.0, 600.0],
            show_in_taskbar: true,
            show_in_window_menu: false,
            title_kind: DesktopTitleKind::Static("Program Installer"),
        },
        is_open: NucleonNativeApp::desktop_component_installer_is_open,
        set_open: NucleonNativeApp::desktop_component_installer_set_open,
        draw: NucleonNativeApp::desktop_component_installer_draw,
        on_open: Some(NucleonNativeApp::desktop_component_installer_on_open),
        on_closed: None,
    },
    DesktopComponentBinding {
        spec: DesktopComponentSpec {
            window: DesktopWindow::TerminalMode,
            hosted_app: DesktopHostedApp::Terminal,
            id_salt: "native_terminal_mode",
            default_size: [720.0, 500.0],
            show_in_taskbar: false,
            show_in_window_menu: false,
            title_kind: DesktopTitleKind::Static("Terminal"),
        },
        is_open: NucleonNativeApp::desktop_component_terminal_mode_is_open,
        set_open: NucleonNativeApp::desktop_component_terminal_mode_set_open,
        draw: NucleonNativeApp::desktop_component_terminal_mode_draw,
        on_open: Some(NucleonNativeApp::desktop_component_terminal_mode_on_open),
        on_closed: None,
    },
    DesktopComponentBinding {
        spec: DesktopComponentSpec {
            window: DesktopWindow::PtyApp,
            hosted_app: DesktopHostedApp::PtyApp,
            id_salt: "native_desktop_pty",
            default_size: [960.0, 600.0],
            show_in_taskbar: true,
            show_in_window_menu: true,
            title_kind: DesktopTitleKind::PtyFallback("PTY App"),
        },
        is_open: NucleonNativeApp::desktop_component_pty_is_open,
        set_open: NucleonNativeApp::desktop_component_pty_set_open,
        draw: NucleonNativeApp::desktop_component_pty_draw,
        on_open: Some(NucleonNativeApp::desktop_component_pty_on_open),
        on_closed: None,
    },
];

impl DesktopComponentSpec {
    pub fn title(self, pty_title: Option<&str>) -> String {
        match self.title_kind {
            DesktopTitleKind::Static(label) => label.to_string(),
            DesktopTitleKind::PtyFallback(fallback) => pty_title.unwrap_or(fallback).to_string(),
        }
    }
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

pub fn desktop_component_spec(window: DesktopWindow) -> &'static DesktopComponentSpec {
    &desktop_component_binding(window).spec
}

pub fn desktop_components() -> &'static [DesktopComponentBinding] {
    &DESKTOP_COMPONENT_BINDINGS
}

pub fn desktop_component_binding(window: DesktopWindow) -> &'static DesktopComponentBinding {
    DESKTOP_COMPONENT_BINDINGS
        .iter()
        .find(|binding| binding.spec.window == window)
        .expect("desktop component binding")
}

pub fn build_shared_desktop_menu_section(section: DesktopMenuSection) -> Vec<DesktopMenuItem> {
    match section {
        DesktopMenuSection::File => vec![
            DesktopMenuItem::Action {
                label: "My Computer".to_string(),
                action: DesktopMenuAction::OpenFileManager,
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

pub fn build_window_tiling_menu_section(has_active_window: bool) -> Vec<DesktopMenuItem> {
    let tile_item = |label: &str, action: DesktopWindowTileAction| {
        if has_active_window {
            DesktopMenuItem::Action {
                label: label.to_string(),
                action: DesktopMenuAction::TileActiveDesktopWindow(action),
            }
        } else {
            DesktopMenuItem::Disabled {
                label: label.to_string(),
            }
        }
    };

    vec![
        tile_item(
            "Tile Left Half (Ctrl/Cmd+Alt+Left)",
            DesktopWindowTileAction::LeftHalf,
        ),
        tile_item(
            "Tile Right Half (Ctrl/Cmd+Alt+Right)",
            DesktopWindowTileAction::RightHalf,
        ),
        tile_item(
            "Tile Top Half (Ctrl/Cmd+Alt+Up)",
            DesktopWindowTileAction::TopHalf,
        ),
        tile_item(
            "Tile Bottom Half (Ctrl/Cmd+Alt+Down)",
            DesktopWindowTileAction::BottomHalf,
        ),
        DesktopMenuItem::Separator,
        tile_item(
            "Tile Top Left Quarter",
            DesktopWindowTileAction::TopLeftQuarter,
        ),
        tile_item(
            "Tile Top Right Quarter",
            DesktopWindowTileAction::TopRightQuarter,
        ),
        tile_item(
            "Tile Bottom Left Quarter",
            DesktopWindowTileAction::BottomLeftQuarter,
        ),
        tile_item(
            "Tile Bottom Right Quarter",
            DesktopWindowTileAction::BottomRightQuarter,
        ),
        DesktopMenuItem::Separator,
        tile_item("Center Window", DesktopWindowTileAction::Center),
    ]
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

pub fn build_window_menu_section<F>(
    entries: &[DesktopWindowMenuEntry],
    title_for: F,
) -> Vec<DesktopMenuItem>
where
    F: Fn(WindowInstanceId) -> String,
{
    if entries.is_empty() {
        return vec![DesktopMenuItem::Label {
            label: "No windows open".to_string(),
        }];
    }

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
                label: format!("{marker}: {}", title_for(entry.id)),
                action: DesktopMenuAction::ActivateDesktopWindow(entry.id),
            }
        })
        .collect()
}

pub fn build_window_menu_entries(
    open_windows: &[WindowInstanceId],
    active_window: Option<WindowInstanceId>,
) -> Vec<DesktopWindowMenuEntry> {
    let mut entries = Vec::new();
    for component in desktop_components() {
        if !component.spec.show_in_window_menu {
            continue;
        }

        let kind = component.spec.window;
        for id in open_windows.iter().filter(|id| id.kind == kind).copied() {
            entries.push(DesktopWindowMenuEntry {
                id,
                open: true,
                active: active_window == Some(id),
            });
        }
    }
    entries
}

pub fn build_taskbar_entries<F>(
    open_windows: &[WindowInstanceId],
    active_window: Option<WindowInstanceId>,
    title_for: F,
) -> Vec<DesktopTaskbarEntry>
where
    F: Fn(WindowInstanceId) -> String,
{
    let mut entries = Vec::new();
    // Iterate each open window instance, grouped by component order.
    for component in desktop_components() {
        if !component.spec.show_in_taskbar {
            continue;
        }
        let kind = component.spec.window;
        // Count how many instances of this kind are open.
        let instances: Vec<WindowInstanceId> = open_windows
            .iter()
            .filter(|id| id.kind == kind)
            .copied()
            .collect();
        if instances.is_empty() {
            continue;
        }
        let titled_instances: Vec<(WindowInstanceId, String)> = instances
            .into_iter()
            .map(|id| (id, title_for(id)))
            .collect();
        for (idx, (id, base_title)) in titled_instances.iter().enumerate() {
            let title_count = titled_instances
                .iter()
                .filter(|(_, other_title)| other_title == base_title)
                .count();
            let label = if title_count > 1 {
                let title_index = titled_instances[..=idx]
                    .iter()
                    .filter(|(_, other_title)| other_title == base_title)
                    .count();
                format!("{} [{}]", base_title, title_index)
            } else {
                base_title.clone()
            };
            entries.push(DesktopTaskbarEntry {
                id: *id,
                label,
                inactive: active_window != Some(*id),
            });
        }
    }
    entries
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

pub fn hosted_app_for_window(window: Option<WindowInstanceId>) -> DesktopHostedApp {
    window
        .map(|id| hosted_app_for_window_kind(id.kind))
        .unwrap_or(DesktopHostedApp::Desktop)
}

pub fn hosted_app_for_window_kind(window: DesktopWindow) -> DesktopHostedApp {
    desktop_component_spec(window).hosted_app
}

#[cfg(test)]
pub fn desktop_window_title(window: DesktopWindow, pty_title: Option<&str>) -> String {
    desktop_component_spec(window).title(pty_title)
}

pub fn desktop_app_menu_name<F>(active_window: Option<WindowInstanceId>, title_for: F) -> String
where
    F: Fn(WindowInstanceId) -> String,
{
    active_window
        .map(title_for)
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
                action: DesktopMenuAction::OpenFileManager,
            } if label == "My Computer"
        )));
        assert!(items.iter().any(|item| matches!(
            item,
            DesktopMenuItem::Action {
                label,
                action: DesktopMenuAction::OpenSettings,
            } if label == "Settings"
        )));
        assert!(items.iter().any(|item| matches!(
            item,
            DesktopMenuItem::Action {
                label,
                action: DesktopMenuAction::CloseActiveDesktopWindow,
            } if label == "Exit"
        )));
        assert!(!items.iter().any(|item| matches!(
            item,
            DesktopMenuItem::Action { label, .. } if label == "Applications"
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
            hosted_app_for_window(Some(WindowInstanceId::primary(DesktopWindow::Editor))),
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
    fn desktop_component_registry_is_single_source_of_truth() {
        let components = desktop_components();
        assert_eq!(components[0].spec.window, DesktopWindow::FileManager);
        assert_eq!(components[5].spec.window, DesktopWindow::Applications);
        assert_eq!(components[6].spec.window, DesktopWindow::Installer);
        assert_eq!(components[7].spec.window, DesktopWindow::TerminalMode);
        assert!(!components[7].spec.show_in_taskbar);
        assert!(!components[6].spec.show_in_window_menu);
        assert_eq!(
            desktop_component_spec(DesktopWindow::Settings).id_salt,
            "native_settings"
        );
    }

    #[test]
    fn desktop_component_bindings_align_with_registry_entries() {
        let components = desktop_components();

        for component in components {
            let binding = desktop_component_binding(component.spec.window);
            assert_eq!(binding.spec.window, component.spec.window);
        }
        assert!(desktop_component_binding(DesktopWindow::Editor)
            .on_open
            .is_none());
        assert!(desktop_component_binding(DesktopWindow::Settings)
            .on_open
            .is_some());
        assert!(desktop_component_binding(DesktopWindow::Editor)
            .on_closed
            .is_some());
        assert!(desktop_component_binding(DesktopWindow::Installer)
            .on_closed
            .is_none());
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
    fn window_tiling_menu_reflects_focus_state() {
        let active = build_window_tiling_menu_section(true);
        let inactive = build_window_tiling_menu_section(false);

        assert!(active.iter().any(|item| matches!(
            item,
            DesktopMenuItem::Action {
                label,
                action: DesktopMenuAction::TileActiveDesktopWindow(
                    DesktopWindowTileAction::LeftHalf
                ),
            } if label.contains("Tile Left Half")
        )));
        assert!(inactive.iter().any(|item| matches!(
            item,
            DesktopMenuItem::Disabled { label } if label.contains("Tile Left Half")
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
        let editor_id = WindowInstanceId::primary(DesktopWindow::Editor);
        let items = build_window_menu_section(
            &[DesktopWindowMenuEntry {
                id: editor_id,
                open: true,
                active: false,
            }],
            |id| desktop_window_title(id.kind, None),
        );

        assert!(items.iter().any(|item| matches!(
            item,
            DesktopMenuItem::Action {
                label,
                action: DesktopMenuAction::ActivateDesktopWindow(id),
            } if label.starts_with("open: ") && id.kind == DesktopWindow::Editor
        )));
    }

    #[test]
    fn empty_window_menu_shows_placeholder() {
        let items = build_window_menu_section(&[], |id| desktop_window_title(id.kind, None));

        assert!(matches!(
            items.as_slice(),
            [DesktopMenuItem::Label { label }] if label == "No windows open"
        ));
    }

    #[test]
    fn window_menu_entries_only_include_open_windows() {
        let entries = build_window_menu_entries(
            &[
                WindowInstanceId::primary(DesktopWindow::Editor),
                WindowInstanceId::primary(DesktopWindow::PtyApp),
            ],
            Some(WindowInstanceId::primary(DesktopWindow::Editor)),
        );

        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].id.kind, DesktopWindow::Editor);
        assert!(entries[0].active);
        assert_eq!(entries[1].id.kind, DesktopWindow::PtyApp);
        assert!(!entries[1].active);
        assert!(entries.iter().all(|entry| entry.open));
        assert!(!entries
            .iter()
            .any(|entry| entry.id.kind == DesktopWindow::Applications));
    }

    #[test]
    fn taskbar_entries_follow_window_order_and_active_marker() {
        let entries = build_taskbar_entries(
            &[
                WindowInstanceId::primary(DesktopWindow::Applications),
                WindowInstanceId::primary(DesktopWindow::Editor),
                WindowInstanceId::primary(DesktopWindow::FileManager),
            ],
            Some(WindowInstanceId::primary(DesktopWindow::Editor)),
            |id| desktop_window_title(id.kind, None),
        );

        assert_eq!(entries[0].id.kind, DesktopWindow::FileManager);
        assert_eq!(entries[1].id.kind, DesktopWindow::Editor);
        assert!(entries[0].inactive);
        assert!(!entries[1].inactive);
    }

    #[test]
    fn taskbar_entries_only_number_duplicate_titles() {
        let entries = build_taskbar_entries(
            &[
                WindowInstanceId::primary(DesktopWindow::PtyApp),
                WindowInstanceId {
                    kind: DesktopWindow::PtyApp,
                    instance: 1,
                },
                WindowInstanceId {
                    kind: DesktopWindow::PtyApp,
                    instance: 2,
                },
                WindowInstanceId {
                    kind: DesktopWindow::PtyApp,
                    instance: 3,
                },
            ],
            Some(WindowInstanceId::primary(DesktopWindow::PtyApp)),
            |id| match id.instance {
                0 => "spotify_player".to_string(),
                1 => "ranger".to_string(),
                _ => "Terminal".to_string(),
            },
        );

        assert_eq!(entries[0].label, "spotify_player");
        assert_eq!(entries[1].label, "ranger");
        assert_eq!(entries[2].label, "Terminal [1]");
        assert_eq!(entries[3].label, "Terminal [2]");
    }

    #[test]
    fn shell_action_can_carry_path_and_program_launches() {
        let action = DesktopShellAction::LaunchByTargetWithPayload {
            target: LaunchTarget::Capability {
                capability: crate::platform::CapabilityId::from("file-browser"),
            },
            payload: DesktopLaunchPayload::RevealPath(PathBuf::from("/tmp/demo.txt")),
        };
        assert!(matches!(
            action,
            DesktopShellAction::LaunchByTargetWithPayload {
                payload: DesktopLaunchPayload::RevealPath(path),
                ..
            } if path == PathBuf::from("/tmp/demo.txt")
        ));
    }
}
