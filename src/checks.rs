use std::process::Command;
use std::path::PathBuf;

#[derive(Debug)]
pub struct PreflightReport {
    pub ok: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

// CLI tools: (binary, description, optional)
const CLI_TOOLS: &[(&str, &str, bool)] = &[
    ("epy", "ebook reader", true),
    ("vim", "text editor (editing)", true),
    ("curl", "internet connectivity", false),
];

pub fn run_preflight() -> PreflightReport {
    let mut errors = Vec::new();
    let mut warnings = Vec::new();

    for (bin, desc, optional) in CLI_TOOLS {
        if !which(bin) {
            let msg = format!("'{bin}' not found ({desc})");
            if *optional {
                warnings.push(msg);
            } else {
                errors.push(msg);
            }
        }
    }

    if which("python3") {
        if !has_python_module("playsound3") {
            warnings.push(
                "'playsound3' Python module not found (optional, used for sound backend)"
                    .to_string(),
            );
        }
    } else {
        warnings.push("'python3' not found (optional, used for sound backend)".to_string());
    }

    PreflightReport {
        ok: errors.is_empty(),
        errors,
        warnings,
    }
}

fn which(bin: &str) -> bool {
    Command::new("which")
        .arg(bin)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn has_python_module(module: &str) -> bool {
    let code = format!("import {module}");
    let python = if cfg!(target_os = "linux") && is_arch_linux() {
        let Some(py) = arch_audio_python_bin() else {
            return false;
        };
        py.to_string_lossy().to_string()
    } else {
        "python3".to_string()
    };
    Command::new(&python)
        .args(["-c", code.as_str()])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

#[cfg(target_os = "linux")]
fn is_arch_linux() -> bool {
    std::path::Path::new("/etc/arch-release").exists()
        || std::fs::read_to_string("/etc/os-release")
            .map(|s| {
                let lower = s.to_lowercase();
                lower.contains("id=arch") || lower.contains("id_like=arch")
            })
            .unwrap_or(false)
}

#[cfg(not(target_os = "linux"))]
fn is_arch_linux() -> bool {
    false
}

fn arch_audio_python_bin() -> Option<PathBuf> {
    let home = std::env::var_os("HOME")?;
    let venv = PathBuf::from(home).join(".local/share/robcos/audio-venv");
    let py3 = venv.join("bin/python3");
    if py3.exists() {
        return Some(py3);
    }
    let py = venv.join("bin/python");
    if py.exists() {
        return Some(py);
    }
    None
}

pub fn print_preflight(report: &PreflightReport) {
    if !report.errors.is_empty() {
        eprintln!("\n╔══════════════════════════════════════════════════╗");
        eprintln!("║         RobcOS - Dependency Error                ║");
        eprintln!("╚══════════════════════════════════════════════════╝");
        for e in &report.errors {
            eprintln!("  ✗ {e}");
        }
    }
    if !report.warnings.is_empty() {
        eprintln!("\n╔══════════════════════════════════════════════════╗");
        eprintln!("║     RobcOS - Optional Dependencies Missing       ║");
        eprintln!("╚══════════════════════════════════════════════════╝");
        for w in &report.warnings {
            eprintln!("  ! {w}");
        }
    }
}
