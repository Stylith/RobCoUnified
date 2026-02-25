use anyhow::Result;
use chrono::Local;
use crossterm::event::{
    self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind, MouseButton,
    MouseEventKind,
};
use crossterm::execute;
use ratatui::{
    layout::Rect,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};
use std::cmp::Ordering;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use crate::apps;
use crate::documents;
use crate::installer;
use crate::settings;
use crate::shell_terminal;
use crate::ui::{dim_style, normal_style, sel_style, session_switch_scope, title_style, Term};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DesktopExit {
    ReturnToTerminal,
    Logout,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StartLaunch {
    Applications,
    Documents,
    Network,
    Games,
    ProgramInstaller,
    Terminal,
    Settings,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StartAction {
    None,
    Launch(StartLaunch),
    ReturnToTerminal,
    Logout,
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

#[derive(Debug, Clone)]
enum WindowKind {
    FileManager(FileManagerState),
}

#[derive(Debug, Clone)]
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

#[derive(Debug, Clone, Copy)]
struct StartState {
    open: bool,
    selected_root: usize,
    selected_program: usize,
    programs_open: bool,
}

impl Default for StartState {
    fn default() -> Self {
        Self {
            open: false,
            selected_root: 0,
            selected_program: 0,
            programs_open: true,
        }
    }
}

#[derive(Debug, Default)]
struct DesktopState {
    windows: Vec<DesktopWindow>,
    next_id: u64,
    cursor_x: u16,
    cursor_y: u16,
    dragging: Option<DragState>,
    last_click: Option<LastClick>,
    start: StartState,
}

const START_ROOT_ITEMS: [&str; 3] = ["Programs >", "Return To Terminal Mode", "Logout"];
const START_PROGRAMS: [(&str, StartLaunch); 7] = [
    ("Applications", StartLaunch::Applications),
    ("Documents", StartLaunch::Documents),
    ("Network", StartLaunch::Network),
    ("Games", StartLaunch::Games),
    ("Program Installer", StartLaunch::ProgramInstaller),
    ("Terminal", StartLaunch::Terminal),
    ("Settings", StartLaunch::Settings),
];

const DOUBLE_CLICK_WINDOW: Duration = Duration::from_millis(450);
static BATTERY_CACHE: Mutex<Option<(String, Instant)>> = Mutex::new(None);

pub fn desktop_mode(terminal: &mut Term, current_user: &str) -> Result<DesktopExit> {
    let _switch_scope = session_switch_scope(false);
    execute!(terminal.backend_mut(), EnableMouseCapture)?;
    let result = run_desktop_loop(terminal, current_user);
    let _ = execute!(terminal.backend_mut(), DisableMouseCapture);
    result
}

fn run_desktop_loop(terminal: &mut Term, current_user: &str) -> Result<DesktopExit> {
    let mut state = DesktopState {
        next_id: 1,
        ..DesktopState::default()
    };
    let mut last_tick = Instant::now();

    loop {
        draw_desktop(terminal, &mut state)?;

        let timeout = Duration::from_millis(16);
        if event::poll(timeout)? {
            match event::read()? {
                Event::Key(key) => {
                    if key.kind != KeyEventKind::Press && key.kind != KeyEventKind::Repeat {
                        continue;
                    }
                    if let Some(exit) = handle_key(terminal, current_user, &mut state, key.code)? {
                        return Ok(exit);
                    }
                }
                Event::Mouse(mouse) => {
                    if let Some(exit) = handle_mouse(terminal, current_user, &mut state, mouse)? {
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
) -> Result<Option<DesktopExit>> {
    if state.start.open {
        match code {
            KeyCode::Esc => {
                state.start.open = false;
                state.start.programs_open = false;
            }
            KeyCode::Up => {
                state.start.selected_root = state.start.selected_root.saturating_sub(1);
            }
            KeyCode::Down => {
                state.start.selected_root =
                    (state.start.selected_root + 1).min(START_ROOT_ITEMS.len() - 1);
            }
            KeyCode::Right => {
                if state.start.selected_root == 0 {
                    state.start.programs_open = true;
                }
            }
            KeyCode::Left => {
                state.start.programs_open = false;
            }
            KeyCode::Tab => {
                if state.start.programs_open {
                    state.start.selected_program =
                        (state.start.selected_program + 1) % START_PROGRAMS.len();
                } else {
                    state.start.selected_root =
                        (state.start.selected_root + 1) % START_ROOT_ITEMS.len();
                }
            }
            KeyCode::Enter | KeyCode::Char(' ') => {
                let action = if state.start.selected_root == 0 {
                    state.start.programs_open = true;
                    StartAction::Launch(START_PROGRAMS[state.start.selected_program].1)
                } else if state.start.selected_root == 1 {
                    StartAction::ReturnToTerminal
                } else {
                    StartAction::Logout
                };
                return run_start_action(terminal, current_user, state, action);
            }
            _ => {}
        }
        return Ok(None);
    }

    match code {
        KeyCode::Esc => {
            if !state.windows.is_empty() {
                state.windows.pop();
            }
        }
        KeyCode::F(10) => {
            state.start.open = true;
            state.start.programs_open = true;
            state.start.selected_root = 0;
        }
        KeyCode::Char('m') | KeyCode::Char('M') => {
            open_file_manager_window(state);
        }
        KeyCode::Up | KeyCode::Down | KeyCode::Enter | KeyCode::Backspace => {
            if let Some(win) = state.windows.last_mut() {
                let WindowKind::FileManager(fm) = &mut win.kind;
                match code {
                    KeyCode::Up => fm.up(),
                    KeyCode::Down => fm.down(),
                    KeyCode::Enter => fm.open_selected(),
                    KeyCode::Backspace => fm.parent(),
                    _ => {}
                }
            }
        }
        _ => {}
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
            state.start.open = !state.start.open;
            state.start.programs_open = true;
            state.start.selected_root = 0;
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
        if let Some(action) = hit_start_menu(mouse.column, mouse.row, size, state) {
            if matches!(mouse.kind, MouseEventKind::Down(MouseButton::Left)) {
                return run_start_action(terminal, current_user, state, action);
            }
            return Ok(None);
        }

        if matches!(mouse.kind, MouseEventKind::Down(MouseButton::Left)) {
            state.start.open = false;
            state.start.programs_open = false;
        }
    }

    if matches!(mouse.kind, MouseEventKind::Down(MouseButton::Left)) {
        if let Some((window_id, hit)) = hit_window(state, mouse.column, mouse.row) {
            focus_window(state, window_id);
            match hit {
                WindowHit::Close => {
                    state.windows.retain(|w| w.id != window_id);
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
    state.start.programs_open = false;

    match action {
        StartAction::None => Ok(None),
        StartAction::ReturnToTerminal => Ok(Some(DesktopExit::ReturnToTerminal)),
        StartAction::Logout => Ok(Some(DesktopExit::Logout)),
        StartAction::Launch(which) => {
            execute!(terminal.backend_mut(), DisableMouseCapture)?;
            let run_result = match which {
                StartLaunch::Applications => apps::apps_menu(terminal),
                StartLaunch::Documents => documents::documents_menu(terminal),
                StartLaunch::Network => apps::network_menu(terminal),
                StartLaunch::Games => apps::games_menu(terminal),
                StartLaunch::ProgramInstaller => installer::appstore_menu(terminal),
                StartLaunch::Terminal => shell_terminal::embedded_terminal(terminal),
                StartLaunch::Settings => settings::settings_menu(terminal, current_user),
            };
            let recapture = execute!(terminal.backend_mut(), EnableMouseCapture);
            run_result?;
            recapture?;
            Ok(None)
        }
    }
}

fn draw_desktop(terminal: &mut Term, state: &mut DesktopState) -> Result<()> {
    let ts = terminal.size()?;
    let size = full_rect(ts.width, ts.height);
    clamp_all_windows(state, desktop_area(size));

    terminal.draw(|f| {
        let size = f.area();
        let top = top_status_area(size);
        let desktop = desktop_area(size);
        let task = taskbar_area(size);

        f.render_widget(Paragraph::new("").style(normal_style()), size);

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
    let center = "Desktop Mode";
    let width = area.width as usize;
    let mut row = vec![' '; width];

    write_text(&mut row, 0, &format!(" {} ", now));
    if width >= batt.len() + 2 {
        let start = width.saturating_sub(batt.len() + 2);
        write_text(&mut row, start, &format!(" {} ", batt));
    }
    let center_start = width.saturating_sub(center.len()) / 2;
    write_text(&mut row, center_start, center);

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

fn draw_start_menu(f: &mut ratatui::Frame, size: Rect, state: &DesktopState) {
    let task = taskbar_area(size);
    let root = start_root_rect(task);
    f.render_widget(
        Block::default().borders(Borders::ALL).style(title_style()),
        root,
    );

    let mut root_lines = Vec::new();
    for (i, label) in START_ROOT_ITEMS.iter().enumerate() {
        let style = if i == state.start.selected_root {
            sel_style()
        } else {
            normal_style()
        };
        root_lines.push(Line::from(Span::styled(format!(" {}", label), style)));
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

    if state.start.programs_open {
        let sub = start_programs_rect(root, size);
        f.render_widget(
            Block::default().borders(Borders::ALL).style(title_style()),
            sub,
        );
        let mut sub_lines = Vec::new();
        for (i, (label, _)) in START_PROGRAMS.iter().enumerate() {
            let style = if i == state.start.selected_program {
                sel_style()
            } else {
                normal_style()
            };
            sub_lines.push(Line::from(Span::styled(format!(" {}", label), style)));
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

fn hit_start_menu(x: u16, y: u16, size: Rect, state: &mut DesktopState) -> Option<StartAction> {
    let root = start_root_rect(taskbar_area(size));
    if point_in_rect(x, y, root) {
        let row = y.saturating_sub(root.y + 1) as usize;
        if row < START_ROOT_ITEMS.len() {
            state.start.selected_root = row;
            state.start.programs_open = row == 0;
            return Some(match row {
                0 => StartAction::None,
                1 => StartAction::ReturnToTerminal,
                _ => StartAction::Logout,
            });
        }
        return Some(StartAction::None);
    }

    if state.start.programs_open {
        let sub = start_programs_rect(root, size);
        if point_in_rect(x, y, sub) {
            let row = y.saturating_sub(sub.y + 1) as usize;
            if row < START_PROGRAMS.len() {
                state.start.selected_program = row;
                return Some(StartAction::Launch(START_PROGRAMS[row].1));
            }
            return Some(StartAction::None);
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
        let WindowKind::FileManager(fm) = &mut win.kind;
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
            let WindowKind::FileManager(fm) = &mut win.kind;
            fm.open_selected();
        }
    }
}

fn open_file_manager_window(state: &mut DesktopState) {
    if let Some(id) = state.windows.iter().find_map(|w| match w.kind {
        WindowKind::FileManager(_) => Some(w.id),
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
    let h = (START_ROOT_ITEMS.len() as u16) + 2;
    Rect {
        x: task.x,
        y: task.y.saturating_sub(h),
        width: 28,
        height: h,
    }
}

fn start_programs_rect(root: Rect, size: Rect) -> Rect {
    let h = (START_PROGRAMS.len() as u16) + 2;
    let mut y = root.y;
    if y + h >= size.height {
        y = size.height.saturating_sub(h);
    }
    Rect {
        x: root.x + root.width.saturating_sub(1),
        y,
        width: 30,
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
