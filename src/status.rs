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
    if guard.as_ref().map_or(true, |c| c.ts.elapsed() > Duration::from_secs(30)) {
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
                return Some(format!("{pct} %{status}"));
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
            return Some(format!("{pct} %{status}"));
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
            return Some(format!("{pct} %"));
        }
    }
    None
}

#[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
fn read_battery() -> Option<String> {
    None
}

// ── Status bar ────────────────────────────────────────────────────────────────

pub fn render_status_bar(f: &mut Frame, area: Rect) {
    if area.height == 0 { return; }

    let now  = Local::now().format("%A, %d. %B - %I:%M%p").to_string();
    let batt = battery_display().unwrap_or_default();

    let left  = Span::styled(format!(" {now}"), sel_style());
    let right = if batt.is_empty() {
        Span::raw("")
    } else {
        Span::styled(format!("{batt} "), sel_style())
    };

    let used = now.len() + 2 + batt.len() + if batt.is_empty() { 0 } else { 1 };
    let pad  = " ".repeat((area.width as usize).saturating_sub(used));

    let line = Line::from(vec![left, Span::styled(pad, sel_style()), right]);
    f.render_widget(Paragraph::new(line), area);
}