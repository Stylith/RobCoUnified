use super::file_manager::{FileEntryRow, FileManagerCommand, NativeFileManagerState};
use super::prompt::{TerminalPrompt, TerminalPromptAction, TerminalPromptKind};
use super::prompt_flow::PromptOutcome;
use super::shared_file_manager_settings::FileManagerSettingsUpdate;
use crate::default_apps::parse_custom_command_line;
use anyhow::{anyhow, Result};
use nucleon_native_file_manager_app::{
    prepare_open_with_launch, FileManagerEditRuntime, OpenWithLaunchRequest,
};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileManagerSelectionPromptKind {
    Rename,
    Move,
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
                format!(
                    "Open With {}",
                    nucleon_native_file_manager_app::open_with_extension_label(ext_key)
                )
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
                let ext_label = nucleon_native_file_manager_app::open_with_extension_label(ext_key);
                if *make_default {
                    format!("Open with command for {} (saved as default):", ext_label)
                } else {
                    format!("Open with command for {}:", ext_label)
                }
            }
            Self::OpenWithEditCommand { ext_key, .. } => {
                format!(
                    "Edit saved command for {}:",
                    nucleon_native_file_manager_app::open_with_extension_label(ext_key)
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

pub fn open_with_cleared_default_status(ext_key: &str) -> String {
    format!(
        "Cleared always-use command for {}.",
        nucleon_native_file_manager_app::open_with_extension_label(ext_key)
    )
}

pub fn open_with_set_default_status(command: &str, ext_key: &str) -> String {
    format!(
        "Now always using {} for {}.",
        command,
        nucleon_native_file_manager_app::open_with_extension_label(ext_key)
    )
}

pub fn open_with_removed_saved_status(ext_key: &str) -> String {
    format!(
        "Removed saved command for {}.",
        nucleon_native_file_manager_app::open_with_extension_label(ext_key)
    )
}

pub fn open_with_updated_saved_status(ext_key: &str) -> String {
    format!(
        "Updated saved command for {}.",
        nucleon_native_file_manager_app::open_with_extension_label(ext_key)
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

fn prompt_edit_status(
    entry: Result<FileEntryRow>,
    apply: impl FnOnce(FileEntryRow) -> Result<String>,
) -> String {
    match entry.and_then(apply) {
        Ok(message) => message,
        Err(err) => format!("File action failed: {err}"),
    }
}
