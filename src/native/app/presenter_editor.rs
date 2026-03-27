use super::super::command_layer::CommandLayerTarget;
use super::super::desktop_app::DesktopWindow;
use super::super::editor_app::{EditorCommand, EditorTextAlign, EDITOR_APP_TITLE};
use super::desktop_window_mgmt::{DesktopHeaderAction, DesktopWindowRectTracking, ResizableDesktopWindowOptions};
use super::RobcoNativeApp;
use eframe::egui::{self, Align2, Context, Id, Key, Layout, RichText, TextEdit};

impl RobcoNativeApp {
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
                if ctx.input(|i| i.key_pressed(Key::Tab)) {
                    self.update_desktop_window_state(DesktopWindow::Editor, false);
                    return;
                }
                if ctx.input(|i| i.key_pressed(Key::Escape)) {
                    if self.editor.ui.find_open {
                        self.run_editor_command(EditorCommand::CloseFind);
                    } else {
                        self.update_desktop_window_state(DesktopWindow::Editor, false);
                    }
                    return;
                }
                if ctx.input(|i| i.key_pressed(Key::F1)) {
                    self.open_command_layer(CommandLayerTarget::Editor);
                }
                if ctx.input(|i| i.key_pressed(Key::N) && i.modifiers.command) {
                    self.run_editor_command(EditorCommand::NewDocument);
                }
            }

            let palette = self.current_shell_palette();
            let text_edit_id = Id::new("terminal_editor_text_edit");
            egui::CentralPanel::default()
                .frame(
                    egui::Frame::none()
                        .fill(palette.bg)
                        .inner_margin(egui::Margin::same(4.0)),
                )
                .show(ctx, |ui| {
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
            header_action = Self::draw_desktop_window_header(ui, &title, maximized);
            if let Some(path) = &self.editor.path {
                ui.small(path.display().to_string());
            }
            if !self.editor.status.is_empty() {
                ui.small(self.editor.status.clone());
            }

            if self.editor.ui.find_open {
                let palette = self.current_shell_palette();
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

            let palette = self.current_shell_palette();
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
            let palette = self.current_shell_palette();
            let mut action: Option<&'static str> = None;
            egui::Window::new("editor_close_confirm")
                .id(Id::new(("editor_close_confirm", wid.instance, generation)))
                .title_bar(false)
                .collapsible(false)
                .resizable(false)
                .fixed_size(egui::vec2(360.0, 132.0))
                .anchor(Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
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
}
