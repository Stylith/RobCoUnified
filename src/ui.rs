use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame, Terminal,
};
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
const OPTION_LIKE_MODS: KeyModifiers = KeyModifiers::ALT.union(KeyModifiers::META);

fn plain_or_shift(mods: KeyModifiers) -> bool {
    mods.is_empty() || mods == KeyModifiers::SHIFT
}

fn option_digit_idx(c: char) -> Option<usize> {
    Some(match c {
        // Common macOS Option outputs across US/intl layouts.
        '\u{00A1}' | '\u{00B9}' => 0, // 1
        '\u{2122}' | '\u{20AC}' | '\u{00B2}' => 1, // 2
        '\u{00A3}' | '\u{00B3}' => 2, // 3
        '\u{00A2}' | '\u{00A4}' => 3, // 4
        '\u{221E}' | '\u{00BD}' => 4, // 5
        '\u{00A7}' | '\u{00BE}' | '\u{00AC}' => 5, // 6
        '\u{00B6}' => 6, // 7
        '\u{2022}' => 7, // 8
        '\u{00AA}' => 8, // 9
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
    let Ok(mut slot) = ALT_ESC_PREFIX.lock() else { return false };
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
    let Ok(mut slot) = SESSION_LEADER_PREFIX.lock() else { return false };
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
    if plain_or_shift(mods) {
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
    if plain_or_shift(mods) {
        let wants_next = matches!(code, KeyCode::Tab)
            || matches!(code, KeyCode::Char('n' | 'N' | '0' | '+'));
        return wants_next && take_recent_session_leader_prefix();
    }

    clear_session_leader_prefix();
    false
}

fn session_idx_from_key(code: KeyCode, mods: KeyModifiers, allow_esc_prefix: bool) -> Option<usize> {
    if let KeyCode::F(n @ 1..=9) = code {
        return Some((n as usize) - 1);
    }

    if mods.contains(KeyModifiers::CONTROL) {
        return match code {
            KeyCode::Char(c @ '1'..='9') => Some((c as usize) - ('1' as usize)),
            KeyCode::Char(' ') => Some(1),    // Ctrl+2 fallback (NUL / Ctrl+Space)
            KeyCode::Esc => Some(2),          // Ctrl+3 fallback on enhanced terminals
            KeyCode::Backspace => Some(7),    // Ctrl+8 fallback on enhanced terminals
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
    Rect { x: area.x + pad, y: area.y, width: area.width.saturating_sub(pad * 2), height: area.height }
}

// ── Style helpers ─────────────────────────────────────────────────────────────

pub fn normal_style() -> Style { Style::default().fg(current_theme_color()) }
pub fn sel_style()    -> Style {
    Style::default().fg(ratatui::style::Color::Black).bg(current_theme_color()).add_modifier(Modifier::BOLD)
}
pub fn title_style()  -> Style { Style::default().fg(current_theme_color()).add_modifier(Modifier::BOLD) }
pub fn dim_style()    -> Style { Style::default().fg(current_theme_color()).add_modifier(Modifier::DIM) }

// ── Header / separator ────────────────────────────────────────────────────────

pub fn render_header(f: &mut Frame, area: Rect) {
    let inner = pad_horizontal(area);
    let lines: Vec<Line> = HEADER_LINES.iter()
        .map(|l| Line::from(Span::styled(*l, title_style())))
        .collect();
    f.render_widget(Paragraph::new(lines).alignment(Alignment::Center), inner);
}

pub fn render_separator(f: &mut Frame, area: Rect) {
    let inner = pad_horizontal(area);
    let sep   = "=".repeat(inner.width as usize);
    f.render_widget(Paragraph::new(sep).alignment(Alignment::Center).style(dim_style()), inner);
}

// ── Menu ──────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MenuResult { Selected(String), Back }

pub fn run_menu(
    terminal: &mut Term,
    title:    &str,
    choices:  &[&str],
    subtitle: Option<&str>,
) -> Result<MenuResult> {
    let selectable: Vec<&str> = choices.iter().copied().filter(|c| *c != "---").collect();
    let mut idx = 0usize;

    loop {
        terminal.draw(|f| {
            let size   = f.area();
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3),
                    Constraint::Length(1),
                    Constraint::Length(1),
                    Constraint::Length(1),
                    if subtitle.is_some() { Constraint::Length(2) } else { Constraint::Length(0) },
                    Constraint::Min(1),
                    Constraint::Length(1),
                ])
                .split(size);

            render_header(f, chunks[0]);
            render_separator(f, chunks[1]);
            f.render_widget(
                Paragraph::new(title).alignment(Alignment::Center).style(title_style()),
                pad_horizontal(chunks[2]),
            );
            render_separator(f, chunks[3]);

            if let Some(sub) = subtitle {
                f.render_widget(
                    Paragraph::new(Span::styled(sub,
                        Style::default().fg(current_theme_color()).add_modifier(Modifier::UNDERLINED)
                    )).alignment(Alignment::Left),
                    pad_horizontal(chunks[4]),
                );
            }

            let lines: Vec<Line> = choices.iter().map(|&choice| {
                if choice == "---" { return Line::from(Span::styled("", dim_style())); }
                let selected = selectable.get(idx).copied() == Some(choice);
                if selected {
                    Line::from(Span::styled(format!("  > {choice}"), sel_style()))
                } else {
                    Line::from(Span::styled(format!("    {choice}"), normal_style()))
                }
            }).collect();
            f.render_widget(Paragraph::new(lines), pad_horizontal(chunks[5]));
            render_status_bar(f, chunks[6]);
        })?;

        if event::poll(Duration::from_millis(25))? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press { continue; }
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

// ── Text input ────────────────────────────────────────────────────────────────

pub fn input_prompt(terminal: &mut Term, prompt: &str) -> Result<Option<String>> {
    let mut buf = String::new();
    loop {
        terminal.draw(|f| {
            let size   = f.area();
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(3), Constraint::Length(1), Constraint::Min(1), Constraint::Length(1)])
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
                if key.kind != KeyEventKind::Press { continue; }
                if check_session_switch(key.code, key.modifiers) {
                    if crate::session::has_switch_request() {
                        return Ok(None);
                    }
                    continue;
                }
                match key.code {
                    KeyCode::Enter     => { crate::sound::play_navigate(); return Ok(Some(buf.trim().to_string())); }
                    KeyCode::Esc       => { crate::sound::play_navigate(); return Ok(None); }
                    KeyCode::Backspace => { if !buf.is_empty() { buf.pop(); crate::sound::play_keypress(); } }
                    KeyCode::Char(c) if (c as u32) >= 32 => { buf.push(c); crate::sound::play_keypress(); }
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
            let size   = f.area();
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(3), Constraint::Length(1), Constraint::Min(1), Constraint::Length(1)])
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
                if key.kind != KeyEventKind::Press { continue; }
                if check_session_switch(key.code, key.modifiers) {
                    if crate::session::has_switch_request() {
                        return Ok(None);
                    }
                    continue;
                }
                match key.code {
                    KeyCode::Enter     => { crate::sound::play_navigate(); return Ok(Some(buf)); }
                    KeyCode::Esc       => { crate::sound::play_navigate(); return Ok(None); }
                    KeyCode::Backspace => { if !buf.is_empty() { buf.pop(); crate::sound::play_keypress(); } }
                    KeyCode::Char(c) if (c as u32) >= 32 => { buf.push(c); crate::sound::play_keypress(); }
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
            let size   = f.area();
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(3), Constraint::Min(1), Constraint::Length(1)])
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
                if key.kind != KeyEventKind::Press { continue; }
                if check_session_switch(key.code, key.modifiers) {
                    if crate::session::has_switch_request() {
                        return Ok(false);
                    }
                    continue;
                }
                match key.code {
                    KeyCode::Char('y') | KeyCode::Char('Y') => { crate::sound::play_navigate(); return Ok(true); }
                    KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => { crate::sound::play_navigate(); return Ok(false); }
                    _ => {}
                }
            }
        }
    }
}

// ── Flash message ─────────────────────────────────────────────────────────────

pub fn flash_message(terminal: &mut Term, message: &str, ms: u64) -> Result<()> {
    terminal.draw(|f| {
        let size   = f.area();
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(1), Constraint::Length(1)])
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
            let size   = f.area();
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3), Constraint::Length(1), Constraint::Length(1),
                    Constraint::Min(1), Constraint::Length(1), Constraint::Length(1),
                ])
                .split(size);
            render_header(f, chunks[0]);
            render_separator(f, chunks[1]);
            f.render_widget(
                Paragraph::new(title).alignment(Alignment::Center).style(title_style()),
                pad_horizontal(chunks[2]),
            );
            let visible_h = chunks[3].height as usize;
            let page: Vec<Line> = lines[offset..].iter().take(visible_h)
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
                if key.kind != KeyEventKind::Press { continue; }
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
                        if offset > 0 {
                            offset -= 1;
                        }
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
                        crate::sound::play_navigate(); break;
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
        let size  = f.area();
        let w     = (message.len() + 6).min(size.width as usize) as u16;
        let h     = 5u16;
        let area  = Rect::new(size.width.saturating_sub(w) / 2, size.height.saturating_sub(h) / 2, w, h);
        let block = Block::default().borders(Borders::ALL).border_style(sel_style()).style(sel_style());
        let inner = block.inner(area);
        f.render_widget(Clear, area);
        f.render_widget(block, area);
        f.render_widget(Paragraph::new(message).alignment(Alignment::Center).style(sel_style()), inner);
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
