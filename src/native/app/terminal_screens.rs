use super::super::about_screen::{draw_about_screen, TerminalAboutRequest};
use super::super::background::BackgroundResult;
use super::super::connections_screen::{
    draw_terminal_connections_screen, TerminalConnectionsRequest,
};
use super::super::default_apps_screen::{draw_default_apps_screen, TerminalDefaultAppsRequest};
use super::super::desktop_app::DesktopWindow;
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
    activate_browser_selection, draw_terminal_document_browser, DocumentBrowserEvent,
    TerminalDocumentBrowserRequest,
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
use super::super::nuke_codes_screen::{draw_nuke_codes_screen, NukeCodesEvent};
use super::super::programs_screen::draw_programs_menu;
use super::super::prompt::{draw_terminal_prompt_overlay, FlashAction, TerminalPromptAction};
use super::super::pty_screen::{draw_embedded_pty, PtyScreenEvent};
use super::super::retro_ui::{current_palette, RetroScreen};
use super::super::settings_screen::{run_terminal_settings_screen, TerminalSettingsEvent};
use super::super::shell_screen::draw_main_menu_screen;
use super::super::terminal_command_palette::{
    draw_command_palette, CommandPaletteState, CommandPaletteTarget,
};
use super::super::terminal_open_with_picker::{draw_open_with_picker, OpenWithPickerAction};
use super::retro_footer_height;
use super::RobcoNativeApp;
use super::{BUILTIN_NUKE_CODES_APP, BUILTIN_TEXT_EDITOR_APP};
use chrono::{Local, Timelike};
use eframe::egui::{self, Color32, Context, Id, Layout, RichText, TopBottomPanel};
use robcos_native_nuke_codes_app::{fetch_nuke_codes, NukeCodesView};
use robcos_native_programs_app::{
    build_terminal_application_entries, build_terminal_game_entries,
    resolve_terminal_applications_request, resolve_terminal_catalog_request,
    resolve_terminal_games_request, DesktopProgramRequest, TerminalProgramRequest,
};
use robcos_native_settings_app::TerminalSettingsPanel;
use std::time::Duration;

impl RobcoNativeApp {
    pub(super) fn draw_terminal_main_menu(&mut self, ctx: &Context) {
        let layout = self.terminal_layout();
        let activated = draw_main_menu_screen(
            ctx,
            &mut self.terminal_nav.main_menu_idx,
            &self.shell_status,
            &format!("RobcOS v{}", env!("CARGO_PKG_VERSION")),
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
        if let Some(action) = activated {
            let action = resolve_main_menu_action(action);
            self.apply_main_menu_selection_action(action);
        }
    }

    pub(super) fn draw_terminal_applications(&mut self, ctx: &Context) {
        let layout = self.terminal_layout();
        let entries = build_terminal_application_entries(
            self.settings.draft.builtin_menu_visibility.text_editor,
            self.settings.draft.builtin_menu_visibility.nuke_codes,
            &catalog_names(ProgramCatalog::Applications),
            BUILTIN_TEXT_EDITOR_APP,
            BUILTIN_NUKE_CODES_APP,
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
        );
        let request = resolve_terminal_applications_request(
            event,
            BUILTIN_TEXT_EDITOR_APP,
            BUILTIN_NUKE_CODES_APP,
        );
        self.apply_terminal_program_request(request, TerminalScreen::Applications);
    }

    pub(super) fn open_nuke_codes_screen(&mut self, return_screen: TerminalScreen) {
        {
            let tx = self.background.sender();
            std::thread::spawn(move || {
                let view = fetch_nuke_codes();
                let _ = tx.send(BackgroundResult::NukeCodesFetched(view));
            });
            self.terminal_nuke_codes = NukeCodesView::Unloaded;
        }
        self.terminal_nav.nuke_codes_return_screen = return_screen;
        self.navigate_to_screen(TerminalScreen::NukeCodes);
        self.apply_status_update(clear_shell_status());
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
                self.editor.open = true;
                if self.editor.path.is_none() {
                    self.new_document();
                }
                self.shell_status = format!("Opened {BUILTIN_TEXT_EDITOR_APP}.");
            }
            TerminalProgramRequest::OpenNukeCodes => {
                self.open_nuke_codes_screen(launch_return_screen);
            }
            TerminalProgramRequest::OpenFileManager => {
                self.terminal_nav.browser_idx = 0;
                self.terminal_nav.browser_return_screen = launch_return_screen;
                self.navigate_to_screen(TerminalScreen::DocumentBrowser);
                self.shell_status = "Opened File Manager.".to_string();
            }
            TerminalProgramRequest::LaunchCatalog { name, catalog } => {
                self.open_embedded_catalog_launch(&name, catalog, launch_return_screen);
            }
        }
    }

    pub(super) fn apply_desktop_program_request(&mut self, request: DesktopProgramRequest) {
        match request {
            DesktopProgramRequest::OpenTextEditor { close_window: _ } => {
                self.open_or_spawn_desktop_window(DesktopWindow::Editor);
            }
            DesktopProgramRequest::OpenNukeCodes { close_window: _ } => {
                self.open_desktop_nuke_codes();
            }
            DesktopProgramRequest::OpenFileManager => {
                self.open_or_spawn_desktop_window(DesktopWindow::FileManager);
            }
            DesktopProgramRequest::LaunchCatalog { name, catalog, .. } => {
                self.open_desktop_catalog_launch(&name, catalog);
            }
        }
    }

    pub(super) fn draw_terminal_documents(&mut self, ctx: &Context) {
        let layout = self.terminal_layout();
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
        if self.terminal_command_palette.open
            && self.terminal_command_palette.target == CommandPaletteTarget::DocumentBrowser
        {
            // Let the palette process and consume keys first
            let palette_action = draw_command_palette(
                ctx,
                &mut self.terminal_command_palette,
                layout.cols,
                layout.rows,
            );
            // Consume any remaining navigation keys so the browser doesn't act on them
            ctx.input_mut(|i| {
                let m = egui::Modifiers::NONE;
                i.consume_key(m, egui::Key::ArrowUp);
                i.consume_key(m, egui::Key::ArrowDown);
                i.consume_key(m, egui::Key::Enter);
                i.consume_key(m, egui::Key::Space);
                i.consume_key(m, egui::Key::Escape);
                i.consume_key(m, egui::Key::Tab);
                i.consume_key(m, egui::Key::Q);
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
            // Draw the browser visually underneath (keys already consumed)
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
            );
            if let Some(action) = palette_action {
                self.apply_fm_palette_action(action);
            }
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
            DocumentBrowserEvent::Activate(_) => {
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
                        self.activate_file_manager_selection();
                    }
                }
            }
            DocumentBrowserEvent::OpenCommandPalette => {
                crate::sound::play_navigate();
                self.terminal_command_palette = CommandPaletteState {
                    open: true,
                    target: CommandPaletteTarget::DocumentBrowser,
                    selected: 0,
                    pending_action: None,
                };
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
        let layout = self.terminal_layout();
        let previous_window_mode = self.settings.draft.native_startup_window_mode;
        let event = run_terminal_settings_screen(
            ctx,
            &mut self.settings.draft,
            &mut self.terminal_settings_panel,
            &mut self.terminal_nav.settings_idx,
            &mut self.terminal_nav.settings_choice,
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
            TerminalSettingsEvent::OpenConnections => {
                self.apply_terminal_screen_open_plan(terminal_screen_open_plan(
                    TerminalScreen::Connections,
                    0,
                    true,
                ));
            }
            TerminalSettingsEvent::OpenEditMenus => {
                self.navigate_to_screen(TerminalScreen::EditMenus);
                self.terminal_edit_menus.reset();
                self.apply_status_update(clear_shell_status());
            }
            TerminalSettingsEvent::OpenDefaultApps => {
                self.apply_terminal_screen_open_plan(terminal_screen_open_plan(
                    TerminalScreen::DefaultApps,
                    0,
                    true,
                ));
            }
            TerminalSettingsEvent::OpenAbout => {
                self.apply_terminal_screen_open_plan(terminal_screen_open_plan(
                    TerminalScreen::About,
                    0,
                    true,
                ));
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
            self.settings.draft.builtin_menu_visibility.nuke_codes,
            self.settings.draft.builtin_menu_visibility.text_editor,
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
            TerminalEditMenusRequest::None => {}
            TerminalEditMenusRequest::BackToSettings => {
                self.apply_terminal_screen_open_plan(terminal_settings_refresh_plan());
            }
            TerminalEditMenusRequest::PersistToggleBuiltinNukeCodes => {
                self.settings.draft.builtin_menu_visibility.nuke_codes =
                    !self.settings.draft.builtin_menu_visibility.nuke_codes;
                self.persist_native_settings();
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
        ) {
            TerminalAboutRequest::None => {}
            TerminalAboutRequest::Back => {
                self.apply_terminal_screen_open_plan(terminal_settings_refresh_plan());
            }
        }
    }

    pub(super) fn draw_terminal_network(&mut self, ctx: &Context) {
        let layout = self.terminal_layout();
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
        );
        let request = resolve_terminal_catalog_request(event, ProgramCatalog::Network);
        self.apply_terminal_program_request(request, TerminalScreen::Network);
    }

    pub(super) fn draw_terminal_games(&mut self, ctx: &Context) {
        let layout = self.terminal_layout();
        let entries = build_terminal_game_entries(&catalog_names(ProgramCatalog::Games));
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
        );
        let request = resolve_terminal_games_request(event);
        self.apply_terminal_program_request(request, TerminalScreen::Games);
    }

    pub(super) fn draw_terminal_nuke_codes(&mut self, ctx: &Context) {
        let layout = self.terminal_layout();
        match draw_nuke_codes_screen(
            ctx,
            &self.terminal_nuke_codes,
            layout.cols,
            layout.rows,
            layout.header_start_row,
            layout.separator_top_row,
            layout.title_row,
            layout.separator_bottom_row,
            layout.menu_start_row,
            layout.status_row,
            layout.content_col,
        ) {
            NukeCodesEvent::None => {}
            NukeCodesEvent::Refresh => {
                let tx = self.background.sender();
                std::thread::spawn(move || {
                    let view = fetch_nuke_codes();
                    let _ = tx.send(BackgroundResult::NukeCodesFetched(view));
                });
                self.terminal_nuke_codes = NukeCodesView::Unloaded;
            }
            NukeCodesEvent::Back => {
                self.navigate_to_screen(self.terminal_nav.nuke_codes_return_screen);
                self.apply_status_update(clear_shell_status());
            }
        }
    }

    pub(super) fn draw_terminal_pty(&mut self, ctx: &Context) {
        let layout = self.terminal_layout();
        let Some(state) = self.terminal_pty.as_mut() else {
            self.navigate_to_screen(TerminalScreen::MainMenu);
            self.shell_status = "No embedded PTY session.".to_string();
            return;
        };
        let event = draw_embedded_pty(
            ctx,
            state,
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
                if let Some(pty) = self.terminal_pty.take() {
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
        let layout = self.terminal_layout();
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
        );
        self.apply_installer_event(event);
    }

    pub(super) fn draw_terminal_user_management(&mut self, ctx: &Context) {
        let layout = self.terminal_layout();
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

    pub(super) fn draw_terminal_status_bar(&self, ctx: &Context) {
        ctx.request_repaint_after(Self::terminal_status_bar_repaint_interval(ctx));
        let palette = current_palette();
        TopBottomPanel::bottom("native_terminal_status_bar")
            .resizable(false)
            .exact_height(retro_footer_height())
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
