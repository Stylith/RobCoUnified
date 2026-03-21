use super::super::desktop_session_service::{
    active_session_index as active_native_session_index, apply_session_switch,
    close_active_session as close_native_session,
    ensure_login_session_entry as ensure_native_login_session_entry,
    request_session_switch as request_native_session_switch,
    session_count as native_session_count,
    take_pending_session_switch as take_native_pending_session_switch,
    user_record as session_user_record, NativePendingSessionSwitch,
};
#[cfg(test)]
use super::super::desktop_session_service::active_session_identity;
use super::RobcoNativeApp;
use super::{ParkedSessionState, SessionState, SESSION_LEADER_WINDOW};
use eframe::egui::{self, Context, Key, Modifiers};
use std::collections::HashMap;
use std::time::Instant;

impl RobcoNativeApp {
    pub(super) fn session_idx_from_digit_key(key: Key) -> Option<usize> {
        match key {
            Key::Num1 => Some(0),
            Key::Num2 => Some(1),
            Key::Num3 => Some(2),
            Key::Num4 => Some(3),
            Key::Num5 => Some(4),
            Key::Num6 => Some(5),
            Key::Num7 => Some(6),
            Key::Num8 => Some(7),
            Key::Num9 => Some(8),
            _ => None,
        }
    }

    pub(super) fn request_session_switch_if_valid(&mut self, target: usize) -> bool {
        request_native_session_switch(target)
    }

    pub(super) fn ensure_login_session_entry(&mut self, username: &str) {
        ensure_native_login_session_entry(username);
    }

    pub(super) fn park_active_session_runtime(&mut self) {
        if self.session.is_none() || native_session_count() == 0 {
            return;
        }
        let Some(idx) = active_native_session_index() else {
            return;
        };
        let parked = ParkedSessionState {
            file_manager: self.file_manager.clone(),
            editor: self.editor.clone(),
            settings: self.settings.clone(),
            applications: self.applications.clone(),
            donkey_kong_window: self.donkey_kong_window.clone(),
            donkey_kong: self.donkey_kong.clone(),
            desktop_nuke_codes_open: self.desktop_nuke_codes_open,
            desktop_installer: std::mem::take(&mut self.desktop_installer),
            terminal_mode: self.terminal_mode.clone(),
            desktop_window_states: self.desktop_window_states.clone(),
            desktop_active_window: self.desktop_active_window,
            desktop_mode_open: self.desktop_mode_open,
            start_root_panel_height: self.start_root_panel_height,
            start_open: self.start_open,
            start_selected_root: self.start_selected_root,
            start_system_selected: self.start_system_selected,
            start_leaf_selected: self.start_leaf_selected,
            start_open_submenu: self.start_open_submenu,
            start_open_leaf: self.start_open_leaf,
            terminal_nav: self.current_terminal_navigation_state(),
            terminal_settings_panel: self.terminal_settings_panel,
            terminal_nuke_codes: self.terminal_nuke_codes.clone(),
            terminal_pty: self.terminal_pty.take(),
            terminal_installer: std::mem::take(&mut self.terminal_installer),
            terminal_edit_menus: std::mem::take(&mut self.terminal_edit_menus),
            terminal_connections: std::mem::take(&mut self.terminal_connections),
            terminal_prompt: self.terminal_prompt.take(),
            terminal_flash: self.terminal_flash.take(),
            session_leader_until: self.session_leader_until.take(),
            desktop_window_generation_seed: self.desktop_window_generation_seed,
            file_manager_runtime: self.file_manager_runtime.clone(),
            shell_status: std::mem::take(&mut self.shell_status),
            start_menu_rename: self.start_menu_rename.take(),
            secondary_windows: std::mem::take(&mut self.secondary_windows),
        };
        self.session_runtime.insert(idx, parked);
    }

    #[cfg(test)]
    pub(super) fn sync_active_session_identity(&mut self) -> bool {
        match active_session_identity() {
            Ok(Some(identity)) => {
                self.session = Some(SessionState {
                    username: identity.username,
                    is_admin: identity.is_admin,
                });
                true
            }
            Ok(None) => {
                self.session = None;
                false
            }
            Err(status) => {
                self.session = None;
                self.shell_status = status;
                false
            }
        }
    }

    pub(super) fn restore_active_session_runtime_if_any(&mut self) -> bool {
        let Some(idx) = active_native_session_index() else {
            return false;
        };
        let Some(parked) = self.session_runtime.remove(&idx) else {
            return false;
        };
        self.file_manager = parked.file_manager;
        self.editor = parked.editor;
        self.settings = parked.settings;
        self.applications = parked.applications;
        self.donkey_kong_window = parked.donkey_kong_window;
        self.donkey_kong = parked.donkey_kong;
        self.desktop_nuke_codes_open = parked.desktop_nuke_codes_open;
        self.desktop_installer = parked.desktop_installer;
        self.terminal_mode = parked.terminal_mode;
        self.desktop_window_states = parked.desktop_window_states;
        self.desktop_active_window = parked.desktop_active_window;
        self.desktop_mode_open = parked.desktop_mode_open;
        self.start_root_panel_height = parked.start_root_panel_height;
        self.start_open = parked.start_open;
        self.start_selected_root = parked.start_selected_root;
        self.start_system_selected = parked.start_system_selected;
        self.start_leaf_selected = parked.start_leaf_selected;
        self.start_open_submenu = parked.start_open_submenu;
        self.start_open_leaf = parked.start_open_leaf;
        self.apply_terminal_navigation_state(parked.terminal_nav);
        self.terminal_settings_panel = parked.terminal_settings_panel;
        self.terminal_nuke_codes = parked.terminal_nuke_codes;
        self.terminal_pty = parked.terminal_pty;
        self.terminal_installer = parked.terminal_installer;
        self.terminal_edit_menus = parked.terminal_edit_menus;
        self.terminal_connections = parked.terminal_connections;
        self.terminal_prompt = parked.terminal_prompt;
        self.terminal_flash = parked.terminal_flash;
        self.session_leader_until = parked.session_leader_until;
        self.desktop_window_generation_seed = parked.desktop_window_generation_seed;
        self.file_manager_runtime = parked.file_manager_runtime;
        self.context_menu_action = None;
        self.shell_status = parked.shell_status;
        self.start_menu_rename = parked.start_menu_rename;
        self.secondary_windows = parked.secondary_windows;
        true
    }

    pub(super) fn apply_pending_session_switch(&mut self) {
        let Some(plan) = take_native_pending_session_switch() else {
            return;
        };

        if matches!(plan, NativePendingSessionSwitch::AlreadyActive) {
            return;
        }

        self.persist_snapshot();
        self.park_active_session_runtime();

        let new_session_status = match &plan {
            NativePendingSessionSwitch::OpenNew { new_index, .. } => {
                Some(format!("Switched to session {}.", new_index + 1))
            }
            _ => None,
        };

        match apply_session_switch(&plan) {
            Ok(Some(identity)) => {
                self.session = Some(SessionState {
                    username: identity.username.clone(),
                    is_admin: identity.is_admin,
                });
                if !self.restore_active_session_runtime_if_any() {
                    if let Some(user) = session_user_record(&identity.username) {
                        self.restore_for_user(&identity.username, &user);
                    } else {
                        self.shell_status = format!("Unknown user '{}'.", identity.username);
                        return;
                    }
                }
                if let Some(status) = new_session_status {
                    self.shell_status = status;
                }
            }
            Ok(None) => {}
            Err(status) => {
                self.shell_status = status;
            }
        }
    }

    pub(super) fn terminate_all_native_pty_children(&mut self) {
        if let Some(mut pty) = self.terminal_pty.take() {
            pty.session.terminate();
        }
        for parked in self.session_runtime.values_mut() {
            if let Some(mut pty) = parked.terminal_pty.take() {
                pty.session.terminate();
            }
        }
    }

    pub(super) fn close_active_session_window(&mut self) {
        self.persist_snapshot();
        let Some(closing_idx) = active_native_session_index() else {
            return;
        };

        let outcome = match close_native_session() {
            Ok(Some(outcome)) => outcome,
            Ok(None) => return,
            Err(status) => {
                self.shell_status = status;
                return;
            }
        };

        if let Some(mut pty) = self.terminal_pty.take() {
            pty.session.terminate();
        }
        if let Some(mut parked) = self.session_runtime.remove(&closing_idx) {
            if let Some(mut pty) = parked.terminal_pty.take() {
                pty.session.terminate();
            }
        }

        // Session indexes are contiguous; shift parked state keys down after removal.
        let mut remapped = HashMap::new();
        for (idx, parked) in self.session_runtime.drain() {
            let new_idx = if idx > outcome.removed_idx {
                idx - 1
            } else {
                idx
            };
            remapped.insert(new_idx, parked);
        }
        self.session_runtime = remapped;

        if let Some(identity) = outcome.active_identity {
            self.session = Some(SessionState {
                username: identity.username.clone(),
                is_admin: identity.is_admin,
            });
            if !self.restore_active_session_runtime_if_any() {
                if let Some(user) = session_user_record(&identity.username) {
                    self.restore_for_user(&identity.username, &user);
                } else {
                    self.shell_status = format!("Unknown user '{}'.", identity.username);
                    return;
                }
            }
        } else {
            self.session = None;
        }
        self.shell_status = format!("Closed session {}.", outcome.removed_idx + 1);
    }

    pub(super) fn capture_session_switch_shortcuts(&mut self, ctx: &Context) {
        if self.session.is_none() {
            self.session_leader_until = None;
            return;
        }

        if self
            .session_leader_until
            .is_some_and(|deadline| Instant::now() > deadline)
        {
            self.session_leader_until = None;
        }

        let events = ctx.input(|i| i.events.clone());
        let mut consumed: Vec<(Modifiers, Key)> = Vec::new();
        let mut switch_target: Option<usize> = None;
        let mut close_active = false;
        let now = Instant::now();

        for event in events {
            let egui::Event::Key {
                key,
                pressed: true,
                modifiers,
                ..
            } = event
            else {
                continue;
            };

            if modifiers.ctrl && key == Key::Q {
                self.session_leader_until = Some(now + SESSION_LEADER_WINDOW);
                consumed.push((modifiers, key));
                continue;
            }

            if self.session_leader_until.is_some() {
                // Native session switching is intentionally strict:
                // only Ctrl+Q followed by plain 1..9 (switch) or W/X (close).
                let plain_follow = !modifiers.ctrl && !modifiers.alt && !modifiers.command;
                if plain_follow {
                    if let Some(idx) = Self::session_idx_from_digit_key(key) {
                        switch_target = Some(idx);
                        consumed.push((modifiers, key));
                        self.session_leader_until = None;
                        break;
                    }
                    if matches!(key, Key::W | Key::X) {
                        close_active = true;
                        consumed.push((modifiers, key));
                        self.session_leader_until = None;
                        break;
                    }
                }
                self.session_leader_until = None;
                continue;
            }
        }

        if !consumed.is_empty() {
            ctx.input_mut(|i| {
                for (mods, key) in &consumed {
                    i.consume_key(*mods, *key);
                }
            });
        }

        if close_active {
            self.close_active_session_window();
            return;
        }

        if let Some(target) = switch_target {
            self.request_session_switch_if_valid(target);
        }
    }
}
