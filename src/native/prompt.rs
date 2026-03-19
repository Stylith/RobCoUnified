pub use super::shared_types::FlashAction;
use crate::config::ConnectionKind;
use crate::connections::NetworkMenuGroup;
use crate::default_apps::DefaultAppSlot;
use robcos_native_edit_menus_app::EditMenuTarget;
use robcos_native_installer_app::{InstallerMenuTarget, InstallerPackageAction};
use std::path::PathBuf;
use std::time::Instant;

#[derive(Debug, Clone)]
pub struct TerminalFlash {
    pub message: String,
    pub until: Instant,
    pub action: FlashAction,
    pub boxed: bool,
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
    FileManagerRename {
        path: PathBuf,
    },
    FileManagerMoveTo {
        path: PathBuf,
    },
    FileManagerOpenWithNewCommand {
        path: PathBuf,
        ext_key: String,
        make_default: bool,
    },
    FileManagerOpenWithEditCommand {
        path: PathBuf,
        ext_key: String,
        previous: String,
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
