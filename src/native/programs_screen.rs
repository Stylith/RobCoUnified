use super::menu::{draw_terminal_menu_screen, paint_terminal_menu_screen};
use super::retro_ui::{ContentBounds, RetroScreen};
use eframe::egui::{Painter, Ui};
pub use nucleon_native_programs_app::ProgramMenuEvent;
use nucleon_native_programs_app::{build_program_menu_items, resolve_program_menu_event};

#[allow(clippy::too_many_arguments)]
pub fn paint_programs_menu(
    ui: &mut Ui,
    screen: &RetroScreen,
    painter: &Painter,
    title: &str,
    subtitle: Option<&str>,
    entries: &[String],
    selected_idx: &mut usize,
    shell_status: &str,
    header_start_row: usize,
    separator_top_row: usize,
    title_row: usize,
    separator_bottom_row: usize,
    subtitle_row: usize,
    menu_start_row: usize,
    status_row: usize,
    bounds: &ContentBounds,
    header_lines: &[String],
) -> ProgramMenuEvent {
    let items = build_program_menu_items(entries);
    let activated = paint_terminal_menu_screen(
        ui,
        screen,
        painter,
        title,
        subtitle,
        &items,
        selected_idx,
        header_start_row,
        separator_top_row,
        title_row,
        separator_bottom_row,
        subtitle_row,
        menu_start_row,
        status_row,
        bounds,
        shell_status,
        header_lines,
    );
    resolve_program_menu_event(entries, activated)
}

#[allow(clippy::too_many_arguments)]
pub fn draw_programs_menu(
    ctx: &eframe::egui::Context,
    title: &str,
    subtitle: Option<&str>,
    entries: &[String],
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
    bounds: &ContentBounds,
    header_lines: &[String],
) -> ProgramMenuEvent {
    let items = build_program_menu_items(entries);
    let activated = draw_terminal_menu_screen(
        ctx,
        title,
        subtitle,
        &items,
        selected_idx,
        cols,
        rows,
        header_start_row,
        separator_top_row,
        title_row,
        separator_bottom_row,
        subtitle_row,
        menu_start_row,
        status_row,
        bounds,
        shell_status,
        header_lines,
    );
    resolve_program_menu_event(entries, activated)
}
