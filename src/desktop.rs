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
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use crate::apps::edit_menus_menu;
use crate::auth::{is_admin, user_management_menu};
use crate::config::{
    get_settings, load_apps, load_categories, load_games, load_networks, persist_settings,
    update_settings, CliAcsMode, CliColorMode, DesktopCliProfiles, DesktopPtyProfileSettings,
    OpenMode, THEMES,
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
struct FileManagerState {
    cwd: PathBuf,
    entries: Vec<FileEntry>,
    selected: usize,
    scroll: usize,
}

impl FileManagerState {
    fn new() -> Self {
        let cwd = dirs::home_dir()
            .or_else(|| std::env::current_dir().ok())
            .unwrap_or_else(|| PathBuf::from("."));
        let entries = read_entries(&cwd);
        Self {
            cwd,
            entries,
            selected: 0,
            scroll: 0,
        }
    }

    fn refresh(&mut self) {
        self.entries = read_entries(&self.cwd);
        if self.selected >= self.entries.len() && !self.entries.is_empty() {
            self.selected = self.entries.len() - 1;
        }
        if self.entries.is_empty() {
            self.selected = 0;
            self.scroll = 0;
        }
    }

    fn open_selected(&mut self) {
        let Some(entry) = self.entries.get(self.selected) else {
            return;
        };
        if entry.is_dir {
            self.cwd = entry.path.clone();
            self.selected = 0;
            self.scroll = 0;
            self.refresh();
        }
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
            self.cwd = parent.to_path_buf();
            self.selected = 0;
            self.scroll = 0;
            self.refresh();
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
    General,
    Startup,
    CliDisplay,
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
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DesktopSettingsHomeItem {
    General,
    Startup,
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

struct PtyWindowState {
    session: crate::pty::PtySession,
    min_w: u16,
    min_h: u16,
    mouse_passthrough: bool,
}

impl Drop for PtyWindowState {
    fn drop(&mut self) {
        self.session.terminate();
    }
}

enum WindowKind {
    FileManager(FileManagerState),
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
const START_SYSTEM: [(&str, StartLaunch); 3] = [
    ("Program Installer", StartLaunch::ProgramInstaller),
    ("Terminal", StartLaunch::Terminal),
    ("Settings", StartLaunch::Settings),
];
const START_PROGRAMS_VIS_ROWS: [Option<usize>; 4] = [Some(0), Some(1), Some(2), Some(3)];
const START_SYSTEM_VIS_ROWS: [Option<usize>; 4] = [Some(0), None, Some(1), Some(2)];

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
static BATTERY_CACHE: Mutex<Option<(String, Instant)>> = Mutex::new(None);

#[derive(Debug, Clone, Copy)]
struct PtyCompatibilityProfile {
    min_w: u16,
    min_h: u16,
    preferred_w: Option<u16>,
    preferred_h: Option<u16>,
    mouse_passthrough: bool,
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
        let mut close_focused = false;
        let mut settings_action = DesktopSettingsAction::None;
        match &mut state.windows[last_idx].kind {
            WindowKind::PtyApp(app) => {
                app.session.send_key(code, modifiers);
                return Ok(None);
            }
            WindowKind::DesktopSettings(settings) => {
                settings_action = handle_desktop_settings_key(settings, code, modifiers);
            }
            WindowKind::FileManager(fm) => match code {
                KeyCode::Esc => {
                    close_focused = true;
                }
                KeyCode::Up => fm.up(),
                KeyCode::Down => fm.down(),
                KeyCode::Enter => fm.open_selected(),
                KeyCode::Backspace => fm.parent(),
                _ => {}
            },
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
    let desk = desktop_area(size);
    let task = taskbar_area(size);

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
    let id = state.next_id;
    state.next_id += 1;
    state.windows.push(DesktopWindow {
        id,
        title,
        rect,
        restore_rect: None,
        minimized: false,
        maximized: false,
        kind: WindowKind::PtyApp(PtyWindowState {
            session,
            min_w: profile.min_w,
            min_h: profile.min_h,
            mouse_passthrough: profile.mouse_passthrough,
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

        draw_top_status(f, top);
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

        draw_cursor(f, state.cursor_x, state.cursor_y, size);
    })?;
    Ok(())
}

fn draw_top_status(f: &mut ratatui::Frame, area: Rect) {
    if area.height == 0 {
        return;
    }
    let now = Local::now().format("%a %Y-%m-%d %I:%M%p").to_string();
    let batt = battery_display();
    let width = area.width as usize;
    let mut row = vec![' '; width];

    write_text(&mut row, 0, &format!(" {} ", now));
    if width >= batt.len() + 2 {
        let start = width.saturating_sub(batt.len() + 2);
        write_text(&mut row, start, &format!(" {} ", batt));
    }

    let line: String = row.into_iter().collect();
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(line, sel_style()))),
        area,
    );
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
    let inner = Rect {
        x: area.x + 1,
        y: area.y + 1,
        width: area.width.saturating_sub(2),
        height: area.height.saturating_sub(2),
    };
    if inner.height == 0 || inner.width == 0 {
        return;
    }

    let mut lines = Vec::new();
    let mut path = fm.cwd.display().to_string();
    if path.chars().count() > inner.width as usize {
        let keep = inner.width as usize - 3;
        path = format!(
            "...{}",
            path.chars()
                .rev()
                .take(keep)
                .collect::<String>()
                .chars()
                .rev()
                .collect::<String>()
        );
    }
    lines.push(Line::from(Span::styled(
        format!("Path: {}", path),
        dim_style(),
    )));
    lines.push(Line::from(Span::styled(
        "-".repeat(inner.width as usize),
        dim_style(),
    )));

    let visible_rows = inner.height.saturating_sub(2) as usize;
    let start = fm.scroll.min(fm.entries.len());
    let end = (start + visible_rows).min(fm.entries.len());
    for (idx, entry) in fm.entries[start..end].iter().enumerate() {
        let absolute_idx = start + idx;
        let icon = if entry.is_dir { "[D]" } else { "[F]" };
        let mut line = format!("{} {}", icon, entry.name);
        if line.chars().count() > inner.width as usize {
            line = line.chars().take(inner.width as usize).collect();
        }
        let style = if absolute_idx == fm.selected && focused {
            sel_style()
        } else {
            normal_style()
        };
        lines.push(Line::from(Span::styled(line, style)));
    }

    f.render_widget(Paragraph::new(lines), inner);
}

fn desktop_settings_home_items(state: &DesktopSettingsState) -> Vec<DesktopSettingsHomeItem> {
    let mut items = vec![
        DesktopSettingsHomeItem::General,
        DesktopSettingsHomeItem::Startup,
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
        DesktopSettingsHomeItem::General => "General",
        DesktopSettingsHomeItem::Startup => "Startup",
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
        DesktopSettingsHomeItem::General => "[*]",
        DesktopSettingsHomeItem::Startup => "[^]",
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

fn desktop_settings_list_offset(settings: &DesktopSettingsState) -> u16 {
    if matches!(&settings.panel, DesktopSettingsPanel::CustomProfileAdd)
        && settings.custom_profile_error.is_some()
    {
        3
    } else {
        2
    }
}

fn desktop_settings_rows(settings: &DesktopSettingsState) -> Vec<String> {
    let s = get_settings();
    match &settings.panel {
        DesktopSettingsPanel::General => vec![
            format!("Theme: {} [cycle]", s.theme),
            format!("Sound: {} [toggle]", if s.sound { "ON" } else { "OFF" }),
            format!("Bootup: {} [toggle]", if s.bootup { "ON" } else { "OFF" }),
            "Back".to_string(),
        ],
        DesktopSettingsPanel::Startup => vec![
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
        DesktopSettingsPanel::General => "General",
        DesktopSettingsPanel::Startup => "Startup",
        DesktopSettingsPanel::CliDisplay => "CLI Display",
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

    f.render_widget(
        Paragraph::new(lines),
        Rect {
            x: content.x + 1,
            y: content.y + desktop_settings_list_offset(settings),
            width: content.width.saturating_sub(1),
            height: content
                .height
                .saturating_sub(desktop_settings_list_offset(settings)),
        },
    );
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
                let area = win.rect.to_rect();
                let content = Rect {
                    x: area.x + 1,
                    y: area.y + 1,
                    width: area.width.saturating_sub(2),
                    height: area.height.saturating_sub(2),
                };
                if !matches!(mouse.kind, MouseEventKind::Down(MouseButton::Left)) {
                    return Ok(());
                }
                if !point_in_rect(mouse.column, mouse.row, content) || content.height < 3 {
                    return Ok(());
                }
                if mouse.row <= content.y + 1 {
                    return Ok(());
                }
                let row = (mouse.row - content.y - 2) as usize;
                let visible_rows = content.height.saturating_sub(2) as usize;
                if row >= visible_rows {
                    return Ok(());
                }
                let idx = fm.scroll + row;
                if idx >= fm.entries.len() {
                    return Ok(());
                }
                fm.selected = idx;
                Some(ClickTarget::FileEntry {
                    window_id: win.id,
                    row: idx,
                })
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
        return Ok(());
    }

    let Some(clicked_target) = clicked_target else {
        return Ok(());
    };
    if is_double_click(state, clicked_target) {
        if let Some(win) = state.windows.last_mut() {
            if let WindowKind::FileManager(fm) = &mut win.kind {
                fm.open_selected();
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
        DesktopSettingsPanel::General => 4,
        DesktopSettingsPanel::Startup => 2,
        DesktopSettingsPanel::CliDisplay => 4,
        DesktopSettingsPanel::ProfileList => DESKTOP_SETTINGS_PROFILE_ITEMS.len() + 2,
        DesktopSettingsPanel::ProfileEdit(_) => 7,
        DesktopSettingsPanel::CustomProfileList => desktop_settings_custom_profile_keys().len() + 2,
        DesktopSettingsPanel::CustomProfileEdit(_) => 8,
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
                DesktopSettingsHomeItem::Startup => {
                    state.panel = DesktopSettingsPanel::Startup;
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
        DesktopSettingsPanel::General => match state.selected {
            0 => {
                desktop_settings_cycle_theme(!reverse);
                DesktopSettingsAction::None
            }
            1 => {
                update_settings(|s| s.sound = !s.sound);
                persist_settings();
                DesktopSettingsAction::None
            }
            2 => {
                update_settings(|s| s.bootup = !s.bootup);
                persist_settings();
                DesktopSettingsAction::None
            }
            _ => {
                state.panel = DesktopSettingsPanel::Home;
                state.selected = 0;
                DesktopSettingsAction::None
            }
        },
        DesktopSettingsPanel::Startup => match state.selected {
            0 => {
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
                5 => desktop_settings_reset_profile(&DesktopProfileTarget::Builtin(slot)),
                6 => {
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
                6 => {
                    desktop_settings_delete_custom_profile(&key);
                    state.panel = DesktopSettingsPanel::CustomProfileList;
                    state.selected = 0;
                }
                7 => {
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
            DesktopSettingsPanel::General if state.selected == 0 => {
                desktop_settings_cycle_theme(false)
            }
            DesktopSettingsPanel::Startup if state.selected == 0 => {
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
            DesktopSettingsPanel::General if state.selected == 0 => {
                desktop_settings_cycle_theme(true)
            }
            DesktopSettingsPanel::Startup if state.selected == 0 => {
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

fn read_entries(path: &Path) -> Vec<FileEntry> {
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
            entries.push(FileEntry {
                name,
                path: p,
                is_dir,
            });
        }
    }

    entries.sort_by(|a, b| match (a.is_dir, b.is_dir) {
        (true, false) => Ordering::Less,
        (false, true) => Ordering::Greater,
        _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
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
