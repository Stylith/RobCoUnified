use crate::config::{
    bundled_bin_dir, load_apps, load_games, load_networks, save_apps, save_games, save_networks,
};
use crate::default_apps::parse_custom_command_line;
use crate::launcher::{command_exists, json_to_cmd};
use serde_json::{Map, Value};
use std::ffi::OsString;
use std::path::{Path, PathBuf};

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

pub const ROBCO_FUN_MENU_LABEL: &str = "RobCo Fun";
pub const BUILTIN_ZETA_INVADERS_GAME: &str = "Zeta Invaders";
pub const BUILTIN_RED_MENACE_GAME: &str = "Red Menace";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameMenuGroups {
    pub robco_fun: Vec<String>,
    pub other_games: Vec<String>,
}

fn builtin_robco_fun_game_specs(name: &str) -> Option<(&'static str, &'static str)> {
    match name {
        BUILTIN_ZETA_INVADERS_GAME => {
            Some(("robcos-native-zeta-invaders-app", "robcos-zeta-invaders"))
        }
        BUILTIN_RED_MENACE_GAME => Some(("robcos-native-red-menace-app", "robcos-red-menace")),
        _ => None,
    }
}

pub fn is_robco_fun_game(name: &str) -> bool {
    builtin_robco_fun_game_specs(name).is_some()
}

pub fn robco_fun_game_names() -> Vec<String> {
    vec![
        BUILTIN_ZETA_INVADERS_GAME.to_string(),
        BUILTIN_RED_MENACE_GAME.to_string(),
    ]
}

pub fn grouped_game_menu_names() -> GameMenuGroups {
    let mut other_games = sorted_source_names(&load_games());
    other_games.retain(|name| !is_robco_fun_game(name));
    GameMenuGroups {
        robco_fun: robco_fun_game_names(),
        other_games,
    }
}

pub fn all_game_menu_names() -> Vec<String> {
    let groups = grouped_game_menu_names();
    groups
        .robco_fun
        .into_iter()
        .chain(groups.other_games)
        .collect()
}

fn platform_binary_file_name(binary_stem: &str) -> OsString {
    #[cfg(target_os = "windows")]
    {
        return OsString::from(format!("{binary_stem}.exe"));
    }

    OsString::from(binary_stem)
}

fn sibling_binary_dirs(current_exe: &Path) -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Some(parent) = current_exe.parent() {
        dirs.push(parent.to_path_buf());
        if parent.file_name().and_then(|name| name.to_str()) == Some("deps") {
            if let Some(grandparent) = parent.parent() {
                dirs.push(grandparent.to_path_buf());
            }
        }
    }
    dirs
}

fn sibling_binary_path(binary_stem: &str) -> Option<PathBuf> {
    let current_exe = std::env::current_exe().ok()?;
    let file_name = platform_binary_file_name(binary_stem);
    sibling_binary_dirs(&current_exe)
        .into_iter()
        .map(|dir| dir.join(&file_name))
        .find(|candidate| candidate.is_file())
}

fn bundled_binary_path(binary_stem: &str) -> Option<PathBuf> {
    let candidate = bundled_bin_dir().join(platform_binary_file_name(binary_stem));
    candidate.is_file().then_some(candidate)
}

fn workspace_manifest_path() -> Option<PathBuf> {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("Cargo.toml");
    manifest.is_file().then_some(manifest)
}

fn built_in_game_argv(name: &str) -> Option<Vec<String>> {
    let (package, binary) = builtin_robco_fun_game_specs(name)?;
    if let Some(path) = bundled_binary_path(binary) {
        return Some(vec![path.to_string_lossy().to_string()]);
    }
    if let Some(path) = sibling_binary_path(binary) {
        return Some(vec![path.to_string_lossy().to_string()]);
    }
    if command_exists(binary) {
        return Some(vec![binary.to_string()]);
    }
    if let Some(manifest) = workspace_manifest_path() {
        return Some(vec![
            "cargo".to_string(),
            "run".to_string(),
            "--manifest-path".to_string(),
            manifest.to_string_lossy().to_string(),
            "-p".to_string(),
            package.to_string(),
            "--bin".to_string(),
            binary.to_string(),
        ]);
    }
    Some(vec![binary.to_string()])
}

fn resolve_builtin_game_launch(name: &str) -> Option<ResolvedProgramLaunch> {
    built_in_game_argv(name).map(|argv| ResolvedProgramLaunch {
        title: name.to_string(),
        argv,
    })
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
    match catalog {
        ProgramCatalog::Games => {
            let source = load_games();
            resolve_program_launch_from_source(name, &source).or_else(|_| {
                resolve_builtin_game_launch(name)
                    .ok_or_else(|| format!("Unknown program '{name}'."))
            })
        }
        ProgramCatalog::Applications | ProgramCatalog::Network => {
            resolve_program_launch_from_source(name, &load_catalog_source(catalog))
        }
    }
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

    #[test]
    fn robco_fun_game_names_are_stable() {
        assert_eq!(
            robco_fun_game_names(),
            vec![
                BUILTIN_ZETA_INVADERS_GAME.to_string(),
                BUILTIN_RED_MENACE_GAME.to_string()
            ]
        );
    }

    #[test]
    fn builtin_game_launch_builds_non_empty_command() {
        let launch = resolve_builtin_game_launch(BUILTIN_ZETA_INVADERS_GAME)
            .expect("expected zeta invaders launch command");

        assert_eq!(launch.title, BUILTIN_ZETA_INVADERS_GAME);
        assert!(!launch.argv.is_empty());
    }

    #[test]
    fn platform_binary_file_name_matches_current_platform_convention() {
        let name = platform_binary_file_name("robcos-settings");

        #[cfg(target_os = "windows")]
        assert_eq!(name, OsString::from("robcos-settings.exe"));
        #[cfg(not(target_os = "windows"))]
        assert_eq!(name, OsString::from("robcos-settings"));
    }
}
