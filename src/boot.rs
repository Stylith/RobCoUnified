use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::{
    layout::Alignment,
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};
use std::time::Duration;

use crate::config::current_theme_color;
use crate::ui::Term;

fn themed_style() -> ratatui::style::Style {
    ratatui::style::Style::default().fg(current_theme_color())
}

const SEQUENCES: &[(&str, u64, u64)] = &[
    ("WELCOME TO ROBCO INDUSTRIES (TM) TERMLINK\nSET TERMINAL/INQUIRE",                                            20, 1500),
    ("RIT-V300\n>SET FILE/PROTECTION-OWNER/RFWD ACCOUNTS.F\n>SET HALT RESTART/MAINT",                              40, 1500),
    ("ROBCO INDUSTRIES (TM) TERMLINK PROTOCOL\nRETROS BIOS\nRBIOS-4.02.08.00 52EE5.E7.E8\nCopyright 2201-2203 Robco Ind.\nUppermem: 64KB\nRoot (5A8)\nMaintenance Mode", 20, 1500),
    ("LOGON ADMIN",                                                                                                 80, 2000),
    ("ROBCO INDUSTRIES UNIFIED OPERATING SYSTEM\nCOPYRIGHT 2075-2077 ROBCO INDUSTRIES\n-SERVER 1-",                40, 1000),
];

pub fn bootup(terminal: &mut Term) -> Result<()> {
    let mut displayed_lines: Vec<String> = Vec::new();

    'outer: for (text, char_delay_ms, pause_ms) in SEQUENCES {
        displayed_lines.clear();
        let all_lines: Vec<&str> = text.lines().collect();

        for &line in &all_lines {
            let mut built = String::new();
            for ch in line.chars() {
                built.push(ch);
                let snapshot = built.clone();
                let mut render_lines = displayed_lines.clone();
                render_lines.push(snapshot);

                terminal.draw(|f| draw_boot(f, &render_lines))?;

                if check_skip()? { break 'outer; }
                std::thread::sleep(Duration::from_millis(*char_delay_ms));
            }
            displayed_lines.push(built);
        }

        // Pause between sequences
        let pause_steps = pause_ms / 50;
        for _ in 0..pause_steps {
            terminal.draw(|f| draw_boot(f, &displayed_lines))?;
            if check_skip()? { break 'outer; }
            std::thread::sleep(Duration::from_millis(50));
        }
    }

    // Final flash
    terminal.draw(|f| {
        let size = f.area();
        f.render_widget(Paragraph::new(""), size);
    })?;
    std::thread::sleep(Duration::from_millis(300));

    Ok(())
}

fn draw_boot(f: &mut Frame, lines: &[String]) {
    let size = f.area();
    let style = themed_style();
    let text_lines: Vec<Line> = lines
        .iter()
        .map(|l| Line::from(Span::styled(l.as_str(), style)))
        .collect();
    let p = Paragraph::new(text_lines).alignment(Alignment::Center);
    // Center vertically
    let top = size.height.saturating_sub(lines.len() as u16) / 2;
    let pad = 3u16;
    let area = ratatui::layout::Rect {
        x: pad, y: top,
        width: size.width.saturating_sub(pad * 2),
        height: size.height.saturating_sub(top),
    };
    f.render_widget(p, area);

    let hint = Paragraph::new(Span::styled(
        "SPACE to skip", ratatui::style::Style::default().fg(ratatui::style::Color::DarkGray)
    )).alignment(Alignment::Center);
    let hint_area = ratatui::layout::Rect {
        x: 0, y: size.height.saturating_sub(1), width: size.width, height: 1
    };
    f.render_widget(hint, hint_area);
}

/// Returns true if the user pressed Space (skip).
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
