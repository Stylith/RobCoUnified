use super::super::desktop_surface_service::DesktopSurfaceEntry;
use super::super::file_manager::{FileEntryRow, FileManagerCommand};
use super::super::file_manager_app::{
    self, FileManagerCommandRequest, FileManagerEditRuntime, FileManagerOpenTarget,
    FileManagerPromptRequest,
};
use super::RobcoNativeApp;
use anyhow::Result;
use eframe::egui;
use std::path::{Path, PathBuf};

impl RobcoNativeApp {
    pub(super) fn file_manager_move_paths_to_dir(
        &mut self,
        paths: Vec<PathBuf>,
        target_dir: &Path,
    ) -> Result<String> {
        self.file_manager_runtime
            .move_paths_to_dir(&mut self.file_manager, paths, target_dir)
    }

    pub(super) fn file_manager_drop_allowed(paths: &[PathBuf], target_dir: &Path) -> bool {
        FileManagerEditRuntime::drop_allowed(paths, target_dir)
    }

    pub(super) fn file_manager_handle_drop_to_dir(
        &mut self,
        paths: Vec<PathBuf>,
        target_dir: PathBuf,
    ) {
        self.shell_status = match self.file_manager_move_paths_to_dir(paths, &target_dir) {
            Ok(message) => message,
            Err(err) => format!("File action failed: {err}"),
        };
        let desktop_dir = crate::config::desktop_dir();
        if target_dir == desktop_dir || target_dir.starts_with(&desktop_dir) {
            self.invalidate_desktop_surface_cache();
        }
    }

    pub(super) fn desktop_entry_row(entry: &DesktopSurfaceEntry) -> FileEntryRow {
        FileEntryRow {
            path: entry.path.clone(),
            label: entry.label.clone(),
            is_dir: entry.is_dir(),
        }
    }

    pub(super) fn create_desktop_folder(&mut self) {
        let desktop_dir = crate::config::desktop_dir();
        self.shell_status = match self
            .file_manager_runtime
            .create_folder_in_dir(&desktop_dir, "New Folder")
        {
            Ok(path) => {
                self.invalidate_desktop_surface_cache();
                format!(
                    "Created {} on the desktop.",
                    path.file_name()
                        .and_then(|name| name.to_str())
                        .unwrap_or("folder")
                )
            }
            Err(err) => format!("Desktop folder create failed: {err}"),
        };
    }

    pub(super) fn paste_to_desktop(&mut self) {
        let desktop_dir = crate::config::desktop_dir();
        self.shell_status = match self
            .file_manager_runtime
            .paste_clipboard_into_dir(&desktop_dir)
        {
            Ok((count, last_dst)) => {
                self.invalidate_desktop_surface_cache();
                if count == 1 {
                    format!(
                        "Pasted {} onto the desktop.",
                        last_dst
                            .as_ref()
                            .and_then(|path| path.file_name())
                            .and_then(|name| name.to_str())
                            .unwrap_or("item")
                    )
                } else {
                    format!("Pasted {count} items onto the desktop.")
                }
            }
            Err(err) => format!("Desktop paste failed: {err}"),
        };
    }

    pub(super) fn open_desktop_surface_path(&mut self, path: PathBuf) {
        if path.is_dir() {
            self.open_file_manager_at(path);
            return;
        }
        match file_manager_app::open_target_for_file_manager_action(
            robcos_native_file_manager_app::FileManagerAction::OpenFile(path),
            &self.live_desktop_file_manager_settings,
        ) {
            Ok(FileManagerOpenTarget::NoOp) => {}
            Ok(FileManagerOpenTarget::Launch(launch)) => {
                self.shell_status = self.launch_open_with_request(launch);
            }
            Ok(FileManagerOpenTarget::OpenInEditor(path)) => {
                self.open_file_with_default_app_or_editor(path);
            }
            Err(status) => self.shell_status = status,
        }
    }

    pub(super) fn open_desktop_item_properties(&mut self, path: PathBuf) {
        let name_draft = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("item")
            .to_string();
        self.desktop_selected_icon = Some(super::DesktopIconSelection::Surface(format!(
            "desktop_item:{name_draft}"
        )));
        self.desktop_item_properties = Some(super::DesktopItemPropertiesState {
            is_dir: path.is_dir(),
            path,
            name_draft,
        });
    }

    pub(super) fn rename_desktop_item(&mut self, path: PathBuf) {
        self.open_desktop_item_properties(path);
    }

    pub(super) fn delete_desktop_item(&mut self, path: PathBuf) {
        let label = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("item")
            .to_string();
        let row = FileEntryRow {
            path: path.clone(),
            label,
            is_dir: path.is_dir(),
        };
        self.shell_status = match self.file_manager_runtime.delete_entries(vec![row]) {
            Ok(count) => {
                self.desktop_item_properties = None;
                self.desktop_selected_icon = None;
                self.invalidate_desktop_surface_cache();
                if count == 1 {
                    "Moved desktop item to trash.".to_string()
                } else {
                    format!("Moved {count} desktop items to trash.")
                }
            }
            Err(err) => format!("Desktop delete failed: {err}"),
        };
    }

    pub(super) fn open_desktop_surface_with_prompt(&mut self, path: PathBuf) {
        let ext_key = file_manager_app::open_with_extension_key(&path);
        self.open_file_manager_prompt(FileManagerPromptRequest::open_with_new_command(
            path, ext_key, false,
        ));
    }

    pub(super) fn run_file_manager_command(&mut self, command: FileManagerCommand) {
        let home_path = self.file_manager_home_path();
        match file_manager_app::run_command(
            command,
            &mut self.file_manager,
            &mut self.file_manager_runtime,
            &home_path,
        ) {
            FileManagerCommandRequest::None => {}
            FileManagerCommandRequest::ActivateSelection => {
                self.activate_file_manager_selection();
            }
            FileManagerCommandRequest::OpenPrompt(request) => {
                self.open_file_manager_prompt(request);
            }
            FileManagerCommandRequest::ApplyDisplaySettings(update) => {
                self.apply_file_manager_display_settings_update(update);
            }
            FileManagerCommandRequest::ReportStatus(status) => {
                self.shell_status = status;
            }
        }
    }

    pub(super) fn attach_generic_context_menu(
        action: &mut Option<super::ContextMenuAction>,
        response: &egui::Response,
    ) {
        response.context_menu(|ui| {
            Self::apply_context_menu_style(ui);
            ui.set_min_width(118.0);
            ui.set_max_width(160.0);

            if ui.button("Copy").clicked() {
                *action = Some(super::ContextMenuAction::GenericCopy);
                ui.close_menu();
            }
            if ui.button("Paste").clicked() {
                *action = Some(super::ContextMenuAction::GenericPaste);
                ui.close_menu();
            }

            Self::retro_separator(ui);

            if ui.button("Select All").clicked() {
                *action = Some(super::ContextMenuAction::GenericSelectAll);
                ui.close_menu();
            }
        });
    }

    pub(super) fn draw_editor_save_as_window(&mut self, _ctx: &egui::Context) {}
}
