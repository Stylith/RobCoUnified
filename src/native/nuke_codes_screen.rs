use super::retro_ui::{current_palette, RetroScreen};
use crate::config::HEADER_LINES;
use eframe::egui::{self, Context, Rect};
pub use robcos_native_nuke_codes_app::{fetch_nuke_codes, NukeCodesEvent, NukeCodesView};
use robcos_native_nuke_codes_app::resolve_nuke_codes_event;

#[allow(clippy::too_many_arguments)]
pub fn draw_nuke_codes_screen(
    ctx: &Context,
    state: &NukeCodesView,
    cols: usize,
    rows: usize,
    header_start_row: usize,
    separator_top_row: usize,
    title_row: usize,
    separator_bottom_row: usize,
    menu_start_row: usize,
    status_row: usize,
    content_col: usize,
) -> NukeCodesEvent {
    let refresh = ctx.input(|i| i.key_pressed(egui::Key::R));
    let back = ctx.input(|i| {
        i.key_pressed(egui::Key::Q)
            || i.key_pressed(egui::Key::Escape)
            || i.key_pressed(egui::Key::Tab)
    });

    let event = resolve_nuke_codes_event(refresh, back);

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
                "NUCLEAR LAUNCH CODES",
                palette.fg,
                true,
            );
            screen.separator(&painter, separator_bottom_row, &palette);

            match state {
                NukeCodesView::Data(codes) => {
                    let block_w = 21usize;
                    let top = screen.row_rect(content_col, menu_start_row, block_w);
                    let bottom = screen.row_rect(content_col, menu_start_row + 2, block_w);
                    let block = Rect::from_min_max(top.min, egui::pos2(bottom.max.x, bottom.max.y));
                    painter.rect_filled(block, 0.0, palette.panel);

                    screen.text(
                        &painter,
                        content_col + 1,
                        menu_start_row,
                        &format!("ALPHA   : {}", codes.alpha),
                        palette.fg,
                    );
                    screen.text(
                        &painter,
                        content_col + 1,
                        menu_start_row + 1,
                        &format!("BRAVO   : {}", codes.bravo),
                        palette.fg,
                    );
                    screen.text(
                        &painter,
                        content_col + 1,
                        menu_start_row + 2,
                        &format!("CHARLIE : {}", codes.charlie),
                        palette.fg,
                    );
                    screen.text(
                        &painter,
                        content_col + 1,
                        menu_start_row + 4,
                        &format!("SOURCE      : {}", codes.source),
                        palette.fg,
                    );
                    screen.text(
                        &painter,
                        content_col + 1,
                        menu_start_row + 5,
                        &format!("FETCHED AT  : {}", codes.fetched_at),
                        palette.fg,
                    );
                }
                NukeCodesView::Error(err) => {
                    screen.text(
                        &painter,
                        content_col + 1,
                        menu_start_row,
                        "UNABLE TO FETCH LIVE CODES",
                        palette.fg,
                    );
                    screen.text(
                        &painter,
                        content_col + 1,
                        menu_start_row + 2,
                        &format!("ERROR: {err}"),
                        palette.dim,
                    );
                }
                NukeCodesView::Unloaded => {
                    screen.text(
                        &painter,
                        content_col + 1,
                        menu_start_row,
                        "Loading launch codes...",
                        palette.fg,
                    );
                }
            }

            screen.text(
                &painter,
                content_col,
                status_row.saturating_sub(3),
                "Press R to refresh. Q / Esc / Tab = Back",
                palette.dim,
            );
        });

    event
}
