use ratatui::{
    layout::Rect,
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};
use chrono::Local;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use crate::ui::sel_style;

// ── Cached battery ────────────────────────────────────────────────────────────

struct BattCache { display: Option<String>, ts: Instant }
static BATT: Mutex<Option<BattCache>> = Mutex::new(None);

fn battery_display() -> Option<String> {
    let mut guard = BATT.lock().ok()?;
    if guard.as_ref().is_none_or(|c| c.ts.elapsed() > Duration::from_secs(30)) {
        let display = read_battery();
        *guard = Some(BattCache { display: display.clone(), ts: Instant::now() });
        return display;
    }
    guard.as_ref().and_then(|c| c.display.clone())
}

#[cfg(target_os = "macos")]
fn read_battery() -> Option<String> {
    let out = std::process::Command::new("pmset")
        .args(["-g", "batt"])
        .output()
        .ok()?;
    let text = String::from_utf8_lossy(&out.stdout);
    for line in text.lines() {
        if let Some(pos) = line.find('%') {
            let before = &line[..pos];
            let num_start = before.rfind(|c: char| !c.is_ascii_digit())
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
                return Some(format!("{pct}%{status}"));
            }
        }
    }
    None
}

#[cfg(target_os = "linux")]
fn read_battery() -> Option<String> {
    for entry in std::fs::read_dir("/sys/class/power_supply").ok()? {
        let path = entry.ok()?.path();
        let kind = std::fs::read_to_string(path.join("type")).ok()?;
        if kind.trim() == "Battery" {
            let cap = std::fs::read_to_string(path.join("capacity")).ok()?;
            let pct: u8 = cap.trim().parse().ok()?;
            let status_raw = std::fs::read_to_string(path.join("status")).unwrap_or_default();
            let status = match status_raw.trim() {
                "Charging"    => "↑",
                "Discharging" => "↓",
                _             => "",
            };
            return Some(format!("{pct}%{status}"));
        }
    }
    None
}

#[cfg(target_os = "windows")]
fn read_battery() -> Option<String> {
    let out = std::process::Command::new("WMIC")
        .args(["Path", "Win32_Battery", "Get", "EstimatedChargeRemaining"])
        .output()
        .ok()?;
    let text = String::from_utf8_lossy(&out.stdout);
    for line in text.lines().skip(1) {
        if let Ok(pct) = line.trim().parse::<u8>() {
            return Some(format!("{pct}%"));
        }
    }
    None
}

#[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
fn read_battery() -> Option<String> {
    None
}

// ── Status bar ────────────────────────────────────────────────────────────────

fn write_segment(
    row: &mut [char],
    occupied: &mut [bool],
    start: usize,
    text: &str,
    mark_occupied: bool,
    skip_occupied: bool,
) {
    for (offset, ch) in text.chars().enumerate() {
        let idx = start + offset;
        if idx >= row.len() { break; }
        if skip_occupied && occupied[idx] { continue; }
        row[idx] = ch;
        if mark_occupied { occupied[idx] = true; }
    }
}

pub fn render_status_bar(f: &mut Frame, area: Rect) {
    if area.height == 0 { return; }

    let ss       = sel_style();
    let now      = Local::now().format("%a %Y-%m-%d %I:%M%p").to_string();
    let batt     = battery_display().unwrap_or_else(|| "--%".to_string());
    let sessions = crate::session::get_sessions();
    let active   = crate::session::active_idx();
    let width    = area.width as usize;

    let left = format!(" {} ", now);
    let center = sessions
        .iter()
        .enumerate()
        .map(|(i, _)| {
            if i == active {
                format!("[{}*]", i + 1)
            } else {
                format!("[{}]", i + 1)
            }
        })
        .collect::<String>();
    let right = format!(" {} ", batt);

    let mut row = vec![' '; width];
    let mut occupied = vec![false; width];

    write_segment(&mut row, &mut occupied, 0, &left, true, false);

    let right_len = right.chars().count();
    let right_start = width.saturating_sub(right_len);
    write_segment(&mut row, &mut occupied, right_start, &right, true, false);

    let center_len = center.chars().count();
    let center_start = width.saturating_sub(center_len) / 2;
    write_segment(&mut row, &mut occupied, center_start, &center, false, true);

    let line = row.into_iter().collect::<String>();
    f.render_widget(Paragraph::new(Line::from(Span::styled(line, ss))), area);
}
