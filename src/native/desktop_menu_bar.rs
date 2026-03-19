// Top menu bar, action dispatch.

use super::app::RobcoNativeApp;
use super::desktop_app::{
    build_active_desktop_menu_section, build_app_control_menu, build_shared_desktop_menu_section,
    build_window_menu_section, desktop_app_menu_name, desktop_components, DesktopMenuAction,
    DesktopMenuBuildContext, DesktopMenuItem, DesktopMenuSection, DesktopWindow,
    DesktopWindowMenuEntry,
};
use super::file_manager_app::FileManagerSettingsUpdate;
use super::retro_ui::current_palette;
use chrono::Local;
use eframe::egui::{self, Color32, Context, Key, Layout, RichText, TopBottomPanel};

impl RobcoNativeApp {
    pub(super) fn apply_global_retro_menu_chrome(ctx: &Context, palette: &super::retro_ui::RetroPalette) {
        let mut global = ctx.style().as_ref().clone();
        global.visuals.panel_fill = palette.bg;
        global.visuals.extreme_bg_color = palette.bg;
        global.visuals.window_fill = palette.bg;
        global.visuals.window_stroke = egui::Stroke::new(2.0, palette.fg);
        global.visuals.window_rounding = egui::Rounding::ZERO;
        global.visuals.menu_rounding = egui::Rounding::ZERO;
        global.visuals.window_shadow = egui::epaint::Shadow::NONE;
        global.visuals.popup_shadow = egui::epaint::Shadow::NONE;
        ctx.set_style(global);
    }

    pub(super) fn apply_top_bar_menu_button_style(ui: &mut egui::Ui) {
        let palette = current_palette();
        let mut style = ui.style().as_ref().clone();
        style.visuals.panel_fill = palette.bg;
        style.visuals.extreme_bg_color = palette.bg;
        style.visuals.window_fill = palette.bg;
        style.visuals.window_stroke = egui::Stroke::new(2.0, palette.fg);
        style.visuals.window_rounding = egui::Rounding::ZERO;
        style.visuals.menu_rounding = egui::Rounding::ZERO;
        style.visuals.window_shadow = egui::epaint::Shadow::NONE;
        style.visuals.popup_shadow = egui::epaint::Shadow::NONE;
        style.visuals.button_frame = false;
        style.visuals.override_text_color = Some(Color32::BLACK);
        style.visuals.widgets.noninteractive.bg_fill = Color32::TRANSPARENT;
        style.visuals.widgets.noninteractive.weak_bg_fill = Color32::TRANSPARENT;
        style.visuals.widgets.noninteractive.bg_stroke = egui::Stroke::NONE;
        style.visuals.widgets.noninteractive.fg_stroke.color = Color32::BLACK;
        style.visuals.widgets.noninteractive.rounding = egui::Rounding::ZERO;
        style.visuals.widgets.noninteractive.expansion = 0.0;
        style.visuals.widgets.inactive.bg_fill = Color32::TRANSPARENT;
        style.visuals.widgets.inactive.weak_bg_fill = Color32::TRANSPARENT;
        style.visuals.widgets.inactive.bg_stroke = egui::Stroke::NONE;
        style.visuals.widgets.inactive.fg_stroke.color = Color32::BLACK;
        style.visuals.widgets.inactive.rounding = egui::Rounding::ZERO;
        style.visuals.widgets.inactive.expansion = 0.0;
        for visuals in [
            &mut style.visuals.widgets.hovered,
            &mut style.visuals.widgets.active,
            &mut style.visuals.widgets.open,
        ] {
            visuals.bg_fill = palette.selected_bg;
            visuals.weak_bg_fill = palette.selected_bg;
            visuals.bg_stroke = egui::Stroke::NONE;
            visuals.fg_stroke.color = Color32::BLACK;
            visuals.rounding = egui::Rounding::ZERO;
            visuals.expansion = 0.0;
        }
        ui.set_style(style);
    }

    pub(super) fn apply_top_dropdown_menu_style(ui: &mut egui::Ui) {
        let palette = current_palette();
        let mut style = ui.style().as_ref().clone();
        style.visuals.override_text_color = None;
        style.visuals.window_fill = palette.bg;
        style.visuals.panel_fill = palette.bg;
        style.visuals.extreme_bg_color = palette.bg;
        style.visuals.window_stroke = egui::Stroke::new(2.0, palette.fg);
        style.visuals.window_rounding = egui::Rounding::ZERO;
        style.visuals.menu_rounding = egui::Rounding::ZERO;
        style.visuals.window_shadow = egui::epaint::Shadow::NONE;
        style.visuals.popup_shadow = egui::epaint::Shadow::NONE;
        style.visuals.button_frame = false;
        style.visuals.widgets.noninteractive.bg_fill = Color32::TRANSPARENT;
        style.visuals.widgets.noninteractive.weak_bg_fill = Color32::TRANSPARENT;
        style.visuals.widgets.noninteractive.bg_stroke = egui::Stroke::NONE;
        style.visuals.widgets.noninteractive.fg_stroke.color = palette.fg;
        style.visuals.widgets.noninteractive.rounding = egui::Rounding::ZERO;
        style.visuals.widgets.noninteractive.expansion = 0.0;
        style.visuals.widgets.inactive.bg_fill = Color32::TRANSPARENT;
        style.visuals.widgets.inactive.weak_bg_fill = Color32::TRANSPARENT;
        style.visuals.widgets.inactive.bg_stroke = egui::Stroke::NONE;
        style.visuals.widgets.inactive.fg_stroke.color = palette.fg;
        style.visuals.widgets.inactive.rounding = egui::Rounding::ZERO;
        style.visuals.widgets.inactive.expansion = 0.0;
        for visuals in [
            &mut style.visuals.widgets.hovered,
            &mut style.visuals.widgets.active,
            &mut style.visuals.widgets.open,
        ] {
            visuals.bg_fill = palette.fg;
            visuals.weak_bg_fill = palette.fg;
            visuals.bg_stroke = egui::Stroke::NONE;
            visuals.fg_stroke.color = Color32::BLACK;
            visuals.rounding = egui::Rounding::ZERO;
            visuals.expansion = 0.0;
        }
        ui.set_style(style);
    }

    pub(super) fn active_editor_text_edit_id(&self) -> egui::Id {
        let generation = self.desktop_window_generation(DesktopWindow::Editor);
        egui::Id::new(("editor_text_edit", generation))
    }

    pub(super) fn apply_desktop_menu_action(&mut self, ctx: &Context, action: &DesktopMenuAction) {
        match action {
            DesktopMenuAction::EditorCommand(command) => self.run_editor_command(*command),
            DesktopMenuAction::EditorTextCommand(command) => {
                self.run_editor_text_command(ctx, self.active_editor_text_edit_id(), *command);
            }
            DesktopMenuAction::OpenRecentEditorFile(path) => {
                self.open_path_in_editor(path.clone());
            }
            DesktopMenuAction::FileManagerCommand(command) => {
                self.run_file_manager_command(*command);
            }
            DesktopMenuAction::OpenFileManagerPrompt(request) => {
                self.open_file_manager_prompt(request.clone());
            }
            DesktopMenuAction::FileManagerLaunchOpenWithCommand {
                path,
                ext_key,
                command,
            } => match self.launch_open_with_command(path, command) {
                Ok(message) => {
                    self.apply_file_manager_settings_update(
                        FileManagerSettingsUpdate::RecordOpenWithCommand {
                            ext_key: ext_key.clone(),
                            command: command.clone(),
                        },
                    );
                    self.shell_status = message;
                }
                Err(err) => {
                    self.shell_status = format!("Open failed: {err}");
                }
            },
            DesktopMenuAction::FileManagerSetOpenWithDefault { ext_key, command } => {
                self.apply_file_manager_settings_update(
                    FileManagerSettingsUpdate::SetOpenWithDefaultCommand {
                        ext_key: ext_key.clone(),
                        command: command.clone(),
                    },
                );
                self.shell_status = if let Some(command) = command {
                    super::file_manager_app::open_with_set_default_status(command, ext_key)
                } else {
                    super::file_manager_app::open_with_cleared_default_status(ext_key)
                };
            }
            DesktopMenuAction::FileManagerRemoveOpenWithCommand { ext_key, command } => {
                self.apply_file_manager_settings_update(
                    FileManagerSettingsUpdate::RemoveOpenWithCommand {
                        ext_key: ext_key.clone(),
                        command: command.clone(),
                    },
                );
                self.shell_status = super::file_manager_app::open_with_removed_saved_status(ext_key);
            }
            DesktopMenuAction::OpenFileManager => {
                self.launch_standalone_file_manager(None);
            }
            DesktopMenuAction::OpenApplications => {
                self.launch_standalone_applications();
            }
            DesktopMenuAction::OpenSettings => {
                self.open_standalone_settings(None);
            }
            DesktopMenuAction::ToggleStartMenu => {
                if self.start_open {
                    self.close_start_menu();
                } else {
                    self.open_start_menu();
                }
            }
            DesktopMenuAction::CloseActiveDesktopWindow => {
                if let Some(window) = self.desktop_active_window {
                    self.close_desktop_window(window);
                }
            }
            DesktopMenuAction::MinimizeActiveDesktopWindow => {
                if let Some(window) = self.desktop_active_window {
                    self.set_desktop_window_minimized(window, true);
                }
            }
            DesktopMenuAction::ActivateDesktopWindow(window) => {
                if *window == DesktopWindow::Editor
                    && !self.desktop_window_is_open(DesktopWindow::Editor)
                    && self.editor.path.is_none()
                {
                    self.new_document();
                } else if !self.desktop_window_is_open(*window) {
                    self.open_desktop_window(*window);
                } else {
                    self.focus_desktop_window(Some(ctx), *window);
                    self.close_desktop_overlays();
                }
            }
            DesktopMenuAction::ActivateTaskbarWindow(window) => {
                if !self.desktop_window_is_open(*window) {
                    self.open_desktop_window(*window);
                } else if self.desktop_window_is_minimized(*window) {
                    self.set_desktop_window_minimized(*window, false);
                    self.close_desktop_overlays();
                } else if self.desktop_active_window == Some(*window) {
                    self.set_desktop_window_minimized(*window, true);
                    self.close_desktop_overlays();
                } else {
                    self.focus_desktop_window(Some(ctx), *window);
                    self.close_desktop_overlays();
                }
            }
            DesktopMenuAction::OpenManual { path, status_label } => {
                self.open_manual_file(path, status_label);
            }
        }
    }

    pub(super) fn draw_desktop_menu_items(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &Context,
        items: &[DesktopMenuItem],
    ) {
        for item in items {
            match item {
                DesktopMenuItem::Action { label, action } => {
                    if ui.button(label).clicked() {
                        self.apply_desktop_menu_action(ctx, action);
                        ui.close_menu();
                    }
                }
                DesktopMenuItem::Disabled { label } => {
                    let _ = Self::retro_disabled_button(ui, label);
                }
                DesktopMenuItem::Label { label } => {
                    ui.label(RichText::new(label).small());
                }
                DesktopMenuItem::Separator => Self::retro_separator(ui),
                DesktopMenuItem::Submenu { label, items } => {
                    ui.menu_button(label, |ui| {
                        Self::apply_top_dropdown_menu_style(ui);
                        self.draw_desktop_menu_items(ui, ctx, items);
                    });
                }
            }
        }
    }

    pub(super) fn draw_top_bar_standard_menu(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &Context,
        section: DesktopMenuSection,
    ) {
        let menu = ui.menu_button(section.label(), |ui| {
            Self::apply_top_dropdown_menu_style(ui);
            if section == DesktopMenuSection::Format {
                ui.set_min_width(160.0);
                ui.set_max_width(220.0);
            }
            let active_app = self.active_desktop_app();
            let menu_context = DesktopMenuBuildContext {
                editor: &self.editor,
                editor_recent_files: &self.settings.draft.editor_recent_files,
                file_manager: &self.file_manager,
                file_manager_runtime: &self.file_manager_runtime,
                file_manager_settings: &self.live_desktop_file_manager_settings,
            };
            let items = build_active_desktop_menu_section(active_app, section, &menu_context);
            if !items.is_empty() {
                self.draw_desktop_menu_items(ui, ctx, &items);
            }
            let shared_items = build_shared_desktop_menu_section(section);
            if !shared_items.is_empty() {
                self.draw_desktop_menu_items(ui, ctx, &shared_items);
            }
        });
        if menu.response.clicked() {
            self.close_desktop_overlays();
        }
    }

    pub(super) fn draw_top_bar_window_menu(&mut self, ui: &mut egui::Ui, ctx: &Context) {
        let menu = ui.menu_button("Window", |ui| {
            Self::apply_top_dropdown_menu_style(ui);
            let entries: Vec<DesktopWindowMenuEntry> = desktop_components()
                .iter()
                .filter(|component| component.spec.show_in_window_menu)
                .map(|component| DesktopWindowMenuEntry {
                    window: component.spec.window,
                    open: self.desktop_window_is_open(component.spec.window),
                    active: self.desktop_active_window == Some(component.spec.window),
                })
                .collect();
            let items = build_window_menu_section(
                &entries,
                self.terminal_pty.as_ref().map(|pty| pty.title.as_str()),
            );
            self.draw_desktop_menu_items(ui, ctx, &items);
        });
        if menu.response.clicked() {
            self.close_desktop_overlays();
        }
    }

    pub(super) fn draw_top_bar_help_menu(&mut self, ui: &mut egui::Ui, ctx: &Context) {
        let menu = ui.menu_button("Help", |ui| {
            Self::apply_top_dropdown_menu_style(ui);
            let items = build_shared_desktop_menu_section(DesktopMenuSection::Help);
            self.draw_desktop_menu_items(ui, ctx, &items);
        });
        if menu.response.clicked() {
            self.close_desktop_overlays();
        }
    }

    pub(super) fn draw_top_bar_menu_section(
        &mut self,
        ctx: &Context,
        ui: &mut egui::Ui,
        section: DesktopMenuSection,
    ) {
        match section {
            DesktopMenuSection::File
            | DesktopMenuSection::Edit
            | DesktopMenuSection::Format
            | DesktopMenuSection::View => self.draw_top_bar_standard_menu(ui, ctx, section),
            DesktopMenuSection::Window => self.draw_top_bar_window_menu(ui, ctx),
            DesktopMenuSection::Help => self.draw_top_bar_help_menu(ui, ctx),
        }
    }

    pub(super) fn draw_top_bar_app_menu(&mut self, ui: &mut egui::Ui, ctx: &Context, app_menu_name: &str) {
        let menu = ui.menu_button(
            RichText::new(app_menu_name).strong().color(Color32::BLACK),
            |ui| {
                Self::apply_top_dropdown_menu_style(ui);
                let items = build_app_control_menu(self.desktop_active_window.is_some());
                self.draw_desktop_menu_items(ui, ctx, &items);
            },
        );
        if menu.response.clicked() {
            self.close_desktop_overlays();
        }
    }

    pub(super) fn draw_top_bar(&mut self, ctx: &Context) {
        let palette = current_palette();
        Self::apply_global_retro_menu_chrome(ctx, &palette);
        let app_menu_name = desktop_app_menu_name(
            self.desktop_active_window,
            self.terminal_pty.as_ref().map(|pty| pty.title.as_str()),
        );
        let active_app = self.active_desktop_app();
        TopBottomPanel::top("native_top_bar")
            .exact_height(30.0)
            .show_separator_line(false)
            .show(ctx, |ui| {
                ui.painter()
                    .rect_filled(ui.max_rect(), 0.0, palette.selected_bg);
                ui.horizontal(|ui| {
                    Self::apply_top_bar_menu_button_style(ui);
                    ui.spacing_mut().item_spacing.x = 14.0;
                    self.draw_top_bar_app_menu(ui, ctx, &app_menu_name);
                    ui.add_space(10.0);
                    for section in active_app.menu_sections() {
                        self.draw_top_bar_menu_section(ctx, ui, *section);
                    }
                    ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                        let batt = crate::status::battery_status_string();
                        if !batt.is_empty() {
                            ui.label(RichText::new(batt).color(Color32::BLACK));
                            ui.add_space(10.0);
                        }
                        let now = Local::now().format("%a %d %b %H:%M").to_string();
                        ui.label(RichText::new(now).color(Color32::BLACK));
                        ui.add_space(10.0);
                        if ui
                            .button(RichText::new("Search").color(Color32::BLACK))
                            .clicked()
                            || ctx.input(|i| i.key_pressed(Key::Space) && i.modifiers.command)
                        {
                            if self.spotlight_open {
                                self.close_spotlight();
                            } else {
                                self.open_spotlight();
                            }
                        }
                    });
                });
            });
    }
}
