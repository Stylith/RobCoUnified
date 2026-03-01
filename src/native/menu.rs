use super::retro_ui::{current_palette, RetroScreen};
use crate::config::HEADER_LINES;
use eframe::egui::{self, Context};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsChoiceKind {
    Theme,
    DefaultOpenMode,
}

#[derive(Debug, Clone, Copy)]
pub struct SettingsChoiceOverlay {
    pub kind: SettingsChoiceKind,
    pub selected: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UserManagementMode {
    Root,
    CreateAuthMethod { username: String },
    CreateHackingDifficulty { username: String },
    DeleteUser,
    ResetPassword,
    ChangeAuthSelectUser,
    ChangeAuthChoose { username: String },
    ChangeAuthHackingDifficulty { username: String },
    ToggleAdmin,
}

#[derive(Debug, Clone)]
pub enum LoginMenuRow {
    User(String),
    Separator,
    Exit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalScreen {
    MainMenu,
    Applications,
    Documents,
    Network,
    Games,
    PtyApp,
    ProgramInstaller,
    Logs,
    DocumentBrowser,
    Settings,
    EditMenus,
    Connections,
    DefaultApps,
    About,
    UserManagement,
}

#[derive(Debug, Clone, Copy)]
pub enum MainMenuAction {
    Applications,
    Documents,
    Network,
    Games,
    ProgramInstaller,
    Terminal,
    DesktopMode,
    Settings,
    Logout,
}

#[derive(Debug, Clone, Copy)]
pub struct MainMenuEntry {
    pub label: &'static str,
    pub action: Option<MainMenuAction>,
}

pub const MAIN_MENU_ENTRIES: &[MainMenuEntry] = &[
    MainMenuEntry {
        label: "Applications",
        action: Some(MainMenuAction::Applications),
    },
    MainMenuEntry {
        label: "Documents",
        action: Some(MainMenuAction::Documents),
    },
    MainMenuEntry {
        label: "Network",
        action: Some(MainMenuAction::Network),
    },
    MainMenuEntry {
        label: "Games",
        action: Some(MainMenuAction::Games),
    },
    MainMenuEntry {
        label: "Program Installer",
        action: Some(MainMenuAction::ProgramInstaller),
    },
    MainMenuEntry {
        label: "Terminal",
        action: Some(MainMenuAction::Terminal),
    },
    MainMenuEntry {
        label: "Desktop Mode",
        action: Some(MainMenuAction::DesktopMode),
    },
    MainMenuEntry {
        label: "---",
        action: None,
    },
    MainMenuEntry {
        label: "Settings",
        action: Some(MainMenuAction::Settings),
    },
    MainMenuEntry {
        label: "Logout",
        action: Some(MainMenuAction::Logout),
    },
];

pub fn selectable_menu_count() -> usize {
    MAIN_MENU_ENTRIES
        .iter()
        .filter(|entry| entry.action.is_some())
        .count()
}

pub fn entry_for_selectable_idx(idx: usize) -> MainMenuEntry {
    MAIN_MENU_ENTRIES
        .iter()
        .copied()
        .filter(|entry| entry.action.is_some())
        .nth(idx)
        .unwrap_or(MAIN_MENU_ENTRIES[0])
}

pub fn login_menu_rows_from_users(usernames: Vec<String>) -> Vec<LoginMenuRow> {
    let mut rows: Vec<LoginMenuRow> = usernames.into_iter().map(LoginMenuRow::User).collect();
    rows.push(LoginMenuRow::Separator);
    rows.push(LoginMenuRow::Exit);
    rows
}

#[allow(clippy::too_many_arguments)]
pub fn draw_terminal_menu_screen(
    ctx: &Context,
    title: &str,
    subtitle: Option<&str>,
    items: &[String],
    selected_idx: &mut usize,
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
    shell_status: &str,
) -> Option<usize> {
    let selectable_rows: Vec<usize> = items
        .iter()
        .enumerate()
        .filter_map(|(idx, item)| if item == "---" { None } else { Some(idx) })
        .collect();
    if selectable_rows.is_empty() {
        return None;
    }
    *selected_idx = (*selected_idx).min(selectable_rows.len().saturating_sub(1));
    if ctx.input(|i| i.key_pressed(egui::Key::ArrowUp)) {
        *selected_idx = selected_idx.saturating_sub(1);
    }
    if ctx.input(|i| i.key_pressed(egui::Key::ArrowDown)) {
        *selected_idx = (*selected_idx + 1).min(selectable_rows.len().saturating_sub(1));
    }

    let enter_pressed =
        ctx.input(|i| i.key_pressed(egui::Key::Enter) || i.key_pressed(egui::Key::Space));
    let mut activated = None;
    if enter_pressed {
        activated = selectable_rows.get(*selected_idx).copied();
    }

    egui::CentralPanel::default()
        .frame(
            egui::Frame::none()
                .fill(current_palette().bg)
                .inner_margin(0.0),
        )
        .show(ctx, |ui| {
            let palette = current_palette();
            let (screen, _) = RetroScreen::new(ui, cols, rows);
            let painter = ui.painter_at(screen.rect);
            screen.paint_bg(&painter, palette.bg);
            for (idx, line) in HEADER_LINES.iter().enumerate() {
                screen.centered_text(&painter, header_start_row + idx, line, palette.fg, true);
            }
            screen.separator(&painter, separator_top_row, &palette);
            screen.centered_text(&painter, title_row, title, palette.fg, true);
            screen.separator(&painter, separator_bottom_row, &palette);
            if let Some(sub) = subtitle {
                screen.underlined_text(&painter, content_col, subtitle_row, sub, palette.fg);
            }
            let mut row = menu_start_row;
            for (idx, item) in items.iter().enumerate() {
                if item == "---" {
                    screen.text(&painter, content_col + 4, row, "---", palette.dim);
                    row += 1;
                    continue;
                }
                let selected = selectable_rows.get(*selected_idx).copied() == Some(idx);
                let text = if selected {
                    format!("  > {item}")
                } else {
                    format!("    {item}")
                };
                let response = screen.selectable_row(
                    ui,
                    &painter,
                    &palette,
                    content_col,
                    row,
                    &text,
                    selected,
                );
                if !enter_pressed && activated.is_none() && response.clicked() {
                    if let Some(sel_idx) = selectable_rows.iter().position(|raw| *raw == idx) {
                        *selected_idx = sel_idx;
                    }
                    activated = Some(idx);
                }
                row += 1;
            }
            if !shell_status.is_empty() {
                screen.text(&painter, content_col, status_row, shell_status, palette.dim);
            }
        });

    activated
}
