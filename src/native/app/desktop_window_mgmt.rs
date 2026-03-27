use super::super::desktop_app::{
    desktop_component_binding, desktop_component_spec, desktop_components, DesktopWindow,
    DesktopWindowTileAction, WindowInstanceId,
};
use super::super::pty_screen::NativePtyState;
use super::super::retro_ui::current_palette;
use crate::theme::{DockPosition, PanelPosition};
use eframe::egui::{self, Color32, Context, Id, Layout, RichText};

use crate::native::editor_app::EditorWindow;

use super::{RobcoNativeApp, SecondaryWindow, SecondaryWindowApp};

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct DesktopWindowState {
    pub(super) minimized: bool,
    pub(super) maximized: bool,
    pub(super) restore_pos: Option<[f32; 2]>,
    pub(super) restore_size: Option<[f32; 2]>,
    pub(super) user_resized: bool,
    pub(super) apply_restore: bool,
    pub(super) generation: u64,
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
    #[allow(dead_code)]
    PositionOnly,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct ResizableDesktopWindowOptions {
    pub(super) min_size: egui::Vec2,
    pub(super) default_size: egui::Vec2,
    pub(super) default_pos: Option<egui::Pos2>,
    pub(super) clamp_restore: bool,
}

impl RobcoNativeApp {
    pub(super) fn spawn_secondary_window(
        &mut self,
        kind: DesktopWindow,
        app: SecondaryWindowApp,
    ) -> WindowInstanceId {
        let instance = self.next_window_instance;
        self.next_window_instance += 1;
        let id = WindowInstanceId { kind, instance };
        // Set up window state with a fresh generation.
        let generation = self.next_desktop_window_generation();
        let state = self.desktop_window_state_mut(id);
        state.minimized = false;
        state.maximized = false;
        state.generation = generation;
        let secondary = SecondaryWindow { id, app };
        self.secondary_windows.push(secondary);
        self.desktop_active_window = Some(id);
        if self.desktop_mode_open {
            self.close_desktop_overlays();
        }
        id
    }

    // ── Instance resolution ─────────────────────────────────────────────

    /// Returns the WindowInstanceId for the current drawing context.
    /// If `drawing_window_id` is set and matches the given kind, returns that;
    /// otherwise returns the primary instance.
    pub(super) fn current_window_id(&self, kind: DesktopWindow) -> WindowInstanceId {
        match self.drawing_window_id {
            Some(id) if id.kind == kind => id,
            _ => WindowInstanceId::primary(kind),
        }
    }

    pub(super) fn active_window_kind(&self) -> Option<DesktopWindow> {
        self.desktop_active_window.map(|id| id.kind)
    }

    /// Check if the primary instance of a window is open (via component binding).
    pub(super) fn desktop_window_is_open(&self, window: DesktopWindow) -> bool {
        (desktop_component_binding(window).is_open)(self)
    }

    /// Check if a specific window instance is open (primary or secondary).
    pub(super) fn is_window_instance_open(&self, id: WindowInstanceId) -> bool {
        if id.instance == 0 {
            self.desktop_window_is_open(id.kind)
        } else {
            self.secondary_windows
                .iter()
                .any(|sw| sw.id == id && sw.is_open())
        }
    }

    pub(super) fn desktop_window_state(&self, id: WindowInstanceId) -> DesktopWindowState {
        self.desktop_window_states
            .get(&id)
            .copied()
            .unwrap_or_default()
    }

    pub(super) fn desktop_window_state_mut(
        &mut self,
        id: WindowInstanceId,
    ) -> &mut DesktopWindowState {
        self.desktop_window_states.entry(id).or_default()
    }

    pub(super) fn desktop_pty_slot_mut(
        &mut self,
        id: WindowInstanceId,
    ) -> Option<&mut Option<NativePtyState>> {
        if id.kind != DesktopWindow::PtyApp {
            return None;
        }
        if id.instance == 0 {
            if !self.primary_desktop_pty_open() {
                return None;
            }
            return Some(&mut self.terminal_pty);
        }
        self.secondary_windows
            .iter_mut()
            .find(|window| window.id == id)
            .and_then(|window| match &mut window.app {
                SecondaryWindowApp::Pty(state) => Some(state),
                _ => None,
            })
    }

    pub(super) fn desktop_pty_state(&self, id: WindowInstanceId) -> Option<&NativePtyState> {
        if id.kind != DesktopWindow::PtyApp {
            return None;
        }
        if id.instance == 0 {
            if !self.primary_desktop_pty_open() {
                return None;
            }
            return self.terminal_pty.as_ref();
        }
        self.secondary_windows
            .iter()
            .find(|window| window.id == id)
            .and_then(|window| match &window.app {
                SecondaryWindowApp::Pty(state) => state.as_ref(),
                _ => None,
            })
    }

    pub(super) fn active_desktop_pty_state_mut(&mut self) -> Option<&mut NativePtyState> {
        let id = self.desktop_active_window?;
        self.desktop_pty_slot_mut(id)?.as_mut()
    }

    pub(super) fn desktop_window_title_for_instance(&self, id: WindowInstanceId) -> String {
        if id.kind == DesktopWindow::PtyApp && id.instance == 0 {
            if let Some(state) = self.desktop_wasm_addon.as_ref() {
                return state.title().to_string();
            }
        }
        if id.kind == DesktopWindow::PtyApp && id.instance > 0 {
            if let Some(title) = self
                .secondary_windows
                .iter()
                .find(|window| window.id == id)
                .and_then(|window| match &window.app {
                    SecondaryWindowApp::WasmAddon { state, .. } => {
                        state.as_ref().map(|state| state.title().to_string())
                    }
                    _ => None,
                })
            {
                return title;
            }
        }
        let pty_title = self.desktop_pty_state(id).map(|state| state.title.as_str());
        desktop_component_spec(id.kind).title(pty_title)
    }

    pub(super) fn terminate_secondary_window_ptys(windows: &mut [SecondaryWindow]) {
        for window in windows {
            if let SecondaryWindowApp::Pty(state) = &mut window.app {
                if let Some(mut pty) = state.take() {
                    pty.session.terminate();
                }
            }
        }
    }

    pub(super) fn desktop_window_generation(&self, id: WindowInstanceId) -> u64 {
        self.desktop_window_states
            .get(&id)
            .map(|state| state.generation)
            .unwrap_or(0)
    }

    pub(super) fn desktop_window_egui_id(&self, id: WindowInstanceId) -> egui::Id {
        let spec = desktop_component_spec(id.kind);
        let gen = self.desktop_window_generation(id);
        Id::new((spec.id_salt, id.instance, gen))
    }

    pub(super) fn next_desktop_window_generation(&mut self) -> u64 {
        let generation = self.desktop_window_generation_seed;
        self.desktop_window_generation_seed =
            self.desktop_window_generation_seed.wrapping_add(1).max(1);
        generation
    }

    // ── Window state queries (use current_window_id for secondary awareness) ──

    pub(super) fn desktop_window_is_minimized(&self, window: DesktopWindow) -> bool {
        let id = self.current_window_id(window);
        if id.instance == 0 {
            self.desktop_window_is_open(window) && self.desktop_window_state(id).minimized
        } else {
            // During swap-and-draw, component binding check works because state is swapped.
            self.desktop_window_state(id).minimized
        }
    }

    pub(super) fn desktop_window_is_maximized(&self, window: DesktopWindow) -> bool {
        let id = self.current_window_id(window);
        if id.instance == 0 {
            self.desktop_window_is_open(window) && self.desktop_window_state(id).maximized
        } else {
            self.desktop_window_state(id).maximized
        }
    }

    pub(super) fn set_desktop_window_minimized(&mut self, window: DesktopWindow, minimized: bool) {
        let id = self.current_window_id(window);
        if id.instance == 0 && !self.desktop_window_is_open(window) {
            return;
        }
        self.set_window_instance_minimized(id, minimized);
    }

    /// Minimize or un-minimize a specific window instance (primary or secondary).
    pub(super) fn set_window_instance_minimized(&mut self, id: WindowInstanceId, minimized: bool) {
        let state = self.desktop_window_state_mut(id);
        state.minimized = minimized;
        if minimized {
            if self.desktop_active_window == Some(id) {
                self.desktop_active_window = self.first_open_desktop_window();
            }
        } else {
            self.desktop_active_window = Some(id);
        }
    }

    pub(super) fn take_desktop_window_restore_dims(
        &mut self,
        window: DesktopWindow,
    ) -> Option<(egui::Pos2, egui::Vec2)> {
        let id = self.current_window_id(window);
        let state = self.desktop_window_state_mut(id);
        if state.maximized || !state.apply_restore {
            return None;
        }
        state.apply_restore = false;
        let pos = state.restore_pos?;
        let size = state.restore_size?;
        Some((egui::pos2(pos[0], pos[1]), egui::vec2(size[0], size[1])))
    }

    pub(super) fn note_desktop_window_rect(&mut self, window: DesktopWindow, rect: egui::Rect) {
        let id = self.current_window_id(window);
        let state = self.desktop_window_state_mut(id);
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
        let id = self.current_window_id(window);
        if id.instance == 0 && !self.desktop_window_is_open(window) {
            return;
        }
        let generation = self.next_desktop_window_generation();
        let state = self.desktop_window_state_mut(id);
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
        self.desktop_active_window = Some(id);
    }

    pub(super) fn tile_active_desktop_window(
        &mut self,
        ctx: &Context,
        target: DesktopWindowTileAction,
    ) -> bool {
        let Some(id) = self.desktop_active_window else {
            return false;
        };
        self.tile_window_instance(ctx, id, target)
    }

    pub(super) fn tile_window_instance(
        &mut self,
        ctx: &Context,
        id: WindowInstanceId,
        target: DesktopWindowTileAction,
    ) -> bool {
        if !self.is_window_instance_open(id)
            || self.desktop_window_state(id).minimized
            || id.kind == DesktopWindow::TerminalMode
        {
            return false;
        }

        let min_size = self.desktop_window_min_size_for_instance(id);
        let restore_size = self
            .desktop_window_state(id)
            .restore_size
            .map(|size| egui::vec2(size[0], size[1]))
            .unwrap_or_else(|| Self::desktop_default_window_size(id.kind));
        let workspace = self.active_desktop_workspace_rect(ctx);
        let (pos, size) =
            Self::desktop_window_tiled_geometry(workspace, target, min_size, restore_size);
        let generation = self.next_desktop_window_generation();
        let state = self.desktop_window_state_mut(id);
        state.restore_pos = Some([pos.x, pos.y]);
        state.restore_size = Some([size.x, size.y]);
        state.user_resized = true;
        state.apply_restore = true;
        state.maximized = false;
        state.minimized = false;
        state.generation = generation;
        self.desktop_active_window = Some(id);
        true
    }

    pub(super) fn handle_desktop_window_tiling_shortcuts(&mut self, ctx: &Context) -> bool {
        if !self.desktop_mode_open
            || self.terminal_prompt.is_some()
            || self.start_open
            || self.spotlight_open
        {
            return false;
        }

        let Some(id) = self.desktop_active_window else {
            return false;
        };
        if !self.is_window_instance_open(id)
            || self.desktop_window_state(id).minimized
            || id.kind == DesktopWindow::TerminalMode
        {
            return false;
        }

        let shortcut = ctx.input(|i| {
            if !i.modifiers.command || !i.modifiers.alt || i.modifiers.shift {
                return None;
            }
            if i.key_pressed(egui::Key::ArrowLeft) {
                Some((
                    egui::Key::ArrowLeft,
                    DesktopWindowTileAction::LeftHalf,
                    i.modifiers,
                ))
            } else if i.key_pressed(egui::Key::ArrowRight) {
                Some((
                    egui::Key::ArrowRight,
                    DesktopWindowTileAction::RightHalf,
                    i.modifiers,
                ))
            } else if i.key_pressed(egui::Key::ArrowUp) {
                Some((
                    egui::Key::ArrowUp,
                    DesktopWindowTileAction::TopHalf,
                    i.modifiers,
                ))
            } else if i.key_pressed(egui::Key::ArrowDown) {
                Some((
                    egui::Key::ArrowDown,
                    DesktopWindowTileAction::BottomHalf,
                    i.modifiers,
                ))
            } else {
                None
            }
        });
        let Some((key, target, modifiers)) = shortcut else {
            return false;
        };
        ctx.input_mut(|i| {
            i.consume_key(modifiers, key);
        });
        self.tile_window_instance(ctx, id, target)
    }

    // ── Static helpers ──────────────────────────────────────────────────

    pub(super) fn desktop_window_restore_size(rect: egui::Rect) -> egui::Vec2 {
        Self::desktop_window_content_size(rect.size())
    }

    fn desktop_window_content_size(outer_size: egui::Vec2) -> egui::Vec2 {
        let margin = Self::desktop_window_frame().total_margin().sum();
        egui::vec2(
            (outer_size.x - margin.x).max(160.0),
            (outer_size.y - margin.y).max(120.0),
        )
    }

    fn desktop_window_outer_size(content_size: egui::Vec2) -> egui::Vec2 {
        content_size + Self::desktop_window_frame().total_margin().sum()
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

    pub(super) fn active_desktop_workspace_rect(&self, ctx: &Context) -> egui::Rect {
        let screen = ctx.screen_rect();
        let top = match self.desktop_active_layout.panel_position {
            PanelPosition::Top => screen.top() + self.desktop_active_layout.panel_height,
            PanelPosition::Bottom | PanelPosition::Hidden => screen.top(),
        };
        let mut bottom = screen.bottom();
        if self.desktop_active_layout.panel_position == PanelPosition::Bottom {
            bottom -= self.desktop_active_layout.panel_height;
        }
        if self.desktop_active_layout.dock_position != DockPosition::Hidden {
            bottom -= self.desktop_active_layout.dock_size;
        }
        let bottom = bottom.max(top + 120.0);
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

    pub(super) fn desktop_header_glyph_button(ui: &mut egui::Ui, label: &str) -> egui::Response {
        let palette = current_palette();
        ui.add(
            egui::Button::new(RichText::new(label).color(palette.selected_fg).monospace())
                .frame(false)
                .fill(Color32::TRANSPARENT)
                .stroke(egui::Stroke::NONE)
                .min_size(egui::vec2(0.0, 0.0)),
        )
    }

    pub(super) fn desktop_default_window_size(window: DesktopWindow) -> egui::Vec2 {
        let [x, y] = desktop_component_spec(window).default_size;
        egui::vec2(x, y)
    }

    pub(super) fn desktop_file_manager_window_min_size() -> egui::Vec2 {
        egui::vec2(760.0, 520.0)
    }

    pub(super) fn desktop_settings_window_min_size() -> egui::Vec2 {
        egui::vec2(520.0, 360.0)
    }

    pub(super) fn desktop_installer_window_min_size() -> egui::Vec2 {
        egui::vec2(500.0, 400.0)
    }

    pub(super) fn desktop_default_window_pos(ctx: &Context, size: egui::Vec2) -> egui::Pos2 {
        let workspace = Self::desktop_workspace_rect(ctx);
        let x = workspace.left() + ((workspace.width() - size.x) * 0.5).max(24.0);
        let y = workspace.top() + ((workspace.height() - size.y) * 0.18).max(24.0);
        egui::pos2(x, y)
    }

    pub(super) fn active_desktop_default_window_pos(
        &self,
        ctx: &Context,
        size: egui::Vec2,
    ) -> egui::Pos2 {
        let workspace = self.active_desktop_workspace_rect(ctx);
        let x = workspace.left() + ((workspace.width() - size.x) * 0.5).max(24.0);
        let y = workspace.top() + ((workspace.height() - size.y) * 0.18).max(24.0);
        egui::pos2(x, y)
    }

    pub(super) fn desktop_clamp_window_size(
        ctx: &Context,
        size: egui::Vec2,
        min_size: egui::Vec2,
    ) -> egui::Vec2 {
        Self::desktop_clamp_window_size_to_workspace(
            Self::desktop_workspace_rect(ctx),
            size,
            min_size,
        )
    }

    pub(super) fn active_desktop_clamp_window_size(
        &self,
        ctx: &Context,
        size: egui::Vec2,
        min_size: egui::Vec2,
    ) -> egui::Vec2 {
        Self::desktop_clamp_window_size_to_workspace(
            self.active_desktop_workspace_rect(ctx),
            size,
            min_size,
        )
    }

    pub(super) fn desktop_clamp_window_pos(
        ctx: &Context,
        pos: egui::Pos2,
        size: egui::Vec2,
    ) -> egui::Pos2 {
        Self::desktop_clamp_window_pos_to_workspace(Self::desktop_workspace_rect(ctx), pos, size)
    }

    pub(super) fn active_desktop_clamp_window_pos(
        &self,
        ctx: &Context,
        pos: egui::Pos2,
        size: egui::Vec2,
    ) -> egui::Pos2 {
        Self::desktop_clamp_window_pos_to_workspace(
            self.active_desktop_workspace_rect(ctx),
            pos,
            size,
        )
    }

    fn desktop_clamp_window_size_to_workspace(
        workspace: egui::Rect,
        size: egui::Vec2,
        min_size: egui::Vec2,
    ) -> egui::Vec2 {
        egui::vec2(
            size.x.clamp(min_size.x, workspace.width().max(min_size.x)),
            size.y.clamp(min_size.y, workspace.height().max(min_size.y)),
        )
    }

    fn desktop_clamp_window_pos_to_workspace(
        workspace: egui::Rect,
        pos: egui::Pos2,
        size: egui::Vec2,
    ) -> egui::Pos2 {
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

    fn desktop_window_min_size_for_instance(&self, id: WindowInstanceId) -> egui::Vec2 {
        match id.kind {
            DesktopWindow::FileManager => Self::desktop_file_manager_window_min_size(),
            DesktopWindow::Editor => egui::vec2(400.0, 300.0),
            DesktopWindow::Settings => Self::desktop_settings_window_min_size(),
            DesktopWindow::Tweaks => Self::desktop_default_window_size(DesktopWindow::Tweaks),
            DesktopWindow::Applications => egui::vec2(320.0, 250.0),
            DesktopWindow::Installer => Self::desktop_installer_window_min_size(),
            DesktopWindow::PtyApp => self
                .desktop_pty_state(id)
                .map(Self::native_pty_window_min_size)
                .unwrap_or_else(|| egui::vec2(640.0, 300.0)),
            DesktopWindow::TerminalMode => {
                Self::desktop_default_window_size(DesktopWindow::TerminalMode)
            }
        }
    }

    fn desktop_window_tiled_geometry(
        workspace: egui::Rect,
        target: DesktopWindowTileAction,
        min_size: egui::Vec2,
        restore_size: egui::Vec2,
    ) -> (egui::Pos2, egui::Vec2) {
        #[derive(Clone, Copy)]
        enum HorizontalAnchor {
            Left,
            Center,
            Right,
        }

        #[derive(Clone, Copy)]
        enum VerticalAnchor {
            Top,
            Center,
            Bottom,
        }

        let min_outer = Self::desktop_window_outer_size(min_size);
        let restore_outer = Self::desktop_window_outer_size(restore_size);
        let (requested_outer, h_anchor, v_anchor) = match target {
            DesktopWindowTileAction::LeftHalf => (
                egui::vec2(workspace.width() * 0.5, workspace.height()),
                HorizontalAnchor::Left,
                VerticalAnchor::Top,
            ),
            DesktopWindowTileAction::RightHalf => (
                egui::vec2(workspace.width() * 0.5, workspace.height()),
                HorizontalAnchor::Right,
                VerticalAnchor::Top,
            ),
            DesktopWindowTileAction::TopHalf => (
                egui::vec2(workspace.width(), workspace.height() * 0.5),
                HorizontalAnchor::Left,
                VerticalAnchor::Top,
            ),
            DesktopWindowTileAction::BottomHalf => (
                egui::vec2(workspace.width(), workspace.height() * 0.5),
                HorizontalAnchor::Left,
                VerticalAnchor::Bottom,
            ),
            DesktopWindowTileAction::TopLeftQuarter => (
                egui::vec2(workspace.width() * 0.5, workspace.height() * 0.5),
                HorizontalAnchor::Left,
                VerticalAnchor::Top,
            ),
            DesktopWindowTileAction::TopRightQuarter => (
                egui::vec2(workspace.width() * 0.5, workspace.height() * 0.5),
                HorizontalAnchor::Right,
                VerticalAnchor::Top,
            ),
            DesktopWindowTileAction::BottomLeftQuarter => (
                egui::vec2(workspace.width() * 0.5, workspace.height() * 0.5),
                HorizontalAnchor::Left,
                VerticalAnchor::Bottom,
            ),
            DesktopWindowTileAction::BottomRightQuarter => (
                egui::vec2(workspace.width() * 0.5, workspace.height() * 0.5),
                HorizontalAnchor::Right,
                VerticalAnchor::Bottom,
            ),
            DesktopWindowTileAction::Center => (
                restore_outer,
                HorizontalAnchor::Center,
                VerticalAnchor::Center,
            ),
        };
        let outer_size =
            Self::desktop_clamp_window_size_to_workspace(workspace, requested_outer, min_outer);
        let pos = egui::pos2(
            match h_anchor {
                HorizontalAnchor::Left => workspace.left(),
                HorizontalAnchor::Center => workspace.center().x - outer_size.x * 0.5,
                HorizontalAnchor::Right => workspace.right() - outer_size.x,
            },
            match v_anchor {
                VerticalAnchor::Top => workspace.top(),
                VerticalAnchor::Center => workspace.center().y - outer_size.y * 0.5,
                VerticalAnchor::Bottom => workspace.bottom() - outer_size.y,
            },
        );
        let pos = Self::desktop_clamp_window_pos_to_workspace(workspace, pos, outer_size);
        (pos, Self::desktop_window_content_size(outer_size))
    }

    // ── Window builders ─────────────────────────────────────────────────

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
        let id = self.current_window_id(desktop_window);
        let maximized = self.desktop_window_is_maximized(desktop_window);
        let restore = self.take_desktop_window_restore_dims(desktop_window);
        let mut window = egui::Window::new(title)
            .id(self.desktop_window_egui_id(id))
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
            let rect = self.active_desktop_workspace_rect(ctx);
            window = window
                .movable(false)
                .resizable(false)
                .fixed_pos(rect.min)
                .fixed_size(rect.size());
        } else if let Some((mut pos, mut size)) = restore {
            if options.clamp_restore {
                size = self.active_desktop_clamp_window_size(ctx, size, options.min_size);
                pos = self.active_desktop_clamp_window_pos(ctx, pos, size);
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
        let id = self.current_window_id(desktop_window);
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
                        let state = self.desktop_window_state_mut(id);
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

    // ── Window lifecycle ────────────────────────────────────────────────

    pub(super) fn prime_desktop_window_defaults(&mut self, window: DesktopWindow) {
        let id = self.current_window_id(window);
        let generation = self.next_desktop_window_generation();
        let state = self.desktop_window_state_mut(id);
        state.restore_pos = None;
        state.restore_size = None;
        state.user_resized = false;
        state.apply_restore = false;
        state.maximized = false;
        state.minimized = false;
        state.generation = generation;
    }

    pub(super) fn set_desktop_window_open(&mut self, window: DesktopWindow, open: bool) {
        let id = self.current_window_id(window);
        if id.instance > 0 {
            // Secondary instances: the component binding controls the swapped-in state.
            // The actual open/close lifecycle is handled by the swap-and-draw pipeline.
            (desktop_component_binding(window).set_open)(self, open);
            if !open {
                self.desktop_window_states.remove(&id);
            }
            return;
        }
        let was_open = self.desktop_window_is_open(window);
        (desktop_component_binding(window).set_open)(self, open);
        if !open {
            self.desktop_window_states.remove(&id);
        } else if !was_open && self.desktop_window_is_open(window) {
            let generation = self.next_desktop_window_generation();
            let state = self.desktop_window_state_mut(id);
            state.minimized = false;
            state.maximized = false;
            state.user_resized = false;
            state.generation = generation;
        } else {
            self.desktop_window_states.entry(id).or_default();
        }
    }

    pub(super) fn first_open_desktop_window(&self) -> Option<WindowInstanceId> {
        // Check secondary windows first (most recently opened).
        for sw in self.secondary_windows.iter().rev() {
            if sw.is_open() && !self.desktop_window_state(sw.id).minimized {
                return Some(sw.id);
            }
        }
        // Then check primary windows.
        desktop_components()
            .iter()
            .rev()
            .map(|component| component.spec.window)
            .find(|window| {
                self.desktop_window_is_open(*window) && !self.desktop_window_is_minimized(*window)
            })
            .map(WindowInstanceId::primary)
    }

    pub(super) fn focus_desktop_window(&mut self, ctx: Option<&Context>, id: WindowInstanceId) {
        self.desktop_active_window = Some(id);
        if let Some(ctx) = ctx {
            let layer_id = egui::LayerId::new(egui::Order::Middle, self.desktop_window_egui_id(id));
            ctx.move_to_top(layer_id);
        }
    }

    pub(super) fn sync_desktop_active_window(&mut self) {
        if let Some(id) = self.desktop_active_window {
            if !self.is_window_instance_open(id) || self.desktop_window_state(id).minimized {
                self.desktop_active_window = self.first_open_desktop_window();
                return;
            }
        }
        if self.desktop_active_window.is_none() {
            self.desktop_active_window = self.first_open_desktop_window();
        }
    }

    pub(super) fn open_desktop_window(&mut self, window: DesktopWindow) {
        let id = WindowInstanceId::primary(window);
        let was_open = self.desktop_window_is_open(window);
        if let Some(on_open) = desktop_component_binding(window).on_open {
            on_open(self, was_open);
        }
        self.set_desktop_window_open(window, true);
        self.set_desktop_window_minimized(window, false);
        self.desktop_active_window = Some(id);
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
            let id = self.current_window_id(window);
            self.focus_desktop_window(Some(ctx), id);
        }
    }

    pub(super) fn handle_closed_desktop_window(&mut self, window: DesktopWindow) {
        let id = self.current_window_id(window);
        // Skip on_closed callbacks for secondary instances — the callback
        // operates on the primary field which is temporarily swapped.
        if id.instance > 0 {
            return;
        }
        if let Some(on_closed) = desktop_component_binding(window).on_closed {
            on_closed(self);
        }
    }

    fn editor_window_mut_for_id(&mut self, id: WindowInstanceId) -> Option<&mut EditorWindow> {
        if id.kind != DesktopWindow::Editor {
            return None;
        }
        if id.instance == 0 {
            return Some(&mut self.editor);
        }
        self.secondary_windows
            .iter_mut()
            .find(|sw| sw.id == id)
            .and_then(|sw| match &mut sw.app {
                SecondaryWindowApp::Editor(editor) => Some(editor),
                SecondaryWindowApp::FileManager { .. } => None,
                SecondaryWindowApp::Pty(_) => None,
                SecondaryWindowApp::WasmAddon { .. } => None,
            })
    }

    fn close_window_instance_unchecked(&mut self, id: WindowInstanceId) {
        if id.instance > 0 {
            if self.drawing_window_id == Some(id) {
                match id.kind {
                    DesktopWindow::FileManager => self.file_manager.open = false,
                    DesktopWindow::Editor => self.editor.open = false,
                    DesktopWindow::PtyApp => {
                        if self.desktop_wasm_addon.is_some() {
                            self.clear_desktop_wasm_addon();
                        } else if let Some(mut pty) = self.take_primary_pty() {
                            pty.session.terminate();
                        }
                    }
                    _ => {}
                }
            } else if let Some(window) = self
                .secondary_windows
                .iter_mut()
                .find(|window| window.id == id)
            {
                match &mut window.app {
                    SecondaryWindowApp::FileManager { state, .. } => state.open = false,
                    SecondaryWindowApp::Editor(editor) => editor.open = false,
                    SecondaryWindowApp::Pty(state) => {
                        if let Some(mut pty) = state.take() {
                            pty.session.terminate();
                        }
                    }
                    SecondaryWindowApp::WasmAddon { state, .. } => {
                        if let Some(state) = state.take() {
                            self.retained_wasm_addons.push(state);
                        }
                    }
                }
            }
            self.desktop_window_states.remove(&id);
            if self.desktop_active_window == Some(id) {
                self.desktop_active_window = self.first_open_desktop_window();
            }
            return;
        }
        let window = id.kind;
        let was_open = self.desktop_window_is_open(window);
        self.set_desktop_window_open(window, false);
        if was_open {
            self.handle_closed_desktop_window(window);
        }
        if self.desktop_active_window == Some(id) {
            self.desktop_active_window = self.first_open_desktop_window();
        }
    }

    pub(super) fn request_close_window_instance(&mut self, id: WindowInstanceId) {
        if self.desktop_mode_open {
            if let Some(editor) = self.editor_window_mut_for_id(id) {
                if editor.dirty {
                    editor.open = true;
                    editor.prompt_close_confirmation();
                    self.desktop_active_window = Some(id);
                    return;
                }
            }
        }
        self.close_window_instance_unchecked(id);
    }

    pub(super) fn close_current_editor_window_unchecked(&mut self) {
        let id = self.current_window_id(DesktopWindow::Editor);
        self.close_window_instance_unchecked(id);
    }

    pub(super) fn close_desktop_window(&mut self, window: DesktopWindow) {
        let id = self.current_window_id(window);
        self.request_close_window_instance(id);
    }

    pub(super) fn update_desktop_window_state(&mut self, window: DesktopWindow, open: bool) {
        let id = self.current_window_id(window);
        if !open {
            self.request_close_window_instance(id);
            return;
        }
        if id.instance > 0 {
            // Secondary instance: set_open updates the swapped-in state.
            // The swap-draw pipeline will check is_open after swap-back
            // and remove closed secondaries.
            return;
        }
        let was_open = self.desktop_window_is_open(window);
        self.set_desktop_window_open(window, open);
        if !was_open {
            self.desktop_active_window = Some(id);
        }
    }

    // ── Drawing pipeline ────────────────────────────────────────────────

    pub(super) fn draw_desktop_window_by_kind(&mut self, ctx: &Context, window: DesktopWindow) {
        (desktop_component_binding(window).draw)(self, ctx);
    }

    pub(super) fn draw_desktop_windows(&mut self, ctx: &Context) {
        self.sync_desktop_active_window();
        let active = self.desktop_active_window;

        // Draw non-active primary windows.
        for window in desktop_components()
            .iter()
            .map(|component| component.spec.window)
        {
            let primary_id = WindowInstanceId::primary(window);
            if active == Some(primary_id) {
                continue;
            }
            if self.desktop_window_is_minimized(window) {
                continue;
            }
            self.draw_desktop_window_by_kind(ctx, window);
        }

        // Draw secondary windows (swap-and-draw pipeline).
        // Take the Vec out to avoid borrow conflicts.
        let mut secondaries = std::mem::take(&mut self.secondary_windows);
        for secondary in &mut secondaries {
            if active == Some(secondary.id) {
                continue;
            }
            if self.desktop_window_state(secondary.id).minimized {
                continue;
            }
            self.swap_draw_secondary(ctx, secondary);
        }

        // Draw the active window last (on top).
        if let Some(active_id) = active {
            if active_id.instance == 0 {
                let window = active_id.kind;
                if !self.desktop_window_is_minimized(window) {
                    self.draw_desktop_window_by_kind(ctx, window);
                }
            } else {
                if let Some(secondary) = secondaries.iter_mut().find(|s| s.id == active_id) {
                    if !self.desktop_window_state(active_id).minimized {
                        self.swap_draw_secondary(ctx, secondary);
                    }
                }
            }
        }

        // Remove closed secondaries.
        secondaries.retain(|s| s.is_open());
        // Clean up window states for removed secondaries.
        let secondary_ids: std::collections::HashSet<WindowInstanceId> =
            secondaries.iter().map(|s| s.id).collect();
        self.desktop_window_states
            .retain(|id, _| id.instance == 0 || secondary_ids.contains(id));

        self.secondary_windows = secondaries;
        self.sync_desktop_active_window();
    }

    /// Swap secondary instance state into primary fields, draw, swap back.
    fn swap_draw_secondary(&mut self, ctx: &Context, secondary: &mut super::SecondaryWindow) {
        let id = secondary.id;
        self.drawing_window_id = Some(id);
        match &mut secondary.app {
            SecondaryWindowApp::FileManager { state, runtime } => {
                std::mem::swap(&mut self.file_manager, state);
                std::mem::swap(&mut self.file_manager_runtime, runtime);
                // Ensure the swapped-in state is marked open.
                self.file_manager.open = true;
                self.draw_file_manager(ctx);
                std::mem::swap(&mut self.file_manager, state);
                std::mem::swap(&mut self.file_manager_runtime, runtime);
            }
            SecondaryWindowApp::Editor(editor) => {
                std::mem::swap(&mut self.editor, editor);
                self.editor.open = true;
                self.draw_editor(ctx);
                std::mem::swap(&mut self.editor, editor);
            }
            SecondaryWindowApp::Pty(state) => {
                let mut swapped_surface = Some(super::super::menu::TerminalShellSurface::Desktop);
                std::mem::swap(&mut self.terminal_pty, state);
                std::mem::swap(&mut self.terminal_pty_surface, &mut swapped_surface);
                self.draw_desktop_pty_window(ctx);
                std::mem::swap(&mut self.terminal_pty_surface, &mut swapped_surface);
                std::mem::swap(&mut self.terminal_pty, state);
            }
            SecondaryWindowApp::WasmAddon {
                state,
                last_frame_at,
            } => {
                std::mem::swap(&mut self.desktop_wasm_addon, state);
                std::mem::swap(&mut self.desktop_wasm_addon_last_frame_at, last_frame_at);
                self.draw_desktop_wasm_addon_window(ctx);
                std::mem::swap(&mut self.desktop_wasm_addon_last_frame_at, last_frame_at);
                std::mem::swap(&mut self.desktop_wasm_addon, state);
            }
        }
        self.drawing_window_id = None;
    }

    // ── Window header ───────────────────────────────────────────────────

    pub(super) fn draw_desktop_window_header(
        ui: &mut egui::Ui,
        _title: &str,
        maximized: bool,
    ) -> DesktopHeaderAction {
        let palette = current_palette();
        let mut action = DesktopHeaderAction::None;
        // egui::Frame handles background fill + margin in a single allocation.
        // No manual allocate_exact_size/child_ui, so no "double use of widget".
        egui::Frame::none()
            .fill(palette.selected_bg)
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

    /// Collect all open window instance IDs (primary + secondary) for the taskbar.
    pub(super) fn all_open_window_instances(&self) -> Vec<WindowInstanceId> {
        let mut ids: Vec<WindowInstanceId> = desktop_components()
            .iter()
            .filter(|component| component.spec.show_in_taskbar)
            .map(|component| component.spec.window)
            .filter(|window| self.desktop_window_is_open(*window))
            .map(WindowInstanceId::primary)
            .collect();
        for sw in &self.secondary_windows {
            if sw.is_open() {
                ids.push(sw.id);
            }
        }
        ids
    }

    /// Close a specific secondary window by ID.
    pub(super) fn close_secondary_window(&mut self, id: WindowInstanceId) {
        self.request_close_window_instance(id);
    }

    /// Close every desktop window and secondary instance.
    pub(super) fn close_all_desktop_windows(&mut self) {
        for binding in desktop_components() {
            (binding.set_open)(self, false);
        }
        Self::terminate_secondary_window_ptys(&mut self.secondary_windows);
        self.secondary_windows.clear();
        self.desktop_window_states.clear();
        self.desktop_active_window = None;
    }
}

#[cfg(test)]
mod tests {
    use super::super::super::desktop_app::DesktopWindowTileAction;
    use super::RobcoNativeApp;
    use eframe::egui;

    #[test]
    fn tiled_geometry_anchors_quarters_and_center() {
        let workspace =
            egui::Rect::from_min_size(egui::pos2(20.0, 30.0), egui::vec2(1000.0, 700.0));
        let min_size = egui::vec2(320.0, 240.0);
        let restore_size = egui::vec2(480.0, 360.0);

        let (top_right_pos, top_right_size) = RobcoNativeApp::desktop_window_tiled_geometry(
            workspace,
            DesktopWindowTileAction::TopRightQuarter,
            min_size,
            restore_size,
        );
        let top_right_outer = RobcoNativeApp::desktop_window_outer_size(top_right_size);
        assert_eq!(top_right_pos.y, workspace.top());
        assert!((top_right_pos.x + top_right_outer.x - workspace.right()).abs() < 0.01);

        let (center_pos, center_size) = RobcoNativeApp::desktop_window_tiled_geometry(
            workspace,
            DesktopWindowTileAction::Center,
            min_size,
            restore_size,
        );
        let center_outer = RobcoNativeApp::desktop_window_outer_size(center_size);
        assert!((center_pos.x + center_outer.x * 0.5 - workspace.center().x).abs() < 0.01);
        assert!((center_pos.y + center_outer.y * 0.5 - workspace.center().y).abs() < 0.01);
    }
}
