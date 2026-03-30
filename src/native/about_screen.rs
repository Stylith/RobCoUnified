use super::retro_ui::{
    active_terminal_decoration, current_palette_for_surface, ContentBounds, RetroScreen,
    ShellSurfaceKind,
};
use eframe::egui::{self, Context, Painter, Ui};
pub use nucleon_native_about_app::TerminalAboutRequest;
use nucleon_native_about_app::{about_ascii_and_fields, get_system_info, resolve_about_request};
use std::time::Duration;

#[allow(clippy::too_many_arguments)]
pub fn paint_about_screen(
    ui: &mut Ui,
    screen: &RetroScreen,
    painter: &Painter,
    header_start_row: usize,
    separator_top_row: usize,
    title_row: usize,
    separator_bottom_row: usize,
    subtitle_row: usize,
    menu_start_row: usize,
    status_row: usize,
    bounds: &ContentBounds,
    header_lines: &[String],
) -> TerminalAboutRequest {
    let ctx = ui.ctx();
    let (ascii, fields) = about_ascii_and_fields();
    let info = get_system_info(&fields);

    let back = ctx.input(|i| i.key_pressed(egui::Key::Escape) || i.key_pressed(egui::Key::Tab))
        || ctx.input(|i| i.key_pressed(egui::Key::Q));
    if !back {
        ctx.request_repaint_after(Duration::from_millis(1000));
    }

    let content_col = bounds.col_start;
    let palette = current_palette_for_surface(ShellSurfaceKind::Terminal);
    let decoration = active_terminal_decoration();
    for (idx, line) in header_lines.iter().enumerate() {
        screen.centered_text(painter, header_start_row + idx, line, palette.fg, true);
    }
    screen.themed_separator(painter, separator_top_row, &palette, &decoration);
    screen.themed_title(painter, title_row, "About", &palette, &decoration);
    screen.themed_separator(painter, separator_bottom_row, &palette, &decoration);

    let mut row = subtitle_row;
    for line in ascii {
        screen.centered_text(painter, row, &line, palette.fg, false);
        row += 1;
    }
    row = row.max(menu_start_row);
    for (key, value) in info {
        screen.text(
            painter,
            content_col,
            row,
            &format!("{key}: {value}"),
            palette.fg,
        );
        row += 1;
    }
    screen.text(
        painter,
        content_col,
        status_row,
        "q/Esc/Tab = back",
        palette.dim,
    );

    resolve_about_request(back)
}

#[allow(clippy::too_many_arguments)]
pub fn draw_about_screen(
    ctx: &Context,
    cols: usize,
    rows: usize,
    header_start_row: usize,
    separator_top_row: usize,
    title_row: usize,
    separator_bottom_row: usize,
    subtitle_row: usize,
    menu_start_row: usize,
    status_row: usize,
    bounds: &ContentBounds,
    header_lines: &[String],
) -> TerminalAboutRequest {
    let mut request = TerminalAboutRequest::None;
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
            request = paint_about_screen(
                ui,
                &screen,
                &painter,
                header_start_row,
                separator_top_row,
                title_row,
                separator_bottom_row,
                subtitle_row,
                menu_start_row,
                status_row,
                bounds,
                header_lines,
            );
        });
    request
}
