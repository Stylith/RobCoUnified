use super::menu::{
    entry_for_selectable_idx, selectable_menu_count, LoginMenuRow, MainMenuAction,
    MAIN_MENU_ENTRIES,
};
use super::prompt::{draw_terminal_prompt_overlay, TerminalPrompt};
use super::retro_ui::{
    active_terminal_decoration, current_palette_for_surface, terminal_menu_row_text,
    ContentBounds, RetroScreen, ShellSurfaceKind,
};
use eframe::egui::{self, Color32, Context, Painter, Ui};

#[allow(clippy::too_many_arguments)]
pub fn paint_login_screen(
    ui: &mut Ui,
    screen: &RetroScreen,
    painter: &Painter,
    rows: &[LoginMenuRow],
    selected_idx: &mut usize,
    error: &str,
    prompt: Option<&TerminalPrompt>,
    header_start_row: usize,
    separator_top_row: usize,
    title_row: usize,
    separator_bottom_row: usize,
    subtitle_row: usize,
    menu_start_row: usize,
    status_row: usize,
    bounds: &ContentBounds,
    header_lines: &[String],
) -> bool {
    let ctx = ui.ctx();
    let selectable_count = rows
        .iter()
        .filter(|row| matches!(row, LoginMenuRow::User(_) | LoginMenuRow::Exit))
        .count();
    if selectable_count > 0 {
        *selected_idx = (*selected_idx).min(selectable_count - 1);
    } else {
        *selected_idx = 0;
    }

    if prompt.is_none() {
        if ctx.input(|i| i.key_pressed(egui::Key::ArrowUp)) {
            let prev = *selected_idx;
            *selected_idx = selected_idx.saturating_sub(1);
            if *selected_idx != prev {
                crate::sound::play_navigate();
            }
        }
        if ctx.input(|i| i.key_pressed(egui::Key::ArrowDown)) {
            let prev = *selected_idx;
            *selected_idx = (*selected_idx + 1).min(selectable_count.saturating_sub(1));
            if *selected_idx != prev {
                crate::sound::play_navigate();
            }
        }
    }

    let enter_pressed = prompt.is_none()
        && ctx.input(|i| i.key_pressed(egui::Key::Enter) || i.key_pressed(egui::Key::Space));
    let mut activated = enter_pressed;

    let content_col = bounds.col_start;
    let palette = current_palette_for_surface(ShellSurfaceKind::Terminal);
    let decoration = active_terminal_decoration();
    for (idx, line) in header_lines.iter().enumerate() {
        screen.centered_text(painter, header_start_row + idx, line, palette.fg, true);
    }
    screen.themed_separator(painter, separator_top_row, &palette, &decoration);
    screen.themed_title(
        painter,
        title_row,
        "NUCLEON TERMLINK - Select User",
        &palette,
        &decoration,
    );
    screen.themed_separator(painter, separator_bottom_row, &palette, &decoration);
    screen.text(
        painter,
        content_col,
        subtitle_row,
        "Welcome. Please select a user.",
        palette.fg,
    );
    if !error.is_empty() {
        screen.text(painter, content_col, status_row, error, Color32::LIGHT_RED);
    }

    let mut row = menu_start_row;
    let mut selectable_idx = 0usize;
    for entry in rows {
        match entry {
            LoginMenuRow::Separator => {
                screen.text(painter, content_col + 4, row, "---", palette.dim);
            }
            LoginMenuRow::User(user) => {
                let selected = selectable_idx == *selected_idx;
                let text = terminal_menu_row_text(user, selected, 2);
                let response = screen.selectable_row(
                    ui,
                    painter,
                    &palette,
                    content_col,
                    row,
                    &text,
                    selected,
                );
                if !enter_pressed && !activated && response.clicked() {
                    *selected_idx = selectable_idx;
                    activated = true;
                }
                selectable_idx += 1;
            }
            LoginMenuRow::Exit => {
                let selected = selectable_idx == *selected_idx;
                let text = terminal_menu_row_text("Exit", selected, 2);
                let response = screen.selectable_row(
                    ui,
                    painter,
                    &palette,
                    content_col,
                    row,
                    &text,
                    selected,
                );
                if !enter_pressed && !activated && response.clicked() {
                    *selected_idx = selectable_idx;
                    activated = true;
                }
                selectable_idx += 1;
            }
        }
        row += 1;
    }

    if let Some(prompt) = prompt {
        draw_terminal_prompt_overlay(ui, screen, prompt);
    }

    activated
}

#[allow(clippy::too_many_arguments)]
pub fn paint_main_menu_screen(
    ui: &mut Ui,
    screen: &RetroScreen,
    painter: &Painter,
    selected_idx: &mut usize,
    shell_status: &str,
    version: &str,
    header_start_row: usize,
    separator_top_row: usize,
    title_row: usize,
    separator_bottom_row: usize,
    subtitle_row: usize,
    menu_start_row: usize,
    status_row: usize,
    bounds: &ContentBounds,
    header_lines: &[String],
) -> Option<MainMenuAction> {
    let ctx = ui.ctx();
    if ctx.input(|i| i.key_pressed(egui::Key::ArrowUp)) {
        let prev = *selected_idx;
        *selected_idx = selected_idx.saturating_sub(1);
        if *selected_idx != prev {
            crate::sound::play_navigate();
        }
    }
    if ctx.input(|i| i.key_pressed(egui::Key::ArrowDown)) {
        let prev = *selected_idx;
        *selected_idx = (*selected_idx + 1).min(selectable_menu_count() - 1);
        if *selected_idx != prev {
            crate::sound::play_navigate();
        }
    }

    let enter_pressed =
        ctx.input(|i| i.key_pressed(egui::Key::Enter) || i.key_pressed(egui::Key::Space));
    let mut activated = None;
    if enter_pressed {
        activated = entry_for_selectable_idx(*selected_idx).action;
    }

    let content_col = bounds.col_start;
    let palette = current_palette_for_surface(ShellSurfaceKind::Terminal);
    let decoration = active_terminal_decoration();
    for (idx, line) in header_lines.iter().enumerate() {
        screen.centered_text(painter, header_start_row + idx, line, palette.fg, true);
    }
    screen.themed_separator(painter, separator_top_row, &palette, &decoration);
    screen.themed_title(painter, title_row, "Main Menu", &palette, &decoration);
    screen.themed_separator(painter, separator_bottom_row, &palette, &decoration);
    screen.themed_subtitle(
        painter,
        content_col,
        subtitle_row,
        version,
        &palette,
        &decoration,
    );

    let mut visible_row = menu_start_row;
    let mut selectable_idx = 0usize;
    for entry in MAIN_MENU_ENTRIES {
        if entry.action.is_none() {
            screen.text(painter, content_col + 4, visible_row, entry.label, palette.dim);
            visible_row += 1;
            continue;
        }
        let selected = selectable_idx == *selected_idx;
        let text = terminal_menu_row_text(entry.label, selected, 2);
        let response = screen.selectable_row(
            ui,
            painter,
            &palette,
            content_col,
            visible_row,
            &text,
            selected,
        );
        if !enter_pressed && activated.is_none() && response.clicked() {
            *selected_idx = selectable_idx;
            activated = entry.action;
        }
        visible_row += 1;
        selectable_idx += 1;
    }

    if !shell_status.is_empty() {
        screen.text(painter, content_col, status_row, shell_status, palette.dim);
    }

    activated
}

#[allow(clippy::too_many_arguments)]
pub fn draw_login_screen(
    ctx: &Context,
    rows: &[LoginMenuRow],
    selected_idx: &mut usize,
    error: &str,
    prompt: Option<&TerminalPrompt>,
    cols: usize,
    screen_rows: usize,
    header_start_row: usize,
    separator_top_row: usize,
    title_row: usize,
    separator_bottom_row: usize,
    subtitle_row: usize,
    menu_start_row: usize,
    status_row: usize,
    bounds: &ContentBounds,
    header_lines: &[String],
) -> bool {
    let mut activated = false;
    egui::CentralPanel::default()
        .frame(
            egui::Frame::none()
                .fill(current_palette_for_surface(ShellSurfaceKind::Terminal).bg)
                .inner_margin(0.0),
        )
        .show(ctx, |ui| {
            let palette = current_palette_for_surface(ShellSurfaceKind::Terminal);
            let (screen, _) = RetroScreen::new(ui, cols, screen_rows);
            let painter = ui.painter_at(screen.rect);
            screen.paint_terminal_background(&painter, &palette);
            activated = paint_login_screen(
                ui,
                &screen,
                &painter,
                rows,
                selected_idx,
                error,
                prompt,
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
    activated
}

#[allow(clippy::too_many_arguments)]
pub fn draw_main_menu_screen(
    ctx: &Context,
    selected_idx: &mut usize,
    shell_status: &str,
    version: &str,
    cols: usize,
    screen_rows: usize,
    header_start_row: usize,
    separator_top_row: usize,
    title_row: usize,
    separator_bottom_row: usize,
    subtitle_row: usize,
    menu_start_row: usize,
    status_row: usize,
    bounds: &ContentBounds,
    header_lines: &[String],
) -> Option<MainMenuAction> {
    let mut activated = None;
    egui::CentralPanel::default()
        .frame(
            egui::Frame::none()
                .fill(current_palette_for_surface(ShellSurfaceKind::Terminal).bg)
                .inner_margin(0.0),
        )
        .show(ctx, |ui| {
            let palette = current_palette_for_surface(ShellSurfaceKind::Terminal);
            let (screen, _) = RetroScreen::new(ui, cols, screen_rows);
            let painter = ui.painter_at(screen.rect);
            screen.paint_terminal_background(&painter, &palette);
            activated = paint_main_menu_screen(
                ui,
                &screen,
                &painter,
                selected_idx,
                shell_status,
                version,
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
    activated
}
