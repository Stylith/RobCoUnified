use super::retro_ui::{current_palette, RetroScreen};
use crate::core::hacking::{
    find_bracket_at, find_word_at, idx_to_cell, HackingGame, SelectOutcome, COLS, COL_WIDTH, ROWS,
};
use eframe::egui::{self, Context};

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

            for col_block in 0..COLS {
                for row in 0..ROWS {
                    let addr = game.base_addr + ((col_block * ROWS + row) * COL_WIDTH) as u16;
                    let sx = 3 + col_block * (COL_WIDTH + 14);
                    let sy = body_top + row;
                    screen.text(&painter, sx, sy, &format!("0x{addr:04X}"), palette.fg);
                }
            }

            for (i, &ch) in game.grid.chars.iter().enumerate() {
                let (row_off, col_off) = idx_to_cell(i);
                let sy = body_top + row_off;
                let sx = 3 + col_off;
                let is_removed = game.grid.word_positions.iter().any(|wp| {
                    game.removed_duds.contains(&wp.word)
                        && i >= wp.start
                        && i < wp.start + wp.word.chars().count()
                });
                let highlighted = hover_word
                    .is_some_and(|hw| i >= hw.start && i < hw.start + hw.word.chars().count())
                    || hover_bracket.is_some_and(|hb| i >= hb.open && i <= hb.close)
                    || i == game.cursor;

                let display = if is_removed { '.' } else { ch };
                if highlighted {
                    let rect = screen.row_rect(sx, sy, 1);
                    painter.rect_filled(rect, 0.0, palette.selected_bg);
                    screen.text(&painter, sx, sy, &display.to_string(), palette.selected_fg);
                } else {
                    screen.text(&painter, sx, sy, &display.to_string(), palette.fg);
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
