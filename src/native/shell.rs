//! New shell architecture: sub-state structs, window manager, and the `DesktopApp` trait.
//!
//! This module defines the TARGET data model for the iced-based RobCoOS shell.
//! Nothing here is wired into the running app yet — that happens in Phase 2.
//! The legacy egui implementation remains untouched here.

#![allow(dead_code)]

use super::desktop_app::DesktopMenuSection;
use super::desktop_launcher_service::{catalog_names, resolve_catalog_launch, ProgramCatalog};
use super::desktop_search_service::NativeSpotlightResult;
use super::desktop_settings_service::{
    load_settings_snapshot, persist_settings_draft,
    pty_force_render_mode as desktop_pty_force_render_mode,
    pty_profile_for_command as desktop_pty_profile_for_command, reload_settings_snapshot,
};
use super::desktop_start_menu::{StartLeaf, StartSubmenu};
use super::desktop_surface_service::DesktopSurfaceEntry;
use super::terminal_canvas::{PtyCanvas, TERMINAL_CELL_HEIGHT, TERMINAL_CELL_WIDTH};
use super::desktop_wm_widget::{DesktopWindowHost, WindowChild};
use super::message::{ContextMenuAction, DesktopIconId, Message, NavDirection};
use super::prompt::{TerminalPrompt, TerminalPromptAction, TerminalPromptKind};
use super::shared_types::DesktopWindow;
use crate::config::{
    base_dir, set_current_user, CliAcsMode, NativeStartupWindowMode, OpenMode, Settings,
    CUSTOM_THEME_NAME, HEADER_LINES, THEMES,
};
use crate::connections::macos_connections_disabled;
use crate::core::auth::{clear_session, UserRecord};
use crate::pty::{PtyLaunchOptions, PtySession};
use chrono::Local;
use crossterm::event::{KeyCode as PtyKeyCode, KeyModifiers as PtyKeyModifiers};
use iced::keyboard::{key::Named, Key, Modifiers};
use iced::widget::{canvas, text_input};
use iced::{Element, Subscription, Task, Theme};
use robcos_native_editor_app::EditorWindow;
use robcos_native_file_manager_app::{
    open_target_for_file_manager_action, FileManagerOpenTarget,
    NativeFileManagerState, OpenWithLaunchRequest,
};
use robcos_native_installer_app::{
    available_runtime_tools, runtime_tool_description, runtime_tool_pkg, runtime_tool_title,
    DesktopInstallerConfirm, DesktopInstallerNotice, DesktopInstallerState,
    DesktopInstallerView, DesktopInstallerEvent, InstallerMenuTarget,
    InstallerPackageAction,
};
use robcos_native_nuke_codes_app::{fetch_nuke_codes, NukeCodesView};
use robcos_native_programs_app::{
    build_desktop_applications_sections, resolve_desktop_applications_request,
    DesktopProgramRequest,
};
use robcos_native_services::desktop_default_apps_service::{
    binding_label_for_slot, default_app_slot_label, DefaultAppSlot,
};
use robcos_native_services::desktop_user_service::sorted_usernames;
use robcos_native_settings_app::{
    desktop_settings_back_target, desktop_settings_connections_nav_items,
    desktop_settings_default_panel, desktop_settings_home_rows,
    desktop_settings_user_management_nav_items, settings_panel_title, NativeSettingsPanel,
    SettingsHomeTileAction,
};
use robcos_native_terminal_app::{
    entry_for_selectable_idx, login_menu_rows_from_users, resolve_login_password_submission,
    resolve_desktop_pty_exit,
    resolve_login_selection_plan, resolve_main_menu_action, resolve_terminal_back_action,
    selectable_menu_count, terminal_runtime_defaults, terminal_screen_open_plan,
    terminal_settings_refresh_plan, terminal_shell_launch_plan,
    terminal_command_launch_plan, LoginMenuRow, MainMenuSelectionAction, TerminalBackAction,
    TerminalBackContext, TerminalDesktopPtyExitPlan, TerminalLoginPasswordPlan,
    TerminalLoginSelectionPlan, TerminalLoginState, TerminalLoginSubmitAction,
    TerminalNavigationState, TerminalPtyLaunchPlan, TerminalScreen,
    TerminalScreenOpenPlan, TerminalSelectionIndexTarget, TerminalShellSurface,
    MAIN_MENU_ENTRIES,
};
use robcos_native_services::desktop_session_service::{
    authenticate_login, bind_login_identity, clear_all_sessions, login_selection_auth_method,
    login_usernames,
};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

const TERMINAL_PROMPT_INPUT_ID: &str = "robcos-iced-terminal-prompt";
const DESKTOP_WINDOW_BORDER_WIDTH: f32 = 2.0;
const DESKTOP_WINDOW_TITLE_BAR_HEIGHT: f32 = 28.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TerminalMenuAction {
    OpenScreen(TerminalScreen),
    OpenDesktopWindow(DesktopWindow),
    OpenDesktopTerminalShell,
    Back,
    ShowStatus(&'static str),
}

#[derive(Debug, Clone, Copy)]
struct TerminalMenuEntry {
    label: &'static str,
    action: Option<TerminalMenuAction>,
}

const APPLICATION_MENU_ENTRIES: &[TerminalMenuEntry] = &[
    TerminalMenuEntry {
        label: "File Manager (desktop app)",
        action: Some(TerminalMenuAction::OpenDesktopWindow(
            DesktopWindow::FileManager,
        )),
    },
    TerminalMenuEntry {
        label: "Editor (desktop app)",
        action: Some(TerminalMenuAction::OpenDesktopWindow(DesktopWindow::Editor)),
    },
    TerminalMenuEntry {
        label: "Nuke Codes",
        action: Some(TerminalMenuAction::OpenScreen(TerminalScreen::NukeCodes)),
    },
    TerminalMenuEntry {
        label: "---",
        action: None,
    },
    TerminalMenuEntry {
        label: "Back",
        action: Some(TerminalMenuAction::Back),
    },
];

const DOCUMENT_MENU_ENTRIES: &[TerminalMenuEntry] = &[
    TerminalMenuEntry {
        label: "Logs",
        action: Some(TerminalMenuAction::OpenScreen(TerminalScreen::Logs)),
    },
    TerminalMenuEntry {
        label: "Document Browser",
        action: Some(TerminalMenuAction::OpenScreen(TerminalScreen::DocumentBrowser)),
    },
    TerminalMenuEntry {
        label: "---",
        action: None,
    },
    TerminalMenuEntry {
        label: "Back",
        action: Some(TerminalMenuAction::Back),
    },
];

const NETWORK_MENU_ENTRIES: &[TerminalMenuEntry] = &[
    TerminalMenuEntry {
        label: "CLI Terminal",
        action: Some(TerminalMenuAction::OpenDesktopTerminalShell),
    },
    TerminalMenuEntry {
        label: "Network Utilities",
        action: Some(TerminalMenuAction::ShowStatus(
            "Network program launchers are not wired in robcos-iced yet.",
        )),
    },
    TerminalMenuEntry {
        label: "---",
        action: None,
    },
    TerminalMenuEntry {
        label: "Back",
        action: Some(TerminalMenuAction::Back),
    },
];

const GAMES_MENU_ENTRIES: &[TerminalMenuEntry] = &[
    TerminalMenuEntry {
        label: "Donkey Kong",
        action: Some(TerminalMenuAction::OpenScreen(TerminalScreen::DonkeyKong)),
    },
    TerminalMenuEntry {
        label: "---",
        action: None,
    },
    TerminalMenuEntry {
        label: "Back",
        action: Some(TerminalMenuAction::Back),
    },
];

const SETTINGS_MENU_ENTRIES: &[TerminalMenuEntry] = &[
    TerminalMenuEntry {
        label: "Connections",
        action: Some(TerminalMenuAction::OpenScreen(TerminalScreen::Connections)),
    },
    TerminalMenuEntry {
        label: "Default Apps",
        action: Some(TerminalMenuAction::OpenScreen(TerminalScreen::DefaultApps)),
    },
    TerminalMenuEntry {
        label: "Edit Menus",
        action: Some(TerminalMenuAction::OpenScreen(TerminalScreen::EditMenus)),
    },
    TerminalMenuEntry {
        label: "User Management",
        action: Some(TerminalMenuAction::OpenScreen(TerminalScreen::UserManagement)),
    },
    TerminalMenuEntry {
        label: "About",
        action: Some(TerminalMenuAction::OpenScreen(TerminalScreen::About)),
    },
    TerminalMenuEntry {
        label: "---",
        action: None,
    },
    TerminalMenuEntry {
        label: "Back",
        action: Some(TerminalMenuAction::Back),
    },
];

const BACK_ONLY_MENU_ENTRIES: &[TerminalMenuEntry] = &[TerminalMenuEntry {
    label: "Back",
    action: Some(TerminalMenuAction::Back),
}];

fn terminal_menu_entries_for_screen(screen: TerminalScreen) -> Option<&'static [TerminalMenuEntry]> {
    match screen {
        TerminalScreen::MainMenu => None,
        TerminalScreen::Applications => Some(APPLICATION_MENU_ENTRIES),
        TerminalScreen::Documents => Some(DOCUMENT_MENU_ENTRIES),
        TerminalScreen::Network => Some(NETWORK_MENU_ENTRIES),
        TerminalScreen::Games => Some(GAMES_MENU_ENTRIES),
        TerminalScreen::Settings => Some(SETTINGS_MENU_ENTRIES),
        TerminalScreen::ProgramInstaller
        | TerminalScreen::Logs
        | TerminalScreen::DocumentBrowser
        | TerminalScreen::Connections
        | TerminalScreen::DefaultApps
        | TerminalScreen::EditMenus
        | TerminalScreen::About
        | TerminalScreen::UserManagement
        | TerminalScreen::DonkeyKong
        | TerminalScreen::NukeCodes
        | TerminalScreen::PtyApp => Some(BACK_ONLY_MENU_ENTRIES),
    }
}

fn terminal_menu_selectable_count(entries: &[TerminalMenuEntry]) -> usize {
    entries.iter().filter(|entry| entry.action.is_some()).count()
}

fn terminal_menu_entry_for_idx(entries: &[TerminalMenuEntry], idx: usize) -> Option<TerminalMenuEntry> {
    entries
        .iter()
        .copied()
        .filter(|entry| entry.action.is_some())
        .nth(idx)
}

fn terminal_back_selected_idx(current: TerminalScreen, target: TerminalScreen) -> usize {
    match (current, target) {
        (TerminalScreen::Applications, TerminalScreen::MainMenu) => 0,
        (TerminalScreen::Documents, TerminalScreen::MainMenu)
        | (TerminalScreen::Logs, TerminalScreen::MainMenu)
        | (TerminalScreen::DocumentBrowser, TerminalScreen::MainMenu) => 1,
        (TerminalScreen::Network, TerminalScreen::MainMenu) => 2,
        (TerminalScreen::Games, TerminalScreen::MainMenu)
        | (TerminalScreen::DonkeyKong, TerminalScreen::MainMenu) => 3,
        (TerminalScreen::ProgramInstaller, TerminalScreen::MainMenu) => 4,
        (TerminalScreen::PtyApp, TerminalScreen::MainMenu) => 5,
        (TerminalScreen::Settings, TerminalScreen::MainMenu)
        | (TerminalScreen::Connections, TerminalScreen::MainMenu)
        | (TerminalScreen::DefaultApps, TerminalScreen::MainMenu)
        | (TerminalScreen::EditMenus, TerminalScreen::MainMenu)
        | (TerminalScreen::About, TerminalScreen::MainMenu)
        | (TerminalScreen::UserManagement, TerminalScreen::MainMenu) => 7,
        (TerminalScreen::Logs, TerminalScreen::Documents) => 0,
        (TerminalScreen::DocumentBrowser, TerminalScreen::Documents) => 1,
        (TerminalScreen::DonkeyKong, TerminalScreen::Games) => 0,
        (TerminalScreen::NukeCodes, TerminalScreen::Applications) => 2,
        (TerminalScreen::Connections, TerminalScreen::Settings) => 0,
        (TerminalScreen::DefaultApps, TerminalScreen::Settings) => 1,
        (TerminalScreen::EditMenus, TerminalScreen::Settings) => 2,
        (TerminalScreen::UserManagement, TerminalScreen::Settings) => 3,
        (TerminalScreen::About, TerminalScreen::Settings) => 4,
        _ => 0,
    }
}

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

#[derive(Debug, Clone)]
pub struct StartMenuRenameState;

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

struct DesktopPtyState {
    title: String,
    completion_message: Option<String>,
    session: PtySession,
    cols_floor: u16,
    rows_floor: u16,
    live_resize: bool,
}

/// Top-level shell state for the iced-based implementation.
///
/// Replaces the legacy egui shell state. Fields are grouped by concern
/// into focused sub-state structs.
///
/// Instantiation and the iced `Application` impl arrive in Phase 2.
pub struct RobcoShell {
    // ── Desktop shell sub-states ────────────────────────────────────────────
    pub windows: WindowManager,
    pub spotlight: SpotlightState,
    pub start_menu: StartMenuState,
    pub surface: DesktopSurfaceState,

    // ── Mode ────────────────────────────────────────────────────────────────
    /// true = desktop mode, false = the full-screen RobCo terminal-mode UI
    pub desktop_mode: bool,

    // ── Terminal mode ───────────────────────────────────────────────────────
    pub terminal_nav: TerminalNavigationState,
    pub login: TerminalLoginState,
    pub login_rows: Vec<LoginMenuRow>,
    pub terminal_prompt: Option<TerminalPrompt>,

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
    /// iced text editor content — the live buffer for the Editor desktop window.
    pub editor_content: iced::widget::text_editor::Content,
    pub settings_panel: Option<NativeSettingsPanel>,
    pub applications_status: String,
    pub installer: DesktopInstallerState,
    pub installer_notice: Option<DesktopInstallerNotice>,
    pub installer_runtime_status: HashMap<String, bool>,
    pub nuke_codes: NukeCodesView,
    desktop_pty: Option<DesktopPtyState>,

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
        let login_rows = login_menu_rows_from_users(login_usernames());
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
        let mut installer = DesktopInstallerState::default();
        installer.ensure_available_pms();

        let shell = Self {
            windows,
            spotlight: SpotlightState::default(),
            start_menu: StartMenuState::default(),
            surface: DesktopSurfaceState::default(),
            desktop_mode: false,
            terminal_nav: terminal_runtime_defaults(),
            login: TerminalLoginState::default(),
            login_rows,
            terminal_prompt: None,
            session_username: None,
            session_is_admin: false,
            file_manager: NativeFileManagerState::new(home),
            editor: EditorWindow::default(),
            editor_content: iced::widget::text_editor::Content::new(),
            settings_panel: None,
            applications_status: String::new(),
            installer,
            installer_notice: None,
            installer_runtime_status: HashMap::new(),
            nuke_codes: NukeCodesView::Unloaded,
            desktop_pty: None,
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
                if self.desktop_mode {
                    self.start_menu.close();
                    self.spotlight.reset();
                }
            }
            Message::CloseSpotlight => {
                if self.desktop_mode {
                    self.spotlight.close();
                    self.start_menu.close();
                } else if self.terminal_prompt.is_some() {
                    self.cancel_terminal_prompt();
                } else {
                    self.handle_terminal_back();
                }
            }
            Message::SpotlightQueryChanged(q) => {
                self.spotlight.query = q;
                self.spotlight.selected = 0;
            }
            Message::SpotlightTabChanged(t) => {
                self.spotlight.set_tab(t);
            }
            Message::SpotlightNavigate(dir) => {
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
            Message::GlobalKeyPressed(key, mods) => {
                return self.handle_global_key_press(key, mods);
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
                use super::desktop_start_menu::{
                    start_root_action_for_idx, start_root_leaf_for_idx, start_root_submenu_for_idx,
                    StartRootAction,
                };
                self.start_menu.selected_root = idx;
                self.start_menu.open_leaf = start_root_leaf_for_idx(idx);
                self.start_menu.open_submenu = start_root_submenu_for_idx(idx);
                self.start_menu.leaf_selected = 0;
                self.start_menu.system_selected = 0;
                if let Some(action) = start_root_action_for_idx(idx) {
                    self.start_menu.close();
                    return match action {
                        StartRootAction::ReturnToTerminal => self.update(Message::DesktopModeToggled),
                        StartRootAction::Logout => self.update(Message::LogoutRequested),
                        StartRootAction::Shutdown => {
                            self.shell_status =
                                "Shutdown is not wired in robcos-iced yet.".to_string();
                            Task::none()
                        }
                    };
                }
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
                match w {
                    DesktopWindow::PtyApp => {
                        if self.desktop_pty.is_none() {
                            return self.open_desktop_terminal_shell();
                        }
                    }
                    DesktopWindow::NukeCodes => {
                        if matches!(self.nuke_codes, NukeCodesView::Unloaded) {
                            self.nuke_codes = fetch_nuke_codes();
                        }
                    }
                    DesktopWindow::Installer => {
                        self.installer.open = true;
                        self.installer.ensure_available_pms();
                        self.refresh_installer_runtime_cache();
                    }
                    DesktopWindow::Settings => {
                        self.settings = load_settings_snapshot();
                    }
                    _ => {}
                }
                if !self.windows.is_open(w) {
                    // Default size/position — will come from WindowManager in Phase 3
                    self.windows.open(w, WindowRect::new(100.0, 60.0, 800.0, 560.0), (400.0, 300.0), true);
                } else {
                    self.windows.bring_to_front(w);
                }
            }
            Message::CloseWindow(w) => {
                if matches!(w, DesktopWindow::PtyApp) {
                    self.close_desktop_pty();
                }
                if matches!(w, DesktopWindow::Installer) {
                    self.installer.open = false;
                }
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
                    WindowHeaderButton::Close => {
                        if matches!(window, DesktopWindow::PtyApp) {
                            self.close_desktop_pty();
                        }
                        if matches!(window, DesktopWindow::Installer) {
                            self.installer.open = false;
                        }
                        self.windows.close(window);
                    }
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
                    DesktopShellAction::OpenTextEditor => {
                        return self.update(Message::OpenWindow(DesktopWindow::Editor));
                    }
                    DesktopShellAction::OpenNukeCodes => {
                        return self.update(Message::OpenWindow(DesktopWindow::NukeCodes));
                    }
                    DesktopShellAction::OpenDesktopTerminalShell => {
                        return self.open_desktop_terminal_shell();
                    }
                    DesktopShellAction::OpenConnectionsSettings => {
                        self.settings_panel = Some(NativeSettingsPanel::Connections);
                        return self.update(Message::OpenWindow(DesktopWindow::Settings));
                    }
                    DesktopShellAction::LaunchConfiguredApp(name) => {
                        return self.launch_catalog_program(&name, ProgramCatalog::Applications);
                    }
                    DesktopShellAction::LaunchNetworkProgram(name) => {
                        return self.launch_catalog_program(&name, ProgramCatalog::Network);
                    }
                    DesktopShellAction::LaunchGameProgram(name) => {
                        return self.launch_catalog_program(&name, ProgramCatalog::Games);
                    }
                    DesktopShellAction::OpenFileManagerAt(path) => {
                        self.file_manager.set_cwd(path);
                        return self.update(Message::OpenWindow(DesktopWindow::FileManager));
                    }
                    DesktopShellAction::OpenPathInEditor(path) => {
                        self.open_editor_path(path);
                        return self.update(Message::OpenWindow(DesktopWindow::Editor));
                    }
                    DesktopShellAction::RevealPathInFileManager(path) => {
                        if let Some(parent) = path.parent() {
                            self.file_manager.set_cwd(parent.to_path_buf());
                            self.file_manager.select(Some(path));
                        }
                        return self.update(Message::OpenWindow(DesktopWindow::FileManager));
                    }
                }
            }
            Message::DesktopModeToggled => {
                self.desktop_mode = !self.desktop_mode;
                self.start_menu.close();
                self.spotlight.close();
                self.shell_status = if self.desktop_mode {
                    "Entered Desktop Mode.".to_string()
                } else {
                    "Returned to terminal mode.".to_string()
                };
            }

            // ── Session ──────────────────────────────────────────────────────
            Message::LoginUsernameSelected(username) => {
                self.login.selected_username = username.clone();
                let mut selectable_idx = 0usize;
                for row in &self.login_rows {
                    match row {
                        LoginMenuRow::User(user) if *user == username => {
                            self.login.selected_idx = selectable_idx;
                            break;
                        }
                        LoginMenuRow::User(_) | LoginMenuRow::Exit => {
                            selectable_idx += 1;
                        }
                        LoginMenuRow::Separator => {}
                    }
                }
            }
            Message::LoginPasswordChanged(password) => {
                self.login.password = password.clone();
                if let Some(prompt) = self.terminal_prompt.as_mut() {
                    if matches!(prompt.action, TerminalPromptAction::LoginPassword) {
                        prompt.buffer = password;
                    }
                }
            }
            Message::LoginSubmitted => {
                return self.submit_login_prompt();
            }
            Message::LogoutRequested => {
                self.handle_logout();
            }

            // ── Terminal mode ───────────────────────────────────────────────
            Message::TerminalNavigate(dir) => {
                if !self.desktop_mode {
                    self.move_terminal_selection(dir);
                }
            }
            Message::TerminalActivateSelected => {
                if !self.desktop_mode {
                    return self.activate_selected_terminal_item();
                }
            }
            Message::TerminalBackRequested => {
                if !self.desktop_mode {
                    if self.terminal_prompt.is_some() {
                        self.cancel_terminal_prompt();
                    } else {
                        self.handle_terminal_back();
                    }
                }
            }
            Message::TerminalPromptCancelled => {
                if !self.desktop_mode {
                    self.cancel_terminal_prompt();
                }
            }
            Message::TerminalSelectionActivated(idx) => {
                if !self.desktop_mode {
                    return self.activate_terminal_selection(idx);
                }
            }

            // ── Desktop surface ──────────────────────────────────────────────
            Message::DesktopIconClicked { id, .. } => {
                self.surface.selected_icon = Some(id);
            }
            Message::DesktopIconDoubleClicked(id) => {
                use super::desktop_surface_service::{desktop_builtin_icons, DesktopBuiltinIconKind};
                use super::message::DesktopIconId;
                self.surface.selected_icon = None;
                if let DesktopIconId::Builtin(key) = &id {
                    if let Some(entry) = desktop_builtin_icons().iter().find(|e| e.key == *key) {
                        if let Some(w) = entry.target_window {
                            return self.update(Message::OpenWindow(w));
                        }
                        // Terminal mode has no target_window — toggle desktop mode.
                        if entry.kind == DesktopBuiltinIconKind::Terminal {
                            return self.update(Message::DesktopModeToggled);
                        }
                    }
                }
            }
            Message::DesktopSelectionCleared => {
                self.surface.selected_icon = None;
            }

            // ── System ───────────────────────────────────────────────────────
            Message::Tick(_) => {
                self.clock = Local::now().format("%H:%M").to_string();
                let _ = self.installer.poll_search();
                self.sync_desktop_pty();
            }
            Message::PersistSnapshotRequested => {
                // Phase 3: call persist_native_shell_snapshot()
            }

            Message::TextEditorAction(action) => {
                self.editor_content.perform(action);
                self.editor.text = editor_content_to_string(&self.editor_content);
                self.editor.dirty = true;
            }

            Message::SettingsPanelChanged(panel) => {
                self.settings_panel = Some(panel);
            }
            Message::SettingsThemeChanged(theme_name) => {
                self.persist_settings_change(|settings| {
                    settings.theme = theme_name.clone();
                });
            }
            Message::SettingsCustomThemeAdjusted { channel, delta } => {
                self.persist_settings_change(|settings| {
                    if settings.theme != CUSTOM_THEME_NAME {
                        settings.theme = CUSTOM_THEME_NAME.to_string();
                    }
                    if let Some(value) = settings.custom_theme_rgb.get_mut(channel) {
                        adjust_u8_clamped(value, delta);
                    }
                });
            }
            Message::SettingsOpenModeChanged(mode) => {
                self.persist_settings_change(|settings| {
                    settings.default_open_mode = mode;
                });
            }
            Message::SettingsWindowModeChanged(mode) => {
                self.persist_settings_change(|settings| {
                    settings.native_startup_window_mode = mode;
                });
            }
            Message::SettingsCliAcsModeChanged(mode) => {
                self.persist_settings_change(|settings| {
                    settings.cli_acs_mode = mode;
                });
            }
            Message::SettingsSoundToggled => {
                self.persist_settings_change(|settings| {
                    settings.sound = !settings.sound;
                });
            }
            Message::SettingsBootupToggled => {
                self.persist_settings_change(|settings| {
                    settings.bootup = !settings.bootup;
                });
            }
            Message::SettingsNavigationHintsToggled => {
                self.persist_settings_change(|settings| {
                    settings.show_navigation_hints = !settings.show_navigation_hints;
                });
            }
            Message::SettingsSystemSoundVolumeAdjusted(delta) => {
                self.persist_settings_change(|settings| {
                    adjust_u8_clamped(&mut settings.system_sound_volume, delta);
                });
            }
            Message::SettingsBuiltinMenuVisibilityToggled { text_editor } => {
                self.persist_settings_change(|settings| {
                    let visibility = &mut settings.builtin_menu_visibility;
                    if text_editor {
                        visibility.text_editor = !visibility.text_editor;
                    } else {
                        visibility.nuke_codes = !visibility.nuke_codes;
                    }
                });
            }
            Message::SettingsSaveRequested => {
                self.settings = persist_settings_draft(&self.settings);
                self.file_manager.refresh_contents();
                self.shell_status = "Settings saved.".to_string();
            }
            Message::SettingsCancelRequested => {
                self.settings = reload_settings_snapshot();
                self.file_manager.refresh_contents();
                self.shell_status = "Reloaded settings from disk.".to_string();
            }

            Message::FileManagerCommand(cmd) => {
                use robcos_native_file_manager_app::FileManagerCommand;
                match cmd {
                    FileManagerCommand::GoUp => {
                        self.file_manager.up();
                    }
                    FileManagerCommand::OpenSelected => {
                        let settings = load_settings_snapshot();
                        match open_target_for_file_manager_action(
                            self.file_manager.activate_selected(),
                            &settings.desktop_file_manager,
                        ) {
                            Ok(target) => return self.handle_file_manager_open_target(target),
                            Err(status) => self.shell_status = status,
                        }
                    }
                    FileManagerCommand::ToggleHiddenFiles => {
                        // hidden files toggle is tracked in settings; handled in Phase 4
                    }
                    _ => {}
                }
            }
            Message::FileManagerRowPressed(path) => {
                if self.file_manager.selected.as_ref() == Some(&path) {
                    let settings = load_settings_snapshot();
                    match open_target_for_file_manager_action(
                        self.file_manager.activate_selected(),
                        &settings.desktop_file_manager,
                    ) {
                        Ok(target) => return self.handle_file_manager_open_target(target),
                        Err(status) => self.shell_status = status,
                    }
                } else {
                    self.file_manager.select(Some(path));
                }
            }
            Message::FileManagerSearchChanged(query) => {
                self.file_manager.update_search_query(query);
            }
            Message::NukeCodesRefreshRequested => {
                self.nuke_codes = fetch_nuke_codes();
            }
            Message::InstallerSearchQueryChanged(query) => {
                self.installer.search_query = query;
            }
            Message::InstallerInstalledFilterChanged(filter) => {
                self.installer.installed_filter = filter;
            }
            Message::InstallerPackageManagerSelected(idx) => {
                if self.installer.select_package_manager(idx) {
                    self.refresh_installer_runtime_cache();
                }
            }
            Message::InstallerSearchRequested => {
                self.installer.do_search();
            }
            Message::InstallerInstalledRequested => {
                self.installer.load_installed();
            }
            Message::InstallerRuntimeToolsRequested => {
                self.refresh_installer_runtime_cache();
                self.installer.view = DesktopInstallerView::RuntimeTools;
            }
            Message::InstallerBackRequested => {
                self.installer.go_back();
            }
            Message::InstallerOpenPackageActions { pkg, installed } => {
                let _ = self.installer.fetch_package_description(&pkg);
                self.installer.view = DesktopInstallerView::PackageActions { pkg, installed };
            }
            Message::InstallerRunPackageAction(action) => {
                if let DesktopInstallerView::PackageActions { pkg, .. } = self.installer.view.clone()
                {
                    self.installer.confirm_dialog = Some(DesktopInstallerConfirm { pkg, action });
                }
            }
            Message::InstallerConfirmAccepted => {
                match self.installer.confirm_action() {
                    DesktopInstallerEvent::LaunchCommand {
                        argv,
                        status: _,
                        completion_message,
                    } => {
                        let plan = terminal_command_launch_plan(
                            TerminalShellSurface::Desktop,
                            "Program Installer",
                            &argv,
                            TerminalScreen::MainMenu,
                            desktop_pty_force_render_mode(&argv),
                        );
                        return self.launch_desktop_pty_plan(plan, completion_message);
                    }
                    DesktopInstallerEvent::None => {}
                }
            }
            Message::InstallerConfirmCancelled => {
                self.installer.confirm_dialog = None;
            }
            Message::InstallerOpenAddToMenu(pkg) => {
                self.installer.display_name_input.clear();
                self.installer.view = DesktopInstallerView::AddToMenu { pkg };
            }
            Message::InstallerDisplayNameChanged(value) => {
                self.installer.display_name_input = value;
            }
            Message::InstallerAddToMenu { pkg, target } => {
                self.installer.add_to_menu(&pkg, target);
            }
            Message::InstallerNoticeDismissed => {
                self.installer_notice = None;
            }

            // All other variants are stubs for Phase 3+
            _ => {}
        }
        Task::none()
    }

    fn terminal_prompt_id() -> text_input::Id {
        text_input::Id::new(TERMINAL_PROMPT_INPUT_ID)
    }

    fn focus_terminal_prompt() -> Task<Message> {
        let id = Self::terminal_prompt_id();
        Task::batch([text_input::focus(id.clone()), text_input::move_cursor_to_end(id)])
    }

    fn refresh_login_rows(&mut self) {
        self.login_rows = login_menu_rows_from_users(login_usernames());
        let selectable_count = self
            .login_rows
            .iter()
            .filter(|row| matches!(row, LoginMenuRow::User(_) | LoginMenuRow::Exit))
            .count();
        self.login.selected_idx = self
            .login
            .selected_idx
            .min(selectable_count.saturating_sub(1));
    }

    fn open_password_prompt(
        &mut self,
        title: impl Into<String>,
        prompt: impl Into<String>,
    ) -> Task<Message> {
        self.terminal_prompt = Some(TerminalPrompt {
            kind: TerminalPromptKind::Password,
            title: title.into(),
            prompt: prompt.into(),
            buffer: String::new(),
            confirm_yes: true,
            action: TerminalPromptAction::LoginPassword,
        });
        self.login.password.clear();
        Self::focus_terminal_prompt()
    }

    fn cancel_terminal_prompt(&mut self) {
        let clear_login_state = self
            .terminal_prompt
            .as_ref()
            .is_some_and(|prompt| matches!(prompt.action, TerminalPromptAction::LoginPassword));
        self.terminal_prompt = None;
        if clear_login_state {
            self.login.password.clear();
            self.login.error.clear();
        }
    }

    fn apply_terminal_screen_open_plan(&mut self, plan: TerminalScreenOpenPlan) {
        self.terminal_nav.screen = plan.screen;
        if plan.clear_settings_choice {
            self.terminal_nav.settings_choice = None;
        }
        if plan.clear_default_app_slot {
            self.terminal_nav.default_app_slot = None;
        }
        if plan.reset_user_management_to_root {
            self.terminal_nav.user_management_mode =
                robcos_native_terminal_app::UserManagementMode::Root;
        }
        match plan.index_target {
            TerminalSelectionIndexTarget::None => {}
            TerminalSelectionIndexTarget::MainMenu => {
                self.terminal_nav.main_menu_idx = plan.selected_idx;
            }
            TerminalSelectionIndexTarget::Applications => {
                self.terminal_nav.apps_idx = plan.selected_idx;
            }
            TerminalSelectionIndexTarget::Documents => {
                self.terminal_nav.documents_idx = plan.selected_idx;
            }
            TerminalSelectionIndexTarget::Logs => {
                self.terminal_nav.logs_idx = plan.selected_idx;
            }
            TerminalSelectionIndexTarget::Network => {
                self.terminal_nav.network_idx = plan.selected_idx;
            }
            TerminalSelectionIndexTarget::Games => {
                self.terminal_nav.games_idx = plan.selected_idx;
            }
            TerminalSelectionIndexTarget::Settings => {
                self.terminal_nav.settings_idx = plan.selected_idx;
            }
            TerminalSelectionIndexTarget::DefaultApps => {
                self.terminal_nav.default_apps_idx = plan.selected_idx;
            }
            TerminalSelectionIndexTarget::UserManagement => {
                self.terminal_nav.user_management_idx = plan.selected_idx;
            }
            TerminalSelectionIndexTarget::DocumentBrowser => {
                self.terminal_nav.browser_idx = plan.selected_idx;
            }
            TerminalSelectionIndexTarget::ProgramInstallerRoot
            | TerminalSelectionIndexTarget::ConnectionsRoot => {}
        }
        if plan.clear_status {
            self.shell_status.clear();
        }
    }

    fn apply_main_menu_selection_action(
        &mut self,
        action: MainMenuSelectionAction,
    ) -> Task<Message> {
        match action {
            MainMenuSelectionAction::OpenScreen {
                screen,
                selected_idx,
                clear_status,
            } => {
                self.apply_terminal_screen_open_plan(terminal_screen_open_plan(
                    screen,
                    selected_idx,
                    clear_status,
                ));
                Task::none()
            }
            MainMenuSelectionAction::OpenTerminalMode => {
                self.desktop_mode = true;
                self.start_menu.close();
                self.spotlight.close();
                self.open_desktop_terminal_shell()
            }
            MainMenuSelectionAction::EnterDesktopMode => self.update(Message::DesktopModeToggled),
            MainMenuSelectionAction::RefreshSettingsAndOpen => {
                self.settings = load_settings_snapshot();
                self.apply_terminal_screen_open_plan(terminal_settings_refresh_plan());
                Task::none()
            }
            MainMenuSelectionAction::BeginLogout => self.update(Message::LogoutRequested),
        }
    }

    fn apply_terminal_menu_action(&mut self, action: TerminalMenuAction) -> Task<Message> {
        match action {
            TerminalMenuAction::OpenScreen(screen) => {
                if matches!(screen, TerminalScreen::DocumentBrowser) {
                    self.terminal_nav.browser_return_screen = TerminalScreen::Documents;
                }
                if matches!(screen, TerminalScreen::NukeCodes) {
                    self.terminal_nav.nuke_codes_return_screen = TerminalScreen::Applications;
                }
                self.apply_terminal_screen_open_plan(terminal_screen_open_plan(screen, 0, true));
                Task::none()
            }
            TerminalMenuAction::OpenDesktopWindow(window) => {
                self.desktop_mode = true;
                self.update(Message::OpenWindow(window))
            }
            TerminalMenuAction::OpenDesktopTerminalShell => {
                self.desktop_mode = true;
                self.open_desktop_terminal_shell()
            }
            TerminalMenuAction::Back => {
                self.handle_terminal_back();
                Task::none()
            }
            TerminalMenuAction::ShowStatus(status) => {
                self.shell_status = status.to_string();
                Task::none()
            }
        }
    }

    fn apply_terminal_login_selection_plan(
        &mut self,
        plan: TerminalLoginSelectionPlan<UserRecord>,
    ) -> Task<Message> {
        self.login.error.clear();
        match plan {
            TerminalLoginSelectionPlan::Exit => {
                self.shell_status = "Close the window to exit RobCoOS.".to_string();
                Task::none()
            }
            TerminalLoginSelectionPlan::PromptPassword { username, prompt } => {
                self.login.selected_username = username;
                self.login.clear_password_and_error();
                self.open_password_prompt(prompt.title, prompt.prompt)
            }
            TerminalLoginSelectionPlan::Submit {
                action,
                missing_username_is_select_user,
            } => self.apply_terminal_login_submit_action(action, missing_username_is_select_user),
            TerminalLoginSelectionPlan::StartHacking { username } => {
                self.login.selected_username = username;
                self.login.error =
                    "Hacking login is not implemented in robcos-iced yet.".to_string();
                Task::none()
            }
            TerminalLoginSelectionPlan::ShowError(error) => {
                self.login.error = error;
                Task::none()
            }
        }
    }

    fn apply_terminal_login_password_plan(
        &mut self,
        plan: TerminalLoginPasswordPlan<UserRecord>,
    ) -> Task<Message> {
        let follow_up = self.apply_terminal_login_submit_action(plan.action, true);
        if let Some(prompt) = plan.reopen_prompt {
            self.open_password_prompt(prompt.title, prompt.prompt)
        } else {
            follow_up
        }
    }

    fn apply_terminal_login_submit_action(
        &mut self,
        action: TerminalLoginSubmitAction<UserRecord>,
        missing_username_is_select_user: bool,
    ) -> Task<Message> {
        self.login.error.clear();
        match action {
            TerminalLoginSubmitAction::MissingUsername => {
                self.login.error = if missing_username_is_select_user {
                    "Select a user.".to_string()
                } else {
                    "Username cannot be empty.".to_string()
                };
                Task::none()
            }
            TerminalLoginSubmitAction::Authenticated { username, user } => {
                bind_login_identity(&username);
                self.settings = load_settings_snapshot();
                self.session_username = Some(username.clone());
                self.session_is_admin = user.is_admin;
                self.desktop_mode = false;
                self.terminal_prompt = None;
                self.login.selected_username = username.clone();
                self.login.clear_password_and_error();
                self.apply_terminal_screen_open_plan(terminal_screen_open_plan(
                    TerminalScreen::MainMenu,
                    0,
                    true,
                ));
                self.shell_status = format!("Logged in as {username}.");
                Task::none()
            }
            TerminalLoginSubmitAction::ShowError(error) => {
                self.login.error = error;
                Task::none()
            }
        }
    }

    fn handle_logout(&mut self) {
        clear_all_sessions();
        clear_session();
        set_current_user(None);
        self.settings = load_settings_snapshot();
        self.session_username = None;
        self.session_is_admin = false;
        self.desktop_mode = false;
        self.spotlight.close();
        self.start_menu.close();
        self.terminal_prompt = None;
        self.terminal_nav = terminal_runtime_defaults();
        self.login.reset();
        self.refresh_login_rows();
        self.shell_status = "Logged out.".to_string();
    }

    fn submit_login_prompt(&mut self) -> Task<Message> {
        let Some(prompt) = self.terminal_prompt.take() else {
            return Task::none();
        };
        if !matches!(prompt.action, TerminalPromptAction::LoginPassword) {
            return Task::none();
        }
        self.login.password = prompt.buffer;
        let plan = resolve_login_password_submission(
            &self.login.selected_username,
            &self.login.password,
            self.session_username.is_some(),
            false,
            authenticate_login,
        );
        self.apply_terminal_login_password_plan(plan)
    }

    fn current_terminal_selectable_count(&self) -> usize {
        if self.session_username.is_none() {
            return self
                .login_rows
                .iter()
                .filter(|row| matches!(row, LoginMenuRow::User(_) | LoginMenuRow::Exit))
                .count();
        }
        if matches!(self.terminal_nav.screen, TerminalScreen::MainMenu) {
            return selectable_menu_count();
        }
        terminal_menu_entries_for_screen(self.terminal_nav.screen)
            .map(terminal_menu_selectable_count)
            .unwrap_or(0)
    }

    fn current_terminal_selected_idx(&self) -> usize {
        if self.session_username.is_none() {
            return self.login.selected_idx;
        }
        match self.terminal_nav.screen {
            TerminalScreen::MainMenu => self.terminal_nav.main_menu_idx,
            TerminalScreen::Applications => self.terminal_nav.apps_idx,
            TerminalScreen::Documents => self.terminal_nav.documents_idx,
            TerminalScreen::Logs => self.terminal_nav.logs_idx,
            TerminalScreen::Network => self.terminal_nav.network_idx,
            TerminalScreen::Games => self.terminal_nav.games_idx,
            TerminalScreen::Settings => self.terminal_nav.settings_idx,
            TerminalScreen::DefaultApps => self.terminal_nav.default_apps_idx,
            TerminalScreen::DocumentBrowser => self.terminal_nav.browser_idx,
            TerminalScreen::UserManagement => self.terminal_nav.user_management_idx,
            TerminalScreen::ProgramInstaller
            | TerminalScreen::Connections
            | TerminalScreen::EditMenus
            | TerminalScreen::About
            | TerminalScreen::DonkeyKong
            | TerminalScreen::NukeCodes
            | TerminalScreen::PtyApp => 0,
        }
    }

    fn set_current_terminal_selected_idx(&mut self, idx: usize) {
        if self.session_username.is_none() {
            self.login.selected_idx = idx;
            return;
        }
        match self.terminal_nav.screen {
            TerminalScreen::MainMenu => self.terminal_nav.main_menu_idx = idx,
            TerminalScreen::Applications => self.terminal_nav.apps_idx = idx,
            TerminalScreen::Documents => self.terminal_nav.documents_idx = idx,
            TerminalScreen::Logs => self.terminal_nav.logs_idx = idx,
            TerminalScreen::Network => self.terminal_nav.network_idx = idx,
            TerminalScreen::Games => self.terminal_nav.games_idx = idx,
            TerminalScreen::Settings => self.terminal_nav.settings_idx = idx,
            TerminalScreen::DefaultApps => self.terminal_nav.default_apps_idx = idx,
            TerminalScreen::DocumentBrowser => self.terminal_nav.browser_idx = idx,
            TerminalScreen::UserManagement => self.terminal_nav.user_management_idx = idx,
            TerminalScreen::ProgramInstaller
            | TerminalScreen::Connections
            | TerminalScreen::EditMenus
            | TerminalScreen::About
            | TerminalScreen::DonkeyKong
            | TerminalScreen::NukeCodes
            | TerminalScreen::PtyApp => {}
        }
    }

    fn move_terminal_selection(&mut self, dir: NavDirection) {
        if self.terminal_prompt.is_some() {
            return;
        }
        let selectable_count = self.current_terminal_selectable_count();
        if selectable_count == 0 {
            return;
        }
        let mut selected_idx = self.current_terminal_selected_idx();
        match dir {
            NavDirection::Up => {
                selected_idx = selected_idx.saturating_sub(1);
            }
            NavDirection::Down => {
                selected_idx = (selected_idx + 1).min(selectable_count.saturating_sub(1));
            }
            _ => return,
        }
        self.set_current_terminal_selected_idx(selected_idx);
    }

    fn activate_selected_terminal_item(&mut self) -> Task<Message> {
        self.activate_terminal_selection(self.current_terminal_selected_idx())
    }

    fn activate_terminal_selection(&mut self, idx: usize) -> Task<Message> {
        if self.terminal_prompt.is_some() {
            return Task::none();
        }
        let selectable_count = self.current_terminal_selectable_count();
        if selectable_count == 0 {
            return Task::none();
        }
        let idx = idx.min(selectable_count.saturating_sub(1));
        self.set_current_terminal_selected_idx(idx);

        if self.session_username.is_none() {
            self.refresh_login_rows();
            self.login.selected_idx = idx;
            let usernames: Vec<String> = self
                .login_rows
                .iter()
                .filter_map(|row| match row {
                    LoginMenuRow::User(user) => Some(user.clone()),
                    LoginMenuRow::Separator | LoginMenuRow::Exit => None,
                })
                .collect();
            let plan = resolve_login_selection_plan(
                idx,
                &usernames,
                login_selection_auth_method,
                |username| authenticate_login(username, ""),
            );
            return self.apply_terminal_login_selection_plan(plan);
        }

        if matches!(self.terminal_nav.screen, TerminalScreen::MainMenu) {
            if let Some(action) = entry_for_selectable_idx(idx).action {
                return self.apply_main_menu_selection_action(resolve_main_menu_action(action));
            }
            return Task::none();
        }

        let Some(entries) = terminal_menu_entries_for_screen(self.terminal_nav.screen) else {
            return Task::none();
        };
        let Some(entry) = terminal_menu_entry_for_idx(entries, idx) else {
            return Task::none();
        };
        let Some(action) = entry.action else {
            return Task::none();
        };
        self.apply_terminal_menu_action(action)
    }

    fn handle_terminal_back(&mut self) {
        let current_screen = self.terminal_nav.screen;
        let action = resolve_terminal_back_action(TerminalBackContext {
            screen: current_screen,
            has_settings_choice: self.terminal_nav.settings_choice.is_some(),
            has_default_app_slot: self.terminal_nav.default_app_slot.is_some(),
            connections_at_root: true,
            installer_at_root: true,
            has_embedded_pty: false,
            pty_return_screen: TerminalScreen::MainMenu,
            nuke_codes_return_screen: self.terminal_nav.nuke_codes_return_screen,
            browser_return_screen: self.terminal_nav.browser_return_screen,
        });

        match action {
            TerminalBackAction::NoOp => {}
            TerminalBackAction::ClearSettingsChoice => {
                self.terminal_nav.settings_choice = None;
            }
            TerminalBackAction::ClearDefaultAppSlot => {
                self.terminal_nav.default_app_slot = None;
            }
            TerminalBackAction::UseConnectionsInnerBack
            | TerminalBackAction::UseInstallerInnerBack => {}
            TerminalBackAction::NavigateTo {
                screen,
                clear_status,
                reset_installer: _,
            } => {
                self.apply_terminal_screen_open_plan(terminal_screen_open_plan(
                    screen,
                    terminal_back_selected_idx(current_screen, screen),
                    clear_status,
                ));
            }
            TerminalBackAction::ClosePtyAndReturn { return_screen } => {
                self.apply_terminal_screen_open_plan(terminal_screen_open_plan(
                    return_screen,
                    terminal_back_selected_idx(current_screen, return_screen),
                    true,
                ));
            }
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        use iced::widget::{column, stack};

        if !self.desktop_mode {
            return self.view_terminal_mode();
        }

        let shell_ui: Element<'_, Message> = column![
            self.view_top_bar(),
            self.view_desktop(),
            self.view_taskbar(),
        ]
        .into();

        if self.spotlight.open {
            stack![shell_ui, self.view_spotlight()].into()
        } else if self.start_menu.open {
            stack![shell_ui, self.view_start_menu()].into()
        } else {
            shell_ui
        }
    }

    fn view_terminal_mode(&self) -> Element<'_, Message> {
        use iced::widget::stack;

        let base = if self.session_username.is_none() {
            self.view_terminal_login_screen()
        } else {
            match self.terminal_nav.screen {
                TerminalScreen::MainMenu => self.view_terminal_main_menu(),
                screen => self.view_terminal_standard_screen(screen),
            }
        };

        if self.terminal_prompt.is_some() {
            stack![base, self.view_terminal_prompt_overlay()].into()
        } else {
            base
        }
    }

    fn view_terminal_login_screen(&self) -> Element<'_, Message> {
        let error_color = iced::Color::from_rgb8(255, 128, 128);
        let (footer, footer_color) = if !self.login.error.is_empty() {
            (self.login.error.clone(), error_color)
        } else if !self.shell_status.is_empty() {
            (
                self.shell_status.clone(),
                super::retro_theme::current_retro_colors().dim.to_iced(),
            )
        } else {
            (
                "Arrow keys move | Enter select".to_string(),
                super::retro_theme::current_retro_colors().dim.to_iced(),
            )
        };

        self.view_terminal_frame(
            "ROBCO TERMLINK - Select User".to_string(),
            Some("Welcome. Please select a user.".to_string()),
            self.view_terminal_login_rows(),
            footer,
            footer_color,
        )
    }

    fn view_terminal_main_menu(&self) -> Element<'_, Message> {
        let user_label = self
            .session_username
            .as_ref()
            .map(|username| {
                format!(
                    "User: {username}{}    RobCoOS {}",
                    if self.session_is_admin { " [ADMIN]" } else { "" },
                    env!("CARGO_PKG_VERSION")
                )
            })
            .unwrap_or_else(|| format!("RobCoOS {}", env!("CARGO_PKG_VERSION")));
        let footer = if self.shell_status.is_empty() {
            "Arrow keys move | Enter select | Esc back".to_string()
        } else {
            self.shell_status.clone()
        };
        self.view_terminal_frame(
            "Main Menu".to_string(),
            Some(user_label),
            self.view_terminal_main_menu_entries(),
            footer,
            super::retro_theme::current_retro_colors().dim.to_iced(),
        )
    }

    fn view_terminal_standard_screen(&self, screen: TerminalScreen) -> Element<'_, Message> {
        use iced::widget::{column, container, text};
        use iced::Length;

        let note = match screen {
            TerminalScreen::Applications => {
                Some("Terminal mode UI is active. Hosted desktop apps remain separate and land in Phase 3h.")
            }
            TerminalScreen::Documents => Some("Document and log browsers will be wired into these routes next."),
            TerminalScreen::Network => {
                Some("This is terminal mode UI, not the actual CLI terminal app.")
            }
            TerminalScreen::Games => Some("Game launch routes exist here; interactive ports follow later."),
            TerminalScreen::ProgramInstaller => {
                Some("Program Installer UI is reserved here; the real hosted app path lands later.")
            }
            TerminalScreen::Settings => {
                Some("Settings sub-routes are wired into terminal mode navigation.")
            }
            TerminalScreen::Connections => {
                Some("Connections settings screen is not rendered in iced yet.")
            }
            TerminalScreen::DefaultApps => {
                Some("Default app editing is not rendered in iced yet.")
            }
            TerminalScreen::EditMenus => {
                Some("Menu editing is not rendered in iced yet.")
            }
            TerminalScreen::About => Some("About screen is not rendered in iced yet."),
            TerminalScreen::UserManagement => {
                Some("User management UI is not rendered in iced yet.")
            }
            TerminalScreen::Logs => Some("Logs browser is not rendered in iced yet."),
            TerminalScreen::DocumentBrowser => {
                Some("Document browser is not rendered in iced yet.")
            }
            TerminalScreen::DonkeyKong => Some("Donkey Kong route is reserved here."),
            TerminalScreen::NukeCodes => Some("Nuke Codes route is reserved here."),
            TerminalScreen::PtyApp => {
                Some("The actual CLI terminal/PTTY app remains separate from terminal mode and lands in Phase 3h.")
            }
            TerminalScreen::MainMenu => None,
        };

        let body: Element<'_, Message> = if let Some(entries) = terminal_menu_entries_for_screen(screen) {
            let mut layout = column![].spacing(12).width(Length::Fill).height(Length::Fill);
            if let Some(note) = note {
                let dim = super::retro_theme::current_retro_colors().dim.to_iced();
                layout = layout.push(
                    container(
                        text(note)
                            .font(iced::Font::MONOSPACE)
                            .size(14)
                            .color(dim)
                    )
                    .width(Length::Fill)
                );
            }
            layout = layout.push(self.view_terminal_menu_entries(entries, self.current_terminal_selected_idx()));
            layout.into()
        } else {
            container(text("TODO").font(iced::Font::MONOSPACE)).into()
        };

        let footer = if self.shell_status.is_empty() {
            "Arrow keys move | Enter select | Esc back".to_string()
        } else {
            self.shell_status.clone()
        };
        self.view_terminal_frame(
            match screen {
                TerminalScreen::Applications => "Applications",
                TerminalScreen::Documents => "Documents",
                TerminalScreen::Network => "Network",
                TerminalScreen::Games => "Games",
                TerminalScreen::DonkeyKong => "Donkey Kong",
                TerminalScreen::NukeCodes => "Nuke Codes",
                TerminalScreen::PtyApp => "CLI Terminal",
                TerminalScreen::ProgramInstaller => "Program Installer",
                TerminalScreen::Logs => "Logs",
                TerminalScreen::DocumentBrowser => "Document Browser",
                TerminalScreen::Settings => "Settings",
                TerminalScreen::EditMenus => "Edit Menus",
                TerminalScreen::Connections => "Connections",
                TerminalScreen::DefaultApps => "Default Apps",
                TerminalScreen::About => "About",
                TerminalScreen::UserManagement => "User Management",
                TerminalScreen::MainMenu => "Main Menu",
            }
            .to_string(),
            self.session_username.as_ref().map(|username| {
                format!(
                    "User: {username}{}",
                    if self.session_is_admin { " [ADMIN]" } else { "" }
                )
            }),
            body,
            footer,
            super::retro_theme::current_retro_colors().dim.to_iced(),
        )
    }

    fn view_terminal_frame<'a>(
        &self,
        title: String,
        subtitle: Option<String>,
        body: Element<'a, Message>,
        footer_text: String,
        footer_color: iced::Color,
    ) -> Element<'a, Message> {
        use iced::widget::{column, container, text, Space};
        use iced::Length;

        let mut header = column![].spacing(0);
        for line in HEADER_LINES {
            header = header.push(
                text(*line)
                    .font(iced::Font::MONOSPACE)
                    .size(20),
            );
        }

        let separator = || {
            container(Space::with_height(1))
                .width(Length::Fill)
                .style(super::retro_iced_theme::separator)
        };

        let mut layout = column![
            header,
            Space::with_height(8),
            separator(),
            Space::with_height(8),
            text(title)
                .font(iced::Font::MONOSPACE)
                .size(22),
            Space::with_height(8),
            separator(),
        ]
        .spacing(0)
        .width(Length::Fill)
        .height(Length::Fill);

        if let Some(subtitle) = subtitle {
            layout = layout.push(Space::with_height(8)).push(
                text(subtitle)
                    .font(iced::Font::MONOSPACE)
                    .size(14),
            );
        }

        layout = layout
            .push(Space::with_height(16))
            .push(container(body).width(Length::Fill).height(Length::Fill))
            .push(Space::with_height(12))
            .push(separator())
            .push(Space::with_height(8))
            .push(
                text(footer_text)
                    .font(iced::Font::MONOSPACE)
                    .size(14)
                    .color(footer_color),
            );

        container(layout)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding([20, 28])
            .style(super::retro_iced_theme::window_background)
            .into()
    }

    fn view_terminal_login_rows(&self) -> Element<'_, Message> {
        use iced::widget::{button, column, container, scrollable, text};
        use iced::Length;

        let palette = super::retro_theme::current_retro_colors();
        let dim = palette.dim.to_iced();

        let mut col = column![].spacing(2).width(Length::Fill);
        let mut selectable_idx = 0usize;

        for row in &self.login_rows {
            match row {
                LoginMenuRow::Separator => {
                    col = col.push(
                        container(
                            text("---")
                                .font(iced::Font::MONOSPACE)
                                .size(16)
                                .color(dim)
                        )
                        .padding([2, 8])
                        .width(Length::Fill),
                    );
                }
                LoginMenuRow::User(user) => {
                    let idx = selectable_idx;
                    let selected = idx == self.login.selected_idx;
                    let style: fn(
                        &Theme,
                        iced::widget::button::Status,
                    ) -> iced::widget::button::Style = if selected {
                        super::retro_iced_theme::retro_button_flat_selected
                    } else {
                        super::retro_iced_theme::retro_button_flat
                    };
                    let label = if selected {
                        format!("> {user}")
                    } else {
                        format!("  {user}")
                    };
                    col = col.push(
                        button(
                            text(label)
                                .font(iced::Font::MONOSPACE)
                                .size(16)
                        )
                        .on_press(Message::TerminalSelectionActivated(idx))
                        .width(Length::Fill)
                        .style(style)
                        .padding([4, 8]),
                    );
                    selectable_idx += 1;
                }
                LoginMenuRow::Exit => {
                    let idx = selectable_idx;
                    let selected = idx == self.login.selected_idx;
                    let style: fn(
                        &Theme,
                        iced::widget::button::Status,
                    ) -> iced::widget::button::Style = if selected {
                        super::retro_iced_theme::retro_button_flat_selected
                    } else {
                        super::retro_iced_theme::retro_button_flat
                    };
                    let label = if selected {
                        "> Exit".to_string()
                    } else {
                        "  Exit".to_string()
                    };
                    col = col.push(
                        button(
                            text(label)
                                .font(iced::Font::MONOSPACE)
                                .size(16)
                        )
                        .on_press(Message::TerminalSelectionActivated(idx))
                        .width(Length::Fill)
                        .style(style)
                        .padding([4, 8]),
                    );
                    selectable_idx += 1;
                }
            }
        }

        scrollable(col)
            .height(Length::Fill)
            .style(super::retro_iced_theme::retro_scrollable)
            .into()
    }

    fn view_terminal_main_menu_entries(&self) -> Element<'_, Message> {
        use iced::widget::{button, column, container, scrollable, text};
        use iced::Length;

        let palette = super::retro_theme::current_retro_colors();
        let dim = palette.dim.to_iced();

        let mut col = column![].spacing(2).width(Length::Fill);
        let mut selectable_idx = 0usize;

        for entry in MAIN_MENU_ENTRIES {
            if entry.action.is_none() {
                col = col.push(
                    container(
                        text(entry.label)
                            .font(iced::Font::MONOSPACE)
                            .size(16)
                            .color(dim)
                    )
                    .padding([2, 8])
                    .width(Length::Fill),
                );
                continue;
            }

            let idx = selectable_idx;
            let selected = idx == self.terminal_nav.main_menu_idx;
            let style: fn(
                &Theme,
                iced::widget::button::Status,
            ) -> iced::widget::button::Style = if selected {
                super::retro_iced_theme::retro_button_flat_selected
            } else {
                super::retro_iced_theme::retro_button_flat
            };
            let label = if selected {
                format!("> {}", entry.label)
            } else {
                format!("  {}", entry.label)
            };
            col = col.push(
                button(
                    text(label)
                        .font(iced::Font::MONOSPACE)
                        .size(16)
                )
                .on_press(Message::TerminalSelectionActivated(idx))
                .width(Length::Fill)
                .style(style)
                .padding([4, 8]),
            );
            selectable_idx += 1;
        }

        scrollable(col)
            .height(iced::Length::Fill)
            .style(super::retro_iced_theme::retro_scrollable)
            .into()
    }

    fn view_terminal_menu_entries(
        &self,
        entries: &[TerminalMenuEntry],
        selected_idx: usize,
    ) -> Element<'_, Message> {
        use iced::widget::{button, column, container, scrollable, text};
        use iced::Length;

        let palette = super::retro_theme::current_retro_colors();
        let dim = palette.dim.to_iced();

        let mut col = column![].spacing(2).width(Length::Fill);
        let mut selectable_idx = 0usize;

        for entry in entries {
            let Some(action) = entry.action else {
                col = col.push(
                    container(
                        text(entry.label)
                            .font(iced::Font::MONOSPACE)
                            .size(16)
                            .color(dim)
                    )
                    .padding([2, 8])
                    .width(Length::Fill),
                );
                continue;
            };

            let idx = selectable_idx;
            let selected = idx == selected_idx;
            let style: fn(
                &Theme,
                iced::widget::button::Status,
            ) -> iced::widget::button::Style = if selected {
                super::retro_iced_theme::retro_button_flat_selected
            } else {
                super::retro_iced_theme::retro_button_flat
            };
            let label = if selected {
                format!("> {}", entry.label)
            } else {
                format!("  {}", entry.label)
            };
            col = col.push(
                button(
                    text(label)
                        .font(iced::Font::MONOSPACE)
                        .size(16)
                )
                .on_press(Message::TerminalSelectionActivated(idx))
                .width(Length::Fill)
                .style(style)
                .padding([4, 8]),
            );
            let _ = action;
            selectable_idx += 1;
        }

        scrollable(col)
            .height(Length::Fill)
            .style(super::retro_iced_theme::retro_scrollable)
            .into()
    }

    fn view_terminal_prompt_overlay(&self) -> Element<'_, Message> {
        use iced::widget::{column, container, row, text, text_input, Space};
        use iced::{Alignment, Length};

        let Some(prompt) = self.terminal_prompt.as_ref() else {
            return Space::with_width(Length::Shrink).into();
        };

        let input = text_input("", &prompt.buffer)
            .id(Self::terminal_prompt_id())
            .on_input(Message::LoginPasswordChanged)
            .on_submit(Message::LoginSubmitted)
            .secure(matches!(prompt.kind, TerminalPromptKind::Password))
            .font(iced::Font::MONOSPACE)
            .size(16)
            .style(super::retro_iced_theme::terminal_text_input)
            .padding([8, 10]);

        let panel = container(
            column![
                text(prompt.title.as_str())
                    .font(iced::Font::MONOSPACE)
                    .size(18),
                text(prompt.prompt.as_str())
                    .font(iced::Font::MONOSPACE)
                    .size(14),
                input,
                text("Enter apply | Esc cancel")
                    .font(iced::Font::MONOSPACE)
                    .size(12)
                    .style(iced::widget::text::secondary),
            ]
            .spacing(12)
            .width(420)
        )
        .padding(18)
        .style(super::retro_iced_theme::overlay_panel);

        container(
            column![
                Space::with_height(Length::Fill),
                row![
                    Space::with_width(Length::Fill),
                    panel,
                    Space::with_width(Length::Fill),
                ]
                .align_y(Alignment::Center),
                Space::with_height(Length::Fill),
            ]
            .width(Length::Fill)
            .height(Length::Fill)
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }

    fn view_top_bar(&self) -> Element<'_, Message> {
        use iced::widget::{button, container, row, text, Space};
        use iced::Length;

        let palette = super::retro_theme::current_retro_colors();
        let dim = palette.dim.to_iced();

        // Active app name (bold, leftmost).
        let active_app_name = self.windows.active()
            .map(|w| format!("{:?}", w))
            .unwrap_or_else(|| "RobCoOS".to_string());

        let app_label = button(
            text(active_app_name).size(14)
        )
        .style(super::retro_iced_theme::retro_button_flat_selected)
        .padding([3, 8]);

        // Standard menu sections.
        let menu_items = ["File", "Edit", "View", "Window", "Help"];
        let mut menu_row = row![app_label].spacing(0).padding([0, 4]);
        for label in menu_items {
            let btn = button(text(label).size(13))
                .style(super::retro_iced_theme::retro_button_panel)
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
            .style(super::retro_iced_theme::panel_background)
            .into()
    }

    fn view_desktop(&self) -> Element<'_, Message> {
        use iced::widget::stack;
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

                let title = match id {
                    DesktopWindow::FileManager => {
                        let name = self.file_manager.cwd
                            .file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("/");
                        format!("File Manager — {}", name)
                    }
                    DesktopWindow::Editor => {
                        let name = self.editor.path
                            .as_deref()
                            .and_then(|p| p.file_name())
                            .and_then(|n| n.to_str())
                            .unwrap_or("Untitled");
                        if self.editor.dirty {
                            format!("Editor — {}*", name)
                        } else {
                            format!("Editor — {}", name)
                        }
                    }
                    DesktopWindow::Settings => "Settings".to_string(),
                    DesktopWindow::Applications => "Applications".to_string(),
                    DesktopWindow::Installer => "Program Installer".to_string(),
                    DesktopWindow::NukeCodes => "Nuke Codes".to_string(),
                    DesktopWindow::PtyApp => self
                        .desktop_pty
                        .as_ref()
                        .map(|pty| pty.title.clone())
                        .unwrap_or_else(|| "Terminal".to_string()),
                    _ => format!("{:?}", id),
                };

                let content: Element<'_, Message> = match id {
                    DesktopWindow::FileManager => self.view_file_manager(),
                    DesktopWindow::Editor => self.view_editor(),
                    DesktopWindow::Settings => self.view_settings_app(),
                    DesktopWindow::Applications => self.view_applications_app(),
                    DesktopWindow::Installer => self.view_installer_app(),
                    DesktopWindow::NukeCodes => self.view_nuke_codes_app(),
                    DesktopWindow::PtyApp => self.view_pty_terminal(),
                    _ => self.view_window_placeholder(id, fg, dim, bg),
                };

                WindowChild { id, rect, title, lifecycle, is_active, resizable, content }
            })
            .collect();

        let wm_layer = Element::from(DesktopWindowHost::new(wm_children));

        // Stack: surface icons behind windows.
        stack![self.view_surface_icons(), wm_layer]
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    // ── Hosted app views ─────────────────────────────────────────────────────

    /// File manager window content.
    fn view_file_manager(&self) -> Element<'_, Message> {
        use iced::widget::{button, column, container, row, scrollable, text, text_input};
        use iced::{Alignment, Length};
        use robcos_native_file_manager_app::FileManagerCommand;

        let cwd_str = self.file_manager.cwd.display().to_string();
        let selected_label = self
            .file_manager
            .selected_row()
            .map(|row| {
                let state = if row.is_dir { "Folder" } else { "File" };
                format!("Selected: {} ({state})", row.label)
            })
            .unwrap_or_else(|| "Nothing selected.".to_string());

        let toolbar = row![
            button(text("↑ Up").size(12))
                .padding([2, 8])
                .style(super::retro_iced_theme::retro_button)
                .on_press(Message::FileManagerCommand(FileManagerCommand::GoUp)),
            button(text("Open").size(12))
                .padding([2, 8])
                .style(super::retro_iced_theme::retro_button)
                .on_press(Message::FileManagerCommand(FileManagerCommand::OpenSelected)),
            text_input("Search files", &self.file_manager.search_query)
                .on_input(Message::FileManagerSearchChanged)
                .style(super::retro_iced_theme::terminal_text_input)
                .padding([4, 8])
                .width(Length::FillPortion(2)),
            text(cwd_str).size(12),
        ]
        .spacing(8)
        .padding([4, 8])
        .align_y(Alignment::Center)
        .width(Length::Fill);

        let rows = self.file_manager.rows();
        let selected = &self.file_manager.selected;
        let file_rows: Vec<Element<'_, Message>> = rows
            .iter()
            .map(|row| {
                let name = row.path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("..")
                    .to_string();
                let icon = row.icon();
                let is_selected = selected.as_deref() == Some(row.path.as_path());
                let style: fn(
                    &iced::Theme,
                    iced::widget::button::Status,
                ) -> iced::widget::button::Style = if is_selected {
                    super::retro_iced_theme::retro_button_flat_selected
                } else {
                    super::retro_iced_theme::retro_button_flat
                };
                let prefix = if is_selected { "> " } else { "  " };
                let label = format!("{prefix}{icon} {name}");
                button(text(label).size(12))
                    .padding([2, 12])
                    .width(Length::Fill)
                    .style(style)
                    .on_press(Message::FileManagerRowPressed(row.path.clone()))
                    .into()
            })
            .collect();

        let file_list = scrollable(
            column(file_rows).width(Length::Fill)
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .style(super::retro_iced_theme::retro_scrollable);

        let footer = container(
            text(format!(
                "{} item(s) | {} | Click once to select, again to open",
                rows.len(),
                selected_label
            ))
            .size(11)
            .style(iced::widget::text::secondary)
        )
        .padding([2, 8])
        .width(Length::Fill);

        let body = column![toolbar, file_list, footer]
            .width(Length::Fill)
            .height(Length::Fill);

        container(body)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(super::retro_iced_theme::window_background)
            .into()
    }

    /// Editor window content — wraps iced's text_editor widget.
    fn view_editor(&self) -> Element<'_, Message> {
        use iced::widget::{column, container, text, text_editor};
        use iced::{Font, Length};

        // Status bar: path + dirty indicator
        let status_str = if self.editor.dirty { "● Modified" } else { "" };
        let status = container(
            text(status_str).size(11).style(iced::widget::text::secondary)
        )
        .padding([2, 8])
        .width(Length::Fill);

        let editor_widget = text_editor(&self.editor_content)
            .on_action(Message::TextEditorAction)
            .font(Font::MONOSPACE)
            .size(13.0)
            .style(super::retro_iced_theme::retro_text_editor)
            .height(Length::Fill);

        column![editor_widget, status]
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    /// Settings window content — scrollable settings panel list.
    fn view_settings_app(&self) -> Element<'_, Message> {
        use iced::widget::{button, column, container, row, scrollable, text};
        use iced::{Alignment, Length};

        let current_panel = self.settings_panel.unwrap_or(desktop_settings_default_panel());
        let title_str = settings_panel_title(current_panel);
        let nav_button = |label: &'static str, msg: Message| {
            button(text(label).size(12))
                .padding([3, 10])
                .style(super::retro_iced_theme::retro_button)
                .on_press(msg)
        };
        let option_button = |label: String, selected: bool, msg: Message| {
            let style: fn(
                &iced::Theme,
                iced::widget::button::Status,
            ) -> iced::widget::button::Style = if selected {
                super::retro_iced_theme::retro_button_selected
            } else {
                super::retro_iced_theme::retro_button
            };
            button(text(label).size(12))
                .padding([4, 12])
                .style(style)
                .on_press(msg)
        };

        let mut header_row = row![text(title_str).size(14),]
            .spacing(8)
            .align_y(Alignment::Center)
            .push(iced::widget::Space::with_width(Length::Fill))
            .push(nav_button("Reload", Message::SettingsCancelRequested));
        if current_panel != NativeSettingsPanel::Home {
            header_row = header_row
                .push(nav_button(
                    "Back",
                    Message::SettingsPanelChanged(desktop_settings_back_target(current_panel)),
                ))
                .push(nav_button(
                    "Home",
                    Message::SettingsPanelChanged(NativeSettingsPanel::Home),
                ));
        }

        let header = container(
            header_row
        )
        .padding([8, 12])
        .width(Length::Fill)
        .style(super::retro_iced_theme::panel_background);

        let mut items: Vec<iced::Element<'_, Message>> = Vec::new();
        match current_panel {
            NativeSettingsPanel::Home => {
                for tile_row in desktop_settings_home_rows(self.session_is_admin) {
                    for tile in tile_row {
                        let label = format!("{} {}", tile.icon, tile.label);
                        let body = if tile.enabled {
                            match tile.action {
                                SettingsHomeTileAction::OpenPanel(panel) => {
                                    option_button(
                                        label,
                                        false,
                                        Message::SettingsPanelChanged(panel),
                                    )
                                    .width(Length::Fill)
                                    .into()
                                }
                                SettingsHomeTileAction::CloseWindow => option_button(
                                    label,
                                    false,
                                    Message::CloseWindow(DesktopWindow::Settings),
                                )
                                .width(Length::Fill)
                                .into(),
                            }
                        } else {
                            container(
                                text(format!("{label} (admin required)"))
                                    .size(12)
                                    .style(iced::widget::text::secondary)
                            )
                            .padding([4, 12])
                            .width(Length::Fill)
                            .into()
                        };
                        items.push(settings_card(tile.label.to_string(), body));
                    }
                }
            }
            NativeSettingsPanel::General => {
                items.push(settings_card(
                    "Default Open Mode".to_string(),
                    row![
                        option_button(
                            "Terminal".to_string(),
                            self.settings.default_open_mode == OpenMode::Terminal,
                            Message::SettingsOpenModeChanged(OpenMode::Terminal),
                        ),
                        option_button(
                            "Desktop".to_string(),
                            self.settings.default_open_mode == OpenMode::Desktop,
                            Message::SettingsOpenModeChanged(OpenMode::Desktop),
                        ),
                    ]
                    .spacing(8)
                    .into(),
                ));
                items.push(settings_card(
                    "System Sound".to_string(),
                    row![
                        option_button(
                            if self.settings.sound {
                                "Enabled".to_string()
                            } else {
                                "Disabled".to_string()
                            },
                            self.settings.sound,
                            Message::SettingsSoundToggled,
                        ),
                    ]
                    .spacing(8)
                    .into(),
                ));
                items.push(settings_card(
                    format!("System Sound Volume: {}%", self.settings.system_sound_volume),
                    row![
                        option_button(
                            "-5".to_string(),
                            false,
                            Message::SettingsSystemSoundVolumeAdjusted(-5),
                        ),
                        option_button(
                            "+5".to_string(),
                            false,
                            Message::SettingsSystemSoundVolumeAdjusted(5),
                        ),
                    ]
                    .spacing(8)
                    .into(),
                ));
                items.push(settings_card(
                    "Bootup".to_string(),
                    row![
                        option_button(
                            if self.settings.bootup {
                                "Enabled".to_string()
                            } else {
                                "Disabled".to_string()
                            },
                            self.settings.bootup,
                            Message::SettingsBootupToggled,
                        ),
                    ]
                    .spacing(8)
                    .into(),
                ));
                items.push(settings_card(
                    "Navigation Hints".to_string(),
                    row![
                        option_button(
                            if self.settings.show_navigation_hints {
                                "Enabled".to_string()
                            } else {
                                "Disabled".to_string()
                            },
                            self.settings.show_navigation_hints,
                            Message::SettingsNavigationHintsToggled,
                        ),
                    ]
                    .spacing(8)
                    .into(),
                ));
            }
            NativeSettingsPanel::Appearance => {
                let window_mode_row = row![
                    option_button(
                        "Windowed".to_string(),
                        self.settings.native_startup_window_mode
                            == NativeStartupWindowMode::Windowed,
                        Message::SettingsWindowModeChanged(NativeStartupWindowMode::Windowed),
                    ),
                    option_button(
                        "Maximized".to_string(),
                        self.settings.native_startup_window_mode
                            == NativeStartupWindowMode::Maximized,
                        Message::SettingsWindowModeChanged(NativeStartupWindowMode::Maximized),
                    ),
                    option_button(
                        "Borderless".to_string(),
                        self.settings.native_startup_window_mode
                            == NativeStartupWindowMode::BorderlessFullscreen,
                        Message::SettingsWindowModeChanged(
                            NativeStartupWindowMode::BorderlessFullscreen,
                        ),
                    ),
                    option_button(
                        "Fullscreen".to_string(),
                        self.settings.native_startup_window_mode
                            == NativeStartupWindowMode::Fullscreen,
                        Message::SettingsWindowModeChanged(NativeStartupWindowMode::Fullscreen),
                    ),
                ]
                .spacing(8);
                items.push(settings_card(
                    "Window Mode".to_string(),
                    window_mode_row.wrap().into(),
                ));
                let mut theme_choices = row![].spacing(8);
                for (name, _) in THEMES {
                    theme_choices = theme_choices.push(option_button(
                        (*name).to_string(),
                        self.settings.theme == *name,
                        Message::SettingsThemeChanged((*name).to_string()),
                    ));
                }
                items.push(settings_card("Theme".to_string(), theme_choices.wrap().into()));
                if self.settings.theme == CUSTOM_THEME_NAME {
                    let [r, g, b] = self.settings.custom_theme_rgb;
                    for (label, value, channel) in [
                        ("Custom Red", r, 0usize),
                        ("Custom Green", g, 1usize),
                        ("Custom Blue", b, 2usize),
                    ] {
                        items.push(settings_card(
                            format!("{label}: {value}"),
                            row![
                                option_button(
                                    "-1".to_string(),
                                    false,
                                    Message::SettingsCustomThemeAdjusted {
                                        channel,
                                        delta: -1,
                                    },
                                ),
                                option_button(
                                    "+1".to_string(),
                                    false,
                                    Message::SettingsCustomThemeAdjusted {
                                        channel,
                                        delta: 1,
                                    },
                                ),
                            ]
                            .spacing(8)
                            .into(),
                        ));
                    }
                }
                items.push(settings_card(
                    "Border Glyphs".to_string(),
                    row![
                        option_button(
                            "ASCII".to_string(),
                            self.settings.cli_acs_mode == CliAcsMode::Ascii,
                            Message::SettingsCliAcsModeChanged(CliAcsMode::Ascii),
                        ),
                        option_button(
                            "Unicode".to_string(),
                            self.settings.cli_acs_mode == CliAcsMode::Unicode,
                            Message::SettingsCliAcsModeChanged(CliAcsMode::Unicode),
                        ),
                    ]
                    .spacing(8)
                    .into(),
                ));
            }
            NativeSettingsPanel::Connections => {
                if macos_connections_disabled() {
                    items.push(settings_card(
                        "Platform".to_string(),
                        text("Connections management is limited on this platform.")
                            .size(12)
                            .style(iced::widget::text::secondary)
                            .into(),
                    ));
                }
                for item in desktop_settings_connections_nav_items() {
                    items.push(settings_card(
                        item.label.to_string(),
                        option_button(
                            format!("Open {}", item.label),
                            false,
                            Message::SettingsPanelChanged(item.panel),
                        )
                        .width(Length::Fill)
                        .into(),
                    ));
                }
            }
            NativeSettingsPanel::ConnectionsNetwork => {
                items.push(settings_card(
                    "Network".to_string(),
                    text("Network controls are not surfaced in the iced shell yet.")
                        .size(12)
                        .style(iced::widget::text::secondary)
                        .into(),
                ));
            }
            NativeSettingsPanel::ConnectionsBluetooth => {
                items.push(settings_card(
                    "Bluetooth".to_string(),
                    text("Bluetooth controls are not surfaced in the iced shell yet.")
                        .size(12)
                        .style(iced::widget::text::secondary)
                        .into(),
                ));
            }
            NativeSettingsPanel::DefaultApps => {
                for slot in [DefaultAppSlot::TextCode, DefaultAppSlot::Ebook] {
                    items.push(settings_card(
                        default_app_slot_label(slot).to_string(),
                        text(binding_label_for_slot(&self.settings, slot))
                            .size(12)
                            .into(),
                    ));
                }
                items.push(settings_card(
                    "Editing".to_string(),
                    text("Default-app editing remains in the dedicated app flows.")
                        .size(12)
                        .style(iced::widget::text::secondary)
                        .into(),
                ));
            }
            NativeSettingsPanel::CliProfiles => {
                let profiles = &self.settings.desktop_cli_profiles;
                for (label, profile) in [
                    ("Default", &profiles.default),
                    ("Calcurse", &profiles.calcurse),
                    ("Spotify Player", &profiles.spotify_player),
                    ("Ranger", &profiles.ranger),
                    ("Reddit", &profiles.reddit),
                ] {
                    items.push(settings_card(
                        label.to_string(),
                        column![
                            text(format!(
                                "Size: {}x{}",
                                profile.preferred_w.unwrap_or(0),
                                profile.preferred_h.unwrap_or(0)
                            ))
                            .size(12),
                            text(format!("Live resize: {}", profile.live_resize))
                                .size(12)
                                .style(iced::widget::text::secondary),
                        ]
                        .spacing(4)
                        .into(),
                    ));
                }
                items.push(settings_card(
                    "Custom Profiles".to_string(),
                    text(format!(
                        "{} custom profile(s) configured.",
                        self.settings.desktop_cli_profiles.custom.len()
                    ))
                    .size(12)
                    .into(),
                ));
            }
            NativeSettingsPanel::EditMenus => {
                items.push(settings_card(
                    "Text Editor".to_string(),
                    row![
                        option_button(
                            if self.settings.builtin_menu_visibility.text_editor {
                                "Visible".to_string()
                            } else {
                                "Hidden".to_string()
                            },
                            self.settings.builtin_menu_visibility.text_editor,
                            Message::SettingsBuiltinMenuVisibilityToggled { text_editor: true },
                        ),
                    ]
                    .spacing(8)
                    .into(),
                ));
                items.push(settings_card(
                    "Nuke Codes".to_string(),
                    row![
                        option_button(
                            if self.settings.builtin_menu_visibility.nuke_codes {
                                "Visible".to_string()
                            } else {
                                "Hidden".to_string()
                            },
                            self.settings.builtin_menu_visibility.nuke_codes,
                            Message::SettingsBuiltinMenuVisibilityToggled { text_editor: false },
                        ),
                    ]
                    .spacing(8)
                    .into(),
                ));
            }
            NativeSettingsPanel::UserManagement => {
                for item in desktop_settings_user_management_nav_items() {
                    items.push(settings_card(
                        item.label.to_string(),
                        option_button(
                            format!("Open {}", item.label),
                            false,
                            Message::SettingsPanelChanged(item.panel),
                        )
                        .width(Length::Fill)
                        .into(),
                    ));
                }
            }
            NativeSettingsPanel::UserManagementViewUsers => {
                for username in sorted_usernames() {
                    items.push(settings_card(
                        username.clone(),
                        text("Configured user account").size(12).into(),
                    ));
                }
            }
            NativeSettingsPanel::UserManagementCreateUser => {
                items.push(settings_card(
                    "Create User".to_string(),
                    text("User creation stays in the dedicated user-management flows.")
                        .size(12)
                        .style(iced::widget::text::secondary)
                        .into(),
                ));
            }
            NativeSettingsPanel::UserManagementEditUsers => {
                items.push(settings_card(
                    "Edit Users".to_string(),
                    text("Bulk user editing is not yet surfaced in the iced shell.")
                        .size(12)
                        .style(iced::widget::text::secondary)
                        .into(),
                ));
            }
            NativeSettingsPanel::UserManagementEditCurrentUser => {
                items.push(settings_card(
                    "Current User".to_string(),
                    text(
                        self.session_username
                            .as_deref()
                            .unwrap_or("No active session")
                            .to_string()
                    )
                    .size(12)
                    .into(),
                ));
            }
            NativeSettingsPanel::About => {
                items.push(settings_card(
                    "RobCoOS".to_string(),
                    column![
                        text("iced shell desktop").size(12),
                        text(format!("Runtime dir: {}", base_dir().display())).size(12),
                        text(format!("Theme: {}", self.settings.theme))
                            .size(12)
                            .style(iced::widget::text::secondary),
                    ]
                    .spacing(4)
                    .into(),
                ));
            }
        }

        let body = scrollable(
            column(items).width(Length::Fill).spacing(8).padding(12)
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .style(super::retro_iced_theme::retro_scrollable);

        container(column![header, body].width(Length::Fill).height(Length::Fill))
            .width(Length::Fill)
            .height(Length::Fill)
            .style(super::retro_iced_theme::window_background)
            .into()
    }

    fn view_applications_app(&self) -> Element<'_, Message> {
        use iced::widget::{button, column, container, scrollable, text};
        use iced::Length;

        let configured = catalog_names(ProgramCatalog::Applications);
        let sections = build_desktop_applications_sections(
            true,
            true,
            &configured,
            "Editor",
            "Nuke Codes",
        );

        let app_button = |label: String, msg: Message| {
            button(text(label).size(12))
                .padding([4, 12])
                .width(Length::Fill)
                .style(super::retro_iced_theme::retro_button)
                .on_press(msg)
                .into()
        };

        let mut rows: Vec<Element<'_, Message>> = vec![
            text("Built-in").size(13).into(),
        ];
        for entry in &sections.builtins {
            let request = resolve_desktop_applications_request(&entry.action);
            let msg = match request {
                DesktopProgramRequest::OpenTextEditor { .. } => {
                    Message::ShellAction(super::desktop_app::DesktopShellAction::OpenTextEditor)
                }
                DesktopProgramRequest::OpenNukeCodes { .. } => {
                    Message::ShellAction(super::desktop_app::DesktopShellAction::OpenNukeCodes)
                }
                DesktopProgramRequest::LaunchCatalog { name, .. } => {
                    Message::ShellAction(super::desktop_app::DesktopShellAction::LaunchConfiguredApp(name))
                }
                DesktopProgramRequest::OpenBuiltinGame => {
                    Message::ShellAction(super::desktop_app::DesktopShellAction::LaunchGameProgram(
                        "Donkey Kong".to_string(),
                    ))
                }
            };
            rows.push(app_button(entry.label.clone(), msg));
        }

        rows.push(text("Configured Apps").size(13).into());
        if sections.configured.is_empty() {
            rows.push(
                text("No configured desktop apps found.")
                    .size(11)
                    .style(iced::widget::text::secondary)
                    .into(),
            );
        } else {
            for entry in &sections.configured {
                let request = resolve_desktop_applications_request(&entry.action);
                let msg = match request {
                    DesktopProgramRequest::LaunchCatalog { name, .. } => {
                        Message::ShellAction(super::desktop_app::DesktopShellAction::LaunchConfiguredApp(name))
                    }
                    DesktopProgramRequest::OpenTextEditor { .. } => {
                        Message::ShellAction(super::desktop_app::DesktopShellAction::OpenTextEditor)
                    }
                    DesktopProgramRequest::OpenNukeCodes { .. } => {
                        Message::ShellAction(super::desktop_app::DesktopShellAction::OpenNukeCodes)
                    }
                    DesktopProgramRequest::OpenBuiltinGame => {
                        Message::ShellAction(super::desktop_app::DesktopShellAction::LaunchGameProgram(
                            "Donkey Kong".to_string(),
                        ))
                    }
                };
                rows.push(app_button(entry.label.clone(), msg));
            }
        }

        if !self.applications_status.is_empty() {
            rows.push(
                text(&self.applications_status)
                    .size(11)
                    .style(iced::widget::text::secondary)
                    .into(),
            );
        }

        container(
            scrollable(column(rows).spacing(6).padding(12))
                .height(Length::Fill)
                .style(super::retro_iced_theme::retro_scrollable)
        )
            .width(Length::Fill)
            .height(Length::Fill)
            .style(super::retro_iced_theme::window_background)
            .into()
    }

    fn view_installer_app(&self) -> Element<'_, Message> {
        use iced::widget::{button, column, container, row, scrollable, text, text_input};
        use iced::Length;

        let nav_button = |label: &'static str, msg: Message| {
            button(text(label).size(12))
                .padding([3, 10])
                .style(super::retro_iced_theme::retro_button)
                .on_press(msg)
        };

        let primary_button = |label: String, msg: Message| {
            button(text(label).size(12))
                .padding([4, 12])
                .width(Length::Fill)
                .style(super::retro_iced_theme::retro_button)
                .on_press(msg)
                .into()
        };

        let pm_label = self
            .installer
            .available_pms
            .get(self.installer.selected_pm_idx)
            .map(|pm| pm.name())
            .unwrap_or("Not Found");

        let toolbar = row![
            nav_button("Home", Message::InstallerBackRequested),
            nav_button("Search", Message::InstallerSearchRequested),
            nav_button("Installed", Message::InstallerInstalledRequested),
            nav_button("Runtime", Message::InstallerRuntimeToolsRequested),
            text(format!("PM: {pm_label}"))
                .size(11)
                .style(iced::widget::text::secondary),
        ]
        .spacing(8)
        .padding([6, 10]);

        let mut body: Vec<Element<'_, Message>> = Vec::new();

        if let Some(notice) = &self.installer_notice {
            body.push(
                container(
                    column![
                        text(if notice.success {
                            "Operation Complete"
                        } else {
                            "Operation Failed"
                        })
                        .size(13),
                        text(&notice.message).size(11),
                        nav_button("Dismiss", Message::InstallerNoticeDismissed),
                    ]
                    .spacing(6),
                )
                .padding(10)
                .style(super::retro_iced_theme::bordered_panel)
                .into(),
            );
        }

        if let Some(confirm) = &self.installer.confirm_dialog {
            body.push(
                container(
                    column![
                        text(format!("{:?} {}?", confirm.action, confirm.pkg))
                            .size(13),
                        row![
                            nav_button("Yes", Message::InstallerConfirmAccepted),
                            nav_button("No", Message::InstallerConfirmCancelled),
                        ]
                        .spacing(8),
                    ]
                    .spacing(6),
                )
                .padding(10)
                .style(super::retro_iced_theme::bordered_panel)
                .into(),
            );
        }

        match &self.installer.view {
            DesktopInstallerView::Home => {
                body.push(
                    text_input("Search packages", &self.installer.search_query)
                        .on_input(Message::InstallerSearchQueryChanged)
                        .on_submit(Message::InstallerSearchRequested)
                        .style(super::retro_iced_theme::terminal_text_input)
                        .padding(8)
                        .into(),
                );
                body.push(primary_button(
                    "Search Package Catalog".to_string(),
                    Message::InstallerSearchRequested,
                ));
                body.push(primary_button(
                    "Show Installed Packages".to_string(),
                    Message::InstallerInstalledRequested,
                ));
                body.push(primary_button(
                    "Runtime Tools".to_string(),
                    Message::InstallerRuntimeToolsRequested,
                ));
                body.push(text("Detected Package Managers").size(13).into());
                for (idx, pm) in self.installer.available_pms.iter().enumerate() {
                    let label = if idx == self.installer.selected_pm_idx {
                        format!("> {}", pm.name())
                    } else {
                        pm.name().to_string()
                    };
                    body.push(primary_button(
                        label,
                        Message::InstallerPackageManagerSelected(idx),
                    ));
                }
                if self.installer.available_pms.is_empty() {
                    body.push(
                        text("No supported package manager found.")
                            .size(11)
                            .style(iced::widget::text::secondary)
                            .into(),
                    );
                }
            }
            DesktopInstallerView::SearchResults => {
                body.push(
                    text_input("Search packages", &self.installer.search_query)
                        .on_input(Message::InstallerSearchQueryChanged)
                        .on_submit(Message::InstallerSearchRequested)
                        .style(super::retro_iced_theme::terminal_text_input)
                        .padding(8)
                        .into(),
                );
                for result in &self.installer.search_results {
                    let label = if result.installed {
                        format!("[installed] {}", result.raw)
                    } else {
                        result.raw.clone()
                    };
                    body.push(primary_button(
                        label,
                        Message::InstallerOpenPackageActions {
                            pkg: result.pkg.clone(),
                            installed: result.installed,
                        },
                    ));
                }
            }
            DesktopInstallerView::Installed => {
                body.push(
                    text_input("Filter installed packages", &self.installer.installed_filter)
                        .on_input(Message::InstallerInstalledFilterChanged)
                        .style(super::retro_iced_theme::terminal_text_input)
                        .padding(8)
                        .into(),
                );
                for pkg in self.installer.filtered_installed() {
                    body.push(primary_button(
                        pkg.clone(),
                        Message::InstallerOpenPackageActions {
                            pkg,
                            installed: true,
                        },
                    ));
                }
            }
            DesktopInstallerView::PackageActions { pkg, installed } => {
                body.push(text(pkg).size(14).into());
                if let Some(description) = self.installer.cached_package_description(pkg) {
                    body.push(
                        text(description)
                            .size(11)
                            .style(iced::widget::text::secondary)
                            .into(),
                    );
                }
                if *installed {
                    for action in [
                        InstallerPackageAction::Update,
                        InstallerPackageAction::Reinstall,
                        InstallerPackageAction::Uninstall,
                    ] {
                        body.push(primary_button(
                            format!("{action:?}"),
                            Message::InstallerRunPackageAction(action),
                        ));
                    }
                    body.push(primary_button(
                        "Add To Menu".to_string(),
                        Message::InstallerOpenAddToMenu(pkg.clone()),
                    ));
                } else {
                    body.push(primary_button(
                        "Install".to_string(),
                        Message::InstallerRunPackageAction(InstallerPackageAction::Install),
                    ));
                }
            }
            DesktopInstallerView::AddToMenu { pkg } => {
                body.push(text(format!("Add {pkg} to menu")).size(14).into());
                body.push(
                    text_input("Display name", &self.installer.display_name_input)
                        .on_input(Message::InstallerDisplayNameChanged)
                        .style(super::retro_iced_theme::terminal_text_input)
                        .padding(8)
                        .into(),
                );
                for (label, target) in [
                    ("Applications", InstallerMenuTarget::Applications),
                    ("Games", InstallerMenuTarget::Games),
                    ("Network", InstallerMenuTarget::Network),
                ] {
                    body.push(primary_button(
                        format!("Add To {label}"),
                        Message::InstallerAddToMenu {
                            pkg: pkg.clone(),
                            target,
                        },
                    ));
                }
            }
            DesktopInstallerView::RuntimeTools => {
                for tool in available_runtime_tools() {
                    let pkg = runtime_tool_pkg(*tool).to_string();
                    let installed = self
                        .installer_runtime_status
                        .get(&pkg)
                        .copied()
                        .unwrap_or(false);
                    let state = if installed { "[installed]" } else { "[not installed]" };
                    body.push(text(format!("{state} {}", runtime_tool_title(*tool))).size(13).into());
                    body.push(
                        text(runtime_tool_description(*tool))
                            .size(11)
                            .style(iced::widget::text::secondary)
                            .into(),
                    );
                    let action = if installed {
                        InstallerPackageAction::Update
                    } else {
                        InstallerPackageAction::Install
                    };
                    body.push(primary_button(
                        format!("{action:?}"),
                        Message::InstallerOpenPackageActions {
                            pkg,
                            installed,
                        },
                    ));
                }
            }
        }

        body.push(
            text(&self.installer.status)
                .size(11)
                .style(iced::widget::text::secondary)
                .into(),
        );

        let content = column(body).spacing(8).padding(12);
        container(
            column![
                toolbar,
                scrollable(content)
                    .height(Length::Fill)
                    .style(super::retro_iced_theme::retro_scrollable)
            ]
        )
            .width(Length::Fill)
            .height(Length::Fill)
            .style(super::retro_iced_theme::window_background)
            .into()
    }

    fn view_nuke_codes_app(&self) -> Element<'_, Message> {
        use iced::widget::{button, column, container, text};
        use iced::Length;

        let refresh = button(text("Refresh").size(12))
            .padding([4, 10])
            .style(super::retro_iced_theme::retro_button)
            .on_press(Message::NukeCodesRefreshRequested);

        let body = match &self.nuke_codes {
            NukeCodesView::Unloaded => column![
                refresh,
                text("Codes are not loaded yet.")
                    .size(12)
                    .style(iced::widget::text::secondary),
            ]
            .spacing(8),
            NukeCodesView::Error(err) => column![
                refresh,
                text("UNABLE TO FETCH LIVE CODES").size(13),
                text(err).size(11).style(iced::widget::text::secondary),
            ]
            .spacing(8),
            NukeCodesView::Data(codes) => column![
                refresh,
                text(format!("ALPHA   : {}", codes.alpha)).size(13),
                text(format!("BRAVO   : {}", codes.bravo)).size(13),
                text(format!("CHARLIE : {}", codes.charlie)).size(13),
                text(format!("Source: {}", codes.source))
                    .size(11)
                    .style(iced::widget::text::secondary),
                text(format!("Fetched: {}", codes.fetched_at))
                    .size(11)
                    .style(iced::widget::text::secondary),
            ]
            .spacing(8),
        };

        container(body.padding(12))
            .width(Length::Fill)
            .height(Length::Fill)
            .style(super::retro_iced_theme::window_background)
            .into()
    }

    fn view_pty_terminal(&self) -> Element<'_, Message> {
        use iced::widget::{column, container, text};
        use iced::Length;

        let palette = super::retro_theme::current_retro_colors();

        let Some(pty) = self.desktop_pty.as_ref() else {
            return container(
                text("Launching terminal...")
                    .size(12)
                    .style(iced::widget::text::secondary),
            )
                .width(Length::Fill)
                .height(Length::Fill)
                .style(super::retro_iced_theme::window_background)
                .into();
        };

        let frame = pty.session.committed_frame();
        let viewport = canvas(PtyCanvas::new(frame.clone(), palette))
            .width(Length::Fill)
            .height(Length::Fill);
        let footer = container(
            text(format!("{}x{}  {}", frame.cols, frame.rows, pty.title))
                .size(11)
                .style(iced::widget::text::secondary),
        )
        .padding([2, 8]);

        container(column![viewport, footer].height(Length::Fill))
            .width(Length::Fill)
            .height(Length::Fill)
            .style(super::retro_iced_theme::window_background)
            .into()
    }

    /// Fallback placeholder for windows without a dedicated view yet.
    fn view_window_placeholder(
        &self,
        id: DesktopWindow,
        fg: iced::Color,
        dim: iced::Color,
        _bg: iced::Color,
    ) -> Element<'_, Message> {
        use iced::widget::{column, container, text};
        use iced::Length;
        container(
            column![
                text(format!("{:?}", id)).size(15).color(fg),
                text("Coming soon").size(11).color(dim),
            ]
            .spacing(6)
            .padding(10),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .style(super::retro_iced_theme::window_background)
        .into()
    }

    /// Render the desktop surface: black background with builtin icon column on the left.
    fn view_surface_icons(&self) -> Element<'_, Message> {
        use super::desktop_surface_service::desktop_builtin_icons;
        use super::message::DesktopIconId;
        use iced::widget::{button, column, container, row, text};
        use iced::Length;

        let palette = super::retro_theme::current_retro_colors();
        let fg = palette.fg.to_iced();
        let selected_fg = palette.selected_fg.to_iced();

        let mut icon_col = column![].spacing(4).padding([8, 4]);

        for entry in desktop_builtin_icons() {
            let icon_id = DesktopIconId::Builtin(entry.key);
            let is_selected = self.surface.selected_icon.as_ref() == Some(&icon_id);
            let tile_style: fn(&Theme) -> iced::widget::container::Style = if is_selected {
                super::retro_iced_theme::icon_tile_selected
            } else {
                super::retro_iced_theme::icon_tile
            };
            let lbl_fg = if is_selected { selected_fg } else { fg };

            let id_clone = icon_id.clone();
            let icon_btn = button(
                column![
                    // ASCII art glyph as icon proxy
                    container(
                        text(entry.ascii).size(10).color(lbl_fg)
                    )
                    .width(64)
                    .style(tile_style)
                    .padding(4),
                    // Label below
                    text(entry.label).size(10).color(lbl_fg),
                ]
                .spacing(2)
                .width(68)
            )
            .on_press(Message::DesktopIconClicked { id: id_clone, shift: false })
            .style(super::retro_iced_theme::transparent_button)
            .padding(2);

            icon_col = icon_col.push(icon_btn);
        }

        // Full-screen black background with icons pinned left.
        container(
            row![icon_col]
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .style(super::retro_iced_theme::window_background)
        .into()
    }

    fn view_taskbar(&self) -> Element<'_, Message> {
        use iced::widget::{button, container, row, text, Space};
        use iced::Length;

        // [Start] button — always filled green.
        let start_label = if self.start_menu.open { "[Close]" } else { "[Start]" };
        let start_btn = button(
            text(start_label).size(14)
        )
        .on_press(Message::StartButtonClicked)
        .style(super::retro_iced_theme::retro_button_selected_strong)
        .padding([4, 10]);

        let mut task_row = row![start_btn, Space::with_width(6)].spacing(3).padding([3, 4]);

        // One button per open window.
        for win in self.windows.z_ordered() {
            let id = win.id;
            let is_active = self.windows.active() == Some(id);
            let is_minimized = win.is_minimized();
            let label = format!(" {:?} ", id);

            let style: fn(
                &Theme,
                iced::widget::button::Status,
            ) -> iced::widget::button::Style = if is_active {
                super::retro_iced_theme::retro_button_panel_active
            } else if is_minimized {
                super::retro_iced_theme::retro_button_panel_minimized
            } else {
                super::retro_iced_theme::retro_button_panel
            };

            let btn = button(text(label).size(12))
                .on_press(Message::TaskbarWindowClicked(id))
                .style(style)
                .padding([4, 8]);

            task_row = task_row.push(btn);
        }

        task_row = task_row.push(Space::with_width(Length::Fill));

        container(task_row)
            .width(Length::Fill)
            .height(32)
            .style(super::retro_iced_theme::panel_background)
            .into()
    }

    /// Render the spotlight / search overlay, centered on screen.
    fn view_spotlight(&self) -> Element<'_, Message> {
        use iced::widget::{
            button, column, container, mouse_area, row, scrollable, text, text_input, Space,
        };
        use iced::{Alignment, Length};

        let palette = super::retro_theme::current_retro_colors();
        let dim = palette.dim.to_iced();

        // ── Search input ──────────────────────────────────────────────────────
        let search_input = text_input("> Search…", &self.spotlight.query)
            .on_input(Message::SpotlightQueryChanged)
            .on_submit(Message::SpotlightActivateSelected)
            .size(18)
            .style(super::retro_iced_theme::terminal_text_input)
            .padding([8, 12]);

        // ── Tab bar ───────────────────────────────────────────────────────────
        let tabs = ["All", "Apps", "Documents", "Files"];
        let mut tab_row = row![].spacing(0);
        for (i, tab_label) in tabs.iter().enumerate() {
            let is_sel = self.spotlight.tab == i as u8;
            let style: fn(
                &Theme,
                iced::widget::button::Status,
            ) -> iced::widget::button::Style = if is_sel {
                super::retro_iced_theme::retro_button_selected
            } else {
                super::retro_iced_theme::retro_button_panel
            };
            tab_row = tab_row.push(
                button(text(*tab_label).size(12))
                    .on_press(Message::SpotlightTabChanged(i as u8))
                    .style(style)
                    .padding([4, 12])
            );
        }

        // ── Results list ──────────────────────────────────────────────────────
        let selected = self.spotlight.selected;
        let mut results_col = column![].spacing(0);

        if self.spotlight.results.is_empty() {
            let hint = if self.spotlight.query.is_empty() {
                "Type to search…"
            } else {
                "No results"
            };
            results_col = results_col.push(
                container(text(hint).size(13).color(dim))
                    .padding([12, 12])
                    .width(Length::Fill)
            );
        } else {
            for (i, result) in self.spotlight.results.iter().enumerate() {
                let is_sel = i == selected;
                let style: fn(
                    &Theme,
                    iced::widget::button::Status,
                ) -> iced::widget::button::Style = if is_sel {
                    super::retro_iced_theme::retro_button_flat_selected
                } else {
                    super::retro_iced_theme::retro_button_flat
                };
                let category_str = format!("[{:?}]", result.category);
                results_col = results_col.push(
                    button(
                        row![
                            text(result.name.as_str()).size(13).width(Length::Fill),
                            text(category_str)
                                .size(11)
                                .color(if is_sel {
                                    palette.selected_fg.to_iced()
                                } else {
                                    dim
                                }),
                        ]
                        .spacing(8)
                        .padding([4, 10])
                    )
                    .on_press(Message::SpotlightActivateSelected)
                    .width(Length::Fill)
                    .style(style)
                    .padding(0)
                );
            }
        }

        let results_scroll = scrollable(results_col)
            .height(300)
            .style(super::retro_iced_theme::retro_scrollable);

        // ── Compose panel ─────────────────────────────────────────────────────
        let panel = container(
            column![
                search_input,
                tab_row,
                container(Space::with_height(1))
                    .width(Length::Fill)
                    .style(super::retro_iced_theme::separator),
                results_scroll,
            ]
            .spacing(0)
            .width(600)
        )
        .style(super::retro_iced_theme::overlay_panel);

        // Centre the panel with an outer dismiss-on-click backdrop.
        let backdrop = mouse_area(
            column![
                Space::with_height(Length::Fill),
                row![
                    Space::with_width(Length::Fill),
                    panel,
                    Space::with_width(Length::Fill),
                ]
                .align_y(Alignment::Center),
                Space::with_height(Length::Fill),
            ]
            .width(Length::Fill)
            .height(Length::Fill)
        )
        .on_press(Message::CloseSpotlight);

        backdrop.into()
    }

    /// Render the start menu panel, anchored to the bottom-left of the screen.
    ///
    /// The panel opens above the [Start] button and contains:
    /// - Left column: root menu items (Applications, Documents, Network, Games, System, …)
    /// - Right panel: submenu or leaf entries for the selected root item
    fn view_start_menu(&self) -> Element<'_, Message> {
        use super::desktop_start_menu::{
            start_root_leaf_for_idx, start_root_submenu_for_idx, StartLeaf, START_ROOT_ITEMS,
            START_ROOT_VIS_ROWS,
        };
        use iced::widget::{button, column, container, mouse_area, row, text, Space};
        use iced::Length;

        let palette = super::retro_theme::current_retro_colors();
        let fg = palette.fg.to_iced();
        let dim = palette.dim.to_iced();

        let selected_root = self.start_menu.selected_root;

        // ── Root column ───────────────────────────────────────────────────────
        let mut root_col = column![].spacing(0).width(200);

        // Header
        root_col = root_col.push(
            container(
                text("R O B C O O S").size(13).color(fg)
            )
            .padding([6, 10])
            .width(Length::Fill)
            .style(super::retro_iced_theme::panel_background)
        );

        // Separator
        root_col = root_col.push(
            container(Space::with_height(1))
                .width(Length::Fill)
                .style(super::retro_iced_theme::separator)
        );

        for (vis_idx, root_slot) in START_ROOT_VIS_ROWS.iter().enumerate() {
            match root_slot {
                None => {
                    // Separator row
                    root_col = root_col.push(
                        container(Space::with_height(1))
                            .width(Length::Fill)
                            .style(super::retro_iced_theme::separator)
                    );
                }
                Some(item_idx) => {
                    let idx = *item_idx;
                    let label = START_ROOT_ITEMS[idx];
                    let is_sel = idx == selected_root;
                    let has_sub = start_root_leaf_for_idx(idx).is_some()
                        || start_root_submenu_for_idx(idx).is_some();
                    let arrow = if has_sub { " >" } else { "  " };
                    let disp = format!("{label}{arrow}");
                    let style: fn(
                        &Theme,
                        iced::widget::button::Status,
                    ) -> iced::widget::button::Style = if is_sel {
                        super::retro_iced_theme::retro_button_flat_selected
                    } else {
                        super::retro_iced_theme::retro_button_flat
                    };

                    root_col = root_col.push(
                        button(
                            row![
                                text(disp).size(13),
                            ]
                            .padding([4, 8])
                        )
                        .on_press(Message::StartMenuSelectRoot(idx))
                        .width(Length::Fill)
                        .style(style)
                        .padding(0)
                    );
                }
            }
            let _ = vis_idx;
        }

        let root_panel = container(root_col)
            .style(super::retro_iced_theme::overlay_panel);

        // ── Right panel (submenu / leaf) ──────────────────────────────────────
        let leaf = start_root_leaf_for_idx(selected_root);
        let submenu = start_root_submenu_for_idx(selected_root);

        let right_panel: Option<Element<'_, Message>> = if let Some(sub) = submenu {
            // System submenu.
            use super::desktop_start_menu::START_SYSTEM_ITEMS;
            let mut sub_col = column![].spacing(0).width(180);
            for (label, _action) in START_SYSTEM_ITEMS.iter() {
                sub_col = sub_col.push(
                    button(
                        text(*label).size(13).width(Length::Fill)
                    )
                    .on_press(Message::StartMenuNavigate(super::message::NavDirection::Right))
                    .width(Length::Fill)
                    .style(super::retro_iced_theme::retro_button_flat)
                    .padding([4, 8])
                );
            }
            let _ = sub;
            Some(
                container(sub_col)
                    .style(super::retro_iced_theme::overlay_panel)
                    .into()
            )
        } else if let Some(lf) = leaf {
            let label = match lf {
                StartLeaf::Applications => "Applications",
                StartLeaf::Documents => "Documents",
                StartLeaf::Network => "Network",
                StartLeaf::Games => "Games",
            };
            let mut leaf_col = column![
                container(text(label).size(13).color(fg))
                    .padding([6, 10])
                    .width(Length::Fill)
                    .style(super::retro_iced_theme::panel_background),
                container(Space::with_height(1))
                    .width(Length::Fill)
                    .style(super::retro_iced_theme::separator),
            ].spacing(0).width(200);
            leaf_col = leaf_col.push(
                text("(Loading…)").size(12).color(dim)
            );
            Some(
                container(leaf_col)
                    .style(super::retro_iced_theme::overlay_panel)
                    .into()
            )
        } else {
            None
        };

        // ── Compose left + optional right panel ───────────────────────────────
        let menu_body: Element<'_, Message> = if let Some(right) = right_panel {
            row![root_panel, right].spacing(0).into()
        } else {
            root_panel.into()
        };

        // ── Position anchored to bottom-left of screen ────────────────────────
        // `stack` positions children absolutely; we push the menu to the
        // bottom-left using a column with a spacer on top.
        let dismiss = mouse_area(
            column![
                Space::with_height(Length::Fill),
                menu_body,
                Space::with_height(32), // taskbar height
            ]
            .width(Length::Fill)
            .height(Length::Fill)
        )
        .on_press(Message::StartMenuClose);

        dismiss.into()
    }

    fn refresh_installer_runtime_cache(&mut self) {
        self.installer_runtime_status.clear();
        for tool in available_runtime_tools() {
            self.installer_runtime_status.insert(
                runtime_tool_pkg(*tool).to_string(),
                self.installer.runtime_tool_installed_cached(*tool),
            );
        }
    }

    fn open_desktop_terminal_shell(&mut self) -> Task<Message> {
        let requested_shell = std::env::var("SHELL").ok();
        let bash_exists = std::path::Path::new("/bin/bash").exists();
        let plan = terminal_shell_launch_plan(
            TerminalShellSurface::Desktop,
            requested_shell.as_deref(),
            bash_exists,
        );
        self.launch_desktop_pty_plan(plan, None)
    }

    fn launch_catalog_program(&mut self, name: &str, catalog: ProgramCatalog) -> Task<Message> {
        match resolve_catalog_launch(name, catalog) {
            Ok(launch) => {
                let plan = terminal_command_launch_plan(
                    TerminalShellSurface::Desktop,
                    &launch.title,
                    &launch.argv,
                    TerminalScreen::MainMenu,
                    desktop_pty_force_render_mode(&launch.argv),
                );
                self.launch_desktop_pty_plan(plan, None)
            }
            Err(err) => {
                self.applications_status = err.clone();
                self.shell_status = err;
                Task::none()
            }
        }
    }

    fn launch_desktop_pty_plan(
        &mut self,
        plan: TerminalPtyLaunchPlan,
        completion_message: Option<String>,
    ) -> Task<Message> {
        if plan.argv.is_empty() {
            self.shell_status = "Error: empty PTY command.".to_string();
            return Task::none();
        }

        if plan.replace_existing_pty {
            self.close_desktop_pty();
        }

        let profile = desktop_pty_profile_for_command(&plan.argv);
        let pty_cols = profile
            .preferred_w
            .unwrap_or(96)
            .max(profile.min_w)
            .clamp(40, 160);
        let pty_rows = profile
            .preferred_h
            .unwrap_or(32)
            .max(profile.min_h)
            .clamp(12, 60);
        let options = PtyLaunchOptions {
            env: plan.env.clone(),
            top_bar: None,
            force_render_mode: plan.force_render_mode,
        };

        let program = plan.argv[0].clone();
        let args: Vec<&str> = plan.argv.iter().skip(1).map(String::as_str).collect();

        match PtySession::spawn(&program, &args, pty_cols, pty_rows, &options) {
            Ok(session) => {
                self.desktop_mode = true;
                self.desktop_pty = Some(DesktopPtyState {
                    title: plan.title.clone(),
                    completion_message,
                    session,
                    cols_floor: pty_cols,
                    rows_floor: pty_rows,
                    live_resize: profile.live_resize,
                });
                if !self.windows.is_open(DesktopWindow::PtyApp) {
                    self.windows.open(
                        DesktopWindow::PtyApp,
                        WindowRect::new(120.0, 72.0, 900.0, 620.0),
                        (480.0, 260.0),
                        true,
                    );
                } else {
                    self.windows.bring_to_front(DesktopWindow::PtyApp);
                }
                if profile.open_fullscreen {
                    self.windows.toggle_maximize(
                        DesktopWindow::PtyApp,
                        WindowRect::new(0.0, 32.0, 1360.0, 808.0),
                    );
                }
                self.shell_status = plan.success_status;
            }
            Err(err) => {
                self.shell_status = format!("Launch failed: {err}");
            }
        }

        Task::none()
    }

    fn close_desktop_pty(&mut self) {
        if let Some(mut pty) = self.desktop_pty.take() {
            pty.session.terminate();
        }
    }

    fn sync_desktop_pty(&mut self) {
        self.resize_desktop_pty_to_window();

        let mut exit_plan = None;
        if let Some(pty) = self.desktop_pty.as_mut() {
            if !pty.session.is_alive() {
                let exit_status = pty.session.exit_status();
                let success = exit_status.as_ref().map(|status| status.success()).unwrap_or(true);
                let exit_code = exit_status.as_ref().map(|status| status.exit_code());
                exit_plan = Some(resolve_desktop_pty_exit(
                    &pty.title,
                    pty.completion_message.as_deref(),
                    success,
                    exit_code,
                ));
            }
        }

        if let Some(plan) = exit_plan {
            self.close_desktop_pty();
            self.windows.close(DesktopWindow::PtyApp);
            self.apply_desktop_pty_exit_plan(plan);
        }
    }

    fn apply_desktop_pty_exit_plan(&mut self, plan: TerminalDesktopPtyExitPlan) {
        self.shell_status = plan.status;
        if let Some(message) = plan.installer_notice_message {
            self.installer_notice = Some(DesktopInstallerNotice {
                message,
                success: plan.installer_notice_success,
            });
        }
        if plan.reopen_installer {
            self.installer.open = true;
            if !self.windows.is_open(DesktopWindow::Installer) {
                self.windows.open(
                    DesktopWindow::Installer,
                    WindowRect::new(140.0, 80.0, 760.0, 560.0),
                    (500.0, 400.0),
                    true,
                );
            } else {
                self.windows.bring_to_front(DesktopWindow::Installer);
            }
            self.refresh_installer_runtime_cache();
        }
    }

    fn resize_desktop_pty_to_window(&mut self) {
        let Some((content_w, content_h)) = self.inner_window_content_size(DesktopWindow::PtyApp)
        else {
            return;
        };
        let Some(pty) = self.desktop_pty.as_mut() else {
            return;
        };

        let cols = if pty.live_resize {
            ((content_w / TERMINAL_CELL_WIDTH).floor() as u16)
                .max(pty.cols_floor)
                .clamp(40, 220)
        } else {
            pty.cols_floor
        };
        let rows = if pty.live_resize {
            ((content_h / TERMINAL_CELL_HEIGHT).floor() as u16)
                .max(pty.rows_floor)
                .clamp(12, 80)
        } else {
            pty.rows_floor
        };
        pty.session.resize(cols.max(1), rows.max(1));
    }

    fn inner_window_content_size(&self, window: DesktopWindow) -> Option<(f32, f32)> {
        let rect = self.windows.get(window)?.rect;
        Some((
            (rect.w - 2.0 * DESKTOP_WINDOW_BORDER_WIDTH).max(1.0),
            (rect.h - DESKTOP_WINDOW_TITLE_BAR_HEIGHT - DESKTOP_WINDOW_BORDER_WIDTH).max(1.0),
        ))
    }

    fn open_editor_path(&mut self, path: PathBuf) {
        match fs::read_to_string(&path) {
            Ok(text) => {
                self.editor.path = Some(path.clone());
                self.editor.text = text.clone();
                self.editor.dirty = false;
                self.editor.status = format!("Opened {}", path.display());
                self.editor_content = iced::widget::text_editor::Content::with_text(&text);
            }
            Err(err) => {
                self.editor.status = format!("Failed to open {}: {err}", path.display());
            }
        }
    }

    fn launch_open_with_request(&mut self, launch: OpenWithLaunchRequest) -> Task<Message> {
        let plan = terminal_command_launch_plan(
            TerminalShellSurface::Desktop,
            &launch.title,
            &launch.argv,
            TerminalScreen::Documents,
            desktop_pty_force_render_mode(&launch.argv),
        );
        let task = self.launch_desktop_pty_plan(plan, None);
        self.shell_status = launch.status_message;
        task
    }

    fn handle_file_manager_open_target(
        &mut self,
        target: FileManagerOpenTarget,
    ) -> Task<Message> {
        match target {
            FileManagerOpenTarget::NoOp => Task::none(),
            FileManagerOpenTarget::Launch(launch) => self.launch_open_with_request(launch),
            FileManagerOpenTarget::OpenInEditor(path) => {
                self.open_editor_path(path);
                self.update(Message::OpenWindow(DesktopWindow::Editor))
            }
        }
    }

    fn persist_settings_change<F>(&mut self, apply: F)
    where
        F: FnOnce(&mut Settings),
    {
        apply(&mut self.settings);
        self.settings = persist_settings_draft(&self.settings);
        self.file_manager.refresh_contents();
    }

    fn handle_global_key_press(&mut self, key: Key, mods: Modifiers) -> Task<Message> {
        if matches!(key.as_ref(), Key::Named(Named::Space)) && mods.command() {
            return self.update(Message::OpenSpotlight);
        }

        if !self.desktop_mode {
            return match key.as_ref() {
                Key::Named(Named::Escape) => self.update(Message::CloseSpotlight),
                Key::Named(Named::ArrowLeft) => self.update(Message::TerminalBackRequested),
                Key::Named(Named::ArrowUp) => {
                    self.update(Message::TerminalNavigate(NavDirection::Up))
                }
                Key::Named(Named::ArrowDown) => {
                    self.update(Message::TerminalNavigate(NavDirection::Down))
                }
                Key::Named(Named::Enter) => self.update(Message::TerminalActivateSelected),
                _ => Task::none(),
            };
        }

        if self.spotlight.open {
            return match key.as_ref() {
                Key::Named(Named::Escape) => self.update(Message::CloseSpotlight),
                Key::Named(Named::ArrowUp) => self.update(Message::SpotlightNavigate(NavDirection::Up)),
                Key::Named(Named::ArrowDown) => {
                    self.update(Message::SpotlightNavigate(NavDirection::Down))
                }
                Key::Named(Named::ArrowLeft) => {
                    self.update(Message::SpotlightNavigate(NavDirection::Left))
                }
                Key::Named(Named::ArrowRight) => {
                    self.update(Message::SpotlightNavigate(NavDirection::Right))
                }
                Key::Named(Named::Enter) => self.update(Message::SpotlightActivateSelected),
                Key::Named(Named::Tab) if mods.shift() => {
                    self.update(Message::SpotlightNavigate(NavDirection::ShiftTab))
                }
                Key::Named(Named::Tab) => self.update(Message::SpotlightNavigate(NavDirection::Tab)),
                _ => Task::none(),
            };
        }

        if self.desktop_pty_accepts_keyboard() {
            return self.handle_desktop_pty_key(&key, mods);
        }

        if matches!(key.as_ref(), Key::Named(Named::Escape)) {
            self.start_menu.close();
        }

        Task::none()
    }

    fn desktop_pty_accepts_keyboard(&self) -> bool {
        self.desktop_mode
            && self.windows.active() == Some(DesktopWindow::PtyApp)
            && self.desktop_pty.is_some()
            && !self.start_menu.open
            && !self.spotlight.open
    }

    fn handle_desktop_pty_key(&mut self, key: &Key, mods: Modifiers) -> Task<Message> {
        let Some(pty) = self.desktop_pty.as_mut() else {
            return Task::none();
        };

        if let Key::Character(text) = key.as_ref() {
            if !mods.control() && !mods.alt() && !mods.logo() {
                pty.session.write(text.as_bytes());
                return Task::none();
            }
            if let Some(ch) = text.chars().next() {
                pty.session
                    .send_key(PtyKeyCode::Char(ch.to_ascii_lowercase()), modifiers_to_pty(mods));
            }
            return Task::none();
        }

        if let Some(code) = named_key_to_pty(key) {
            pty.session.send_key(code, modifiers_to_pty(mods));
        }

        Task::none()
    }

    /// Return the application theme.
    ///
    /// Phase 2: uses iced's built-in Dark theme.
    /// Phase 4: replace with a custom `RetroTheme` that applies the full palette.
    pub fn theme(&self) -> Theme {
        super::retro_iced_theme::retro_theme()
    }

    /// Return active subscriptions.
    ///
    /// Phase 3b/3e: clock tick + global keyboard shortcuts (Cmd+Space, Escape).
    /// Phase 3g: also add PTY output stream.
    pub fn subscription(&self) -> Subscription<Message> {
        use iced::keyboard;

        let clock_tick =
            iced::time::every(std::time::Duration::from_secs(30)).map(Message::Tick);
        let hotkeys = keyboard::on_key_press(map_global_key_press);

        let mut subs = vec![clock_tick, hotkeys];
        if self.desktop_pty.is_some() || self.installer.search_in_flight() {
            subs.push(
                iced::time::every(std::time::Duration::from_millis(33)).map(Message::Tick),
            );
        }

        Subscription::batch(subs)
    }
}

fn map_global_key_press(key: Key, mods: Modifiers) -> Option<Message> {
    Some(Message::GlobalKeyPressed(key, mods))
}

fn named_key_to_pty(key: &Key) -> Option<PtyKeyCode> {
    match key.as_ref() {
        Key::Named(Named::ArrowUp) => Some(PtyKeyCode::Up),
        Key::Named(Named::ArrowDown) => Some(PtyKeyCode::Down),
        Key::Named(Named::ArrowLeft) => Some(PtyKeyCode::Left),
        Key::Named(Named::ArrowRight) => Some(PtyKeyCode::Right),
        Key::Named(Named::Escape) => Some(PtyKeyCode::Esc),
        Key::Named(Named::Tab) => Some(PtyKeyCode::Tab),
        Key::Named(Named::Backspace) => Some(PtyKeyCode::Backspace),
        Key::Named(Named::Enter) => Some(PtyKeyCode::Enter),
        Key::Named(Named::Home) => Some(PtyKeyCode::Home),
        Key::Named(Named::End) => Some(PtyKeyCode::End),
        Key::Named(Named::Insert) => Some(PtyKeyCode::Insert),
        Key::Named(Named::Delete) => Some(PtyKeyCode::Delete),
        Key::Named(Named::PageUp) => Some(PtyKeyCode::PageUp),
        Key::Named(Named::PageDown) => Some(PtyKeyCode::PageDown),
        Key::Named(Named::Space) => Some(PtyKeyCode::Char(' ')),
        _ => None,
    }
}

fn modifiers_to_pty(mods: Modifiers) -> PtyKeyModifiers {
    let mut out = PtyKeyModifiers::empty();
    if mods.control() {
        out |= PtyKeyModifiers::CONTROL;
    }
    if mods.alt() {
        out |= PtyKeyModifiers::ALT;
    }
    if mods.shift() {
        out |= PtyKeyModifiers::SHIFT;
    }
    out
}

fn settings_card<'a>(title: String, body: Element<'a, Message>) -> Element<'a, Message> {
    use iced::widget::{column, container, text};
    use iced::Length;

    container(
        column![
            text(title).size(13),
            body,
        ]
        .spacing(6),
    )
    .padding(10)
    .width(Length::Fill)
    .style(super::retro_iced_theme::bordered_panel)
    .into()
}

fn adjust_u8_clamped(value: &mut u8, delta: i16) {
    let next = (*value as i16 + delta).clamp(0, u8::MAX as i16);
    *value = next as u8;
}

fn editor_content_to_string(content: &iced::widget::text_editor::Content) -> String {
    (0..content.line_count())
        .filter_map(|idx| content.line(idx).map(|line| line.to_string()))
        .collect::<Vec<_>>()
        .join("\n")
}
