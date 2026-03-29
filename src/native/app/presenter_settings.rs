use super::super::background::BackgroundResult;
use super::super::desktop_app::DesktopWindow;
use super::super::desktop_settings_service::persist_settings_draft;
use super::super::desktop_status_service::{clear_settings_status, saved_settings_status};
use super::desktop_window_mgmt::{DesktopHeaderAction, DesktopWindowRectTracking};
use super::NucleonNativeApp;
use crate::config::{ConnectionKind, OpenMode};
use eframe::egui::{self, Context, RichText};
use nucleon_native_settings_app::{
    desktop_settings_back_target, desktop_settings_connections_nav_items,
    desktop_settings_user_management_nav_items, settings_panel_title, NativeSettingsPanel,
    SettingsHomeTileAction,
};

impl NucleonNativeApp {
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
        let default_pos = self.active_desktop_default_window_pos(ctx, default_size);
        let mut window = egui::Window::new("Settings")
            .id(egui_id)
            .open(&mut open)
            .title_bar(false)
            .frame(self.desktop_window_frame())
            .resizable(false)
            .default_pos(default_pos)
            .fixed_size(default_size);
        if maximized {
            let rect = self.active_desktop_workspace_rect(ctx);
            window = window
                .movable(false)
                .fixed_pos(rect.min)
                .fixed_size(rect.size());
        } else if let Some((pos, _size)) = restore {
            let pos = self.active_desktop_clamp_window_pos(ctx, pos, default_size);
            window = window.current_pos(pos);
        }
        let mut close_requested = false;
        let shown = window.show(ctx, |ui| {
            Self::apply_settings_control_style(ui);
            header_action = Self::draw_desktop_window_header(
                ui,
                "Settings",
                maximized,
                self.desktop_active_window == Some(wid),
                &self.desktop_active_desktop_style,
            );
            let is_admin = self.session.as_ref().is_some_and(|s| s.is_admin);
            let panel = self.settings.panel;
            let mut changed = false;
            let window_mode_changed = false;
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
                                            if panel == NativeSettingsPanel::Appearance {
                                                self.open_tweaks_from_settings();
                                            } else if panel == NativeSettingsPanel::Addons {
                                                self.open_addons_from_settings();
                                            } else {
                                                next_panel = Some(panel);
                                            }
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
                                        let palette = self.current_shell_palette();
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
                            NativeSettingsPanel::Addons => {}
                            NativeSettingsPanel::Appearance => {}
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
}
