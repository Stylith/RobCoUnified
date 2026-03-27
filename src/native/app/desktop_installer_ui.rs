use super::super::desktop_app::DesktopWindow;
use super::super::installer_screen::{
    available_runtime_tools, cached_package_description as installer_cached_package_description,
    runtime_tool_action_for_selection, runtime_tool_actions, runtime_tool_description,
    runtime_tool_installed_cached as installer_runtime_tool_installed_cached, runtime_tool_pkg,
    runtime_tool_title, DesktopInstallerConfirm, DesktopInstallerEvent, DesktopInstallerState,
    DesktopInstallerView, InstallerCategory, InstallerMenuTarget, InstallerPackageAction,
};
use super::super::retro_ui::{current_palette, RetroPalette};
use super::super::{
    install_user_addon, installed_addon_inventory_sections, remove_installed_addon,
    repository_sync_action_for_manifest, set_addon_enabled_override, InstalledAddonRecord,
    RepositoryAddonAction, RepositoryAddonRecord,
};
use super::DesktopHeaderAction;
use eframe::egui::{self, Color32, Context, Id, Key, RichText, TextureHandle};

use super::RobcoNativeApp;

enum AddonRowAction {
    None,
    Toggle,
    Remove,
    RepositorySync(RepositoryAddonAction),
}

impl RobcoNativeApp {
    // ─── Desktop Program Installer ─────────────────────────────────────────────

    pub(super) fn draw_installer(&mut self, ctx: &Context) {
        if !self.desktop_installer.open
            || self.desktop_window_is_minimized(DesktopWindow::Installer)
        {
            return;
        }
        if self.desktop_installer.search_in_flight() || self.desktop_installer.addon_install_in_flight() {
            ctx.request_repaint_after(std::time::Duration::from_millis(50));
        }
        let _ = self.desktop_installer.poll_search();
        let wid = self.current_window_id(DesktopWindow::Installer);
        {
            let state = self.desktop_window_state(wid);
            if state.maximized && (state.restore_pos.is_none() || state.restore_size.is_none()) {
                let state = self.desktop_window_state_mut(wid);
                state.maximized = false;
            }
        }
        let mut open = self.desktop_installer.open;
        let maximized = self.desktop_window_is_maximized(DesktopWindow::Installer);
        let mut header_action = DesktopHeaderAction::None;
        let generation = self.desktop_window_generation(wid);
        let egui_id = self.desktop_window_egui_id(wid);
        let default_size = Self::desktop_default_window_size(DesktopWindow::Installer);
        let min_size = Self::desktop_installer_window_min_size();
        let default_pos = self.active_desktop_default_window_pos(ctx, default_size);
        let workspace_rect = self.active_desktop_workspace_rect(ctx);
        let restore = self.take_desktop_window_restore_dims(DesktopWindow::Installer);
        let mut window = egui::Window::new("Program Installer")
            .id(egui_id)
            .open(&mut open)
            .title_bar(false)
            .frame(Self::desktop_window_frame())
            .resizable(false)
            .min_size([min_size.x, min_size.y])
            .max_size(workspace_rect.size())
            .constrain_to(workspace_rect)
            .default_pos(default_pos)
            .default_size([default_size.x, default_size.y]);
        if maximized {
            window = window
                .movable(false)
                .resizable(false)
                .fixed_pos(workspace_rect.min)
                .fixed_size(workspace_rect.size());
        } else if let Some((pos, _size)) = restore {
            let pos = self.active_desktop_clamp_window_pos(ctx, pos, default_size);
            window = window.current_pos(pos).fixed_size(default_size);
        } else {
            window = window.fixed_size(default_size);
        }

        let palette = current_palette();
        let mut deferred_back = false;
        let mut deferred_search = false;
        let mut deferred_load_installed = false;
        let mut deferred_open_installed_actions: Option<String> = None;
        let mut deferred_open_search_actions: Option<(String, bool)> = None;
        let mut deferred_confirm_setup: Option<(String, InstallerPackageAction)> = None;
        let mut deferred_confirm_yes = false;
        let mut deferred_confirm_no = false;
        let mut deferred_notice_close = false;
        let mut deferred_add_to_menu: Option<(String, InstallerMenuTarget)> = None;
        let mut deferred_open_add_to_menu: Option<String> = None;
        let mut deferred_open_runtime_tools = false;
        let mut deferred_open_addons = false;
        let mut deferred_repository_action: Option<(crate::platform::AddonId, RepositoryAddonAction)> = None;

        let view = self.desktop_installer.view.clone();
        let status = self.desktop_installer.status.clone();
        let has_confirm = self.desktop_installer.confirm_dialog.is_some();
        let notice = self.desktop_installer.notice.clone();
        let tex_apps = self
            .asset_cache
            .as_ref()
            .map(|c| c.icon_applications.clone());
        let tex_tools = self.asset_cache.as_ref().map(|c| c.icon_terminal.clone());
        let tex_network = self
            .asset_cache
            .as_ref()
            .map(|c| c.icon_connections.clone());
        let tex_games = self.installer_games_texture(ctx);

        let shown = window.show(ctx, |ui| {
            Self::apply_installer_widget_style(ui, palette);

            egui::TopBottomPanel::top(Id::new(("inst_top", generation)))
                .frame(egui::Frame::none())
                .show_inside(ui, |ui| {
                    header_action =
                        Self::draw_desktop_window_header(ui, "RobCo Program Installer", maximized);
                });

            egui::TopBottomPanel::bottom(Id::new(("inst_bottom", generation)))
                .frame(egui::Frame::none().inner_margin(egui::Margin::symmetric(8.0, 4.0)))
                .exact_height(28.0)
                .show_inside(ui, |ui| {
                    if !status.is_empty() {
                        ui.label(RichText::new(&status).color(palette.dim));
                    } else {
                        ui.allocate_space(egui::vec2(ui.available_width(), 0.0));
                    }
                });

            if has_confirm {
                egui::TopBottomPanel::bottom(Id::new(("inst_confirm", generation)))
                    .frame(
                        egui::Frame::none()
                            .fill(palette.panel)
                            .stroke(egui::Stroke::new(1.0, palette.fg))
                            .inner_margin(egui::Margin::same(12.0)),
                    )
                    .show_inside(ui, |ui| {
                        if let Some(ref confirm) = self.desktop_installer.confirm_dialog {
                            let action_label = match confirm.action {
                                InstallerPackageAction::Install => "Install",
                                InstallerPackageAction::Update => "Update",
                                InstallerPackageAction::Reinstall => "Reinstall",
                                InstallerPackageAction::Uninstall => "Uninstall",
                            };
                            ui.label(
                                RichText::new(format!("{} {}?", action_label, confirm.pkg))
                                    .color(palette.fg)
                                    .strong(),
                            );
                            ui.add_space(8.0);
                            ui.horizontal(|ui| {
                                if ui
                                    .button(RichText::new("[ Yes ]").color(palette.fg))
                                    .clicked()
                                {
                                    deferred_confirm_yes = true;
                                }
                                ui.add_space(12.0);
                                if ui
                                    .button(RichText::new("[ No ]").color(palette.fg))
                                    .clicked()
                                {
                                    deferred_confirm_no = true;
                                }
                            });
                        }
                    });
            }

            if let Some(notice) = notice.as_ref() {
                egui::Area::new(Id::new(("inst_notice", generation)))
                    .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
                    .order(egui::Order::Foreground)
                    .show(ctx, |ui| {
                        Self::apply_installer_widget_style(ui, palette);
                        egui::Frame::none()
                            .fill(palette.bg)
                            .stroke(egui::Stroke::new(2.0, palette.fg))
                            .inner_margin(egui::Margin::same(14.0))
                            .show(ui, |ui| {
                                ui.set_min_width(360.0);
                                ui.label(
                                    RichText::new(if notice.success {
                                        "Operation Complete"
                                    } else {
                                        "Operation Failed"
                                    })
                                    .color(palette.fg)
                                    .strong()
                                    .heading(),
                                );
                                ui.add_space(8.0);
                                ui.label(RichText::new(&notice.message).color(palette.fg));
                                ui.add_space(12.0);
                                if ui
                                    .button(RichText::new("[ OK ]").color(palette.fg))
                                    .clicked()
                                {
                                    deferred_notice_close = true;
                                }
                            });
                    });
            }

            egui::CentralPanel::default()
                .frame(egui::Frame::none().inner_margin(egui::Margin::same(16.0)))
                .show_inside(ui, |ui| match view {
                    DesktopInstallerView::Home => {
                        Self::draw_installer_home(
                            ui,
                            &mut self.desktop_installer,
                            palette,
                            &mut deferred_search,
                            &mut deferred_load_installed,
                            &mut deferred_open_runtime_tools,
                            &mut deferred_open_addons,
                            [&tex_apps, &tex_tools, &tex_network, &tex_games],
                        );
                    }
                    DesktopInstallerView::SearchResults => {
                        Self::draw_installer_search_results(
                            ui,
                            &mut self.desktop_installer,
                            palette,
                            &mut deferred_back,
                            &mut deferred_open_search_actions,
                        );
                    }
                    DesktopInstallerView::Installed => {
                        Self::draw_installer_installed(
                            ui,
                            &mut self.desktop_installer,
                            palette,
                            &mut deferred_back,
                            &mut deferred_open_installed_actions,
                        );
                    }
                    DesktopInstallerView::PackageActions { ref pkg, installed } => {
                        let pkg = pkg.clone();
                        Self::draw_installer_package_actions(
                            ui,
                            &mut self.desktop_installer,
                            palette,
                            &pkg,
                            installed,
                            &mut deferred_back,
                            &mut deferred_confirm_setup,
                            &mut deferred_open_add_to_menu,
                        );
                    }
                    DesktopInstallerView::AddToMenu { ref pkg } => {
                        let pkg = pkg.clone();
                        Self::draw_installer_add_to_menu(
                            ui,
                            &mut self.desktop_installer,
                            palette,
                            &pkg,
                            &mut deferred_back,
                            &mut deferred_add_to_menu,
                        );
                    }
                    DesktopInstallerView::RuntimeTools => {
                        Self::draw_installer_runtime_tools(
                            ui,
                            &mut self.desktop_installer,
                            palette,
                            &mut deferred_back,
                            &mut deferred_confirm_setup,
                        );
                    }
                    DesktopInstallerView::Addons => {
                        Self::draw_installer_addons(
                            ui,
                            &mut self.desktop_installer,
                            palette,
                            &mut deferred_back,
                            &mut deferred_repository_action,
                        );
                    }
                });
        });

        let shown_rect = shown.as_ref().map(|inner| inner.response.rect);
        let shown_contains_pointer = shown
            .as_ref()
            .is_some_and(|inner| inner.response.contains_pointer());
        if let Some(rect) = shown_rect {
            if !maximized {
                let state = self.desktop_window_state_mut(wid);
                state.restore_pos = Some([rect.min.x, rect.min.y]);
            }
            self.maybe_activate_desktop_window_from_click(
                ctx,
                DesktopWindow::Installer,
                shown_contains_pointer,
            );
        }

        // Sync open state
        if !open {
            self.desktop_installer.open = false;
        }
        self.update_desktop_window_state(DesktopWindow::Installer, self.desktop_installer.open);

        // Handle header buttons
        match header_action {
            DesktopHeaderAction::Close => self.close_desktop_window(DesktopWindow::Installer),
            DesktopHeaderAction::Minimize => {
                self.set_desktop_window_minimized(DesktopWindow::Installer, true)
            }
            DesktopHeaderAction::ToggleMaximize => {
                self.toggle_desktop_window_maximized(DesktopWindow::Installer, shown_rect)
            }
            DesktopHeaderAction::None => {}
        }

        // Process deferred actions
        if deferred_back {
            self.desktop_installer.go_back();
        }
        if deferred_search {
            self.desktop_installer.do_search();
        }
        if deferred_load_installed {
            self.desktop_installer.load_installed();
        }
        if deferred_open_runtime_tools {
            self.desktop_installer.view = DesktopInstallerView::RuntimeTools;
        }
        if deferred_open_addons {
            self.desktop_installer.view = DesktopInstallerView::Addons;
        }
        if let Some(pkg) = deferred_open_installed_actions {
            self.desktop_installer.view = DesktopInstallerView::PackageActions {
                pkg,
                installed: true,
            };
        }
        if let Some((pkg, installed)) = deferred_open_search_actions {
            self.desktop_installer.view = DesktopInstallerView::PackageActions { pkg, installed };
        }
        if let Some((pkg, action)) = deferred_confirm_setup {
            self.desktop_installer.confirm_dialog = Some(DesktopInstallerConfirm { pkg, action });
        }
        if deferred_confirm_yes {
            let event = self.desktop_installer.confirm_action();
            if let DesktopInstallerEvent::LaunchCommand {
                argv,
                status,
                completion_message,
            } = event
            {
                self.desktop_installer.status = status.clone();
                self.launch_shell_command_in_desktop_surface("Program Installer", &argv);
                if let Some(pty) = self.active_desktop_pty_state_mut() {
                    pty.completion_message = completion_message;
                }
            }
        }
        if deferred_confirm_no {
            self.desktop_installer.confirm_dialog = None;
        }
        if deferred_notice_close {
            self.desktop_installer.notice = None;
        }
        if let Some(pkg) = deferred_open_add_to_menu {
            self.desktop_installer.display_name_input = pkg.clone();
            self.desktop_installer.view = DesktopInstallerView::AddToMenu { pkg };
        }
        if let Some((pkg, target)) = deferred_add_to_menu {
            self.desktop_installer.add_to_menu(&pkg, target);
            self.invalidate_program_catalog_cache();
        }
        if let Some((addon_id, action)) = deferred_repository_action {
            self.start_repository_addon_install(addon_id, action, true);
        }
    }

    // ── Installer sub-views ─────────────────────────────────────────────────

    pub(super) fn apply_installer_widget_style(ui: &mut egui::Ui, palette: RetroPalette) {
        ui.visuals_mut().window_fill = palette.bg;
        ui.visuals_mut().panel_fill = palette.bg;
        ui.visuals_mut().faint_bg_color = palette.bg;
        let widgets = &mut ui.visuals_mut().widgets;
        widgets.inactive.bg_fill = palette.bg;
        widgets.inactive.weak_bg_fill = palette.bg;
        widgets.inactive.bg_stroke = egui::Stroke::new(1.0, palette.fg);
        widgets.inactive.fg_stroke = egui::Stroke::new(1.0, palette.fg);
        widgets.hovered.bg_fill = palette.hovered_bg;
        widgets.hovered.weak_bg_fill = palette.hovered_bg;
        widgets.hovered.bg_stroke = egui::Stroke::new(1.0, palette.fg);
        widgets.hovered.fg_stroke = egui::Stroke::new(1.0, palette.fg);
        widgets.active.bg_fill = palette.active_bg;
        widgets.active.weak_bg_fill = palette.active_bg;
        widgets.active.bg_stroke = egui::Stroke::new(1.0, palette.fg);
        widgets.active.fg_stroke = egui::Stroke::new(1.0, palette.fg);
        widgets.open.bg_fill = palette.hovered_bg;
        widgets.open.weak_bg_fill = palette.hovered_bg;
        widgets.open.bg_stroke = egui::Stroke::new(1.0, palette.fg);
        widgets.open.fg_stroke = egui::Stroke::new(1.0, palette.fg);
        widgets.noninteractive.bg_fill = palette.bg;
        widgets.noninteractive.weak_bg_fill = palette.bg;
        widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0, palette.fg);
        widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0, palette.fg);
        ui.visuals_mut().extreme_bg_color = palette.panel;
        ui.visuals_mut().code_bg_color = palette.bg;
        ui.visuals_mut().window_shadow = egui::epaint::Shadow::NONE;
        ui.visuals_mut().popup_shadow = egui::epaint::Shadow::NONE;
        ui.visuals_mut().window_rounding = egui::Rounding::ZERO;
        ui.visuals_mut().menu_rounding = egui::Rounding::ZERO;
        ui.visuals_mut().selection.bg_fill = palette.selection_bg;
        ui.visuals_mut().selection.stroke = egui::Stroke::new(1.0, palette.fg);
        ui.visuals_mut().text_cursor.stroke = egui::Stroke::new(1.5, palette.fg);
        ui.visuals_mut().widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0, palette.dim);
        ui.visuals_mut().widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0, palette.fg);
    }

    pub(super) fn apply_installer_dropdown_style(ui: &mut egui::Ui, palette: RetroPalette) {
        let mut style = ui.style().as_ref().clone();
        style.visuals.window_fill = palette.bg;
        style.visuals.panel_fill = palette.bg;
        style.visuals.window_stroke = egui::Stroke::new(1.0, palette.fg);
        style.visuals.window_rounding = egui::Rounding::ZERO;
        style.visuals.menu_rounding = egui::Rounding::ZERO;
        style.visuals.window_shadow = egui::epaint::Shadow::NONE;
        style.visuals.popup_shadow = egui::epaint::Shadow::NONE;
        style.visuals.widgets.noninteractive.bg_fill = palette.bg;
        style.visuals.widgets.noninteractive.weak_bg_fill = palette.bg;
        style.visuals.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0, palette.fg);
        style.visuals.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0, palette.fg);
        style.visuals.widgets.inactive.bg_fill = palette.bg;
        style.visuals.widgets.inactive.weak_bg_fill = palette.bg;
        style.visuals.widgets.inactive.bg_stroke = egui::Stroke::new(1.0, palette.fg);
        style.visuals.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, palette.fg);
        style.visuals.widgets.hovered.bg_fill = palette.hovered_bg;
        style.visuals.widgets.hovered.weak_bg_fill = palette.hovered_bg;
        style.visuals.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, palette.fg);
        style.visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, palette.fg);
        style.visuals.widgets.active.bg_fill = palette.active_bg;
        style.visuals.widgets.active.weak_bg_fill = palette.active_bg;
        style.visuals.widgets.active.bg_stroke = egui::Stroke::new(1.0, palette.fg);
        style.visuals.widgets.active.fg_stroke = egui::Stroke::new(1.0, palette.fg);
        style.visuals.widgets.open = style.visuals.widgets.hovered;
        ui.set_style(style);
    }

    fn installer_link_button(
        ui: &mut egui::Ui,
        text: RichText,
        palette: RetroPalette,
    ) -> egui::Response {
        ui.scope(|ui| {
            let mut style = ui.style().as_ref().clone();
            style.visuals.widgets.inactive.bg_fill = Color32::TRANSPARENT;
            style.visuals.widgets.inactive.weak_bg_fill = Color32::TRANSPARENT;
            style.visuals.widgets.inactive.bg_stroke = egui::Stroke::NONE;
            style.visuals.widgets.hovered.bg_fill = palette.hovered_bg;
            style.visuals.widgets.hovered.weak_bg_fill = palette.hovered_bg;
            style.visuals.widgets.hovered.bg_stroke = egui::Stroke::NONE;
            style.visuals.widgets.active.bg_fill = palette.active_bg;
            style.visuals.widgets.active.weak_bg_fill = palette.active_bg;
            style.visuals.widgets.active.bg_stroke = egui::Stroke::NONE;
            style.visuals.widgets.open = style.visuals.widgets.hovered;
            style.spacing.button_padding = egui::vec2(8.0, 4.0);
            ui.set_style(style);
            ui.add(egui::Button::new(text))
        })
        .inner
    }

    fn installer_description_preview(desc: &str, limit: usize) -> String {
        let trimmed = desc.trim();
        let count = trimmed.chars().count();
        if count <= limit {
            trimmed.to_string()
        } else {
            format!(
                "{}...",
                trimmed
                    .chars()
                    .take(limit.saturating_sub(3).max(1))
                    .collect::<String>()
                    .trim_end()
            )
        }
    }

    fn draw_installer_home(
        ui: &mut egui::Ui,
        state: &mut DesktopInstallerState,
        palette: RetroPalette,
        deferred_search: &mut bool,
        deferred_load_installed: &mut bool,
        deferred_open_runtime_tools: &mut bool,
        deferred_open_addons: &mut bool,
        icons: [&Option<TextureHandle>; 4], // [apps, tools, network, games]
    ) {
        ui.vertical_centered(|ui| {
            ui.add_space(12.0);
            ui.label(
                RichText::new("RobCo Program Installer")
                    .color(palette.fg)
                    .heading()
                    .strong()
                    .underline(),
            );
            ui.add_space(16.0);

            // ── Search bar ──────────────────────────────────────────────
            let search_width = ui.available_width().min(500.0);
            ui.allocate_ui_with_layout(
                egui::vec2(search_width, 32.0),
                egui::Layout::left_to_right(egui::Align::Center),
                |ui| {
                    let search_field = ui.add_sized(
                        [search_width - 80.0, 28.0],
                        egui::TextEdit::singleline(&mut state.search_query)
                            .hint_text("Search packages...")
                            .text_color(palette.fg)
                            .frame(true),
                    );
                    if search_field.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)) {
                        *deferred_search = true;
                    }
                    if ui
                        .button(RichText::new("Search").color(palette.fg))
                        .clicked()
                    {
                        *deferred_search = true;
                    }
                },
            );

            ui.add_space(24.0);

            // ── Category cards with SVG icons ───────────────────────────
            let card_size = egui::vec2(130.0, 120.0);
            let icon_size = 48.0;
            let categories = [
                (InstallerCategory::Apps, 0usize),
                (InstallerCategory::Tools, 1),
                (InstallerCategory::Network, 2),
                (InstallerCategory::Games, 3),
            ];

            ui.horizontal(|ui| {
                let total_width = categories.len() as f32 * (card_size.x + 16.0) - 16.0;
                let avail = ui.available_width();
                if avail > total_width {
                    ui.add_space((avail - total_width) / 2.0);
                }

                for (cat, icon_idx) in &categories {
                    let (resp, painter) = ui.allocate_painter(card_size, egui::Sense::click());
                    let rect = resp.rect;
                    // Card border
                    painter.rect_stroke(rect, 0.0, egui::Stroke::new(1.0, palette.fg));
                    // Hover highlight
                    if resp.hovered() {
                        painter.rect_filled(rect, 0.0, palette.hovered_bg);
                    }
                    // SVG icon (tinted to theme color)
                    if let Some(tex) = icons[*icon_idx] {
                        let icon_rect = egui::Rect::from_center_size(
                            rect.center() - egui::vec2(0.0, 14.0),
                            egui::vec2(icon_size, icon_size),
                        );
                        Self::paint_tinted_texture(&painter, tex, icon_rect, palette.fg);
                    }
                    // Label
                    painter.text(
                        egui::pos2(rect.center().x, rect.bottom() - 18.0),
                        egui::Align2::CENTER_CENTER,
                        cat.label(),
                        egui::FontId::monospace(16.0),
                        palette.fg,
                    );

                    if resp.clicked() {
                        state.search_query = cat.label().to_lowercase();
                        *deferred_search = true;
                    }
                    ui.add_space(16.0);
                }
            });

            ui.add_space(24.0);

            // ── Installed apps button ───────────────────────────────────
            let installed_btn = Self::installer_link_button(
                ui,
                RichText::new("Installed apps").color(palette.fg).heading(),
                palette,
            );
            if installed_btn.clicked() {
                *deferred_load_installed = true;
            }

            ui.add_space(8.0);

            // ── Runtime tools link ──────────────────────────────────────
            let runtime_btn = Self::installer_link_button(
                ui,
                RichText::new("Runtime Tools").color(palette.dim),
                palette,
            );
            if runtime_btn.clicked() {
                *deferred_open_runtime_tools = true;
            }

            ui.add_space(8.0);

            let addons_btn = Self::installer_link_button(
                ui,
                RichText::new("Installed Addons").color(palette.dim),
                palette,
            );
            if addons_btn.clicked() {
                *deferred_open_addons = true;
            }

            ui.add_space(8.0);

            // ── Package manager selector ─────────────────────────────────
            state.ensure_available_pms();
            if state.available_pms.len() > 1 {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Package Manager:").color(palette.dim).small());
                    let current_label = state.pm_label().to_string();
                    ui.scope(|ui| {
                        Self::apply_installer_dropdown_style(ui, palette);
                        egui::ComboBox::from_id_salt("pm_selector")
                            .selected_text(RichText::new(&current_label).color(palette.fg).small())
                            .show_ui(ui, |ui| {
                                Self::apply_installer_dropdown_style(ui, palette);
                                for (idx, pm) in state.available_pms.clone().iter().enumerate() {
                                    let selected = idx == state.selected_pm_idx;
                                    let text_color =
                                        if selected { Color32::BLACK } else { palette.fg };
                                    if ui
                                        .selectable_label(
                                            selected,
                                            RichText::new(pm.name()).color(text_color),
                                        )
                                        .clicked()
                                    {
                                        state.select_package_manager(idx);
                                    }
                                }
                            });
                    });
                });
            } else {
                ui.label(
                    RichText::new(format!("Package Manager: {}", state.pm_label()))
                        .color(palette.dim)
                        .small(),
                );
            }
        });
    }

    fn draw_installer_search_results(
        ui: &mut egui::Ui,
        state: &mut DesktopInstallerState,
        palette: RetroPalette,
        deferred_back: &mut bool,
        deferred_open_actions: &mut Option<(String, bool)>,
    ) {
        const HEADER_H: f32 = 28.0;
        const FOOTER_H: f32 = 40.0;
        const RESULTS_PER_PAGE: usize = 20;
        let total = state.search_results.len();
        let row_height = 58.0;
        let page_size = RESULTS_PER_PAGE.max(1);
        let total_pages = total.div_ceil(page_size).max(1);
        state.search_page = state.search_page.min(total_pages.saturating_sub(1));
        let start = state.search_page * page_size;
        let end = (start + page_size).min(total);
        egui::TopBottomPanel::top("inst_search_top")
            .frame(egui::Frame::none())
            .exact_height(HEADER_H)
            .show_inside(ui, |ui| {
                ui.horizontal(|ui| {
                    if ui
                        .button(RichText::new("< Back").color(palette.fg))
                        .clicked()
                    {
                        *deferred_back = true;
                    }
                    ui.add_space(8.0);
                    ui.label(
                        RichText::new(format!(
                            "Search Results: \"{}\"  ({} found)",
                            state.search_query,
                            state.search_results.len()
                        ))
                        .color(palette.fg)
                        .strong(),
                    );
                });
            });

        egui::TopBottomPanel::bottom("inst_search_bottom")
            .frame(egui::Frame::none())
            .exact_height(FOOTER_H)
            .show_inside(ui, |ui| {
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if state.search_page > 0
                        && ui
                            .button(RichText::new("< Prev").color(palette.fg))
                            .clicked()
                    {
                        state.search_page -= 1;
                    }
                    ui.add_space(8.0);
                    ui.label(
                        RichText::new(format!("Page {}/{}", state.search_page + 1, total_pages))
                            .color(palette.dim),
                    );
                    ui.add_space(8.0);
                    if state.search_page + 1 < total_pages
                        && ui
                            .button(RichText::new("Next >").color(palette.fg))
                            .clicked()
                    {
                        state.search_page += 1;
                    }
                });
            });

        let available = ui.available_rect_before_wrap();
        let body_size = egui::vec2(available.width().max(240.0), available.height().max(120.0));
        let body_rect = egui::Rect::from_min_size(available.min, body_size);
        ui.allocate_rect(body_rect, egui::Sense::hover());
        ui.scope_builder(egui::UiBuilder::new().max_rect(body_rect), |ui| {
            ui.set_min_size(body_size);
            ui.set_max_size(body_size);
            ui.style_mut().spacing.scroll = egui::style::ScrollStyle::solid();
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysVisible)
                .show(ui, |ui| {
                    let row_width = (ui.available_width() - 2.0).floor().max(220.0);
                    for idx in start..end {
                        let result = &state.search_results[idx];
                        let desc_preview = result
                            .description
                            .as_ref()
                            .cloned()
                            .or_else(|| installer_cached_package_description(state, &result.pkg))
                            .as_ref()
                            .map(|desc| Self::installer_description_preview(desc, 72));
                        let (_, row_rect) =
                            ui.allocate_space(egui::vec2(row_width, row_height - 4.0));
                        ui.scope_builder(egui::UiBuilder::new().max_rect(row_rect), |ui| {
                            let frame = egui::Frame::none()
                                .stroke(egui::Stroke::new(1.0, palette.fg))
                                .inner_margin(egui::Margin::same(2.0));
                            let content_width = (row_width - 4.0).max(80.0);
                            frame.show(ui, |ui| {
                                ui.set_min_width(content_width);
                                ui.set_max_width(content_width);
                                ui.set_min_height(row_height - 8.0);
                                let button_width = 112.0;
                                let text_width = (content_width - button_width - 24.0).max(140.0);
                                ui.horizontal(|ui| {
                                    ui.allocate_ui_with_layout(
                                        egui::vec2(text_width, 0.0),
                                        egui::Layout::left_to_right(egui::Align::Center),
                                        |ui| {
                                            let status_text = if result.installed {
                                                "[installed]"
                                            } else {
                                                "[get]"
                                            };
                                            let status_color = if result.installed {
                                                palette.dim
                                            } else {
                                                palette.fg
                                            };
                                            ui.label(
                                                RichText::new(status_text).color(status_color),
                                            );
                                            ui.add_space(6.0);
                                            ui.add_sized(
                                                [ui.available_width().max(80.0), 0.0],
                                                egui::Label::new(
                                                    RichText::new(&result.pkg)
                                                        .color(palette.fg)
                                                        .strong(),
                                                )
                                                .truncate(),
                                            );
                                        },
                                    );
                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            let btn_label = if result.installed {
                                                "Actions"
                                            } else {
                                                "Install"
                                            };
                                            if ui
                                                .add_sized(
                                                    [button_width, 24.0],
                                                    egui::Button::new(
                                                        RichText::new(format!("[ {btn_label} ]"))
                                                            .color(palette.fg),
                                                    ),
                                                )
                                                .clicked()
                                            {
                                                *deferred_open_actions =
                                                    Some((result.pkg.clone(), result.installed));
                                            }
                                        },
                                    );
                                });
                                ui.add_space(2.0);
                                let desc_text = desc_preview.unwrap_or_else(|| {
                                    if state.can_fetch_descriptions() {
                                        String::new()
                                    } else {
                                        "Description unavailable while offline.".to_string()
                                    }
                                });
                                if !desc_text.is_empty() {
                                    ui.add_sized(
                                        [(content_width - 8.0).max(80.0), 0.0],
                                        egui::Label::new(
                                            RichText::new(desc_text).color(palette.dim),
                                        )
                                        .truncate(),
                                    );
                                }
                            });
                        });
                    }
                });
        });
    }

    fn draw_installer_installed(
        ui: &mut egui::Ui,
        state: &mut DesktopInstallerState,
        palette: RetroPalette,
        deferred_back: &mut bool,
        deferred_open_actions: &mut Option<String>,
    ) {
        const HEADER_H: f32 = 28.0;
        const FOOTER_H: f32 = 40.0;
        const RESULTS_PER_PAGE: usize = 20;
        let filtered = state.filtered_installed();
        let total = filtered.len();
        let row_height = 58.0;
        let page_size = RESULTS_PER_PAGE.max(1);
        let total_pages = total.div_ceil(page_size).max(1);
        state.installed_page = state.installed_page.min(total_pages.saturating_sub(1));
        let start = state.installed_page * page_size;
        let end = (start + page_size).min(total);
        egui::TopBottomPanel::top("inst_installed_top")
            .frame(egui::Frame::none())
            .exact_height(HEADER_H)
            .show_inside(ui, |ui| {
                ui.horizontal(|ui| {
                    if ui
                        .button(RichText::new("< Back").color(palette.fg))
                        .clicked()
                    {
                        *deferred_back = true;
                    }
                    ui.add_space(8.0);
                    ui.label(RichText::new("Installed Apps").color(palette.fg).strong());
                    ui.add_space(16.0);
                    ui.label(RichText::new("Filter:").color(palette.dim));
                    ui.add_sized(
                        [200.0, 0.0],
                        egui::TextEdit::singleline(&mut state.installed_filter)
                            .hint_text("type to filter...")
                            .text_color(palette.fg),
                    );
                });
            });

        egui::TopBottomPanel::bottom("inst_installed_bottom")
            .frame(egui::Frame::none())
            .exact_height(FOOTER_H)
            .show_inside(ui, |ui| {
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if state.installed_page > 0
                        && ui
                            .button(RichText::new("< Prev").color(palette.fg))
                            .clicked()
                    {
                        state.installed_page -= 1;
                    }
                    ui.add_space(8.0);
                    ui.label(
                        RichText::new(format!(
                            "Page {}/{}  ({} packages)",
                            state.installed_page + 1,
                            total_pages,
                            total
                        ))
                        .color(palette.dim),
                    );
                    ui.add_space(8.0);
                    if state.installed_page + 1 < total_pages
                        && ui
                            .button(RichText::new("Next >").color(palette.fg))
                            .clicked()
                    {
                        state.installed_page += 1;
                    }
                });
            });

        let available = ui.available_rect_before_wrap();
        let body_size = egui::vec2(available.width().max(240.0), available.height().max(120.0));
        let body_rect = egui::Rect::from_min_size(available.min, body_size);
        ui.allocate_rect(body_rect, egui::Sense::hover());
        ui.scope_builder(egui::UiBuilder::new().max_rect(body_rect), |ui| {
            ui.set_min_size(body_size);
            ui.set_max_size(body_size);
            ui.style_mut().spacing.scroll = egui::style::ScrollStyle::solid();
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysVisible)
                .show(ui, |ui| {
                    let row_width = (ui.available_width() - 2.0).floor().max(220.0);
                    for idx in start..end {
                        let pkg = &filtered[idx];
                        let desc_preview = installer_cached_package_description(state, pkg)
                            .map(|desc| Self::installer_description_preview(&desc, 72));
                        let (_, row_rect) =
                            ui.allocate_space(egui::vec2(row_width, row_height - 4.0));
                        ui.scope_builder(egui::UiBuilder::new().max_rect(row_rect), |ui| {
                            let frame = egui::Frame::none()
                                .stroke(egui::Stroke::new(1.0, palette.fg))
                                .inner_margin(egui::Margin::same(2.0));
                            let content_width = (row_width - 4.0).max(80.0);
                            frame.show(ui, |ui| {
                                ui.set_min_width(content_width);
                                ui.set_max_width(content_width);
                                ui.set_min_height(row_height - 8.0);
                                let button_width = 112.0;
                                let text_width = (content_width - button_width - 24.0).max(140.0);
                                ui.horizontal(|ui| {
                                    ui.add_sized(
                                        [text_width, 0.0],
                                        egui::Label::new(
                                            RichText::new(pkg).color(palette.fg).strong(),
                                        )
                                        .truncate(),
                                    );
                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            if ui
                                                .add_sized(
                                                    [button_width, 24.0],
                                                    egui::Button::new(
                                                        RichText::new("[ Actions ]")
                                                            .color(palette.fg),
                                                    ),
                                                )
                                                .clicked()
                                            {
                                                *deferred_open_actions = Some(pkg.clone());
                                            }
                                        },
                                    );
                                });
                                ui.add_space(2.0);
                                let desc_text = desc_preview.unwrap_or_else(|| {
                                    if state.can_fetch_descriptions() {
                                        String::new()
                                    } else {
                                        "Description unavailable while offline.".to_string()
                                    }
                                });
                                if !desc_text.is_empty() {
                                    ui.add_sized(
                                        [(content_width - 8.0).max(80.0), 0.0],
                                        egui::Label::new(
                                            RichText::new(desc_text).color(palette.dim),
                                        )
                                        .truncate(),
                                    );
                                }
                            });
                        });
                    }
                });
        });
    }

    fn draw_installer_addons(
        ui: &mut egui::Ui,
        state: &mut DesktopInstallerState,
        palette: RetroPalette,
        deferred_back: &mut bool,
        deferred_repository_action: &mut Option<(crate::platform::AddonId, RepositoryAddonAction)>,
    ) {
        const HEADER_H: f32 = 28.0;
        const FOOTER_H: f32 = 40.0;
        let sections = installed_addon_inventory_sections();
        let total = sections.essential.len() + sections.optional.len();
        let row_height = 58.0;

        egui::TopBottomPanel::top("inst_addons_top")
            .frame(egui::Frame::none())
            .exact_height(HEADER_H + 44.0)
            .show_inside(ui, |ui| {
                ui.horizontal(|ui| {
                    if ui
                        .button(RichText::new("< Back").color(palette.fg))
                        .clicked()
                    {
                        *deferred_back = true;
                    }
                    ui.add_space(8.0);
                    ui.label(RichText::new("Installed Addons").color(palette.fg).strong());
                });
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Path:").color(palette.fg));
                    ui.add_sized(
                        [320.0, 0.0],
                        egui::TextEdit::singleline(&mut state.addon_install_path_input)
                            .hint_text("/path/to/manifest.json, addon directory, or addon archive")
                            .text_color(palette.fg),
                    );
                    if ui
                        .button(RichText::new("[ Install ]").color(palette.fg))
                        .clicked()
                    {
                        let path = state.addon_install_path_input.trim().to_string();
                        if path.is_empty() {
                            state.status = "Addon path cannot be empty.".to_string();
                        } else {
                            match install_user_addon(&path) {
                                Ok(message) => {
                                    state.status = message;
                                    state.addon_install_path_input.clear();
                                }
                                Err(err) => state.status = err,
                            }
                        }
                    }
                });
            });

        egui::TopBottomPanel::bottom("inst_addons_bottom")
            .frame(egui::Frame::none())
            .exact_height(
                FOOTER_H
                    + if sections.issues.is_empty() { 0.0 } else { 56.0 }
                    + if sections.repository_issue.is_some() {
                        40.0
                    } else {
                        0.0
                    },
            )
            .show_inside(ui, |ui| {
                ui.add_space(8.0);
                ui.label(
                    RichText::new(format!(
                        "{} installed | {} repository | {} essential | {} optional | {} issue(s)",
                        total,
                        sections.repository_available.len(),
                        sections.essential.len(),
                        sections.optional.len(),
                        sections.issues.len() + usize::from(sections.repository_issue.is_some())
                    ))
                    .color(palette.dim),
                );
                if let Some(issue) = sections.issues.first() {
                    ui.add_space(8.0);
                    ui.label(
                        RichText::new(format!(
                            "Discovery issue [{}]: {}",
                            Self::addon_scope_name(issue.scope),
                            issue.manifest_path.display()
                        ))
                        .color(Color32::from_rgb(255, 210, 120)),
                    );
                    ui.label(
                        RichText::new(&issue.detail)
                            .color(Color32::from_rgb(255, 210, 120)),
                    );
                }
                if let Some(issue) = sections.repository_issue.as_ref() {
                    ui.add_space(8.0);
                    ui.label(
                        RichText::new("Repository feed issue:")
                            .color(Color32::from_rgb(255, 210, 120)),
                    );
                    ui.label(RichText::new(issue).color(Color32::from_rgb(255, 210, 120)));
                }
            });

        let available = ui.available_rect_before_wrap();
        let body_size = egui::vec2(available.width().max(240.0), available.height().max(120.0));
        let body_rect = egui::Rect::from_min_size(available.min, body_size);
        ui.allocate_rect(body_rect, egui::Sense::hover());
        ui.scope_builder(egui::UiBuilder::new().max_rect(body_rect), |ui| {
            ui.set_min_size(body_size);
            ui.set_max_size(body_size);
            ui.style_mut().spacing.scroll = egui::style::ScrollStyle::solid();
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysVisible)
                .show(ui, |ui| {
                    let row_width = (ui.available_width() - 2.0).floor().max(220.0);
                    if !sections.essential.is_empty() {
                        ui.label(
                            RichText::new("Essential Addons")
                                .color(palette.dim)
                                .strong(),
                        );
                        ui.add_space(6.0);
                        for record in &sections.essential {
                            if matches!(
                                Self::draw_installer_addon_row(
                                    ui, palette, row_width, row_height, record,
                                ),
                                AddonRowAction::Remove
                            ) {
                                match remove_installed_addon(record.manifest.id.clone()) {
                                    Ok(message) => state.status = message,
                                    Err(err) => state.status = err,
                                }
                            }
                        }
                    }

                    if !sections.optional.is_empty() {
                        if !sections.essential.is_empty() {
                            ui.add_space(12.0);
                            ui.separator();
                            ui.add_space(12.0);
                        }
                        ui.label(RichText::new("Optional Addons").color(palette.dim).strong());
                        ui.add_space(6.0);
                        for record in &sections.optional {
                            match Self::draw_installer_addon_row(
                                ui, palette, row_width, row_height, record,
                            ) {
                                AddonRowAction::Toggle => {
                                    let next_enabled = !record.effective_enabled;
                                    let override_value =
                                        Self::addon_override_value(record, next_enabled);
                                    match set_addon_enabled_override(
                                        record.manifest.id.clone(),
                                        override_value,
                                    ) {
                                        Ok(()) => {
                                            state.status = format!(
                                                "{} {}.",
                                                record.manifest.display_name,
                                                if next_enabled { "enabled" } else { "disabled" }
                                            );
                                        }
                                        Err(err) => state.status = err,
                                    }
                                }
                                AddonRowAction::Remove => {
                                    match remove_installed_addon(record.manifest.id.clone()) {
                                        Ok(message) => state.status = message,
                                        Err(err) => state.status = err,
                                    }
                                }
                                AddonRowAction::RepositorySync(action) => {
                                    *deferred_repository_action =
                                        Some((record.manifest.id.clone(), action));
                                }
                                AddonRowAction::None => {}
                            }
                        }
                    }

                    if !sections.repository_available.is_empty() {
                        if !sections.essential.is_empty() || !sections.optional.is_empty() {
                            ui.add_space(12.0);
                            ui.separator();
                            ui.add_space(12.0);
                        }
                        ui.label(RichText::new("Repository Addons").color(palette.dim).strong());
                        ui.add_space(6.0);
                        for record in &sections.repository_available {
                            if let Some(action) = Self::draw_installer_repository_addon_row(
                                ui,
                                palette,
                                row_width,
                                row_height,
                                record,
                            ) {
                                *deferred_repository_action =
                                    Some((record.manifest.id.clone(), action));
                            }
                        }
                    }
                });
        });
    }

    fn addon_override_value(record: &InstalledAddonRecord, enabled: bool) -> Option<bool> {
        if enabled == record.manifest.enabled_by_default {
            None
        } else {
            Some(enabled)
        }
    }

    fn draw_installer_addon_row(
        ui: &mut egui::Ui,
        palette: RetroPalette,
        row_width: f32,
        row_height: f32,
        record: &InstalledAddonRecord,
    ) -> AddonRowAction {
        let (_, row_rect) = ui.allocate_space(egui::vec2(row_width, row_height - 4.0));
        let mut action = AddonRowAction::None;
        ui.scope_builder(egui::UiBuilder::new().max_rect(row_rect), |ui| {
            let frame = egui::Frame::none()
                .stroke(egui::Stroke::new(1.0, palette.fg))
                .inner_margin(egui::Margin::same(2.0));
            let content_width = (row_width - 4.0).max(80.0);
            frame.show(ui, |ui| {
                ui.set_min_width(content_width);
                ui.set_max_width(content_width);
                ui.set_min_height(row_height - 8.0);
                let action_width = if record.manifest.essential {
                    if Self::addon_can_be_removed(record) {
                        220.0
                    } else {
                        120.0
                    }
                } else if Self::addon_can_be_removed(record) {
                    320.0
                } else {
                    208.0
                };
                let text_width = (content_width - action_width - 24.0).max(140.0);
                ui.horizontal(|ui| {
                    ui.add_sized(
                        [text_width, 0.0],
                        egui::Label::new(
                            RichText::new(&record.manifest.display_name)
                                .color(palette.fg)
                                .strong(),
                        )
                        .truncate(),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if Self::addon_can_be_removed(record)
                            && ui
                                .button(RichText::new("Remove").color(palette.fg))
                                .clicked()
                        {
                            action = AddonRowAction::Remove;
                        }
                        if let Ok(Some(sync_action)) = repository_sync_action_for_manifest(&record.manifest) {
                            if ui
                                .button(RichText::new(sync_action.label()).color(palette.fg))
                                .clicked()
                            {
                                action = AddonRowAction::RepositorySync(sync_action);
                            }
                        }
                        if record.manifest.essential {
                            ui.label(RichText::new("[ required ]").color(palette.fg));
                        } else if ui
                            .button(
                                RichText::new(Self::addon_toggle_label(record)).color(palette.fg),
                            )
                            .clicked()
                        {
                            action = AddonRowAction::Toggle;
                        }
                    });
                });
                ui.add_space(2.0);
                ui.add_sized(
                    [(content_width - 8.0).max(80.0), 0.0],
                    egui::Label::new(
                        RichText::new(format!(
                            "{} | {} | {} | {}",
                            record.manifest.id,
                            Self::addon_enabled_chip(record),
                            Self::addon_scope_label(record),
                            Self::addon_source_label(record),
                        ))
                        .color(palette.dim),
                    )
                    .truncate(),
                );
            });
        });
        action
    }

    fn draw_installer_repository_addon_row(
        ui: &mut egui::Ui,
        palette: RetroPalette,
        row_width: f32,
        row_height: f32,
        record: &RepositoryAddonRecord,
    ) -> Option<RepositoryAddonAction> {
        let (_, row_rect) = ui.allocate_space(egui::vec2(row_width, row_height - 4.0));
        let mut status = None;
        ui.scope_builder(egui::UiBuilder::new().max_rect(row_rect), |ui| {
            let frame = egui::Frame::none()
                .stroke(egui::Stroke::new(1.0, palette.dim))
                .inner_margin(egui::Margin::same(2.0));
            let content_width = (row_width - 4.0).max(80.0);
            frame.show(ui, |ui| {
                ui.set_min_width(content_width);
                ui.set_max_width(content_width);
                ui.set_min_height(row_height - 8.0);
                let text_width = (content_width - 104.0).max(140.0);
                ui.horizontal(|ui| {
                    ui.add_sized(
                        [text_width, 0.0],
                        egui::Label::new(
                            RichText::new(&record.manifest.display_name)
                                .color(palette.fg)
                                .strong(),
                        )
                        .truncate(),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .button(RichText::new("Install").color(palette.fg))
                            .clicked()
                        {
                            status = Some(RepositoryAddonAction::Install);
                        }
                    });
                });
                ui.add_space(2.0);
                ui.add_sized(
                    [(content_width - 8.0).max(80.0), 0.0],
                    egui::Label::new(
                        RichText::new(format!(
                            "{} | v{} | {} | {}",
                            record.manifest.id,
                            Self::repository_release_version(record),
                            Self::repository_release_channel(record),
                            record.repository_source.display()
                        ))
                        .color(palette.dim),
                    )
                    .truncate(),
                );
            });
        });
        status
    }

    fn addon_can_be_removed(record: &InstalledAddonRecord) -> bool {
        record.manifest.scope == crate::platform::AddonScope::User && record.manifest_path.is_some()
    }

    fn addon_enabled_chip(record: &InstalledAddonRecord) -> &'static str {
        if record.manifest.essential {
            "required"
        } else if record.effective_enabled {
            "[ enabled ]"
        } else {
            "[ disabled ]"
        }
    }

    fn addon_toggle_label(record: &InstalledAddonRecord) -> &'static str {
        if record.effective_enabled {
            "Disable"
        } else {
            "Enable"
        }
    }

    fn addon_scope_label(record: &InstalledAddonRecord) -> &'static str {
        Self::addon_scope_name(record.manifest.scope)
    }

    fn addon_source_label(record: &InstalledAddonRecord) -> String {
        record
            .manifest_path
            .as_ref()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| "static fallback manifest".to_string())
    }

    fn addon_scope_name(scope: crate::platform::AddonScope) -> &'static str {
        match scope {
            crate::platform::AddonScope::Bundled => "bundled",
            crate::platform::AddonScope::System => "system",
            crate::platform::AddonScope::User => "user",
        }
    }

    fn repository_release_version(record: &RepositoryAddonRecord) -> &str {
        record
            .release
            .as_ref()
            .map(|release| release.version.as_str())
            .unwrap_or(record.manifest.version.as_str())
    }

    fn repository_release_channel(record: &RepositoryAddonRecord) -> &str {
        record
            .release
            .as_ref()
            .and_then(|release| release.channel.as_deref())
            .unwrap_or("default")
    }

    fn draw_installer_package_actions(
        ui: &mut egui::Ui,
        state: &mut DesktopInstallerState,
        palette: RetroPalette,
        pkg: &str,
        installed: bool,
        deferred_back: &mut bool,
        deferred_confirm: &mut Option<(String, InstallerPackageAction)>,
        deferred_open_add_to_menu: &mut Option<String>,
    ) {
        ui.horizontal(|ui| {
            if ui
                .button(RichText::new("< Back").color(palette.fg))
                .clicked()
            {
                *deferred_back = true;
            }
            ui.add_space(8.0);
            ui.label(RichText::new("App Details").color(palette.dim).strong());
        });
        ui.separator();
        ui.add_space(12.0);

        let description = state.fetch_package_description(pkg);
        let status_label = if installed { "Installed" } else { "Available" };

        egui::Frame::none()
            .fill(palette.panel)
            .stroke(egui::Stroke::new(1.0, palette.fg))
            .inner_margin(egui::Margin::same(18.0))
            .show(ui, |ui| {
                ui.vertical_centered(|ui| {
                    ui.label(RichText::new(pkg).color(palette.fg).heading().strong());
                    ui.add_space(4.0);
                    ui.label(
                        RichText::new(format!("{} via {}", status_label, state.pm_label()))
                            .color(palette.dim),
                    );
                });
                ui.add_space(14.0);
                ui.separator();
                ui.add_space(14.0);
                ui.label(RichText::new("Description").color(palette.fg).strong());
                ui.add_space(6.0);
                match description {
                    Some(desc) => {
                        ui.label(RichText::new(desc).color(palette.dim));
                    }
                    None => {
                        let message = if state.can_fetch_descriptions() {
                            "Description unavailable."
                        } else {
                            "Description unavailable while offline."
                        };
                        ui.label(RichText::new(message).color(palette.dim));
                    }
                }
            });

        ui.add_space(16.0);

        if installed {
            ui.horizontal_wrapped(|ui| {
                if ui
                    .button(RichText::new("[ Update ]").color(palette.fg))
                    .clicked()
                {
                    *deferred_confirm = Some((pkg.to_string(), InstallerPackageAction::Update));
                }
                ui.add_space(8.0);
                if ui
                    .button(RichText::new("[ Reinstall ]").color(palette.fg))
                    .clicked()
                {
                    *deferred_confirm = Some((pkg.to_string(), InstallerPackageAction::Reinstall));
                }
                ui.add_space(8.0);
                if ui
                    .button(RichText::new("[ Uninstall ]").color(palette.fg))
                    .clicked()
                {
                    *deferred_confirm = Some((pkg.to_string(), InstallerPackageAction::Uninstall));
                }
                ui.add_space(8.0);
                if ui
                    .button(RichText::new("[ Add to Menu ]").color(palette.fg))
                    .clicked()
                {
                    *deferred_open_add_to_menu = Some(pkg.to_string());
                }
            });
        } else if ui
            .button(RichText::new("[ Install ]").color(palette.fg))
            .clicked()
        {
            *deferred_confirm = Some((pkg.to_string(), InstallerPackageAction::Install));
        }
    }

    fn draw_installer_add_to_menu(
        ui: &mut egui::Ui,
        state: &mut DesktopInstallerState,
        palette: RetroPalette,
        pkg: &str,
        deferred_back: &mut bool,
        deferred_add: &mut Option<(String, InstallerMenuTarget)>,
    ) {
        ui.horizontal(|ui| {
            if ui
                .button(RichText::new("< Back").color(palette.fg))
                .clicked()
            {
                *deferred_back = true;
            }
            ui.add_space(8.0);
            ui.label(
                RichText::new(format!("Add \"{}\" to Menu", pkg))
                    .color(palette.fg)
                    .strong(),
            );
        });
        ui.separator();
        ui.add_space(12.0);

        ui.horizontal(|ui| {
            ui.label(RichText::new("Display Name:").color(palette.fg));
            ui.add_sized(
                [250.0, 0.0],
                egui::TextEdit::singleline(&mut state.display_name_input)
                    .hint_text(pkg)
                    .text_color(palette.fg),
            );
        });
        ui.add_space(16.0);

        ui.label(RichText::new("Choose target menu:").color(palette.fg));
        ui.add_space(8.0);

        ui.horizontal(|ui| {
            if ui
                .button(RichText::new("[ Applications ]").color(palette.fg))
                .clicked()
            {
                *deferred_add = Some((pkg.to_string(), InstallerMenuTarget::Applications));
            }
            ui.add_space(8.0);
            if ui
                .button(RichText::new("[ Games ]").color(palette.fg))
                .clicked()
            {
                *deferred_add = Some((pkg.to_string(), InstallerMenuTarget::Games));
            }
            ui.add_space(8.0);
            if ui
                .button(RichText::new("[ Network ]").color(palette.fg))
                .clicked()
            {
                *deferred_add = Some((pkg.to_string(), InstallerMenuTarget::Network));
            }
        });
    }

    fn draw_installer_runtime_tools(
        ui: &mut egui::Ui,
        state: &mut DesktopInstallerState,
        palette: RetroPalette,
        deferred_back: &mut bool,
        deferred_confirm: &mut Option<(String, InstallerPackageAction)>,
    ) {
        ui.horizontal(|ui| {
            if ui
                .button(RichText::new("< Back").color(palette.fg))
                .clicked()
            {
                *deferred_back = true;
            }
            ui.add_space(8.0);
            ui.label(RichText::new("Runtime Tools").color(palette.fg).strong());
        });
        ui.separator();
        ui.add_space(12.0);

        for (idx, tool) in available_runtime_tools().iter().copied().enumerate() {
            if idx > 0 {
                ui.add_space(12.0);
            }
            let installed = installer_runtime_tool_installed_cached(state, tool);
            let status = if installed {
                "[installed]"
            } else {
                "[not installed]"
            };
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(format!(
                        "{} {} — {}",
                        status,
                        runtime_tool_title(tool),
                        runtime_tool_description(tool)
                    ))
                    .color(palette.fg),
                );
            });
            ui.horizontal(|ui| {
                for (action_idx, action) in runtime_tool_actions(installed).iter().enumerate() {
                    let label = match action {
                        InstallerPackageAction::Install => "[ Install ]",
                        InstallerPackageAction::Update => "[ Update ]",
                        InstallerPackageAction::Reinstall => "[ Reinstall ]",
                        InstallerPackageAction::Uninstall => "[ Uninstall ]",
                    };
                    if ui.button(RichText::new(label).color(palette.fg)).clicked() {
                        *deferred_confirm =
                            runtime_tool_action_for_selection(installed, action_idx)
                                .map(|action| (runtime_tool_pkg(tool).to_string(), action));
                    }
                }
            });
        }
    }
}
