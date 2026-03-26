use super::super::desktop_app::DesktopWindow;
use super::RobcoNativeApp;
use eframe::egui::Context;

impl RobcoNativeApp {
    pub(crate) fn desktop_component_file_manager_is_open(&self) -> bool {
        self.file_manager.open
    }

    pub(crate) fn desktop_component_file_manager_set_open(&mut self, open: bool) {
        self.file_manager.open = open;
    }

    pub(crate) fn desktop_component_file_manager_draw(&mut self, ctx: &Context) {
        self.draw_file_manager(ctx);
    }

    pub(crate) fn desktop_component_editor_is_open(&self) -> bool {
        self.editor.open
    }

    pub(crate) fn desktop_component_editor_set_open(&mut self, open: bool) {
        self.editor.open = open;
    }

    pub(crate) fn desktop_component_editor_draw(&mut self, ctx: &Context) {
        self.draw_editor(ctx);
    }

    pub(crate) fn desktop_component_editor_on_closed(&mut self) {
        if self.desktop_mode_open {
            self.editor.reset_for_desktop_new_document();
            self.editor.ui.reset_search();
        }
    }

    pub(crate) fn desktop_component_settings_is_open(&self) -> bool {
        self.settings.open
    }

    pub(crate) fn desktop_component_settings_set_open(&mut self, open: bool) {
        self.settings.open = open;
    }

    pub(crate) fn desktop_component_settings_draw(&mut self, ctx: &Context) {
        self.draw_settings(ctx);
    }

    pub(crate) fn desktop_component_settings_on_open(&mut self, _was_open: bool) {
        let requested_panel = self.pending_settings_panel.take();
        self.reset_desktop_settings_window();
        if let Some(panel) = requested_panel {
            self.settings.panel = self.coerce_desktop_settings_panel(panel);
        }
        self.prime_desktop_window_defaults(DesktopWindow::Settings);
    }

    pub(crate) fn desktop_component_applications_is_open(&self) -> bool {
        self.applications.open
    }

    pub(crate) fn desktop_component_applications_set_open(&mut self, open: bool) {
        self.applications.open = open;
    }

    pub(crate) fn desktop_component_applications_draw(&mut self, ctx: &Context) {
        self.draw_applications(ctx);
    }

    pub(crate) fn desktop_component_installer_is_open(&self) -> bool {
        self.desktop_installer.open
    }

    pub(crate) fn desktop_component_installer_set_open(&mut self, open: bool) {
        self.desktop_installer.open = open;
    }

    pub(crate) fn desktop_component_installer_draw(&mut self, ctx: &Context) {
        self.draw_installer(ctx);
    }

    pub(crate) fn desktop_component_installer_on_open(&mut self, was_open: bool) {
        if !was_open {
            self.prime_desktop_window_defaults(DesktopWindow::Installer);
        }
    }

    pub(crate) fn desktop_component_terminal_mode_is_open(&self) -> bool {
        self.terminal_mode.open
    }

    pub(crate) fn desktop_component_terminal_mode_set_open(&mut self, open: bool) {
        self.terminal_mode.open = open;
    }

    pub(crate) fn desktop_component_terminal_mode_draw(&mut self, ctx: &Context) {
        self.draw_terminal_mode(ctx);
    }

    pub(crate) fn desktop_component_terminal_mode_on_open(&mut self, was_open: bool) {
        if !was_open {
            self.prime_desktop_window_defaults(DesktopWindow::TerminalMode);
        }
    }

    pub(crate) fn desktop_component_pty_is_open(&self) -> bool {
        self.primary_desktop_pty_open()
    }

    pub(crate) fn desktop_component_pty_set_open(&mut self, open: bool) {
        if !open && self.primary_desktop_pty_open() {
            if let Some(mut pty) = self.take_primary_pty() {
                pty.session.terminate();
            }
        }
    }

    pub(crate) fn desktop_component_pty_draw(&mut self, ctx: &Context) {
        self.draw_desktop_pty_window(ctx);
    }

    pub(crate) fn desktop_component_pty_on_open(&mut self, was_open: bool) {
        if !was_open {
            self.prime_desktop_window_defaults(DesktopWindow::PtyApp);
        }
    }
}
