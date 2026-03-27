use super::super::desktop_app::{DesktopWindow, WindowInstanceId};
use super::super::desktop_session_service::{
    bind_login_identity, last_session_username, user_record as session_user_record,
};
use super::super::desktop_status_service::{clear_settings_status, clear_shell_status};
use super::RobcoNativeApp;
use super::SecondaryWindowApp;
use crate::config::{get_current_user, OpenMode};
use crate::core::auth::AuthMethod;
use eframe::egui::{self, Context};
use robcos_native_settings_app::{desktop_settings_default_panel, NativeSettingsPanel};
use std::path::PathBuf;
use std::time::Duration;

const NUCLEON_AUTOLOGIN_USER_ENV: &str = "NUCLEON_AUTOLOGIN_USER";
const LEGACY_ROBCOS_AUTOLOGIN_USER_ENV: &str = "ROBCOS_AUTOLOGIN_USER";

fn autologin_user_override() -> Option<String> {
    [NUCLEON_AUTOLOGIN_USER_ENV, LEGACY_ROBCOS_AUTOLOGIN_USER_ENV]
        .into_iter()
        .find_map(|name| {
            std::env::var(name)
                .ok()
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
        })
}

impl RobcoNativeApp {
    pub(super) fn apply_autologin_open_mode(&mut self) {
        if matches!(self.settings.draft.default_open_mode, OpenMode::Desktop) {
            self.desktop_mode_open = true;
            self.close_start_menu();
            self.sync_desktop_active_window();
        }
    }

    pub(super) fn maybe_apply_profile_autologin(&mut self) {
        if self.session.is_some() {
            return;
        }
        let Some(username) = autologin_user_override() else {
            return;
        };
        let Some(user) = session_user_record(&username) else {
            return;
        };
        if user.auth_method != AuthMethod::NoPassword {
            return;
        }
        bind_login_identity(&username);
        self.ensure_login_session_entry(&username);
        self.restore_for_user(&username, &user);
        self.apply_autologin_open_mode();
    }

    fn restore_standalone_session_identity(&mut self, session_username: Option<String>) {
        let session_username = session_username
            .and_then(|username| {
                let trimmed = username.trim().to_string();
                (!trimmed.is_empty()).then_some(trimmed)
            })
            .or_else(get_current_user)
            .or_else(last_session_username);
        if let Some(username) = session_username {
            if let Some(user) = session_user_record(&username) {
                bind_login_identity(&username);
                self.ensure_login_session_entry(&username);
                self.restore_for_user(&username, &user);
            }
        }
    }

    fn prepare_standalone_window_shell(
        &mut self,
        session_username: Option<String>,
        desktop_mode_open: bool,
    ) {
        self.restore_standalone_session_identity(session_username);
        self.desktop_window_states.clear();
        self.close_desktop_overlays();
        self.terminal_prompt = None;
        self.pending_settings_panel = None;
        self.desktop_mode_open = desktop_mode_open;
        self.desktop_active_window = None;
        self.apply_status_update(clear_shell_status());
    }

    pub(crate) fn prepare_standalone_settings_window(
        &mut self,
        session_username: Option<String>,
        panel: Option<NativeSettingsPanel>,
    ) {
        self.prepare_standalone_window_shell(session_username, false);
        self.reset_desktop_settings_window();
        self.prime_desktop_window_defaults(DesktopWindow::Settings);
        self.settings.open = true;
        self.settings.panel = self
            .coerce_desktop_settings_panel(panel.unwrap_or_else(desktop_settings_default_panel));
        self.file_manager.open = false;
        self.picking_icon_for_shortcut = None;
        self.picking_wallpaper = false;
        self.desktop_active_window = Some(WindowInstanceId::primary(DesktopWindow::Settings));
        self.apply_status_update(clear_settings_status());
    }

    pub(crate) fn update_standalone_settings_window(&mut self, ctx: &Context) {
        self.process_background_results(ctx);
        self.maybe_sync_settings_from_disk(ctx);
        self.sync_desktop_appearance(ctx);
        self.sync_terminal_appearance();
        self.sync_native_display_effects();
        self.dispatch_context_menu_action(ctx);
        if self.terminal_prompt.is_some() {
            self.handle_terminal_prompt_input(ctx);
            self.consume_terminal_prompt_keys(ctx);
        }
        let file_manager_first = self.active_window_kind() != Some(DesktopWindow::FileManager)
            || !self.file_manager.open;
        if file_manager_first {
            self.draw_file_manager(ctx);
            self.draw_settings(ctx);
        } else {
            self.draw_settings(ctx);
            self.draw_file_manager(ctx);
        }
        self.draw_terminal_prompt_overlay_global(ctx);
        if !self.settings.open && !self.file_manager.open && self.terminal_prompt.is_none() {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        ctx.request_repaint_after(Duration::from_millis(500));
    }

    pub(crate) fn prepare_standalone_tweaks_window(&mut self, session_username: Option<String>) {
        self.prepare_standalone_window_shell(session_username, true);
        self.prime_desktop_window_defaults(DesktopWindow::Tweaks);
        self.tweaks_open = true;
        self.desktop_active_window = Some(WindowInstanceId::primary(DesktopWindow::Tweaks));
    }

    pub(crate) fn update_standalone_tweaks_window(&mut self, ctx: &Context) {
        self.process_background_results(ctx);
        self.maybe_sync_settings_from_disk(ctx);
        self.sync_desktop_appearance(ctx);
        self.sync_terminal_appearance();
        self.sync_native_display_effects();
        self.draw_tweaks(ctx);
        if !self.tweaks_open {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        ctx.request_repaint_after(Duration::from_millis(500));
    }

    pub(crate) fn prepare_standalone_editor_window(
        &mut self,
        session_username: Option<String>,
        start_path: Option<PathBuf>,
    ) {
        self.prepare_standalone_window_shell(session_username, true);
        self.file_manager.open = false;
        self.picking_icon_for_shortcut = None;
        self.picking_wallpaper = false;
        self.editor.reset_for_desktop_new_document();
        self.editor.status.clear();
        self.editor.ui.reset_search();
        self.prime_desktop_window_defaults(DesktopWindow::Editor);
        if let Some(path) = start_path {
            self.open_embedded_path_in_editor(path);
        } else {
            self.new_document();
        }
        self.desktop_active_window = Some(WindowInstanceId::primary(DesktopWindow::Editor));
    }

    pub(crate) fn update_standalone_editor_window(&mut self, ctx: &Context) {
        self.process_background_results(ctx);
        self.maybe_sync_settings_from_disk(ctx);
        self.sync_desktop_appearance(ctx);
        self.sync_terminal_appearance();
        self.sync_native_display_effects();
        if self.terminal_prompt.is_some() {
            self.handle_terminal_prompt_input(ctx);
            self.consume_terminal_prompt_keys(ctx);
        }
        let file_manager_first = self.active_window_kind() != Some(DesktopWindow::FileManager)
            || !self.file_manager.open;
        if file_manager_first {
            self.draw_file_manager(ctx);
            self.draw_editor(ctx);
        } else {
            self.draw_editor(ctx);
            self.draw_file_manager(ctx);
        }
        self.draw_terminal_prompt_overlay_global(ctx);
        self.maybe_intercept_viewport_close_for_unsaved_editor(ctx);
        if !self.editor.open && !self.file_manager.open && self.terminal_prompt.is_none() {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        ctx.request_repaint_after(Duration::from_millis(500));
    }

    fn dirty_editor_window_for_close_request(&self) -> Option<WindowInstanceId> {
        if let Some(active) = self.desktop_active_window {
            if active.kind == DesktopWindow::Editor {
                if active.instance == 0 {
                    if self.editor.open && self.editor.dirty {
                        return Some(active);
                    }
                } else if self.secondary_windows.iter().any(|window| {
                    window.id == active
                        && matches!(&window.app, SecondaryWindowApp::Editor(editor) if editor.open && editor.dirty)
                }) {
                    return Some(active);
                }
            }
        }
        if self.editor.open && self.editor.dirty {
            return Some(WindowInstanceId::primary(DesktopWindow::Editor));
        }
        self.secondary_windows
            .iter()
            .find_map(|window| match &window.app {
                SecondaryWindowApp::Editor(editor) if editor.open && editor.dirty => {
                    Some(window.id)
                }
                _ => None,
            })
    }

    pub(super) fn maybe_intercept_viewport_close_for_unsaved_editor(&mut self, ctx: &Context) {
        if !self.desktop_mode_open {
            if ctx.input(|i| i.viewport().close_requested()) {
                self.persist_snapshot();
            }
            return;
        }
        if !ctx.input(|i| i.viewport().close_requested()) {
            return;
        }
        if let Some(id) = self.dirty_editor_window_for_close_request() {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.request_close_window_instance(id);
        } else {
            self.persist_snapshot();
        }
    }

    pub(crate) fn prepare_standalone_applications_window(
        &mut self,
        session_username: Option<String>,
    ) {
        self.prepare_standalone_window_shell(session_username, true);
        self.applications.status.clear();
        self.prime_desktop_window_defaults(DesktopWindow::Applications);
        self.applications.open = true;
        self.desktop_active_window = Some(WindowInstanceId::primary(DesktopWindow::Applications));
    }

    pub(crate) fn update_standalone_applications_window(&mut self, ctx: &Context) {
        self.process_background_results(ctx);
        self.maybe_sync_settings_from_disk(ctx);
        self.sync_desktop_appearance(ctx);
        self.sync_terminal_appearance();
        self.sync_native_display_effects();
        self.draw_applications(ctx);
        if !self.applications.open {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        ctx.request_repaint_after(Duration::from_millis(500));
    }

    pub(crate) fn prepare_standalone_installer_window(&mut self, session_username: Option<String>) {
        self.prepare_standalone_window_shell(session_username, true);
        self.prime_desktop_window_defaults(DesktopWindow::Installer);
        self.desktop_installer.open = true;
        self.desktop_active_window = Some(WindowInstanceId::primary(DesktopWindow::Installer));
    }

    pub(crate) fn update_standalone_installer_window(&mut self, ctx: &Context) {
        self.process_background_results(ctx);
        self.process_desktop_pty_input_early(ctx);
        self.maybe_sync_settings_from_disk(ctx);
        self.sync_desktop_appearance(ctx);
        self.sync_terminal_appearance();
        self.sync_native_display_effects();
        self.sync_native_cursor_mode();
        let pty_last = self.active_window_kind() == Some(DesktopWindow::PtyApp)
            && self.primary_desktop_pty_open();
        if pty_last {
            self.draw_installer(ctx);
            self.draw_desktop_pty_window(ctx);
        } else {
            self.draw_desktop_pty_window(ctx);
            self.draw_installer(ctx);
        }
        if !self.desktop_installer.open && !self.primary_desktop_pty_open() {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        ctx.request_repaint_after(Duration::from_millis(500));
    }
}
