mod about_screen;
mod applications_standalone;
pub mod app;
mod connections_screen;
mod data;
mod default_apps_screen;
mod desktop_app;
mod desktop_menu_bar;
mod desktop_spotlight;
mod desktop_start_menu;
mod desktop_surface;
mod desktop_taskbar;
mod desktop_window_mgmt;
mod document_browser;
mod donkey_kong;
mod edit_menus_screen;
mod editor_standalone;
mod editor_app;
mod file_manager;
mod file_manager_app;
mod file_manager_desktop;
mod file_manager_menu;
mod file_manager_prompt;
mod file_manager_standalone;
mod hacking_screen;
mod installer_standalone;
mod installer_screen;
mod menu;
mod nuke_codes_screen;
mod nuke_codes_standalone;
mod programs_screen;
mod prompt;
mod prompt_flow;
mod pty_screen;
mod retro_ui;
mod settings_screen;
mod settings_standalone;
mod shell_screen;
mod standalone_launcher;

pub use robcos_native_services::{
    desktop_connections_service, desktop_default_apps_service, desktop_documents_service,
    desktop_file_service, desktop_launcher_service, desktop_search_service,
    desktop_session_service, desktop_settings_service, desktop_shortcuts_service,
    desktop_status_service, desktop_surface_service, desktop_user_service,
    shared_file_manager_settings, shared_types,
};
pub use robcos_native_settings_app::NativeSettingsPanel;

pub use app::{apply_native_appearance, configure_native_context, RobcoNativeApp};
pub use applications_standalone::RobcoNativeApplicationsApp;
pub use editor_standalone::RobcoNativeEditorApp;
pub use file_manager_standalone::RobcoNativeFileManagerApp;
pub use installer_standalone::RobcoNativeInstallerApp;
pub use nuke_codes_standalone::RobcoNativeNukeCodesApp;
pub use settings_standalone::{
    standalone_settings_panel_arg, standalone_settings_panel_from_arg, RobcoNativeSettingsApp,
};
pub use standalone_launcher::ROBCOS_NATIVE_STANDALONE_USER_ENV;
