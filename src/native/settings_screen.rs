use super::menu::{SettingsChoiceKind, SettingsChoiceOverlay};
use super::retro_ui::{current_palette, RetroScreen};
use crate::config::{CliAcsMode, OpenMode, Settings, CUSTOM_THEME_NAME, HEADER_LINES, THEMES};
use crate::connections::macos_connections_disabled;
use eframe::egui::{self, Context};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalSettingsEvent {
    None,
    Persist,
    Back,
    OpenConnections,
    OpenEditMenus,
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
    CustomThemeRed,
    CustomThemeGreen,
    CustomThemeBlue,
    BorderGlyphs,
    DefaultOpenMode,
    Connections,
    EditMenus,
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
        } else if ctx.input(|i| i.key_pressed(egui::Key::Enter) || i.key_pressed(egui::Key::Space))
        {
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
        if ctx.input(|i| i.key_pressed(egui::Key::ArrowLeft))
            && adjust_settings_slider(draft, *selected_idx, is_admin, -1)
        {
            event = TerminalSettingsEvent::Persist;
        }
        if ctx.input(|i| i.key_pressed(egui::Key::ArrowRight))
            && adjust_settings_slider(draft, *selected_idx, is_admin, 1)
        {
            event = TerminalSettingsEvent::Persist;
        }
        if ctx.input(|i| i.key_pressed(egui::Key::Enter) || i.key_pressed(egui::Key::Space)) {
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
            for (idx, (item, _row_id)) in items.iter().enumerate() {
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
                }
            }

            if !shell_status.is_empty() {
                screen.text(&painter, content_col, status_row, shell_status, palette.dim);
            }
        });

    event
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
        SettingsRowId::CustomThemeRed
        | SettingsRowId::CustomThemeGreen
        | SettingsRowId::CustomThemeBlue => TerminalSettingsEvent::None,
        SettingsRowId::BorderGlyphs => {
            draft.cli_acs_mode = match draft.cli_acs_mode {
                CliAcsMode::Ascii => CliAcsMode::Unicode,
                CliAcsMode::Unicode => CliAcsMode::Ascii,
            };
            TerminalSettingsEvent::Persist
        }
        SettingsRowId::DefaultOpenMode => {
            open_settings_choice(draft, choice_overlay, SettingsChoiceKind::DefaultOpenMode);
            TerminalSettingsEvent::None
        }
        SettingsRowId::Connections => TerminalSettingsEvent::OpenConnections,
        SettingsRowId::EditMenus => TerminalSettingsEvent::OpenEditMenus,
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
                "Border Glyphs: {} [toggle]",
                match draft.cli_acs_mode {
                    CliAcsMode::Ascii => "ASCII",
                    CliAcsMode::Unicode => "Unicode Smooth",
                }
            ),
            SettingsRowId::BorderGlyphs,
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
    if draft.theme == CUSTOM_THEME_NAME {
        let [r, g, b] = draft.custom_theme_rgb;
        rows.splice(
            4..4,
            [
                (
                    format!("Custom Theme Red: {r} [adjust]"),
                    SettingsRowId::CustomThemeRed,
                ),
                (
                    format!("Custom Theme Green: {g} [adjust]"),
                    SettingsRowId::CustomThemeGreen,
                ),
                (
                    format!("Custom Theme Blue: {b} [adjust]"),
                    SettingsRowId::CustomThemeBlue,
                ),
            ],
        );
    }
    if !macos_connections_disabled() {
        rows.push(("Connections".to_string(), SettingsRowId::Connections));
    }
    rows.push(("Edit Menus".to_string(), SettingsRowId::EditMenus));
    rows.push(("Default Apps".to_string(), SettingsRowId::DefaultApps));
    rows.push(("About".to_string(), SettingsRowId::About));
    if is_admin {
        rows.push(("User Management".to_string(), SettingsRowId::UserManagement));
    }
    rows.push(("Back".to_string(), SettingsRowId::Back));
    rows
}

fn adjust_settings_slider(draft: &mut Settings, idx: usize, is_admin: bool, delta: i16) -> bool {
    let rows = terminal_settings_rows(draft, is_admin);
    let Some((_, row_id)) = rows.get(idx) else {
        return false;
    };
    match row_id {
        SettingsRowId::CustomThemeRed => {
            adjust_rgb_component(&mut draft.custom_theme_rgb[0], delta);
            if draft.theme != CUSTOM_THEME_NAME {
                draft.theme = CUSTOM_THEME_NAME.to_string();
            }
            true
        }
        SettingsRowId::CustomThemeGreen => {
            adjust_rgb_component(&mut draft.custom_theme_rgb[1], delta);
            if draft.theme != CUSTOM_THEME_NAME {
                draft.theme = CUSTOM_THEME_NAME.to_string();
            }
            true
        }
        SettingsRowId::CustomThemeBlue => {
            adjust_rgb_component(&mut draft.custom_theme_rgb[2], delta);
            if draft.theme != CUSTOM_THEME_NAME {
                draft.theme = CUSTOM_THEME_NAME.to_string();
            }
            true
        }
        _ => false,
    }
}

fn adjust_rgb_component(value: &mut u8, delta: i16) {
    let next = (*value as i16 + delta).clamp(0, 255);
    *value = next as u8;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::get_settings;

    #[test]
    fn terminal_settings_rows_include_default_apps_and_about() {
        let draft = get_settings();
        let user_rows = terminal_settings_rows(&draft, false);
        assert!(user_rows.iter().any(|(label, _)| label == "Edit Menus"));
        assert!(user_rows.iter().any(|(label, _)| label == "Default Apps"));
        assert!(user_rows
            .iter()
            .any(|(label, _)| label.starts_with("Border Glyphs: ")));
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
        let edit_menus_idx = rows
            .iter()
            .position(|(_, id)| *id == SettingsRowId::EditMenus)
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
            handle_settings_activation(&mut draft, edit_menus_idx, &mut overlay, false),
            TerminalSettingsEvent::OpenEditMenus
        ));
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
    fn border_glyphs_row_toggles_acs_mode() {
        let mut draft = get_settings();
        let mut overlay = None;
        let rows = terminal_settings_rows(&draft, false);
        let idx = rows
            .iter()
            .position(|(_, id)| *id == SettingsRowId::BorderGlyphs)
            .expect("border glyph row");
        let before = draft.cli_acs_mode;
        assert!(matches!(
            handle_settings_activation(&mut draft, idx, &mut overlay, false),
            TerminalSettingsEvent::Persist
        ));
        assert_ne!(draft.cli_acs_mode, before);
    }

    #[test]
    fn custom_rgb_rows_show_only_for_custom_theme() {
        let mut draft = get_settings();
        draft.theme = "Green (Default)".to_string();
        let base_rows = terminal_settings_rows(&draft, false);
        assert!(!base_rows
            .iter()
            .any(|(_, id)| matches!(id, SettingsRowId::CustomThemeRed)));
        assert!(!base_rows
            .iter()
            .any(|(_, id)| matches!(id, SettingsRowId::CustomThemeGreen)));
        assert!(!base_rows
            .iter()
            .any(|(_, id)| matches!(id, SettingsRowId::CustomThemeBlue)));

        draft.theme = CUSTOM_THEME_NAME.to_string();
        let custom_rows = terminal_settings_rows(&draft, false);
        assert!(custom_rows
            .iter()
            .any(|(_, id)| matches!(id, SettingsRowId::CustomThemeRed)));
        assert!(custom_rows
            .iter()
            .any(|(_, id)| matches!(id, SettingsRowId::CustomThemeGreen)));
        assert!(custom_rows
            .iter()
            .any(|(_, id)| matches!(id, SettingsRowId::CustomThemeBlue)));
    }

    #[test]
    fn custom_rgb_row_adjusts_value_and_keeps_custom_theme() {
        let mut draft = get_settings();
        draft.theme = CUSTOM_THEME_NAME.to_string();
        draft.custom_theme_rgb = [10, 20, 30];
        let rows = terminal_settings_rows(&draft, false);
        let red_idx = rows
            .iter()
            .position(|(_, id)| matches!(id, SettingsRowId::CustomThemeRed))
            .expect("red row");
        assert!(adjust_settings_slider(&mut draft, red_idx, false, 5));
        assert_eq!(draft.custom_theme_rgb[0], 15);
        assert_eq!(draft.theme, CUSTOM_THEME_NAME);
    }
}
