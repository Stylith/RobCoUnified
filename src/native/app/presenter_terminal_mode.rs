use super::super::desktop_app::{DesktopWindow, WindowInstanceId};
use super::RobcoNativeApp;
use eframe::egui::Context;

impl RobcoNativeApp {
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
