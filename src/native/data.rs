use crate::config::{base_dir, state_root_dir};
use std::path::{Path, PathBuf};

pub fn home_dir_fallback() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
}

pub fn documents_dir() -> PathBuf {
    dirs::document_dir().unwrap_or_else(home_dir_fallback)
}

pub fn word_processor_dir(username: &str) -> PathBuf {
    let dir = documents_dir().join("ROBCO Word Processor").join(username);
    let _ = std::fs::create_dir_all(&dir);
    dir
}

pub fn logs_dir() -> PathBuf {
    let dir = state_root_dir().join("journal_entries");
    let legacy_dir = base_dir().join("journal_entries");
    migrate_directory_tree_if_needed(&dir, &legacy_dir);
    let _ = std::fs::create_dir_all(&dir);
    dir
}

pub fn save_text_file(path: &PathBuf, text: &str) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, text)?;
    Ok(())
}

fn migrate_directory_tree_if_needed(target_dir: &Path, legacy_dir: &Path) {
    if target_dir == legacy_dir || target_dir.exists() || !legacy_dir.is_dir() {
        return;
    }
    copy_directory_tree(legacy_dir, target_dir);
}

fn copy_directory_tree(source_dir: &Path, target_dir: &Path) {
    let _ = std::fs::create_dir_all(target_dir);
    let Ok(entries) = std::fs::read_dir(source_dir) else {
        return;
    };
    for entry in entries.flatten() {
        let source = entry.path();
        let target = target_dir.join(entry.file_name());
        if source.is_dir() {
            copy_directory_tree(&source, &target);
        } else if !target.exists() {
            if let Some(parent) = target.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            let _ = std::fs::copy(&source, &target);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TempDirGuard {
        path: PathBuf,
    }

    impl TempDirGuard {
        fn new(prefix: &str) -> Self {
            let unique = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("test clock")
                .as_nanos();
            let path = std::env::temp_dir().join(format!(
                "robco_native_data_{prefix}_{}_{}",
                std::process::id(),
                unique
            ));
            std::fs::create_dir_all(&path).expect("create temp dir");
            Self { path }
        }
    }

    impl Drop for TempDirGuard {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }

    #[test]
    fn migrate_directory_tree_if_needed_copies_legacy_files_once() {
        let temp = TempDirGuard::new("migrate_logs");
        let legacy = temp.path.join("legacy");
        let target = temp.path.join("target");
        std::fs::create_dir_all(&legacy).expect("create legacy dir");
        std::fs::write(legacy.join("a.txt"), "alpha").expect("write legacy file");
        std::fs::create_dir_all(legacy.join("nested")).expect("create nested dir");
        std::fs::write(legacy.join("nested").join("b.txt"), "beta").expect("write nested file");

        migrate_directory_tree_if_needed(&target, &legacy);

        assert_eq!(
            std::fs::read_to_string(target.join("a.txt")).expect("read copied file"),
            "alpha"
        );
        assert_eq!(
            std::fs::read_to_string(target.join("nested").join("b.txt"))
                .expect("read nested copied file"),
            "beta"
        );

        std::fs::write(target.join("a.txt"), "new").expect("overwrite target file");
        migrate_directory_tree_if_needed(&target, &legacy);
        assert_eq!(
            std::fs::read_to_string(target.join("a.txt")).expect("read target file"),
            "new"
        );
    }
}
