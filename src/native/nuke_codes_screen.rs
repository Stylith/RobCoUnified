use super::retro_ui::{current_palette, RetroScreen};
use crate::config::HEADER_LINES;
use chrono::Local;
use eframe::egui::{self, Context, Rect};
use std::process::Command;

#[derive(Debug, Clone)]
pub struct NukeCodesData {
    pub alpha: String,
    pub bravo: String,
    pub charlie: String,
    pub source: String,
    pub fetched_at: String,
}

#[derive(Debug, Clone, Default)]
pub enum NukeCodesView {
    #[default]
    Unloaded,
    Data(NukeCodesData),
    Error(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NukeCodesEvent {
    None,
    Refresh,
    Back,
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

pub fn fetch_nuke_codes() -> NukeCodesView {
    let mut last_error = "no provider attempts".to_string();
    for (source, url) in PROVIDERS {
        match fetch_html(url).and_then(|html| extract_codes(&html).map(|(a, b, c)| (a, b, c))) {
            Ok((alpha, bravo, charlie)) => {
                return NukeCodesView::Data(NukeCodesData {
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
    NukeCodesView::Error(last_error)
}

#[allow(clippy::too_many_arguments)]
pub fn draw_nuke_codes_screen(
    ctx: &Context,
    state: &NukeCodesView,
    cols: usize,
    rows: usize,
    header_start_row: usize,
    separator_top_row: usize,
    title_row: usize,
    separator_bottom_row: usize,
    menu_start_row: usize,
    status_row: usize,
    content_col: usize,
) -> NukeCodesEvent {
    let refresh = ctx.input(|i| i.key_pressed(egui::Key::R));
    let back = ctx.input(|i| {
        i.key_pressed(egui::Key::Q)
            || i.key_pressed(egui::Key::Escape)
            || i.key_pressed(egui::Key::Tab)
    });

    let event = if refresh {
        NukeCodesEvent::Refresh
    } else if back {
        NukeCodesEvent::Back
    } else {
        NukeCodesEvent::None
    };

    egui::CentralPanel::default()
        .frame(
            egui::Frame::none()
                .fill(current_palette().bg)
                .inner_margin(0.0),
        )
        .show(ctx, |ui| {
            let palette = current_palette();
            let (screen, _) = RetroScreen::new(ui, cols, rows);
            let painter = ui.painter_at(screen.rect);
            screen.paint_bg(&painter, palette.bg);
            for (idx, line) in HEADER_LINES.iter().enumerate() {
                screen.centered_text(&painter, header_start_row + idx, line, palette.fg, true);
            }
            screen.separator(&painter, separator_top_row, &palette);
            screen.centered_text(
                &painter,
                title_row,
                "NUCLEAR LAUNCH CODES",
                palette.fg,
                true,
            );
            screen.separator(&painter, separator_bottom_row, &palette);

            match state {
                NukeCodesView::Data(codes) => {
                    let block_w = 21usize;
                    let top = screen.row_rect(content_col, menu_start_row, block_w);
                    let bottom = screen.row_rect(content_col, menu_start_row + 2, block_w);
                    let block = Rect::from_min_max(top.min, egui::pos2(bottom.max.x, bottom.max.y));
                    painter.rect_filled(block, 0.0, palette.panel);

                    screen.text(
                        &painter,
                        content_col + 1,
                        menu_start_row,
                        &format!("ALPHA   : {}", codes.alpha),
                        palette.fg,
                    );
                    screen.text(
                        &painter,
                        content_col + 1,
                        menu_start_row + 1,
                        &format!("BRAVO   : {}", codes.bravo),
                        palette.fg,
                    );
                    screen.text(
                        &painter,
                        content_col + 1,
                        menu_start_row + 2,
                        &format!("CHARLIE : {}", codes.charlie),
                        palette.fg,
                    );
                    screen.text(
                        &painter,
                        content_col + 1,
                        menu_start_row + 4,
                        &format!("SOURCE      : {}", codes.source),
                        palette.fg,
                    );
                    screen.text(
                        &painter,
                        content_col + 1,
                        menu_start_row + 5,
                        &format!("FETCHED AT  : {}", codes.fetched_at),
                        palette.fg,
                    );
                }
                NukeCodesView::Error(err) => {
                    screen.text(
                        &painter,
                        content_col + 1,
                        menu_start_row,
                        "UNABLE TO FETCH LIVE CODES",
                        palette.fg,
                    );
                    screen.text(
                        &painter,
                        content_col + 1,
                        menu_start_row + 2,
                        &format!("ERROR: {err}"),
                        palette.dim,
                    );
                }
                NukeCodesView::Unloaded => {
                    screen.text(
                        &painter,
                        content_col + 1,
                        menu_start_row,
                        "Loading launch codes...",
                        palette.fg,
                    );
                }
            }

            screen.text(
                &painter,
                content_col,
                status_row.saturating_sub(3),
                "Press R to refresh. Q / Esc / Tab = Back",
                palette.dim,
            );
        });

    event
}

fn fetch_html(url: &str) -> Result<String, String> {
    let output = Command::new("curl")
        .args(["-fsSL", "--connect-timeout", "8", "--max-time", "16", url])
        .output()
        .map_err(|e| format!("curl spawn failed: {e}"))?;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        return Err(format!("curl failed: {}", err.trim()));
    }

    String::from_utf8(output.stdout).map_err(|e| format!("invalid utf8: {e}"))
}

fn extract_codes(html: &str) -> Result<(String, String, String), String> {
    let alpha = extract_code_for(html, &["alpha", "site alpha", "silo alpha"]);
    let bravo = extract_code_for(html, &["bravo", "site bravo", "silo bravo"]);
    let charlie = extract_code_for(html, &["charlie", "site charlie", "silo charlie"]);

    match (alpha, bravo, charlie) {
        (Some(a), Some(b), Some(c)) => Ok((a, b, c)),
        _ => Err("could not parse alpha/bravo/charlie codes".to_string()),
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
