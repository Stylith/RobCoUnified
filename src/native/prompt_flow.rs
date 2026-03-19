use super::prompt::TerminalPrompt;
use crate::config::ConnectionKind;
use crate::connections::NetworkMenuGroup;
use crate::default_apps::DefaultAppSlot;
use robcos_native_edit_menus_app::EditMenuTarget;
use robcos_native_installer_app::{InstallerMenuTarget, InstallerPackageAction};
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
    NewLogName(String),
    Noop,
}
