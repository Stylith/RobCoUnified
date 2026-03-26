use super::super::desktop_app::DesktopShellAction;
use super::super::desktop_launcher_service::{catalog_names, ProgramCatalog, ROBCO_FUN_MENU_LABEL};
use super::super::desktop_search_service::{
    start_application_entries, start_document_entries, start_network_entries,
    NativeStartLeafAction, NativeStartLeafEntry,
};
use super::super::edit_menus_screen::EditMenuTarget;
use super::super::editor_app::EDITOR_APP_TITLE;
use super::super::prompt::FlashAction;
use super::super::retro_ui::current_palette;
use crate::config::install_profile;
use crate::native::{installed_hosted_game_names, is_installed_hosted_game};
use crate::platform::{InstallProfile, LaunchTarget};
use eframe::egui::{self, Align2, Color32, Context, FontFamily, FontId, Id, Key, RichText};

use super::RobcoNativeApp;

use crate::sound;

// ── Context menu actions (referenced in draw_start_panel) ───────────────────
// These are defined in app.rs; we import them via super.
use super::ContextMenuAction;

// ── Types ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum StartSubmenu {
    System,
    RobCoFun,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum StartLeaf {
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

// ── Constants ─────────────────────────────────────────────────────────────────

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

// ── Free functions ────────────────────────────────────────────────────────────

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

fn start_system_action_visibility_target(action: StartSystemAction) -> LaunchTarget {
    match action {
        StartSystemAction::ProgramInstaller => super::launch_registry::installer_launch_target(),
        StartSystemAction::Terminal => super::launch_registry::terminal_launch_target(),
        StartSystemAction::FileManager => super::launch_registry::file_manager_launch_target(),
        StartSystemAction::Settings => super::launch_registry::settings_launch_target(),
        StartSystemAction::Connections => super::launch_registry::connections_launch_target(),
    }
}

pub(super) fn start_system_items_for_profile(
    profile: InstallProfile,
) -> Vec<(&'static str, StartSystemAction)> {
    START_SYSTEM_ITEMS
        .iter()
        .copied()
        .filter(|(_, action)| {
            super::launch_registry::desktop_launch_target_available_for_profile(
                &start_system_action_visibility_target(*action),
                profile,
            )
        })
        .collect()
}

// ── impl RobcoNativeApp ───────────────────────────────────────────────────────

impl RobcoNativeApp {
    pub(super) fn open_start_menu(&mut self) {
        self.close_spotlight();
        self.start_open = true;
        self.start_selected_root = 0;
        self.start_system_selected = 0;
        self.start_leaf_selected = 0;
        self.start_open_submenu = None;
        self.start_open_leaf = None;
    }

    pub(super) fn close_start_menu(&mut self) {
        self.start_open = false;
        self.start_open_submenu = None;
        self.start_open_leaf = None;
    }

    pub(super) fn close_start_menu_panel(&mut self) {
        self.start_open_submenu = None;
        self.start_open_leaf = None;
        self.start_system_selected = 0;
        self.start_leaf_selected = 0;
    }

    pub(super) fn start_menu_open_current_panel(&mut self) {
        let idx = self.start_selected_root;
        if start_root_leaf_for_idx(idx).is_some() || start_root_submenu_for_idx(idx).is_some() {
            self.set_start_panel_for_root(idx);
        }
    }

    pub(super) fn start_menu_move_root_selection(&mut self, delta: isize) {
        let max_idx = START_ROOT_ITEMS.len().saturating_sub(1) as isize;
        let next = (self.start_selected_root as isize + delta).clamp(0, max_idx) as usize;
        if next == self.start_selected_root {
            return;
        }
        if self.start_open_leaf.is_some() || self.start_open_submenu.is_some() {
            self.set_start_panel_for_root(next);
        } else {
            self.start_selected_root = next;
        }
    }

    pub(super) fn start_menu_move_panel_selection(&mut self, delta: isize) {
        if let Some(submenu) = self.start_open_submenu {
            let items_len = match submenu {
                StartSubmenu::System => self.start_system_items().len(),
                StartSubmenu::RobCoFun => self.start_robco_fun_items().len(),
            };
            if items_len > 0 {
                let max_idx = items_len.saturating_sub(1) as isize;
                self.start_system_selected =
                    (self.start_system_selected as isize + delta).clamp(0, max_idx) as usize;
            }
        } else if let Some(leaf) = self.start_open_leaf {
            let items_len = self.start_leaf_items(leaf).len();
            if items_len > 0 {
                let max_idx = items_len.saturating_sub(1) as isize;
                self.start_leaf_selected =
                    (self.start_leaf_selected as isize + delta).clamp(0, max_idx) as usize;
            }
        } else {
            self.start_menu_move_root_selection(delta);
        }
    }

    pub(super) fn activate_start_menu_selection(&mut self) {
        if let Some(submenu) = self.start_open_submenu {
            match submenu {
                StartSubmenu::System => {
                    let items = self.start_system_items();
                    if let Some((_, action)) = items.get(self.start_system_selected) {
                        self.run_start_system_action(*action);
                    }
                }
                StartSubmenu::RobCoFun => {
                    let items = self.start_robco_fun_items();
                    if let Some(item) = items.get(self.start_system_selected) {
                        self.run_start_leaf_action(item.action.clone());
                    }
                }
            }
            return;
        }

        if let Some(leaf) = self.start_open_leaf {
            let items = self.start_leaf_items(leaf);
            if let Some(item) = items.get(self.start_leaf_selected) {
                if leaf == StartLeaf::Games && item.label == ROBCO_FUN_MENU_LABEL {
                    self.start_open_submenu = Some(StartSubmenu::RobCoFun);
                    self.start_system_selected = 0;
                } else {
                    self.run_start_leaf_action(item.action.clone());
                }
            }
            return;
        }

        if let Some(action) = start_root_action_for_idx(self.start_selected_root) {
            self.run_start_root_action(action);
        } else {
            self.start_menu_open_current_panel();
        }
    }

    pub(super) fn handle_start_menu_keyboard(&mut self, ctx: &Context) {
        if !self.start_open {
            return;
        }

        let mut handled = false;
        ctx.input_mut(|i| {
            if i.key_pressed(Key::ArrowUp) {
                self.start_menu_move_panel_selection(-1);
                i.consume_key(egui::Modifiers::NONE, Key::ArrowUp);
                handled = true;
            } else if i.key_pressed(Key::ArrowDown) {
                self.start_menu_move_panel_selection(1);
                i.consume_key(egui::Modifiers::NONE, Key::ArrowDown);
                handled = true;
            } else if i.key_pressed(Key::ArrowRight) {
                if self.start_open_submenu == Some(StartSubmenu::RobCoFun) {
                    // Already inside the innermost games submenu.
                } else if self.start_open_leaf == Some(StartLeaf::Games) {
                    let items = self.start_leaf_items(StartLeaf::Games);
                    if items
                        .get(self.start_leaf_selected)
                        .is_some_and(|item| item.label == ROBCO_FUN_MENU_LABEL)
                    {
                        self.start_open_submenu = Some(StartSubmenu::RobCoFun);
                        self.start_system_selected = 0;
                    }
                } else if self.start_open_leaf.is_none() && self.start_open_submenu.is_none() {
                    self.start_menu_open_current_panel();
                }
                i.consume_key(egui::Modifiers::NONE, Key::ArrowRight);
                handled = true;
            } else if i.key_pressed(Key::ArrowLeft) {
                if self.start_open_submenu == Some(StartSubmenu::RobCoFun) {
                    self.start_open_submenu = None;
                    self.start_system_selected = 0;
                } else if self.start_open_leaf.is_some() || self.start_open_submenu.is_some() {
                    self.close_start_menu_panel();
                } else {
                    self.close_start_menu();
                }
                i.consume_key(egui::Modifiers::NONE, Key::ArrowLeft);
                handled = true;
            } else if i.key_pressed(Key::Enter) {
                self.activate_start_menu_selection();
                i.consume_key(egui::Modifiers::NONE, Key::Enter);
                handled = true;
            }
        });

        if handled {
            self.close_spotlight();
        }
    }

    pub(super) fn set_start_panel_for_root(&mut self, root_idx: usize) {
        self.start_selected_root = root_idx.min(START_ROOT_ITEMS.len().saturating_sub(1));
        self.start_open_leaf = start_root_leaf_for_idx(self.start_selected_root);
        self.start_open_submenu = start_root_submenu_for_idx(self.start_selected_root);
        self.start_leaf_selected = 0;
        self.start_system_selected = 0;
    }

    pub(super) fn start_system_items(&self) -> Vec<(&'static str, StartSystemAction)> {
        start_system_items_for_profile(install_profile())
    }

    pub(super) fn start_leaf_menu_target(
        action: &NativeStartLeafAction,
    ) -> Option<(EditMenuTarget, String)> {
        match action {
            NativeStartLeafAction::LaunchConfiguredApp(name) => {
                Some((EditMenuTarget::Applications, name.clone()))
            }
            NativeStartLeafAction::LaunchNetworkProgram(name) => {
                Some((EditMenuTarget::Network, name.clone()))
            }
            NativeStartLeafAction::LaunchGameProgram(name) if !is_installed_hosted_game(name) => {
                Some((EditMenuTarget::Games, name.clone()))
            }
            _ => None,
        }
    }

    pub(super) fn start_robco_fun_items(&self) -> Vec<NativeStartLeafEntry> {
        installed_hosted_game_names()
            .into_iter()
            .map(|label| NativeStartLeafEntry {
                action: NativeStartLeafAction::LaunchGameProgram(label.clone()),
                label,
            })
            .collect()
    }

    pub(super) fn start_leaf_items(&self, leaf: StartLeaf) -> Vec<NativeStartLeafEntry> {
        let profile = install_profile();
        match leaf {
            StartLeaf::Applications => start_application_entries(
                self.settings.draft.builtin_menu_visibility.text_editor
                    && super::launch_registry::desktop_launch_target_available_for_profile(
                        &super::launch_registry::editor_launch_target(),
                        profile,
                    ),
                EDITOR_APP_TITLE,
            ),
            StartLeaf::Documents => start_document_entries(
                self.session
                    .as_ref()
                    .map(|session| session.username.as_str()),
            ),
            StartLeaf::Network => start_network_entries(),
            StartLeaf::Games => {
                let hosted_games = installed_hosted_game_names();
                let mut other_games = catalog_names(ProgramCatalog::Games);
                other_games.retain(|name| !hosted_games.iter().any(|hosted| hosted == name));
                let mut items = Vec::new();
                if !hosted_games.is_empty() {
                    items.push(NativeStartLeafEntry {
                        label: ROBCO_FUN_MENU_LABEL.to_string(),
                        action: NativeStartLeafAction::None,
                    });
                }
                items.extend(
                    other_games
                        .into_iter()
                        .map(|label| NativeStartLeafEntry {
                            action: NativeStartLeafAction::LaunchGameProgram(label.clone()),
                            label,
                        }),
                );
                if items.is_empty() {
                    items.push(NativeStartLeafEntry {
                        label: "(No games installed)".to_string(),
                        action: NativeStartLeafAction::None,
                    });
                }
                items
            }
        }
    }

    pub(super) fn run_start_root_action(&mut self, action: StartRootAction) {
        match action {
            StartRootAction::ReturnToTerminal => {
                self.close_start_menu();
                sound::play_logout();
                self.terminate_all_native_pty_children();
                self.close_all_desktop_windows();
                self.desktop_mode_open = false;
            }
            StartRootAction::Logout => {
                self.close_start_menu();
                self.begin_logout();
            }
            StartRootAction::Shutdown => {
                self.close_start_menu();
                self.queue_terminal_flash("Shutting down...", 800, FlashAction::ExitApp);
            }
        }
    }

    pub(super) fn run_start_system_action(&mut self, action: StartSystemAction) {
        self.close_start_menu();
        let action = match action {
            StartSystemAction::ProgramInstaller => {
                DesktopShellAction::LaunchByTarget(super::launch_registry::installer_launch_target())
            }
            StartSystemAction::Terminal => DesktopShellAction::LaunchByTargetWithPayload {
                target: super::launch_registry::terminal_launch_target(),
                payload: super::super::desktop_app::DesktopLaunchPayload::OpenTerminalShell,
            },
            StartSystemAction::FileManager => DesktopShellAction::LaunchByTarget(
                super::launch_registry::file_manager_launch_target(),
            ),
            StartSystemAction::Settings => {
                DesktopShellAction::LaunchByTarget(super::launch_registry::settings_launch_target())
            }
            StartSystemAction::Connections => DesktopShellAction::LaunchByTarget(
                super::launch_registry::connections_launch_target(),
            ),
        };
        self.execute_desktop_shell_action(action);
    }

    pub(super) fn run_start_leaf_action(&mut self, action: NativeStartLeafAction) {
        let action = match action {
            NativeStartLeafAction::None => return,
            NativeStartLeafAction::OpenTextEditor => {
                DesktopShellAction::LaunchByTarget(super::launch_registry::editor_launch_target())
            }
            NativeStartLeafAction::LaunchConfiguredApp(name) => {
                DesktopShellAction::LaunchConfiguredApp(name)
            }
            NativeStartLeafAction::OpenDocumentCategory(path) => {
                DesktopShellAction::OpenFileManagerAt(path)
            }
            NativeStartLeafAction::LaunchNetworkProgram(name) => {
                DesktopShellAction::LaunchNetworkProgram(name)
            }
            NativeStartLeafAction::LaunchGameProgram(name) => {
                DesktopShellAction::LaunchGameProgram(name)
            }
        };
        self.execute_desktop_shell_action(action);
    }

    pub(super) fn draw_start_panel(&mut self, ctx: &Context) {
        if !self.start_open {
            return;
        }
        const ROOT_W: f32 = 270.0;
        const SUB_W: f32 = 250.0;
        const LEAF_W: f32 = 270.0;
        const ROW_H: f32 = 24.0;
        const PANEL_PAD_H: f32 = 16.0;
        const TASKBAR_H: f32 = 32.0;
        const ROOT_LEFT: f32 = 8.0;
        const EDGE_PAD: f32 = 8.0;

        let palette = current_palette();
        let screen = ctx.screen_rect();
        let taskbar_top = screen.bottom() - TASKBAR_H;
        let root_x = self
            .desktop_start_button_rect
            .map(|rect| rect.left().max(screen.left() + ROOT_LEFT))
            .unwrap_or(screen.left() + ROOT_LEFT);
        let root_y = (taskbar_top - self.start_root_panel_height).max(screen.top() + EDGE_PAD);
        let mut branch_anchor_y = screen.top() + EDGE_PAD;
        let mut branch_x = root_x + ROOT_W - 2.0;
        let mut root_rect: Option<egui::Rect> = None;

        egui::Area::new(Id::new("native_start_root_panel"))
            .fixed_pos([root_x, root_y])
            .interactable(true)
            .show(ctx, |ui| {
                let frame = egui::Frame::none()
                    .fill(palette.panel)
                    .stroke(egui::Stroke::new(2.0, palette.fg))
                    .inner_margin(egui::Margin::same(8.0));
                let frame_response = frame.show(ui, |ui| {
                    Self::apply_start_menu_highlight_style(ui);
                    ui.set_min_width(ROOT_W);
                    ui.set_max_width(ROOT_W);
                    ui.label(RichText::new("Start").strong().color(palette.fg));
                    Self::retro_separator(ui);

                    for row in START_ROOT_VIS_ROWS {
                        match row {
                            Some(idx) => {
                                let label = START_ROOT_ITEMS[idx];
                                let has_panel = start_root_leaf_for_idx(idx).is_some()
                                    || start_root_submenu_for_idx(idx).is_some();
                                let suffix = if has_panel { " >" } else { "" };
                                let selected = self.start_selected_root == idx;
                                let response = Self::start_menu_row(
                                    ui,
                                    &format!("{label}{suffix}"),
                                    selected,
                                    ROOT_W - 16.0,
                                );
                                if response.hovered() {
                                    self.set_start_panel_for_root(idx);
                                }
                                if response.clicked() {
                                    if let Some(action) = start_root_action_for_idx(idx) {
                                        self.run_start_root_action(action);
                                    } else if has_panel {
                                        self.set_start_panel_for_root(idx);
                                    }
                                }
                                if self.start_selected_root == idx {
                                    branch_anchor_y = response.rect.top() - 2.0;
                                }
                            }
                            None => {
                                Self::retro_separator(ui);
                            }
                        }
                    }
                });
                root_rect = Some(frame_response.response.rect);
                self.start_root_panel_height = frame_response.response.rect.height();
                branch_x = frame_response.response.rect.right() - 2.0;
            });
        let Some(root_rect) = root_rect else {
            return;
        };

        let mut leaf_rect: Option<egui::Rect> = None;
        let mut leaf_branch_anchor_y = screen.top() + EDGE_PAD;
        let mut leaf_branch_x = branch_x + LEAF_W - 2.0;

        if let Some(leaf) = self.start_open_leaf {
            let items = self.start_leaf_items(leaf);
            self.start_leaf_selected = self.start_leaf_selected.min(items.len().saturating_sub(1));
            let leaf_h = PANEL_PAD_H + ROW_H * (items.len() as f32);
            let leaf_y =
                branch_anchor_y.clamp(screen.top() + EDGE_PAD, root_rect.bottom() - leaf_h);
            let mut leaf_context_action: Option<ContextMenuAction> = None;
            egui::Area::new(Id::new("native_start_leaf_panel"))
                .fixed_pos([branch_x, leaf_y])
                .interactable(true)
                .show(ctx, |ui| {
                    let frame_response = egui::Frame::none()
                        .fill(palette.panel)
                        .stroke(egui::Stroke::new(2.0, palette.fg))
                        .inner_margin(egui::Margin::same(8.0))
                        .show(ui, |ui| {
                            Self::apply_start_menu_highlight_style(ui);
                            ui.set_min_width(LEAF_W);
                            ui.set_max_width(LEAF_W);
                            for (idx, item) in items.iter().enumerate() {
                                let selected = self.start_leaf_selected == idx;
                                let display_label = if leaf == StartLeaf::Games
                                    && item.label == ROBCO_FUN_MENU_LABEL
                                {
                                    format!("{} >", item.label)
                                } else {
                                    item.label.clone()
                                };
                                let response = Self::start_menu_row(
                                    ui,
                                    &display_label,
                                    selected,
                                    LEAF_W - 16.0,
                                );
                                if response.hovered() {
                                    self.start_leaf_selected = idx;
                                    if leaf == StartLeaf::Games
                                        && item.label == ROBCO_FUN_MENU_LABEL
                                    {
                                        self.start_open_submenu = Some(StartSubmenu::RobCoFun);
                                    } else {
                                        self.start_open_submenu = None;
                                    }
                                }
                                if response.clicked() {
                                    if leaf == StartLeaf::Games
                                        && item.label == ROBCO_FUN_MENU_LABEL
                                    {
                                        self.start_open_submenu = Some(StartSubmenu::RobCoFun);
                                        self.start_system_selected = 0;
                                    } else {
                                        self.start_open_submenu = None;
                                        self.run_start_leaf_action(item.action.clone());
                                    }
                                }
                                if leaf == StartLeaf::Games
                                    && item.label == ROBCO_FUN_MENU_LABEL
                                    && (selected
                                        || self.start_open_submenu == Some(StartSubmenu::RobCoFun))
                                {
                                    leaf_branch_anchor_y = response.rect.top() - 2.0;
                                }
                                if matches!(
                                    leaf,
                                    StartLeaf::Applications | StartLeaf::Games | StartLeaf::Network
                                ) && !matches!(item.action, NativeStartLeafAction::None)
                                {
                                    let item_label = item.label.clone();
                                    let item_action = item.action.clone();
                                    let removable_item = Self::start_leaf_menu_target(&item_action);
                                    response.context_menu(|ui| {
                                        Self::apply_context_menu_style(ui);
                                        ui.set_min_width(136.0);
                                        ui.set_max_width(180.0);
                                        if let Some((target, name)) = removable_item.as_ref() {
                                            if ui.button("Rename").clicked() {
                                                leaf_context_action =
                                                    Some(ContextMenuAction::RenameStartMenuEntry {
                                                        target: *target,
                                                        name: name.clone(),
                                                    });
                                                ui.close_menu();
                                            }
                                            Self::retro_separator(ui);
                                        }
                                        if ui.button("Create Shortcut").clicked() {
                                            leaf_context_action =
                                                Some(ContextMenuAction::CreateShortcut {
                                                    label: item_label.clone(),
                                                    action: item_action.clone(),
                                                });
                                            ui.close_menu();
                                        }
                                        if let Some((target, name)) = removable_item.as_ref() {
                                            Self::retro_separator(ui);
                                            if ui
                                                .button(format!("Remove from {}", target.title()))
                                                .clicked()
                                            {
                                                leaf_context_action =
                                                    Some(ContextMenuAction::RemoveStartMenuEntry {
                                                        target: *target,
                                                        name: name.clone(),
                                                    });
                                                ui.close_menu();
                                            }
                                        }
                                    });
                                }
                            }
                        });
                    leaf_rect = Some(frame_response.response.rect);
                    leaf_branch_x = frame_response.response.rect.right() - 2.0;
                });
            if let Some(action) = leaf_context_action {
                self.context_menu_action = Some(action);
            }
        }

        if let Some(submenu) = self.start_open_submenu {
            match submenu {
                StartSubmenu::System => {
                    let items = self.start_system_items();
                    self.start_system_selected = self
                        .start_system_selected
                        .min(items.len().saturating_sub(1));
                    let sub_h = PANEL_PAD_H + ROW_H * (items.len() as f32);
                    let sub_y =
                        branch_anchor_y.clamp(screen.top() + EDGE_PAD, root_rect.bottom() - sub_h);
                    egui::Area::new(Id::new("native_start_submenu_panel"))
                        .fixed_pos([branch_x, sub_y])
                        .interactable(true)
                        .show(ctx, |ui| {
                            egui::Frame::none()
                                .fill(palette.panel)
                                .stroke(egui::Stroke::new(2.0, palette.fg))
                                .inner_margin(egui::Margin::same(8.0))
                                .show(ui, |ui| {
                                    Self::apply_start_menu_highlight_style(ui);
                                    ui.set_min_width(SUB_W);
                                    ui.set_max_width(SUB_W);
                                    for (idx, (label, action)) in items.iter().enumerate() {
                                        let selected = self.start_system_selected == idx;
                                        let response =
                                            Self::start_menu_row(ui, label, selected, SUB_W - 16.0);
                                        if response.hovered() {
                                            self.start_system_selected = idx;
                                        }
                                        if response.clicked() {
                                            self.run_start_system_action(*action);
                                        }
                                    }
                                });
                        });
                }
                StartSubmenu::RobCoFun => {
                    let items = self.start_robco_fun_items();
                    self.start_system_selected = self
                        .start_system_selected
                        .min(items.len().saturating_sub(1));
                    let anchor_bottom = leaf_rect
                        .map(|rect| rect.bottom())
                        .unwrap_or(root_rect.bottom());
                    let sub_h = PANEL_PAD_H + ROW_H * (items.len() as f32);
                    let sub_y =
                        leaf_branch_anchor_y.clamp(screen.top() + EDGE_PAD, anchor_bottom - sub_h);
                    egui::Area::new(Id::new("native_start_robco_fun_submenu_panel"))
                        .fixed_pos([leaf_branch_x, sub_y])
                        .interactable(true)
                        .show(ctx, |ui| {
                            egui::Frame::none()
                                .fill(palette.panel)
                                .stroke(egui::Stroke::new(2.0, palette.fg))
                                .inner_margin(egui::Margin::same(8.0))
                                .show(ui, |ui| {
                                    Self::apply_start_menu_highlight_style(ui);
                                    ui.set_min_width(LEAF_W);
                                    ui.set_max_width(LEAF_W);
                                    for (idx, item) in items.iter().enumerate() {
                                        let selected = self.start_system_selected == idx;
                                        let response = Self::start_menu_row(
                                            ui,
                                            &item.label,
                                            selected,
                                            LEAF_W - 16.0,
                                        );
                                        if response.hovered() {
                                            self.start_system_selected = idx;
                                        }
                                        if response.clicked() {
                                            self.run_start_leaf_action(item.action.clone());
                                        }
                                    }
                                });
                        });
                }
            }
        }
    }

    pub(super) fn draw_start_menu_rename_window(&mut self, ctx: &Context) {
        let Some(rename) = self.start_menu_rename.clone() else {
            return;
        };

        let palette = current_palette();
        let mut close = false;
        let mut apply = false;
        let mut name_input = rename.name_input.clone();

        egui::Window::new("start_menu_rename_window")
            .title_bar(false)
            .collapsible(false)
            .resizable(false)
            .fixed_size(egui::vec2(320.0, 124.0))
            .anchor(Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
            .frame(
                egui::Frame::none()
                    .fill(palette.panel)
                    .stroke(egui::Stroke::new(2.0, palette.fg))
                    .inner_margin(egui::Margin::same(12.0)),
            )
            .show(ctx, |ui| {
                Self::apply_context_menu_style(ui);
                ui.label(
                    RichText::new(format!("Rename {}", rename.target.singular()))
                        .strong()
                        .color(palette.fg),
                );
                ui.add_space(8.0);
                ui.label(RichText::new(&rename.original_name).color(palette.dim));
                ui.add_space(6.0);
                let response = ui.add(
                    egui::TextEdit::singleline(&mut name_input)
                        .desired_width(f32::INFINITY)
                        .text_color(palette.fg)
                        .cursor_at_end(true),
                );
                if response.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)) {
                    apply = true;
                }
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    if ui.button("Rename").clicked() {
                        apply = true;
                    }
                    if ui.button("Cancel").clicked() {
                        close = true;
                    }
                });
            });

        if let Some(rename_state) = &mut self.start_menu_rename {
            rename_state.name_input = name_input;
        }
        if apply {
            if let Some(rename_state) = self.start_menu_rename.take() {
                self.rename_program_entry(
                    rename_state.target,
                    &rename_state.original_name,
                    &rename_state.name_input,
                );
            }
            self.close_start_menu();
        } else if close {
            self.start_menu_rename = None;
        }
    }

    pub(super) fn apply_start_menu_highlight_style(ui: &mut egui::Ui) {
        let palette = current_palette();
        let mut style = ui.style().as_ref().clone();
        let stroke = egui::Stroke::new(2.0, palette.fg);
        style.visuals.window_stroke = stroke;
        style.visuals.window_rounding = egui::Rounding::ZERO;
        style.visuals.menu_rounding = egui::Rounding::ZERO;
        style.visuals.window_shadow = egui::epaint::Shadow::NONE;
        style.visuals.popup_shadow = egui::epaint::Shadow::NONE;
        style.visuals.selection.bg_fill = palette.fg;
        style.visuals.selection.stroke = stroke;
        style.visuals.widgets.noninteractive.bg_fill = palette.panel;
        style.visuals.widgets.noninteractive.weak_bg_fill = palette.panel;
        style.visuals.widgets.noninteractive.bg_stroke = stroke;
        style.visuals.widgets.noninteractive.fg_stroke = stroke;
        style.visuals.widgets.noninteractive.rounding = egui::Rounding::ZERO;
        style.visuals.widgets.noninteractive.expansion = 0.0;
        style.visuals.widgets.inactive.bg_fill = palette.panel;
        style.visuals.widgets.inactive.weak_bg_fill = palette.panel;
        style.visuals.widgets.inactive.bg_stroke = stroke;
        style.visuals.widgets.inactive.fg_stroke = stroke;
        style.visuals.widgets.inactive.rounding = egui::Rounding::ZERO;
        style.visuals.widgets.inactive.expansion = 0.0;
        style.visuals.widgets.hovered.bg_fill = palette.fg;
        style.visuals.widgets.hovered.weak_bg_fill = palette.fg;
        style.visuals.widgets.hovered.bg_stroke = stroke;
        style.visuals.widgets.hovered.fg_stroke.color = Color32::BLACK;
        style.visuals.widgets.hovered.rounding = egui::Rounding::ZERO;
        style.visuals.widgets.hovered.expansion = 0.0;
        style.visuals.widgets.active.bg_fill = palette.fg;
        style.visuals.widgets.active.weak_bg_fill = palette.fg;
        style.visuals.widgets.active.bg_stroke = stroke;
        style.visuals.widgets.active.fg_stroke.color = Color32::BLACK;
        style.visuals.widgets.active.rounding = egui::Rounding::ZERO;
        style.visuals.widgets.active.expansion = 0.0;
        style.visuals.widgets.open.bg_fill = palette.fg;
        style.visuals.widgets.open.weak_bg_fill = palette.fg;
        style.visuals.widgets.open.bg_stroke = stroke;
        style.visuals.widgets.open.fg_stroke.color = Color32::BLACK;
        style.visuals.widgets.open.rounding = egui::Rounding::ZERO;
        style.visuals.widgets.open.expansion = 0.0;
        ui.set_style(style);
    }

    pub(super) fn start_menu_row(
        ui: &mut egui::Ui,
        label: &str,
        selected: bool,
        width: f32,
    ) -> egui::Response {
        let palette = current_palette();
        let (rect, response) =
            ui.allocate_exact_size(egui::vec2(width, 26.0), egui::Sense::click());
        let active = selected || response.hovered();
        let fill = if active { palette.fg } else { palette.panel };
        let text_color = if active { Color32::BLACK } else { palette.fg };
        ui.painter().rect_filled(rect, 0.0, fill);
        ui.painter().text(
            egui::pos2(rect.left() + 8.0, rect.center().y),
            Align2::LEFT_CENTER,
            label,
            FontId::new(20.0, FontFamily::Monospace),
            text_color,
        );
        response
    }
}

#[cfg(test)]
mod tests {
    use super::{start_system_items_for_profile, StartSystemAction};
    use crate::platform::InstallProfile;

    #[test]
    fn mac_launcher_hides_connections_from_start_system_items() {
        let items = start_system_items_for_profile(InstallProfile::MacLauncher);

        assert!(!items
            .iter()
            .any(|(_, action)| matches!(action, StartSystemAction::Connections)));
    }

    #[test]
    fn linux_desktop_keeps_connections_in_start_system_items() {
        let items = start_system_items_for_profile(InstallProfile::LinuxDesktop);

        assert!(items
            .iter()
            .any(|(_, action)| matches!(action, StartSystemAction::Connections)));
    }
}
