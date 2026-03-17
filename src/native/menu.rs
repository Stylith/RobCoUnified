use super::retro_ui::{current_palette, RetroScreen};
use crate::config::HEADER_LINES;
use eframe::egui::{self, Context};
pub use robcos_native_terminal_app::{
    entry_for_selectable_idx, login_menu_rows_from_users, selectable_menu_count,
    terminal_runtime_defaults, LoginMenuRow, MainMenuAction, SettingsChoiceKind,
    SettingsChoiceOverlay, TerminalScreen, UserManagementMode, MAIN_MENU_ENTRIES,
};

fn selectable_row_indices(items: &[String]) -> Vec<usize> {
    items
        .iter()
        .enumerate()
        .filter_map(|(idx, item)| if item == "---" { None } else { Some(idx) })
        .collect()
}

#[allow(clippy::too_many_arguments)]
pub fn draw_terminal_menu_screen(
    ctx: &Context,
    title: &str,
    subtitle: Option<&str>,
    items: &[String],
    selected_idx: &mut usize,
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
    shell_status: &str,
) -> Option<usize> {
    let selectable_rows = selectable_row_indices(items);
    if selectable_rows.is_empty() {
        return None;
    }
    *selected_idx = (*selected_idx).min(selectable_rows.len().saturating_sub(1));
    if ctx.input(|i| i.key_pressed(egui::Key::ArrowUp)) {
        let prev = *selected_idx;
        *selected_idx = selected_idx.saturating_sub(1);
        if *selected_idx != prev {
            crate::sound::play_navigate();
        }
    }
    if ctx.input(|i| i.key_pressed(egui::Key::ArrowDown)) {
        let prev = *selected_idx;
        *selected_idx = (*selected_idx + 1).min(selectable_rows.len().saturating_sub(1));
        if *selected_idx != prev {
            crate::sound::play_navigate();
        }
    }

    let enter_pressed =
        ctx.input(|i| i.key_pressed(egui::Key::Enter) || i.key_pressed(egui::Key::Space));
    let mut activated = None;
    if enter_pressed {
        activated = selectable_rows.get(*selected_idx).copied();
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
            screen.centered_text(&painter, title_row, title, palette.fg, true);
            screen.separator(&painter, separator_bottom_row, &palette);
            if let Some(sub) = subtitle {
                screen.underlined_text(&painter, content_col, subtitle_row, sub, palette.fg);
            }
            let mut row = menu_start_row;
            for (idx, item) in items.iter().enumerate() {
                if item == "---" {
                    screen.text(&painter, content_col + 4, row, "---", palette.dim);
                    row += 1;
                    continue;
                }
                let selected = selectable_rows.get(*selected_idx).copied() == Some(idx);
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
                if !enter_pressed && activated.is_none() && response.clicked() {
                    if let Some(sel_idx) = selectable_rows.iter().position(|raw| *raw == idx) {
                        *selected_idx = sel_idx;
                    }
                    activated = Some(idx);
                }
                row += 1;
            }
            if !shell_status.is_empty() {
                screen.text(&painter, content_col, status_row, shell_status, palette.dim);
            }
        });

    activated
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selectable_rows_skip_separators_and_keep_back_selectable() {
        let items = vec![
            "Applications".to_string(),
            "---".to_string(),
            "Settings".to_string(),
            "Back".to_string(),
        ];
        let rows = selectable_row_indices(&items);
        assert_eq!(rows, vec![0, 2, 3]);
    }
}
