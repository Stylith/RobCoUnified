use super::super::desktop_app::DesktopWindow;
use super::super::desktop_app::{
    build_taskbar_entries, DesktopMenuAction, DesktopTaskbarEntry, WindowInstanceId,
};
use super::super::retro_ui::current_palette;
use crate::theme::DockPosition;
use eframe::egui::{self, Color32, Context, RichText, TopBottomPanel};

use super::RobcoNativeApp;

struct TaskbarGroup {
    kind: DesktopWindow,
    base_label: String,
    entries: Vec<DesktopTaskbarEntry>,
}

fn taskbar_base_label(label: &str) -> &str {
    let Some((base, suffix)) = label.rsplit_once(" [") else {
        return label;
    };
    let Some(number_text) = suffix.strip_suffix(']') else {
        return label;
    };
    if number_text.chars().all(|ch| ch.is_ascii_digit()) {
        base
    } else {
        label
    }
}

fn group_taskbar_entries(entries: Vec<DesktopTaskbarEntry>) -> Vec<TaskbarGroup> {
    let mut groups = Vec::new();
    for entry in entries {
        let base_label = taskbar_base_label(&entry.label).to_string();
        if let Some(group) = groups.iter_mut().find(|group: &&mut TaskbarGroup| {
            group.kind == entry.id.kind && group.base_label == base_label
        }) {
            group.entries.push(entry);
        } else {
            groups.push(TaskbarGroup {
                kind: entry.id.kind,
                base_label,
                entries: vec![entry],
            });
        }
    }
    groups
}

impl RobcoNativeApp {
    pub(super) fn draw_desktop_taskbar(
        &mut self,
        ctx: &Context,
        position: DockPosition,
        size: f32,
    ) {
        self.sync_desktop_active_window();
        let panel = match position {
            DockPosition::Bottom => TopBottomPanel::bottom("native_desktop_taskbar"),
            DockPosition::Left | DockPosition::Right => {
                TopBottomPanel::bottom("native_desktop_taskbar")
            }
            DockPosition::Hidden => return,
        };
        panel
            .exact_height(size)
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
                    let open_windows = self.all_open_window_instances();
                    let entries =
                        build_taskbar_entries(&open_windows, self.desktop_active_window, |id| {
                            self.desktop_window_title_for_instance(id)
                        });
                    let groups = group_taskbar_entries(entries);
                    for group in groups {
                        let has_multiple = group.entries.len() > 1;

                        if !has_multiple {
                            // Single instance — simple button.
                            let entry = &group.entries[0];
                            let response =
                                Self::desktop_bar_button(ui, &entry.label, entry.inactive, false);
                            if response.clicked() {
                                self.apply_desktop_menu_action(
                                    ctx,
                                    &DesktopMenuAction::ActivateTaskbarWindow(entry.id),
                                );
                                if !self.desktop_window_state(entry.id).minimized {
                                    let layer_id = egui::LayerId::new(
                                        egui::Order::Middle,
                                        self.desktop_window_egui_id(entry.id),
                                    );
                                    ctx.move_to_top(layer_id);
                                }
                            }
                        } else {
                            // Multiple instances — show combined button, right-click
                            // pops up an instance picker.
                            let any_active = group.entries.iter().any(|e| !e.inactive);
                            let popup_id = egui::Id::new((
                                "taskbar_instance_popup",
                                group.kind as u8,
                                group.base_label.as_str(),
                            ));
                            let response = Self::desktop_bar_button(
                                ui,
                                format!("{} [{}]", group.base_label, group.entries.len()),
                                !any_active,
                                false,
                            );
                            if response.clicked() {
                                // Cycle through instances on click.
                                let active_idx =
                                    group.entries.iter().position(|e| !e.inactive).unwrap_or(0);
                                let next = (active_idx + 1) % group.entries.len();
                                let next_id = group.entries[next].id;
                                self.apply_desktop_menu_action(
                                    ctx,
                                    &DesktopMenuAction::ActivateTaskbarWindow(next_id),
                                );
                                if !self.desktop_window_state(next_id).minimized {
                                    let layer_id = egui::LayerId::new(
                                        egui::Order::Middle,
                                        self.desktop_window_egui_id(next_id),
                                    );
                                    ctx.move_to_top(layer_id);
                                }
                            }
                            if response.secondary_clicked() {
                                ui.memory_mut(|mem| mem.toggle_popup(popup_id));
                            }
                            // Instance picker popup on right-click.
                            let ids: Vec<(WindowInstanceId, String, bool)> = group
                                .entries
                                .iter()
                                .map(|e| (e.id, e.label.clone(), e.inactive))
                                .collect();
                            egui::popup_below_widget(
                                ui,
                                popup_id,
                                &response,
                                egui::PopupCloseBehavior::CloseOnClick,
                                |ui| {
                                    Self::apply_desktop_panel_button_style(ui);
                                    let palette = current_palette();
                                    for (id, label, inactive) in &ids {
                                        let minimized = self.desktop_window_state(*id).minimized;
                                        let status = if minimized {
                                            " (min)"
                                        } else if !inactive {
                                            " *"
                                        } else {
                                            ""
                                        };
                                        ui.horizontal(|ui| {
                                            if ui
                                                .button(
                                                    RichText::new(format!("{}{}", label, status))
                                                        .color(palette.fg),
                                                )
                                                .clicked()
                                            {
                                                self.apply_desktop_menu_action(
                                                    ctx,
                                                    &DesktopMenuAction::ActivateTaskbarWindow(*id),
                                                );
                                            }
                                            if id.instance > 0 {
                                                if ui
                                                    .button(RichText::new("[X]").color(palette.fg))
                                                    .clicked()
                                                {
                                                    self.close_secondary_window(*id);
                                                }
                                            }
                                        });
                                    }
                                },
                            );
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn taskbar_base_label_only_strips_numeric_instance_suffix() {
        assert_eq!(taskbar_base_label("Terminal [2]"), "Terminal");
        assert_eq!(taskbar_base_label("spotify_player"), "spotify_player");
        assert_eq!(taskbar_base_label("App [beta]"), "App [beta]");
    }

    #[test]
    fn group_taskbar_entries_separates_named_pty_apps() {
        let groups = group_taskbar_entries(vec![
            DesktopTaskbarEntry {
                id: WindowInstanceId::primary(DesktopWindow::PtyApp),
                label: "Terminal".to_string(),
                inactive: false,
            },
            DesktopTaskbarEntry {
                id: WindowInstanceId {
                    kind: DesktopWindow::PtyApp,
                    instance: 1,
                },
                label: "ranger".to_string(),
                inactive: true,
            },
            DesktopTaskbarEntry {
                id: WindowInstanceId {
                    kind: DesktopWindow::PtyApp,
                    instance: 2,
                },
                label: "Terminal [2]".to_string(),
                inactive: true,
            },
        ]);

        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].base_label, "Terminal");
        assert_eq!(groups[0].entries.len(), 2);
        assert_eq!(groups[1].base_label, "ranger");
        assert_eq!(groups[1].entries.len(), 1);
    }
}
