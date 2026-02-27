use anyhow::{anyhow, Result};
use chrono::Local;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    text::{Line, Span},
    widgets::Paragraph,
};
use std::process::Command;
use std::time::Duration;

use crate::status::render_status_bar;
use crate::ui::{
    dim_style, normal_style, pad_horizontal, render_header, render_separator, sel_style,
    title_style, Term,
};

#[derive(Debug, Clone)]
struct NukeCodes {
    alpha: String,
    bravo: String,
    charlie: String,
    source: String,
    fetched_at: String,
}

const PROVIDERS: &[(&str, &str)] = &[
    ("NukaCrypt", "https://nukacrypt.com/"),
    (
        "NukaCrypt Legacy",
        "https://nukacrypt.com/php/home.php?hm=1",
    ),
    ("NukaPD Mirror", "https://www.nukapd.com/silo-codes"),
    ("NukaTrader Mirror", "https://nukatrader.com/launchcodes/"),
];

pub fn nuke_codes_screen(terminal: &mut Term) -> Result<()> {
    let mut state = fetch_codes();
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
                    Constraint::Length(7),
                    Constraint::Min(1),
                    Constraint::Length(1),
                    Constraint::Length(1),
                ])
                .split(size);

            render_header(f, chunks[0]);
            render_separator(f, chunks[1]);
            f.render_widget(
                Paragraph::new("NUCLEAR LAUNCH CODES")
                    .alignment(Alignment::Center)
                    .style(title_style()),
                pad_horizontal(chunks[2]),
            );
            render_separator(f, chunks[3]);

            let body = match &state {
                Ok(codes) => vec![
                    Line::from(Span::styled(
                        format!("  ALPHA   : {}", codes.alpha),
                        sel_style(),
                    )),
                    Line::from(Span::styled(
                        format!("  BRAVO   : {}", codes.bravo),
                        sel_style(),
                    )),
                    Line::from(Span::styled(
                        format!("  CHARLIE : {}", codes.charlie),
                        sel_style(),
                    )),
                    Line::from(""),
                    Line::from(Span::styled(
                        format!("  SOURCE      : {}", codes.source),
                        normal_style(),
                    )),
                    Line::from(Span::styled(
                        format!("  FETCHED AT  : {}", codes.fetched_at),
                        normal_style(),
                    )),
                ],
                Err(err) => vec![
                    Line::from(Span::styled("  UNABLE TO FETCH LIVE CODES", sel_style())),
                    Line::from(""),
                    Line::from(Span::styled(format!("  ERROR: {}", err), normal_style())),
                    Line::from(""),
                    Line::from(Span::styled(
                        "  CHECK INTERNET / SOURCE AVAILABILITY",
                        dim_style(),
                    )),
                ],
            };
            f.render_widget(Paragraph::new(body), pad_horizontal(chunks[4]));

            let notes = vec![
                Line::from(Span::styled(
                    "Codes rotate weekly. Press R to refresh now.",
                    dim_style(),
                )),
                Line::from(Span::styled("Q / Esc / Tab = Back", dim_style())),
            ];
            f.render_widget(Paragraph::new(notes), pad_horizontal(chunks[5]));
            render_separator(f, chunks[6]);
            render_status_bar(f, chunks[7]);
        })?;

        if event::poll(Duration::from_millis(35))? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }
                if crate::ui::check_session_switch_pub(key.code, key.modifiers) {
                    if crate::session::has_switch_request() {
                        break;
                    }
                    continue;
                }
                match key.code {
                    KeyCode::Char('r') | KeyCode::Char('R') => {
                        state = fetch_codes();
                    }
                    KeyCode::Char('q') | KeyCode::Esc | KeyCode::Tab => break,
                    _ => {}
                }
            }
        }
    }
    Ok(())
}

fn fetch_codes() -> Result<NukeCodes> {
    let mut last_error = String::from("no provider attempts");
    for (source, url) in PROVIDERS {
        match fetch_html(url).and_then(|html| extract_codes(&html).map(|(a, b, c)| (html, a, b, c)))
        {
            Ok((_html, alpha, bravo, charlie)) => {
                return Ok(NukeCodes {
                    alpha,
                    bravo,
                    charlie,
                    source: (*source).to_string(),
                    fetched_at: Local::now().format("%Y-%m-%d %I:%M %p").to_string(),
                });
            }
            Err(err) => {
                last_error = format!("{source}: {err}");
            }
        }
    }
    Err(anyhow!(last_error))
}

fn fetch_html(url: &str) -> Result<String> {
    let output = Command::new("curl")
        .args(["-fsSL", "--connect-timeout", "8", "--max-time", "16", url])
        .output()
        .map_err(|e| anyhow!("curl spawn failed: {e}"))?;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow!("curl failed: {}", err.trim()));
    }

    String::from_utf8(output.stdout).map_err(|e| anyhow!("invalid utf8: {e}"))
}

fn extract_codes(html: &str) -> Result<(String, String, String)> {
    let alpha = extract_code_for(html, &["alpha", "site alpha", "silo alpha"]);
    let bravo = extract_code_for(html, &["bravo", "site bravo", "silo bravo"]);
    let charlie = extract_code_for(html, &["charlie", "site charlie", "silo charlie"]);

    match (alpha, bravo, charlie) {
        (Some(a), Some(b), Some(c)) => Ok((a, b, c)),
        _ => Err(anyhow!("could not parse alpha/bravo/charlie codes")),
    }
}

fn extract_code_for(html: &str, labels: &[&str]) -> Option<String> {
    let lower = html.to_lowercase();
    labels
        .iter()
        .find_map(|label| {
            let mut start = 0usize;
            while let Some(pos) = lower[start..].find(label) {
                let abs = start + pos;
                let left = abs.saturating_sub(120);
                let right = (abs + 220).min(html.len());
                if let Some(code) = first_eight_digit_code(&html[left..right]) {
                    return Some(code);
                }
                start = abs + label.len();
            }
            None
        })
        .or_else(|| first_eight_digit_code(html))
}

fn first_eight_digit_code(s: &str) -> Option<String> {
    let bytes = s.as_bytes();
    if bytes.len() < 8 {
        return None;
    }

    for i in 0..=(bytes.len() - 8) {
        let window = &bytes[i..i + 8];
        if !window.iter().all(|b| b.is_ascii_digit()) {
            continue;
        }
        let prev_ok = i == 0 || !bytes[i - 1].is_ascii_digit();
        let next_ok = i + 8 == bytes.len() || !bytes[i + 8].is_ascii_digit();
        if prev_ok && next_ok {
            return Some(String::from_utf8_lossy(window).to_string());
        }
    }
    None
}
