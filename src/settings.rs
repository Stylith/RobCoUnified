use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    text::{Line, Span},
    widgets::Paragraph,
};
use std::time::Duration;
use sysinfo::System;

use crate::auth::{is_admin, user_management_menu};
use crate::config::{
    get_settings, load_about, persist_settings, update_settings, CliAcsMode, CliColorMode,
    OpenMode, THEMES,
};
use crate::status::render_status_bar;
use crate::ui::{
    dim_style, normal_style, pad_horizontal, render_header, render_separator, run_menu, MenuResult,
    Term,
};

// ── System info ────────────────────────────────────────────────────────────────

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
            "Theme" => get_settings().theme,
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

// ── About screen ──────────────────────────────────────────────────────────────

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

pub fn about_screen(terminal: &mut Term) -> Result<()> {
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

    loop {
        terminal.draw(|f| {
            let size = f.area();
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3),
                    Constraint::Length(1),
                    Constraint::Length(ascii.len() as u16 + 1),
                    Constraint::Min(1),
                    Constraint::Length(1),
                    Constraint::Length(1),
                ])
                .split(size);

            render_header(f, chunks[0]);
            render_separator(f, chunks[1]);

            let art: Vec<Line> = ascii
                .iter()
                .map(|l| Line::from(Span::styled(l.as_str(), normal_style())))
                .collect();
            f.render_widget(
                Paragraph::new(art).alignment(Alignment::Center),
                pad_horizontal(chunks[2]),
            );

            let info_lines: Vec<Line> = info
                .iter()
                .map(|(k, v)| Line::from(Span::styled(format!("{k}: {v}"), normal_style())))
                .collect();
            f.render_widget(
                Paragraph::new(info_lines).alignment(Alignment::Center),
                pad_horizontal(chunks[3]),
            );

            let hint = Paragraph::new("q/Esc = back").style(dim_style());
            f.render_widget(hint, pad_horizontal(chunks[4]));
            render_status_bar(f, chunks[5]);
        })?;

        if event::poll(Duration::from_millis(30))? {
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
                    KeyCode::Char('q') | KeyCode::Esc | KeyCode::Tab => break,
                    _ => {}
                }
            }
        }
    }
    Ok(())
}

// ── Theme menu ────────────────────────────────────────────────────────────────

pub fn theme_menu(terminal: &mut Term) -> Result<()> {
    let theme_names: Vec<&str> = THEMES.iter().map(|(n, _)| *n).collect();
    let mut opts: Vec<&str> = theme_names.clone();
    opts.push("---");
    opts.push("Back");

    if let MenuResult::Selected(t) = run_menu(terminal, "Select Theme", &opts, None)? {
        if t != "Back" && THEMES.iter().any(|(n, _)| *n == t) {
            update_settings(|s| s.theme = t);
            persist_settings();
        }
    }
    Ok(())
}

// ── Settings menu ─────────────────────────────────────────────────────────────

pub fn cli_menu(terminal: &mut Term) -> Result<()> {
    loop {
        let s = get_settings();
        let styled_label = if s.cli_styled_render {
            "Styled PTY Rendering: ON  [toggle]"
        } else {
            "Styled PTY Rendering: OFF [toggle]"
        };
        let color_label = format!(
            "PTY Color Mode: {} [cycle]",
            match s.cli_color_mode {
                CliColorMode::ThemeLock => "Theme Lock",
                CliColorMode::PaletteMap => "Palette-map (Theme Shades)",
                CliColorMode::Color => "Color (Default Terminal)",
                CliColorMode::Monochrome => "Monochrome",
            }
        );
        let border_label = format!(
            "Border Glyphs: {} [toggle]",
            match s.cli_acs_mode {
                CliAcsMode::Ascii => "ASCII",
                CliAcsMode::Unicode => "Unicode Smooth",
            }
        );
        let choices = [
            styled_label.to_string(),
            color_label.clone(),
            border_label.clone(),
            "---".to_string(),
            "Back".to_string(),
        ];
        let choice_refs: Vec<&str> = choices.iter().map(String::as_str).collect();

        match run_menu(
            terminal,
            "CLI",
            &choice_refs,
            Some("Affects embedded terminal apps"),
        )? {
            MenuResult::Back => break,
            MenuResult::Selected(sel) => match sel.as_str() {
                "Back" => break,
                l if l == styled_label => {
                    update_settings(|s| s.cli_styled_render = !s.cli_styled_render);
                    persist_settings();
                }
                l if l == color_label => {
                    update_settings(|s| {
                        s.cli_color_mode = match s.cli_color_mode {
                            CliColorMode::ThemeLock => CliColorMode::PaletteMap,
                            CliColorMode::PaletteMap => CliColorMode::Color,
                            CliColorMode::Color => CliColorMode::Monochrome,
                            CliColorMode::Monochrome => CliColorMode::ThemeLock,
                        };
                    });
                    persist_settings();
                }
                l if l == border_label => {
                    update_settings(|s| {
                        s.cli_acs_mode = match s.cli_acs_mode {
                            CliAcsMode::Ascii => CliAcsMode::Unicode,
                            CliAcsMode::Unicode => CliAcsMode::Ascii,
                        };
                    });
                    persist_settings();
                }
                _ => {}
            },
        }
    }
    Ok(())
}

pub fn settings_menu(terminal: &mut Term, current_user: &str) -> Result<()> {
    use crate::apps::edit_menus_menu;

    let admin = is_admin(current_user);

    loop {
        let s = get_settings();
        let sound_label = if s.sound {
            "Sound: ON  [toggle]"
        } else {
            "Sound: OFF [toggle]"
        };
        let bootup_label = if s.bootup {
            "Bootup: ON [toggle]"
        } else {
            "Bootup: OFF [toggle]"
        };
        let open_mode_label = format!(
            "Default Open Mode: {} [toggle]",
            match s.default_open_mode {
                OpenMode::Terminal => "Terminal",
                OpenMode::Desktop => "Desktop",
            }
        );

        let mut choices = vec!["About", "Theme", "CLI", "Edit Menus"];
        if admin {
            choices.push("User Management");
        }
        choices.extend_from_slice(&[
            open_mode_label.as_str(),
            bootup_label,
            sound_label,
            "---",
            "Back",
        ]);

        match run_menu(terminal, "Settings", &choices, None)? {
            MenuResult::Back => break,
            MenuResult::Selected(s) => match s.as_str() {
                "About" => about_screen(terminal)?,
                "Theme" => theme_menu(terminal)?,
                "CLI" => cli_menu(terminal)?,
                "Edit Menus" => edit_menus_menu(terminal)?,
                "User Management" => user_management_menu(terminal, current_user)?,
                l if l == open_mode_label => {
                    update_settings(|s| {
                        s.default_open_mode = match s.default_open_mode {
                            OpenMode::Terminal => OpenMode::Desktop,
                            OpenMode::Desktop => OpenMode::Terminal,
                        }
                    });
                    persist_settings();
                }
                l if l == sound_label => {
                    update_settings(|s| s.sound = !s.sound);
                    persist_settings();
                }
                l if l == bootup_label => {
                    update_settings(|s| s.bootup = !s.bootup);
                    persist_settings();
                }
                _ => break,
            },
        }
    }
    Ok(())
}
