use super::menu::draw_terminal_menu_screen;
use crate::config::{
    get_current_user, load_apps, load_games, load_networks, save_apps, save_games, save_networks,
};
use serde_json::Value;
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
    Uninstall,
}

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub raw: String,
    pub pkg: String,
    pub installed: bool,
}

#[derive(Debug, Clone)]
enum InstallerView {
    Root,
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
    pub installed_idx: usize,
    pub action_idx: usize,
    pub add_menu_idx: usize,
    pub search_results: Vec<SearchResult>,
    pub search_query: String,
    pub installed_packages: Vec<String>,
    pub installed_filter: String,
    package_manager: Option<PackageManager>,
}

impl Default for TerminalInstallerState {
    fn default() -> Self {
        Self {
            view: InstallerView::Root,
            root_idx: 0,
            search_idx: 0,
            installed_idx: 0,
            action_idx: 0,
            add_menu_idx: 0,
            search_results: Vec::new(),
            search_query: String::new(),
            installed_packages: Vec::new(),
            installed_filter: String::new(),
            package_manager: PackageManager::detect(),
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
    },
    Status(String),
}

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

    fn search(self, query: &str) -> Vec<SearchResult> {
        let out = Command::new(self.name())
            .args(["search", query])
            .output()
            .ok()
            .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
            .unwrap_or_default();
        out.lines()
            .filter(|l| !l.is_empty() && !l.starts_with('='))
            .filter_map(|line| {
                let pkg = line
                    .split_whitespace()
                    .next()
                    .unwrap_or("")
                    .split('/')
                    .next()
                    .unwrap_or("")
                    .to_string();
                if pkg.is_empty() {
                    return None;
                }
                Some(SearchResult {
                    raw: line.to_string(),
                    installed: which(&pkg),
                    pkg,
                })
            })
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
        "Install Audio Runtime (playsound)".to_string(),
    ];
    if cfg!(target_os = "macos") {
        items.push("Install Bluetooth Utility (blueutil)".to_string());
    }
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
            state.view = InstallerView::Installed;
            InstallerEvent::None
        }
        Some(2) => {
            if !which("python3") {
                return InstallerEvent::Status(
                    "python3 not found. Install Python first.".to_string(),
                );
            }
            if has_python_module("playsound") {
                return InstallerEvent::Status("playsound is already installed.".to_string());
            }
            if !has_internet() {
                return InstallerEvent::Status("Error: No internet connection.".to_string());
            }
            InstallerEvent::OpenConfirmAction {
                pkg: "playsound".to_string(),
                action: InstallerPackageAction::Install,
            }
        }
        Some(3) if cfg!(target_os = "macos") => {
            if which("blueutil") {
                return InstallerEvent::Status("blueutil is already installed.".to_string());
            }
            if !which("brew") {
                return InstallerEvent::Status(
                    "Homebrew not found. Install brew first.".to_string(),
                );
            }
            if !has_internet() {
                return InstallerEvent::Status("Error: No internet connection.".to_string());
            }
            InstallerEvent::LaunchCommand {
                argv: vec![
                    "brew".to_string(),
                    "install".to_string(),
                    "blueutil".to_string(),
                ],
                status: "Launching blueutil install in terminal...".to_string(),
            }
        }
        Some(_) => InstallerEvent::BackToMainMenu,
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
    let mut items: Vec<String> = state
        .search_results
        .iter()
        .map(|result| {
            format!(
                "{} {}",
                if result.installed {
                    "[installed]"
                } else {
                    "[get]"
                },
                result.raw
            )
        })
        .collect();
    items.push("---".to_string());
    items.push("Back".to_string());
    let subtitle = format!("Query: {}", state.search_query);
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
        Some(idx) if idx < state.search_results.len() => {
            let pkg = state.search_results[idx].pkg.clone();
            state.action_idx = 0;
            state.view = InstallerView::SearchActions { pkg };
            InstallerEvent::None
        }
        Some(_) => {
            state.view = InstallerView::Root;
            InstallerEvent::None
        }
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
    let mut items = vec![filter_label.clone(), "---".to_string()];
    items.extend(filtered.iter().cloned());
    items.push("---".to_string());
    items.push("Back".to_string());
    let activated = draw_terminal_menu_screen(
        ctx,
        "Installed Apps",
        Some("Manage installed packages"),
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
        shell_status,
    );
    match activated {
        Some(0) => InstallerEvent::OpenFilterPrompt,
        Some(idx) if idx > 1 && idx < filtered.len() + 2 => {
            let pkg = filtered[idx - 2].clone();
            state.action_idx = 0;
            state.view = InstallerView::InstalledActions { pkg };
            InstallerEvent::None
        }
        Some(_) => {
            state.view = InstallerView::Root;
            InstallerEvent::None
        }
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
    let activated = draw_terminal_menu_screen(
        ctx,
        pkg,
        Some("Search result actions"),
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
        "Uninstall".to_string(),
        "Add to Menu".to_string(),
        "---".to_string(),
        "Back".to_string(),
    ];
    let activated = draw_terminal_menu_screen(
        ctx,
        pkg,
        Some("Installed package actions"),
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
            action: InstallerPackageAction::Uninstall,
        },
        Some(2) => {
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
    if state.search_results.is_empty() {
        InstallerEvent::Status("No results found.".to_string())
    } else {
        state.view = InstallerView::SearchResults;
        InstallerEvent::None
    }
}

pub fn apply_filter(state: &mut TerminalInstallerState, filter: &str) {
    state.installed_filter = filter.trim().to_string();
    state.installed_idx = 0;
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
            } else {
                pm.install_cmd(pkg)
            }
        }
        InstallerPackageAction::Update => pm.update_cmd(pkg),
        InstallerPackageAction::Uninstall => pm.remove_cmd(pkg),
    };
    let verb = match action {
        InstallerPackageAction::Install => "install",
        InstallerPackageAction::Update => "update",
        InstallerPackageAction::Uninstall => "remove",
    };
    InstallerEvent::LaunchCommand {
        argv,
        status: format!("Launching {verb} for {pkg} in terminal..."),
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

fn is_admin(username: String) -> bool {
    crate::core::auth::load_users()
        .get(&username)
        .map(|u| u.is_admin)
        .unwrap_or(false)
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
