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
pub struct EditMenusViewModel {
    pub title: &'static str,
    pub subtitle: Option<String>,
    pub items: Vec<String>,
    pub selected_idx: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TerminalEditMenusRequest {
    None,
    BackToSettings,
    PersistToggleBuiltinNukeCodes,
    PersistToggleBuiltinTextEditor,
    OpenPromptAddProgramName {
        target: EditMenuTarget,
        title: String,
        prompt: String,
    },
    OpenPromptAddCategoryName {
        title: String,
        prompt: String,
    },
    OpenConfirmDelete {
        target: EditMenuTarget,
        title: String,
        prompt: String,
        name: String,
    },
    Status(String),
}

pub fn build_edit_menus_view_model(
    state: &TerminalEditMenusState,
    entries: EditMenusEntries<'_>,
    nuke_codes_visible: bool,
    text_editor_visible: bool,
) -> EditMenusViewModel {
    match state.view {
        EditMenusView::Root => EditMenusViewModel {
            title: "Edit Menus",
            subtitle: None,
            items: vec![
                "Edit Applications".to_string(),
                "Edit Documents".to_string(),
                "Edit Network".to_string(),
                "Edit Games".to_string(),
                "---".to_string(),
                "Back".to_string(),
            ],
            selected_idx: state.root_idx,
        },
        EditMenusView::Applications => EditMenusViewModel {
            title: "Edit Applications",
            subtitle: None,
            items: vec![
                format!(
                    "Nuke Codes in Applications: {} [toggle]",
                    if nuke_codes_visible { "VISIBLE" } else { "HIDDEN" }
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
            selected_idx: state.applications_idx,
        },
        EditMenusView::Documents => EditMenusViewModel {
            title: "Edit Documents",
            subtitle: None,
            items: vec![
                "Add Category".to_string(),
                "Delete Category".to_string(),
                "---".to_string(),
                "Back".to_string(),
            ],
            selected_idx: state.documents_idx,
        },
        EditMenusView::Network => EditMenusViewModel {
            title: "Edit Network",
            subtitle: None,
            items: vec![
                "Add Network".to_string(),
                "Delete Network".to_string(),
                "---".to_string(),
                "Back".to_string(),
            ],
            selected_idx: state.network_idx,
        },
        EditMenusView::Games => EditMenusViewModel {
            title: "Edit Games",
            subtitle: None,
            items: vec![
                "Add Game".to_string(),
                "Delete Game".to_string(),
                "---".to_string(),
                "Back".to_string(),
            ],
            selected_idx: state.games_idx,
        },
        EditMenusView::DeleteApplications => EditMenusViewModel {
            title: "Delete App",
            subtitle: None,
            items: delete_items(entries.applications),
            selected_idx: state.delete_applications_idx,
        },
        EditMenusView::DeleteDocuments => EditMenusViewModel {
            title: "Delete Category",
            subtitle: None,
            items: delete_items(entries.documents),
            selected_idx: state.delete_documents_idx,
        },
        EditMenusView::DeleteNetwork => EditMenusViewModel {
            title: "Delete Network Program",
            subtitle: None,
            items: delete_items(entries.network),
            selected_idx: state.delete_network_idx,
        },
        EditMenusView::DeleteGames => EditMenusViewModel {
            title: "Delete Game",
            subtitle: None,
            items: delete_items(entries.games),
            selected_idx: state.delete_games_idx,
        },
    }
}

pub fn apply_edit_menus_activation(
    state: &mut TerminalEditMenusState,
    entries: EditMenusEntries<'_>,
    activated: Option<usize>,
) -> TerminalEditMenusRequest {
    let Some(idx) = activated else {
        return TerminalEditMenusRequest::None;
    };

    match state.view {
        EditMenusView::Root => match idx {
            0 => {
                state.view = EditMenusView::Applications;
                TerminalEditMenusRequest::None
            }
            1 => {
                state.view = EditMenusView::Documents;
                TerminalEditMenusRequest::None
            }
            2 => {
                state.view = EditMenusView::Network;
                TerminalEditMenusRequest::None
            }
            3 => {
                state.view = EditMenusView::Games;
                TerminalEditMenusRequest::None
            }
            _ => TerminalEditMenusRequest::BackToSettings,
        },
        EditMenusView::Applications => match idx {
            0 => TerminalEditMenusRequest::PersistToggleBuiltinNukeCodes,
            1 => TerminalEditMenusRequest::PersistToggleBuiltinTextEditor,
            3 => TerminalEditMenusRequest::OpenPromptAddProgramName {
                target: EditMenuTarget::Applications,
                title: format!("Edit {}", EditMenuTarget::Applications.title()),
                prompt: format!(
                    "Enter {} display name:",
                    EditMenuTarget::Applications.singular()
                ),
            },
            4 => {
                if entries.applications.is_empty() {
                    TerminalEditMenusRequest::Status("Error: App list is empty.".to_string())
                } else {
                    state.view = EditMenusView::DeleteApplications;
                    state.delete_applications_idx = 0;
                    TerminalEditMenusRequest::None
                }
            }
            _ => {
                state.view = EditMenusView::Root;
                TerminalEditMenusRequest::None
            }
        },
        EditMenusView::Documents => match idx {
            0 => TerminalEditMenusRequest::OpenPromptAddCategoryName {
                title: "Edit Documents".to_string(),
                prompt: "Enter category name:".to_string(),
            },
            1 => {
                if entries.documents.is_empty() {
                    TerminalEditMenusRequest::Status("Error: No categories to delete.".to_string())
                } else {
                    state.view = EditMenusView::DeleteDocuments;
                    state.delete_documents_idx = 0;
                    TerminalEditMenusRequest::None
                }
            }
            _ => {
                state.view = EditMenusView::Root;
                TerminalEditMenusRequest::None
            }
        },
        EditMenusView::Network => match idx {
            0 => TerminalEditMenusRequest::OpenPromptAddProgramName {
                target: EditMenuTarget::Network,
                title: format!("Edit {}", EditMenuTarget::Network.title()),
                prompt: format!("Enter {} display name:", EditMenuTarget::Network.singular()),
            },
            1 => {
                if entries.network.is_empty() {
                    TerminalEditMenusRequest::Status(
                        "Error: Network Program list is empty.".to_string(),
                    )
                } else {
                    state.view = EditMenusView::DeleteNetwork;
                    state.delete_network_idx = 0;
                    TerminalEditMenusRequest::None
                }
            }
            _ => {
                state.view = EditMenusView::Root;
                TerminalEditMenusRequest::None
            }
        },
        EditMenusView::Games => match idx {
            0 => TerminalEditMenusRequest::OpenPromptAddProgramName {
                target: EditMenuTarget::Games,
                title: format!("Edit {}", EditMenuTarget::Games.title()),
                prompt: format!("Enter {} display name:", EditMenuTarget::Games.singular()),
            },
            1 => {
                if entries.games.is_empty() {
                    TerminalEditMenusRequest::Status("Error: Game list is empty.".to_string())
                } else {
                    state.view = EditMenusView::DeleteGames;
                    state.delete_games_idx = 0;
                    TerminalEditMenusRequest::None
                }
            }
            _ => {
                state.view = EditMenusView::Root;
                TerminalEditMenusRequest::None
            }
        },
        EditMenusView::DeleteApplications => {
            if idx < entries.applications.len() {
                TerminalEditMenusRequest::OpenConfirmDelete {
                    target: EditMenuTarget::Applications,
                    title: format!("Delete {}", EditMenuTarget::Applications.singular()),
                    prompt: format!("Delete '{}'?", entries.applications[idx]),
                    name: entries.applications[idx].clone(),
                }
            } else {
                state.view = EditMenusView::Applications;
                TerminalEditMenusRequest::None
            }
        }
        EditMenusView::DeleteDocuments => {
            if idx < entries.documents.len() {
                TerminalEditMenusRequest::OpenConfirmDelete {
                    target: EditMenuTarget::Documents,
                    title: "Delete Category".to_string(),
                    prompt: format!("Delete category '{}'?", entries.documents[idx]),
                    name: entries.documents[idx].clone(),
                }
            } else {
                state.view = EditMenusView::Documents;
                TerminalEditMenusRequest::None
            }
        }
        EditMenusView::DeleteNetwork => {
            if idx < entries.network.len() {
                TerminalEditMenusRequest::OpenConfirmDelete {
                    target: EditMenuTarget::Network,
                    title: format!("Delete {}", EditMenuTarget::Network.singular()),
                    prompt: format!("Delete '{}'?", entries.network[idx]),
                    name: entries.network[idx].clone(),
                }
            } else {
                state.view = EditMenusView::Network;
                TerminalEditMenusRequest::None
            }
        }
        EditMenusView::DeleteGames => {
            if idx < entries.games.len() {
                TerminalEditMenusRequest::OpenConfirmDelete {
                    target: EditMenuTarget::Games,
                    title: format!("Delete {}", EditMenuTarget::Games.singular()),
                    prompt: format!("Delete '{}'?", entries.games[idx]),
                    name: entries.games[idx].clone(),
                }
            } else {
                state.view = EditMenusView::Games;
                TerminalEditMenusRequest::None
            }
        }
    }
}

pub fn apply_edit_menus_selected_idx(state: &mut TerminalEditMenusState, idx: usize) {
    match state.view {
        EditMenusView::Root => state.root_idx = idx,
        EditMenusView::Applications => state.applications_idx = idx,
        EditMenusView::Documents => state.documents_idx = idx,
        EditMenusView::Network => state.network_idx = idx,
        EditMenusView::Games => state.games_idx = idx,
        EditMenusView::DeleteApplications => state.delete_applications_idx = idx,
        EditMenusView::DeleteDocuments => state.delete_documents_idx = idx,
        EditMenusView::DeleteNetwork => state.delete_network_idx = idx,
        EditMenusView::DeleteGames => state.delete_games_idx = idx,
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
            true,
        );
        assert!(model.items.iter().any(|item| item == "Edit Documents"));
    }

    #[test]
    fn delete_documents_uses_category_prompt() {
        let mut state = TerminalEditMenusState::default();
        state.view = EditMenusView::DeleteDocuments;
        let request = apply_edit_menus_activation(
            &mut state,
            EditMenusEntries {
                applications: &[],
                documents: &["Work".to_string()],
                network: &[],
                games: &[],
            },
            Some(0),
        );
        assert!(matches!(
            request,
            TerminalEditMenusRequest::OpenConfirmDelete {
                target: EditMenuTarget::Documents,
                ..
            }
        ));
    }
}
