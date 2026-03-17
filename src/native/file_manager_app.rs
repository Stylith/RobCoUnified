use super::desktop_app::{DesktopMenuAction, DesktopMenuItem, DesktopMenuSection};
use super::editor_app::EditorCommand;
use super::file_manager::{
    FileEntryRow, FileManagerAction, FileManagerCommand, NativeFileManagerState,
};
use super::prompt::{TerminalPrompt, TerminalPromptAction, TerminalPromptKind};
use super::prompt_flow::PromptOutcome;
pub use super::shared_file_manager_settings::{
    open_with_default_for_extension, open_with_history_for_extension,
    FileManagerDisplaySettingsUpdate, FileManagerSettingsUpdate,
};
#[cfg(test)]
pub use super::shared_file_manager_settings::{
    push_open_with_history, record_open_with_command_in_settings,
    remove_open_with_command_in_settings, replace_open_with_command_in_settings,
    set_open_with_default_in_settings, sync_open_with_settings_to_draft,
};
use crate::config::{
    base_dir, DesktopFileManagerSettings, FileManagerSortMode, FileManagerViewMode,
};
use crate::default_apps::parse_custom_command_line;
use crate::launcher::command_exists;
use anyhow::{anyhow, Result};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

pub const FILE_MANAGER_OPEN_WITH_NO_EXT_KEY: &str = "__no_ext__";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileManagerClipboardMode {
    Copy,
    Cut,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileManagerClipboardItem {
    pub paths: Vec<PathBuf>,
    pub mode: FileManagerClipboardMode,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeFileManagerDragPayload {
    pub paths: Vec<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileManagerEditOp {
    CopyCreated { src: PathBuf, dst: PathBuf },
    Moved { from: PathBuf, to: PathBuf },
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FileManagerEditRuntime {
    pub clipboard: Option<FileManagerClipboardItem>,
    pub undo_stack: Vec<FileManagerEditOp>,
    pub redo_stack: Vec<FileManagerEditOp>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenWithLaunchRequest {
    pub argv: Vec<String>,
    pub title: String,
    pub status_message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileManagerOpenWithState {
    pub ext_key: String,
    pub ext_label: String,
    pub saved_commands: Vec<String>,
    pub current_default: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileManagerSelectionPromptKind {
    Rename,
    Move,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileManagerPickMode {
    None,
    SaveAs,
    ShortcutIcon(usize),
    Wallpaper,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileManagerSelectionActivation {
    ActivateSelection,
    FillSaveAsName(String),
    PickShortcutIcon { shortcut_idx: usize, path: PathBuf },
    PickWallpaper(PathBuf),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileManagerOpenTarget {
    NoOp,
    Launch(OpenWithLaunchRequest),
    OpenInEditor(PathBuf),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileManagerPickerCommit {
    SetShortcutIcon { shortcut_idx: usize, path: PathBuf },
    SetWallpaper(PathBuf),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileManagerCommandRequest {
    None,
    ActivateSelection,
    OpenPrompt(FileManagerPromptRequest),
    ApplyDisplaySettings(FileManagerDisplaySettingsUpdate),
    ReportStatus(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileManagerPromptAction {
    Launch(OpenWithLaunchRequest),
    ApplySettingsUpdate(FileManagerSettingsUpdate),
    ReportStatus(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileManagerPromptResolution {
    Rename {
        path: PathBuf,
        name: String,
    },
    Move {
        path: PathBuf,
        destination: String,
    },
    OpenWithNewCommand {
        path: PathBuf,
        ext_key: String,
        make_default: bool,
        command: String,
        launch: OpenWithLaunchRequest,
    },
    OpenWithEditCommand {
        path: PathBuf,
        ext_key: String,
        previous: String,
        command: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileManagerPromptRequest {
    Rename {
        path: PathBuf,
        label: String,
    },
    Move {
        path: PathBuf,
    },
    OpenWithNewCommand {
        path: PathBuf,
        ext_key: String,
        make_default: bool,
    },
    OpenWithEditCommand {
        path: PathBuf,
        ext_key: String,
        previous: String,
    },
}

impl FileManagerPromptRequest {
    pub fn rename(entry: &FileEntryRow) -> Self {
        Self::Rename {
            path: entry.path.clone(),
            label: entry.label.clone(),
        }
    }

    pub fn move_to(entry: &FileEntryRow) -> Self {
        Self::Move {
            path: entry.path.clone(),
        }
    }

    pub fn open_with_new_command(path: PathBuf, ext_key: String, make_default: bool) -> Self {
        Self::OpenWithNewCommand {
            path,
            ext_key,
            make_default,
        }
    }

    pub fn open_with_edit_command(path: PathBuf, ext_key: String, previous: String) -> Self {
        Self::OpenWithEditCommand {
            path,
            ext_key,
            previous,
        }
    }

    pub fn title(&self) -> String {
        match self {
            Self::Rename { .. } => "Rename".to_string(),
            Self::Move { .. } => "Move To".to_string(),
            Self::OpenWithNewCommand { ext_key, .. }
            | Self::OpenWithEditCommand { ext_key, .. } => {
                format!("Open With {}", open_with_extension_label(ext_key))
            }
        }
    }

    pub fn prompt(&self) -> String {
        match self {
            Self::Rename { label, .. } => format!("Rename {} to:", label),
            Self::Move { .. } => "Move to (dir or full path):".to_string(),
            Self::OpenWithNewCommand {
                ext_key,
                make_default,
                ..
            } => {
                let ext_label = open_with_extension_label(ext_key);
                if *make_default {
                    format!("Open with command for {} (saved as default):", ext_label)
                } else {
                    format!("Open with command for {}:", ext_label)
                }
            }
            Self::OpenWithEditCommand { ext_key, .. } => {
                format!(
                    "Edit saved command for {}:",
                    open_with_extension_label(ext_key)
                )
            }
        }
    }

    pub fn initial_buffer(&self) -> String {
        match self {
            Self::Rename { label, .. } => label.clone(),
            Self::OpenWithEditCommand { previous, .. } => previous.clone(),
            Self::Move { .. } | Self::OpenWithNewCommand { .. } => String::new(),
        }
    }

    pub fn to_terminal_prompt(&self) -> TerminalPrompt {
        let action = match self {
            Self::Rename { path, .. } => {
                TerminalPromptAction::FileManagerRename { path: path.clone() }
            }
            Self::Move { path } => TerminalPromptAction::FileManagerMoveTo { path: path.clone() },
            Self::OpenWithNewCommand {
                path,
                ext_key,
                make_default,
            } => TerminalPromptAction::FileManagerOpenWithNewCommand {
                path: path.clone(),
                ext_key: ext_key.clone(),
                make_default: *make_default,
            },
            Self::OpenWithEditCommand {
                path,
                ext_key,
                previous,
            } => TerminalPromptAction::FileManagerOpenWithEditCommand {
                path: path.clone(),
                ext_key: ext_key.clone(),
                previous: previous.clone(),
            },
        };
        TerminalPrompt {
            kind: TerminalPromptKind::Input,
            title: self.title(),
            prompt: self.prompt(),
            buffer: self.initial_buffer(),
            confirm_yes: true,
            action,
        }
    }
}

pub fn open_with_extension_key(path: &Path) -> String {
    path.extension()
        .and_then(|s| s.to_str())
        .map(|s| s.trim().to_ascii_lowercase())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| FILE_MANAGER_OPEN_WITH_NO_EXT_KEY.to_string())
}

pub fn open_with_extension_label(ext_key: &str) -> String {
    if ext_key == FILE_MANAGER_OPEN_WITH_NO_EXT_KEY {
        "(no extension)".to_string()
    } else {
        format!(".{ext_key}")
    }
}

pub fn open_with_command_title(program: &str) -> String {
    let name = Path::new(program)
        .file_name()
        .and_then(|s| s.to_str())
        .filter(|s| !s.is_empty())
        .unwrap_or(program);
    if name.eq_ignore_ascii_case("spotify_player") {
        "spotify".to_string()
    } else {
        name.to_string()
    }
}

pub fn open_with_state_for_path(
    path: &Path,
    fm: &DesktopFileManagerSettings,
) -> FileManagerOpenWithState {
    let ext_key = open_with_extension_key(path);
    FileManagerOpenWithState {
        ext_label: open_with_extension_label(&ext_key),
        saved_commands: open_with_history_for_extension(fm, &ext_key),
        current_default: open_with_default_for_extension(fm, &ext_key),
        ext_key,
    }
}

fn build_file_manager_open_with_menu(
    path: &Path,
    fm: &DesktopFileManagerSettings,
) -> Vec<DesktopMenuItem> {
    let open_with = open_with_state_for_path(path, fm);
    let mut items = Vec::new();

    for command in &open_with.saved_commands {
        let is_default = open_with.current_default.as_deref() == Some(command.as_str());
        let label = if is_default {
            format!("Use: {command} [default]")
        } else {
            format!("Use: {command}")
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

    if !open_with.saved_commands.is_empty() {
        items.push(DesktopMenuItem::Separator);
    }

    items.push(DesktopMenuItem::Action {
        label: "New Command...".to_string(),
        action: DesktopMenuAction::OpenFileManagerPrompt(
            FileManagerPromptRequest::open_with_new_command(
                path.to_path_buf(),
                open_with.ext_key.clone(),
                false,
            ),
        ),
    });
    items.push(DesktopMenuItem::Action {
        label: format!("New Command + Always Use for {}", open_with.ext_label),
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
            let items = vec![
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
            ];
            items
        }
        DesktopMenuSection::Format | DesktopMenuSection::Window | DesktopMenuSection::Help => {
            Vec::new()
        }
    }
}

pub fn prepare_open_with_launch(path: &Path, command_line: &str) -> Result<OpenWithLaunchRequest> {
    let normalized = command_line.trim();
    let Some(mut argv) = parse_custom_command_line(normalized) else {
        return Err(anyhow!("Invalid command line: {normalized}"));
    };
    let program = argv.first().cloned().unwrap_or_default();
    if !program.is_empty() && !command_exists(&program) {
        return Err(anyhow!("Command `{program}` was not found in PATH."));
    }
    argv.push(path.display().to_string());
    Ok(OpenWithLaunchRequest {
        title: format!(
            "{} - {}",
            open_with_command_title(&argv[0]),
            path_display_name(path)
        ),
        status_message: format!("Opened {} in PTY", path_display_name(path)),
        argv,
    })
}

pub fn default_open_with_launch_for_path(
    path: &Path,
    fm: &DesktopFileManagerSettings,
) -> Option<Result<OpenWithLaunchRequest>> {
    let state = open_with_state_for_path(path, fm);
    state
        .current_default
        .as_deref()
        .map(|command| prepare_open_with_launch(path, command))
}

pub fn selection_activation_for_selected_path(
    selected_path: Option<PathBuf>,
    pick_mode: FileManagerPickMode,
) -> FileManagerSelectionActivation {
    let Some(selected_path) = selected_path else {
        return FileManagerSelectionActivation::ActivateSelection;
    };
    if !selected_path.is_file() {
        return FileManagerSelectionActivation::ActivateSelection;
    }
    match pick_mode {
        FileManagerPickMode::None => FileManagerSelectionActivation::ActivateSelection,
        FileManagerPickMode::SaveAs => selected_path
            .file_name()
            .and_then(|name| name.to_str())
            .map(|name| FileManagerSelectionActivation::FillSaveAsName(name.to_string()))
            .unwrap_or(FileManagerSelectionActivation::ActivateSelection),
        FileManagerPickMode::ShortcutIcon(shortcut_idx) => {
            FileManagerSelectionActivation::PickShortcutIcon {
                shortcut_idx,
                path: selected_path,
            }
        }
        FileManagerPickMode::Wallpaper => {
            FileManagerSelectionActivation::PickWallpaper(selected_path)
        }
    }
}

pub fn open_target_for_file_manager_action(
    action: FileManagerAction,
    fm: &DesktopFileManagerSettings,
) -> Result<FileManagerOpenTarget, String> {
    match action {
        FileManagerAction::None | FileManagerAction::ChangedDir => Ok(FileManagerOpenTarget::NoOp),
        FileManagerAction::OpenFile(path) => {
            if let Some(result) = default_open_with_launch_for_path(&path, fm) {
                result
                    .map(FileManagerOpenTarget::Launch)
                    .map_err(|err| format!("Open failed: {err}"))
            } else {
                Ok(FileManagerOpenTarget::OpenInEditor(path))
            }
        }
    }
}

pub fn commit_picker_selection(
    selected_file: Option<FileEntryRow>,
    pick_mode: FileManagerPickMode,
) -> Result<FileManagerPickerCommit, String> {
    let selected_file = match pick_mode {
        FileManagerPickMode::ShortcutIcon(_) => {
            selected_file.ok_or_else(|| "Select an SVG file first.".to_string())?
        }
        FileManagerPickMode::Wallpaper => {
            selected_file.ok_or_else(|| "Select an image file first.".to_string())?
        }
        FileManagerPickMode::None | FileManagerPickMode::SaveAs => {
            return Err("No picker action is active.".to_string());
        }
    };

    match pick_mode {
        FileManagerPickMode::ShortcutIcon(shortcut_idx) => {
            Ok(FileManagerPickerCommit::SetShortcutIcon {
                shortcut_idx,
                path: selected_file.path,
            })
        }
        FileManagerPickMode::Wallpaper => {
            Ok(FileManagerPickerCommit::SetWallpaper(selected_file.path))
        }
        FileManagerPickMode::None | FileManagerPickMode::SaveAs => {
            Err("No picker action is active.".to_string())
        }
    }
}

pub fn run_command(
    command: FileManagerCommand,
    file_manager: &mut NativeFileManagerState,
    runtime: &mut FileManagerEditRuntime,
    home_path: &Path,
) -> FileManagerCommandRequest {
    match command {
        FileManagerCommand::OpenSelected => FileManagerCommandRequest::ActivateSelection,
        FileManagerCommand::ClearSearch => {
            file_manager.clear_search();
            FileManagerCommandRequest::None
        }
        FileManagerCommand::NewFolder => {
            command_status_request(runtime.create_new_folder(file_manager))
        }
        FileManagerCommand::NewTab => {
            file_manager.open_tab_here();
            FileManagerCommandRequest::None
        }
        FileManagerCommand::PreviousTab => {
            file_manager.switch_to_previous_tab();
            FileManagerCommandRequest::None
        }
        FileManagerCommand::NextTab => {
            file_manager.switch_to_next_tab();
            FileManagerCommandRequest::None
        }
        FileManagerCommand::CloseTab => {
            file_manager.close_active_tab();
            FileManagerCommandRequest::None
        }
        FileManagerCommand::OpenHome => {
            file_manager.set_cwd(home_path.to_path_buf());
            FileManagerCommandRequest::None
        }
        FileManagerCommand::GoUp => {
            file_manager.up();
            FileManagerCommandRequest::None
        }
        FileManagerCommand::Copy => command_status_request(runtime.set_clipboard_from_entries(
            &file_manager.selected_rows_for_action(),
            FileManagerClipboardMode::Copy,
        )),
        FileManagerCommand::Cut => command_status_request(runtime.set_clipboard_from_entries(
            &file_manager.selected_rows_for_action(),
            FileManagerClipboardMode::Cut,
        )),
        FileManagerCommand::Paste => command_status_request(runtime.paste_clipboard(file_manager)),
        FileManagerCommand::Duplicate => command_status_request(
            runtime.duplicate_selected(file_manager, file_manager.selected_rows_for_action()),
        ),
        FileManagerCommand::Rename | FileManagerCommand::Move => {
            match prompt_request_for_command(command, file_manager.selected_row()) {
                Ok(Some(request)) => FileManagerCommandRequest::OpenPrompt(request),
                Ok(None) => FileManagerCommandRequest::None,
                Err(status) => FileManagerCommandRequest::ReportStatus(status),
            }
        }
        FileManagerCommand::Delete => command_status_request(
            runtime.delete_selected(file_manager, file_manager.selected_rows_for_action()),
        ),
        FileManagerCommand::Undo => command_status_request(runtime.undo(file_manager)),
        FileManagerCommand::Redo => command_status_request(runtime.redo(file_manager)),
        FileManagerCommand::ToggleTreePanel => FileManagerCommandRequest::ApplyDisplaySettings(
            FileManagerDisplaySettingsUpdate::ToggleTreePanel,
        ),
        FileManagerCommand::ToggleHiddenFiles => FileManagerCommandRequest::ApplyDisplaySettings(
            FileManagerDisplaySettingsUpdate::ToggleHiddenFiles,
        ),
        FileManagerCommand::SetViewMode(mode) => FileManagerCommandRequest::ApplyDisplaySettings(
            FileManagerDisplaySettingsUpdate::SetViewMode(mode),
        ),
        FileManagerCommand::SetSortMode(mode) => FileManagerCommandRequest::ApplyDisplaySettings(
            FileManagerDisplaySettingsUpdate::SetSortMode(mode),
        ),
    }
}

pub fn open_with_cleared_default_status(ext_key: &str) -> String {
    format!(
        "Cleared always-use command for {}.",
        open_with_extension_label(ext_key)
    )
}

pub fn open_with_set_default_status(command: &str, ext_key: &str) -> String {
    format!(
        "Now always using {} for {}.",
        command,
        open_with_extension_label(ext_key)
    )
}

pub fn open_with_removed_saved_status(ext_key: &str) -> String {
    format!(
        "Removed saved command for {}.",
        open_with_extension_label(ext_key)
    )
}

pub fn open_with_updated_saved_status(ext_key: &str) -> String {
    format!(
        "Updated saved command for {}.",
        open_with_extension_label(ext_key)
    )
}

pub fn prompt_request_for_selection(
    entry: Option<FileEntryRow>,
    kind: FileManagerSelectionPromptKind,
) -> Result<FileManagerPromptRequest, String> {
    let Some(entry) = entry else {
        return Err("Select a file or folder first.".to_string());
    };
    Ok(match kind {
        FileManagerSelectionPromptKind::Rename => FileManagerPromptRequest::rename(&entry),
        FileManagerSelectionPromptKind::Move => FileManagerPromptRequest::move_to(&entry),
    })
}

pub fn prompt_request_for_command(
    command: FileManagerCommand,
    entry: Option<FileEntryRow>,
) -> Result<Option<FileManagerPromptRequest>, String> {
    match command {
        FileManagerCommand::Rename => Ok(Some(prompt_request_for_selection(
            entry,
            FileManagerSelectionPromptKind::Rename,
        )?)),
        FileManagerCommand::Move => Ok(Some(prompt_request_for_selection(
            entry,
            FileManagerSelectionPromptKind::Move,
        )?)),
        _ => Ok(None),
    }
}

pub fn resolve_prompt_outcome(
    outcome: &PromptOutcome,
) -> Option<Result<FileManagerPromptResolution, String>> {
    match outcome {
        PromptOutcome::FileManagerRename { path, name } => {
            Some(Ok(FileManagerPromptResolution::Rename {
                path: path.clone(),
                name: name.clone(),
            }))
        }
        PromptOutcome::FileManagerMoveTo { path, destination } => {
            Some(Ok(FileManagerPromptResolution::Move {
                path: path.clone(),
                destination: destination.clone(),
            }))
        }
        PromptOutcome::FileManagerOpenWithNewCommand {
            path,
            ext_key,
            make_default,
            command,
        } => {
            let command = command.trim().to_string();
            if command.is_empty() {
                return Some(Err("Open With canceled.".to_string()));
            }
            match prepare_open_with_launch(path, &command) {
                Ok(launch) => Some(Ok(FileManagerPromptResolution::OpenWithNewCommand {
                    path: path.clone(),
                    ext_key: ext_key.clone(),
                    make_default: *make_default,
                    command,
                    launch,
                })),
                Err(err) if err.to_string().contains("Invalid command line") => {
                    Some(Err("Error: invalid command line".to_string()))
                }
                Err(err) => Some(Err(format!("Open failed: {err}"))),
            }
        }
        PromptOutcome::FileManagerOpenWithEditCommand {
            path,
            ext_key,
            previous,
            command,
        } => {
            let command = command.trim().to_string();
            if command.is_empty() {
                return Some(Err("Edited command cannot be empty.".to_string()));
            }
            if parse_custom_command_line(&command).is_none() {
                return Some(Err("Error: invalid command line".to_string()));
            }
            Some(Ok(FileManagerPromptResolution::OpenWithEditCommand {
                path: path.clone(),
                ext_key: ext_key.clone(),
                previous: previous.clone(),
                command,
            }))
        }
        _ => None,
    }
}

pub fn apply_prompt_outcome(
    outcome: &PromptOutcome,
    file_manager: &mut NativeFileManagerState,
    runtime: &mut FileManagerEditRuntime,
) -> Option<Vec<FileManagerPromptAction>> {
    let resolution = resolve_prompt_outcome(outcome)?;
    Some(match resolution {
        Ok(FileManagerPromptResolution::Rename { path, name }) => {
            if file_manager.selected.as_ref() != Some(&path) {
                file_manager.select(Some(path));
            }
            vec![FileManagerPromptAction::ReportStatus(prompt_edit_status(
                file_manager
                    .selected_row()
                    .ok_or_else(|| anyhow!("Select a file or folder first.")),
                |entry| runtime.rename_selected(file_manager, entry, name),
            ))]
        }
        Ok(FileManagerPromptResolution::Move { path, destination }) => {
            if file_manager.selected.as_ref() != Some(&path) {
                file_manager.select(Some(path));
            }
            vec![FileManagerPromptAction::ReportStatus(prompt_edit_status(
                file_manager
                    .selected_row()
                    .ok_or_else(|| anyhow!("Select a file or folder first.")),
                |entry| runtime.move_selected(file_manager, entry, destination),
            ))]
        }
        Ok(FileManagerPromptResolution::OpenWithNewCommand {
            path: _,
            ext_key,
            make_default,
            command,
            launch,
        }) => {
            let mut actions = vec![
                FileManagerPromptAction::Launch(launch),
                FileManagerPromptAction::ApplySettingsUpdate(
                    FileManagerSettingsUpdate::RecordOpenWithCommand {
                        ext_key: ext_key.clone(),
                        command: command.clone(),
                    },
                ),
            ];
            if make_default {
                actions.push(FileManagerPromptAction::ApplySettingsUpdate(
                    FileManagerSettingsUpdate::SetOpenWithDefaultCommand {
                        ext_key,
                        command: Some(command),
                    },
                ));
            }
            actions
        }
        Ok(FileManagerPromptResolution::OpenWithEditCommand {
            path,
            ext_key,
            previous,
            command,
        }) => {
            if file_manager.selected.as_ref() != Some(&path) {
                file_manager.select(Some(path));
            }
            vec![
                FileManagerPromptAction::ApplySettingsUpdate(
                    FileManagerSettingsUpdate::ReplaceOpenWithCommand {
                        ext_key: ext_key.clone(),
                        old_command: previous,
                        new_command: command,
                    },
                ),
                FileManagerPromptAction::ReportStatus(open_with_updated_saved_status(&ext_key)),
            ]
        }
        Err(status) => vec![FileManagerPromptAction::ReportStatus(status)],
    })
}

pub fn selected_file(mut entries: Vec<FileEntryRow>) -> Option<FileEntryRow> {
    if entries.len() != 1 {
        return None;
    }
    let entry = entries.pop()?;
    entry.path.is_file().then_some(entry)
}

impl FileManagerEditRuntime {
    pub fn has_clipboard(&self) -> bool {
        self.clipboard
            .as_ref()
            .is_some_and(|clipboard| !clipboard.paths.is_empty())
    }

    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    pub fn set_clipboard_from_entries(
        &mut self,
        entries: &[FileEntryRow],
        mode: FileManagerClipboardMode,
    ) -> Result<String> {
        if entries.is_empty() {
            return Err(anyhow!("Select a file or folder first."));
        }
        self.clipboard = Some(FileManagerClipboardItem {
            paths: entries.iter().map(|entry| entry.path.clone()).collect(),
            mode: mode.clone(),
        });
        let noun = if entries.len() == 1 {
            entries[0].label.clone()
        } else {
            format!("{} items", entries.len())
        };
        Ok(match mode {
            FileManagerClipboardMode::Copy => format!("Copied {noun}"),
            FileManagerClipboardMode::Cut => format!("Cut {noun}"),
        })
    }

    pub fn create_new_folder(
        &mut self,
        file_manager: &mut NativeFileManagerState,
    ) -> Result<String> {
        let dst = unique_path_in_dir(&file_manager.cwd, "New Folder");
        std::fs::create_dir_all(&dst)
            .map_err(|e| anyhow!("Failed creating {}: {e}", dst.display()))?;
        file_manager.select(Some(dst.clone()));
        Ok(format!("Created {}", path_display_name(&dst)))
    }

    pub fn duplicate_selected(
        &mut self,
        file_manager: &mut NativeFileManagerState,
        entries: Vec<FileEntryRow>,
    ) -> Result<String> {
        if entries.is_empty() {
            return Err(anyhow!("Select a file or folder first."));
        }
        let mut created = Vec::new();
        for entry in entries {
            let Some(parent) = entry.path.parent() else {
                continue;
            };
            let name = path_display_name(&entry.path);
            let dst = unique_copy_path_in_dir(parent, &name, true);
            copy_path_recursive(&entry.path, &dst)?;
            self.record_edit_op(FileManagerEditOp::CopyCreated {
                src: entry.path,
                dst: dst.clone(),
            });
            created.push(dst);
        }
        let Some(last) = created.last().cloned() else {
            return Err(anyhow!("Cannot duplicate this selection."));
        };
        file_manager.select(Some(last.clone()));
        if created.len() == 1 {
            Ok(format!("Duplicated as {}", path_display_name(&last)))
        } else {
            Ok(format!("Duplicated {} items", created.len()))
        }
    }

    pub fn rename_selected(
        &mut self,
        file_manager: &mut NativeFileManagerState,
        entry: FileEntryRow,
        new_name: String,
    ) -> Result<String> {
        let Some(parent) = entry.path.parent() else {
            return Err(anyhow!("Cannot rename this item."));
        };
        let name = new_name.trim();
        if name.is_empty() {
            return Err(anyhow!("Name cannot be empty."));
        }
        if name.contains('/') || name.contains('\\') {
            return Err(anyhow!("Name cannot contain path separators."));
        }
        if name == entry.label {
            return Ok("Name unchanged.".to_string());
        }
        let dst = parent.join(name);
        if dst.exists() {
            return Err(anyhow!("Destination already exists: {}", dst.display()));
        }
        move_path(&entry.path, &dst)?;
        self.record_edit_op(FileManagerEditOp::Moved {
            from: entry.path,
            to: dst.clone(),
        });
        file_manager.select(Some(dst.clone()));
        Ok(format!("Renamed to {}", path_display_name(&dst)))
    }

    pub fn move_selected(
        &mut self,
        file_manager: &mut NativeFileManagerState,
        entry: FileEntryRow,
        raw_destination: String,
    ) -> Result<String> {
        let mut dst = PathBuf::from(raw_destination.trim());
        if dst.as_os_str().is_empty() {
            return Err(anyhow!("Destination cannot be empty."));
        }
        if dst.is_relative() {
            dst = file_manager.cwd.join(dst);
        }
        if dst.exists() && dst.is_dir() {
            dst = dst.join(path_display_name(&entry.path));
        }
        if dst == entry.path {
            return Ok("Item already at destination.".to_string());
        }
        move_path(&entry.path, &dst)?;
        self.record_edit_op(FileManagerEditOp::Moved {
            from: entry.path.clone(),
            to: dst.clone(),
        });
        if let Some(parent) = dst.parent() {
            file_manager.set_cwd(parent.to_path_buf());
        }
        file_manager.select(Some(dst.clone()));
        Ok(format!("Moved to {}", dst.display()))
    }

    pub fn move_paths_to_dir(
        &mut self,
        file_manager: &mut NativeFileManagerState,
        paths: Vec<PathBuf>,
        target_dir: &Path,
    ) -> Result<String> {
        if !target_dir.is_dir() {
            return Err(anyhow!("Destination folder does not exist."));
        }
        let mut seen = HashSet::new();
        let mut moved = Vec::new();
        let target_dir = target_dir.to_path_buf();
        for src in paths {
            if !seen.insert(src.clone()) || !src.exists() {
                continue;
            }
            if !can_move_path_to_dir(&src, &target_dir) {
                continue;
            }
            let source_name = path_display_name(&src);
            let mut dst = target_dir.join(&source_name);
            if dst.exists() {
                dst = unique_path_in_dir(&target_dir, &source_name);
            }
            if dst == src {
                continue;
            }
            move_path(&src, &dst)?;
            self.record_edit_op(FileManagerEditOp::Moved {
                from: src,
                to: dst.clone(),
            });
            moved.push(dst);
        }
        if moved.is_empty() {
            return Err(anyhow!("Nothing to move."));
        }
        if target_dir == file_manager.cwd {
            if let Some(last) = moved.last().cloned() {
                file_manager.select(Some(last));
            }
        } else {
            file_manager.ensure_selection_valid();
        }
        if moved.len() == 1 {
            Ok(format!("Moved {}", path_display_name(&moved[0])))
        } else {
            Ok(format!("Moved {} items", moved.len()))
        }
    }

    pub fn drop_allowed(paths: &[PathBuf], target_dir: &Path) -> bool {
        paths
            .iter()
            .any(|src| src.exists() && can_move_path_to_dir(src, target_dir))
    }

    pub fn paste_clipboard(&mut self, file_manager: &mut NativeFileManagerState) -> Result<String> {
        let Some(clipboard) = self.clipboard.clone() else {
            return Err(anyhow!("Clipboard is empty."));
        };
        let target_dir = file_manager.cwd.clone();
        let mut changed = 0usize;
        let mut last_dst: Option<PathBuf> = None;

        match clipboard.mode {
            FileManagerClipboardMode::Copy => {
                for src in clipboard.paths {
                    if !src.exists() {
                        continue;
                    }
                    let source_name = path_display_name(&src);
                    let mut dst = target_dir.join(&source_name);
                    if dst.exists() {
                        dst = unique_copy_path_in_dir(&target_dir, &source_name, false);
                    }
                    copy_path_recursive(&src, &dst)?;
                    self.record_edit_op(FileManagerEditOp::CopyCreated {
                        src,
                        dst: dst.clone(),
                    });
                    changed += 1;
                    last_dst = Some(dst);
                }
            }
            FileManagerClipboardMode::Cut => {
                for src in clipboard.paths {
                    if !src.exists() {
                        continue;
                    }
                    let source_name = path_display_name(&src);
                    let source_parent = src.parent().map(Path::to_path_buf);
                    if source_parent.as_deref() == Some(target_dir.as_path()) {
                        continue;
                    }
                    let mut dst = target_dir.join(&source_name);
                    if dst.exists() {
                        dst = unique_path_in_dir(&target_dir, &source_name);
                    }
                    move_path(&src, &dst)?;
                    self.record_edit_op(FileManagerEditOp::Moved {
                        from: src,
                        to: dst.clone(),
                    });
                    changed += 1;
                    last_dst = Some(dst);
                }
                self.clipboard = None;
            }
        }

        if changed == 0 {
            return Err(anyhow!("Clipboard source no longer exists."));
        }
        if let Some(dst) = last_dst {
            file_manager.select(Some(dst.clone()));
            if changed == 1 {
                Ok(format!("Pasted {}", path_display_name(&dst)))
            } else {
                Ok(format!("Pasted {changed} items"))
            }
        } else {
            Err(anyhow!("Clipboard source no longer exists."))
        }
    }

    pub fn delete_selected(
        &mut self,
        file_manager: &mut NativeFileManagerState,
        entries: Vec<FileEntryRow>,
    ) -> Result<String> {
        if entries.is_empty() {
            return Err(anyhow!("Select a file or folder first."));
        }
        let trash_dir = base_dir().join(".fm_trash");
        std::fs::create_dir_all(&trash_dir)
            .map_err(|e| anyhow!("Failed creating trash dir {}: {e}", trash_dir.display()))?;
        let mut moved = 0usize;
        for entry in entries {
            let name = path_display_name(&entry.path);
            let trash_target = unique_path_in_dir(&trash_dir, &name);
            move_path(&entry.path, &trash_target)?;
            self.record_edit_op(FileManagerEditOp::Moved {
                from: entry.path,
                to: trash_target,
            });
            moved += 1;
        }
        file_manager.ensure_selection_valid();
        if moved == 1 {
            Ok("Moved item to trash".to_string())
        } else {
            Ok(format!("Moved {moved} items to trash"))
        }
    }

    pub fn undo(&mut self, file_manager: &mut NativeFileManagerState) -> Result<String> {
        let Some(op) = self.undo_stack.pop() else {
            return Err(anyhow!("Nothing to undo."));
        };
        apply_edit_op(&op, true)?;
        self.redo_stack.push(op);
        file_manager.ensure_selection_valid();
        Ok("Undo complete".to_string())
    }

    pub fn redo(&mut self, file_manager: &mut NativeFileManagerState) -> Result<String> {
        let Some(op) = self.redo_stack.pop() else {
            return Err(anyhow!("Nothing to redo."));
        };
        apply_edit_op(&op, false)?;
        self.undo_stack.push(op);
        file_manager.ensure_selection_valid();
        Ok("Redo complete".to_string())
    }

    fn record_edit_op(&mut self, op: FileManagerEditOp) {
        self.undo_stack.push(op);
        self.redo_stack.clear();
        if self.undo_stack.len() > 100 {
            let overflow = self.undo_stack.len().saturating_sub(100);
            self.undo_stack.drain(0..overflow);
        }
    }
}

fn split_file_name(name: &str) -> (&str, &str) {
    if let Some((stem, _ext)) = name.rsplit_once('.') {
        if !stem.is_empty() {
            return (stem, &name[stem.len()..]);
        }
    }
    (name, "")
}

fn path_display_name(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(|name| name.to_string())
        .unwrap_or_else(|| path.display().to_string())
}

fn unique_copy_path_in_dir(dir: &Path, original_name: &str, prefer_copy_suffix: bool) -> PathBuf {
    let direct = dir.join(original_name);
    if !prefer_copy_suffix && !direct.exists() {
        return direct;
    }
    let (stem, ext) = split_file_name(original_name);
    for index in 1..=9999usize {
        let candidate = if index == 1 {
            format!("{stem} copy{ext}")
        } else {
            format!("{stem} copy {index}{ext}")
        };
        let path = dir.join(candidate);
        if !path.exists() {
            return path;
        }
    }
    direct
}

fn command_status_request(result: Result<String>) -> FileManagerCommandRequest {
    match result {
        Ok(message) => FileManagerCommandRequest::ReportStatus(message),
        Err(err) => FileManagerCommandRequest::ReportStatus(format!("File action failed: {err}")),
    }
}

fn prompt_edit_status(
    entry: Result<FileEntryRow>,
    apply: impl FnOnce(FileEntryRow) -> Result<String>,
) -> String {
    match entry.and_then(apply) {
        Ok(message) => message,
        Err(err) => format!("File action failed: {err}"),
    }
}

fn unique_path_in_dir(dir: &Path, original_name: &str) -> PathBuf {
    let direct = dir.join(original_name);
    if !direct.exists() {
        return direct;
    }
    let (stem, ext) = split_file_name(original_name);
    for index in 1..=9999usize {
        let candidate = dir.join(format!("{stem} ({index}){ext}"));
        if !candidate.exists() {
            return candidate;
        }
    }
    direct
}

fn copy_path_recursive(src: &Path, dst: &Path) -> Result<()> {
    let meta =
        std::fs::metadata(src).map_err(|e| anyhow!("Failed reading {}: {e}", src.display()))?;
    if meta.is_dir() {
        std::fs::create_dir_all(dst)
            .map_err(|e| anyhow!("Failed creating {}: {e}", dst.display()))?;
        for item in
            std::fs::read_dir(src).map_err(|e| anyhow!("Failed listing {}: {e}", src.display()))?
        {
            let item = item.map_err(|e| anyhow!("Failed reading {} entry: {e}", src.display()))?;
            let child_src = item.path();
            let child_dst = dst.join(item.file_name());
            copy_path_recursive(&child_src, &child_dst)?;
        }
    } else {
        if let Some(parent) = dst.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| anyhow!("Failed creating {}: {e}", parent.display()))?;
        }
        std::fs::copy(src, dst)
            .map_err(|e| anyhow!("Failed copying {} -> {}: {e}", src.display(), dst.display()))?;
    }
    Ok(())
}

fn remove_path_recursive(path: &Path) -> Result<()> {
    if !path.exists() {
        return Ok(());
    }
    if path.is_dir() {
        std::fs::remove_dir_all(path)
            .map_err(|e| anyhow!("Failed deleting {}: {e}", path.display()))?;
    } else {
        std::fs::remove_file(path)
            .map_err(|e| anyhow!("Failed deleting {}: {e}", path.display()))?;
    }
    Ok(())
}

fn move_path(src: &Path, dst: &Path) -> Result<()> {
    if src == dst {
        return Ok(());
    }
    if dst.exists() {
        return Err(anyhow!("Destination already exists: {}", dst.display()));
    }
    if let Some(parent) = dst.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| anyhow!("Failed creating {}: {e}", parent.display()))?;
    }
    match std::fs::rename(src, dst) {
        Ok(_) => Ok(()),
        Err(_) => {
            copy_path_recursive(src, dst)?;
            remove_path_recursive(src)
        }
    }
}

fn apply_edit_op(op: &FileManagerEditOp, reverse: bool) -> Result<()> {
    match op {
        FileManagerEditOp::CopyCreated { src, dst } => {
            if reverse {
                remove_path_recursive(dst)
            } else {
                copy_path_recursive(src, dst)
            }
        }
        FileManagerEditOp::Moved { from, to } => {
            if reverse {
                move_path(to, from)
            } else {
                move_path(from, to)
            }
        }
    }
}

fn can_move_path_to_dir(src: &Path, target_dir: &Path) -> bool {
    if src == target_dir {
        return false;
    }
    if src.parent().is_some_and(|parent| parent == target_dir) {
        return false;
    }
    !(src.is_dir() && target_dir.starts_with(src))
}

#[cfg(test)]
mod tests {
    use super::*;

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
                "robco_native_file_manager_app_{prefix}_{}_{}",
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
    fn prompt_request_builds_rename_prompt_copy() {
        let request = FileManagerPromptRequest::rename(&FileEntryRow {
            path: PathBuf::from("/tmp/demo.txt"),
            label: "demo.txt".to_string(),
            is_dir: false,
        });

        assert_eq!(request.title(), "Rename");
        assert_eq!(request.prompt(), "Rename demo.txt to:");
        assert_eq!(request.initial_buffer(), "demo.txt");
    }

    #[test]
    fn prompt_request_converts_to_terminal_prompt() {
        let request = FileManagerPromptRequest::open_with_new_command(
            PathBuf::from("/tmp/demo.txt"),
            "txt".to_string(),
            true,
        );
        let prompt = request.to_terminal_prompt();

        assert_eq!(prompt.title, "Open With .txt");
        assert_eq!(
            prompt.prompt,
            "Open with command for .txt (saved as default):"
        );
        assert!(matches!(
            prompt.action,
            TerminalPromptAction::FileManagerOpenWithNewCommand {
                make_default: true,
                ..
            }
        ));
    }

    #[test]
    fn prompt_request_for_command_uses_selected_entry() {
        let entry = FileEntryRow {
            path: PathBuf::from("/tmp/demo.txt"),
            label: "demo.txt".to_string(),
            is_dir: false,
        };

        let request = prompt_request_for_command(FileManagerCommand::Rename, Some(entry))
            .expect("rename prompt should resolve")
            .expect("rename should open a prompt");

        assert_eq!(request.title(), "Rename");
        assert_eq!(request.initial_buffer(), "demo.txt");
    }

    #[test]
    fn open_with_history_deduplicates_and_caps_entries() {
        let mut history = Vec::new();
        for idx in 0..10 {
            push_open_with_history(&mut history, &format!("cmd-{idx}"));
        }
        push_open_with_history(&mut history, "cmd-4");

        assert_eq!(history.len(), 8);
        assert_eq!(history.first().map(String::as_str), Some("cmd-4"));
        assert_eq!(history.last().map(String::as_str), Some("cmd-2"));
    }

    #[test]
    fn open_with_settings_helpers_update_default_and_history() {
        let mut settings = DesktopFileManagerSettings::default();
        set_open_with_default_in_settings(&mut settings, "txt", Some("nano"));
        set_open_with_default_in_settings(&mut settings, "txt", Some("hx"));

        assert_eq!(
            settings
                .open_with_default_by_extension
                .get("txt")
                .map(String::as_str),
            Some("hx")
        );
        assert_eq!(
            settings
                .open_with_by_extension
                .get("txt")
                .and_then(|history| history.first())
                .map(String::as_str),
            Some("hx")
        );

        replace_open_with_command_in_settings(&mut settings, "txt", "hx", "micro");
        assert_eq!(
            settings
                .open_with_default_by_extension
                .get("txt")
                .map(String::as_str),
            Some("micro")
        );

        remove_open_with_command_in_settings(&mut settings, "txt", "micro");
        assert!(settings.open_with_default_by_extension.get("txt").is_none());
    }

    #[test]
    fn open_with_lookup_helpers_read_saved_values() {
        let mut settings = DesktopFileManagerSettings::default();
        record_open_with_command_in_settings(&mut settings, "txt", "hx");
        set_open_with_default_in_settings(&mut settings, "txt", Some("hx"));

        assert_eq!(
            open_with_history_for_extension(&settings, "txt")
                .first()
                .map(String::as_str),
            Some("hx")
        );
        assert_eq!(
            open_with_default_for_extension(&settings, "txt").as_deref(),
            Some("hx")
        );
    }

    #[test]
    fn settings_update_applies_and_syncs_open_with_values() {
        let mut live = DesktopFileManagerSettings::default();
        let mut draft = DesktopFileManagerSettings::default();

        FileManagerSettingsUpdate::RecordOpenWithCommand {
            ext_key: "txt".to_string(),
            command: "hx".to_string(),
        }
        .apply(&mut live);
        FileManagerSettingsUpdate::SetOpenWithDefaultCommand {
            ext_key: "txt".to_string(),
            command: Some("hx".to_string()),
        }
        .apply(&mut live);
        sync_open_with_settings_to_draft(&live, &mut draft);

        assert_eq!(
            draft
                .open_with_by_extension
                .get("txt")
                .and_then(|history| history.first())
                .map(String::as_str),
            Some("hx")
        );
        assert_eq!(
            draft
                .open_with_default_by_extension
                .get("txt")
                .map(String::as_str),
            Some("hx")
        );
    }

    #[test]
    fn open_with_state_collects_label_history_and_default() {
        let mut settings = DesktopFileManagerSettings::default();
        record_open_with_command_in_settings(&mut settings, "txt", "hx");
        set_open_with_default_in_settings(&mut settings, "txt", Some("hx"));

        let state = open_with_state_for_path(Path::new("/tmp/demo.txt"), &settings);

        assert_eq!(state.ext_key, "txt");
        assert_eq!(state.ext_label, ".txt");
        assert_eq!(state.saved_commands, vec!["hx".to_string()]);
        assert_eq!(state.current_default.as_deref(), Some("hx"));
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
                if label == "Use: hx [default]"
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

    #[test]
    fn prepare_open_with_launch_rejects_invalid_command_line() {
        let path = PathBuf::from("/tmp/demo.txt");
        let err = prepare_open_with_launch(&path, "hx \"unterminated")
            .expect_err("invalid command line should fail");

        assert!(err.to_string().contains("Invalid command line"));
    }

    #[test]
    fn resolve_prompt_outcome_prepares_open_with_launch() {
        let outcome = PromptOutcome::FileManagerOpenWithNewCommand {
            path: PathBuf::from("/tmp/demo.txt"),
            ext_key: "txt".to_string(),
            make_default: false,
            command: "echo".to_string(),
        };

        let resolved = resolve_prompt_outcome(&outcome)
            .expect("file-manager prompt should resolve")
            .expect("valid command should succeed");

        match resolved {
            FileManagerPromptResolution::OpenWithNewCommand {
                command, launch, ..
            } => {
                assert_eq!(command, "echo");
                assert_eq!(
                    launch.argv.last().map(String::as_str),
                    Some("/tmp/demo.txt")
                );
            }
            other => panic!("unexpected resolution: {other:?}"),
        }
    }

    #[test]
    fn selection_activation_prioritizes_pick_mode_for_files() {
        let temp = TempDirGuard::new("selection_activation");
        let file_path = temp.path.join("document.txt");
        std::fs::write(&file_path, "demo").expect("write temp file");

        assert_eq!(
            selection_activation_for_selected_path(
                Some(file_path.clone()),
                FileManagerPickMode::SaveAs,
            ),
            FileManagerSelectionActivation::FillSaveAsName("document.txt".to_string())
        );
        assert_eq!(
            selection_activation_for_selected_path(
                Some(file_path.clone()),
                FileManagerPickMode::ShortcutIcon(3),
            ),
            FileManagerSelectionActivation::PickShortcutIcon {
                shortcut_idx: 3,
                path: file_path.clone(),
            }
        );
        assert_eq!(
            selection_activation_for_selected_path(Some(file_path), FileManagerPickMode::Wallpaper),
            FileManagerSelectionActivation::PickWallpaper(temp.path.join("document.txt"))
        );
    }

    #[test]
    fn open_target_for_file_manager_action_prefers_default_open_with() {
        let mut settings = DesktopFileManagerSettings::default();
        set_open_with_default_in_settings(&mut settings, "txt", Some("echo"));

        let target = open_target_for_file_manager_action(
            FileManagerAction::OpenFile(PathBuf::from("/tmp/demo.txt")),
            &settings,
        )
        .expect("open-with target should resolve");

        match target {
            FileManagerOpenTarget::Launch(launch) => {
                assert_eq!(
                    launch.argv.last().map(String::as_str),
                    Some("/tmp/demo.txt")
                );
            }
            other => panic!("unexpected target: {other:?}"),
        }
    }

    #[test]
    fn open_target_for_file_manager_action_falls_back_to_editor() {
        let target = open_target_for_file_manager_action(
            FileManagerAction::OpenFile(PathBuf::from("/tmp/demo.txt")),
            &DesktopFileManagerSettings::default(),
        )
        .expect("editor fallback should resolve");

        assert_eq!(
            target,
            FileManagerOpenTarget::OpenInEditor(PathBuf::from("/tmp/demo.txt"))
        );
    }

    #[test]
    fn commit_picker_selection_builds_icon_and_wallpaper_results() {
        let icon_entry = FileEntryRow {
            path: PathBuf::from("/tmp/icon.svg"),
            label: "icon.svg".to_string(),
            is_dir: false,
        };
        let wallpaper_entry = FileEntryRow {
            path: PathBuf::from("/tmp/wallpaper.png"),
            label: "wallpaper.png".to_string(),
            is_dir: false,
        };

        assert_eq!(
            commit_picker_selection(Some(icon_entry), FileManagerPickMode::ShortcutIcon(2),),
            Ok(FileManagerPickerCommit::SetShortcutIcon {
                shortcut_idx: 2,
                path: PathBuf::from("/tmp/icon.svg"),
            })
        );
        assert_eq!(
            commit_picker_selection(Some(wallpaper_entry), FileManagerPickMode::Wallpaper),
            Ok(FileManagerPickerCommit::SetWallpaper(PathBuf::from(
                "/tmp/wallpaper.png"
            )))
        );
    }

    #[test]
    fn commit_picker_selection_requires_matching_selection() {
        assert_eq!(
            commit_picker_selection(None, FileManagerPickMode::ShortcutIcon(1)),
            Err("Select an SVG file first.".to_string())
        );
        assert_eq!(
            commit_picker_selection(None, FileManagerPickMode::Wallpaper),
            Err("Select an image file first.".to_string())
        );
    }

    #[test]
    fn run_command_creates_folder_and_reports_status() {
        let temp = TempDirGuard::new("run_command_new_folder");
        std::fs::create_dir_all(temp.path.join("New Folder")).expect("seed existing folder");
        let mut file_manager = NativeFileManagerState::new(temp.path.clone());
        let mut runtime = FileManagerEditRuntime::default();

        let request = run_command(
            FileManagerCommand::NewFolder,
            &mut file_manager,
            &mut runtime,
            &temp.path,
        );

        assert_eq!(
            request,
            FileManagerCommandRequest::ReportStatus("Created New Folder (1)".to_string())
        );
        assert_eq!(
            file_manager.selected,
            Some(temp.path.join("New Folder (1)"))
        );
    }

    #[test]
    fn run_command_requests_prompt_and_display_updates() {
        let temp = TempDirGuard::new("run_command_requests");
        let file_path = temp.path.join("demo.txt");
        std::fs::write(&file_path, "demo").expect("write temp file");
        let mut file_manager = NativeFileManagerState::new(temp.path.clone());
        file_manager.select(Some(file_path.clone()));
        let mut runtime = FileManagerEditRuntime::default();

        assert_eq!(
            run_command(
                FileManagerCommand::Rename,
                &mut file_manager,
                &mut runtime,
                &temp.path,
            ),
            FileManagerCommandRequest::OpenPrompt(FileManagerPromptRequest::Rename {
                path: file_path.clone(),
                label: "demo.txt".to_string(),
            })
        );
        assert_eq!(
            run_command(
                FileManagerCommand::SetViewMode(FileManagerViewMode::List),
                &mut file_manager,
                &mut runtime,
                &temp.path,
            ),
            FileManagerCommandRequest::ApplyDisplaySettings(
                FileManagerDisplaySettingsUpdate::SetViewMode(FileManagerViewMode::List),
            )
        );
    }

    #[test]
    fn selected_file_requires_exactly_one_file_entry() {
        let temp = TempDirGuard::new("selected_file");
        let file_path = temp.path.join("file.txt");
        std::fs::write(&file_path, "demo").expect("write temp file");
        let dir_path = temp.path.join("folder");
        std::fs::create_dir_all(&dir_path).expect("create temp dir");

        let file = FileEntryRow {
            path: file_path,
            label: "file.txt".to_string(),
            is_dir: false,
        };
        let dir = FileEntryRow {
            path: dir_path,
            label: "folder".to_string(),
            is_dir: true,
        };

        assert_eq!(
            selected_file(vec![file.clone()])
                .as_ref()
                .map(|entry| entry.label.as_str()),
            Some("file.txt")
        );
        assert!(selected_file(vec![dir]).is_none());
        assert!(selected_file(vec![file.clone(), file]).is_none());
    }
}
