use std::collections::HashSet;
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

static LOG_ONCE_KEYS: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();

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

pub fn log_once(key: &str, component: &str, message: &str) {
    let lock = LOG_ONCE_KEYS.get_or_init(|| Mutex::new(HashSet::new()));
    let Ok(mut set) = lock.lock() else {
        return;
    };
    if set.insert(key.to_string()) {
        drop(set);
        log(component, message);
    }
}

