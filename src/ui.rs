use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame, Terminal,
};
use std::time::Duration;

use crate::config::{current_theme_color, HEADER_LINES};
use crate::status::render_status_bar;

pub type Term = Terminal<ratatui::backend::CrosstermBackend<std::io::Stdout>>;

// ── Color helpers ─────────────────────────────────────────────────────────────

pub fn normal_style()   -> Style { Style::default().fg(current_theme_color()) }
pub fn sel_style()      -> Style { Style::default().fg(ratatui::style::Color::Black).bg(current_theme_color()).add_modifier(Modifier::BOLD) }
pub fn title_style()    -> Style { Style::default().fg(current_theme_color()).add_modifier(Modifier::BOLD) }
pub fn dim_style()      -> Style { Style::default().fg(current_theme_color()).add_modifier(Modifier::DIM) }

// ── Header ────────────────────────────────────────────────────────────────────

pub fn render_header(f: &mut Frame, area: Rect) {
    let lines: Vec<Line> = HEADER_LINES
        .iter()
        .map(|l| Line::from(Span::styled(*l, title_style())))
        .collect();
    let p = Paragraph::new(lines).alignment(Alignment::Center);
    f.render_widget(p, area);
}

pub fn render_separator(f: &mut Frame, area: Rect) {
    let sep = "=".repeat(area.width.saturating_sub(4) as usize);
    let p = Paragraph::new(sep).alignment(Alignment::Center).style(dim_style());
    f.render_widget(p, area);
}

// ── Standard page layout ──────────────────────────────────────────────────────
/// Returns [header(3), sep(1), title(1), sep(1), content_area, status(1)]

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
                    if subtitle.is_some() { Constraint::Length(2) } else { Constraint::Length(0) },
                    Constraint::Min(1),
                    Constraint::Length(1),
                ])
                .split(size);

            render_header(f, chunks[0]);
            render_separator(f, chunks[1]);

            let title_p = Paragraph::new(title).alignment(Alignment::Center).style(title_style());
            f.render_widget(title_p, chunks[2]);
            render_separator(f, chunks[3]);

            if let Some(sub) = subtitle {
                let sp = Paragraph::new(Span::styled(sub, dim_style()))
                    .alignment(Alignment::Left);
                f.render_widget(sp, chunks[4]);
            }

            let content_area = chunks[5];
            let mut lines: Vec<Line> = Vec::new();
            for &choice in choices {
                if choice == "---" {
                    lines.push(Line::from(Span::styled("", dim_style())));
                    continue;
                }
                let selected = selectable.get(idx).copied() == Some(choice);
                if selected {
                    lines.push(Line::from(Span::styled(
                        format!("  > {choice}"),
                        sel_style(),
                    )));
                } else {
                    lines.push(Line::from(Span::styled(
                        format!("    {choice}"),
                        normal_style(),
                    )));
                }
            }
            let p = Paragraph::new(lines);
            f.render_widget(p, content_area);

            render_status_bar(f, chunks[6]);
        })?;

        if event::poll(Duration::from_millis(200))? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press { continue; }
                match key.code {
                    KeyCode::Up | KeyCode::Char('k') => {
                        if !selectable.is_empty() {
                            idx = idx.saturating_sub(1).max(0);
                            // Wrap
                            if idx == 0 && selectable.len() > 1 { /* already at top */ }
                        }
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        if !selectable.is_empty() {
                            idx = (idx + 1).min(selectable.len() - 1);
                        }
                    }
                    KeyCode::Enter | KeyCode::Char(' ') => {
                        if let Some(&sel) = selectable.get(idx) {
                            return Ok(MenuResult::Selected(sel.to_string()));
                        }
                    }
                    KeyCode::Char('q') | KeyCode::Esc | KeyCode::Tab => {
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

            let display = format!("{prompt}\n\n  > {buf}█");
            let p = Paragraph::new(display).style(normal_style());
            f.render_widget(p, chunks[2]);
            render_status_bar(f, chunks[3]);
        })?;

        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press { continue; }
                match key.code {
                    KeyCode::Enter => return Ok(Some(buf.trim().to_string())),
                    KeyCode::Esc   => return Ok(None),
                    KeyCode::Backspace => {
                        if !buf.is_empty() { buf.pop(); }
                    }
                    KeyCode::Char(c) => {
                        if (c as u32) >= 32 { buf.push(c); }
                    }
                    _ => {}
                }
            }
        }
    }
}

// ── Confirmation dialog ───────────────────────────────────────────────────────

pub fn confirm(terminal: &mut Term, message: &str) -> Result<bool> {
    loop {
        terminal.draw(|f| {
            let size = f.area();
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(3), Constraint::Min(1), Constraint::Length(1)])
                .split(size);
            render_header(f, chunks[0]);

            let msg = format!("{message}\n\n  [y] Yes    [n] No");
            let p = Paragraph::new(msg).style(normal_style());
            f.render_widget(p, chunks[1]);
            render_status_bar(f, chunks[2]);
        })?;

        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press { continue; }
                match key.code {
                    KeyCode::Char('y') | KeyCode::Char('Y') => return Ok(true),
                    KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => return Ok(false),
                    _ => {}
                }
            }
        }
    }
}

// ── Message flash ─────────────────────────────────────────────────────────────

pub fn flash_message(terminal: &mut Term, message: &str, ms: u64) -> Result<()> {
    terminal.draw(|f| {
        let size = f.area();
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(1), Constraint::Length(1)])
            .split(size);
        render_header(f, chunks[0]);
        let p = Paragraph::new(format!("\n  {message}")).style(normal_style());
        f.render_widget(p, chunks[1]);
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

            let tp = Paragraph::new(title).alignment(Alignment::Center).style(title_style());
            f.render_widget(tp, chunks[2]);

            let visible_h = chunks[3].height as usize;
            let page: Vec<Line> = lines[offset..]
                .iter()
                .take(visible_h)
                .map(|l| Line::from(Span::styled(*l, normal_style())))
                .collect();
            f.render_widget(Paragraph::new(page), chunks[3]);

            let hint = Paragraph::new("↑↓ scroll   q/Enter = back").style(dim_style());
            f.render_widget(hint, chunks[4]);
            render_status_bar(f, chunks[5]);
        })?;

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press { continue; }
                match key.code {
                    KeyCode::Up | KeyCode::Char('k') => { if offset > 0 { offset -= 1; } }
                    KeyCode::Down | KeyCode::Char('j') => {
                        let max = lines.len().saturating_sub(1);
                        if offset < max { offset += 1; }
                    }
                    KeyCode::Char('q') | KeyCode::Esc | KeyCode::Enter | KeyCode::Tab => break,
                    _ => {}
                }
            }
        }
    }
    Ok(())
}

// ── Box overlay message ───────────────────────────────────────────────────────

pub fn box_message(terminal: &mut Term, message: &str, ms: u64) -> Result<()> {
    terminal.draw(|f| {
        let size = f.area();
        let w = (message.len() + 6).min(size.width as usize) as u16;
        let h = 5u16;
        let x = size.width.saturating_sub(w) / 2;
        let y = size.height.saturating_sub(h) / 2;
        let area = Rect::new(x, y, w, h);
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(sel_style())
            .style(sel_style());
        let inner = block.inner(area);
        f.render_widget(Clear, area);
        f.render_widget(block, area);
        let p = Paragraph::new(message)
            .alignment(Alignment::Center)
            .style(sel_style());
        f.render_widget(p, inner);
    })?;
    std::thread::sleep(Duration::from_millis(ms));
    Ok(())
}
