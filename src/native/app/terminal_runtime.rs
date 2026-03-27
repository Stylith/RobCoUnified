use super::super::desktop_app::{DesktopWindow, WindowInstanceId};
use super::super::desktop_settings_service::{
    pty_force_render_mode as desktop_pty_force_render_mode,
    pty_profile_for_command as desktop_pty_profile_for_command,
};
use super::super::desktop_status_service::shell_status;
use super::super::desktop_user_service::{
    create_user as create_desktop_user, update_user_auth_method,
};
use super::super::installer_screen::DesktopInstallerNotice;
use super::super::menu::{
    terminal_command_launch_plan, terminal_shell_launch_plan, TerminalDesktopPtyExitPlan,
    TerminalEmbeddedPtyExitPlan, TerminalFlashActionPlan, TerminalFlashPtyLaunchPlan,
    TerminalLoginPasswordPlan, TerminalPtyLaunchPlan, TerminalScreen, TerminalShellSurface,
    TerminalUserManagementPromptPlan, TerminalUserPasswordFlow, UserManagementMode,
};
use super::super::prompt::{FlashAction, TerminalFlash, TerminalPromptAction};
use super::super::pty_screen::{spawn_embedded_pty_with_options, NativePtyState};
use super::RobcoNativeApp;
use crate::core::auth::{AuthMethod, UserRecord};
use std::path::Path;
use std::time::{Duration, Instant};

impl RobcoNativeApp {
    pub(super) fn primary_embedded_pty_open(&self) -> bool {
        self.terminal_pty.is_some()
            && self.terminal_pty_surface == Some(TerminalShellSurface::Embedded)
    }

    pub(super) fn primary_desktop_pty_open(&self) -> bool {
        self.terminal_pty.is_some()
            && self.terminal_pty_surface == Some(TerminalShellSurface::Desktop)
    }

    pub(super) fn take_primary_pty(&mut self) -> Option<NativePtyState> {
        self.terminal_pty_surface = None;
        self.terminal_pty.take()
    }

    fn set_primary_pty(&mut self, state: NativePtyState, surface: TerminalShellSurface) {
        self.terminal_pty = Some(state);
        self.terminal_pty_surface = Some(surface);
    }

    pub(super) fn navigate_to_screen(&mut self, screen: TerminalScreen) {
        let previous = self.terminal_nav.screen;
        if previous != screen {
            crate::sound::play_navigate();
        }
        self.terminal_nav.screen = screen;
    }

    pub(super) fn set_user_management_mode(
        &mut self,
        mode: UserManagementMode,
        selected_idx: usize,
    ) {
        let changed = self.terminal_nav.user_management_mode != mode
            || self.terminal_nav.user_management_idx != selected_idx;
        if changed {
            crate::sound::play_navigate();
        }
        self.terminal_nav.user_management_mode = mode;
        self.terminal_nav.user_management_idx = selected_idx;
    }

    pub(super) fn apply_terminal_login_password_plan(
        &mut self,
        plan: TerminalLoginPasswordPlan<UserRecord>,
    ) {
        self.apply_terminal_login_submit_action(plan.action, true);
        if let Some(prompt) = plan.reopen_prompt {
            self.open_password_prompt(prompt.title, prompt.prompt);
        }
    }

    pub(super) fn apply_terminal_user_management_prompt_plan(
        &mut self,
        plan: TerminalUserManagementPromptPlan,
    ) {
        match plan {
            TerminalUserManagementPromptPlan::Status(message) => {
                self.apply_status_update(shell_status(message));
            }
            TerminalUserManagementPromptPlan::SetMode {
                mode,
                selected_idx,
                suppress_next_menu_submit,
            } => {
                self.set_user_management_mode(mode, selected_idx);
                self.terminal_nav.suppress_next_menu_submit = suppress_next_menu_submit;
            }
            TerminalUserManagementPromptPlan::OpenPasswordConfirm {
                flow,
                username,
                first_password,
                prompt,
            } => {
                let action = match flow {
                    TerminalUserPasswordFlow::Create => {
                        TerminalPromptAction::CreatePasswordConfirm {
                            username,
                            first_password,
                        }
                    }
                    TerminalUserPasswordFlow::Reset => TerminalPromptAction::ResetPasswordConfirm {
                        username,
                        first_password,
                    },
                    TerminalUserPasswordFlow::ChangeAuth => {
                        TerminalPromptAction::ChangeAuthPasswordConfirm {
                            username,
                            first_password,
                        }
                    }
                };
                self.open_password_prompt_with_action(prompt.title, prompt.prompt, action);
            }
            TerminalUserManagementPromptPlan::ApplyPassword {
                flow,
                username,
                password,
            } => {
                match flow {
                    TerminalUserPasswordFlow::Create => {
                        self.apply_shell_status_result(create_desktop_user(
                            &username,
                            AuthMethod::Password,
                            Some(&password),
                        ));
                        self.invalidate_user_cache();
                    }
                    TerminalUserPasswordFlow::Reset => {
                        self.apply_shell_status_result(
                            update_user_auth_method(
                                &username,
                                AuthMethod::Password,
                                Some(&password),
                            )
                            .map(|_| "Password updated.".to_string()),
                        );
                        self.invalidate_user_cache();
                    }
                    TerminalUserPasswordFlow::ChangeAuth => {
                        self.apply_shell_status_result(update_user_auth_method(
                            &username,
                            AuthMethod::Password,
                            Some(&password),
                        ));
                        self.invalidate_user_cache();
                    }
                }
                self.set_user_management_mode(UserManagementMode::Root, 0);
            }
        }
    }

    pub(super) fn login_usernames(&self) -> Vec<String> {
        super::super::desktop_session_service::login_usernames()
    }

    pub(super) fn queue_terminal_flash(
        &mut self,
        message: impl Into<String>,
        ms: u64,
        action: FlashAction,
    ) {
        self.terminal_flash = Some(TerminalFlash {
            message: message.into(),
            until: Instant::now() + Duration::from_millis(ms),
            action,
            boxed: false,
        });
    }

    pub(super) fn queue_terminal_flash_boxed(
        &mut self,
        message: impl Into<String>,
        ms: u64,
        action: FlashAction,
    ) {
        self.terminal_flash = Some(TerminalFlash {
            message: message.into(),
            until: Instant::now() + Duration::from_millis(ms),
            action,
            boxed: true,
        });
    }

    pub(super) fn queue_session_flash_plan(
        &mut self,
        plan: super::super::desktop_session_service::NativeSessionFlashPlan,
    ) {
        self.terminal_flash = Some(TerminalFlash {
            message: plan.message,
            until: Instant::now() + Duration::from_millis(plan.duration_ms),
            action: plan.action,
            boxed: plan.boxed,
        });
    }

    fn apply_terminal_pty_launch_plan(
        &mut self,
        plan: TerminalPtyLaunchPlan,
        desktop_window: bool,
    ) {
        let spawn_secondary_desktop_pty = desktop_window && self.desktop_component_pty_is_open();
        if !desktop_window {
            self.clear_terminal_wasm_addon();
        }
        if !spawn_secondary_desktop_pty
            && (plan.replace_existing_pty || self.terminal_pty.is_some())
        {
            if let Some(mut previous) = self.take_primary_pty() {
                previous.session.terminate();
            }
        }
        let profile = desktop_pty_profile_for_command(&plan.argv);
        let pty_cols = profile
            .preferred_w
            .unwrap_or(96)
            .max(profile.min_w)
            .clamp(40, 160);
        let pty_rows = profile
            .preferred_h
            .unwrap_or(32)
            .max(profile.min_h)
            .clamp(10, 60);
        let options = crate::pty::PtyLaunchOptions {
            env: plan.env,
            top_bar: None,
            force_render_mode: plan.force_render_mode,
        };
        match spawn_embedded_pty_with_options(
            &plan.title,
            &plan.argv,
            plan.return_screen,
            pty_cols,
            pty_rows,
            options,
        ) {
            Ok(mut state) => {
                state.desktop_cols_floor = Some(pty_cols);
                state.desktop_rows_floor = Some(pty_rows);
                state.desktop_live_resize = profile.live_resize;
                if plan.use_fixed_terminal_metrics {
                    state.fixed_cell_w = Some(super::super::pty_screen::TERMINAL_MODE_PTY_CELL_W);
                    state.fixed_cell_h = Some(super::super::pty_screen::TERMINAL_MODE_PTY_CELL_H);
                    state.fixed_font_scale = Some(0.94);
                    state.fixed_font_width_divisor = Some(0.44);
                }
                if desktop_window {
                    let window_id = if spawn_secondary_desktop_pty {
                        self.spawn_secondary_window(
                            DesktopWindow::PtyApp,
                            super::SecondaryWindowApp::Pty(Some(state)),
                        )
                    } else {
                        self.set_primary_pty(state, TerminalShellSurface::Desktop);
                        self.open_desktop_window(DesktopWindow::PtyApp);
                        WindowInstanceId::primary(DesktopWindow::PtyApp)
                    };
                    let window = self.desktop_window_state_mut(window_id);
                    window.maximized = profile.open_fullscreen;
                } else {
                    self.set_primary_pty(state, TerminalShellSurface::Embedded);
                    self.navigate_to_screen(TerminalScreen::PtyApp);
                }
                self.shell_status = plan.success_status;
            }
            Err(err) => {
                self.shell_status = err;
            }
        }
    }

    fn apply_terminal_flash_pty_launch_plan(&mut self, plan: TerminalFlashPtyLaunchPlan) {
        self.apply_terminal_pty_launch_plan(plan.launch, false);
        if let Some(state) = self.terminal_pty.as_mut() {
            state.completion_message = plan.completion_message;
        }
        self.shell_status = plan.status;
    }

    pub(super) fn apply_terminal_flash_action_plan(&mut self, plan: TerminalFlashActionPlan) {
        match plan {
            TerminalFlashActionPlan::StartHacking {
                username,
                difficulty,
            } => {
                crate::sound::play_navigate();
                self.login.start_hacking(username, difficulty);
            }
            TerminalFlashActionPlan::LaunchPty(plan) => {
                self.apply_terminal_flash_pty_launch_plan(plan);
            }
        }
    }

    pub(super) fn apply_terminal_embedded_pty_exit_plan(
        &mut self,
        plan: TerminalEmbeddedPtyExitPlan,
    ) {
        self.navigate_to_screen(plan.return_screen);
        if let Some(message) = plan.boxed_flash_message.clone() {
            self.queue_terminal_flash_boxed(message.clone(), 1600, FlashAction::Noop);
            self.shell_status = message;
        } else {
            self.shell_status = plan.status;
        }
    }

    pub(super) fn apply_terminal_desktop_pty_exit_plan(
        &mut self,
        plan: TerminalDesktopPtyExitPlan,
    ) {
        self.shell_status = plan.status.clone();
        if let Some(message) = plan.installer_notice_message {
            self.desktop_installer.status = message.clone();
            self.desktop_installer.notice = Some(DesktopInstallerNotice {
                message,
                success: plan.installer_notice_success,
            });
        }
        if plan.reopen_installer {
            self.open_desktop_window(DesktopWindow::Installer);
        }
    }

    pub(super) fn open_embedded_pty(
        &mut self,
        title: &str,
        cmd: &[String],
        return_screen: TerminalScreen,
    ) {
        let plan = terminal_command_launch_plan(
            TerminalShellSurface::Embedded,
            title,
            cmd,
            return_screen,
            desktop_pty_force_render_mode(cmd),
        );
        self.apply_terminal_pty_launch_plan(plan, false);
    }

    pub(super) fn open_desktop_pty(&mut self, title: &str, cmd: &[String]) {
        let plan = terminal_command_launch_plan(
            TerminalShellSurface::Desktop,
            title,
            cmd,
            TerminalScreen::MainMenu,
            desktop_pty_force_render_mode(cmd),
        );
        self.apply_terminal_pty_launch_plan(plan, true);
    }

    pub(super) fn open_embedded_terminal_shell(&mut self) {
        let requested_shell = std::env::var("SHELL").ok();
        let bash_exists = Path::new("/bin/bash").exists();
        let plan = terminal_shell_launch_plan(
            TerminalShellSurface::Embedded,
            requested_shell.as_deref(),
            bash_exists,
        );
        self.apply_terminal_pty_launch_plan(plan, false);
    }

    pub(super) fn open_desktop_terminal_shell(&mut self) {
        let requested_shell = std::env::var("SHELL").ok();
        let bash_exists = Path::new("/bin/bash").exists();
        let plan = terminal_shell_launch_plan(
            TerminalShellSurface::Desktop,
            requested_shell.as_deref(),
            bash_exists,
        );
        self.apply_terminal_pty_launch_plan(plan, true);
    }
}
