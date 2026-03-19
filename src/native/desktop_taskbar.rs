use super::app::RobcoNativeApp;
use super::desktop_app::{
    build_taskbar_entries, desktop_components, DesktopMenuAction, DesktopWindow,
};
use super::retro_ui::current_palette;
use eframe::egui::{self, Color32, Context, RichText, TopBottomPanel};

impl RobcoNativeApp {
    pub(super) fn draw_desktop_taskbar(&mut self, ctx: &Context) {
        self.sync_desktop_active_window();
        TopBottomPanel::bottom("native_desktop_taskbar")
            .exact_height(32.0)
            .show_separator_line(false)
            .show(ctx, |ui| {
                let palette = current_palette();
                ui.painter()
                    .rect_filled(ui.max_rect(), 0.0, palette.selected_bg);

                ui.horizontal(|ui| {
                    Self::apply_desktop_panel_button_style(ui);
                    ui.spacing_mut().item_spacing.x = 8.0;
                    let start_response = ui.add(
                        egui::Label::new(
                            RichText::new("[Start]")
                                .strong()
                                .monospace()
                                .color(Color32::BLACK),
                        )
                        .sense(egui::Sense::click()),
                    );
                    self.desktop_start_button_rect = Some(start_response.rect);
                    if start_response.clicked() {
                        if self.start_open {
                            self.close_start_menu();
                        } else {
                            self.open_start_menu();
                        }
                    }
                    ui.label(RichText::new("|").monospace().color(Color32::BLACK));
                    ui.add_space(8.0);
                    let open_windows: Vec<DesktopWindow> = desktop_components()
                        .iter()
                        .filter(|component| component.spec.show_in_taskbar)
                        .map(|component| component.spec.window)
                        .filter(|window| self.desktop_window_is_open(*window))
                        .collect();
                    let entries = build_taskbar_entries(
                        &open_windows,
                        self.desktop_active_window,
                        self.terminal_pty.as_ref().map(|pty| pty.title.as_str()),
                    );
                    for entry in entries {
                        if Self::desktop_bar_button(ui, entry.label, entry.inactive, false)
                            .clicked()
                        {
                            self.apply_desktop_menu_action(
                                ctx,
                                &DesktopMenuAction::ActivateTaskbarWindow(entry.window),
                            );
                            if !self.desktop_window_is_minimized(entry.window) {
                                let layer_id = egui::LayerId::new(
                                    egui::Order::Middle,
                                    self.desktop_window_egui_id(entry.window),
                                );
                                ctx.move_to_top(layer_id);
                            }
                        }
                    }
                });
            });
    }

    pub(super) fn desktop_bar_button(
        ui: &mut egui::Ui,
        label: impl Into<String>,
        active: bool,
        bold: bool,
    ) -> egui::Response {
        let palette = current_palette();
        let label = label.into();
        let fill = if active { palette.fg } else { palette.panel };
        let text = if active {
            RichText::new(label.clone()).color(Color32::BLACK)
        } else {
            RichText::new(label.clone()).color(palette.fg)
        };
        let text = if bold { text.strong() } else { text };
        let response = ui.add(
            egui::Button::new(text)
                .fill(fill)
                .stroke(egui::Stroke::new(2.0, palette.fg)),
        );
        if active {
            let text = if bold {
                RichText::new(label).strong()
            } else {
                RichText::new(label)
            };
            let font = egui::TextStyle::Button.resolve(ui.style());
            ui.painter().text(
                response.rect.center(),
                egui::Align2::CENTER_CENTER,
                text.text(),
                font,
                Color32::BLACK,
            );
        }
        response
    }

    pub(super) fn apply_desktop_panel_button_style(ui: &mut egui::Ui) {
        let palette = current_palette();
        let mut style = ui.style().as_ref().clone();
        let stroke = egui::Stroke::new(2.0, palette.fg);
        style.visuals.override_text_color = None;
        style.visuals.window_stroke = stroke;
        style.visuals.window_rounding = egui::Rounding::ZERO;
        style.visuals.menu_rounding = egui::Rounding::ZERO;
        style.visuals.window_shadow = egui::epaint::Shadow::NONE;
        style.visuals.popup_shadow = egui::epaint::Shadow::NONE;
        style.visuals.selection.bg_fill = palette.panel;
        style.visuals.selection.stroke = stroke;
        style.visuals.widgets.noninteractive.bg_fill = palette.panel;
        style.visuals.widgets.noninteractive.weak_bg_fill = palette.panel;
        style.visuals.widgets.noninteractive.bg_stroke = stroke;
        style.visuals.widgets.noninteractive.fg_stroke = stroke;
        style.visuals.widgets.noninteractive.rounding = egui::Rounding::ZERO;
        style.visuals.widgets.noninteractive.expansion = 0.0;
        style.visuals.widgets.inactive.bg_fill = palette.panel;
        style.visuals.widgets.inactive.weak_bg_fill = palette.panel;
        style.visuals.widgets.inactive.bg_stroke = stroke;
        style.visuals.widgets.inactive.fg_stroke = stroke;
        style.visuals.widgets.inactive.rounding = egui::Rounding::ZERO;
        style.visuals.widgets.inactive.expansion = 0.0;
        for visuals in [
            &mut style.visuals.widgets.hovered,
            &mut style.visuals.widgets.active,
            &mut style.visuals.widgets.open,
        ] {
            visuals.bg_fill = palette.panel;
            visuals.weak_bg_fill = palette.panel;
            visuals.bg_stroke = stroke;
            visuals.fg_stroke = stroke;
            visuals.rounding = egui::Rounding::ZERO;
            visuals.expansion = 0.0;
        }
        ui.set_style(style);
    }
}
