use super::menu::draw_terminal_menu_screen;
use crate::config::{
    get_current_user, load_apps, load_games, load_networks, save_apps, save_games, save_networks,
};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::process::{Command, Stdio};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstallerMenuTarget {
    Applications,
    Games,
    Network,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstallerPackageAction {
    Install,
    Update,
    Reinstall,
    Uninstall,
}

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub raw: String,
    pub pkg: String,
    pub description: Option<String>,
    pub installed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RuntimeTool {
    PlaySound,
    Blueutil,
}

#[derive(Debug, Clone)]
enum InstallerView {
    Root,
    RuntimeTools,
    RuntimeToolActions { tool: RuntimeTool },
    SearchResults,
    Installed,
    SearchActions { pkg: String },
    InstalledActions { pkg: String },
    AddToMenu { pkg: String },
}

#[derive(Debug, Clone)]
pub struct TerminalInstallerState {
    view: InstallerView,
    pub root_idx: usize,
    pub search_idx: usize,
    pub search_page: usize,
    pub installed_idx: usize,
    pub installed_page: usize,
    pub runtime_tools_idx: usize,
    pub action_idx: usize,
    pub add_menu_idx: usize,
    pub search_results: Vec<SearchResult>,
    pub search_query: String,
    pub installed_packages: Vec<String>,
    pub installed_filter: String,
    package_manager: Option<PackageManager>,
    package_descriptions: HashMap<String, Option<String>>,
    runtime_playsound_installed: Option<bool>,
    runtime_blueutil_installed: Option<bool>,
}

impl Default for TerminalInstallerState {
    fn default() -> Self {
        Self {
            view: InstallerView::Root,
            root_idx: 0,
            search_idx: 0,
            search_page: 0,
            installed_idx: 0,
            installed_page: 0,
            runtime_tools_idx: 0,
            action_idx: 0,
            add_menu_idx: 0,
            search_results: Vec::new(),
            search_query: String::new(),
            installed_packages: Vec::new(),
            installed_filter: String::new(),
            package_manager: PackageManager::detect(),
            package_descriptions: HashMap::new(),
            runtime_playsound_installed: None,
            runtime_blueutil_installed: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InstallerEvent {
    None,
    BackToMainMenu,
    OpenSearchPrompt,
    OpenFilterPrompt,
    OpenConfirmAction {
        pkg: String,
        action: InstallerPackageAction,
    },
    OpenDisplayNamePrompt {
        pkg: String,
        target: InstallerMenuTarget,
    },
    LaunchCommand {
        argv: Vec<String>,
        status: String,
        completion_message: Option<String>,
    },
    Status(String),
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum PackageManager {
    Brew,
    Apt,
    Dnf,
    Yay,
    Pacman,
    Zypper,
}

impl PackageManager {
    fn detect() -> Option<Self> {
        Self::detect_all().into_iter().next()
    }

    pub(crate) fn detect_all() -> Vec<Self> {
        let pms: &[(&str, PackageManager)] = &[
            ("brew", PackageManager::Brew),
            ("apt", PackageManager::Apt),
            ("dnf", PackageManager::Dnf),
            ("yay", PackageManager::Yay),
            ("pacman", PackageManager::Pacman),
            ("zypper", PackageManager::Zypper),
        ];
        let mut found = Vec::new();
        let mut seen = std::collections::HashSet::new();
        for (bin, pm) in pms {
            if which(bin) && seen.insert(std::mem::discriminant(pm)) {
                found.push(*pm);
            }
        }
        found
    }

    pub(crate) fn name(self) -> &'static str {
        match self {
            PackageManager::Brew => "brew",
            PackageManager::Apt => "apt",
            PackageManager::Dnf => "dnf",
            PackageManager::Yay => "yay",
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
            PackageManager::Yay => vec![
                "yay".into(),
                "-S".into(),
                "--noconfirm".into(),
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
            PackageManager::Yay => vec![
                "yay".into(),
                "-R".into(),
                "--noconfirm".into(),
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
            PackageManager::Yay => vec![
                "yay".into(),
                "-S".into(),
                "--noconfirm".into(),
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
                "update".into(),
                pkg.into(),
            ],
        }
    }

    fn reinstall_cmd(self, pkg: &str) -> Vec<String> {
        match self {
            PackageManager::Brew => vec!["brew".into(), "reinstall".into(), pkg.into()],
            PackageManager::Apt => vec![
                "sudo".into(),
                "apt".into(),
                "install".into(),
                "--reinstall".into(),
                "-y".into(),
                pkg.into(),
            ],
            PackageManager::Dnf => vec![
                "sudo".into(),
                "dnf".into(),
                "reinstall".into(),
                "-y".into(),
                pkg.into(),
            ],
            PackageManager::Yay => vec![
                "yay".into(),
                "-S".into(),
                "--noconfirm".into(),
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
                "--force".into(),
                pkg.into(),
            ],
        }
    }

    fn search(self, query: &str) -> Vec<SearchResult> {
        let out = match self {
            PackageManager::Brew => Command::new("brew").args(["search", query]).output().ok(),
            PackageManager::Apt => Command::new("apt-cache")
                .args(["search", query])
                .output()
                .ok(),
            PackageManager::Dnf => Command::new("dnf").args(["search", query]).output().ok(),
            PackageManager::Yay => Command::new("yay").args(["-Ss", query]).output().ok(),
            PackageManager::Pacman => Command::new("pacman").args(["-Ss", query]).output().ok(),
            PackageManager::Zypper => Command::new("zypper").args(["se", query]).output().ok(),
        }
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
        .unwrap_or_default();
        let installed: HashSet<String> = self.list_installed().into_iter().collect();
        if matches!(self, PackageManager::Pacman | PackageManager::Yay) {
            let lines: Vec<&str> = out.lines().collect();
            let mut results = Vec::new();
            let mut idx = 0usize;
            while idx < lines.len() {
                let header = lines[idx];
                let Some(pkg) = search_pkg_name(self, header) else {
                    idx += 1;
                    continue;
                };
                let desc = lines
                    .get(idx + 1)
                    .filter(|next| next.starts_with(' '))
                    .map(|next| next.trim().to_string());
                let raw = if let Some(desc) = &desc {
                    format!("{} - {}", header.trim(), desc)
                } else {
                    header.trim().to_string()
                };
                results.push(SearchResult {
                    raw,
                    description: desc.clone(),
                    installed: installed.contains(&pkg),
                    pkg,
                });
                idx += if desc.is_some() { 2 } else { 1 };
            }
            return results;
        }

        out.lines()
            .filter_map(|line| {
                let pkg = search_pkg_name(self, line)?;
                let raw = if let Some(desc) = search_description(self, line) {
                    format!("{pkg} - {desc}")
                } else {
                    line.trim().to_string()
                };
                Some(SearchResult {
                    raw,
                    description: search_description(self, line),
                    installed: installed.contains(&pkg),
                    pkg,
                })
            })
            .collect()
    }
}

fn search_pkg_name(pm: PackageManager, line: &str) -> Option<String> {
    let line = line.trim_end();
    if line.is_empty() || line.starts_with('=') || line.starts_with("warning:") {
        return None;
    }
    if matches!(pm, PackageManager::Pacman | PackageManager::Yay) && line.starts_with(' ') {
        // pacman descriptions are on indented lines; package header is non-indented.
        return None;
    }
    if line.starts_with("Sorting...")
        || line.starts_with("Full Text Search...")
        || line.starts_with("S | Name")
        || line.starts_with("--")
    {
        return None;
    }
    let token = line.split_whitespace().next()?;
    let pkg = if let Some((_, rest)) = token.split_once('/') {
        rest
    } else {
        token
    };
    if pkg.is_empty() {
        return None;
    }
    Some(pkg.to_string())
}

fn search_description(pm: PackageManager, line: &str) -> Option<String> {
    let line = line.trim();
    match pm {
        PackageManager::Apt => line.split_once(" - ").map(|(_, d)| d.trim().to_string()),
        PackageManager::Dnf => line.split_once(':').map(|(_, d)| d.trim().to_string()),
        PackageManager::Zypper => line.split('|').nth(2).map(|d| d.trim().to_string()),
        _ => None,
    }
    .filter(|d| !d.is_empty())
}

fn installer_page_size(menu_start_row: usize, status_row: usize) -> usize {
    status_row
        .saturating_sub(menu_start_row)
        // Keep room for separators/navigation rows so "Back" never collides
        // with the shell-status line at the bottom.
        .saturating_sub(6)
        .max(6)
}

impl PackageManager {
    fn list_installed(self) -> Vec<String> {
        let (bin, args): (&str, &[&str]) = match self {
            PackageManager::Brew => ("brew", &["list"]),
            PackageManager::Apt => ("apt", &["list", "--installed"]),
            PackageManager::Dnf => ("dnf", &["list", "installed"]),
            PackageManager::Yay => ("yay", &["-Q"]),
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
                            .next_back()
                            .unwrap_or("")
                            .to_string()
                    })
                    .filter(|s| !s.is_empty())
                    .collect()
            })
            .unwrap_or_default()
    }

    fn package_description(self, pkg: &str) -> Option<String> {
        fn split_value_after_colon(line: &str) -> Option<String> {
            line.split_once(':')
                .map(|(_, v)| v.trim().to_string())
                .filter(|v| !v.is_empty())
        }

        let output = match self {
            PackageManager::Brew => Command::new("brew")
                .args(["info", "--json=v2", pkg])
                .output()
                .ok()
                .map(|o| {
                    let text = String::from_utf8_lossy(&o.stdout).to_string();
                    serde_json::from_str::<serde_json::Value>(&text)
                        .ok()
                        .and_then(|v| {
                            v.get("formulae")
                                .and_then(|arr| arr.get(0))
                                .and_then(|f| f.get("desc"))
                                .and_then(|d| d.as_str())
                                .map(|s| s.to_string())
                        })
                        .unwrap_or_default()
                }),
            PackageManager::Apt => Command::new("apt-cache")
                .args(["show", pkg])
                .output()
                .ok()
                .map(|o| String::from_utf8_lossy(&o.stdout).to_string()),
            PackageManager::Dnf => Command::new("dnf")
                .args(["info", pkg])
                .output()
                .ok()
                .map(|o| String::from_utf8_lossy(&o.stdout).to_string()),
            PackageManager::Yay => Command::new("yay")
                .args(["-Si", pkg])
                .output()
                .ok()
                .map(|o| String::from_utf8_lossy(&o.stdout).to_string()),
            PackageManager::Pacman => Command::new("pacman")
                .args(["-Si", pkg])
                .output()
                .ok()
                .map(|o| String::from_utf8_lossy(&o.stdout).to_string()),
            PackageManager::Zypper => Command::new("zypper")
                .args(["info", pkg])
                .output()
                .ok()
                .map(|o| String::from_utf8_lossy(&o.stdout).to_string()),
        }?;

        let desc = match self {
            PackageManager::Brew => output,
            PackageManager::Apt => output
                .lines()
                .find_map(|line| line.strip_prefix("Description:").map(str::trim))
                .map(str::to_string)
                .unwrap_or_default(),
            PackageManager::Dnf => output
                .lines()
                .find_map(|line| {
                    if line.trim_start().starts_with("Summary") {
                        split_value_after_colon(line)
                    } else {
                        None
                    }
                })
                .unwrap_or_default(),
            PackageManager::Yay | PackageManager::Pacman => output
                .lines()
                .find_map(|line| {
                    if line.trim_start().starts_with("Description") {
                        split_value_after_colon(line)
                    } else {
                        None
                    }
                })
                .unwrap_or_default(),
            PackageManager::Zypper => output
                .lines()
                .find_map(|line| {
                    if line.trim_start().starts_with("Summary") {
                        split_value_after_colon(line)
                    } else {
                        None
                    }
                })
                .unwrap_or_default(),
        };
        if desc.is_empty() {
            None
        } else {
            Some(desc)
        }
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

fn is_arch_based_linux() -> bool {
    if !cfg!(target_os = "linux") {
        return false;
    }
    std::path::Path::new("/etc/arch-release").exists()
        || std::fs::read_to_string("/etc/os-release")
            .map(|s| {
                s.lines().any(|line| {
                    let lower = line.to_ascii_lowercase();
                    lower.starts_with("id=arch") || lower.contains("id_like=arch")
                })
            })
            .unwrap_or(false)
}

fn is_admin(username: String) -> bool {
    crate::core::auth::load_users()
        .get(&username)
        .map(|u| u.is_admin)
        .unwrap_or(false)
}

impl TerminalInstallerState {
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    pub fn back(&mut self) -> bool {
        match self.view {
            InstallerView::Root => true,
            InstallerView::SearchResults | InstallerView::Installed => {
                self.view = InstallerView::Root;
                false
            }
            InstallerView::RuntimeTools => {
                self.view = InstallerView::Root;
                false
            }
            InstallerView::RuntimeToolActions { .. } => {
                self.view = InstallerView::RuntimeTools;
                false
            }
            InstallerView::SearchActions { .. } => {
                self.view = InstallerView::SearchResults;
                false
            }
            InstallerView::InstalledActions { .. } | InstallerView::AddToMenu { .. } => {
                self.view = InstallerView::Installed;
                false
            }
        }
    }

    fn package_description(&mut self, pkg: &str) -> Option<String> {
        if let Some(desc) = self.package_descriptions.get(pkg) {
            return desc.clone();
        }
        let fetched = self
            .search_results
            .iter()
            .find(|r| r.pkg == pkg)
            .and_then(|r| r.description.clone())
            .or_else(|| {
                self.package_manager
                    .and_then(|manager| manager.package_description(pkg))
            });
        self.package_descriptions
            .insert(pkg.to_string(), fetched.clone());
        fetched
    }

    fn cached_package_description(&self, pkg: &str) -> Option<String> {
        self.package_descriptions
            .get(pkg)
            .and_then(|desc| desc.clone())
    }

    fn refresh_runtime_tool_cache(&mut self) {
        if self.runtime_playsound_installed.is_none() {
            self.runtime_playsound_installed = Some(has_python_module("playsound"));
        }
        if cfg!(target_os = "macos") && self.runtime_blueutil_installed.is_none() {
            self.runtime_blueutil_installed = Some(which("blueutil"));
        }
    }

    fn runtime_tool_installed_cached(&mut self, tool: RuntimeTool) -> bool {
        self.refresh_runtime_tool_cache();
        match tool {
            RuntimeTool::PlaySound => self.runtime_playsound_installed.unwrap_or(false),
            RuntimeTool::Blueutil => self.runtime_blueutil_installed.unwrap_or(false),
        }
    }

    fn invalidate_runtime_tool_cache_for_pkg(&mut self, pkg: &str) {
        match pkg {
            "playsound" => self.runtime_playsound_installed = None,
            "blueutil" => self.runtime_blueutil_installed = None,
            _ => {}
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn draw_installer_screen(
    ctx: &eframe::egui::Context,
    state: &mut TerminalInstallerState,
    shell_status: &str,
    cols: usize,
    rows: usize,
    header_start_row: usize,
    separator_top_row: usize,
    title_row: usize,
    separator_bottom_row: usize,
    subtitle_row: usize,
    menu_start_row: usize,
    status_row: usize,
    content_col: usize,
) -> InstallerEvent {
    if !is_admin(get_current_user().unwrap_or_default()) {
        return InstallerEvent::Status("Access denied. Admin only.".to_string());
    }

    match state.view.clone() {
        InstallerView::Root => draw_root(
            ctx,
            state,
            shell_status,
            cols,
            rows,
            header_start_row,
            separator_top_row,
            title_row,
            separator_bottom_row,
            subtitle_row,
            menu_start_row,
            status_row,
            content_col,
        ),
        InstallerView::SearchResults => draw_search_results(
            ctx,
            state,
            shell_status,
            cols,
            rows,
            header_start_row,
            separator_top_row,
            title_row,
            separator_bottom_row,
            subtitle_row,
            menu_start_row,
            status_row,
            content_col,
        ),
        InstallerView::RuntimeTools => draw_runtime_tools(
            ctx,
            state,
            shell_status,
            cols,
            rows,
            header_start_row,
            separator_top_row,
            title_row,
            separator_bottom_row,
            subtitle_row,
            menu_start_row,
            status_row,
            content_col,
        ),
        InstallerView::RuntimeToolActions { tool } => draw_runtime_tool_actions(
            ctx,
            state,
            tool,
            shell_status,
            cols,
            rows,
            header_start_row,
            separator_top_row,
            title_row,
            separator_bottom_row,
            subtitle_row,
            menu_start_row,
            status_row,
            content_col,
        ),
        InstallerView::Installed => draw_installed(
            ctx,
            state,
            shell_status,
            cols,
            rows,
            header_start_row,
            separator_top_row,
            title_row,
            separator_bottom_row,
            subtitle_row,
            menu_start_row,
            status_row,
            content_col,
        ),
        InstallerView::SearchActions { pkg } => draw_search_actions(
            ctx,
            state,
            &pkg,
            shell_status,
            cols,
            rows,
            header_start_row,
            separator_top_row,
            title_row,
            separator_bottom_row,
            subtitle_row,
            menu_start_row,
            status_row,
            content_col,
        ),
        InstallerView::InstalledActions { pkg } => draw_installed_actions(
            ctx,
            state,
            &pkg,
            shell_status,
            cols,
            rows,
            header_start_row,
            separator_top_row,
            title_row,
            separator_bottom_row,
            subtitle_row,
            menu_start_row,
            status_row,
            content_col,
        ),
        InstallerView::AddToMenu { pkg } => draw_add_to_menu(
            ctx,
            state,
            &pkg,
            shell_status,
            cols,
            rows,
            header_start_row,
            separator_top_row,
            title_row,
            separator_bottom_row,
            subtitle_row,
            menu_start_row,
            status_row,
            content_col,
        ),
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_root(
    ctx: &eframe::egui::Context,
    state: &mut TerminalInstallerState,
    shell_status: &str,
    cols: usize,
    rows: usize,
    header_start_row: usize,
    separator_top_row: usize,
    title_row: usize,
    separator_bottom_row: usize,
    subtitle_row: usize,
    menu_start_row: usize,
    status_row: usize,
    content_col: usize,
) -> InstallerEvent {
    let pm_label = state
        .package_manager
        .map(|p| p.name().to_string())
        .unwrap_or_else(|| "Not Found".to_string());
    let mut items = vec![
        "Search".to_string(),
        "Installed Apps".to_string(),
        "Runtime Tools".to_string(),
    ];
    items.push("---".to_string());
    items.push("Back".to_string());
    let activated = draw_terminal_menu_screen(
        ctx,
        "Program Installer",
        Some(&format!("Package Manager: {pm_label}")),
        &items,
        &mut state.root_idx,
        cols,
        rows,
        header_start_row,
        separator_top_row,
        title_row,
        separator_bottom_row,
        subtitle_row,
        menu_start_row,
        status_row,
        content_col,
        shell_status,
    );
    match activated {
        Some(0) => InstallerEvent::OpenSearchPrompt,
        Some(1) => {
            state.installed_packages = state
                .package_manager
                .map(|p| p.list_installed())
                .unwrap_or_default();
            state.installed_idx = 0;
            state.installed_page = 0;
            state.view = InstallerView::Installed;
            InstallerEvent::Status(format!(
                "Loaded {} installed package(s).",
                state.installed_packages.len()
            ))
        }
        Some(2) => {
            state.runtime_playsound_installed = None;
            state.runtime_blueutil_installed = None;
            state.view = InstallerView::RuntimeTools;
            state.runtime_tools_idx = 0;
            InstallerEvent::None
        }
        Some(_) => InstallerEvent::BackToMainMenu,
        None => InstallerEvent::None,
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_runtime_tools(
    ctx: &eframe::egui::Context,
    state: &mut TerminalInstallerState,
    shell_status: &str,
    cols: usize,
    rows: usize,
    header_start_row: usize,
    separator_top_row: usize,
    title_row: usize,
    separator_bottom_row: usize,
    subtitle_row: usize,
    menu_start_row: usize,
    status_row: usize,
    content_col: usize,
) -> InstallerEvent {
    #[derive(Clone, Copy)]
    enum RuntimeRow {
        Tool(RuntimeTool),
        Back,
        Ignore,
    }
    let playsound_installed = state.runtime_tool_installed_cached(RuntimeTool::PlaySound);
    let mut items = vec![runtime_tool_menu_label(RuntimeTool::PlaySound, playsound_installed)];
    let mut runtime_rows = vec![RuntimeRow::Tool(RuntimeTool::PlaySound)];
    if cfg!(target_os = "macos") {
        let blueutil_installed = state.runtime_tool_installed_cached(RuntimeTool::Blueutil);
        items.push(runtime_tool_menu_label(RuntimeTool::Blueutil, blueutil_installed));
        runtime_rows.push(RuntimeRow::Tool(RuntimeTool::Blueutil));
    }
    items.push("---".to_string());
    runtime_rows.push(RuntimeRow::Ignore);
    items.push("Back".to_string());
    runtime_rows.push(RuntimeRow::Back);
    let subtitle = runtime_rows
        .get(state.runtime_tools_idx)
        .and_then(|row| match row {
            RuntimeRow::Tool(tool) => Some(runtime_tool_description(*tool)),
            _ => None,
        })
        .unwrap_or("Choose a runtime tool");
    let activated = draw_terminal_menu_screen(
        ctx,
        "Runtime Tools",
        Some(subtitle),
        &items,
        &mut state.runtime_tools_idx,
        cols,
        rows,
        header_start_row,
        separator_top_row,
        title_row,
        separator_bottom_row,
        subtitle_row,
        menu_start_row,
        status_row,
        content_col,
        shell_status,
    );
    match activated {
        Some(idx) => match runtime_rows.get(idx) {
            Some(RuntimeRow::Tool(tool)) => {
                state.action_idx = 0;
                state.view = InstallerView::RuntimeToolActions { tool: *tool };
                InstallerEvent::None
            }
            Some(RuntimeRow::Back) => {
                state.view = InstallerView::Root;
                InstallerEvent::None
            }
            _ => InstallerEvent::None,
        },
        None => InstallerEvent::None,
    }
}

fn runtime_tool_pkg(tool: RuntimeTool) -> &'static str {
    match tool {
        RuntimeTool::PlaySound => "playsound",
        RuntimeTool::Blueutil => "blueutil",
    }
}

fn runtime_tool_title(tool: RuntimeTool) -> &'static str {
    match tool {
        RuntimeTool::PlaySound => "Audio Runtime (playsound)",
        RuntimeTool::Blueutil => "Bluetooth Utility (blueutil)",
    }
}

fn runtime_tool_description(tool: RuntimeTool) -> &'static str {
    match tool {
        RuntimeTool::PlaySound => "Python audio runtime (pip)",
        RuntimeTool::Blueutil => "macOS Bluetooth utility (Homebrew)",
    }
}

fn runtime_tool_menu_label(tool: RuntimeTool, installed: bool) -> String {
    let state = if installed {
        "[installed]"
    } else {
        "[not installed]"
    };
    format!("{state} {}", runtime_tool_title(tool))
}

#[allow(clippy::too_many_arguments)]
fn draw_runtime_tool_actions(
    ctx: &eframe::egui::Context,
    state: &mut TerminalInstallerState,
    tool: RuntimeTool,
    shell_status: &str,
    cols: usize,
    rows: usize,
    header_start_row: usize,
    separator_top_row: usize,
    title_row: usize,
    separator_bottom_row: usize,
    subtitle_row: usize,
    menu_start_row: usize,
    status_row: usize,
    content_col: usize,
) -> InstallerEvent {
    let installed = state.runtime_tool_installed_cached(tool);
    let items = if installed {
        vec![
            "Update".to_string(),
            "Reinstall".to_string(),
            "Uninstall".to_string(),
            "---".to_string(),
            "Back".to_string(),
        ]
    } else {
        vec!["Install".to_string(), "---".to_string(), "Back".to_string()]
    };
    let subtitle = format!(
        "{} | {}",
        runtime_tool_description(tool),
        if installed { "Installed" } else { "Not installed" }
    );
    let activated = draw_terminal_menu_screen(
        ctx,
        runtime_tool_title(tool),
        Some(&subtitle),
        &items,
        &mut state.action_idx,
        cols,
        rows,
        header_start_row,
        separator_top_row,
        title_row,
        separator_bottom_row,
        subtitle_row,
        menu_start_row,
        status_row,
        content_col,
        shell_status,
    );
    let pkg = runtime_tool_pkg(tool).to_string();
    match activated {
        Some(0) if !installed => InstallerEvent::OpenConfirmAction {
            pkg,
            action: InstallerPackageAction::Install,
        },
        Some(0) => InstallerEvent::OpenConfirmAction {
            pkg,
            action: InstallerPackageAction::Update,
        },
        Some(1) if installed => InstallerEvent::OpenConfirmAction {
            pkg,
            action: InstallerPackageAction::Reinstall,
        },
        Some(2) if installed => InstallerEvent::OpenConfirmAction {
            pkg,
            action: InstallerPackageAction::Uninstall,
        },
        Some(_) => {
            state.view = InstallerView::RuntimeTools;
            InstallerEvent::None
        }
        None => InstallerEvent::None,
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_search_results(
    ctx: &eframe::egui::Context,
    state: &mut TerminalInstallerState,
    shell_status: &str,
    cols: usize,
    rows: usize,
    header_start_row: usize,
    separator_top_row: usize,
    title_row: usize,
    separator_bottom_row: usize,
    subtitle_row: usize,
    menu_start_row: usize,
    status_row: usize,
    content_col: usize,
) -> InstallerEvent {
    #[derive(Clone)]
    enum SearchRow {
        Package(usize),
        Prev,
        Next,
        Back,
        Ignore,
    }
    let total = state.search_results.len();
    let page_size = installer_page_size(menu_start_row, status_row);
    let total_pages = total.div_ceil(page_size).max(1);
    state.search_page = state.search_page.min(total_pages.saturating_sub(1));
    let start = state.search_page * page_size;
    let end = (start + page_size).min(total);
    let mut items: Vec<String> = Vec::new();
    let mut row_actions: Vec<SearchRow> = Vec::new();
    for idx in start..end {
        let result = &state.search_results[idx];
        items.push(format!(
            "{} {}",
            if result.installed {
                "[installed]"
            } else {
                "[get]"
            },
            result.raw
        ));
        row_actions.push(SearchRow::Package(idx));
    }
    if state.search_page > 0 {
        items.push("< Prev Page".to_string());
        row_actions.push(SearchRow::Prev);
    }
    if end < total {
        items.push("> Next Page".to_string());
        row_actions.push(SearchRow::Next);
    }
    items.push("---".to_string());
    row_actions.push(SearchRow::Ignore);
    items.push("Back".to_string());
    row_actions.push(SearchRow::Back);
    let subtitle = format!(
        "Query: {}  Page {}/{}",
        state.search_query,
        state.search_page + 1,
        total_pages
    );
    let activated = draw_terminal_menu_screen(
        ctx,
        "Search Results",
        Some(&subtitle),
        &items,
        &mut state.search_idx,
        cols,
        rows,
        header_start_row,
        separator_top_row,
        title_row,
        separator_bottom_row,
        subtitle_row,
        menu_start_row,
        status_row,
        content_col,
        shell_status,
    );
    match activated {
        Some(idx) => match row_actions.get(idx) {
            Some(SearchRow::Package(pkg_idx)) => {
                let pkg = state.search_results[*pkg_idx].pkg.clone();
                state.action_idx = 0;
                state.view = InstallerView::SearchActions { pkg };
                InstallerEvent::None
            }
            Some(SearchRow::Prev) => {
                state.search_page = state.search_page.saturating_sub(1);
                state.search_idx = 0;
                InstallerEvent::None
            }
            Some(SearchRow::Next) => {
                state.search_page = (state.search_page + 1).min(total_pages.saturating_sub(1));
                state.search_idx = 0;
                InstallerEvent::None
            }
            Some(SearchRow::Back) => {
                state.view = InstallerView::Root;
                InstallerEvent::None
            }
            _ => InstallerEvent::None,
        },
        None => InstallerEvent::None,
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_installed(
    ctx: &eframe::egui::Context,
    state: &mut TerminalInstallerState,
    shell_status: &str,
    cols: usize,
    rows: usize,
    header_start_row: usize,
    separator_top_row: usize,
    title_row: usize,
    separator_bottom_row: usize,
    subtitle_row: usize,
    menu_start_row: usize,
    status_row: usize,
    content_col: usize,
) -> InstallerEvent {
    #[derive(Clone)]
    enum InstalledRow {
        Filter,
        Package(String),
        Prev,
        Next,
        Back,
        Ignore,
    }
    let filter_label = if state.installed_filter.is_empty() {
        "Filter...".to_string()
    } else {
        format!("Filter: {}", state.installed_filter)
    };
    let filtered: Vec<String> = state
        .installed_packages
        .iter()
        .filter(|p| {
            state.installed_filter.is_empty()
                || p.to_lowercase()
                    .contains(&state.installed_filter.to_lowercase())
        })
        .cloned()
        .collect();
    let total = filtered.len();
    let page_size = installer_page_size(menu_start_row, status_row);
    let total_pages = total.div_ceil(page_size).max(1);
    state.installed_page = state.installed_page.min(total_pages.saturating_sub(1));
    let start = state.installed_page * page_size;
    let end = (start + page_size).min(total);

    let mut items = vec![filter_label.clone(), "---".to_string()];
    let mut row_actions = vec![InstalledRow::Filter, InstalledRow::Ignore];
    for pkg in &filtered[start..end] {
        items.push(pkg.clone());
        row_actions.push(InstalledRow::Package(pkg.clone()));
    }
    if state.installed_page > 0 {
        items.push("< Prev Page".to_string());
        row_actions.push(InstalledRow::Prev);
    }
    if end < total {
        items.push("> Next Page".to_string());
        row_actions.push(InstalledRow::Next);
    }
    items.push("---".to_string());
    row_actions.push(InstalledRow::Ignore);
    items.push("Back".to_string());
    row_actions.push(InstalledRow::Back);
    let selectable_rows: Vec<usize> = items
        .iter()
        .enumerate()
        .filter_map(|(idx, item)| if item == "---" { None } else { Some(idx) })
        .collect();
    let subtitle = selectable_rows
        .get(state.installed_idx)
        .copied()
        .and_then(|raw_idx| match row_actions.get(raw_idx) {
            Some(InstalledRow::Package(pkg)) => state.cached_package_description(pkg),
            _ => None,
        });
    let installed_status = format!(
        "{} packages installed   Page {}/{}",
        total,
        state.installed_page + 1,
        total_pages
    );
    let status_line = if shell_status.is_empty() {
        installed_status
    } else {
        format!("{installed_status} | {shell_status}")
    };
    let activated = draw_terminal_menu_screen(
        ctx,
        "Installed Apps",
        subtitle.as_deref(),
        &items,
        &mut state.installed_idx,
        cols,
        rows,
        header_start_row,
        separator_top_row,
        title_row,
        separator_bottom_row,
        subtitle_row,
        menu_start_row,
        status_row,
        content_col,
        &status_line,
    );
    match activated {
        Some(idx) => match row_actions.get(idx) {
            Some(InstalledRow::Filter) => InstallerEvent::OpenFilterPrompt,
            Some(InstalledRow::Package(pkg)) => {
                state.action_idx = 0;
                state.view = InstallerView::InstalledActions { pkg: pkg.clone() };
                InstallerEvent::None
            }
            Some(InstalledRow::Prev) => {
                state.installed_page = state.installed_page.saturating_sub(1);
                state.installed_idx = 0;
                InstallerEvent::None
            }
            Some(InstalledRow::Next) => {
                state.installed_page =
                    (state.installed_page + 1).min(total_pages.saturating_sub(1));
                state.installed_idx = 0;
                InstallerEvent::None
            }
            Some(InstalledRow::Back) => {
                state.view = InstallerView::Root;
                InstallerEvent::None
            }
            _ => InstallerEvent::None,
        },
        None => InstallerEvent::None,
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_search_actions(
    ctx: &eframe::egui::Context,
    state: &mut TerminalInstallerState,
    pkg: &str,
    shell_status: &str,
    cols: usize,
    rows: usize,
    header_start_row: usize,
    separator_top_row: usize,
    title_row: usize,
    separator_bottom_row: usize,
    subtitle_row: usize,
    menu_start_row: usize,
    status_row: usize,
    content_col: usize,
) -> InstallerEvent {
    let items = vec!["Install".to_string(), "---".to_string(), "Back".to_string()];
    let subtitle = state
        .package_description(pkg)
        .unwrap_or_else(|| "Search result actions".to_string());
    let activated = draw_terminal_menu_screen(
        ctx,
        pkg,
        Some(&subtitle),
        &items,
        &mut state.action_idx,
        cols,
        rows,
        header_start_row,
        separator_top_row,
        title_row,
        separator_bottom_row,
        subtitle_row,
        menu_start_row,
        status_row,
        content_col,
        shell_status,
    );
    match activated {
        Some(0) => InstallerEvent::OpenConfirmAction {
            pkg: pkg.to_string(),
            action: InstallerPackageAction::Install,
        },
        Some(_) => {
            state.view = InstallerView::SearchResults;
            InstallerEvent::None
        }
        None => InstallerEvent::None,
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_installed_actions(
    ctx: &eframe::egui::Context,
    state: &mut TerminalInstallerState,
    pkg: &str,
    shell_status: &str,
    cols: usize,
    rows: usize,
    header_start_row: usize,
    separator_top_row: usize,
    title_row: usize,
    separator_bottom_row: usize,
    subtitle_row: usize,
    menu_start_row: usize,
    status_row: usize,
    content_col: usize,
) -> InstallerEvent {
    let items = vec![
        "Update".to_string(),
        "Reinstall".to_string(),
        "Uninstall".to_string(),
        "Add to Menu".to_string(),
        "---".to_string(),
        "Back".to_string(),
    ];
    let subtitle = state
        .package_description(pkg)
        .unwrap_or_else(|| "Installed package actions".to_string());
    let activated = draw_terminal_menu_screen(
        ctx,
        pkg,
        Some(&subtitle),
        &items,
        &mut state.action_idx,
        cols,
        rows,
        header_start_row,
        separator_top_row,
        title_row,
        separator_bottom_row,
        subtitle_row,
        menu_start_row,
        status_row,
        content_col,
        shell_status,
    );
    match activated {
        Some(0) => InstallerEvent::OpenConfirmAction {
            pkg: pkg.to_string(),
            action: InstallerPackageAction::Update,
        },
        Some(1) => InstallerEvent::OpenConfirmAction {
            pkg: pkg.to_string(),
            action: InstallerPackageAction::Reinstall,
        },
        Some(2) => InstallerEvent::OpenConfirmAction {
            pkg: pkg.to_string(),
            action: InstallerPackageAction::Uninstall,
        },
        Some(3) => {
            state.add_menu_idx = 0;
            state.view = InstallerView::AddToMenu {
                pkg: pkg.to_string(),
            };
            InstallerEvent::None
        }
        Some(_) => {
            state.view = InstallerView::Installed;
            InstallerEvent::None
        }
        None => InstallerEvent::None,
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_add_to_menu(
    ctx: &eframe::egui::Context,
    state: &mut TerminalInstallerState,
    pkg: &str,
    shell_status: &str,
    cols: usize,
    rows: usize,
    header_start_row: usize,
    separator_top_row: usize,
    title_row: usize,
    separator_bottom_row: usize,
    subtitle_row: usize,
    menu_start_row: usize,
    status_row: usize,
    content_col: usize,
) -> InstallerEvent {
    let items = vec![
        "Applications".to_string(),
        "Games".to_string(),
        "Network".to_string(),
        "---".to_string(),
        "Back".to_string(),
    ];
    let activated = draw_terminal_menu_screen(
        ctx,
        "Add to Menu",
        Some(pkg),
        &items,
        &mut state.add_menu_idx,
        cols,
        rows,
        header_start_row,
        separator_top_row,
        title_row,
        separator_bottom_row,
        subtitle_row,
        menu_start_row,
        status_row,
        content_col,
        shell_status,
    );
    match activated {
        Some(0) => InstallerEvent::OpenDisplayNamePrompt {
            pkg: pkg.to_string(),
            target: InstallerMenuTarget::Applications,
        },
        Some(1) => InstallerEvent::OpenDisplayNamePrompt {
            pkg: pkg.to_string(),
            target: InstallerMenuTarget::Games,
        },
        Some(2) => InstallerEvent::OpenDisplayNamePrompt {
            pkg: pkg.to_string(),
            target: InstallerMenuTarget::Network,
        },
        Some(_) => {
            state.view = InstallerView::InstalledActions {
                pkg: pkg.to_string(),
            };
            InstallerEvent::None
        }
        None => InstallerEvent::None,
    }
}

pub fn apply_search_query(state: &mut TerminalInstallerState, query: &str) -> InstallerEvent {
    let query = query.trim().to_string();
    if query.is_empty() {
        return InstallerEvent::Status("Search cancelled.".to_string());
    }
    if !has_internet() {
        return InstallerEvent::Status("Error: No internet connection.".to_string());
    }
    let Some(pm) = state.package_manager else {
        return InstallerEvent::Status("Error: No supported package manager found.".to_string());
    };
    state.search_results = pm.search(&query);
    state.search_query = query;
    state.search_idx = 0;
    state.search_page = 0;
    if state.search_results.is_empty() {
        InstallerEvent::Status("No results found.".to_string())
    } else {
        state.view = InstallerView::SearchResults;
        InstallerEvent::Status(format!(
            "Found {} result(s).",
            state.search_results.len()
        ))
    }
}

pub fn apply_filter(state: &mut TerminalInstallerState, filter: &str) {
    state.installed_filter = filter.trim().to_string();
    state.installed_idx = 0;
    state.installed_page = 0;
}

pub fn settle_view_after_package_command(state: &mut TerminalInstallerState) {
    match state.view.clone() {
        InstallerView::InstalledActions { .. } => {
            state.view = InstallerView::Installed;
            state.action_idx = 0;
        }
        InstallerView::SearchActions { .. } => {
            state.view = InstallerView::SearchResults;
            state.action_idx = 0;
        }
        InstallerView::RuntimeToolActions { tool } => {
            state.view = InstallerView::RuntimeTools;
            state.action_idx = 0;
            state.invalidate_runtime_tool_cache_for_pkg(runtime_tool_pkg(tool));
        }
        _ => {}
    }
}

pub fn build_package_command(
    state: &TerminalInstallerState,
    pkg: &str,
    action: InstallerPackageAction,
) -> InstallerEvent {
    let Some(pm) = state.package_manager else {
        return InstallerEvent::Status("Error: No supported package manager found.".to_string());
    };
    if !has_internet()
        && matches!(
            action,
            InstallerPackageAction::Install | InstallerPackageAction::Update
        )
    {
        return InstallerEvent::Status("Error: No internet connection.".to_string());
    }
    let argv = match action {
        InstallerPackageAction::Install => {
            if pkg == "playsound" {
                if is_arch_based_linux() {
                    if !which("yay") {
                        return InstallerEvent::Status("yay not found. Install yay first.".to_string());
                    }
                    vec![
                        "yay".to_string(),
                        "-S".to_string(),
                        "--noconfirm".to_string(),
                        "python-playsound".to_string(),
                    ]
                } else {
                    if !which("python3") {
                        return InstallerEvent::Status(
                            "python3 not found. Install Python first.".to_string(),
                        );
                    }
                    vec![
                        "python3".to_string(),
                        "-m".to_string(),
                        "pip".to_string(),
                        "install".to_string(),
                        "--user".to_string(),
                        "--upgrade".to_string(),
                        "playsound".to_string(),
                    ]
                }
            } else {
                pm.install_cmd(pkg)
            }
        }
        InstallerPackageAction::Reinstall => {
            if pkg == "playsound" {
                if is_arch_based_linux() {
                    if !which("yay") {
                        return InstallerEvent::Status("yay not found. Install yay first.".to_string());
                    }
                    vec![
                        "yay".to_string(),
                        "-S".to_string(),
                        "--noconfirm".to_string(),
                        "python-playsound".to_string(),
                    ]
                } else {
                    if !which("python3") {
                        return InstallerEvent::Status(
                            "python3 not found. Install Python first.".to_string(),
                        );
                    }
                    vec![
                        "python3".to_string(),
                        "-m".to_string(),
                        "pip".to_string(),
                        "install".to_string(),
                        "--user".to_string(),
                        "--upgrade".to_string(),
                        "--force-reinstall".to_string(),
                        "playsound".to_string(),
                    ]
                }
            } else {
                pm.reinstall_cmd(pkg)
            }
        }
        InstallerPackageAction::Update => {
            if pkg == "playsound" {
                if is_arch_based_linux() {
                    if !which("yay") {
                        return InstallerEvent::Status("yay not found. Install yay first.".to_string());
                    }
                    vec![
                        "yay".to_string(),
                        "-S".to_string(),
                        "--noconfirm".to_string(),
                        "python-playsound".to_string(),
                    ]
                } else {
                    if !which("python3") {
                        return InstallerEvent::Status(
                            "python3 not found. Install Python first.".to_string(),
                        );
                    }
                    vec![
                        "python3".to_string(),
                        "-m".to_string(),
                        "pip".to_string(),
                        "install".to_string(),
                        "--user".to_string(),
                        "--upgrade".to_string(),
                        "playsound".to_string(),
                    ]
                }
            } else {
                pm.update_cmd(pkg)
            }
        }
        InstallerPackageAction::Uninstall => {
            if pkg == "playsound" {
                if is_arch_based_linux() {
                    if !which("yay") {
                        return InstallerEvent::Status("yay not found. Install yay first.".to_string());
                    }
                    vec![
                        "yay".to_string(),
                        "-R".to_string(),
                        "--noconfirm".to_string(),
                        "python-playsound".to_string(),
                    ]
                } else {
                    if !which("python3") {
                        return InstallerEvent::Status(
                            "python3 not found. Install Python first.".to_string(),
                        );
                    }
                    vec![
                        "python3".to_string(),
                        "-m".to_string(),
                        "pip".to_string(),
                        "uninstall".to_string(),
                        "-y".to_string(),
                        "playsound".to_string(),
                    ]
                }
            } else {
                pm.remove_cmd(pkg)
            }
        }
    };
    let status = match action {
        InstallerPackageAction::Install => format!("Installing {pkg}..."),
        InstallerPackageAction::Update => format!("Updating {pkg}..."),
        InstallerPackageAction::Reinstall => format!("Reinstalling {pkg}..."),
        InstallerPackageAction::Uninstall => format!("Uninstalling {pkg}..."),
    };
    let completion_message = match action {
        InstallerPackageAction::Install => format!("{pkg} installed."),
        InstallerPackageAction::Update => format!("{pkg} updated."),
        InstallerPackageAction::Reinstall => format!("{pkg} reinstalled."),
        InstallerPackageAction::Uninstall => format!("{pkg} uninstalled."),
    };
    InstallerEvent::LaunchCommand {
        argv,
        status,
        completion_message: Some(completion_message),
    }
}

pub fn add_package_to_menu(
    state: &mut TerminalInstallerState,
    pkg: &str,
    target: InstallerMenuTarget,
    display_name: &str,
) -> InstallerEvent {
    let display = if display_name.trim().is_empty() {
        pkg.to_string()
    } else {
        display_name.trim().to_string()
    };
    let val = Value::Array(vec![Value::String(pkg.to_string())]);
    match target {
        InstallerMenuTarget::Applications => {
            let mut d = load_apps();
            d.insert(display, val);
            save_apps(&d);
        }
        InstallerMenuTarget::Games => {
            let mut d = load_games();
            d.insert(display, val);
            save_games(&d);
        }
        InstallerMenuTarget::Network => {
            let mut d = load_networks();
            d.insert(display, val);
            save_networks(&d);
        }
    }
    state.view = InstallerView::InstalledActions {
        pkg: pkg.to_string(),
    };
    InstallerEvent::Status("Added to menu.".to_string())
}

// ─── Desktop Installer GUI ──────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DesktopInstallerView {
    Home,
    SearchResults,
    Installed,
    PackageActions { pkg: String, installed: bool },
    AddToMenu { pkg: String },
    RuntimeTools,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstallerCategory {
    Apps,
    Tools,
    Network,
    Games,
}

impl InstallerCategory {
    pub fn label(self) -> &'static str {
        match self {
            Self::Apps => "Apps",
            Self::Tools => "Tools",
            Self::Network => "Network",
            Self::Games => "Games",
        }
    }
}

pub struct DesktopInstallerConfirm {
    pub pkg: String,
    pub action: InstallerPackageAction,
}

pub enum DesktopInstallerEvent {
    None,
    LaunchCommand {
        argv: Vec<String>,
        status: String,
        completion_message: Option<String>,
    },
}

#[derive(Debug, Clone)]
pub struct DesktopInstallerNotice {
    pub message: String,
    pub success: bool,
}

pub struct DesktopInstallerState {
    pub open: bool,
    pub view: DesktopInstallerView,
    pub search_query: String,
    pub search_results: Vec<SearchResult>,
    pub installed_packages: Vec<String>,
    pub installed_filter: String,
    pub installed_page: usize,
    pub search_page: usize,
    pub status: String,
    pub available_pms: Vec<PackageManager>,
    pub selected_pm_idx: usize,
    package_descriptions: HashMap<String, Option<String>>,
    runtime_playsound_installed: Option<bool>,
    runtime_blueutil_installed: Option<bool>,
    pub confirm_dialog: Option<DesktopInstallerConfirm>,
    pub notice: Option<DesktopInstallerNotice>,
    pub display_name_input: String,
}

impl Default for DesktopInstallerState {
    fn default() -> Self {
        let available = PackageManager::detect_all();
        Self {
            open: false,
            view: DesktopInstallerView::Home,
            search_query: String::new(),
            search_results: Vec::new(),
            installed_packages: Vec::new(),
            installed_filter: String::new(),
            installed_page: 0,
            search_page: 0,
            status: String::new(),
            available_pms: available,
            selected_pm_idx: 0,
            package_descriptions: HashMap::new(),
            runtime_playsound_installed: None,
            runtime_blueutil_installed: None,
            confirm_dialog: None,
            notice: None,
            display_name_input: String::new(),
        }
    }
}

impl DesktopInstallerState {
    pub fn selected_pm(&self) -> Option<PackageManager> {
        self.available_pms.get(self.selected_pm_idx).copied()
    }

    pub fn go_back(&mut self) {
        self.confirm_dialog = None;
        self.view = match &self.view {
            DesktopInstallerView::Home => return,
            DesktopInstallerView::SearchResults => DesktopInstallerView::Home,
            DesktopInstallerView::Installed => DesktopInstallerView::Home,
            DesktopInstallerView::PackageActions { .. } => {
                if self.search_results.is_empty() {
                    DesktopInstallerView::Installed
                } else {
                    DesktopInstallerView::SearchResults
                }
            }
            DesktopInstallerView::AddToMenu { pkg } => {
                DesktopInstallerView::PackageActions {
                    pkg: pkg.clone(),
                    installed: true,
                }
            }
            DesktopInstallerView::RuntimeTools => DesktopInstallerView::Home,
        };
    }

    pub fn do_search(&mut self) {
        let query = self.search_query.trim().to_string();
        if query.is_empty() {
            self.status = "Enter a search term.".to_string();
            return;
        }
        if !has_internet() {
            self.status = "Error: No internet connection.".to_string();
            self.notice = Some(DesktopInstallerNotice {
                message: "Search requires an internet connection.".to_string(),
                success: false,
            });
            return;
        }
        let Some(pm) = self.selected_pm() else {
            self.status = "Error: No package manager found.".to_string();
            return;
        };
        self.notice = None;
        self.search_results = pm.search(&query);
        self.search_page = 0;
        if self.search_results.is_empty() {
            self.status = "No results found.".to_string();
        } else {
            self.status = format!("Found {} result(s).", self.search_results.len());
            self.view = DesktopInstallerView::SearchResults;
        }
    }

    pub fn load_installed(&mut self) {
        self.installed_packages = self
            .selected_pm()
            .map(|p| p.list_installed())
            .unwrap_or_default();
        self.installed_page = 0;
        self.installed_filter.clear();
        self.status = format!(
            "Loaded {} installed package(s).",
            self.installed_packages.len()
        );
        self.view = DesktopInstallerView::Installed;
    }

    pub fn filtered_installed(&self) -> Vec<String> {
        self.installed_packages
            .iter()
            .filter(|p| {
                self.installed_filter.is_empty()
                    || p.to_lowercase()
                        .contains(&self.installed_filter.to_lowercase())
            })
            .cloned()
            .collect()
    }

    pub fn package_description_cached(&self, pkg: &str) -> Option<String> {
        self.package_descriptions
            .get(pkg)
            .and_then(|d| d.clone())
    }

    pub fn can_fetch_descriptions(&self) -> bool {
        has_internet()
    }

    pub fn fetch_package_description(&mut self, pkg: &str) -> Option<String> {
        if let Some(desc) = self.package_descriptions.get(pkg) {
            return desc.clone();
        }
        let fetched = self
            .search_results
            .iter()
            .find(|r| r.pkg == pkg)
            .and_then(|r| r.description.clone())
            .or_else(|| {
                if has_internet() {
                    self.selected_pm()
                        .and_then(|pm| pm.package_description(pkg))
                } else {
                    None
                }
            });
        self.package_descriptions
            .insert(pkg.to_string(), fetched.clone());
        fetched
    }

    pub fn pm_label(&self) -> &str {
        self.selected_pm()
            .map(|p| p.name())
            .unwrap_or("Not Found")
    }

    pub fn confirm_action(&mut self) -> DesktopInstallerEvent {
        let Some(confirm) = self.confirm_dialog.take() else {
            return DesktopInstallerEvent::None;
        };
        let Some(pm) = self.selected_pm() else {
            self.status = "Error: No package manager found.".to_string();
            return DesktopInstallerEvent::None;
        };
        if !has_internet()
            && matches!(
                confirm.action,
                InstallerPackageAction::Install | InstallerPackageAction::Update
            )
        {
            self.status = "Error: No internet connection.".to_string();
            return DesktopInstallerEvent::None;
        }
        let pkg = &confirm.pkg;
        let argv = match confirm.action {
            InstallerPackageAction::Install => {
                if pkg == "playsound" {
                    playsound_install_cmd()
                } else {
                    pm.install_cmd(pkg)
                }
            }
            InstallerPackageAction::Reinstall => {
                if pkg == "playsound" {
                    playsound_reinstall_cmd()
                } else {
                    pm.reinstall_cmd(pkg)
                }
            }
            InstallerPackageAction::Update => {
                if pkg == "playsound" {
                    playsound_update_cmd()
                } else {
                    pm.update_cmd(pkg)
                }
            }
            InstallerPackageAction::Uninstall => {
                if pkg == "playsound" {
                    playsound_uninstall_cmd()
                } else {
                    pm.remove_cmd(pkg)
                }
            }
        };
        let status = match confirm.action {
            InstallerPackageAction::Install => format!("Installing {pkg}..."),
            InstallerPackageAction::Update => format!("Updating {pkg}..."),
            InstallerPackageAction::Reinstall => format!("Reinstalling {pkg}..."),
            InstallerPackageAction::Uninstall => format!("Uninstalling {pkg}..."),
        };
        let completion_message = match confirm.action {
            InstallerPackageAction::Install => format!("{pkg} installed."),
            InstallerPackageAction::Update => format!("{pkg} updated."),
            InstallerPackageAction::Reinstall => format!("{pkg} reinstalled."),
            InstallerPackageAction::Uninstall => format!("{pkg} uninstalled."),
        };
        self.status = status.clone();
        DesktopInstallerEvent::LaunchCommand {
            argv,
            status,
            completion_message: Some(completion_message),
        }
    }

    pub fn add_to_menu(&mut self, pkg: &str, target: InstallerMenuTarget) {
        let display = if self.display_name_input.trim().is_empty() {
            pkg.to_string()
        } else {
            self.display_name_input.trim().to_string()
        };
        let val = Value::Array(vec![Value::String(pkg.to_string())]);
        match target {
            InstallerMenuTarget::Applications => {
                let mut d = load_apps();
                d.insert(display, val);
                save_apps(&d);
            }
            InstallerMenuTarget::Games => {
                let mut d = load_games();
                d.insert(display, val);
                save_games(&d);
            }
            InstallerMenuTarget::Network => {
                let mut d = load_networks();
                d.insert(display, val);
                save_networks(&d);
            }
        }
        self.display_name_input.clear();
        self.status = "Added to menu.".to_string();
        self.view = DesktopInstallerView::PackageActions {
            pkg: pkg.to_string(),
            installed: true,
        };
    }

    pub fn runtime_playsound_installed(&mut self) -> bool {
        if self.runtime_playsound_installed.is_none() {
            self.runtime_playsound_installed = Some(has_python_module("playsound"));
        }
        self.runtime_playsound_installed.unwrap_or(false)
    }

    pub fn runtime_blueutil_installed(&mut self) -> bool {
        if cfg!(target_os = "macos") && self.runtime_blueutil_installed.is_none() {
            self.runtime_blueutil_installed = Some(which("blueutil"));
        }
        self.runtime_blueutil_installed.unwrap_or(false)
    }
}

fn playsound_install_cmd() -> Vec<String> {
    if is_arch_based_linux() {
        vec![
            "yay".into(), "-S".into(), "--noconfirm".into(),
            "python-playsound".into(),
        ]
    } else {
        vec![
            "python3".into(), "-m".into(), "pip".into(), "install".into(),
            "--user".into(), "--upgrade".into(), "playsound".into(),
        ]
    }
}

fn playsound_reinstall_cmd() -> Vec<String> {
    if is_arch_based_linux() {
        playsound_install_cmd()
    } else {
        vec![
            "python3".into(), "-m".into(), "pip".into(), "install".into(),
            "--user".into(), "--upgrade".into(), "--force-reinstall".into(),
            "playsound".into(),
        ]
    }
}

fn playsound_update_cmd() -> Vec<String> {
    playsound_install_cmd()
}

fn playsound_uninstall_cmd() -> Vec<String> {
    if is_arch_based_linux() {
        vec![
            "yay".into(), "-R".into(), "--noconfirm".into(),
            "python-playsound".into(),
        ]
    } else {
        vec![
            "python3".into(), "-m".into(), "pip".into(), "uninstall".into(),
            "-y".into(), "playsound".into(),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn back_from_search_actions_returns_to_search_results() {
        let mut state = TerminalInstallerState {
            view: InstallerView::SearchActions {
                pkg: "pkg".to_string(),
            },
            ..Default::default()
        };
        assert!(!state.back());
        assert!(matches!(state.view, InstallerView::SearchResults));
    }

    #[test]
    fn empty_search_reports_cancelled() {
        let mut state = TerminalInstallerState::default();
        let event = apply_search_query(&mut state, "   ");
        assert!(matches!(
            event,
            InstallerEvent::Status(ref s) if s == "Search cancelled."
        ));
    }
}
