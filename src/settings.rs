use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    text::{Line, Span},
    widgets::Paragraph,
};
use std::collections::HashSet;
use std::time::Duration;
use sysinfo::System;

use crate::auth::{is_admin, user_management_menu};
use crate::config::{
    get_settings, load_about, persist_settings, take_default_apps_prompt_pending, update_settings,
    CliAcsMode, CliColorMode, ConnectionKind, OpenMode, THEMES,
};
use crate::connections::{
    bluetooth_installer_hint, choose_discovered_connection, connect_connection,
    disconnect_connection, filter_discovered_connections, filter_network_discovered_group,
    filter_network_saved_group, forget_saved_connection, kind_label as connection_kind_label,
    kind_plural_label, macos_blueutil_missing, macos_connections_disabled,
    macos_connections_disabled_hint, network_group_label, network_menu_groups,
    network_requires_password, refresh_discovered_connections, saved_connections, saved_row_label,
    DiscoveredConnection, NetworkMenuGroup,
};
use crate::default_apps::{
    binding_label, default_app_choices, parse_custom_command_line, set_binding_for_slot,
    slot_label, DefaultAppChoiceAction, DefaultAppSlot,
};
use crate::status::render_status_bar;
use crate::ui::{
    dim_style, flash_message, input_prompt, is_back_menu_label, normal_style, pad_horizontal,
    render_header, render_separator, run_menu, MenuResult, Term,
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

fn select_default_app_for_slot(terminal: &mut Term, slot: DefaultAppSlot) -> Result<()> {
    loop {
        let choices = default_app_choices(slot);
        let mut rows: Vec<String> = choices.iter().map(|c| c.label.clone()).collect();
        rows.push("---".to_string());
        rows.push("Back".to_string());
        let refs: Vec<&str> = rows.iter().map(String::as_str).collect();

        match run_menu(
            terminal,
            &format!("Default App: {}", slot_label(slot)),
            &refs,
            None,
        )? {
            MenuResult::Back => break,
            MenuResult::Selected(sel) if sel == "Back" => break,
            MenuResult::Selected(sel) => {
                let Some(choice) = choices.iter().find(|c| c.label == sel) else {
                    continue;
                };
                match &choice.action {
                    DefaultAppChoiceAction::Set(binding) => {
                        update_settings(|s| set_binding_for_slot(s, slot, binding.clone()));
                        persist_settings();
                        break;
                    }
                    DefaultAppChoiceAction::PromptCustom => {
                        let prompt = format!("{} command (example: epy):", slot_label(slot));
                        let Some(raw) = input_prompt(terminal, &prompt)? else {
                            continue;
                        };
                        let Some(argv) = parse_custom_command_line(raw.trim()) else {
                            flash_message(terminal, "Error: invalid command line", 1200)?;
                            continue;
                        };
                        update_settings(|s| {
                            set_binding_for_slot(
                                s,
                                slot,
                                crate::config::DefaultAppBinding::CustomArgv { argv: argv.clone() },
                            )
                        });
                        persist_settings();
                        break;
                    }
                }
            }
        }
    }
    Ok(())
}

pub fn default_apps_menu(terminal: &mut Term) -> Result<()> {
    loop {
        let s = get_settings();
        let text_label = format!(
            "Text/Code Files: {} [choose]",
            binding_label(&s.default_apps.text_code)
        );
        let ebook_label = format!(
            "Ebook Files: {} [choose]",
            binding_label(&s.default_apps.ebook)
        );
        let rows = [
            text_label.clone(),
            ebook_label.clone(),
            "---".to_string(),
            "Back".to_string(),
        ];
        let refs: Vec<&str> = rows.iter().map(String::as_str).collect();
        match run_menu(terminal, "Default Apps", &refs, None)? {
            MenuResult::Back => break,
            MenuResult::Selected(sel) if sel == "Back" => break,
            MenuResult::Selected(sel) if sel == text_label => {
                select_default_app_for_slot(terminal, DefaultAppSlot::TextCode)?;
            }
            MenuResult::Selected(sel) if sel == ebook_label => {
                select_default_app_for_slot(terminal, DefaultAppSlot::Ebook)?;
            }
            _ => {}
        }
    }
    Ok(())
}

fn parse_action_slot(label: &str, action: &str) -> Option<usize> {
    let rest = label.strip_prefix(action)?.trim_start();
    let rest = rest.strip_prefix('[')?;
    let idx = rest.split(']').next()?.trim().parse::<usize>().ok()?;
    idx.checked_sub(1)
}

fn maybe_prompt_network_password(
    terminal: &mut Term,
    kind: ConnectionKind,
    detail: &str,
) -> Result<Option<String>> {
    if !matches!(kind, ConnectionKind::Network) || !network_requires_password(detail) {
        return Ok(Some(String::new()));
    }
    input_prompt(terminal, "Wi-Fi password (leave blank to cancel):")
}

fn bluetooth_disconnect_targets(discovered: &[DiscoveredConnection]) -> Vec<DiscoveredConnection> {
    let mut out = Vec::new();
    let mut seen = HashSet::new();

    for item in discovered {
        let name = item.name.trim();
        if name.is_empty() {
            continue;
        }
        let key = name.to_ascii_lowercase();
        if seen.insert(key) {
            out.push(item.clone());
        }
    }

    for entry in saved_connections(ConnectionKind::Bluetooth) {
        let name = entry.name.trim();
        if name.is_empty() {
            continue;
        }
        let key = name.to_ascii_lowercase();
        if seen.insert(key) {
            out.push(DiscoveredConnection {
                name: entry.name,
                detail: entry.detail,
            });
        }
    }

    out
}

fn saved_connections_menu(
    terminal: &mut Term,
    kind: ConnectionKind,
    group: Option<NetworkMenuGroup>,
) -> Result<()> {
    loop {
        let saved_all = saved_connections(kind);
        let saved = if matches!(kind, ConnectionKind::Network) {
            filter_network_saved_group(&saved_all, group.unwrap_or(NetworkMenuGroup::All))
        } else {
            saved_all
        };
        if saved.is_empty() {
            flash_message(
                terminal,
                &format!("No saved {}.", kind_plural_label(kind).to_ascii_lowercase()),
                1000,
            )?;
            break;
        }
        let mut rows = Vec::new();
        for (idx, entry) in saved.iter().enumerate() {
            rows.push(format!("Connect [{}]: {}", idx + 1, saved_row_label(entry)));
            rows.push(format!("Disconnect [{}]: {}", idx + 1, entry.name));
            rows.push(format!("Forget  [{}]: {}", idx + 1, entry.name));
        }
        rows.push("---".to_string());
        rows.push("Back".to_string());
        let refs: Vec<&str> = rows.iter().map(String::as_str).collect();

        match run_menu(
            terminal,
            &format!(
                "Saved {}{}",
                kind_plural_label(kind),
                group
                    .filter(|_| matches!(kind, ConnectionKind::Network))
                    .map(|g| format!(" ({})", network_group_label(g)))
                    .unwrap_or_default()
            ),
            &refs,
            Some("Connect or forget previous targets"),
        )? {
            MenuResult::Back => break,
            MenuResult::Selected(sel) if sel == "Back" => break,
            MenuResult::Selected(sel) => {
                if let Some(slot) = parse_action_slot(&sel, "Connect") {
                    if let Some(entry) = saved.get(slot) {
                        let Some(password) =
                            maybe_prompt_network_password(terminal, kind, entry.detail.as_str())?
                        else {
                            continue;
                        };
                        let msg = connect_connection(
                            kind,
                            &entry.name,
                            Some(entry.detail.as_str()),
                            if password.trim().is_empty() {
                                None
                            } else {
                                Some(password.trim())
                            },
                        )?;
                        flash_message(terminal, &msg, 900)?;
                    }
                } else if let Some(slot) = parse_action_slot(&sel, "Disconnect") {
                    if let Some(entry) = saved.get(slot) {
                        let msg = disconnect_connection(
                            kind,
                            Some(entry.name.as_str()),
                            Some(entry.detail.as_str()),
                        );
                        flash_message(terminal, &msg, 900)?;
                    }
                } else if let Some(slot) = parse_action_slot(&sel, "Forget") {
                    if let Some(entry) = saved.get(slot) {
                        if forget_saved_connection(kind, &entry.name) {
                            flash_message(terminal, "Removed.", 800)?;
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

fn connections_kind_menu(
    terminal: &mut Term,
    kind: ConnectionKind,
    group: Option<NetworkMenuGroup>,
) -> Result<()> {
    let mut discovered = refresh_discovered_connections(kind);
    loop {
        let view_discovered = if matches!(kind, ConnectionKind::Network) {
            filter_network_discovered_group(&discovered, group.unwrap_or(NetworkMenuGroup::All))
        } else {
            discovered.clone()
        };
        let view_saved = if matches!(kind, ConnectionKind::Network) {
            filter_network_saved_group(
                &saved_connections(kind),
                group.unwrap_or(NetworkMenuGroup::All),
            )
        } else {
            saved_connections(kind)
        };
        let refresh_label = format!(
            "Refresh Available {} ({})",
            kind_plural_label(kind),
            view_discovered.len()
        );
        let saved_label = format!("Saved {} ({})", kind_plural_label(kind), view_saved.len());
        let disconnect_label = if matches!(kind, ConnectionKind::Bluetooth) {
            "Disconnect Device..."
        } else {
            "Disconnect Active"
        };
        let rows = [
            "Search and Connect".to_string(),
            refresh_label.clone(),
            "Connect to Available".to_string(),
            disconnect_label.to_string(),
            saved_label.clone(),
            "---".to_string(),
            "Back".to_string(),
        ];
        let refs: Vec<&str> = rows.iter().map(String::as_str).collect();

        match run_menu(
            terminal,
            &format!(
                "Connections — {}{}",
                connection_kind_label(kind),
                group
                    .filter(|_| matches!(kind, ConnectionKind::Network))
                    .map(|g| format!(" ({})", network_group_label(g)))
                    .unwrap_or_default()
            ),
            &refs,
            Some("Search, refresh, connect, manage saved"),
        )? {
            MenuResult::Back => break,
            MenuResult::Selected(sel) if sel == "Back" => break,
            MenuResult::Selected(sel) if sel == "Search and Connect" => {
                if discovered.is_empty() {
                    discovered = refresh_discovered_connections(kind);
                }
                let Some(raw) = input_prompt(terminal, "Search query:")? else {
                    continue;
                };
                let query = raw.trim();
                if query.is_empty() {
                    flash_message(terminal, "Enter a search query.", 900)?;
                    continue;
                }
                let filtered = filter_discovered_connections(&view_discovered, query);
                if filtered.is_empty() {
                    flash_message(terminal, "No matches found.", 900)?;
                    continue;
                }
                if let Some(target) =
                    choose_discovered_connection(terminal, kind, "Search Results", &filtered, true)?
                {
                    let Some(password) =
                        maybe_prompt_network_password(terminal, kind, target.detail.as_str())?
                    else {
                        continue;
                    };
                    let msg = connect_connection(
                        kind,
                        &target.name,
                        Some(target.detail.as_str()),
                        if password.trim().is_empty() {
                            None
                        } else {
                            Some(password.trim())
                        },
                    )?;
                    flash_message(terminal, &msg, 900)?;
                }
            }
            MenuResult::Selected(sel) if sel == refresh_label => {
                discovered = refresh_discovered_connections(kind);
                flash_message(
                    terminal,
                    &format!(
                        "Found {} target(s).",
                        if matches!(kind, ConnectionKind::Network) {
                            filter_network_discovered_group(
                                &discovered,
                                group.unwrap_or(NetworkMenuGroup::All),
                            )
                            .len()
                        } else {
                            discovered.len()
                        }
                    ),
                    900,
                )?;
            }
            MenuResult::Selected(sel) if sel == "Connect to Available" => {
                if discovered.is_empty() {
                    discovered = refresh_discovered_connections(kind);
                }
                let filtered = if matches!(kind, ConnectionKind::Network) {
                    filter_network_discovered_group(
                        &discovered,
                        group.unwrap_or(NetworkMenuGroup::All),
                    )
                } else {
                    discovered.clone()
                };
                if let Some(target) = choose_discovered_connection(
                    terminal,
                    kind,
                    &format!("Available {}", kind_plural_label(kind)),
                    &filtered,
                    true,
                )? {
                    let Some(password) =
                        maybe_prompt_network_password(terminal, kind, target.detail.as_str())?
                    else {
                        continue;
                    };
                    let msg = connect_connection(
                        kind,
                        &target.name,
                        Some(target.detail.as_str()),
                        if password.trim().is_empty() {
                            None
                        } else {
                            Some(password.trim())
                        },
                    )?;
                    flash_message(terminal, &msg, 900)?;
                }
            }
            MenuResult::Selected(sel) if sel == disconnect_label => {
                if matches!(kind, ConnectionKind::Bluetooth) {
                    let targets = bluetooth_disconnect_targets(&view_discovered);
                    if targets.is_empty() {
                        flash_message(terminal, "No bluetooth devices available.", 1000)?;
                        continue;
                    }
                    if let Some(target) = choose_discovered_connection(
                        terminal,
                        kind,
                        "Disconnect Bluetooth Device",
                        &targets,
                        false,
                    )? {
                        let msg = disconnect_connection(
                            kind,
                            Some(target.name.as_str()),
                            Some(target.detail.as_str()),
                        );
                        flash_message(terminal, &msg, 900)?;
                    }
                } else {
                    let msg = disconnect_connection(kind, None, None);
                    flash_message(terminal, &msg, 900)?;
                }
            }
            MenuResult::Selected(sel) if sel == saved_label => {
                saved_connections_menu(terminal, kind, group)?;
            }
            _ => {}
        }
    }
    Ok(())
}

fn connections_network_menu(terminal: &mut Term) -> Result<()> {
    loop {
        let mut rows: Vec<String> = network_menu_groups()
            .iter()
            .map(|g| format!("{} Networks", network_group_label(*g)))
            .collect();
        rows.push("---".to_string());
        rows.push("Back".to_string());
        let refs: Vec<&str> = rows.iter().map(String::as_str).collect();
        match run_menu(terminal, "Connections — Network", &refs, None)? {
            MenuResult::Back => break,
            MenuResult::Selected(sel) if sel == "Back" => break,
            MenuResult::Selected(sel) => {
                let selected = network_menu_groups()
                    .into_iter()
                    .find(|g| format!("{} Networks", network_group_label(*g)) == sel);
                if let Some(group) = selected {
                    connections_kind_menu(terminal, ConnectionKind::Network, Some(group))?;
                }
            }
        }
    }
    Ok(())
}

pub fn connections_menu(terminal: &mut Term) -> Result<()> {
    if macos_connections_disabled() {
        flash_message(terminal, macos_connections_disabled_hint(), 1700)?;
        return Ok(());
    }
    loop {
        let (rows, subtitle) = terminal_connections_menu_rows();
        let refs: Vec<&str> = rows.iter().map(String::as_str).collect();
        match run_menu(terminal, "Connections", &refs, subtitle)? {
            MenuResult::Back => break,
            MenuResult::Selected(sel) => match sel.as_str() {
                sel if is_back_menu_label(sel) => break,
                "Network" => connections_network_menu(terminal)?,
                "Bluetooth" => connections_kind_menu(terminal, ConnectionKind::Bluetooth, None)?,
                _ => break,
            },
        }
    }
    Ok(())
}

fn terminal_connections_menu_rows() -> (Vec<String>, Option<&'static str>) {
    let mut rows = vec!["Network".to_string()];
    let subtitle = if macos_blueutil_missing() {
        Some(bluetooth_installer_hint())
    } else {
        rows.push("Bluetooth".to_string());
        None
    };
    rows.push("---".to_string());
    rows.push("Back".to_string());
    (rows, subtitle)
}

pub fn prompt_default_apps_first_login(terminal: &mut Term, username: &str) -> Result<()> {
    if !take_default_apps_prompt_pending(username) {
        return Ok(());
    }
    flash_message(terminal, "Set default apps for your files.", 1100)?;
    default_apps_menu(terminal)
}

fn settings_general_rows() -> Vec<String> {
    let s = get_settings();
    let open_mode_label = format!(
        "Default Open Mode: {} [toggle]",
        match s.default_open_mode {
            OpenMode::Terminal => "Terminal",
            OpenMode::Desktop => "Desktop",
        }
    );
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
    let nav_hints_label = if s.show_navigation_hints {
        "Navigation Hints: ON [toggle]"
    } else {
        "Navigation Hints: OFF [toggle]"
    };
    vec![
        sound_label.to_string(),
        bootup_label.to_string(),
        nav_hints_label.to_string(),
        open_mode_label,
        "---".to_string(),
        "Back".to_string(),
    ]
}

fn settings_general_menu(terminal: &mut Term) -> Result<()> {
    loop {
        let rows = settings_general_rows();
        let refs: Vec<&str> = rows.iter().map(String::as_str).collect();

        match run_menu(terminal, "Settings — General", &refs, None)? {
            MenuResult::Back => break,
            MenuResult::Selected(sel) if sel == "Back" => break,
            MenuResult::Selected(sel) => match sel.as_str() {
                l if l == rows[3] => {
                    update_settings(|s| {
                        s.default_open_mode = match s.default_open_mode {
                            OpenMode::Terminal => OpenMode::Desktop,
                            OpenMode::Desktop => OpenMode::Terminal,
                        }
                    });
                    persist_settings();
                }
                l if l == rows[0] => {
                    update_settings(|s| s.sound = !s.sound);
                    persist_settings();
                }
                l if l == rows[1] => {
                    update_settings(|s| s.bootup = !s.bootup);
                    persist_settings();
                }
                l if l == rows[2] => {
                    update_settings(|s| s.show_navigation_hints = !s.show_navigation_hints);
                    persist_settings();
                }
                _ => {}
            },
        }
    }
    Ok(())
}

fn settings_appearance_menu(terminal: &mut Term) -> Result<()> {
    loop {
        let choices = terminal_appearance_menu_choices();
        match run_menu(terminal, "Settings — Appearance", &choices, None)? {
            MenuResult::Back => break,
            MenuResult::Selected(sel) => match sel.as_str() {
                sel if is_back_menu_label(sel) => break,
                "Theme" => theme_menu(terminal)?,
                "CLI Display" => cli_menu(terminal)?,
                _ => break,
            },
        }
    }
    Ok(())
}

fn terminal_appearance_menu_choices() -> [&'static str; 4] {
    ["Theme", "CLI Display", "---", "Back"]
}

pub fn settings_menu(terminal: &mut Term, current_user: &str) -> Result<()> {
    use crate::apps::edit_menus_menu;

    let admin = is_admin(current_user);

    loop {
        let mut choices = terminal_settings_root_choices(admin);
        if admin {
            choices.push("User Management");
        }
        choices.extend_from_slice(&["About", "---", "Back"]);

        match run_menu(terminal, "Settings", &choices, None)? {
            MenuResult::Back => break,
            MenuResult::Selected(s) => match s.as_str() {
                s if is_back_menu_label(s) => break,
                "General" => settings_general_menu(terminal)?,
                "Appearance" => settings_appearance_menu(terminal)?,
                "About" => about_screen(terminal)?,
                "Default Apps" => default_apps_menu(terminal)?,
                "Connections" => connections_menu(terminal)?,
                "Edit Menus" => edit_menus_menu(terminal)?,
                "User Management" => user_management_menu(terminal, current_user)?,
                _ => break,
            },
        }
    }
    Ok(())
}

fn terminal_settings_root_choices(_admin: bool) -> Vec<&'static str> {
    let mut choices = vec!["General", "Appearance", "Default Apps", "Edit Menus"];
    if !macos_connections_disabled() {
        choices.push("Connections");
    }
    choices
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn terminal_appearance_menu_keeps_cli_display_under_appearance() {
        assert_eq!(
            terminal_appearance_menu_choices(),
            ["Theme", "CLI Display", "---", "Back"]
        );
    }

    #[test]
    fn terminal_settings_root_choices_hide_connections_when_disabled() {
        let choices = terminal_settings_root_choices(false);
        let has_connections = choices.contains(&"Connections");
        assert_eq!(has_connections, !macos_connections_disabled());
    }

    #[test]
    fn terminal_connections_menu_rows_hide_bluetooth_when_unavailable() {
        let (rows, subtitle) = terminal_connections_menu_rows();
        let has_bluetooth = rows.iter().any(|row| row == "Bluetooth");
        assert_eq!(has_bluetooth, !macos_blueutil_missing());
        if macos_blueutil_missing() {
            assert_eq!(subtitle, Some(bluetooth_installer_hint()));
        } else {
            assert!(subtitle.is_none());
        }
    }

    #[test]
    fn terminal_appearance_menu_back_row_is_recognized() {
        let back = terminal_appearance_menu_choices()
            .last()
            .copied()
            .expect("back row");
        assert!(is_back_menu_label(back));
    }

    #[test]
    fn terminal_connections_menu_back_row_is_recognized() {
        let (rows, _) = terminal_connections_menu_rows();
        let back = rows.last().expect("back row");
        assert!(is_back_menu_label(back));
    }

    #[test]
    fn terminal_general_menu_does_not_include_hacking_difficulty() {
        let rows = settings_general_rows();
        assert!(!rows
            .iter()
            .any(|row| row.starts_with("Hacking Difficulty: ")));
    }
}
