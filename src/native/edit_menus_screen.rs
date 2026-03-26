use super::menu::draw_terminal_menu_screen;
use eframe::egui::Context;
pub use robcos_native_edit_menus_app::{
    apply_edit_menus_activation, apply_edit_menus_selected_idx, build_edit_menus_view_model,
    EditMenuTarget, EditMenusEntries, TerminalEditMenusRequest, TerminalEditMenusState,
};

#[allow(clippy::too_many_arguments)]
pub fn draw_edit_menus_screen(
    ctx: &Context,
    state: &mut TerminalEditMenusState,
    entries: EditMenusEntries<'_>,
    text_editor_visible: bool,
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
) -> TerminalEditMenusRequest {
    let model = build_edit_menus_view_model(state, entries, text_editor_visible);
    let mut selected_idx = model.selected_idx;
    let activated = draw_terminal_menu_screen(
        ctx,
        model.title,
        model.subtitle.as_deref(),
        &model.items,
        &mut selected_idx,
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
    apply_edit_menus_selected_idx(state, selected_idx);
    apply_edit_menus_activation(state, entries, activated)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn target_labels_match_expected() {
        assert_eq!(EditMenuTarget::Applications.title(), "Applications");
        assert_eq!(EditMenuTarget::Applications.singular(), "App");
        assert_eq!(EditMenuTarget::Documents.title(), "Documents");
        assert_eq!(EditMenuTarget::Documents.singular(), "Category");
        assert_eq!(EditMenuTarget::Network.title(), "Network");
        assert_eq!(EditMenuTarget::Network.singular(), "Network Program");
        assert_eq!(EditMenuTarget::Games.title(), "Games");
        assert_eq!(EditMenuTarget::Games.singular(), "Game");
    }

    #[test]
    fn applications_view_contains_delete_row() {
        let mut state = TerminalEditMenusState::default();
        let _ = apply_edit_menus_activation(
            &mut state,
            EditMenusEntries {
                applications: &[],
                documents: &[],
                network: &[],
                games: &[],
            },
            Some(0),
        );
        let model = build_edit_menus_view_model(
            &state,
            EditMenusEntries {
                applications: &[],
                documents: &[],
                network: &[],
                games: &[],
            },
            true,
        );
        assert!(model.items.iter().any(|item| item == "Delete App"));
    }

    #[test]
    fn root_view_contains_edit_documents_row() {
        let state = TerminalEditMenusState::default();
        let model = build_edit_menus_view_model(
            &state,
            EditMenusEntries {
                applications: &[],
                documents: &[],
                network: &[],
                games: &[],
            },
            true,
        );
        assert!(model.items.iter().any(|item| item == "Edit Documents"));
    }
}
