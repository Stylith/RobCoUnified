use std::collections::{HashMap, HashSet};
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread;

use robcos_shared::config::{
    base_dir, load_apps, load_games, load_networks, save_apps, save_games, save_networks,
};

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
pub enum RuntimeTool {
    PlaySound,
    Blueutil,
}

const DEFAULT_RUNTIME_TOOLS: &[RuntimeTool] = &[RuntimeTool::PlaySound];
const MACOS_RUNTIME_TOOLS: &[RuntimeTool] = &[RuntimeTool::PlaySound, RuntimeTool::Blueutil];

pub fn available_runtime_tools() -> &'static [RuntimeTool] {
    if cfg!(target_os = "macos") {
        MACOS_RUNTIME_TOOLS
    } else {
        DEFAULT_RUNTIME_TOOLS
    }
}

pub fn runtime_tool_title(tool: RuntimeTool) -> &'static str {
    match tool {
        RuntimeTool::PlaySound => "Audio Runtime (playsound)",
        RuntimeTool::Blueutil => "Bluetooth Utility (blueutil)",
    }
}

pub fn runtime_tool_description(tool: RuntimeTool) -> &'static str {
    match tool {
        RuntimeTool::PlaySound => "Python audio runtime (pip)",
        RuntimeTool::Blueutil => "macOS Bluetooth utility (Homebrew)",
    }
}

pub fn runtime_tool_menu_label(tool: RuntimeTool, installed: bool) -> String {
    let state = if installed {
        "[installed]"
    } else {
        "[not installed]"
    };
    format!("{state} {}", runtime_tool_title(tool))
}

pub fn runtime_tool_actions(installed: bool) -> &'static [InstallerPackageAction] {
    if installed {
        &[
            InstallerPackageAction::Update,
            InstallerPackageAction::Reinstall,
            InstallerPackageAction::Uninstall,
        ]
    } else {
        &[InstallerPackageAction::Install]
    }
}

pub fn runtime_tool_action_for_selection(
    installed: bool,
    selected_idx: usize,
) -> Option<InstallerPackageAction> {
    runtime_tool_actions(installed).get(selected_idx).copied()
}

#[derive(Debug, Clone)]
pub enum InstallerView {
    Root,
    PackageManagerSelect,
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
    pub view: InstallerView,
    pub root_idx: usize,
    pub pm_select_idx: usize,
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
    pub available_pms: Vec<PackageManager>,
    pub selected_pm_idx: usize,
    pub package_descriptions: HashMap<String, Option<String>>,
    pub runtime_playsound_installed: Option<bool>,
    pub runtime_blueutil_installed: Option<bool>,
}

impl Default for TerminalInstallerState {
    fn default() -> Self {
        Self {
            view: InstallerView::Root,
            root_idx: 0,
            pm_select_idx: 0,
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
            available_pms: Vec::new(),
            selected_pm_idx: 0,
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

struct DesktopSearchResponse {
    query: String,
    results: Vec<SearchResult>,
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
    installed_description_cache: Option<HashMap<String, HashMap<String, String>>>,
    runtime_playsound_installed: Option<bool>,
    runtime_blueutil_installed: Option<bool>,
    search_receiver: Option<Receiver<DesktopSearchResponse>>,
    pub confirm_dialog: Option<DesktopInstallerConfirm>,
    pub notice: Option<DesktopInstallerNotice>,
    pub display_name_input: String,
}

impl Default for DesktopInstallerState {
    fn default() -> Self {
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
            available_pms: Vec::new(),
            selected_pm_idx: 0,
            package_descriptions: HashMap::new(),
            installed_description_cache: None,
            runtime_playsound_installed: None,
            runtime_blueutil_installed: None,
            search_receiver: None,
            confirm_dialog: None,
            notice: None,
            display_name_input: String::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackageManager {
    Brew,
    Apt,
    Dnf,
    Yay,
    Pacman,
    Zypper,
}

impl PackageManager {
    pub fn detect_all() -> Vec<Self> {
        let pms: &[(&str, PackageManager)] = &[
            ("brew", PackageManager::Brew),
            ("apt", PackageManager::Apt),
            ("dnf", PackageManager::Dnf),
            ("yay", PackageManager::Yay),
            ("pacman", PackageManager::Pacman),
            ("zypper", PackageManager::Zypper),
        ];
        let mut found = Vec::new();
        let mut seen = HashSet::new();
        for (bin, pm) in pms {
            if which(bin) && seen.insert(std::mem::discriminant(pm)) {
                found.push(*pm);
            }
        }
        found
    }

    pub fn name(self) -> &'static str {
        match self {
            PackageManager::Brew => "brew",
            PackageManager::Apt => "apt",
            PackageManager::Dnf => "dnf",
            PackageManager::Yay => "yay",
            PackageManager::Pacman => "pacman",
            PackageManager::Zypper => "zypper",
        }
    }

    fn executable(self) -> String {
        command_name_or_path(self.name())
    }

    pub fn install_cmd(self, pkg: &str) -> Vec<String> {
        let exe = self.executable();
        match self {
            PackageManager::Brew => vec![exe, "install".into(), pkg.into()],
            PackageManager::Apt => vec![
                "sudo".into(),
                exe,
                "install".into(),
                "-y".into(),
                pkg.into(),
            ],
            PackageManager::Dnf => vec![
                "sudo".into(),
                exe,
                "install".into(),
                "-y".into(),
                pkg.into(),
            ],
            PackageManager::Yay => vec![exe, "-S".into(), "--noconfirm".into(), pkg.into()],
            PackageManager::Pacman => vec![
                "sudo".into(),
                exe,
                "-S".into(),
                "--noconfirm".into(),
                pkg.into(),
            ],
            PackageManager::Zypper => vec![
                "sudo".into(),
                exe,
                "-n".into(),
                "install".into(),
                pkg.into(),
            ],
        }
    }

    pub fn remove_cmd(self, pkg: &str) -> Vec<String> {
        let exe = self.executable();
        match self {
            PackageManager::Brew => vec![exe, "uninstall".into(), pkg.into()],
            PackageManager::Apt => {
                vec!["sudo".into(), exe, "remove".into(), "-y".into(), pkg.into()]
            }
            PackageManager::Dnf => {
                vec!["sudo".into(), exe, "remove".into(), "-y".into(), pkg.into()]
            }
            PackageManager::Yay => vec![exe, "-R".into(), "--noconfirm".into(), pkg.into()],
            PackageManager::Pacman => vec![
                "sudo".into(),
                exe,
                "-R".into(),
                "--noconfirm".into(),
                pkg.into(),
            ],
            PackageManager::Zypper => {
                vec!["sudo".into(), exe, "-n".into(), "remove".into(), pkg.into()]
            }
        }
    }

    pub fn update_cmd(self, pkg: &str) -> Vec<String> {
        let exe = self.executable();
        match self {
            PackageManager::Brew => vec![exe, "upgrade".into(), pkg.into()],
            PackageManager::Apt => vec![
                "sudo".into(),
                exe,
                "upgrade".into(),
                "-y".into(),
                pkg.into(),
            ],
            PackageManager::Dnf => vec![
                "sudo".into(),
                exe,
                "upgrade".into(),
                "-y".into(),
                pkg.into(),
            ],
            PackageManager::Yay => vec![exe, "-S".into(), "--noconfirm".into(), pkg.into()],
            PackageManager::Pacman => vec![
                "sudo".into(),
                exe,
                "-S".into(),
                "--noconfirm".into(),
                pkg.into(),
            ],
            PackageManager::Zypper => {
                vec!["sudo".into(), exe, "-n".into(), "update".into(), pkg.into()]
            }
        }
    }

    pub fn reinstall_cmd(self, pkg: &str) -> Vec<String> {
        let exe = self.executable();
        match self {
            PackageManager::Brew => vec![exe, "reinstall".into(), pkg.into()],
            PackageManager::Apt => vec![
                "sudo".into(),
                exe,
                "install".into(),
                "--reinstall".into(),
                "-y".into(),
                pkg.into(),
            ],
            PackageManager::Dnf => vec![
                "sudo".into(),
                exe,
                "reinstall".into(),
                "-y".into(),
                pkg.into(),
            ],
            PackageManager::Yay => vec![exe, "-S".into(), "--noconfirm".into(), pkg.into()],
            PackageManager::Pacman => vec![
                "sudo".into(),
                exe,
                "-S".into(),
                "--noconfirm".into(),
                pkg.into(),
            ],
            PackageManager::Zypper => vec![
                "sudo".into(),
                exe,
                "-n".into(),
                "install".into(),
                "--force".into(),
                pkg.into(),
            ],
        }
    }

    pub fn search(self, query: &str) -> Vec<SearchResult> {
        let exe = self.executable();
        let out = match self {
            PackageManager::Brew => Command::new(&exe).args(["search", query]).output().ok(),
            PackageManager::Apt => Command::new(command_name_or_path("apt-cache"))
                .args(["search", query])
                .output()
                .ok(),
            PackageManager::Dnf => Command::new(&exe).args(["search", query]).output().ok(),
            PackageManager::Yay => Command::new(&exe).args(["-Ss", query]).output().ok(),
            PackageManager::Pacman => Command::new(&exe).args(["-Ss", query]).output().ok(),
            PackageManager::Zypper => Command::new(&exe).args(["se", query]).output().ok(),
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

    pub fn list_installed(self) -> Vec<String> {
        let args: &[&str] = match self {
            PackageManager::Brew => &["list"],
            PackageManager::Apt => &["list", "--installed"],
            PackageManager::Dnf => &["list", "installed"],
            PackageManager::Yay => &["-Q"],
            PackageManager::Pacman => &["-Q"],
            PackageManager::Zypper => &["se", "--installed-only"],
        };
        let exe = self.executable();
        Command::new(&exe)
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

    pub fn package_description(self, pkg: &str) -> Option<String> {
        fn split_value_after_colon(line: &str) -> Option<String> {
            line.split_once(':')
                .map(|(_, v)| v.trim().to_string())
                .filter(|v| !v.is_empty())
        }

        let exe = self.executable();
        let output = match self {
            PackageManager::Brew => Command::new(&exe)
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
            PackageManager::Apt => Command::new(command_name_or_path("apt-cache"))
                .args(["show", pkg])
                .output()
                .ok()
                .map(|o| String::from_utf8_lossy(&o.stdout).to_string()),
            PackageManager::Dnf => Command::new(&exe)
                .args(["info", pkg])
                .output()
                .ok()
                .map(|o| String::from_utf8_lossy(&o.stdout).to_string()),
            PackageManager::Yay => Command::new(&exe)
                .args(["-Si", pkg])
                .output()
                .ok()
                .map(|o| String::from_utf8_lossy(&o.stdout).to_string()),
            PackageManager::Pacman => Command::new(&exe)
                .args(["-Si", pkg])
                .output()
                .ok()
                .map(|o| String::from_utf8_lossy(&o.stdout).to_string()),
            PackageManager::Zypper => Command::new(&exe)
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

fn command_search_dirs(path_var: Option<&OsStr>) -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    let mut push_unique = |dir: PathBuf| {
        if !dirs.contains(&dir) {
            dirs.push(dir);
        }
    };

    if let Some(path_var) = path_var {
        for dir in std::env::split_paths(path_var) {
            push_unique(dir);
        }
    }

    for extra in [
        "/opt/homebrew/bin",
        "/usr/local/bin",
        "/opt/local/bin",
        "/usr/bin",
        "/bin",
        "/usr/sbin",
        "/sbin",
    ] {
        push_unique(PathBuf::from(extra));
    }

    dirs
}

fn resolve_command_path_from_search_dirs(bin: &str, search_dirs: &[PathBuf]) -> Option<PathBuf> {
    if bin.contains(std::path::MAIN_SEPARATOR) {
        let path = PathBuf::from(bin);
        return path.is_file().then_some(path);
    }

    for dir in search_dirs {
        let candidate = dir.join(bin);
        if candidate.is_file() {
            return Some(candidate);
        }
    }

    None
}

fn resolve_command_path(bin: &str) -> Option<PathBuf> {
    let search_dirs = command_search_dirs(std::env::var_os("PATH").as_deref());
    resolve_command_path_from_search_dirs(bin, &search_dirs)
}

fn command_name_or_path(bin: &str) -> String {
    resolve_command_path(bin)
        .map(|path| path.to_string_lossy().into_owned())
        .unwrap_or_else(|| bin.to_string())
}

pub fn which(bin: &str) -> bool {
    resolve_command_path(bin).is_some()
}

pub fn has_internet() -> bool {
    let curl = command_name_or_path("curl");
    Command::new(curl)
        .args(["-s", "--max-time", "3", "https://www.google.com"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

pub fn has_python_module(module: &str) -> bool {
    if !which("python3") {
        return false;
    }
    let code = format!("import {module}");
    let python = command_name_or_path("python3");
    Command::new(python)
        .args(["-c", code.as_str()])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

pub fn is_arch_based_linux() -> bool {
    if !cfg!(target_os = "linux") {
        return false;
    }
    Path::new("/etc/arch-release").exists()
        || std::fs::read_to_string("/etc/os-release")
            .map(|s| {
                s.lines().any(|line| {
                    let lower = line.to_ascii_lowercase();
                    lower.starts_with("id=arch") || lower.contains("id_like=arch")
                })
            })
            .unwrap_or(false)
}

impl DesktopInstallerState {
    pub fn ensure_available_pms(&mut self) {
        if self.available_pms.is_empty() {
            self.available_pms = PackageManager::detect_all();
            self.selected_pm_idx = self
                .selected_pm_idx
                .min(self.available_pms.len().saturating_sub(1));
        }
    }

    fn installed_cache_key(pm: PackageManager) -> String {
        pm.name().to_string()
    }

    fn ensure_installed_description_cache(
        &mut self,
    ) -> &mut HashMap<String, HashMap<String, String>> {
        self.installed_description_cache
            .get_or_insert_with(load_installed_description_cache)
    }

    fn installed_description_cached_for_pm(&self, pm: PackageManager, pkg: &str) -> Option<String> {
        self.installed_description_cache
            .as_ref()
            .and_then(|cache| cache.get(&Self::installed_cache_key(pm)))
            .and_then(|pkgs| pkgs.get(pkg))
            .cloned()
    }

    fn persist_installed_description(&mut self, pm: PackageManager, pkg: &str, desc: &str) {
        let cache = self.ensure_installed_description_cache();
        cache
            .entry(Self::installed_cache_key(pm))
            .or_default()
            .insert(pkg.to_string(), desc.to_string());
        save_installed_description_cache(cache);
    }

    pub fn selected_pm(&mut self) -> Option<PackageManager> {
        self.ensure_available_pms();
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
            DesktopInstallerView::AddToMenu { pkg } => DesktopInstallerView::PackageActions {
                pkg: pkg.clone(),
                installed: true,
            },
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
        self.search_receiver = None;
        self.status = format!("Searching for \"{query}\"...");
        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            let results = pm.search(&query);
            let _ = tx.send(DesktopSearchResponse { query, results });
        });
        self.search_receiver = Some(rx);
    }

    pub fn search_in_flight(&self) -> bool {
        self.search_receiver.is_some()
    }

    pub fn poll_search(&mut self) -> bool {
        let Some(rx) = self.search_receiver.as_ref() else {
            return false;
        };
        match rx.try_recv() {
            Ok(response) => {
                self.search_receiver = None;
                self.search_results = response.results;
                self.search_page = 0;
                self.search_query = response.query;
                if self.search_results.is_empty() {
                    self.status = "No results found.".to_string();
                } else {
                    self.status = format!("Found {} result(s).", self.search_results.len());
                    self.view = DesktopInstallerView::SearchResults;
                }
                true
            }
            Err(TryRecvError::Empty) => false,
            Err(TryRecvError::Disconnected) => {
                self.search_receiver = None;
                self.status = "Search failed.".to_string();
                true
            }
        }
    }

    pub fn load_installed(&mut self) {
        let selected_pm = self.selected_pm();
        self.installed_packages = selected_pm.map(|p| p.list_installed()).unwrap_or_default();
        if let Some(pm) = selected_pm {
            for pkg in &self.installed_packages {
                if let Some(desc) = self.installed_description_cached_for_pm(pm, pkg) {
                    self.package_descriptions.insert(pkg.clone(), Some(desc));
                }
            }
        }
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

    pub fn can_fetch_descriptions(&self) -> bool {
        has_internet()
    }

    pub fn fetch_package_description(&mut self, pkg: &str) -> Option<String> {
        if let Some(desc) = self.package_descriptions.get(pkg) {
            return desc.clone();
        }
        let selected_pm = self.selected_pm();
        let installed_pkg = self
            .installed_packages
            .iter()
            .any(|installed| installed == pkg);
        if installed_pkg {
            if let Some(pm) = selected_pm {
                if let Some(desc) = self.installed_description_cached_for_pm(pm, pkg) {
                    self.package_descriptions
                        .insert(pkg.to_string(), Some(desc.clone()));
                    return Some(desc);
                }
            }
        }
        let fetched = self
            .search_results
            .iter()
            .find(|r| r.pkg == pkg)
            .and_then(|r| r.description.clone())
            .or_else(|| {
                if has_internet() {
                    selected_pm.and_then(|pm| pm.package_description(pkg))
                } else {
                    None
                }
            });
        if installed_pkg {
            if let (Some(pm), Some(desc)) = (selected_pm, fetched.as_ref()) {
                self.persist_installed_description(pm, pkg, desc);
            }
        }
        self.package_descriptions
            .insert(pkg.to_string(), fetched.clone());
        fetched
    }

    pub fn cached_package_description(&self, pkg: &str) -> Option<String> {
        self.package_descriptions
            .get(pkg)
            .and_then(|desc| desc.clone())
    }

    pub fn pm_label(&mut self) -> &str {
        self.selected_pm().map(|p| p.name()).unwrap_or("Not Found")
    }

    fn reset_for_package_manager_change(&mut self) {
        self.search_results.clear();
        self.search_query.clear();
        self.search_page = 0;
        self.installed_packages.clear();
        self.installed_filter.clear();
        self.installed_page = 0;
        self.package_descriptions.clear();
        self.confirm_dialog = None;
        self.notice = None;
    }

    pub fn select_package_manager(&mut self, idx: usize) -> bool {
        self.ensure_available_pms();
        if idx >= self.available_pms.len() || idx == self.selected_pm_idx {
            return false;
        }
        self.selected_pm_idx = idx;
        self.reset_for_package_manager_change();
        true
    }

    pub fn confirm_action(&mut self) -> DesktopInstallerEvent {
        let Some(confirm) = self.confirm_dialog.take() else {
            return DesktopInstallerEvent::None;
        };
        match build_desktop_installer_event(self.selected_pm(), confirm, has_internet()) {
            Ok(DesktopInstallerEvent::LaunchCommand {
                argv,
                status,
                completion_message,
            }) => {
                self.status = status.clone();
                DesktopInstallerEvent::LaunchCommand {
                    argv,
                    status,
                    completion_message,
                }
            }
            Ok(DesktopInstallerEvent::None) => DesktopInstallerEvent::None,
            Err(err) => {
                self.status = err;
                DesktopInstallerEvent::None
            }
        }
    }

    pub fn add_to_menu(&mut self, pkg: &str, target: InstallerMenuTarget) {
        let display = if self.display_name_input.trim().is_empty() {
            pkg.to_string()
        } else {
            self.display_name_input.trim().to_string()
        };
        let val = serde_json::Value::Array(vec![serde_json::Value::String(pkg.to_string())]);
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

    pub fn runtime_tool_installed(&mut self, tool: RuntimeTool) -> bool {
        match tool {
            RuntimeTool::PlaySound => self.runtime_playsound_installed(),
            RuntimeTool::Blueutil => self.runtime_blueutil_installed(),
        }
    }

    fn refresh_runtime_tool_cache(&mut self) {
        if self.runtime_playsound_installed.is_none() {
            self.runtime_playsound_installed = Some(has_python_module("playsound"));
        }
        if cfg!(target_os = "macos") && self.runtime_blueutil_installed.is_none() {
            self.runtime_blueutil_installed = Some(which("blueutil"));
        }
    }

    pub fn runtime_tool_installed_cached(&mut self, tool: RuntimeTool) -> bool {
        self.refresh_runtime_tool_cache();
        match tool {
            RuntimeTool::PlaySound => self.runtime_playsound_installed.unwrap_or(false),
            RuntimeTool::Blueutil => self.runtime_blueutil_installed.unwrap_or(false),
        }
    }
}

fn installed_description_cache_path() -> PathBuf {
    base_dir().join("installed_package_descriptions.json")
}

fn load_installed_description_cache() -> HashMap<String, HashMap<String, String>> {
    std::fs::read_to_string(installed_description_cache_path())
        .ok()
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default()
}

fn save_installed_description_cache(cache: &HashMap<String, HashMap<String, String>>) {
    if let Ok(raw) = serde_json::to_string_pretty(cache) {
        let _ = std::fs::write(installed_description_cache_path(), raw);
    }
}

pub fn build_desktop_installer_event(
    pm: Option<PackageManager>,
    confirm: DesktopInstallerConfirm,
    has_internet: bool,
) -> Result<DesktopInstallerEvent, String> {
    let Some(pm) = pm else {
        return Err("Error: No package manager found.".to_string());
    };
    if !has_internet
        && matches!(
            confirm.action,
            InstallerPackageAction::Install | InstallerPackageAction::Update
        )
    {
        return Err("Error: No internet connection.".to_string());
    }
    let pkg = confirm.pkg;
    let argv = match confirm.action {
        InstallerPackageAction::Install => {
            if pkg == "playsound" {
                playsound_install_cmd()
            } else {
                pm.install_cmd(&pkg)
            }
        }
        InstallerPackageAction::Reinstall => {
            if pkg == "playsound" {
                playsound_reinstall_cmd()
            } else {
                pm.reinstall_cmd(&pkg)
            }
        }
        InstallerPackageAction::Update => {
            if pkg == "playsound" {
                playsound_update_cmd()
            } else {
                pm.update_cmd(&pkg)
            }
        }
        InstallerPackageAction::Uninstall => {
            if pkg == "playsound" {
                playsound_uninstall_cmd()
            } else {
                pm.remove_cmd(&pkg)
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
    Ok(DesktopInstallerEvent::LaunchCommand {
        argv,
        status,
        completion_message: Some(completion_message),
    })
}

pub fn runtime_tool_pkg(tool: RuntimeTool) -> &'static str {
    match tool {
        RuntimeTool::PlaySound => "playsound",
        RuntimeTool::Blueutil => "blueutil",
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
    let Some(pm) = state.selected_pm() else {
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
        InstallerEvent::Status(format!("Found {} result(s).", state.search_results.len()))
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
    state: &mut TerminalInstallerState,
    pkg: &str,
    action: InstallerPackageAction,
) -> InstallerEvent {
    let Some(pm) = state.selected_pm() else {
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
                        return InstallerEvent::Status(
                            "yay not found. Install yay first.".to_string(),
                        );
                    }
                    playsound_install_cmd()
                } else {
                    if !which("python3") {
                        return InstallerEvent::Status(
                            "python3 not found. Install Python first.".to_string(),
                        );
                    }
                    playsound_install_cmd()
                }
            } else {
                pm.install_cmd(pkg)
            }
        }
        InstallerPackageAction::Reinstall => {
            if pkg == "playsound" {
                if is_arch_based_linux() {
                    if !which("yay") {
                        return InstallerEvent::Status(
                            "yay not found. Install yay first.".to_string(),
                        );
                    }
                    playsound_reinstall_cmd()
                } else {
                    if !which("python3") {
                        return InstallerEvent::Status(
                            "python3 not found. Install Python first.".to_string(),
                        );
                    }
                    playsound_reinstall_cmd()
                }
            } else {
                pm.reinstall_cmd(pkg)
            }
        }
        InstallerPackageAction::Update => {
            if pkg == "playsound" {
                if is_arch_based_linux() {
                    if !which("yay") {
                        return InstallerEvent::Status(
                            "yay not found. Install yay first.".to_string(),
                        );
                    }
                    playsound_update_cmd()
                } else {
                    if !which("python3") {
                        return InstallerEvent::Status(
                            "python3 not found. Install Python first.".to_string(),
                        );
                    }
                    playsound_update_cmd()
                }
            } else {
                pm.update_cmd(pkg)
            }
        }
        InstallerPackageAction::Uninstall => {
            if pkg == "playsound" {
                if is_arch_based_linux() {
                    if !which("yay") {
                        return InstallerEvent::Status(
                            "yay not found. Install yay first.".to_string(),
                        );
                    }
                    playsound_uninstall_cmd()
                } else {
                    if !which("python3") {
                        return InstallerEvent::Status(
                            "python3 not found. Install Python first.".to_string(),
                        );
                    }
                    playsound_uninstall_cmd()
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
    let val = serde_json::Value::Array(vec![serde_json::Value::String(pkg.to_string())]);
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

impl TerminalInstallerState {
    pub fn ensure_available_pms(&mut self) {
        if self.available_pms.is_empty() {
            self.available_pms = PackageManager::detect_all();
            self.selected_pm_idx = self
                .selected_pm_idx
                .min(self.available_pms.len().saturating_sub(1));
            self.pm_select_idx = self
                .pm_select_idx
                .min(self.available_pms.len().saturating_sub(1));
        }
    }

    pub fn is_at_root(&self) -> bool {
        matches!(self.view, InstallerView::Root)
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }

    pub fn back(&mut self) -> bool {
        match self.view {
            InstallerView::Root => true,
            InstallerView::PackageManagerSelect
            | InstallerView::SearchResults
            | InstallerView::Installed
            | InstallerView::RuntimeTools => {
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

    pub fn package_description(&mut self, pkg: &str) -> Option<String> {
        if let Some(desc) = self.package_descriptions.get(pkg) {
            return desc.clone();
        }
        let fetched = self
            .search_results
            .iter()
            .find(|r| r.pkg == pkg)
            .and_then(|r| r.description.clone())
            .or_else(|| {
                self.selected_pm()
                    .and_then(|manager| manager.package_description(pkg))
            });
        self.package_descriptions
            .insert(pkg.to_string(), fetched.clone());
        fetched
    }

    pub fn cached_package_description(&self, pkg: &str) -> Option<String> {
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

    pub fn runtime_tool_installed_cached(&mut self, tool: RuntimeTool) -> bool {
        self.refresh_runtime_tool_cache();
        match tool {
            RuntimeTool::PlaySound => self.runtime_playsound_installed.unwrap_or(false),
            RuntimeTool::Blueutil => self.runtime_blueutil_installed.unwrap_or(false),
        }
    }

    pub fn invalidate_runtime_tool_cache_for_pkg(&mut self, pkg: &str) {
        match pkg {
            "playsound" => self.runtime_playsound_installed = None,
            "blueutil" => self.runtime_blueutil_installed = None,
            _ => {}
        }
    }

    pub fn clear_runtime_tool_caches(&mut self) {
        self.runtime_playsound_installed = None;
        self.runtime_blueutil_installed = None;
    }

    pub fn selected_pm(&mut self) -> Option<PackageManager> {
        self.ensure_available_pms();
        self.available_pms.get(self.selected_pm_idx).copied()
    }

    pub fn pm_label(&mut self) -> &str {
        self.selected_pm()
            .map(|pm| pm.name())
            .unwrap_or("Not Found")
    }

    fn reset_for_package_manager_change(&mut self) {
        self.search_results.clear();
        self.search_query.clear();
        self.search_idx = 0;
        self.search_page = 0;
        self.installed_packages.clear();
        self.installed_filter.clear();
        self.installed_idx = 0;
        self.installed_page = 0;
        self.action_idx = 0;
        self.add_menu_idx = 0;
        self.package_descriptions.clear();
    }

    pub fn select_package_manager(&mut self, idx: usize) -> bool {
        if idx >= self.available_pms.len() || idx == self.selected_pm_idx {
            self.pm_select_idx = self
                .selected_pm_idx
                .min(self.available_pms.len().saturating_sub(1));
            return false;
        }
        self.selected_pm_idx = idx;
        self.pm_select_idx = idx;
        self.reset_for_package_manager_change();
        true
    }
}

fn search_pkg_name(pm: PackageManager, line: &str) -> Option<String> {
    let line = line.trim_end();
    if line.is_empty() || line.starts_with('=') || line.starts_with("warning:") {
        return None;
    }
    if matches!(pm, PackageManager::Pacman | PackageManager::Yay) && line.starts_with(' ') {
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

fn playsound_install_cmd() -> Vec<String> {
    if is_arch_based_linux() {
        vec![
            command_name_or_path("yay"),
            "-S".into(),
            "--noconfirm".into(),
            "python-playsound".into(),
        ]
    } else {
        vec![
            command_name_or_path("python3"),
            "-m".into(),
            "pip".into(),
            "install".into(),
            "--user".into(),
            "--upgrade".into(),
            "playsound".into(),
        ]
    }
}

fn playsound_reinstall_cmd() -> Vec<String> {
    if is_arch_based_linux() {
        playsound_install_cmd()
    } else {
        vec![
            command_name_or_path("python3"),
            "-m".into(),
            "pip".into(),
            "install".into(),
            "--user".into(),
            "--upgrade".into(),
            "--force-reinstall".into(),
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
            command_name_or_path("yay"),
            "-R".into(),
            "--noconfirm".into(),
            "python-playsound".into(),
        ]
    } else {
        vec![
            command_name_or_path("python3"),
            "-m".into(),
            "pip".into(),
            "uninstall".into(),
            "-y".into(),
            "playsound".into(),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn unique_temp_dir(label: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("robcos-installer-{label}-{unique}"));
        fs::create_dir_all(&dir).expect("create temp dir");
        dir
    }

    #[test]
    fn back_from_search_actions_returns_to_results() {
        let mut state = TerminalInstallerState {
            view: InstallerView::SearchActions {
                pkg: "fd".to_string(),
            },
            ..TerminalInstallerState::default()
        };
        assert!(!state.back());
        assert!(matches!(state.view, InstallerView::SearchResults));
    }

    #[test]
    fn select_package_manager_resets_search_and_installed_state() {
        let mut state = TerminalInstallerState {
            available_pms: vec![PackageManager::Brew, PackageManager::Apt],
            selected_pm_idx: 0,
            pm_select_idx: 0,
            search_results: vec![SearchResult {
                raw: "ripgrep - line search".to_string(),
                pkg: "ripgrep".to_string(),
                description: Some("line search".to_string()),
                installed: true,
            }],
            search_query: "ripgrep".to_string(),
            installed_packages: vec!["ripgrep".to_string()],
            installed_filter: "rip".to_string(),
            search_idx: 2,
            installed_idx: 3,
            action_idx: 1,
            add_menu_idx: 1,
            ..TerminalInstallerState::default()
        };
        assert!(state.select_package_manager(1));
        assert_eq!(state.selected_pm(), Some(PackageManager::Apt));
        assert!(state.search_results.is_empty());
        assert!(state.installed_packages.is_empty());
        assert!(state.search_query.is_empty());
        assert!(state.installed_filter.is_empty());
        assert_eq!(state.search_idx, 0);
        assert_eq!(state.installed_idx, 0);
    }

    #[test]
    fn desktop_installer_event_builds_install_launch() {
        let event = build_desktop_installer_event(
            Some(PackageManager::Brew),
            DesktopInstallerConfirm {
                pkg: "fd".to_string(),
                action: InstallerPackageAction::Install,
            },
            true,
        )
        .expect("desktop launch event");
        match event {
            DesktopInstallerEvent::LaunchCommand {
                argv,
                status,
                completion_message,
            } => {
                assert!(argv.first().is_some_and(|cmd| cmd.ends_with("brew")));
                assert_eq!(&argv[1..], ["install", "fd"]);
                assert_eq!(status, "Installing fd...");
                assert_eq!(completion_message, Some("fd installed.".to_string()));
            }
            DesktopInstallerEvent::None => panic!("unexpected none event"),
        }
    }

    #[test]
    fn resolve_command_path_from_search_dirs_finds_binary_outside_path() {
        let dir = unique_temp_dir("brew-path");
        let brew = dir.join("brew");
        fs::write(&brew, b"#!/bin/sh\n").expect("write fake brew");

        let resolved = resolve_command_path_from_search_dirs("brew", &[dir.clone()]);
        assert_eq!(resolved.as_deref(), Some(brew.as_path()));

        let _ = fs::remove_dir_all(dir);
    }
}
