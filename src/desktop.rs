use anyhow::{anyhow, Result};
use chrono::Local;
use crossterm::event::{
    self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind, KeyModifiers,
    MouseButton, MouseEventKind,
};
use crossterm::execute;
use ratatui::{
    layout::Rect,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};
use std::cmp::Ordering;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use crate::apps::edit_menus_menu;
use crate::auth::{is_admin, user_management_menu};
use crate::config::{
    get_settings, load_apps, load_categories, load_games, load_networks, persist_settings,
    update_settings, CliAcsMode, CliColorMode, DesktopCliProfiles, DesktopFileManagerSettings,
    DesktopPtyProfileSettings, FileManagerSortMode, FileManagerTextOpenMode, FileManagerViewMode,
    OpenMode, WallpaperSizeMode, THEMES,
};
use crate::documents;
use crate::launcher::json_to_cmd;
use crate::ui::{
    dim_style, flash_message, normal_style, sel_style, session_switch_scope, title_style, Term,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DesktopExit {
    ReturnToTerminal,
    Logout,
    Shutdown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StartLaunch {
    ProgramInstaller,
    Terminal,
    Settings,
    FileManager,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum StartAction {
    None,
    Launch(StartLaunch),
    LaunchCommand { title: String, cmd: Vec<String> },
    LaunchNukeCodes,
    OpenDocumentLogs,
    OpenDocumentCategory { name: String, path: PathBuf },
    ReturnToTerminal,
    Logout,
    Shutdown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StartSubmenu {
    Programs,
    System,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StartProgramsLeaf {
    Applications,
    Documents,
    Network,
    Games,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StartHoverTarget {
    Submenu(StartSubmenu),
    Leaf(StartProgramsLeaf),
}

#[derive(Debug, Clone, Copy)]
struct WinRect {
    x: i32,
    y: i32,
    w: u16,
    h: u16,
}

impl WinRect {
    fn contains(self, x: u16, y: u16) -> bool {
        let x0 = self.x.max(0) as u16;
        let y0 = self.y.max(0) as u16;
        let x1 = x0.saturating_add(self.w);
        let y1 = y0.saturating_add(self.h);
        x >= x0 && x < x1 && y >= y0 && y < y1
    }

    fn to_rect(self) -> Rect {
        Rect {
            x: self.x.max(0) as u16,
            y: self.y.max(0) as u16,
            width: self.w,
            height: self.h,
        }
    }
}

#[derive(Debug, Clone)]
struct FileEntry {
    name: String,
    path: PathBuf,
    is_dir: bool,
}

#[derive(Debug, Clone)]
struct FileTreeItem {
    line: String,
    path: Option<PathBuf>,
}

#[derive(Debug, Clone)]
struct FileManagerState {
    cwd: PathBuf,
    tabs: Vec<PathBuf>,
    active_tab: usize,
    all_entries: Vec<FileEntry>,
    entries: Vec<FileEntry>,
    selected: usize,
    scroll: usize,
    tree_selected: usize,
    tree_scroll: usize,
    tree_focus: bool,
    search_query: String,
    search_mode: bool,
}

#[derive(Debug, Clone)]
struct FileManagerSettingsState {
    selected: usize,
}

#[derive(Debug, Clone, Copy)]
enum FileManagerOpenRequest {
    Builtin,
    External,
}

const FILE_MANAGER_HEADER_ROWS: u16 = 4;
const FILE_MANAGER_GRID_CELL_WIDTH: u16 = 18;
const FILE_MANAGER_GRID_CELL_HEIGHT: u16 = 3;
const FILE_MANAGER_TREE_MIN_WIDTH: u16 = 16;
const FILE_MANAGER_TREE_MAX_WIDTH: u16 = 28;
const FILE_MANAGER_TREE_MIN_TOTAL_WIDTH: u16 = 50;
const FILE_MANAGER_TREE_GAP: u16 = 1;
const FILE_MANAGER_ENTRY_MIN_WIDTH: u16 = 16;

impl FileManagerState {
    fn new() -> Self {
        let cwd = dirs::home_dir()
            .or_else(|| std::env::current_dir().ok())
            .unwrap_or_else(|| PathBuf::from("."));
        let all_entries = read_entries(&cwd, &get_settings().desktop_file_manager);
        Self {
            cwd: cwd.clone(),
            tabs: vec![cwd],
            active_tab: 0,
            all_entries: all_entries.clone(),
            entries: all_entries,
            selected: 0,
            scroll: 0,
            tree_selected: 0,
            tree_scroll: 0,
            tree_focus: false,
            search_query: String::new(),
            search_mode: false,
        }
    }

    fn sync_active_tab_path(&mut self) {
        if self.tabs.is_empty() {
            self.tabs.push(self.cwd.clone());
            self.active_tab = 0;
            return;
        }
        self.active_tab = self.active_tab.min(self.tabs.len().saturating_sub(1));
        self.tabs[self.active_tab] = self.cwd.clone();
    }

    fn set_cwd(&mut self, path: PathBuf) {
        self.cwd = path;
        self.sync_active_tab_path();
    }

    fn refresh(&mut self) {
        self.sync_active_tab_path();
        self.all_entries = read_entries(&self.cwd, &get_settings().desktop_file_manager);
        self.apply_search_filter();
        if self.selected >= self.entries.len() && !self.entries.is_empty() {
            self.selected = self.entries.len() - 1;
        }
        self.scroll = self.scroll.min(self.entries.len().saturating_sub(1));
        if self.entries.is_empty() {
            self.selected = 0;
            self.scroll = 0;
        }
        let tree_items = file_manager_tree_items(
            &self.cwd,
            get_settings().desktop_file_manager.show_hidden_files,
        );
        if tree_items.is_empty() {
            self.tree_selected = 0;
            self.tree_scroll = 0;
        } else {
            self.tree_selected = self
                .tree_selected
                .min(tree_items.len().saturating_sub(1));
            if tree_items
                .get(self.tree_selected)
                .and_then(|item| item.path.as_ref())
                .is_none()
            {
                self.tree_selected = file_manager_tree_selected_for_cwd(&tree_items, &self.cwd);
            }
            self.tree_scroll = self.tree_scroll.min(tree_items.len().saturating_sub(1));
        }
    }

    fn activate_selected(
        &mut self,
        request: FileManagerOpenRequest,
    ) -> Option<(PathBuf, FileManagerOpenRequest)> {
        let Some(entry) = self.entries.get(self.selected) else {
            return None;
        };
        if entry.is_dir {
            if matches!(request, FileManagerOpenRequest::External) {
                return None;
            }
            self.set_cwd(entry.path.clone());
            self.selected = 0;
            self.scroll = 0;
            self.refresh();
            return None;
        }
        Some((entry.path.clone(), request))
    }

    fn up(&mut self) {
        self.selected = self.selected.saturating_sub(1);
    }

    fn down(&mut self) {
        if self.selected + 1 < self.entries.len() {
            self.selected += 1;
        }
    }

    fn parent(&mut self) {
        if let Some(parent) = self.cwd.parent() {
            self.set_cwd(parent.to_path_buf());
            self.selected = 0;
            self.scroll = 0;
            self.refresh();
        }
    }

    fn open_tab(&mut self, path: PathBuf) {
        self.tabs.push(path.clone());
        self.active_tab = self.tabs.len().saturating_sub(1);
        self.cwd = path;
        self.selected = 0;
        self.scroll = 0;
        self.tree_focus = false;
        self.search_mode = false;
        self.refresh();
    }

    fn open_tab_here(&mut self) {
        self.open_tab(self.cwd.clone());
    }

    fn switch_to_tab(&mut self, idx: usize) -> bool {
        if idx >= self.tabs.len() {
            return false;
        }
        if idx == self.active_tab {
            return true;
        }
        self.active_tab = idx;
        self.cwd = self.tabs[idx].clone();
        self.selected = 0;
        self.scroll = 0;
        self.tree_focus = false;
        self.search_mode = false;
        self.refresh();
        true
    }

    fn switch_tab_relative(&mut self, forward: bool) -> bool {
        if self.tabs.len() <= 1 {
            return false;
        }
        let next = if forward {
            (self.active_tab + 1) % self.tabs.len()
        } else if self.active_tab == 0 {
            self.tabs.len().saturating_sub(1)
        } else {
            self.active_tab - 1
        };
        self.switch_to_tab(next)
    }

    fn close_active_tab(&mut self) -> bool {
        if self.tabs.len() <= 1 {
            return false;
        }
        self.tabs.remove(self.active_tab);
        if self.active_tab >= self.tabs.len() {
            self.active_tab = self.tabs.len().saturating_sub(1);
        }
        self.cwd = self.tabs[self.active_tab].clone();
        self.selected = 0;
        self.scroll = 0;
        self.tree_focus = false;
        self.search_mode = false;
        self.refresh();
        true
    }

    fn tree_move_selection(&mut self, forward: bool) {
        let items = file_manager_tree_items(
            &self.cwd,
            get_settings().desktop_file_manager.show_hidden_files,
        );
        if items.is_empty() {
            self.tree_selected = 0;
            self.tree_scroll = 0;
            return;
        }
        self.tree_selected = file_manager_step_tree_selection(&items, self.tree_selected, forward)
            .unwrap_or_else(|| file_manager_tree_selected_for_cwd(&items, &self.cwd));
    }

    fn open_selected_tree_path(&mut self) -> bool {
        let items = file_manager_tree_items(
            &self.cwd,
            get_settings().desktop_file_manager.show_hidden_files,
        );
        let Some(path) = items
            .get(self.tree_selected)
            .and_then(|item| item.path.clone())
        else {
            return false;
        };
        self.set_cwd(path);
        self.selected = 0;
        self.scroll = 0;
        self.refresh();
        true
    }

    fn apply_search_filter(&mut self) {
        let q = self.search_query.trim().to_ascii_lowercase();
        if q.is_empty() {
            self.entries = self.all_entries.clone();
            return;
        }
        self.entries = self
            .all_entries
            .iter()
            .filter(|entry| entry.name.to_ascii_lowercase().contains(&q))
            .cloned()
            .collect();
    }

    fn update_search_query(&mut self, query: String) {
        self.search_query = query;
        self.apply_search_filter();
        if self.selected >= self.entries.len() {
            self.selected = self.entries.len().saturating_sub(1);
        }
        if self.entries.is_empty() {
            self.selected = 0;
            self.scroll = 0;
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DesktopProfileSlot {
    Default,
    Calcurse,
    SpotifyPlayer,
    Ranger,
    Reddit,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum DesktopSettingsPanel {
    Home,
    Appearance,
    General,
    CliDisplay,
    Wallpapers,
    WallpaperSize,
    WallpaperChoose,
    WallpaperDelete,
    WallpaperAdd,
    WallpaperPaste,
    ProfileList,
    ProfileEdit(DesktopProfileSlot),
    CustomProfileList,
    CustomProfileEdit(String),
    CustomProfileAdd,
    About,
}

struct DesktopSettingsState {
    panel: DesktopSettingsPanel,
    selected: usize,
    hovered: Option<usize>,
    is_admin: bool,
    custom_profile_input: String,
    custom_profile_error: Option<String>,
    wallpaper_name_input: String,
    wallpaper_path_input: String,
    wallpaper_art_input: String,
    wallpaper_error: Option<String>,
}

impl Default for DesktopSettingsState {
    fn default() -> Self {
        Self {
            panel: DesktopSettingsPanel::Home,
            selected: 0,
            hovered: None,
            is_admin: false,
            custom_profile_input: String::new(),
            custom_profile_error: None,
            wallpaper_name_input: String::new(),
            wallpaper_path_input: String::new(),
            wallpaper_art_input: String::new(),
            wallpaper_error: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DesktopSettingsHomeItem {
    Appearance,
    General,
    CliDisplay,
    CliProfiles,
    EditMenus,
    UserManagement,
    About,
    Close,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DesktopSettingsAction {
    None,
    CloseWindow,
    OpenEditMenus,
    OpenUserManagement,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum WallpaperRowAction {
    None,
    Set(String),
    OpenSizeMenu,
    OpenChooseMenu,
    OpenDeleteMenu,
    AddCustom,
    Back,
}

#[derive(Debug, Clone)]
struct WallpaperRow {
    label: String,
    action: WallpaperRowAction,
}

struct PtyWindowState {
    session: crate::pty::PtySession,
    min_w: u16,
    min_h: u16,
    mouse_passthrough: bool,
    manual_key: String,
}

impl Drop for PtyWindowState {
    fn drop(&mut self) {
        self.session.terminate();
    }
}

enum WindowKind {
    FileManager(FileManagerState),
    FileManagerSettings(FileManagerSettingsState),
    DesktopSettings(DesktopSettingsState),
    PtyApp(PtyWindowState),
}

struct DesktopWindow {
    id: u64,
    title: String,
    rect: WinRect,
    restore_rect: Option<WinRect>,
    minimized: bool,
    maximized: bool,
    kind: WindowKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ResizeCorner {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

#[derive(Debug, Clone, Copy)]
enum DragAction {
    Move {
        dx: i32,
        dy: i32,
    },
    Resize {
        corner: ResizeCorner,
        origin: WinRect,
    },
}

#[derive(Debug, Clone, Copy)]
struct DragState {
    window_id: u64,
    action: DragAction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ClickTarget {
    DesktopIconMyComputer,
    FileEntry { window_id: u64, row: usize },
}

#[derive(Debug, Clone, Copy)]
struct LastClick {
    target: ClickTarget,
    at: Instant,
}

#[derive(Debug, Clone, Copy)]
struct TaskButton {
    window_id: u64,
    rect: Rect,
}

struct TaskbarLayout {
    buttons: Vec<TaskButton>,
    prev_rect: Option<Rect>,
    next_rect: Option<Rect>,
    can_scroll_left: bool,
    can_scroll_right: bool,
}

impl TaskbarLayout {
    fn empty() -> Self {
        Self {
            buttons: Vec::new(),
            prev_rect: None,
            next_rect: None,
            can_scroll_left: false,
            can_scroll_right: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TopMenuKind {
    App,
    File,
    Edit,
    View,
    Window,
    Help,
}

#[derive(Debug, Clone)]
struct TopMenuState {
    open: Option<TopMenuKind>,
    hover_label: Option<TopMenuKind>,
    hover_item: Option<usize>,
}

impl Default for TopMenuState {
    fn default() -> Self {
        Self {
            open: None,
            hover_label: None,
            hover_item: None,
        }
    }
}

#[derive(Debug, Clone)]
struct TopMenuLabel {
    kind: TopMenuKind,
    text: String,
    rect: Rect,
}

#[derive(Debug, Clone)]
struct TopMenuItem {
    label: String,
    shortcut: Option<String>,
    action: TopMenuAction,
    enabled: bool,
}

#[derive(Debug, Clone)]
enum TopMenuAction {
    None,
    OpenStart,
    OpenSettings,
    OpenProgramInstaller,
    OpenFileManager,
    OpenFileManagerSettings,
    NewFileManagerTab,
    CloseFileManagerTab,
    NextFileManagerTab,
    PrevFileManagerTab,
    OpenSelectedFileBuiltin,
    OpenSelectedFileExternal,
    OpenEditMenus,
    OpenUserManagement,
    ShowAppShortcuts,
    CloseFocusedWindow,
    MinimizeFocusedWindow,
    ToggleMaxFocusedWindow,
    FocusWindow(u64),
    OpenAppManual,
    OpenUserManual,
}

#[derive(Debug, Clone)]
struct HelpPopupState {
    title: String,
    lines: Vec<String>,
    scroll: usize,
}

#[derive(Debug, Clone, Default)]
struct SpotlightState {
    open: bool,
    query: String,
    selected: usize,
}

#[derive(Debug, Clone)]
struct SpotlightItem {
    label: String,
    action: TopMenuAction,
}

#[derive(Debug, Clone)]
struct StartLeafItem {
    label: String,
    action: StartAction,
}

#[derive(Debug, Clone)]
struct StartState {
    open: bool,
    selected_root: usize,
    selected_program: usize,
    selected_system: usize,
    selected_leaf_apps: usize,
    selected_leaf_docs: usize,
    selected_leaf_network: usize,
    selected_leaf_games: usize,
    open_submenu: Option<StartSubmenu>,
    open_leaf: Option<StartProgramsLeaf>,
    hover_candidate: Option<(StartHoverTarget, Instant)>,
    app_items: Vec<StartLeafItem>,
    document_items: Vec<StartLeafItem>,
    network_items: Vec<StartLeafItem>,
    game_items: Vec<StartLeafItem>,
}

impl Default for StartState {
    fn default() -> Self {
        Self {
            open: false,
            selected_root: 0,
            selected_program: 0,
            selected_system: 0,
            selected_leaf_apps: 0,
            selected_leaf_docs: 0,
            selected_leaf_network: 0,
            selected_leaf_games: 0,
            open_submenu: Some(StartSubmenu::Programs),
            open_leaf: None,
            hover_candidate: None,
            app_items: Vec::new(),
            document_items: Vec::new(),
            network_items: Vec::new(),
            game_items: Vec::new(),
        }
    }
}

#[derive(Default)]
struct DesktopState {
    windows: Vec<DesktopWindow>,
    next_id: u64,
    cursor_x: u16,
    cursor_y: u16,
    dragging: Option<DragState>,
    task_scroll: usize,
    last_click: Option<LastClick>,
    start: StartState,
    top_menu: TopMenuState,
    help_popup: Option<HelpPopupState>,
    spotlight: SpotlightState,
}

const START_ROOT_ITEMS: [(&str, Option<StartSubmenu>); 5] = [
    ("Programs", Some(StartSubmenu::Programs)),
    ("System", Some(StartSubmenu::System)),
    ("Return To Terminal Mode", None),
    ("Logout", None),
    ("Shutdown", None),
];
const START_ROOT_VIS_ROWS: [Option<usize>; 6] = [Some(0), Some(1), None, Some(2), Some(3), Some(4)];
const START_PROGRAMS: [(&str, StartProgramsLeaf); 4] = [
    ("Applications", StartProgramsLeaf::Applications),
    ("Documents", StartProgramsLeaf::Documents),
    ("Network", StartProgramsLeaf::Network),
    ("Games", StartProgramsLeaf::Games),
];
const START_SYSTEM: [(&str, StartLaunch); 4] = [
    ("Program Installer", StartLaunch::ProgramInstaller),
    ("Terminal", StartLaunch::Terminal),
    ("File Manager", StartLaunch::FileManager),
    ("Settings", StartLaunch::Settings),
];
const START_PROGRAMS_VIS_ROWS: [Option<usize>; 4] = [Some(0), Some(1), Some(2), Some(3)];
const START_SYSTEM_VIS_ROWS: [Option<usize>; 4] = [Some(0), Some(1), Some(2), Some(3)];
const TOP_SPOTLIGHT_ICON: &str = "⌕";

fn submenu_for_root(idx: usize) -> Option<StartSubmenu> {
    START_ROOT_ITEMS.get(idx).and_then(|(_, sub)| *sub)
}

fn submenu_items_system() -> &'static [(&'static str, StartLaunch)] {
    &START_SYSTEM
}

fn submenu_items_programs() -> &'static [(&'static str, StartProgramsLeaf)] {
    &START_PROGRAMS
}

fn submenu_items_len(sub: StartSubmenu) -> usize {
    match sub {
        StartSubmenu::Programs => START_PROGRAMS.len(),
        StartSubmenu::System => START_SYSTEM.len(),
    }
}

fn submenu_selected_idx(state: &StartState, sub: StartSubmenu) -> usize {
    match sub {
        StartSubmenu::Programs => state.selected_program,
        StartSubmenu::System => state.selected_system,
    }
}

fn submenu_selected_idx_mut(state: &mut StartState, sub: StartSubmenu) -> &mut usize {
    match sub {
        StartSubmenu::Programs => &mut state.selected_program,
        StartSubmenu::System => &mut state.selected_system,
    }
}

fn submenu_visual_rows(sub: StartSubmenu) -> &'static [Option<usize>] {
    match sub {
        StartSubmenu::Programs => &START_PROGRAMS_VIS_ROWS,
        StartSubmenu::System => &START_SYSTEM_VIS_ROWS,
    }
}

fn leaf_items(state: &StartState, leaf: StartProgramsLeaf) -> &[StartLeafItem] {
    match leaf {
        StartProgramsLeaf::Applications => &state.app_items,
        StartProgramsLeaf::Documents => &state.document_items,
        StartProgramsLeaf::Network => &state.network_items,
        StartProgramsLeaf::Games => &state.game_items,
    }
}

fn leaf_selected_idx(state: &StartState, leaf: StartProgramsLeaf) -> usize {
    match leaf {
        StartProgramsLeaf::Applications => state.selected_leaf_apps,
        StartProgramsLeaf::Documents => state.selected_leaf_docs,
        StartProgramsLeaf::Network => state.selected_leaf_network,
        StartProgramsLeaf::Games => state.selected_leaf_games,
    }
}

fn leaf_selected_idx_mut(state: &mut StartState, leaf: StartProgramsLeaf) -> &mut usize {
    match leaf {
        StartProgramsLeaf::Applications => &mut state.selected_leaf_apps,
        StartProgramsLeaf::Documents => &mut state.selected_leaf_docs,
        StartProgramsLeaf::Network => &mut state.selected_leaf_network,
        StartProgramsLeaf::Games => &mut state.selected_leaf_games,
    }
}

fn leaf_from_program_idx(idx: usize) -> Option<StartProgramsLeaf> {
    submenu_items_programs().get(idx).map(|(_, leaf)| *leaf)
}

fn program_idx_for_leaf(leaf: StartProgramsLeaf) -> usize {
    submenu_items_programs()
        .iter()
        .position(|(_, value)| *value == leaf)
        .unwrap_or(0)
}

fn clamp_idx(idx: &mut usize, len: usize) {
    if len == 0 {
        *idx = 0;
    } else if *idx >= len {
        *idx = len - 1;
    }
}

fn normalize_start_selection(state: &mut StartState) {
    clamp_idx(&mut state.selected_root, START_ROOT_ITEMS.len());
    clamp_idx(&mut state.selected_program, START_PROGRAMS.len());
    clamp_idx(&mut state.selected_system, START_SYSTEM.len());
    clamp_idx(&mut state.selected_leaf_apps, state.app_items.len());
    clamp_idx(&mut state.selected_leaf_docs, state.document_items.len());
    clamp_idx(&mut state.selected_leaf_network, state.network_items.len());
    clamp_idx(&mut state.selected_leaf_games, state.game_items.len());

    if state.open_submenu != Some(StartSubmenu::Programs) {
        state.open_leaf = None;
    }
    if let Some(leaf) = state.open_leaf {
        if leaf_items(state, leaf).is_empty() {
            state.open_leaf = None;
        }
    }
}

fn sorted_json_keys(map: &serde_json::Map<String, serde_json::Value>) -> Vec<String> {
    let mut keys: Vec<String> = map.keys().cloned().collect();
    keys.sort_by_key(|k| k.to_lowercase());
    keys
}

fn build_command_leaf_items(
    map: serde_json::Map<String, serde_json::Value>,
    empty_label: &str,
) -> Vec<StartLeafItem> {
    let mut items = Vec::new();
    for key in sorted_json_keys(&map) {
        if let Some(v) = map.get(&key) {
            let cmd = json_to_cmd(v);
            if !cmd.is_empty() {
                items.push(StartLeafItem {
                    label: key.clone(),
                    action: StartAction::LaunchCommand { title: key, cmd },
                });
            }
        }
    }
    if items.is_empty() {
        items.push(StartLeafItem {
            label: empty_label.to_string(),
            action: StartAction::None,
        });
    }
    items
}

fn refresh_start_leaf_items(state: &mut StartState) {
    let apps = load_apps();
    let mut app_items = vec![StartLeafItem {
        label: BUILTIN_NUKE_CODES_APP.to_string(),
        action: StartAction::LaunchNukeCodes,
    }];
    for key in sorted_json_keys(&apps) {
        if key == BUILTIN_NUKE_CODES_APP {
            continue;
        }
        if let Some(v) = apps.get(&key) {
            let cmd = json_to_cmd(v);
            if !cmd.is_empty() {
                app_items.push(StartLeafItem {
                    label: key.clone(),
                    action: StartAction::LaunchCommand { title: key, cmd },
                });
            }
        }
    }

    let categories = load_categories();
    let mut document_items = vec![StartLeafItem {
        label: "Logs".to_string(),
        action: StartAction::OpenDocumentLogs,
    }];
    for key in sorted_json_keys(&categories) {
        if let Some(path) = categories.get(&key).and_then(|v| v.as_str()) {
            document_items.push(StartLeafItem {
                label: key.clone(),
                action: StartAction::OpenDocumentCategory {
                    name: key,
                    path: PathBuf::from(path),
                },
            });
        }
    }

    state.app_items = app_items;
    state.document_items = document_items;
    state.network_items = build_command_leaf_items(load_networks(), "(No network apps)");
    state.game_items = build_command_leaf_items(load_games(), "(No games installed)");
    normalize_start_selection(state);
}

fn open_start_menu(state: &mut DesktopState) {
    close_top_menu(state);
    state.help_popup = None;
    refresh_start_leaf_items(&mut state.start);
    state.start.open = true;
    state.start.selected_root = 0;
    state.start.selected_program = 0;
    state.start.open_submenu = Some(StartSubmenu::Programs);
    state.start.open_leaf = None;
    state.start.hover_candidate = None;
    normalize_start_selection(&mut state.start);
}

fn close_start_menu(state: &mut StartState) {
    state.open = false;
    state.open_submenu = None;
    state.open_leaf = None;
    state.hover_candidate = None;
}

fn close_top_menu(state: &mut DesktopState) {
    state.top_menu.open = None;
    state.top_menu.hover_label = None;
    state.top_menu.hover_item = None;
}

fn is_hover_target_open(state: &StartState, target: StartHoverTarget) -> bool {
    match target {
        StartHoverTarget::Submenu(sub) => state.open_submenu == Some(sub),
        StartHoverTarget::Leaf(leaf) => {
            state.open_submenu == Some(StartSubmenu::Programs) && state.open_leaf == Some(leaf)
        }
    }
}

fn apply_hover_target(state: &mut StartState, target: StartHoverTarget) {
    match target {
        StartHoverTarget::Submenu(sub) => {
            state.open_submenu = Some(sub);
            if sub != StartSubmenu::Programs {
                state.open_leaf = None;
            }
        }
        StartHoverTarget::Leaf(leaf) => {
            state.open_submenu = Some(StartSubmenu::Programs);
            state.selected_program = program_idx_for_leaf(leaf);
            state.open_leaf = Some(leaf);
        }
    }
}

const DOUBLE_CLICK_WINDOW: Duration = Duration::from_millis(450);
const START_HOVER_DELAY: Duration = Duration::from_millis(170);
const BUILTIN_NUKE_CODES_APP: &str = "Nuke Codes";
const TITLE_MIN_BUTTON: &str = "[-]";
const TITLE_MAX_BUTTON: &str = "[+]";
const TITLE_RESTORE_BUTTON: &str = "[R]";
const TITLE_CLOSE_BUTTON: &str = "[X]";
const TASK_PAGER_PREV: &str = "[<]";
const TASK_PAGER_NEXT: &str = "[>]";
const TASK_START_BUTTON: &str = "[Start]";
const TASK_START_SEPARATOR: &str = " | ";
const MIN_WINDOW_W: u16 = 20;
const MIN_WINDOW_H: u16 = 8;
const CUSTOM_PROFILE_ADD_LABEL: &str = "Add Custom Profile";
const DESKTOP_SETTINGS_PROFILE_ITEMS: [(DesktopProfileSlot, &str); 5] = [
    (DesktopProfileSlot::Default, "Default"),
    (DesktopProfileSlot::Calcurse, "Calcurse"),
    (DesktopProfileSlot::SpotifyPlayer, "Spotify Player"),
    (DesktopProfileSlot::Ranger, "Ranger"),
    (DesktopProfileSlot::Reddit, "Reddit"),
];
const NO_ENV_OVERRIDES: &[(&str, &str)] = &[];
const CALCURSE_ENV_OVERRIDES: &[(&str, &str)] = &[("NCURSES_NO_UTF8_ACS", "1")];
const WALLPAPER_DEFAULT_ROBCO: &[&str] = &[
    "██████╗  ██████╗ ██████╗  ██████╗  ██████╗",
    "██╔══██╗██╔═══██╗██╔══██╗██╔════╝ ██╔═══██╗",
    "██████╔╝██║   ██║██████╔╝██║      ██║   ██║",
    "██╔══██╗██║   ██║██╔══██╗██║      ██║   ██║",
    "██║  ██║╚██████╔╝██████╔╝╚██████╗ ╚██████╔╝",
    "╚═╝  ╚═╝ ╚═════╝ ╚═════╝  ╚═════╝  ╚═════╝",
];
const DEFAULT_DESKTOP_WALLPAPERS: &[(&str, &[&str])] = &[("RobCo", WALLPAPER_DEFAULT_ROBCO)];
static BATTERY_CACHE: Mutex<Option<(String, Instant)>> = Mutex::new(None);
static SHORTCUT_HINT_CACHE: OnceLock<Mutex<HashMap<String, Vec<(String, String)>>>> =
    OnceLock::new();

#[derive(Debug, Clone, Copy)]
struct PtyCompatibilityProfile {
    min_w: u16,
    min_h: u16,
    preferred_w: Option<u16>,
    preferred_h: Option<u16>,
    mouse_passthrough: bool,
    open_fullscreen: bool,
    env: &'static [(&'static str, &'static str)],
}

fn queue_start_hover(state: &mut StartState, target: StartHoverTarget) {
    if is_hover_target_open(state, target) {
        state.hover_candidate = None;
        return;
    }
    match state.hover_candidate {
        Some((existing, at)) if existing == target => {
            if at.elapsed() >= START_HOVER_DELAY {
                apply_hover_target(state, target);
                state.hover_candidate = None;
            }
        }
        _ => state.hover_candidate = Some((target, Instant::now())),
    }
}

fn advance_start_hover(state: &mut DesktopState) {
    if !state.start.open {
        state.start.hover_candidate = None;
        return;
    }
    if let Some((target, at)) = state.start.hover_candidate {
        if at.elapsed() >= START_HOVER_DELAY {
            apply_hover_target(&mut state.start, target);
            state.start.hover_candidate = None;
        }
    }
}

pub fn desktop_mode(terminal: &mut Term, current_user: &str) -> Result<DesktopExit> {
    let _switch_scope = session_switch_scope(false);
    let _ = terminal.hide_cursor();
    execute!(terminal.backend_mut(), EnableMouseCapture)?;
    let result = run_desktop_loop(terminal, current_user);
    let _ = execute!(terminal.backend_mut(), DisableMouseCapture);
    let _ = terminal.show_cursor();
    result
}

fn run_desktop_loop(terminal: &mut Term, current_user: &str) -> Result<DesktopExit> {
    let mut state = DesktopState {
        next_id: 1,
        ..DesktopState::default()
    };
    let mut last_tick = Instant::now();

    loop {
        reap_closed_pty_windows(&mut state);
        advance_start_hover(&mut state);
        draw_desktop(terminal, &mut state)?;

        let timeout = Duration::from_millis(16);
        if event::poll(timeout)? {
            match event::read()? {
                Event::Key(key) => {
                    if key.kind != KeyEventKind::Press && key.kind != KeyEventKind::Repeat {
                        continue;
                    }
                    if let Some(exit) =
                        handle_key(terminal, current_user, &mut state, key.code, key.modifiers)?
                    {
                        terminate_all_pty_windows(&mut state);
                        return Ok(exit);
                    }
                }
                Event::Mouse(mouse) => {
                    if let Some(exit) = handle_mouse(terminal, current_user, &mut state, mouse)? {
                        terminate_all_pty_windows(&mut state);
                        return Ok(exit);
                    }
                }
                Event::Resize(_, _) => {
                    let ts = terminal.size()?;
                    let size = full_rect(ts.width, ts.height);
                    clamp_all_windows(&mut state, desktop_area(size));
                }
                _ => {}
            }
        }

        if last_tick.elapsed() > Duration::from_millis(250) {
            last_tick = Instant::now();
        }
    }
}

fn handle_key(
    terminal: &mut Term,
    current_user: &str,
    state: &mut DesktopState,
    code: KeyCode,
    modifiers: KeyModifiers,
) -> Result<Option<DesktopExit>> {
    if let Some(popup) = &mut state.help_popup {
        match code {
            KeyCode::Esc | KeyCode::Enter | KeyCode::Char('q') | KeyCode::Char('Q') => {
                state.help_popup = None;
            }
            KeyCode::Up => {
                popup.scroll = popup.scroll.saturating_sub(1);
            }
            KeyCode::Down => {
                popup.scroll = (popup.scroll + 1).min(popup.lines.len().saturating_sub(1));
            }
            KeyCode::PageUp => {
                popup.scroll = popup.scroll.saturating_sub(8);
            }
            KeyCode::PageDown => {
                popup.scroll = (popup.scroll + 8).min(popup.lines.len().saturating_sub(1));
            }
            _ => {}
        }
        return Ok(None);
    }

    if state.spotlight.open {
        match code {
            KeyCode::Esc => {
                spotlight_close(state);
            }
            KeyCode::Up => {
                state.spotlight.selected = state.spotlight.selected.saturating_sub(1);
            }
            KeyCode::Down | KeyCode::Tab => {
                let max = spotlight_items(state).len().saturating_sub(1);
                state.spotlight.selected = (state.spotlight.selected + 1).min(max);
            }
            KeyCode::Enter => {
                let action = spotlight_items(state)
                    .get(state.spotlight.selected)
                    .map(|item| item.action.clone());
                spotlight_close(state);
                if let Some(action) = action {
                    run_top_menu_action(terminal, current_user, state, action)?;
                }
            }
            KeyCode::Backspace => {
                let _ = state.spotlight.query.pop();
                state.spotlight.selected = 0;
            }
            KeyCode::Char(c) => {
                if !modifiers.contains(KeyModifiers::CONTROL)
                    && !modifiers.contains(KeyModifiers::ALT)
                    && !modifiers.contains(KeyModifiers::SUPER)
                {
                    state.spotlight.query.push(c);
                    state.spotlight.selected = 0;
                }
            }
            _ => {}
        }
        spotlight_clamp_selection(state);
        return Ok(None);
    }

    if modifiers.contains(KeyModifiers::CONTROL)
        && matches!(code, KeyCode::Char(' ') | KeyCode::Char('k') | KeyCode::Char('K'))
    {
        close_top_menu(state);
        close_start_menu(&mut state.start);
        spotlight_open(state);
        return Ok(None);
    }

    if let Some(kind) = state.top_menu.open {
        match code {
            KeyCode::Esc => {
                close_top_menu(state);
            }
            KeyCode::Left | KeyCode::Right => {
                let order = top_menu_order();
                let cur = order.iter().position(|k| *k == kind).unwrap_or(0);
                let next_idx = if matches!(code, KeyCode::Right) {
                    (cur + 1) % order.len()
                } else if cur == 0 {
                    order.len().saturating_sub(1)
                } else {
                    cur - 1
                };
                let next_kind = order[next_idx];
                state.top_menu.open = Some(next_kind);
                state.top_menu.hover_label = Some(next_kind);
                let items = top_menu_items(state, next_kind);
                state.top_menu.hover_item = first_enabled_menu_item(&items);
            }
            KeyCode::Up | KeyCode::Down => {
                let items = top_menu_items(state, kind);
                state.top_menu.hover_item = step_enabled_menu_item(
                    &items,
                    state.top_menu.hover_item,
                    matches!(code, KeyCode::Down),
                );
            }
            KeyCode::Enter | KeyCode::Char(' ') => {
                let items = top_menu_items(state, kind);
                let Some(idx) = state
                    .top_menu
                    .hover_item
                    .or_else(|| first_enabled_menu_item(&items))
                else {
                    close_top_menu(state);
                    return Ok(None);
                };
                if let Some(item) = items.get(idx) {
                    if item.enabled {
                        let action = item.action.clone();
                        close_top_menu(state);
                        run_top_menu_action(terminal, current_user, state, action)?;
                    } else {
                        close_top_menu(state);
                    }
                } else {
                    close_top_menu(state);
                }
            }
            _ => {}
        }
        return Ok(None);
    }

    if state.start.open {
        match code {
            KeyCode::Esc => {
                close_start_menu(&mut state.start);
            }
            KeyCode::Up => {
                if let Some(leaf) = state.start.open_leaf {
                    let sel = leaf_selected_idx_mut(&mut state.start, leaf);
                    *sel = sel.saturating_sub(1);
                } else if let Some(sub) = state.start.open_submenu {
                    let sel = submenu_selected_idx_mut(&mut state.start, sub);
                    *sel = sel.saturating_sub(1);
                } else {
                    state.start.selected_root = state.start.selected_root.saturating_sub(1);
                    state.start.open_submenu = submenu_for_root(state.start.selected_root);
                    if state.start.open_submenu != Some(StartSubmenu::Programs) {
                        state.start.open_leaf = None;
                    }
                    state.start.hover_candidate = None;
                }
            }
            KeyCode::Down => {
                if let Some(leaf) = state.start.open_leaf {
                    let max = leaf_items(&state.start, leaf).len().saturating_sub(1);
                    let sel = leaf_selected_idx_mut(&mut state.start, leaf);
                    *sel = (*sel + 1).min(max);
                } else if let Some(sub) = state.start.open_submenu {
                    let max = submenu_items_len(sub).saturating_sub(1);
                    let sel = submenu_selected_idx_mut(&mut state.start, sub);
                    *sel = (*sel + 1).min(max);
                } else {
                    state.start.selected_root =
                        (state.start.selected_root + 1).min(START_ROOT_ITEMS.len() - 1);
                    state.start.open_submenu = submenu_for_root(state.start.selected_root);
                    if state.start.open_submenu != Some(StartSubmenu::Programs) {
                        state.start.open_leaf = None;
                    }
                    state.start.hover_candidate = None;
                }
            }
            KeyCode::Right => {
                if state.start.open_submenu == Some(StartSubmenu::Programs) {
                    if let Some(leaf) = leaf_from_program_idx(state.start.selected_program) {
                        state.start.open_leaf = Some(leaf);
                    }
                } else if let Some(sub) = submenu_for_root(state.start.selected_root) {
                    state.start.open_submenu = Some(sub);
                    if sub != StartSubmenu::Programs {
                        state.start.open_leaf = None;
                    }
                    state.start.hover_candidate = None;
                }
            }
            KeyCode::Left => {
                if state.start.open_leaf.is_some() {
                    state.start.open_leaf = None;
                } else {
                    state.start.open_submenu = None;
                }
                state.start.hover_candidate = None;
            }
            KeyCode::Tab => {
                if let Some(leaf) = state.start.open_leaf {
                    let len = leaf_items(&state.start, leaf).len();
                    if len > 0 {
                        let sel = leaf_selected_idx_mut(&mut state.start, leaf);
                        *sel = (*sel + 1) % len;
                    }
                } else if let Some(sub) = state.start.open_submenu {
                    let len = submenu_items_len(sub);
                    if len > 0 {
                        let sel = submenu_selected_idx_mut(&mut state.start, sub);
                        *sel = (*sel + 1) % len;
                    }
                } else {
                    state.start.selected_root =
                        (state.start.selected_root + 1) % START_ROOT_ITEMS.len();
                    state.start.open_submenu = submenu_for_root(state.start.selected_root);
                    if state.start.open_submenu != Some(StartSubmenu::Programs) {
                        state.start.open_leaf = None;
                    }
                    state.start.hover_candidate = None;
                }
            }
            KeyCode::Enter | KeyCode::Char(' ') => {
                let action = if let Some(leaf) = state.start.open_leaf {
                    let items = leaf_items(&state.start, leaf);
                    if items.is_empty() {
                        StartAction::None
                    } else {
                        let idx = leaf_selected_idx(&state.start, leaf)
                            .min(items.len().saturating_sub(1));
                        items[idx].action.clone()
                    }
                } else if let Some(sub) = submenu_for_root(state.start.selected_root) {
                    if state.start.open_submenu == Some(sub) {
                        match sub {
                            StartSubmenu::Programs => {
                                if let Some(leaf) =
                                    leaf_from_program_idx(state.start.selected_program)
                                {
                                    state.start.open_leaf = Some(leaf);
                                }
                                StartAction::None
                            }
                            StartSubmenu::System => {
                                let items = submenu_items_system();
                                let idx = submenu_selected_idx(&state.start, sub)
                                    .min(items.len().saturating_sub(1));
                                StartAction::Launch(items[idx].1)
                            }
                        }
                    } else {
                        state.start.open_submenu = Some(sub);
                        if sub != StartSubmenu::Programs {
                            state.start.open_leaf = None;
                        }
                        state.start.hover_candidate = None;
                        StartAction::None
                    }
                } else if state.start.selected_root == 2 {
                    StartAction::ReturnToTerminal
                } else if state.start.selected_root == 3 {
                    StartAction::Logout
                } else {
                    StartAction::Shutdown
                };
                if !matches!(action, StartAction::None) {
                    return run_start_action(terminal, current_user, state, action);
                }
            }
            _ => {}
        }
        normalize_start_selection(&mut state.start);
        return Ok(None);
    }

    if matches!(code, KeyCode::F(10)) {
        open_start_menu(state);
        return Ok(None);
    }

    if let Some(last_idx) = focused_visible_window_idx(state) {
        let focused_id = state.windows[last_idx].id;
        let focused_area = state.windows[last_idx].rect.to_rect();
        let mut close_focused = false;
        let mut settings_action = DesktopSettingsAction::None;
        let mut file_open_request: Option<(PathBuf, FileManagerOpenRequest)> = None;
        let mut refresh_file_managers = false;
        match &mut state.windows[last_idx].kind {
            WindowKind::PtyApp(app) => {
                app.session.send_key(code, modifiers);
                return Ok(None);
            }
            WindowKind::DesktopSettings(settings) => {
                settings_action = handle_desktop_settings_key(settings, code, modifiers);
            }
            WindowKind::FileManager(fm) => {
                let s = get_settings().desktop_file_manager;
                let content = file_manager_content_rect(focused_area);
                let (tree_area, entry_area) =
                    file_manager_tree_and_entry_rects(content, s.show_tree_panel);
                if tree_area.is_none() {
                    fm.tree_focus = false;
                }
                if modifiers.contains(KeyModifiers::CONTROL) {
                    match code {
                        KeyCode::Char('f') | KeyCode::Char('F') => {
                            fm.search_mode = true;
                            fm.tree_focus = false;
                        }
                        KeyCode::Char('t') | KeyCode::Char('T') => {
                            fm.open_tab_here();
                            file_manager_ensure_selection_visible(fm, entry_area);
                        }
                        KeyCode::Char('w') | KeyCode::Char('W') => {
                            let _ = fm.close_active_tab();
                            file_manager_ensure_selection_visible(fm, entry_area);
                        }
                        KeyCode::Tab => {
                            let _ = fm.switch_tab_relative(!modifiers.contains(KeyModifiers::SHIFT));
                            file_manager_ensure_selection_visible(fm, entry_area);
                        }
                        KeyCode::BackTab => {
                            let _ = fm.switch_tab_relative(false);
                            file_manager_ensure_selection_visible(fm, entry_area);
                        }
                        _ => {}
                    }
                } else {
                    if fm.search_mode {
                        match code {
                            KeyCode::Esc | KeyCode::Enter => {
                                fm.search_mode = false;
                            }
                            KeyCode::Backspace => {
                                let mut next = fm.search_query.clone();
                                let _ = next.pop();
                                fm.update_search_query(next);
                                file_manager_ensure_selection_visible(fm, entry_area);
                            }
                            KeyCode::Char(c) => {
                                if !modifiers.contains(KeyModifiers::ALT)
                                    && !modifiers.contains(KeyModifiers::SUPER)
                                {
                                    let mut next = fm.search_query.clone();
                                    next.push(c);
                                    fm.update_search_query(next);
                                    file_manager_ensure_selection_visible(fm, entry_area);
                                }
                            }
                            _ => {}
                        }
                    } else if matches!(code, KeyCode::Tab) && tree_area.is_some() {
                        fm.tree_focus = !fm.tree_focus;
                        if fm.tree_focus {
                            if let Some(tree_rect) = tree_area {
                                file_manager_ensure_tree_selection_visible(fm, tree_rect);
                            }
                        } else {
                            file_manager_ensure_selection_visible(fm, entry_area);
                        }
                    } else if fm.tree_focus && tree_area.is_some() {
                        match code {
                            KeyCode::Esc => {
                                close_focused = true;
                            }
                            KeyCode::Up => {
                                fm.tree_move_selection(false);
                                let _ = fm.open_selected_tree_path();
                                if let Some(tree_rect) = tree_area {
                                    file_manager_ensure_tree_selection_visible(fm, tree_rect);
                                }
                                file_manager_ensure_selection_visible(fm, entry_area);
                            }
                            KeyCode::Down => {
                                fm.tree_move_selection(true);
                                let _ = fm.open_selected_tree_path();
                                if let Some(tree_rect) = tree_area {
                                    file_manager_ensure_tree_selection_visible(fm, tree_rect);
                                }
                                file_manager_ensure_selection_visible(fm, entry_area);
                            }
                            KeyCode::Enter => {
                                let _ = fm.open_selected_tree_path();
                                if let Some(tree_rect) = tree_area {
                                    file_manager_ensure_tree_selection_visible(fm, tree_rect);
                                }
                                file_manager_ensure_selection_visible(fm, entry_area);
                            }
                            KeyCode::Right => {
                                fm.tree_focus = false;
                                file_manager_ensure_selection_visible(fm, entry_area);
                            }
                            _ => {}
                        }
                    } else {
                        match code {
                        KeyCode::Esc => {
                            close_focused = true;
                        }
                        KeyCode::Up => {
                            if matches!(s.view_mode, FileManagerViewMode::Grid) {
                                let (cols, _) = file_manager_grid_metrics(entry_area);
                                if cols > 0 {
                                    fm.selected = fm.selected.saturating_sub(cols);
                                }
                            } else {
                                fm.up();
                            }
                            file_manager_ensure_selection_visible(fm, entry_area);
                        }
                        KeyCode::Down => {
                            if matches!(s.view_mode, FileManagerViewMode::Grid) {
                                let (cols, _) = file_manager_grid_metrics(entry_area);
                                if cols > 0 && !fm.entries.is_empty() {
                                    fm.selected = (fm.selected + cols).min(fm.entries.len() - 1);
                                }
                            } else {
                                fm.down();
                            }
                            file_manager_ensure_selection_visible(fm, entry_area);
                        }
                        KeyCode::Left => {
                            if matches!(s.view_mode, FileManagerViewMode::Grid) {
                                fm.selected = fm.selected.saturating_sub(1);
                                file_manager_ensure_selection_visible(fm, entry_area);
                            }
                        }
                        KeyCode::Right => {
                            if matches!(s.view_mode, FileManagerViewMode::Grid) {
                                if fm.selected + 1 < fm.entries.len() {
                                    fm.selected += 1;
                                }
                                file_manager_ensure_selection_visible(fm, entry_area);
                            }
                        }
                        KeyCode::Enter => {
                            file_open_request = fm.activate_selected(FileManagerOpenRequest::Builtin)
                        }
                        KeyCode::Char('x') | KeyCode::Char('X') => {
                            file_open_request = fm.activate_selected(FileManagerOpenRequest::External)
                        }
                        KeyCode::Backspace => {
                            fm.parent();
                            file_manager_ensure_selection_visible(fm, entry_area);
                        }
                        _ => {}
                    }
                    }
                }
            }
            WindowKind::FileManagerSettings(settings) => {
                let (refresh, close) = handle_file_manager_settings_key(settings, code, modifiers);
                if refresh {
                    refresh_file_managers = true;
                }
                if close {
                    close_focused = true;
                }
            }
        }
        if !matches!(settings_action, DesktopSettingsAction::None) {
            run_desktop_settings_action(
                terminal,
                current_user,
                state,
                focused_id,
                settings_action,
            )?;
        }
        if matches!(settings_action, DesktopSettingsAction::CloseWindow) {
            close_focused = false;
        }
        if refresh_file_managers {
            refresh_all_file_manager_windows(state);
        }
        if let Some((path, request)) = file_open_request {
            handle_file_open_request(terminal, state, &path, request)?;
        }
        if close_focused {
            close_window_by_id(state, focused_id);
        }
    } else if matches!(code, KeyCode::Char('m') | KeyCode::Char('M')) {
        open_file_manager_window(state);
    }

    Ok(None)
}

fn handle_mouse(
    terminal: &mut Term,
    current_user: &str,
    state: &mut DesktopState,
    mouse: crossterm::event::MouseEvent,
) -> Result<Option<DesktopExit>> {
    state.cursor_x = mouse.column;
    state.cursor_y = mouse.row;

    let term_size = terminal.size()?;
    let size = full_rect(term_size.width, term_size.height);
    let top = top_status_area(size);
    let desk = desktop_area(size);
    let task = taskbar_area(size);

    if let Some(popup) = &mut state.help_popup {
        match mouse.kind {
            MouseEventKind::ScrollUp => {
                popup.scroll = popup.scroll.saturating_sub(1);
            }
            MouseEventKind::ScrollDown => {
                popup.scroll = (popup.scroll + 1).min(popup.lines.len().saturating_sub(1));
            }
            MouseEventKind::Down(MouseButton::Left) => {
                state.help_popup = None;
            }
            _ => {}
        }
        return Ok(None);
    }

    if state.spotlight.open {
        let overlay = spotlight_overlay_rect(size);
        match mouse.kind {
            MouseEventKind::ScrollUp => {
                state.spotlight.selected = state.spotlight.selected.saturating_sub(1);
                spotlight_clamp_selection(state);
                return Ok(None);
            }
            MouseEventKind::ScrollDown => {
                let max = spotlight_items(state).len().saturating_sub(1);
                state.spotlight.selected = (state.spotlight.selected + 1).min(max);
                spotlight_clamp_selection(state);
                return Ok(None);
            }
            MouseEventKind::Down(MouseButton::Left) => {
                if let Some(icon) = top_status_spotlight_rect(top) {
                    if point_in_rect(mouse.column, mouse.row, icon) {
                        return Ok(None);
                    }
                }
                if let Some(area) = overlay {
                    if point_in_rect(mouse.column, mouse.row, area) {
                        let list = spotlight_list_area(area);
                        if point_in_rect(mouse.column, mouse.row, list) {
                            let row = (mouse.row - list.y) as usize;
                            let items = spotlight_items(state);
                            if !items.is_empty() {
                                let visible = list.height as usize;
                                let start = state
                                    .spotlight
                                    .selected
                                    .saturating_sub(visible.saturating_sub(1));
                                let idx = start + row;
                                if idx < items.len() {
                                    state.spotlight.selected = idx;
                                    let action = items[idx].action.clone();
                                    spotlight_close(state);
                                    run_top_menu_action(terminal, current_user, state, action)?;
                                }
                            }
                        }
                        return Ok(None);
                    }
                }
                spotlight_close(state);
                return Ok(None);
            }
            _ => {
                return Ok(None);
            }
        }
    }

    if matches!(
        mouse.kind,
        MouseEventKind::Moved | MouseEventKind::Down(MouseButton::Left)
    ) {
        let spotlight_hit = top_status_spotlight_rect(top)
            .is_some_and(|rect| point_in_rect(mouse.column, mouse.row, rect));
        let label_hit = hit_top_menu_label(top, state, mouse.column, mouse.row);
        if matches!(mouse.kind, MouseEventKind::Moved) {
            state.top_menu.hover_label = label_hit;
            if let Some(open_kind) = state.top_menu.open {
                if let Some(label_kind) = label_hit {
                    if label_kind != open_kind {
                        state.top_menu.open = Some(label_kind);
                        let items = top_menu_items(state, label_kind);
                        state.top_menu.hover_item = first_enabled_menu_item(&items);
                    } else {
                        state.top_menu.hover_item = None;
                    }
                } else {
                    state.top_menu.hover_item =
                        hit_top_menu_item(top, state, open_kind, mouse.column, mouse.row);
                }
            } else {
                state.top_menu.hover_item = None;
            }

            let over_dropdown = state
                .top_menu
                .open
                .and_then(|kind| top_menu_dropdown_rect(top, state, kind))
                .is_some_and(|rect| point_in_rect(mouse.column, mouse.row, rect));
            if label_hit.is_some() || over_dropdown {
                return Ok(None);
            }
        } else if matches!(mouse.kind, MouseEventKind::Down(MouseButton::Left)) {
            if spotlight_hit {
                close_top_menu(state);
                close_start_menu(&mut state.start);
                spotlight_open(state);
                return Ok(None);
            }
            if let Some(kind) = label_hit {
                if state.top_menu.open == Some(kind) {
                    close_top_menu(state);
                } else {
                    close_start_menu(&mut state.start);
                    state.top_menu.open = Some(kind);
                    state.top_menu.hover_label = Some(kind);
                    let items = top_menu_items(state, kind);
                    state.top_menu.hover_item = first_enabled_menu_item(&items);
                }
                return Ok(None);
            }

            if let Some(open_kind) = state.top_menu.open {
                if let Some(item_idx) =
                    hit_top_menu_item(top, state, open_kind, mouse.column, mouse.row)
                {
                    let items = top_menu_items(state, open_kind);
                    if let Some(item) = items.get(item_idx) {
                        if item.enabled {
                            let action = item.action.clone();
                            close_top_menu(state);
                            run_top_menu_action(terminal, current_user, state, action)?;
                        } else {
                            close_top_menu(state);
                        }
                    } else {
                        close_top_menu(state);
                    }
                    return Ok(None);
                }
                if top_menu_dropdown_rect(top, state, open_kind)
                    .is_some_and(|rect| point_in_rect(mouse.column, mouse.row, rect))
                {
                    return Ok(None);
                }
                close_top_menu(state);
            }
        }
    }

    if let MouseEventKind::Drag(MouseButton::Left) = mouse.kind {
        if let Some(drag) = state.dragging {
            if let Some(win) = state.windows.iter_mut().find(|w| w.id == drag.window_id) {
                match drag.action {
                    DragAction::Move { dx, dy } => {
                        win.rect.x = i32::from(mouse.column) - dx;
                        win.rect.y = i32::from(mouse.row) - dy;
                        let (min_w, min_h) = min_window_size_for_kind(&win.kind);
                        clamp_window_with_min(&mut win.rect, desk, min_w, min_h);
                    }
                    DragAction::Resize { corner, origin } => {
                        if !win.maximized {
                            let (min_w, min_h) = min_window_size_for_kind(&win.kind);
                            apply_corner_resize(
                                &mut win.rect,
                                origin,
                                corner,
                                mouse.column,
                                mouse.row,
                                desk,
                                min_w,
                                min_h,
                            );
                        }
                    }
                }
            }
            return Ok(None);
        }
        if send_mouse_to_focused_pty(state, mouse) {
            return Ok(None);
        }
        return Ok(None);
    }

    if let MouseEventKind::Up(MouseButton::Left) = mouse.kind {
        let was_dragging = state.dragging.take().is_some();
        if was_dragging {
            return Ok(None);
        }
        if send_mouse_to_focused_pty(state, mouse) {
            return Ok(None);
        }
        return Ok(None);
    }

    if matches!(
        mouse.kind,
        MouseEventKind::ScrollUp
            | MouseEventKind::ScrollDown
            | MouseEventKind::ScrollLeft
            | MouseEventKind::ScrollRight
    ) {
        if send_mouse_to_focused_pty(state, mouse) {
            return Ok(None);
        }
        if handle_file_manager_scroll_mouse(state, mouse) {
            return Ok(None);
        }
        return Ok(None);
    }

    if !matches!(
        mouse.kind,
        MouseEventKind::Down(MouseButton::Left) | MouseEventKind::Moved
    ) {
        return Ok(None);
    }

    if point_in_rect(mouse.column, mouse.row, start_button_rect(task)) {
        if matches!(mouse.kind, MouseEventKind::Down(MouseButton::Left)) {
            if state.start.open {
                close_start_menu(&mut state.start);
            } else {
                open_start_menu(state);
            }
        }
        return Ok(None);
    }

    if matches!(mouse.kind, MouseEventKind::Down(MouseButton::Left)) {
        let layout = taskbar_layout(state, task);
        if let Some(prev) = layout.prev_rect {
            if point_in_rect(mouse.column, mouse.row, prev) {
                if layout.can_scroll_left {
                    state.task_scroll = state.task_scroll.saturating_sub(1);
                }
                return Ok(None);
            }
        }
        if let Some(next) = layout.next_rect {
            if point_in_rect(mouse.column, mouse.row, next) {
                if layout.can_scroll_right {
                    state.task_scroll =
                        (state.task_scroll + 1).min(state.windows.len().saturating_sub(1));
                }
                return Ok(None);
            }
        }
        for btn in layout.buttons {
            if point_in_rect(mouse.column, mouse.row, btn.rect) {
                activate_window_from_taskbar(state, btn.window_id, desk);
                return Ok(None);
            }
        }
    }

    if state.start.open {
        let is_click = matches!(mouse.kind, MouseEventKind::Down(MouseButton::Left));
        if let Some(action) = hit_start_menu(mouse.column, mouse.row, size, state, is_click) {
            if matches!(mouse.kind, MouseEventKind::Down(MouseButton::Left))
                && !matches!(action, StartAction::None)
            {
                return run_start_action(terminal, current_user, state, action);
            }
            return Ok(None);
        }

        if matches!(mouse.kind, MouseEventKind::Down(MouseButton::Left)) {
            close_start_menu(&mut state.start);
        }
    }

    if matches!(mouse.kind, MouseEventKind::Down(MouseButton::Left)) {
        if let Some((window_id, hit)) = hit_window(state, mouse.column, mouse.row) {
            focus_window(state, window_id);
            match hit {
                WindowHit::Close => {
                    close_window_by_id(state, window_id);
                }
                WindowHit::Minimize => {
                    minimize_window_by_id(state, window_id);
                }
                WindowHit::Maximize => {
                    toggle_maximize_window_by_id(state, window_id, desk);
                }
                WindowHit::Title => {
                    if let Some(win) = state.windows.iter().find(|w| w.id == window_id) {
                        if !win.maximized {
                            state.dragging = Some(DragState {
                                window_id,
                                action: DragAction::Move {
                                    dx: i32::from(mouse.column) - win.rect.x,
                                    dy: i32::from(mouse.row) - win.rect.y,
                                },
                            });
                        }
                    }
                }
                WindowHit::Resize(corner) => {
                    if let Some(win) = state.windows.iter().find(|w| w.id == window_id) {
                        if !win.maximized {
                            state.dragging = Some(DragState {
                                window_id,
                                action: DragAction::Resize {
                                    corner,
                                    origin: win.rect,
                                },
                            });
                        }
                    }
                }
                WindowHit::Content => {
                    handle_window_content_mouse(terminal, current_user, state, mouse)?;
                }
            }
            return Ok(None);
        }

        if hit_my_computer_icon(mouse.column, mouse.row, desk) {
            if is_double_click(state, ClickTarget::DesktopIconMyComputer) {
                open_file_manager_window(state);
            }
            return Ok(None);
        }
    }

    if matches!(mouse.kind, MouseEventKind::Moved) {
        if let Some(idx) = focused_visible_window_idx(state) {
            let area = state.windows[idx].rect.to_rect();
            if let WindowKind::DesktopSettings(settings) = &mut state.windows[idx].kind {
                let _ = handle_desktop_settings_mouse(settings, area, mouse);
                return Ok(None);
            }
        }
        let _ = send_mouse_to_focused_pty(state, mouse);
    }

    Ok(None)
}

fn run_start_action(
    terminal: &mut Term,
    current_user: &str,
    state: &mut DesktopState,
    action: StartAction,
) -> Result<Option<DesktopExit>> {
    state.start.open = false;
    state.start.open_submenu = None;
    state.start.open_leaf = None;
    state.start.hover_candidate = None;

    match action {
        StartAction::None => Ok(None),
        StartAction::ReturnToTerminal => Ok(Some(DesktopExit::ReturnToTerminal)),
        StartAction::Logout => Ok(Some(DesktopExit::Logout)),
        StartAction::Shutdown => Ok(Some(DesktopExit::Shutdown)),
        StartAction::Launch(which) => {
            let launch_result = match which {
                StartLaunch::Terminal => open_pty_window_named(
                    terminal,
                    state,
                    &default_shell_command(),
                    Some("Terminal"),
                ),
                StartLaunch::ProgramInstaller => open_pty_window_named(
                    terminal,
                    state,
                    &build_desktop_tool_command(current_user, "program-installer")?,
                    Some("Program Installer"),
                ),
                StartLaunch::Settings => {
                    open_desktop_settings_window(terminal, state, current_user);
                    Ok(())
                }
                StartLaunch::FileManager => {
                    open_file_manager_window(state);
                    Ok(())
                }
            };
            if let Err(err) = launch_result {
                flash_message(terminal, &format!("Launch failed: {err}"), 1200)?;
            }
            Ok(None)
        }
        StartAction::LaunchCommand { title, cmd } => {
            if let Err(err) = open_pty_window_named(terminal, state, &cmd, Some(title.as_str())) {
                flash_message(terminal, &format!("Launch failed: {err}"), 1200)?;
            }
            Ok(None)
        }
        StartAction::LaunchNukeCodes => {
            if let Err(err) = open_pty_window_named(
                terminal,
                state,
                &build_desktop_tool_command(current_user, "nuke-codes")?,
                Some("Nuke Codes"),
            ) {
                flash_message(terminal, &format!("Launch failed: {err}"), 1200)?;
            }
            Ok(None)
        }
        StartAction::OpenDocumentLogs => {
            run_with_mouse_capture_paused(terminal, documents::logs_menu)?;
            Ok(None)
        }
        StartAction::OpenDocumentCategory { name, path } => {
            run_with_mouse_capture_paused(terminal, |terminal| {
                documents::open_documents_category(terminal, &name, &path)
            })?;
            Ok(None)
        }
    }
}

fn run_with_mouse_capture_paused<F>(terminal: &mut Term, run: F) -> Result<()>
where
    F: FnOnce(&mut Term) -> Result<()>,
{
    execute!(terminal.backend_mut(), DisableMouseCapture)?;
    let run_result = run(terminal);
    let recapture = execute!(terminal.backend_mut(), EnableMouseCapture);
    run_result?;
    recapture?;
    Ok(())
}

fn run_desktop_settings_action(
    terminal: &mut Term,
    current_user: &str,
    state: &mut DesktopState,
    window_id: u64,
    action: DesktopSettingsAction,
) -> Result<()> {
    match action {
        DesktopSettingsAction::None => {}
        DesktopSettingsAction::CloseWindow => close_window_by_id(state, window_id),
        DesktopSettingsAction::OpenEditMenus => {
            run_with_mouse_capture_paused(terminal, edit_menus_menu)?;
        }
        DesktopSettingsAction::OpenUserManagement => {
            run_with_mouse_capture_paused(terminal, |t| user_management_menu(t, current_user))?;
        }
    }
    Ok(())
}

fn top_menu_dropdown_rect(area: Rect, state: &DesktopState, kind: TopMenuKind) -> Option<Rect> {
    let labels = top_menu_labels(area, state);
    let label = labels.iter().find(|l| l.kind == kind)?;
    let items = top_menu_items(state, kind);
    if items.is_empty() {
        return None;
    }
    let width = items
        .iter()
        .map(|i| {
            i.label.chars().count()
                + i.shortcut
                    .as_ref()
                    .map(|s| s.chars().count() + 3)
                    .unwrap_or(0)
        })
        .max()
        .unwrap_or(8)
        .min(56) as u16
        + 4;
    Some(Rect {
        x: label.rect.x,
        y: area.y.saturating_add(1),
        width,
        height: (items.len() as u16).saturating_add(2),
    })
}

fn hit_top_menu_label(area: Rect, state: &DesktopState, x: u16, y: u16) -> Option<TopMenuKind> {
    top_menu_labels(area, state)
        .into_iter()
        .find(|label| point_in_rect(x, y, label.rect))
        .map(|label| label.kind)
}

fn hit_top_menu_item(
    area: Rect,
    state: &DesktopState,
    kind: TopMenuKind,
    x: u16,
    y: u16,
) -> Option<usize> {
    let menu_rect = top_menu_dropdown_rect(area, state, kind)?;
    if !point_in_rect(x, y, menu_rect) {
        return None;
    }
    if x == menu_rect.x
        || x == menu_rect
            .x
            .saturating_add(menu_rect.width)
            .saturating_sub(1)
        || y == menu_rect.y
        || y == menu_rect
            .y
            .saturating_add(menu_rect.height)
            .saturating_sub(1)
    {
        return None;
    }
    let row = y.saturating_sub(menu_rect.y + 1) as usize;
    let items = top_menu_items(state, kind);
    if row < items.len() {
        Some(row)
    } else {
        None
    }
}

fn wrap_manual_text(text: &str, width: usize) -> Vec<String> {
    if width < 8 {
        return text.lines().map(|l| l.to_string()).collect();
    }
    let mut out = Vec::new();
    for line in text.lines() {
        if line.is_empty() {
            out.push(String::new());
            continue;
        }
        let mut cur = String::new();
        for word in line.split_whitespace() {
            if cur.is_empty() {
                cur.push_str(word);
            } else if cur.chars().count() + 1 + word.chars().count() <= width {
                cur.push(' ');
                cur.push_str(word);
            } else {
                out.push(cur);
                cur = word.to_string();
            }
        }
        if !cur.is_empty() {
            out.push(cur);
        }
    }
    if out.is_empty() {
        out.push(String::new());
    }
    out
}

fn shortcut_hint_cache() -> &'static Mutex<HashMap<String, Vec<(String, String)>>> {
    SHORTCUT_HINT_CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn push_unique_path(paths: &mut Vec<PathBuf>, path: PathBuf) {
    if !paths.iter().any(|p| p == &path) {
        paths.push(path);
    }
}

fn manual_search_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    push_unique_path(&mut roots, crate::config::base_dir());
    if let Ok(cwd) = std::env::current_dir() {
        push_unique_path(&mut roots, cwd.clone());
        if let Some(parent) = cwd.parent() {
            push_unique_path(&mut roots, parent.to_path_buf());
        }
    }
    if let Some(parent) = crate::config::base_dir().parent() {
        push_unique_path(&mut roots, parent.to_path_buf());
    }
    roots
}

fn manual_key_aliases(key: &str) -> Vec<String> {
    let mut keys = Vec::new();
    let mut push = |value: String| {
        if !value.is_empty() && !keys.contains(&value) {
            keys.push(value);
        }
    };
    let base = slugify_manual_key(key);
    if !base.is_empty() {
        push(base.clone());
        push(base.replace('_', "-"));
        push(base.replace('-', "_"));
    }
    push(key.trim().to_ascii_lowercase());
    keys
}

fn manual_paths_for_key(key: &str) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    let keys = manual_key_aliases(key);
    for root in manual_search_roots() {
        let manual_dirs = [root.join("manuals"), root.join("docs").join("manuals")];
        for dir in manual_dirs {
            for alias in &keys {
                for ext in ["txt", "md"] {
                    paths.push(dir.join(format!("{alias}.{ext}")));
                }
            }
        }
    }
    paths
}

fn focused_app_manual_context(state: &DesktopState) -> Option<(String, Vec<String>)> {
    let idx = focused_visible_window_idx(state)?;
    let win = &state.windows[idx];
    let (title, key) = match &win.kind {
        WindowKind::PtyApp(app) => (win.title.clone(), app.manual_key.clone()),
        WindowKind::DesktopSettings(_) => ("Settings".to_string(), "settings".to_string()),
        WindowKind::FileManager(_) => ("My Computer".to_string(), "my_computer".to_string()),
        WindowKind::FileManagerSettings(_) => (
            "File Manager Settings".to_string(),
            "file_manager_settings".to_string(),
        ),
    };
    let mut keys = manual_key_aliases(&key);
    let title_key = slugify_manual_key(&win.title);
    if !title_key.is_empty() {
        for alias in manual_key_aliases(&title_key) {
            if !keys.contains(&alias) {
                keys.push(alias);
            }
        }
    }
    Some((title, keys))
}

fn read_first_manual_file(keys: &[String]) -> Option<String> {
    for key in keys {
        for path in manual_paths_for_key(&key) {
            if let Ok(text) = std::fs::read_to_string(&path) {
                if !text.trim().is_empty() {
                    return Some(text);
                }
            }
        }
    }
    None
}

fn strip_ansi_sequences(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\u{1b}' {
            if matches!(chars.peek(), Some('[')) {
                let _ = chars.next();
                for c in chars.by_ref() {
                    if c.is_ascii_alphabetic() {
                        break;
                    }
                }
            }
            continue;
        }
        out.push(ch);
    }
    out
}

fn strip_overstrikes(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\u{8}' {
            continue;
        }
        if matches!(chars.peek(), Some('\u{8}')) {
            let _ = chars.next();
            if let Some(next) = chars.next() {
                out.push(next);
            } else {
                out.push(ch);
            }
        } else {
            out.push(ch);
        }
    }
    out
}

fn man_page_candidates(keys: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    for key in keys {
        for alias in manual_key_aliases(key) {
            let candidate = alias.trim().to_string();
            if candidate.is_empty() {
                continue;
            }
            if candidate
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '+' | '.'))
                && !out.contains(&candidate)
            {
                out.push(candidate);
            }
        }
    }
    out
}

fn read_man_page_text(keys: &[String]) -> Option<String> {
    for page in man_page_candidates(keys) {
        let output = std::process::Command::new("man")
            .arg(&page)
            .env("MANPAGER", "cat")
            .env("PAGER", "cat")
            .env("MANWIDTH", "110")
            .output()
            .ok()?;
        if !output.status.success() {
            continue;
        }
        let raw = String::from_utf8_lossy(&output.stdout).to_string();
        let cleaned = strip_overstrikes(&strip_ansi_sequences(&raw));
        if cleaned.trim().is_empty() {
            continue;
        }
        return Some(cleaned);
    }
    None
}

fn load_manual_text_for_keys(keys: &[String]) -> Option<String> {
    read_first_manual_file(keys).or_else(|| read_man_page_text(keys))
}

fn load_manual_text_for_focused_app(state: &DesktopState) -> Option<(String, String)> {
    let (title, keys) = focused_app_manual_context(state)?;
    load_manual_text_for_keys(&keys).map(|text| (title, text))
}

fn split_shortcut_line(line: &str) -> Option<(String, String)> {
    let trimmed = line.trim();
    if trimmed.len() < 4 {
        return None;
    }
    for sep in [" - ", " -- ", " : ", ": ", "\t"] {
        if let Some((left, right)) = trimmed.split_once(sep) {
            let key = left
                .trim()
                .trim_matches(|c| c == '`' || c == '*' || c == '"' || c == '\'');
            let desc = right.trim();
            if key.is_empty() || desc.is_empty() {
                continue;
            }
            if key.chars().count() > 18 || key.split_whitespace().count() > 3 {
                continue;
            }
            if desc.chars().count() < 3 {
                continue;
            }
            return Some((desc.to_string(), key.to_string()));
        }
    }

    let bytes = trimmed.as_bytes();
    let mut split_at = None;
    for i in 1..bytes.len().saturating_sub(1) {
        if bytes[i] == b' ' && bytes[i - 1] == b' ' {
            split_at = Some(i - 1);
            break;
        }
    }
    let idx = split_at?;
    let key = trimmed[..idx].trim();
    let desc = trimmed[idx..].trim();
    if key.is_empty() || desc.is_empty() {
        return None;
    }
    if key.chars().count() > 18 || key.split_whitespace().count() > 3 || desc.chars().count() < 3 {
        return None;
    }
    Some((desc.to_string(), key.to_string()))
}

fn extract_shortcut_hints(text: &str, limit: usize) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = Vec::new();
    for line in text.lines() {
        if out.len() >= limit {
            break;
        }
        let Some((desc, key)) = split_shortcut_line(line) else {
            continue;
        };
        if out
            .iter()
            .any(|(_, existing)| existing.eq_ignore_ascii_case(&key))
        {
            continue;
        }
        out.push((desc, key));
    }
    out
}

fn shortcut_hints_for_focused_app(state: &DesktopState) -> Vec<(String, String)> {
    let Some((_, keys)) = focused_app_manual_context(state) else {
        return Vec::new();
    };
    let cache_key = keys.join("|");
    if let Ok(cache) = shortcut_hint_cache().lock() {
        if let Some(cached) = cache.get(&cache_key) {
            return cached.clone();
        }
    }
    let hints = load_manual_text_for_keys(&keys)
        .map(|text| extract_shortcut_hints(&text, 6))
        .unwrap_or_default();
    if let Ok(mut cache) = shortcut_hint_cache().lock() {
        cache.insert(cache_key, hints.clone());
    }
    hints
}

fn open_shortcuts_popup(state: &mut DesktopState) {
    if let Some((title, _)) = focused_app_manual_context(state) {
        let hints = shortcut_hints_for_focused_app(state);
        let text = if hints.is_empty() {
            "No shortcut hints could be extracted from available docs/man page.".to_string()
        } else {
            hints
                .into_iter()
                .map(|(desc, key)| format!("{desc} : {key}"))
                .collect::<Vec<_>>()
                .join("\n")
        };
        open_help_popup(state, &format!("{title} Shortcuts"), &text);
        return;
    }
    open_help_popup(state, "App Shortcuts", "No active app window.");
}

fn user_manual_paths() -> Vec<PathBuf> {
    let mut out = Vec::new();
    for root in manual_search_roots() {
        push_unique_path(&mut out, root.join("USER_MANUAL.md"));
        push_unique_path(&mut out, root.join("README.md"));
        push_unique_path(&mut out, root.join("docs").join("USER_MANUAL.md"));
        push_unique_path(&mut out, root.join("docs").join("README.md"));
    }
    out
}

fn open_help_popup(state: &mut DesktopState, title: &str, text: &str) {
    state.help_popup = Some(HelpPopupState {
        title: title.to_string(),
        lines: wrap_manual_text(text, 92),
        scroll: 0,
    });
}

fn open_user_manual_popup(state: &mut DesktopState) {
    for path in user_manual_paths() {
        if let Ok(text) = std::fs::read_to_string(path) {
            open_help_popup(state, "User Manual", &text);
            return;
        }
    }
    open_help_popup(state, "User Manual", "Manual file not found.");
}

fn open_app_manual_popup(state: &mut DesktopState) {
    if let Some((title, text)) = load_manual_text_for_focused_app(state) {
        open_help_popup(state, &format!("{title} Manual"), &text);
    } else {
        let fallback =
            "No manual found for this app.\n\nAdd a file in manuals/<app>.txt or manuals/<app>.md.";
        open_help_popup(state, "App Manual", fallback);
    }
}

fn run_top_menu_action(
    terminal: &mut Term,
    current_user: &str,
    state: &mut DesktopState,
    action: TopMenuAction,
) -> Result<()> {
    match action {
        TopMenuAction::None => {}
        TopMenuAction::OpenStart => open_start_menu(state),
        TopMenuAction::OpenSettings => open_desktop_settings_window(terminal, state, current_user),
        TopMenuAction::OpenProgramInstaller => {
            if let Ok(cmd) = build_desktop_tool_command(current_user, "program-installer") {
                let _ = open_pty_window_named(terminal, state, &cmd, Some("Program Installer"));
            }
        }
        TopMenuAction::OpenFileManager => open_file_manager_window(state),
        TopMenuAction::OpenFileManagerSettings => open_file_manager_settings_window(state),
        TopMenuAction::NewFileManagerTab => {
            if let Some(fm) = focused_file_manager_mut(state) {
                fm.open_tab_here();
            }
        }
        TopMenuAction::CloseFileManagerTab => {
            if let Some(fm) = focused_file_manager_mut(state) {
                let _ = fm.close_active_tab();
            }
        }
        TopMenuAction::NextFileManagerTab => {
            if let Some(fm) = focused_file_manager_mut(state) {
                let _ = fm.switch_tab_relative(true);
            }
        }
        TopMenuAction::PrevFileManagerTab => {
            if let Some(fm) = focused_file_manager_mut(state) {
                let _ = fm.switch_tab_relative(false);
            }
        }
        TopMenuAction::OpenSelectedFileBuiltin => {
            open_focused_file_manager_selection(terminal, state, FileManagerOpenRequest::Builtin)?;
        }
        TopMenuAction::OpenSelectedFileExternal => {
            open_focused_file_manager_selection(terminal, state, FileManagerOpenRequest::External)?;
        }
        TopMenuAction::OpenEditMenus => {
            run_with_mouse_capture_paused(terminal, edit_menus_menu)?;
        }
        TopMenuAction::OpenUserManagement => {
            run_with_mouse_capture_paused(terminal, |t| user_management_menu(t, current_user))?;
        }
        TopMenuAction::ShowAppShortcuts => open_shortcuts_popup(state),
        TopMenuAction::CloseFocusedWindow => {
            if let Some(id) = focused_window_id(state) {
                close_window_by_id(state, id);
            }
        }
        TopMenuAction::MinimizeFocusedWindow => {
            if let Some(id) = focused_window_id(state) {
                minimize_window_by_id(state, id);
            }
        }
        TopMenuAction::ToggleMaxFocusedWindow => {
            if let Some(id) = focused_window_id(state) {
                let size = terminal.size()?;
                toggle_maximize_window_by_id(
                    state,
                    id,
                    desktop_area(full_rect(size.width, size.height)),
                );
            }
        }
        TopMenuAction::FocusWindow(id) => {
            if let Ok(size) = terminal.size() {
                activate_window_from_taskbar(
                    state,
                    id,
                    desktop_area(full_rect(size.width, size.height)),
                );
            } else {
                focus_window(state, id);
            }
        }
        TopMenuAction::OpenAppManual => open_app_manual_popup(state),
        TopMenuAction::OpenUserManual => open_user_manual_popup(state),
    }
    Ok(())
}

fn open_pty_window_named(
    terminal: &mut Term,
    state: &mut DesktopState,
    cmd: &[String],
    title_override: Option<&str>,
) -> Result<()> {
    if cmd.is_empty() {
        return Ok(());
    }

    let size = terminal.size()?;
    let full = full_rect(size.width, size.height);
    let desk = desktop_area(full);
    if desk.width < 24 || desk.height < 8 {
        return Ok(());
    }

    let cmd = rewrite_legacy_command(cmd);
    let profile = pty_profile_for_program(&cmd[0]);
    let offset = ((state.windows.len() % 6) as i32) * 2;
    let base_w = profile
        .preferred_w
        .unwrap_or_else(|| desk.width.saturating_sub(10).clamp(44, 120));
    let base_h = profile
        .preferred_h
        .unwrap_or_else(|| desk.height.saturating_sub(5).clamp(12, 36));
    let mut rect = WinRect {
        x: desk.x as i32 + 4 + offset,
        y: desk.y as i32 + 2 + offset,
        w: base_w,
        h: base_h,
    };
    clamp_window_with_min(&mut rect, desk, profile.min_w, profile.min_h);
    let mut restore_rect = None;
    let mut maximized = false;
    if profile.open_fullscreen {
        restore_rect = Some(rect);
        maximized = true;
        rect = winrect_from_rect(desk);
    }

    let cols = rect.w.saturating_sub(2).max(1);
    let rows = rect.h.saturating_sub(2).max(1);
    let options = crate::pty::PtyLaunchOptions {
        env: profile
            .env
            .iter()
            .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
            .collect(),
        top_bar: None,
    };
    let session = spawn_desktop_pty_with_fallback(&cmd, cols, rows, &options)?;

    let title = title_override
        .map(str::to_string)
        .unwrap_or_else(|| command_title(&cmd[0]));
    let manual_key = manual_key_for_command(&cmd, title_override.unwrap_or(&title));
    let id = state.next_id;
    state.next_id += 1;
    state.windows.push(DesktopWindow {
        id,
        title,
        rect,
        restore_rect,
        minimized: false,
        maximized,
        kind: WindowKind::PtyApp(PtyWindowState {
            session,
            min_w: profile.min_w,
            min_h: profile.min_h,
            mouse_passthrough: profile.mouse_passthrough,
            manual_key,
        }),
    });
    Ok(())
}

fn spawn_desktop_pty_with_fallback(
    cmd: &[String],
    cols: u16,
    rows: u16,
    options: &crate::pty::PtyLaunchOptions,
) -> Result<crate::pty::PtySession> {
    if cmd.is_empty() {
        return Err(anyhow!("empty command"));
    }

    let program = &cmd[0];
    let args: Vec<&str> = cmd[1..].iter().map(String::as_str).collect();
    match crate::pty::PtySession::spawn(program, &args, cols, rows, &options) {
        Ok(session) => Ok(session),
        Err(primary_err) => {
            let Some(shell_cmd) = build_shell_fallback_command(cmd) else {
                return Err(primary_err);
            };
            let shell_program = &shell_cmd[0];
            let shell_args: Vec<&str> = shell_cmd[1..].iter().map(String::as_str).collect();
            match crate::pty::PtySession::spawn(shell_program, &shell_args, cols, rows, &options) {
                Ok(session) => Ok(session),
                Err(shell_err) => Err(anyhow!(
                    "launch failed: {primary_err}; shell fallback failed: {shell_err}"
                )),
            }
        }
    }
}

fn rewrite_legacy_command(cmd: &[String]) -> Vec<String> {
    if cmd.is_empty() {
        return Vec::new();
    }
    let mut out = cmd.to_vec();
    if out[0] == "rtv" && !command_exists("rtv") && command_exists("tuir") {
        out[0] = "tuir".to_string();
    }
    out
}

fn command_exists(name: &str) -> bool {
    if name.is_empty() {
        return false;
    }
    if name.contains('/') {
        return Path::new(name).is_file();
    }
    std::env::var_os("PATH")
        .is_some_and(|path| std::env::split_paths(&path).any(|dir| dir.join(name).is_file()))
}

fn build_shell_fallback_command(cmd: &[String]) -> Option<Vec<String>> {
    if cmd.is_empty() {
        return None;
    }
    if cmd[0].contains('/') {
        return None;
    }
    let shell = std::env::var("SHELL")
        .ok()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "/bin/sh".to_string());
    let line = cmd
        .iter()
        .map(|part| shell_quote(part))
        .collect::<Vec<_>>()
        .join(" ");
    Some(vec![shell, "-ic".to_string(), line])
}

fn shell_quote(value: &str) -> String {
    if value.is_empty() {
        return "''".to_string();
    }
    if value
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || "_-./:@%+=,".contains(c))
    {
        return value.to_string();
    }
    format!("'{}'", value.replace('\'', "'\"'\"'"))
}

fn command_title(program: &str) -> String {
    let name = Path::new(program)
        .file_name()
        .and_then(|s| s.to_str())
        .map(str::to_string)
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| program.to_string());
    let lower = name.to_ascii_lowercase();
    if lower == "spotify_player" {
        return "spotify".to_string();
    }
    name
}

fn slugify_manual_key(value: &str) -> String {
    let mut out = String::new();
    let mut prev_us = false;
    for ch in value.chars() {
        let c = ch.to_ascii_lowercase();
        if c.is_ascii_alphanumeric() {
            out.push(c);
            prev_us = false;
        } else if (c == '_' || c == '-' || c == ' ') && !prev_us && !out.is_empty() {
            out.push('_');
            prev_us = true;
        }
    }
    if out.ends_with('_') {
        out.pop();
    }
    out
}

fn manual_key_for_command(cmd: &[String], display_title: &str) -> String {
    if let Some(i) = cmd.iter().position(|part| part == "--desktop-tool") {
        if let Some(tool) = cmd.get(i + 1) {
            let key = slugify_manual_key(tool);
            if !key.is_empty() {
                return key;
            }
        }
    }
    if let Some(base) = normalize_profile_key(cmd.first().map(String::as_str).unwrap_or("")) {
        if !base.is_empty() {
            return base;
        }
    }
    let title_key = slugify_manual_key(display_title);
    if !title_key.is_empty() {
        return title_key;
    }
    "app".to_string()
}

fn pty_profile_for_program(program: &str) -> PtyCompatibilityProfile {
    let base = normalize_profile_key(program).unwrap_or_else(|| program.to_ascii_lowercase());

    let settings = get_settings();
    let profiles = settings.desktop_cli_profiles;
    if let Some(custom) = profiles.custom.get(&base) {
        return profile_from_settings(custom, NO_ENV_OVERRIDES);
    }
    match base.as_str() {
        name if name.starts_with("calcurse") => {
            profile_from_settings(&profiles.calcurse, CALCURSE_ENV_OVERRIDES)
        }
        "spotify_player" => profile_from_settings(&profiles.spotify_player, NO_ENV_OVERRIDES),
        "ranger" => profile_from_settings(&profiles.ranger, NO_ENV_OVERRIDES),
        "tuir" | "rtv" => profile_from_settings(&profiles.reddit, NO_ENV_OVERRIDES),
        _ => profile_from_settings(&profiles.default, NO_ENV_OVERRIDES),
    }
}

fn profile_from_settings(
    profile: &DesktopPtyProfileSettings,
    env: &'static [(&'static str, &'static str)],
) -> PtyCompatibilityProfile {
    let min_w = profile.min_w.max(MIN_WINDOW_W);
    let min_h = profile.min_h.max(MIN_WINDOW_H);
    let preferred_w = profile.preferred_w.filter(|w| *w >= min_w);
    let preferred_h = profile.preferred_h.filter(|h| *h >= min_h);
    PtyCompatibilityProfile {
        min_w,
        min_h,
        preferred_w,
        preferred_h,
        mouse_passthrough: profile.mouse_passthrough,
        open_fullscreen: profile.open_fullscreen,
        env,
    }
}

fn default_shell_command() -> Vec<String> {
    let shell = std::env::var("SHELL")
        .ok()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "/bin/zsh".to_string());
    vec![shell]
}

fn build_desktop_tool_command(current_user: &str, tool: &str) -> Result<Vec<String>> {
    let exe = std::env::current_exe()?;
    let exe = exe.to_string_lossy().to_string();
    Ok(vec![
        exe,
        "--desktop-tool".to_string(),
        tool.to_string(),
        "--desktop-user".to_string(),
        current_user.to_string(),
        "--no-preflight".to_string(),
    ])
}

fn close_window_by_id(state: &mut DesktopState, window_id: u64) {
    if let Some(pos) = state.windows.iter().position(|w| w.id == window_id) {
        let mut removed = state.windows.remove(pos);
        if let WindowKind::PtyApp(app) = &mut removed.kind {
            app.session.terminate();
            crate::config::reload_settings();
        }
    }
}

fn reap_closed_pty_windows(state: &mut DesktopState) {
    let mut idx = 0;
    while idx < state.windows.len() {
        let is_alive = {
            let win = &mut state.windows[idx];
            match &mut win.kind {
                WindowKind::PtyApp(app) => app.session.is_alive(),
                WindowKind::DesktopSettings(_) => true,
                WindowKind::FileManager(_) => true,
                WindowKind::FileManagerSettings(_) => true,
            }
        };
        if is_alive {
            idx += 1;
        } else {
            let mut removed = state.windows.remove(idx);
            if let WindowKind::PtyApp(app) = &mut removed.kind {
                app.session.terminate();
                crate::config::reload_settings();
            }
        }
    }
}

fn terminate_all_pty_windows(state: &mut DesktopState) {
    for win in &mut state.windows {
        if let WindowKind::PtyApp(app) = &mut win.kind {
            app.session.terminate();
        }
    }
}

fn sync_pty_window_sizes(state: &mut DesktopState) {
    for win in &mut state.windows {
        if let WindowKind::PtyApp(app) = &mut win.kind {
            let area = win.rect.to_rect();
            let cols = area.width.saturating_sub(2).max(1);
            let rows = area.height.saturating_sub(2).max(1);
            app.session.resize(cols, rows);
        }
    }
}

fn draw_desktop(terminal: &mut Term, state: &mut DesktopState) -> Result<()> {
    let ts = terminal.size()?;
    let size = full_rect(ts.width, ts.height);
    clamp_all_windows(state, desktop_area(size));
    state.task_scroll = state.task_scroll.min(state.windows.len().saturating_sub(1));
    sync_pty_window_sizes(state);

    terminal.draw(|f| {
        let size = f.area();
        let top = top_status_area(size);
        let desktop = desktop_area(size);
        let task = taskbar_area(size);

        // Fully clear each frame so overlapped windows cannot leak old cells.
        f.render_widget(Clear, size);

        draw_top_status(f, top, state);
        draw_desktop_background(f, desktop);
        draw_taskbar(f, state, task);

        let focused = focused_visible_window_id(state);
        for win in &state.windows {
            let is_focused = Some(win.id) == focused;
            draw_window(f, win, is_focused);
        }

        if state.start.open {
            draw_start_menu(f, size, state);
        }

        draw_top_menu_overlay(f, top, state);
        draw_spotlight_overlay(f, size, state);

        if let Some(popup) = &state.help_popup {
            draw_help_popup(f, size, popup);
        }

        draw_cursor(f, state.cursor_x, state.cursor_y, size);
    })?;
    Ok(())
}

fn focused_window_title(state: &DesktopState) -> Option<String> {
    let idx = focused_visible_window_idx(state)?;
    Some(state.windows[idx].title.clone())
}

fn top_app_menu_name(state: &DesktopState) -> String {
    focused_window_title(state).unwrap_or_else(|| "Desktop".to_string())
}

fn top_menu_order() -> [TopMenuKind; 6] {
    [
        TopMenuKind::App,
        TopMenuKind::File,
        TopMenuKind::Edit,
        TopMenuKind::View,
        TopMenuKind::Window,
        TopMenuKind::Help,
    ]
}

fn top_menu_label_text(kind: TopMenuKind, state: &DesktopState) -> String {
    match kind {
        TopMenuKind::App => top_app_menu_name(state),
        TopMenuKind::File => "File".to_string(),
        TopMenuKind::Edit => "Edit".to_string(),
        TopMenuKind::View => "View".to_string(),
        TopMenuKind::Window => "Window".to_string(),
        TopMenuKind::Help => "Help".to_string(),
    }
}

fn top_menu_labels(area: Rect, state: &DesktopState) -> Vec<TopMenuLabel> {
    let mut labels = Vec::new();
    if area.width == 0 {
        return labels;
    }
    let mut x = area.x.saturating_add(1);
    let max_x = area.x.saturating_add(area.width);
    for kind in top_menu_order() {
        let text = top_menu_label_text(kind, state);
        let w = text.chars().count() as u16;
        if w == 0 || x.saturating_add(w) > max_x {
            break;
        }
        labels.push(TopMenuLabel {
            kind,
            text,
            rect: Rect {
                x,
                y: area.y,
                width: w,
                height: 1,
            },
        });
        x = x.saturating_add(w).saturating_add(2);
    }
    labels
}

fn top_status_right_text() -> String {
    let now = Local::now().format("%a %d %b | %H.%M").to_string();
    let batt = battery_display();
    let batt_clean = batt
        .chars()
        .take_while(|c| c.is_ascii_digit())
        .collect::<String>();
    let batt_text = if batt_clean.is_empty() {
        "--%".to_string()
    } else {
        format!("{batt_clean}%")
    };
    format!("{now} | {batt_text}")
}

fn top_status_spotlight_rect(area: Rect) -> Option<Rect> {
    let right = top_status_right_text();
    let right_len = right.chars().count() as u16;
    if area.width <= right_len + 3 {
        return None;
    }
    let right_start = area.x + area.width - right_len - 1;
    let icon_x = right_start.saturating_sub(2);
    if icon_x <= area.x {
        return None;
    }
    Some(Rect {
        x: icon_x,
        y: area.y,
        width: 1,
        height: 1,
    })
}

fn draw_top_status(f: &mut ratatui::Frame, area: Rect, state: &DesktopState) {
    if area.height == 0 {
        return;
    }
    let width = area.width as usize;
    let mut row = vec![' '; width];
    let right = top_status_right_text();
    if width >= right.chars().count() + 1 {
        let start = width.saturating_sub(right.chars().count() + 1);
        write_text(&mut row, start, &right);
    }
    if let Some(icon_rect) = top_status_spotlight_rect(area) {
        write_text_in_area(&mut row, area, icon_rect.x, TOP_SPOTLIGHT_ICON);
    }

    let line: String = row.into_iter().collect();
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(line, sel_style()))),
        area,
    );

    for label in top_menu_labels(area, state) {
        let active = state.top_menu.open == Some(label.kind)
            || (state.top_menu.open.is_none() && state.top_menu.hover_label == Some(label.kind));
        let style = if active { normal_style() } else { sel_style() };
        f.render_widget(
            Paragraph::new(Line::from(Span::styled(label.text, style))),
            label.rect,
        );
    }

    if let Some(icon_rect) = top_status_spotlight_rect(area) {
        let style = if state.spotlight.open {
            normal_style()
        } else {
            sel_style()
        };
        f.render_widget(
            Paragraph::new(Line::from(Span::styled(TOP_SPOTLIGHT_ICON, style))),
            icon_rect,
        );
    }
}

fn draw_top_menu_overlay(f: &mut ratatui::Frame, area: Rect, state: &DesktopState) {
    let Some(kind) = state.top_menu.open else {
        return;
    };
    let labels = top_menu_labels(area, state);
    let Some(label) = labels.iter().find(|l| l.kind == kind) else {
        return;
    };
    let items = top_menu_items(state, kind);
    if items.is_empty() {
        return;
    }
    let width = items
        .iter()
        .map(|i| {
            i.label.chars().count()
                + i.shortcut
                    .as_ref()
                    .map(|s| s.chars().count() + 3)
                    .unwrap_or(0)
        })
        .max()
        .unwrap_or(8)
        .min(56) as u16
        + 4;
    let area = Rect {
        x: label.rect.x,
        y: label.rect.y.saturating_add(1),
        width,
        height: (items.len() as u16).saturating_add(2),
    };
    f.render_widget(Clear, area);
    f.render_widget(
        Block::default().borders(Borders::ALL).style(title_style()),
        area,
    );
    let inner_w = area.width.saturating_sub(2) as usize;
    let mut lines = Vec::new();
    for (idx, item) in items.iter().enumerate() {
        if item.label.is_empty() {
            lines.push(Line::from(Span::styled("-".repeat(inner_w), dim_style())));
        } else {
            let style = if state.top_menu.hover_item == Some(idx) && item.enabled {
                sel_style()
            } else if item.enabled {
                normal_style()
            } else {
                dim_style()
            };
            lines.push(Line::from(Span::styled(
                format_top_menu_row(inner_w, &item.label, item.shortcut.as_deref()),
                style,
            )));
        }
    }
    f.render_widget(
        Paragraph::new(lines),
        Rect {
            x: area.x + 1,
            y: area.y + 1,
            width: area.width.saturating_sub(2),
            height: area.height.saturating_sub(2),
        },
    );
}

fn draw_help_popup(f: &mut ratatui::Frame, size: Rect, popup: &HelpPopupState) {
    if size.width < 24 || size.height < 10 {
        return;
    }
    let width = size.width.saturating_sub(12).clamp(24, 100);
    let height = size.height.saturating_sub(8).clamp(8, 24);
    let area = Rect {
        x: size.x + (size.width.saturating_sub(width)) / 2,
        y: size.y + (size.height.saturating_sub(height)) / 2,
        width,
        height,
    };
    f.render_widget(Clear, area);
    f.render_widget(
        Block::default().borders(Borders::ALL).style(title_style()),
        area,
    );
    let title = format!(" {} ", popup.title);
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(title, title_style()))),
        Rect {
            x: area.x + 1,
            y: area.y,
            width: area.width.saturating_sub(2),
            height: 1,
        },
    );
    let visible = area.height.saturating_sub(3) as usize;
    let start = popup.scroll.min(popup.lines.len().saturating_sub(visible));
    let end = (start + visible).min(popup.lines.len());
    let lines: Vec<Line> = popup.lines[start..end]
        .iter()
        .map(|line| Line::from(Span::styled(line.as_str(), normal_style())))
        .collect();
    f.render_widget(
        Paragraph::new(lines),
        Rect {
            x: area.x + 1,
            y: area.y + 1,
            width: area.width.saturating_sub(2),
            height: area.height.saturating_sub(2),
        },
    );
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            "Esc / Click = Close   Up/Down = Scroll",
            dim_style(),
        ))),
        Rect {
            x: area.x + 1,
            y: area.y + area.height.saturating_sub(1),
            width: area.width.saturating_sub(2),
            height: 1,
        },
    );
}

fn focused_window_id(state: &DesktopState) -> Option<u64> {
    focused_visible_window_idx(state).map(|idx| state.windows[idx].id)
}

fn focused_window_kind(state: &DesktopState) -> Option<&WindowKind> {
    let idx = focused_visible_window_idx(state)?;
    Some(&state.windows[idx].kind)
}

fn focused_file_manager_selected_entry(state: &DesktopState) -> Option<FileEntry> {
    let idx = focused_visible_window_idx(state)?;
    match &state.windows[idx].kind {
        WindowKind::FileManager(fm) => fm.entries.get(fm.selected).cloned(),
        _ => None,
    }
}

fn focused_file_manager_mut(state: &mut DesktopState) -> Option<&mut FileManagerState> {
    let idx = focused_visible_window_idx(state)?;
    match &mut state.windows[idx].kind {
        WindowKind::FileManager(fm) => Some(fm),
        _ => None,
    }
}

fn top_menu_separator_item() -> TopMenuItem {
    TopMenuItem {
        label: String::new(),
        shortcut: None,
        action: TopMenuAction::None,
        enabled: false,
    }
}

fn top_menu_items(state: &DesktopState, kind: TopMenuKind) -> Vec<TopMenuItem> {
    match kind {
        TopMenuKind::App => {
            let Some(title) = focused_window_title(state) else {
                return vec![TopMenuItem {
                    label: "No active app".to_string(),
                    shortcut: None,
                    action: TopMenuAction::None,
                    enabled: false,
                }];
            };
            let app_hint = match focused_window_kind(state) {
                Some(WindowKind::FileManager(_)) => {
                    "Open Enter | External X | Ctrl+T New Tab | Ctrl+Tab Next Tab"
                }
                Some(WindowKind::FileManagerSettings(_)) => "Adjust and apply with Enter/Space",
                Some(WindowKind::DesktopSettings(_)) => "Navigate Arrows | Select Enter",
                Some(WindowKind::PtyApp(_)) => "Keys pass through to app",
                None => "",
            };
            vec![
                TopMenuItem {
                    label: format!("Close {title}"),
                    shortcut: Some("Ctrl+W".to_string()),
                    action: TopMenuAction::CloseFocusedWindow,
                    enabled: true,
                },
                TopMenuItem {
                    label: "Minimize".to_string(),
                    shortcut: Some("Ctrl+M".to_string()),
                    action: TopMenuAction::MinimizeFocusedWindow,
                    enabled: true,
                },
                TopMenuItem {
                    label: "Maximize/Restore".to_string(),
                    shortcut: Some("Ctrl+Enter".to_string()),
                    action: TopMenuAction::ToggleMaxFocusedWindow,
                    enabled: true,
                },
                TopMenuItem {
                    label: app_hint.to_string(),
                    shortcut: None,
                    action: TopMenuAction::None,
                    enabled: false,
                },
            ]
        }
        TopMenuKind::File => {
            let mut items = Vec::new();
            if let Some(WindowKind::FileManager(fm)) = focused_window_kind(state) {
                let selected = focused_file_manager_selected_entry(state);
                let has_any = selected.is_some();
                let has_file = selected.as_ref().is_some_and(|e| !e.is_dir);
                let has_extra_tabs = fm.tabs.len() > 1;
                items.push(TopMenuItem {
                    label: "File Manager Settings".to_string(),
                    shortcut: None,
                    action: TopMenuAction::OpenFileManagerSettings,
                    enabled: true,
                });
                items.push(TopMenuItem {
                    label: "New Tab".to_string(),
                    shortcut: Some("Ctrl+T".to_string()),
                    action: TopMenuAction::NewFileManagerTab,
                    enabled: true,
                });
                items.push(TopMenuItem {
                    label: "Close Tab".to_string(),
                    shortcut: Some("Ctrl+W".to_string()),
                    action: TopMenuAction::CloseFileManagerTab,
                    enabled: has_extra_tabs,
                });
                items.push(TopMenuItem {
                    label: "Next Tab".to_string(),
                    shortcut: Some("Ctrl+Tab".to_string()),
                    action: TopMenuAction::NextFileManagerTab,
                    enabled: has_extra_tabs,
                });
                items.push(TopMenuItem {
                    label: "Previous Tab".to_string(),
                    shortcut: Some("Ctrl+Shift+Tab".to_string()),
                    action: TopMenuAction::PrevFileManagerTab,
                    enabled: has_extra_tabs,
                });
                items.push(TopMenuItem {
                    label: "Open Selected".to_string(),
                    shortcut: Some("Enter".to_string()),
                    action: TopMenuAction::OpenSelectedFileBuiltin,
                    enabled: has_any,
                });
                items.push(TopMenuItem {
                    label: "Open Selected Externally".to_string(),
                    shortcut: Some("X".to_string()),
                    action: TopMenuAction::OpenSelectedFileExternal,
                    enabled: has_file,
                });
                items.push(top_menu_separator_item());
            }
            items.push(TopMenuItem {
                label: "Settings".to_string(),
                shortcut: None,
                action: TopMenuAction::OpenSettings,
                enabled: true,
            });
            items.push(TopMenuItem {
                label: "Open Start Menu".to_string(),
                shortcut: Some("F10".to_string()),
                action: TopMenuAction::OpenStart,
                enabled: true,
            });
            items.push(top_menu_separator_item());
            items.push(TopMenuItem {
                label: "Program Installer".to_string(),
                shortcut: None,
                action: TopMenuAction::OpenProgramInstaller,
                enabled: true,
            });
            items
        }
        TopMenuKind::Edit => {
            vec![
                TopMenuItem {
                    label: "Edit Menus".to_string(),
                    shortcut: None,
                    action: TopMenuAction::OpenEditMenus,
                    enabled: true,
                },
                TopMenuItem {
                    label: "User Management".to_string(),
                    shortcut: None,
                    action: TopMenuAction::OpenUserManagement,
                    enabled: true,
                },
                TopMenuItem {
                    label: "App Shortcuts".to_string(),
                    shortcut: None,
                    action: TopMenuAction::ShowAppShortcuts,
                    enabled: focused_window_id(state).is_some(),
                },
            ]
        }
        TopMenuKind::View => vec![
            TopMenuItem {
                label: "My Computer".to_string(),
                shortcut: Some("M".to_string()),
                action: TopMenuAction::OpenFileManager,
                enabled: true,
            },
            TopMenuItem {
                label: "Maximize/Restore Focused".to_string(),
                shortcut: Some("Ctrl+Enter".to_string()),
                action: TopMenuAction::ToggleMaxFocusedWindow,
                enabled: focused_window_id(state).is_some(),
            },
        ],
        TopMenuKind::Window => {
            let mut items = Vec::new();
            for win in state.windows.iter().rev().take(8) {
                items.push(TopMenuItem {
                    label: win.title.clone(),
                    shortcut: None,
                    action: TopMenuAction::FocusWindow(win.id),
                    enabled: true,
                });
            }
            if items.is_empty() {
                items.push(TopMenuItem {
                    label: "No windows".to_string(),
                    shortcut: None,
                    action: TopMenuAction::None,
                    enabled: false,
                });
            }
            items
        }
        TopMenuKind::Help => vec![
            TopMenuItem {
                label: "App Manual".to_string(),
                shortcut: Some("F1".to_string()),
                action: TopMenuAction::OpenAppManual,
                enabled: focused_window_id(state).is_some(),
            },
            TopMenuItem {
                label: "User Manual".to_string(),
                shortcut: None,
                action: TopMenuAction::OpenUserManual,
                enabled: true,
            },
        ],
    }
}

fn spotlight_items(state: &DesktopState) -> Vec<SpotlightItem> {
    let mut items = vec![
        SpotlightItem {
            label: "My Computer".to_string(),
            action: TopMenuAction::OpenFileManager,
        },
        SpotlightItem {
            label: "Settings".to_string(),
            action: TopMenuAction::OpenSettings,
        },
        SpotlightItem {
            label: "Program Installer".to_string(),
            action: TopMenuAction::OpenProgramInstaller,
        },
        SpotlightItem {
            label: "Open Start Menu".to_string(),
            action: TopMenuAction::OpenStart,
        },
        SpotlightItem {
            label: "App Manual".to_string(),
            action: TopMenuAction::OpenAppManual,
        },
        SpotlightItem {
            label: "User Manual".to_string(),
            action: TopMenuAction::OpenUserManual,
        },
    ];
    if matches!(focused_window_kind(state), Some(WindowKind::FileManager(_))) {
        items.push(SpotlightItem {
            label: "File Manager Settings".to_string(),
            action: TopMenuAction::OpenFileManagerSettings,
        });
    }
    for win in state.windows.iter().rev().take(10) {
        items.push(SpotlightItem {
            label: format!("Switch to {}", win.title),
            action: TopMenuAction::FocusWindow(win.id),
        });
    }

    let q = state.spotlight.query.trim().to_ascii_lowercase();
    if q.is_empty() {
        return items;
    }
    let mut filtered: Vec<SpotlightItem> = items
        .into_iter()
        .filter(|item| item.label.to_ascii_lowercase().contains(&q))
        .collect();
    filtered.sort_by_key(|item| {
        let key = item.label.to_ascii_lowercase();
        (!key.starts_with(&q), key)
    });
    filtered
}

fn spotlight_overlay_rect(size: Rect) -> Option<Rect> {
    if size.width < 36 || size.height < 8 {
        return None;
    }
    let width = size.width.saturating_sub(20).clamp(36, 72);
    let height = size.height.saturating_sub(6).clamp(7, 14);
    Some(Rect {
        x: size.x + (size.width.saturating_sub(width)) / 2,
        y: size.y + 2,
        width,
        height,
    })
}

fn spotlight_search_row(area: Rect) -> Rect {
    Rect {
        x: area.x + 1,
        y: area.y + 1,
        width: area.width.saturating_sub(2),
        height: 1,
    }
}

fn spotlight_list_area(area: Rect) -> Rect {
    Rect {
        x: area.x + 1,
        y: area.y + 2,
        width: area.width.saturating_sub(2),
        height: area.height.saturating_sub(3),
    }
}

fn spotlight_open(state: &mut DesktopState) {
    state.spotlight.open = true;
    state.spotlight.query.clear();
    state.spotlight.selected = 0;
}

fn spotlight_close(state: &mut DesktopState) {
    state.spotlight.open = false;
    state.spotlight.query.clear();
    state.spotlight.selected = 0;
}

fn spotlight_clamp_selection(state: &mut DesktopState) {
    let max = spotlight_items(state).len().saturating_sub(1);
    state.spotlight.selected = state.spotlight.selected.min(max);
}

fn draw_spotlight_overlay(f: &mut ratatui::Frame, size: Rect, state: &DesktopState) {
    if !state.spotlight.open {
        return;
    }
    let Some(area) = spotlight_overlay_rect(size) else {
        return;
    };
    f.render_widget(Clear, area);
    f.render_widget(
        Block::default().borders(Borders::ALL).style(title_style()),
        area,
    );

    let search_row = spotlight_search_row(area);
    let query_text = format!("{TOP_SPOTLIGHT_ICON} {}_", state.spotlight.query);
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            truncate_with_ellipsis(&query_text, search_row.width as usize),
            sel_style(),
        ))),
        search_row,
    );

    let list = spotlight_list_area(area);
    if list.height == 0 || list.width == 0 {
        return;
    }
    let items = spotlight_items(state);
    if items.is_empty() {
        f.render_widget(
            Paragraph::new(Line::from(Span::styled("No results", dim_style()))),
            list,
        );
        return;
    }

    let visible = list.height as usize;
    let start = state
        .spotlight
        .selected
        .saturating_sub(visible.saturating_sub(1));
    let end = (start + visible).min(items.len());
    let mut lines = Vec::new();
    for (idx, item) in items[start..end].iter().enumerate() {
        let absolute = start + idx;
        let style = if absolute == state.spotlight.selected {
            sel_style()
        } else {
            normal_style()
        };
        lines.push(Line::from(Span::styled(
            truncate_with_ellipsis(&item.label, list.width as usize),
            style,
        )));
    }
    f.render_widget(Paragraph::new(lines), list);
}

fn first_enabled_menu_item(items: &[TopMenuItem]) -> Option<usize> {
    items.iter().position(|i| i.enabled)
}

fn step_enabled_menu_item(
    items: &[TopMenuItem],
    current: Option<usize>,
    forward: bool,
) -> Option<usize> {
    if items.is_empty() || !items.iter().any(|i| i.enabled) {
        return None;
    }
    let start = current.unwrap_or_else(|| {
        if forward {
            items.len().saturating_sub(1)
        } else {
            0
        }
    });
    for offset in 1..=items.len() {
        let idx = if forward {
            (start + offset) % items.len()
        } else {
            (start + items.len() - (offset % items.len())) % items.len()
        };
        if items[idx].enabled {
            return Some(idx);
        }
    }
    first_enabled_menu_item(items)
}

fn format_top_menu_row(width: usize, label: &str, shortcut: Option<&str>) -> String {
    if width == 0 {
        return String::new();
    }
    let mut chars = vec![' '; width];
    write_text(&mut chars, 0, label);
    if let Some(short) = shortcut {
        let short_len = short.chars().count();
        if short_len < width {
            let start = width.saturating_sub(short_len);
            write_text(&mut chars, start, short);
        }
    }
    chars.into_iter().collect()
}

fn draw_desktop_background(f: &mut ratatui::Frame, area: Rect) {
    if area.height == 0 || area.width == 0 {
        return;
    }

    let mut lines = Vec::new();
    for _ in 0..area.height {
        lines.push(Line::from(Span::styled(
            " ".repeat(area.width as usize),
            normal_style(),
        )));
    }
    f.render_widget(Paragraph::new(lines), area);

    let settings = get_settings();
    let wallpaper = resolve_wallpaper_lines(&settings);
    let wallpaper = render_wallpaper_for_mode(
        &wallpaper,
        settings.desktop_wallpaper_size_mode,
        area.width as usize,
        area.height as usize,
    );
    if !wallpaper.is_empty() {
        let art_h = wallpaper.len();
        let art_w = wallpaper
            .iter()
            .map(|line| line.chars().count())
            .max()
            .unwrap_or(0);
        if art_h > 0 && art_w > 0 {
            let start_x = area.x + area.width.saturating_sub(art_w as u16) / 2;
            let start_y = area.y + area.height.saturating_sub(art_h as u16) / 2;
            let mut art_lines = Vec::new();
            for line in wallpaper {
                art_lines.push(Line::from(Span::styled(line, dim_style())));
            }
            f.render_widget(
                Paragraph::new(art_lines),
                Rect {
                    x: start_x,
                    y: start_y,
                    width: art_w as u16,
                    height: art_h as u16,
                },
            );
        }
    }

    // Fixed desktop icon: My Computer
    if area.height >= 4 && area.width >= 14 {
        let ix = area.x + 2;
        let iy = area.y + 1;
        let icon_lines = vec![
            Line::from(Span::styled(" [PC] ", title_style())),
            Line::from(Span::styled("My Computer", normal_style())),
        ];
        f.render_widget(
            Paragraph::new(icon_lines),
            Rect {
                x: ix,
                y: iy,
                width: 12,
                height: 2,
            },
        );
    }
}

fn draw_taskbar(f: &mut ratatui::Frame, state: &DesktopState, area: Rect) {
    if area.height == 0 {
        return;
    }
    let width = area.width as usize;
    if width == 0 {
        return;
    }

    let mut row = vec![' '; width];
    write_text_in_area(&mut row, area, area.x, TASK_START_BUTTON);
    write_text_in_area(
        &mut row,
        area,
        area.x.saturating_add(start_button_rect(area).width),
        TASK_START_SEPARATOR,
    );

    let layout = taskbar_layout(state, area);
    if let Some(prev) = layout.prev_rect {
        let text = if layout.can_scroll_left {
            TASK_PAGER_PREV
        } else {
            "   "
        };
        write_text_in_area(&mut row, area, prev.x, text);
    }
    if let Some(next) = layout.next_rect {
        let text = if layout.can_scroll_right {
            TASK_PAGER_NEXT
        } else {
            "   "
        };
        write_text_in_area(&mut row, area, next.x, text);
    }
    for btn in layout.buttons {
        if let Some(win) = state.windows.iter().find(|w| w.id == btn.window_id) {
            let text = task_button_text(win);
            write_text_in_area(&mut row, area, btn.rect.x, &text);
        }
    }

    let line: String = row.into_iter().collect();
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(line, sel_style()))),
        area,
    );
}

fn draw_window(f: &mut ratatui::Frame, win: &DesktopWindow, focused: bool) {
    if win.minimized {
        return;
    }

    let area = win.rect.to_rect();
    if area.width < 8 || area.height < 4 {
        return;
    }

    // Ensure this window is fully opaque over anything behind it.
    f.render_widget(Clear, area);

    let border_style = if focused { title_style() } else { dim_style() };
    f.render_widget(
        Block::default().borders(Borders::ALL).style(border_style),
        area,
    );

    let title_color = if focused { sel_style() } else { dim_style() };
    let mut chars: Vec<char> = vec![' '; area.width.saturating_sub(2) as usize];
    let text = format!(" {} ", win.title);
    write_text(&mut chars, 0, &text);
    let max_button = if win.maximized {
        TITLE_RESTORE_BUTTON
    } else {
        TITLE_MAX_BUTTON
    };
    let buttons = format!("{}{}{}", TITLE_MIN_BUTTON, max_button, TITLE_CLOSE_BUTTON);
    if chars.len() >= buttons.len() {
        let button_x = chars.len() - buttons.len();
        write_text(&mut chars, button_x, &buttons);
    }
    let title_line: String = chars.into_iter().collect();
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(title_line, title_color))),
        Rect {
            x: area.x + 1,
            y: area.y,
            width: area.width - 2,
            height: 1,
        },
    );

    match &win.kind {
        WindowKind::FileManager(fm) => draw_file_manager_window(f, area, fm, focused),
        WindowKind::FileManagerSettings(settings) => {
            draw_file_manager_settings_window(f, area, settings, focused)
        }
        WindowKind::DesktopSettings(settings) => {
            draw_desktop_settings_window(f, area, settings, focused)
        }
        WindowKind::PtyApp(app) => draw_pty_window(f, area, app),
    }
}

fn draw_file_manager_window(
    f: &mut ratatui::Frame,
    area: Rect,
    fm: &FileManagerState,
    focused: bool,
) {
    let content = file_manager_content_rect(area);
    if content.height == 0 || content.width == 0 {
        return;
    }
    let cfg = get_settings().desktop_file_manager;
    let (tree_area, entry_area) = file_manager_tree_and_entry_rects(content, cfg.show_tree_panel);

    let mut header = Vec::new();
    header.push(Line::from(Span::styled(
        file_manager_tab_line(fm, content.width as usize),
        if focused { normal_style() } else { dim_style() },
    )));
    let search_text = if fm.search_mode && focused {
        format!("Search: {}_", fm.search_query)
    } else if fm.search_query.is_empty() {
        "Search: (Ctrl+F)".to_string()
    } else {
        format!("Search: {}", fm.search_query)
    };
    header.push(Line::from(Span::styled(
        truncate_with_ellipsis(&search_text, content.width as usize),
        if focused && fm.search_mode {
            sel_style()
        } else {
            dim_style()
        },
    )));
    header.push(Line::from(Span::styled(
        truncate_with_ellipsis(&format!("Path: {}", fm.cwd.display()), content.width as usize),
        dim_style(),
    )));
    if content.height >= FILE_MANAGER_HEADER_ROWS {
        header.push(Line::from(Span::styled(
            "-".repeat(content.width as usize),
            dim_style(),
        )));
    }
    if !header.is_empty() {
        f.render_widget(
            Paragraph::new(header),
            Rect {
                x: content.x,
                y: content.y,
                width: content.width,
                height: content.height.min(FILE_MANAGER_HEADER_ROWS),
            },
        );
    }

    if entry_area.height == 0 || entry_area.width == 0 {
        return;
    }

    if let Some(tree) = tree_area {
        let tree_items = file_manager_tree_items(&fm.cwd, cfg.show_hidden_files);
        let visible = tree.height as usize;
        let start = fm.tree_scroll.min(tree_items.len().saturating_sub(visible));
        let end = (start + visible).min(tree_items.len());
        let mut tree_lines = Vec::new();
        for idx in start..end {
            let item = &tree_items[idx];
            let style = if focused && fm.tree_focus && idx == fm.tree_selected {
                sel_style()
            } else {
                dim_style()
            };
            tree_lines.push(Line::from(Span::styled(
                truncate_with_ellipsis(&item.line, tree.width as usize),
                style,
            )));
        }
        while tree_lines.len() < visible {
            tree_lines.push(Line::from(""));
        }
        f.render_widget(Paragraph::new(tree_lines), tree);

        if entry_area.x > tree.x {
            let sep_x = entry_area.x - 1;
            let sep_lines: Vec<Line> = (0..entry_area.height)
                .map(|_| Line::from(Span::styled("|", dim_style())))
                .collect();
            f.render_widget(
                Paragraph::new(sep_lines),
                Rect {
                    x: sep_x,
                    y: entry_area.y,
                    width: 1,
                    height: entry_area.height,
                },
            );
        }
    }

    match cfg.view_mode {
        FileManagerViewMode::List => {
            let visible_rows = file_manager_list_visible_rows(entry_area);
            let start = fm
                .scroll
                .min(file_manager_list_max_scroll(fm.entries.len(), visible_rows));
            let mut lines = Vec::new();
            for row in 0..visible_rows {
                let Some(entry) = fm.entries.get(start + row) else {
                    lines.push(Line::from(""));
                    continue;
                };
                let mut text = format!("{} {}", file_manager_entry_icon(entry), entry.name);
                text = truncate_with_ellipsis(&text, entry_area.width as usize);
                let style = if focused && start + row == fm.selected {
                    sel_style()
                } else {
                    normal_style()
                };
                lines.push(Line::from(Span::styled(text, style)));
            }
            f.render_widget(Paragraph::new(lines), entry_area);
        }
        FileManagerViewMode::Grid => {
            let (cols, visible_rows) = file_manager_grid_metrics(entry_area);
            if cols == 0 || visible_rows == 0 {
                return;
            }
            let start_row = fm.scroll.min(file_manager_grid_max_scroll(
                fm.entries.len(),
                cols,
                visible_rows,
            ));
            let cell_width = (entry_area.width / cols as u16).max(1);

            for vis_row in 0..visible_rows {
                for col in 0..cols {
                    let idx = (start_row + vis_row) * cols + col;
                    let Some(entry) = fm.entries.get(idx) else {
                        continue;
                    };

                    let x = entry_area.x + (col as u16 * cell_width);
                    let y = entry_area.y + (vis_row as u16 * FILE_MANAGER_GRID_CELL_HEIGHT);
                    if x >= entry_area.x + entry_area.width || y >= entry_area.y + entry_area.height {
                        continue;
                    }
                    let width = if col + 1 == cols {
                        entry_area.x + entry_area.width - x
                    } else {
                        cell_width.min(entry_area.x + entry_area.width - x)
                    };
                    let height = FILE_MANAGER_GRID_CELL_HEIGHT.min(entry_area.y + entry_area.height - y);
                    let style = if focused && idx == fm.selected {
                        sel_style()
                    } else {
                        normal_style()
                    };

                    let icon = centered_text(file_manager_entry_icon(entry), width as usize);
                    let name = centered_text(&entry.name, width as usize);
                    let mut lines = vec![Line::from(Span::styled(icon, style))];
                    if height > 1 {
                        lines.push(Line::from(Span::styled(name, style)));
                    }
                    while lines.len() < height as usize {
                        lines.push(Line::from(Span::styled(" ".repeat(width as usize), style)));
                    }
                    f.render_widget(
                        Paragraph::new(lines),
                        Rect {
                            x,
                            y,
                            width,
                            height,
                        },
                    );
                }
            }
        }
    }
}

fn file_manager_content_rect(area: Rect) -> Rect {
    Rect {
        x: area.x + 1,
        y: area.y + 1,
        width: area.width.saturating_sub(2),
        height: area.height.saturating_sub(2),
    }
}

fn file_manager_body_rect(content: Rect) -> Rect {
    Rect {
        x: content.x,
        y: content.y + FILE_MANAGER_HEADER_ROWS,
        width: content.width,
        height: content.height.saturating_sub(FILE_MANAGER_HEADER_ROWS),
    }
}

fn file_manager_tree_and_entry_rects(content: Rect, show_tree_panel: bool) -> (Option<Rect>, Rect) {
    let body = file_manager_body_rect(content);
    if !show_tree_panel
        || body.width < FILE_MANAGER_TREE_MIN_TOTAL_WIDTH
        || body.width <= FILE_MANAGER_ENTRY_MIN_WIDTH + FILE_MANAGER_TREE_GAP
    {
        return (None, body);
    }

    let desired_tree = (body.width / 4)
        .max(FILE_MANAGER_TREE_MIN_WIDTH)
        .min(FILE_MANAGER_TREE_MAX_WIDTH);
    let max_tree = body
        .width
        .saturating_sub(FILE_MANAGER_TREE_GAP)
        .saturating_sub(FILE_MANAGER_ENTRY_MIN_WIDTH);
    let tree_w = desired_tree.min(max_tree);
    if tree_w < FILE_MANAGER_TREE_MIN_WIDTH {
        return (None, body);
    }

    let tree_rect = Rect {
        x: body.x,
        y: body.y,
        width: tree_w,
        height: body.height,
    };
    let entry_rect = Rect {
        x: body.x + tree_w + FILE_MANAGER_TREE_GAP,
        y: body.y,
        width: body.width.saturating_sub(tree_w + FILE_MANAGER_TREE_GAP),
        height: body.height,
    };
    (Some(tree_rect), entry_rect)
}

fn file_manager_entry_rect(content: Rect, show_tree_panel: bool) -> Rect {
    let (_, entry) = file_manager_tree_and_entry_rects(content, show_tree_panel);
    entry
}

fn file_manager_tab_title(path: &Path) -> String {
    let home = dirs::home_dir();
    if home.as_ref().is_some_and(|h| h == path) {
        return "~".to_string();
    }
    if path == Path::new("/") {
        return "/".to_string();
    }
    path.file_name()
        .and_then(|s| s.to_str())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .unwrap_or_else(|| path.display().to_string())
}

fn file_manager_tab_line(fm: &FileManagerState, width: usize) -> String {
    let mut line = String::from("Tabs ");
    if width == 0 {
        return line;
    }
    for (idx, tab) in fm.tabs.iter().enumerate() {
        let mut title = file_manager_tab_title(tab);
        title = truncate_with_ellipsis(&title, 12);
        let seg = if idx == fm.active_tab {
            format!("[{}:{}*]", idx + 1, title)
        } else {
            format!("[{}:{}]", idx + 1, title)
        };
        if line.chars().count() < width {
            if line.chars().count() > "Tabs ".chars().count() {
                line.push(' ');
            }
            line.push_str(&seg);
        } else {
            break;
        }
        if line.chars().count() >= width {
            break;
        }
    }
    truncate_with_ellipsis(&line, width)
}

fn file_manager_tab_index_at(fm: &FileManagerState, width: usize, x: usize) -> Option<usize> {
    if width == 0 {
        return None;
    }
    let mut cursor = "Tabs ".chars().count();
    for (idx, tab) in fm.tabs.iter().enumerate() {
        let title = truncate_with_ellipsis(&file_manager_tab_title(tab), 12);
        let seg = if idx == fm.active_tab {
            format!("[{}:{}*]", idx + 1, title)
        } else {
            format!("[{}:{}]", idx + 1, title)
        };
        if idx > 0 {
            cursor += 1;
        }
        let start = cursor;
        let end = (cursor + seg.chars().count()).min(width);
        if x >= start && x < end {
            return Some(idx);
        }
        cursor += seg.chars().count();
        if cursor >= width {
            break;
        }
    }
    None
}

fn file_manager_tree_items(cwd: &Path, show_hidden: bool) -> Vec<FileTreeItem> {
    let root = PathBuf::from("/");

    let mut items = vec![FileTreeItem {
        line: "Folders".to_string(),
        path: None,
    }];

    items.push(FileTreeItem {
        line: "* /".to_string(),
        path: Some(root.clone()),
    });

    let rel = cwd.strip_prefix(&root).unwrap_or(cwd);
    let comps: Vec<String> = rel
        .components()
        .filter_map(|c| {
            let s = c.as_os_str().to_string_lossy().to_string();
            if s.is_empty() || s == "/" {
                None
            } else {
                Some(s)
            }
        })
        .collect();

    let mut running = root.clone();
    for (depth, comp) in comps.iter().enumerate() {
        running = running.join(comp);
        items.push(FileTreeItem {
            line: format!("{}|- {}", "  ".repeat(depth + 1), comp),
            path: Some(running.clone()),
        });
    }

    let mut child_dirs: Vec<String> = std::fs::read_dir(cwd)
        .ok()
        .into_iter()
        .flat_map(|iter| iter.flatten())
        .filter_map(|entry| {
            let path = entry.path();
            if !path.is_dir() {
                return None;
            }
            let name = entry.file_name().to_string_lossy().to_string();
            if !show_hidden && name.starts_with('.') {
                return None;
            }
            Some(name)
        })
        .collect();
    child_dirs.sort_by_key(|n| n.to_lowercase());
    let child_indent = "  ".repeat(comps.len() + 1);
    for name in child_dirs {
        items.push(FileTreeItem {
            line: format!("{child_indent}+- {name}"),
            path: Some(cwd.join(&name)),
        });
    }
    items
}

fn file_manager_tree_selected_for_cwd(items: &[FileTreeItem], cwd: &Path) -> usize {
    items
        .iter()
        .position(|item| item.path.as_ref().is_some_and(|p| p == cwd))
        .or_else(|| items.iter().position(|item| item.path.is_some()))
        .unwrap_or(0)
}

fn file_manager_step_tree_selection(
    items: &[FileTreeItem],
    current: usize,
    forward: bool,
) -> Option<usize> {
    if items.is_empty() || !items.iter().any(|i| i.path.is_some()) {
        return None;
    }
    let start = current.min(items.len().saturating_sub(1));
    for offset in 1..=items.len() {
        let idx = if forward {
            (start + offset) % items.len()
        } else {
            (start + items.len() - (offset % items.len())) % items.len()
        };
        if items[idx].path.is_some() {
            return Some(idx);
        }
    }
    items.iter().position(|i| i.path.is_some())
}

fn file_manager_ensure_tree_selection_visible(fm: &mut FileManagerState, tree_rect: Rect) {
    let items = file_manager_tree_items(
        &fm.cwd,
        get_settings().desktop_file_manager.show_hidden_files,
    );
    if items.is_empty() {
        fm.tree_selected = 0;
        fm.tree_scroll = 0;
        return;
    }
    fm.tree_selected = fm.tree_selected.min(items.len().saturating_sub(1));
    if items
        .get(fm.tree_selected)
        .and_then(|item| item.path.as_ref())
        .is_none()
    {
        fm.tree_selected = file_manager_tree_selected_for_cwd(&items, &fm.cwd);
    }

    let visible = tree_rect.height as usize;
    if visible == 0 {
        fm.tree_scroll = 0;
        return;
    }
    let max_scroll = items.len().saturating_sub(visible);
    fm.tree_scroll = fm.tree_scroll.min(max_scroll);
    if fm.tree_selected < fm.tree_scroll {
        fm.tree_scroll = fm.tree_selected;
    } else if fm.tree_selected >= fm.tree_scroll + visible {
        fm.tree_scroll = fm.tree_selected + 1 - visible;
    }
}

fn file_manager_tree_apply_scroll_delta(
    fm: &mut FileManagerState,
    tree_rect: Rect,
    delta: isize,
) -> bool {
    if delta == 0 {
        return false;
    }
    let items = file_manager_tree_items(
        &fm.cwd,
        get_settings().desktop_file_manager.show_hidden_files,
    );
    if items.is_empty() {
        fm.tree_scroll = 0;
        return false;
    }
    let visible = tree_rect.height as usize;
    if visible == 0 {
        fm.tree_scroll = 0;
        return false;
    }
    let max_scroll = items.len().saturating_sub(visible);
    let before = fm.tree_scroll;
    if delta < 0 {
        fm.tree_scroll = fm.tree_scroll.saturating_sub((-delta) as usize);
    } else {
        fm.tree_scroll = (fm.tree_scroll + delta as usize).min(max_scroll);
    }
    fm.tree_scroll != before
}

fn file_manager_list_visible_rows(entry_rect: Rect) -> usize {
    entry_rect.height as usize
}

fn file_manager_grid_metrics(entry_rect: Rect) -> (usize, usize) {
    if entry_rect.width == 0 || entry_rect.height == 0 {
        return (0, 0);
    }
    let cols = ((entry_rect.width / FILE_MANAGER_GRID_CELL_WIDTH).max(1)) as usize;
    let visible_rows = ((entry_rect.height / FILE_MANAGER_GRID_CELL_HEIGHT).max(1)) as usize;
    (cols, visible_rows)
}

fn file_manager_total_grid_rows(entry_count: usize, cols: usize) -> usize {
    if entry_count == 0 || cols == 0 {
        0
    } else {
        (entry_count + cols - 1) / cols
    }
}

fn file_manager_list_max_scroll(entry_count: usize, visible_rows: usize) -> usize {
    entry_count.saturating_sub(visible_rows)
}

fn file_manager_grid_max_scroll(entry_count: usize, cols: usize, visible_rows: usize) -> usize {
    file_manager_total_grid_rows(entry_count, cols).saturating_sub(visible_rows)
}

fn file_manager_ensure_selection_visible(fm: &mut FileManagerState, entry_rect: Rect) {
    if fm.entries.is_empty() {
        fm.selected = 0;
        fm.scroll = 0;
        return;
    }

    fm.selected = fm.selected.min(fm.entries.len() - 1);
    match get_settings().desktop_file_manager.view_mode {
        FileManagerViewMode::List => {
            let visible_rows = file_manager_list_visible_rows(entry_rect);
            if visible_rows == 0 {
                fm.scroll = 0;
                return;
            }
            let max_scroll = file_manager_list_max_scroll(fm.entries.len(), visible_rows);
            fm.scroll = fm.scroll.min(max_scroll);
            if fm.selected < fm.scroll {
                fm.scroll = fm.selected;
            } else if fm.selected >= fm.scroll + visible_rows {
                fm.scroll = fm.selected + 1 - visible_rows;
            }
        }
        FileManagerViewMode::Grid => {
            let (cols, visible_rows) = file_manager_grid_metrics(entry_rect);
            if cols == 0 || visible_rows == 0 {
                fm.scroll = 0;
                return;
            }
            let max_scroll = file_manager_grid_max_scroll(fm.entries.len(), cols, visible_rows);
            fm.scroll = fm.scroll.min(max_scroll);
            let selected_row = fm.selected / cols;
            if selected_row < fm.scroll {
                fm.scroll = selected_row;
            } else if selected_row >= fm.scroll + visible_rows {
                fm.scroll = selected_row + 1 - visible_rows;
            }
        }
    }
}

fn file_manager_apply_scroll_delta(fm: &mut FileManagerState, entry_rect: Rect, delta: isize) -> bool {
    if delta == 0 || fm.entries.is_empty() {
        return false;
    }

    match get_settings().desktop_file_manager.view_mode {
        FileManagerViewMode::List => {
            let visible_rows = file_manager_list_visible_rows(entry_rect);
            if visible_rows == 0 {
                return false;
            }
            let max_scroll = file_manager_list_max_scroll(fm.entries.len(), visible_rows);
            let before = fm.scroll;
            if delta < 0 {
                fm.scroll = fm.scroll.saturating_sub((-delta) as usize);
            } else {
                fm.scroll = (fm.scroll + delta as usize).min(max_scroll);
            }
            let last_visible = (fm.scroll + visible_rows.saturating_sub(1)).min(fm.entries.len() - 1);
            if fm.selected < fm.scroll {
                fm.selected = fm.scroll;
            } else if fm.selected > last_visible {
                fm.selected = last_visible;
            }
            fm.scroll != before
        }
        FileManagerViewMode::Grid => {
            let (cols, visible_rows) = file_manager_grid_metrics(entry_rect);
            if cols == 0 || visible_rows == 0 {
                return false;
            }
            let max_scroll = file_manager_grid_max_scroll(fm.entries.len(), cols, visible_rows);
            let before = fm.scroll;
            if delta < 0 {
                fm.scroll = fm.scroll.saturating_sub((-delta) as usize);
            } else {
                fm.scroll = (fm.scroll + delta as usize).min(max_scroll);
            }
            let selected_col = fm.selected % cols;
            let first_visible_row = fm.scroll;
            let last_visible_row = fm.scroll + visible_rows.saturating_sub(1);
            let selected_row = fm.selected / cols;
            if selected_row < first_visible_row {
                fm.selected = (first_visible_row * cols + selected_col).min(fm.entries.len() - 1);
            } else if selected_row > last_visible_row {
                fm.selected = (last_visible_row * cols + selected_col).min(fm.entries.len() - 1);
            }
            fm.scroll != before
        }
    }
}

fn file_manager_entry_icon(entry: &FileEntry) -> &'static str {
    if entry.name == ".." {
        return "[UP]";
    }
    if entry.is_dir {
        return "[DIR]";
    }
    let ext = Path::new(&entry.name)
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    match ext.as_str() {
        "txt" | "md" | "rs" | "json" | "toml" | "yaml" | "yml" => "[TXT]",
        "png" | "jpg" | "jpeg" | "gif" | "bmp" | "svg" | "webp" => "[IMG]",
        "zip" | "tar" | "gz" | "bz2" | "xz" | "7z" => "[ARC]",
        "mp3" | "wav" | "flac" | "ogg" => "[AUD]",
        "mp4" | "mkv" | "mov" | "webm" => "[VID]",
        "sh" | "exe" | "app" | "bat" | "cmd" => "[APP]",
        _ => "[FILE]",
    }
}

fn truncate_with_ellipsis(text: &str, max_chars: usize) -> String {
    if max_chars == 0 {
        return String::new();
    }
    if text.chars().count() <= max_chars {
        return text.to_string();
    }
    if max_chars <= 3 {
        return ".".repeat(max_chars);
    }
    let mut out: String = text.chars().take(max_chars - 3).collect();
    out.push_str("...");
    out
}

fn centered_text(text: &str, width: usize) -> String {
    if width == 0 {
        return String::new();
    }
    let clipped = truncate_with_ellipsis(text, width);
    let used = clipped.chars().count();
    if used >= width {
        return clipped;
    }
    let left = (width - used) / 2;
    let right = width - used - left;
    format!("{}{}{}", " ".repeat(left), clipped, " ".repeat(right))
}

fn file_manager_settings_rows() -> Vec<String> {
    let s = get_settings().desktop_file_manager;
    vec![
        format!(
            "Show Hidden Files: {} [toggle]",
            if s.show_hidden_files { "ON" } else { "OFF" }
        ),
        format!(
            "Show Left Tree Panel: {} [toggle]",
            if s.show_tree_panel { "ON" } else { "OFF" }
        ),
        format!(
            "View Mode: {} [toggle]",
            match s.view_mode {
                FileManagerViewMode::Grid => "Grid",
                FileManagerViewMode::List => "List",
            }
        ),
        format!(
            "Sort By: {} [cycle]",
            match s.sort_mode {
                FileManagerSortMode::Name => "Name",
                FileManagerSortMode::Type => "Type",
            }
        ),
        format!(
            "Directories First: {} [toggle]",
            if s.directories_first { "ON" } else { "OFF" }
        ),
        format!(
            "Open Text Files In: {} [toggle]",
            match s.text_open_mode {
                FileManagerTextOpenMode::Editor => "Editor",
                FileManagerTextOpenMode::Viewer => "Viewer",
            }
        ),
        "Back".to_string(),
    ]
}

fn draw_file_manager_settings_window(
    f: &mut ratatui::Frame,
    area: Rect,
    settings: &FileManagerSettingsState,
    focused: bool,
) {
    let inner = Rect {
        x: area.x + 1,
        y: area.y + 1,
        width: area.width.saturating_sub(2),
        height: area.height.saturating_sub(2),
    };
    if inner.height == 0 || inner.width == 0 {
        return;
    }
    let rows = file_manager_settings_rows();
    let mut lines = Vec::new();
    for (idx, row) in rows.iter().enumerate() {
        let style = if focused && settings.selected == idx {
            sel_style()
        } else {
            normal_style()
        };
        lines.push(Line::from(Span::styled(row.as_str(), style)));
    }
    f.render_widget(Paragraph::new(lines), inner);
}

fn desktop_settings_home_items(state: &DesktopSettingsState) -> Vec<DesktopSettingsHomeItem> {
    let mut items = vec![
        DesktopSettingsHomeItem::Appearance,
        DesktopSettingsHomeItem::General,
        DesktopSettingsHomeItem::CliDisplay,
        DesktopSettingsHomeItem::CliProfiles,
        DesktopSettingsHomeItem::EditMenus,
    ];
    if state.is_admin {
        items.push(DesktopSettingsHomeItem::UserManagement);
    }
    items.push(DesktopSettingsHomeItem::About);
    items.push(DesktopSettingsHomeItem::Close);
    items
}

fn desktop_settings_home_label(item: DesktopSettingsHomeItem) -> &'static str {
    match item {
        DesktopSettingsHomeItem::Appearance => "Appearance",
        DesktopSettingsHomeItem::General => "General",
        DesktopSettingsHomeItem::CliDisplay => "CLI Display",
        DesktopSettingsHomeItem::CliProfiles => "CLI Profiles",
        DesktopSettingsHomeItem::EditMenus => "Edit Menus",
        DesktopSettingsHomeItem::UserManagement => "User Management",
        DesktopSettingsHomeItem::About => "About",
        DesktopSettingsHomeItem::Close => "Close",
    }
}

fn desktop_settings_home_icon(item: DesktopSettingsHomeItem) -> &'static str {
    match item {
        DesktopSettingsHomeItem::Appearance => "[A]",
        DesktopSettingsHomeItem::General => "[*]",
        DesktopSettingsHomeItem::CliDisplay => "[#]",
        DesktopSettingsHomeItem::CliProfiles => "[=]",
        DesktopSettingsHomeItem::EditMenus => "[M]",
        DesktopSettingsHomeItem::UserManagement => "[U]",
        DesktopSettingsHomeItem::About => "[i]",
        DesktopSettingsHomeItem::Close => "[X]",
    }
}

fn desktop_settings_home_tiles(content: Rect, count: usize) -> Vec<Rect> {
    if content.width < 22 || content.height < 10 || count == 0 {
        return Vec::new();
    }
    let gap_x = 3u16;
    let gap_y = 1u16;
    let cols = if content.width >= 70 {
        4usize
    } else if content.width >= 52 {
        3usize
    } else {
        2usize
    };
    let cols = cols.min(count.max(1));
    let rows = count.div_ceil(cols);
    let tile_w = ((content.width.saturating_sub(4) - gap_x.saturating_mul(cols as u16 - 1))
        / cols as u16)
        .max(8);
    let tile_h = 4u16;
    let needed_h = rows as u16 * tile_h + rows.saturating_sub(1) as u16 * gap_y;
    if needed_h + 2 > content.height {
        return Vec::new();
    }

    let start_x = content.x + 2;
    let start_y = content.y + 2;
    let mut out = Vec::with_capacity(count);
    for idx in 0..count {
        let col = (idx % cols) as u16;
        let row = (idx / cols) as u16;
        out.push(Rect {
            x: start_x + col * (tile_w + gap_x),
            y: start_y + row * (tile_h + gap_y),
            width: tile_w,
            height: tile_h,
        });
    }
    out
}

fn desktop_settings_home_title_rect(tile: Rect) -> Rect {
    Rect {
        x: tile.x,
        y: tile.y + 2,
        width: tile.width,
        height: 1,
    }
}

fn desktop_profile_slot_title(slot: DesktopProfileSlot) -> &'static str {
    match slot {
        DesktopProfileSlot::Default => "Default",
        DesktopProfileSlot::Calcurse => "Calcurse",
        DesktopProfileSlot::SpotifyPlayer => "Spotify Player",
        DesktopProfileSlot::Ranger => "Ranger",
        DesktopProfileSlot::Reddit => "Reddit/TUIR",
    }
}

fn desktop_settings_custom_profile_keys() -> Vec<String> {
    get_settings()
        .desktop_cli_profiles
        .custom
        .keys()
        .cloned()
        .collect()
}

fn sanitize_wallpaper_line(line: &str) -> String {
    line.chars()
        .filter_map(|ch| match ch {
            '\u{feff}' | '\u{200b}' | '\u{200c}' | '\u{200d}' | '\u{2060}' => None,
            '\u{00a0}' | '\u{1680}' | '\u{2000}' | '\u{2001}' | '\u{2002}' | '\u{2003}'
            | '\u{2004}' | '\u{2005}' | '\u{2006}' | '\u{2007}' | '\u{2008}' | '\u{2009}'
            | '\u{200a}' | '\u{202f}' | '\u{205f}' | '\u{3000}' | '\u{2800}' => Some(' '),
            '\t' => Some(' '),
            _ => Some(ch),
        })
        .collect()
}

fn is_wallpaper_space(ch: char) -> bool {
    ch.is_whitespace() || matches!(ch, '\u{2800}')
}

fn normalize_wallpaper_lines(lines: Vec<String>) -> Vec<String> {
    let mut lines: Vec<String> = lines
        .into_iter()
        .map(|line| {
            let sanitized = sanitize_wallpaper_line(&line);
            sanitized
                .trim_end_matches(is_wallpaper_space)
                .to_string()
        })
        .collect();
    while lines
        .first()
        .is_some_and(|l| l.chars().all(is_wallpaper_space))
    {
        lines.remove(0);
    }
    while lines
        .last()
        .is_some_and(|l| l.chars().all(is_wallpaper_space))
    {
        lines.pop();
    }
    if lines.is_empty() {
        return lines;
    }

    let mut min_leading = usize::MAX;
    let mut max_right = 0usize;
    for line in &lines {
        if line.chars().all(is_wallpaper_space) {
            continue;
        }
        let chars: Vec<char> = line.chars().collect();
        let leading = chars.iter().take_while(|c| is_wallpaper_space(**c)).count();
        let trailing = chars
            .iter()
            .rev()
            .take_while(|c| is_wallpaper_space(**c))
            .count();
        let right = chars.len().saturating_sub(trailing);
        min_leading = min_leading.min(leading);
        max_right = max_right.max(right);
    }
    if min_leading == usize::MAX || max_right <= min_leading {
        return lines;
    }
    let crop_w = max_right - min_leading;
    lines
        .into_iter()
        .map(|line| {
            if line.chars().all(is_wallpaper_space) {
                String::new()
            } else {
                line.chars().skip(min_leading).take(crop_w).collect()
            }
        })
        .collect()
}

fn is_default_wallpaper(name: &str) -> bool {
    DEFAULT_DESKTOP_WALLPAPERS
        .iter()
        .any(|(n, _)| n.eq_ignore_ascii_case(name))
}

fn default_wallpaper_lines(name: &str) -> Option<Vec<String>> {
    DEFAULT_DESKTOP_WALLPAPERS
        .iter()
        .find(|(n, _)| n.eq_ignore_ascii_case(name))
        .map(|(_, lines)| {
            normalize_wallpaper_lines(lines.iter().map(|s| (*s).to_string()).collect())
        })
}

fn resolve_wallpaper_lines(settings: &crate::config::Settings) -> Vec<String> {
    if let Some(lines) = settings
        .desktop_wallpapers_custom
        .get(&settings.desktop_wallpaper)
        .cloned()
    {
        return normalize_wallpaper_lines(lines);
    }
    if let Some(lines) = default_wallpaper_lines(&settings.desktop_wallpaper) {
        return lines;
    }
    DEFAULT_DESKTOP_WALLPAPERS
        .first()
        .map(|(_, lines)| lines.iter().map(|s| (*s).to_string()).collect())
        .unwrap_or_default()
}

fn wallpaper_name_exists(settings: &crate::config::Settings, name: &str) -> bool {
    is_default_wallpaper(name) || settings.desktop_wallpapers_custom.contains_key(name)
}

fn custom_wallpaper_names(settings: &crate::config::Settings) -> Vec<String> {
    settings.desktop_wallpapers_custom.keys().cloned().collect()
}

fn wallpaper_lines_for_name(settings: &crate::config::Settings, name: &str) -> Option<Vec<String>> {
    if let Some(lines) = settings.desktop_wallpapers_custom.get(name).cloned() {
        return Some(normalize_wallpaper_lines(lines));
    }
    default_wallpaper_lines(name)
}

fn desktop_wallpaper_rows() -> Vec<WallpaperRow> {
    let s = get_settings();
    let mut rows = Vec::new();
    rows.push(WallpaperRow {
        label: format!("Current: {}", s.desktop_wallpaper),
        action: WallpaperRowAction::None,
    });
    rows.push(WallpaperRow {
        label: format!(
            "Size Mode: {} [choose]",
            wallpaper_size_mode_label(s.desktop_wallpaper_size_mode)
        ),
        action: WallpaperRowAction::OpenSizeMenu,
    });

    for (name, _) in DEFAULT_DESKTOP_WALLPAPERS {
        rows.push(WallpaperRow {
            label: format!("Set Default: {name}"),
            action: WallpaperRowAction::Set((*name).to_string()),
        });
    }

    rows.push(WallpaperRow {
        label: "Choose Custom Wallpaper...".to_string(),
        action: if s.desktop_wallpapers_custom.is_empty() {
            WallpaperRowAction::None
        } else {
            WallpaperRowAction::OpenChooseMenu
        },
    });
    rows.push(WallpaperRow {
        label: "Delete Custom Wallpaper...".to_string(),
        action: if s.desktop_wallpapers_custom.is_empty() {
            WallpaperRowAction::None
        } else {
            WallpaperRowAction::OpenDeleteMenu
        },
    });

    rows.push(WallpaperRow {
        label: "Add Custom Wallpaper".to_string(),
        action: WallpaperRowAction::AddCustom,
    });
    rows.push(WallpaperRow {
        label: "Back".to_string(),
        action: WallpaperRowAction::Back,
    });
    rows
}

fn wallpaper_size_mode_label(mode: WallpaperSizeMode) -> &'static str {
    match mode {
        WallpaperSizeMode::DefaultSize => "Default Size",
        WallpaperSizeMode::FitToScreen => "Fit to Screen",
        WallpaperSizeMode::Centered => "Centered",
        WallpaperSizeMode::Tile => "Tile",
        WallpaperSizeMode::Stretch => "Stretch",
    }
}

fn wallpaper_size_rows() -> Vec<String> {
    let current = get_settings().desktop_wallpaper_size_mode;
    let mut rows = Vec::new();
    for mode in [
        WallpaperSizeMode::DefaultSize,
        WallpaperSizeMode::FitToScreen,
        WallpaperSizeMode::Centered,
        WallpaperSizeMode::Tile,
        WallpaperSizeMode::Stretch,
    ] {
        let marker = if mode == current { "*" } else { " " };
        rows.push(format!("[{marker}] {}", wallpaper_size_mode_label(mode)));
    }
    rows.push("Back".to_string());
    rows
}

fn wallpaper_choose_rows() -> Vec<String> {
    let s = get_settings();
    let mut rows = custom_wallpaper_names(&s);
    if rows.is_empty() {
        rows.push("(No custom wallpapers)".to_string());
    }
    rows.push("Back".to_string());
    rows
}

fn wallpaper_delete_rows() -> Vec<String> {
    let s = get_settings();
    let mut rows: Vec<String> = custom_wallpaper_names(&s)
        .into_iter()
        .map(|name| format!("Delete: {name}"))
        .collect();
    if rows.is_empty() {
        rows.push("(No custom wallpapers)".to_string());
    }
    rows.push("Back".to_string());
    rows
}

fn wallpaper_preview_name(settings: &DesktopSettingsState) -> Option<String> {
    let cfg = get_settings();
    let idx = settings.hovered.unwrap_or(settings.selected);
    match settings.panel {
        DesktopSettingsPanel::Wallpapers => {
            let rows = desktop_wallpaper_rows();
            match rows.get(idx).map(|r| &r.action) {
                Some(WallpaperRowAction::Set(name)) => Some(name.clone()),
                _ => Some(cfg.desktop_wallpaper),
            }
        }
        DesktopSettingsPanel::WallpaperSize => Some(cfg.desktop_wallpaper),
        DesktopSettingsPanel::WallpaperChoose => {
            let names = custom_wallpaper_names(&cfg);
            names.get(idx).cloned()
        }
        DesktopSettingsPanel::WallpaperDelete => {
            let names = custom_wallpaper_names(&cfg);
            names.get(idx).cloned()
        }
        _ => None,
    }
}

fn wallpaper_source_grid(lines: &[String]) -> (Vec<Vec<char>>, usize, usize) {
    let src_h = lines.len();
    let src_w = lines.iter().map(|line| line.chars().count()).max().unwrap_or(0);
    if src_h == 0 || src_w == 0 {
        return (Vec::new(), src_w, src_h);
    }
    let rows: Vec<Vec<char>> = lines
        .iter()
        .map(|line| {
            let mut row: Vec<char> = line.chars().collect();
            if row.len() < src_w {
                row.resize(src_w, ' ');
            }
            row
        })
        .collect();
    (rows, src_w, src_h)
}

fn fit_wallpaper_to_area(lines: &[String], max_w: usize, max_h: usize) -> Vec<String> {
    if lines.is_empty() || max_w == 0 || max_h == 0 {
        return Vec::new();
    }
    let (source_rows, src_w, src_h) = wallpaper_source_grid(lines);
    if src_h == 0 || src_w == 0 {
        return Vec::new();
    }

    let scale_x = max_w as f32 / src_w as f32;
    let scale_y = max_h as f32 / src_h as f32;
    let scale = scale_x.min(scale_y);
    if !scale.is_finite() || scale <= 0.0 {
        return Vec::new();
    }

    let dst_w = ((src_w as f32 * scale).round() as usize).clamp(1, max_w);
    let dst_h = ((src_h as f32 * scale).round() as usize).clamp(1, max_h);

    let mut scaled = Vec::with_capacity(dst_h);
    for y in 0..dst_h {
        let src_y = y.saturating_mul(src_h) / dst_h;
        let src_row = &source_rows[src_y.min(src_h - 1)];
        let mut row = String::with_capacity(dst_w);
        for x in 0..dst_w {
            let src_x = x.saturating_mul(src_w) / dst_w;
            row.push(src_row[src_x.min(src_w - 1)]);
        }
        scaled.push(row);
    }

    scaled
}

fn default_wallpaper_to_area(lines: &[String], max_w: usize, max_h: usize) -> Vec<String> {
    if lines.is_empty() || max_w == 0 || max_h == 0 {
        return Vec::new();
    }
    let (source_rows, src_w, src_h) = wallpaper_source_grid(lines);
    if src_h == 0 || src_w == 0 {
        return Vec::new();
    }

    let out_w = src_w.min(max_w);
    let out_h = src_h.min(max_h);
    let start_x = src_w.saturating_sub(out_w) / 2;
    let start_y = src_h.saturating_sub(out_h) / 2;

    let mut out = Vec::with_capacity(out_h);
    for y in 0..out_h {
        let src_row = &source_rows[(start_y + y).min(src_h - 1)];
        out.push(src_row[start_x..start_x + out_w].iter().collect());
    }
    out
}

fn tile_wallpaper_to_area(lines: &[String], max_w: usize, max_h: usize) -> Vec<String> {
    if lines.is_empty() || max_w == 0 || max_h == 0 {
        return Vec::new();
    }
    let (source_rows, src_w, src_h) = wallpaper_source_grid(lines);
    if src_h == 0 || src_w == 0 {
        return Vec::new();
    }

    let mut out = Vec::with_capacity(max_h);
    for y in 0..max_h {
        let src_row = &source_rows[y % src_h];
        let mut row = String::with_capacity(max_w);
        for x in 0..max_w {
            row.push(src_row[x % src_w]);
        }
        out.push(row);
    }
    out
}

fn centered_wallpaper_to_area(lines: &[String], max_w: usize, max_h: usize) -> Vec<String> {
    if lines.is_empty() || max_w == 0 || max_h == 0 {
        return Vec::new();
    }

    let target_w = ((max_w * 70) / 100).clamp(24, max_w);
    let target_h = ((max_h * 70) / 100).clamp(8, max_h);
    let min_w = ((max_w * 40) / 100).clamp(12, max_w);
    let min_h = ((max_h * 40) / 100).clamp(4, max_h);

    let mut out = fit_wallpaper_to_area(lines, target_w, target_h);
    let out_w = out.iter().map(|line| line.chars().count()).max().unwrap_or(0);
    let out_h = out.len();
    if out_w < min_w || out_h < min_h {
        out = fit_wallpaper_to_area(lines, min_w, min_h);
    }
    out
}

fn stretch_wallpaper_to_area(lines: &[String], max_w: usize, max_h: usize) -> Vec<String> {
    if lines.is_empty() || max_w == 0 || max_h == 0 {
        return Vec::new();
    }
    let (source_rows, src_w, src_h) = wallpaper_source_grid(lines);
    if src_h == 0 || src_w == 0 {
        return Vec::new();
    }

    let mut out = Vec::with_capacity(max_h);
    for y in 0..max_h {
        let src_y = y.saturating_mul(src_h) / max_h;
        let src_row = &source_rows[src_y.min(src_h - 1)];
        let mut row = String::with_capacity(max_w);
        for x in 0..max_w {
            let src_x = x.saturating_mul(src_w) / max_w;
            row.push(src_row[src_x.min(src_w - 1)]);
        }
        out.push(row);
    }
    out
}

fn render_wallpaper_for_mode(
    lines: &[String],
    mode: WallpaperSizeMode,
    max_w: usize,
    max_h: usize,
) -> Vec<String> {
    match mode {
        WallpaperSizeMode::DefaultSize => default_wallpaper_to_area(lines, max_w, max_h),
        WallpaperSizeMode::FitToScreen => fit_wallpaper_to_area(lines, max_w, max_h),
        WallpaperSizeMode::Centered => centered_wallpaper_to_area(lines, max_w, max_h),
        WallpaperSizeMode::Tile => tile_wallpaper_to_area(lines, max_w, max_h),
        WallpaperSizeMode::Stretch => stretch_wallpaper_to_area(lines, max_w, max_h),
    }
}

fn desktop_settings_add_wallpaper(state: &mut DesktopSettingsState) {
    state.wallpaper_error = None;
    let name = state.wallpaper_name_input.trim();
    let path = state.wallpaper_path_input.trim();

    if name.is_empty() {
        state.wallpaper_error = Some("Enter wallpaper name".to_string());
        return;
    }
    let text = if !state.wallpaper_art_input.trim().is_empty() {
        state.wallpaper_art_input.clone()
    } else if !path.is_empty() {
        let input_path = PathBuf::from(path);
        match std::fs::read_to_string(&input_path) {
            Ok(text) => text,
            Err(_) => {
                state.wallpaper_error = Some("Could not read file path".to_string());
                return;
            }
        }
    } else {
        state.wallpaper_error = Some("Paste art or enter wallpaper file path".to_string());
        return;
    };
    let lines: Vec<String> = text
        .lines()
        .map(|line| line.trim_end_matches('\r').to_string())
        .collect();
    let lines = normalize_wallpaper_lines(lines);
    if lines.is_empty() {
        state.wallpaper_error = Some("Wallpaper file is empty".to_string());
        return;
    }
    if lines.len() > 120 {
        state.wallpaper_error = Some("Wallpaper too tall (max 120 lines)".to_string());
        return;
    }

    let key = name.to_string();
    let mut name_in_use = false;
    update_settings(|s| {
        if is_default_wallpaper(&key) {
            name_in_use = true;
            return;
        }
        if wallpaper_name_exists(s, &key) {
            name_in_use = true;
            return;
        }
        s.desktop_wallpapers_custom
            .insert(key.clone(), lines.clone());
        s.desktop_wallpaper = key.clone();
    });
    if name_in_use {
        state.wallpaper_error = Some("Wallpaper name already exists".to_string());
        return;
    }

    persist_settings();
    state.wallpaper_name_input.clear();
    state.wallpaper_path_input.clear();
    state.wallpaper_art_input.clear();
    state.wallpaper_error = None;
    state.panel = DesktopSettingsPanel::Wallpapers;
    state.selected = 0;
}

fn desktop_settings_delete_custom_wallpaper(name: &str) {
    update_settings(|s| {
        s.desktop_wallpapers_custom.remove(name);
        if s.desktop_wallpaper == name {
            s.desktop_wallpaper = DEFAULT_DESKTOP_WALLPAPERS
                .first()
                .map(|(n, _)| (*n).to_string())
                .unwrap_or_else(|| "Vault Door".to_string());
        }
    });
    persist_settings();
}

fn desktop_settings_set_wallpaper(name: &str) {
    update_settings(|s| {
        if wallpaper_name_exists(s, name) {
            s.desktop_wallpaper = name.to_string();
        }
    });
    persist_settings();
}

fn desktop_settings_set_wallpaper_size_mode(mode: WallpaperSizeMode) {
    update_settings(|s| s.desktop_wallpaper_size_mode = mode);
    persist_settings();
}

fn desktop_settings_list_offset(settings: &DesktopSettingsState) -> u16 {
    if matches!(&settings.panel, DesktopSettingsPanel::CustomProfileAdd)
        && settings.custom_profile_error.is_some()
    {
        return 3;
    }
    if matches!(&settings.panel, DesktopSettingsPanel::WallpaperAdd)
        && settings.wallpaper_error.is_some()
    {
        return 3;
    }
    2
}

fn desktop_settings_rows(settings: &DesktopSettingsState) -> Vec<String> {
    let s = get_settings();
    match &settings.panel {
        DesktopSettingsPanel::Appearance => vec![
            format!("Theme: {} [cycle]", s.theme),
            "Wallpapers".to_string(),
            "Back".to_string(),
        ],
        DesktopSettingsPanel::General => vec![
            format!("Sound: {} [toggle]", if s.sound { "ON" } else { "OFF" }),
            format!("Bootup: {} [toggle]", if s.bootup { "ON" } else { "OFF" }),
            format!(
                "Default Open Mode: {} [toggle]",
                match s.default_open_mode {
                    OpenMode::Terminal => "Terminal",
                    OpenMode::Desktop => "Desktop",
                }
            ),
            "Back".to_string(),
        ],
        DesktopSettingsPanel::CliDisplay => vec![
            format!(
                "Styled PTY Rendering: {} [toggle]",
                if s.cli_styled_render { "ON" } else { "OFF" }
            ),
            format!(
                "PTY Color Mode: {} [cycle]",
                match s.cli_color_mode {
                    CliColorMode::ThemeLock => "Theme Lock",
                    CliColorMode::PaletteMap => "Palette-map",
                    CliColorMode::Color => "Color",
                    CliColorMode::Monochrome => "Monochrome",
                }
            ),
            format!(
                "Border Glyphs: {} [toggle]",
                match s.cli_acs_mode {
                    CliAcsMode::Ascii => "ASCII",
                    CliAcsMode::Unicode => "Unicode Smooth",
                }
            ),
            "Back".to_string(),
        ],
        DesktopSettingsPanel::Wallpapers => desktop_wallpaper_rows()
            .into_iter()
            .map(|row| row.label)
            .collect(),
        DesktopSettingsPanel::WallpaperSize => wallpaper_size_rows(),
        DesktopSettingsPanel::WallpaperChoose => wallpaper_choose_rows(),
        DesktopSettingsPanel::WallpaperDelete => wallpaper_delete_rows(),
        DesktopSettingsPanel::WallpaperAdd => vec![
            format!(
                "Name: {}",
                if settings.wallpaper_name_input.trim().is_empty() {
                    "<wallpaper name>"
                } else {
                    settings.wallpaper_name_input.trim()
                }
            ),
            format!(
                "Art File: {}",
                if settings.wallpaper_path_input.trim().is_empty() {
                    "<path/to/ascii.txt>"
                } else {
                    settings.wallpaper_path_input.trim()
                }
            ),
            format!(
                "Paste Art Editor: {}",
                if settings.wallpaper_art_input.trim().is_empty() {
                    "empty [open]"
                } else {
                    "has content [open]"
                }
            ),
            "Clear Pasted Art".to_string(),
            "Save Wallpaper".to_string(),
            "Back".to_string(),
        ],
        DesktopSettingsPanel::WallpaperPaste => Vec::new(),
        DesktopSettingsPanel::ProfileList => {
            let mut rows: Vec<String> = DESKTOP_SETTINGS_PROFILE_ITEMS
                .iter()
                .map(|(_, name)| format!("{name} Profile"))
                .collect();
            rows.push("Custom Profiles".to_string());
            rows.push("Back".to_string());
            rows
        }
        DesktopSettingsPanel::ProfileEdit(slot) => {
            let p = desktop_settings_profile_for_slot(&s.desktop_cli_profiles, *slot);
            vec![
                format!("Min Width: {}", p.min_w),
                format!("Min Height: {}", p.min_h),
                format!(
                    "Preferred Width: {}",
                    p.preferred_w
                        .map(|v| v.to_string())
                        .unwrap_or_else(|| "Auto".to_string())
                ),
                format!(
                    "Preferred Height: {}",
                    p.preferred_h
                        .map(|v| v.to_string())
                        .unwrap_or_else(|| "Auto".to_string())
                ),
                format!(
                    "Mouse Passthrough: {} [toggle]",
                    if p.mouse_passthrough { "ON" } else { "OFF" }
                ),
                format!(
                    "Open Fullscreen by Default: {} [toggle]",
                    if p.open_fullscreen { "ON" } else { "OFF" }
                ),
                "Reset Profile Defaults".to_string(),
                "Back".to_string(),
            ]
        }
        DesktopSettingsPanel::CustomProfileList => {
            let mut rows: Vec<String> = desktop_settings_custom_profile_keys()
                .into_iter()
                .map(|key| format!("{key} Profile"))
                .collect();
            rows.push(CUSTOM_PROFILE_ADD_LABEL.to_string());
            rows.push("Back".to_string());
            rows
        }
        DesktopSettingsPanel::CustomProfileEdit(key) => {
            let p = s
                .desktop_cli_profiles
                .custom
                .get(key)
                .cloned()
                .unwrap_or_default();
            vec![
                format!("Command: {key}"),
                format!("Min Width: {}", p.min_w),
                format!("Min Height: {}", p.min_h),
                format!(
                    "Preferred Width: {}",
                    p.preferred_w
                        .map(|v| v.to_string())
                        .unwrap_or_else(|| "Auto".to_string())
                ),
                format!(
                    "Preferred Height: {}",
                    p.preferred_h
                        .map(|v| v.to_string())
                        .unwrap_or_else(|| "Auto".to_string())
                ),
                format!(
                    "Mouse Passthrough: {} [toggle]",
                    if p.mouse_passthrough { "ON" } else { "OFF" }
                ),
                format!(
                    "Open Fullscreen by Default: {} [toggle]",
                    if p.open_fullscreen { "ON" } else { "OFF" }
                ),
                "Delete Custom Profile".to_string(),
                "Back".to_string(),
            ]
        }
        DesktopSettingsPanel::CustomProfileAdd => vec![
            format!(
                "Command: {}",
                if settings.custom_profile_input.trim().is_empty() {
                    "<type command name>"
                } else {
                    settings.custom_profile_input.trim()
                }
            ),
            "Create Profile".to_string(),
            "Back".to_string(),
        ],
        DesktopSettingsPanel::About => vec![
            format!("Version: v{}", env!("CARGO_PKG_VERSION")),
            format!("Current Theme: {}", s.theme),
            format!(
                "Default Open Mode: {}",
                match s.default_open_mode {
                    OpenMode::Terminal => "Terminal",
                    OpenMode::Desktop => "Desktop",
                }
            ),
            "Back".to_string(),
        ],
        DesktopSettingsPanel::Home => Vec::new(),
    }
}

fn draw_desktop_settings_window(
    f: &mut ratatui::Frame,
    area: Rect,
    settings: &DesktopSettingsState,
    focused: bool,
) {
    let content = Rect {
        x: area.x + 1,
        y: area.y + 1,
        width: area.width.saturating_sub(2),
        height: area.height.saturating_sub(2),
    };
    if content.width < 8 || content.height < 6 {
        return;
    }

    let header = match &settings.panel {
        DesktopSettingsPanel::Home => "Settings",
        DesktopSettingsPanel::Appearance => "Appearance",
        DesktopSettingsPanel::General => "General",
        DesktopSettingsPanel::CliDisplay => "CLI Display",
        DesktopSettingsPanel::Wallpapers => "Wallpapers",
        DesktopSettingsPanel::WallpaperSize => "Wallpaper Size",
        DesktopSettingsPanel::WallpaperAdd => "Add Wallpaper",
        DesktopSettingsPanel::WallpaperChoose => "Choose Wallpaper",
        DesktopSettingsPanel::WallpaperDelete => "Delete Wallpaper",
        DesktopSettingsPanel::WallpaperPaste => "Paste Wallpaper Art",
        DesktopSettingsPanel::ProfileList => "CLI Profiles",
        DesktopSettingsPanel::ProfileEdit(slot) => desktop_profile_slot_title(*slot),
        DesktopSettingsPanel::CustomProfileList => "Custom Profiles",
        DesktopSettingsPanel::CustomProfileEdit(_) => "Custom Profile",
        DesktopSettingsPanel::CustomProfileAdd => "Add Custom Profile",
        DesktopSettingsPanel::About => "About",
    };
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            format!(" {header} "),
            title_style(),
        ))),
        Rect {
            x: content.x,
            y: content.y,
            width: content.width,
            height: 1,
        },
    );

    if matches!(&settings.panel, DesktopSettingsPanel::Home) {
        let home_items = desktop_settings_home_items(settings);
        let tiles = desktop_settings_home_tiles(content, home_items.len());
        for (idx, tile) in tiles.into_iter().enumerate() {
            let item = home_items[idx];
            let icon_style = normal_style();
            let label_style = if focused && settings.hovered == Some(idx) {
                sel_style()
            } else {
                normal_style()
            };
            let icon = desktop_settings_home_icon(item);
            let label = desktop_settings_home_label(item);
            let icon_len = icon.chars().count() as u16;
            let icon_x = tile.x + tile.width.saturating_sub(icon_len) / 2;
            let label_len = label.chars().count() as u16;
            let label_x = tile.x + tile.width.saturating_sub(label_len) / 2;

            f.render_widget(
                Paragraph::new(Line::from(Span::styled(icon, icon_style))),
                Rect {
                    x: icon_x,
                    y: tile.y,
                    width: icon_len.max(1),
                    height: 1,
                },
            );
            f.render_widget(
                Paragraph::new(Line::from(Span::styled(label, label_style))),
                Rect {
                    x: label_x,
                    y: desktop_settings_home_title_rect(tile).y,
                    width: label_len.max(1),
                    height: 1,
                },
            );
        }
        return;
    }

    if matches!(&settings.panel, DesktopSettingsPanel::WallpaperPaste) {
        let name = if settings.wallpaper_name_input.trim().is_empty() {
            "<set name in previous screen>"
        } else {
            settings.wallpaper_name_input.trim()
        };
        f.render_widget(
            Paragraph::new(Line::from(Span::styled(
                format!("Name: {name}"),
                normal_style(),
            ))),
            Rect {
                x: content.x + 1,
                y: content.y + 1,
                width: content.width.saturating_sub(2),
                height: 1,
            },
        );
        let instruction = "Paste ASCII art here. Esc = Done, Backspace = Delete.";
        f.render_widget(
            Paragraph::new(Line::from(Span::styled(instruction, dim_style()))),
            Rect {
                x: content.x + 1,
                y: content.y + 2,
                width: content.width.saturating_sub(2),
                height: 1,
            },
        );

        let box_area = Rect {
            x: content.x + 1,
            y: content.y + 4,
            width: content.width.saturating_sub(2),
            height: content.height.saturating_sub(5),
        };
        if box_area.width >= 4 && box_area.height >= 3 {
            f.render_widget(
                Block::default().borders(Borders::ALL).style(title_style()),
                box_area,
            );
            let inner = Rect {
                x: box_area.x + 1,
                y: box_area.y + 1,
                width: box_area.width.saturating_sub(2),
                height: box_area.height.saturating_sub(2),
            };
            if inner.width > 0 && inner.height > 0 {
                let all_lines: Vec<String> = if settings.wallpaper_art_input.is_empty() {
                    Vec::new()
                } else {
                    normalize_wallpaper_lines(
                        settings
                            .wallpaper_art_input
                            .lines()
                            .map(|line| line.to_string())
                            .collect(),
                    )
                };
                let visible = inner.height as usize;
                let start = all_lines.len().saturating_sub(visible);
                let mut lines = Vec::new();
                for line in all_lines.into_iter().skip(start) {
                    let clipped: String = line.chars().take(inner.width as usize).collect();
                    lines.push(Line::from(Span::styled(clipped, normal_style())));
                }
                if lines.is_empty() {
                    lines.push(Line::from(Span::styled("<empty>", dim_style())));
                }
                f.render_widget(Paragraph::new(lines), inner);
            }
        }
        return;
    }

    if let Some(err) = settings.custom_profile_error.as_ref() {
        if matches!(&settings.panel, DesktopSettingsPanel::CustomProfileAdd) {
            f.render_widget(
                Paragraph::new(Line::from(Span::styled(format!(" ! {err}"), dim_style()))),
                Rect {
                    x: content.x,
                    y: content.y + 2,
                    width: content.width,
                    height: 1,
                },
            );
        }
    }
    if let Some(err) = settings.wallpaper_error.as_ref() {
        if matches!(&settings.panel, DesktopSettingsPanel::WallpaperAdd) {
            f.render_widget(
                Paragraph::new(Line::from(Span::styled(format!(" ! {err}"), dim_style()))),
                Rect {
                    x: content.x,
                    y: content.y + 2,
                    width: content.width,
                    height: 1,
                },
            );
        }
    }

    let list_y = content.y + desktop_settings_list_offset(settings);
    let list_h = content
        .height
        .saturating_sub(desktop_settings_list_offset(settings));
    let list_x = content.x + 1;
    let list_w = content.width.saturating_sub(1);
    let show_preview = matches!(
        settings.panel,
        DesktopSettingsPanel::Wallpapers
            | DesktopSettingsPanel::WallpaperSize
            | DesktopSettingsPanel::WallpaperChoose
    ) && content.width >= 70
        && list_h >= 8;

    let (rows_area, preview_area) = if show_preview {
        let left_w = ((list_w as u32) * 48 / 100) as u16;
        let right_w = list_w.saturating_sub(left_w + 1);
        (
            Rect {
                x: list_x,
                y: list_y,
                width: left_w.max(18),
                height: list_h,
            },
            Some(Rect {
                x: list_x + left_w + 1,
                y: list_y,
                width: right_w,
                height: list_h,
            }),
        )
    } else {
        (
            Rect {
                x: list_x,
                y: list_y,
                width: list_w,
                height: list_h,
            },
            None,
        )
    };

    let rows = desktop_settings_rows(settings);
    let mut lines = Vec::new();
    for (idx, row) in rows.iter().enumerate() {
        let style = if focused && settings.hovered == Some(idx) {
            sel_style()
        } else {
            normal_style()
        };
        lines.push(Line::from(Span::styled(row.as_str(), style)));
    }
    f.render_widget(Paragraph::new(lines), rows_area);

    if let Some(preview) = preview_area {
        if preview.width >= 6 && preview.height >= 4 {
            let title = wallpaper_preview_name(settings)
                .map(|name| format!(" Preview: {name} "))
                .unwrap_or_else(|| " Preview ".to_string());
            f.render_widget(
                Block::default()
                    .borders(Borders::ALL)
                    .title(Line::from(Span::styled(title, title_style())))
                    .style(title_style()),
                preview,
            );

            if let Some(name) = wallpaper_preview_name(settings) {
                let cfg = get_settings();
                if let Some(lines) = wallpaper_lines_for_name(&cfg, &name) {
                    let inner = Rect {
                        x: preview.x + 1,
                        y: preview.y + 1,
                        width: preview.width.saturating_sub(2),
                        height: preview.height.saturating_sub(2),
                    };
                    if inner.width > 0 && inner.height > 0 {
                        let render = render_wallpaper_for_mode(
                            &lines,
                            cfg.desktop_wallpaper_size_mode,
                            inner.width as usize,
                            inner.height as usize,
                        );
                        let render_h = render.len();
                        let render_w = render
                            .iter()
                            .map(|line| line.chars().count())
                            .max()
                            .unwrap_or(0);
                        if render_h > 0 && render_w > 0 {
                            let mut render_lines = Vec::new();
                            for line in render {
                                render_lines.push(Line::from(Span::styled(line, normal_style())));
                            }
                            f.render_widget(
                                Paragraph::new(render_lines),
                                Rect {
                                    x: inner.x + inner.width.saturating_sub(render_w as u16) / 2,
                                    y: inner.y + inner.height.saturating_sub(render_h as u16) / 2,
                                    width: render_w as u16,
                                    height: render_h as u16,
                                },
                            );
                        }
                    }
                }
            }
        }
    }
}

fn draw_pty_window(f: &mut ratatui::Frame, area: Rect, app: &PtyWindowState) {
    let inner = Rect {
        x: area.x + 1,
        y: area.y + 1,
        width: area.width.saturating_sub(2),
        height: area.height.saturating_sub(2),
    };
    if inner.height == 0 || inner.width == 0 {
        return;
    }
    app.session.render(f, inner);
}

fn draw_start_menu(f: &mut ratatui::Frame, size: Rect, state: &DesktopState) {
    let task = taskbar_area(size);
    let root = start_root_rect(task);
    f.render_widget(Clear, root);
    f.render_widget(
        Block::default().borders(Borders::ALL).style(title_style()),
        root,
    );

    let inner_root_w = root.width.saturating_sub(2) as usize;
    let mut root_lines = Vec::new();
    for row in START_ROOT_VIS_ROWS {
        match row {
            Some(i) => {
                let (label, submenu) = START_ROOT_ITEMS[i];
                let style = if i == state.start.selected_root {
                    sel_style()
                } else {
                    normal_style()
                };
                let arrow = if submenu.is_some() { Some('>') } else { None };
                root_lines.push(Line::from(Span::styled(
                    format_menu_row(inner_root_w, label, arrow),
                    style,
                )));
            }
            None => {
                root_lines.push(Line::from(Span::styled(
                    "-".repeat(inner_root_w),
                    dim_style(),
                )));
            }
        }
    }
    f.render_widget(
        Paragraph::new(root_lines),
        Rect {
            x: root.x + 1,
            y: root.y + 1,
            width: root.width.saturating_sub(2),
            height: root.height.saturating_sub(2),
        },
    );

    if let Some(submenu) = state.start.open_submenu {
        let sub = start_submenu_rect(root, size, submenu);
        f.render_widget(Clear, sub);
        f.render_widget(
            Block::default().borders(Borders::ALL).style(title_style()),
            sub,
        );
        let inner_sub_w = sub.width.saturating_sub(2) as usize;
        let mut sub_lines = Vec::new();
        let rows = submenu_visual_rows(submenu);
        let selected = submenu_selected_idx(&state.start, submenu);
        for row in rows {
            match row {
                Some(i) => {
                    let (label, arrow) = match submenu {
                        StartSubmenu::Programs => {
                            let (label, _) = submenu_items_programs()[*i];
                            (label, Some('>'))
                        }
                        StartSubmenu::System => {
                            let (label, _) = submenu_items_system()[*i];
                            (label, None)
                        }
                    };
                    let style = if *i == selected {
                        sel_style()
                    } else {
                        normal_style()
                    };
                    sub_lines.push(Line::from(Span::styled(
                        format_menu_row(inner_sub_w, label, arrow),
                        style,
                    )));
                }
                None => {
                    sub_lines.push(Line::from(Span::styled(
                        "-".repeat(inner_sub_w),
                        dim_style(),
                    )));
                }
            }
        }
        f.render_widget(
            Paragraph::new(sub_lines),
            Rect {
                x: sub.x + 1,
                y: sub.y + 1,
                width: sub.width.saturating_sub(2),
                height: sub.height.saturating_sub(2),
            },
        );

        if submenu == StartSubmenu::Programs {
            if let Some(leaf) = state.start.open_leaf {
                let leaf_rect = start_leaf_rect(sub, size, &state.start, leaf);
                f.render_widget(Clear, leaf_rect);
                f.render_widget(
                    Block::default().borders(Borders::ALL).style(title_style()),
                    leaf_rect,
                );
                let inner_leaf_w = leaf_rect.width.saturating_sub(2) as usize;
                let mut leaf_lines = Vec::new();
                let items = leaf_items(&state.start, leaf);
                let selected_leaf = leaf_selected_idx(&state.start, leaf);
                for (idx, item) in items.iter().enumerate() {
                    let style = if idx == selected_leaf {
                        sel_style()
                    } else {
                        normal_style()
                    };
                    leaf_lines.push(Line::from(Span::styled(
                        format_menu_row(inner_leaf_w, &item.label, None),
                        style,
                    )));
                }
                f.render_widget(
                    Paragraph::new(leaf_lines),
                    Rect {
                        x: leaf_rect.x + 1,
                        y: leaf_rect.y + 1,
                        width: leaf_rect.width.saturating_sub(2),
                        height: leaf_rect.height.saturating_sub(2),
                    },
                );
            }
        }
    }
}

fn draw_cursor(f: &mut ratatui::Frame, x: u16, y: u16, size: Rect) {
    if x >= size.width || y >= size.height {
        return;
    }
    f.render_widget(
        Paragraph::new(Line::from(Span::styled("+", sel_style()))),
        Rect {
            x,
            y,
            width: 1,
            height: 1,
        },
    );
}

fn hit_start_menu(
    x: u16,
    y: u16,
    size: Rect,
    state: &mut DesktopState,
    is_click: bool,
) -> Option<StartAction> {
    let root = start_root_rect(taskbar_area(size));
    if point_in_rect(x, y, root) {
        let row = y.saturating_sub(root.y + 1) as usize;
        if row < START_ROOT_VIS_ROWS.len() {
            let Some(root_idx) = START_ROOT_VIS_ROWS[row] else {
                return Some(StartAction::None);
            };
            state.start.selected_root = root_idx;
            if let Some(sub) = submenu_for_root(root_idx) {
                if is_click {
                    apply_hover_target(&mut state.start, StartHoverTarget::Submenu(sub));
                    state.start.hover_candidate = None;
                } else {
                    queue_start_hover(&mut state.start, StartHoverTarget::Submenu(sub));
                }
                return Some(StartAction::None);
            }
            state.start.open_submenu = None;
            state.start.open_leaf = None;
            state.start.hover_candidate = None;
            return Some(match root_idx {
                2 => StartAction::ReturnToTerminal,
                3 => StartAction::Logout,
                4 => StartAction::Shutdown,
                _ => StartAction::None,
            });
        }
        return Some(StartAction::None);
    }

    if let Some(submenu) = state.start.open_submenu {
        let sub = start_submenu_rect(root, size, submenu);
        if point_in_rect(x, y, sub) {
            let row = y.saturating_sub(sub.y + 1) as usize;
            let vis = submenu_visual_rows(submenu);
            if row < vis.len() {
                let Some(item_idx) = vis[row] else {
                    return Some(StartAction::None);
                };
                *submenu_selected_idx_mut(&mut state.start, submenu) = item_idx;
                return Some(match submenu {
                    StartSubmenu::Programs => {
                        if let Some(leaf) = leaf_from_program_idx(item_idx) {
                            if is_click {
                                apply_hover_target(&mut state.start, StartHoverTarget::Leaf(leaf));
                                state.start.hover_candidate = None;
                            } else {
                                queue_start_hover(&mut state.start, StartHoverTarget::Leaf(leaf));
                            }
                        }
                        StartAction::None
                    }
                    StartSubmenu::System => {
                        if is_click {
                            let items = submenu_items_system();
                            StartAction::Launch(items[item_idx].1)
                        } else {
                            StartAction::None
                        }
                    }
                });
            }
            return Some(StartAction::None);
        }

        if submenu == StartSubmenu::Programs {
            if let Some(leaf) = state.start.open_leaf {
                let leaf_rect = start_leaf_rect(sub, size, &state.start, leaf);
                if point_in_rect(x, y, leaf_rect) {
                    let row = y.saturating_sub(leaf_rect.y + 1) as usize;
                    let leaf_len = leaf_items(&state.start, leaf).len();
                    if row < leaf_len {
                        *leaf_selected_idx_mut(&mut state.start, leaf) = row;
                        if is_click {
                            return Some(leaf_items(&state.start, leaf)[row].action.clone());
                        }
                    }
                    return Some(StartAction::None);
                }
            }
        }
    }

    None
}

fn hit_window(state: &DesktopState, x: u16, y: u16) -> Option<(u64, WindowHit)> {
    for win in state.windows.iter().rev() {
        if win.minimized {
            continue;
        }
        let rect = win.rect;
        if !rect.contains(x, y) {
            continue;
        }
        let area = rect.to_rect();
        if point_in_rect(x, y, title_close_button_rect(area)) {
            return Some((win.id, WindowHit::Close));
        }
        if point_in_rect(x, y, title_max_button_rect(area)) {
            return Some((win.id, WindowHit::Maximize));
        }
        if point_in_rect(x, y, title_min_button_rect(area)) {
            return Some((win.id, WindowHit::Minimize));
        }
        if !win.maximized {
            if let Some(corner) = hit_resize_corner(area, x, y) {
                return Some((win.id, WindowHit::Resize(corner)));
            }
        }
        if y == area.y {
            return Some((win.id, WindowHit::Title));
        }
        return Some((win.id, WindowHit::Content));
    }
    None
}

fn handle_window_content_mouse(
    terminal: &mut Term,
    current_user: &str,
    state: &mut DesktopState,
    mouse: crossterm::event::MouseEvent,
) -> Result<()> {
    let Some(idx_last) = state.windows.len().checked_sub(1) else {
        return Ok(());
    };
    let mut close_window_id = None;
    let mut settings_action = DesktopSettingsAction::None;
    let mut settings_window_id = None;
    let mut refresh_file_managers = false;
    let clicked_target = {
        let win = &mut state.windows[idx_last];
        let rect = win.rect;
        match &mut win.kind {
            WindowKind::PtyApp(app) => {
                if !app.mouse_passthrough {
                    return Ok(());
                }
                if let Some((col, row)) = pty_local_coords_from_rect(rect, mouse.column, mouse.row)
                {
                    app.session
                        .send_mouse_event(mouse.kind, mouse.modifiers, col, row);
                }
                return Ok(());
            }
            WindowKind::DesktopSettings(settings) => {
                settings_action = handle_desktop_settings_mouse(settings, rect.to_rect(), mouse);
                settings_window_id = Some(win.id);
                if matches!(settings_action, DesktopSettingsAction::CloseWindow) {
                    close_window_id = Some(win.id);
                }
                None
            }
            WindowKind::FileManager(fm) => {
                if !matches!(mouse.kind, MouseEventKind::Down(MouseButton::Left)) {
                    return Ok(());
                }
                let cfg = get_settings().desktop_file_manager;
                let content = file_manager_content_rect(win.rect.to_rect());
                if !point_in_rect(mouse.column, mouse.row, content) {
                    return Ok(());
                }
                if mouse.row == content.y {
                    let rel_x = mouse.column.saturating_sub(content.x) as usize;
                    if let Some(tab_idx) = file_manager_tab_index_at(fm, content.width as usize, rel_x) {
                        let _ = fm.switch_to_tab(tab_idx);
                        let entry_area = file_manager_entry_rect(content, cfg.show_tree_panel);
                        file_manager_ensure_selection_visible(fm, entry_area);
                    }
                    return Ok(());
                }
                if mouse.row == content.y.saturating_add(1) {
                    fm.search_mode = true;
                    fm.tree_focus = false;
                    return Ok(());
                }
                let (tree_area, entry_area) =
                    file_manager_tree_and_entry_rects(content, cfg.show_tree_panel);
                if let Some(tree_rect) = tree_area {
                    if point_in_rect(mouse.column, mouse.row, tree_rect) {
                        let items = file_manager_tree_items(&fm.cwd, cfg.show_hidden_files);
                        let row = (mouse.row - tree_rect.y) as usize;
                        let visible = tree_rect.height as usize;
                        if row >= visible {
                            return Ok(());
                        }
                        let start = fm.tree_scroll.min(items.len().saturating_sub(visible));
                        let idx = start + row;
                        if idx >= items.len() {
                            return Ok(());
                        }
                        fm.tree_selected = idx;
                        fm.tree_focus = true;
                        fm.search_mode = false;
                        let _ = fm.open_selected_tree_path();
                        file_manager_ensure_tree_selection_visible(fm, tree_rect);
                        file_manager_ensure_selection_visible(fm, entry_area);
                        return Ok(());
                    }
                }

                if !point_in_rect(mouse.column, mouse.row, entry_area) || entry_area.height == 0 {
                    return Ok(());
                }
                let idx = match cfg.view_mode {
                    FileManagerViewMode::List => {
                        let row = (mouse.row - entry_area.y) as usize;
                        let visible_rows = entry_area.height as usize;
                        if row >= visible_rows {
                            return Ok(());
                        }
                        let start = fm
                            .scroll
                            .min(file_manager_list_max_scroll(fm.entries.len(), visible_rows));
                        let idx = start + row;
                        if idx >= fm.entries.len() {
                            return Ok(());
                        }
                        idx
                    }
                    FileManagerViewMode::Grid => {
                        let (cols, visible_rows) = file_manager_grid_metrics(entry_area);
                        if cols == 0 || visible_rows == 0 {
                            return Ok(());
                        }
                        let start_row = fm.scroll.min(file_manager_grid_max_scroll(
                            fm.entries.len(),
                            cols,
                            visible_rows,
                        ));
                        let cell_width = (entry_area.width / cols as u16).max(1);
                        let col = ((mouse.column - entry_area.x) / cell_width) as usize;
                        let row =
                            ((mouse.row - entry_area.y) / FILE_MANAGER_GRID_CELL_HEIGHT) as usize;
                        if col >= cols || row >= visible_rows {
                            return Ok(());
                        }
                        let idx = (start_row + row) * cols + col;
                        if idx >= fm.entries.len() {
                            return Ok(());
                        }
                        idx
                    }
                };
                fm.selected = idx;
                fm.tree_focus = false;
                fm.search_mode = false;
                file_manager_ensure_selection_visible(fm, entry_area);
                Some(ClickTarget::FileEntry {
                    window_id: win.id,
                    row: idx,
                })
            }
            WindowKind::FileManagerSettings(settings) => {
                if !matches!(mouse.kind, MouseEventKind::Down(MouseButton::Left)) {
                    return Ok(());
                }
                let area = win.rect.to_rect();
                let content = Rect {
                    x: area.x + 1,
                    y: area.y + 1,
                    width: area.width.saturating_sub(2),
                    height: area.height.saturating_sub(2),
                };
                if !point_in_rect(mouse.column, mouse.row, content) {
                    return Ok(());
                }
                let row = mouse.row.saturating_sub(content.y) as usize;
                if row >= file_manager_settings_rows().len() {
                    return Ok(());
                }
                settings.selected = row;
                let (refresh, close) =
                    handle_file_manager_settings_key(settings, KeyCode::Enter, KeyModifiers::NONE);
                if refresh {
                    refresh_file_managers = true;
                }
                if close {
                    close_window_id = Some(win.id);
                }
                None
            }
        }
    };

    if !matches!(settings_action, DesktopSettingsAction::None) {
        if let Some(window_id) = settings_window_id {
            run_desktop_settings_action(terminal, current_user, state, window_id, settings_action)?;
        }
    }

    if let Some(id) = close_window_id {
        if state.windows.iter().any(|w| w.id == id) {
            close_window_by_id(state, id);
        }
        if refresh_file_managers {
            refresh_all_file_manager_windows(state);
        }
        return Ok(());
    }

    if refresh_file_managers {
        refresh_all_file_manager_windows(state);
    }

    let Some(clicked_target) = clicked_target else {
        return Ok(());
    };
    if is_double_click(state, clicked_target) {
        if let ClickTarget::FileEntry { window_id, .. } = clicked_target {
            let pending = if let Some(win) = state.windows.iter_mut().find(|w| w.id == window_id) {
                if let WindowKind::FileManager(fm) = &mut win.kind {
                    fm.activate_selected(FileManagerOpenRequest::Builtin)
                } else {
                    None
                }
            } else {
                None
            };
            if let Some((path, request)) = pending {
                handle_file_open_request(terminal, state, &path, request)?;
            }
        }
    }
    Ok(())
}

fn pty_local_coords_from_rect(rect: WinRect, x: u16, y: u16) -> Option<(u16, u16)> {
    let area = rect.to_rect();
    let content = Rect {
        x: area.x + 1,
        y: area.y + 1,
        width: area.width.saturating_sub(2),
        height: area.height.saturating_sub(2),
    };
    if content.width == 0 || content.height == 0 || !point_in_rect(x, y, content) {
        return None;
    }
    Some((x - content.x + 1, y - content.y + 1))
}

fn send_mouse_to_focused_pty(
    state: &mut DesktopState,
    mouse: crossterm::event::MouseEvent,
) -> bool {
    let Some(idx) = focused_visible_window_idx(state) else {
        return false;
    };
    let win = &mut state.windows[idx];
    if win.minimized {
        return false;
    }
    let rect = win.rect;
    let Some((col, row)) = pty_local_coords_from_rect(rect, mouse.column, mouse.row) else {
        return false;
    };
    let WindowKind::PtyApp(app) = &mut win.kind else {
        return false;
    };
    if !app.mouse_passthrough {
        return false;
    }
    app.session
        .send_mouse_event(mouse.kind, mouse.modifiers, col, row);
    true
}

fn handle_file_manager_scroll_mouse(state: &mut DesktopState, mouse: crossterm::event::MouseEvent) -> bool {
    let delta = match mouse.kind {
        MouseEventKind::ScrollUp => -1,
        MouseEventKind::ScrollDown => 1,
        _ => 0,
    };
    if delta == 0 {
        return false;
    }

    let mut target: Option<(usize, bool)> = None;
    for idx in (0..state.windows.len()).rev() {
        let win = &state.windows[idx];
        if win.minimized || !matches!(win.kind, WindowKind::FileManager(_)) {
            continue;
        }
        let content = file_manager_content_rect(win.rect.to_rect());
        let show_tree = get_settings().desktop_file_manager.show_tree_panel;
        let (tree_area, entry_area) = file_manager_tree_and_entry_rects(content, show_tree);
        if tree_area.is_some_and(|tree| point_in_rect(mouse.column, mouse.row, tree)) {
            target = Some((idx, true));
            break;
        }
        if point_in_rect(mouse.column, mouse.row, entry_area) {
            target = Some((idx, false));
            break;
        }
    }
    if target.is_none() {
        if let Some(idx) = focused_visible_window_idx(state) {
            if let WindowKind::FileManager(fm) = &state.windows[idx].kind {
                let show_tree = get_settings().desktop_file_manager.show_tree_panel;
                target = Some((idx, show_tree && fm.tree_focus));
            }
        }
    }

    let Some((idx, tree_target)) = target else {
        return false;
    };
    let content = file_manager_content_rect(state.windows[idx].rect.to_rect());
    let show_tree = get_settings().desktop_file_manager.show_tree_panel;
    let (tree_area, entry_area) = file_manager_tree_and_entry_rects(content, show_tree);
    let WindowKind::FileManager(fm) = &mut state.windows[idx].kind else {
        return false;
    };
    if tree_target {
        let Some(tree_rect) = tree_area else {
            return false;
        };
        file_manager_tree_apply_scroll_delta(fm, tree_rect, delta)
    } else {
        file_manager_apply_scroll_delta(fm, entry_area, delta)
    }
}

fn open_file_manager_window(state: &mut DesktopState) {
    if let Some(id) = state.windows.iter().find_map(|w| {
        if matches!(&w.kind, WindowKind::FileManager(_)) {
            Some(w.id)
        } else {
            None
        }
    }) {
        focus_window(state, id);
        return;
    }

    let id = state.next_id;
    state.next_id += 1;
    state.windows.push(DesktopWindow {
        id,
        title: "My Computer".to_string(),
        rect: WinRect {
            x: 8,
            y: 4,
            w: 72,
            h: 22,
        },
        restore_rect: None,
        minimized: false,
        maximized: false,
        kind: WindowKind::FileManager(FileManagerState::new()),
    });
}

fn open_file_manager_settings_window(state: &mut DesktopState) {
    if let Some(id) = state.windows.iter().find_map(|w| {
        if matches!(&w.kind, WindowKind::FileManagerSettings(_)) {
            Some(w.id)
        } else {
            None
        }
    }) {
        focus_window(state, id);
        return;
    }

    let id = state.next_id;
    state.next_id += 1;
    state.windows.push(DesktopWindow {
        id,
        title: "File Manager Settings".to_string(),
        rect: WinRect {
            x: 14,
            y: 6,
            w: 58,
            h: 14,
        },
        restore_rect: None,
        minimized: false,
        maximized: false,
        kind: WindowKind::FileManagerSettings(FileManagerSettingsState { selected: 0 }),
    });
}

fn refresh_all_file_manager_windows(state: &mut DesktopState) {
    for win in &mut state.windows {
        if let WindowKind::FileManager(fm) = &mut win.kind {
            fm.refresh();
        }
    }
}

fn handle_file_manager_settings_key(
    settings: &mut FileManagerSettingsState,
    code: KeyCode,
    _modifiers: KeyModifiers,
) -> (bool, bool) {
    let rows = file_manager_settings_rows();
    let max = rows.len().saturating_sub(1);
    match code {
        KeyCode::Up => settings.selected = settings.selected.saturating_sub(1),
        KeyCode::Down => settings.selected = (settings.selected + 1).min(max),
        KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') => return (false, true),
        KeyCode::Left => {
            if settings.selected == 3 {
                update_settings(|s| {
                    s.desktop_file_manager.sort_mode = match s.desktop_file_manager.sort_mode {
                        FileManagerSortMode::Name => FileManagerSortMode::Type,
                        FileManagerSortMode::Type => FileManagerSortMode::Name,
                    };
                });
                persist_settings();
                return (true, false);
            }
        }
        KeyCode::Right => {
            if settings.selected == 3 {
                update_settings(|s| {
                    s.desktop_file_manager.sort_mode = match s.desktop_file_manager.sort_mode {
                        FileManagerSortMode::Name => FileManagerSortMode::Type,
                        FileManagerSortMode::Type => FileManagerSortMode::Name,
                    };
                });
                persist_settings();
                return (true, false);
            }
        }
        KeyCode::Enter | KeyCode::Char(' ') => match settings.selected {
            0 => {
                update_settings(|s| {
                    s.desktop_file_manager.show_hidden_files =
                        !s.desktop_file_manager.show_hidden_files;
                });
                persist_settings();
                return (true, false);
            }
            1 => {
                update_settings(|s| {
                    s.desktop_file_manager.show_tree_panel = !s.desktop_file_manager.show_tree_panel;
                });
                persist_settings();
                return (true, false);
            }
            2 => {
                update_settings(|s| {
                    s.desktop_file_manager.view_mode = match s.desktop_file_manager.view_mode {
                        FileManagerViewMode::Grid => FileManagerViewMode::List,
                        FileManagerViewMode::List => FileManagerViewMode::Grid,
                    };
                });
                persist_settings();
                return (true, false);
            }
            3 => {
                update_settings(|s| {
                    s.desktop_file_manager.sort_mode = match s.desktop_file_manager.sort_mode {
                        FileManagerSortMode::Name => FileManagerSortMode::Type,
                        FileManagerSortMode::Type => FileManagerSortMode::Name,
                    };
                });
                persist_settings();
                return (true, false);
            }
            4 => {
                update_settings(|s| {
                    s.desktop_file_manager.directories_first =
                        !s.desktop_file_manager.directories_first;
                });
                persist_settings();
                return (true, false);
            }
            5 => {
                update_settings(|s| {
                    s.desktop_file_manager.text_open_mode = match s.desktop_file_manager.text_open_mode
                    {
                        FileManagerTextOpenMode::Editor => FileManagerTextOpenMode::Viewer,
                        FileManagerTextOpenMode::Viewer => FileManagerTextOpenMode::Editor,
                    };
                });
                persist_settings();
                return (true, false);
            }
            6 => return (false, true),
            _ => {}
        },
        _ => {}
    }
    (false, false)
}

fn open_focused_file_manager_selection(
    terminal: &mut Term,
    state: &mut DesktopState,
    request: FileManagerOpenRequest,
) -> Result<()> {
    let Some(idx) = focused_visible_window_idx(state) else {
        return Ok(());
    };
    let pending = match &mut state.windows[idx].kind {
        WindowKind::FileManager(fm) => fm.activate_selected(request),
        _ => None,
    };
    if let Some((path, mode)) = pending {
        handle_file_open_request(terminal, state, &path, mode)?;
    }
    Ok(())
}

fn handle_file_open_request(
    terminal: &mut Term,
    _state: &mut DesktopState,
    path: &Path,
    request: FileManagerOpenRequest,
) -> Result<()> {
    match request {
        FileManagerOpenRequest::Builtin => {
            let open_mode = get_settings().desktop_file_manager.text_open_mode;
            run_with_mouse_capture_paused(terminal, |t| match open_mode {
                FileManagerTextOpenMode::Editor => documents::edit_text_file(t, path),
                FileManagerTextOpenMode::Viewer => documents::view_text_file(t, path),
            })?;
        }
        FileManagerOpenRequest::External => {
            #[cfg(target_os = "macos")]
            let status = std::process::Command::new("open").arg(path).status();
            #[cfg(target_os = "linux")]
            let status = std::process::Command::new("xdg-open").arg(path).status();
            #[cfg(target_os = "windows")]
            let status = std::process::Command::new("cmd")
                .args(["/C", "start", "", &path.display().to_string()])
                .status();
            #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
            let status = std::process::Command::new("xdg-open").arg(path).status();
            if status.is_err() {
                flash_message(terminal, "External open failed", 1000)?;
            }
        }
    }
    Ok(())
}

fn open_desktop_settings_window(terminal: &mut Term, state: &mut DesktopState, current_user: &str) {
    if let Some(id) = state.windows.iter().find_map(|w| {
        if matches!(&w.kind, WindowKind::DesktopSettings(_)) {
            Some(w.id)
        } else {
            None
        }
    }) {
        if let Some(win) = state.windows.iter_mut().find(|w| w.id == id) {
            win.title = "Settings".to_string();
            if let WindowKind::DesktopSettings(settings) = &mut win.kind {
                settings.is_admin = is_admin(current_user);
            }
        }
        focus_window(state, id);
        return;
    }

    let (rect, id) = if let Ok(size) = terminal.size() {
        let full = full_rect(size.width, size.height);
        let desk = desktop_area(full);
        let mut rect = WinRect {
            x: desk.x as i32 + 6,
            y: desk.y as i32 + 3,
            w: desk.width.saturating_sub(12).clamp(64, 112),
            h: desk.height.saturating_sub(6).clamp(18, 40),
        };
        clamp_window_with_min(&mut rect, desk, 64, 18);
        (rect, state.next_id)
    } else {
        (
            WinRect {
                x: 10,
                y: 4,
                w: 84,
                h: 26,
            },
            state.next_id,
        )
    };

    let mut settings_state = DesktopSettingsState::default();
    settings_state.is_admin = is_admin(current_user);

    state.next_id += 1;
    state.windows.push(DesktopWindow {
        id,
        title: "Settings".to_string(),
        rect,
        restore_rect: None,
        minimized: false,
        maximized: false,
        kind: WindowKind::DesktopSettings(settings_state),
    });
}

fn desktop_settings_profile_for_slot_mut(
    profiles: &mut DesktopCliProfiles,
    slot: DesktopProfileSlot,
) -> &mut DesktopPtyProfileSettings {
    match slot {
        DesktopProfileSlot::Default => &mut profiles.default,
        DesktopProfileSlot::Calcurse => &mut profiles.calcurse,
        DesktopProfileSlot::SpotifyPlayer => &mut profiles.spotify_player,
        DesktopProfileSlot::Ranger => &mut profiles.ranger,
        DesktopProfileSlot::Reddit => &mut profiles.reddit,
    }
}

fn desktop_settings_profile_for_slot(
    profiles: &DesktopCliProfiles,
    slot: DesktopProfileSlot,
) -> &DesktopPtyProfileSettings {
    match slot {
        DesktopProfileSlot::Default => &profiles.default,
        DesktopProfileSlot::Calcurse => &profiles.calcurse,
        DesktopProfileSlot::SpotifyPlayer => &profiles.spotify_player,
        DesktopProfileSlot::Ranger => &profiles.ranger,
        DesktopProfileSlot::Reddit => &profiles.reddit,
    }
}

fn desktop_settings_default_profile(slot: DesktopProfileSlot) -> DesktopPtyProfileSettings {
    let defaults = DesktopCliProfiles::default();
    desktop_settings_profile_for_slot(&defaults, slot).clone()
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum DesktopProfileTarget {
    Builtin(DesktopProfileSlot),
    Custom(String),
}

fn normalize_profile_key(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    let base = Path::new(trimmed)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or(trimmed)
        .trim();
    if base.is_empty() {
        None
    } else {
        Some(base.to_ascii_lowercase())
    }
}

fn is_builtin_profile_key(key: &str) -> bool {
    matches!(
        key,
        "calcurse" | "spotify_player" | "ranger" | "tuir" | "rtv"
    )
}

fn desktop_settings_profile_for_target_mut<'a>(
    profiles: &'a mut DesktopCliProfiles,
    target: &DesktopProfileTarget,
) -> Option<&'a mut DesktopPtyProfileSettings> {
    match target {
        DesktopProfileTarget::Builtin(slot) => {
            Some(desktop_settings_profile_for_slot_mut(profiles, *slot))
        }
        DesktopProfileTarget::Custom(key) => profiles.custom.get_mut(key),
    }
}

fn desktop_settings_profile_default_for_target(
    target: &DesktopProfileTarget,
) -> DesktopPtyProfileSettings {
    match target {
        DesktopProfileTarget::Builtin(slot) => desktop_settings_default_profile(*slot),
        DesktopProfileTarget::Custom(_) => DesktopPtyProfileSettings::default(),
    }
}

fn desktop_settings_row_count(state: &DesktopSettingsState) -> usize {
    match &state.panel {
        DesktopSettingsPanel::Home => desktop_settings_home_items(state).len(),
        DesktopSettingsPanel::Appearance => 3,
        DesktopSettingsPanel::General => 4,
        DesktopSettingsPanel::CliDisplay => 4,
        DesktopSettingsPanel::Wallpapers => desktop_wallpaper_rows().len(),
        DesktopSettingsPanel::WallpaperSize => wallpaper_size_rows().len(),
        DesktopSettingsPanel::WallpaperChoose => wallpaper_choose_rows().len(),
        DesktopSettingsPanel::WallpaperDelete => wallpaper_delete_rows().len(),
        DesktopSettingsPanel::WallpaperAdd => 6,
        DesktopSettingsPanel::WallpaperPaste => 0,
        DesktopSettingsPanel::ProfileList => DESKTOP_SETTINGS_PROFILE_ITEMS.len() + 2,
        DesktopSettingsPanel::ProfileEdit(_) => 8,
        DesktopSettingsPanel::CustomProfileList => desktop_settings_custom_profile_keys().len() + 2,
        DesktopSettingsPanel::CustomProfileEdit(_) => 9,
        DesktopSettingsPanel::CustomProfileAdd => 3,
        DesktopSettingsPanel::About => 4,
    }
}

fn desktop_settings_reset_selection(state: &mut DesktopSettingsState) {
    let max = desktop_settings_row_count(state).saturating_sub(1);
    state.selected = state.selected.min(max);
    if state
        .hovered
        .is_some_and(|idx| idx >= desktop_settings_row_count(state))
    {
        state.hovered = None;
    }
}

fn desktop_settings_apply_open_mode_toggle() {
    update_settings(|s| {
        s.default_open_mode = match s.default_open_mode {
            OpenMode::Terminal => OpenMode::Desktop,
            OpenMode::Desktop => OpenMode::Terminal,
        };
    });
    persist_settings();
}

fn desktop_settings_cycle_theme(forward: bool) {
    let current = get_settings().theme;
    let idx = THEMES
        .iter()
        .position(|(name, _)| *name == current)
        .unwrap_or(0);
    let next_idx = if forward {
        (idx + 1) % THEMES.len()
    } else if idx == 0 {
        THEMES.len().saturating_sub(1)
    } else {
        idx - 1
    };
    let next = THEMES[next_idx].0.to_string();
    update_settings(|s| s.theme = next);
    persist_settings();
}

fn desktop_settings_cycle_color(forward: bool) {
    update_settings(|s| {
        s.cli_color_mode = match (s.cli_color_mode, forward) {
            (CliColorMode::ThemeLock, true) => CliColorMode::PaletteMap,
            (CliColorMode::PaletteMap, true) => CliColorMode::Color,
            (CliColorMode::Color, true) => CliColorMode::Monochrome,
            (CliColorMode::Monochrome, true) => CliColorMode::ThemeLock,
            (CliColorMode::ThemeLock, false) => CliColorMode::Monochrome,
            (CliColorMode::PaletteMap, false) => CliColorMode::ThemeLock,
            (CliColorMode::Color, false) => CliColorMode::PaletteMap,
            (CliColorMode::Monochrome, false) => CliColorMode::Color,
        };
    });
    persist_settings();
}

fn desktop_settings_adjust_profile_number(target: &DesktopProfileTarget, row: usize, delta: i16) {
    if delta == 0 {
        return;
    }
    update_settings(|s| {
        let Some(p) = desktop_settings_profile_for_target_mut(&mut s.desktop_cli_profiles, target)
        else {
            return;
        };
        match row {
            0 => {
                let next =
                    (i32::from(p.min_w) + i32::from(delta)).clamp(i32::from(MIN_WINDOW_W), 240);
                p.min_w = next as u16;
                if let Some(w) = p.preferred_w {
                    if w < p.min_w {
                        p.preferred_w = Some(p.min_w);
                    }
                }
            }
            1 => {
                let next =
                    (i32::from(p.min_h) + i32::from(delta)).clamp(i32::from(MIN_WINDOW_H), 120);
                p.min_h = next as u16;
                if let Some(h) = p.preferred_h {
                    if h < p.min_h {
                        p.preferred_h = Some(p.min_h);
                    }
                }
            }
            2 => {
                if delta > 0 {
                    let base = p.preferred_w.unwrap_or(p.min_w);
                    p.preferred_w = Some((base.saturating_add(delta as u16)).clamp(p.min_w, 240));
                } else if let Some(cur) = p.preferred_w {
                    let d = (-delta) as u16;
                    let next = cur.saturating_sub(d);
                    p.preferred_w = if next < p.min_w { None } else { Some(next) };
                }
            }
            3 => {
                if delta > 0 {
                    let base = p.preferred_h.unwrap_or(p.min_h);
                    p.preferred_h = Some((base.saturating_add(delta as u16)).clamp(p.min_h, 120));
                } else if let Some(cur) = p.preferred_h {
                    let d = (-delta) as u16;
                    let next = cur.saturating_sub(d);
                    p.preferred_h = if next < p.min_h { None } else { Some(next) };
                }
            }
            _ => {}
        }
    });
    persist_settings();
}

fn desktop_settings_toggle_profile_mouse(target: &DesktopProfileTarget) {
    update_settings(|s| {
        let Some(p) = desktop_settings_profile_for_target_mut(&mut s.desktop_cli_profiles, target)
        else {
            return;
        };
        p.mouse_passthrough = !p.mouse_passthrough;
    });
    persist_settings();
}

fn desktop_settings_toggle_profile_fullscreen(target: &DesktopProfileTarget) {
    update_settings(|s| {
        let Some(p) = desktop_settings_profile_for_target_mut(&mut s.desktop_cli_profiles, target)
        else {
            return;
        };
        p.open_fullscreen = !p.open_fullscreen;
    });
    persist_settings();
}

fn desktop_settings_reset_profile(target: &DesktopProfileTarget) {
    let defaults = desktop_settings_profile_default_for_target(target);
    update_settings(|s| {
        let Some(p) = desktop_settings_profile_for_target_mut(&mut s.desktop_cli_profiles, target)
        else {
            return;
        };
        *p = defaults;
    });
    persist_settings();
}

fn desktop_settings_add_custom_profile(state: &mut DesktopSettingsState) {
    state.custom_profile_error = None;
    let Some(key) = normalize_profile_key(&state.custom_profile_input) else {
        state.custom_profile_error = Some("Enter a command name first".to_string());
        return;
    };
    if is_builtin_profile_key(&key) {
        state.custom_profile_error =
            Some("Use the built-in app profile for that command".to_string());
        return;
    }
    if key
        .chars()
        .any(|c| !(c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.' | '+')))
    {
        state.custom_profile_error = Some("Use letters, numbers, _, -, ., + only".to_string());
        return;
    }

    let key_for_insert = key.clone();
    let mut created = false;
    update_settings(|s| {
        let custom = &mut s.desktop_cli_profiles.custom;
        if !custom.contains_key(&key_for_insert) {
            custom.insert(key_for_insert.clone(), DesktopPtyProfileSettings::default());
            created = true;
        }
    });
    if created {
        persist_settings();
        state.panel = DesktopSettingsPanel::CustomProfileEdit(key);
        state.selected = 0;
        state.custom_profile_input.clear();
        state.custom_profile_error = None;
    } else {
        state.custom_profile_error = Some("Profile already exists".to_string());
    }
}

fn desktop_settings_delete_custom_profile(key: &str) {
    update_settings(|s| {
        s.desktop_cli_profiles.custom.remove(key);
    });
    persist_settings();
}

fn handle_desktop_settings_activate(
    state: &mut DesktopSettingsState,
    reverse: bool,
) -> DesktopSettingsAction {
    let action = match state.panel.clone() {
        DesktopSettingsPanel::Home => {
            let items = desktop_settings_home_items(state);
            let Some(item) = items.get(state.selected).copied() else {
                return DesktopSettingsAction::None;
            };
            match item {
                DesktopSettingsHomeItem::General => {
                    state.panel = DesktopSettingsPanel::General;
                    state.selected = 0;
                    DesktopSettingsAction::None
                }
                DesktopSettingsHomeItem::Appearance => {
                    state.panel = DesktopSettingsPanel::Appearance;
                    state.selected = 0;
                    DesktopSettingsAction::None
                }
                DesktopSettingsHomeItem::CliDisplay => {
                    state.panel = DesktopSettingsPanel::CliDisplay;
                    state.selected = 0;
                    DesktopSettingsAction::None
                }
                DesktopSettingsHomeItem::CliProfiles => {
                    state.panel = DesktopSettingsPanel::ProfileList;
                    state.selected = 0;
                    DesktopSettingsAction::None
                }
                DesktopSettingsHomeItem::EditMenus => DesktopSettingsAction::OpenEditMenus,
                DesktopSettingsHomeItem::UserManagement => {
                    DesktopSettingsAction::OpenUserManagement
                }
                DesktopSettingsHomeItem::About => {
                    state.panel = DesktopSettingsPanel::About;
                    state.selected = 0;
                    DesktopSettingsAction::None
                }
                DesktopSettingsHomeItem::Close => DesktopSettingsAction::CloseWindow,
            }
        }
        DesktopSettingsPanel::Appearance => match state.selected {
            0 => {
                desktop_settings_cycle_theme(!reverse);
                DesktopSettingsAction::None
            }
            1 => {
                state.panel = DesktopSettingsPanel::Wallpapers;
                state.selected = 0;
                DesktopSettingsAction::None
            }
            _ => {
                state.panel = DesktopSettingsPanel::Home;
                state.selected = 0;
                DesktopSettingsAction::None
            }
        },
        DesktopSettingsPanel::General => match state.selected {
            0 => {
                update_settings(|s| s.sound = !s.sound);
                persist_settings();
                DesktopSettingsAction::None
            }
            1 => {
                update_settings(|s| s.bootup = !s.bootup);
                persist_settings();
                DesktopSettingsAction::None
            }
            2 => {
                desktop_settings_apply_open_mode_toggle();
                DesktopSettingsAction::None
            }
            _ => {
                state.panel = DesktopSettingsPanel::Home;
                state.selected = 0;
                DesktopSettingsAction::None
            }
        },
        DesktopSettingsPanel::CliDisplay => match state.selected {
            0 => {
                update_settings(|s| s.cli_styled_render = !s.cli_styled_render);
                persist_settings();
                DesktopSettingsAction::None
            }
            1 => {
                desktop_settings_cycle_color(!reverse);
                DesktopSettingsAction::None
            }
            2 => {
                update_settings(|s| {
                    s.cli_acs_mode = match s.cli_acs_mode {
                        CliAcsMode::Ascii => CliAcsMode::Unicode,
                        CliAcsMode::Unicode => CliAcsMode::Ascii,
                    };
                });
                persist_settings();
                DesktopSettingsAction::None
            }
            _ => {
                state.panel = DesktopSettingsPanel::Home;
                state.selected = 0;
                DesktopSettingsAction::None
            }
        },
        DesktopSettingsPanel::Wallpapers => {
            let rows = desktop_wallpaper_rows();
            let action = rows
                .get(state.selected)
                .map(|row| row.action.clone())
                .unwrap_or(WallpaperRowAction::None);
            match action {
                WallpaperRowAction::None => {}
                WallpaperRowAction::Set(name) => desktop_settings_set_wallpaper(&name),
                WallpaperRowAction::OpenSizeMenu => {
                    state.panel = DesktopSettingsPanel::WallpaperSize;
                    state.selected = match get_settings().desktop_wallpaper_size_mode {
                        WallpaperSizeMode::DefaultSize => 0,
                        WallpaperSizeMode::FitToScreen => 1,
                        WallpaperSizeMode::Centered => 2,
                        WallpaperSizeMode::Tile => 3,
                        WallpaperSizeMode::Stretch => 4,
                    };
                }
                WallpaperRowAction::OpenChooseMenu => {
                    state.panel = DesktopSettingsPanel::WallpaperChoose;
                    state.selected = 0;
                }
                WallpaperRowAction::OpenDeleteMenu => {
                    state.panel = DesktopSettingsPanel::WallpaperDelete;
                    state.selected = 0;
                }
                WallpaperRowAction::AddCustom => {
                    state.panel = DesktopSettingsPanel::WallpaperAdd;
                    state.selected = 0;
                    state.wallpaper_error = None;
                    state.wallpaper_name_input.clear();
                    state.wallpaper_path_input.clear();
                    state.wallpaper_art_input.clear();
                }
                WallpaperRowAction::Back => {
                    state.panel = DesktopSettingsPanel::Appearance;
                    state.selected = 0;
                }
            }
            DesktopSettingsAction::None
        }
        DesktopSettingsPanel::WallpaperSize => {
            let modes = [
                WallpaperSizeMode::DefaultSize,
                WallpaperSizeMode::FitToScreen,
                WallpaperSizeMode::Centered,
                WallpaperSizeMode::Tile,
                WallpaperSizeMode::Stretch,
            ];
            if state.selected < modes.len() {
                desktop_settings_set_wallpaper_size_mode(modes[state.selected]);
                state.panel = DesktopSettingsPanel::Wallpapers;
                state.selected = 0;
            } else {
                state.panel = DesktopSettingsPanel::Wallpapers;
                state.selected = 0;
            }
            DesktopSettingsAction::None
        }
        DesktopSettingsPanel::WallpaperChoose => {
            let names = custom_wallpaper_names(&get_settings());
            if state.selected < names.len() {
                desktop_settings_set_wallpaper(&names[state.selected]);
                state.panel = DesktopSettingsPanel::Wallpapers;
                state.selected = 0;
            } else {
                state.panel = DesktopSettingsPanel::Wallpapers;
                state.selected = 0;
            }
            DesktopSettingsAction::None
        }
        DesktopSettingsPanel::WallpaperDelete => {
            let names = custom_wallpaper_names(&get_settings());
            if state.selected < names.len() {
                let to_delete = names[state.selected].clone();
                desktop_settings_delete_custom_wallpaper(&to_delete);
                state.selected = state.selected.saturating_sub(1);
            } else {
                state.panel = DesktopSettingsPanel::Wallpapers;
                state.selected = 0;
            }
            DesktopSettingsAction::None
        }
        DesktopSettingsPanel::WallpaperAdd => {
            match state.selected {
                2 => {
                    if state.wallpaper_name_input.trim().is_empty() {
                        state.wallpaper_error = Some("Enter wallpaper name first".to_string());
                    } else {
                        state.panel = DesktopSettingsPanel::WallpaperPaste;
                        state.wallpaper_error = None;
                    }
                }
                3 => {
                    state.wallpaper_art_input.clear();
                    state.wallpaper_error = None;
                }
                4 => desktop_settings_add_wallpaper(state),
                5 => {
                    state.panel = DesktopSettingsPanel::Wallpapers;
                    state.selected = 0;
                    state.wallpaper_error = None;
                }
                _ => {}
            }
            DesktopSettingsAction::None
        }
        DesktopSettingsPanel::WallpaperPaste => DesktopSettingsAction::None,
        DesktopSettingsPanel::ProfileList => {
            if state.selected < DESKTOP_SETTINGS_PROFILE_ITEMS.len() {
                let slot = DESKTOP_SETTINGS_PROFILE_ITEMS[state.selected].0;
                state.panel = DesktopSettingsPanel::ProfileEdit(slot);
                state.selected = 0;
            } else if state.selected == DESKTOP_SETTINGS_PROFILE_ITEMS.len() {
                state.panel = DesktopSettingsPanel::CustomProfileList;
                state.selected = 0;
            } else {
                state.panel = DesktopSettingsPanel::Home;
                state.selected = 0;
            }
            DesktopSettingsAction::None
        }
        DesktopSettingsPanel::ProfileEdit(slot) => {
            match state.selected {
                4 => desktop_settings_toggle_profile_mouse(&DesktopProfileTarget::Builtin(slot)),
                5 => desktop_settings_toggle_profile_fullscreen(&DesktopProfileTarget::Builtin(slot)),
                6 => desktop_settings_reset_profile(&DesktopProfileTarget::Builtin(slot)),
                7 => {
                    state.panel = DesktopSettingsPanel::ProfileList;
                    state.selected = 0;
                }
                _ => {}
            }
            DesktopSettingsAction::None
        }
        DesktopSettingsPanel::CustomProfileList => {
            let keys = desktop_settings_custom_profile_keys();
            if state.selected < keys.len() {
                state.panel = DesktopSettingsPanel::CustomProfileEdit(keys[state.selected].clone());
                state.selected = 0;
            } else if state.selected == keys.len() {
                state.panel = DesktopSettingsPanel::CustomProfileAdd;
                state.selected = 0;
                state.custom_profile_error = None;
            } else {
                state.panel = DesktopSettingsPanel::ProfileList;
                state.selected = DESKTOP_SETTINGS_PROFILE_ITEMS.len();
            }
            DesktopSettingsAction::None
        }
        DesktopSettingsPanel::CustomProfileEdit(key) => {
            match state.selected {
                5 => desktop_settings_toggle_profile_mouse(&DesktopProfileTarget::Custom(
                    key.clone(),
                )),
                6 => desktop_settings_toggle_profile_fullscreen(&DesktopProfileTarget::Custom(
                    key.clone(),
                )),
                7 => {
                    desktop_settings_delete_custom_profile(&key);
                    state.panel = DesktopSettingsPanel::CustomProfileList;
                    state.selected = 0;
                }
                8 => {
                    state.panel = DesktopSettingsPanel::CustomProfileList;
                    state.selected = 0;
                }
                _ => {}
            }
            DesktopSettingsAction::None
        }
        DesktopSettingsPanel::CustomProfileAdd => {
            match state.selected {
                1 => desktop_settings_add_custom_profile(state),
                2 => {
                    state.panel = DesktopSettingsPanel::CustomProfileList;
                    state.selected = 0;
                }
                _ => {}
            }
            DesktopSettingsAction::None
        }
        DesktopSettingsPanel::About => {
            if state.selected >= 3 {
                state.panel = DesktopSettingsPanel::Home;
                state.selected = 0;
            }
            DesktopSettingsAction::None
        }
    };
    if state.panel != DesktopSettingsPanel::Home {
        state.hovered = None;
    }
    desktop_settings_reset_selection(state);
    action
}

fn handle_desktop_settings_back(state: &mut DesktopSettingsState) -> DesktopSettingsAction {
    match state.panel.clone() {
        DesktopSettingsPanel::Home => DesktopSettingsAction::CloseWindow,
        DesktopSettingsPanel::Appearance => {
            state.panel = DesktopSettingsPanel::Home;
            state.selected = 0;
            state.hovered = None;
            DesktopSettingsAction::None
        }
        DesktopSettingsPanel::Wallpapers => {
            state.panel = DesktopSettingsPanel::Appearance;
            state.selected = 0;
            state.hovered = None;
            DesktopSettingsAction::None
        }
        DesktopSettingsPanel::WallpaperSize
        | DesktopSettingsPanel::WallpaperChoose
        | DesktopSettingsPanel::WallpaperDelete => {
            state.panel = DesktopSettingsPanel::Wallpapers;
            state.selected = 0;
            state.hovered = None;
            DesktopSettingsAction::None
        }
        DesktopSettingsPanel::WallpaperAdd => {
            state.panel = DesktopSettingsPanel::Wallpapers;
            state.selected = 0;
            state.wallpaper_error = None;
            state.hovered = None;
            DesktopSettingsAction::None
        }
        DesktopSettingsPanel::WallpaperPaste => {
            state.panel = DesktopSettingsPanel::WallpaperAdd;
            state.selected = 2;
            state.hovered = None;
            DesktopSettingsAction::None
        }
        DesktopSettingsPanel::ProfileEdit(_) => {
            state.panel = DesktopSettingsPanel::ProfileList;
            state.selected = 0;
            state.hovered = None;
            DesktopSettingsAction::None
        }
        DesktopSettingsPanel::CustomProfileList => {
            state.panel = DesktopSettingsPanel::ProfileList;
            state.selected = DESKTOP_SETTINGS_PROFILE_ITEMS.len();
            state.hovered = None;
            DesktopSettingsAction::None
        }
        DesktopSettingsPanel::CustomProfileEdit(_) => {
            state.panel = DesktopSettingsPanel::CustomProfileList;
            state.selected = 0;
            state.hovered = None;
            DesktopSettingsAction::None
        }
        DesktopSettingsPanel::CustomProfileAdd => {
            state.panel = DesktopSettingsPanel::CustomProfileList;
            state.selected = 0;
            state.custom_profile_error = None;
            state.hovered = None;
            DesktopSettingsAction::None
        }
        _ => {
            state.panel = DesktopSettingsPanel::Home;
            state.selected = 0;
            state.hovered = None;
            DesktopSettingsAction::None
        }
    }
}

fn handle_desktop_settings_key(
    state: &mut DesktopSettingsState,
    code: KeyCode,
    modifiers: KeyModifiers,
) -> DesktopSettingsAction {
    state.hovered = None;
    let step = if modifiers.contains(KeyModifiers::SHIFT) {
        5
    } else {
        1
    };

    if matches!(&state.panel, DesktopSettingsPanel::WallpaperPaste) {
        match code {
            KeyCode::Esc => {
                state.panel = DesktopSettingsPanel::WallpaperAdd;
                state.selected = 2;
            }
            KeyCode::Enter => state.wallpaper_art_input.push('\n'),
            KeyCode::Tab => state.wallpaper_art_input.push_str("    "),
            KeyCode::Backspace => {
                let _ = state.wallpaper_art_input.pop();
            }
            KeyCode::Char(c)
                if !modifiers.contains(KeyModifiers::CONTROL)
                    && !modifiers.contains(KeyModifiers::ALT)
                    && !c.is_control() =>
            {
                state.wallpaper_art_input.push(c);
            }
            _ => {}
        }
        return DesktopSettingsAction::None;
    }

    if matches!(&state.panel, DesktopSettingsPanel::CustomProfileAdd) {
        match code {
            KeyCode::Char(c)
                if !modifiers.contains(KeyModifiers::CONTROL)
                    && !modifiers.contains(KeyModifiers::ALT)
                    && state.selected == 0
                    && !c.is_control() =>
            {
                state.custom_profile_input.push(c);
                state.custom_profile_error = None;
                desktop_settings_reset_selection(state);
                return DesktopSettingsAction::None;
            }
            KeyCode::Backspace if state.selected == 0 => {
                state.custom_profile_input.pop();
                state.custom_profile_error = None;
                desktop_settings_reset_selection(state);
                return DesktopSettingsAction::None;
            }
            _ => {}
        }
    }

    if matches!(&state.panel, DesktopSettingsPanel::WallpaperAdd) {
        match code {
            KeyCode::Char(c)
                if !modifiers.contains(KeyModifiers::CONTROL)
                    && !modifiers.contains(KeyModifiers::ALT)
                    && (state.selected == 0 || state.selected == 1)
                    && !c.is_control() =>
            {
                if state.selected == 0 {
                    state.wallpaper_name_input.push(c);
                } else {
                    state.wallpaper_path_input.push(c);
                }
                state.wallpaper_error = None;
                desktop_settings_reset_selection(state);
                return DesktopSettingsAction::None;
            }
            KeyCode::Backspace if state.selected == 0 => {
                state.wallpaper_name_input.pop();
                state.wallpaper_error = None;
                desktop_settings_reset_selection(state);
                return DesktopSettingsAction::None;
            }
            KeyCode::Backspace if state.selected == 1 => {
                state.wallpaper_path_input.pop();
                state.wallpaper_error = None;
                desktop_settings_reset_selection(state);
                return DesktopSettingsAction::None;
            }
            _ => {}
        }
    }

    match code {
        KeyCode::Esc | KeyCode::Backspace => return handle_desktop_settings_back(state),
        KeyCode::Up => {
            if matches!(&state.panel, DesktopSettingsPanel::Home) {
                let cols = if desktop_settings_home_items(state).len() >= 6 {
                    4
                } else {
                    2
                };
                state.selected = state.selected.saturating_sub(cols);
            } else {
                state.selected = state.selected.saturating_sub(1);
            }
        }
        KeyCode::Down => {
            if matches!(&state.panel, DesktopSettingsPanel::Home) {
                let cols = if desktop_settings_home_items(state).len() >= 6 {
                    4
                } else {
                    2
                };
                state.selected = (state.selected + cols).min(desktop_settings_row_count(state) - 1);
            } else {
                state.selected = (state.selected + 1).min(desktop_settings_row_count(state) - 1);
            }
        }
        KeyCode::Left => match state.panel.clone() {
            DesktopSettingsPanel::Home => {
                state.selected = state.selected.saturating_sub(1);
            }
            DesktopSettingsPanel::Appearance if state.selected == 0 => {
                desktop_settings_cycle_theme(false)
            }
            DesktopSettingsPanel::General if state.selected == 2 => {
                desktop_settings_apply_open_mode_toggle()
            }
            DesktopSettingsPanel::CliDisplay if state.selected == 1 => {
                desktop_settings_cycle_color(false)
            }
            DesktopSettingsPanel::ProfileEdit(slot) if state.selected < 4 => {
                desktop_settings_adjust_profile_number(
                    &DesktopProfileTarget::Builtin(slot),
                    state.selected,
                    -(step as i16),
                );
            }
            DesktopSettingsPanel::CustomProfileEdit(key) if (1..=4).contains(&state.selected) => {
                desktop_settings_adjust_profile_number(
                    &DesktopProfileTarget::Custom(key),
                    state.selected - 1,
                    -(step as i16),
                );
            }
            _ => {}
        },
        KeyCode::Right => match state.panel.clone() {
            DesktopSettingsPanel::Home => {
                let max = desktop_settings_row_count(state).saturating_sub(1);
                state.selected = (state.selected + 1).min(max);
            }
            DesktopSettingsPanel::Appearance if state.selected == 0 => {
                desktop_settings_cycle_theme(true)
            }
            DesktopSettingsPanel::General if state.selected == 2 => {
                desktop_settings_apply_open_mode_toggle()
            }
            DesktopSettingsPanel::CliDisplay if state.selected == 1 => {
                desktop_settings_cycle_color(true)
            }
            DesktopSettingsPanel::ProfileEdit(slot) if state.selected < 4 => {
                desktop_settings_adjust_profile_number(
                    &DesktopProfileTarget::Builtin(slot),
                    state.selected,
                    step as i16,
                );
            }
            DesktopSettingsPanel::CustomProfileEdit(key) if (1..=4).contains(&state.selected) => {
                desktop_settings_adjust_profile_number(
                    &DesktopProfileTarget::Custom(key),
                    state.selected - 1,
                    step as i16,
                );
            }
            _ => {}
        },
        KeyCode::Char('+') | KeyCode::Char('=') => {
            if let DesktopSettingsPanel::ProfileEdit(slot) = state.panel.clone() {
                if state.selected < 4 {
                    desktop_settings_adjust_profile_number(
                        &DesktopProfileTarget::Builtin(slot),
                        state.selected,
                        step as i16,
                    );
                }
            } else if let DesktopSettingsPanel::CustomProfileEdit(key) = state.panel.clone() {
                if (1..=4).contains(&state.selected) {
                    desktop_settings_adjust_profile_number(
                        &DesktopProfileTarget::Custom(key),
                        state.selected - 1,
                        step as i16,
                    );
                }
            }
        }
        KeyCode::Char('-') => {
            if let DesktopSettingsPanel::ProfileEdit(slot) = state.panel.clone() {
                if state.selected < 4 {
                    desktop_settings_adjust_profile_number(
                        &DesktopProfileTarget::Builtin(slot),
                        state.selected,
                        -(step as i16),
                    );
                }
            } else if let DesktopSettingsPanel::CustomProfileEdit(key) = state.panel.clone() {
                if (1..=4).contains(&state.selected) {
                    desktop_settings_adjust_profile_number(
                        &DesktopProfileTarget::Custom(key),
                        state.selected - 1,
                        -(step as i16),
                    );
                }
            }
        }
        KeyCode::Enter | KeyCode::Char(' ') => {
            return handle_desktop_settings_activate(state, false);
        }
        KeyCode::Tab => {
            let max = desktop_settings_row_count(state).saturating_sub(1);
            state.selected = if max == 0 {
                0
            } else {
                (state.selected + 1) % (max + 1)
            };
        }
        KeyCode::Char('q') => return handle_desktop_settings_back(state),
        _ => {}
    }

    desktop_settings_reset_selection(state);
    DesktopSettingsAction::None
}

fn handle_desktop_settings_mouse(
    state: &mut DesktopSettingsState,
    area: Rect,
    mouse: crossterm::event::MouseEvent,
) -> DesktopSettingsAction {
    let uses_preview = matches!(
        state.panel,
        DesktopSettingsPanel::Wallpapers
            | DesktopSettingsPanel::WallpaperSize
            | DesktopSettingsPanel::WallpaperChoose
    ) && area.width >= 72
        && area.height >= 10;

    if matches!(mouse.kind, MouseEventKind::Moved) {
        let content = Rect {
            x: area.x + 1,
            y: area.y + 1,
            width: area.width.saturating_sub(2),
            height: area.height.saturating_sub(2),
        };
        if !point_in_rect(mouse.column, mouse.row, content) {
            state.hovered = None;
            return DesktopSettingsAction::None;
        }
        if matches!(&state.panel, DesktopSettingsPanel::Home) {
            let tiles =
                desktop_settings_home_tiles(content, desktop_settings_home_items(state).len());
            state.hovered = tiles.iter().enumerate().find_map(|(idx, tile)| {
                if point_in_rect(
                    mouse.column,
                    mouse.row,
                    desktop_settings_home_title_rect(*tile),
                ) {
                    Some(idx)
                } else {
                    None
                }
            });
            return DesktopSettingsAction::None;
        }

        let list_y = content.y + desktop_settings_list_offset(state);
        let list_w = content.width.saturating_sub(1);
        let list_x = content.x + 1;
        let max_list_x = if uses_preview {
            list_x + (((list_w as u32) * 48 / 100) as u16).max(18)
        } else {
            list_x + list_w
        };
        if mouse.column < list_x || mouse.column >= max_list_x {
            state.hovered = None;
            return DesktopSettingsAction::None;
        }
        if mouse.row < list_y {
            state.hovered = None;
            return DesktopSettingsAction::None;
        }
        let row = (mouse.row - list_y) as usize;
        if row >= desktop_settings_row_count(state) {
            state.hovered = None;
        } else {
            state.hovered = Some(row);
        }
        return DesktopSettingsAction::None;
    }

    if !matches!(mouse.kind, MouseEventKind::Down(MouseButton::Left)) {
        return DesktopSettingsAction::None;
    }

    let content = Rect {
        x: area.x + 1,
        y: area.y + 1,
        width: area.width.saturating_sub(2),
        height: area.height.saturating_sub(2),
    };
    if !point_in_rect(mouse.column, mouse.row, content) {
        state.hovered = None;
        return DesktopSettingsAction::None;
    }

    if matches!(&state.panel, DesktopSettingsPanel::Home) {
        for (idx, tile) in
            desktop_settings_home_tiles(content, desktop_settings_home_items(state).len())
                .into_iter()
                .enumerate()
        {
            if point_in_rect(mouse.column, mouse.row, tile) {
                state.selected = idx;
                return handle_desktop_settings_activate(state, false);
            }
        }
        return DesktopSettingsAction::None;
    }

    let list_y = content.y + desktop_settings_list_offset(state);
    let list_w = content.width.saturating_sub(1);
    let list_x = content.x + 1;
    let max_list_x = if uses_preview {
        list_x + (((list_w as u32) * 48 / 100) as u16).max(18)
    } else {
        list_x + list_w
    };
    if mouse.column < list_x || mouse.column >= max_list_x {
        return DesktopSettingsAction::None;
    }
    if mouse.row < list_y {
        return DesktopSettingsAction::None;
    }
    let row = (mouse.row - list_y) as usize;
    if row >= desktop_settings_row_count(state) {
        return DesktopSettingsAction::None;
    }
    state.selected = row;
    state.hovered = Some(row);
    handle_desktop_settings_activate(state, false)
}

fn focus_window(state: &mut DesktopState, id: u64) {
    if let Some(pos) = state.windows.iter().position(|w| w.id == id) {
        let win = state.windows.remove(pos);
        state.windows.push(win);
    }
}

fn focused_visible_window_idx(state: &DesktopState) -> Option<usize> {
    state.windows.iter().rposition(|w| !w.minimized)
}

fn focused_visible_window_id(state: &DesktopState) -> Option<u64> {
    focused_visible_window_idx(state).map(|idx| state.windows[idx].id)
}

fn activate_window_from_taskbar(state: &mut DesktopState, id: u64, desk: Rect) {
    if let Some(win) = state.windows.iter_mut().find(|w| w.id == id) {
        if win.minimized {
            win.minimized = false;
            if win.maximized {
                win.rect = winrect_from_rect(desk);
            }
        }
    }
    focus_window(state, id);
}

fn minimize_window_by_id(state: &mut DesktopState, id: u64) {
    if let Some(win) = state.windows.iter_mut().find(|w| w.id == id) {
        win.minimized = true;
    }
}

fn toggle_maximize_window_by_id(state: &mut DesktopState, id: u64, desk: Rect) {
    if let Some(win) = state.windows.iter_mut().find(|w| w.id == id) {
        if win.maximized {
            win.maximized = false;
            if let Some(prev) = win.restore_rect.take() {
                win.rect = prev;
            }
            let (min_w, min_h) = min_window_size_for_kind(&win.kind);
            clamp_window_with_min(&mut win.rect, desk, min_w, min_h);
        } else {
            win.restore_rect = Some(win.rect);
            win.maximized = true;
            win.minimized = false;
            win.rect = winrect_from_rect(desk);
        }
    }
}

fn clamp_all_windows(state: &mut DesktopState, desk: Rect) {
    for win in &mut state.windows {
        if win.maximized {
            win.rect = winrect_from_rect(desk);
        } else {
            let (min_w, min_h) = min_window_size_for_kind(&win.kind);
            clamp_window_with_min(&mut win.rect, desk, min_w, min_h);
        }
    }
}

fn clamp_window_with_min(rect: &mut WinRect, desk: Rect, min_w: u16, min_h: u16) {
    if desk.width < 8 || desk.height < 4 {
        return;
    }
    let max_w = desk.width.saturating_sub(1).max(1);
    let max_h = desk.height.saturating_sub(1).max(1);
    let min_w_eff = min_w.min(max_w).max(1);
    let min_h_eff = min_h.min(max_h).max(1);

    rect.w = rect.w.min(max_w).max(min_w_eff);
    rect.h = rect.h.min(max_h).max(min_h_eff);

    let min_x = desk.x as i32;
    let min_y = desk.y as i32;
    let max_x = desk
        .x
        .saturating_add(desk.width)
        .saturating_sub(rect.w)
        .saturating_sub(1) as i32;
    let max_y = desk
        .y
        .saturating_add(desk.height)
        .saturating_sub(rect.h)
        .saturating_sub(1) as i32;

    rect.x = rect.x.clamp(min_x, max_x.max(min_x));
    rect.y = rect.y.clamp(min_y, max_y.max(min_y));
}

fn min_window_size_for_kind(kind: &WindowKind) -> (u16, u16) {
    match kind {
        WindowKind::PtyApp(app) => (app.min_w, app.min_h),
        WindowKind::DesktopSettings(_) => (64, 18),
        WindowKind::FileManager(_) => (MIN_WINDOW_W, MIN_WINDOW_H),
        WindowKind::FileManagerSettings(_) => (46, 10),
    }
}

fn apply_corner_resize(
    rect: &mut WinRect,
    origin: WinRect,
    corner: ResizeCorner,
    mouse_x: u16,
    mouse_y: u16,
    desk: Rect,
    min_w: u16,
    min_h: u16,
) {
    let min_w = i32::from(min_w.max(1));
    let min_h = i32::from(min_h.max(1));

    let mut left = origin.x;
    let mut top = origin.y;
    let mut right = origin.x + i32::from(origin.w);
    let mut bottom = origin.y + i32::from(origin.h);

    let mx = i32::from(mouse_x);
    let my = i32::from(mouse_y);
    match corner {
        ResizeCorner::TopLeft => {
            left = mx;
            top = my;
        }
        ResizeCorner::TopRight => {
            right = mx + 1;
            top = my;
        }
        ResizeCorner::BottomLeft => {
            left = mx;
            bottom = my + 1;
        }
        ResizeCorner::BottomRight => {
            right = mx + 1;
            bottom = my + 1;
        }
    }

    let desk_left = i32::from(desk.x);
    let desk_top = i32::from(desk.y);
    let desk_right = i32::from(desk.x.saturating_add(desk.width).saturating_sub(1));
    let desk_bottom = i32::from(desk.y.saturating_add(desk.height).saturating_sub(1));

    match corner {
        ResizeCorner::TopLeft => {
            left = left.clamp(desk_left, right - min_w);
            top = top.clamp(desk_top, bottom - min_h);
        }
        ResizeCorner::TopRight => {
            right = right.clamp(left + min_w, desk_right);
            top = top.clamp(desk_top, bottom - min_h);
        }
        ResizeCorner::BottomLeft => {
            left = left.clamp(desk_left, right - min_w);
            bottom = bottom.clamp(top + min_h, desk_bottom);
        }
        ResizeCorner::BottomRight => {
            right = right.clamp(left + min_w, desk_right);
            bottom = bottom.clamp(top + min_h, desk_bottom);
        }
    }

    rect.x = left;
    rect.y = top;
    rect.w = (right - left).max(min_w) as u16;
    rect.h = (bottom - top).max(min_h) as u16;
    clamp_window_with_min(rect, desk, min_w as u16, min_h as u16);
}

fn winrect_from_rect(area: Rect) -> WinRect {
    WinRect {
        x: area.x as i32,
        y: area.y as i32,
        w: area.width.max(1),
        h: area.height.max(1),
    }
}

fn hit_my_computer_icon(x: u16, y: u16, desk: Rect) -> bool {
    let icon = my_computer_icon_rect(desk);
    point_in_rect(x, y, icon)
}

fn my_computer_icon_rect(desk: Rect) -> Rect {
    Rect {
        x: desk.x + 2,
        y: desk.y + 1,
        width: 12.min(desk.width.saturating_sub(2)),
        height: 2.min(desk.height.saturating_sub(1)),
    }
}

fn is_double_click(state: &mut DesktopState, target: ClickTarget) -> bool {
    let now = Instant::now();
    if let Some(prev) = state.last_click {
        if prev.target == target && now.duration_since(prev.at) <= DOUBLE_CLICK_WINDOW {
            state.last_click = None;
            return true;
        }
    }
    state.last_click = Some(LastClick { target, at: now });
    false
}

fn task_button_text(win: &DesktopWindow) -> String {
    let mut label = win.title.clone();
    if label.len() > 16 {
        label.truncate(16);
    }
    if win.minimized {
        format!("({label})")
    } else {
        format!("[{label}]")
    }
}

fn taskbar_layout(state: &DesktopState, task: Rect) -> TaskbarLayout {
    if task.height == 0 || task.width == 0 {
        return TaskbarLayout::empty();
    }

    let mut layout = TaskbarLayout::empty();
    let start_w = start_button_rect(task).width;
    let sep_w = TASK_START_SEPARATOR.len() as u16;
    let task_x_end = task.x.saturating_add(task.width);
    let base_x = task.x.saturating_add(start_w).saturating_add(sep_w);
    if base_x >= task_x_end {
        return layout;
    }

    let labels: Vec<(u64, String)> = state
        .windows
        .iter()
        .map(|w| (w.id, task_button_text(w)))
        .collect();
    if labels.is_empty() {
        return layout;
    }

    let content_width = task.width.saturating_sub(start_w.saturating_add(sep_w)) as usize;
    let total_needed: usize = labels.iter().map(|(_, t)| t.len() + 1).sum();
    let scroll = state.task_scroll.min(labels.len().saturating_sub(1));
    let paging = total_needed > content_width || scroll > 0;

    if !paging {
        let mut x = base_x;
        for (window_id, text) in labels {
            let width = text.len() as u16;
            if x + width >= task_x_end {
                break;
            }
            layout.buttons.push(TaskButton {
                window_id,
                rect: Rect {
                    x,
                    y: task.y,
                    width,
                    height: 1,
                },
            });
            x = x.saturating_add(width).saturating_add(1);
        }
        return layout;
    }

    let pager_w = TASK_PAGER_PREV.len() as u16;
    let prev_rect = Rect {
        x: base_x,
        y: task.y,
        width: pager_w,
        height: 1,
    };
    let next_rect = Rect {
        x: task_x_end.saturating_sub(pager_w),
        y: task.y,
        width: pager_w,
        height: 1,
    };
    if prev_rect.x.saturating_add(prev_rect.width) >= next_rect.x {
        return layout;
    }
    layout.prev_rect = Some(prev_rect);
    layout.next_rect = Some(next_rect);

    let mut x = prev_rect
        .x
        .saturating_add(prev_rect.width)
        .saturating_add(1);
    let max_x = next_rect.x.saturating_sub(1);
    let mut idx = scroll;
    while idx < labels.len() {
        let (window_id, text) = &labels[idx];
        let width = text.len() as u16;
        if width == 0 || x + width > max_x {
            break;
        }
        layout.buttons.push(TaskButton {
            window_id: *window_id,
            rect: Rect {
                x,
                y: task.y,
                width,
                height: 1,
            },
        });
        x = x.saturating_add(width).saturating_add(1);
        idx += 1;
    }

    layout.can_scroll_left = scroll > 0;
    layout.can_scroll_right = idx < labels.len();
    layout
}

fn top_status_area(size: Rect) -> Rect {
    Rect {
        x: size.x,
        y: size.y,
        width: size.width,
        height: if size.height > 0 { 1 } else { 0 },
    }
}

fn full_rect(width: u16, height: u16) -> Rect {
    Rect {
        x: 0,
        y: 0,
        width,
        height,
    }
}

fn taskbar_area(size: Rect) -> Rect {
    Rect {
        x: size.x,
        y: size.y + size.height.saturating_sub(1),
        width: size.width,
        height: if size.height > 1 { 1 } else { 0 },
    }
}

fn desktop_area(size: Rect) -> Rect {
    let top = if size.height > 0 { 1 } else { 0 };
    let bottom = if size.height > 1 { 1 } else { 0 };
    Rect {
        x: size.x,
        y: size.y + top,
        width: size.width,
        height: size.height.saturating_sub(top + bottom),
    }
}

fn start_button_rect(task: Rect) -> Rect {
    Rect {
        x: task.x,
        y: task.y,
        width: (TASK_START_BUTTON.len() as u16).min(task.width),
        height: task.height,
    }
}

fn title_close_button_rect(area: Rect) -> Rect {
    Rect {
        x: area.x
            + area
                .width
                .saturating_sub(TITLE_CLOSE_BUTTON.len() as u16 + 1),
        y: area.y,
        width: TITLE_CLOSE_BUTTON.len() as u16,
        height: 1,
    }
}

fn title_max_button_rect(area: Rect) -> Rect {
    let close = title_close_button_rect(area);
    Rect {
        x: close.x.saturating_sub(TITLE_MAX_BUTTON.len() as u16),
        y: area.y,
        width: TITLE_MAX_BUTTON.len() as u16,
        height: 1,
    }
}

fn title_min_button_rect(area: Rect) -> Rect {
    let max = title_max_button_rect(area);
    Rect {
        x: max.x.saturating_sub(TITLE_MIN_BUTTON.len() as u16),
        y: area.y,
        width: TITLE_MIN_BUTTON.len() as u16,
        height: 1,
    }
}

fn hit_resize_corner(area: Rect, x: u16, y: u16) -> Option<ResizeCorner> {
    if area.width < 4 || area.height < 4 {
        return None;
    }
    let left = area.x;
    let right = area.x.saturating_add(area.width).saturating_sub(1);
    let top = area.y;
    let bottom = area.y.saturating_add(area.height).saturating_sub(1);

    if x == left && y == top {
        Some(ResizeCorner::TopLeft)
    } else if x == right && y == top {
        Some(ResizeCorner::TopRight)
    } else if x == left && y == bottom {
        Some(ResizeCorner::BottomLeft)
    } else if x == right && y == bottom {
        Some(ResizeCorner::BottomRight)
    } else {
        None
    }
}

fn start_root_rect(task: Rect) -> Rect {
    let h = (START_ROOT_VIS_ROWS.len() as u16) + 2;
    let width = 34u16.min(task.width.max(12));
    Rect {
        x: task.x,
        y: task.y.saturating_sub(h),
        width,
        height: h,
    }
}

fn start_submenu_rect(root: Rect, size: Rect, submenu: StartSubmenu) -> Rect {
    let h = (submenu_visual_rows(submenu).len() as u16) + 2;
    let longest = match submenu {
        StartSubmenu::Programs => submenu_items_programs()
            .iter()
            .map(|(label, _)| label.chars().count())
            .max()
            .unwrap_or(8),
        StartSubmenu::System => submenu_items_system()
            .iter()
            .map(|(label, _)| label.chars().count())
            .max()
            .unwrap_or(8),
    };
    let width = ((longest + 5).min(44)) as u16;
    let mut y = root.y;
    if y + h >= size.height {
        y = size.height.saturating_sub(h);
    }
    Rect {
        x: root.x + root.width.saturating_sub(1),
        y,
        width,
        height: h,
    }
}

fn start_leaf_rect(sub: Rect, size: Rect, start: &StartState, leaf: StartProgramsLeaf) -> Rect {
    let items = leaf_items(start, leaf);
    let h = ((items.len() as u16) + 2).max(3);
    let longest = items
        .iter()
        .map(|item| item.label.chars().count())
        .max()
        .unwrap_or(8);
    let x = sub.x + sub.width.saturating_sub(1);
    let max_w = size.width.saturating_sub(x).max(12);
    let width = ((longest + 4).min(52)) as u16;
    let mut y = sub.y;
    if y + h >= size.height {
        y = size.height.saturating_sub(h);
    }
    Rect {
        x,
        y,
        width: width.min(max_w),
        height: h,
    }
}

fn point_in_rect(x: u16, y: u16, r: Rect) -> bool {
    x >= r.x && x < r.x.saturating_add(r.width) && y >= r.y && y < r.y.saturating_add(r.height)
}

fn write_text(buf: &mut [char], start: usize, text: &str) {
    for (i, ch) in text.chars().enumerate() {
        let idx = start + i;
        if idx >= buf.len() {
            break;
        }
        buf[idx] = ch;
    }
}

fn write_text_in_area(buf: &mut [char], area: Rect, x: u16, text: &str) {
    if x < area.x {
        return;
    }
    let start = (x - area.x) as usize;
    write_text(buf, start, text);
}

fn format_menu_row(width: usize, label: &str, right_arrow: Option<char>) -> String {
    if width == 0 {
        return String::new();
    }
    let mut chars = vec![' '; width];
    write_text(&mut chars, 0, &format!(" {}", label));
    if let Some(arrow) = right_arrow {
        if width >= 2 {
            chars[width - 2] = arrow;
        }
    }
    chars.into_iter().collect()
}

fn read_entries(path: &Path, settings: &DesktopFileManagerSettings) -> Vec<FileEntry> {
    let mut entries = Vec::new();
    if let Some(parent) = path.parent() {
        entries.push(FileEntry {
            name: "..".to_string(),
            path: parent.to_path_buf(),
            is_dir: true,
        });
    }

    if let Ok(read) = std::fs::read_dir(path) {
        for entry in read.flatten() {
            let p = entry.path();
            let is_dir = p.is_dir();
            let name = entry.file_name().to_string_lossy().to_string();
            if !settings.show_hidden_files && name.starts_with('.') {
                continue;
            }
            entries.push(FileEntry {
                name,
                path: p,
                is_dir,
            });
        }
    }

    entries.sort_by(|a, b| {
        if settings.directories_first {
            match (a.is_dir, b.is_dir) {
                (true, false) => return Ordering::Less,
                (false, true) => return Ordering::Greater,
                _ => {}
            }
        }
        match settings.sort_mode {
            FileManagerSortMode::Name => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
            FileManagerSortMode::Type => {
                let a_ext = Path::new(&a.name)
                    .extension()
                    .and_then(|s| s.to_str())
                    .unwrap_or("")
                    .to_ascii_lowercase();
                let b_ext = Path::new(&b.name)
                    .extension()
                    .and_then(|s| s.to_str())
                    .unwrap_or("")
                    .to_ascii_lowercase();
                a_ext
                    .cmp(&b_ext)
                    .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
            }
        }
    });
    entries
}

fn battery_display() -> String {
    if let Ok(mut cache) = BATTERY_CACHE.lock() {
        if let Some((value, at)) = &*cache {
            if at.elapsed() <= Duration::from_secs(30) {
                return value.clone();
            }
        }
        let value = read_battery_now();
        *cache = Some((value.clone(), Instant::now()));
        return value;
    }
    read_battery_now()
}

fn read_battery_now() -> String {
    #[cfg(target_os = "macos")]
    {
        let out = std::process::Command::new("pmset")
            .args(["-g", "batt"])
            .output();
        if let Ok(out) = out {
            let text = String::from_utf8_lossy(&out.stdout);
            for line in text.lines() {
                if let Some(pos) = line.find('%') {
                    let before = &line[..pos];
                    let num_start = before
                        .rfind(|c: char| !c.is_ascii_digit())
                        .map(|i| i + 1)
                        .unwrap_or(0);
                    if let Ok(pct) = before[num_start..].trim().parse::<u8>() {
                        let status = if line.contains("charging") && !line.contains("discharging") {
                            "↑"
                        } else if line.contains("discharging") {
                            "↓"
                        } else {
                            ""
                        };
                        return format!("{pct}%{status}");
                    }
                }
            }
        }
        return "--%".to_string();
    }

    #[cfg(target_os = "linux")]
    {
        if let Ok(rd) = std::fs::read_dir("/sys/class/power_supply") {
            for entry in rd.flatten() {
                let kind = std::fs::read_to_string(entry.path().join("type")).unwrap_or_default();
                if kind.trim() == "Battery" {
                    let cap =
                        std::fs::read_to_string(entry.path().join("capacity")).unwrap_or_default();
                    if let Ok(pct) = cap.trim().parse::<u8>() {
                        let status = std::fs::read_to_string(entry.path().join("status"))
                            .unwrap_or_default();
                        let suffix = match status.trim() {
                            "Charging" => "↑",
                            "Discharging" => "↓",
                            _ => "",
                        };
                        return format!("{pct}%{suffix}");
                    }
                }
            }
        }
        return "--%".to_string();
    }

    #[cfg(target_os = "windows")]
    {
        let out = std::process::Command::new("WMIC")
            .args(["Path", "Win32_Battery", "Get", "EstimatedChargeRemaining"])
            .output();
        if let Ok(out) = out {
            let text = String::from_utf8_lossy(&out.stdout);
            for line in text.lines().skip(1) {
                if let Ok(pct) = line.trim().parse::<u8>() {
                    return format!("{pct}%");
                }
            }
        }
        return "--%".to_string();
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        "--%".to_string()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WindowHit {
    Title,
    Minimize,
    Maximize,
    Close,
    Resize(ResizeCorner),
    Content,
}
