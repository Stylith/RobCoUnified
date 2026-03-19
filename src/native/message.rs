//! Central message / event bus for the iced-based RobCoOS shell.
//!
//! Every user action and system event flows through [`Message`]. The shell's
//! `update()` dispatches each variant to the appropriate sub-state handler.
//! This replaces the direct method-call style used in the egui implementation.

#![allow(dead_code)]

use super::desktop_app::{DesktopMenuAction, DesktopShellAction};
use super::desktop_start_menu::{StartLeaf, StartSubmenu};
use super::desktop_search_service::NativeSpotlightResult;
use super::desktop_surface_service::DesktopSurfaceEntry;
use super::shared_types::DesktopWindow;
use crate::config::DesktopIconSortMode;
use robcos_native_editor_app::{EditorCommand, EditorTextCommand};
use robcos_native_file_manager_app::FileManagerCommand;
use robcos_native_settings_app::NativeSettingsPanel;
use std::path::PathBuf;
use std::time::Instant;

// ── Helper enums ─────────────────────────────────────────────────────────────

/// Keyboard navigation direction, used in start menu and spotlight.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NavDirection {
    Up,
    Down,
    Left,
    Right,
    Tab,
    ShiftTab,
}

/// Which button in a window's title-bar chrome was pressed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowHeaderButton {
    Close,
    Minimize,
    Maximize,
    Restore,
}

/// Stable identity of a desktop icon across builtin, surface-entry, and shortcut categories.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DesktopIconId {
    Builtin(&'static str),
    Surface(String),
    Shortcut(usize),
}

/// Outcome of an async file operation (copy, move, delete, mkdir, etc.).
#[derive(Debug, Clone)]
pub enum FileOpResult {
    Ok,
    Err(String),
}

/// All right-click context-menu actions on the desktop surface and start menu.
///
/// This mirrors `app::ContextMenuAction` (which is `pub(super)` on `RobcoNativeApp`).
/// During Phase 3 the duplicate in `app.rs` will be removed in favour of this one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContextMenuAction {
    // Desktop icon actions
    Open,
    OpenWith,
    Rename,
    Cut,
    Copy,
    Paste,
    Duplicate,
    Delete,
    Properties,
    // Desktop surface actions
    PasteToDesktop,
    NewFolder,
    ChangeAppearance,
    OpenSettings,
    // Generic text-area actions
    GenericCopy,
    GenericPaste,
    GenericSelectAll,
    // Desktop icon sort/layout
    SortDesktopIcons(DesktopIconSortMode),
    ToggleSnapToGrid,
    // Shortcut actions
    LaunchShortcut(String),
    OpenShortcutProperties(usize),
    DeleteShortcut(usize),
    CreateShortcut {
        label: String,
        /// Raw action string that will be resolved into a DesktopShellAction.
        action_key: String,
    },
    // Start-menu entry management
    RenameStartMenuEntry { target_key: String, current_name: String },
    RemoveStartMenuEntry { target_key: String, name: String },
    // Desktop item (surface file/folder) actions
    OpenDesktopItem(PathBuf),
    OpenDesktopItemWith(PathBuf),
    RenameDesktopItem(PathBuf),
    DeleteDesktopItem(PathBuf),
    OpenDesktopItemProperties(PathBuf),
}

// ── Message ───────────────────────────────────────────────────────────────────

/// Top-level message type. Every interaction in the shell produces a `Message`.
///
/// The shell's `update(&mut self, msg: Message) -> Command<Message>` dispatches
/// each variant to the relevant sub-state handler. Handlers may return follow-up
/// messages via `Command::perform` (async) or `Command::none` (sync).
#[derive(Debug, Clone)]
pub enum Message {
    // ── Window management ────────────────────────────────────────────────────
    /// Open (or un-minimise) a desktop window.
    OpenWindow(DesktopWindow),
    /// Close a desktop window entirely.
    CloseWindow(DesktopWindow),
    /// Minimise a desktop window to the taskbar.
    MinimizeWindow(DesktopWindow),
    /// Toggle maximised state of a desktop window.
    ToggleMaximizeWindow(DesktopWindow),
    /// Bring a window to the front and give it focus.
    FocusWindow(DesktopWindow),
    /// User dragged a window's title bar by (dx, dy) pixels.
    WindowTitleBarDragged { window: DesktopWindow, dx: f32, dy: f32 },
    /// User dragged a window resize handle; new size requested.
    WindowResizeHandleDragged { window: DesktopWindow, w: f32, h: f32 },
    /// One of the title-bar chrome buttons was clicked.
    WindowHeaderButtonClicked { window: DesktopWindow, button: WindowHeaderButton },
    /// The WM widget finished dragging a window to a new position.
    WindowMoved { window: DesktopWindow, x: f32, y: f32 },
    /// The WM widget finished resizing a window to a new size.
    WindowResized { window: DesktopWindow, w: f32, h: f32 },

    // ── Taskbar ──────────────────────────────────────────────────────────────
    TaskbarWindowClicked(DesktopWindow),
    StartButtonClicked,

    // ── Start menu ───────────────────────────────────────────────────────────
    StartMenuClose,
    StartMenuSelectRoot(usize),
    StartMenuSelectSystem(usize),
    StartMenuSelectLeaf(usize),
    StartMenuOpenSubmenu(StartSubmenu),
    StartMenuOpenLeaf(StartLeaf),
    StartMenuActivate,
    StartMenuNavigate(NavDirection),
    /// Commit a pending start-menu entry rename.
    StartMenuRenameCommitted { original_name: String, new_name: String },

    // ── Spotlight / search ───────────────────────────────────────────────────
    OpenSpotlight,
    CloseSpotlight,
    SpotlightQueryChanged(String),
    SpotlightTabChanged(u8),
    SpotlightNavigate(NavDirection),
    SpotlightActivateSelected,
    /// Async: search service returned updated results.
    SpotlightResultsReady(Vec<NativeSpotlightResult>),

    // ── Menu bar ─────────────────────────────────────────────────────────────
    /// Action from the top menu bar (File, Edit, Format, View, Window, Help).
    MenuAction(DesktopMenuAction),
    /// High-level shell action (open window, launch app, open path, etc.).
    ShellAction(DesktopShellAction),

    // ── Desktop surface ───────────────────────────────────────────────────────
    /// Single click on a desktop icon.
    DesktopIconClicked { id: DesktopIconId, shift: bool },
    /// Double click on a desktop icon (open / launch).
    DesktopIconDoubleClicked(DesktopIconId),
    /// User started dragging an icon.
    DesktopIconDragStarted(DesktopIconId),
    /// Icon drag in progress — pointer moved to (x, y).
    DesktopIconDragged { id: DesktopIconId, x: f32, y: f32 },
    /// Icon drag released at (x, y).
    DesktopIconDropped { id: DesktopIconId, x: f32, y: f32 },
    /// Right-click on the empty desktop background at (x, y).
    DesktopBackgroundRightClicked { x: f32, y: f32 },
    /// A context menu item was chosen.
    DesktopContextMenuAction(ContextMenuAction),
    /// Clicked empty space — deselect any selected icon.
    DesktopSelectionCleared,
    /// OS drag-and-drop: files landed on the desktop.
    FilesDroppedOnDesktop(Vec<PathBuf>),
    /// User opened the wallpaper picker.
    WallpaperPickerOpened,
    /// User picked a wallpaper file.
    WallpaperSelected(PathBuf),
    /// User opened the icon picker for a shortcut.
    IconPickerOpened { shortcut_idx: usize },
    /// User picked an icon file for a shortcut.
    ShortcutIconSelected { shortcut_idx: usize, path: PathBuf },
    /// Async: directory scan for desktop surface entries completed.
    DesktopSurfaceScanned(Vec<DesktopSurfaceEntry>),

    // ── Session / auth ────────────────────────────────────────────────────────
    LoginUsernameSelected(String),
    LoginPasswordChanged(String),
    LoginSubmitted,
    LogoutRequested,
    SessionSwitchRequested(usize),

    // ── Terminal / PTY ────────────────────────────────────────────────────────
    /// Switch the shell between terminal mode and desktop mode.
    DesktopModeToggled,
    /// Move the current full-screen terminal UI selection.
    TerminalNavigate(NavDirection),
    /// Activate the currently selected full-screen terminal UI item.
    TerminalActivateSelected,
    /// Navigate back within the full-screen terminal UI.
    TerminalBackRequested,
    /// Close the active full-screen terminal prompt without submitting it.
    TerminalPromptCancelled,
    /// User input bytes to send to the PTY.
    PtyInput(Vec<u8>),
    /// Async: PTY produced output bytes.
    PtyOutput(Vec<u8>),
    /// Async: PTY child process updated its title string.
    PtyTitleChanged(String),
    /// Async: PTY child process exited.
    PtyExited,
    /// Activate a specific selectable row in the full-screen terminal UI.
    TerminalSelectionActivated(usize),

    // ── Settings ──────────────────────────────────────────────────────────────
    SettingsPanelChanged(NativeSettingsPanel),
    SettingsSaveRequested,
    SettingsCancelRequested,

    // ── Editor ────────────────────────────────────────────────────────────────
    EditorCommand(EditorCommand),
    EditorTextCommand(EditorTextCommand),
    EditorFileOpenRequested(PathBuf),
    EditorSaveAsRequested,

    // ── File manager ─────────────────────────────────────────────────────────
    FileManagerCommand(FileManagerCommand),
    /// Async: file drop received by the file manager.
    FileManagerDropReceived { paths: Vec<PathBuf>, target: Option<PathBuf> },

    // ── Async results ─────────────────────────────────────────────────────────
    /// A background file operation finished.
    FileOperationCompleted(FileOpResult),
    /// Async: an icon PNG was loaded from disk or build-time embed.
    IconImageLoaded { name: String, size: u16, data: Vec<u8> },
    /// Async: wallpaper image decoded and ready for upload to GPU.
    WallpaperImageLoaded(Vec<u8>),

    // ── System ────────────────────────────────────────────────────────────────
    /// Settings file on disk changed; reload snapshot.
    SettingsFileChanged,
    /// Persist current session/settings to disk.
    PersistSnapshotRequested,
    /// Regular tick for cursor blink, flash timers, idle checks, etc.
    Tick(Instant),
}
