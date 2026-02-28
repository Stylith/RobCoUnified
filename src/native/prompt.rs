use super::edit_menus_screen::EditMenuTarget;
use super::retro_ui::{current_palette, RetroScreen};
use crate::config::ConnectionKind;
use crate::config::HEADER_LINES;
use crate::connections::NetworkMenuGroup;
use crate::core::auth::UserRecord;
use crate::default_apps::DefaultAppSlot;
use crate::native::installer_screen::{InstallerMenuTarget, InstallerPackageAction};
use eframe::egui::{self, Context};
use std::time::Instant;

#[derive(Debug, Clone)]
pub enum FlashAction {
    ExitApp,
    FinishLogout,
    FinishLogin { username: String, user: UserRecord },
    StartHacking { username: String },
}

#[derive(Debug, Clone)]
pub struct TerminalFlash {
    pub message: String,
    pub until: Instant,
    pub action: FlashAction,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalPromptKind {
    Input,
    Password,
    Confirm,
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TerminalPromptAction {
    LoginPassword,
    CreateUsername,
    CreatePassword {
        username: String,
    },
    CreatePasswordConfirm {
        username: String,
        first_password: String,
    },
    ResetPassword {
        username: String,
    },
    ResetPasswordConfirm {
        username: String,
        first_password: String,
    },
    ChangeAuthPassword {
        username: String,
    },
    ChangeAuthPasswordConfirm {
        username: String,
        first_password: String,
    },
    ConfirmDeleteUser {
        username: String,
    },
    ConfirmToggleAdmin {
        username: String,
    },
    DefaultAppCustom {
        slot: DefaultAppSlot,
    },
    ConnectionSearch {
        kind: ConnectionKind,
        group: Option<NetworkMenuGroup>,
    },
    ConnectionPassword {
        kind: ConnectionKind,
        name: String,
        detail: String,
    },
    InstallerSearch,
    InstallerFilter,
    InstallerDisplayName {
        pkg: String,
        target: InstallerMenuTarget,
    },
    ConfirmInstallerAction {
        pkg: String,
        action: InstallerPackageAction,
    },
    EditMenuAddProgramName {
        target: EditMenuTarget,
    },
    EditMenuAddProgramCommand {
        target: EditMenuTarget,
        name: String,
    },
    EditMenuAddCategoryName,
    EditMenuAddCategoryPath {
        name: String,
    },
    ConfirmEditMenuDelete {
        target: EditMenuTarget,
        name: String,
    },
    NewLogName,
    Noop,
}

#[derive(Debug, Clone)]
pub struct TerminalPrompt {
    pub kind: TerminalPromptKind,
    pub title: String,
    pub prompt: String,
    pub buffer: String,
    pub confirm_yes: bool,
    pub action: TerminalPromptAction,
}

pub fn draw_terminal_prompt_overlay(
    ui: &mut egui::Ui,
    screen: &RetroScreen,
    prompt: &TerminalPrompt,
) {
    let palette = current_palette();
    let painter = ui.painter_at(screen.rect);
    screen.boxed_panel(&painter, &palette, 23, 12, 46, 8);
    screen.text(&painter, 26, 13, &prompt.title, palette.fg);
    screen.text(&painter, 26, 15, &prompt.prompt, palette.fg);
    match prompt.kind {
        TerminalPromptKind::Input => {
            let line = format!("> {}_", prompt.buffer);
            screen.text(&painter, 26, 17, &line, palette.fg);
            screen.text(
                &painter,
                26,
                19,
                "Enter apply | Esc/Tab cancel",
                palette.dim,
            );
        }
        TerminalPromptKind::Password => {
            let masked = format!("> {}_", "*".repeat(prompt.buffer.chars().count()));
            screen.text(&painter, 26, 17, &masked, palette.fg);
            screen.text(
                &painter,
                26,
                19,
                "Enter log in | Esc/Tab back | Backspace delete",
                palette.dim,
            );
        }
        TerminalPromptKind::Confirm => {
            let yes = if prompt.confirm_yes { "[Yes]" } else { " Yes " };
            let no = if prompt.confirm_yes { " No " } else { "[No]" };
            screen.text(&painter, 26, 17, &format!("{yes}   {no}"), palette.fg);
            screen.text(
                &painter,
                26,
                19,
                "Left/Right choose | Enter apply",
                palette.dim,
            );
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn draw_terminal_flash(
    ctx: &Context,
    message: &str,
    cols: usize,
    rows: usize,
    header_start_row: usize,
    separator_top_row: usize,
    separator_bottom_row: usize,
    status_row: usize,
    content_col: usize,
) {
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
            screen.separator(&painter, separator_bottom_row, &palette);
            screen.text(&painter, content_col, status_row, message, palette.fg);
        });
}
