use super::menu::{SettingsChoiceKind, SettingsChoiceOverlay};
use super::retro_ui::{current_palette, RetroScreen};
use crate::config::{OpenMode, Settings, HEADER_LINES, THEMES};
use crate::connections::macos_connections_disabled;
use eframe::egui::{self, Context};

pub const NATIVE_UI_SCALE_OPTIONS: &[f32] = &[0.85, 1.0, 1.2, 1.4, 1.7, 2.0, 2.3, 2.6];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalSettingsEvent {
    None,
    Persist,
    Back,
    OpenConnections,
    OpenDefaultApps,
    OpenAbout,
    EnterUserManagement,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SettingsRowId {
    Sound,
    Bootup,
    NavigationHints,
    Theme,
    InterfaceSize,
    DefaultOpenMode,
    Connections,
    DefaultApps,
    About,
    UserManagement,
    Back,
}

#[allow(clippy::too_many_arguments)]
pub fn run_terminal_settings_screen(
    ctx: &Context,
    draft: &mut Settings,
    selected_idx: &mut usize,
    choice_overlay: &mut Option<SettingsChoiceOverlay>,
    is_admin: bool,
    shell_status: &str,
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
) -> TerminalSettingsEvent {
    let items = terminal_settings_rows(draft, is_admin);
    *selected_idx = (*selected_idx).min(items.len().saturating_sub(1));

    let mut event = TerminalSettingsEvent::None;
    if let Some(mut overlay) = *choice_overlay {
        let choice_items = settings_choice_items(overlay.kind);
        if ctx.input(|i| i.key_pressed(egui::Key::ArrowUp)) {
            overlay.selected = overlay.selected.saturating_sub(1);
        }
        if ctx.input(|i| i.key_pressed(egui::Key::ArrowDown)) {
            overlay.selected = (overlay.selected + 1).min(choice_items.len().saturating_sub(1));
        }
        if ctx.input(|i| i.key_pressed(egui::Key::Escape) || i.key_pressed(egui::Key::Tab)) {
            *choice_overlay = None;
        } else if ctx.input(|i| i.key_pressed(egui::Key::Enter)) {
            apply_settings_choice(draft, overlay.kind, overlay.selected);
            *choice_overlay = None;
            event = TerminalSettingsEvent::Persist;
        } else {
            *choice_overlay = Some(overlay);
        }
    } else {
        if ctx.input(|i| i.key_pressed(egui::Key::ArrowUp)) {
            *selected_idx = selected_idx.saturating_sub(1);
        }
        if ctx.input(|i| i.key_pressed(egui::Key::ArrowDown)) {
            *selected_idx = (*selected_idx + 1).min(items.len().saturating_sub(1));
        }
        if matches!(
            items.get(*selected_idx).map(|(_, id)| *id),
            Some(SettingsRowId::InterfaceSize)
        ) {
            if ctx.input(|i| i.key_pressed(egui::Key::ArrowLeft)) && step_interface_size(draft, -1)
            {
                event = TerminalSettingsEvent::Persist;
            }
            if ctx.input(|i| i.key_pressed(egui::Key::ArrowRight)) && step_interface_size(draft, 1)
            {
                event = TerminalSettingsEvent::Persist;
            }
        }
        if ctx.input(|i| i.key_pressed(egui::Key::Enter)) {
            event = handle_settings_activation(draft, *selected_idx, choice_overlay, is_admin);
        }
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
            for (idx, line) in HEADER_LINES.iter().enumerate() {
                screen.centered_text(&painter, header_start_row + idx, line, palette.fg, true);
            }
            screen.separator(&painter, separator_top_row, &palette);
            screen.centered_text(&painter, title_row, "Settings", palette.fg, true);
            screen.separator(&painter, separator_bottom_row, &palette);
            screen.underlined_text(
                &painter,
                content_col,
                subtitle_row,
                "Native terminal-style settings",
                palette.fg,
            );

            let choice_items = choice_overlay.map(|overlay| settings_choice_items(overlay.kind));
            let mut row = menu_start_row;
            for (idx, (item, row_id)) in items.iter().enumerate() {
                let selected = idx == *selected_idx;
                let text = if selected {
                    format!("  > {item}")
                } else {
                    format!("    {item}")
                };
                let response = screen.selectable_row(
                    ui,
                    &painter,
                    &palette,
                    content_col,
                    row,
                    &text,
                    selected,
                );
                if response.clicked() {
                    *selected_idx = idx;
                    if choice_overlay.is_some() {
                        *choice_overlay = None;
                    } else {
                        event = handle_settings_activation(draft, idx, choice_overlay, is_admin);
                    }
                }
                row += 1;

                if selected {
                    if let (Some(overlay), Some(choice_items)) =
                        (*choice_overlay, choice_items.as_ref())
                    {
                        for (choice_idx, choice) in choice_items.iter().enumerate() {
                            let choice_selected = choice_idx == overlay.selected;
                            let choice_text = if choice_selected {
                                format!("      > {choice}")
                            } else {
                                format!("        {choice}")
                            };
                            let response = screen.selectable_row(
                                ui,
                                &painter,
                                &palette,
                                content_col,
                                row,
                                &choice_text,
                                choice_selected,
                            );
                            if response.clicked() {
                                *choice_overlay = None;
                                apply_settings_choice(draft, overlay.kind, choice_idx);
                                event = TerminalSettingsEvent::Persist;
                            }
                            row += 1;
                        }
                        screen.text(
                            &painter,
                            content_col + 4,
                            row,
                            "Enter apply | Esc/Tab close",
                            palette.dim,
                        );
                        row += 1;
                    }
                    if matches!(row_id, SettingsRowId::InterfaceSize) {
                        let slider = format!(
                            "        {}  Left/Right adjust",
                            interface_size_slider_text(draft.native_ui_scale, 18)
                        );
                        screen.text(&painter, content_col, row, &slider, palette.dim);
                        row += 1;
                    }
                }
            }

            if !shell_status.is_empty() {
                screen.text(&painter, content_col, status_row, shell_status, palette.dim);
            }
        });

    event
}

pub fn interface_size_slider_text(scale: f32, width: usize) -> String {
    let width = width.max(4);
    let idx = NATIVE_UI_SCALE_OPTIONS
        .iter()
        .position(|v| (*v - scale).abs() < 0.001)
        .unwrap_or(1);
    let max = NATIVE_UI_SCALE_OPTIONS.len().saturating_sub(1).max(1);
    let fill = ((idx * (width - 1)) + (max / 2)) / max;
    let mut chars = vec!['-'; width];
    for ch in chars.iter_mut().take(fill) {
        *ch = '=';
    }
    chars[fill.min(width - 1)] = '|';
    format!("[{}]", chars.into_iter().collect::<String>())
}

fn handle_settings_activation(
    draft: &mut Settings,
    idx: usize,
    choice_overlay: &mut Option<SettingsChoiceOverlay>,
    is_admin: bool,
) -> TerminalSettingsEvent {
    let rows = terminal_settings_rows(draft, is_admin);
    let Some((_, row_id)) = rows.get(idx) else {
        return TerminalSettingsEvent::Back;
    };
    match row_id {
        SettingsRowId::Sound => {
            draft.sound = !draft.sound;
            TerminalSettingsEvent::Persist
        }
        SettingsRowId::Bootup => {
            draft.bootup = !draft.bootup;
            TerminalSettingsEvent::Persist
        }
        SettingsRowId::NavigationHints => {
            draft.show_navigation_hints = !draft.show_navigation_hints;
            TerminalSettingsEvent::Persist
        }
        SettingsRowId::Theme => {
            open_settings_choice(draft, choice_overlay, SettingsChoiceKind::Theme);
            TerminalSettingsEvent::None
        }
        SettingsRowId::InterfaceSize => TerminalSettingsEvent::None,
        SettingsRowId::DefaultOpenMode => {
            open_settings_choice(draft, choice_overlay, SettingsChoiceKind::DefaultOpenMode);
            TerminalSettingsEvent::None
        }
        SettingsRowId::Connections => TerminalSettingsEvent::OpenConnections,
        SettingsRowId::DefaultApps => TerminalSettingsEvent::OpenDefaultApps,
        SettingsRowId::About => TerminalSettingsEvent::OpenAbout,
        SettingsRowId::UserManagement => TerminalSettingsEvent::EnterUserManagement,
        SettingsRowId::Back => TerminalSettingsEvent::Back,
    }
}

fn open_settings_choice(
    draft: &Settings,
    choice_overlay: &mut Option<SettingsChoiceOverlay>,
    kind: SettingsChoiceKind,
) {
    let selected = match kind {
        SettingsChoiceKind::Theme => THEMES
            .iter()
            .position(|(name, _)| *name == draft.theme)
            .unwrap_or(0),
        SettingsChoiceKind::DefaultOpenMode => match draft.default_open_mode {
            OpenMode::Terminal => 0,
            OpenMode::Desktop => 1,
        },
    };
    *choice_overlay = Some(SettingsChoiceOverlay { kind, selected });
}

fn settings_choice_items(kind: SettingsChoiceKind) -> Vec<String> {
    match kind {
        SettingsChoiceKind::Theme => THEMES.iter().map(|(name, _)| (*name).to_string()).collect(),
        SettingsChoiceKind::DefaultOpenMode => vec!["Terminal".to_string(), "Desktop".to_string()],
    }
}

fn apply_settings_choice(draft: &mut Settings, kind: SettingsChoiceKind, selected: usize) {
    match kind {
        SettingsChoiceKind::Theme => {
            if let Some((name, _)) = THEMES.get(selected) {
                draft.theme = (*name).to_string();
            }
        }
        SettingsChoiceKind::DefaultOpenMode => {
            draft.default_open_mode = if selected == 0 {
                OpenMode::Terminal
            } else {
                OpenMode::Desktop
            };
        }
    }
}

fn terminal_settings_rows(draft: &Settings, is_admin: bool) -> Vec<(String, SettingsRowId)> {
    let mut rows = vec![
        (
            format!("Sound: {} [toggle]", if draft.sound { "ON" } else { "OFF" }),
            SettingsRowId::Sound,
        ),
        (
            format!(
                "Bootup: {} [toggle]",
                if draft.bootup { "ON" } else { "OFF" }
            ),
            SettingsRowId::Bootup,
        ),
        (
            format!(
                "Navigation Hints: {} [toggle]",
                if draft.show_navigation_hints {
                    "ON"
                } else {
                    "OFF"
                }
            ),
            SettingsRowId::NavigationHints,
        ),
        (
            format!("Theme: {} [choose]", draft.theme),
            SettingsRowId::Theme,
        ),
        (
            format!(
                "Interface Size: {}% [adjust]",
                (draft.native_ui_scale * 100.0).round() as i32
            ),
            SettingsRowId::InterfaceSize,
        ),
        (
            format!(
                "Default Open Mode: {} [choose]",
                match draft.default_open_mode {
                    OpenMode::Terminal => "Terminal",
                    OpenMode::Desktop => "Desktop",
                }
            ),
            SettingsRowId::DefaultOpenMode,
        ),
    ];
    if !macos_connections_disabled() {
        rows.push(("Connections".to_string(), SettingsRowId::Connections));
    }
    rows.push(("Default Apps".to_string(), SettingsRowId::DefaultApps));
    rows.push(("About".to_string(), SettingsRowId::About));
    if is_admin {
        rows.push(("User Management".to_string(), SettingsRowId::UserManagement));
    }
    rows.push(("Back".to_string(), SettingsRowId::Back));
    rows
}

fn step_interface_size(draft: &mut Settings, delta: isize) -> bool {
    let current_idx = NATIVE_UI_SCALE_OPTIONS
        .iter()
        .position(|v| (*v - draft.native_ui_scale).abs() < 0.001)
        .unwrap_or(1);
    let next_idx = if delta < 0 {
        current_idx.saturating_sub(delta.unsigned_abs())
    } else {
        (current_idx + delta as usize).min(NATIVE_UI_SCALE_OPTIONS.len().saturating_sub(1))
    };
    if next_idx == current_idx {
        return false;
    }
    draft.native_ui_scale = NATIVE_UI_SCALE_OPTIONS[next_idx];
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::get_settings;

    #[test]
    fn terminal_settings_rows_include_default_apps_and_about() {
        let draft = get_settings();
        let user_rows = terminal_settings_rows(&draft, false);
        assert!(user_rows.iter().any(|(label, _)| label == "Default Apps"));
        assert!(user_rows.iter().any(|(label, _)| label == "About"));
        assert_eq!(
            user_rows.last().map(|(label, _)| label.as_str()),
            Some("Back")
        );

        let admin_rows = terminal_settings_rows(&draft, true);
        assert!(admin_rows
            .iter()
            .any(|(label, _)| label == "User Management"));
        assert_eq!(
            admin_rows.last().map(|(label, _)| label.as_str()),
            Some("Back")
        );
    }

    #[test]
    fn handle_settings_activation_routes_new_rows_correctly() {
        let mut draft = get_settings();
        let mut overlay = None;
        let rows = terminal_settings_rows(&draft, true);
        if let Some(connections_idx) = rows
            .iter()
            .position(|(_, id)| *id == SettingsRowId::Connections)
        {
            assert!(matches!(
                handle_settings_activation(&mut draft, connections_idx, &mut overlay, true),
                TerminalSettingsEvent::OpenConnections
            ));
        }
        let default_apps_idx = rows
            .iter()
            .position(|(_, id)| *id == SettingsRowId::DefaultApps)
            .unwrap();
        let about_idx = rows
            .iter()
            .position(|(_, id)| *id == SettingsRowId::About)
            .unwrap();
        let user_mgmt_idx = rows
            .iter()
            .position(|(_, id)| *id == SettingsRowId::UserManagement)
            .unwrap();

        assert!(matches!(
            handle_settings_activation(&mut draft, default_apps_idx, &mut overlay, false),
            TerminalSettingsEvent::OpenDefaultApps
        ));
        assert!(matches!(
            handle_settings_activation(&mut draft, about_idx, &mut overlay, false),
            TerminalSettingsEvent::OpenAbout
        ));
        assert!(matches!(
            handle_settings_activation(&mut draft, user_mgmt_idx, &mut overlay, false),
            TerminalSettingsEvent::Back
        ));
        assert!(matches!(
            handle_settings_activation(&mut draft, user_mgmt_idx, &mut overlay, true),
            TerminalSettingsEvent::EnterUserManagement
        ));
    }

    #[test]
    fn connections_row_respects_platform_capability() {
        let draft = get_settings();
        let rows = terminal_settings_rows(&draft, false);
        let has_connections = rows.iter().any(|(_, id)| *id == SettingsRowId::Connections);
        assert_eq!(has_connections, !macos_connections_disabled());
    }

    #[test]
    fn step_interface_size_changes_to_neighbor_value() {
        let mut draft = get_settings();
        draft.native_ui_scale = NATIVE_UI_SCALE_OPTIONS[1];
        assert!(step_interface_size(&mut draft, 1));
        assert_eq!(draft.native_ui_scale, NATIVE_UI_SCALE_OPTIONS[2]);
        assert!(step_interface_size(&mut draft, -1));
        assert_eq!(draft.native_ui_scale, NATIVE_UI_SCALE_OPTIONS[1]);
    }
}
