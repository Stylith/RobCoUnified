use super::menu::draw_terminal_menu_screen;
use crate::launcher::json_to_cmd;
use serde_json::{Map, Value};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProgramMenuEvent {
    None,
    Back,
    Launch(String),
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
    content_col: usize,
) -> ProgramMenuEvent {
    let mut items = entries.to_vec();
    items.push("---".to_string());
    items.push("Back".to_string());
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
        content_col,
        shell_status,
    );
    match activated {
        Some(idx) if idx < entries.len() => ProgramMenuEvent::Launch(entries[idx].clone()),
        Some(_) => ProgramMenuEvent::Back,
        None => ProgramMenuEvent::None,
    }
}

pub fn resolve_program_command(
    name: &str,
    source: &Map<String, Value>,
) -> Result<Vec<String>, String> {
    let Some(value) = source.get(name) else {
        return Err(format!("Unknown program '{name}'."));
    };
    let argv = json_to_cmd(value);
    if argv.first().is_none() {
        return Err("Error: empty command.".to_string());
    }
    Ok(argv)
}
