use super::desktop_documents_service::document_category_entries;
use super::desktop_launcher_service::{catalog_names, ProgramCatalog};
use std::collections::HashSet;
use std::path::PathBuf;

fn home_dir_fallback() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
}

fn documents_dir() -> PathBuf {
    dirs::document_dir().unwrap_or_else(home_dir_fallback)
}

fn word_processor_dir(username: &str) -> PathBuf {
    let dir = documents_dir().join("ROBCO Word Processor").join(username);
    let _ = std::fs::create_dir_all(&dir);
    dir
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeStartLeafAction {
    None,
    LaunchNukeCodes,
    OpenTextEditor,
    LaunchConfiguredApp(String),
    OpenDocumentCategory(PathBuf),
    LaunchNetworkProgram(String),
    LaunchGameProgram(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeStartLeafEntry {
    pub label: String,
    pub action: NativeStartLeafAction,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeSpotlightCategory {
    App,
    Game,
    Document,
    File,
    System,
    Network,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeSpotlightResult {
    pub name: String,
    pub category: NativeSpotlightCategory,
    pub path: Option<PathBuf>,
}

fn start_application_entries_from_names(
    application_names: Vec<String>,
    show_nuke_codes: bool,
    show_text_editor: bool,
    text_editor_name: &str,
    nuke_codes_name: &str,
) -> Vec<NativeStartLeafEntry> {
    let mut items = Vec::new();
    if show_nuke_codes {
        items.push(NativeStartLeafEntry {
            label: nuke_codes_name.to_string(),
            action: NativeStartLeafAction::LaunchNukeCodes,
        });
    }
    if show_text_editor {
        items.push(NativeStartLeafEntry {
            label: text_editor_name.to_string(),
            action: NativeStartLeafAction::OpenTextEditor,
        });
    }
    for name in application_names {
        if name == nuke_codes_name || name == text_editor_name {
            continue;
        }
        items.push(NativeStartLeafEntry {
            label: name.clone(),
            action: NativeStartLeafAction::LaunchConfiguredApp(name),
        });
    }
    if items.is_empty() {
        items.push(NativeStartLeafEntry {
            label: "(No applications)".to_string(),
            action: NativeStartLeafAction::None,
        });
    }
    items
}

pub fn start_application_entries(
    show_nuke_codes: bool,
    show_text_editor: bool,
    text_editor_name: &str,
    nuke_codes_name: &str,
) -> Vec<NativeStartLeafEntry> {
    start_application_entries_from_names(
        catalog_names(ProgramCatalog::Applications),
        show_nuke_codes,
        show_text_editor,
        text_editor_name,
        nuke_codes_name,
    )
}

pub fn start_document_entries(username: Option<&str>) -> Vec<NativeStartLeafEntry> {
    let mut items = Vec::new();
    if let Some(username) = username {
        items.push(NativeStartLeafEntry {
            label: "My Documents".to_string(),
            action: NativeStartLeafAction::OpenDocumentCategory(word_processor_dir(username)),
        });
    }
    for (name, path) in document_category_entries() {
        items.push(NativeStartLeafEntry {
            label: name,
            action: NativeStartLeafAction::OpenDocumentCategory(path),
        });
    }
    if items.is_empty() {
        items.push(NativeStartLeafEntry {
            label: "(No documents)".to_string(),
            action: NativeStartLeafAction::None,
        });
    }
    items
}

pub fn start_network_entries() -> Vec<NativeStartLeafEntry> {
    let mut items = Vec::new();
    for key in catalog_names(ProgramCatalog::Network) {
        items.push(NativeStartLeafEntry {
            label: key.clone(),
            action: NativeStartLeafAction::LaunchNetworkProgram(key),
        });
    }
    if items.is_empty() {
        items.push(NativeStartLeafEntry {
            label: "(No network apps)".to_string(),
            action: NativeStartLeafAction::None,
        });
    }
    items
}

pub fn start_game_entries() -> Vec<NativeStartLeafEntry> {
    let mut items = Vec::new();
    for key in catalog_names(ProgramCatalog::Games) {
        items.push(NativeStartLeafEntry {
            label: key.clone(),
            action: NativeStartLeafAction::LaunchGameProgram(key),
        });
    }
    if items.is_empty() {
        items.push(NativeStartLeafEntry {
            label: "(No games installed)".to_string(),
            action: NativeStartLeafAction::None,
        });
    }
    items
}

pub fn spotlight_category_tag(category: &NativeSpotlightCategory) -> &'static str {
    match category {
        NativeSpotlightCategory::App => "APP",
        NativeSpotlightCategory::Game => "GAME",
        NativeSpotlightCategory::Document => "DOC",
        NativeSpotlightCategory::File => "FILE",
        NativeSpotlightCategory::System => "SYS",
        NativeSpotlightCategory::Network => "NET",
    }
}

pub fn gather_spotlight_results(
    query: &str,
    tab: u8,
    active_username: Option<&str>,
    text_editor_name: &str,
    nuke_codes_name: &str,
) -> Vec<NativeSpotlightResult> {
    gather_spotlight_results_with_names(
        query,
        tab,
        active_username,
        text_editor_name,
        nuke_codes_name,
        catalog_names(ProgramCatalog::Applications),
        catalog_names(ProgramCatalog::Games),
        catalog_names(ProgramCatalog::Network),
    )
}

fn gather_spotlight_results_with_names(
    query: &str,
    tab: u8,
    active_username: Option<&str>,
    text_editor_name: &str,
    nuke_codes_name: &str,
    application_names: Vec<String>,
    game_names: Vec<String>,
    network_names: Vec<String>,
) -> Vec<NativeSpotlightResult> {
    let query = query.to_lowercase();
    let matches_query =
        |name: &str| -> bool { query.is_empty() || name.to_lowercase().contains(&query) };
    let mut results = Vec::new();

    if tab == 0 || tab == 1 {
        for name in &[
            "File Manager",
            "Settings",
            "Terminal",
            text_editor_name,
            nuke_codes_name,
        ] {
            if matches_query(name) {
                results.push(NativeSpotlightResult {
                    name: (*name).to_string(),
                    category: NativeSpotlightCategory::System,
                    path: None,
                });
            }
        }
        for name in application_names {
            if name != nuke_codes_name && name != text_editor_name && matches_query(&name) {
                results.push(NativeSpotlightResult {
                    name,
                    category: NativeSpotlightCategory::App,
                    path: None,
                });
            }
        }
        for name in game_names {
            if matches_query(&name) {
                results.push(NativeSpotlightResult {
                    name,
                    category: NativeSpotlightCategory::Game,
                    path: None,
                });
            }
        }
        for name in network_names {
            if matches_query(&name) {
                results.push(NativeSpotlightResult {
                    name,
                    category: NativeSpotlightCategory::Network,
                    path: None,
                });
            }
        }
    }

    if tab == 0 || tab == 2 {
        if let Some(username) = active_username {
            let doc_dir = word_processor_dir(username);
            if doc_dir.is_dir() {
                if let Ok(entries) = std::fs::read_dir(&doc_dir) {
                    for entry in entries.flatten() {
                        let name = entry.file_name().to_string_lossy().to_string();
                        if entry.file_type().map_or(false, |t| t.is_file()) && matches_query(&name)
                        {
                            results.push(NativeSpotlightResult {
                                name,
                                category: NativeSpotlightCategory::Document,
                                path: Some(entry.path()),
                            });
                        }
                    }
                }
            }
        }
    }

    if tab == 0 || tab == 3 {
        let dirs_to_search = [home_dir_fallback(), documents_dir()];
        let mut seen = HashSet::new();
        for dir in &dirs_to_search {
            if !dir.is_dir() {
                continue;
            }
            if let Ok(entries) = std::fs::read_dir(dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if seen.contains(&path) {
                        continue;
                    }
                    seen.insert(path.clone());
                    let name = entry.file_name().to_string_lossy().to_string();
                    if matches_query(&name) {
                        results.push(NativeSpotlightResult {
                            name,
                            category: NativeSpotlightCategory::File,
                            path: Some(path),
                        });
                    }
                }
            }
        }
    }

    results.truncate(50);
    results
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn start_application_entries_hide_duplicate_builtins() {
        let items = start_application_entries_from_names(
            vec!["ROBCO Word Processor".to_string(), "Hex".to_string()],
            true,
            true,
            "ROBCO Word Processor",
            "Nuke Codes",
        );

        assert_eq!(items[0].label, "Nuke Codes");
        assert_eq!(items[1].label, "ROBCO Word Processor");
        assert_eq!(items[2].label, "Hex");
    }

    #[test]
    fn gather_spotlight_results_includes_system_and_catalog_hits() {
        let results = gather_spotlight_results_with_names(
            "hel",
            1,
            None,
            "ROBCO Word Processor",
            "Nuke Codes",
            vec!["Helix".to_string()],
            vec!["Missile Command".to_string()],
            Vec::new(),
        );

        assert_eq!(
            results,
            vec![NativeSpotlightResult {
                name: "Helix".to_string(),
                category: NativeSpotlightCategory::App,
                path: None,
            }]
        );
    }
}
