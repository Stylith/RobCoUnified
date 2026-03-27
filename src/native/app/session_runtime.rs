use super::super::desktop_session_service::{
    logout_flash_plan, persist_shell_snapshot as persist_native_shell_snapshot,
    restore_session_plan as build_native_session_restore_plan,
};
use super::super::desktop_status_service::{clear_settings_status, clear_shell_status};
use super::super::editor_app::EditorWindow;
use super::super::installer_screen::DesktopInstallerState;
use super::super::menu::{terminal_runtime_defaults, TerminalNavigationState, TerminalScreen};
use super::RobcoNativeApp;
use super::SessionState;
use crate::core::auth::UserRecord;
use robcos_native_settings_app::desktop_settings_default_panel;

impl RobcoNativeApp {
    pub(super) fn restore_for_user(&mut self, username: &str, user: &UserRecord) {
        let settings = crate::native::desktop_settings_service::reload_settings_snapshot();
        let plan = build_native_session_restore_plan(username, user, settings.default_open_mode);
        self.session = Some(SessionState {
            username: plan.identity.username,
            is_admin: plan.identity.is_admin,
        });
        self.login.hacking = None;
        self.file_manager.cwd = plan.file_manager_dir;
        self.file_manager.open = false;
        self.file_manager.selected = None;
        self.editor = EditorWindow::default();
        self.replace_settings_draft(settings);
        self.apply_status_update(clear_settings_status());
        self.settings.panel = desktop_settings_default_panel();
        self.desktop_installer = DesktopInstallerState::default();
        self.terminal_mode.status.clear();
        self.reset_shell_runtime_for_session(plan.launch_default_desktop);
        self.apply_status_update(clear_shell_status());
    }

    fn reset_shell_runtime_for_session(&mut self, launch_default_desktop: bool) {
        let terminal_defaults = terminal_runtime_defaults();
        self.desktop_window_states.clear();
        self.desktop_active_window = None;
        self.pending_settings_panel = None;
        self.start_open = !launch_default_desktop;
        self.start_selected_root = 0;
        self.start_system_selected = 0;
        self.start_leaf_selected = 0;
        self.start_open_submenu = None;
        self.start_open_leaf = None;
        self.desktop_mode_open = launch_default_desktop;
        self.apply_terminal_navigation_state(terminal_defaults);
        if let Some(mut pty) = self.take_primary_pty() {
            pty.session.terminate();
        }
        self.clear_terminal_wasm_addon();
        self.clear_desktop_wasm_addon();
        Self::terminate_secondary_window_ptys(&mut self.secondary_windows);
        self.secondary_windows.clear();
        self.terminal_installer.reset();
        self.terminal_edit_menus.reset();
        self.terminal_connections.reset();
        self.terminal_prompt = None;
        self.terminal_flash = None;
        self.session_leader_until = None;
        self.terminal_tweaks_surface_dropdown_open = false;
        self.terminal_tweaks_open_dropdown = None;
        self.terminal_tweaks_desktop_expanded_menu = Some(0);
        self.terminal_tweaks_terminal_expanded_menu = Some(0);
    }

    pub(super) fn current_terminal_navigation_state(&self) -> TerminalNavigationState {
        self.terminal_nav.clone()
    }

    pub(super) fn apply_terminal_navigation_state(&mut self, state: TerminalNavigationState) {
        self.terminal_nav = state;
    }

    pub(super) fn persist_snapshot(&self) {
        if let Some(session) = &self.session {
            persist_native_shell_snapshot(
                &session.username,
                &self.file_manager.cwd,
                self.editor.path.as_deref(),
            );
        }
    }

    pub(super) fn begin_logout(&mut self) {
        let already_logging_out = self.terminal_flash.as_ref().is_some_and(|flash| {
            matches!(
                &flash.action,
                crate::native::prompt::FlashAction::FinishLogout
            )
        });
        let Some(plan) = logout_flash_plan(already_logging_out) else {
            return;
        };
        crate::sound::play_logout();
        self.persist_snapshot();
        self.terminate_all_native_pty_children();
        self.terminal_prompt = None;
        self.terminal_nav.screen = TerminalScreen::MainMenu;
        self.close_start_menu();
        self.desktop_mode_open = false;
        self.desktop_active_window = None;
        self.session_leader_until = None;
        self.queue_session_flash_plan(plan);
    }

    pub(super) fn finish_logout(&mut self) {
        let _ = crate::native::desktop_settings_service::reload_settings_snapshot();
        self.terminate_all_native_pty_children();
        crate::native::desktop_session_service::clear_all_sessions();
        self.session_runtime.clear();
        self.session = None;
        self.login.reset();
        self.file_manager.open = false;
        self.editor.open = false;
        self.settings.open = false;
        self.settings.panel = desktop_settings_default_panel();
        self.applications.open = false;
        self.terminal_mode.open = false;
        self.reset_shell_runtime_for_logout();
        self.apply_status_update(clear_shell_status());
    }

    fn reset_shell_runtime_for_logout(&mut self) {
        self.reset_shell_runtime_for_session(false);
        self.terminal_pty = None;
        self.terminal_pty_surface = None;
    }
}
