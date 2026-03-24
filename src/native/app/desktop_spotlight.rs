use super::super::desktop_app::{DesktopShellAction, DesktopWindow};
use super::super::desktop_search_service::{
    gather_spotlight_results, spotlight_category_tag, NativeSpotlightCategory,
    NativeSpotlightResult,
};
use super::super::desktop_session_service::active_session_username as active_native_session_username;
use super::super::editor_app::EDITOR_APP_TITLE;
use super::super::retro_ui::current_palette;
use eframe::egui::{self, Color32, Context, Key, RichText, TextEdit};

const BUILTIN_NUKE_CODES_APP: &str = "Nuke Codes";
const BUILTIN_TEXT_EDITOR_APP: &str = EDITOR_APP_TITLE;

use super::RobcoNativeApp;

impl RobcoNativeApp {
    pub(super) fn spotlight_gather_results(&mut self) {
        let query = self.spotlight_query.to_lowercase();
        let tab = self.spotlight_tab;
        // Skip if query+tab haven't changed
        if query == self.spotlight_last_query && tab == self.spotlight_last_tab {
            return;
        }
        self.spotlight_last_query = query.clone();
        self.spotlight_last_tab = tab;
        let active_username = active_native_session_username();
        self.spotlight_results = gather_spotlight_results(
            &query,
            tab,
            active_username.as_deref(),
            BUILTIN_TEXT_EDITOR_APP,
            BUILTIN_NUKE_CODES_APP,
        );
        self.spotlight_selected = 0;
    }

    pub(super) fn spotlight_activate_result(&mut self, result: &NativeSpotlightResult) {
        self.close_spotlight();
        self.spotlight_query.clear();
        if let Some(action) = self.spotlight_action_for_result(result) {
            self.execute_desktop_shell_action(action);
        }
    }

    pub(super) fn draw_spotlight(&mut self, ctx: &Context) {
        if !self.spotlight_open {
            return;
        }

        // Close on Escape
        if ctx.input(|i| i.key_pressed(Key::Escape)) {
            self.close_spotlight();
            return;
        }

        // Arrow key navigation
        let mut scroll_selected_into_view = false;
        if ctx.input(|i| i.key_pressed(Key::ArrowDown)) {
            if !self.spotlight_results.is_empty() {
                let next = (self.spotlight_selected + 1).min(self.spotlight_results.len() - 1);
                if next != self.spotlight_selected {
                    self.spotlight_selected = next;
                    scroll_selected_into_view = true;
                }
            }
        }
        if ctx.input(|i| i.key_pressed(Key::ArrowUp)) {
            let next = self.spotlight_selected.saturating_sub(1);
            if next != self.spotlight_selected {
                self.spotlight_selected = next;
                scroll_selected_into_view = true;
            }
        }
        if ctx.input(|i| i.key_pressed(Key::ArrowRight)) {
            self.move_spotlight_tab(1);
            scroll_selected_into_view = true;
        }
        if ctx.input(|i| i.key_pressed(Key::ArrowLeft)) {
            self.move_spotlight_tab(-1);
            scroll_selected_into_view = true;
        }
        if ctx.input(|i| i.key_pressed(Key::Tab) && !i.modifiers.shift) {
            self.move_spotlight_tab(1);
            scroll_selected_into_view = true;
            ctx.input_mut(|i| {
                i.consume_key(egui::Modifiers::NONE, Key::Tab);
            });
        }
        if ctx.input(|i| i.key_pressed(Key::Tab) && i.modifiers.shift) {
            self.move_spotlight_tab(-1);
            scroll_selected_into_view = true;
            ctx.input_mut(|i| {
                i.consume_key(
                    egui::Modifiers {
                        shift: true,
                        ..Default::default()
                    },
                    Key::Tab,
                );
            });
        }

        // Enter to activate
        let mut activate_idx: Option<usize> = None;
        if ctx.input(|i| i.key_pressed(Key::Enter)) && !self.spotlight_results.is_empty() {
            activate_idx = Some(self.spotlight_selected);
        }

        // Gather results
        let prev_query = self.spotlight_last_query.clone();
        let prev_tab = self.spotlight_last_tab;
        self.spotlight_gather_results();
        if self.spotlight_last_query != prev_query || self.spotlight_last_tab != prev_tab {
            scroll_selected_into_view = true;
        }

        let palette = current_palette();
        let screen = ctx.screen_rect();
        let box_width = 600.0_f32.min(screen.width() - 40.0);
        let box_height = 420.0_f32.min(screen.height() - 80.0);

        egui::Window::new("spotlight_window")
            .title_bar(false)
            .resizable(false)
            .collapsible(false)
            .fixed_size(egui::vec2(box_width, box_height))
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
            .order(egui::Order::Foreground)
            .frame(
                egui::Frame::none()
                    .fill(palette.bg)
                    .stroke(egui::Stroke::new(2.0, palette.fg))
                    .shadow(egui::epaint::Shadow::NONE)
                    .inner_margin(egui::Margin::same(12.0)),
            )
            .show(ctx, |ui| {
                let v = ui.visuals_mut();
                v.override_text_color = Some(palette.fg);
                v.extreme_bg_color = palette.bg;
                v.selection.bg_fill = palette.fg;
                v.selection.stroke = egui::Stroke::new(1.0, palette.fg);
                // noninteractive (labels, frames)
                v.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0, palette.fg);
                v.widgets.noninteractive.bg_fill = Color32::TRANSPARENT;
                v.widgets.noninteractive.weak_bg_fill = Color32::TRANSPARENT;
                v.widgets.noninteractive.bg_stroke = egui::Stroke::NONE;
                // inactive (buttons at rest)
                v.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, palette.fg);
                v.widgets.inactive.bg_fill = Color32::TRANSPARENT;
                v.widgets.inactive.weak_bg_fill = Color32::TRANSPARENT;
                v.widgets.inactive.bg_stroke = egui::Stroke::new(1.0, palette.fg);
                // hovered
                v.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, palette.fg);
                v.widgets.hovered.bg_fill = palette.panel;
                v.widgets.hovered.weak_bg_fill = palette.panel;
                v.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, palette.fg);
                v.widgets.hovered.expansion = 0.0;
                // active (pressed)
                v.widgets.active.fg_stroke = egui::Stroke::new(1.0, Color32::BLACK);
                v.widgets.active.bg_fill = palette.fg;
                v.widgets.active.weak_bg_fill = palette.fg;
                v.widgets.active.bg_stroke = egui::Stroke::new(1.0, palette.fg);

                // Search input
                let search_resp = ui.add(
                    TextEdit::singleline(&mut self.spotlight_query)
                        .desired_width(box_width - 48.0)
                        .hint_text("Search apps, documents, files…")
                        .font(egui::TextStyle::Body),
                );
                // Auto-focus
                if search_resp.gained_focus() || !search_resp.has_focus() {
                    search_resp.request_focus();
                }

                ui.add_space(6.0);

                // Tab buttons
                ui.horizontal(|ui| {
                    let tabs = ["All", "Apps", "Documents", "Files"];
                    for (i, label) in tabs.iter().enumerate() {
                        let selected = self.spotlight_tab == i as u8;
                        let text = if selected {
                            RichText::new(*label).color(Color32::BLACK).strong()
                        } else {
                            RichText::new(*label).color(palette.fg)
                        };
                        let btn = egui::Button::new(text);
                        let btn = if selected {
                            btn.fill(palette.fg)
                        } else {
                            btn.fill(palette.panel)
                        };
                        if ui.add(btn).clicked() {
                            self.set_spotlight_tab(i as u8);
                        }
                    }
                });

                ui.add_space(4.0);

                // Results
                let results_height = ui.available_height();
                egui::ScrollArea::vertical()
                    .max_height(results_height)
                    .auto_shrink(false)
                    .show(ui, |ui| {
                        if self.spotlight_results.is_empty() {
                            if self.spotlight_query.is_empty() {
                                ui.label(RichText::new("Type to search…").color(palette.dim));
                            } else {
                                ui.label(RichText::new("No results found.").color(palette.dim));
                            }
                        } else {
                            for (i, result) in self.spotlight_results.iter().enumerate() {
                                let selected = i == self.spotlight_selected;
                                let cat_label = spotlight_category_tag(&result.category);
                                let display = format!("[{cat_label}]  {}", result.name);
                                let text_color = if selected { Color32::BLACK } else { palette.fg };
                                let resp = ui.add(egui::SelectableLabel::new(
                                    selected,
                                    RichText::new(display).color(text_color),
                                ));
                                if resp.clicked() {
                                    activate_idx = Some(i);
                                }
                                if selected && scroll_selected_into_view {
                                    resp.scroll_to_me(None);
                                }
                            }
                        }
                    });
            });

        // Activate after UI is done (deferred to avoid borrow issues)
        if let Some(idx) = activate_idx {
            if idx < self.spotlight_results.len() {
                let result = self.spotlight_results[idx].clone();
                self.spotlight_activate_result(&result);
            }
        }
    }

    pub(super) fn open_spotlight(&mut self) {
        self.close_start_menu();
        self.spotlight_open = true;
        self.spotlight_tab = 0;
        self.spotlight_query.clear();
        self.spotlight_selected = 0;
        self.spotlight_results.clear();
        self.spotlight_last_query.clear();
        self.spotlight_last_tab = u8::MAX;
    }

    pub(super) fn close_spotlight(&mut self) {
        self.spotlight_open = false;
    }

    pub(super) fn set_spotlight_tab(&mut self, tab: u8) {
        let next = tab.min(3);
        if self.spotlight_tab == next {
            return;
        }
        self.spotlight_tab = next;
        self.spotlight_selected = 0;
        self.spotlight_last_tab = u8::MAX;
    }

    pub(super) fn move_spotlight_tab(&mut self, delta: i8) {
        let current = self.spotlight_tab as i8;
        let next = (current + delta).clamp(0, 3) as u8;
        self.set_spotlight_tab(next);
    }

    pub(super) fn close_desktop_overlays(&mut self) {
        self.close_start_menu();
        self.close_spotlight();
    }

    pub(super) fn spotlight_action_for_result(
        &self,
        result: &NativeSpotlightResult,
    ) -> Option<DesktopShellAction> {
        match &result.category {
            NativeSpotlightCategory::System => match result.name.as_str() {
                "File Manager" => Some(DesktopShellAction::LaunchByTarget(
                    super::launch_registry::file_manager_launch_target(),
                )),
                "Settings" => Some(DesktopShellAction::LaunchByTarget(
                    super::launch_registry::settings_launch_target(),
                )),
                "Terminal" => Some(DesktopShellAction::OpenWindow(DesktopWindow::TerminalMode)),
                n if n == BUILTIN_TEXT_EDITOR_APP => Some(DesktopShellAction::OpenTextEditor),
                n if n == BUILTIN_NUKE_CODES_APP => {
                    Some(DesktopShellAction::OpenWindow(DesktopWindow::NukeCodes))
                }
                _ => None,
            },
            NativeSpotlightCategory::App => {
                Some(DesktopShellAction::LaunchConfiguredApp(result.name.clone()))
            }
            NativeSpotlightCategory::Game => {
                Some(DesktopShellAction::LaunchGameProgram(result.name.clone()))
            }
            NativeSpotlightCategory::Network => Some(DesktopShellAction::LaunchNetworkProgram(
                result.name.clone(),
            )),
            NativeSpotlightCategory::Document => result
                .path
                .clone()
                .map(DesktopShellAction::OpenPathInEditor),
            NativeSpotlightCategory::File => result
                .path
                .clone()
                .map(DesktopShellAction::RevealPathInFileManager),
        }
    }
}
