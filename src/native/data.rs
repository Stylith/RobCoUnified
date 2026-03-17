use std::path::PathBuf;

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
    let dir = PathBuf::from("journal_entries");
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
