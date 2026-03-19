//! New shell architecture: sub-state structs, window manager, and the `DesktopApp` trait.
//!
//! This module defines the TARGET data model for the iced-based RobCoOS shell.
//! Nothing here is wired into the running app yet — that happens in Phase 2.
//! The existing [`super::app::RobcoNativeApp`] / egui code is untouched.

#![allow(dead_code)]

use super::app::StartMenuRenameState;
use super::desktop_app::{DesktopMenuAction, DesktopMenuSection, DesktopShellAction};
use super::desktop_search_service::NativeSpotlightResult;
use super::desktop_settings_service::load_settings_snapshot;
use super::desktop_start_menu::{StartLeaf, StartSubmenu};
use super::desktop_surface_service::DesktopSurfaceEntry;
use super::desktop_wm_widget::{DesktopWindowHost, WindowChild};
use super::message::{ContextMenuAction, DesktopIconId, Message};
use super::shared_types::DesktopWindow;
use crate::config::{DesktopIconSortMode, Settings};
use chrono::Local;
use iced::{Element, Subscription, Task, Theme};
use robcos_native_editor_app::EditorWindow;
use robcos_native_file_manager_app::{FileManagerAction, NativeFileManagerState};
use robcos_native_settings_app::NativeSettingsPanel;
use std::collections::HashMap;
use std::path::PathBuf;

// ── Egui-free window geometry ─────────────────────────────────────────────────

/// A 2-D rectangle that doesn't depend on egui or iced.
/// Used for inner desktop window positions and sizes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WindowRect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl WindowRect {
    pub fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self { x, y, w, h }
    }

    pub fn contains(&self, px: f32, py: f32) -> bool {
        px >= self.x && px <= self.x + self.w && py >= self.y && py <= self.y + self.h
    }

    pub fn right(&self) -> f32 { self.x + self.w }
    pub fn bottom(&self) -> f32 { self.y + self.h }

    /// Clamp position so the window stays on-screen given a workspace rect.
    pub fn clamped_to(self, workspace: WindowRect) -> Self {
        let x = self.x.clamp(workspace.x, (workspace.right() - self.w).max(workspace.x));
        let y = self.y.clamp(workspace.y, (workspace.bottom() - self.h).max(workspace.y));
        Self { x, y, ..self }
    }
}

/// Open / minimised / maximised state of an inner window.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WindowLifecycle {
    #[default]
    Normal,
    Minimized,
    Maximized,
}

// ── WindowManager ─────────────────────────────────────────────────────────────

/// State for a single inner (desktop) window.
#[derive(Debug, Clone)]
pub struct ManagedWindow {
    pub id: DesktopWindow,
    pub rect: WindowRect,
    pub lifecycle: WindowLifecycle,
    /// Saved rect to restore when un-maximising or un-minimising.
    pub restore_rect: Option<WindowRect>,
    pub min_size: (f32, f32),
    pub resizable: bool,
    /// Monotonically-increasing stamp for identity after close/reopen cycles.
    pub generation: u64,
}

impl ManagedWindow {
    pub fn is_minimized(&self) -> bool {
        self.lifecycle == WindowLifecycle::Minimized
    }

    pub fn is_maximized(&self) -> bool {
        self.lifecycle == WindowLifecycle::Maximized
    }

    pub fn is_visible(&self) -> bool {
        self.lifecycle != WindowLifecycle::Minimized
    }
}

/// In-progress title-bar drag.
#[derive(Debug, Clone)]
pub struct WindowDrag {
    pub window: DesktopWindow,
    pub origin_rect: WindowRect,
}

/// In-progress resize-handle drag.
#[derive(Debug, Clone)]
pub struct WindowResize {
    pub window: DesktopWindow,
    pub origin_rect: WindowRect,
}

/// Manages all inner desktop windows: positions, z-order, focus, drag/resize.
///
/// Replaces the scattered `desktop_window_states` / `desktop_active_window` /
/// `desktop_window_generation_seed` fields on `RobcoNativeApp`.
#[derive(Debug, Default)]
pub struct WindowManager {
    /// Per-window state. A window that is not in this map is closed.
    windows: HashMap<DesktopWindow, ManagedWindow>,
    /// Front-to-back z-order; `z_order[0]` is the topmost window.
    z_order: Vec<DesktopWindow>,
    /// Currently focused window.
    active: Option<DesktopWindow>,
    /// Drag in progress (title bar drag).
    drag: Option<WindowDrag>,
    /// Resize in progress.
    resize: Option<WindowResize>,
    generation_seed: u64,
}

impl WindowManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_open(&self, window: DesktopWindow) -> bool {
        self.windows.contains_key(&window)
    }

    pub fn get(&self, window: DesktopWindow) -> Option<&ManagedWindow> {
        self.windows.get(&window)
    }

    pub fn get_mut(&mut self, window: DesktopWindow) -> Option<&mut ManagedWindow> {
        self.windows.get_mut(&window)
    }

    pub fn active(&self) -> Option<DesktopWindow> {
        self.active
    }

    /// Z-ordered list of open windows, topmost first.
    pub fn z_ordered(&self) -> impl Iterator<Item = &ManagedWindow> {
        self.z_order.iter().filter_map(|id| self.windows.get(id))
    }

    /// Bring `window` to the top of the z-order and mark it active.
    pub fn bring_to_front(&mut self, window: DesktopWindow) {
        self.z_order.retain(|&id| id != window);
        self.z_order.insert(0, window);
        self.active = Some(window);
    }

    pub fn open(&mut self, window: DesktopWindow, rect: WindowRect, min_size: (f32, f32), resizable: bool) {
        self.generation_seed += 1;
        let managed = ManagedWindow {
            id: window,
            rect,
            lifecycle: WindowLifecycle::Normal,
            restore_rect: None,
            min_size,
            resizable,
            generation: self.generation_seed,
        };
        self.windows.insert(window, managed);
        self.bring_to_front(window);
    }

    pub fn close(&mut self, window: DesktopWindow) {
        self.windows.remove(&window);
        self.z_order.retain(|&id| id != window);
        if self.active == Some(window) {
            self.active = self.z_order.first().copied();
        }
    }

    pub fn minimize(&mut self, window: DesktopWindow) {
        if let Some(w) = self.windows.get_mut(&window) {
            if w.lifecycle == WindowLifecycle::Normal {
                w.restore_rect = Some(w.rect);
            }
            w.lifecycle = WindowLifecycle::Minimized;
        }
        if self.active == Some(window) {
            // Activate the next visible window.
            self.active = self.z_order.iter()
                .find(|&&id| id != window && self.windows.get(&id).map_or(false, |w| w.is_visible()))
                .copied();
        }
    }

    pub fn toggle_maximize(&mut self, window: DesktopWindow, workspace: WindowRect) {
        let Some(w) = self.windows.get_mut(&window) else { return };
        if w.lifecycle == WindowLifecycle::Maximized {
            if let Some(r) = w.restore_rect.take() {
                w.rect = r;
            }
            w.lifecycle = WindowLifecycle::Normal;
        } else {
            w.restore_rect = Some(w.rect);
            w.rect = workspace;
            w.lifecycle = WindowLifecycle::Maximized;
        }
    }
}

// ── SpotlightState ────────────────────────────────────────────────────────────

/// All state for the spotlight / search overlay.
///
/// Replaces `spotlight_*` fields scattered across `RobcoNativeApp`.
#[derive(Debug, Default)]
pub struct SpotlightState {
    pub open: bool,
    pub query: String,
    /// 0=All 1=Apps 2=Documents 3=Files
    pub tab: u8,
    pub selected: usize,
    pub results: Vec<NativeSpotlightResult>,
    // Cache keys to avoid redundant searches.
    last_query: String,
    last_tab: u8,
}

impl SpotlightState {
    /// Reset to initial state (called when opening spotlight).
    pub fn reset(&mut self) {
        self.open = true;
        self.query.clear();
        self.tab = 0;
        self.selected = 0;
        self.results.clear();
        self.last_query.clear();
        self.last_tab = u8::MAX;
    }

    pub fn close(&mut self) {
        self.open = false;
    }

    pub fn needs_refresh(&self) -> bool {
        self.query.to_lowercase() != self.last_query || self.tab != self.last_tab
    }

    pub fn mark_refreshed(&mut self) {
        self.last_query = self.query.to_lowercase();
        self.last_tab = self.tab;
    }

    pub fn set_tab(&mut self, tab: u8) {
        let next = tab.min(3);
        if self.tab != next {
            self.tab = next;
            self.selected = 0;
            self.last_tab = u8::MAX;
        }
    }
}

// ── StartMenuState ────────────────────────────────────────────────────────────

/// All state for the start menu panel.
///
/// Replaces `start_*` fields on `RobcoNativeApp`.
#[derive(Debug, Default)]
pub struct StartMenuState {
    pub open: bool,
    pub selected_root: usize,
    pub system_selected: usize,
    pub leaf_selected: usize,
    pub open_submenu: Option<StartSubmenu>,
    pub open_leaf: Option<StartLeaf>,
    /// Height of the last-rendered root panel (used to anchor the leaf panel).
    pub panel_height: f32,
    /// Pending rename operation (shows a rename input window).
    pub rename: Option<StartMenuRenameState>,
    /// Screen-space rect of the [Start] button (used to anchor the menu).
    pub start_button_pos: Option<(f32, f32)>,
}

impl StartMenuState {
    pub fn close(&mut self) {
        self.open = false;
        self.open_submenu = None;
        self.open_leaf = None;
    }
}

// ── DesktopSurfaceState ───────────────────────────────────────────────────────

/// In-progress desktop icon drag.
#[derive(Debug, Clone)]
pub struct IconDrag {
    pub id: DesktopIconId,
    /// Pixel offset from icon origin at drag start.
    pub offset_x: f32,
    pub offset_y: f32,
    /// Current pointer position.
    pub current_x: f32,
    pub current_y: f32,
}

/// All state for the desktop background surface (icons, wallpaper, pickers, drag).
///
/// Replaces the surface-related fields scattered across `RobcoNativeApp`.
#[derive(Debug, Default)]
pub struct DesktopSurfaceState {
    pub selected_icon: Option<DesktopIconId>,
    /// Pending context menu action to execute after the menu closes.
    pub context_menu_action: Option<ContextMenuAction>,
    pub picking_wallpaper: bool,
    pub picking_icon_for_shortcut: Option<usize>,
    pub icon_drag: Option<IconDrag>,
    /// Cached surface entries (populated async by directory scan).
    pub entries: Vec<DesktopSurfaceEntry>,
}

// ── DesktopApp trait ─────────────────────────────────────────────────────────

/// Interface that every hosted desktop application must implement.
///
/// The shell holds a `Vec<Box<dyn DesktopApp>>` and calls these methods to
/// orchestrate app lifecycle, menus, and event routing.
///
/// `view()` is intentionally absent here — it requires `iced::Element<Message>`,
/// which will be added in Phase 2 once iced is a dependency.
pub trait DesktopApp: std::fmt::Debug {
    /// The `DesktopWindow` variant this app occupies.
    fn window_id(&self) -> DesktopWindow;

    /// Title displayed in the window chrome.
    fn title(&self) -> &str;

    /// Default (first-open) size as (width, height) in logical pixels.
    fn default_size(&self) -> (f32, f32);

    /// Minimum allowed size as (width, height) in logical pixels.
    fn min_size(&self) -> (f32, f32) {
        (400.0, 300.0)
    }

    /// Whether the window can be resized by the user.
    fn resizable(&self) -> bool {
        true
    }

    /// Whether this app appears in the taskbar while open.
    fn show_in_taskbar(&self) -> bool {
        true
    }

    /// Menu sections this app contributes to the top menu bar.
    /// Return an empty Vec for apps with no menus.
    fn menu_sections(&self) -> Vec<DesktopMenuSection>;

    /// Build the menu items for a specific section.
    /// Called by the menu bar to populate the dropdown for `section`.
    fn build_menu_section(&self, section: DesktopMenuSection) -> Vec<super::desktop_app::DesktopMenuItem> {
        let _ = section;
        vec![]
    }

    /// Handle a message directed at this app.
    ///
    /// Returns any follow-up messages the shell should process next frame.
    /// Pure state mutation — no iced Commands here; those come via the shell's
    /// own `update()` wrapping this call.
    fn update(&mut self, msg: &Message) -> Vec<Message>;

    /// Called when the window is first opened (or re-opened after close).
    fn on_open(&mut self) {}

    /// Called when the window is closed.
    fn on_close(&mut self) {}
}

// ── RobcoShell ────────────────────────────────────────────────────────────────

/// Top-level shell state for the iced-based implementation.
///
/// Replaces [`super::app::RobcoNativeApp`]. Fields are grouped by concern
/// into focused sub-state structs.
///
/// Instantiation and the iced `Application` impl arrive in Phase 2.
#[derive(Debug)]
pub struct RobcoShell {
    // ── Desktop shell sub-states ────────────────────────────────────────────
    pub windows: WindowManager,
    pub spotlight: SpotlightState,
    pub start_menu: StartMenuState,
    pub surface: DesktopSurfaceState,

    // ── Mode ────────────────────────────────────────────────────────────────
    /// true = desktop mode, false = terminal mode (full-screen PTY canvas)
    pub desktop_mode: bool,

    // ── Session ─────────────────────────────────────────────────────────────
    /// Logged-in username. `None` = login screen is shown.
    pub session_username: Option<String>,
    pub session_is_admin: bool,

    // ── Hosted app state ────────────────────────────────────────────────────
    // These will be refactored into Box<dyn DesktopApp> entries during Phase 3.
    // For now they remain as concrete types so we don't need to port all apps
    // before the iced scaffold compiles.
    pub file_manager: NativeFileManagerState,
    pub editor: EditorWindow,
    pub settings_panel: Option<NativeSettingsPanel>,

    // ── Settings ────────────────────────────────────────────────────────────
    pub settings: Settings,

    // ── Status bar ──────────────────────────────────────────────────────────
    pub shell_status: String,

    // ── Clock ────────────────────────────────────────────────────────────────
    /// Last clock string, refreshed on Tick.
    pub clock: String,
}

// ── RobcoShell iced Application methods ──────────────────────────────────────

impl RobcoShell {
    /// Construct the initial shell state and return it alongside the first Task.
    ///
    /// Called by the iced entry point via `run_with(RobcoShell::new)`.
    pub fn new() -> (Self, Task<Message>) {
        let settings = load_settings_snapshot();
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        let mut windows = WindowManager::new();
        // Open two demo windows so the WM widget is testable on launch.
        windows.open(
            DesktopWindow::FileManager,
            WindowRect::new(60.0, 40.0, 640.0, 420.0),
            (400.0, 300.0),
            true,
        );
        windows.open(
            DesktopWindow::Editor,
            WindowRect::new(200.0, 80.0, 600.0, 400.0),
            (400.0, 300.0),
            true,
        );

        let shell = Self {
            windows,
            spotlight: SpotlightState::default(),
            start_menu: StartMenuState::default(),
            surface: DesktopSurfaceState::default(),
            desktop_mode: true,
            session_username: None,
            session_is_admin: false,
            file_manager: NativeFileManagerState::new(home),
            editor: EditorWindow::default(),
            settings_panel: None,
            settings,
            shell_status: String::new(),
            clock: Local::now().format("%H:%M").to_string(),
        };
        (shell, Task::none())
    }

    /// Dispatch a message to the appropriate sub-state handler.
    ///
    /// Returns a Task for any async follow-up work. Most variants are sync
    /// (Task::none). Async variants (PTY, file ops, search) will be added
    /// in Phase 3 via Task::perform / Subscription.
    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            // ── Spotlight ───────────────────────────────────────────────────
            Message::OpenSpotlight => {
                self.start_menu.close();
                self.spotlight.reset();
            }
            Message::CloseSpotlight => {
                self.spotlight.close();
            }
            Message::SpotlightQueryChanged(q) => {
                self.spotlight.query = q;
                self.spotlight.selected = 0;
            }
            Message::SpotlightTabChanged(t) => {
                self.spotlight.set_tab(t);
            }
            Message::SpotlightNavigate(dir) => {
                use super::message::NavDirection;
                match dir {
                    NavDirection::Down => {
                        if self.spotlight.selected + 1 < self.spotlight.results.len() {
                            self.spotlight.selected += 1;
                        }
                    }
                    NavDirection::Up => {
                        self.spotlight.selected = self.spotlight.selected.saturating_sub(1);
                    }
                    NavDirection::Right | NavDirection::Tab => {
                        self.spotlight.set_tab(self.spotlight.tab.saturating_add(1));
                    }
                    NavDirection::Left | NavDirection::ShiftTab => {
                        self.spotlight.set_tab(self.spotlight.tab.saturating_sub(1));
                    }
                }
            }
            Message::SpotlightActivateSelected => {
                self.spotlight.close();
                self.spotlight.query.clear();
            }
            Message::SpotlightResultsReady(results) => {
                self.spotlight.results = results;
                self.spotlight.selected = 0;
                self.spotlight.mark_refreshed();
            }

            // ── Start menu ──────────────────────────────────────────────────
            Message::StartButtonClicked => {
                if self.start_menu.open {
                    self.start_menu.close();
                } else {
                    self.spotlight.close();
                    self.start_menu.open = true;
                }
            }
            Message::StartMenuClose => {
                self.start_menu.close();
            }
            Message::StartMenuSelectRoot(idx) => {
                self.start_menu.selected_root = idx;
            }
            Message::StartMenuSelectSystem(idx) => {
                self.start_menu.system_selected = idx;
            }
            Message::StartMenuSelectLeaf(idx) => {
                self.start_menu.leaf_selected = idx;
            }
            Message::StartMenuOpenSubmenu(s) => {
                self.start_menu.open_submenu = Some(s);
            }
            Message::StartMenuOpenLeaf(l) => {
                self.start_menu.open_leaf = Some(l);
            }
            Message::StartMenuActivate => {
                // Phase 3: resolve action and execute it
            }
            Message::StartMenuNavigate(_dir) => {
                // Phase 3: keyboard nav within start menu
            }

            // ── Window management ────────────────────────────────────────────
            Message::OpenWindow(w) => {
                if !self.windows.is_open(w) {
                    // Default size/position — will come from WindowManager in Phase 3
                    self.windows.open(w, WindowRect::new(100.0, 60.0, 800.0, 560.0), (400.0, 300.0), true);
                } else {
                    self.windows.bring_to_front(w);
                }
            }
            Message::CloseWindow(w) => {
                self.windows.close(w);
            }
            Message::MinimizeWindow(w) => {
                self.windows.minimize(w);
            }
            Message::ToggleMaximizeWindow(w) => {
                // Workspace rect approximation; real one comes from layout in Phase 3
                self.windows.toggle_maximize(w, WindowRect::new(0.0, 32.0, 1360.0, 808.0));
            }
            Message::FocusWindow(w) => {
                self.windows.bring_to_front(w);
            }
            Message::WindowHeaderButtonClicked { window, button } => {
                use super::message::WindowHeaderButton;
                match button {
                    WindowHeaderButton::Close => self.windows.close(window),
                    WindowHeaderButton::Minimize => self.windows.minimize(window),
                    WindowHeaderButton::Maximize | WindowHeaderButton::Restore => {
                        self.windows.toggle_maximize(window, WindowRect::new(0.0, 32.0, 1360.0, 808.0));
                    }
                }
            }
            Message::WindowMoved { window, x, y } => {
                if let Some(w) = self.windows.get_mut(window) {
                    w.rect.x = x;
                    w.rect.y = y;
                }
            }
            Message::WindowResized { window, w, h } => {
                if let Some(win) = self.windows.get_mut(window) {
                    win.rect.w = w;
                    win.rect.h = h;
                }
            }

            // ── Taskbar ──────────────────────────────────────────────────────
            Message::TaskbarWindowClicked(w) => {
                if self.windows.get(w).map_or(false, |m| m.is_minimized()) {
                    // Un-minimise
                    if let Some(win) = self.windows.get_mut(w) {
                        win.lifecycle = WindowLifecycle::Normal;
                    }
                    self.windows.bring_to_front(w);
                } else if self.windows.active() == Some(w) {
                    self.windows.minimize(w);
                } else {
                    self.windows.bring_to_front(w);
                }
            }

            // ── Shell actions ────────────────────────────────────────────────
            Message::ShellAction(action) => {
                use super::desktop_app::DesktopShellAction;
                match action {
                    DesktopShellAction::OpenWindow(w) => {
                        return self.update(Message::OpenWindow(w));
                    }
                    _ => { /* Phase 3 */ }
                }
            }
            Message::DesktopModeToggled => {
                self.desktop_mode = !self.desktop_mode;
            }

            // ── Session ──────────────────────────────────────────────────────
            Message::LogoutRequested => {
                self.session_username = None;
                self.session_is_admin = false;
            }

            // ── Desktop surface ──────────────────────────────────────────────
            Message::DesktopSelectionCleared => {
                self.surface.selected_icon = None;
            }
            Message::DesktopIconClicked { id, .. } => {
                self.surface.selected_icon = Some(id);
            }

            // ── System ───────────────────────────────────────────────────────
            Message::Tick(_) => {
                self.clock = Local::now().format("%H:%M").to_string();
            }
            Message::PersistSnapshotRequested => {
                // Phase 3: call persist_native_shell_snapshot()
            }

            // All other variants are stubs for Phase 3+
            _ => {}
        }
        Task::none()
    }

    pub fn view(&self) -> Element<'_, Message> {
        use iced::widget::column;
        column![
            self.view_top_bar(),
            self.view_desktop(),
            self.view_taskbar(),
        ]
        .into()
    }

    fn view_top_bar(&self) -> Element<'_, Message> {
        use iced::widget::{button, container, row, text, Space};
        use iced::Length;

        let palette = super::retro_theme::current_retro_colors();
        let fg = palette.fg.to_iced();
        let panel_bg = palette.panel.to_iced();
        let selected_fg = palette.selected_fg.to_iced();
        let selected_bg = palette.selected_bg.to_iced();
        let dim = palette.dim.to_iced();

        // Active app name (bold, leftmost).
        let active_app_name = self.windows.active()
            .map(|w| format!("{:?}", w))
            .unwrap_or_else(|| "RobCoOS".to_string());

        let app_label = button(
            text(active_app_name).size(14).color(selected_fg)
        )
        .style(move |_t, _s| button::Style {
            background: Some(iced::Background::Color(selected_bg)),
            text_color: selected_fg,
            border: iced::Border::default(),
            ..button::Style::default()
        })
        .padding([3, 8]);

        // Standard menu sections.
        let menu_items = ["File", "Edit", "View", "Window", "Help"];
        let mut menu_row = row![app_label].spacing(0).padding([0, 4]);
        for label in menu_items {
            let fg2 = fg;
            let btn = button(text(label).size(13).color(fg2))
                .style(move |_t, status| {
                    use iced::widget::button::Status;
                    let bg = match status {
                        Status::Hovered | Status::Pressed => {
                            Some(iced::Background::Color(palette.hovered_bg.to_iced()))
                        }
                        _ => None,
                    };
                    button::Style {
                        background: bg,
                        text_color: fg2,
                        border: iced::Border::default(),
                        ..button::Style::default()
                    }
                })
                .padding([3, 8]);
            menu_row = menu_row.push(btn);
        }

        let clock_str = self.clock.clone();
        let top_row = row![
            menu_row,
            Space::with_width(Length::Fill),
            text(clock_str).size(13).color(dim),
            Space::with_width(8),
        ]
        .align_y(iced::Alignment::Center)
        .height(28);

        container(top_row)
            .width(Length::Fill)
            .style(move |_t| container::Style {
                background: Some(iced::Background::Color(panel_bg)),
                ..container::Style::default()
            })
            .into()
    }

    fn view_desktop(&self) -> Element<'_, Message> {
        use iced::widget::{column, container, text};
        use iced::Length;

        let palette = super::retro_theme::current_retro_colors();
        let fg = palette.fg.to_iced();
        let bg = palette.bg.to_iced();
        let dim = palette.dim.to_iced();

        let wm_children: Vec<WindowChild<'_>> = self.windows.z_ordered()
            .map(|w| {
                let id = w.id;
                let is_active = self.windows.active() == Some(id);
                let lifecycle = w.lifecycle;
                let resizable = w.resizable;
                let rect = w.rect;
                let title = format!("{:?}", id);

                let content: Element<'_, Message> = container(
                    column![
                        text(format!("{:?}", id)).size(15).color(fg),
                        text("Window content placeholder").size(11).color(dim),
                    ]
                    .spacing(6)
                    .padding(10)
                )
                .width(Length::Fill)
                .height(Length::Fill)
                .style(move |_t| container::Style {
                    background: Some(iced::Background::Color(bg)),
                    ..container::Style::default()
                })
                .into();

                WindowChild { id, rect, title, lifecycle, is_active, resizable, content }
            })
            .collect();

        container(Element::from(DesktopWindowHost::new(wm_children)))
            .width(Length::Fill)
            .height(Length::Fill)
            .style(move |_t| container::Style {
                background: Some(iced::Background::Color(bg)),
                ..container::Style::default()
            })
            .into()
    }

    fn view_taskbar(&self) -> Element<'_, Message> {
        use iced::widget::{button, container, row, text, Space};
        use iced::{Border, Length};

        let palette = super::retro_theme::current_retro_colors();
        let fg = palette.fg.to_iced();
        let panel_bg = palette.panel.to_iced();
        let selected_fg = palette.selected_fg.to_iced();
        let selected_bg = palette.selected_bg.to_iced();
        let active_bg = palette.active_bg.to_iced();

        // [Start] button — always filled green.
        let start_label = if self.start_menu.open { "[Close]" } else { "[Start]" };
        let start_btn = button(
            text(start_label).size(14).color(selected_fg)
        )
        .on_press(Message::StartButtonClicked)
        .style(move |_t, _s| button::Style {
            background: Some(iced::Background::Color(selected_bg)),
            text_color: selected_fg,
            border: Border {
                color: fg,
                width: 2.0,
                radius: 0.0.into(),
            },
            ..button::Style::default()
        })
        .padding([4, 10]);

        let mut task_row = row![start_btn, Space::with_width(6)].spacing(3).padding([3, 4]);

        // One button per open window.
        for win in self.windows.z_ordered() {
            let id = win.id;
            let is_active = self.windows.active() == Some(id);
            let is_minimized = win.is_minimized();
            let label = format!(" {:?} ", id);

            let (btn_bg, btn_fg, border_w) = if is_active {
                (active_bg, selected_fg, 2.0_f32)
            } else if is_minimized {
                (panel_bg, palette.dim.to_iced(), 1.0_f32)
            } else {
                (panel_bg, fg, 1.0_f32)
            };

            let btn = button(text(label).size(12).color(btn_fg))
                .on_press(Message::TaskbarWindowClicked(id))
                .style(move |_t, _s| button::Style {
                    background: Some(iced::Background::Color(btn_bg)),
                    text_color: btn_fg,
                    border: Border {
                        color: fg,
                        width: border_w,
                        radius: 0.0.into(),
                    },
                    ..button::Style::default()
                })
                .padding([4, 8]);

            task_row = task_row.push(btn);
        }

        task_row = task_row.push(Space::with_width(Length::Fill));

        container(task_row)
            .width(Length::Fill)
            .height(32)
            .style(move |_t| container::Style {
                background: Some(iced::Background::Color(panel_bg)),
                ..container::Style::default()
            })
            .into()
    }

    /// Return the application theme.
    ///
    /// Phase 2: uses iced's built-in Dark theme.
    /// Phase 4: replace with a custom `RetroTheme` that applies the full palette.
    pub fn theme(&self) -> Theme {
        Theme::Dark
    }

    /// Return active subscriptions.
    ///
    /// Phase 3b: clock tick every 30 seconds.
    /// Phase 3g: also add PTY output stream.
    pub fn subscription(&self) -> Subscription<Message> {
        iced::time::every(std::time::Duration::from_secs(30))
            .map(Message::Tick)
    }
}
