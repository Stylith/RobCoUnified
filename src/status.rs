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

// ── Cached system info ────────────────────────────────────────────────────────

struct BattCache { pct: Option<f32>, ts: Instant }
static BATT: Mutex<Option<BattCache>> = Mutex::new(None);

fn battery_pct() -> Option<f32> {
    let mut guard = BATT.lock().ok()?;
    if guard.as_ref().map_or(true, |c| c.ts.elapsed() > Duration::from_secs(30)) {
        // sysinfo doesn't expose battery; use /sys/class/power_supply on Linux
        let pct = read_battery_linux();
        *guard = Some(BattCache { pct, ts: Instant::now() });
    }
    guard.as_ref().and_then(|c| c.pct)
}

fn read_battery_linux() -> Option<f32> {
    for entry in std::fs::read_dir("/sys/class/power_supply").ok()? {
        let path = entry.ok()?.path();
        let kind = std::fs::read_to_string(path.join("type")).ok()?;
        if kind.trim() == "Battery" {
            let cap = std::fs::read_to_string(path.join("capacity")).ok()?;
            return cap.trim().parse().ok();
        }
    }
    None
}

// ── Status bar ────────────────────────────────────────────────────────────────

pub fn render_status_bar(f: &mut Frame, area: Rect) {
    if area.height == 0 { return; }

    let now = Local::now().format("%A, %d. %B - %I:%M%p").to_string();
    let batt = battery_pct().map(|p| format!("{p:.0}%")).unwrap_or_default();

    let left  = Span::styled(format!(" {now}"), sel_style());
    let right = if batt.is_empty() {
        Span::raw("")
    } else {
        Span::styled(format!("{batt} "), sel_style())
    };

    // Pad center
    let used = now.len() + 2 + batt.len() + (if batt.is_empty() { 0 } else { 1 });
    let pad  = " ".repeat((area.width as usize).saturating_sub(used));

    let line = Line::from(vec![left, Span::styled(pad, sel_style()), right]);
    f.render_widget(Paragraph::new(line), area);
}


