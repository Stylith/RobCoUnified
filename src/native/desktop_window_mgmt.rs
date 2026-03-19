use super::app::RobcoNativeApp;
use super::desktop_app::{
    desktop_component_binding, desktop_component_spec, desktop_components, hosted_app_for_window,
    DesktopHostedApp, DesktopWindow,
};
use super::retro_ui::current_palette;
use eframe::egui::{self, Color32, Context, Id, Layout, RichText};

// ── Types ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, Default)]
pub(super) struct DesktopWindowState {
    pub minimized: bool,
    pub maximized: bool,
    pub restore_pos: Option<[f32; 2]>,
    pub restore_size: Option<[f32; 2]>,
    pub user_resized: bool,
    pub apply_restore: bool,
    pub generation: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum DesktopHeaderAction {
    None,
    Minimize,
    ToggleMaximize,
    Close,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum DesktopWindowRectTracking {
    FullRect,
    PositionOnly,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct ResizableDesktopWindowOptions {
    pub min_size: egui::Vec2,
    pub default_size: egui::Vec2,
    pub default_pos: Option<egui::Pos2>,
    pub clamp_restore: bool,
}

// ── impl RobcoNativeApp — window management ─────────────────────────────

impl RobcoNativeApp {
    pub(super) fn desktop_window_is_open(&self, window: DesktopWindow) -> bool {
        (desktop_component_binding(window).is_open)(self)
    }

    pub(super) fn desktop_window_state(&self, window: DesktopWindow) -> DesktopWindowState {
        self.desktop_window_states
            .get(&window)
            .copied()
            .unwrap_or_default()
    }

    pub(super) fn desktop_window_state_mut(
        &mut self,
        window: DesktopWindow,
    ) -> &mut DesktopWindowState {
        self.desktop_window_states.entry(window).or_default()
    }

    pub(super) fn desktop_window_generation(&self, window: DesktopWindow) -> u64 {
        self.desktop_window_states
            .get(&window)
            .map(|state| state.generation)
            .unwrap_or(0)
    }

    pub(super) fn desktop_window_egui_id(&self, window: DesktopWindow) -> egui::Id {
        let gen = self.desktop_window_generation(window);
        Id::new((desktop_component_spec(window).id_salt, gen))
    }

    pub(super) fn next_desktop_window_generation(&mut self) -> u64 {
        let generation = self.desktop_window_generation_seed;
        self.desktop_window_generation_seed =
            self.desktop_window_generation_seed.wrapping_add(1).max(1);
        generation
    }

    pub(super) fn desktop_window_is_minimized(&self, window: DesktopWindow) -> bool {
        self.desktop_window_is_open(window) && self.desktop_window_state(window).minimized
    }

    pub(super) fn desktop_window_is_maximized(&self, window: DesktopWindow) -> bool {
        self.desktop_window_is_open(window) && self.desktop_window_state(window).maximized
    }

    pub(super) fn set_desktop_window_minimized(&mut self, window: DesktopWindow, minimized: bool) {
        if !self.desktop_window_is_open(window) {
            return;
        }
        let state = self.desktop_window_state_mut(window);
        state.minimized = minimized;
        if minimized {
            if self.desktop_active_window == Some(window) {
                self.desktop_active_window = self.first_open_desktop_window();
            }
        } else {
            self.desktop_active_window = Some(window);
        }
    }

    pub(super) fn take_desktop_window_restore_dims(
        &mut self,
        window: DesktopWindow,
    ) -> Option<(egui::Pos2, egui::Vec2)> {
        let state = self.desktop_window_state_mut(window);
        if state.maximized || !state.apply_restore {
            return None;
        }
        state.apply_restore = false;
        let pos = state.restore_pos?;
        let size = state.restore_size?;
        Some((egui::pos2(pos[0], pos[1]), egui::vec2(size[0], size[1])))
    }

    pub(super) fn note_desktop_window_rect(&mut self, window: DesktopWindow, rect: egui::Rect) {
        let state = self.desktop_window_state_mut(window);
        state.restore_pos = Some([rect.min.x, rect.min.y]);
        let restore_size = Self::desktop_window_restore_size(rect);
        state.restore_size = Some([restore_size.x, restore_size.y]);
        state.apply_restore = false;
    }

    pub(super) fn toggle_desktop_window_maximized(
        &mut self,
        window: DesktopWindow,
        current_rect: Option<egui::Rect>,
    ) {
        if !self.desktop_window_is_open(window) {
            return;
        }
        let generation = self.next_desktop_window_generation();
        let state = self.desktop_window_state_mut(window);
        if state.maximized {
            state.maximized = false;
            state.apply_restore = true;
            state.generation = generation;
        } else {
            if let Some(rect) = current_rect {
                state.restore_pos = Some([rect.min.x, rect.min.y]);
                let restore_size = Self::desktop_window_restore_size(rect);
                state.restore_size = Some([restore_size.x, restore_size.y]);
                state.user_resized = true;
            }
            state.maximized = true;
            state.apply_restore = false;
            state.generation = generation;
        }
        state.minimized = false;
        self.desktop_active_window = Some(window);
    }

    pub(super) fn desktop_window_restore_size(rect: egui::Rect) -> egui::Vec2 {
        let margin = Self::desktop_window_frame().total_margin().sum();
        egui::vec2(
            (rect.width() - margin.x).max(160.0),
            (rect.height() - margin.y).max(120.0),
        )
    }

    pub(super) fn desktop_default_window_size(window: DesktopWindow) -> egui::Vec2 {
        let [x, y] = desktop_component_spec(window).default_size;
        egui::vec2(x, y)
    }

    pub(super) fn desktop_file_manager_window_min_size() -> egui::Vec2 {
        egui::vec2(760.0, 520.0)
    }

    pub(super) fn desktop_default_window_pos(ctx: &Context, size: egui::Vec2) -> egui::Pos2 {
        let workspace = Self::desktop_workspace_rect(ctx);
        let x = workspace.left() + ((workspace.width() - size.x) * 0.5).max(24.0);
        let y = workspace.top() + ((workspace.height() - size.y) * 0.18).max(24.0);
        egui::pos2(x, y)
    }

    pub(super) fn desktop_clamp_window_size(
        ctx: &Context,
        size: egui::Vec2,
        min_size: egui::Vec2,
    ) -> egui::Vec2 {
        let workspace = Self::desktop_workspace_rect(ctx);
        egui::vec2(
            size.x.clamp(min_size.x, workspace.width().max(min_size.x)),
            size.y.clamp(min_size.y, workspace.height().max(min_size.y)),
        )
    }

    pub(super) fn desktop_clamp_window_pos(
        ctx: &Context,
        pos: egui::Pos2,
        size: egui::Vec2,
    ) -> egui::Pos2 {
        let workspace = Self::desktop_workspace_rect(ctx);
        egui::pos2(
            pos.x.clamp(
                workspace.left(),
                (workspace.right() - size.x).max(workspace.left()),
            ),
            pos.y.clamp(
                workspace.top(),
                (workspace.bottom() - size.y).max(workspace.top()),
            ),
        )
    }

    pub(super) fn build_resizable_desktop_window<'open, Title>(
        &mut self,
        ctx: &Context,
        desktop_window: DesktopWindow,
        title: Title,
        open: &'open mut bool,
        options: ResizableDesktopWindowOptions,
    ) -> (egui::Window<'open>, bool)
    where
        Title: Into<egui::WidgetText>,
    {
        let maximized = self.desktop_window_is_maximized(desktop_window);
        let restore = self.take_desktop_window_restore_dims(desktop_window);
        let mut window = egui::Window::new(title)
            .id(self.desktop_window_egui_id(desktop_window))
            .open(open)
            .title_bar(false)
            .frame(Self::desktop_window_frame())
            .resizable(true)
            .min_size(options.min_size)
            .default_size(options.default_size);
        if let Some(default_pos) = options.default_pos {
            window = window.default_pos(default_pos);
        }
        if maximized {
            let rect = Self::desktop_workspace_rect(ctx);
            window = window
                .movable(false)
                .resizable(false)
                .fixed_pos(rect.min)
                .fixed_size(rect.size());
        } else if let Some((mut pos, mut size)) = restore {
            if options.clamp_restore {
                size = Self::desktop_clamp_window_size(ctx, size, options.min_size);
                pos = Self::desktop_clamp_window_pos(ctx, pos, size);
            }
            window = window.current_pos(pos).default_size(size);
        }
        (window, maximized)
    }

    pub(super) fn finish_desktop_window_host(
        &mut self,
        ctx: &Context,
        desktop_window: DesktopWindow,
        open: &mut bool,
        maximized: bool,
        shown_rect: Option<egui::Rect>,
        shown_contains_pointer: bool,
        rect_tracking: DesktopWindowRectTracking,
        header_action: DesktopHeaderAction,
    ) {
        self.maybe_activate_desktop_window_from_click(ctx, desktop_window, shown_contains_pointer);
        if !maximized {
            match rect_tracking {
                DesktopWindowRectTracking::FullRect => {
                    if let Some(rect) = shown_rect {
                        self.note_desktop_window_rect(desktop_window, rect);
                    }
                }
                DesktopWindowRectTracking::PositionOnly => {
                    if let Some(pos) = shown_rect.map(|rect| rect.min) {
                        let state = self.desktop_window_state_mut(desktop_window);
                        state.restore_pos = Some([pos.x, pos.y]);
                    }
                }
            }
        }
        match header_action {
            DesktopHeaderAction::None => {}
            DesktopHeaderAction::Close => *open = false,
            DesktopHeaderAction::Minimize => {
                self.set_desktop_window_minimized(desktop_window, true);
            }
            DesktopHeaderAction::ToggleMaximize => {
                self.toggle_desktop_window_maximized(desktop_window, shown_rect);
            }
        }
        self.update_desktop_window_state(desktop_window, *open);
    }

    pub(super) fn prime_desktop_window_defaults(&mut self, window: DesktopWindow) {
        let generation = self.next_desktop_window_generation();
        let state = self.desktop_window_state_mut(window);
        state.restore_pos = None;
        state.restore_size = None;
        state.user_resized = false;
        state.apply_restore = false;
        state.maximized = false;
        state.minimized = false;
        state.generation = generation;
    }

    pub(super) fn set_desktop_window_open(&mut self, window: DesktopWindow, open: bool) {
        let was_open = self.desktop_window_is_open(window);
        (desktop_component_binding(window).set_open)(self, open);
        if !open {
            self.desktop_window_states.remove(&window);
        } else if !was_open && self.desktop_window_is_open(window) {
            let generation = self.next_desktop_window_generation();
            let state = self.desktop_window_state_mut(window);
            state.minimized = false;
            state.maximized = false;
            state.user_resized = false;
            state.generation = generation;
        } else {
            self.desktop_window_states.entry(window).or_default();
        }
    }

    pub(super) fn first_open_desktop_window(&self) -> Option<DesktopWindow> {
        desktop_components()
            .iter()
            .rev()
            .map(|component| component.spec.window)
            .find(|window| {
                self.desktop_window_is_open(*window) && !self.desktop_window_is_minimized(*window)
            })
    }

    pub(super) fn focus_desktop_window(&mut self, ctx: Option<&Context>, window: DesktopWindow) {
        self.desktop_active_window = Some(window);
        if let Some(ctx) = ctx {
            let layer_id =
                egui::LayerId::new(egui::Order::Middle, self.desktop_window_egui_id(window));
            ctx.move_to_top(layer_id);
        }
    }

    pub(super) fn sync_desktop_active_window(&mut self) {
        if self.desktop_active_window.is_some_and(|window| {
            !self.desktop_window_is_open(window) || self.desktop_window_is_minimized(window)
        }) {
            self.desktop_active_window = self.first_open_desktop_window();
            return;
        }
        if self.desktop_active_window.is_none() {
            self.desktop_active_window = self.first_open_desktop_window();
        }
    }

    pub(super) fn open_desktop_window(&mut self, window: DesktopWindow) {
        let was_open = self.desktop_window_is_open(window);
        if let Some(on_open) = desktop_component_binding(window).on_open {
            on_open(self, was_open);
        }
        self.set_desktop_window_open(window, true);
        self.set_desktop_window_minimized(window, false);
        self.desktop_active_window = Some(window);
        if self.desktop_mode_open {
            self.close_desktop_overlays();
        }
    }

    pub(super) fn maybe_activate_desktop_window_from_click(
        &mut self,
        ctx: &Context,
        window: DesktopWindow,
        contains_pointer: bool,
    ) {
        let clicked_inside = ctx.input(|i| {
            (i.pointer.primary_clicked() || i.pointer.secondary_clicked()) && contains_pointer
        });
        if clicked_inside {
            self.focus_desktop_window(Some(ctx), window);
        }
    }

    pub(super) fn handle_closed_desktop_window(&mut self, window: DesktopWindow) {
        if let Some(on_closed) = desktop_component_binding(window).on_closed {
            on_closed(self);
        }
    }

    pub(super) fn close_desktop_window(&mut self, window: DesktopWindow) {
        let was_open = self.desktop_window_is_open(window);
        self.set_desktop_window_open(window, false);
        if was_open {
            self.handle_closed_desktop_window(window);
        }
        if self.desktop_active_window == Some(window) {
            self.desktop_active_window = self.first_open_desktop_window();
        }
    }

    pub(super) fn update_desktop_window_state(&mut self, window: DesktopWindow, open: bool) {
        let was_open = self.desktop_window_is_open(window);
        self.set_desktop_window_open(window, open);
        if was_open && !open {
            self.handle_closed_desktop_window(window);
        }
        if !open && self.desktop_active_window == Some(window) {
            self.desktop_active_window = self.first_open_desktop_window();
        }
    }

    pub(super) fn active_desktop_app(&self) -> DesktopHostedApp {
        hosted_app_for_window(self.desktop_active_window)
    }

    // ── Drawing ──────────────────────────────────────────────────────────

    pub(super) fn draw_desktop_window_by_kind(&mut self, ctx: &Context, window: DesktopWindow) {
        (desktop_component_binding(window).draw)(self, ctx);
    }

    pub(super) fn draw_desktop_windows(&mut self, ctx: &Context) {
        self.sync_desktop_active_window();
        let active = self.desktop_active_window;
        for window in desktop_components()
            .iter()
            .map(|component| component.spec.window)
        {
            if Some(window) == active {
                continue;
            }
            if self.desktop_window_is_minimized(window) {
                continue;
            }
            self.draw_desktop_window_by_kind(ctx, window);
        }
        if let Some(window) = active {
            if !self.desktop_window_is_minimized(window) {
                self.draw_desktop_window_by_kind(ctx, window);
            }
        }
        self.sync_desktop_active_window();
    }

    pub(super) fn desktop_workspace_rect(ctx: &Context) -> egui::Rect {
        const TOP_BAR_H: f32 = 30.0;
        const TASKBAR_H: f32 = 32.0;
        let screen = ctx.screen_rect();
        let top = screen.top() + TOP_BAR_H;
        let bottom = (screen.bottom() - TASKBAR_H).max(top + 120.0);
        egui::Rect::from_min_max(
            egui::pos2(screen.left(), top),
            egui::pos2(screen.right(), bottom),
        )
    }

    pub(super) fn desktop_window_frame() -> egui::Frame {
        let palette = current_palette();
        egui::Frame::none()
            .fill(palette.bg)
            .stroke(egui::Stroke::new(1.0, palette.fg))
            .inner_margin(egui::Margin::same(1.0))
    }

    pub(super) fn desktop_header_glyph_button(
        ui: &mut egui::Ui,
        label: &str,
    ) -> egui::Response {
        ui.add(
            egui::Button::new(RichText::new(label).color(Color32::BLACK).monospace())
                .frame(false)
                .fill(Color32::TRANSPARENT)
                .stroke(egui::Stroke::NONE)
                .min_size(egui::vec2(0.0, 0.0)),
        )
    }

    pub(super) fn draw_desktop_window_header(
        ui: &mut egui::Ui,
        _title: &str,
        maximized: bool,
    ) -> DesktopHeaderAction {
        let palette = current_palette();
        let mut action = DesktopHeaderAction::None;
        egui::Frame::none()
            .fill(palette.fg)
            .inner_margin(egui::Margin::symmetric(8.0, 4.0))
            .show(ui, |ui| {
                ui.set_min_height(20.0);
                ui.horizontal(|ui| {
                    ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add_space(8.0);
                        if Self::desktop_header_glyph_button(ui, "[X]").clicked() {
                            action = DesktopHeaderAction::Close;
                        }
                        if Self::desktop_header_glyph_button(
                            ui,
                            if maximized { "[R]" } else { "[+]" },
                        )
                        .clicked()
                        {
                            action = DesktopHeaderAction::ToggleMaximize;
                        }
                        if Self::desktop_header_glyph_button(ui, "[-]").clicked() {
                            action = DesktopHeaderAction::Minimize;
                        }
                    });
                });
            });
        ui.add_space(2.0);
        action
    }
}
