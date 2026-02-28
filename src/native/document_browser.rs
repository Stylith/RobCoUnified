use super::file_manager::{FileManagerAction, NativeFileManagerState};
use super::retro_ui::{current_palette, RetroScreen};
use crate::config::HEADER_LINES;
use eframe::egui::{self, Context};
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct DocumentBrowserRow {
    pub label: String,
    pub path: Option<PathBuf>,
}

pub fn browser_rows(file_manager: &NativeFileManagerState) -> Vec<DocumentBrowserRow> {
    let mut rows = Vec::new();
    for row in file_manager.rows() {
        let label = if row.is_dir {
            if row.label == ".." {
                "../".to_string()
            } else {
                format!("[DIR] {}", row.label)
            }
        } else {
            row.label
        };
        rows.push(DocumentBrowserRow {
            label,
            path: Some(row.path),
        });
    }
    if rows.is_empty() {
        rows.push(DocumentBrowserRow {
            label: "(empty)".to_string(),
            path: None,
        });
    }
    rows
}

pub fn activate_browser_selection(
    file_manager: &mut NativeFileManagerState,
    selected_idx: usize,
) -> FileManagerAction {
    let rows = browser_rows(file_manager);
    let idx = selected_idx.min(rows.len().saturating_sub(1));
    let Some(path) = rows.get(idx).and_then(|row| row.path.clone()) else {
        return FileManagerAction::None;
    };
    file_manager.select(Some(path));
    file_manager.activate_selected()
}

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
