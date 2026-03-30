#[cfg(test)]
use super::super::desktop_session_service::active_session_identity;
use super::super::desktop_session_service::{
    active_session_index as active_native_session_index, apply_session_switch,
    close_active_session as close_native_session,
    ensure_login_session_entry as ensure_native_login_session_entry,
    request_session_switch as request_native_session_switch, session_count as native_session_count,
    take_pending_session_switch as take_native_pending_session_switch,
    user_record as session_user_record, NativePendingSessionSwitch,
};
use super::NucleonNativeApp;
use super::{ParkedSessionState, SessionState, SESSION_LEADER_WINDOW};
use eframe::egui::{self, Context, Key, Modifiers};
use std::collections::HashMap;
use std::time::Instant;

impl NucleonNativeApp {
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
            tweaks_open: self.tweaks_open,
            addons_open: self.addons_open,
            addons_sidebar_category: self.addons_sidebar_category,
            addons_addon_subcategory: self.addons_addon_subcategory,
            addons_theme_subcategory: self.addons_theme_subcategory,
            applications: self.applications.clone(),
            addons_repo_cache: None,
            addons_repo_fetch_in_progress: false,
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
            terminal_pty: self.terminal_pty.take(),
            terminal_pty_surface: self.terminal_pty_surface.take(),
            terminal_wasm_addon: self.terminal_wasm_addon.take(),
            terminal_wasm_addon_return_screen: self.terminal_wasm_addon_return_screen.take(),
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
            desktop_wasm_addon: self.desktop_wasm_addon.take(),
            tweaks_tab: self.tweaks_tab,
            tweaks_wallpaper_surface: self.tweaks_wallpaper_surface,
            tweaks_theme_surface: self.tweaks_theme_surface,
            tweaks_layout_overrides_open: self.tweaks_layout_overrides_open,
            tweaks_customize_colors_open: self.tweaks_customize_colors_open,
            tweaks_editing_color_token: self.tweaks_editing_color_token,
            terminal_tweaks_active_section: self.terminal_tweaks_active_section,
            terminal_tweaks_open_dropdown: self.terminal_tweaks_open_dropdown,
            desktop_color_overrides: self.desktop_color_overrides.clone(),
            terminal_color_overrides: self.terminal_color_overrides.clone(),
            desktop_active_desktop_style_id: self.desktop_active_desktop_style_id.clone(),
            desktop_active_icon_pack_id: self.desktop_active_icon_pack_id.clone(),
            desktop_active_sound_pack_id: self.desktop_active_sound_pack_id.clone(),
            desktop_active_cursor_pack_id: self.desktop_active_cursor_pack_id.clone(),
            desktop_active_font_id: self.desktop_active_font_id.clone(),
            desktop_active_desktop_style: self.desktop_active_desktop_style.clone(),
            terminal_active_theme: self.terminal_active_theme.clone(),
            terminal_theme_options: self.terminal_theme_options.clone(),
            terminal_active_font_id: self.terminal_active_font_id.clone(),
            dashboard_nav_index: self.dashboard_nav_index,
            dashboard_nav_focused: self.dashboard_nav_focused,
            dashboard_recent_files: self.dashboard_recent_files.clone(),
            terminal_decoration: self.terminal_decoration.clone(),
            picking_terminal_wallpaper: self.picking_terminal_wallpaper,
            picking_theme_import: self.picking_theme_import,
            active_sound_pack_path: self.active_sound_pack_path.clone(),
            active_asset_pack_path: self.active_asset_pack_path.clone(),
            active_cursor_pack: self.active_cursor_pack.clone(),
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
        self.tweaks_open = parked.tweaks_open;
        self.addons_open = parked.addons_open;
        self.addons_sidebar_category = parked.addons_sidebar_category;
        self.addons_addon_subcategory = parked.addons_addon_subcategory;
        self.addons_theme_subcategory = parked.addons_theme_subcategory;
        self.applications = parked.applications;
        self.addons_repo_cache = parked.addons_repo_cache;
        self.addons_repo_fetch_in_progress = parked.addons_repo_fetch_in_progress;
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
        self.terminal_pty = parked.terminal_pty;
        self.terminal_pty_surface = parked.terminal_pty_surface;
        self.terminal_wasm_addon = parked.terminal_wasm_addon;
        self.terminal_wasm_addon_return_screen = parked.terminal_wasm_addon_return_screen;
        self.terminal_wasm_addon_last_frame_at = None;
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
        self.desktop_wasm_addon = parked.desktop_wasm_addon;
        self.desktop_wasm_addon_last_frame_at = None;
        self.tweaks_tab = parked.tweaks_tab;
        self.tweaks_wallpaper_surface = parked.tweaks_wallpaper_surface;
        self.tweaks_theme_surface = parked.tweaks_theme_surface;
        self.tweaks_layout_overrides_open = parked.tweaks_layout_overrides_open;
        self.tweaks_customize_colors_open = parked.tweaks_customize_colors_open;
        self.tweaks_editing_color_token = parked.tweaks_editing_color_token;
        self.terminal_tweaks_active_section = parked.terminal_tweaks_active_section;
        self.terminal_tweaks_open_dropdown = parked.terminal_tweaks_open_dropdown;
        self.desktop_color_overrides = parked.desktop_color_overrides;
        self.terminal_color_overrides = parked.terminal_color_overrides;
        self.desktop_active_desktop_style_id = parked.desktop_active_desktop_style_id;
        self.desktop_active_icon_pack_id = parked.desktop_active_icon_pack_id;
        self.desktop_active_sound_pack_id = parked.desktop_active_sound_pack_id;
        self.desktop_active_cursor_pack_id = parked.desktop_active_cursor_pack_id;
        self.desktop_active_font_id = parked.desktop_active_font_id;
        self.desktop_active_desktop_style = parked.desktop_active_desktop_style;
        self.terminal_active_theme = parked.terminal_active_theme;
        self.terminal_theme_options = parked.terminal_theme_options;
        self.terminal_active_font_id = parked.terminal_active_font_id;
        self.dashboard_nav_index = parked.dashboard_nav_index;
        self.dashboard_nav_focused = parked.dashboard_nav_focused;
        self.dashboard_recent_files = parked.dashboard_recent_files;
        self.terminal_decoration = parked.terminal_decoration;
        self.picking_terminal_wallpaper = parked.picking_terminal_wallpaper;
        self.picking_theme_import = parked.picking_theme_import;
        self.active_sound_pack_path = parked.active_sound_pack_path;
        self.active_asset_pack_path = parked.active_asset_pack_path;
        self.active_cursor_pack = parked.active_cursor_pack;
        crate::sound::set_active_sound_pack(self.active_sound_pack_path.clone());
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
        if let Some(mut pty) = self.take_primary_pty() {
            pty.session.terminate();
        }
        self.clear_terminal_wasm_addon();
        self.clear_desktop_wasm_addon();
        Self::terminate_secondary_window_ptys(&mut self.secondary_windows);
        for parked in self.session_runtime.values_mut() {
            if let Some(mut pty) = parked.terminal_pty.take() {
                pty.session.terminate();
            }
            parked.terminal_pty_surface = None;
            parked.terminal_wasm_addon = None;
            parked.terminal_wasm_addon_return_screen = None;
            Self::terminate_secondary_window_ptys(&mut parked.secondary_windows);
            parked.desktop_wasm_addon = None;
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

        if let Some(mut pty) = self.take_primary_pty() {
            pty.session.terminate();
        }
        Self::terminate_secondary_window_ptys(&mut self.secondary_windows);
        if let Some(mut parked) = self.session_runtime.remove(&closing_idx) {
            if let Some(mut pty) = parked.terminal_pty.take() {
                pty.session.terminate();
            }
            parked.terminal_pty_surface = None;
            Self::terminate_secondary_window_ptys(&mut parked.secondary_windows);
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
