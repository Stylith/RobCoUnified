use super::super::desktop_session_service::{
    authenticate_login, has_pending_session_switch as has_native_pending_session_switch,
    login_selection_auth_method, user_record as session_user_record,
};
use super::super::hacking_screen::{draw_hacking_screen, draw_locked_screen, HackingScreenEvent};
use super::super::menu::{
    login_menu_rows_from_users, resolve_hacking_screen_event, resolve_login_selection_plan,
    resolve_terminal_flash_action, TerminalHackingUiEvent, TerminalLoginScreenMode, TerminalScreen,
};
use super::super::prompt::{draw_terminal_flash, draw_terminal_flash_boxed, FlashAction};
use super::super::shell_slots::ShellSlot;
use super::super::terminal_slots::TerminalSlot;
use super::NucleonNativeApp;
use eframe::egui::{self, Align2, Color32, Context, Id, Key, RichText};
use std::io::Write;
use std::time::SystemTime;
use std::time::{Duration, Instant};

use super::software_cursor::draw_software_cursor;

const NUCLEON_STARTUP_PROFILE_LOG_ENV: &str = "NUCLEON_STARTUP_PROFILE_LOG";
const NUCLEON_REPAINT_TRACE_LOG_ENV: &str = "NUCLEON_REPAINT_TRACE_LOG";

fn first_var_os(names: &[&str]) -> Option<std::ffi::OsString> {
    names.iter().find_map(std::env::var_os)
}

impl NucleonNativeApp {
    fn append_startup_profile_marker(marker: &str) {
        let Some(path) = first_var_os(&[NUCLEON_STARTUP_PROFILE_LOG_ENV]) else {
            return;
        };
        let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
        else {
            return;
        };
        let timestamp_ms = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|duration| duration.as_millis())
            .unwrap_or(0);
        let _ = writeln!(file, "{timestamp_ms} {marker}");
    }

    pub(super) fn maybe_write_startup_profile_markers(&mut self) {
        if !self.startup_profile_session_logged && self.session.is_some() {
            Self::append_startup_profile_marker("session_ready");
            self.startup_profile_session_logged = true;
        }
        if !self.startup_profile_desktop_logged && self.session.is_some() && self.desktop_mode_open
        {
            Self::append_startup_profile_marker("desktop_ready");
            self.startup_profile_desktop_logged = true;
        }
    }

    pub(super) fn maybe_trace_repaint_causes(&mut self, ctx: &Context) {
        let Some(path) = first_var_os(&[NUCLEON_REPAINT_TRACE_LOG_ENV]) else {
            return;
        };
        let pass = ctx.cumulative_pass_nr();
        if pass == 0 || pass == self.repaint_trace_last_pass {
            return;
        }
        self.repaint_trace_last_pass = pass;
        let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
        else {
            return;
        };
        let timestamp_ms = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|duration| duration.as_millis())
            .unwrap_or(0);
        let causes = ctx.repaint_causes();
        let cause_text = if causes.is_empty() {
            "none".to_string()
        } else {
            causes
                .into_iter()
                .map(|cause| cause.to_string())
                .collect::<Vec<_>>()
                .join(" | ")
        };
        let requested = ctx.has_requested_repaint();
        let input_summary = ctx.input(|input| {
            format!(
                "events={} pointer_delta=({:.2},{:.2}) motion={:?} latest_pos={:?}",
                input.events.len(),
                input.pointer.delta().x,
                input.pointer.delta().y,
                input.pointer.motion(),
                input.pointer.latest_pos(),
            )
        });
        let mode = if self.desktop_mode_open {
            "desktop"
        } else {
            "terminal"
        };
        let _ = writeln!(
            file,
            "{timestamp_ms} pass={pass} mode={mode} requested={requested} causes={cause_text} input={input_summary}"
        );
    }

    pub(super) fn process_desktop_pty_input_early(&mut self, ctx: &Context) {
        let active_id = self.desktop_active_window.filter(|id| {
            self.desktop_mode_open && id.kind == super::super::desktop_app::DesktopWindow::PtyApp
        });
        let mut early_pty_close = None;
        let mut consumed_pty_input = false;
        if let Some(active_id) = active_id {
            let handled_tile_shortcut = self.handle_desktop_window_tiling_shortcuts(ctx);
            if let Some(state) = self
                .desktop_pty_slot_mut(active_id)
                .and_then(|slot| slot.as_mut())
            {
                if !handled_tile_shortcut {
                    if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(Key::Q)) {
                        early_pty_close = Some(active_id);
                    }
                    if ctx.input(|i| i.modifiers.ctrl && i.modifiers.shift && i.key_pressed(Key::P))
                    {
                        state.show_perf_overlay = !state.show_perf_overlay;
                    }
                    super::super::pty_screen::handle_pty_input(ctx, &mut state.session);
                }
                consumed_pty_input = true;
            }
        }
        if consumed_pty_input {
            ctx.input_mut(|i| {
                i.events.retain(|e| {
                    !matches!(
                        e,
                        egui::Event::Key { .. } | egui::Event::Text(_) | egui::Event::Paste(_)
                    )
                });
            });
        }
        if let Some(id) = early_pty_close {
            self.request_close_window_instance(id);
        }
    }

    fn draw_login(&mut self, ctx: &Context) {
        let layout = self.terminal_layout();
        let header_lines = self.active_terminal_header_lines().to_vec();
        match self.login.mode {
            TerminalLoginScreenMode::SelectUser => {
                let rows = login_menu_rows_from_users(self.login_usernames());
                if self.terminal_prompt.is_some() {
                    self.handle_terminal_prompt_input(ctx);
                }
                let activated = super::super::shell_screen::draw_login_screen(
                    ctx,
                    &rows,
                    &mut self.login.selected_idx,
                    &self.login.error,
                    self.terminal_prompt.as_ref(),
                    layout.cols,
                    layout.rows,
                    layout.header_start_row,
                    layout.separator_top_row,
                    layout.title_row,
                    layout.separator_bottom_row,
                    layout.subtitle_row,
                    layout.menu_start_row,
                    layout.status_row,
                    layout.content_col,
                    &header_lines,
                );
                if activated {
                    let usernames = self.login_usernames();
                    let plan = resolve_login_selection_plan(
                        self.login.selected_idx,
                        &usernames,
                        login_selection_auth_method,
                        |username| authenticate_login(username, ""),
                    );
                    self.apply_terminal_login_selection_plan(plan);
                }
            }
            TerminalLoginScreenMode::Hacking => {
                let (username, event) = match self.login.hacking.as_mut() {
                    Some(hacking) => (
                        hacking.username.clone(),
                        draw_hacking_screen(
                            ctx,
                            &mut hacking.game,
                            layout.cols,
                            layout.rows,
                            layout.status_row,
                            layout.status_row_alt,
                        ),
                    ),
                    None => {
                        crate::sound::play_navigate();
                        self.login.mode = TerminalLoginScreenMode::SelectUser;
                        return;
                    }
                };
                match event {
                    HackingScreenEvent::None => {}
                    HackingScreenEvent::Cancel => {
                        self.apply_terminal_hacking_plan(resolve_hacking_screen_event(
                            &username,
                            TerminalHackingUiEvent::Cancel,
                            session_user_record,
                        ))
                    }
                    HackingScreenEvent::Success => {
                        self.apply_terminal_hacking_plan(resolve_hacking_screen_event(
                            &username,
                            TerminalHackingUiEvent::Success,
                            session_user_record,
                        ))
                    }
                    HackingScreenEvent::LockedOut => {
                        self.apply_terminal_hacking_plan(resolve_hacking_screen_event(
                            &username,
                            TerminalHackingUiEvent::LockedOut,
                            session_user_record,
                        ))
                    }
                    HackingScreenEvent::ExitLocked => {}
                }
            }
            TerminalLoginScreenMode::Locked => {
                if matches!(
                    draw_locked_screen(ctx, layout.cols, layout.rows, layout.status_row_alt),
                    HackingScreenEvent::ExitLocked
                ) {
                    self.login.show_user_selection();
                }
            }
        }
    }

    pub(super) fn draw_terminal_runtime(&mut self, ctx: &Context) {
        if self.terminal_nav.suppress_next_menu_submit {
            ctx.input_mut(|i| {
                i.consume_key(egui::Modifiers::NONE, Key::Enter);
                i.consume_key(egui::Modifiers::NONE, Key::Space);
            });
            self.terminal_nav.suppress_next_menu_submit = false;
        }

        let terminal_layout = self.terminal_active_layout.clone();
        let registry = std::mem::replace(
            &mut self.terminal_slot_registry,
            super::super::terminal_slots::TerminalSlotRegistry::classic(),
        );
        registry.render_slot(TerminalSlot::StatusBar, self, ctx, &terminal_layout);
        registry.render_slot(TerminalSlot::Screen, self, ctx, &terminal_layout);
        registry.render_slot(TerminalSlot::Overlay, self, ctx, &terminal_layout);
        self.terminal_slot_registry = registry;

        self.draw_file_manager(ctx);
        self.draw_editor(ctx);
        self.draw_settings(ctx);
        self.draw_applications(ctx);
        self.draw_terminal_mode(ctx);
    }

    pub(super) fn update_native_shell_frame(&mut self, ctx: &Context) {
        self.release_retained_wasm_addons();
        self.process_background_results(ctx);
        self.process_ipc_messages(ctx);

        self.process_desktop_pty_input_early(ctx);
        self.maybe_sync_settings_from_disk(ctx);
        self.sync_desktop_appearance(ctx);
        self.sync_terminal_appearance(ctx);
        self.sync_native_display_effects();
        self.sync_native_cursor_mode();

        if let Some(flash) = &self.terminal_flash {
            if Instant::now() >= flash.until {
                let action = flash.action.clone();
                self.terminal_flash = None;
                match action {
                    FlashAction::Noop => {}
                    FlashAction::ExitApp => {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                    FlashAction::FinishLogout => self.finish_logout(),
                    FlashAction::FinishLogin { username, user } => {
                        self.ensure_login_session_entry(&username);
                        self.restore_for_user(&username, &user);
                    }
                    _ => {
                        if let Some(plan) =
                            resolve_terminal_flash_action(&action, self.live_hacking_difficulty)
                        {
                            self.apply_terminal_flash_action_plan(plan);
                        }
                    }
                }
            } else {
                ctx.request_repaint_after(flash.until.saturating_duration_since(Instant::now()));
                let layout = self.terminal_layout();
                let header_lines = self.active_terminal_header_lines().to_vec();
                self.draw_terminal_status_bar(
                    ctx,
                    self.terminal_active_layout.status_bar_position,
                    self.terminal_active_layout.status_bar_height,
                );
                let show_hacking_wait = self.session.is_none()
                    && matches!(self.login.mode, TerminalLoginScreenMode::Hacking)
                    && matches!(&flash.action, FlashAction::FinishLogin { .. });
                if show_hacking_wait {
                    self.draw_login(ctx);
                    return;
                }
                if flash.boxed {
                    draw_terminal_flash_boxed(
                        ctx,
                        &flash.message,
                        layout.cols,
                        layout.rows,
                        layout.header_start_row,
                        layout.separator_top_row,
                        layout.separator_bottom_row,
                        &header_lines,
                    );
                } else {
                    draw_terminal_flash(
                        ctx,
                        &flash.message,
                        layout.cols,
                        layout.rows,
                        layout.header_start_row,
                        layout.separator_top_row,
                        layout.separator_bottom_row,
                        layout.status_row,
                        layout.content_col,
                        &header_lines,
                    );
                }
                return;
            }
        }

        self.maybe_write_startup_profile_markers();
        self.maybe_trace_repaint_causes(ctx);

        if self.session.is_none() {
            self.draw_terminal_status_bar(
                ctx,
                self.terminal_active_layout.status_bar_position,
                self.terminal_active_layout.status_bar_height,
            );
            self.draw_login(ctx);
            return;
        }

        if !self.desktop_mode_open {
            self.capture_session_switch_shortcuts(ctx);
            if has_native_pending_session_switch() {
                self.apply_pending_session_switch();
            }
        }

        self.dispatch_context_menu_action(ctx);

        if !self.desktop_mode_open
            && !matches!(self.terminal_nav.screen, TerminalScreen::PtyApp)
            && !self.editor.open
        {
            let is_browser = matches!(self.terminal_nav.screen, TerminalScreen::DocumentBrowser);
            let back = if is_browser {
                ctx.input(|i| i.key_pressed(Key::Escape))
            } else {
                ctx.input(|i| i.key_pressed(Key::Escape) || i.key_pressed(Key::Tab))
            };
            if back {
                self.handle_terminal_back();
            }
        }

        if self.terminal_prompt.is_some() {
            self.handle_terminal_prompt_input(ctx);
            self.consume_terminal_prompt_keys(ctx);
        }

        if self.desktop_mode_open {
            if self.icon_cache_dirty {
                self.asset_cache = Some(self.build_asset_cache(ctx));
                self.icon_cache_dirty = false;
            }
            if ctx.input(|i| i.key_pressed(Key::Escape)) {
                let had_overlay = self.start_open || self.spotlight_open;
                if had_overlay {
                    self.close_desktop_overlays();
                }
            }
            self.handle_desktop_window_tiling_shortcuts(ctx);
            self.handle_start_menu_keyboard(ctx);
            self.handle_desktop_file_manager_shortcuts(ctx);
            let layout = self.desktop_active_layout.clone();
            let registry = std::mem::replace(
                &mut self.slot_registry,
                super::super::shell_slots::SlotRegistry::classic(),
            );
            registry.render_slot(ShellSlot::Panel, self, ctx, &layout);
            registry.render_slot(ShellSlot::Dock, self, ctx, &layout);
            registry.render_slot(ShellSlot::Desktop, self, ctx, &layout);
            self.slot_registry = registry;
        } else {
            self.draw_terminal_runtime(ctx);
        }
        if self.desktop_mode_open {
            self.draw_desktop_windows(ctx);
            let layout = self.desktop_active_layout.clone();
            let registry = std::mem::replace(
                &mut self.slot_registry,
                super::super::shell_slots::SlotRegistry::classic(),
            );
            registry.render_slot(ShellSlot::Launcher, self, ctx, &layout);
            self.slot_registry = registry;
            self.draw_start_menu_rename_window(ctx);
            let layout = self.desktop_active_layout.clone();
            let registry = std::mem::replace(
                &mut self.slot_registry,
                super::super::shell_slots::SlotRegistry::classic(),
            );
            registry.render_slot(ShellSlot::Spotlight, self, ctx, &layout);
            self.slot_registry = registry;
        }
        self.draw_shortcut_properties_window(ctx);
        self.draw_desktop_item_properties_window(ctx);
        self.draw_editor_save_as_window(ctx);
        if self.desktop_mode_open {
            self.draw_terminal_prompt_overlay_global(ctx);
        }

        self.maybe_intercept_viewport_close_for_unsaved_editor(ctx);

        if self.session.is_some() && self.editor.open && self.editor.dirty {
            egui::Area::new(Id::new("native_unsaved_badge"))
                .anchor(Align2::RIGHT_BOTTOM, [-16.0, -16.0])
                .show(ctx, |ui| {
                    ui.label(RichText::new("Unsaved changes").color(Color32::LIGHT_RED));
                });
        }

        if self.desktop_mode_open && self.settings.draft.desktop_show_cursor {
            draw_software_cursor(
                ctx,
                self.settings.draft.desktop_cursor_scale,
                self.active_cursor_pack.as_ref(),
            );
        }

        let idle_repaint = if self.settings.draft.display_effects.needs_animation() {
            Duration::from_millis(33)
        } else {
            Duration::from_millis(500)
        };
        ctx.request_repaint_after(idle_repaint);
    }
}
