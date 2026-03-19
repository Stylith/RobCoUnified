#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StartSubmenu {
    System,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StartLeaf {
    Applications,
    Documents,
    Network,
    Games,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum StartSystemAction {
    ProgramInstaller,
    Terminal,
    FileManager,
    Settings,
    Connections,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum StartRootAction {
    ReturnToTerminal,
    Logout,
    Shutdown,
}

pub(super) const START_ROOT_ITEMS: [&str; 8] = [
    "Applications",
    "Documents",
    "Network",
    "Games",
    "System",
    "Return To Terminal Mode",
    "Logout",
    "Shutdown",
];

pub(super) const START_ROOT_VIS_ROWS: [Option<usize>; 9] = [
    Some(0),
    Some(1),
    Some(2),
    Some(3),
    Some(4),
    None,
    Some(5),
    Some(6),
    Some(7),
];

pub(super) const START_SYSTEM_ITEMS: [(&str, StartSystemAction); 5] = [
    ("Program Installer", StartSystemAction::ProgramInstaller),
    ("Terminal", StartSystemAction::Terminal),
    ("File Manager", StartSystemAction::FileManager),
    ("Settings", StartSystemAction::Settings),
    ("Connections", StartSystemAction::Connections),
];

pub(super) fn start_root_leaf_for_idx(idx: usize) -> Option<StartLeaf> {
    match idx {
        0 => Some(StartLeaf::Applications),
        1 => Some(StartLeaf::Documents),
        2 => Some(StartLeaf::Network),
        3 => Some(StartLeaf::Games),
        _ => None,
    }
}

pub(super) fn start_root_submenu_for_idx(idx: usize) -> Option<StartSubmenu> {
    if idx == 4 {
        Some(StartSubmenu::System)
    } else {
        None
    }
}

pub(super) fn start_root_action_for_idx(idx: usize) -> Option<StartRootAction> {
    match idx {
        5 => Some(StartRootAction::ReturnToTerminal),
        6 => Some(StartRootAction::Logout),
        7 => Some(StartRootAction::Shutdown),
        _ => None,
    }
}
