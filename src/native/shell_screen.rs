use super::menu::{
    entry_for_selectable_idx, selectable_menu_count, LoginMenuRow, MainMenuAction,
    MAIN_MENU_ENTRIES,
};
use super::prompt::{draw_terminal_prompt_overlay, TerminalPrompt};
use super::retro_ui::{current_palette, RetroScreen};
use crate::config::HEADER_LINES;
use eframe::egui::{self, Color32, Context};

#[allow(clippy::too_many_arguments)]
pub fn draw_login_screen(
    ctx: &Context,
    rows: &[LoginMenuRow],
    selected_idx: &mut usize,
    error: &str,
    prompt: Option<&TerminalPrompt>,
    cols: usize,
    screen_rows: usize,
    header_start_row: usize,
    separator_top_row: usize,
    title_row: usize,
    separator_bottom_row: usize,
    subtitle_row: usize,
    menu_start_row: usize,
    status_row: usize,
    content_col: usize,
) -> bool {
    if prompt.is_none() {
        if ctx.input(|i| i.key_pressed(egui::Key::ArrowUp)) {
            *selected_idx = selected_idx.saturating_sub(1);
        }
        if ctx.input(|i| i.key_pressed(egui::Key::ArrowDown)) {
            *selected_idx = (*selected_idx + 1).min(rows.len().saturating_sub(2));
        }
    }

    let enter_pressed = prompt.is_none() && ctx.input(|i| i.key_pressed(egui::Key::Enter));
    let mut activated = enter_pressed;

    egui::CentralPanel::default()
        .frame(
            egui::Frame::none()
                .fill(current_palette().bg)
                .inner_margin(0.0),
        )
        .show(ctx, |ui| {
            let palette = current_palette();
            let (screen, _) = RetroScreen::new(ui, cols, screen_rows);
            let painter = ui.painter_at(screen.rect);
            screen.paint_bg(&painter, palette.bg);
            for (idx, line) in HEADER_LINES.iter().enumerate() {
                screen.centered_text(&painter, header_start_row + idx, line, palette.fg, true);
            }
            screen.separator(&painter, separator_top_row, &palette);
            screen.centered_text(
                &painter,
                title_row,
                "ROBCO TERMLINK - Select User",
                palette.fg,
                true,
            );
            screen.separator(&painter, separator_bottom_row, &palette);
            screen.text(
                &painter,
                content_col,
                subtitle_row,
                "Welcome. Please select a user.",
                palette.fg,
            );
            if !error.is_empty() {
                screen.text(&painter, content_col, status_row, error, Color32::LIGHT_RED);
            }

            let mut row = menu_start_row;
            let mut selectable_idx = 0usize;
            for entry in rows {
                match entry {
                    LoginMenuRow::Separator => {
                        screen.text(&painter, content_col + 4, row, "---", palette.dim);
                    }
                    LoginMenuRow::User(user) => {
                        let selected = selectable_idx == *selected_idx;
                        let text = if selected {
                            format!("  > {user}")
                        } else {
                            format!("    {user}")
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
                        if !enter_pressed && !activated && response.clicked() {
                            *selected_idx = selectable_idx;
                            activated = true;
                        }
                        selectable_idx += 1;
                    }
                    LoginMenuRow::Exit => {
                        let selected = selectable_idx == *selected_idx;
                        let text = if selected {
                            "  > Exit".to_string()
                        } else {
                            "    Exit".to_string()
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
                        if !enter_pressed && !activated && response.clicked() {
                            *selected_idx = selectable_idx;
                            activated = true;
                        }
                        selectable_idx += 1;
                    }
                }
                row += 1;
            }

            if let Some(prompt) = prompt {
                draw_terminal_prompt_overlay(ui, &screen, prompt);
            }
        });

    activated
}

#[allow(clippy::too_many_arguments)]
pub fn draw_main_menu_screen(
    ctx: &Context,
    selected_idx: &mut usize,
    shell_status: &str,
    version: &str,
    cols: usize,
    screen_rows: usize,
    header_start_row: usize,
    separator_top_row: usize,
    title_row: usize,
    separator_bottom_row: usize,
    subtitle_row: usize,
    menu_start_row: usize,
    status_row: usize,
    content_col: usize,
) -> Option<MainMenuAction> {
    if ctx.input(|i| i.key_pressed(egui::Key::ArrowUp)) {
        *selected_idx = selected_idx.saturating_sub(1);
    }
    if ctx.input(|i| i.key_pressed(egui::Key::ArrowDown)) {
        *selected_idx = (*selected_idx + 1).min(selectable_menu_count() - 1);
    }

    let enter_pressed = ctx.input(|i| i.key_pressed(egui::Key::Enter));
    let mut activated = None;
    if enter_pressed {
        activated = entry_for_selectable_idx(*selected_idx).action;
    }

    egui::CentralPanel::default()
        .frame(
            egui::Frame::none()
                .fill(current_palette().bg)
                .inner_margin(0.0),
        )
        .show(ctx, |ui| {
            let palette = current_palette();
            let (screen, _) = RetroScreen::new(ui, cols, screen_rows);
            let painter = ui.painter_at(screen.rect);
            screen.paint_bg(&painter, palette.bg);
            for (idx, line) in HEADER_LINES.iter().enumerate() {
                screen.centered_text(&painter, header_start_row + idx, line, palette.fg, true);
            }
            screen.separator(&painter, separator_top_row, &palette);
            screen.centered_text(&painter, title_row, "Main Menu", palette.fg, true);
            screen.separator(&painter, separator_bottom_row, &palette);
            screen.underlined_text(&painter, content_col, subtitle_row, version, palette.fg);

            let mut visible_row = menu_start_row;
            let mut selectable_idx = 0usize;
            for entry in MAIN_MENU_ENTRIES {
                if entry.action.is_none() {
                    screen.text(
                        &painter,
                        content_col + 4,
                        visible_row,
                        entry.label,
                        palette.dim,
                    );
                    visible_row += 1;
                    continue;
                }
                let selected = selectable_idx == *selected_idx;
                let text = if selected {
                    format!("  > {}", entry.label)
                } else {
                    format!("    {}", entry.label)
                };
                let response = screen.selectable_row(
                    ui,
                    &painter,
                    &palette,
                    content_col,
                    visible_row,
                    &text,
                    selected,
                );
                if !enter_pressed && activated.is_none() && response.clicked() {
                    *selected_idx = selectable_idx;
                    activated = entry.action;
                }
                visible_row += 1;
                selectable_idx += 1;
            }

            if !shell_status.is_empty() {
                screen.text(&painter, content_col, status_row, shell_status, palette.dim);
            }
        });

    activated
}
