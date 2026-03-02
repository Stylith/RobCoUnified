use anyhow::Result;
use std::process::{Command, Stdio};

use crate::auth::is_admin;
use crate::config::{
    get_current_user, load_apps, load_games, load_networks, save_apps, save_games, save_networks,
};
use crate::launcher::with_suspended;
use crate::ui::{
    box_message, confirm, flash_message, input_prompt, is_back_menu_label, run_menu, MenuResult,
    Term,
};

// ── Package manager detection ─────────────────────────────────────────────────

#[derive(Debug, Clone, Copy)]
enum PackageManager {
    Brew,
    Apt,
    Dnf,
    Pacman,
    Zypper,
}

impl PackageManager {
    fn detect() -> Option<Self> {
        let pms: &[(&str, PackageManager)] = &[
            ("brew", PackageManager::Brew),
            ("apt", PackageManager::Apt),
            ("apt-get", PackageManager::Apt),
            ("dnf", PackageManager::Dnf),
            ("pacman", PackageManager::Pacman),
            ("zypper", PackageManager::Zypper),
        ];
        for (bin, pm) in pms {
            if which(bin) {
                return Some(*pm);
            }
        }
        None
    }

    fn name(self) -> &'static str {
        match self {
            PackageManager::Brew => "brew",
            PackageManager::Apt => "apt",
            PackageManager::Dnf => "dnf",
            PackageManager::Pacman => "pacman",
            PackageManager::Zypper => "zypper",
        }
    }

    fn install_cmd(self, pkg: &str) -> Vec<String> {
        match self {
            PackageManager::Brew => vec!["brew".into(), "install".into(), pkg.into()],
            PackageManager::Apt => vec![
                "sudo".into(),
                "apt".into(),
                "install".into(),
                "-y".into(),
                pkg.into(),
            ],
            PackageManager::Dnf => vec![
                "sudo".into(),
                "dnf".into(),
                "install".into(),
                "-y".into(),
                pkg.into(),
            ],
            PackageManager::Pacman => vec![
                "sudo".into(),
                "pacman".into(),
                "-S".into(),
                "--noconfirm".into(),
                pkg.into(),
            ],
            PackageManager::Zypper => vec![
                "sudo".into(),
                "zypper".into(),
                "-n".into(),
                "install".into(),
                pkg.into(),
            ],
        }
    }

    fn remove_cmd(self, pkg: &str) -> Vec<String> {
        match self {
            PackageManager::Brew => vec!["brew".into(), "uninstall".into(), pkg.into()],
            PackageManager::Apt => vec![
                "sudo".into(),
                "apt".into(),
                "remove".into(),
                "-y".into(),
                pkg.into(),
            ],
            PackageManager::Dnf => vec![
                "sudo".into(),
                "dnf".into(),
                "remove".into(),
                "-y".into(),
                pkg.into(),
            ],
            PackageManager::Pacman => vec![
                "sudo".into(),
                "pacman".into(),
                "-R".into(),
                "--noconfirm".into(),
                pkg.into(),
            ],
            PackageManager::Zypper => vec![
                "sudo".into(),
                "zypper".into(),
                "-n".into(),
                "remove".into(),
                pkg.into(),
            ],
        }
    }

    fn update_cmd(self, pkg: &str) -> Vec<String> {
        match self {
            PackageManager::Brew => vec!["brew".into(), "upgrade".into(), pkg.into()],
            PackageManager::Apt => vec![
                "sudo".into(),
                "apt".into(),
                "upgrade".into(),
                "-y".into(),
                pkg.into(),
            ],
            PackageManager::Dnf => vec![
                "sudo".into(),
                "dnf".into(),
                "upgrade".into(),
                "-y".into(),
                pkg.into(),
            ],
            PackageManager::Pacman => vec!["sudo".into(), "pacman".into(), "-U".into(), pkg.into()],
            PackageManager::Zypper => vec![
                "sudo".into(),
                "zypper".into(),
                "-n".into(),
                "update".into(),
                pkg.into(),
            ],
        }
    }

    fn search(self, query: &str) -> Vec<String> {
        let out = match self {
            PackageManager::Brew => Command::new("brew").args(["search", query]).output().ok(),
            PackageManager::Apt => Command::new("apt-cache")
                .args(["search", query])
                .output()
                .ok(),
            PackageManager::Dnf => Command::new("dnf").args(["search", query]).output().ok(),
            PackageManager::Pacman => Command::new("pacman").args(["-Ss", query]).output().ok(),
            PackageManager::Zypper => Command::new("zypper").args(["se", query]).output().ok(),
        }
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
        .unwrap_or_default();
        out.lines()
            .filter(|l| {
                let line = l.trim_end();
                !line.is_empty()
                    && !line.starts_with('=')
                    && !line.starts_with("warning:")
                    && !line.starts_with("Sorting...")
                    && !line.starts_with("Full Text Search...")
                    && !line.starts_with("S | Name")
                    && !line.starts_with("--")
                    && !(matches!(self, PackageManager::Pacman) && l.starts_with(' '))
            })
            .map(str::to_string)
            .collect()
    }

    fn list_installed(self) -> Vec<String> {
        let (bin, args): (&str, &[&str]) = match self {
            PackageManager::Brew => ("brew", &["list"]),
            PackageManager::Apt => ("apt", &["list", "--installed"]),
            PackageManager::Dnf => ("dnf", &["list", "installed"]),
            PackageManager::Pacman => ("pacman", &["-Q"]),
            PackageManager::Zypper => ("zypper", &["se", "--installed-only"]),
        };
        Command::new(bin)
            .args(args)
            .output()
            .ok()
            .map(|o| {
                String::from_utf8_lossy(&o.stdout)
                    .lines()
                    .filter(|l| {
                        !l.trim().is_empty()
                            && !l.starts_with("Listing")
                            && !l.starts_with("WARNING")
                    })
                    .map(|l| {
                        l.split_whitespace()
                            .next()
                            .unwrap_or("")
                            .split('/')
                            .next()
                            .unwrap_or("")
                            .to_string()
                    })
                    .filter(|s| !s.is_empty())
                    .collect()
            })
            .unwrap_or_default()
    }
}

fn which(bin: &str) -> bool {
    std::process::Command::new("which")
        .arg(bin)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn has_internet() -> bool {
    Command::new("curl")
        .args(["-s", "--max-time", "3", "https://www.google.com"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn has_python_module(module: &str) -> bool {
    if !which("python3") {
        return false;
    }
    let code = format!("import {module}");
    Command::new("python3")
        .args(["-c", code.as_str()])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn install_playsound_runtime(terminal: &mut Term) -> Result<()> {
    if !which("python3") {
        return flash_message(terminal, "python3 not found. Install Python first.", 1200);
    }
    if has_python_module("playsound") {
        return flash_message(terminal, "playsound is already installed.", 1000);
    }
    if !has_internet() {
        return flash_message(terminal, "Error: No internet connection.", 1000);
    }
    if !confirm(terminal, "Install Python package: playsound?")? {
        return Ok(());
    }

    let pip_args = ["-m", "pip", "install", "--user", "--upgrade", "playsound"];
    let mut install_ok = false;
    let run_result = with_suspended(terminal, || {
        let first = Command::new("python3").args(pip_args).status()?;
        if first.success() {
            install_ok = true;
            return Ok(());
        }

        let _ = Command::new("python3")
            .args(["-m", "ensurepip", "--upgrade"])
            .status();
        let retry = Command::new("python3").args(pip_args).status()?;
        install_ok = retry.success();
        Ok(())
    });

    if run_result.is_err() {
        return flash_message(terminal, "Failed to run pip install.", 1400);
    }

    if install_ok && has_python_module("playsound") {
        box_message(terminal, "playsound installed.", 1000)?;
    } else {
        flash_message(
            terminal,
            "Install completed with errors. Run: python3 -m pip install --user playsound",
            2200,
        )?;
    }
    Ok(())
}

fn install_blueutil_runtime(terminal: &mut Term) -> Result<()> {
    if !cfg!(target_os = "macos") {
        return flash_message(
            terminal,
            "blueutil installer is available on macOS only.",
            1200,
        );
    }
    if which("blueutil") {
        return flash_message(terminal, "blueutil is already installed.", 1000);
    }
    if !which("brew") {
        return flash_message(terminal, "Homebrew not found. Install brew first.", 1200);
    }
    if !has_internet() {
        return flash_message(terminal, "Error: No internet connection.", 1000);
    }
    if !confirm(terminal, "Install blueutil via Homebrew?")? {
        return Ok(());
    }

    let mut install_ok = false;
    let run_result = with_suspended(terminal, || {
        let status = Command::new("brew")
            .args(["install", "blueutil"])
            .status()?;
        install_ok = status.success();
        Ok(())
    });
    if run_result.is_err() {
        return flash_message(terminal, "Failed to run brew install.", 1400);
    }

    if install_ok && which("blueutil") {
        box_message(terminal, "blueutil installed.", 1000)?;
    } else {
        flash_message(
            terminal,
            "Install completed with errors. Run: brew install blueutil",
            2200,
        )?;
    }
    Ok(())
}

// ── Appstore menu ─────────────────────────────────────────────────────────────

pub fn appstore_menu(terminal: &mut Term) -> Result<()> {
    let user = get_current_user().unwrap_or_default();
    if !is_admin(&user) {
        return flash_message(terminal, "Access denied. Admin only.", 1000);
    }

    let pm = PackageManager::detect();
    let pm_label = pm.map(|p| p.name()).unwrap_or("Not Found");

    loop {
        let mut choices = vec![
            "Search".to_string(),
            "Installed Apps".to_string(),
            "Install Audio Runtime (playsound)".to_string(),
        ];
        if cfg!(target_os = "macos") {
            choices.push("Install Bluetooth Utility (blueutil)".to_string());
        }
        choices.push("---".to_string());
        choices.push("Back".to_string());
        let refs: Vec<&str> = choices.iter().map(String::as_str).collect();

        match run_menu(
            terminal,
            "Program Installer",
            &refs,
            Some(&format!("Package Manager: {pm_label}")),
        )? {
            MenuResult::Back => break,
            MenuResult::Selected(s) => match s.as_str() {
                s if is_back_menu_label(s) => break,
                "Search" => search_menu(terminal, pm)?,
                "Installed Apps" => installed_menu(terminal, pm)?,
                "Install Audio Runtime (playsound)" => install_playsound_runtime(terminal)?,
                "Install Bluetooth Utility (blueutil)" => install_blueutil_runtime(terminal)?,
                _ => break,
            },
        }
    }
    Ok(())
}

fn search_menu(terminal: &mut Term, pm: Option<PackageManager>) -> Result<()> {
    let query = match input_prompt(terminal, "Search packages:")? {
        Some(q) if !q.is_empty() => q,
        _ => return Ok(()),
    };
    if !has_internet() {
        return flash_message(terminal, "Error: No internet connection.", 1000);
    }
    flash_message(terminal, "Searching...", 500)?;

    let results = pm.map(|p| p.search(&query)).unwrap_or_default();
    if results.is_empty() {
        return flash_message(terminal, "No results found.", 800);
    }

    let mut page = 0usize;
    let page_size = 20usize;

    loop {
        let start = page * page_size;
        let end = (start + page_size).min(results.len());
        let total_pages = results.len().div_ceil(page_size);

        let mut choices: Vec<String> = results[start..end]
            .iter()
            .map(|r| {
                let cmd = r.split_whitespace().next().unwrap_or("");
                let status = if which(cmd) {
                    "[installed]"
                } else {
                    "[get]      "
                };
                format!("{status} {r}")
            })
            .collect();
        if page > 0 {
            choices.push("< Prev Page".into());
        }
        if end < results.len() {
            choices.push("> Next Page".into());
        }
        choices.push("---".into());
        choices.push("Back".into());
        let opts: Vec<&str> = choices.iter().map(String::as_str).collect();

        match run_menu(
            terminal,
            "Search Results",
            &opts,
            Some(&format!("Query: {query}  Page {}/{total_pages}", page + 1)),
        )? {
            MenuResult::Back => break,
            MenuResult::Selected(s) if s == "Back" => break,
            MenuResult::Selected(s) if s == "> Next Page" => {
                page += 1;
            }
            MenuResult::Selected(s) if s == "< Prev Page" => {
                page = page.saturating_sub(1);
            }
            MenuResult::Selected(s) => {
                let token = s.split_whitespace().nth(1).unwrap_or("");
                let pkg = token
                    .split_once('/')
                    .map(|(_, name)| name)
                    .unwrap_or(token)
                    .to_string();
                install_pkg_dialog(terminal, pm, &pkg)?;
            }
        }
    }
    Ok(())
}

fn install_pkg_dialog(terminal: &mut Term, pm: Option<PackageManager>, pkg: &str) -> Result<()> {
    if which(pkg) {
        return flash_message(terminal, &format!("{pkg} is already installed."), 800);
    }
    let Some(pm) = pm else {
        return flash_message(terminal, "Error: No supported package manager found.", 800);
    };
    if confirm(terminal, &format!("Install {pkg}?"))? {
        let cmd = pm.install_cmd(pkg);
        with_suspended(terminal, || {
            Command::new(&cmd[0]).args(&cmd[1..]).status()?;
            Ok(())
        })?;
        box_message(terminal, &format!("{pkg} installed."), 1500)?;
    }
    Ok(())
}

fn installed_menu(terminal: &mut Term, pm: Option<PackageManager>) -> Result<()> {
    flash_message(terminal, "Loading...", 400)?;
    let mut installed = pm.map(|p| p.list_installed()).unwrap_or_default();

    let mut page = 0usize;
    let mut filter = String::new();
    let page_size = 20usize;

    loop {
        let filtered: Vec<&String> = installed
            .iter()
            .filter(|p| filter.is_empty() || p.to_lowercase().contains(&filter.to_lowercase()))
            .collect();
        let total = filtered.len();
        let start = (page * page_size).min(total);
        let end = (start + page_size).min(total);
        let total_pages = (total + page_size - 1).max(1) / page_size;

        let search_label = if filter.is_empty() {
            "Search...".to_string()
        } else {
            format!("Search: {filter}")
        };
        let mut choices = vec![search_label.clone(), "---".into()];
        choices.extend(filtered[start..end].iter().map(|p| format!("  {p}")));
        if page > 0 {
            choices.push("< Prev Page".into());
        }
        if end < total {
            choices.push("> Next Page".into());
        }
        choices.push("---".into());
        choices.push("Back".into());
        let opts: Vec<&str> = choices.iter().map(String::as_str).collect();

        match run_menu(
            terminal,
            "Installed Apps",
            &opts,
            Some(&format!(
                "{total} packages   Page {}/{total_pages}",
                page + 1
            )),
        )? {
            MenuResult::Back => break,
            MenuResult::Selected(s) if s == "Back" => break,
            MenuResult::Selected(s) if s == "> Next Page" => {
                page += 1;
            }
            MenuResult::Selected(s) if s == "< Prev Page" => {
                page = page.saturating_sub(1);
            }
            MenuResult::Selected(s) if s == search_label => {
                filter = input_prompt(terminal, "Filter:")?.unwrap_or_default();
                page = 0;
            }
            MenuResult::Selected(s) => {
                let pkg = s.trim().to_string();
                pkg_action_menu(terminal, pm, &pkg, &mut installed)?;
            }
        }
    }
    Ok(())
}

fn pkg_action_menu(
    terminal: &mut Term,
    pm: Option<PackageManager>,
    pkg: &str,
    installed: &mut Vec<String>,
) -> Result<()> {
    loop {
        match run_menu(
            terminal,
            pkg,
            &["Update", "Uninstall", "Add to Menu", "---", "Back"],
            None,
        )? {
            MenuResult::Back => break,
            MenuResult::Selected(s) => match s.as_str() {
                s if is_back_menu_label(s) => break,
                "Update" => {
                    if let Some(pm) = pm {
                        if !has_internet() {
                            flash_message(terminal, "No internet.", 800)?;
                            continue;
                        }
                        if confirm(terminal, &format!("Update {pkg}?"))? {
                            let cmd = pm.update_cmd(pkg);
                            with_suspended(terminal, || {
                                Command::new(&cmd[0]).args(&cmd[1..]).status()?;
                                Ok(())
                            })?;
                            box_message(terminal, &format!("{pkg} updated."), 1000)?;
                        }
                    }
                }
                "Uninstall" => {
                    if let Some(pm) = pm {
                        if confirm(terminal, &format!("Uninstall {pkg}?"))? {
                            let cmd = pm.remove_cmd(pkg);
                            with_suspended(terminal, || {
                                Command::new(&cmd[0]).args(&cmd[1..]).status()?;
                                Ok(())
                            })?;
                            box_message(terminal, &format!("{pkg} uninstalled."), 1000)?;
                            installed.retain(|p| p != pkg);
                            break;
                        }
                    }
                }
                "Add to Menu" => {
                    let menu_choice = run_menu(
                        terminal,
                        "Add to Menu",
                        &["Applications", "Games", "Network", "---", "Back"],
                        None,
                    )?;
                    if let MenuResult::Selected(m) = menu_choice {
                        if !is_back_menu_label(&m) {
                            let display =
                                input_prompt(terminal, &format!("Display name for '{pkg}':"))?
                                    .unwrap_or_else(|| pkg.to_string());
                            let val = serde_json::Value::Array(vec![serde_json::Value::String(
                                pkg.to_string(),
                            )]);
                            match m.as_str() {
                                "Applications" => {
                                    let mut d = load_apps();
                                    d.insert(display, val);
                                    save_apps(&d);
                                }
                                "Games" => {
                                    let mut d = load_games();
                                    d.insert(display, val);
                                    save_games(&d);
                                }
                                "Network" => {
                                    let mut d = load_networks();
                                    d.insert(display, val);
                                    save_networks(&d);
                                }
                                _ => {}
                            }
                            box_message(terminal, &format!("Added to {m}."), 800)?;
                        }
                    }
                }
                _ => break,
            },
        }
    }
    Ok(())
}
