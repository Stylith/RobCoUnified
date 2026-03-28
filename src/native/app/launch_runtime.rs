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
use super::super::wasm_addon_runtime::WasmHostedAddonState;
use super::super::NativeSettingsPanel;
use super::launch_registry::{
    self, resolve_desktop_launch_target, resolve_terminal_launch_target,
    unresolved_launch_target_status, unresolved_terminal_launch_target_status, NativeDesktopLaunch,
    NativeTerminalLaunch,
};
use super::{NucleonNativeApp, SecondaryWindowApp, BUILTIN_TEXT_EDITOR_APP};
use crate::native::installed_wasm_addon_module_by_display_name;
use crate::platform::LaunchTarget;
use crate::platform::{HostedAddonSize, HostedAddonSurface};
use nucleon_native_programs_app::{resolve_desktop_games_request, DesktopProgramRequest};
use std::path::{Path, PathBuf};

impl NucleonNativeApp {
    pub(super) fn release_retained_wasm_addons(&mut self) {
        self.retained_wasm_addons.clear();
    }

    pub(super) fn launch_shell_command_in_desktop_surface(&mut self, title: &str, argv: &[String]) {
        self.open_desktop_pty(title, argv);
    }

    pub(super) fn launch_shell_command_in_embedded_surface(
        &mut self,
        title: &str,
        argv: &[String],
        return_screen: TerminalScreen,
    ) {
        self.open_embedded_pty(title, argv, return_screen);
    }

    pub(super) fn launch_shell_command_on_active_surface(
        &mut self,
        title: &str,
        argv: &[String],
        return_screen: TerminalScreen,
    ) {
        if self.desktop_mode_open {
            self.launch_shell_command_in_desktop_surface(title, argv);
        } else {
            self.launch_shell_command_in_embedded_surface(title, argv, return_screen);
        }
    }

    pub(super) fn clear_terminal_wasm_addon(&mut self) {
        if let Some(state) = self.terminal_wasm_addon.take() {
            self.retained_wasm_addons.push(state);
        }
        self.terminal_wasm_addon_return_screen = None;
        self.terminal_wasm_addon_last_frame_at = None;
    }

    pub(super) fn clear_desktop_wasm_addon(&mut self) {
        if let Some(state) = self.desktop_wasm_addon.take() {
            self.retained_wasm_addons.push(state);
        }
        self.desktop_wasm_addon_last_frame_at = None;
    }

    pub(super) fn launch_embedded_wasm_addon(
        &mut self,
        name: &str,
        return_screen: TerminalScreen,
    ) -> Result<(), String> {
        let module = installed_wasm_addon_module_by_display_name(name)
            .ok_or_else(|| format!("Installed addon '{name}' is not a runnable WASM addon."))?;
        let state = WasmHostedAddonState::spawn(
            &module,
            HostedAddonSurface::Terminal,
            HostedAddonSize {
                width: 640.0,
                height: 480.0,
            },
        )?;
        if let Some(mut pty) = self.take_primary_pty() {
            pty.session.terminate();
        }
        self.clear_terminal_wasm_addon();
        self.terminal_wasm_addon = Some(state);
        self.terminal_wasm_addon_return_screen = Some(return_screen);
        self.terminal_wasm_addon_last_frame_at = None;
        self.navigate_to_screen(TerminalScreen::PtyApp);
        self.shell_status = format!("Opened {name}.");
        Ok(())
    }

    pub(super) fn launch_desktop_wasm_addon(&mut self, name: &str) -> Result<(), String> {
        let module = installed_wasm_addon_module_by_display_name(name)
            .ok_or_else(|| format!("Installed addon '{name}' is not a runnable WASM addon."))?;
        let spawn_secondary_desktop_addon = self.desktop_component_pty_is_open();
        let state = WasmHostedAddonState::spawn(
            &module,
            HostedAddonSurface::Desktop,
            HostedAddonSize {
                width: 960.0,
                height: 600.0,
            },
        )?;
        if spawn_secondary_desktop_addon {
            let id = self.spawn_secondary_window(
                DesktopWindow::PtyApp,
                SecondaryWindowApp::WasmAddon {
                    state: Some(state),
                    last_frame_at: None,
                },
            );
            let window = self.desktop_window_state_mut(id);
            window.maximized = false;
        } else {
            if let Some(mut pty) = self.take_primary_pty() {
                pty.session.terminate();
            }
            self.desktop_wasm_addon = Some(state);
            self.desktop_wasm_addon_last_frame_at = None;
            self.open_desktop_window(DesktopWindow::PtyApp);
        }
        self.shell_status = format!("Opened {name}.");
        Ok(())
    }

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

    pub(super) fn launch_settings_via_registry(&mut self) {
        self.execute_desktop_shell_action(DesktopShellAction::LaunchByTarget(
            launch_registry::settings_launch_target(),
        ));
    }

    pub(super) fn launch_settings_panel_via_registry(&mut self, panel: NativeSettingsPanel) {
        self.execute_desktop_launch_target_with_payload(
            launch_registry::settings_launch_target(),
            DesktopLaunchPayload::OpenSettingsPanel(panel),
        );
    }

    pub(super) fn launch_file_manager_path_via_registry(&mut self, path: PathBuf) {
        self.execute_desktop_launch_target_with_payload(
            launch_registry::file_manager_launch_target(),
            DesktopLaunchPayload::OpenPath(path),
        );
    }

    pub(super) fn reveal_path_in_file_manager_via_registry(&mut self, path: PathBuf) {
        self.execute_desktop_launch_target_with_payload(
            launch_registry::file_manager_launch_target(),
            DesktopLaunchPayload::RevealPath(path),
        );
    }

    pub(super) fn launch_editor_path_via_registry(&mut self, path: PathBuf) {
        self.execute_desktop_launch_target_with_payload(
            launch_registry::editor_launch_target(),
            DesktopLaunchPayload::OpenPath(path),
        );
    }

    pub(super) fn launch_editor_path_on_active_surface(
        &mut self,
        path: PathBuf,
        return_screen: TerminalScreen,
    ) {
        if self.desktop_mode_open {
            self.launch_editor_path_via_registry(path);
        } else {
            self.execute_terminal_launch_target_with_path(
                launch_registry::editor_launch_target(),
                path,
                return_screen,
            );
        }
    }

    pub(super) fn open_desktop_settings_panel(&mut self, panel: NativeSettingsPanel) {
        if panel == NativeSettingsPanel::Appearance {
            self.open_tweaks_from_settings();
            return;
        }
        self.pending_settings_panel = Some(self.coerce_desktop_settings_panel(panel));
        self.open_desktop_window(DesktopWindow::Settings);
    }

    fn apply_desktop_launch(&mut self, launch: NativeDesktopLaunch) {
        match launch {
            NativeDesktopLaunch::OpenWindow(window) => {
                self.open_or_spawn_desktop_window(window);
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
                NativeDesktopLaunch::OpenSettingsPanel(_),
                DesktopLaunchPayload::OpenSettingsPanel(panel),
            ) => self.open_desktop_settings_panel(panel),
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
        self.execute_desktop_launch_target_with_payload(
            launch_registry::terminal_launch_target(),
            DesktopLaunchPayload::OpenTerminalShell,
        );
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
                self.launch_file_manager_path_via_registry(path);
            }
            DesktopShellAction::LaunchNetworkProgram(name) => {
                self.apply_desktop_program_request(DesktopProgramRequest::LaunchCatalog {
                    name,
                    catalog: ProgramCatalog::Network,
                    close_window: true,
                });
            }
            DesktopShellAction::LaunchGameProgram(name) => {
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
        if matches!(
            catalog,
            ProgramCatalog::Applications | ProgramCatalog::Games
        ) && installed_wasm_addon_module_by_display_name(name).is_some()
        {
            if let Err(err) = self.launch_desktop_wasm_addon(name) {
                self.shell_status = err;
            }
            return;
        }
        match resolve_catalog_launch(name, catalog) {
            Ok(launch) => self.launch_shell_command_in_desktop_surface(&launch.title, &launch.argv),
            Err(err) => self.shell_status = err,
        }
    }

    pub(super) fn open_embedded_catalog_launch(
        &mut self,
        name: &str,
        catalog: ProgramCatalog,
        return_screen: TerminalScreen,
    ) {
        if matches!(
            catalog,
            ProgramCatalog::Applications | ProgramCatalog::Games
        ) && installed_wasm_addon_module_by_display_name(name).is_some()
        {
            if let Err(err) = self.launch_embedded_wasm_addon(name, return_screen) {
                self.shell_status = err;
            }
            return;
        }
        match resolve_catalog_launch(name, catalog) {
            Ok(launch) => self.launch_shell_command_in_embedded_surface(
                &launch.title,
                &launch.argv,
                return_screen,
            ),
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
