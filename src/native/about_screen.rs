use super::retro_ui::{current_palette, RetroScreen};
use eframe::egui::{self, Context};
pub use robcos_native_about_app::TerminalAboutRequest;
use robcos_native_about_app::{about_ascii_and_fields, get_system_info, resolve_about_request};
use std::time::Duration;

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
    content_col: usize,
) -> TerminalAboutRequest {
    let (ascii, fields) = about_ascii_and_fields();
    let info = get_system_info(&fields);

    let back = ctx.input(|i| i.key_pressed(egui::Key::Escape) || i.key_pressed(egui::Key::Tab))
        || ctx.input(|i| i.key_pressed(egui::Key::Q));
    if !back {
        ctx.request_repaint_after(Duration::from_millis(1000));
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
            for (idx, line) in crate::config::HEADER_LINES.iter().enumerate() {
                screen.centered_text(&painter, header_start_row + idx, line, palette.fg, true);
            }
            screen.separator(&painter, separator_top_row, &palette);
            screen.centered_text(&painter, title_row, "About", palette.fg, true);
            screen.separator(&painter, separator_bottom_row, &palette);

            let mut row = subtitle_row;
            for line in ascii {
                screen.centered_text(&painter, row, &line, palette.fg, false);
                row += 1;
            }
            row = row.max(menu_start_row);
            for (key, value) in info {
                screen.text(
                    &painter,
                    content_col,
                    row,
                    &format!("{key}: {value}"),
                    palette.fg,
                );
                row += 1;
            }
            screen.text(
                &painter,
                content_col,
                status_row,
                "q/Esc/Tab = back",
                palette.dim,
            );
        });

    resolve_about_request(back)
}
