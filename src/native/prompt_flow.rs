use super::edit_menus_screen::EditMenuTarget;
use super::prompt::{TerminalPrompt, TerminalPromptAction, TerminalPromptKind};
use crate::config::ConnectionKind;
use crate::connections::NetworkMenuGroup;
use crate::default_apps::DefaultAppSlot;
use crate::native::installer_screen::{InstallerMenuTarget, InstallerPackageAction};
use eframe::egui::{self, Context, Key};
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub enum PromptOutcome {
    Cancel,
    Continue(TerminalPrompt),
    LoginPassword(String),
    CreateUsername(String),
    CreatePasswordFirst {
        username: String,
        password: String,
    },
    CreatePasswordConfirm {
        username: String,
        first_password: String,
        confirmation: String,
    },
    ResetPasswordFirst {
        username: String,
        password: String,
    },
    ResetPasswordConfirm {
        username: String,
        first_password: String,
        confirmation: String,
    },
    ChangeAuthPasswordFirst {
        username: String,
        password: String,
    },
    ChangeAuthPasswordConfirm {
        username: String,
        first_password: String,
        confirmation: String,
    },
    ConfirmDeleteUser {
        username: String,
        confirmed: bool,
    },
    ConfirmToggleAdmin {
        username: String,
        confirmed: bool,
    },
    DefaultAppCustom {
        slot: DefaultAppSlot,
        raw: String,
    },
    ConnectionSearch {
        kind: ConnectionKind,
        group: Option<NetworkMenuGroup>,
        query: String,
    },
    ConnectionPassword {
        kind: ConnectionKind,
        name: String,
        detail: String,
        password: String,
    },
    InstallerSearch(String),
    InstallerFilter(String),
    InstallerAddonPath(String),
    InstallerDisplayName {
        pkg: String,
        target: InstallerMenuTarget,
        display_name: String,
    },
    ConfirmInstallerAction {
        pkg: String,
        action: InstallerPackageAction,
        confirmed: bool,
    },
    EditMenuAddProgramName {
        target: EditMenuTarget,
        name: String,
    },
    EditMenuAddProgramCommand {
        target: EditMenuTarget,
        name: String,
        command: String,
    },
    EditMenuAddCategoryName(String),
    EditMenuAddCategoryPath {
        name: String,
        path: String,
    },
    FileManagerRename {
        path: PathBuf,
        name: String,
    },
    FileManagerMoveTo {
        path: PathBuf,
        destination: String,
    },
    FileManagerOpenWithNewCommand {
        path: PathBuf,
        ext_key: String,
        make_default: bool,
        command: String,
    },
    FileManagerOpenWithEditCommand {
        path: PathBuf,
        ext_key: String,
        previous: String,
        command: String,
    },
    ConfirmEditMenuDelete {
        target: EditMenuTarget,
        name: String,
        confirmed: bool,
    },
    EditorSaveAsPath(String),
    NewLogName(String),
    Noop,
}

pub fn handle_prompt_input(ctx: &Context, mut prompt: TerminalPrompt) -> PromptOutcome {
    match prompt.kind {
        TerminalPromptKind::Input | TerminalPromptKind::Password => {
            let password_prompt = matches!(prompt.kind, TerminalPromptKind::Password);
            if ctx.input(|i| i.key_pressed(Key::Escape) || i.key_pressed(Key::Tab)) {
                return PromptOutcome::Cancel;
            }
            if ctx.input(|i| i.key_pressed(Key::Backspace)) {
                if prompt.buffer.pop().is_some() && password_prompt {
                    crate::sound::play_keypress();
                }
            }
            let events = ctx.input(|i| i.events.clone());
            for event in events {
                if let egui::Event::Text(text) = event {
                    let mut pushed = false;
                    for ch in text.chars() {
                        if !ch.is_control() {
                            prompt.buffer.push(ch);
                            pushed = true;
                        }
                    }
                    if pushed && password_prompt {
                        crate::sound::play_keypress();
                    }
                }
            }
            if ctx.input(|i| i.key_pressed(Key::Enter)) {
                return match prompt.action {
                    TerminalPromptAction::LoginPassword => {
                        PromptOutcome::LoginPassword(prompt.buffer)
                    }
                    TerminalPromptAction::CreateUsername => {
                        PromptOutcome::CreateUsername(prompt.buffer)
                    }
                    TerminalPromptAction::CreatePassword { username } => {
                        PromptOutcome::CreatePasswordFirst {
                            username,
                            password: prompt.buffer,
                        }
                    }
                    TerminalPromptAction::CreatePasswordConfirm {
                        username,
                        first_password,
                    } => PromptOutcome::CreatePasswordConfirm {
                        username,
                        first_password,
                        confirmation: prompt.buffer,
                    },
                    TerminalPromptAction::ResetPassword { username } => {
                        PromptOutcome::ResetPasswordFirst {
                            username,
                            password: prompt.buffer,
                        }
                    }
                    TerminalPromptAction::ResetPasswordConfirm {
                        username,
                        first_password,
                    } => PromptOutcome::ResetPasswordConfirm {
                        username,
                        first_password,
                        confirmation: prompt.buffer,
                    },
                    TerminalPromptAction::ChangeAuthPassword { username } => {
                        PromptOutcome::ChangeAuthPasswordFirst {
                            username,
                            password: prompt.buffer,
                        }
                    }
                    TerminalPromptAction::ChangeAuthPasswordConfirm {
                        username,
                        first_password,
                    } => PromptOutcome::ChangeAuthPasswordConfirm {
                        username,
                        first_password,
                        confirmation: prompt.buffer,
                    },
                    TerminalPromptAction::DefaultAppCustom { slot } => {
                        PromptOutcome::DefaultAppCustom {
                            slot,
                            raw: prompt.buffer,
                        }
                    }
                    TerminalPromptAction::ConnectionSearch { kind, group } => {
                        PromptOutcome::ConnectionSearch {
                            kind,
                            group,
                            query: prompt.buffer,
                        }
                    }
                    TerminalPromptAction::ConnectionPassword { kind, name, detail } => {
                        PromptOutcome::ConnectionPassword {
                            kind,
                            name,
                            detail,
                            password: prompt.buffer,
                        }
                    }
                    TerminalPromptAction::InstallerSearch => {
                        PromptOutcome::InstallerSearch(prompt.buffer)
                    }
                    TerminalPromptAction::InstallerFilter => {
                        PromptOutcome::InstallerFilter(prompt.buffer)
                    }
                    TerminalPromptAction::InstallerAddonPath => {
                        PromptOutcome::InstallerAddonPath(prompt.buffer)
                    }
                    TerminalPromptAction::InstallerDisplayName { pkg, target } => {
                        PromptOutcome::InstallerDisplayName {
                            pkg,
                            target,
                            display_name: prompt.buffer,
                        }
                    }
                    TerminalPromptAction::EditMenuAddProgramName { target } => {
                        PromptOutcome::EditMenuAddProgramName {
                            target,
                            name: prompt.buffer,
                        }
                    }
                    TerminalPromptAction::EditMenuAddProgramCommand { target, name } => {
                        PromptOutcome::EditMenuAddProgramCommand {
                            target,
                            name,
                            command: prompt.buffer,
                        }
                    }
                    TerminalPromptAction::EditMenuAddCategoryName => {
                        PromptOutcome::EditMenuAddCategoryName(prompt.buffer)
                    }
                    TerminalPromptAction::EditMenuAddCategoryPath { name } => {
                        PromptOutcome::EditMenuAddCategoryPath {
                            name,
                            path: prompt.buffer,
                        }
                    }
                    TerminalPromptAction::FileManagerRename { path } => {
                        PromptOutcome::FileManagerRename {
                            path,
                            name: prompt.buffer,
                        }
                    }
                    TerminalPromptAction::FileManagerMoveTo { path } => {
                        PromptOutcome::FileManagerMoveTo {
                            path,
                            destination: prompt.buffer,
                        }
                    }
                    TerminalPromptAction::FileManagerOpenWithNewCommand {
                        path,
                        ext_key,
                        make_default,
                    } => PromptOutcome::FileManagerOpenWithNewCommand {
                        path,
                        ext_key,
                        make_default,
                        command: prompt.buffer,
                    },
                    TerminalPromptAction::FileManagerOpenWithEditCommand {
                        path,
                        ext_key,
                        previous,
                    } => PromptOutcome::FileManagerOpenWithEditCommand {
                        path,
                        ext_key,
                        previous,
                        command: prompt.buffer,
                    },
                    TerminalPromptAction::EditorSaveAsPath => {
                        PromptOutcome::EditorSaveAsPath(prompt.buffer)
                    }
                    TerminalPromptAction::NewLogName => PromptOutcome::NewLogName(prompt.buffer),
                    TerminalPromptAction::Noop => PromptOutcome::Noop,
                    TerminalPromptAction::ConfirmDeleteUser { .. }
                    | TerminalPromptAction::ConfirmToggleAdmin { .. }
                    | TerminalPromptAction::ConfirmInstallerAction { .. }
                    | TerminalPromptAction::ConfirmEditMenuDelete { .. } => PromptOutcome::Cancel,
                };
            }
            PromptOutcome::Continue(prompt)
        }
        TerminalPromptKind::Confirm => {
            if ctx.input(|i| i.key_pressed(Key::Escape) || i.key_pressed(Key::Tab)) {
                return PromptOutcome::Cancel;
            }
            if ctx.input(|i| i.key_pressed(Key::ArrowLeft)) {
                prompt.confirm_yes = true;
            }
            if ctx.input(|i| i.key_pressed(Key::ArrowRight)) {
                prompt.confirm_yes = false;
            }
            if ctx.input(|i| i.key_pressed(Key::Enter)) {
                return match prompt.action {
                    TerminalPromptAction::ConfirmDeleteUser { username } => {
                        PromptOutcome::ConfirmDeleteUser {
                            username,
                            confirmed: prompt.confirm_yes,
                        }
                    }
                    TerminalPromptAction::ConfirmToggleAdmin { username } => {
                        PromptOutcome::ConfirmToggleAdmin {
                            username,
                            confirmed: prompt.confirm_yes,
                        }
                    }
                    TerminalPromptAction::ConfirmInstallerAction { pkg, action } => {
                        PromptOutcome::ConfirmInstallerAction {
                            pkg,
                            action,
                            confirmed: prompt.confirm_yes,
                        }
                    }
                    TerminalPromptAction::ConfirmEditMenuDelete { target, name } => {
                        PromptOutcome::ConfirmEditMenuDelete {
                            target,
                            name,
                            confirmed: prompt.confirm_yes,
                        }
                    }
                    _ => PromptOutcome::Noop,
                };
            }
            PromptOutcome::Continue(prompt)
        }
    }
}
