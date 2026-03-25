use crate::config::{
    desktop_dir, home_dir_fallback as shared_home_dir_fallback, journal_entries_dir,
    word_processor_documents_dir,
};
use std::path::{Path, PathBuf};

pub fn home_dir_fallback() -> PathBuf {
    shared_home_dir_fallback()
}

pub fn word_processor_dir(username: &str) -> PathBuf {
    word_processor_documents_dir(username)
}

pub fn desktop_surface_dir() -> PathBuf {
    desktop_dir()
}

pub fn path_targets_desktop_surface(path: &Path) -> bool {
    let desktop_dir = desktop_surface_dir();
    path == desktop_dir || path.starts_with(&desktop_dir)
}

pub fn logs_dir() -> PathBuf {
    journal_entries_dir()
}

pub fn save_text_file(path: &PathBuf, text: &str) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, text)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn word_processor_dir_uses_shared_compatibility_path() {
        let dir = word_processor_dir("adi");
        assert!(dir.ends_with(
            std::path::Path::new("users")
                .join("adi")
                .join("documents")
                .join("word-processor")
        ));
    }

    #[test]
    fn path_targets_desktop_surface_matches_root_and_children() {
        let desktop = desktop_surface_dir();
        assert!(path_targets_desktop_surface(&desktop));
        assert!(path_targets_desktop_surface(&desktop.join("note.txt")));
        assert!(!path_targets_desktop_surface(std::path::Path::new(
            "/tmp/not-desktop"
        )));
    }
}
