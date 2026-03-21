use super::file_manager::NativeFileManagerState;
use super::retro_ui::{current_palette, RetroScreen};
use crate::config::HEADER_LINES;
use eframe::egui::{self, Context};
pub use robcos_native_document_browser_app::{
    activate_browser_selection, browser_rows, TerminalDocumentBrowserRequest,
};

pub enum DocumentBrowserEvent {
    None,
    Activate(usize),
    GoBack,
    Quit,
    OpenCommandPalette,
    Copy,
    Cut,
    Paste,
    Delete,
    Rename,
    Undo,
    Redo,
    NewFolder,
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
) -> DocumentBrowserEvent {
    let rows_data = browser_rows(file_manager);
    *selected_idx = (*selected_idx).min(rows_data.len().saturating_sub(1));
    if ctx.input(|i| i.key_pressed(egui::Key::ArrowUp)) {
        *selected_idx = selected_idx.saturating_sub(1);
    }
    if ctx.input(|i| i.key_pressed(egui::Key::ArrowDown)) {
        *selected_idx = (*selected_idx + 1).min(rows_data.len().saturating_sub(1));
    }

    let mut event = DocumentBrowserEvent::None;
    if ctx.input(|i| i.key_pressed(egui::Key::Enter) || i.key_pressed(egui::Key::Space)) {
        event = DocumentBrowserEvent::Activate(*selected_idx);
    } else if ctx.input(|i| i.key_pressed(egui::Key::Tab)) {
        event = DocumentBrowserEvent::GoBack;
    } else if ctx.input(|i| i.key_pressed(egui::Key::Q)) {
        event = DocumentBrowserEvent::Quit;
    } else if ctx.input(|i| i.key_pressed(egui::Key::F1)) {
        event = DocumentBrowserEvent::OpenCommandPalette;
    } else if ctx.input(|i| i.key_pressed(egui::Key::C) && i.modifiers.command) {
        event = DocumentBrowserEvent::Copy;
    } else if ctx.input(|i| i.key_pressed(egui::Key::X) && i.modifiers.command) {
        event = DocumentBrowserEvent::Cut;
    } else if ctx.input(|i| i.key_pressed(egui::Key::V) && i.modifiers.command) {
        event = DocumentBrowserEvent::Paste;
    } else if ctx.input(|i| i.key_pressed(egui::Key::Delete) || i.key_pressed(egui::Key::Backspace)) {
        event = DocumentBrowserEvent::Delete;
    } else if ctx.input(|i| i.key_pressed(egui::Key::F2)) {
        event = DocumentBrowserEvent::Rename;
    } else if ctx.input(|i| i.key_pressed(egui::Key::Z) && i.modifiers.command) {
        event = DocumentBrowserEvent::Undo;
    } else if ctx.input(|i| i.key_pressed(egui::Key::Y) && i.modifiers.command) {
        event = DocumentBrowserEvent::Redo;
    } else if ctx.input(|i| i.key_pressed(egui::Key::N) && i.modifiers.command && i.modifiers.shift) {
        event = DocumentBrowserEvent::NewFolder;
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
            let visible_rows = status_row.saturating_sub(menu_start_row);
            let scroll_offset = if rows_data.len() <= visible_rows {
                0
            } else if *selected_idx < visible_rows / 2 {
                0
            } else if *selected_idx + visible_rows / 2 >= rows_data.len() {
                rows_data.len().saturating_sub(visible_rows)
            } else {
                selected_idx.saturating_sub(visible_rows / 2)
            };
            let end = (scroll_offset + visible_rows).min(rows_data.len());
            for data_idx in scroll_offset..end {
                let row_data = &rows_data[data_idx];
                let selected = data_idx == *selected_idx;
                let text = if selected {
                    format!("  > {}", row_data.label)
                } else {
                    format!("    {}", row_data.label)
                };
                let row = menu_start_row + (data_idx - scroll_offset);
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
                    *selected_idx = data_idx;
                    event = DocumentBrowserEvent::Activate(data_idx);
                }
            }
            screen.text(
                &painter,
                content_col,
                status_row,
                "Enter open | Tab back | Q quit | Up/Down | F1 cmds",
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

    event
}
