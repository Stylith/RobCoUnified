use super::super::about_screen::{draw_about_screen, TerminalAboutRequest};
use super::super::command_layer::CommandLayerTarget;
use super::super::connections_screen::{
    draw_terminal_connections_screen, TerminalConnectionsRequest,
};
use super::super::default_apps_screen::{draw_default_apps_screen, TerminalDefaultAppsRequest};
use super::super::desktop_default_apps_service::apply_default_app_binding;
use super::super::desktop_documents_service::document_category_path;
use super::super::desktop_launcher_service::{catalog_names, ProgramCatalog};
use super::super::desktop_session_service::session_tabs as native_session_tabs;
use super::super::desktop_settings_service::cycle_hacking_difficulty_in_settings;
use super::super::desktop_status_service::{clear_shell_status, saved_shell_status};
use super::super::desktop_user_service::{
    create_user as create_desktop_user, update_user_auth_method,
};
use super::super::document_browser::{
    activate_browser_selection, draw_terminal_document_browser, sync_browser_selection,
    DocumentBrowserEvent, TerminalDocumentBrowserRequest,
};
use super::super::edit_menus_screen::{
    draw_edit_menus_screen, EditMenuTarget, EditMenusEntries, TerminalEditMenusRequest,
};
use super::super::file_manager::FileManagerCommand;
use super::super::installer_screen::{
    draw_installer_screen, settle_view_after_package_command, InstallerEvent,
    InstallerPackageAction,
};
use super::super::menu::{
    draw_terminal_menu_screen, handle_user_management_selection, plan_user_management_action,
    resolve_embedded_pty_exit, resolve_main_menu_action, terminal_screen_open_plan,
    terminal_settings_refresh_plan, user_management_screen_for_mode, TerminalScreen,
    UserManagementExecutionPlan, UserManagementMode,
};
use super::super::programs_screen::{draw_programs_menu, ProgramMenuEvent};
use super::super::prompt::{draw_terminal_prompt_overlay, FlashAction, TerminalPromptAction};
use super::super::pty_screen::{draw_embedded_pty, PtyScreenEvent};
use super::super::retro_ui::{current_palette_for_surface, RetroScreen, ShellSurfaceKind};
use super::super::settings_screen::{run_terminal_settings_screen, TerminalSettingsEvent};
use super::super::shell_screen::draw_main_menu_screen;
use super::super::terminal_open_with_picker::{draw_open_with_picker, OpenWithPickerAction};
use super::super::wasm_addon_runtime::{collect_hosted_keyboard_input, draw_hosted_addon_frame};
use super::launch_registry::{editor_launch_target, file_manager_launch_target};
use super::NucleonNativeApp;
use super::BUILTIN_TEXT_EDITOR_APP;
use crate::native::{installed_hosted_application_names, installed_hosted_game_names};
use crate::theme::TerminalStatusBarPosition;
use chrono::{Local, Timelike};
use eframe::egui::{self, Color32, Context, Id, Layout, RichText, Stroke, TopBottomPanel};
use nucleon_native_programs_app::{
    build_terminal_application_entries, build_terminal_game_entries,
    resolve_terminal_applications_request, resolve_terminal_catalog_request,
    resolve_terminal_games_request, DesktopProgramRequest, TerminalProgramRequest,
};
use nucleon_native_settings_app::TerminalSettingsPanel;
use nucleon_shared::platform::{HostedAddonSize, LaunchTarget};
use std::time::Duration;

impl NucleonNativeApp {
    fn draw_terminal_game_shell<F>(ctx: &Context, title: &str, controls: &str, draw_game: F)
    where
        F: FnOnce(&mut egui::Ui),
    {
        let palette = current_palette_for_surface(ShellSurfaceKind::Terminal);
        egui::CentralPanel::default()
            .frame(
                egui::Frame::none()
                    .fill(palette.bg)
                    .inner_margin(egui::Margin::symmetric(16.0, 12.0)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(title).monospace().strong().color(palette.fg));
                    ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            RichText::new("TAB BACK")
                                .monospace()
                                .small()
                                .color(palette.dim),
                        );
                    });
                });
                ui.add_space(4.0);
                ui.label(
                    RichText::new(controls)
                        .monospace()
                        .small()
                        .color(palette.dim),
                );
                ui.add_space(10.0);
                egui::Frame::none()
                    .stroke(Stroke::new(1.0, palette.dim))
                    .inner_margin(egui::Margin::same(10.0))
                    .show(ui, |ui| {
                        ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
                        draw_game(ui);
                    });
            });
    }

    pub(super) fn draw_terminal_main_menu(&mut self, ctx: &Context) {
        let layout = self.terminal_layout();
        let header_lines = self.active_terminal_header_lines().to_vec();
        let activated = draw_main_menu_screen(
            ctx,
            &mut self.terminal_nav.main_menu_idx,
            &self.shell_status,
            &format!("NucleonOS v{}", env!("CARGO_PKG_VERSION")),
            layout.cols,
            layout.rows,
            layout.header_start_row,
            layout.separator_top_row,
            layout.title_row,
            layout.separator_bottom_row,
            layout.subtitle_row,
            layout.menu_start_row,
            layout.status_row,
            layout.content_col,
            &header_lines,
        );
        if let Some(action) = activated {
            let action = resolve_main_menu_action(action);
            self.apply_main_menu_selection_action(action);
        }
    }

    pub(super) fn draw_terminal_applications(&mut self, ctx: &Context) {
        let layout = self.terminal_layout();
        let header_lines = self.active_terminal_header_lines().to_vec();
        let (show_file_manager, show_text_editor) = self.visible_application_builtins();
        let mut configured_names = catalog_names(ProgramCatalog::Applications);
        for name in installed_hosted_application_names() {
            if !configured_names.iter().any(|existing| existing == &name) {
                configured_names.push(name);
            }
        }
        configured_names.sort();
        let entries = build_terminal_application_entries(
            show_file_manager,
            show_text_editor,
            &configured_names,
            BUILTIN_TEXT_EDITOR_APP,
        );
        let event = draw_programs_menu(
            ctx,
            "Applications",
            Some("Built-in and configured apps"),
            &entries,
            &mut self.terminal_nav.apps_idx,
            &self.shell_status,
            layout.cols,
            layout.rows,
            layout.header_start_row,
            layout.separator_top_row,
            layout.title_row,
            layout.separator_bottom_row,
            layout.subtitle_row,
            layout.menu_start_row,
            layout.status_row,
            layout.content_col,
            &header_lines,
        );
        let request = resolve_terminal_applications_request(event, BUILTIN_TEXT_EDITOR_APP);
        self.apply_terminal_program_request(request, TerminalScreen::Applications);
    }

    pub(super) fn apply_terminal_program_request(
        &mut self,
        request: TerminalProgramRequest,
        launch_return_screen: TerminalScreen,
    ) {
        match request {
            TerminalProgramRequest::None => {}
            TerminalProgramRequest::BackToMainMenu => {
                self.navigate_to_screen(TerminalScreen::MainMenu);
                self.apply_status_update(clear_shell_status());
            }
            TerminalProgramRequest::OpenTextEditor => {
                self.execute_terminal_launch_target(editor_launch_target(), launch_return_screen);
            }
            TerminalProgramRequest::OpenFileManager => {
                self.execute_terminal_launch_target(
                    file_manager_launch_target(),
                    launch_return_screen,
                );
            }
            TerminalProgramRequest::LaunchCatalog { name, catalog } => {
                self.open_embedded_catalog_launch(&name, catalog, launch_return_screen);
            }
        }
    }

    pub(super) fn apply_desktop_program_request(&mut self, request: DesktopProgramRequest) {
        match request {
            DesktopProgramRequest::OpenTextEditor { close_window: _ } => {
                self.launch_editor_via_registry();
            }
            DesktopProgramRequest::OpenFileManager => {
                self.launch_file_manager_via_registry();
            }
            DesktopProgramRequest::LaunchCatalog { name, catalog, .. } => {
                self.open_desktop_catalog_launch(&name, catalog);
            }
        }
    }

    pub(super) fn draw_terminal_documents(&mut self, ctx: &Context) {
        let layout = self.terminal_layout();
        let header_lines = self.active_terminal_header_lines().to_vec();
        let mut items = vec!["Logs".to_string()];
        items.extend(Self::sorted_document_categories());
        items.push("---".to_string());
        items.push("Back".to_string());
        let mut selected = self
            .terminal_nav
            .documents_idx
            .min(items.len().saturating_sub(1));
        let activated = draw_terminal_menu_screen(
            ctx,
            "Documents",
            Some("Select Document Type"),
            &items,
            &mut selected,
            layout.cols,
            layout.rows,
            layout.header_start_row,
            layout.separator_top_row,
            layout.title_row,
            layout.separator_bottom_row,
            layout.subtitle_row,
            layout.menu_start_row,
            layout.status_row,
            layout.content_col,
            &self.shell_status,
            &header_lines,
        );
        self.terminal_nav.documents_idx = selected;
        if let Some(idx) = activated {
            let selected = items[idx].as_str();
            match selected {
                "Logs" => {
                    self.navigate_to_screen(TerminalScreen::Logs);
                    self.terminal_nav.logs_idx = 0;
                    self.apply_status_update(clear_shell_status());
                }
                "Back" => {
                    self.navigate_to_screen(TerminalScreen::MainMenu);
                    self.apply_status_update(clear_shell_status());
                }
                "---" => {}
                category => {
                    let Some(path) = document_category_path(category) else {
                        self.shell_status = format!("Error: invalid category '{category}'.");
                        return;
                    };
                    self.open_document_browser_at(path, TerminalScreen::Documents);
                }
            }
        }
    }

    pub(super) fn draw_terminal_logs(&mut self, ctx: &Context) {
        let layout = self.terminal_layout();
        let header_lines = self.active_terminal_header_lines().to_vec();
        let items = vec![
            "New Log".to_string(),
            "View Logs".to_string(),
            "---".to_string(),
            "Back".to_string(),
        ];
        let mut selected = self
            .terminal_nav
            .logs_idx
            .min(items.len().saturating_sub(1));
        let activated = draw_terminal_menu_screen(
            ctx,
            "Logs",
            None,
            &items,
            &mut selected,
            layout.cols,
            layout.rows,
            layout.header_start_row,
            layout.separator_top_row,
            layout.title_row,
            layout.separator_bottom_row,
            layout.subtitle_row,
            layout.menu_start_row,
            layout.status_row,
            layout.content_col,
            &self.shell_status,
            &header_lines,
        );
        self.terminal_nav.logs_idx = selected;
        if let Some(idx) = activated {
            match items[idx].as_str() {
                "New Log" => {
                    let default_stem = Local::now().format("%Y-%m-%d").to_string();
                    self.open_input_prompt(
                        "New Log",
                        format!("Document name (.txt default, blank for {default_stem}.txt):"),
                        TerminalPromptAction::NewLogName,
                    );
                }
                "View Logs" => self.open_log_view(),
                "Back" => {
                    self.navigate_to_screen(TerminalScreen::Documents);
                    self.apply_status_update(clear_shell_status());
                }
                _ => {}
            }
        }
    }

    pub(super) fn draw_terminal_document_browser(&mut self, ctx: &Context) {
        let layout = self.terminal_layout();
        let header_lines = self.active_terminal_header_lines().to_vec();
        sync_browser_selection(&mut self.file_manager, self.terminal_nav.browser_idx);
        // If open-with picker is open, handle it as overlay
        if let Some(ref mut picker) = self.terminal_open_with_picker {
            if picker.open {
                let picker_action = draw_open_with_picker(ctx, picker, layout.cols, layout.rows);
                // Consume remaining navigation keys so the browser doesn't act on them
                ctx.input_mut(|i| {
                    let m = egui::Modifiers::NONE;
                    i.consume_key(m, egui::Key::ArrowUp);
                    i.consume_key(m, egui::Key::ArrowDown);
                    i.consume_key(m, egui::Key::Enter);
                    i.consume_key(m, egui::Key::Space);
                    i.consume_key(m, egui::Key::Escape);
                    i.consume_key(m, egui::Key::Tab);
                    i.consume_key(m, egui::Key::Q);
                    i.consume_key(m, egui::Key::O);
                    i.consume_key(m, egui::Key::F1);
                    i.consume_key(m, egui::Key::F2);
                    i.consume_key(m, egui::Key::Delete);
                    i.consume_key(m, egui::Key::Backspace);
                    i.consume_key(egui::Modifiers::COMMAND, egui::Key::C);
                    i.consume_key(egui::Modifiers::COMMAND, egui::Key::X);
                    i.consume_key(egui::Modifiers::COMMAND, egui::Key::V);
                    i.consume_key(egui::Modifiers::COMMAND, egui::Key::Z);
                    i.consume_key(egui::Modifiers::COMMAND, egui::Key::Y);
                    i.consume_key(
                        egui::Modifiers::COMMAND | egui::Modifiers::SHIFT,
                        egui::Key::N,
                    );
                });
                // Draw browser underneath
                draw_terminal_document_browser(
                    ctx,
                    &self.file_manager,
                    &mut self.terminal_nav.browser_idx,
                    &self.shell_status,
                    layout.cols,
                    layout.rows,
                    layout.header_start_row,
                    layout.separator_top_row,
                    layout.title_row,
                    layout.separator_bottom_row,
                    layout.subtitle_row,
                    layout.menu_start_row,
                    layout.status_row,
                    layout.status_row_alt,
                    layout.content_col,
                    false,
                    &header_lines,
                );
                if let Some(action) = picker_action {
                    match action {
                        OpenWithPickerAction::LaunchCommand { command } => {
                            self.apply_open_with_picker_launch(command);
                        }
                        OpenWithPickerAction::OpenOtherPrompt => {
                            self.apply_open_with_picker_other();
                        }
                    }
                }
                // Check if picker was closed (e.g. by Esc)
                if self
                    .terminal_open_with_picker
                    .as_ref()
                    .is_some_and(|p| !p.open)
                {
                    self.terminal_open_with_picker = None;
                }
                return;
            }
        }
        // If command palette is open for the document browser, handle it
        if self.command_layer_open_for(CommandLayerTarget::FileManager) {
            self.draw_command_layer_at(
                ctx,
                CommandLayerTarget::FileManager,
                self.terminal_command_layer_bar_pos(ctx),
                ctx.screen_rect(),
            );
            draw_terminal_document_browser(
                ctx,
                &self.file_manager,
                &mut self.terminal_nav.browser_idx,
                &self.shell_status,
                layout.cols,
                layout.rows,
                layout.header_start_row,
                layout.separator_top_row,
                layout.title_row,
                layout.separator_bottom_row,
                layout.subtitle_row,
                layout.menu_start_row,
                layout.status_row,
                layout.status_row_alt,
                layout.content_col,
                false,
                &header_lines,
            );
            return;
        }
        let event = draw_terminal_document_browser(
            ctx,
            &self.file_manager,
            &mut self.terminal_nav.browser_idx,
            &self.shell_status,
            layout.cols,
            layout.rows,
            layout.header_start_row,
            layout.separator_top_row,
            layout.title_row,
            layout.separator_bottom_row,
            layout.subtitle_row,
            layout.menu_start_row,
            layout.status_row,
            layout.status_row_alt,
            layout.content_col,
            true,
            &header_lines,
        );
        match event {
            DocumentBrowserEvent::None => {}
            DocumentBrowserEvent::Quit => {
                crate::sound::play_navigate();
                self.navigate_to_screen(self.terminal_nav.browser_return_screen);
                self.apply_status_update(clear_shell_status());
            }
            DocumentBrowserEvent::GoBack => {
                crate::sound::play_navigate();
                self.file_manager.up();
                self.terminal_nav.browser_idx = 0;
            }
            DocumentBrowserEvent::Activate => {
                match activate_browser_selection(
                    &mut self.file_manager,
                    self.terminal_nav.browser_idx,
                ) {
                    TerminalDocumentBrowserRequest::None => {}
                    TerminalDocumentBrowserRequest::ChangedDir => {
                        crate::sound::play_navigate();
                        self.terminal_nav.browser_idx = 0;
                    }
                    TerminalDocumentBrowserRequest::OpenFile(path) => {
                        crate::sound::play_navigate();
                        self.file_manager.select(Some(path));
                        if self.file_manager_picker_active() {
                            self.file_manager_activate_or_pick();
                        } else {
                            self.activate_file_manager_selection();
                        }
                    }
                }
            }
            DocumentBrowserEvent::OpenCommandPalette => {
                crate::sound::play_navigate();
                self.open_command_layer(CommandLayerTarget::FileManager);
            }
            DocumentBrowserEvent::Copy => {
                crate::sound::play_navigate();
                self.run_file_manager_command(FileManagerCommand::Copy);
            }
            DocumentBrowserEvent::Cut => {
                crate::sound::play_navigate();
                self.run_file_manager_command(FileManagerCommand::Cut);
            }
            DocumentBrowserEvent::Paste => {
                crate::sound::play_navigate();
                self.run_file_manager_command(FileManagerCommand::Paste);
            }
            DocumentBrowserEvent::Delete => {
                crate::sound::play_navigate();
                self.run_file_manager_command(FileManagerCommand::Delete);
            }
            DocumentBrowserEvent::Rename => {
                crate::sound::play_navigate();
                self.run_file_manager_command(FileManagerCommand::Rename);
            }
            DocumentBrowserEvent::Undo => {
                crate::sound::play_navigate();
                self.run_file_manager_command(FileManagerCommand::Undo);
            }
            DocumentBrowserEvent::Redo => {
                crate::sound::play_navigate();
                self.run_file_manager_command(FileManagerCommand::Redo);
            }
            DocumentBrowserEvent::NewFolder => {
                crate::sound::play_navigate();
                self.run_file_manager_command(FileManagerCommand::NewFolder);
            }
            DocumentBrowserEvent::OpenWith => {
                crate::sound::play_navigate();
                self.open_terminal_open_with_picker();
            }
        }
    }

    pub(super) fn draw_terminal_settings(&mut self, ctx: &Context) {
        if matches!(
            self.terminal_settings_panel,
            TerminalSettingsPanel::Appearance
        ) {
            self.draw_terminal_tweaks_screen(ctx);
            return;
        }
        let layout = self.terminal_layout();
        let header_lines = self.active_terminal_header_lines().to_vec();
        let previous_window_mode = self.settings.draft.native_startup_window_mode;
        let visibility = self.terminal_settings_visibility();
        let event = run_terminal_settings_screen(
            ctx,
            &mut self.settings.draft,
            &mut self.terminal_settings_panel,
            &mut self.terminal_nav.settings_idx,
            &mut self.terminal_nav.settings_choice,
            visibility,
            self.session.as_ref().is_some_and(|s| s.is_admin),
            &self.shell_status,
            layout.cols,
            layout.rows,
            layout.header_start_row,
            layout.separator_top_row,
            layout.title_row,
            layout.separator_bottom_row,
            layout.subtitle_row,
            layout.menu_start_row,
            layout.status_row,
            layout.content_col,
            &header_lines,
        );
        match event {
            TerminalSettingsEvent::None => {}
            TerminalSettingsEvent::Persist => {
                self.persist_native_settings();
                if self.settings.draft.native_startup_window_mode != previous_window_mode {
                    self.apply_native_window_mode(ctx);
                }
            }
            TerminalSettingsEvent::OpenPanel(panel) => {
                self.terminal_settings_panel = panel;
                if matches!(panel, TerminalSettingsPanel::Appearance) {
                    self.terminal_tweaks_open_dropdown = None;
                }
                self.terminal_nav.settings_idx = 0;
                self.terminal_nav.settings_choice = None;
                self.apply_status_update(clear_shell_status());
            }
            TerminalSettingsEvent::Back => {
                if matches!(self.terminal_settings_panel, TerminalSettingsPanel::Home) {
                    self.apply_terminal_screen_open_plan(terminal_screen_open_plan(
                        TerminalScreen::MainMenu,
                        0,
                        true,
                    ));
                } else if matches!(
                    self.terminal_settings_panel,
                    TerminalSettingsPanel::AppearanceEffects
                ) {
                    self.terminal_settings_panel = TerminalSettingsPanel::Appearance;
                    self.terminal_nav.settings_idx = 0;
                    self.terminal_nav.settings_choice = None;
                    self.apply_status_update(clear_shell_status());
                } else {
                    self.terminal_settings_panel = TerminalSettingsPanel::Home;
                    self.terminal_nav.settings_idx = 0;
                    self.terminal_nav.settings_choice = None;
                    self.apply_status_update(clear_shell_status());
                }
            }
            TerminalSettingsEvent::OpenCapability(capability) => {
                self.execute_terminal_launch_target(
                    LaunchTarget::Capability { capability },
                    TerminalScreen::Settings,
                );
            }
            TerminalSettingsEvent::EnterUserManagement => {
                self.apply_terminal_screen_open_plan(terminal_screen_open_plan(
                    TerminalScreen::UserManagement,
                    0,
                    true,
                ));
            }
        }
    }

    pub(super) fn draw_terminal_edit_menus(&mut self, ctx: &Context) {
        let layout = self.terminal_layout();
        let header_lines = self.active_terminal_header_lines().to_vec();
        let (_, show_text_editor) = self.visible_application_builtins();
        let applications = self.edit_program_entries(EditMenuTarget::Applications);
        let documents = self.edit_program_entries(EditMenuTarget::Documents);
        let network = self.edit_program_entries(EditMenuTarget::Network);
        let games = self.edit_program_entries(EditMenuTarget::Games);
        let event = draw_edit_menus_screen(
            ctx,
            &mut self.terminal_edit_menus,
            EditMenusEntries {
                applications: &applications,
                documents: &documents,
                network: &network,
                games: &games,
            },
            show_text_editor,
            &self.shell_status,
            layout.cols,
            layout.rows,
            layout.header_start_row,
            layout.separator_top_row,
            layout.title_row,
            layout.separator_bottom_row,
            layout.subtitle_row,
            layout.menu_start_row,
            layout.status_row,
            layout.content_col,
            &header_lines,
        );
        match event {
            TerminalEditMenusRequest::None => {}
            TerminalEditMenusRequest::BackToSettings => {
                self.apply_terminal_screen_open_plan(terminal_settings_refresh_plan());
            }
            TerminalEditMenusRequest::PersistToggleBuiltinTextEditor => {
                self.settings.draft.builtin_menu_visibility.text_editor =
                    !self.settings.draft.builtin_menu_visibility.text_editor;
                self.persist_native_settings();
            }
            TerminalEditMenusRequest::OpenPromptAddProgramName {
                target,
                title,
                prompt,
            } => {
                self.open_input_prompt(
                    title,
                    prompt,
                    TerminalPromptAction::EditMenuAddProgramName { target },
                );
            }
            TerminalEditMenusRequest::OpenPromptAddCategoryName { title, prompt } => {
                self.open_input_prompt(
                    title,
                    prompt,
                    TerminalPromptAction::EditMenuAddCategoryName,
                );
            }
            TerminalEditMenusRequest::OpenConfirmDelete {
                target,
                title,
                prompt,
                name,
            } => {
                self.open_confirm_prompt(
                    title,
                    prompt,
                    TerminalPromptAction::ConfirmEditMenuDelete { target, name },
                );
            }
            TerminalEditMenusRequest::Status(status) => {
                self.shell_status = status;
            }
        }
    }

    pub(super) fn apply_terminal_connections_request(
        &mut self,
        request: TerminalConnectionsRequest,
    ) {
        match request {
            TerminalConnectionsRequest::None => {}
            TerminalConnectionsRequest::BackToSettings => {
                self.apply_terminal_screen_open_plan(terminal_settings_refresh_plan());
            }
            TerminalConnectionsRequest::NavigateToView {
                view,
                clear_status,
                reset_kind_idx,
                reset_picker_idx,
            } => {
                crate::sound::play_navigate();
                self.terminal_connections.view = view;
                if reset_kind_idx {
                    self.terminal_connections.kind_idx = 0;
                }
                if reset_picker_idx {
                    self.terminal_connections.picker_idx = 0;
                }
                if clear_status {
                    self.apply_status_update(clear_shell_status());
                }
            }
            TerminalConnectionsRequest::OpenPromptSearch {
                kind,
                group,
                title,
                prompt,
            } => {
                self.open_input_prompt(
                    &title,
                    prompt,
                    TerminalPromptAction::ConnectionSearch { kind, group },
                );
            }
            TerminalConnectionsRequest::OpenPasswordPrompt {
                kind,
                target,
                title,
                prompt,
            } => {
                self.open_password_prompt_with_action(
                    &title,
                    prompt,
                    TerminalPromptAction::ConnectionPassword {
                        kind,
                        name: target.name,
                        detail: target.detail,
                    },
                );
            }
            TerminalConnectionsRequest::ConnectImmediate { kind, target } => {
                self.connect_target(kind, target, None);
            }
            TerminalConnectionsRequest::Status {
                status,
                back_to_settings,
            } => {
                self.shell_status = status;
                if back_to_settings {
                    self.apply_terminal_screen_open_plan(terminal_settings_refresh_plan());
                }
            }
        }
    }

    pub(super) fn draw_terminal_connections(&mut self, ctx: &Context) {
        let layout = self.terminal_layout();
        let header_lines = self.active_terminal_header_lines().to_vec();
        let request = draw_terminal_connections_screen(
            ctx,
            &mut self.terminal_connections,
            &self.shell_status,
            layout.cols,
            layout.rows,
            layout.header_start_row,
            layout.separator_top_row,
            layout.title_row,
            layout.separator_bottom_row,
            layout.subtitle_row,
            layout.menu_start_row,
            layout.status_row,
            layout.content_col,
            &header_lines,
        );
        self.apply_terminal_connections_request(request);
    }

    pub(super) fn draw_terminal_prompt_overlay_global(&self, ctx: &Context) {
        let layout = self.terminal_layout();
        let Some(prompt) = self.terminal_prompt.as_ref() else {
            return;
        };
        let viewport = ctx.screen_rect();
        egui::Area::new(Id::new("native_terminal_prompt_overlay"))
            .order(egui::Order::Foreground)
            .fixed_pos(viewport.min)
            .show(ctx, |ui| {
                ui.set_min_size(viewport.size());
                let (screen, _) = RetroScreen::new(ui, layout.cols, layout.rows);
                draw_terminal_prompt_overlay(ui, &screen, prompt);
            });
    }

    pub(super) fn draw_terminal_default_apps(&mut self, ctx: &Context) {
        let layout = self.terminal_layout();
        let header_lines = self.active_terminal_header_lines().to_vec();
        let event = draw_default_apps_screen(
            ctx,
            &self.settings.draft,
            &mut self.terminal_nav.default_apps_idx,
            &mut self.terminal_nav.default_app_choice_idx,
            &mut self.terminal_nav.default_app_slot,
            &self.shell_status,
            layout.cols,
            layout.rows,
            layout.header_start_row,
            layout.separator_top_row,
            layout.title_row,
            layout.separator_bottom_row,
            layout.subtitle_row,
            layout.menu_start_row,
            layout.status_row,
            layout.content_col,
            &header_lines,
        );
        match event {
            TerminalDefaultAppsRequest::None => {}
            TerminalDefaultAppsRequest::BackToSettings => {
                self.apply_terminal_screen_open_plan(terminal_settings_refresh_plan());
            }
            TerminalDefaultAppsRequest::OpenSlot(slot) => {
                crate::sound::play_navigate();
                self.terminal_nav.default_app_slot = Some(slot);
                self.terminal_nav.default_app_choice_idx = 0;
            }
            TerminalDefaultAppsRequest::CloseSlotPicker => {
                crate::sound::play_navigate();
                self.terminal_nav.default_app_slot = None;
            }
            TerminalDefaultAppsRequest::ApplyBinding { slot, binding } => {
                apply_default_app_binding(&mut self.settings.draft, slot, binding);
                self.persist_native_settings();
                self.terminal_nav.default_app_slot = None;
            }
            TerminalDefaultAppsRequest::PromptCustom { slot, prompt_label } => {
                self.open_input_prompt(
                    "Default Apps",
                    format!("{prompt_label} command (example: epy):"),
                    TerminalPromptAction::DefaultAppCustom { slot },
                );
            }
        }
    }

    pub(super) fn draw_terminal_about(&mut self, ctx: &Context) {
        let layout = self.terminal_layout();
        let header_lines = self.active_terminal_header_lines().to_vec();
        match draw_about_screen(
            ctx,
            layout.cols,
            layout.rows,
            layout.header_start_row,
            layout.separator_top_row,
            layout.title_row,
            layout.separator_bottom_row,
            layout.subtitle_row,
            layout.menu_start_row,
            layout.status_row,
            layout.content_col,
            &header_lines,
        ) {
            TerminalAboutRequest::None => {}
            TerminalAboutRequest::Back => {
                self.apply_terminal_screen_open_plan(terminal_settings_refresh_plan());
            }
        }
    }

    pub(super) fn draw_terminal_network(&mut self, ctx: &Context) {
        let layout = self.terminal_layout();
        let header_lines = self.active_terminal_header_lines().to_vec();
        let entries = catalog_names(ProgramCatalog::Network);
        let event = draw_programs_menu(
            ctx,
            "Network",
            Some("Select Network Program"),
            &entries,
            &mut self.terminal_nav.network_idx,
            &self.shell_status,
            layout.cols,
            layout.rows,
            layout.header_start_row,
            layout.separator_top_row,
            layout.title_row,
            layout.separator_bottom_row,
            layout.subtitle_row,
            layout.menu_start_row,
            layout.status_row,
            layout.content_col,
            &header_lines,
        );
        let request = resolve_terminal_catalog_request(event, ProgramCatalog::Network);
        self.apply_terminal_program_request(request, TerminalScreen::Network);
    }

    pub(super) fn draw_terminal_games(&mut self, ctx: &Context) {
        let layout = self.terminal_layout();
        let header_lines = self.active_terminal_header_lines().to_vec();
        let mut configured_names = catalog_names(ProgramCatalog::Games);
        for name in installed_hosted_game_names() {
            if !configured_names.iter().any(|existing| existing == &name) {
                configured_names.push(name);
            }
        }
        configured_names.sort();
        let entries = build_terminal_game_entries(&configured_names);
        let event = draw_programs_menu(
            ctx,
            "Games",
            Some("Select Game"),
            &entries,
            &mut self.terminal_nav.games_idx,
            &self.shell_status,
            layout.cols,
            layout.rows,
            layout.header_start_row,
            layout.separator_top_row,
            layout.title_row,
            layout.separator_bottom_row,
            layout.subtitle_row,
            layout.menu_start_row,
            layout.status_row,
            layout.content_col,
            &header_lines,
        );
        match event {
            ProgramMenuEvent::None => {}
            ProgramMenuEvent::Back => {
                self.navigate_to_screen(TerminalScreen::MainMenu);
                self.apply_status_update(clear_shell_status());
            }
            other => {
                let request = resolve_terminal_games_request(other);
                self.apply_terminal_program_request(request, TerminalScreen::Games);
            }
        }
    }

    pub(super) fn draw_terminal_pty(&mut self, ctx: &Context) {
        if self.terminal_wasm_addon.is_some() {
            self.draw_terminal_wasm_addon(ctx);
            return;
        }
        let layout = self.terminal_layout();
        if !self.primary_embedded_pty_open() {
            self.navigate_to_screen(TerminalScreen::MainMenu);
            self.shell_status = "No embedded PTY session.".to_string();
            return;
        };
        let Some(state) = self.terminal_pty.as_mut() else {
            self.navigate_to_screen(TerminalScreen::MainMenu);
            self.shell_status = "No embedded PTY session.".to_string();
            return;
        };
        let event = draw_embedded_pty(
            ctx,
            state,
            ShellSurfaceKind::Terminal,
            &self.shell_status,
            layout.cols,
            layout.rows,
            layout.header_start_row,
            layout.separator_top_row,
            layout.title_row,
            layout.separator_bottom_row,
            layout.subtitle_row,
            layout.menu_start_row,
            layout.status_row,
            layout.content_col,
        );
        match event {
            PtyScreenEvent::None => {}
            PtyScreenEvent::CloseRequested => self.handle_terminal_back(),
            PtyScreenEvent::ProcessExited => {
                if let Some(pty) = self.take_primary_pty() {
                    let plan = resolve_embedded_pty_exit(
                        &pty.title,
                        pty.return_screen,
                        pty.completion_message.as_deref(),
                    );
                    self.apply_terminal_embedded_pty_exit_plan(plan);
                } else {
                    self.navigate_to_screen(TerminalScreen::MainMenu);
                    self.shell_status = "PTY session exited.".to_string();
                }
            }
        }
    }

    fn draw_terminal_wasm_addon(&mut self, ctx: &Context) {
        if ctx.input(|i| i.key_pressed(egui::Key::Tab)) {
            let return_screen = self
                .terminal_wasm_addon_return_screen
                .unwrap_or(TerminalScreen::Games);
            self.clear_terminal_wasm_addon();
            self.navigate_to_screen(return_screen);
            self.apply_status_update(clear_shell_status());
            return;
        }

        let dt = Self::next_embedded_game_dt(&mut self.terminal_wasm_addon_last_frame_at);
        let input = collect_hosted_keyboard_input(ctx, true);
        let title = self
            .terminal_wasm_addon
            .as_ref()
            .map(|state| state.title().to_string())
            .unwrap_or_else(|| "Addon".to_string());
        let mut failed = None;

        Self::draw_terminal_game_shell(
            ctx,
            &title,
            "TAB BACK  ARROWS/WASD MOVE  SPACE/ENTER ACTION",
            |ui| {
                let available = ui.available_size_before_wrap();
                let size = HostedAddonSize {
                    width: available.x.max(1.0),
                    height: available.y.max(1.0),
                };
                if let Some(state) = self.terminal_wasm_addon.as_mut() {
                    if let Err(err) = state.update(size, dt, input) {
                        failed = Some(err);
                    } else {
                        draw_hosted_addon_frame(ui, state);
                    }
                }
            },
        );

        if let Some(err) = failed {
            let return_screen = self
                .terminal_wasm_addon_return_screen
                .unwrap_or(TerminalScreen::Games);
            self.clear_terminal_wasm_addon();
            self.navigate_to_screen(return_screen);
            self.shell_status = err;
            return;
        }

        ctx.request_repaint();
    }

    pub(super) fn apply_installer_event(&mut self, event: InstallerEvent) {
        match event {
            InstallerEvent::None => {}
            InstallerEvent::BackToMainMenu => {
                self.apply_terminal_screen_open_plan(terminal_screen_open_plan(
                    TerminalScreen::MainMenu,
                    0,
                    true,
                ));
            }
            InstallerEvent::OpenSearchPrompt => {
                self.open_input_prompt(
                    "Program Installer",
                    "Search packages:",
                    TerminalPromptAction::InstallerSearch,
                );
            }
            InstallerEvent::OpenFilterPrompt => {
                self.open_input_prompt(
                    "Installed Apps",
                    "Filter:",
                    TerminalPromptAction::InstallerFilter,
                );
            }
            InstallerEvent::OpenAddonInstallPrompt => {
                self.open_input_prompt(
                    "Installed Addons",
                    "Manifest or addon folder path:",
                    TerminalPromptAction::InstallerAddonPath,
                );
            }
            InstallerEvent::OpenConfirmAction { pkg, action } => {
                let prompt = match action {
                    InstallerPackageAction::Install => format!("Install {pkg}?"),
                    InstallerPackageAction::Update => format!("Update {pkg}?"),
                    InstallerPackageAction::Reinstall => format!("Reinstall {pkg}?"),
                    InstallerPackageAction::Uninstall => format!("Uninstall {pkg}?"),
                };
                self.open_confirm_prompt(
                    "Program Installer",
                    prompt,
                    TerminalPromptAction::ConfirmInstallerAction { pkg, action },
                );
            }
            InstallerEvent::OpenDisplayNamePrompt { pkg, target } => {
                self.open_input_prompt(
                    "Add to Menu",
                    format!("Display name for '{pkg}':"),
                    TerminalPromptAction::InstallerDisplayName { pkg, target },
                );
            }
            InstallerEvent::StartRepositoryAddonInstall {
                addon_id,
                action_label,
            } => {
                let action = match action_label.as_str() {
                    "Update" => crate::native::RepositoryAddonAction::Update,
                    "Reinstall" => crate::native::RepositoryAddonAction::Reinstall,
                    _ => crate::native::RepositoryAddonAction::Install,
                };
                self.start_repository_addon_install(
                    crate::platform::AddonId::from(addon_id),
                    action,
                    false,
                );
            }
            InstallerEvent::LaunchCommand {
                argv,
                status,
                completion_message,
            } => {
                settle_view_after_package_command(&mut self.terminal_installer);
                self.queue_terminal_flash(
                    status.clone(),
                    700,
                    FlashAction::LaunchPty {
                        title: "Program Installer".to_string(),
                        argv,
                        return_screen: TerminalScreen::ProgramInstaller,
                        status: status.clone(),
                        completion_message,
                    },
                );
                self.shell_status = status;
            }
            InstallerEvent::Status(status) => {
                self.queue_terminal_flash(status.clone(), 650, FlashAction::Noop);
                self.shell_status = status;
            }
        }
    }

    pub(super) fn draw_terminal_program_installer(&mut self, ctx: &Context) {
        if self.terminal_installer.addon_install_in_flight.is_some() {
            ctx.request_repaint_after(std::time::Duration::from_millis(50));
        }
        let layout = self.terminal_layout();
        let header_lines = self.active_terminal_header_lines().to_vec();
        let event = draw_installer_screen(
            ctx,
            &mut self.terminal_installer,
            &self.shell_status,
            layout.cols,
            layout.rows,
            layout.header_start_row,
            layout.separator_top_row,
            layout.title_row,
            layout.separator_bottom_row,
            layout.subtitle_row,
            layout.menu_start_row,
            layout.status_row,
            layout.content_col,
            &header_lines,
        );
        self.apply_installer_event(event);
    }

    pub(super) fn draw_terminal_user_management(&mut self, ctx: &Context) {
        let layout = self.terminal_layout();
        let header_lines = self.active_terminal_header_lines().to_vec();
        let mode = self.terminal_nav.user_management_mode.clone();
        let screen = user_management_screen_for_mode(
            &mode,
            self.session.as_ref().map(|s| s.username.as_str()),
            self.live_hacking_difficulty,
        );
        let mut selected = self.terminal_nav.user_management_idx.min(
            screen
                .items
                .iter()
                .filter(|i| i.as_str() != "---")
                .count()
                .saturating_sub(1),
        );
        let refs = screen.items;
        let activated = draw_terminal_menu_screen(
            ctx,
            screen.title,
            screen.subtitle.as_deref(),
            &refs,
            &mut selected,
            layout.cols,
            layout.rows,
            layout.header_start_row,
            layout.separator_top_row,
            layout.title_row,
            layout.separator_bottom_row,
            layout.subtitle_row,
            layout.menu_start_row,
            layout.status_row,
            layout.content_col,
            &self.shell_status,
            &header_lines,
        );
        self.terminal_nav.user_management_idx = selected;
        if let Some(idx) = activated {
            let selected_label = refs[idx].clone();
            let action = handle_user_management_selection(
                &mode,
                &selected_label,
                self.session.as_ref().map(|s| s.username.as_str()),
            );
            match plan_user_management_action(action) {
                UserManagementExecutionPlan::None => {}
                UserManagementExecutionPlan::OpenCreateUserPrompt => self.open_input_prompt(
                    "Create User",
                    "New username:",
                    TerminalPromptAction::CreateUsername,
                ),
                UserManagementExecutionPlan::CycleHackingDifficulty => {
                    cycle_hacking_difficulty_in_settings(&mut self.settings.draft);
                    self.sync_runtime_settings_cache();
                    self.apply_status_update(saved_shell_status());
                }
                UserManagementExecutionPlan::SetMode { mode, selected_idx } => {
                    self.set_user_management_mode(mode, selected_idx);
                }
                UserManagementExecutionPlan::BackToSettings => {
                    self.apply_terminal_screen_open_plan(terminal_settings_refresh_plan());
                    self.terminal_nav.user_management_idx = 0;
                }
                UserManagementExecutionPlan::OpenCreatePasswordPrompt { username } => {
                    self.open_password_prompt_with_action(
                        "Create User",
                        format!("Password for {username}"),
                        TerminalPromptAction::CreatePassword { username },
                    );
                }
                UserManagementExecutionPlan::ApplyCreateUser { username, method } => {
                    self.apply_shell_status_result(create_desktop_user(&username, method, None));
                    self.invalidate_user_cache();
                    self.set_user_management_mode(UserManagementMode::Root, 0);
                }
                UserManagementExecutionPlan::OpenConfirmDeleteUser { username } => {
                    self.open_confirm_prompt(
                        "Delete User",
                        format!("Delete user '{username}'?"),
                        TerminalPromptAction::ConfirmDeleteUser { username },
                    );
                }
                UserManagementExecutionPlan::OpenResetPasswordPrompt { username } => {
                    self.open_password_prompt_with_action(
                        "Reset Password",
                        format!("New password for '{username}'"),
                        TerminalPromptAction::ResetPassword { username },
                    );
                }
                UserManagementExecutionPlan::OpenChangeAuthPasswordPrompt { username } => {
                    self.open_password_prompt_with_action(
                        "Change Auth Method",
                        format!("New password for '{username}'"),
                        TerminalPromptAction::ChangeAuthPassword { username },
                    );
                }
                UserManagementExecutionPlan::ApplyChangeAuthMethod { username, method } => {
                    self.apply_shell_status_result(update_user_auth_method(
                        &username, method, None,
                    ));
                    self.invalidate_user_cache();
                    self.set_user_management_mode(UserManagementMode::Root, 0);
                }
                UserManagementExecutionPlan::OpenConfirmToggleAdmin { username } => {
                    self.open_confirm_prompt(
                        "Toggle Admin",
                        format!("Toggle admin for '{username}'?"),
                        TerminalPromptAction::ConfirmToggleAdmin { username },
                    );
                }
                UserManagementExecutionPlan::Status(status) => {
                    self.shell_status = status;
                }
            }
        }
    }

    pub(super) fn terminal_status_bar_repaint_interval(ctx: &Context) -> Duration {
        if !ctx.input(|i| i.focused) {
            return Duration::from_secs(300);
        }
        let now = Local::now();
        Duration::from_secs(u64::from((60 - now.second()).max(1)))
    }

    pub(super) fn draw_terminal_status_bar(
        &self,
        ctx: &Context,
        position: TerminalStatusBarPosition,
        height: f32,
    ) {
        ctx.request_repaint_after(Self::terminal_status_bar_repaint_interval(ctx));
        let palette = current_palette_for_surface(ShellSurfaceKind::Terminal);
        let panel = match position {
            TerminalStatusBarPosition::Bottom => {
                TopBottomPanel::bottom("native_terminal_status_bar")
            }
            TerminalStatusBarPosition::Hidden => return,
        };
        panel
            .resizable(false)
            .exact_height(height)
            .show_separator_line(false)
            .frame(
                egui::Frame::none()
                    .fill(palette.fg)
                    .inner_margin(egui::Margin::symmetric(6.0, 4.0)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    // Left: date/time
                    let now = Local::now().format("%a %Y-%m-%d %I:%M%p").to_string();
                    ui.label(RichText::new(now).color(Color32::BLACK).strong());

                    // Center: session tabs [1*] [2] [3]
                    let tabs = native_session_tabs();
                    if !tabs.labels.is_empty() {
                        let tabs = tabs.labels.join(" ");
                        // Approximate centering
                        let avail = ui.available_width();
                        let tab_width = tabs.len() as f32 * 8.0;
                        let spacing = ((avail - tab_width) / 2.0).max(8.0);
                        ui.add_space(spacing);
                        ui.label(RichText::new(tabs).color(Color32::BLACK).strong());
                    }

                    // Right: battery (if available)
                    ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                        let batt = crate::status::battery_status_string();
                        if !batt.is_empty() {
                            ui.label(RichText::new(batt).color(Color32::BLACK).strong());
                        }
                    });
                });
            });
    }
}
