use anyhow::Result;
use serde_json::Value;
use std::path::Path;

use crate::config::{load_categories, save_categories};
use crate::ui::{Term, run_menu, input_prompt, confirm, flash_message, MenuResult};

pub fn edit_documents_menu(terminal: &mut Term) -> Result<()> {
    loop {
        match run_menu(
            terminal, "Edit Documents",
            &["Add Category", "Delete Category", "---", "Back"], None,
        )? {
            MenuResult::Back => break,
            MenuResult::Selected(s) => match s.as_str() {
                "Add Category"    => add_category(terminal)?,
                "Delete Category" => delete_category(terminal)?,
                _                 => break,
            }
        }
    }
    Ok(())
}

fn add_category(terminal: &mut Term) -> Result<()> {
    let name = match input_prompt(terminal, "Enter category name:")? {
        Some(n) if !n.is_empty() => n,
        _ => { return flash_message(terminal, "Error: Invalid input.", 800); }
    };
    let path_str = match input_prompt(terminal, "Enter folder path:")? {
        Some(p) if !p.is_empty() => p,
        _ => { return flash_message(terminal, "Error: Invalid input.", 800); }
    };
    let expanded = shellexpand_tilde(&path_str);
    let path = Path::new(&expanded);
    if !path.exists() || !path.is_dir() {
        return flash_message(terminal, "Error: Invalid directory.", 800);
    }
    let mut cats = load_categories();
    cats.insert(name, Value::String(expanded));
    save_categories(&cats);
    flash_message(terminal, "Category added.", 800)
}

fn delete_category(terminal: &mut Term) -> Result<()> {
    let cats = load_categories();
    if cats.is_empty() {
        return flash_message(terminal, "Error: No categories to delete.", 800);
    }
    let mut opts: Vec<String> = cats.keys().cloned().collect();
    opts.push("Back".to_string());
    let opts_ref: Vec<&str> = opts.iter().map(String::as_str).collect();

    if let MenuResult::Selected(sel) = run_menu(terminal, "Delete Category", &opts_ref, None)? {
        if sel != "Back" && cats.contains_key(&sel) {
            if confirm(terminal, &format!("Delete category '{sel}'?"))? {
                let mut cats = load_categories();
                cats.remove(&sel);
                save_categories(&cats);
                flash_message(terminal, "Deleted.", 800)?;
            } else {
                flash_message(terminal, "Cancelled.", 600)?;
            }
        }
    }
    Ok(())
}

/// Simple ~ expansion (no full shell expansion for safety).
fn shellexpand_tilde(s: &str) -> String {
    if let Some(rest) = s.strip_prefix('~') {
        if let Some(home) = dirs::home_dir() {
            return format!("{}{}", home.display(), rest);
        }
    }
    s.to_string()
}
