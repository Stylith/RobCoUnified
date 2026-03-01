use super::retro_ui::{current_palette, RetroScreen};
use crate::core::hacking::{
    find_bracket_at, find_word_at, HackingGame, SelectOutcome, COLS, COL_WIDTH, ROWS,
};
use eframe::egui::{self, Align2, Context, Pos2};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HackingScreenEvent {
    None,
    Cancel,
    Success,
    LockedOut,
    ExitLocked,
}

#[allow(clippy::too_many_arguments)]
pub fn draw_hacking_screen(
    ctx: &Context,
    game: &mut HackingGame,
    cols: usize,
    screen_rows: usize,
    status_row: usize,
    _footer_row: usize,
) -> HackingScreenEvent {
    if ctx.input(|i| i.key_pressed(egui::Key::ArrowRight)) {
        game.move_right();
    }
    if ctx.input(|i| i.key_pressed(egui::Key::ArrowLeft)) {
        game.move_left();
    }
    if ctx.input(|i| i.key_pressed(egui::Key::ArrowDown)) {
        game.move_down();
    }
    if ctx.input(|i| i.key_pressed(egui::Key::ArrowUp)) {
        game.move_up();
    }
    if ctx.input(|i| i.key_pressed(egui::Key::Escape) || i.key_pressed(egui::Key::Tab)) {
        return HackingScreenEvent::Cancel;
    }
    if ctx.input(|i| i.key_pressed(egui::Key::Enter)) {
        return match game.select() {
            SelectOutcome::Success => HackingScreenEvent::Success,
            SelectOutcome::LockedOut => HackingScreenEvent::LockedOut,
            _ => HackingScreenEvent::None,
        };
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

            screen.centered_text(
                &painter,
                0,
                "ROBCO INDUSTRIES (TM) TERMLINK PROTOCOL",
                palette.fg,
                true,
            );

            let boxes = format!(
                "{}{}",
                "■ ".repeat(game.attempts),
                "□ ".repeat(game.profile.max_tries.saturating_sub(game.attempts))
            );
            let warn = if game.attempts <= 1 {
                format!("!!! WARNING: LOCKOUT IMMINENT !!!  {}", boxes.trim())
            } else {
                format!("{} ATTEMPT(S) LEFT:  {}", game.attempts, boxes.trim())
            };
            screen.text(&painter, 3, 2, &warn, palette.fg);

            let hover_word = find_word_at(game.cursor, &game.grid.word_positions);
            let hover_bracket = find_bracket_at(game.cursor, &game.grid.bracket_pairs);
            let body_top = 5usize;
            let log_x = 50usize;
            let font = screen.font().clone();
            let row_cell = screen.row_rect(0, body_top, 1);
            let row_height = row_cell.height();
            let text_inset = ((row_height - font.size).max(0.0) * 0.5).floor();
            let glyph_advance = painter
                .layout_no_wrap("W".to_string(), font.clone(), palette.fg)
                .size()
                .x
                .max(1.0);
            let addr_width = painter
                .layout_no_wrap("0xF000".to_string(), font.clone(), palette.dim)
                .size()
                .x
                .max(1.0);
            let addr_gap = (row_cell.width() * 0.70).max(4.0);
            let mut removed = vec![false; game.grid.chars.len()];
            for wp in &game.grid.word_positions {
                if game.removed_duds.contains(&wp.word) {
                    let len = wp.word.chars().count();
                    for idx in wp.start..wp.start + len {
                        if let Some(cell) = removed.get_mut(idx) {
                            *cell = true;
                        }
                    }
                }
            }

            for col_block in 0..COLS {
                let addr_col = 3 + col_block * (COL_WIDTH + 14);
                let addr_x = screen.row_rect(addr_col, body_top, 1).left();
                let chars_x = addr_x + addr_width + addr_gap;
                for row in 0..ROWS {
                    let sy = body_top + row;
                    let row_rect = screen.row_rect(addr_col, sy, 1);
                    let text_y = row_rect.top() + text_inset;
                    let addr = game.base_addr + ((col_block * ROWS + row) * COL_WIDTH) as u16;
                    painter.text(
                        Pos2::new(addr_x, text_y),
                        Align2::LEFT_TOP,
                        format!("0x{addr:04X}"),
                        font.clone(),
                        palette.dim,
                    );

                    let row_start = col_block * ROWS * COL_WIDTH + row * COL_WIDTH;
                    for offset in 0..COL_WIDTH {
                        let idx = row_start + offset;
                        let display = if removed[idx] {
                            '.'
                        } else {
                            game.grid.chars[idx]
                        };

                        let highlighted = hover_word.is_some_and(|hw| {
                            idx >= hw.start && idx < hw.start + hw.word.chars().count()
                        }) || hover_bracket
                            .is_some_and(|hb| idx >= hb.open && idx <= hb.close)
                            || idx == game.cursor;

                        let char_x = (chars_x + offset as f32 * glyph_advance).floor();
                        if highlighted {
                            let width = glyph_advance.max(1.0).ceil();
                            let rect = screen.text_band_rect(sy, char_x, width);
                            painter.rect_filled(rect, 0.0, palette.selected_bg);
                            painter.text(
                                Pos2::new(char_x, text_y),
                                Align2::LEFT_TOP,
                                display.to_string(),
                                font.clone(),
                                palette.selected_fg,
                            );
                        } else {
                            painter.text(
                                Pos2::new(char_x, text_y),
                                Align2::LEFT_TOP,
                                display.to_string(),
                                font.clone(),
                                palette.fg,
                            );
                        }
                    }
                }
            }

            for (li, entry) in game.log.iter().rev().take(ROWS).enumerate() {
                let sy = body_top + (ROWS - 1 - li);
                screen.text(&painter, log_x, sy, entry, palette.fg);
            }

            if status_row > 0 {
                let row = status_row.saturating_sub(1);
                let clear = screen.row_rect(0, row, cols);
                painter.rect_filled(clear, 0.0, palette.bg);
            }
        });

    HackingScreenEvent::None
}

#[allow(clippy::too_many_arguments)]
pub fn draw_locked_screen(
    ctx: &Context,
    cols: usize,
    screen_rows: usize,
    _footer_row: usize,
) -> HackingScreenEvent {
    if ctx.input(|i| i.key_pressed(egui::Key::Enter) || i.key_pressed(egui::Key::Space)) {
        return HackingScreenEvent::ExitLocked;
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
            screen.centered_text(&painter, 14, "TERMINAL LOCKED", palette.fg, true);
            screen.centered_text(
                &painter,
                16,
                "PLEASE CONTACT AN ADMINISTRATOR",
                palette.fg,
                false,
            );
            screen.centered_text(&painter, 30, "[ Press ENTER to Exit ]", palette.dim, false);
        });

    HackingScreenEvent::None
}
