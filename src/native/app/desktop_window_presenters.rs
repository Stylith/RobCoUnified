use super::super::background::BackgroundResult;
use super::super::command_layer::CommandLayerTarget;
use super::super::desktop_app::{DesktopWindow, WindowInstanceId};
use super::super::desktop_settings_service::persist_settings_draft;
use super::super::desktop_status_service::{clear_settings_status, saved_settings_status};
use super::super::desktop_surface_service::{
    desktop_builtin_icons, set_builtin_icon_visible, set_desktop_icon_style,
    set_wallpaper_size_mode as set_desktop_wallpaper_size_mode, wallpaper_browser_start_dir,
};
use super::super::editor_app::{EditorCommand, EditorTextAlign, EDITOR_APP_TITLE};
use super::super::file_manager_desktop;
use super::super::menu::{resolve_desktop_pty_exit, TerminalDesktopPtyExitPlan};
use super::super::nuke_codes_screen::{fetch_nuke_codes, NukeCodesView};
use super::super::pty_screen::{draw_embedded_pty_in_ui_focused, PtyScreenEvent};
use super::super::retro_ui::{current_palette, FIXED_PTY_CELL_H, FIXED_PTY_CELL_W};
use super::super::wasm_addon_runtime::{
    collect_hosted_keyboard_input, draw_hosted_addon_frame, WasmHostedAddonState,
};
use super::desktop_window_mgmt::{
    DesktopHeaderAction, DesktopWindowRectTracking, ResizableDesktopWindowOptions,
};
use super::RobcoNativeApp;
use crate::config::ConnectionKind;
use crate::config::{
    CliAcsMode, CliColorMode, DesktopIconStyle, NativeStartupWindowMode, OpenMode,
    WallpaperSizeMode, CUSTOM_THEME_NAME, THEMES,
};
use eframe::egui::{self, Context, Id, Key, Layout, RichText, TextEdit};
use robcos_native_programs_app::{resolve_desktop_applications_request, DesktopProgramRequest};
use robcos_native_red_menace_app::input_from_ctx as red_menace_input_from_ctx;
use robcos_native_settings_app::{
    desktop_settings_back_target, desktop_settings_connections_nav_items,
    desktop_settings_user_management_nav_items, settings_panel_title, NativeSettingsPanel,
    SettingsHomeTileAction,
};
use robcos_native_zeta_invaders_app::input_from_ctx as zeta_invaders_input_from_ctx;
use robcos_shared::platform::{HostedAddonSize, HostedAddonSurface};
use std::path::PathBuf;

impl RobcoNativeApp {
    pub(super) fn draw_file_manager(&mut self, ctx: &Context) {
        if !self.file_manager.open || self.desktop_window_is_minimized(DesktopWindow::FileManager) {
            return;
        }
        let wid = self.current_window_id(DesktopWindow::FileManager);
        let save_picker_mode = self.editor.save_as_input.is_some();
        let mut open = self.file_manager.open;
        let maximized = self.desktop_window_is_maximized(DesktopWindow::FileManager);
        let restore = self.take_desktop_window_restore_dims(DesktopWindow::FileManager);
        let mut header_action = DesktopHeaderAction::None;
        let generation = self.desktop_window_generation(wid);
        let egui_id = self.desktop_window_egui_id(wid);
        let default_size = Self::desktop_default_window_size(DesktopWindow::FileManager);
        let min_size = Self::desktop_file_manager_window_min_size();
        let save_picker_size = egui::vec2(860.0, 560.0);
        let title = if wid.instance > 0 {
            format!("File Manager [{}]", wid.instance + 1)
        } else {
            "File Manager".to_string()
        };
        let mut window = egui::Window::new(&title)
            .id(egui_id)
            .open(&mut open)
            .title_bar(false)
            .frame(Self::desktop_window_frame())
            .resizable(true)
            .min_size(min_size)
            .default_size([default_size.x, default_size.y]);
        if save_picker_mode {
            window = window.resizable(false);
            if let Some((pos, _)) = restore {
                window = window.current_pos(pos).fixed_size(save_picker_size);
            } else {
                let pos = Self::desktop_default_window_pos(ctx, save_picker_size);
                window = window.current_pos(pos).fixed_size(save_picker_size);
            }
        } else if maximized {
            let rect = Self::desktop_workspace_rect(ctx);
            window = window
                .movable(false)
                .resizable(false)
                .fixed_pos(rect.min)
                .fixed_size(rect.size());
        } else if let Some((pos, size)) = restore {
            let size = Self::desktop_clamp_window_size(ctx, size, min_size);
            let pos = Self::desktop_clamp_window_pos(ctx, pos, size);
            // Unmaximize or first open with a saved size: generation was bumped so egui
            // has no memory for this ID — default_size sets the initial size correctly.
            window = window.current_pos(pos).default_size(size);
        }
        self.file_manager.ensure_selection_valid();
        let rows = self.file_manager.rows();
        let action_selection_paths: Vec<PathBuf> = self
            .file_manager_selected_entries()
            .into_iter()
            .map(|entry| entry.path)
            .collect();
        let has_editable_selection = !action_selection_paths.is_empty();
        let has_single_file_selection =
            action_selection_paths.len() == 1 && action_selection_paths[0].is_file();
        let has_clipboard = self.file_manager_runtime.has_clipboard();
        let desktop_model = file_manager_desktop::build_desktop_view_model(
            &self.file_manager,
            &self.live_desktop_file_manager_settings,
            &rows,
            self.file_manager_selection_count(),
            has_editable_selection,
            has_single_file_selection,
            has_clipboard,
            self.editor.save_as_input.clone(),
            self.picking_icon_for_shortcut,
            self.picking_wallpaper,
        );
        let footer_model = file_manager_desktop::build_footer_model(&desktop_model);

        self.preload_file_manager_svg_previews(ctx, &desktop_model.rows);

        let search_id = Id::new(("native_file_manager_search", wid.instance, generation));
        let shown = window.show(ctx, |ui| {
            Self::apply_settings_control_style(ui);
            self.draw_file_manager_top_panel(
                ctx,
                ui,
                generation,
                maximized,
                save_picker_mode,
                &desktop_model,
                &search_id,
                &mut header_action,
            );
            self.draw_file_manager_footer_panel(ui, generation, save_picker_mode, &footer_model);
            self.draw_file_manager_tree_panel(ui, generation, save_picker_mode, &desktop_model);
            let (open_with_entries, known_app_count) =
                super::file_manager_desktop_presenter::build_open_with_context_entries(
                    &self.file_manager,
                    &self.live_desktop_file_manager_settings,
                );
            self.draw_file_manager_content_panel(
                ctx,
                ui,
                generation,
                save_picker_mode,
                &desktop_model,
                &action_selection_paths,
                has_editable_selection,
                has_single_file_selection,
                has_clipboard,
                &open_with_entries,
                known_app_count,
            );
        });
        let shown_rect = shown.as_ref().map(|inner| inner.response.rect);
        let shown_contains_pointer = shown
            .as_ref()
            .is_some_and(|inner| inner.response.contains_pointer());
        self.maybe_activate_desktop_window_from_click(
            ctx,
            DesktopWindow::FileManager,
            shown_contains_pointer,
        );
        if !maximized && !save_picker_mode {
            // Always save the full rect. egui owns window sizing for resizable windows
            // and will not inflate it — only user drag changes it.
            if let Some(rect) = shown_rect {
                self.note_desktop_window_rect(DesktopWindow::FileManager, rect);
            }
        }
        match header_action {
            DesktopHeaderAction::None => {}
            DesktopHeaderAction::Close => open = false,
            DesktopHeaderAction::Minimize => {
                self.set_desktop_window_minimized(DesktopWindow::FileManager, true)
            }
            DesktopHeaderAction::ToggleMaximize => {
                self.toggle_desktop_window_maximized(DesktopWindow::FileManager, shown_rect)
            }
        }
        // If the inner closure forced file_manager.open to false (e.g. Choose Icon),
        // honour that — the local `open` bool was never updated inside the closure.
        if !self.file_manager.open {
            open = false;
        }
        // If the file manager was closed while in a pick mode, cancel the pick.
        if !open {
            if self.editor.should_close_after_save() {
                self.editor.prompt_close_confirmation();
            }
            self.editor.save_as_input = None;
            self.picking_icon_for_shortcut = None;
            self.picking_wallpaper = false;
        }
        self.update_desktop_window_state(DesktopWindow::FileManager, open);
    }

    pub(super) fn draw_editor(&mut self, ctx: &Context) {
        let terminal_command_layer_open =
            !self.desktop_mode_open && self.command_layer_open_for(CommandLayerTarget::Editor);
        if !self.editor.open {
            return;
        }
        if self.desktop_mode_open && self.desktop_window_is_minimized(DesktopWindow::Editor) {
            return;
        }
        if !terminal_command_layer_open
            && ctx.input(|i| i.key_pressed(Key::S) && i.modifiers.command)
        {
            self.run_editor_command(EditorCommand::Save);
        }
        if !terminal_command_layer_open
            && ctx.input(|i| i.key_pressed(Key::F) && i.modifiers.command)
        {
            self.run_editor_command(EditorCommand::OpenFind);
        }
        if !terminal_command_layer_open
            && ctx.input(|i| i.key_pressed(Key::H) && i.modifiers.command)
        {
            self.run_editor_command(EditorCommand::OpenFindReplace);
        }
        if self.desktop_mode_open
            && ctx.input(|i| i.key_pressed(Key::Escape))
            && self.editor.ui.find_open
        {
            self.run_editor_command(EditorCommand::CloseFind);
        }
        let title = self
            .editor
            .path
            .as_ref()
            .and_then(|p| p.file_name())
            .and_then(|p| p.to_str())
            .unwrap_or(EDITOR_APP_TITLE)
            .to_string();

        if !self.desktop_mode_open {
            if terminal_command_layer_open {
                self.draw_command_layer_at(
                    ctx,
                    CommandLayerTarget::Editor,
                    self.terminal_command_layer_bar_pos(ctx),
                    ctx.screen_rect(),
                );
            }

            if !terminal_command_layer_open {
                // Tab closes the editor (go back)
                if ctx.input(|i| i.key_pressed(Key::Tab)) {
                    self.update_desktop_window_state(DesktopWindow::Editor, false);
                    return;
                }
                // Esc closes the editor (go back)
                if ctx.input(|i| i.key_pressed(Key::Escape)) {
                    if self.editor.ui.find_open {
                        self.run_editor_command(EditorCommand::CloseFind);
                    } else {
                        self.update_desktop_window_state(DesktopWindow::Editor, false);
                    }
                    return;
                }
                // F1 opens the local window menu strip.
                if ctx.input(|i| i.key_pressed(Key::F1)) {
                    self.open_command_layer(CommandLayerTarget::Editor);
                }
                if ctx.input(|i| i.key_pressed(Key::N) && i.modifiers.command) {
                    self.run_editor_command(EditorCommand::NewDocument);
                }
            }

            let palette = current_palette();

            // Editor always draws its CentralPanel (even when palette overlays on top)
            let text_edit_id = Id::new("terminal_editor_text_edit");
            egui::CentralPanel::default()
                .frame(
                    egui::Frame::none()
                        .fill(palette.bg)
                        .inner_margin(egui::Margin::same(4.0)),
                )
                .show(ctx, |ui| {
                    // Header: title + hints
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(&title).color(palette.fg).strong());
                        ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(
                                RichText::new("F1:Menu  Esc:Back  ^S:Save  ^N:New  ^F:Find")
                                    .color(palette.dim)
                                    .small(),
                            );
                        });
                    });
                    if let Some(path) = &self.editor.path {
                        ui.label(
                            RichText::new(path.display().to_string())
                                .color(palette.dim)
                                .small(),
                        );
                    }
                    if !self.editor.status.is_empty() {
                        ui.label(
                            RichText::new(&self.editor.status)
                                .color(palette.dim)
                                .small(),
                        );
                    }

                    // Block cursor in theme color
                    let char_width = 16.0 * 0.6;
                    ui.visuals_mut().text_cursor.stroke = egui::Stroke::new(char_width, palette.fg);
                    let edit = TextEdit::multiline(&mut self.editor.text)
                        .id(text_edit_id)
                        .lock_focus(true)
                        .frame(false)
                        .font(egui::TextStyle::Monospace);
                    let response = ui.add_sized(ui.available_size(), edit);
                    if response.changed() {
                        self.editor.dirty = true;
                    }
                });
            // Auto-focus the text edit so typing works immediately without mouse click
            if !terminal_command_layer_open {
                ctx.memory_mut(|m| m.request_focus(text_edit_id));
            }
            return;
        }

        let wid = self.current_window_id(DesktopWindow::Editor);
        let editor_title = if wid.instance > 0 {
            format!("{} [{}]", title, wid.instance + 1)
        } else {
            title.clone()
        };
        let mut open = self.editor.open;
        let mut header_action = DesktopHeaderAction::None;
        let (window, maximized) = self.build_resizable_desktop_window(
            ctx,
            DesktopWindow::Editor,
            &editor_title,
            &mut open,
            ResizableDesktopWindowOptions {
                min_size: egui::vec2(400.0, 300.0),
                default_size: Self::desktop_default_window_size(DesktopWindow::Editor),
                default_pos: None,
                clamp_restore: false,
            },
        );
        let generation = self.desktop_window_generation(wid);
        let text_edit_id = Id::new(("editor_text_edit", wid.instance, generation));
        let shown = window.show(ctx, |ui| {
            // ── HEADER ───────────────────────────────────────────────────────
            header_action = Self::draw_desktop_window_header(ui, &title, maximized);
            if let Some(path) = &self.editor.path {
                ui.small(path.display().to_string());
            }
            if !self.editor.status.is_empty() {
                ui.small(self.editor.status.clone());
            }

            // ── FIND/REPLACE BAR ─────────────────────────────────────────────
            if self.editor.ui.find_open {
                let palette = current_palette();
                egui::Frame::none()
                    .fill(palette.panel)
                    .inner_margin(egui::Margin::symmetric(4.0, 4.0))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("Find:").color(palette.dim));
                            ui.add_space(4.0);
                            let find_resp = ui.add(
                                TextEdit::singleline(&mut self.editor.ui.find_query)
                                    .desired_width(180.0)
                                    .hint_text("search text"),
                            );
                            if find_resp.lost_focus()
                                && ctx.input(|i| i.key_pressed(egui::Key::Enter))
                            {
                                self.editor_find_next(ctx, text_edit_id);
                            }
                            if ui.button("Find Next").clicked() {
                                self.editor_find_next(ctx, text_edit_id);
                            }
                            if self.editor.ui.find_replace_visible {
                                ui.separator();
                                ui.label(RichText::new("Replace:").color(palette.dim));
                                ui.add(
                                    TextEdit::singleline(&mut self.editor.ui.replace_query)
                                        .desired_width(180.0)
                                        .hint_text("replacement"),
                                );
                                if ui.button("Replace").clicked() {
                                    self.editor_replace_one(ctx, text_edit_id);
                                }
                                if ui.button("Replace All").clicked() {
                                    self.editor_replace_all();
                                }
                            }
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if ui.button("[X]").clicked() {
                                        self.run_editor_command(EditorCommand::CloseFind);
                                    }
                                },
                            );
                        });
                    });
            }

            // ── TEXT EDITOR AREA ─────────────────────────────────────────────
            let palette = current_palette();
            let char_width = self.editor.font_size * 0.6;
            ui.visuals_mut().text_cursor.stroke = egui::Stroke::new(char_width, palette.fg);
            if (self.editor.font_size - 16.0).abs() > 0.1 {
                ui.style_mut().text_styles.insert(
                    egui::TextStyle::Monospace,
                    egui::FontId::new(self.editor.font_size, egui::FontFamily::Monospace),
                );
            }
            let text_align = match self.editor.ui.text_align {
                EditorTextAlign::Center => egui::Align::Center,
                EditorTextAlign::Right => egui::Align::RIGHT,
                EditorTextAlign::Left => egui::Align::LEFT,
            };

            // Host the editor inside a bounded scroll region so overflowing text does not
            // push the outer window taller than the user-sized frame.
            let remaining = ui.available_size();
            egui::ScrollArea::both()
                .auto_shrink([false, false])
                .max_height(remaining.y)
                .max_width(remaining.x)
                .show(ui, |ui| {
                    let mut edit = TextEdit::multiline(&mut self.editor.text)
                        .id(text_edit_id)
                        .lock_focus(true)
                        .frame(false)
                        .font(egui::TextStyle::Monospace)
                        .horizontal_align(text_align);
                    if !self.editor.word_wrap {
                        edit = edit.desired_width(f32::INFINITY);
                    }
                    let response = ui.add_sized(remaining, edit);
                    Self::attach_generic_context_menu(&mut self.context_menu_action, &response);
                    if response.changed() {
                        self.editor.dirty = true;
                    }
                });
        });
        let shown_rect = shown.as_ref().map(|inner| inner.response.rect);
        let shown_contains_pointer = shown
            .as_ref()
            .is_some_and(|inner| inner.response.contains_pointer());
        self.finish_desktop_window_host(
            ctx,
            DesktopWindow::Editor,
            &mut open,
            maximized,
            shown_rect,
            shown_contains_pointer,
            DesktopWindowRectTracking::FullRect,
            header_action,
        );
        if self.editor.close_confirmation_visible() {
            let palette = current_palette();
            let mut action: Option<&'static str> = None;
            egui::Window::new("editor_close_confirm")
                .id(Id::new(("editor_close_confirm", wid.instance, generation)))
                .title_bar(false)
                .collapsible(false)
                .resizable(false)
                .fixed_size(egui::vec2(360.0, 132.0))
                .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
                .frame(Self::desktop_window_frame())
                .show(ctx, |ui| {
                    Self::apply_settings_control_style(ui);
                    ui.add_space(6.0);
                    ui.label(
                        RichText::new("Are you sure you want to quit?")
                            .strong()
                            .color(palette.fg),
                    );
                    ui.add_space(14.0);
                    ui.horizontal(|ui| {
                        if ui.button("Save").clicked() {
                            action = Some("save");
                        }
                        if ui.button("Cancel").clicked() {
                            action = Some("cancel");
                        }
                        if ui.button("Quit").clicked() {
                            action = Some("quit");
                        }
                    });
                });

            match action {
                Some("save") => self.confirm_editor_close_save(),
                Some("cancel") => self.editor.cancel_close_confirmation(),
                Some("quit") => {
                    self.editor.cancel_close_confirmation();
                    self.close_current_editor_window_unchecked();
                }
                _ => {}
            }
        }
    }

    pub(super) fn draw_settings(&mut self, ctx: &Context) {
        if !self.settings.open || self.desktop_window_is_minimized(DesktopWindow::Settings) {
            return;
        }
        let wid = self.current_window_id(DesktopWindow::Settings);
        let mut open = self.settings.open;
        let maximized = self.desktop_window_is_maximized(DesktopWindow::Settings);
        let restore = self.take_desktop_window_restore_dims(DesktopWindow::Settings);
        let mut header_action = DesktopHeaderAction::None;
        let egui_id = self.desktop_window_egui_id(wid);
        let default_size = Self::desktop_default_window_size(DesktopWindow::Settings);
        let default_pos = Self::desktop_default_window_pos(ctx, default_size);
        let mut window = egui::Window::new("Settings")
            .id(egui_id)
            .open(&mut open)
            .title_bar(false)
            .frame(Self::desktop_window_frame())
            .resizable(false)
            .default_pos(default_pos)
            .fixed_size(default_size);
        if maximized {
            let rect = Self::desktop_workspace_rect(ctx);
            window = window
                .movable(false)
                .fixed_pos(rect.min)
                .fixed_size(rect.size());
        } else if let Some((pos, _size)) = restore {
            // Settings stays fixed-size in desktop mode; only restore position after
            // un-maximize or other lifecycle hops that bump the egui window ID.
            let pos = Self::desktop_clamp_window_pos(ctx, pos, default_size);
            window = window.current_pos(pos);
        }
        let mut close_requested = false;
        let shown = window.show(ctx, |ui| {
            Self::apply_settings_control_style(ui);
            header_action = Self::draw_desktop_window_header(ui, "Settings", maximized);
            let is_admin = self.session.as_ref().is_some_and(|s| s.is_admin);
            let panel = self.settings.panel;
            let mut changed = false;
            let mut window_mode_changed = false;
            let mut next_panel = None;

            let panel_title = settings_panel_title(panel);

            ui.add_space(4.0);
            if matches!(panel, NativeSettingsPanel::Home) {
                ui.label(RichText::new("Settings").strong().size(28.0));
                ui.add_space(14.0);
            } else {
                ui.horizontal(|ui| {
                    if ui.button("Back").clicked() {
                        next_panel = Some(desktop_settings_back_target(panel));
                    }
                    ui.strong(panel_title);
                });
                ui.separator();
                ui.add_space(4.0);
            }

            match panel {
                NativeSettingsPanel::Home => {
                    let rows = self.settings_home_rows_for_session(is_admin);
                    let tile_w = 140.0;
                    let tile_h = 112.0;
                    let gap_x = 34.0;
                    let row_gap = 24.0;
                    let icon_font_size = 22.0;
                    let label_font_size = 22.0;

                    ui.add_space(6.0);

                    for (row_idx, row) in rows.iter().enumerate() {
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = gap_x;
                            for tile in row {
                                let panel_texture = match tile.action {
                                    SettingsHomeTileAction::OpenPanel(panel) => {
                                        self.settings_panel_texture(ctx, panel)
                                    }
                                    SettingsHomeTileAction::CloseWindow => None,
                                };
                                let response = Self::retro_settings_tile(
                                    ui,
                                    panel_texture.as_ref(),
                                    tile.icon,
                                    tile.label,
                                    tile.enabled,
                                    egui::vec2(tile_w, tile_h),
                                    icon_font_size,
                                    label_font_size,
                                );
                                if response.clicked() {
                                    match tile.action {
                                        SettingsHomeTileAction::CloseWindow => {
                                            close_requested = true;
                                        }
                                        SettingsHomeTileAction::OpenPanel(panel) => {
                                            next_panel = Some(panel);
                                        }
                                    }
                                }
                            }
                            for _ in row.len()..4 {
                                ui.add_space(tile_w);
                            }
                        });
                        ui.add_space(if row_idx == rows.len() - 1 {
                            0.0
                        } else {
                            row_gap
                        });
                    }
                    if !is_admin {
                        ui.small("User Management requires an admin session.");
                    }
                }
                _ => {
                    let body_max_height = ui.available_height().max(120.0);
                    egui::ScrollArea::vertical()
                        .auto_shrink([false, false])
                        .max_height(body_max_height)
                        .show(ui, |ui| match panel {
                            NativeSettingsPanel::General => {
                                Self::settings_two_columns(ui, |left, right| {
                                    Self::settings_section(left, "Startup", |left| {
                                        left.label("Default Open Mode");
                                        left.horizontal(|ui| {
                                            if Self::retro_choice_button(
                                                ui,
                                                "Terminal",
                                                self.settings.draft.default_open_mode
                                                    == OpenMode::Terminal,
                                            )
                                            .clicked()
                                                && self.settings.draft.default_open_mode
                                                    != OpenMode::Terminal
                                            {
                                                self.settings.draft.default_open_mode =
                                                    OpenMode::Terminal;
                                                changed = true;
                                            }
                                            if Self::retro_choice_button(
                                                ui,
                                                "Desktop",
                                                self.settings.draft.default_open_mode
                                                    == OpenMode::Desktop,
                                            )
                                            .clicked()
                                                && self.settings.draft.default_open_mode
                                                    != OpenMode::Desktop
                                            {
                                                self.settings.draft.default_open_mode =
                                                    OpenMode::Desktop;
                                                changed = true;
                                            }
                                        });
                                        left.add_space(8.0);
                                        left.small(
                                            "Choose which interface opens first after login.",
                                        );
                                    });

                                    Self::settings_section(right, "Options", |right| {
                                        let palette = current_palette();
                                        if Self::retro_checkbox_row(
                                            right,
                                            &mut self.settings.draft.sound,
                                            "Enable sound",
                                        )
                                        .clicked()
                                        {
                                            changed = true;
                                        }
                                        right.add_space(8.0);
                                        right.label("System sound volume");
                                        right.visuals_mut().selection.bg_fill = palette.fg;
                                        right.visuals_mut().widgets.inactive.bg_fill = palette.dim;
                                        if right
                                            .add(
                                                egui::Slider::new(
                                                    &mut self.settings.draft.system_sound_volume,
                                                    0..=100,
                                                )
                                                .suffix("%"),
                                            )
                                            .changed()
                                        {
                                            changed = true;
                                        }
                                        if Self::retro_checkbox_row(
                                            right,
                                            &mut self.settings.draft.bootup,
                                            "Play bootup on login",
                                        )
                                        .clicked()
                                        {
                                            changed = true;
                                        }
                                        if Self::retro_checkbox_row(
                                            right,
                                            &mut self.settings.draft.show_navigation_hints,
                                            "Show navigation hints",
                                        )
                                        .clicked()
                                        {
                                            changed = true;
                                        }
                                    });
                                });
                            }
                            NativeSettingsPanel::Appearance => {
                                let palette = current_palette();
                                // ── Tab bar ───────────────────────────────────────────────────
                                let tabs = ["Background", "Display", "Colors", "Icons", "Terminal"];
                                ui.horizontal(|ui| {
                                    for (i, label) in tabs.iter().enumerate() {
                                        let active = self.appearance_tab == i as u8;
                                        let color = if active { palette.fg } else { palette.dim };
                                        let btn = ui.add(
                                            egui::Button::new(
                                                RichText::new(*label).color(color).strong(),
                                            )
                                            .stroke(egui::Stroke::new(
                                                if active { 2.0 } else { 1.0 },
                                                color,
                                            ))
                                            .fill(if active { palette.panel } else { palette.bg }),
                                        );
                                        if btn.clicked() {
                                            self.appearance_tab = i as u8;
                                        }
                                    }
                                });
                                ui.add_space(10.0);
                                Self::retro_separator(ui);
                                ui.add_space(8.0);
                                // ── Tab content ───────────────────────────────────────────────
                                match self.appearance_tab {
                                    // ── Background ─────────────────────────────────────────────
                                    0 => {
                                        Self::settings_section(ui, "Wallpaper", |ui| {
                                            ui.label("Wallpaper Path");
                                            ui.horizontal(|ui| {
                                                let w = Self::responsive_input_width(
                                                    ui, 0.72, 160.0, 400.0,
                                                );
                                                if ui
                                                    .add(
                                                        TextEdit::singleline(
                                                            &mut self
                                                                .settings
                                                                .draft
                                                                .desktop_wallpaper,
                                                        )
                                                        .desired_width(w)
                                                        .hint_text("/path/to/image.png"),
                                                    )
                                                    .changed()
                                                {
                                                    changed = true;
                                                }
                                                if ui.button("Browse…").clicked() {
                                                    let start = wallpaper_browser_start_dir(
                                                        &self.settings.draft.desktop_wallpaper,
                                                    );
                                                    self.picking_wallpaper = true;
                                                    self.open_embedded_file_manager_at(start);
                                                }
                                            });
                                            ui.add_space(8.0);
                                            ui.horizontal(|ui| {
                                                ui.label("Wallpaper Mode");
                                                let selected = match self
                                                    .settings
                                                    .draft
                                                    .desktop_wallpaper_size_mode
                                                {
                                                    WallpaperSizeMode::DefaultSize => {
                                                        "Default Size"
                                                    }
                                                    WallpaperSizeMode::FitToScreen => {
                                                        "Fit To Screen"
                                                    }
                                                    WallpaperSizeMode::Centered => "Centered",
                                                    WallpaperSizeMode::Tile => "Tile",
                                                    WallpaperSizeMode::Stretch => "Stretch",
                                                };
                                                egui::ComboBox::from_id_salt(
                                                    "native_settings_wallpaper_mode",
                                                )
                                                .selected_text(
                                                    RichText::new(selected).color(palette.fg),
                                                )
                                                .show_ui(ui, |ui| {
                                                    Self::apply_settings_control_style(ui);
                                                    for (mode, label) in [
                                                        (
                                                            WallpaperSizeMode::DefaultSize,
                                                            "Default Size",
                                                        ),
                                                        (
                                                            WallpaperSizeMode::FitToScreen,
                                                            "Fit To Screen",
                                                        ),
                                                        (WallpaperSizeMode::Centered, "Centered"),
                                                        (WallpaperSizeMode::Tile, "Tile"),
                                                        (WallpaperSizeMode::Stretch, "Stretch"),
                                                    ] {
                                                        if Self::retro_choice_button(
                                                            ui,
                                                            label,
                                                            self.settings
                                                                .draft
                                                                .desktop_wallpaper_size_mode
                                                                == mode,
                                                        )
                                                        .clicked()
                                                        {
                                                            set_desktop_wallpaper_size_mode(
                                                                &mut self.settings.draft,
                                                                mode,
                                                            );
                                                            changed = true;
                                                            ui.close_menu();
                                                        }
                                                    }
                                                });
                                            });
                                        });
                                    }
                                    1 => {
                                        Self::settings_section(ui, "Window", |ui| {
                                            ui.label("Window Mode");
                                            ui.horizontal_wrapped(|ui| {
                                                for mode in [
                                                    NativeStartupWindowMode::Windowed,
                                                    NativeStartupWindowMode::Maximized,
                                                    NativeStartupWindowMode::BorderlessFullscreen,
                                                    NativeStartupWindowMode::Fullscreen,
                                                ] {
                                                    if Self::retro_choice_button(
                                                        ui,
                                                        mode.label(),
                                                        self.settings.draft
                                                            .native_startup_window_mode
                                                            == mode,
                                                    )
                                                    .clicked()
                                                        && self.settings.draft
                                                            .native_startup_window_mode
                                                            != mode
                                                    {
                                                        self.settings.draft
                                                            .native_startup_window_mode = mode;
                                                        changed = true;
                                                        window_mode_changed = true;
                                                    }
                                                }
                                            });
                                            ui.add_space(8.0);
                                            ui.small(
                                                "Applies immediately and persists across launches. Windowed is the safest mode on older GPUs.",
                                            );
                                        });
                                        ui.add_space(10.0);
                                        changed |= self.draw_settings_display_effects_panel(ui);
                                    }
                                    // ── Colors ─────────────────────────────────────────────────
                                    2 => {
                                        Self::settings_section(ui, "Theme Color", |ui| {
                                            ui.horizontal(|ui| {
                                                ui.label("Theme");
                                                let mut current_idx = THEMES
                                                    .iter()
                                                    .position(|(name, _)| {
                                                        *name == self.settings.draft.theme
                                                    })
                                                    .unwrap_or(0);
                                                egui::ComboBox::from_id_salt(
                                                    "native_settings_theme",
                                                )
                                                .selected_text(
                                                    RichText::new(THEMES[current_idx].0)
                                                        .color(palette.fg),
                                                )
                                                .show_ui(ui, |ui| {
                                                    Self::apply_settings_control_style(ui);
                                                    for (idx, (name, _)) in
                                                        THEMES.iter().enumerate()
                                                    {
                                                        if Self::retro_choice_button(
                                                            ui,
                                                            *name,
                                                            current_idx == idx,
                                                        )
                                                        .clicked()
                                                        {
                                                            current_idx = idx;
                                                            self.settings.draft.theme =
                                                                (*name).to_string();
                                                            changed = true;
                                                            ui.close_menu();
                                                        }
                                                    }
                                                });
                                            });
                                            if self.settings.draft.theme == CUSTOM_THEME_NAME {
                                                let mut rgb = self.settings.draft.custom_theme_rgb;
                                                let preview_color =
                                                    egui::Color32::from_rgb(rgb[0], rgb[1], rgb[2]);
                                                // Make slider rails visible: track in custom color,
                                                // unfilled portion in dim. Without this, the rail
                                                // is BLACK-on-BLACK (invisible) due to settings style.
                                                ui.visuals_mut().selection.bg_fill = preview_color;
                                                ui.visuals_mut().widgets.inactive.bg_fill =
                                                    palette.dim;
                                                changed |= ui
                                                    .add(
                                                        egui::Slider::new(&mut rgb[0], 0..=255)
                                                            .text("Red"),
                                                    )
                                                    .changed();
                                                changed |= ui
                                                    .add(
                                                        egui::Slider::new(&mut rgb[1], 0..=255)
                                                            .text("Green"),
                                                    )
                                                    .changed();
                                                changed |= ui
                                                    .add(
                                                        egui::Slider::new(&mut rgb[2], 0..=255)
                                                            .text("Blue"),
                                                    )
                                                    .changed();
                                                if rgb != self.settings.draft.custom_theme_rgb {
                                                    self.settings.draft.custom_theme_rgb = rgb;
                                                }
                                            }
                                        });
                                    }
                                    // ── Icons ──────────────────────────────────────────────────
                                    3 => {
                                        Self::settings_section(ui, "Desktop Icons", |ui| {
                                            ui.horizontal(|ui| {
                                                ui.label("Icon Style");
                                                let selected =
                                                    match self.settings.draft.desktop_icon_style {
                                                        DesktopIconStyle::Dos => "DOS",
                                                        DesktopIconStyle::Win95 => "Win95",
                                                        DesktopIconStyle::Minimal => "Minimal",
                                                        DesktopIconStyle::NoIcons => "No Icons",
                                                    };
                                                egui::ComboBox::from_id_salt(
                                                    "native_settings_desktop_icons",
                                                )
                                                .selected_text(
                                                    RichText::new(selected).color(palette.fg),
                                                )
                                                .show_ui(ui, |ui| {
                                                    Self::apply_settings_control_style(ui);
                                                    for (style, label) in [
                                                        (DesktopIconStyle::Dos, "DOS"),
                                                        (DesktopIconStyle::Win95, "Win95"),
                                                        (DesktopIconStyle::Minimal, "Minimal"),
                                                        (DesktopIconStyle::NoIcons, "No Icons"),
                                                    ] {
                                                        if Self::retro_choice_button(
                                                            ui,
                                                            label,
                                                            self.settings.draft.desktop_icon_style
                                                                == style,
                                                        )
                                                        .clicked()
                                                        {
                                                            set_desktop_icon_style(
                                                                &mut self.settings.draft,
                                                                style,
                                                            );
                                                            changed = true;
                                                            ui.close_menu();
                                                        }
                                                    }
                                                });
                                            });
                                            ui.add_space(8.0);
                                            ui.label(
                                                RichText::new("Built-in Desktop Icons")
                                                    .color(palette.fg)
                                                    .strong(),
                                            );
                                            ui.add_space(4.0);
                                            for entry in desktop_builtin_icons() {
                                                let mut visible = !self
                                                    .settings
                                                    .draft
                                                    .desktop_hidden_builtin_icons
                                                    .contains(entry.key);
                                                if Self::retro_checkbox_row(
                                                    ui,
                                                    &mut visible,
                                                    &format!("Show {}", entry.label),
                                                )
                                                .clicked()
                                                {
                                                    set_builtin_icon_visible(
                                                        &mut self.settings.draft,
                                                        entry.key,
                                                        visible,
                                                    );
                                                    changed = true;
                                                }
                                            }
                                            ui.add_space(8.0);
                                            if Self::retro_checkbox_row(
                                                ui,
                                                &mut self.settings.draft.desktop_show_cursor,
                                                "Show desktop cursor",
                                            )
                                            .clicked()
                                            {
                                                changed = true;
                                            }
                                            if self.settings.draft.desktop_show_cursor {
                                                ui.add_space(6.0);
                                                ui.scope(|ui| {
                                                    ui.visuals_mut().selection.bg_fill = palette.fg;
                                                    ui.visuals_mut().widgets.inactive.bg_fill =
                                                        palette.dim;
                                                    changed |= ui
                                                        .add(
                                                            egui::Slider::new(
                                                                &mut self
                                                                    .settings
                                                                    .draft
                                                                    .desktop_cursor_scale,
                                                                0.5..=2.5,
                                                            )
                                                            .text("Cursor Size"),
                                                        )
                                                        .changed();
                                                });
                                            }
                                        });
                                    }
                                    // ── Terminal ───────────────────────────────────────────────
                                    _ => {
                                        Self::settings_section(ui, "PTY Display", |ui| {
                                            if Self::retro_checkbox_row(
                                                ui,
                                                &mut self.settings.draft.cli_styled_render,
                                                "Styled PTY rendering",
                                            )
                                            .clicked()
                                            {
                                                changed = true;
                                            }
                                            ui.add_space(8.0);
                                            ui.horizontal(|ui| {
                                                ui.label("PTY Color Mode");
                                                let selected =
                                                    match self.settings.draft.cli_color_mode {
                                                        CliColorMode::ThemeLock => "Theme Lock",
                                                        CliColorMode::PaletteMap => "Palette-map",
                                                        CliColorMode::Color => "Color",
                                                        CliColorMode::Monochrome => "Monochrome",
                                                    };
                                                egui::ComboBox::from_id_salt(
                                                    "native_settings_cli_color",
                                                )
                                                .selected_text(
                                                    RichText::new(selected).color(palette.fg),
                                                )
                                                .show_ui(ui, |ui| {
                                                    Self::apply_settings_control_style(ui);
                                                    for (mode, label) in [
                                                        (CliColorMode::ThemeLock, "Theme Lock"),
                                                        (CliColorMode::PaletteMap, "Palette-map"),
                                                        (CliColorMode::Color, "Color"),
                                                        (CliColorMode::Monochrome, "Monochrome"),
                                                    ] {
                                                        if Self::retro_choice_button(
                                                            ui,
                                                            label,
                                                            self.settings.draft.cli_color_mode
                                                                == mode,
                                                        )
                                                        .clicked()
                                                            && self.settings.draft.cli_color_mode
                                                                != mode
                                                        {
                                                            self.settings.draft.cli_color_mode =
                                                                mode;
                                                            changed = true;
                                                            ui.close_menu();
                                                        }
                                                    }
                                                });
                                            });
                                            ui.add_space(8.0);
                                            if ui
                                                .button(match self.settings.draft.cli_acs_mode {
                                                    CliAcsMode::Ascii => "Border Glyphs: ASCII",
                                                    CliAcsMode::Unicode => {
                                                        "Border Glyphs: Unicode Smooth"
                                                    }
                                                })
                                                .clicked()
                                            {
                                                self.settings.draft.cli_acs_mode =
                                                    match self.settings.draft.cli_acs_mode {
                                                        CliAcsMode::Ascii => CliAcsMode::Unicode,
                                                        CliAcsMode::Unicode => CliAcsMode::Ascii,
                                                    };
                                                changed = true;
                                            }
                                        });
                                    }
                                }
                            }
                            NativeSettingsPanel::DefaultApps => {
                                changed |= self.draw_settings_default_apps_panel(ui);
                            }
                            NativeSettingsPanel::Connections => {
                                ui.vertical(|ui| {
                                    for item in desktop_settings_connections_nav_items() {
                                        if Self::retro_full_width_button(ui, item.label).clicked() {
                                            next_panel = Some(item.panel);
                                        }
                                    }
                                });
                            }
                            NativeSettingsPanel::ConnectionsNetwork => {
                                self.draw_settings_connections_kind_panel(
                                    ui,
                                    ConnectionKind::Network,
                                );
                            }
                            NativeSettingsPanel::ConnectionsBluetooth => {
                                self.draw_settings_connections_kind_panel(
                                    ui,
                                    ConnectionKind::Bluetooth,
                                );
                            }
                            NativeSettingsPanel::CliProfiles => {
                                changed |= self.draw_settings_cli_profiles_panel(ui);
                            }
                            NativeSettingsPanel::EditMenus => {
                                changed |= self.draw_settings_edit_menus_panel(ui);
                            }
                            NativeSettingsPanel::UserManagement => {
                                if is_admin {
                                    ui.vertical(|ui| {
                                        for item in desktop_settings_user_management_nav_items() {
                                            if Self::retro_full_width_button(ui, item.label)
                                                .clicked()
                                            {
                                                next_panel = Some(item.panel);
                                            }
                                        }
                                    });
                                } else {
                                    ui.small("User Management requires an admin session.");
                                }
                            }
                            NativeSettingsPanel::UserManagementViewUsers => {
                                if is_admin {
                                    self.draw_settings_user_view_panel(ui);
                                } else {
                                    ui.small("User Management requires an admin session.");
                                }
                            }
                            NativeSettingsPanel::UserManagementCreateUser => {
                                if is_admin {
                                    self.draw_settings_user_create_panel(ui);
                                } else {
                                    ui.small("User Management requires an admin session.");
                                }
                            }
                            NativeSettingsPanel::UserManagementEditUsers => {
                                if is_admin {
                                    self.draw_settings_user_edit_panel(ui, false);
                                } else {
                                    ui.small("User Management requires an admin session.");
                                }
                            }
                            NativeSettingsPanel::UserManagementEditCurrentUser => {
                                if is_admin {
                                    self.draw_settings_user_edit_panel(ui, true);
                                } else {
                                    ui.small("User Management requires an admin session.");
                                }
                            }
                            NativeSettingsPanel::About => {
                                ui.label(format!("Version: v{}", env!("CARGO_PKG_VERSION")));
                                ui.label(format!("Theme: {}", self.settings.draft.theme));
                                ui.label(format!(
                                    "Default Open Mode: {}",
                                    match self.settings.draft.default_open_mode {
                                        OpenMode::Terminal => "Terminal",
                                        OpenMode::Desktop => "Desktop",
                                    }
                                ));
                                ui.label(format!(
                                    "Window Mode: {}",
                                    self.settings.draft.native_startup_window_mode.label()
                                ));
                            }
                            NativeSettingsPanel::Home => {}
                        });
                }
            }

            if let Some(panel) = next_panel {
                self.settings.panel = panel;
                self.apply_status_update(clear_settings_status());
            }
            ui.separator();
            if changed {
                {
                    let draft = self.settings.draft.clone();
                    let tx = self.background.sender();
                    std::thread::spawn(move || {
                        persist_settings_draft(&draft);
                        let _ = tx.send(BackgroundResult::SettingsPersisted);
                    });
                }
                self.sync_runtime_settings_cache();
                self.invalidate_desktop_icon_layout_cache();
                self.invalidate_program_catalog_cache();
                self.invalidate_saved_connections_cache();
                self.refresh_settings_sync_marker();
                if window_mode_changed {
                    self.apply_native_window_mode(ctx);
                }
                self.apply_status_update(saved_settings_status());
            }
            if !self.settings.status.is_empty() {
                ui.small(&self.settings.status);
            }
        });
        if close_requested {
            open = false;
        }
        let shown_rect = shown.as_ref().map(|inner| inner.response.rect);
        let shown_contains_pointer = shown
            .as_ref()
            .is_some_and(|inner| inner.response.contains_pointer());
        self.finish_desktop_window_host(
            ctx,
            DesktopWindow::Settings,
            &mut open,
            maximized,
            shown_rect,
            shown_contains_pointer,
            DesktopWindowRectTracking::PositionOnly,
            header_action,
        );
    }

    pub(super) fn draw_applications(&mut self, ctx: &Context) {
        if !self.applications.open || self.desktop_window_is_minimized(DesktopWindow::Applications)
        {
            return;
        }
        let mut open = self.applications.open;
        let mut close_after_launch = false;
        let mut header_action = DesktopHeaderAction::None;
        let (window, maximized) = self.build_resizable_desktop_window(
            ctx,
            DesktopWindow::Applications,
            "Applications",
            &mut open,
            ResizableDesktopWindowOptions {
                min_size: egui::vec2(320.0, 250.0),
                default_size: Self::desktop_default_window_size(DesktopWindow::Applications),
                default_pos: None,
                clamp_restore: false,
            },
        );
        let shown = window.show(ctx, |ui| {
            Self::apply_settings_control_style(ui);
            header_action = Self::draw_desktop_window_header(ui, "Applications", maximized);
            let sections = self.desktop_applications_sections();
            let body_max_height = ui.available_height().max(120.0);
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .max_height(body_max_height)
                .show(ui, |ui| {
                    ui.heading("Built-in");
                    for entry in &sections.builtins {
                        if Self::retro_full_width_button(ui, entry.label.as_str()).clicked() {
                            let request = resolve_desktop_applications_request(&entry.action);
                            close_after_launch = matches!(
                                request,
                                DesktopProgramRequest::OpenNukeCodes { close_window: true }
                                    | DesktopProgramRequest::LaunchCatalog {
                                        close_window: true,
                                        ..
                                    }
                            );
                            self.apply_desktop_program_request(request);
                        }
                    }
                    ui.separator();
                    ui.heading("Configured Apps");
                    for entry in &sections.configured {
                        if Self::retro_full_width_button(ui, entry.label.as_str()).clicked() {
                            let request = resolve_desktop_applications_request(&entry.action);
                            close_after_launch = matches!(
                                request,
                                DesktopProgramRequest::OpenNukeCodes { close_window: true }
                                    | DesktopProgramRequest::LaunchCatalog {
                                        close_window: true,
                                        ..
                                    }
                            );
                            self.apply_desktop_program_request(request);
                        }
                    }
                    if !self.applications.status.is_empty() {
                        ui.separator();
                        ui.small(&self.applications.status);
                    }
                });
        });
        let shown_rect = shown.as_ref().map(|inner| inner.response.rect);
        let shown_contains_pointer = shown
            .as_ref()
            .is_some_and(|inner| inner.response.contains_pointer());
        if close_after_launch {
            open = false;
        }
        self.finish_desktop_window_host(
            ctx,
            DesktopWindow::Applications,
            &mut open,
            maximized,
            shown_rect,
            shown_contains_pointer,
            DesktopWindowRectTracking::FullRect,
            header_action,
        );
    }

    pub(super) fn draw_nuke_codes_window(&mut self, ctx: &Context) {
        if !self.desktop_nuke_codes_open
            || self.desktop_window_is_minimized(DesktopWindow::NukeCodes)
        {
            return;
        }
        let mut open = self.desktop_nuke_codes_open;
        let mut header_action = DesktopHeaderAction::None;
        let mut refresh = false;
        let (window, maximized) = self.build_resizable_desktop_window(
            ctx,
            DesktopWindow::NukeCodes,
            "Nuke Codes",
            &mut open,
            ResizableDesktopWindowOptions {
                min_size: egui::vec2(300.0, 200.0),
                default_size: Self::desktop_default_window_size(DesktopWindow::NukeCodes),
                default_pos: None,
                clamp_restore: false,
            },
        );
        let shown = window.show(ctx, |ui| {
            Self::apply_settings_control_style(ui);
            let title = self
                .desktop_nuke_codes_wasm
                .as_ref()
                .map(|state| state.title().to_string())
                .unwrap_or_else(|| "Nuke Codes".to_string());
            header_action = Self::draw_desktop_window_header(ui, &title, maximized);
            if Self::retro_full_width_button(ui, "Refresh").clicked() {
                refresh = true;
            }
            ui.separator();
            ui.add_space(12.0);
            if self.nuke_codes_uses_wasm_addon() {
                let available = ui.available_size_before_wrap();
                let size = HostedAddonSize {
                    width: available.x.max(320.0),
                    height: available.y.max(200.0),
                };
                let dt = ctx.input(|i| i.stable_dt).max(1.0 / 60.0);
                let result = (|| -> Result<(), String> {
                    let module = super::super::installed_wasm_addon_module(
                        &crate::platform::AddonId::from("tools.nuke-codes"),
                    )
                    .ok_or_else(|| "Installed WASM module disappeared.".to_string())?;
                    if self.desktop_nuke_codes_wasm.is_none() {
                        self.desktop_nuke_codes_wasm = Some(WasmHostedAddonState::spawn(
                            &module,
                            HostedAddonSurface::Desktop,
                            size,
                        )?);
                    }
                    if let Some(state) = self.desktop_nuke_codes_wasm.as_mut() {
                        state.update(size, dt, Vec::new())?;
                        draw_hosted_addon_frame(ui, state);
                        if let Some(status) = &state.frame().status_line {
                            ui.add_space(8.0);
                            ui.small(status);
                        }
                    }
                    Ok(())
                })();
                if let Err(err) = result {
                    self.desktop_nuke_codes_wasm = None;
                    ui.monospace("WASM ADDON FAILED");
                    ui.small(err);
                }
            } else {
                match &self.terminal_nuke_codes {
                    NukeCodesView::Unloaded => {
                        ui.monospace("Codes are not loaded yet.");
                    }
                    NukeCodesView::Error(err) => {
                        ui.monospace("UNABLE TO FETCH LIVE CODES");
                        ui.small(format!("ERROR: {err}"));
                    }
                    NukeCodesView::Data(codes) => {
                        ui.monospace(format!("ALPHA   : {}", codes.alpha));
                        ui.monospace(format!("BRAVO   : {}", codes.bravo));
                        ui.monospace(format!("CHARLIE : {}", codes.charlie));
                        ui.add_space(6.0);
                        ui.small(format!("Source: {}", codes.source));
                        ui.small(format!("Fetched: {}", codes.fetched_at));
                    }
                }
            }
        });
        let shown_rect = shown.as_ref().map(|inner| inner.response.rect);
        let shown_contains_pointer = shown
            .as_ref()
            .is_some_and(|inner| inner.response.contains_pointer());
        // Context menus are attached to specific content widgets inside the
        // window closure, not to the outer Area response (which causes
        // "double use of widget" ID collisions in egui 0.29).
        self.maybe_activate_desktop_window_from_click(
            ctx,
            DesktopWindow::NukeCodes,
            shown_contains_pointer,
        );
        if refresh {
            if self.nuke_codes_uses_wasm_addon() {
                self.desktop_nuke_codes_wasm = None;
            } else {
                let tx = self.background.sender();
                std::thread::spawn(move || {
                    let view = fetch_nuke_codes();
                    let _ = tx.send(BackgroundResult::NukeCodesFetched(view));
                });
                self.terminal_nuke_codes = NukeCodesView::Unloaded;
            }
        }
        self.finish_desktop_window_host(
            ctx,
            DesktopWindow::NukeCodes,
            &mut open,
            maximized,
            shown_rect,
            shown_contains_pointer,
            DesktopWindowRectTracking::FullRect,
            header_action,
        );
    }

    pub(super) fn draw_zeta_invaders_window(&mut self, ctx: &Context) {
        if !self.zeta_invaders.open || self.desktop_window_is_minimized(DesktopWindow::ZetaInvaders)
        {
            return;
        }
        let input_enabled = self.active_window_kind() == Some(DesktopWindow::ZetaInvaders)
            && !self.start_open
            && !self.spotlight_open
            && self.terminal_prompt.is_none();
        let dt = Self::next_embedded_game_dt(&mut self.zeta_invaders.last_frame_at);
        if !self.zeta_invaders_uses_wasm_addon() {
            let input = if input_enabled {
                zeta_invaders_input_from_ctx(ctx)
            } else {
                Default::default()
            };
            self.zeta_invaders.game.update(&input, dt);
            if self.zeta_invaders.atlas.is_none() {
                self.zeta_invaders.atlas =
                    Some(robcos_native_zeta_invaders_app::AtlasTextures::new(ctx));
            }
        }

        let mut open = self.zeta_invaders.open;
        let mut header_action = DesktopHeaderAction::None;
        let (window, maximized) = self.build_resizable_desktop_window(
            ctx,
            DesktopWindow::ZetaInvaders,
            "Zeta Invaders",
            &mut open,
            ResizableDesktopWindowOptions {
                min_size: egui::vec2(480.0, 360.0),
                default_size: Self::desktop_default_window_size(DesktopWindow::ZetaInvaders),
                default_pos: None,
                clamp_restore: true,
            },
        );
        let shown = window.show(ctx, |ui| {
            Self::apply_settings_control_style(ui);
            let title = self
                .desktop_zeta_invaders_wasm
                .as_ref()
                .map(|state| state.title().to_string())
                .unwrap_or_else(|| "Zeta Invaders".to_string());
            header_action = Self::draw_desktop_window_header(ui, &title, maximized);
            ui.add_space(6.0);
            ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
            if self.zeta_invaders_uses_wasm_addon() {
                let available = ui.available_size_before_wrap();
                let size = HostedAddonSize {
                    width: available.x.max(320.0),
                    height: available.y.max(240.0),
                };
                let input = collect_hosted_keyboard_input(ctx, input_enabled);
                let result = (|| -> Result<(), String> {
                    let module = super::super::installed_wasm_addon_module(
                        &crate::platform::AddonId::from("games.zeta-invaders"),
                    )
                    .ok_or_else(|| "Installed WASM module disappeared.".to_string())?;
                    if self.desktop_zeta_invaders_wasm.is_none() {
                        self.desktop_zeta_invaders_wasm = Some(WasmHostedAddonState::spawn(
                            &module,
                            HostedAddonSurface::Desktop,
                            size,
                        )?);
                    }
                    if let Some(state) = self.desktop_zeta_invaders_wasm.as_mut() {
                        state.update(size, dt, input)?;
                        draw_hosted_addon_frame(ui, state);
                        if let Some(status) = &state.frame().status_line {
                            ui.add_space(8.0);
                            ui.small(status);
                        }
                    }
                    Ok(())
                })();
                if let Err(err) = result {
                    self.desktop_zeta_invaders_wasm = None;
                    ui.monospace("WASM ADDON FAILED");
                    ui.small(err);
                }
            } else {
                let atlas = self
                    .zeta_invaders
                    .atlas
                    .as_ref()
                    .expect("zeta invaders atlas should be loaded");
                self.zeta_invaders.game.draw(ui, atlas);
            }
        });
        let shown_rect = shown.as_ref().map(|inner| inner.response.rect);
        let shown_contains_pointer = shown
            .as_ref()
            .is_some_and(|inner| inner.response.contains_pointer());
        self.finish_desktop_window_host(
            ctx,
            DesktopWindow::ZetaInvaders,
            &mut open,
            maximized,
            shown_rect,
            shown_contains_pointer,
            DesktopWindowRectTracking::FullRect,
            header_action,
        );
    }

    pub(super) fn draw_red_menace_window(&mut self, ctx: &Context) {
        if !self.red_menace.open || self.desktop_window_is_minimized(DesktopWindow::RedMenace) {
            return;
        }
        let input_enabled = self.active_window_kind() == Some(DesktopWindow::RedMenace)
            && !self.start_open
            && !self.spotlight_open
            && self.terminal_prompt.is_none();
        let dt = Self::next_embedded_game_dt(&mut self.red_menace.last_frame_at);
        let input = if input_enabled {
            red_menace_input_from_ctx(ctx)
        } else {
            Default::default()
        };
        self.red_menace.game.update(&input, dt);

        let mut open = self.red_menace.open;
        let mut header_action = DesktopHeaderAction::None;
        let (window, maximized) = self.build_resizable_desktop_window(
            ctx,
            DesktopWindow::RedMenace,
            "Red Menace",
            &mut open,
            ResizableDesktopWindowOptions {
                min_size: egui::vec2(620.0, 460.0),
                default_size: Self::desktop_default_window_size(DesktopWindow::RedMenace),
                default_pos: None,
                clamp_restore: true,
            },
        );
        let shown = window.show(ctx, |ui| {
            Self::apply_settings_control_style(ui);
            header_action = Self::draw_desktop_window_header(ui, "Red Menace", maximized);
            ui.add_space(6.0);
            ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
            self.red_menace.game.draw(ui);
        });
        let shown_rect = shown.as_ref().map(|inner| inner.response.rect);
        let shown_contains_pointer = shown
            .as_ref()
            .is_some_and(|inner| inner.response.contains_pointer());
        self.finish_desktop_window_host(
            ctx,
            DesktopWindow::RedMenace,
            &mut open,
            maximized,
            shown_rect,
            shown_contains_pointer,
            DesktopWindowRectTracking::FullRect,
            header_action,
        );
    }

    pub(super) fn draw_desktop_pty_window(&mut self, ctx: &Context) {
        if self.desktop_window_is_minimized(DesktopWindow::PtyApp) {
            return;
        }
        if !self.primary_desktop_pty_open() {
            self.update_desktop_window_state(DesktopWindow::PtyApp, false);
            return;
        }
        let wid = self.current_window_id(DesktopWindow::PtyApp);
        let default_size = Self::desktop_default_window_size(DesktopWindow::PtyApp);
        let default_pos = Self::desktop_default_window_pos(ctx, default_size);
        let pty_focused = self.desktop_active_window == Some(wid);
        let Some(pty_state) = self.terminal_pty.as_ref() else {
            self.update_desktop_window_state(DesktopWindow::PtyApp, false);
            return;
        };
        let title = pty_state.title.clone();
        let window_title = if wid.instance > 0 {
            format!("{} [{}]", title, wid.instance + 1)
        } else {
            title.clone()
        };
        let min_size = Self::native_pty_window_min_size(pty_state);
        let mut open = true;
        let mut header_action = DesktopHeaderAction::None;
        let mut event = PtyScreenEvent::None;
        let (window, maximized) = self.build_resizable_desktop_window(
            ctx,
            DesktopWindow::PtyApp,
            window_title.clone(),
            &mut open,
            ResizableDesktopWindowOptions {
                min_size,
                default_size,
                default_pos: Some(default_pos),
                clamp_restore: true,
            },
        );
        let Some(state) = self.terminal_pty.as_mut() else {
            self.update_desktop_window_state(DesktopWindow::PtyApp, false);
            return;
        };
        let shown = window.show(ctx, |ui| {
            // NOTE: do NOT call apply_settings_control_style here — it changes
            // extreme_bg_color and margins, which destabilizes available_size()
            // causing resize oscillation (constant SIGWINCH) for ncurses apps.
            header_action = Self::draw_desktop_window_header(ui, &window_title, maximized);
            let available = ui.available_size();
            let cols_floor = state.desktop_cols_floor.unwrap_or(40) as usize;
            let rows_floor = state.desktop_rows_floor.unwrap_or(20).saturating_add(1) as usize;
            let (cols, rows) = if state.desktop_live_resize {
                (
                    ((available.x / FIXED_PTY_CELL_W).floor() as usize)
                        .max(cols_floor)
                        .clamp(40, 220),
                    ((available.y / FIXED_PTY_CELL_H).floor() as usize)
                        .max(rows_floor)
                        .clamp(20, 60),
                )
            } else {
                (cols_floor, rows_floor)
            };
            ui.allocate_ui_with_layout(available, Layout::top_down(egui::Align::Min), |ui| {
                event = draw_embedded_pty_in_ui_focused(ui, ctx, state, cols, rows, pty_focused);
            });
        });
        let shown_rect = shown.as_ref().map(|inner| inner.response.rect);
        let shown_contains_pointer = shown
            .as_ref()
            .is_some_and(|inner| inner.response.contains_pointer());
        // Context menus are attached to specific content widgets inside the
        // window closure, not to the outer Area response (which causes
        // "double use of widget" ID collisions in egui 0.29).
        let completion_message = state.completion_message.clone();
        let title_for_exit = state.title.clone();
        let mut desktop_exit_plan: Option<TerminalDesktopPtyExitPlan> = None;

        match event {
            PtyScreenEvent::None => {}
            PtyScreenEvent::CloseRequested => open = false,
            PtyScreenEvent::ProcessExited => {
                let exit_status = state.session.exit_status();
                let success = exit_status
                    .as_ref()
                    .map(|status| status.success())
                    .unwrap_or(true);
                let exit_code = exit_status.as_ref().map(|status| status.exit_code());
                open = false;
                desktop_exit_plan = Some(resolve_desktop_pty_exit(
                    &title_for_exit,
                    completion_message.as_deref(),
                    success,
                    exit_code,
                ));
            }
        }

        self.maybe_activate_desktop_window_from_click(
            ctx,
            DesktopWindow::PtyApp,
            shown_contains_pointer,
        );
        if !maximized {
            if let Some(rect) = shown_rect {
                self.note_desktop_window_rect(DesktopWindow::PtyApp, rect);
            }
        }
        if let Some(plan) = desktop_exit_plan {
            self.apply_terminal_desktop_pty_exit_plan(plan);
        }
        self.finish_desktop_window_host(
            ctx,
            DesktopWindow::PtyApp,
            &mut open,
            maximized,
            shown_rect,
            shown_contains_pointer,
            DesktopWindowRectTracking::FullRect,
            header_action,
        );
    }

    pub(super) fn draw_terminal_mode(&mut self, ctx: &Context) {
        if !self.terminal_mode.open || self.desktop_window_is_minimized(DesktopWindow::TerminalMode)
        {
            return;
        }
        let _ = ctx;
        self.terminal_mode.open = false;
        self.desktop_window_states
            .remove(&WindowInstanceId::primary(DesktopWindow::TerminalMode));
        self.launch_desktop_terminal_shell_via_registry();
    }
}
