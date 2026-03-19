#![allow(unused_imports, dead_code)]

#[cfg(test)]
use super::file_manager::FileEntryRow;
use super::file_manager::{FileManagerCommand, NativeFileManagerState};
pub use super::file_manager_prompt::{
    apply_prompt_outcome, open_with_cleared_default_status, open_with_removed_saved_status,
    open_with_set_default_status, prompt_request_for_command, FileManagerPromptAction,
    FileManagerPromptRequest,
};
#[cfg(test)]
pub use super::file_manager_prompt::{resolve_prompt_outcome, FileManagerPromptResolution};
#[cfg(test)]
pub use super::shared_file_manager_settings::{
    open_with_default_for_extension, open_with_history_for_extension, push_open_with_history,
    record_open_with_command_in_settings, remove_open_with_command_in_settings,
    replace_open_with_command_in_settings, set_open_with_default_in_settings,
    sync_open_with_settings_to_draft,
};
pub use super::shared_file_manager_settings::{
    FileManagerDisplaySettingsUpdate, FileManagerSettingsUpdate,
};
#[cfg(test)]
use crate::config::{DesktopFileManagerSettings, FileManagerViewMode};
use anyhow::Result;
use std::path::Path;
#[cfg(test)]
use std::path::PathBuf;

#[cfg(test)]
use robcos_native_file_manager_app::FileManagerAction;
pub use robcos_native_file_manager_app::{
    commit_picker_selection, open_target_for_file_manager_action, open_with_extension_key,
    prepare_open_with_launch, selected_file, selection_activation_for_selected_path,
    FileManagerClipboardMode, FileManagerEditRuntime, FileManagerOpenTarget, FileManagerPickMode,
    FileManagerPickerCommit, FileManagerSelectionActivation, NativeFileManagerDragPayload,
    OpenWithLaunchRequest,
};
#[cfg(test)]
pub use robcos_native_file_manager_app::{open_with_state_for_path, FileManagerClipboardItem};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileManagerCommandRequest {
    None,
    ActivateSelection,
    OpenPrompt(FileManagerPromptRequest),
    ApplyDisplaySettings(FileManagerDisplaySettingsUpdate),
    ReportStatus(String),
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

fn command_status_request(result: Result<String>) -> FileManagerCommandRequest {
    match result {
        Ok(message) => FileManagerCommandRequest::ReportStatus(message),
        Err(err) => FileManagerCommandRequest::ReportStatus(format!("File action failed: {err}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native::prompt::TerminalPromptAction;
    use crate::native::prompt_flow::PromptOutcome;

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
