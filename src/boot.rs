use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::{
    layout::{Alignment, Rect},
    style::Modifier,
    text::Span,
    widgets::Paragraph,
    Frame,
};
use std::time::Duration;

use crate::config::{current_theme_color, HEADER_LINES};
use crate::ui::Term;

const H_PAD: u16 = 3;
const PAUSE_TICK_MS: u64 = 16;

fn themed_style() -> ratatui::style::Style {
    ratatui::style::Style::default().fg(current_theme_color())
}

fn title_style() -> ratatui::style::Style {
    ratatui::style::Style::default()
        .fg(current_theme_color())
        .add_modifier(Modifier::BOLD)
}

fn dim_style() -> ratatui::style::Style {
    ratatui::style::Style::default()
        .fg(ratatui::style::Color::DarkGray)
}

// The last entry is the ROBCO header — rendered centered like the main UI header.
// All others are plain terminal-style left-aligned output.
const SEQUENCES: &[(&str, u64, u64)] = &[
    ("WELCOME TO ROBCO INDUSTRIES (TM) TERMLINK\nSET TERMINAL/INQUIRE",                                            64, 220),
    ("RIT-V300\n>SET FILE/PROTECTION-OWNER/RFWD ACCOUNTS.F\n>SET HALT RESTART/MAINT",                              66, 230),
    ("ROBCO INDUSTRIES (TM) TERMLINK PROTOCOL\nRETROS BIOS\nRBIOS-4.02.08.00 52EE5.E7.E8\nCopyright 2201-2203 Robco Ind.\nUppermem: 64KB\nRoot (5A8)\nMaintenance Mode", 64, 260),
    ("LOGON ADMIN",                                                                                                  68, 280),
    // Sentinel — rendered as centered header, not plain text
    ("__HEADER__",                                                                                                   28, 420),
];

fn typing_delay_ms(base_ms: u64, ch: char) -> u64 {
    base_ms
        + match ch {
            ' ' => 0,
            '.' | ',' | ':' | ';' | '/' => 6,
            '>' | '-' | '=' | '(' | ')' => 2,
            _ => 0,
        }
}

pub fn bootup(terminal: &mut Term) -> Result<()> {
    let mut displayed_lines: Vec<String> = Vec::new();

    'outer: for (text, char_delay_ms, pause_ms) in SEQUENCES {
        if *text == "__HEADER__" {
            // Render the ROBCO header centered, like the main UI
            crate::sound::play_boot_header();
            let pause_steps = pause_ms / PAUSE_TICK_MS;
            for _ in 0..pause_steps {
                terminal.draw(|f| draw_header(f))?;
                if check_skip()? { break 'outer; }
                std::thread::sleep(Duration::from_millis(PAUSE_TICK_MS));
            }
            break 'outer;
        }

        displayed_lines.clear();
        let all_lines: Vec<&str> = text.lines().collect();

        for &line in &all_lines {
            let mut built = String::new();
            for ch in line.chars() {
                built.push(ch);
                let mut render_lines = displayed_lines.clone();
                render_lines.push(built.clone());

                crate::sound::play_boot_key();
                terminal.draw(|f| draw_terminal(f, &render_lines))?;

                if check_skip()? { break 'outer; }
                std::thread::sleep(Duration::from_millis(typing_delay_ms(*char_delay_ms, ch)));
            }
            displayed_lines.push(built);
        }

        let pause_steps = pause_ms / PAUSE_TICK_MS;
        for _ in 0..pause_steps {
            terminal.draw(|f| draw_terminal(f, &displayed_lines))?;
            if check_skip()? { break 'outer; }
            std::thread::sleep(Duration::from_millis(PAUSE_TICK_MS));
        }
    }

    // Brief blank flash before handing off to login
    terminal.draw(|f| {
        f.render_widget(Paragraph::new(""), f.area());
    })?;
    std::thread::sleep(Duration::from_millis(120));

    Ok(())
}

/// Plain terminal output — left-aligned, starting at top-left with H_PAD margin.
fn draw_terminal(f: &mut Frame, lines: &[String]) {
    let size = f.area();
    let style = themed_style();

    // Clear
    f.render_widget(Paragraph::new(""), size);

    for (i, line) in lines.iter().enumerate() {
        let y = i as u16;
        if y >= size.height.saturating_sub(1) { break; }
        f.render_widget(
            Paragraph::new(Span::styled(line.as_str(), style)),
            Rect { x: H_PAD, y, width: size.width.saturating_sub(H_PAD * 2), height: 1 },
        );
    }

    // Skip hint at bottom
    f.render_widget(
        Paragraph::new(Span::styled("SPACE to skip", dim_style())).alignment(Alignment::Center),
        Rect { x: 0, y: size.height.saturating_sub(1), width: size.width, height: 1 },
    );
}

/// Render HEADER_LINES centered exactly like the main UI header + separators.
fn draw_header(f: &mut Frame) {
    let size = f.area();

    f.render_widget(Paragraph::new(""), size);

    // Separator line
    let sep = "=".repeat(size.width.saturating_sub(H_PAD * 2) as usize);

    let mid = size.height / 2;
    let total_rows = HEADER_LINES.len() as u16 + 2; // 2 separator rows
    let start_y = mid.saturating_sub(total_rows / 2);

    // Top separator
    f.render_widget(
        Paragraph::new(Span::styled(sep.as_str(), themed_style())).alignment(Alignment::Center),
        Rect { x: H_PAD, y: start_y, width: size.width.saturating_sub(H_PAD * 2), height: 1 },
    );

    // Header lines
    for (i, &line) in HEADER_LINES.iter().enumerate() {
        f.render_widget(
            Paragraph::new(Span::styled(line, title_style())).alignment(Alignment::Center),
            Rect { x: H_PAD, y: start_y + 1 + i as u16, width: size.width.saturating_sub(H_PAD * 2), height: 1 },
        );
    }

    // Bottom separator
    f.render_widget(
        Paragraph::new(Span::styled(sep.as_str(), themed_style())).alignment(Alignment::Center),
        Rect { x: H_PAD, y: start_y + 1 + HEADER_LINES.len() as u16, width: size.width.saturating_sub(H_PAD * 2), height: 1 },
    );

    // Skip hint
    f.render_widget(
        Paragraph::new(Span::styled("SPACE to skip", dim_style())).alignment(Alignment::Center),
        Rect { x: 0, y: size.height.saturating_sub(1), width: size.width, height: 1 },
    );
}

fn check_skip() -> Result<bool> {
    if event::poll(Duration::from_millis(0))? {
        if let Event::Key(k) = event::read()? {
            if k.kind == KeyEventKind::Press {
                if matches!(k.code, KeyCode::Char(' ') | KeyCode::Enter | KeyCode::Esc) {
                    return Ok(true);
                }
            }
        }
    }
    Ok(false)
}
