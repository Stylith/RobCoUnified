use crate::config::{load_categories, save_categories};
use serde_json::Value;
use std::path::PathBuf;

fn sorted_keys(data: &serde_json::Map<String, Value>) -> Vec<String> {
    let mut out = data.keys().cloned().collect::<Vec<_>>();
    out.sort_by_key(|name| name.to_ascii_lowercase());
    out
}

fn expand_tilde(raw: &str) -> PathBuf {
    if let Some(rest) = raw.strip_prefix('~') {
        if let Some(home) = dirs::home_dir() {
            return PathBuf::from(format!("{}{}", home.display(), rest));
        }
    }
    PathBuf::from(raw)
}

pub fn document_category_names() -> Vec<String> {
    sorted_keys(&load_categories())
}

pub fn document_category_entries() -> Vec<(String, PathBuf)> {
    let categories = load_categories();
    let mut entries = Vec::new();
    for name in sorted_keys(&categories) {
        if let Some(path) = categories.get(&name).and_then(|value| value.as_str()) {
            entries.push((name, PathBuf::from(path)));
        }
    }
    entries
}

pub fn document_category_path(name: &str) -> Option<PathBuf> {
    load_categories()
        .get(name)
        .and_then(|value| value.as_str())
        .map(PathBuf::from)
}

pub fn add_document_category(name: String, path_raw: &str) -> Result<String, String> {
    let expanded = expand_tilde(path_raw.trim());
    if !expanded.is_dir() {
        return Err("Error: Invalid directory.".to_string());
    }
    let mut categories = load_categories();
    categories.insert(name, Value::String(expanded.to_string_lossy().to_string()));
    save_categories(&categories);
    Ok("Category added.".to_string())
}

fn rename_document_category_in(
    categories: &mut serde_json::Map<String, Value>,
    old_name: &str,
    new_name: &str,
) -> Result<String, String> {
    let new_name = new_name.trim();
    if new_name.is_empty() {
        return Err("Name cannot be empty.".to_string());
    }
    if new_name == old_name {
        return Err("Name unchanged.".to_string());
    }
    if categories.contains_key(new_name) {
        return Err(format!("{new_name} already exists."));
    }

    let Some(entry) = categories.remove(old_name) else {
        return Err(format!("{old_name} was not found."));
    };
    categories.insert(new_name.to_string(), entry);
    Ok(format!("{old_name} renamed to {new_name}."))
}

pub fn rename_document_category(old_name: &str, new_name: &str) -> Result<String, String> {
    let mut categories = load_categories();
    let message = rename_document_category_in(&mut categories, old_name, new_name)?;
    save_categories(&categories);
    Ok(message)
}

pub fn delete_document_category(name: &str) -> String {
    let mut categories = load_categories();
    categories.remove(name);
    save_categories(&categories);
    "Deleted.".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{load_categories, save_categories};
    use serde_json::{Map, Value};
    use std::sync::{Mutex, OnceLock};

    fn documents_test_guard() -> std::sync::MutexGuard<'static, ()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
            .lock()
            .expect("desktop documents service test lock")
    }

    struct CategoriesRestore {
        backup: Map<String, Value>,
    }

    impl CategoriesRestore {
        fn capture() -> Self {
            Self {
                backup: load_categories(),
            }
        }
    }

    impl Drop for CategoriesRestore {
        fn drop(&mut self) {
            save_categories(&self.backup);
        }
    }

    #[test]
    fn document_category_names_are_sorted_case_insensitively() {
        let _guard = documents_test_guard();
        let _restore = CategoriesRestore::capture();
        let mut categories = Map::new();
        categories.insert("zeta".to_string(), Value::String("/tmp/zeta".to_string()));
        categories.insert("Alpha".to_string(), Value::String("/tmp/alpha".to_string()));
        save_categories(&categories);

        assert_eq!(
            document_category_names(),
            vec!["Alpha".to_string(), "zeta".to_string()]
        );
    }

    #[test]
    fn rename_document_category_rejects_duplicates() {
        let _guard = documents_test_guard();
        let _restore = CategoriesRestore::capture();
        let mut categories = Map::new();
        categories.insert("Docs".to_string(), Value::String("/tmp/docs".to_string()));
        categories.insert("Logs".to_string(), Value::String("/tmp/logs".to_string()));

        let err = rename_document_category_in(&mut categories, "Docs", "Logs")
            .expect_err("duplicate name");
        assert_eq!(err, "Logs already exists.");
    }
}
