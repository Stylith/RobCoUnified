use anyhow::Result;
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

use crate::config::{load_apps, load_categories, load_games, load_networks};
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
    LaunchCommand(Vec<String>),
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

struct PtyWindowState {
    session: crate::pty::PtySession,
}

impl Drop for PtyWindowState {
    fn drop(&mut self) {
        self.session.terminate();
    }
}

enum WindowKind {
    FileManager(FileManagerState),
    PtyApp(PtyWindowState),
}

struct DesktopWindow {
    id: u64,
    title: String,
    rect: WinRect,
    kind: WindowKind,
}

#[derive(Debug, Clone, Copy)]
struct DragState {
    window_id: u64,
    dx: i32,
    dy: i32,
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
                    label: key,
                    action: StartAction::LaunchCommand(cmd),
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
                    label: key,
                    action: StartAction::LaunchCommand(cmd),
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
static BATTERY_CACHE: Mutex<Option<(String, Instant)>> = Mutex::new(None);

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

    if let Some(last_idx) = state.windows.len().checked_sub(1) {
        let focused_id = state.windows[last_idx].id;
        let mut close_focused = false;
        match &mut state.windows[last_idx].kind {
            WindowKind::PtyApp(app) => {
                app.session.send_key(code, modifiers);
                return Ok(None);
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
                win.rect.x = i32::from(mouse.column) - drag.dx;
                win.rect.y = i32::from(mouse.row) - drag.dy;
                clamp_window(&mut win.rect, desk);
            }
        }
        return Ok(None);
    }

    if let MouseEventKind::Up(MouseButton::Left) = mouse.kind {
        state.dragging = None;
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
        for btn in task_buttons(state, task) {
            if point_in_rect(mouse.column, mouse.row, btn.rect) {
                focus_window(state, btn.window_id);
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
                WindowHit::Title => {
                    if let Some(win) = state.windows.iter().find(|w| w.id == window_id) {
                        state.dragging = Some(DragState {
                            window_id,
                            dx: i32::from(mouse.column) - win.rect.x,
                            dy: i32::from(mouse.row) - win.rect.y,
                        });
                    }
                }
                WindowHit::Content => {
                    handle_window_content_click(state, mouse.column, mouse.row);
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
                StartLaunch::Settings => open_pty_window_named(
                    terminal,
                    state,
                    &build_desktop_tool_command(current_user, "settings")?,
                    Some("Settings"),
                ),
            };
            if let Err(err) = launch_result {
                flash_message(terminal, &format!("Launch failed: {err}"), 1200)?;
            }
            Ok(None)
        }
        StartAction::LaunchCommand(cmd) => {
            if let Err(err) = open_pty_window(terminal, state, &cmd) {
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

fn open_pty_window(terminal: &mut Term, state: &mut DesktopState, cmd: &[String]) -> Result<()> {
    open_pty_window_named(terminal, state, cmd, None)
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

    let offset = ((state.windows.len() % 6) as i32) * 2;
    let base_w = desk.width.saturating_sub(10).clamp(44, 120);
    let base_h = desk.height.saturating_sub(5).clamp(12, 36);
    let mut rect = WinRect {
        x: desk.x as i32 + 4 + offset,
        y: desk.y as i32 + 2 + offset,
        w: base_w,
        h: base_h,
    };
    clamp_window(&mut rect, desk);

    let cols = rect.w.saturating_sub(2).max(1);
    let rows = rect.h.saturating_sub(2).max(1);
    let program = &cmd[0];
    let args: Vec<&str> = cmd[1..].iter().map(String::as_str).collect();
    let session = crate::pty::PtySession::spawn(
        program,
        &args,
        cols,
        rows,
        &crate::pty::PtyLaunchOptions::default(),
    )?;

    let title = title_override
        .map(str::to_string)
        .unwrap_or_else(|| command_title(program));
    let id = state.next_id;
    state.next_id += 1;
    state.windows.push(DesktopWindow {
        id,
        title,
        rect,
        kind: WindowKind::PtyApp(PtyWindowState { session }),
    });
    Ok(())
}

fn command_title(program: &str) -> String {
    Path::new(program)
        .file_name()
        .and_then(|s| s.to_str())
        .map(str::to_string)
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| program.to_string())
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

        let focused = state.windows.last().map(|w| w.id);
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
    let mut row = vec![' '; width];
    write_text(&mut row, 0, "[Start]");

    let mut x = 8usize;
    for win in &state.windows {
        let mut label = win.title.clone();
        if label.len() > 16 {
            label.truncate(16);
        }
        let text = format!("[{}]", label);
        if x + text.len() >= width {
            break;
        }
        write_text(&mut row, x, &text);
        x += text.len() + 1;
    }

    let line: String = row.into_iter().collect();
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(line, sel_style()))),
        area,
    );
}

fn draw_window(f: &mut ratatui::Frame, win: &DesktopWindow, focused: bool) {
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
    if chars.len() >= 3 {
        let close_x = chars.len() - 3;
        write_text(&mut chars, close_x, "[X]");
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
        let rect = win.rect;
        if !rect.contains(x, y) {
            continue;
        }
        let area = rect.to_rect();
        let close_rect = Rect {
            x: area.x + area.width.saturating_sub(4),
            y: area.y,
            width: 3,
            height: 1,
        };
        if point_in_rect(x, y, close_rect) {
            return Some((win.id, WindowHit::Close));
        }
        if y == area.y {
            return Some((win.id, WindowHit::Title));
        }
        return Some((win.id, WindowHit::Content));
    }
    None
}

fn handle_window_content_click(state: &mut DesktopState, x: u16, y: u16) {
    let Some(idx_last) = state.windows.len().checked_sub(1) else {
        return;
    };
    let clicked_target = {
        let win = &mut state.windows[idx_last];
        let WindowKind::FileManager(fm) = &mut win.kind else {
            return;
        };
        let area = win.rect.to_rect();
        let content = Rect {
            x: area.x + 1,
            y: area.y + 1,
            width: area.width.saturating_sub(2),
            height: area.height.saturating_sub(2),
        };
        if !point_in_rect(x, y, content) || content.height < 3 {
            return;
        }
        if y <= content.y + 1 {
            return;
        }
        let row = (y - content.y - 2) as usize;
        let visible_rows = content.height.saturating_sub(2) as usize;
        if row >= visible_rows {
            return;
        }
        let idx = fm.scroll + row;
        if idx >= fm.entries.len() {
            return;
        }
        fm.selected = idx;
        ClickTarget::FileEntry {
            window_id: win.id,
            row: idx,
        }
    };

    if is_double_click(state, clicked_target) {
        if let Some(win) = state.windows.last_mut() {
            if let WindowKind::FileManager(fm) = &mut win.kind {
                fm.open_selected();
            }
        }
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
        kind: WindowKind::FileManager(FileManagerState::new()),
    });
}

fn focus_window(state: &mut DesktopState, id: u64) {
    if let Some(pos) = state.windows.iter().position(|w| w.id == id) {
        let win = state.windows.remove(pos);
        state.windows.push(win);
    }
}

fn clamp_all_windows(state: &mut DesktopState, desk: Rect) {
    for win in &mut state.windows {
        clamp_window(&mut win.rect, desk);
    }
}

fn clamp_window(rect: &mut WinRect, desk: Rect) {
    if desk.width < 8 || desk.height < 4 {
        return;
    }
    rect.w = rect.w.min(desk.width.saturating_sub(1)).max(20);
    rect.h = rect.h.min(desk.height.saturating_sub(1)).max(8);

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

fn task_buttons(state: &DesktopState, task: Rect) -> Vec<TaskButton> {
    let mut out = Vec::new();
    let mut x = task.x + 8;
    for win in &state.windows {
        let mut label = win.title.clone();
        if label.len() > 16 {
            label.truncate(16);
        }
        let width = (label.len() + 2 + 2) as u16;
        if x + width >= task.x + task.width {
            break;
        }
        out.push(TaskButton {
            window_id: win.id,
            rect: Rect {
                x,
                y: task.y,
                width,
                height: 1,
            },
        });
        x += width + 1;
    }
    out
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
        width: 7.min(task.width),
        height: task.height,
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
    Close,
    Content,
}
