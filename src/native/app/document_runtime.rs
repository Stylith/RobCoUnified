use super::super::data::{home_dir_fallback, save_text_file, word_processor_dir};
use super::super::desktop_app::DesktopWindow;
use super::super::desktop_file_service::{load_text_document, open_directory_location};
use super::super::desktop_settings_service::{
    apply_file_manager_settings_update as apply_desktop_file_manager_settings_update,
    load_settings_snapshot,
};
use super::super::desktop_shortcuts_service::set_shortcut_icon as set_desktop_shortcut_icon;
use super::super::desktop_surface_service::set_wallpaper_path as set_desktop_wallpaper_path;
use super::super::editor_app::{EditorCommand, EditorWindow};
use super::super::file_manager::{FileEntryRow, FileManagerCommand};
use super::super::file_manager_app::{
    self, FileManagerEditRuntime, FileManagerOpenTarget, FileManagerPickMode,
    FileManagerPickerCommit, FileManagerPromptRequest, FileManagerSelectionActivation,
    FileManagerSettingsUpdate, OpenWithLaunchRequest,
};
use super::super::file_manager_desktop::{
    self, FileManagerDesktopFooterAction, FileManagerDesktopFooterRequest,
};
use super::super::menu::TerminalScreen;
use super::super::prompt::TerminalPromptAction;
use super::super::terminal_open_with_picker::OpenWithPickerState;
use super::RobcoNativeApp;
use crate::default_apps::{resolve_document_open, ResolvedDocumentOpen};
use anyhow::Result;
use eframe::egui::{self, Key};
use std::path::{Path, PathBuf};

impl RobcoNativeApp {
    pub(super) fn apply_file_manager_settings_update(&mut self, update: FileManagerSettingsUpdate) {
        apply_desktop_file_manager_settings_update(&mut self.settings.draft, update);
        self.sync_runtime_settings_cache();
    }

    pub(super) fn launch_open_with_command(
        &mut self,
        path: &Path,
        command_line: &str,
    ) -> Result<String> {
        let launch = file_manager_app::prepare_open_with_launch(path, command_line)?;
        Ok(self.launch_open_with_request(launch))
    }

    pub(super) fn launch_open_with_request(&mut self, launch: OpenWithLaunchRequest) -> String {
        self.launch_shell_command_on_active_surface(
            &launch.title,
            &launch.argv,
            self.terminal_nav.screen,
        );
        launch.status_message
    }

    pub(super) fn file_manager_selected_file(&self) -> Option<FileEntryRow> {
        file_manager_app::selected_file(self.file_manager_selected_entries())
    }

    fn unique_path_in_dir(dir: &Path, original_name: &str) -> PathBuf {
        let direct = dir.join(original_name);
        if !direct.exists() {
            return direct;
        }
        let (stem, ext) = Self::split_file_name(original_name);
        for index in 1..=9999usize {
            let candidate = dir.join(format!("{stem} ({index}){ext}"));
            if !candidate.exists() {
                return candidate;
            }
        }
        direct
    }

    pub(super) fn open_file_manager_at(&mut self, path: PathBuf) {
        if self.desktop_window_is_open(DesktopWindow::FileManager) {
            self.spawn_secondary_window(
                DesktopWindow::FileManager,
                super::SecondaryWindowApp::FileManager {
                    state: super::super::file_manager::NativeFileManagerState::new(path),
                    runtime: FileManagerEditRuntime::default(),
                },
            );
        } else {
            self.open_embedded_file_manager_at(path);
            self.open_desktop_window(DesktopWindow::FileManager);
        }
    }

    pub(super) fn open_embedded_file_manager_at(&mut self, path: PathBuf) {
        match open_directory_location(path) {
            Ok(location) => self.apply_file_manager_location(location),
            Err(status) => self.shell_status = status,
        }
    }

    pub(super) fn open_terminal_open_with_picker(&mut self) {
        let Some(row) =
            file_manager_app::selected_file(self.file_manager.selected_rows_for_action())
        else {
            self.shell_status = "Select a file first.".to_string();
            return;
        };
        let ext_key = file_manager_app::open_with_extension_key(&row.path);
        let settings = load_settings_snapshot();
        let saved_commands =
            robcos_native_services::shared_file_manager_settings::open_with_history_for_extension(
                &settings.desktop_file_manager,
                &ext_key,
            );
        self.terminal_open_with_picker =
            Some(OpenWithPickerState::new(row.path, ext_key, saved_commands));
    }

    pub(super) fn apply_open_with_picker_launch(&mut self, command: String) {
        let Some(picker) = self.terminal_open_with_picker.take() else {
            return;
        };
        match file_manager_app::prepare_open_with_launch(&picker.path, &command) {
            Ok(launch) => {
                let ext_key = picker.ext_key.clone();
                self.shell_status = self.launch_open_with_request(launch);
                self.apply_file_manager_settings_update(
                    FileManagerSettingsUpdate::RecordOpenWithCommand { ext_key, command },
                );
            }
            Err(err) => {
                self.shell_status = format!("Open failed: {err}");
            }
        }
    }

    pub(super) fn apply_open_with_picker_other(&mut self) {
        let Some(picker) = self.terminal_open_with_picker.take() else {
            return;
        };
        self.open_file_manager_prompt(FileManagerPromptRequest::open_with_new_command(
            picker.path,
            picker.ext_key,
            false,
        ));
    }

    fn default_editor_save_name(&self) -> String {
        self.editor
            .path
            .as_ref()
            .and_then(|path| path.file_name())
            .and_then(|name| name.to_str())
            .filter(|name| !name.is_empty())
            .unwrap_or("document.txt")
            .to_string()
    }

    fn editor_save_base_dir(&self) -> PathBuf {
        self.editor
            .path
            .as_ref()
            .and_then(|path| path.parent().map(Path::to_path_buf))
            .or_else(|| {
                self.session
                    .as_ref()
                    .map(|session| word_processor_dir(&session.username))
            })
            .unwrap_or_else(home_dir_fallback)
    }

    fn default_editor_save_target(&self) -> PathBuf {
        self.editor.path.clone().unwrap_or_else(|| {
            self.editor_save_base_dir()
                .join(self.default_editor_save_name())
        })
    }

    fn expand_tilde_path(raw: &str) -> PathBuf {
        if let Some(rest) = raw.strip_prefix('~') {
            return PathBuf::from(format!("{}{}", home_dir_fallback().display(), rest));
        }
        PathBuf::from(raw)
    }

    fn resolve_editor_save_target(&self, raw_path: &str) -> Result<PathBuf, String> {
        let trimmed = raw_path.trim();
        if trimmed.is_empty() {
            return Err("Enter a file path first.".to_string());
        }
        let expanded = Self::expand_tilde_path(trimmed);
        let target = if expanded.is_absolute() {
            expanded
        } else {
            self.editor_save_base_dir().join(expanded)
        };
        let Some(file_name) = target
            .file_name()
            .and_then(|name| name.to_str())
            .filter(|name| !name.trim().is_empty() && *name != "." && *name != "..")
        else {
            return Err("Enter a file path, not just a folder.".to_string());
        };
        if file_name.contains(std::path::MAIN_SEPARATOR) {
            return Err("Enter a valid file path.".to_string());
        }
        Ok(target)
    }

    pub(super) fn open_editor_save_as_picker(&mut self) {
        if self.desktop_mode_open {
            let start_dir = self.editor_save_base_dir();
            self.editor.save_as_input = Some(self.default_editor_save_name());
            self.open_embedded_file_manager_at(start_dir);
            if let Some(path) = self.editor.path.clone() {
                self.file_manager.select(Some(path));
            }
            self.editor.status =
                "Choose a folder in My Computer, enter a file name, then click Save Here."
                    .to_string();
            self.desktop_active_window = Some(
                super::super::desktop_app::WindowInstanceId::primary(DesktopWindow::FileManager),
            );
            return;
        }

        let default_target = self.default_editor_save_target();
        self.open_input_prompt_with_buffer(
            "Save As",
            "Enter file path:",
            default_target.display().to_string(),
            TerminalPromptAction::EditorSaveAsPath,
        );
        self.editor.status = "Enter a file path and press Enter to save.".to_string();
    }

    fn save_editor_as_target(
        &mut self,
        requested_target: PathBuf,
        close_file_manager: bool,
    ) -> bool {
        let Some(file_name) = requested_target.file_name().and_then(|name| name.to_str()) else {
            self.editor.status = "Enter a file path, not just a folder.".to_string();
            return false;
        };
        let target = if let Some(parent) = requested_target.parent() {
            Self::unique_path_in_dir(parent, file_name)
        } else {
            requested_target.clone()
        };
        let renamed_to_avoid_collision = target != requested_target;
        match save_text_file(&target, &self.editor.text) {
            Ok(()) => {
                let label = target
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("document")
                    .to_string();
                self.editor.path = Some(target.clone());
                self.editor.dirty = false;
                self.editor.status = if renamed_to_avoid_collision {
                    format!("Name already existed. Saved as {label}.")
                } else {
                    format!("Saved {label}.")
                };
                let should_close_after_save = self.editor.should_close_after_save();
                self.editor.cancel_close_confirmation();
                self.push_editor_recent_file(&target);
                self.editor.save_as_input = None;
                if close_file_manager {
                    self.file_manager.open = false;
                }
                if should_close_after_save {
                    self.close_current_editor_window_unchecked();
                } else if self.desktop_mode_open {
                    self.open_desktop_window(DesktopWindow::Editor);
                } else {
                    self.editor.open = true;
                }
                true
            }
            Err(err) => {
                self.editor.status = format!("Save failed: {err}");
                false
            }
        }
    }

    pub(super) fn complete_editor_save_as_from_picker(&mut self) {
        let Some(name_draft) = self.editor.save_as_input.clone() else {
            return;
        };
        let file_name = name_draft.trim();
        if file_name.is_empty() {
            self.editor.status = "Enter a file name first.".to_string();
            return;
        }
        let name_path = Path::new(file_name);
        if name_path.file_name().and_then(|name| name.to_str()) != Some(file_name)
            || name_path.components().count() != 1
        {
            self.editor.status =
                "Enter a file name only. Use My Computer to choose the folder.".to_string();
            return;
        }
        let requested_target = self.file_manager.cwd.join(file_name);
        let _ = self.save_editor_as_target(requested_target, true);
    }

    pub(super) fn save_editor_from_prompt_path(&mut self, raw_path: &str) -> bool {
        let target = match self.resolve_editor_save_target(raw_path) {
            Ok(target) => target,
            Err(status) => {
                self.editor.status = status;
                return false;
            }
        };
        self.save_editor_as_target(target, false)
    }

    pub(super) fn open_path_in_editor(&mut self, path: PathBuf) {
        if self.desktop_window_is_open(DesktopWindow::Editor) {
            let mut editor = EditorWindow::default();
            if let Ok(document) = load_text_document(path.clone()) {
                editor.path = Some(document.path.clone());
                editor.text = document.text;
            }
            self.spawn_secondary_window(
                DesktopWindow::Editor,
                super::SecondaryWindowApp::Editor(editor),
            );
        } else {
            self.open_embedded_path_in_editor(path);
        }
    }

    pub(super) fn open_embedded_path_in_editor(&mut self, path: PathBuf) {
        match load_text_document(path.clone()) {
            Ok(document) => {
                self.editor.path = Some(document.path.clone());
                self.editor.text = document.text;
                self.editor.dirty = false;
                self.editor.cancel_close_confirmation();
                self.editor.status = "Opened document.".to_string();
                self.push_editor_recent_file(&document.path);
                self.open_desktop_window(DesktopWindow::Editor);
            }
            Err(status) => {
                self.editor.status = format!("Open failed: {status}");
                self.open_desktop_window(DesktopWindow::Editor);
            }
        }
    }

    pub(super) fn activate_file_manager_selection(&mut self) {
        let settings = load_settings_snapshot();
        match file_manager_app::open_target_for_file_manager_action(
            self.file_manager.activate_selected(),
            &settings.desktop_file_manager,
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

    pub(super) fn open_file_with_default_app_or_editor(&mut self, path: PathBuf) {
        match resolve_document_open(&path) {
            Some(ResolvedDocumentOpen::ExternalArgv(argv)) => {
                let display = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("file")
                    .to_string();
                let title = format!(
                    "{} - {}",
                    robcos_native_file_manager_app::open_with_command_title(&argv[0]),
                    display,
                );
                self.launch_shell_command_on_active_surface(
                    &title,
                    &argv,
                    super::super::menu::TerminalScreen::DocumentBrowser,
                );
                self.shell_status = format!("Opened {display}");
            }
            Some(ResolvedDocumentOpen::BuiltinAddon(addon_id))
                if addon_id.as_str() == "shell.editor" =>
            {
                self.open_path_with_active_shell_editor(path);
            }
            Some(ResolvedDocumentOpen::BuiltinAddon(_)) | None => {
                self.open_path_with_active_shell_editor(path);
            }
        }
    }

    fn open_path_with_active_shell_editor(&mut self, path: PathBuf) {
        self.launch_editor_path_on_active_surface(path, TerminalScreen::DocumentBrowser);
    }

    pub(super) fn file_manager_activate_or_pick(&mut self) {
        let pick_mode = if self.editor.save_as_input.is_some() {
            FileManagerPickMode::SaveAs
        } else if let Some(pick_idx) = self.picking_icon_for_shortcut {
            FileManagerPickMode::ShortcutIcon(pick_idx)
        } else if self.picking_wallpaper {
            FileManagerPickMode::Wallpaper
        } else {
            FileManagerPickMode::None
        };
        match file_manager_app::selection_activation_for_selected_path(
            self.file_manager.selected.clone(),
            pick_mode,
        ) {
            FileManagerSelectionActivation::ActivateSelection => {}
            FileManagerSelectionActivation::FillSaveAsName(name) => {
                self.editor.save_as_input = Some(name);
                self.complete_editor_save_as_from_picker();
                return;
            }
            FileManagerSelectionActivation::PickShortcutIcon { shortcut_idx, path } => {
                self.apply_file_manager_picker_commit(FileManagerPickerCommit::SetShortcutIcon {
                    shortcut_idx,
                    path,
                });
                return;
            }
            FileManagerSelectionActivation::PickWallpaper(path) => {
                self.apply_file_manager_picker_commit(FileManagerPickerCommit::SetWallpaper(path));
                return;
            }
        }
        self.activate_file_manager_selection();
    }

    pub(super) fn new_document(&mut self) {
        if self.desktop_mode_open {
            self.editor.reset_for_desktop_new_document();
            self.open_desktop_window(DesktopWindow::Editor);
            return;
        }
        let Some(session) = &self.session else {
            return;
        };
        let base = word_processor_dir(&session.username);
        let mut path = base.join("document.txt");
        let mut idx = 1usize;
        while path.exists() {
            path = base.join(format!("document-{idx}.txt"));
            idx += 1;
        }
        self.editor.prepare_new_document_at(path);
        self.open_desktop_window(DesktopWindow::Editor);
    }

    pub(super) fn run_editor_command(&mut self, command: EditorCommand) {
        match command {
            EditorCommand::Save => self.save_editor(),
            EditorCommand::SaveAs => self.open_editor_save_as_picker(),
            EditorCommand::NewDocument => self.new_document(),
            EditorCommand::OpenFind => self.editor.ui.open_find(),
            EditorCommand::OpenFindReplace => self.editor.ui.open_find_replace(),
            EditorCommand::CloseFind => self.editor.ui.close_find(),
            EditorCommand::ToggleWordWrap => {
                self.editor.word_wrap = !self.editor.word_wrap;
            }
            EditorCommand::IncreaseFontSize => {
                self.editor.font_size = (self.editor.font_size + 2.0).min(32.0);
            }
            EditorCommand::DecreaseFontSize => {
                self.editor.font_size = (self.editor.font_size - 2.0).max(10.0);
            }
            EditorCommand::ResetFontSize => {
                self.editor.font_size = 16.0;
            }
            EditorCommand::SetTextAlign(alignment) => {
                self.editor.ui.set_text_align(alignment);
            }
            EditorCommand::ToggleLineNumbers => {
                self.editor.ui.toggle_line_numbers();
            }
        }
    }

    pub(super) fn apply_file_manager_picker_commit(&mut self, commit: FileManagerPickerCommit) {
        match commit {
            FileManagerPickerCommit::SetShortcutIcon { shortcut_idx, path } => {
                if let Some(path_str) =
                    set_desktop_shortcut_icon(&mut self.settings.draft, shortcut_idx, &path)
                {
                    self.shortcut_icon_missing.remove(&path_str);
                    if let Some(props) = &mut self.shortcut_properties {
                        if props.shortcut_idx == shortcut_idx {
                            props.icon_path_draft = Some(path_str);
                        }
                    }
                }
                self.picking_icon_for_shortcut = None;
                self.file_manager.open = false;
                self.persist_native_settings();
            }
            FileManagerPickerCommit::SetWallpaper(path) => {
                set_desktop_wallpaper_path(&mut self.settings.draft, &path);
                self.picking_wallpaper = false;
                self.file_manager.open = false;
                self.persist_native_settings();
            }
        }
    }

    fn commit_file_manager_picker(&mut self, pick_mode: FileManagerPickMode) {
        match file_manager_app::commit_picker_selection(
            self.file_manager_selected_file(),
            pick_mode,
        ) {
            Ok(commit) => self.apply_file_manager_picker_commit(commit),
            Err(status) => self.shell_status = status,
        }
    }

    pub(super) fn apply_file_manager_desktop_footer_action(
        &mut self,
        action: FileManagerDesktopFooterAction,
    ) {
        match file_manager_desktop::resolve_footer_action(action) {
            FileManagerDesktopFooterRequest::RunCommand(command) => {
                self.run_file_manager_command(command);
            }
            FileManagerDesktopFooterRequest::NewDocument => self.new_document(),
            FileManagerDesktopFooterRequest::CompleteSaveAs => {
                self.complete_editor_save_as_from_picker()
            }
            FileManagerDesktopFooterRequest::CancelSavePicker => {
                self.editor.save_as_input = None;
                self.editor.status = "Save canceled.".to_string();
                self.file_manager.open = false;
                self.open_desktop_window(DesktopWindow::Editor);
            }
            FileManagerDesktopFooterRequest::CommitIconPicker => {
                let pick_mode = self
                    .picking_icon_for_shortcut
                    .map(FileManagerPickMode::ShortcutIcon)
                    .unwrap_or(FileManagerPickMode::None);
                self.commit_file_manager_picker(pick_mode);
            }
            FileManagerDesktopFooterRequest::CancelIconPicker => {
                self.picking_icon_for_shortcut = None;
            }
            FileManagerDesktopFooterRequest::CommitWallpaperPicker => {
                self.commit_file_manager_picker(FileManagerPickMode::Wallpaper);
            }
            FileManagerDesktopFooterRequest::CancelWallpaperPicker => {
                self.picking_wallpaper = false;
            }
        }
    }

    pub(super) fn handle_desktop_file_manager_shortcuts(&mut self, ctx: &egui::Context) {
        if self.active_window_kind() != Some(DesktopWindow::FileManager)
            || self.terminal_prompt.is_some()
        {
            return;
        }
        if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(Key::C)) {
            self.run_file_manager_command(FileManagerCommand::Copy);
        } else if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(Key::X)) {
            self.run_file_manager_command(FileManagerCommand::Cut);
        } else if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(Key::V)) {
            self.run_file_manager_command(FileManagerCommand::Paste);
        } else if ctx.input(|i| i.modifiers.ctrl && i.modifiers.shift && i.key_pressed(Key::N)) {
            self.run_file_manager_command(FileManagerCommand::NewFolder);
        } else if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(Key::D)) {
            self.run_file_manager_command(FileManagerCommand::Duplicate);
        } else if ctx.input(|i| i.key_pressed(Key::F2)) {
            self.run_file_manager_command(FileManagerCommand::Rename);
        } else if ctx.input(|i| i.modifiers.ctrl && i.modifiers.shift && i.key_pressed(Key::M)) {
            self.run_file_manager_command(FileManagerCommand::Move);
        } else if ctx.input(|i| i.key_pressed(Key::Delete)) {
            self.run_file_manager_command(FileManagerCommand::Delete);
        } else if ctx.input(|i| i.key_pressed(Key::Enter)) {
            self.file_manager_activate_or_pick();
        } else if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(Key::Z)) {
            self.run_file_manager_command(FileManagerCommand::Undo);
        } else if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(Key::Y)) {
            self.run_file_manager_command(FileManagerCommand::Redo);
        }
    }
}
