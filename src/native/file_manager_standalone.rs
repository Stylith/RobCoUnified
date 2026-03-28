use super::data::{home_dir_fallback, save_text_file};
use super::desktop_file_service::{
    open_directory_location, reveal_path_location, FileManagerLocation,
};
use super::desktop_settings_service::{
    apply_file_manager_display_settings_update, apply_file_manager_settings_update,
    load_settings_snapshot,
};
use super::file_manager::{FileEntryRow, FileManagerCommand, NativeFileManagerState};
use super::file_manager_app::{
    self, FileManagerCommandRequest, FileManagerEditRuntime, FileManagerOpenTarget,
    FileManagerPromptAction, FileManagerPromptRequest, OpenWithLaunchRequest,
};
use super::file_manager_desktop::{
    self, build_footer_model, resolve_footer_action, FileManagerDesktopFooterAction,
};
use super::prompt_flow::PromptOutcome;
use super::retro_ui::current_palette;
use crate::config::{FileManagerViewMode, Settings};
use eframe::egui::{
    self, Align, Align2, CentralPanel, Color32, ComboBox, Context, Id, Key, Label, Layout,
    RichText, ScrollArea, SelectableLabel, SidePanel, TextEdit, TopBottomPanel, Vec2,
};
use std::path::{Path, PathBuf};
use std::process::Command;

const STANDALONE_FILE_MANAGER_DEFAULT_SIZE: [f32; 2] = [1200.0, 760.0];
const STANDALONE_FILE_MANAGER_MIN_SIZE: [f32; 2] = [760.0, 520.0];

#[derive(Debug, Clone, PartialEq, Eq)]
enum StandaloneContextAction {
    Open,
    OpenWith,
    OpenWithCommand(String),
    Rename,
    Cut,
    Copy,
    Paste,
    NewFolder,
    Duplicate,
    Delete,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum FileManagerDialog {
    Rename {
        path: PathBuf,
        value: String,
    },
    Move {
        path: PathBuf,
        value: String,
    },
    OpenWith {
        path: PathBuf,
        ext_key: String,
        value: String,
        make_default: bool,
        saved_commands: Vec<String>,
        current_default: Option<String>,
    },
}

impl FileManagerDialog {
    fn from_prompt_request(request: FileManagerPromptRequest, settings: &Settings) -> Option<Self> {
        match request {
            FileManagerPromptRequest::Rename { path, label } => {
                Some(Self::Rename { path, value: label })
            }
            FileManagerPromptRequest::Move { path } => Some(Self::Move {
                path,
                value: String::new(),
            }),
            FileManagerPromptRequest::OpenWithNewCommand {
                path,
                ext_key,
                make_default,
            } => {
                let saved_commands = settings
                    .desktop_file_manager
                    .open_with_by_extension
                    .get(&ext_key)
                    .cloned()
                    .unwrap_or_default();
                let current_default = settings
                    .desktop_file_manager
                    .open_with_default_by_extension
                    .get(&ext_key)
                    .cloned();
                Some(Self::OpenWith {
                    path,
                    ext_key,
                    value: current_default.clone().unwrap_or_default(),
                    make_default,
                    saved_commands,
                    current_default,
                })
            }
            FileManagerPromptRequest::OpenWithEditCommand {
                path,
                ext_key,
                previous,
            } => {
                let saved_commands = settings
                    .desktop_file_manager
                    .open_with_by_extension
                    .get(&ext_key)
                    .cloned()
                    .unwrap_or_default();
                let current_default = settings
                    .desktop_file_manager
                    .open_with_default_by_extension
                    .get(&ext_key)
                    .cloned();
                Some(Self::OpenWith {
                    path,
                    ext_key,
                    value: previous,
                    make_default: false,
                    saved_commands,
                    current_default,
                })
            }
        }
    }

    fn title(&self) -> &'static str {
        match self {
            Self::Rename { .. } => "Rename",
            Self::Move { .. } => "Move To",
            Self::OpenWith { .. } => "Open With",
        }
    }

    fn confirm_label(&self) -> &'static str {
        match self {
            Self::Rename { .. } => "Rename",
            Self::Move { .. } => "Move",
            Self::OpenWith { .. } => "Open",
        }
    }

    fn prompt_text(&self) -> String {
        match self {
            Self::Rename { path, .. } => format!(
                "Rename {} to:",
                path.file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("item")
            ),
            Self::Move { .. } => "Move to (folder or full path):".to_string(),
            Self::OpenWith {
                ext_key,
                current_default,
                ..
            } => {
                let ext_label = nucleon_native_file_manager_app::open_with_extension_label(ext_key);
                if let Some(default_command) = current_default {
                    format!("Command for {ext_label} (default: {default_command})")
                } else {
                    format!("Command for {ext_label}")
                }
            }
        }
    }

    fn to_outcome(&self) -> PromptOutcome {
        match self {
            Self::Rename { path, value } => PromptOutcome::FileManagerRename {
                path: path.clone(),
                name: value.clone(),
            },
            Self::Move { path, value } => PromptOutcome::FileManagerMoveTo {
                path: path.clone(),
                destination: value.clone(),
            },
            Self::OpenWith {
                path,
                ext_key,
                value,
                make_default,
                ..
            } => PromptOutcome::FileManagerOpenWithNewCommand {
                path: path.clone(),
                ext_key: ext_key.clone(),
                make_default: *make_default,
                command: value.clone(),
            },
        }
    }
}

pub struct NucleonNativeFileManagerApp {
    file_manager: NativeFileManagerState,
    runtime: FileManagerEditRuntime,
    settings_draft: Settings,
    status: String,
    search_focus_requested: bool,
    pending_context_action: Option<StandaloneContextAction>,
    dialog: Option<FileManagerDialog>,
}

impl Default for NucleonNativeFileManagerApp {
    fn default() -> Self {
        Self::new(None)
    }
}

impl NucleonNativeFileManagerApp {
    pub fn new(requested_path: Option<PathBuf>) -> Self {
        let settings_draft = load_settings_snapshot();
        let start_dir = home_dir_fallback();
        let mut app = Self {
            file_manager: NativeFileManagerState::new(start_dir.clone()),
            runtime: FileManagerEditRuntime::default(),
            settings_draft,
            status: String::new(),
            search_focus_requested: false,
            pending_context_action: None,
            dialog: None,
        };
        match resolve_standalone_start_location(requested_path) {
            Ok(location) => app.apply_location(location),
            Err(status) => {
                app.apply_location(FileManagerLocation {
                    cwd: start_dir,
                    selected: None,
                });
                app.status = status;
            }
        }
        app.file_manager.open = true;
        app
    }

    pub fn default_window_size() -> [f32; 2] {
        STANDALONE_FILE_MANAGER_DEFAULT_SIZE
    }

    pub fn min_window_size() -> [f32; 2] {
        STANDALONE_FILE_MANAGER_MIN_SIZE
    }

    fn apply_location(&mut self, location: FileManagerLocation) {
        self.file_manager.set_cwd(location.cwd);
        if let Some(selected) = location.selected {
            self.file_manager.select(Some(selected));
        }
        self.file_manager.ensure_selection_valid();
    }

    fn sync_settings_snapshot(&mut self) {
        self.settings_draft = load_settings_snapshot();
        self.file_manager.ensure_selection_valid();
    }

    fn selected_entries(&self) -> Vec<FileEntryRow> {
        self.file_manager.selected_rows_for_action()
    }

    fn selected_file(&self) -> Option<FileEntryRow> {
        file_manager_app::selected_file(self.selected_entries())
    }

    fn selection_count(&self) -> usize {
        self.selected_entries().len()
    }

    fn has_editable_selection(&self) -> bool {
        !self.selected_entries().is_empty()
    }

    fn has_single_file_selection(&self) -> bool {
        self.selected_file().is_some()
    }

    fn apply_status(&mut self, status: impl Into<String>) {
        self.status = status.into();
    }

    fn select_path(&mut self, path: PathBuf, ctrl_toggle: bool) {
        if ctrl_toggle {
            self.file_manager.toggle_selected_path(&path);
        } else {
            self.file_manager.select(Some(path));
        }
    }

    fn open_prompt(&mut self, request: FileManagerPromptRequest) {
        self.dialog = FileManagerDialog::from_prompt_request(request, &self.settings_draft);
    }

    fn apply_prompt_outcome(&mut self, outcome: PromptOutcome) {
        let Some(actions) = file_manager_app::apply_prompt_outcome(
            &outcome,
            &mut self.file_manager,
            &mut self.runtime,
        ) else {
            return;
        };
        self.dialog = None;
        for action in actions {
            match action {
                FileManagerPromptAction::Launch(launch) => {
                    self.apply_status(self.launch_open_with_request(launch));
                }
                FileManagerPromptAction::ApplySettingsUpdate(update) => {
                    apply_file_manager_settings_update(&mut self.settings_draft, update);
                    self.sync_settings_snapshot();
                    super::ipc::notify_settings_changed();
                }
                FileManagerPromptAction::ReportStatus(status) => self.apply_status(status),
            }
        }
    }

    fn run_command(&mut self, command: FileManagerCommand) {
        let request = file_manager_app::run_command(
            command,
            &mut self.file_manager,
            &mut self.runtime,
            &home_dir_fallback(),
        );
        self.handle_command_request(request);
    }

    fn handle_command_request(&mut self, request: FileManagerCommandRequest) {
        match request {
            FileManagerCommandRequest::None => {}
            FileManagerCommandRequest::ActivateSelection => self.activate_selection(),
            FileManagerCommandRequest::OpenPrompt(request) => self.open_prompt(request),
            FileManagerCommandRequest::ApplyDisplaySettings(update) => {
                apply_file_manager_display_settings_update(&mut self.settings_draft, update);
                self.sync_settings_snapshot();
                super::ipc::notify_settings_changed();
            }
            FileManagerCommandRequest::ReportStatus(status) => self.apply_status(status),
        }
    }

    fn activate_selection(&mut self) {
        let settings = self.settings_draft.desktop_file_manager.clone();
        match file_manager_app::open_target_for_file_manager_action(
            self.file_manager.activate_selected(),
            &settings,
        ) {
            Ok(FileManagerOpenTarget::NoOp) => {}
            Ok(FileManagerOpenTarget::Launch(launch)) => {
                self.apply_status(self.launch_open_with_request(launch));
            }
            Ok(FileManagerOpenTarget::OpenInEditor(path)) => {
                if super::ipc::shell_is_running() {
                    super::ipc::request_open_in_editor(&path);
                    self.apply_status(format!("Opened {} in editor.", path.display()));
                } else {
                    self.apply_status(
                        open_path_externally(&path)
                            .unwrap_or_else(|err| format!("Open failed: {err}")),
                    );
                }
            }
            Err(status) => self.apply_status(status),
        }
    }

    fn launch_open_with_request(&self, launch: OpenWithLaunchRequest) -> String {
        match launch_argv(&launch.argv) {
            Ok(()) => launch.status_message,
            Err(err) => format!("Open failed: {err}"),
        }
    }

    fn open_with_selected(&mut self) {
        let Some(entry) = self.selected_file() else {
            self.apply_status("Select a file first.");
            return;
        };
        let ext_key = file_manager_app::open_with_extension_key(&entry.path);
        self.open_prompt(FileManagerPromptRequest::open_with_new_command(
            entry.path, ext_key, false,
        ));
    }

    fn launch_open_with_command(&mut self, command: String) {
        let Some(entry) = self.selected_file() else {
            self.apply_status("Select a file first.");
            return;
        };
        let ext_key = file_manager_app::open_with_extension_key(&entry.path);
        match file_manager_app::prepare_open_with_launch(&entry.path, &command) {
            Ok(launch) => {
                self.apply_status(self.launch_open_with_request(launch));
                use nucleon_native_services::shared_file_manager_settings::FileManagerSettingsUpdate;
                apply_file_manager_settings_update(
                    &mut self.settings_draft,
                    FileManagerSettingsUpdate::RecordOpenWithCommand { ext_key, command },
                );
                self.sync_settings_snapshot();
                super::ipc::notify_settings_changed();
            }
            Err(err) => {
                self.apply_status(format!("Open failed: {err}"));
            }
        }
    }

    fn create_new_document(&mut self) {
        let path = unique_path_in_dir(&self.file_manager.cwd, "New Document.txt");
        match save_text_file(&path, "") {
            Ok(()) => {
                self.file_manager.refresh_contents();
                self.file_manager.select(Some(path.clone()));
                self.apply_status(format!(
                    "Created {}",
                    path.file_name()
                        .and_then(|name| name.to_str())
                        .unwrap_or("document")
                ));
            }
            Err(err) => self.apply_status(format!("Create failed: {err}")),
        }
    }

    fn apply_footer_action(&mut self, action: FileManagerDesktopFooterAction) {
        match resolve_footer_action(action) {
            file_manager_desktop::FileManagerDesktopFooterRequest::RunCommand(command) => {
                self.run_command(command);
            }
            file_manager_desktop::FileManagerDesktopFooterRequest::NewDocument => {
                self.create_new_document();
            }
            file_manager_desktop::FileManagerDesktopFooterRequest::CompleteSaveAs
            | file_manager_desktop::FileManagerDesktopFooterRequest::CancelSavePicker
            | file_manager_desktop::FileManagerDesktopFooterRequest::CommitIconPicker
            | file_manager_desktop::FileManagerDesktopFooterRequest::CancelIconPicker
            | file_manager_desktop::FileManagerDesktopFooterRequest::CommitWallpaperPicker
            | file_manager_desktop::FileManagerDesktopFooterRequest::CancelWallpaperPicker
            | file_manager_desktop::FileManagerDesktopFooterRequest::CommitThemeImportPicker
            | file_manager_desktop::FileManagerDesktopFooterRequest::CancelThemeImportPicker => {}
        }
    }

    fn apply_context_action(&mut self, action: StandaloneContextAction) {
        match action {
            StandaloneContextAction::Open => self.run_command(FileManagerCommand::OpenSelected),
            StandaloneContextAction::OpenWith => self.open_with_selected(),
            StandaloneContextAction::OpenWithCommand(command) => {
                self.launch_open_with_command(command);
            }
            StandaloneContextAction::Rename => self.run_command(FileManagerCommand::Rename),
            StandaloneContextAction::Cut => self.run_command(FileManagerCommand::Cut),
            StandaloneContextAction::Copy => self.run_command(FileManagerCommand::Copy),
            StandaloneContextAction::Paste => self.run_command(FileManagerCommand::Paste),
            StandaloneContextAction::NewFolder => self.run_command(FileManagerCommand::NewFolder),
            StandaloneContextAction::Duplicate => self.run_command(FileManagerCommand::Duplicate),
            StandaloneContextAction::Delete => self.run_command(FileManagerCommand::Delete),
        }
    }

    fn handle_keyboard_shortcuts(&mut self, ctx: &Context) {
        if self.dialog.is_some() {
            return;
        }
        if ctx.input(|input| input.modifiers.command && input.key_pressed(Key::F)) {
            self.search_focus_requested = true;
        }
        if ctx.input(|input| input.modifiers.command && input.key_pressed(Key::T)) {
            self.run_command(FileManagerCommand::NewTab);
        }
        if ctx.input(|input| input.modifiers.command && input.key_pressed(Key::W)) {
            self.file_manager.close_active_tab();
        }
        if ctx.input(|input| input.modifiers.command && input.key_pressed(Key::C)) {
            self.run_command(FileManagerCommand::Copy);
        }
        if ctx.input(|input| input.modifiers.command && input.key_pressed(Key::X)) {
            self.run_command(FileManagerCommand::Cut);
        }
        if ctx.input(|input| input.modifiers.command && input.key_pressed(Key::V)) {
            self.run_command(FileManagerCommand::Paste);
        }
        if ctx.input(|input| input.modifiers.command && input.key_pressed(Key::D)) {
            self.run_command(FileManagerCommand::Duplicate);
        }
        if ctx.input(|input| input.key_pressed(Key::Enter)) {
            self.run_command(FileManagerCommand::OpenSelected);
        }
        if ctx.input(|input| input.key_pressed(Key::Backspace)) {
            self.run_command(FileManagerCommand::GoUp);
        }
        if ctx.input(|input| input.key_pressed(Key::Delete)) {
            self.run_command(FileManagerCommand::Delete);
        }
        if ctx.input(|input| input.key_pressed(Key::F2)) {
            self.run_command(FileManagerCommand::Rename);
        }
    }

    fn attach_context_menu(
        &mut self,
        response: &egui::Response,
        has_selection: bool,
        has_file_selection: bool,
        has_clipboard: bool,
    ) {
        response.context_menu(|ui| {
            if ui
                .add_enabled(has_selection, egui::Button::new("Open"))
                .clicked()
            {
                self.pending_context_action = Some(StandaloneContextAction::Open);
                ui.close_menu();
            }
            if has_file_selection {
                ui.menu_button("Open With", |ui| {
                    if let Some(entry) = self.selected_file() {
                        let ext_key = file_manager_app::open_with_extension_key(&entry.path);
                        let known_apps =
                            nucleon_native_file_manager_app::known_apps_for_extension(&ext_key);
                        let open_with = nucleon_native_file_manager_app::open_with_state_for_path(
                            &entry.path,
                            &self.settings_draft.desktop_file_manager,
                        );
                        let mut known_commands: std::collections::HashSet<String> =
                            std::collections::HashSet::new();

                        for app in &known_apps {
                            known_commands.insert(app.command.clone());
                            if ui.button(app.label.as_str()).clicked() {
                                self.pending_context_action = Some(
                                    StandaloneContextAction::OpenWithCommand(app.command.clone()),
                                );
                                ui.close_menu();
                            }
                        }

                        let has_saved = open_with
                            .saved_commands
                            .iter()
                            .any(|c| !known_commands.contains(c));
                        if !known_apps.is_empty() && has_saved {
                            ui.separator();
                        }
                        for command in &open_with.saved_commands {
                            if known_commands.contains(command) {
                                continue;
                            }
                            if ui.button(command.as_str()).clicked() {
                                self.pending_context_action =
                                    Some(StandaloneContextAction::OpenWithCommand(command.clone()));
                                ui.close_menu();
                            }
                        }

                        if !known_apps.is_empty() || !open_with.saved_commands.is_empty() {
                            ui.separator();
                        }

                        if ui.button("Other...").clicked() {
                            self.pending_context_action = Some(StandaloneContextAction::OpenWith);
                            ui.close_menu();
                        }
                    }
                });
            }
            ui.separator();
            if ui
                .add_enabled(has_selection, egui::Button::new("Rename"))
                .clicked()
            {
                self.pending_context_action = Some(StandaloneContextAction::Rename);
                ui.close_menu();
            }
            ui.separator();
            if ui
                .add_enabled(has_selection, egui::Button::new("Cut"))
                .clicked()
            {
                self.pending_context_action = Some(StandaloneContextAction::Cut);
                ui.close_menu();
            }
            if ui
                .add_enabled(has_selection, egui::Button::new("Copy"))
                .clicked()
            {
                self.pending_context_action = Some(StandaloneContextAction::Copy);
                ui.close_menu();
            }
            if ui
                .add_enabled(has_clipboard, egui::Button::new("Paste"))
                .clicked()
            {
                self.pending_context_action = Some(StandaloneContextAction::Paste);
                ui.close_menu();
            }
            if ui.button("New Folder").clicked() {
                self.pending_context_action = Some(StandaloneContextAction::NewFolder);
                ui.close_menu();
            }
            if ui
                .add_enabled(has_selection, egui::Button::new("Duplicate"))
                .clicked()
            {
                self.pending_context_action = Some(StandaloneContextAction::Duplicate);
                ui.close_menu();
            }
            ui.separator();
            if ui
                .add_enabled(has_selection, egui::Button::new("Delete"))
                .clicked()
            {
                self.pending_context_action = Some(StandaloneContextAction::Delete);
                ui.close_menu();
            }
        });
    }

    fn draw_top_panel(
        &mut self,
        ctx: &Context,
        model: &file_manager_desktop::FileManagerDesktopViewModel,
    ) {
        let palette = current_palette();
        let search_id = Id::new("standalone_file_manager_search");

        TopBottomPanel::top("standalone_file_manager_top")
            .frame(egui::Frame::none().fill(palette.bg))
            .show(ctx, |ui| {
                ui.add_space(4.0);
                ui.horizontal_wrapped(|ui| {
                    let mut switch_to_tab = None;
                    let mut close_tab = None;
                    for (idx, tab) in model.tabs.iter().enumerate() {
                        let title = truncate_label(&tab.title, 12);
                        let response = ui.selectable_label(
                            tab.active,
                            RichText::new(format!(
                                "[{}:{}{}]",
                                idx + 1,
                                title,
                                if tab.active { "*" } else { "" }
                            ))
                            .monospace(),
                        );
                        if response.clicked() {
                            switch_to_tab = Some(idx);
                        }
                        if model.close_tab_enabled() && ui.button("x").clicked() {
                            close_tab = Some(idx);
                        }
                    }
                    if ui.button("+").clicked() {
                        self.run_command(FileManagerCommand::NewTab);
                    }
                    if let Some(idx) = close_tab {
                        self.file_manager.close_tab(idx);
                    } else if let Some(idx) = switch_to_tab {
                        let _ = self.file_manager.switch_to_tab(idx);
                    }
                });
                ui.add_space(4.0);

                ui.horizontal(|ui| {
                    ComboBox::from_id_salt("standalone_file_manager_drive_picker")
                        .width(160.0)
                        .selected_text(model.current_drive_label.as_deref().unwrap_or("Drive"))
                        .show_ui(ui, |ui| {
                            for drive in &model.drives {
                                if ui.selectable_label(drive.active, &drive.label).clicked() {
                                    self.file_manager
                                        .open_selected_tree_path(drive.path.clone());
                                    ui.close_menu();
                                }
                            }
                        });

                    let mut search_query = model.search_query.clone();
                    if self.search_focus_requested {
                        ui.memory_mut(|memory| memory.request_focus(search_id));
                        self.search_focus_requested = false;
                    }
                    let search_width = (ui.available_width() - 216.0).max(200.0);
                    let response = ui.add_sized(
                        [search_width, 28.0],
                        TextEdit::singleline(&mut search_query)
                            .id(search_id)
                            .hint_text("filter files and folders"),
                    );
                    if response.changed() {
                        self.file_manager.update_search_query(search_query);
                    }
                });
                ui.add_space(4.0);

                ui.horizontal(|ui| {
                    if ui.selectable_label(model.show_tree_panel, "Tree").clicked() {
                        self.run_command(FileManagerCommand::ToggleTreePanel);
                    }
                    if ui
                        .selectable_label(model.view_mode == FileManagerViewMode::List, "List")
                        .clicked()
                    {
                        self.run_command(FileManagerCommand::SetViewMode(
                            FileManagerViewMode::List,
                        ));
                    }
                    if ui
                        .selectable_label(model.view_mode == FileManagerViewMode::Grid, "Grid")
                        .clicked()
                    {
                        self.run_command(FileManagerCommand::SetViewMode(
                            FileManagerViewMode::Grid,
                        ));
                    }
                });
                ui.add_space(4.0);

                ui.horizontal(|ui| {
                    ui.label(RichText::new("Location:").strong().color(palette.fg));
                    ui.add(
                        Label::new(
                            RichText::new(&model.path_label)
                                .monospace()
                                .color(palette.fg),
                        )
                        .truncate(),
                    );
                });
                ui.add_space(4.0);
            });
    }

    fn draw_footer(
        &mut self,
        ctx: &Context,
        footer_model: &file_manager_desktop::FileManagerDesktopFooterModel,
    ) {
        TopBottomPanel::bottom("standalone_file_manager_bottom").show(ctx, |ui| {
            ui.horizontal(|ui| {
                for (idx, item) in footer_model.status_items.iter().enumerate() {
                    if idx > 0 {
                        ui.separator();
                    }
                    ui.label(item);
                }
                if !self.status.is_empty() {
                    ui.separator();
                    ui.label(RichText::new(&self.status).italics());
                }
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    for button in &footer_model.trailing_buttons {
                        if ui.button(button.label).clicked() {
                            self.apply_footer_action(button.action);
                        }
                    }
                });
            });
        });
    }

    fn draw_tree_panel(
        &mut self,
        ctx: &Context,
        model: &file_manager_desktop::FileManagerDesktopViewModel,
    ) {
        if !model.show_tree_panel {
            return;
        }
        SidePanel::left("standalone_file_manager_tree")
            .resizable(true)
            .width_range(160.0..=320.0)
            .default_width(220.0)
            .show(ctx, |ui| {
                ScrollArea::vertical().show(ui, |ui| {
                    for item in &model.tree_items {
                        if item.path.is_none() {
                            let line = item.line.trim();
                            if line.is_empty() {
                                continue;
                            }
                            ui.add_space(4.0);
                            ui.label(RichText::new(line).strong());
                            continue;
                        }
                        let Some(path) = item.path.as_ref() else {
                            continue;
                        };
                        let selected = Some(path) == self.file_manager.tree_selected.as_ref();
                        if ui
                            .selectable_label(selected, RichText::new(&item.line).monospace())
                            .clicked()
                        {
                            self.file_manager.open_selected_tree_path(path.clone());
                        }
                    }
                });
            });
    }

    fn draw_content_panel(
        &mut self,
        ctx: &Context,
        model: &file_manager_desktop::FileManagerDesktopViewModel,
    ) {
        let palette = current_palette();
        let has_editable_selection = model.status.has_editable_selection;
        let has_single_file_selection = model.status.has_single_file_selection;
        let has_clipboard = model.status.has_clipboard;

        CentralPanel::default()
            .frame(egui::Frame::none().fill(palette.bg))
            .show(ctx, |ui| {
                if model.rows.is_empty() {
                    ui.label("No files match the current search.");
                    return;
                }

                match model.view_mode {
                    FileManagerViewMode::List => {
                        ScrollArea::vertical().show(ui, |ui| {
                            for row in &model.rows {
                                let selected = self.file_manager.is_path_selected(&row.path);
                                let response = ui.add(SelectableLabel::new(
                                    selected,
                                    RichText::new(format!("{} {}", row.icon(), row.label))
                                        .monospace()
                                        .color(if selected { Color32::BLACK } else { palette.fg }),
                                ));
                                self.handle_row_interaction(
                                    ctx,
                                    &response,
                                    row,
                                    has_editable_selection,
                                    has_single_file_selection,
                                    has_clipboard,
                                );
                            }
                            let background = ui.allocate_rect(
                                ui.available_rect_before_wrap(),
                                egui::Sense::click(),
                            );
                            if background.clicked() {
                                self.file_manager.clear_multi_selection();
                            }
                            self.attach_context_menu(
                                &background,
                                has_editable_selection,
                                has_single_file_selection,
                                has_clipboard,
                            );
                        });
                    }
                    FileManagerViewMode::Grid => {
                        let tile_width = 150.0;
                        let available_width = ui.available_width();
                        let columns = model.grid_columns(available_width, tile_width);
                        ScrollArea::vertical().show(ui, |ui| {
                            for chunk in model.rows.chunks(columns) {
                                ui.allocate_ui_with_layout(
                                    Vec2::new(available_width, 72.0),
                                    Layout::left_to_right(Align::Min),
                                    |ui| {
                                        for row in chunk {
                                            let selected =
                                                self.file_manager.is_path_selected(&row.path);
                                            let response = ui.add_sized(
                                                [tile_width - 8.0, 64.0],
                                                SelectableLabel::new(
                                                    selected,
                                                    RichText::new(format!(
                                                        "{}\n{}",
                                                        row.icon(),
                                                        truncate_label(&row.label, 16)
                                                    ))
                                                    .monospace()
                                                    .color(if selected {
                                                        Color32::BLACK
                                                    } else {
                                                        palette.fg
                                                    }),
                                                ),
                                            );
                                            self.handle_row_interaction(
                                                ctx,
                                                &response,
                                                row,
                                                has_editable_selection,
                                                has_single_file_selection,
                                                has_clipboard,
                                            );
                                        }
                                    },
                                );
                            }
                            let background = ui.allocate_rect(
                                ui.available_rect_before_wrap(),
                                egui::Sense::click(),
                            );
                            if background.clicked() {
                                self.file_manager.clear_multi_selection();
                            }
                            self.attach_context_menu(
                                &background,
                                has_editable_selection,
                                has_single_file_selection,
                                has_clipboard,
                            );
                        });
                    }
                }
            });
    }

    fn handle_row_interaction(
        &mut self,
        ctx: &Context,
        response: &egui::Response,
        row: &FileEntryRow,
        has_editable_selection: bool,
        has_single_file_selection: bool,
        has_clipboard: bool,
    ) {
        if response.secondary_clicked() && !self.file_manager.is_path_selected(&row.path) {
            self.file_manager.select(Some(row.path.clone()));
        }
        if response.clicked() {
            let ctrl_toggle = ctx.input(|input| input.modifiers.ctrl);
            self.select_path(row.path.clone(), ctrl_toggle);
        }
        if response.double_clicked() {
            self.file_manager.select(Some(row.path.clone()));
            self.activate_selection();
        }
        self.attach_context_menu(
            response,
            has_editable_selection,
            has_single_file_selection,
            has_clipboard,
        );
    }

    fn draw_dialog(&mut self, ctx: &Context) {
        let Some(mut dialog) = self.dialog.clone() else {
            return;
        };

        let mut apply = false;
        let mut close = false;

        egui::Window::new(dialog.title())
            .anchor(Align2::CENTER_CENTER, [0.0, 0.0])
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.set_min_width(460.0);
                ui.label(dialog.prompt_text());
                ui.add_space(8.0);

                match &mut dialog {
                    FileManagerDialog::Rename { value, .. }
                    | FileManagerDialog::Move { value, .. } => {
                        ui.add_sized([420.0, 28.0], TextEdit::singleline(value));
                    }
                    FileManagerDialog::OpenWith {
                        value,
                        make_default,
                        saved_commands,
                        current_default,
                        ..
                    } => {
                        ui.add_sized([420.0, 28.0], TextEdit::singleline(value));
                        if !saved_commands.is_empty() {
                            ui.add_space(8.0);
                            ui.label("Saved commands:");
                            ui.horizontal_wrapped(|ui| {
                                for command in saved_commands {
                                    if ui.button(command.as_str()).clicked() {
                                        *value = command.clone();
                                    }
                                }
                            });
                        }
                        if let Some(current_default) = current_default {
                            ui.add_space(6.0);
                            ui.label(format!("Current default: {current_default}"));
                        }
                        ui.add_space(8.0);
                        ui.checkbox(make_default, "Always use this command for this file type");
                    }
                }

                ui.add_space(12.0);
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui.button("Cancel").clicked() {
                        close = true;
                    }
                    if ui.button(dialog.confirm_label()).clicked() {
                        apply = true;
                    }
                });
            });

        if ctx.input(|input| input.key_pressed(Key::Escape)) {
            close = true;
        }
        if ctx.input(|input| input.key_pressed(Key::Enter)) {
            apply = true;
        }

        if close {
            self.dialog = None;
        } else if apply {
            self.apply_prompt_outcome(dialog.to_outcome());
        } else {
            self.dialog = Some(dialog);
        }
    }
}

impl eframe::App for NucleonNativeFileManagerApp {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        self.file_manager.ensure_selection_valid();
        self.handle_keyboard_shortcuts(ctx);

        let rows = self.file_manager.rows();
        let settings = self.settings_draft.desktop_file_manager.clone();
        let desktop_model = file_manager_desktop::build_desktop_view_model(
            &self.file_manager,
            &settings,
            &rows,
            self.selection_count(),
            self.has_editable_selection(),
            self.has_single_file_selection(),
            self.runtime.has_clipboard(),
            None,
            None,
            false,
            false,
        );
        let footer_model = build_footer_model(&desktop_model);

        self.draw_top_panel(ctx, &desktop_model);
        self.draw_footer(ctx, &footer_model);
        self.draw_tree_panel(ctx, &desktop_model);
        self.draw_content_panel(ctx, &desktop_model);
        self.draw_dialog(ctx);

        if let Some(action) = self.pending_context_action.take() {
            self.apply_context_action(action);
        }

        ctx.request_repaint_after(std::time::Duration::from_millis(500));
    }
}

fn resolve_standalone_start_location(
    requested_path: Option<PathBuf>,
) -> Result<FileManagerLocation, String> {
    match requested_path {
        Some(path) => reveal_path_location(path),
        None => open_directory_location(home_dir_fallback()),
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

fn split_file_name(name: &str) -> (&str, &str) {
    if let Some((stem, _ext)) = name.rsplit_once('.') {
        if !stem.is_empty() {
            return (stem, &name[stem.len()..]);
        }
    }
    (name, "")
}

fn truncate_label(text: &str, max_chars: usize) -> String {
    let total_chars = text.chars().count();
    if total_chars <= max_chars {
        return text.to_string();
    }
    if max_chars <= 3 {
        return ".".repeat(max_chars);
    }
    let suffix_budget = ((max_chars - 3) + 1) / 2;
    let mut suffix: String = text
        .chars()
        .skip(total_chars.saturating_sub(suffix_budget))
        .collect();
    if total_chars > suffix_budget && suffix.starts_with('.') {
        suffix.remove(0);
    }
    let prefix_budget = max_chars.saturating_sub(3 + suffix.chars().count());
    let prefix: String = text.chars().take(prefix_budget).collect();
    format!("{prefix}...{suffix}")
}

fn launch_argv(argv: &[String]) -> Result<(), String> {
    let Some(program) = argv.first() else {
        return Err("No command configured.".to_string());
    };
    Command::new(program)
        .args(&argv[1..])
        .spawn()
        .map(|_| ())
        .map_err(|err| err.to_string())
}

fn open_path_externally(path: &Path) -> Result<String, String> {
    #[cfg(target_os = "macos")]
    let mut command = {
        let mut command = Command::new("open");
        command.arg(path);
        command
    };

    #[cfg(target_os = "linux")]
    let mut command = {
        let mut command = Command::new("xdg-open");
        command.arg(path);
        command
    };

    #[cfg(target_os = "windows")]
    let mut command = {
        let mut command = Command::new("cmd");
        command.args(["/C", "start", "", &path.display().to_string()]);
        command
    };

    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    let mut command = {
        let mut command = Command::new("xdg-open");
        command.arg(path);
        command
    };

    command
        .spawn()
        .map_err(|err| err.to_string())
        .map(|_| format!("Opened {}.", path.display()))
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
                "nucleon_native_file_manager_standalone_{prefix}_{}_{}",
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
    fn resolve_standalone_start_location_defaults_to_home() {
        let location = resolve_standalone_start_location(None).expect("default location");

        assert_eq!(location.selected, None);
        assert!(location.cwd.is_dir());
    }

    #[test]
    fn resolve_standalone_start_location_reveals_file_argument() {
        let temp = TempDirGuard::new("reveal");
        let file = temp.path.join("demo.txt");
        std::fs::write(&file, "hello").expect("write temp file");

        let location =
            resolve_standalone_start_location(Some(file.clone())).expect("reveal location");

        assert_eq!(location.cwd, temp.path);
        assert_eq!(location.selected, Some(file));
    }

    #[test]
    fn unique_path_in_dir_appends_numeric_suffix() {
        let temp = TempDirGuard::new("unique");
        let first = temp.path.join("New Document.txt");
        std::fs::write(&first, "").expect("write first file");

        let unique = unique_path_in_dir(&temp.path, "New Document.txt");

        assert_eq!(
            unique.file_name().and_then(|name| name.to_str()),
            Some("New Document (1).txt")
        );
    }
}
