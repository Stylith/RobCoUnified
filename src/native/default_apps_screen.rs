use super::desktop_default_apps_service::{default_app_slot_label, DefaultAppSlot};
use super::menu::draw_terminal_menu_screen;
use crate::config::Settings;
use eframe::egui::Context;
pub use robcos_native_default_apps_app::TerminalDefaultAppsRequest;
use robcos_native_default_apps_app::{
    build_default_app_choice_items, build_default_apps_root_items,
    resolve_default_apps_choice_event, resolve_default_apps_root_event,
    resolve_terminal_default_apps_request,
};

#[allow(clippy::too_many_arguments)]
pub fn draw_default_apps_screen(
    ctx: &Context,
    draft: &Settings,
    root_idx: &mut usize,
    choice_idx: &mut usize,
    active_slot: &mut Option<DefaultAppSlot>,
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
) -> TerminalDefaultAppsRequest {
    match active_slot {
        Some(slot) => {
            let items = build_default_app_choice_items(*slot);
            let activated = draw_terminal_menu_screen(
                ctx,
                &format!("Default App: {}", default_app_slot_label(*slot)),
                None,
                &items,
                choice_idx,
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
            resolve_terminal_default_apps_request(resolve_default_apps_choice_event(
                *slot, activated,
            ))
        }
        None => {
            let items = build_default_apps_root_items(draft);
            let activated = draw_terminal_menu_screen(
                ctx,
                "Default Apps",
                Some("Set default apps for your files."),
                &items,
                root_idx,
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
            resolve_terminal_default_apps_request(resolve_default_apps_root_event(activated))
        }
    }
}
