use robcos_native_services::desktop_launcher_service::ProgramCatalog;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProgramMenuEvent {
    None,
    Back,
    Launch(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TerminalCatalogMenuAction {
    None,
    Back,
    Launch(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TerminalApplicationsAction {
    None,
    Back,
    OpenTextEditor,
    OpenNukeCodes,
    OpenFileManager,
    LaunchConfigured(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DesktopApplicationsAction {
    OpenTextEditor,
    OpenNukeCodes,
    OpenFileManager,
    LaunchConfigured(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DesktopGamesAction {
    LaunchConfigured(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TerminalProgramRequest {
    None,
    BackToMainMenu,
    OpenTextEditor,
    OpenNukeCodes,
    OpenFileManager,
    LaunchCatalog {
        name: String,
        catalog: ProgramCatalog,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DesktopProgramRequest {
    OpenTextEditor {
        close_window: bool,
    },
    OpenNukeCodes {
        close_window: bool,
    },
    OpenFileManager,
    LaunchCatalog {
        name: String,
        catalog: ProgramCatalog,
        close_window: bool,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DesktopProgramEntry<T> {
    pub label: String,
    pub action: T,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DesktopApplicationsSections {
    pub builtins: Vec<DesktopProgramEntry<DesktopApplicationsAction>>,
    pub configured: Vec<DesktopProgramEntry<DesktopApplicationsAction>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TerminalGamesAction {
    None,
    Back,
    LaunchConfigured(String),
}

pub fn build_program_menu_items(entries: &[String]) -> Vec<String> {
    let mut items = entries.to_vec();
    items.push("---".to_string());
    items.push("Back".to_string());
    items
}

pub fn resolve_program_menu_event(
    entries: &[String],
    activated: Option<usize>,
) -> ProgramMenuEvent {
    match activated {
        Some(idx) if idx < entries.len() => ProgramMenuEvent::Launch(entries[idx].clone()),
        Some(_) => ProgramMenuEvent::Back,
        None => ProgramMenuEvent::None,
    }
}

pub const BUILTIN_FILE_MANAGER_APP: &str = "File Manager";

pub fn build_terminal_application_entries(
    show_text_editor: bool,
    show_nuke_codes: bool,
    configured_names: &[String],
    text_editor_label: &str,
    nuke_codes_label: &str,
) -> Vec<String> {
    let mut entries = Vec::new();
    entries.push(BUILTIN_FILE_MANAGER_APP.to_string());
    if show_nuke_codes {
        entries.push(nuke_codes_label.to_string());
    }
    if show_text_editor {
        entries.push(text_editor_label.to_string());
    }
    entries.extend(
        configured_names
            .iter()
            .filter(|name| {
                *name != nuke_codes_label
                    && *name != text_editor_label
                    && *name != BUILTIN_FILE_MANAGER_APP
            })
            .cloned(),
    );
    entries
}

pub fn resolve_desktop_applications_action(
    label: &str,
    text_editor_label: &str,
    nuke_codes_label: &str,
) -> DesktopApplicationsAction {
    if label == BUILTIN_FILE_MANAGER_APP {
        DesktopApplicationsAction::OpenFileManager
    } else if label == text_editor_label {
        DesktopApplicationsAction::OpenTextEditor
    } else if label == nuke_codes_label {
        DesktopApplicationsAction::OpenNukeCodes
    } else {
        DesktopApplicationsAction::LaunchConfigured(label.to_string())
    }
}

pub fn build_desktop_applications_sections(
    show_file_manager: bool,
    show_text_editor: bool,
    show_nuke_codes: bool,
    configured_names: &[String],
    text_editor_label: &str,
    nuke_codes_label: &str,
) -> DesktopApplicationsSections {
    let mut builtin_labels = Vec::new();
    if show_file_manager {
        builtin_labels.push(BUILTIN_FILE_MANAGER_APP.to_string());
    }
    if show_nuke_codes {
        builtin_labels.push(nuke_codes_label.to_string());
    }
    if show_text_editor {
        builtin_labels.push(text_editor_label.to_string());
    }
    let builtins = builtin_labels
        .into_iter()
        .map(|label| DesktopProgramEntry {
            action: resolve_desktop_applications_action(&label, text_editor_label, nuke_codes_label),
            label,
        })
        .collect();
    let configured = build_terminal_application_entries(
        false,
        false,
        configured_names,
        text_editor_label,
        nuke_codes_label,
    )
    .into_iter()
    .filter(|label| label != BUILTIN_FILE_MANAGER_APP)
    .map(|label| DesktopProgramEntry {
        action: resolve_desktop_applications_action(&label, text_editor_label, nuke_codes_label),
        label,
    })
    .collect();
    DesktopApplicationsSections {
        builtins,
        configured,
    }
}

pub fn resolve_desktop_games_action(label: &str) -> DesktopGamesAction {
    DesktopGamesAction::LaunchConfigured(label.to_string())
}

pub fn build_terminal_game_entries(configured_names: &[String]) -> Vec<String> {
    configured_names.to_vec()
}

pub fn resolve_catalog_menu_action(event: ProgramMenuEvent) -> TerminalCatalogMenuAction {
    match event {
        ProgramMenuEvent::None => TerminalCatalogMenuAction::None,
        ProgramMenuEvent::Back => TerminalCatalogMenuAction::Back,
        ProgramMenuEvent::Launch(name) => TerminalCatalogMenuAction::Launch(name),
    }
}

pub fn resolve_terminal_applications_request(
    event: ProgramMenuEvent,
    text_editor_label: &str,
    nuke_codes_label: &str,
) -> TerminalProgramRequest {
    match resolve_terminal_applications_action(event, text_editor_label, nuke_codes_label) {
        TerminalApplicationsAction::None => TerminalProgramRequest::None,
        TerminalApplicationsAction::Back => TerminalProgramRequest::BackToMainMenu,
        TerminalApplicationsAction::OpenTextEditor => TerminalProgramRequest::OpenTextEditor,
        TerminalApplicationsAction::OpenNukeCodes => TerminalProgramRequest::OpenNukeCodes,
        TerminalApplicationsAction::OpenFileManager => TerminalProgramRequest::OpenFileManager,
        TerminalApplicationsAction::LaunchConfigured(name) => {
            TerminalProgramRequest::LaunchCatalog {
                name,
                catalog: ProgramCatalog::Applications,
            }
        }
    }
}

pub fn resolve_terminal_catalog_request(
    event: ProgramMenuEvent,
    catalog: ProgramCatalog,
) -> TerminalProgramRequest {
    match resolve_catalog_menu_action(event) {
        TerminalCatalogMenuAction::None => TerminalProgramRequest::None,
        TerminalCatalogMenuAction::Back => TerminalProgramRequest::BackToMainMenu,
        TerminalCatalogMenuAction::Launch(name) => {
            TerminalProgramRequest::LaunchCatalog { name, catalog }
        }
    }
}

pub fn resolve_terminal_games_request(event: ProgramMenuEvent) -> TerminalProgramRequest {
    match resolve_terminal_games_action(event) {
        TerminalGamesAction::None => TerminalProgramRequest::None,
        TerminalGamesAction::Back => TerminalProgramRequest::BackToMainMenu,
        TerminalGamesAction::LaunchConfigured(name) => TerminalProgramRequest::LaunchCatalog {
            name,
            catalog: ProgramCatalog::Games,
        },
    }
}

pub fn resolve_desktop_applications_request(
    action: &DesktopApplicationsAction,
) -> DesktopProgramRequest {
    match action {
        DesktopApplicationsAction::OpenTextEditor => DesktopProgramRequest::OpenTextEditor {
            close_window: false,
        },
        DesktopApplicationsAction::OpenNukeCodes => {
            DesktopProgramRequest::OpenNukeCodes { close_window: true }
        }
        DesktopApplicationsAction::OpenFileManager => DesktopProgramRequest::OpenFileManager,
        DesktopApplicationsAction::LaunchConfigured(name) => DesktopProgramRequest::LaunchCatalog {
            name: name.clone(),
            catalog: ProgramCatalog::Applications,
            close_window: true,
        },
    }
}

pub fn resolve_desktop_games_request(name: &str) -> DesktopProgramRequest {
    match resolve_desktop_games_action(name) {
        DesktopGamesAction::LaunchConfigured(name) => DesktopProgramRequest::LaunchCatalog {
            name,
            catalog: ProgramCatalog::Games,
            close_window: true,
        },
    }
}

pub fn resolve_terminal_applications_action(
    event: ProgramMenuEvent,
    text_editor_label: &str,
    nuke_codes_label: &str,
) -> TerminalApplicationsAction {
    match event {
        ProgramMenuEvent::None => TerminalApplicationsAction::None,
        ProgramMenuEvent::Back => TerminalApplicationsAction::Back,
        ProgramMenuEvent::Launch(name) if name == BUILTIN_FILE_MANAGER_APP => {
            TerminalApplicationsAction::OpenFileManager
        }
        ProgramMenuEvent::Launch(name) if name == text_editor_label => {
            TerminalApplicationsAction::OpenTextEditor
        }
        ProgramMenuEvent::Launch(name) if name == nuke_codes_label => {
            TerminalApplicationsAction::OpenNukeCodes
        }
        ProgramMenuEvent::Launch(name) => TerminalApplicationsAction::LaunchConfigured(name),
    }
}

pub fn resolve_terminal_games_action(event: ProgramMenuEvent) -> TerminalGamesAction {
    match event {
        ProgramMenuEvent::None => TerminalGamesAction::None,
        ProgramMenuEvent::Back => TerminalGamesAction::Back,
        ProgramMenuEvent::Launch(name) => TerminalGamesAction::LaunchConfigured(name),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_program_menu_items_appends_separator_and_back() {
        let items = build_program_menu_items(&["One".to_string(), "Two".to_string()]);
        assert_eq!(items, vec!["One", "Two", "---", "Back"]);
    }

    #[test]
    fn resolve_program_menu_event_launches_known_entry() {
        let entries = vec!["One".to_string(), "Two".to_string()];
        assert_eq!(
            resolve_program_menu_event(&entries, Some(1)),
            ProgramMenuEvent::Launch("Two".to_string())
        );
    }

    #[test]
    fn resolve_program_menu_event_routes_back_for_footer_selection() {
        let entries = vec!["One".to_string()];
        assert_eq!(
            resolve_program_menu_event(&entries, Some(2)),
            ProgramMenuEvent::Back
        );
        assert_eq!(
            resolve_program_menu_event(&entries, None),
            ProgramMenuEvent::None
        );
    }

    #[test]
    fn build_terminal_application_entries_prefers_builtins_and_filters_duplicates() {
        let entries = build_terminal_application_entries(
            true,
            true,
            &[
                "Editor".to_string(),
                "Nuke Codes".to_string(),
                "Custom App".to_string(),
            ],
            "Editor",
            "Nuke Codes",
        );
        assert_eq!(
            entries,
            vec!["File Manager", "Nuke Codes", "Editor", "Custom App"]
        );
    }

    #[test]
    fn resolve_terminal_applications_action_maps_builtin_labels() {
        assert_eq!(
            resolve_terminal_applications_action(
                ProgramMenuEvent::Launch("Editor".to_string()),
                "Editor",
                "Nuke Codes",
            ),
            TerminalApplicationsAction::OpenTextEditor
        );
        assert_eq!(
            resolve_terminal_applications_action(
                ProgramMenuEvent::Launch("Custom".to_string()),
                "Editor",
                "Nuke Codes",
            ),
            TerminalApplicationsAction::LaunchConfigured("Custom".to_string())
        );
    }

    #[test]
    fn resolve_desktop_applications_action_maps_builtin_labels() {
        assert_eq!(
            resolve_desktop_applications_action("Editor", "Editor", "Nuke Codes"),
            DesktopApplicationsAction::OpenTextEditor
        );
        assert_eq!(
            resolve_desktop_applications_action("Nuke Codes", "Editor", "Nuke Codes"),
            DesktopApplicationsAction::OpenNukeCodes
        );
        assert_eq!(
            resolve_desktop_applications_action("Custom", "Editor", "Nuke Codes"),
            DesktopApplicationsAction::LaunchConfigured("Custom".to_string())
        );
    }

    #[test]
    fn build_desktop_applications_sections_splits_builtins_and_configured() {
        let sections = build_desktop_applications_sections(
            true,
            true,
            true,
            &[
                "Editor".to_string(),
                "Nuke Codes".to_string(),
                "Custom".to_string(),
            ],
            "Editor",
            "Nuke Codes",
        );
        assert_eq!(
            sections.builtins,
            vec![
                DesktopProgramEntry {
                    label: "File Manager".to_string(),
                    action: DesktopApplicationsAction::OpenFileManager,
                },
                DesktopProgramEntry {
                    label: "Nuke Codes".to_string(),
                    action: DesktopApplicationsAction::OpenNukeCodes,
                },
                DesktopProgramEntry {
                    label: "Editor".to_string(),
                    action: DesktopApplicationsAction::OpenTextEditor,
                }
            ]
        );
        assert_eq!(
            sections.configured,
            vec![DesktopProgramEntry {
                label: "Custom".to_string(),
                action: DesktopApplicationsAction::LaunchConfigured("Custom".to_string()),
            }]
        );
    }

    #[test]
    fn build_desktop_applications_sections_can_hide_file_manager_builtin() {
        let sections = build_desktop_applications_sections(
            false,
            true,
            true,
            &[],
            "Editor",
            "Nuke Codes",
        );

        assert_eq!(
            sections.builtins,
            vec![
                DesktopProgramEntry {
                    label: "Nuke Codes".to_string(),
                    action: DesktopApplicationsAction::OpenNukeCodes,
                },
                DesktopProgramEntry {
                    label: "Editor".to_string(),
                    action: DesktopApplicationsAction::OpenTextEditor,
                }
            ]
        );
    }

    #[test]
    fn resolve_terminal_games_action_maps_builtin_game() {
        assert_eq!(
            resolve_terminal_games_action(ProgramMenuEvent::Launch("Custom".to_string())),
            TerminalGamesAction::LaunchConfigured("Custom".to_string())
        );
        assert_eq!(
            resolve_catalog_menu_action(ProgramMenuEvent::Back),
            TerminalCatalogMenuAction::Back
        );
        assert_eq!(
            resolve_desktop_games_action("Custom"),
            DesktopGamesAction::LaunchConfigured("Custom".to_string())
        );
    }

    #[test]
    fn resolve_terminal_requests_assign_catalogs() {
        assert_eq!(
            resolve_terminal_catalog_request(
                ProgramMenuEvent::Launch("Net".to_string()),
                ProgramCatalog::Network
            ),
            TerminalProgramRequest::LaunchCatalog {
                name: "Net".to_string(),
                catalog: ProgramCatalog::Network,
            }
        );
        assert_eq!(
            resolve_terminal_games_request(ProgramMenuEvent::Launch("Custom".to_string())),
            TerminalProgramRequest::LaunchCatalog {
                name: "Custom".to_string(),
                catalog: ProgramCatalog::Games,
            }
        );
    }

    #[test]
    fn resolve_desktop_requests_encode_close_behavior() {
        assert_eq!(
            resolve_desktop_applications_request(&DesktopApplicationsAction::OpenTextEditor),
            DesktopProgramRequest::OpenTextEditor {
                close_window: false
            }
        );
        assert_eq!(
            resolve_desktop_games_request("Custom"),
            DesktopProgramRequest::LaunchCatalog {
                name: "Custom".to_string(),
                catalog: ProgramCatalog::Games,
                close_window: true,
            }
        );
    }
}
