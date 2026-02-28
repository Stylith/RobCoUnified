use super::menu::draw_terminal_menu_screen;
use eframe::egui::Context;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditMenuTarget {
    Applications,
    Documents,
    Network,
    Games,
}

impl EditMenuTarget {
    pub fn title(self) -> &'static str {
        match self {
            EditMenuTarget::Applications => "Applications",
            EditMenuTarget::Documents => "Documents",
            EditMenuTarget::Network => "Network",
            EditMenuTarget::Games => "Games",
        }
    }

    pub fn singular(self) -> &'static str {
        match self {
            EditMenuTarget::Applications => "App",
            EditMenuTarget::Documents => "Category",
            EditMenuTarget::Network => "Network Program",
            EditMenuTarget::Games => "Game",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EditMenusView {
    Root,
    Applications,
    Documents,
    Network,
    Games,
    DeleteApplications,
    DeleteDocuments,
    DeleteNetwork,
    DeleteGames,
}

#[derive(Debug, Clone)]
pub struct TerminalEditMenusState {
    view: EditMenusView,
    root_idx: usize,
    applications_idx: usize,
    documents_idx: usize,
    network_idx: usize,
    games_idx: usize,
    delete_applications_idx: usize,
    delete_documents_idx: usize,
    delete_network_idx: usize,
    delete_games_idx: usize,
}

impl Default for TerminalEditMenusState {
    fn default() -> Self {
        Self {
            view: EditMenusView::Root,
            root_idx: 0,
            applications_idx: 0,
            documents_idx: 0,
            network_idx: 0,
            games_idx: 0,
            delete_applications_idx: 0,
            delete_documents_idx: 0,
            delete_network_idx: 0,
            delete_games_idx: 0,
        }
    }
}

impl TerminalEditMenusState {
    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

#[derive(Clone, Copy)]
pub struct EditMenusEntries<'a> {
    pub applications: &'a [String],
    pub documents: &'a [String],
    pub network: &'a [String],
    pub games: &'a [String],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditMenusEvent {
    None,
    BackToSettings,
    ToggleBuiltinNukeCodes,
    ToggleBuiltinTextEditor,
    PromptAddProgramName(EditMenuTarget),
    PromptAddCategoryName,
    ConfirmDeleteProgram {
        target: EditMenuTarget,
        name: String,
    },
    ConfirmDeleteCategory {
        name: String,
    },
    Status(String),
}

#[allow(clippy::too_many_arguments)]
pub fn draw_edit_menus_screen(
    ctx: &Context,
    state: &mut TerminalEditMenusState,
    entries: EditMenusEntries<'_>,
    nuke_codes_visible: bool,
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
) -> EditMenusEvent {
    let (title, subtitle, items, selected_idx) =
        view_payload(state, entries, nuke_codes_visible, text_editor_visible);
    let activated = draw_terminal_menu_screen(
        ctx,
        title,
        subtitle.as_deref(),
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
    let Some(idx) = activated else {
        return EditMenusEvent::None;
    };

    match state.view {
        EditMenusView::Root => match items[idx].as_str() {
            "Edit Applications" => {
                state.view = EditMenusView::Applications;
                EditMenusEvent::None
            }
            "Edit Documents" => {
                state.view = EditMenusView::Documents;
                EditMenusEvent::None
            }
            "Edit Network" => {
                state.view = EditMenusView::Network;
                EditMenusEvent::None
            }
            "Edit Games" => {
                state.view = EditMenusView::Games;
                EditMenusEvent::None
            }
            _ => EditMenusEvent::BackToSettings,
        },
        EditMenusView::Applications => match items[idx].as_str() {
            label if label.starts_with("Nuke Codes in Applications:") => {
                EditMenusEvent::ToggleBuiltinNukeCodes
            }
            label if label.starts_with("ROBCO Word Processor in Applications:") => {
                EditMenusEvent::ToggleBuiltinTextEditor
            }
            "Add App" => EditMenusEvent::PromptAddProgramName(EditMenuTarget::Applications),
            "Delete App" => {
                if entries.applications.is_empty() {
                    EditMenusEvent::Status("Error: App list is empty.".to_string())
                } else {
                    state.view = EditMenusView::DeleteApplications;
                    state.delete_applications_idx = 0;
                    EditMenusEvent::None
                }
            }
            _ => {
                state.view = EditMenusView::Root;
                EditMenusEvent::None
            }
        },
        EditMenusView::Documents => match items[idx].as_str() {
            "Add Category" => EditMenusEvent::PromptAddCategoryName,
            "Delete Category" => {
                if entries.documents.is_empty() {
                    EditMenusEvent::Status("Error: No categories to delete.".to_string())
                } else {
                    state.view = EditMenusView::DeleteDocuments;
                    state.delete_documents_idx = 0;
                    EditMenusEvent::None
                }
            }
            _ => {
                state.view = EditMenusView::Root;
                EditMenusEvent::None
            }
        },
        EditMenusView::Network => match items[idx].as_str() {
            "Add Network" => EditMenusEvent::PromptAddProgramName(EditMenuTarget::Network),
            "Delete Network" => {
                if entries.network.is_empty() {
                    EditMenusEvent::Status("Error: Network Program list is empty.".to_string())
                } else {
                    state.view = EditMenusView::DeleteNetwork;
                    state.delete_network_idx = 0;
                    EditMenusEvent::None
                }
            }
            _ => {
                state.view = EditMenusView::Root;
                EditMenusEvent::None
            }
        },
        EditMenusView::Games => match items[idx].as_str() {
            "Add Game" => EditMenusEvent::PromptAddProgramName(EditMenuTarget::Games),
            "Delete Game" => {
                if entries.games.is_empty() {
                    EditMenusEvent::Status("Error: Game list is empty.".to_string())
                } else {
                    state.view = EditMenusView::DeleteGames;
                    state.delete_games_idx = 0;
                    EditMenusEvent::None
                }
            }
            _ => {
                state.view = EditMenusView::Root;
                EditMenusEvent::None
            }
        },
        EditMenusView::DeleteApplications => {
            if idx < entries.applications.len() {
                EditMenusEvent::ConfirmDeleteProgram {
                    target: EditMenuTarget::Applications,
                    name: entries.applications[idx].clone(),
                }
            } else {
                state.view = EditMenusView::Applications;
                EditMenusEvent::None
            }
        }
        EditMenusView::DeleteDocuments => {
            if idx < entries.documents.len() {
                EditMenusEvent::ConfirmDeleteCategory {
                    name: entries.documents[idx].clone(),
                }
            } else {
                state.view = EditMenusView::Documents;
                EditMenusEvent::None
            }
        }
        EditMenusView::DeleteNetwork => {
            if idx < entries.network.len() {
                EditMenusEvent::ConfirmDeleteProgram {
                    target: EditMenuTarget::Network,
                    name: entries.network[idx].clone(),
                }
            } else {
                state.view = EditMenusView::Network;
                EditMenusEvent::None
            }
        }
        EditMenusView::DeleteGames => {
            if idx < entries.games.len() {
                EditMenusEvent::ConfirmDeleteProgram {
                    target: EditMenuTarget::Games,
                    name: entries.games[idx].clone(),
                }
            } else {
                state.view = EditMenusView::Games;
                EditMenusEvent::None
            }
        }
    }
}

fn view_payload<'a>(
    state: &'a mut TerminalEditMenusState,
    entries: EditMenusEntries<'_>,
    nuke_codes_visible: bool,
    text_editor_visible: bool,
) -> (&'static str, Option<String>, Vec<String>, &'a mut usize) {
    match state.view {
        EditMenusView::Root => (
            "Edit Menus",
            None,
            vec![
                "Edit Applications".to_string(),
                "Edit Documents".to_string(),
                "Edit Network".to_string(),
                "Edit Games".to_string(),
                "---".to_string(),
                "Back".to_string(),
            ],
            &mut state.root_idx,
        ),
        EditMenusView::Applications => (
            "Edit Applications",
            None,
            vec![
                format!(
                    "Nuke Codes in Applications: {} [toggle]",
                    if nuke_codes_visible {
                        "VISIBLE"
                    } else {
                        "HIDDEN"
                    }
                ),
                format!(
                    "ROBCO Word Processor in Applications: {} [toggle]",
                    if text_editor_visible {
                        "VISIBLE"
                    } else {
                        "HIDDEN"
                    }
                ),
                "---".to_string(),
                "Add App".to_string(),
                "Delete App".to_string(),
                "---".to_string(),
                "Back".to_string(),
            ],
            &mut state.applications_idx,
        ),
        EditMenusView::Documents => (
            "Edit Documents",
            None,
            vec![
                "Add Category".to_string(),
                "Delete Category".to_string(),
                "---".to_string(),
                "Back".to_string(),
            ],
            &mut state.documents_idx,
        ),
        EditMenusView::Network => (
            "Edit Network",
            None,
            vec![
                "Add Network".to_string(),
                "Delete Network".to_string(),
                "---".to_string(),
                "Back".to_string(),
            ],
            &mut state.network_idx,
        ),
        EditMenusView::Games => (
            "Edit Games",
            None,
            vec![
                "Add Game".to_string(),
                "Delete Game".to_string(),
                "---".to_string(),
                "Back".to_string(),
            ],
            &mut state.games_idx,
        ),
        EditMenusView::DeleteApplications => (
            "Delete App",
            None,
            delete_items(entries.applications),
            &mut state.delete_applications_idx,
        ),
        EditMenusView::DeleteDocuments => (
            "Delete Category",
            None,
            delete_items(entries.documents),
            &mut state.delete_documents_idx,
        ),
        EditMenusView::DeleteNetwork => (
            "Delete Network Program",
            None,
            delete_items(entries.network),
            &mut state.delete_network_idx,
        ),
        EditMenusView::DeleteGames => (
            "Delete Game",
            None,
            delete_items(entries.games),
            &mut state.delete_games_idx,
        ),
    }
}

fn delete_items(entries: &[String]) -> Vec<String> {
    if entries.is_empty() {
        return vec!["Back".to_string()];
    }
    let mut out = entries.to_vec();
    out.push("---".to_string());
    out.push("Back".to_string());
    out
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
    fn delete_items_returns_back_when_empty() {
        assert_eq!(delete_items(&[]), vec!["Back".to_string()]);
    }

    #[test]
    fn root_view_contains_edit_documents_row() {
        let mut state = TerminalEditMenusState::default();
        state.view = EditMenusView::Root;
        let (_title, _subtitle, items, _selected) = view_payload(
            &mut state,
            EditMenusEntries {
                applications: &[],
                documents: &[],
                network: &[],
                games: &[],
            },
            true,
            true,
        );
        assert!(items.iter().any(|item| item == "Edit Documents"));
    }
}
