use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame, Terminal,
};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use crate::config::{current_theme_color, HEADER_LINES};
use crate::status::render_status_bar;

pub type Term = Terminal<ratatui::backend::CrosstermBackend<std::io::Stdout>>;

// ── Session switch interception ───────────────────────────────────────────────
// Alt+[1-9] (Option on macOS) switches/creates sessions.
// Ctrl+[1-9] also accepted where terminals support it.
// In PTY mode, Ctrl+Q then [1-9] switches directly, and Ctrl+Q then N
// switches to "next", creating it if needed.

const ALT_ESC_WINDOW: Duration = Duration::from_millis(250);
static ALT_ESC_PREFIX: Mutex<Option<Instant>> = Mutex::new(None);
const SESSION_LEADER_WINDOW: Duration = Duration::from_millis(1200);
static SESSION_LEADER_PREFIX: Mutex<Option<Instant>> = Mutex::new(None);
static SESSION_SWITCH_ENABLED: AtomicBool = AtomicBool::new(true);
const OPTION_LIKE_MODS: KeyModifiers = KeyModifiers::ALT.union(KeyModifiers::META);

pub struct SessionSwitchScope {
    previous: bool,
}

impl Drop for SessionSwitchScope {
    fn drop(&mut self) {
        SESSION_SWITCH_ENABLED.store(self.previous, Ordering::Relaxed);
    }
}

pub fn session_switch_scope(enabled: bool) -> SessionSwitchScope {
    let previous = SESSION_SWITCH_ENABLED.swap(enabled, Ordering::Relaxed);
    SessionSwitchScope { previous }
}

fn plain_or_shift(mods: KeyModifiers) -> bool {
    mods.is_empty() || mods == KeyModifiers::SHIFT
}

fn leader_follow_mods(mods: KeyModifiers) -> bool {
    plain_or_shift(mods)
        || mods == KeyModifiers::CONTROL
        || mods == (KeyModifiers::CONTROL | KeyModifiers::SHIFT)
}

fn option_digit_idx(c: char) -> Option<usize> {
    Some(match c {
        // Common macOS Option outputs across US/intl layouts.
        '\u{00A1}' | '\u{00B9}' => 0,              // 1
        '\u{2122}' | '\u{20AC}' | '\u{00B2}' => 1, // 2
        '\u{00A3}' | '\u{00B3}' => 2,              // 3
        '\u{00A2}' | '\u{00A4}' => 3,              // 4
        '\u{221E}' | '\u{00BD}' => 4,              // 5
        '\u{00A7}' | '\u{00BE}' | '\u{00AC}' => 5, // 6
        '\u{00B6}' => 6,                           // 7
        '\u{2022}' => 7,                           // 8
        '\u{00AA}' => 8,                           // 9
        _ => return None,
    })
}

fn alt_punct_digit_idx(c: char) -> Option<usize> {
    Some(match c {
        '!' => 0,
        '@' => 1,
        '#' => 2,
        '$' => 3,
        '%' => 4,
        '^' => 5,
        '&' => 6,
        '*' => 7,
        '(' => 8,
        _ => return None,
    })
}

fn remember_alt_escape_prefix() {
    if let Ok(mut slot) = ALT_ESC_PREFIX.lock() {
        *slot = Some(Instant::now());
    }
}

fn take_recent_alt_escape_prefix() -> bool {
    let Ok(mut slot) = ALT_ESC_PREFIX.lock() else {
        return false;
    };
    let active = slot
        .as_ref()
        .map(|t| t.elapsed() <= ALT_ESC_WINDOW)
        .unwrap_or(false);
    *slot = None;
    active
}

fn clear_alt_escape_prefix() {
    if let Ok(mut slot) = ALT_ESC_PREFIX.lock() {
        *slot = None;
    }
}

fn remember_session_leader_prefix() {
    if let Ok(mut slot) = SESSION_LEADER_PREFIX.lock() {
        *slot = Some(Instant::now());
    }
}

fn take_recent_session_leader_prefix() -> bool {
    let Ok(mut slot) = SESSION_LEADER_PREFIX.lock() else {
        return false;
    };
    let active = slot
        .as_ref()
        .map(|t| t.elapsed() <= SESSION_LEADER_WINDOW)
        .unwrap_or(false);
    *slot = None;
    active
}

fn clear_session_leader_prefix() {
    if let Ok(mut slot) = SESSION_LEADER_PREFIX.lock() {
        *slot = None;
    }
}

fn session_idx_from_digit_like_char(c: char) -> Option<usize> {
    if ('1'..='9').contains(&c) {
        return Some((c as usize) - ('1' as usize));
    }
    option_digit_idx(c).or_else(|| alt_punct_digit_idx(c))
}

fn session_idx_from_leader_chord(code: KeyCode, mods: KeyModifiers) -> Option<usize> {
    if leader_follow_mods(mods) {
        if let KeyCode::Char(c) = code {
            if take_recent_session_leader_prefix() {
                return session_idx_from_digit_like_char(c);
            }
        }
        return None;
    }

    clear_session_leader_prefix();
    None
}

fn leader_requests_next_session(code: KeyCode, mods: KeyModifiers) -> bool {
    if leader_follow_mods(mods) {
        let wants_next =
            matches!(code, KeyCode::Tab) || matches!(code, KeyCode::Char('n' | 'N' | '0' | '+'));
        return wants_next && take_recent_session_leader_prefix();
    }

    clear_session_leader_prefix();
    false
}

fn session_idx_from_key(
    code: KeyCode,
    mods: KeyModifiers,
    allow_esc_prefix: bool,
) -> Option<usize> {
    if let KeyCode::F(n @ 1..=9) = code {
        return Some((n as usize) - 1);
    }

    if mods.contains(KeyModifiers::CONTROL) {
        return match code {
            KeyCode::Char(c @ '1'..='9') => Some((c as usize) - ('1' as usize)),
            KeyCode::Char(' ') => Some(1), // Ctrl+2 fallback (NUL / Ctrl+Space)
            KeyCode::Esc => Some(2),       // Ctrl+3 fallback on enhanced terminals
            KeyCode::Backspace => Some(7), // Ctrl+8 fallback on enhanced terminals
            _ => None,
        };
    }

    if mods.intersects(OPTION_LIKE_MODS) {
        return match code {
            KeyCode::Char(c @ '1'..='9') => Some((c as usize) - ('1' as usize)),
            KeyCode::Char(c) => option_digit_idx(c).or_else(|| alt_punct_digit_idx(c)),
            _ => None,
        };
    }

    if allow_esc_prefix {
        if matches!(code, KeyCode::Esc) && mods.is_empty() {
            remember_alt_escape_prefix();
            return None;
        }

        if mods.is_empty() {
            if let KeyCode::Char(c) = code {
                // Some terminals emit Option-modified Unicode directly with no ALT bit.
                if let Some(idx) = option_digit_idx(c) {
                    clear_alt_escape_prefix();
                    return Some(idx);
                }

                if take_recent_alt_escape_prefix() {
                    if let Some(idx) = ('1'..='9')
                        .position(|d| d == c)
                        .or_else(|| alt_punct_digit_idx(c))
                    {
                        return Some(idx);
                    }
                }
            } else {
                clear_alt_escape_prefix();
            }
        }
    }

    None
}

fn is_leader_trigger(code: KeyCode, mods: KeyModifiers) -> bool {
    mods.contains(KeyModifiers::CONTROL) && matches!(code, KeyCode::Char('q' | 'Q'))
}

fn check_session_switch_with_mode(
    code: KeyCode,
    mods: KeyModifiers,
    allow_esc_prefix: bool,
    allow_leader: bool,
) -> bool {
    if !SESSION_SWITCH_ENABLED.load(Ordering::Relaxed) {
        return false;
    }

    if allow_leader && is_leader_trigger(code, mods) {
        remember_session_leader_prefix();
        return true;
    }

    if allow_leader {
        if leader_requests_next_session(code, mods) {
            let count = crate::session::session_count();
            if count < crate::session::MAX_SESSIONS {
                crate::session::request_switch(count);
                return true;
            }
        }

        if let Some(idx) = session_idx_from_leader_chord(code, mods) {
            let count = crate::session::session_count();
            if idx < count || (idx == count && count < crate::session::MAX_SESSIONS) {
                crate::session::request_switch(idx);
                return true;
            }
        }
    }

    if let Some(idx) = session_idx_from_key(code, mods, allow_esc_prefix) {
        let count = crate::session::session_count();
        if idx < count || (idx == count && count < crate::session::MAX_SESSIONS) {
            crate::session::request_switch(idx);
            return true;
        }
    }
    false
}

fn check_session_switch(code: KeyCode, mods: KeyModifiers) -> bool {
    check_session_switch_with_mode(code, mods, false, true)
}

pub fn check_session_switch_pub(code: KeyCode, mods: KeyModifiers) -> bool {
    check_session_switch(code, mods)
}

pub fn check_session_switch_pty_pub(code: KeyCode, mods: KeyModifiers) -> bool {
    check_session_switch_with_mode(code, mods, true, true)
}

// ── Padding ───────────────────────────────────────────────────────────────────

const H_PAD: u16 = 3;

pub fn pad_horizontal(area: Rect) -> Rect {
    let pad = H_PAD.min(area.width / 2);
    Rect {
        x: area.x + pad,
        y: area.y,
        width: area.width.saturating_sub(pad * 2),
        height: area.height,
    }
}

// ── Style helpers ─────────────────────────────────────────────────────────────

pub fn normal_style() -> Style {
    Style::default().fg(current_theme_color())
}
pub fn sel_style() -> Style {
    Style::default()
        .fg(ratatui::style::Color::Black)
        .bg(current_theme_color())
        .add_modifier(Modifier::BOLD)
}
pub fn title_style() -> Style {
    Style::default()
        .fg(current_theme_color())
        .add_modifier(Modifier::BOLD)
}
pub fn dim_style() -> Style {
    Style::default()
        .fg(current_theme_color())
        .add_modifier(Modifier::DIM)
}

// ── Header / separator ────────────────────────────────────────────────────────

pub fn render_header(f: &mut Frame, area: Rect) {
    let inner = pad_horizontal(area);
    let lines: Vec<Line> = HEADER_LINES
        .iter()
        .map(|l| Line::from(Span::styled(*l, title_style())))
        .collect();
    f.render_widget(Paragraph::new(lines).alignment(Alignment::Center), inner);
}

pub fn render_separator(f: &mut Frame, area: Rect) {
    let inner = pad_horizontal(area);
    let sep = "=".repeat(inner.width as usize);
    f.render_widget(
        Paragraph::new(sep)
            .alignment(Alignment::Center)
            .style(dim_style()),
        inner,
    );
}

// ── Menu ──────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MenuResult {
    Selected(String),
    Back,
}

pub fn run_menu(
    terminal: &mut Term,
    title: &str,
    choices: &[&str],
    subtitle: Option<&str>,
) -> Result<MenuResult> {
    let selectable: Vec<&str> = choices.iter().copied().filter(|c| *c != "---").collect();
    let mut idx = 0usize;

    loop {
        terminal.draw(|f| {
            let size = f.area();
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3),
                    Constraint::Length(1),
                    Constraint::Length(1),
                    Constraint::Length(1),
                    if subtitle.is_some() {
                        Constraint::Length(2)
                    } else {
                        Constraint::Length(0)
                    },
                    Constraint::Min(1),
                    Constraint::Length(1),
                ])
                .split(size);

            render_header(f, chunks[0]);
            render_separator(f, chunks[1]);
            f.render_widget(
                Paragraph::new(title)
                    .alignment(Alignment::Center)
                    .style(title_style()),
                pad_horizontal(chunks[2]),
            );
            render_separator(f, chunks[3]);

            if let Some(sub) = subtitle {
                f.render_widget(
                    Paragraph::new(Span::styled(
                        sub,
                        Style::default()
                            .fg(current_theme_color())
                            .add_modifier(Modifier::UNDERLINED),
                    ))
                    .alignment(Alignment::Left),
                    pad_horizontal(chunks[4]),
                );
            }

            let lines: Vec<Line> = choices
                .iter()
                .map(|&choice| {
                    if choice == "---" {
                        return Line::from(Span::styled("", dim_style()));
                    }
                    let selected = selectable.get(idx).copied() == Some(choice);
                    if selected {
                        Line::from(Span::styled(format!("  > {choice}"), sel_style()))
                    } else {
                        Line::from(Span::styled(format!("    {choice}"), normal_style()))
                    }
                })
                .collect();
            f.render_widget(Paragraph::new(lines), pad_horizontal(chunks[5]));
            render_status_bar(f, chunks[6]);
        })?;

        if event::poll(Duration::from_millis(25))? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }
                if check_session_switch(key.code, key.modifiers) {
                    if crate::session::has_switch_request() {
                        crate::sound::play_navigate();
                        return Ok(MenuResult::Back);
                    }
                    continue;
                }
                match key.code {
                    KeyCode::Up | KeyCode::Char('k') => {
                        if !selectable.is_empty() {
                            let prev = idx;
                            idx = idx.saturating_sub(1);
                            if idx != prev {
                                crate::sound::play_navigate_repeat();
                            }
                        }
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        if !selectable.is_empty() {
                            let prev = idx;
                            idx = (idx + 1).min(selectable.len() - 1);
                            if idx != prev {
                                crate::sound::play_navigate_repeat();
                            }
                        }
                    }
                    KeyCode::Enter | KeyCode::Char(' ') => {
                        crate::sound::play_navigate();
                        if let Some(&sel) = selectable.get(idx) {
                            return Ok(MenuResult::Selected(sel.to_string()));
                        }
                    }
                    KeyCode::Char('q') | KeyCode::Esc | KeyCode::Tab => {
                        crate::sound::play_navigate();
                        return Ok(MenuResult::Back);
                    }
                    _ => {}
                }
            }
        }
    }
}

pub fn run_menu_compact(
    terminal: &mut Term,
    title: &str,
    choices: &[&str],
    subtitle: Option<&str>,
) -> Result<MenuResult> {
    let selectable_rows: Vec<usize> = choices
        .iter()
        .enumerate()
        .filter_map(|(idx, choice)| (*choice != "---").then_some(idx))
        .collect();
    let mut selected_idx = 0usize;
    let mut scroll = 0usize;

    loop {
        let selected_row = selectable_rows
            .get(selected_idx)
            .copied()
            .unwrap_or_default();
        terminal.draw(|f| {
            let size = f.area();
            let max_choice_w = choices
                .iter()
                .map(|c| c.chars().count())
                .max()
                .unwrap_or(0)
                .max(title.chars().count())
                .max(subtitle.map(|s| s.chars().count()).unwrap_or(0));
            let box_w = ((max_choice_w + 8) as u16).clamp(34, 72);
            let box_h = ((choices.len() + 5) as u16).clamp(8, 18);
            let w = box_w.min(size.width.saturating_sub(4)).max(18);
            let h = box_h.min(size.height.saturating_sub(2)).max(6);
            let area = Rect {
                x: size.x + size.width.saturating_sub(w) / 2,
                y: size.y + size.height.saturating_sub(h) / 2,
                width: w,
                height: h,
            };

            f.render_widget(Clear, area);
            f.render_widget(
                Block::default().borders(Borders::ALL).style(title_style()),
                area,
            );

            let inner = Rect {
                x: area.x + 1,
                y: area.y + 1,
                width: area.width.saturating_sub(2),
                height: area.height.saturating_sub(2),
            };
            if inner.width == 0 || inner.height == 0 {
                return;
            }

            let title_text: String = title.chars().take(inner.width as usize).collect();
            f.render_widget(
                Paragraph::new(Line::from(Span::styled(title_text, title_style())))
                    .alignment(Alignment::Center),
                Rect {
                    x: inner.x,
                    y: inner.y,
                    width: inner.width,
                    height: 1,
                },
            );

            let mut list_y = inner.y + 1;
            if let Some(sub) = subtitle {
                let sub_text: String = sub.chars().take(inner.width as usize).collect();
                f.render_widget(
                    Paragraph::new(Line::from(Span::styled(sub_text, dim_style())))
                        .alignment(Alignment::Left),
                    Rect {
                        x: inner.x,
                        y: list_y,
                        width: inner.width,
                        height: 1,
                    },
                );
                list_y = list_y.saturating_add(1);
            }

            let list_h = inner.y + inner.height - list_y;
            let visible = list_h.max(1) as usize;
            let max_scroll = choices.len().saturating_sub(visible);
            let mut start = scroll.min(max_scroll);
            if selected_row < start {
                start = selected_row;
            } else if selected_row >= start.saturating_add(visible) {
                start = selected_row.saturating_sub(visible.saturating_sub(1));
            }
            scroll = start.min(max_scroll);

            let lines: Vec<Line> = choices
                .iter()
                .enumerate()
                .skip(scroll)
                .take(visible)
                .map(|(idx, choice)| {
                    if *choice == "---" {
                        let sep = "─".repeat(inner.width.saturating_sub(2) as usize);
                        return Line::from(Span::styled(sep, dim_style()));
                    }
                    if idx == selected_row {
                        Line::from(Span::styled(format!("> {choice}"), sel_style()))
                    } else {
                        Line::from(Span::styled(format!("  {choice}"), normal_style()))
                    }
                })
                .collect();

            f.render_widget(
                Paragraph::new(lines),
                Rect {
                    x: inner.x,
                    y: list_y,
                    width: inner.width,
                    height: list_h,
                },
            );
        })?;

        if event::poll(Duration::from_millis(25))? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }
                if check_session_switch(key.code, key.modifiers) {
                    if crate::session::has_switch_request() {
                        crate::sound::play_navigate();
                        return Ok(MenuResult::Back);
                    }
                    continue;
                }
                match key.code {
                    KeyCode::Up | KeyCode::Char('k') => {
                        if !selectable_rows.is_empty() {
                            let prev = selected_idx;
                            selected_idx = selected_idx.saturating_sub(1);
                            if selected_idx != prev {
                                crate::sound::play_navigate_repeat();
                            }
                        }
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        if !selectable_rows.is_empty() {
                            let prev = selected_idx;
                            selected_idx = (selected_idx + 1).min(selectable_rows.len() - 1);
                            if selected_idx != prev {
                                crate::sound::play_navigate_repeat();
                            }
                        }
                    }
                    KeyCode::Enter | KeyCode::Char(' ') => {
                        crate::sound::play_navigate();
                        if let Some(row_idx) = selectable_rows.get(selected_idx).copied() {
                            if let Some(sel) = choices.get(row_idx) {
                                return Ok(MenuResult::Selected((*sel).to_string()));
                            }
                        }
                    }
                    KeyCode::Char('q') | KeyCode::Esc | KeyCode::Tab => {
                        crate::sound::play_navigate();
                        return Ok(MenuResult::Back);
                    }
                    _ => {}
                }
            }
        }
    }
}

// ── Text input ────────────────────────────────────────────────────────────────

pub fn input_prompt(terminal: &mut Term, prompt: &str) -> Result<Option<String>> {
    let mut buf = String::new();
    loop {
        terminal.draw(|f| {
            let size = f.area();
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3),
                    Constraint::Length(1),
                    Constraint::Min(1),
                    Constraint::Length(1),
                ])
                .split(size);
            render_header(f, chunks[0]);
            render_separator(f, chunks[1]);
            f.render_widget(
                Paragraph::new(format!("{prompt}\n\n  > {buf}█")).style(normal_style()),
                pad_horizontal(chunks[2]),
            );
            render_status_bar(f, chunks[3]);
        })?;

        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }
                if check_session_switch(key.code, key.modifiers) {
                    if crate::session::has_switch_request() {
                        return Ok(None);
                    }
                    continue;
                }
                match key.code {
                    KeyCode::Enter => {
                        crate::sound::play_navigate();
                        return Ok(Some(buf.trim().to_string()));
                    }
                    KeyCode::Esc => {
                        crate::sound::play_navigate();
                        return Ok(None);
                    }
                    KeyCode::Backspace => {
                        if !buf.is_empty() {
                            buf.pop();
                            crate::sound::play_keypress();
                        }
                    }
                    KeyCode::Char(c) if (c as u32) >= 32 => {
                        buf.push(c);
                        crate::sound::play_keypress();
                    }
                    _ => {}
                }
            }
        }
    }
}

// ── Password input ────────────────────────────────────────────────────────────

pub fn password_prompt(terminal: &mut Term, prompt: &str) -> Result<Option<String>> {
    let mut buf = String::new();
    loop {
        terminal.draw(|f| {
            let size = f.area();
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3),
                    Constraint::Length(1),
                    Constraint::Min(1),
                    Constraint::Length(1),
                ])
                .split(size);
            render_header(f, chunks[0]);
            render_separator(f, chunks[1]);
            let masked = "*".repeat(buf.len());
            f.render_widget(
                Paragraph::new(format!("{prompt}\n\n  > {masked}█")).style(normal_style()),
                pad_horizontal(chunks[2]),
            );
            render_status_bar(f, chunks[3]);
        })?;

        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }
                if check_session_switch(key.code, key.modifiers) {
                    if crate::session::has_switch_request() {
                        return Ok(None);
                    }
                    continue;
                }
                match key.code {
                    KeyCode::Enter => {
                        crate::sound::play_navigate();
                        return Ok(Some(buf));
                    }
                    KeyCode::Esc => {
                        crate::sound::play_navigate();
                        return Ok(None);
                    }
                    KeyCode::Backspace => {
                        if !buf.is_empty() {
                            buf.pop();
                            crate::sound::play_keypress();
                        }
                    }
                    KeyCode::Char(c) if (c as u32) >= 32 => {
                        buf.push(c);
                        crate::sound::play_keypress();
                    }
                    _ => {}
                }
            }
        }
    }
}

// ── Confirm ───────────────────────────────────────────────────────────────────

pub fn confirm(terminal: &mut Term, message: &str) -> Result<bool> {
    loop {
        terminal.draw(|f| {
            let size = f.area();
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3),
                    Constraint::Min(1),
                    Constraint::Length(1),
                ])
                .split(size);
            render_header(f, chunks[0]);
            f.render_widget(
                Paragraph::new(format!("{message}\n\n  [y] Yes    [n] No")).style(normal_style()),
                pad_horizontal(chunks[1]),
            );
            render_status_bar(f, chunks[2]);
        })?;

        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }
                if check_session_switch(key.code, key.modifiers) {
                    if crate::session::has_switch_request() {
                        return Ok(false);
                    }
                    continue;
                }
                match key.code {
                    KeyCode::Char('y') | KeyCode::Char('Y') => {
                        crate::sound::play_navigate();
                        return Ok(true);
                    }
                    KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                        crate::sound::play_navigate();
                        return Ok(false);
                    }
                    _ => {}
                }
            }
        }
    }
}

// ── Flash message ─────────────────────────────────────────────────────────────

pub fn flash_message(terminal: &mut Term, message: &str, ms: u64) -> Result<()> {
    terminal.draw(|f| {
        let size = f.area();
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(1),
                Constraint::Length(1),
            ])
            .split(size);
        render_header(f, chunks[0]);
        f.render_widget(
            Paragraph::new(format!("\n  {message}")).style(normal_style()),
            pad_horizontal(chunks[1]),
        );
        render_status_bar(f, chunks[2]);
    })?;
    std::thread::sleep(Duration::from_millis(ms));
    Ok(())
}

// ── Pager ─────────────────────────────────────────────────────────────────────

pub fn pager(terminal: &mut Term, text: &str, title: &str) -> Result<()> {
    let lines: Vec<&str> = text.lines().collect();
    let mut offset = 0usize;

    loop {
        terminal.draw(|f| {
            let size = f.area();
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3),
                    Constraint::Length(1),
                    Constraint::Length(1),
                    Constraint::Min(1),
                    Constraint::Length(1),
                    Constraint::Length(1),
                ])
                .split(size);
            render_header(f, chunks[0]);
            render_separator(f, chunks[1]);
            f.render_widget(
                Paragraph::new(title)
                    .alignment(Alignment::Center)
                    .style(title_style()),
                pad_horizontal(chunks[2]),
            );
            let visible_h = chunks[3].height as usize;
            let page: Vec<Line> = lines[offset..]
                .iter()
                .take(visible_h)
                .map(|l| Line::from(Span::styled(*l, normal_style())))
                .collect();
            f.render_widget(Paragraph::new(page), pad_horizontal(chunks[3]));
            f.render_widget(
                Paragraph::new("↑↓ scroll   q/Enter = back").style(dim_style()),
                pad_horizontal(chunks[4]),
            );
            render_status_bar(f, chunks[5]);
        })?;

        if event::poll(Duration::from_millis(30))? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }
                if check_session_switch(key.code, key.modifiers) {
                    if crate::session::has_switch_request() {
                        crate::sound::play_navigate();
                        break;
                    }
                    continue;
                }
                match key.code {
                    KeyCode::Up | KeyCode::Char('k') => {
                        let prev = offset;
                        offset = offset.saturating_sub(1);
                        if offset != prev {
                            crate::sound::play_navigate_repeat();
                        }
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        let prev = offset;
                        if offset < lines.len().saturating_sub(1) {
                            offset += 1;
                        }
                        if offset != prev {
                            crate::sound::play_navigate_repeat();
                        }
                    }
                    KeyCode::Char('q') | KeyCode::Esc | KeyCode::Enter | KeyCode::Tab => {
                        crate::sound::play_navigate();
                        break;
                    }
                    _ => {}
                }
            }
        }
    }
    Ok(())
}

// ── Box overlay ───────────────────────────────────────────────────────────────

pub fn box_message(terminal: &mut Term, message: &str, ms: u64) -> Result<()> {
    terminal.draw(|f| {
        let size = f.area();
        let w = (message.len() + 6).min(size.width as usize) as u16;
        let h = 5u16;
        let area = Rect::new(
            size.width.saturating_sub(w) / 2,
            size.height.saturating_sub(h) / 2,
            w,
            h,
        );
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(sel_style())
            .style(sel_style());
        let inner = block.inner(area);
        f.render_widget(Clear, area);
        f.render_widget(block, area);
        f.render_widget(
            Paragraph::new(message)
                .alignment(Alignment::Center)
                .style(sel_style()),
            inner,
        );
    })?;
    std::thread::sleep(Duration::from_millis(ms));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_modified_digits_to_session_indexes() {
        assert_eq!(
            session_idx_from_key(KeyCode::Char('1'), KeyModifiers::CONTROL, false),
            Some(0)
        );
        assert_eq!(
            session_idx_from_key(KeyCode::Char('9'), KeyModifiers::ALT, false),
            Some(8)
        );
        assert_eq!(
            session_idx_from_key(KeyCode::Char('4'), KeyModifiers::META, false),
            Some(3)
        );
        assert_eq!(
            session_idx_from_key(KeyCode::F(7), KeyModifiers::NONE, false),
            Some(6)
        );
    }

    #[test]
    fn maps_option_symbols_with_alt_modifier() {
        assert_eq!(
            session_idx_from_key(KeyCode::Char('\u{00A1}'), KeyModifiers::ALT, false),
            Some(0)
        );
        assert_eq!(
            session_idx_from_key(KeyCode::Char('\u{00AA}'), KeyModifiers::ALT, false),
            Some(8)
        );
    }

    #[test]
    fn plain_digit_without_modifier_does_not_switch() {
        assert_eq!(
            session_idx_from_key(KeyCode::Char('2'), KeyModifiers::NONE, false),
            None
        );
    }

    #[test]
    fn ctrl_fallback_codes_are_supported() {
        assert_eq!(
            session_idx_from_key(KeyCode::Char(' '), KeyModifiers::CONTROL, false),
            Some(1)
        );
        assert_eq!(
            session_idx_from_key(KeyCode::Backspace, KeyModifiers::CONTROL, false),
            Some(7)
        );
    }

    #[test]
    fn pty_mode_accepts_esc_prefixed_alt_digits() {
        clear_alt_escape_prefix();
        assert_eq!(
            session_idx_from_key(KeyCode::Esc, KeyModifiers::NONE, true),
            None
        );
        assert_eq!(
            session_idx_from_key(KeyCode::Char('4'), KeyModifiers::NONE, true),
            Some(3)
        );
    }

    #[test]
    fn leader_chord_maps_digit_after_ctrl_q() {
        clear_session_leader_prefix();
        remember_session_leader_prefix();
        assert_eq!(
            session_idx_from_leader_chord(KeyCode::Char('6'), KeyModifiers::NONE),
            Some(5)
        );
    }

    #[test]
    fn leader_chord_accepts_shifted_digit_punctuation() {
        clear_session_leader_prefix();
        remember_session_leader_prefix();
        assert_eq!(
            session_idx_from_leader_chord(KeyCode::Char('#'), KeyModifiers::SHIFT),
            Some(2)
        );
    }

    #[test]
    fn leader_next_shortcut_is_detected() {
        clear_session_leader_prefix();
        remember_session_leader_prefix();
        assert!(leader_requests_next_session(
            KeyCode::Char('n'),
            KeyModifiers::NONE
        ));
    }

    #[test]
    fn leader_chord_requires_leader_mode_enabled() {
        remember_session_leader_prefix();
        assert!(!check_session_switch_with_mode(
            KeyCode::Char('3'),
            KeyModifiers::NONE,
            false,
            false
        ));
        clear_session_leader_prefix();
    }
}
