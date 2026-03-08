use std::io::Write;
use std::path::PathBuf;

fn diagnostics_path() -> PathBuf {
    if let Ok(path) = std::env::var("ROBCOS_DIAG_PATH") {
        return PathBuf::from(path);
    }
    if let Some(home) = std::env::var_os("HOME") {
        return PathBuf::from(home).join(".local/share/robcos/diagnostics.log");
    }
    std::env::temp_dir().join("robcos_diagnostics.log")
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
