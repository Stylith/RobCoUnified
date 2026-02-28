use anyhow::Result;
use serde_json::{Map, Value};

use crate::config::{
    get_settings, load_apps, load_games, load_networks, persist_settings, save_apps, save_games,
    save_networks, update_settings,
};
use crate::launcher::{json_to_cmd, launch_in_pty};
use crate::ui::{
    confirm, flash_message, input_prompt, is_back_menu_label, run_menu, MenuResult, Term,
};

const BUILTIN_NUKE_CODES_APP: &str = "Nuke Codes";

// ── Generic add/delete ────────────────────────────────────────────────────────

fn add_entry<L, S>(terminal: &mut Term, kind: &str, mut load: L, mut save: S) -> Result<()>
where
    L: FnMut() -> Map<String, Value>,
    S: FnMut(&Map<String, Value>),
{
    let name = match input_prompt(terminal, &format!("Enter {kind} display name:"))? {
        Some(n) if !n.is_empty() => n,
        _ => {
            flash_message(terminal, "Error: Invalid input.", 800)?;
            return Ok(());
        }
    };
    let cmd_str = match input_prompt(terminal, &format!("Enter launch command for '{name}':"))? {
        Some(c) if !c.is_empty() => c,
        _ => {
            flash_message(terminal, "Error: Invalid input.", 800)?;
            return Ok(());
        }
    };
    // Split command into array
    let parts: Vec<Value> = cmd_str
        .split_whitespace()
        .map(|s| Value::String(s.to_string()))
        .collect();
    let mut data = load();
    data.insert(name.clone(), Value::Array(parts));
    save(&data);
    flash_message(terminal, &format!("{name} added."), 800)
}

fn delete_entry<L, S>(terminal: &mut Term, kind: &str, mut load: L, mut save: S) -> Result<()>
where
    L: FnMut() -> Map<String, Value>,
    S: FnMut(&Map<String, Value>),
{
    let data = load();
    if data.is_empty() {
        return flash_message(terminal, &format!("Error: {kind} list is empty."), 800);
    }
    let keys: Vec<String> = data.keys().cloned().collect();
    let mut opts: Vec<String> = keys.clone();
    opts.push("Back".to_string());
    let opts_ref: Vec<&str> = opts.iter().map(String::as_str).collect();

    if let MenuResult::Selected(sel) =
        run_menu(terminal, &format!("Delete {kind}"), &opts_ref, None)?
    {
        if sel != "Back" && data.contains_key(&sel) {
            if confirm(terminal, &format!("Delete '{sel}'?"))? {
                let mut data = load();
                data.remove(&sel);
                save(&data);
                flash_message(terminal, &format!("{sel} deleted."), 800)?;
            } else {
                flash_message(terminal, "Cancelled.", 600)?;
            }
        }
    }
    Ok(())
}

// ── Apps ──────────────────────────────────────────────────────────────────────

pub fn apps_menu(terminal: &mut Term) -> Result<()> {
    loop {
        if crate::session::has_switch_request() {
            break;
        }
        let apps = load_apps();
        let nuke_codes_visible = get_settings().builtin_menu_visibility.nuke_codes;
        let mut choices: Vec<String> = Vec::new();
        if nuke_codes_visible {
            choices.push(BUILTIN_NUKE_CODES_APP.to_string());
        }
        choices.extend(
            apps.keys()
                .filter(|name| name.as_str() != BUILTIN_NUKE_CODES_APP)
                .cloned(),
        );
        choices.push("---".to_string());
        choices.push("Back".to_string());
        let opts: Vec<&str> = choices.iter().map(String::as_str).collect();

        match run_menu(terminal, "Applications", &opts, Some("Select App"))? {
            MenuResult::Back => break,
            MenuResult::Selected(s) if s == "Back" => break,
            MenuResult::Selected(s) if s == BUILTIN_NUKE_CODES_APP => {
                crate::nuke_codes::nuke_codes_screen(terminal)?;
                if crate::session::has_switch_request() {
                    break;
                }
            }
            MenuResult::Selected(s) => {
                if let Some(v) = apps.get(&s) {
                    launch_in_pty(terminal, &json_to_cmd(v))?;
                    if crate::session::has_switch_request() {
                        break;
                    }
                }
            }
        }
    }
    Ok(())
}

pub fn games_menu(terminal: &mut Term) -> Result<()> {
    loop {
        if crate::session::has_switch_request() {
            break;
        }
        let games = load_games();
        let mut choices: Vec<String> = games.keys().cloned().collect();
        choices.push("---".to_string());
        choices.push("Back".to_string());
        let opts: Vec<&str> = choices.iter().map(String::as_str).collect();

        match run_menu(terminal, "Games", &opts, Some("Select Game"))? {
            MenuResult::Back => break,
            MenuResult::Selected(s) if s == "Back" => break,
            MenuResult::Selected(s) => {
                if let Some(v) = games.get(&s) {
                    launch_in_pty(terminal, &json_to_cmd(v))?;
                    if crate::session::has_switch_request() {
                        break;
                    }
                }
            }
        }
    }
    Ok(())
}

pub fn network_menu(terminal: &mut Term) -> Result<()> {
    loop {
        if crate::session::has_switch_request() {
            break;
        }
        let nets = load_networks();
        let mut choices: Vec<String> = nets.keys().cloned().collect();
        choices.push("---".to_string());
        choices.push("Back".to_string());
        let opts: Vec<&str> = choices.iter().map(String::as_str).collect();

        match run_menu(terminal, "Network", &opts, Some("Select Network Program"))? {
            MenuResult::Back => break,
            MenuResult::Selected(s) if s == "Back" => break,
            MenuResult::Selected(s) => {
                if let Some(v) = nets.get(&s) {
                    launch_in_pty(terminal, &json_to_cmd(v))?;
                    if crate::session::has_switch_request() {
                        break;
                    }
                }
            }
        }
    }
    Ok(())
}

// ── Edit menus ────────────────────────────────────────────────────────────────

pub fn edit_apps_menu(terminal: &mut Term) -> Result<()> {
    loop {
        let nuke_codes_label = if get_settings().builtin_menu_visibility.nuke_codes {
            "Nuke Codes in Applications: VISIBLE [toggle]"
        } else {
            "Nuke Codes in Applications: HIDDEN [toggle]"
        };
        match run_menu(
            terminal,
            "Edit Applications",
            &[
                nuke_codes_label,
                "---",
                "Add App",
                "Delete App",
                "---",
                "Back",
            ],
            None,
        )? {
            MenuResult::Back => break,
            MenuResult::Selected(s) => match s.as_str() {
                s if is_back_menu_label(s) => break,
                l if l == nuke_codes_label => {
                    update_settings(|cfg| {
                        cfg.builtin_menu_visibility.nuke_codes =
                            !cfg.builtin_menu_visibility.nuke_codes;
                    });
                    persist_settings();
                }
                "Add App" => add_entry(terminal, "App", load_apps, save_apps)?,
                "Delete App" => delete_entry(terminal, "App", load_apps, save_apps)?,
                _ => break,
            },
        }
    }
    Ok(())
}

pub fn edit_games_menu(terminal: &mut Term) -> Result<()> {
    loop {
        match run_menu(
            terminal,
            "Edit Games",
            &["Add Game", "Delete Game", "---", "Back"],
            None,
        )? {
            MenuResult::Back => break,
            MenuResult::Selected(s) => match s.as_str() {
                s if is_back_menu_label(s) => break,
                "Add Game" => add_entry(terminal, "Game", load_games, save_games)?,
                "Delete Game" => delete_entry(terminal, "Game", load_games, save_games)?,
                _ => break,
            },
        }
    }
    Ok(())
}

pub fn edit_network_menu(terminal: &mut Term) -> Result<()> {
    loop {
        match run_menu(
            terminal,
            "Edit Network",
            &["Add Network", "Delete Network", "---", "Back"],
            None,
        )? {
            MenuResult::Back => break,
            MenuResult::Selected(s) => match s.as_str() {
                s if is_back_menu_label(s) => break,
                "Add Network" => {
                    add_entry(terminal, "Network Program", load_networks, save_networks)?
                }
                "Delete Network" => {
                    delete_entry(terminal, "Network Program", load_networks, save_networks)?
                }
                _ => break,
            },
        }
    }
    Ok(())
}

pub fn edit_menus_menu(terminal: &mut Term) -> Result<()> {
    use crate::docedit::edit_documents_menu;
    loop {
        match run_menu(
            terminal,
            "Edit Menus",
            &[
                "Edit Applications",
                "Edit Documents",
                "Edit Network",
                "Edit Games",
                "---",
                "Back",
            ],
            None,
        )? {
            MenuResult::Back => break,
            MenuResult::Selected(s) => match s.as_str() {
                s if is_back_menu_label(s) => break,
                "Edit Applications" => edit_apps_menu(terminal)?,
                "Edit Documents" => edit_documents_menu(terminal)?,
                "Edit Network" => edit_network_menu(terminal)?,
                "Edit Games" => edit_games_menu(terminal)?,
                _ => break,
            },
        }
    }
    Ok(())
}
