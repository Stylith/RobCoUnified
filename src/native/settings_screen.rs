use super::menu::SettingsChoiceOverlay;
use super::retro_ui::{current_palette, RetroScreen};
use crate::config::{Settings, HEADER_LINES};
use eframe::egui::{self, Context};
pub use robcos_native_settings_app::TerminalSettingsEvent;
use robcos_native_settings_app::{
    adjust_settings_slider, apply_settings_choice, handle_settings_activation,
    settings_choice_items, terminal_settings_rows,
};

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
