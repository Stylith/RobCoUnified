use crate::config::diagnostics_log_file;
use std::io::Write;
use std::path::PathBuf;

const NUCLEON_DIAG_PATH_ENV: &str = "NUCLEON_DIAG_PATH";

fn diagnostics_path() -> PathBuf {
    std::env::var(NUCLEON_DIAG_PATH_ENV)
        .map(PathBuf::from)
        .unwrap_or_else(|_| diagnostics_log_file())
}

fn ensure_parent(path: &PathBuf) {
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
}

fn append_line(line: &str) {
    let path = diagnostics_path();
    ensure_parent(&path);
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
    {
        let _ = writeln!(file, "{line}");
    }
}

pub fn log(component: &str, message: &str) {
    let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
    append_line(&format!("[{now}] [{component}] {message}"));
}
