use crate::config::{load_apps, load_games, load_networks, save_apps, save_games, save_networks};
use crate::default_apps::parse_custom_command_line;
use crate::launcher::json_to_cmd;
use serde_json::{Map, Value};

fn resolve_program_command(name: &str, source: &Map<String, Value>) -> Result<Vec<String>, String> {
    let Some(value) = source.get(name) else {
        return Err(format!("Unknown program '{name}'."));
    };
    let argv = json_to_cmd(value);
    if argv.is_empty() {
        return Err("Error: empty command.".to_string());
    }
    Ok(argv)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProgramCatalog {
    Applications,
    Network,
    Games,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedProgramLaunch {
    pub title: String,
    pub argv: Vec<String>,
}

pub fn all_game_menu_names() -> Vec<String> {
    sorted_source_names(&load_games())
}

fn load_catalog_source(catalog: ProgramCatalog) -> Map<String, Value> {
    match catalog {
        ProgramCatalog::Applications => load_apps(),
        ProgramCatalog::Network => load_networks(),
        ProgramCatalog::Games => load_games(),
    }
}

fn save_catalog_source(catalog: ProgramCatalog, source: &Map<String, Value>) {
    match catalog {
        ProgramCatalog::Applications => save_apps(source),
        ProgramCatalog::Network => save_networks(source),
        ProgramCatalog::Games => save_games(source),
    }
}

pub fn resolve_program_launch_from_source(
    name: &str,
    source: &Map<String, Value>,
) -> Result<ResolvedProgramLaunch, String> {
    resolve_program_command(name, source).map(|argv| ResolvedProgramLaunch {
        title: name.to_string(),
        argv,
    })
}

fn sorted_source_names(source: &Map<String, Value>) -> Vec<String> {
    let mut names: Vec<String> = source.keys().cloned().collect();
    names.sort();
    names
}

pub fn catalog_names(catalog: ProgramCatalog) -> Vec<String> {
    sorted_source_names(&load_catalog_source(catalog))
}

pub fn resolve_catalog_launch(
    name: &str,
    catalog: ProgramCatalog,
) -> Result<ResolvedProgramLaunch, String> {
    resolve_program_launch_from_source(name, &load_catalog_source(catalog))
}

pub fn resolve_catalog_command_line(name: &str, catalog: ProgramCatalog) -> Option<String> {
    resolve_catalog_launch(name, catalog)
        .ok()
        .map(|launch| launch.argv.join(" "))
}

pub fn parse_catalog_command_line(raw: &str) -> Result<Vec<String>, String> {
    let Some(argv) = parse_custom_command_line(raw.trim()) else {
        return Err("Error: invalid command line".to_string());
    };
    if argv.is_empty() {
        return Err("Error: invalid command line".to_string());
    }
    Ok(argv)
}

fn insert_catalog_entry_into_source(
    source: &mut Map<String, Value>,
    name: String,
    argv: Vec<String>,
) {
    source.insert(
        name,
        Value::Array(argv.into_iter().map(Value::String).collect()),
    );
}

fn rename_catalog_entry_in_source(
    source: &mut Map<String, Value>,
    old_name: &str,
    new_name: &str,
) -> Result<(), String> {
    if source.contains_key(new_name) {
        return Err(format!("{new_name} already exists."));
    }
    let Some(entry) = source.remove(old_name) else {
        return Err(format!("{old_name} was not found."));
    };
    source.insert(new_name.to_string(), entry);
    Ok(())
}

pub fn add_catalog_entry(catalog: ProgramCatalog, name: String, argv: Vec<String>) -> String {
    let mut source = load_catalog_source(catalog);
    insert_catalog_entry_into_source(&mut source, name.clone(), argv);
    save_catalog_source(catalog, &source);
    format!("{name} added.")
}

pub fn delete_catalog_entry(catalog: ProgramCatalog, name: &str) -> String {
    let mut source = load_catalog_source(catalog);
    source.remove(name);
    save_catalog_source(catalog, &source);
    format!("{name} deleted.")
}

pub fn rename_catalog_entry(
    catalog: ProgramCatalog,
    old_name: &str,
    new_name: &str,
) -> Result<String, String> {
    let mut source = load_catalog_source(catalog);
    rename_catalog_entry_in_source(&mut source, old_name, new_name)?;
    save_catalog_source(catalog, &source);
    Ok(format!("{old_name} renamed to {new_name}."))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn source_with_command(name: &str, argv: &[&str]) -> Map<String, Value> {
        let mut source = Map::new();
        source.insert(
            name.to_string(),
            Value::Array(
                argv.iter()
                    .map(|item| Value::String((*item).to_string()))
                    .collect(),
            ),
        );
        source
    }

    #[test]
    fn resolve_program_launch_from_source_returns_title_and_argv() {
        let source = source_with_command("Editor", &["hx", "demo.txt"]);

        let launch =
            resolve_program_launch_from_source("Editor", &source).expect("resolve program launch");

        assert_eq!(launch.title, "Editor");
        assert_eq!(launch.argv, vec!["hx".to_string(), "demo.txt".to_string()]);
    }

    #[test]
    fn resolve_program_launch_from_source_reports_unknown_program() {
        let source = Map::new();
        let err =
            resolve_program_launch_from_source("Missing", &source).expect_err("missing program");

        assert!(err.contains("Missing"));
    }

    #[test]
    fn resolve_program_launch_from_source_rejects_empty_commands() {
        let mut source = Map::new();
        source.insert("Broken".to_string(), Value::Array(Vec::new()));

        let err = resolve_program_launch_from_source("Broken", &source).expect_err("empty command");

        assert_eq!(err, "Error: empty command.");
    }

    #[test]
    fn resolve_program_command_line_from_source_joins_arguments() {
        let source = source_with_command("Shell", &["bash", "-lc", "echo test"]);

        let command_line = resolve_program_launch_from_source("Shell", &source)
            .ok()
            .map(|launch| launch.argv.join(" "))
            .expect("command line");

        assert_eq!(command_line, "bash -lc echo test");
    }

    #[test]
    fn sorted_source_names_returns_sorted_catalog_entries() {
        let source = source_with_command("Zed", &["zed"]);
        let mut extended = source;
        extended.insert(
            "Alpha".to_string(),
            Value::Array(vec![Value::String("alpha".to_string())]),
        );

        assert_eq!(
            sorted_source_names(&extended),
            vec!["Alpha".to_string(), "Zed".to_string()]
        );
    }

    #[test]
    fn insert_catalog_entry_into_source_stores_command_array() {
        let mut source = Map::new();

        insert_catalog_entry_into_source(
            &mut source,
            "Editor".to_string(),
            vec!["hx".to_string(), "demo.txt".to_string()],
        );

        assert_eq!(
            source.get("Editor"),
            Some(&Value::Array(vec![
                Value::String("hx".to_string()),
                Value::String("demo.txt".to_string()),
            ]))
        );
    }

    #[test]
    fn rename_catalog_entry_in_source_rejects_duplicate_names() {
        let mut source = source_with_command("Alpha", &["alpha"]);
        source.insert(
            "Beta".to_string(),
            Value::Array(vec![Value::String("beta".to_string())]),
        );

        let err = rename_catalog_entry_in_source(&mut source, "Alpha", "Beta")
            .expect_err("duplicate rename should fail");

        assert_eq!(err, "Beta already exists.");
        assert!(source.contains_key("Alpha"));
    }
}
