use std::process::Command;

#[derive(Debug)]
pub struct PreflightReport {
    pub ok:       bool,
    pub errors:   Vec<String>,
    pub warnings: Vec<String>,
}

// CLI tools: (binary, description, optional)
const CLI_TOOLS: &[(&str, &str, bool)] = &[
    ("tmux", "multi-window support",  false),
    ("epy",  "ebook reader",          true),
    ("vim",  "text editor (editing)", true),
    ("curl", "internet connectivity", false),
];

pub fn run_preflight() -> PreflightReport {
    let mut errors   = Vec::new();
    let mut warnings = Vec::new();

    for (bin, desc, optional) in CLI_TOOLS {
        if !which(bin) {
            let msg = format!("'{bin}' not found ({desc})");
            if *optional { warnings.push(msg); } else { errors.push(msg); }
        }
    }

    if which("python3") {
        if !has_python_module("playsound") {
            warnings.push(
                "'playsound' Python module not found (optional, used for legacy sound backend)"
                    .to_string(),
            );
        }
    } else {
        warnings.push("'python3' not found (optional, used for playsound backend)".to_string());
    }

    PreflightReport { ok: errors.is_empty(), errors, warnings }
}

fn which(bin: &str) -> bool {
    Command::new("which").arg(bin).output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn has_python_module(module: &str) -> bool {
    let code = format!("import {module}");
    Command::new("python3")
        .args(["-c", code.as_str()])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

#[allow(dead_code)]
pub fn has_tmux() -> bool { which("tmux") }
#[allow(dead_code)]
pub fn in_tmux()  -> bool { std::env::var("TMUX").is_ok() }

pub fn print_preflight(report: &PreflightReport) {
    if !report.errors.is_empty() {
        eprintln!("\n╔══════════════════════════════════════════════════╗");
        eprintln!("║         RobcOS - Dependency Error                ║");
        eprintln!("╚══════════════════════════════════════════════════╝");
        for e in &report.errors   { eprintln!("  ✗ {e}"); }
    }
    if !report.warnings.is_empty() {
        eprintln!("\n╔══════════════════════════════════════════════════╗");
        eprintln!("║     RobcOS - Optional Dependencies Missing       ║");
        eprintln!("╚══════════════════════════════════════════════════╝");
        for w in &report.warnings { eprintln!("  ! {w}"); }
    }
}
