use super::super::desktop_app::DesktopWindow;
use super::super::menu::{resolve_desktop_pty_exit, TerminalDesktopPtyExitPlan};
use super::super::pty_screen::{draw_embedded_pty_in_ui_focused, PtyScreenEvent};
use super::super::retro_ui::{ShellSurfaceKind, FIXED_PTY_CELL_H, FIXED_PTY_CELL_W};
use super::super::wasm_addon_runtime::{collect_hosted_keyboard_input, draw_hosted_addon_frame};
use super::desktop_window_mgmt::{DesktopHeaderAction, DesktopWindowRectTracking, ResizableDesktopWindowOptions};
use super::RobcoNativeApp;
use crate::platform::HostedAddonSize;
use eframe::egui::{self, Context, Layout};

impl RobcoNativeApp {
    pub(super) fn draw_desktop_pty_window(&mut self, ctx: &Context) {
        let wid = self.current_window_id(DesktopWindow::PtyApp);
        if wid.instance == 0 && self.desktop_wasm_addon.is_some() {
            self.draw_desktop_wasm_addon_window(ctx);
            return;
        }
        if self.desktop_window_is_minimized(DesktopWindow::PtyApp) {
            return;
        }
        if !self.primary_desktop_pty_open() {
            self.update_desktop_window_state(DesktopWindow::PtyApp, false);
            return;
        }
        let default_size = Self::desktop_default_window_size(DesktopWindow::PtyApp);
        let default_pos = self.active_desktop_default_window_pos(ctx, default_size);
        let pty_focused = self.desktop_active_window == Some(wid);
        let Some(pty_state) = self.terminal_pty.as_ref() else {
            self.update_desktop_window_state(DesktopWindow::PtyApp, false);
            return;
        };
        let title = pty_state.title.clone();
        let window_title = if wid.instance > 0 {
            format!("{} [{}]", title, wid.instance + 1)
        } else {
            title.clone()
        };
        let min_size = Self::native_pty_window_min_size(pty_state);
        let mut open = true;
        let mut header_action = DesktopHeaderAction::None;
        let mut event = PtyScreenEvent::None;
        let (window, maximized) = self.build_resizable_desktop_window(
            ctx,
            DesktopWindow::PtyApp,
            window_title.clone(),
            &mut open,
            ResizableDesktopWindowOptions {
                min_size,
                default_size,
                default_pos: Some(default_pos),
                clamp_restore: true,
            },
        );
        let Some(state) = self.terminal_pty.as_mut() else {
            self.update_desktop_window_state(DesktopWindow::PtyApp, false);
            return;
        };
        let shown = window.show(ctx, |ui| {
            header_action = Self::draw_desktop_window_header(ui, &window_title, maximized);
            let available = ui.available_size();
            let cols_floor = state.desktop_cols_floor.unwrap_or(40) as usize;
            let rows_floor = state.desktop_rows_floor.unwrap_or(20).saturating_add(1) as usize;
            let (cols, rows) = if state.desktop_live_resize {
                (
                    ((available.x / FIXED_PTY_CELL_W).floor() as usize)
                        .max(cols_floor)
                        .clamp(40, 220),
                    ((available.y / FIXED_PTY_CELL_H).floor() as usize)
                        .max(rows_floor)
                        .clamp(20, 60),
                )
            } else {
                (cols_floor, rows_floor)
            };
            ui.allocate_ui_with_layout(available, Layout::top_down(egui::Align::Min), |ui| {
                event = draw_embedded_pty_in_ui_focused(
                    ui,
                    ctx,
                    state,
                    cols,
                    rows,
                    pty_focused,
                    ShellSurfaceKind::Desktop,
                );
            });
        });
        let shown_rect = shown.as_ref().map(|inner| inner.response.rect);
        let shown_contains_pointer = shown
            .as_ref()
            .is_some_and(|inner| inner.response.contains_pointer());
        let completion_message = state.completion_message.clone();
        let title_for_exit = state.title.clone();
        let mut desktop_exit_plan: Option<TerminalDesktopPtyExitPlan> = None;

        match event {
            PtyScreenEvent::None => {}
            PtyScreenEvent::CloseRequested => open = false,
            PtyScreenEvent::ProcessExited => {
                let exit_status = state.session.exit_status();
                let success = exit_status
                    .as_ref()
                    .map(|status| status.success())
                    .unwrap_or(true);
                let exit_code = exit_status.as_ref().map(|status| status.exit_code());
                open = false;
                desktop_exit_plan = Some(resolve_desktop_pty_exit(
                    &title_for_exit,
                    completion_message.as_deref(),
                    success,
                    exit_code,
                ));
            }
        }

        self.maybe_activate_desktop_window_from_click(
            ctx,
            DesktopWindow::PtyApp,
            shown_contains_pointer,
        );
        if !maximized {
            if let Some(rect) = shown_rect {
                self.note_desktop_window_rect(DesktopWindow::PtyApp, rect);
            }
        }
        if let Some(plan) = desktop_exit_plan {
            self.apply_terminal_desktop_pty_exit_plan(plan);
        }
        self.finish_desktop_window_host(
            ctx,
            DesktopWindow::PtyApp,
            &mut open,
            maximized,
            shown_rect,
            shown_contains_pointer,
            DesktopWindowRectTracking::FullRect,
            header_action,
        );
    }

    pub(super) fn draw_desktop_wasm_addon_window(&mut self, ctx: &Context) {
        if self.desktop_window_is_minimized(DesktopWindow::PtyApp) {
            return;
        }
        let wid = self.current_window_id(DesktopWindow::PtyApp);
        let default_size = Self::desktop_default_window_size(DesktopWindow::PtyApp);
        let default_pos = self.active_desktop_default_window_pos(ctx, default_size);
        let focused = self.desktop_active_window == Some(wid);
        let title = self
            .desktop_wasm_addon
            .as_ref()
            .map(|state| state.title().to_string())
            .unwrap_or_else(|| "Addon".to_string());
        let mut open = true;
        let mut header_action = DesktopHeaderAction::None;
        let mut close_requested = false;
        let (window, maximized) = self.build_resizable_desktop_window(
            ctx,
            DesktopWindow::PtyApp,
            title.clone(),
            &mut open,
            ResizableDesktopWindowOptions {
                min_size: egui::vec2(480.0, 320.0),
                default_size,
                default_pos: Some(default_pos),
                clamp_restore: true,
            },
        );
        let shown = window.show(ctx, |ui| {
            header_action = Self::draw_desktop_window_header(ui, &title, maximized);
            let available = ui.available_size();
            let size = HostedAddonSize {
                width: available.x.max(1.0),
                height: available.y.max(1.0),
            };
            let dt = Self::next_embedded_game_dt(&mut self.desktop_wasm_addon_last_frame_at);
            let input = collect_hosted_keyboard_input(ctx, focused);
            let mut failed = None;
            if let Some(state) = self.desktop_wasm_addon.as_mut() {
                if let Err(err) = state.update(size, dt, input) {
                    failed = Some(err);
                } else {
                    draw_hosted_addon_frame(ui, state);
                }
            }
            if let Some(err) = failed {
                self.clear_desktop_wasm_addon();
                self.shell_status = err;
                close_requested = true;
            }
        });
        if close_requested {
            open = false;
        }
        let shown_rect = shown.as_ref().map(|inner| inner.response.rect);
        let shown_contains_pointer = shown
            .as_ref()
            .is_some_and(|inner| inner.response.contains_pointer());
        self.finish_desktop_window_host(
            ctx,
            DesktopWindow::PtyApp,
            &mut open,
            maximized,
            shown_rect,
            shown_contains_pointer,
            DesktopWindowRectTracking::FullRect,
            header_action,
        );
        if open && self.desktop_wasm_addon.is_some() {
            ctx.request_repaint();
        }
    }
}
