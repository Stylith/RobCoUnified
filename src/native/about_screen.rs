use super::retro_ui::{current_palette, RetroScreen};
use crate::config::load_about;
use eframe::egui::{self, Context};
use std::time::Duration;
use sysinfo::System;

const DEFAULT_ASCII: &[&str] = &[
    "██████╗  ██████╗ ██████╗  ██████╗  ██████╗ ",
    "██╔══██╗██╔═══██╗██╔══██╗██╔════╝ ██╔═══██╗",
    "██████╔╝██║   ██║██████╔╝██║      ██║   ██║",
    "██╔══██╗██║   ██║██╔══██╗██║      ██║   ██║",
    "██║  ██║╚██████╔╝██████╔╝╚██████╗ ╚██████╔╝",
    "╚═╝  ╚═╝ ╚═════╝ ╚═════╝  ╚═════╝  ╚═════╝ ",
];

const DEFAULT_FIELDS: &[&str] = &[
    "OS", "Hostname", "CPU", "RAM", "Uptime", "Battery", "Theme", "Shell",
];

#[allow(clippy::too_many_arguments)]
pub fn draw_about_screen(
    ctx: &Context,
    cols: usize,
    rows: usize,
    header_start_row: usize,
    separator_top_row: usize,
    title_row: usize,
    separator_bottom_row: usize,
    subtitle_row: usize,
    menu_start_row: usize,
    status_row: usize,
    content_col: usize,
) -> bool {
    let config = load_about();
    let ascii: Vec<String> = if config.ascii.is_empty() {
        DEFAULT_ASCII.iter().map(|s| s.to_string()).collect()
    } else {
        config.ascii.clone()
    };
    let fields: Vec<String> = if config.fields.is_empty() {
        DEFAULT_FIELDS.iter().map(|s| s.to_string()).collect()
    } else {
        config.fields.clone()
    };
    let info = get_system_info(&fields);

    let back = ctx.input(|i| i.key_pressed(egui::Key::Escape) || i.key_pressed(egui::Key::Tab))
        || ctx.input(|i| i.key_pressed(egui::Key::Q));
    if !back {
        ctx.request_repaint_after(Duration::from_millis(1000));
    }

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
            for (idx, line) in crate::config::HEADER_LINES.iter().enumerate() {
                screen.centered_text(&painter, header_start_row + idx, line, palette.fg, true);
            }
            screen.separator(&painter, separator_top_row, &palette);
            screen.centered_text(&painter, title_row, "About", palette.fg, true);
            screen.separator(&painter, separator_bottom_row, &palette);

            let mut row = subtitle_row;
            for line in ascii {
                screen.centered_text(&painter, row, &line, palette.fg, false);
                row += 1;
            }
            row = row.max(menu_start_row);
            for (key, value) in info {
                screen.text(
                    &painter,
                    content_col,
                    row,
                    &format!("{key}: {value}"),
                    palette.fg,
                );
                row += 1;
            }
            screen.text(
                &painter,
                content_col,
                status_row,
                "q/Esc/Tab = back",
                palette.dim,
            );
        });

    back
}

fn get_system_info(fields: &[String]) -> Vec<(String, String)> {
    let mut sys = System::new_all();
    sys.refresh_all();

    let mut info = Vec::new();
    for field in fields {
        let val: String = match field.as_str() {
            "OS" => format!(
                "{} {}",
                System::name().unwrap_or_default(),
                System::os_version().unwrap_or_default()
            ),
            "Hostname" => System::host_name().unwrap_or_default(),
            "CPU" => sys
                .cpus()
                .first()
                .map(|c| c.brand().to_string())
                .unwrap_or_default(),
            "RAM" => {
                let used = sys.used_memory() / 1024 / 1024;
                let total = sys.total_memory() / 1024 / 1024;
                format!("{used} MB / {total} MB")
            }
            "Uptime" => {
                let secs = System::uptime();
                format!("{}h {}m", secs / 3600, (secs % 3600) / 60)
            }
            "Battery" => battery_str(),
            "Theme" => crate::config::get_settings().theme,
            "Shell" => std::env::var("SHELL").unwrap_or_default(),
            "Rust" => format!("v{}", env!("CARGO_PKG_VERSION")),
            _ => continue,
        };
        info.push((field.clone(), val));
    }
    info
}

fn battery_str() -> String {
    if let Ok(rd) = std::fs::read_dir("/sys/class/power_supply") {
        for entry in rd.flatten() {
            let kind = std::fs::read_to_string(entry.path().join("type")).unwrap_or_default();
            if kind.trim() == "Battery" {
                let cap =
                    std::fs::read_to_string(entry.path().join("capacity")).unwrap_or_default();
                if let Ok(n) = cap.trim().parse::<u8>() {
                    return format!("{n}%");
                }
            }
        }
    }
    "N/A".to_string()
}
