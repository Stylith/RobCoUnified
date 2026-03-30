use super::file_manager::NativeFileManagerState;
use super::retro_ui::{
    active_terminal_decoration, current_palette_for_surface, terminal_menu_row_text,
    ContentBounds, RetroScreen, ShellSurfaceKind,
};
use eframe::egui::{self, Context, Painter, Ui};
pub use nucleon_native_document_browser_app::{
    activate_browser_selection, browser_rows, sync_browser_selection,
    TerminalDocumentBrowserRequest,
};

pub enum DocumentBrowserEvent {
    None,
    Activate,
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
    OpenWith,
}

#[allow(clippy::too_many_arguments)]
pub fn paint_terminal_document_browser(
    ui: &mut Ui,
    screen: &RetroScreen,
    painter: &Painter,
    file_manager: &NativeFileManagerState,
    selected_idx: &mut usize,
    shell_status: &str,
    header_start_row: usize,
    separator_top_row: usize,
    title_row: usize,
    separator_bottom_row: usize,
    subtitle_row: usize,
    menu_start_row: usize,
    status_row: usize,
    status_row_alt: usize,
    bounds: &ContentBounds,
    input_enabled: bool,
    header_lines: &[String],
) -> DocumentBrowserEvent {
    let ctx = ui.ctx();
    let rows_data = browser_rows(file_manager);
    *selected_idx = (*selected_idx).min(rows_data.len().saturating_sub(1));
    if input_enabled {
        if ctx.input(|i| i.key_pressed(egui::Key::ArrowUp)) {
            *selected_idx = selected_idx.saturating_sub(1);
        }
        if ctx.input(|i| i.key_pressed(egui::Key::ArrowDown)) {
            *selected_idx = (*selected_idx + 1).min(rows_data.len().saturating_sub(1));
        }
    }

    let mut event = DocumentBrowserEvent::None;
    if input_enabled
        && ctx.input(|i| i.key_pressed(egui::Key::Enter) || i.key_pressed(egui::Key::Space))
    {
        event = DocumentBrowserEvent::Activate;
    } else if input_enabled && ctx.input(|i| i.key_pressed(egui::Key::Tab)) {
        event = DocumentBrowserEvent::GoBack;
    } else if input_enabled && ctx.input(|i| i.key_pressed(egui::Key::Q)) {
        event = DocumentBrowserEvent::Quit;
    } else if input_enabled && ctx.input(|i| i.key_pressed(egui::Key::F1)) {
        event = DocumentBrowserEvent::OpenCommandPalette;
    } else if input_enabled && ctx.input(|i| i.key_pressed(egui::Key::C) && i.modifiers.command) {
        event = DocumentBrowserEvent::Copy;
    } else if input_enabled && ctx.input(|i| i.key_pressed(egui::Key::X) && i.modifiers.command) {
        event = DocumentBrowserEvent::Cut;
    } else if input_enabled && ctx.input(|i| i.key_pressed(egui::Key::V) && i.modifiers.command) {
        event = DocumentBrowserEvent::Paste;
    } else if input_enabled
        && ctx.input(|i| i.key_pressed(egui::Key::Delete) || i.key_pressed(egui::Key::Backspace))
    {
        event = DocumentBrowserEvent::Delete;
    } else if input_enabled && ctx.input(|i| i.key_pressed(egui::Key::F2)) {
        event = DocumentBrowserEvent::Rename;
    } else if input_enabled && ctx.input(|i| i.key_pressed(egui::Key::Z) && i.modifiers.command) {
        event = DocumentBrowserEvent::Undo;
    } else if input_enabled && ctx.input(|i| i.key_pressed(egui::Key::Y) && i.modifiers.command) {
        event = DocumentBrowserEvent::Redo;
    } else if input_enabled
        && ctx.input(|i| i.key_pressed(egui::Key::N) && i.modifiers.command && i.modifiers.shift)
    {
        event = DocumentBrowserEvent::NewFolder;
    } else if input_enabled && ctx.input(|i| i.key_pressed(egui::Key::O) && !i.modifiers.command) {
        event = DocumentBrowserEvent::OpenWith;
    }

    let content_col = bounds.col_start;
    let palette = current_palette_for_surface(ShellSurfaceKind::Terminal);
    let decoration = active_terminal_decoration();
    for (idx, line) in header_lines.iter().enumerate() {
        screen.centered_text(painter, header_start_row + idx, line, palette.fg, true);
    }
    screen.themed_separator(painter, separator_top_row, &palette, &decoration);
    screen.themed_title(painter, title_row, "Open Documents", &palette, &decoration);
    screen.themed_separator(painter, separator_bottom_row, &palette, &decoration);
    screen.themed_subtitle(
        painter,
        content_col,
        subtitle_row,
        &file_manager.cwd.display().to_string(),
        &palette,
        &decoration,
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
        let text = terminal_menu_row_text(&row_data.label, selected, 2);
        let row = menu_start_row + (data_idx - scroll_offset);
        let response = screen.selectable_row(
            ui,
            painter,
            &palette,
            content_col,
            row,
            &text,
            selected,
        );
        if input_enabled && response.clicked() {
            *selected_idx = data_idx;
            event = DocumentBrowserEvent::Activate;
        }
    }
    screen.text(
        painter,
        content_col,
        status_row,
        "Enter open | O open-with | Tab back | Q quit | Up/Down | F1 menu",
        palette.dim,
    );
    if !shell_status.is_empty() {
        screen.text(
            painter,
            content_col,
            status_row_alt,
            shell_status,
            palette.dim,
        );
    }

    event
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
    bounds: &ContentBounds,
    input_enabled: bool,
    header_lines: &[String],
) -> DocumentBrowserEvent {
    let mut event = DocumentBrowserEvent::None;
    egui::CentralPanel::default()
        .frame(
            egui::Frame::none()
                .fill(current_palette_for_surface(ShellSurfaceKind::Terminal).bg)
                .inner_margin(0.0),
        )
        .show(ctx, |ui| {
            let palette = current_palette_for_surface(ShellSurfaceKind::Terminal);
            let (screen, _) = RetroScreen::new(ui, cols, rows);
            let painter = ui.painter_at(screen.rect);
            screen.paint_terminal_background(&painter, &palette);
            event = paint_terminal_document_browser(
                ui,
                &screen,
                &painter,
                file_manager,
                selected_idx,
                shell_status,
                header_start_row,
                separator_top_row,
                title_row,
                separator_bottom_row,
                subtitle_row,
                menu_start_row,
                status_row,
                status_row_alt,
                bounds,
                input_enabled,
                header_lines,
            );
        });
    event
}
