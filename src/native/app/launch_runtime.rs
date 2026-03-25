use super::super::data::home_dir_fallback;
use super::super::desktop_app::{DesktopLaunchPayload, DesktopShellAction, DesktopWindow};
use super::super::desktop_file_service::{load_text_document, reveal_path_location};
use super::super::desktop_launcher_service::{resolve_catalog_launch, ProgramCatalog};
use super::super::editor_app::EditorWindow;
use super::super::file_manager::NativeFileManagerState;
use super::super::file_manager_app::FileManagerEditRuntime;
use super::super::menu::{
    terminal_screen_open_plan, terminal_settings_refresh_plan, TerminalScreen,
};
use super::super::NativeSettingsPanel;
use super::launch_registry::{
    self, resolve_desktop_launch_target, resolve_terminal_launch_target,
    unresolved_launch_target_status, unresolved_terminal_launch_target_status, NativeDesktopLaunch,
    NativeTerminalLaunch,
};
use super::{RobcoNativeApp, SecondaryWindowApp, BUILTIN_TEXT_EDITOR_APP};
use crate::platform::LaunchTarget;
use robcos_native_programs_app::{resolve_desktop_games_request, DesktopProgramRequest};
use std::path::{Path, PathBuf};

impl RobcoNativeApp {
    /// Open a desktop window if not already open, otherwise spawn a secondary
    /// embedded instance inside the shell.
    pub(super) fn open_or_spawn_desktop_window(&mut self, window: DesktopWindow) {
        if !self.desktop_window_is_open(window) {
            self.open_desktop_window(window);
            return;
        }
        // Already open — try to create a secondary embedded instance.
        let secondary_app = match window {
            DesktopWindow::FileManager => Some(SecondaryWindowApp::FileManager {
                state: NativeFileManagerState::new(home_dir_fallback()),
                runtime: FileManagerEditRuntime::default(),
            }),
            DesktopWindow::Editor => Some(SecondaryWindowApp::Editor(EditorWindow::default())),
            // Window types that don't support multi-instance: just focus existing.
            _ => None,
        };
        if let Some(app) = secondary_app {
            self.spawn_secondary_window(window, app);
        } else {
            self.open_desktop_window(window);
        }
    }

    pub(super) fn open_desktop_settings_window(&mut self) {
        self.pending_settings_panel = None;
        self.open_desktop_window(DesktopWindow::Settings);
    }

    pub(super) fn launch_file_manager_via_registry(&mut self) {
        self.execute_desktop_shell_action(DesktopShellAction::LaunchByTarget(
            launch_registry::file_manager_launch_target(),
        ));
    }

    pub(super) fn launch_editor_via_registry(&mut self) {
        self.execute_desktop_shell_action(DesktopShellAction::LaunchByTarget(
            launch_registry::editor_launch_target(),
        ));
    }

    pub(super) fn launch_nuke_codes_via_registry(&mut self) {
        self.execute_desktop_shell_action(DesktopShellAction::LaunchByTarget(
            launch_registry::nuke_codes_launch_target(),
        ));
    }

    pub(super) fn launch_settings_via_registry(&mut self) {
        self.execute_desktop_shell_action(DesktopShellAction::LaunchByTarget(
            launch_registry::settings_launch_target(),
        ));
    }

    pub(super) fn open_desktop_settings_panel(&mut self, panel: NativeSettingsPanel) {
        self.pending_settings_panel = Some(self.coerce_desktop_settings_panel(panel));
        self.open_desktop_window(DesktopWindow::Settings);
    }

    fn apply_desktop_launch(&mut self, launch: NativeDesktopLaunch) {
        match launch {
            NativeDesktopLaunch::OpenWindow(window) => {
                self.open_or_spawn_desktop_window(window);
            }
            NativeDesktopLaunch::OpenNukeCodes => {
                self.open_desktop_nuke_codes();
            }
            NativeDesktopLaunch::OpenSettingsPanel(Some(panel)) => {
                self.open_desktop_settings_panel(panel);
            }
            NativeDesktopLaunch::OpenSettingsPanel(None) => {
                self.open_desktop_settings_window();
            }
        }
    }

    fn execute_desktop_launch_target(&mut self, target: LaunchTarget) {
        match resolve_desktop_launch_target(&target) {
            Some(launch) => self.apply_desktop_launch(launch),
            None => {
                self.shell_status = unresolved_launch_target_status(&target);
            }
        }
    }

    fn reveal_path_in_file_manager(&mut self, path: std::path::PathBuf) {
        if self.desktop_window_is_open(DesktopWindow::FileManager) {
            let dir = path
                .parent()
                .map(Path::to_path_buf)
                .unwrap_or_else(home_dir_fallback);
            self.spawn_secondary_window(
                DesktopWindow::FileManager,
                SecondaryWindowApp::FileManager {
                    state: NativeFileManagerState::new(dir),
                    runtime: FileManagerEditRuntime::default(),
                },
            );
        } else {
            match reveal_path_location(path) {
                Ok(location) => {
                    self.apply_file_manager_location(location);
                    self.open_desktop_window(DesktopWindow::FileManager);
                }
                Err(status) => self.shell_status = status,
            }
        }
    }

    fn execute_desktop_launch_target_with_payload(
        &mut self,
        target: LaunchTarget,
        payload: DesktopLaunchPayload,
    ) {
        let Some(launch) = resolve_desktop_launch_target(&target) else {
            self.shell_status = unresolved_launch_target_status(&target);
            return;
        };
        match (launch, payload) {
            (
                NativeDesktopLaunch::OpenWindow(DesktopWindow::TerminalMode),
                DesktopLaunchPayload::OpenTerminalShell,
            ) => self.open_desktop_terminal_shell(),
            (
                NativeDesktopLaunch::OpenWindow(DesktopWindow::FileManager),
                DesktopLaunchPayload::OpenPath(path),
            ) => self.open_file_manager_at(path),
            (
                NativeDesktopLaunch::OpenWindow(DesktopWindow::FileManager),
                DesktopLaunchPayload::RevealPath(path),
            ) => self.reveal_path_in_file_manager(path),
            (
                NativeDesktopLaunch::OpenWindow(DesktopWindow::Editor),
                DesktopLaunchPayload::OpenPath(path),
            ) => self.open_path_in_editor(path),
            _ => {
                self.shell_status =
                    "Launch target is wired but does not support the requested payload."
                        .to_string();
            }
        }
    }

    pub(super) fn launch_desktop_terminal_shell_via_registry(&mut self) {
        self.execute_desktop_shell_action(DesktopShellAction::LaunchByTargetWithPayload {
            target: launch_registry::terminal_launch_target(),
            payload: DesktopLaunchPayload::OpenTerminalShell,
        });
    }

    pub(super) fn execute_desktop_shell_action(&mut self, action: DesktopShellAction) {
        match action {
            DesktopShellAction::LaunchByTarget(target) => {
                self.execute_desktop_launch_target(target)
            }
            DesktopShellAction::LaunchByTargetWithPayload { target, payload } => {
                self.execute_desktop_launch_target_with_payload(target, payload);
            }
            DesktopShellAction::LaunchConfiguredApp(name) => {
                self.apply_desktop_program_request(DesktopProgramRequest::LaunchCatalog {
                    name,
                    catalog: ProgramCatalog::Applications,
                    close_window: true,
                });
            }
            DesktopShellAction::OpenFileManagerAt(path) => {
                self.execute_desktop_launch_target_with_payload(
                    launch_registry::file_manager_launch_target(),
                    DesktopLaunchPayload::OpenPath(path),
                );
            }
            DesktopShellAction::LaunchNetworkProgram(name) => {
                self.apply_desktop_program_request(DesktopProgramRequest::LaunchCatalog {
                    name,
                    catalog: ProgramCatalog::Network,
                    close_window: true,
                });
            }
            DesktopShellAction::LaunchGameProgram(name) => {
                if self.open_hosted_robco_fun_game(&name) {
                    return;
                }
                let request = resolve_desktop_games_request(&name);
                self.apply_desktop_program_request(request);
            }
        }
    }

    pub(super) fn execute_terminal_launch_target(
        &mut self,
        target: LaunchTarget,
        return_screen: TerminalScreen,
    ) {
        let Some(launch) = resolve_terminal_launch_target(&target) else {
            crate::sound::play_error();
            self.shell_status = unresolved_terminal_launch_target_status(&target);
            return;
        };
        match launch {
            NativeTerminalLaunch::OpenScreen(TerminalScreen::Settings) => {
                self.apply_terminal_screen_open_plan(terminal_settings_refresh_plan());
            }
            NativeTerminalLaunch::OpenScreen(screen) => {
                if matches!(screen, TerminalScreen::EditMenus) {
                    self.terminal_edit_menus.reset();
                }
                self.apply_terminal_screen_open_plan(terminal_screen_open_plan(screen, 0, true));
            }
            NativeTerminalLaunch::OpenEmbeddedTerminalShell => {
                self.open_embedded_terminal_shell();
            }
            NativeTerminalLaunch::OpenDocumentBrowser => {
                self.terminal_nav.browser_idx = 0;
                self.terminal_nav.browser_return_screen = return_screen;
                self.navigate_to_screen(TerminalScreen::DocumentBrowser);
                self.shell_status = "Opened File Manager.".to_string();
            }
            NativeTerminalLaunch::OpenEditor => {
                self.editor.open = true;
                if self.editor.path.is_none() {
                    self.new_document();
                }
                self.shell_status = format!("Opened {BUILTIN_TEXT_EDITOR_APP}.");
            }
            NativeTerminalLaunch::OpenNukeCodes => {
                self.open_nuke_codes_screen(return_screen);
            }
        }
    }

    pub(super) fn execute_terminal_launch_target_with_path(
        &mut self,
        target: LaunchTarget,
        path: PathBuf,
        return_screen: TerminalScreen,
    ) {
        let Some(launch) = resolve_terminal_launch_target(&target) else {
            crate::sound::play_error();
            self.shell_status = unresolved_terminal_launch_target_status(&target);
            return;
        };
        match launch {
            NativeTerminalLaunch::OpenDocumentBrowser => {
                self.open_embedded_file_manager_at(path);
                self.terminal_nav.browser_idx = 0;
                self.terminal_nav.browser_return_screen = return_screen;
                self.navigate_to_screen(TerminalScreen::DocumentBrowser);
                self.shell_status = "Opened File Manager.".to_string();
            }
            NativeTerminalLaunch::OpenEditor => {
                self.open_embedded_path_in_editor(path);
                self.shell_status = format!("Opened {BUILTIN_TEXT_EDITOR_APP}.");
            }
            _ => {
                self.shell_status =
                    "Launch target is wired but does not support the requested payload."
                        .to_string();
            }
        }
    }

    pub(super) fn open_desktop_catalog_launch(&mut self, name: &str, catalog: ProgramCatalog) {
        match resolve_catalog_launch(name, catalog) {
            Ok(launch) => self.open_desktop_pty(&launch.title, &launch.argv),
            Err(err) => self.shell_status = err,
        }
    }

    pub(super) fn open_embedded_catalog_launch(
        &mut self,
        name: &str,
        catalog: ProgramCatalog,
        return_screen: TerminalScreen,
    ) {
        match resolve_catalog_launch(name, catalog) {
            Ok(launch) => self.open_embedded_pty(&launch.title, &launch.argv, return_screen),
            Err(err) => self.shell_status = err,
        }
    }

    pub(super) fn open_manual_file(&mut self, path: &str, status_label: &str) {
        let manual = std::path::PathBuf::from(path);
        match load_text_document(manual) {
            Ok(document) => {
                self.editor.path = Some(document.path);
                self.editor.text = document.text;
                self.editor.dirty = false;
                self.editor.cancel_close_confirmation();
                self.editor.status = format!("Opened {status_label}.");
                self.open_desktop_window(DesktopWindow::Editor);
            }
            Err(status) => {
                self.shell_status = format!("{status_label} unavailable: {status}");
            }
        }
    }
}
