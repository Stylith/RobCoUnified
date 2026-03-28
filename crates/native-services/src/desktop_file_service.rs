use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadedTextDocument {
    pub path: PathBuf,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileManagerLocation {
    pub cwd: PathBuf,
    pub selected: Option<PathBuf>,
}

pub fn load_text_document(path: PathBuf) -> Result<LoadedTextDocument, String> {
    std::fs::read_to_string(&path)
        .map(|text| LoadedTextDocument { path, text })
        .map_err(|err| err.to_string())
}

pub fn open_directory_location(path: PathBuf) -> Result<FileManagerLocation, String> {
    if !path.is_dir() {
        return Err(format!("Error: '{}' not found.", path.display()));
    }
    Ok(FileManagerLocation {
        cwd: path,
        selected: None,
    })
}

pub fn reveal_path_location(path: PathBuf) -> Result<FileManagerLocation, String> {
    if path.is_dir() {
        return open_directory_location(path);
    }
    if !path.exists() {
        return Err(format!("Error: '{}' not found.", path.display()));
    }
    let cwd = path
        .parent()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/"));
    Ok(FileManagerLocation {
        cwd,
        selected: Some(path),
    })
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
                "nucleon_native_desktop_file_service_{prefix}_{}_{}",
                std::process::id(),
                unique
            ));
            std::fs::create_dir_all(&path).expect("create temp test dir");
            Self { path }
        }
    }

    impl Drop for TempDirGuard {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }

    #[test]
    fn load_text_document_reads_existing_file() {
        let temp = TempDirGuard::new("load_text");
        let path = temp.path.join("demo.txt");
        std::fs::write(&path, "hello").expect("write temp file");

        let loaded = load_text_document(path.clone()).expect("load text document");

        assert_eq!(loaded.path, path);
        assert_eq!(loaded.text, "hello");
    }

    #[test]
    fn reveal_path_location_selects_file_in_parent_directory() {
        let temp = TempDirGuard::new("reveal_path");
        let path = temp.path.join("demo.txt");
        std::fs::write(&path, "hello").expect("write temp file");

        let location = reveal_path_location(path.clone()).expect("reveal path");

        assert_eq!(location.cwd, temp.path);
        assert_eq!(location.selected, Some(path));
    }

    #[test]
    fn open_directory_location_requires_existing_directory() {
        let missing = std::env::temp_dir().join("nucleon_native_missing_directory");
        let err = open_directory_location(missing.clone()).expect_err("missing directory");

        assert!(err.contains(&missing.display().to_string()));
    }
}
