use robcos_shared::config::{get_settings, load_about};
use sysinfo::System;

const DEFAULT_ASCII: &[&str] = &[
    "██████╗  ██████╗ ██████╗  ██████╗  ██████╗ ",
    "██╔══██╗██╔═══██╗██╔══██╗██╔════╝ ██╔═══██╗",
    "██████╔╝██║   ██║██████╔╝██║      ██║   ██║",
    "██╔══██╗██║   ██║██╔══██╗██║      ██║   ██║",
    "██║  ██║╚██████╔╝██████╔╝╚██████╗ ╚██████╔╝",
    "╚═╝  ╚═╝ ╚═════╝ ╚═════╝  ╚═════╝  ╚═════╝ ",
];

const DEFAULT_FIELDS: &[&str] = &["OS", "Hostname", "CPU", "RAM", "Uptime", "Battery", "Theme", "Shell"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalAboutRequest {
    None,
    Back,
}

pub fn about_ascii_and_fields() -> (Vec<String>, Vec<String>) {
    let config = load_about();
    let ascii = if config.ascii.is_empty() {
        DEFAULT_ASCII.iter().map(|s| s.to_string()).collect()
    } else {
        config.ascii
    };
    let fields = if config.fields.is_empty() {
        DEFAULT_FIELDS.iter().map(|s| s.to_string()).collect()
    } else {
        config.fields
    };
    (ascii, fields)
}

pub fn resolve_about_request(back: bool) -> TerminalAboutRequest {
    if back {
        TerminalAboutRequest::Back
    } else {
        TerminalAboutRequest::None
    }
}

pub fn get_system_info(fields: &[String]) -> Vec<(String, String)> {
    let mut sys = System::new_all();
    sys.refresh_all();

    let mut info = Vec::new();
    for field in fields {
        let val: String = match field.as_str() {
            "OS" => format!(
                "{} {}",
                System::name().unwrap_or_default(),
                System::os_version().unwrap_or_default()
            ),
            "Hostname" => System::host_name().unwrap_or_default(),
            "CPU" => sys
                .cpus()
                .first()
                .map(|c| c.brand().to_string())
                .unwrap_or_default(),
            "RAM" => {
                let used = sys.used_memory() / 1024 / 1024;
                let total = sys.total_memory() / 1024 / 1024;
                format!("{used} MB / {total} MB")
            }
            "Uptime" => {
                let secs = System::uptime();
                format!("{}h {}m", secs / 3600, (secs % 3600) / 60)
            }
            "Battery" => battery_str(),
            "Theme" => get_settings().theme,
            "Shell" => std::env::var("SHELL").unwrap_or_default(),
            "Rust" => format!("v{}", env!("CARGO_PKG_VERSION")),
            _ => continue,
        };
        info.push((field.clone(), val));
    }
    info
}

fn battery_str() -> String {
    if let Ok(rd) = std::fs::read_dir("/sys/class/power_supply") {
        for entry in rd.flatten() {
            let kind = std::fs::read_to_string(entry.path().join("type")).unwrap_or_default();
            if kind.trim() == "Battery" {
                let cap =
                    std::fs::read_to_string(entry.path().join("capacity")).unwrap_or_default();
                if let Ok(n) = cap.trim().parse::<u8>() {
                    return format!("{n}%");
                }
            }
        }
    }
    "N/A".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_about_request_maps_back_key() {
        assert_eq!(resolve_about_request(true), TerminalAboutRequest::Back);
        assert_eq!(resolve_about_request(false), TerminalAboutRequest::None);
    }

    #[test]
    fn get_system_info_skips_unknown_fields() {
        let fields = vec!["Rust".to_string(), "Unknown".to_string()];
        let info = get_system_info(&fields);
        assert_eq!(info.len(), 1);
        assert_eq!(info[0].0, "Rust");
    }
}
