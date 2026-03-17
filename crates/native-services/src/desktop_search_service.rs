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

fn game_program_names(builtin_game_name: &str) -> Vec<String> {
    let mut names = vec![builtin_game_name.to_string()];
    names.extend(
        catalog_names(ProgramCatalog::Games)
            .into_iter()
            .filter(|name| name != builtin_game_name),
    );
    names
}

pub fn start_application_entries(
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
    for name in catalog_names(ProgramCatalog::Applications) {
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

pub fn start_game_entries(builtin_game_name: &str) -> Vec<NativeStartLeafEntry> {
    let mut items = Vec::new();
    for key in game_program_names(builtin_game_name) {
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
    builtin_game_name: &str,
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
        for name in catalog_names(ProgramCatalog::Applications) {
            if name != nuke_codes_name && name != text_editor_name && matches_query(&name) {
                results.push(NativeSpotlightResult {
                    name,
                    category: NativeSpotlightCategory::App,
                    path: None,
                });
            }
        }
        for name in game_program_names(builtin_game_name) {
            if matches_query(&name) {
                results.push(NativeSpotlightResult {
                    name,
                    category: NativeSpotlightCategory::Game,
                    path: None,
                });
            }
        }
        for name in catalog_names(ProgramCatalog::Network) {
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
    use crate::config::{save_apps, save_games, save_networks};
    use serde_json::{Map, Value};
    use std::sync::{Mutex, OnceLock};

    fn search_test_guard() -> std::sync::MutexGuard<'static, ()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
            .lock()
            .expect("desktop search service test lock")
    }

    struct CatalogRestore {
        apps: Map<String, Value>,
        games: Map<String, Value>,
        networks: Map<String, Value>,
    }

    impl CatalogRestore {
        fn capture() -> Self {
            Self {
                apps: crate::config::load_apps(),
                games: crate::config::load_games(),
                networks: crate::config::load_networks(),
            }
        }
    }

    impl Drop for CatalogRestore {
        fn drop(&mut self) {
            save_apps(&self.apps);
            save_games(&self.games);
            save_networks(&self.networks);
        }
    }

    #[test]
    fn start_application_entries_hide_duplicate_builtins() {
        let _guard = search_test_guard();
        let _restore = CatalogRestore::capture();
        let mut apps = Map::new();
        apps.insert(
            "ROBCO Word Processor".to_string(),
            Value::Array(vec![Value::String("fake".to_string())]),
        );
        apps.insert(
            "Hex".to_string(),
            Value::Array(vec![Value::String("hx".to_string())]),
        );
        save_apps(&apps);

        let items = start_application_entries(true, true, "ROBCO Word Processor", "Nuke Codes");

        assert_eq!(items[0].label, "Nuke Codes");
        assert_eq!(items[1].label, "ROBCO Word Processor");
        assert_eq!(items[2].label, "Hex");
    }

    #[test]
    fn gather_spotlight_results_includes_system_and_catalog_hits() {
        let _guard = search_test_guard();
        let _restore = CatalogRestore::capture();
        let mut apps = Map::new();
        apps.insert(
            "Helix".to_string(),
            Value::Array(vec![Value::String("hx".to_string())]),
        );
        save_apps(&apps);
        save_games(&Map::new());
        save_networks(&Map::new());

        let results = gather_spotlight_results(
            "hel",
            1,
            None,
            "ROBCO Word Processor",
            "Nuke Codes",
            "Donkey Kong",
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
