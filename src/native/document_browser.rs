use super::file_manager::NativeFileManagerState;
use super::retro_ui::{current_palette, RetroScreen};
use crate::config::HEADER_LINES;
use eframe::egui::{self, Context};
pub use robcos_native_document_browser_app::{
    activate_browser_selection, browser_rows, TerminalDocumentBrowserRequest,
};

#[allow(clippy::too_many_arguments)]
pub fn draw_terminal_document_browser(
    ctx: &Context,
    file_manager: &NativeFileManagerState,
    selected_idx: &mut usize,
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
    status_row_alt: usize,
    content_col: usize,
) -> Option<usize> {
    let rows_data = browser_rows(file_manager);
    *selected_idx = (*selected_idx).min(rows_data.len().saturating_sub(1));
    if ctx.input(|i| i.key_pressed(egui::Key::ArrowUp)) {
        *selected_idx = selected_idx.saturating_sub(1);
    }
    if ctx.input(|i| i.key_pressed(egui::Key::ArrowDown)) {
        *selected_idx = (*selected_idx + 1).min(rows_data.len().saturating_sub(1));
    }

    let mut activated = None;
    if ctx.input(|i| i.key_pressed(egui::Key::Enter) || i.key_pressed(egui::Key::Space)) {
        activated = Some(*selected_idx);
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
            screen.centered_text(&painter, title_row, "Open Documents", palette.fg, true);
            screen.separator(&painter, separator_bottom_row, &palette);
            screen.underlined_text(
                &painter,
                content_col,
                subtitle_row,
                &file_manager.cwd.display().to_string(),
                palette.fg,
            );
            let mut row = menu_start_row;
            for (idx, row_data) in rows_data.iter().enumerate() {
                let selected = idx == *selected_idx;
                let text = if selected {
                    format!("  > {}", row_data.label)
                } else {
                    format!("    {}", row_data.label)
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
                    activated = Some(idx);
                }
                row += 1;
            }
            screen.text(
                &painter,
                content_col,
                status_row,
                "Enter open | Tab back | Up/Down move",
                palette.dim,
            );
            if !shell_status.is_empty() {
                screen.text(
                    &painter,
                    content_col,
                    status_row_alt,
                    shell_status,
                    palette.dim,
                );
            }
        });

    activated
}
