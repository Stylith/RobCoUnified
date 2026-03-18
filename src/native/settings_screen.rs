use super::menu::SettingsChoiceOverlay;
use super::retro_ui::{current_palette, RetroScreen};
use crate::config::{Settings, HEADER_LINES};
use eframe::egui::{self, Context};
pub use robcos_native_settings_app::TerminalSettingsEvent;
use robcos_native_settings_app::{
    adjust_settings_slider, apply_settings_choice, handle_settings_activation,
    settings_choice_items, terminal_settings_rows, TerminalSettingsPanel,
};

#[allow(clippy::too_many_arguments)]
pub fn run_terminal_settings_screen(
    ctx: &Context,
    draft: &mut Settings,
    panel: &mut TerminalSettingsPanel,
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
    let items = terminal_settings_rows_for_panel(*panel, draft, is_admin);
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
            && adjust_settings_slider(*panel, draft, *selected_idx, is_admin, -1)
        {
            event = TerminalSettingsEvent::Persist;
        }
        if ctx.input(|i| i.key_pressed(egui::Key::ArrowRight))
            && adjust_settings_slider(*panel, draft, *selected_idx, is_admin, 1)
        {
            event = TerminalSettingsEvent::Persist;
        }
        if ctx.input(|i| i.key_pressed(egui::Key::Enter) || i.key_pressed(egui::Key::Space)) {
            event = handle_settings_activation(*panel, draft, *selected_idx, choice_overlay, is_admin);
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
            screen.centered_text(
                &painter,
                title_row,
                terminal_settings_title(*panel),
                palette.fg,
                true,
            );
            screen.separator(&painter, separator_bottom_row, &palette);
            screen.underlined_text(
                &painter,
                content_col,
                subtitle_row,
                terminal_settings_subtitle(*panel),
                palette.fg,
            );

            let choice_items = choice_overlay.map(|overlay| settings_choice_items(overlay.kind));
            let mut row = menu_start_row;
            for (idx, item) in items.iter().enumerate() {
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
                        event = handle_settings_activation(
                            *panel,
                            draft,
                            idx,
                            choice_overlay,
                            is_admin,
                        );
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

fn terminal_settings_rows_for_panel(
    panel: TerminalSettingsPanel,
    draft: &Settings,
    is_admin: bool,
) -> Vec<String> {
    match panel {
        TerminalSettingsPanel::Home => terminal_settings_rows(draft, is_admin),
        TerminalSettingsPanel::General => vec![
            format!(
                "Default Open Mode: {} [choose]",
                match draft.default_open_mode {
                    crate::config::OpenMode::Terminal => "Terminal",
                    crate::config::OpenMode::Desktop => "Desktop",
                }
            ),
            format!("Sound: {} [toggle]", if draft.sound { "ON" } else { "OFF" }),
            format!(
                "System Sound Volume: {}% [adjust]",
                draft.system_sound_volume
            ),
            format!(
                "Bootup: {} [toggle]",
                if draft.bootup { "ON" } else { "OFF" }
            ),
            format!(
                "Navigation Hints: {} [toggle]",
                if draft.show_navigation_hints {
                    "ON"
                } else {
                    "OFF"
                }
            ),
            "Back".to_string(),
        ],
        TerminalSettingsPanel::Appearance => {
            let mut items = vec![format!("Theme: {} [choose]", draft.theme)];
            if draft.theme == crate::config::CUSTOM_THEME_NAME {
                let [r, g, b] = draft.custom_theme_rgb;
                items.push(format!("Custom Theme Red: {r} [adjust]"));
                items.push(format!("Custom Theme Green: {g} [adjust]"));
                items.push(format!("Custom Theme Blue: {b} [adjust]"));
            }
            items.push(format!(
                "Border Glyphs: {} [toggle]",
                match draft.cli_acs_mode {
                    crate::config::CliAcsMode::Ascii => "ASCII",
                    crate::config::CliAcsMode::Unicode => "Unicode Smooth",
                }
            ));
            items.push("Back".to_string());
            items
        }
    }
}

fn terminal_settings_title(panel: TerminalSettingsPanel) -> &'static str {
    match panel {
        TerminalSettingsPanel::Home => "Settings",
        TerminalSettingsPanel::General => "Settings - General",
        TerminalSettingsPanel::Appearance => "Settings - Appearance",
    }
}

fn terminal_settings_subtitle(panel: TerminalSettingsPanel) -> &'static str {
    match panel {
        TerminalSettingsPanel::Home => "Choose a settings panel",
        TerminalSettingsPanel::General => "General system settings",
        TerminalSettingsPanel::Appearance => "Theme and display settings",
    }
}
