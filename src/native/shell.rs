//! New shell architecture: sub-state structs, window manager, and the `DesktopApp` trait.
//!
//! This module defines the TARGET data model for the iced-based RobCoOS shell.
//! Nothing here is wired into the running app yet — that happens in Phase 2.
//! The existing [`super::app::RobcoNativeApp`] / egui code is untouched.

#![allow(dead_code)]

use super::desktop_app::{DesktopMenuAction, DesktopMenuSection, DesktopShellAction};
use super::desktop_search_service::NativeSpotlightResult;
use super::desktop_start_menu::{StartLeaf, StartSubmenu};
use super::app::StartMenuRenameState;
use super::desktop_surface_service::DesktopSurfaceEntry;
use super::shared_types::DesktopWindow;
use super::message::{ContextMenuAction, DesktopIconId, Message};
use robcos_native_editor_app::EditorWindow;
use robcos_native_file_manager_app::{FileManagerAction, NativeFileManagerState};
use robcos_native_settings_app::NativeSettingsPanel;
use crate::config::{DesktopIconSortMode, Settings};
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
    pub settings_panel: NativeSettingsPanel,

    // ── Settings ────────────────────────────────────────────────────────────
    pub settings: Settings,

    // ── Status bar ──────────────────────────────────────────────────────────
    pub shell_status: String,
}
