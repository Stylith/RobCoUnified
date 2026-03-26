use super::super::connections_screen::apply_search_query as apply_connection_search_query;
use super::super::connections_screen::resolve_terminal_connections_request;
use super::super::desktop_connections_service::{
    connect_connection_and_refresh_settings, connection_requires_password,
    connections_macos_disabled_hint, DiscoveredConnection,
};
use super::super::desktop_default_apps_service::{
    apply_default_app_binding, resolve_custom_default_app_binding,
};
use super::super::desktop_session_service::{
    authenticate_login, bind_login_identity, hacking_start_flash_plan, login_flash_plan,
};
use super::super::desktop_settings_service::reload_settings_snapshot;
use super::super::desktop_status_service::{
    cancelled_shell_status, clear_shell_status, invalid_input_shell_status, shell_status,
};
use super::super::desktop_user_service::{
    delete_user as delete_desktop_user, toggle_user_admin as toggle_desktop_user_admin, user_exists,
};
use super::super::installer_screen::{
    add_package_to_menu, apply_filter as apply_installer_filter,
    apply_search_query as apply_installer_search_query, build_package_command,
};
use super::super::menu::{
    resolve_create_username_prompt, resolve_login_password_submission,
    resolve_terminal_back_action, resolve_user_password_confirm_prompt,
    resolve_user_password_first_prompt, terminal_screen_open_plan, MainMenuSelectionAction,
    TerminalBackAction, TerminalBackContext, TerminalHackingPlan, TerminalLoginScreenMode,
    TerminalLoginSelectionPlan, TerminalLoginSubmitAction, TerminalScreen, TerminalScreenOpenPlan,
    TerminalSelectionIndexTarget, TerminalUserPasswordFlow, UserManagementMode,
};
use super::super::prompt::{FlashAction, TerminalPromptAction};
use super::super::prompt_flow::{handle_prompt_input, PromptOutcome};
use crate::native::install_user_addon;
use crate::config::ConnectionKind;
use crate::core::auth::UserRecord;
use eframe::egui::{Context, Key, Modifiers};
use robcos_native_settings_app::TerminalSettingsPanel;

use super::RobcoNativeApp;

impl RobcoNativeApp {
    pub(super) fn apply_terminal_login_selection_plan(
        &mut self,
        plan: TerminalLoginSelectionPlan<UserRecord>,
    ) {
        self.login.error.clear();
        match plan {
            TerminalLoginSelectionPlan::Exit => {
                crate::sound::play_logout();
                self.queue_terminal_flash("Exiting...", 800, FlashAction::ExitApp);
            }
            TerminalLoginSelectionPlan::PromptPassword { username, prompt } => {
                self.login.selected_username = username;
                self.login.clear_password_and_error();
                self.login.mode = TerminalLoginScreenMode::SelectUser;
                self.open_password_prompt(prompt.title, prompt.prompt);
            }
            TerminalLoginSelectionPlan::Submit {
                action,
                missing_username_is_select_user,
            } => {
                crate::sound::play_navigate();
                self.apply_terminal_login_submit_action(action, missing_username_is_select_user);
            }
            TerminalLoginSelectionPlan::StartHacking { username } => {
                crate::sound::play_navigate();
                self.login.selected_username = username.clone();
                self.login.error.clear();
                self.terminal_prompt = None;
                self.queue_session_flash_plan(hacking_start_flash_plan(username));
            }
            TerminalLoginSelectionPlan::ShowError(error) => {
                crate::sound::play_error();
                self.login.error = error;
            }
        }
    }

    pub(super) fn apply_terminal_login_submit_action(
        &mut self,
        action: TerminalLoginSubmitAction<UserRecord>,
        missing_username_is_select_user: bool,
    ) {
        self.login.error.clear();
        match action {
            TerminalLoginSubmitAction::MissingUsername => {
                crate::sound::play_error();
                self.login.error = if missing_username_is_select_user {
                    "Select a user.".to_string()
                } else {
                    "Username cannot be empty.".to_string()
                };
            }
            TerminalLoginSubmitAction::Authenticated { username, user } => {
                crate::sound::play_login();
                bind_login_identity(&username);
                self.login.selected_username = username.clone();
                self.login.password.clear();
                self.login.error.clear();
                self.terminal_prompt = None;
                self.queue_session_flash_plan(login_flash_plan(username, user));
            }
            TerminalLoginSubmitAction::ShowError(error) => {
                crate::sound::play_error();
                self.login.error = error;
            }
        }
    }

    pub(super) fn apply_terminal_hacking_plan(&mut self, plan: TerminalHackingPlan<UserRecord>) {
        match plan {
            TerminalHackingPlan::ShowUserSelection => {
                self.login.show_user_selection();
            }
            TerminalHackingPlan::ShowLocked => {
                crate::sound::play_navigate();
                self.login.show_locked();
            }
            TerminalHackingPlan::Submit {
                action,
                fallback_to_user_selection_on_error,
            } => {
                let unknown_user = fallback_to_user_selection_on_error
                    && matches!(action, TerminalLoginSubmitAction::ShowError(_));
                self.apply_terminal_login_submit_action(action, false);
                if unknown_user {
                    crate::sound::play_navigate();
                    self.login.show_user_selection();
                }
            }
        }
    }

    pub(super) fn apply_main_menu_selection_action(&mut self, action: MainMenuSelectionAction) {
        match action {
            MainMenuSelectionAction::OpenScreen {
                screen,
                selected_idx,
                clear_status,
            } => match screen {
                TerminalScreen::Applications => self.execute_terminal_launch_target(
                    super::launch_registry::programs_launch_target(),
                    TerminalScreen::MainMenu,
                ),
                TerminalScreen::ProgramInstaller => self.execute_terminal_launch_target(
                    super::launch_registry::installer_launch_target(),
                    TerminalScreen::MainMenu,
                ),
                _ => self.apply_terminal_screen_open_plan(terminal_screen_open_plan(
                    screen,
                    selected_idx,
                    clear_status,
                )),
            },
            MainMenuSelectionAction::OpenTerminalMode => {
                self.execute_terminal_launch_target(
                    super::launch_registry::terminal_launch_target(),
                    TerminalScreen::MainMenu,
                );
            }
            MainMenuSelectionAction::EnterDesktopMode => {
                crate::sound::play_login();
                self.terminate_all_native_pty_children();
                self.close_all_desktop_windows();
                self.desktop_mode_open = true;
                self.close_start_menu();
                self.sync_desktop_active_window();
                self.shell_status = "Entered Desktop Mode.".to_string();
            }
            MainMenuSelectionAction::RefreshSettingsAndOpen => {
                let settings = reload_settings_snapshot();
                self.replace_settings_draft(settings);
                self.execute_terminal_launch_target(
                    super::launch_registry::settings_launch_target(),
                    TerminalScreen::MainMenu,
                );
            }
            MainMenuSelectionAction::BeginLogout => self.begin_logout(),
        }
    }

    pub(super) fn apply_terminal_screen_open_plan(&mut self, plan: TerminalScreenOpenPlan) {
        self.navigate_to_screen(plan.screen);
        if matches!(plan.screen, TerminalScreen::Settings) {
            self.terminal_settings_panel = TerminalSettingsPanel::Home;
        }
        if plan.reset_installer {
            self.terminal_installer.reset();
        }
        if plan.reset_connections {
            self.terminal_connections.reset();
        }
        if plan.clear_settings_choice {
            self.terminal_nav.settings_choice = None;
        }
        if plan.clear_default_app_slot {
            self.terminal_nav.default_app_slot = None;
        }
        if plan.reset_user_management_to_root {
            self.terminal_nav.user_management_mode = UserManagementMode::Root;
        }
        match plan.index_target {
            TerminalSelectionIndexTarget::None => {}
            TerminalSelectionIndexTarget::MainMenu => {
                self.terminal_nav.main_menu_idx = plan.selected_idx
            }
            TerminalSelectionIndexTarget::Applications => {
                self.terminal_nav.apps_idx = plan.selected_idx
            }
            TerminalSelectionIndexTarget::Documents => {
                self.terminal_nav.documents_idx = plan.selected_idx
            }
            TerminalSelectionIndexTarget::Logs => self.terminal_nav.logs_idx = plan.selected_idx,
            TerminalSelectionIndexTarget::Network => {
                self.terminal_nav.network_idx = plan.selected_idx
            }
            TerminalSelectionIndexTarget::Games => self.terminal_nav.games_idx = plan.selected_idx,
            TerminalSelectionIndexTarget::ProgramInstallerRoot => {
                self.terminal_installer.root_idx = plan.selected_idx;
            }
            TerminalSelectionIndexTarget::Settings => {
                self.terminal_nav.settings_idx = plan.selected_idx;
            }
            TerminalSelectionIndexTarget::ConnectionsRoot => {
                self.terminal_connections.root_idx = plan.selected_idx;
            }
            TerminalSelectionIndexTarget::DefaultApps => {
                self.terminal_nav.default_apps_idx = plan.selected_idx;
            }
            TerminalSelectionIndexTarget::UserManagement => {
                self.terminal_nav.user_management_idx = plan.selected_idx;
            }
            TerminalSelectionIndexTarget::DocumentBrowser => {
                self.terminal_nav.browser_idx = plan.selected_idx;
            }
        }
        if plan.clear_status {
            self.apply_status_update(clear_shell_status());
        }
    }

    pub(super) fn handle_terminal_back(&mut self) {
        if matches!(self.terminal_nav.screen, TerminalScreen::Settings)
            && self.terminal_nav.settings_choice.is_none()
            && !matches!(self.terminal_settings_panel, TerminalSettingsPanel::Home)
        {
            crate::sound::play_navigate();
            self.terminal_settings_panel = TerminalSettingsPanel::Home;
            self.terminal_nav.settings_idx = 0;
            self.apply_status_update(clear_shell_status());
            return;
        }
        let action = resolve_terminal_back_action(TerminalBackContext {
            screen: self.terminal_nav.screen,
            has_settings_choice: self.terminal_nav.settings_choice.is_some(),
            has_default_app_slot: self.terminal_nav.default_app_slot.is_some(),
            connections_at_root: self.terminal_connections.is_at_root(),
            installer_at_root: self.terminal_installer.is_at_root(),
            has_embedded_pty: self.primary_embedded_pty_open(),
            pty_return_screen: self
                .primary_embedded_pty_open()
                .then_some(self.terminal_pty.as_ref())
                .flatten()
                .as_ref()
                .map(|pty| pty.return_screen)
                .unwrap_or(TerminalScreen::MainMenu),
            game_return_screen: self.terminal_nav.game_return_screen,
            browser_return_screen: self.terminal_nav.browser_return_screen,
        });
        match action {
            TerminalBackAction::NoOp => {}
            TerminalBackAction::ClearSettingsChoice => {
                crate::sound::play_navigate();
                self.terminal_nav.settings_choice = None;
            }
            TerminalBackAction::ClearDefaultAppSlot => {
                crate::sound::play_navigate();
                self.terminal_nav.default_app_slot = None;
            }
            TerminalBackAction::UseConnectionsInnerBack => {
                crate::sound::play_navigate();
                let _ = self.terminal_connections.back();
                self.apply_status_update(clear_shell_status());
            }
            TerminalBackAction::UseInstallerInnerBack => {
                crate::sound::play_navigate();
                let _ = self.terminal_installer.back();
                self.apply_status_update(clear_shell_status());
            }
            TerminalBackAction::NavigateTo {
                screen,
                clear_status,
                reset_installer,
            } => {
                self.navigate_to_screen(screen);
                if reset_installer {
                    self.terminal_installer.reset();
                }
                if clear_status {
                    self.apply_status_update(clear_shell_status());
                }
            }
            TerminalBackAction::ClosePtyAndReturn { return_screen } => {
                if let Some(mut pty) = self.take_primary_pty() {
                    pty.session.terminate();
                    self.navigate_to_screen(return_screen);
                    self.shell_status = format!("Closed {}.", pty.title);
                } else {
                    self.navigate_to_screen(TerminalScreen::MainMenu);
                    self.apply_status_update(clear_shell_status());
                }
            }
        }
    }

    pub(super) fn handle_terminal_prompt_input(&mut self, ctx: &Context) {
        let Some(prompt) = self.terminal_prompt.clone() else {
            return;
        };
        let prompt_action = prompt.action.clone();
        let outcome = handle_prompt_input(ctx, prompt);
        if self.handle_file_manager_prompt_outcome(&outcome) {
            return;
        }
        match outcome {
            PromptOutcome::Cancel => {
                crate::sound::play_navigate();
                self.terminal_prompt = None;
                if matches!(prompt_action, TerminalPromptAction::LoginPassword) {
                    self.login.password.clear();
                    self.login.error.clear();
                }
            }
            PromptOutcome::Continue(prompt) => {
                self.terminal_prompt = Some(prompt);
            }
            PromptOutcome::LoginPassword(password) => {
                self.terminal_prompt = None;
                self.login.password = password;
                let plan = resolve_login_password_submission(
                    &self.login.selected_username,
                    &self.login.password,
                    self.session.is_some(),
                    self.terminal_flash.is_some(),
                    authenticate_login,
                );
                self.apply_terminal_login_password_plan(plan);
            }
            PromptOutcome::CreateUsername(raw_username) => {
                self.terminal_prompt = None;
                let exists = user_exists(raw_username.trim());
                let plan = resolve_create_username_prompt(&raw_username, exists);
                self.apply_terminal_user_management_prompt_plan(plan);
            }
            PromptOutcome::CreatePasswordFirst { username, password } => {
                self.terminal_prompt = None;
                let plan = resolve_user_password_first_prompt(
                    TerminalUserPasswordFlow::Create,
                    username,
                    password,
                );
                self.apply_terminal_user_management_prompt_plan(plan);
            }
            PromptOutcome::CreatePasswordConfirm {
                username,
                first_password,
                confirmation,
            } => {
                self.terminal_prompt = None;
                let plan = resolve_user_password_confirm_prompt(
                    TerminalUserPasswordFlow::Create,
                    username,
                    first_password,
                    confirmation,
                );
                self.apply_terminal_user_management_prompt_plan(plan);
            }
            PromptOutcome::ResetPasswordFirst { username, password } => {
                self.terminal_prompt = None;
                let plan = resolve_user_password_first_prompt(
                    TerminalUserPasswordFlow::Reset,
                    username,
                    password,
                );
                self.apply_terminal_user_management_prompt_plan(plan);
            }
            PromptOutcome::ResetPasswordConfirm {
                username,
                first_password,
                confirmation,
            } => {
                self.terminal_prompt = None;
                let plan = resolve_user_password_confirm_prompt(
                    TerminalUserPasswordFlow::Reset,
                    username,
                    first_password,
                    confirmation,
                );
                self.apply_terminal_user_management_prompt_plan(plan);
            }
            PromptOutcome::ChangeAuthPasswordFirst { username, password } => {
                self.terminal_prompt = None;
                let plan = resolve_user_password_first_prompt(
                    TerminalUserPasswordFlow::ChangeAuth,
                    username,
                    password,
                );
                self.apply_terminal_user_management_prompt_plan(plan);
            }
            PromptOutcome::ChangeAuthPasswordConfirm {
                username,
                first_password,
                confirmation,
            } => {
                self.terminal_prompt = None;
                let plan = resolve_user_password_confirm_prompt(
                    TerminalUserPasswordFlow::ChangeAuth,
                    username,
                    first_password,
                    confirmation,
                );
                self.apply_terminal_user_management_prompt_plan(plan);
            }
            PromptOutcome::ConfirmDeleteUser {
                username,
                confirmed,
            } => {
                self.terminal_prompt = None;
                if confirmed {
                    self.apply_shell_status_result(delete_desktop_user(&username));
                    self.invalidate_user_cache();
                }
                self.set_user_management_mode(UserManagementMode::Root, 0);
            }
            PromptOutcome::ConfirmToggleAdmin {
                username,
                confirmed,
            } => {
                self.terminal_prompt = None;
                if confirmed {
                    self.apply_shell_status_result(toggle_desktop_user_admin(&username));
                    self.invalidate_user_cache();
                }
                self.set_user_management_mode(UserManagementMode::Root, 0);
            }
            PromptOutcome::EditMenuAddProgramName { target, name } => {
                self.terminal_prompt = None;
                let name = name.trim().to_string();
                if name.is_empty() {
                    self.apply_status_update(invalid_input_shell_status());
                    return;
                }
                self.open_input_prompt(
                    format!("Edit {}", target.title()),
                    format!("Enter launch command for '{name}':"),
                    TerminalPromptAction::EditMenuAddProgramCommand { target, name },
                );
            }
            PromptOutcome::EditMenuAddProgramCommand {
                target,
                name,
                command,
            } => {
                self.terminal_prompt = None;
                self.add_program_entry(target, name, command);
            }
            PromptOutcome::EditMenuAddCategoryName(name) => {
                self.terminal_prompt = None;
                let name = name.trim().to_string();
                if name.is_empty() {
                    self.apply_status_update(invalid_input_shell_status());
                    return;
                }
                self.open_input_prompt(
                    "Edit Documents",
                    "Enter folder path:",
                    TerminalPromptAction::EditMenuAddCategoryPath { name },
                );
            }
            PromptOutcome::EditMenuAddCategoryPath { name, path } => {
                self.terminal_prompt = None;
                if path.trim().is_empty() {
                    self.apply_status_update(invalid_input_shell_status());
                    return;
                }
                self.add_document_category(name, path);
            }
            PromptOutcome::FileManagerRename { .. }
            | PromptOutcome::FileManagerMoveTo { .. }
            | PromptOutcome::FileManagerOpenWithNewCommand { .. }
            | PromptOutcome::FileManagerOpenWithEditCommand { .. } => {
                unreachable!("file manager prompt outcomes are handled before this match")
            }
            PromptOutcome::ConfirmEditMenuDelete {
                target,
                name,
                confirmed,
            } => {
                self.terminal_prompt = None;
                if confirmed {
                    self.delete_program_entry(target, &name);
                } else {
                    self.apply_status_update(cancelled_shell_status());
                }
            }
            PromptOutcome::NewLogName(name) => {
                self.terminal_prompt = None;
                self.create_or_open_log(&name);
            }
            PromptOutcome::EditorSaveAsPath(path) => {
                self.terminal_prompt = None;
                self.save_editor_from_prompt_path(&path);
            }
            PromptOutcome::Noop => {
                self.terminal_prompt = None;
            }
            PromptOutcome::DefaultAppCustom { slot, raw } => {
                self.terminal_prompt = None;
                match resolve_custom_default_app_binding(&raw) {
                    Ok(binding) => {
                        apply_default_app_binding(&mut self.settings.draft, slot, binding);
                        self.persist_native_settings();
                    }
                    Err(status) => {
                        self.shell_status = status;
                    }
                }
            }
            PromptOutcome::InstallerSearch(query) => {
                self.terminal_prompt = None;
                let event = apply_installer_search_query(&mut self.terminal_installer, &query);
                self.apply_installer_event(event);
            }
            PromptOutcome::InstallerFilter(filter) => {
                self.terminal_prompt = None;
                apply_installer_filter(&mut self.terminal_installer, &filter);
            }
            PromptOutcome::InstallerAddonPath(path) => {
                self.terminal_prompt = None;
                let trimmed = path.trim();
                if trimmed.is_empty() {
                    self.apply_status_update(shell_status("Addon path cannot be empty."));
                } else {
                    self.apply_status_update(shell_status(
                        install_user_addon(trimmed).unwrap_or_else(|status| status),
                    ));
                }
            }
            PromptOutcome::InstallerDisplayName {
                pkg,
                target,
                display_name,
            } => {
                self.terminal_prompt = None;
                let event =
                    add_package_to_menu(&mut self.terminal_installer, &pkg, target, &display_name);
                self.invalidate_program_catalog_cache();
                self.apply_installer_event(event);
            }
            PromptOutcome::ConfirmInstallerAction {
                pkg,
                action,
                confirmed,
            } => {
                self.terminal_prompt = None;
                if confirmed {
                    let event = build_package_command(&mut self.terminal_installer, &pkg, action);
                    self.apply_installer_event(event);
                } else {
                    self.apply_status_update(cancelled_shell_status());
                }
            }
            PromptOutcome::ConnectionSearch { kind, group, query } => {
                self.terminal_prompt = None;
                let event = apply_connection_search_query(
                    &mut self.terminal_connections,
                    kind,
                    group,
                    &query,
                );
                let request = resolve_terminal_connections_request(
                    &mut self.terminal_connections,
                    event,
                    connections_macos_disabled_hint(),
                );
                self.apply_terminal_connections_request(request);
            }
            PromptOutcome::ConnectionPassword {
                kind,
                name,
                detail,
                password,
            } => {
                self.terminal_prompt = None;
                if matches!(kind, ConnectionKind::Network)
                    && connection_requires_password(&detail)
                    && password.trim().is_empty()
                {
                    self.apply_status_update(cancelled_shell_status());
                    return;
                }
                let target = DiscoveredConnection { name, detail };
                self.connect_target(
                    kind,
                    target,
                    if password.trim().is_empty() {
                        None
                    } else {
                        Some(password)
                    },
                );
            }
        }
    }

    pub(super) fn consume_terminal_prompt_keys(&self, ctx: &Context) {
        ctx.input_mut(|i| {
            for mods in [Modifiers::NONE, Modifiers::SHIFT] {
                i.consume_key(mods, Key::Enter);
                i.consume_key(mods, Key::Space);
                i.consume_key(mods, Key::Tab);
                i.consume_key(mods, Key::Escape);
                i.consume_key(mods, Key::ArrowUp);
                i.consume_key(mods, Key::ArrowDown);
                i.consume_key(mods, Key::ArrowLeft);
                i.consume_key(mods, Key::ArrowRight);
                i.consume_key(mods, Key::Backspace);
            }
        });
    }

    pub(super) fn connect_target(
        &mut self,
        kind: ConnectionKind,
        target: DiscoveredConnection,
        password: Option<String>,
    ) {
        match connect_connection_and_refresh_settings(kind, &target, password.as_deref()) {
            Ok((settings, status)) => {
                self.replace_settings_draft(settings);
                self.shell_status = status;
            }
            Err(err) => self.shell_status = err.to_string(),
        }
    }
}
