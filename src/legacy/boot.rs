use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::{
    layout::{Alignment, Rect},
    text::Span,
    widgets::Paragraph,
    Frame,
};
use std::time::{Duration, Instant};

use crate::config::current_theme_color;
use crate::ui::Term;

const H_PAD: u16 = 3;
const PAUSE_TICK_MS: u64 = 50;
const AUDIO_READY_TIMEOUT_MS: u64 = 1400;

fn sleep_on_pace(next_tick: &mut Instant, step_ms: u64) {
    *next_tick += Duration::from_millis(step_ms);
    let now = Instant::now();
    if *next_tick > now {
        std::thread::sleep(*next_tick - now);
    } else {
        // If drawing/event handling ran long, resync to avoid accumulating jitter.
        *next_tick = now;
    }
}

fn themed_style() -> ratatui::style::Style {
    ratatui::style::Style::default().fg(current_theme_color())
}

fn dim_style() -> ratatui::style::Style {
    ratatui::style::Style::default().fg(ratatui::style::Color::DarkGray)
}

// Mirrors old Python timings and structure:
// (text, char_delay_ms, pause_ms, centered)
const SEQUENCES: &[(&str, u64, u64, bool)] = &[
    (
        "WELCOME TO ROBCO INDUSTRIES (TM) TERMLINK\nSET TERMINAL/INQUIRE",
        40,
        2000,
        false,
    ),
    (
        "RIT-V300\n>SET FILE/PROTECTION-OWNER/RFWD ACCOUNTS.F\n>SET HALT RESTART/MAINT",
        50,
        2000,
        false,
    ),
    (
        "ROBCO INDUSTRIES (TM) TERMLINK PROTOCOL\nRETROS BIOS\nRBIOS-4.02.08.00 52EE5.E7.E8\nCopyright 2201-2203 Robco Ind.\nUppermem: 64KB\nRoot (5A8)\nMaintenance Mode",
        30,
        2000,
        false,
    ),
    ("LOGON ADMIN", 100, 3000, false),
    (
        "ROBCO INDUSTRIES UNIFIED OPERATING SYSTEM\nCOPYRIGHT 2075-2077 ROBCO INDUSTRIES\n-SERVER 1-",
        50,
        2000,
        true,
    ),
];

pub fn bootup(terminal: &mut Term) -> Result<()> {
    if crate::config::get_settings().sound {
        // Do not draw first boot text until audio helper has initialized.
        crate::sound::wait_boot_audio_ready(AUDIO_READY_TIMEOUT_MS);
    }

    'outer: for (text, char_delay_ms, pause_ms, centered) in SEQUENCES {
        let mut next_tick = Instant::now();
        if *centered {
            let src_lines: Vec<&str> = text.lines().collect();
            let mut lines: Vec<String> = vec![String::new(); src_lines.len()];

            for (li, src) in src_lines.iter().enumerate() {
                for ch in src.chars() {
                    lines[li].push(ch);
                    crate::sound::play_keypress(); // charscroll, like old centered phase
                    terminal.draw(|f| draw_centered_text(f, &lines))?;
                    if check_skip()? {
                        break 'outer;
                    }
                    sleep_on_pace(&mut next_tick, *char_delay_ms);
                }
            }
        } else {
            let mut lines: Vec<String> = vec![String::new()];
            for ch in text.chars() {
                if check_skip()? {
                    break 'outer;
                }
                if ch == '\n' {
                    lines.push(String::new());
                } else {
                    if let Some(last) = lines.last_mut() {
                        last.push(ch);
                    }
                    crate::sound::play_boot_key(); // random charsingle set, like old
                }
                terminal.draw(|f| draw_terminal(f, &lines))?;
                sleep_on_pace(&mut next_tick, *char_delay_ms);
            }
        }

        let pause_steps = pause_ms / PAUSE_TICK_MS;
        for _ in 0..pause_steps {
            if check_skip()? {
                break 'outer;
            }
            std::thread::sleep(Duration::from_millis(PAUSE_TICK_MS));
        }
    }

    crate::sound::play_login();
    std::thread::sleep(Duration::from_millis(170));

    terminal.draw(|f| {
        f.render_widget(Paragraph::new(""), f.area());
    })?;
    std::thread::sleep(Duration::from_millis(120));

    Ok(())
}

fn draw_terminal(f: &mut Frame, lines: &[String]) {
    let size = f.area();
    f.render_widget(Paragraph::new(""), size);

    for (i, line) in lines.iter().enumerate() {
        let y = i as u16;
        if y >= size.height.saturating_sub(1) {
            break;
        }
        f.render_widget(
            Paragraph::new(Span::styled(line.as_str(), themed_style())),
            Rect {
                x: H_PAD,
                y,
                width: size.width.saturating_sub(H_PAD * 2),
                height: 1,
            },
        );
    }

    f.render_widget(
        Paragraph::new(Span::styled("SPACE to skip", dim_style())).alignment(Alignment::Center),
        Rect {
            x: 0,
            y: size.height.saturating_sub(1),
            width: size.width,
            height: 1,
        },
    );
}

fn draw_centered_text(f: &mut Frame, lines: &[String]) {
    let size = f.area();
    f.render_widget(Paragraph::new(""), size);

    for (i, line) in lines.iter().enumerate() {
        let y = i as u16;
        if y >= size.height.saturating_sub(1) {
            break;
        }
        let line_w = line.chars().count() as u16;
        let x = size.width.saturating_sub(line_w) / 2;
        f.render_widget(
            Paragraph::new(Span::styled(line.as_str(), themed_style())),
            Rect {
                x,
                y,
                width: size.width.saturating_sub(x),
                height: 1,
            },
        );
    }

    f.render_widget(
        Paragraph::new(Span::styled("SPACE to skip", dim_style())).alignment(Alignment::Center),
        Rect {
            x: 0,
            y: size.height.saturating_sub(1),
            width: size.width,
            height: 1,
        },
    );
}

fn check_skip() -> Result<bool> {
    if event::poll(Duration::from_millis(0))? {
        if let Event::Key(k) = event::read()? {
            if k.kind == KeyEventKind::Press && matches!(k.code, KeyCode::Char(' ')) {
                return Ok(true);
            }
        }
    }
    Ok(false)
}
