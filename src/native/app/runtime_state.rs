use super::super::background::BackgroundResult;
use super::super::desktop_settings_service::{
    load_settings_snapshot, persist_settings_draft, reload_settings_snapshot,
};
use super::super::desktop_status_service::{
    clear_settings_status, saved_shell_status, NativeStatusUpdate, NativeStatusValue,
};
use super::super::desktop_user_service::{sorted_user_records, sorted_usernames};
use super::super::settings_standalone::standalone_settings_panel_from_arg;
use super::RobcoNativeApp;
use crate::config::{
    current_settings_file, ConnectionKind, NativeStartupWindowMode, SavedConnection, Settings,
};
use crate::core::auth::UserRecord;
use eframe::egui::{self, Context};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};

impl RobcoNativeApp {
    pub(super) fn sync_runtime_settings_cache(&mut self) {
        self.live_desktop_file_manager_settings = self.settings.draft.desktop_file_manager.clone();
        self.live_hacking_difficulty = self.settings.draft.hacking_difficulty;
    }

    pub(super) fn invalidate_desktop_surface_cache(&mut self) {
        self.desktop_surface_entries_cache = None;
        self.invalidate_desktop_icon_layout_cache();
        if self.file_manager.cwd == crate::config::desktop_dir() {
            self.file_manager.refresh_contents();
        }
    }

    pub(super) fn invalidate_program_catalog_cache(&mut self) {
        self.desktop_applications_sections_cache = None;
    }

    pub(super) fn invalidate_edit_menu_entries_cache(
        &mut self,
        target: super::super::edit_menus_screen::EditMenuTarget,
    ) {
        match target {
            super::super::edit_menus_screen::EditMenuTarget::Applications => {
                self.edit_menu_entries_cache.applications = None
            }
            super::super::edit_menus_screen::EditMenuTarget::Documents => {
                self.edit_menu_entries_cache.documents = None
            }
            super::super::edit_menus_screen::EditMenuTarget::Network => {
                self.edit_menu_entries_cache.network = None
            }
            super::super::edit_menus_screen::EditMenuTarget::Games => {
                self.edit_menu_entries_cache.games = None
            }
        }
    }

    pub(super) fn invalidate_user_cache(&mut self) {
        self.sorted_user_records_cache = None;
        self.sorted_usernames_cache = None;
    }

    pub(super) fn invalidate_saved_connections_cache(&mut self) {
        self.saved_network_connections_cache = None;
        self.saved_bluetooth_connections_cache = None;
    }

    fn current_settings_file_path() -> PathBuf {
        current_settings_file()
    }

    pub(super) fn current_settings_file_mtime() -> Option<SystemTime> {
        std::fs::metadata(Self::current_settings_file_path())
            .ok()
            .and_then(|metadata| metadata.modified().ok())
    }

    pub(super) fn refresh_settings_sync_marker(&mut self) {
        self.last_settings_file_mtime = Self::current_settings_file_mtime();
        self.last_settings_sync_check = Instant::now();
    }

    pub(super) fn replace_settings_draft(&mut self, draft: Settings) {
        self.settings.draft = draft;
        self.sync_runtime_settings_cache();
        self.invalidate_desktop_icon_layout_cache();
        self.invalidate_program_catalog_cache();
        self.invalidate_saved_connections_cache();
        self.refresh_settings_sync_marker();
    }

    pub(super) fn process_background_results(&mut self, ctx: &Context) {
        let results = self.background.poll();
        if results.is_empty() {
            return;
        }
        for result in results {
            match result {
                BackgroundResult::NukeCodesFetched(view) => {
                    self.terminal_nuke_codes = view;
                }
                BackgroundResult::SettingsPersisted => {
                    super::super::ipc::notify_settings_changed();
                }
            }
        }
        ctx.request_repaint();
    }

    pub(super) fn process_ipc_messages(&mut self, ctx: &Context) {
        let messages = self.ipc.poll();
        if messages.is_empty() {
            return;
        }
        for msg in messages {
            match msg {
                super::super::ipc::IpcMessage::SettingsChanged => {
                    let settings = reload_settings_snapshot();
                    self.replace_settings_draft(settings);
                }
                super::super::ipc::IpcMessage::OpenInEditor { path } => {
                    self.open_path_in_editor(std::path::PathBuf::from(path));
                }
                super::super::ipc::IpcMessage::RevealInFileManager { path } => {
                    self.open_file_manager_at(std::path::PathBuf::from(path));
                }
                super::super::ipc::IpcMessage::OpenSettings { panel } => {
                    let panel = panel.and_then(|p| standalone_settings_panel_from_arg(&p));
                    if let Some(panel) = panel {
                        self.open_desktop_settings_panel(panel);
                    } else {
                        self.launch_settings_via_registry();
                    }
                }
                super::super::ipc::IpcMessage::AppClosed { .. }
                | super::super::ipc::IpcMessage::Ping => {}
            }
        }
        ctx.request_repaint();
    }

    pub(super) fn maybe_sync_settings_from_disk(&mut self, ctx: &Context) {
        const SETTINGS_SYNC_INTERVAL: Duration = Duration::from_millis(500);

        if self.settings.open || self.last_settings_sync_check.elapsed() < SETTINGS_SYNC_INTERVAL {
            return;
        }
        self.last_settings_sync_check = Instant::now();

        let current_mtime = Self::current_settings_file_mtime();
        if current_mtime == self.last_settings_file_mtime {
            return;
        }

        let previous_window_mode = self.settings.draft.native_startup_window_mode;
        let settings = reload_settings_snapshot();
        self.replace_settings_draft(settings);
        if self.settings.draft.native_startup_window_mode != previous_window_mode {
            self.apply_native_window_mode(ctx);
        }
    }

    pub(super) fn saved_connections_cached(
        &mut self,
        kind: ConnectionKind,
    ) -> Arc<Vec<SavedConnection>> {
        let cache = match kind {
            ConnectionKind::Network => &mut self.saved_network_connections_cache,
            ConnectionKind::Bluetooth => &mut self.saved_bluetooth_connections_cache,
        };
        if cache.is_none() {
            *cache = Some(Arc::new(
                super::super::desktop_connections_service::saved_connections_for_kind(kind),
            ));
        }
        cache
            .as_ref()
            .expect("saved connections cache initialized")
            .clone()
    }

    pub(super) fn edit_menu_entries_cached(
        &mut self,
        target: super::super::edit_menus_screen::EditMenuTarget,
    ) -> Arc<Vec<String>> {
        let cached = match target {
            super::super::edit_menus_screen::EditMenuTarget::Applications => {
                self.edit_menu_entries_cache.applications.clone()
            }
            super::super::edit_menus_screen::EditMenuTarget::Documents => {
                self.edit_menu_entries_cache.documents.clone()
            }
            super::super::edit_menus_screen::EditMenuTarget::Network => {
                self.edit_menu_entries_cache.network.clone()
            }
            super::super::edit_menus_screen::EditMenuTarget::Games => {
                self.edit_menu_entries_cache.games.clone()
            }
        };
        if let Some(entries) = cached {
            return entries;
        }
        let entries = Arc::new(self.edit_program_entries(target));
        match target {
            super::super::edit_menus_screen::EditMenuTarget::Applications => {
                self.edit_menu_entries_cache.applications = Some(entries.clone())
            }
            super::super::edit_menus_screen::EditMenuTarget::Documents => {
                self.edit_menu_entries_cache.documents = Some(entries.clone())
            }
            super::super::edit_menus_screen::EditMenuTarget::Network => {
                self.edit_menu_entries_cache.network = Some(entries.clone())
            }
            super::super::edit_menus_screen::EditMenuTarget::Games => {
                self.edit_menu_entries_cache.games = Some(entries.clone())
            }
        }
        entries
    }

    pub(super) fn sorted_user_records_cached(&mut self) -> Arc<Vec<(String, UserRecord)>> {
        self.sorted_user_records_cache
            .get_or_insert_with(|| Arc::new(sorted_user_records()))
            .clone()
    }

    pub(super) fn sorted_usernames_cached(&mut self) -> Arc<Vec<String>> {
        self.sorted_usernames_cache
            .get_or_insert_with(|| Arc::new(sorted_usernames()))
            .clone()
    }

    pub(super) fn apply_status_update(&mut self, update: NativeStatusUpdate) {
        if let Some(shell) = update.shell {
            match shell {
                NativeStatusValue::Set(message) => self.shell_status = message,
                NativeStatusValue::Clear => self.shell_status.clear(),
            }
        }
        if let Some(settings) = update.settings {
            match settings {
                NativeStatusValue::Set(message) => self.settings.status = message,
                NativeStatusValue::Clear => self.settings.status.clear(),
            }
        }
    }

    pub(super) fn reset_desktop_settings_window(&mut self) {
        let draft = load_settings_snapshot();
        let defaults = robcos_native_settings_app::build_desktop_settings_ui_defaults(
            &draft,
            self.session
                .as_ref()
                .map(|session| session.username.as_str()),
        );
        self.replace_settings_draft(draft);
        self.apply_status_update(clear_settings_status());
        self.settings.panel = defaults.panel;
        self.settings.default_app_custom_text_code = defaults.default_app_custom_text_code;
        self.settings.default_app_custom_ebook = defaults.default_app_custom_ebook;
        self.settings.scanned_networks.clear();
        self.settings.scanned_bluetooth.clear();
        self.settings.connection_password.clear();
        self.settings.edit_target = super::super::edit_menus_screen::EditMenuTarget::Applications;
        self.settings.edit_name_input.clear();
        self.settings.edit_value_input.clear();
        self.settings.cli_profile_slot = defaults.cli_profile_slot;
        self.settings.user_create_username.clear();
        self.settings.user_create_auth = defaults.user_create_auth;
        self.settings.user_create_password.clear();
        self.settings.user_create_password_confirm.clear();
        self.settings.user_edit_password.clear();
        self.settings.user_edit_password_confirm.clear();
        self.settings.user_delete_confirm.clear();
        self.settings.user_selected = defaults.user_selected;
        self.settings.user_selected_loaded_for = defaults.user_selected_loaded_for;
        self.settings.user_edit_auth = defaults.user_edit_auth;
    }

    pub(super) fn persist_native_settings(&mut self) {
        {
            let draft = self.settings.draft.clone();
            let tx = self.background.sender();
            std::thread::spawn(move || {
                persist_settings_draft(&draft);
                let _ = tx.send(BackgroundResult::SettingsPersisted);
            });
        }
        self.sync_runtime_settings_cache();
        self.invalidate_desktop_icon_layout_cache();
        self.invalidate_program_catalog_cache();
        self.invalidate_saved_connections_cache();
        self.refresh_settings_sync_marker();
        self.apply_status_update(saved_shell_status());
    }

    pub(super) fn apply_native_window_mode(&self, ctx: &Context) {
        match self.settings.draft.native_startup_window_mode {
            NativeStartupWindowMode::Windowed => {
                ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(false));
                ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(false));
                ctx.send_viewport_cmd(egui::ViewportCommand::Decorations(true));
            }
            NativeStartupWindowMode::Maximized => {
                ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(false));
                ctx.send_viewport_cmd(egui::ViewportCommand::Decorations(true));
                ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(true));
            }
            NativeStartupWindowMode::BorderlessFullscreen => {
                ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(false));
                ctx.send_viewport_cmd(egui::ViewportCommand::Decorations(false));
                ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(true));
            }
            NativeStartupWindowMode::Fullscreen => {
                ctx.send_viewport_cmd(egui::ViewportCommand::Decorations(false));
                ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(false));
                ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(true));
            }
        }
    }
}
