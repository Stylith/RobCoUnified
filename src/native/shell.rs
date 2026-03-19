//! New shell architecture: sub-state structs, window manager, and the `DesktopApp` trait.
//!
//! This module defines the TARGET data model for the iced-based RobCoOS shell.
//! Nothing here is wired into the running app yet — that happens in Phase 2.
//! The existing [`super::app::RobcoNativeApp`] / egui code is untouched.

#![allow(dead_code)]

use super::app::StartMenuRenameState;
use super::desktop_app::DesktopMenuSection;
use super::desktop_search_service::NativeSpotlightResult;
use super::desktop_settings_service::load_settings_snapshot;
use super::desktop_start_menu::{StartLeaf, StartSubmenu};
use super::desktop_surface_service::DesktopSurfaceEntry;
use super::desktop_wm_widget::{DesktopWindowHost, WindowChild};
use super::message::{ContextMenuAction, DesktopIconId, Message, NavDirection};
use super::prompt::{TerminalPrompt, TerminalPromptAction, TerminalPromptKind};
use super::shared_types::DesktopWindow;
use crate::config::{set_current_user, Settings, HEADER_LINES};
use crate::core::auth::{clear_session, UserRecord};
use chrono::Local;
use iced::widget::text_input;
use iced::{Element, Subscription, Task, Theme};
use robcos_native_editor_app::EditorWindow;
use robcos_native_file_manager_app::NativeFileManagerState;
use robcos_native_settings_app::NativeSettingsPanel;
use robcos_native_terminal_app::{
    entry_for_selectable_idx, login_menu_rows_from_users, resolve_login_password_submission,
    resolve_login_selection_plan, resolve_main_menu_action, resolve_terminal_back_action,
    selectable_menu_count, terminal_runtime_defaults, terminal_screen_open_plan,
    terminal_settings_refresh_plan, LoginMenuRow, MainMenuSelectionAction, TerminalBackAction,
    TerminalBackContext, TerminalLoginPasswordPlan, TerminalLoginSelectionPlan,
    TerminalLoginState, TerminalLoginSubmitAction, TerminalNavigationState,
    TerminalScreen, TerminalScreenOpenPlan, TerminalSelectionIndexTarget, MAIN_MENU_ENTRIES,
};
use robcos_native_services::desktop_session_service::{
    authenticate_login, bind_login_identity, clear_all_sessions, login_selection_auth_method,
    login_usernames,
};
use std::collections::HashMap;
use std::path::PathBuf;

const TERMINAL_PROMPT_INPUT_ID: &str = "robcos-iced-terminal-prompt";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TerminalMenuAction {
    OpenScreen(TerminalScreen),
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
        action: Some(TerminalMenuAction::ShowStatus(
            "Hosted desktop apps stay separate from terminal mode and land in Phase 3h.",
        )),
    },
    TerminalMenuEntry {
        label: "Editor (desktop app)",
        action: Some(TerminalMenuAction::ShowStatus(
            "Hosted desktop apps stay separate from terminal mode and land in Phase 3h.",
        )),
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
        action: Some(TerminalMenuAction::OpenScreen(TerminalScreen::PtyApp)),
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
            }
            Message::PersistSnapshotRequested => {
                // Phase 3: call persist_native_shell_snapshot()
            }

            Message::TextEditorAction(action) => {
                self.editor_content.perform(action);
                self.editor.dirty = true;
            }

            Message::FileManagerCommand(cmd) => {
                use robcos_native_file_manager_app::FileManagerCommand;
                match cmd {
                    FileManagerCommand::GoUp => {
                        self.file_manager.up();
                    }
                    FileManagerCommand::OpenSelected => {
                        let action = self.file_manager.activate_selected();
                        use robcos_native_file_manager_app::FileManagerAction;
                        if let FileManagerAction::ChangedDir = action {
                            // directory was entered; view cache already refreshed
                        }
                    }
                    FileManagerCommand::ToggleHiddenFiles => {
                        // hidden files toggle is tracked in settings; handled in Phase 4
                    }
                    _ => {}
                }
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
                self.apply_terminal_screen_open_plan(terminal_screen_open_plan(
                    TerminalScreen::PtyApp,
                    0,
                    true,
                ));
                self.shell_status =
                    "CLI terminal/PTTY work remains separate and lands in Phase 3h.".to_string();
                Task::none()
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
                if matches!(screen, TerminalScreen::PtyApp) {
                    self.shell_status =
                        "CLI terminal/PTTY work remains separate and lands in Phase 3h."
                            .to_string();
                }
                Task::none()
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

        let palette = super::retro_theme::current_retro_colors();
        let fg = palette.fg.to_iced();
        let bg = palette.bg.to_iced();
        let dim = palette.dim.to_iced();

        let mut header = column![].spacing(0);
        for line in HEADER_LINES {
            header = header.push(
                text(*line)
                    .font(iced::Font::MONOSPACE)
                    .size(20)
                    .color(fg),
            );
        }

        let separator = || {
            container(Space::with_height(1))
                .width(Length::Fill)
                .style(move |_t| container::Style {
                    background: Some(iced::Background::Color(dim)),
                    ..container::Style::default()
                })
        };

        let mut layout = column![
            header,
            Space::with_height(8),
            separator(),
            Space::with_height(8),
            text(title)
                .font(iced::Font::MONOSPACE)
                .size(22)
                .color(fg),
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
                    .size(14)
                    .color(fg),
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
            .style(move |_t| container::Style {
                background: Some(iced::Background::Color(bg)),
                ..container::Style::default()
            })
            .into()
    }

    fn view_terminal_login_rows(&self) -> Element<'_, Message> {
        use iced::widget::{button, column, container, scrollable, text};
        use iced::{Border, Length};

        let palette = super::retro_theme::current_retro_colors();
        let fg = palette.fg.to_iced();
        let bg = palette.bg.to_iced();
        let dim = palette.dim.to_iced();
        let selected_bg = palette.selected_bg.to_iced();
        let selected_fg = palette.selected_fg.to_iced();
        let hovered_bg = palette.hovered_bg.to_iced();

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
                    let item_bg = if selected { selected_bg } else { bg };
                    let item_fg = if selected { selected_fg } else { fg };
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
                                .color(item_fg)
                        )
                        .on_press(Message::TerminalSelectionActivated(idx))
                        .width(Length::Fill)
                        .style(move |_t, status| {
                            use iced::widget::button::Status;
                            let bg_color = match status {
                                Status::Hovered => hovered_bg,
                                _ => item_bg,
                            };
                            button::Style {
                                background: Some(iced::Background::Color(bg_color)),
                                text_color: item_fg,
                                border: Border::default(),
                                ..button::Style::default()
                            }
                        })
                        .padding([4, 8]),
                    );
                    selectable_idx += 1;
                }
                LoginMenuRow::Exit => {
                    let idx = selectable_idx;
                    let selected = idx == self.login.selected_idx;
                    let item_bg = if selected { selected_bg } else { bg };
                    let item_fg = if selected { selected_fg } else { fg };
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
                                .color(item_fg)
                        )
                        .on_press(Message::TerminalSelectionActivated(idx))
                        .width(Length::Fill)
                        .style(move |_t, status| {
                            use iced::widget::button::Status;
                            let bg_color = match status {
                                Status::Hovered => hovered_bg,
                                _ => item_bg,
                            };
                            button::Style {
                                background: Some(iced::Background::Color(bg_color)),
                                text_color: item_fg,
                                border: Border::default(),
                                ..button::Style::default()
                            }
                        })
                        .padding([4, 8]),
                    );
                    selectable_idx += 1;
                }
            }
        }

        scrollable(col).height(Length::Fill).into()
    }

    fn view_terminal_main_menu_entries(&self) -> Element<'_, Message> {
        use iced::widget::{button, column, container, scrollable, text};
        use iced::{Border, Length};

        let palette = super::retro_theme::current_retro_colors();
        let fg = palette.fg.to_iced();
        let bg = palette.bg.to_iced();
        let dim = palette.dim.to_iced();
        let selected_bg = palette.selected_bg.to_iced();
        let selected_fg = palette.selected_fg.to_iced();
        let hovered_bg = palette.hovered_bg.to_iced();

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
            let item_bg = if selected { selected_bg } else { bg };
            let item_fg = if selected { selected_fg } else { fg };
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
                        .color(item_fg)
                )
                .on_press(Message::TerminalSelectionActivated(idx))
                .width(Length::Fill)
                .style(move |_t, status| {
                    use iced::widget::button::Status;
                    let bg_color = match status {
                        Status::Hovered => hovered_bg,
                        _ => item_bg,
                    };
                    button::Style {
                        background: Some(iced::Background::Color(bg_color)),
                        text_color: item_fg,
                        border: Border::default(),
                        ..button::Style::default()
                    }
                })
                .padding([4, 8]),
            );
            selectable_idx += 1;
        }

        scrollable(col).height(iced::Length::Fill).into()
    }

    fn view_terminal_menu_entries(
        &self,
        entries: &[TerminalMenuEntry],
        selected_idx: usize,
    ) -> Element<'_, Message> {
        use iced::widget::{button, column, container, scrollable, text};
        use iced::{Border, Length};

        let palette = super::retro_theme::current_retro_colors();
        let fg = palette.fg.to_iced();
        let bg = palette.bg.to_iced();
        let dim = palette.dim.to_iced();
        let selected_bg = palette.selected_bg.to_iced();
        let selected_fg = palette.selected_fg.to_iced();
        let hovered_bg = palette.hovered_bg.to_iced();

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
            let item_bg = if selected { selected_bg } else { bg };
            let item_fg = if selected { selected_fg } else { fg };
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
                        .color(item_fg)
                )
                .on_press(Message::TerminalSelectionActivated(idx))
                .width(Length::Fill)
                .style(move |_t, status| {
                    use iced::widget::button::Status;
                    let bg_color = match status {
                        Status::Hovered => hovered_bg,
                        _ => item_bg,
                    };
                    button::Style {
                        background: Some(iced::Background::Color(bg_color)),
                        text_color: item_fg,
                        border: Border::default(),
                        ..button::Style::default()
                    }
                })
                .padding([4, 8]),
            );
            let _ = action;
            selectable_idx += 1;
        }

        scrollable(col).height(Length::Fill).into()
    }

    fn view_terminal_prompt_overlay(&self) -> Element<'_, Message> {
        use iced::widget::{column, container, row, text, text_input, Space};
        use iced::{Alignment, Border, Length};

        let Some(prompt) = self.terminal_prompt.as_ref() else {
            return Space::with_width(Length::Shrink).into();
        };

        let palette = super::retro_theme::current_retro_colors();
        let fg = palette.fg.to_iced();
        let bg = palette.bg.to_iced();
        let dim = palette.dim.to_iced();
        let selected_bg = palette.selected_bg.to_iced();

        let input = text_input("", &prompt.buffer)
            .id(Self::terminal_prompt_id())
            .on_input(Message::LoginPasswordChanged)
            .on_submit(Message::LoginSubmitted)
            .secure(matches!(prompt.kind, TerminalPromptKind::Password))
            .font(iced::Font::MONOSPACE)
            .size(16)
            .style(move |_t, _s| iced::widget::text_input::Style {
                background: iced::Background::Color(bg),
                border: Border {
                    color: fg,
                    width: 2.0,
                    radius: 0.0.into(),
                },
                icon: fg,
                placeholder: dim,
                value: fg,
                selection: selected_bg,
            })
            .padding([8, 10]);

        let panel = container(
            column![
                text(prompt.title.as_str())
                    .font(iced::Font::MONOSPACE)
                    .size(18)
                    .color(fg),
                text(prompt.prompt.as_str())
                    .font(iced::Font::MONOSPACE)
                    .size(14)
                    .color(fg),
                input,
                text("Enter apply | Esc cancel")
                    .font(iced::Font::MONOSPACE)
                    .size(12)
                    .color(dim),
            ]
            .spacing(12)
            .width(420)
        )
        .padding(18)
        .style(move |_t| container::Style {
            background: Some(iced::Background::Color(bg)),
            border: Border {
                color: fg,
                width: 2.0,
                radius: 0.0.into(),
            },
            ..container::Style::default()
        });

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
                    DesktopWindow::PtyApp => "Terminal".to_string(),
                    _ => format!("{:?}", id),
                };

                let content: Element<'_, Message> = match id {
                    DesktopWindow::FileManager => self.view_file_manager(),
                    DesktopWindow::Editor => self.view_editor(),
                    DesktopWindow::Settings => self.view_settings_app(),
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
        use iced::widget::{button, column, container, row, scrollable, text};
        use iced::{Alignment, Length};
        use robcos_native_file_manager_app::FileManagerCommand;

        let palette = super::retro_theme::current_retro_colors();
        let fg = palette.fg.to_iced();
        let bg = palette.bg.to_iced();
        let dim = palette.dim.to_iced();
        let selected_bg = palette.selected_bg.to_iced();
        let selected_fg = palette.selected_fg.to_iced();

        // Toolbar: Up button + current path
        let cwd_str = self.file_manager.cwd.display().to_string();
        let toolbar = row![
            button(text("↑ Up").size(12).color(fg))
                .padding([2, 8])
                .style(move |_t, _s| iced::widget::button::Style {
                    background: Some(iced::Background::Color(bg)),
                    text_color: fg,
                    border: iced::Border { color: dim, width: 1.0, radius: 2.0.into() },
                    ..Default::default()
                })
                .on_press(Message::FileManagerCommand(FileManagerCommand::GoUp)),
            text(cwd_str).size(12).color(fg),
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
                let (row_bg, row_fg) = if is_selected {
                    (selected_bg, selected_fg)
                } else {
                    (bg, fg)
                };
                let label = format!("{} {}", icon, name);
                button(text(label).size(12).color(row_fg))
                    .padding([2, 12])
                    .width(Length::Fill)
                    .style(move |_t, _s| iced::widget::button::Style {
                        background: Some(iced::Background::Color(row_bg)),
                        text_color: row_fg,
                        ..Default::default()
                    })
                    .on_press(Message::FileManagerCommand(FileManagerCommand::OpenSelected))
                    .into()
            })
            .collect();

        let file_list = scrollable(
            column(file_rows).width(Length::Fill)
        )
        .width(Length::Fill)
        .height(Length::Fill);

        let body = column![toolbar, file_list]
            .width(Length::Fill)
            .height(Length::Fill);

        container(body)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(move |_t| container::Style {
                background: Some(iced::Background::Color(bg)),
                ..Default::default()
            })
            .into()
    }

    /// Editor window content — wraps iced's text_editor widget.
    fn view_editor(&self) -> Element<'_, Message> {
        use iced::widget::{column, container, text, text_editor};
        use iced::{Font, Length};

        let palette = super::retro_theme::current_retro_colors();
        let dim = palette.dim.to_iced();

        // Status bar: path + dirty indicator
        let status_str = if self.editor.dirty { "● Modified" } else { "" };
        let status = container(
            text(status_str).size(11).color(dim)
        )
        .padding([2, 8])
        .width(Length::Fill);

        let editor_widget = text_editor(&self.editor_content)
            .on_action(Message::TextEditorAction)
            .font(Font::MONOSPACE)
            .size(13.0)
            .height(Length::Fill);

        column![editor_widget, status]
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    /// Settings window content — scrollable settings panel list.
    fn view_settings_app(&self) -> Element<'_, Message> {
        use iced::widget::{column, container, scrollable, text};
        use iced::Length;
        use robcos_native_settings_app::{desktop_settings_home_rows, settings_panel_title};

        let palette = super::retro_theme::current_retro_colors();
        let fg = palette.fg.to_iced();
        let bg = palette.bg.to_iced();
        let dim = palette.dim.to_iced();

        let current_panel = self.settings_panel.unwrap_or(
            robcos_native_settings_app::desktop_settings_default_panel()
        );
        let title_str = settings_panel_title(current_panel);

        let header = container(
            text(title_str).size(14).color(fg)
        )
        .padding([8, 12])
        .width(iced::Length::Fill);

        let rows = desktop_settings_home_rows(self.session_is_admin);
        let mut items: Vec<iced::Element<'_, Message>> = Vec::new();
        for tile_row in &rows {
            for tile in tile_row {
                let label = tile.label;
                items.push(
                    container(text(label).size(12).color(fg))
                        .padding([4, 12])
                        .width(iced::Length::Fill)
                        .style(move |_t| container::Style {
                            border: iced::Border { color: dim, width: 0.0, radius: 0.0.into() },
                            ..Default::default()
                        })
                        .into()
                );
            }
        }

        let body = scrollable(
            column(items).width(Length::Fill).spacing(2)
        )
        .width(Length::Fill)
        .height(Length::Fill);

        container(column![header, body].width(Length::Fill).height(Length::Fill))
            .width(Length::Fill)
            .height(Length::Fill)
            .style(move |_t| container::Style {
                background: Some(iced::Background::Color(bg)),
                ..Default::default()
            })
            .into()
    }

    /// Fallback placeholder for windows without a dedicated view yet.
    fn view_window_placeholder(
        &self,
        id: DesktopWindow,
        fg: iced::Color,
        dim: iced::Color,
        bg: iced::Color,
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
        .style(move |_t| container::Style {
            background: Some(iced::Background::Color(bg)),
            ..container::Style::default()
        })
        .into()
    }

    /// Render the desktop surface: black background with builtin icon column on the left.
    fn view_surface_icons(&self) -> Element<'_, Message> {
        use super::desktop_surface_service::desktop_builtin_icons;
        use super::message::DesktopIconId;
        use iced::widget::{button, column, container, row, text};
        use iced::{Border, Length};

        let palette = super::retro_theme::current_retro_colors();
        let fg = palette.fg.to_iced();
        let bg = palette.bg.to_iced();
        let selected_bg = palette.selected_bg.to_iced();
        let selected_fg = palette.selected_fg.to_iced();

        let mut icon_col = column![].spacing(4).padding([8, 4]);

        for entry in desktop_builtin_icons() {
            let icon_id = DesktopIconId::Builtin(entry.key);
            let is_selected = self.surface.selected_icon.as_ref() == Some(&icon_id);

            let (lbl_bg, lbl_fg) = if is_selected {
                (selected_bg, selected_fg)
            } else {
                (bg, fg)
            };

            let id_clone = icon_id.clone();
            let icon_btn = button(
                column![
                    // ASCII art glyph as icon proxy
                    container(
                        text(entry.ascii).size(10).color(lbl_fg)
                    )
                    .width(64)
                    .style(move |_t| container::Style {
                        background: Some(iced::Background::Color(lbl_bg)),
                        border: Border { color: fg, width: 1.0, radius: 0.0.into() },
                        ..container::Style::default()
                    })
                    .padding(4),
                    // Label below
                    text(entry.label).size(10).color(lbl_fg),
                ]
                .spacing(2)
                .width(68)
            )
            .on_press(Message::DesktopIconClicked { id: id_clone, shift: false })
            .style(move |_t, _s| button::Style {
                background: None,
                ..button::Style::default()
            })
            .padding(2);

            icon_col = icon_col.push(icon_btn);
        }

        // Full-screen black background with icons pinned left.
        container(
            row![icon_col]
        )
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

    /// Render the spotlight / search overlay, centered on screen.
    fn view_spotlight(&self) -> Element<'_, Message> {
        use iced::widget::{
            button, column, container, mouse_area, row, scrollable, text, text_input, Space,
        };
        use iced::{Alignment, Border, Length};

        let palette = super::retro_theme::current_retro_colors();
        let fg = palette.fg.to_iced();
        let bg = palette.bg.to_iced();
        let dim = palette.dim.to_iced();
        let panel_bg = palette.panel.to_iced();
        let selected_bg = palette.selected_bg.to_iced();
        let selected_fg = palette.selected_fg.to_iced();
        let hovered_bg = palette.hovered_bg.to_iced();

        // ── Search input ──────────────────────────────────────────────────────
        let search_input = text_input("> Search…", &self.spotlight.query)
            .on_input(Message::SpotlightQueryChanged)
            .on_submit(Message::SpotlightActivateSelected)
            .size(18)
            .style(move |_t, _s| iced::widget::text_input::Style {
                background: iced::Background::Color(bg),
                border: Border { color: fg, width: 2.0, radius: 0.0.into() },
                icon: fg,
                placeholder: dim,
                value: fg,
                selection: selected_bg,
            })
            .padding([8, 12]);

        // ── Tab bar ───────────────────────────────────────────────────────────
        let tabs = ["All", "Apps", "Documents", "Files"];
        let mut tab_row = row![].spacing(0);
        for (i, tab_label) in tabs.iter().enumerate() {
            let is_sel = self.spotlight.tab == i as u8;
            let (tab_bg, tab_fg) = if is_sel {
                (selected_bg, selected_fg)
            } else {
                (panel_bg, fg)
            };
            tab_row = tab_row.push(
                button(text(*tab_label).size(12).color(tab_fg))
                    .on_press(Message::SpotlightTabChanged(i as u8))
                    .style(move |_t, _s| button::Style {
                        background: Some(iced::Background::Color(tab_bg)),
                        text_color: tab_fg,
                        border: Border { color: fg, width: 1.0, radius: 0.0.into() },
                        ..button::Style::default()
                    })
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
                let (item_bg, item_fg) = if is_sel {
                    (selected_bg, selected_fg)
                } else {
                    (bg, fg)
                };
                let category_str = format!("[{:?}]", result.category);
                results_col = results_col.push(
                    button(
                        row![
                            text(result.name.as_str()).size(13).color(item_fg).width(Length::Fill),
                            text(category_str).size(11).color(if is_sel { selected_fg } else { dim }),
                        ]
                        .spacing(8)
                        .padding([4, 10])
                    )
                    .on_press(Message::SpotlightActivateSelected)
                    .width(Length::Fill)
                    .style(move |_t, status| {
                        use iced::widget::button::Status;
                        let bg_color = match status {
                            Status::Hovered => hovered_bg,
                            _ => item_bg,
                        };
                        button::Style {
                            background: Some(iced::Background::Color(bg_color)),
                            text_color: item_fg,
                            border: Border::default(),
                            ..button::Style::default()
                        }
                    })
                    .padding(0)
                );
            }
        }

        let results_scroll = scrollable(results_col)
            .height(300);

        // ── Compose panel ─────────────────────────────────────────────────────
        let panel = container(
            column![
                search_input,
                tab_row,
                container(Space::with_height(1))
                    .width(Length::Fill)
                    .style(move |_t| container::Style {
                        background: Some(iced::Background::Color(dim)),
                        ..container::Style::default()
                    }),
                results_scroll,
            ]
            .spacing(0)
            .width(600)
        )
        .style(move |_t| container::Style {
            background: Some(iced::Background::Color(bg)),
            border: Border { color: fg, width: 2.0, radius: 0.0.into() },
            ..container::Style::default()
        });

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
        use iced::{Border, Length};

        let palette = super::retro_theme::current_retro_colors();
        let fg = palette.fg.to_iced();
        let bg = palette.bg.to_iced();
        let panel_bg = palette.panel.to_iced();
        let dim = palette.dim.to_iced();
        let selected_bg = palette.selected_bg.to_iced();
        let selected_fg = palette.selected_fg.to_iced();
        let hovered_bg = palette.hovered_bg.to_iced();

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
            .style(move |_t| container::Style {
                background: Some(iced::Background::Color(panel_bg)),
                border: Border { color: fg, width: 0.0, radius: 0.0.into() },
                ..container::Style::default()
            })
        );

        // Separator
        root_col = root_col.push(
            container(Space::with_height(1))
                .width(Length::Fill)
                .style(move |_t| container::Style {
                    background: Some(iced::Background::Color(dim)),
                    ..container::Style::default()
                })
        );

        for (vis_idx, root_slot) in START_ROOT_VIS_ROWS.iter().enumerate() {
            match root_slot {
                None => {
                    // Separator row
                    root_col = root_col.push(
                        container(Space::with_height(1))
                            .width(Length::Fill)
                            .style(move |_t| container::Style {
                                background: Some(iced::Background::Color(dim)),
                                ..container::Style::default()
                            })
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

                    let (item_bg, item_fg) = if is_sel {
                        (selected_bg, selected_fg)
                    } else {
                        (bg, fg)
                    };

                    root_col = root_col.push(
                        button(
                            row![
                                text(disp).size(13).color(item_fg),
                            ]
                            .padding([4, 8])
                        )
                        .on_press(Message::StartMenuSelectRoot(idx))
                        .width(Length::Fill)
                        .style(move |_t, status| {
                            use iced::widget::button::Status;
                            let bg = match status {
                                Status::Hovered => Some(iced::Background::Color(hovered_bg)),
                                _ => Some(iced::Background::Color(item_bg)),
                            };
                            button::Style {
                                background: bg,
                                text_color: item_fg,
                                border: Border::default(),
                                ..button::Style::default()
                            }
                        })
                        .padding(0)
                    );
                }
            }
            let _ = vis_idx;
        }

        let root_panel = container(root_col)
            .style(move |_t| container::Style {
                background: Some(iced::Background::Color(bg)),
                border: Border { color: fg, width: 2.0, radius: 0.0.into() },
                ..container::Style::default()
            });

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
                        text(*label).size(13).color(fg).width(Length::Fill)
                    )
                    .on_press(Message::StartMenuNavigate(super::message::NavDirection::Right))
                    .width(Length::Fill)
                    .style(move |_t, status| {
                        use iced::widget::button::Status;
                        let bg_color = match status {
                            Status::Hovered | Status::Pressed => hovered_bg,
                            _ => bg,
                        };
                        button::Style {
                            background: Some(iced::Background::Color(bg_color)),
                            text_color: fg,
                            border: Border::default(),
                            ..button::Style::default()
                        }
                    })
                    .padding([4, 8])
                );
            }
            let _ = sub;
            Some(
                container(sub_col)
                    .style(move |_t| container::Style {
                        background: Some(iced::Background::Color(bg)),
                        border: Border { color: fg, width: 2.0, radius: 0.0.into() },
                        ..container::Style::default()
                    })
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
                    .style(move |_t| container::Style {
                        background: Some(iced::Background::Color(panel_bg)),
                        ..container::Style::default()
                    }),
                container(Space::with_height(1))
                    .width(Length::Fill)
                    .style(move |_t| container::Style {
                        background: Some(iced::Background::Color(dim)),
                        ..container::Style::default()
                    }),
            ].spacing(0).width(200);
            leaf_col = leaf_col.push(
                text("(Loading…)").size(12).color(dim)
            );
            Some(
                container(leaf_col)
                    .style(move |_t| container::Style {
                        background: Some(iced::Background::Color(bg)),
                        border: Border { color: fg, width: 2.0, radius: 0.0.into() },
                        ..container::Style::default()
                    })
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

    /// Return the application theme.
    ///
    /// Phase 2: uses iced's built-in Dark theme.
    /// Phase 4: replace with a custom `RetroTheme` that applies the full palette.
    pub fn theme(&self) -> Theme {
        Theme::Dark
    }

    /// Return active subscriptions.
    ///
    /// Phase 3b/3e: clock tick + global keyboard shortcuts (Cmd+Space, Escape).
    /// Phase 3g: also add PTY output stream.
    pub fn subscription(&self) -> Subscription<Message> {
        use iced::keyboard::{self, key::Named, Key, Modifiers};

        let tick = iced::time::every(std::time::Duration::from_secs(30))
            .map(Message::Tick);

        let hotkeys = keyboard::on_key_press(|key, mods| {
            match key {
                Key::Named(Named::Space) if mods.contains(Modifiers::COMMAND) => {
                    Some(Message::OpenSpotlight)
                }
                Key::Named(Named::Escape) => Some(Message::CloseSpotlight),
                Key::Named(Named::ArrowLeft) => Some(Message::TerminalBackRequested),
                Key::Named(Named::ArrowUp) => {
                    Some(Message::TerminalNavigate(NavDirection::Up))
                }
                Key::Named(Named::ArrowDown) => {
                    Some(Message::TerminalNavigate(NavDirection::Down))
                }
                Key::Named(Named::Enter) => {
                    Some(Message::TerminalActivateSelected)
                }
                _ => None,
            }
        });

        Subscription::batch([tick, hotkeys])
    }
}
