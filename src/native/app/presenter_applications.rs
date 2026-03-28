use super::super::desktop_app::DesktopWindow;
use super::desktop_window_mgmt::{
    DesktopHeaderAction, DesktopWindowRectTracking, ResizableDesktopWindowOptions,
};
use super::NucleonNativeApp;
use eframe::egui::{self, Context};
use nucleon_native_programs_app::{resolve_desktop_applications_request, DesktopProgramRequest};

impl NucleonNativeApp {
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
            header_action = Self::draw_desktop_window_header(
                ui,
                "Applications",
                maximized,
                &self.desktop_active_shell_style,
            );
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
                                DesktopProgramRequest::LaunchCatalog {
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
                                DesktopProgramRequest::LaunchCatalog {
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
}
