mod about_screen;
pub mod app;
mod connections_screen;
mod data;
mod default_apps_screen;
mod desktop_app;
mod document_browser;
mod donkey_kong;
mod edit_menus_screen;
mod editor_app;
mod file_manager;
mod file_manager_app;
mod file_manager_menu;
mod file_manager_prompt;
mod file_manager_desktop;
mod hacking_screen;
mod installer_screen;
mod menu;
mod nuke_codes_screen;
mod programs_screen;
mod prompt;
mod prompt_flow;
mod pty_screen;
mod retro_ui;
mod settings_screen;
mod shell_actions;
mod shell_screen;
mod user_management;

pub use robcos_native_services::{
    desktop_connections_service, desktop_default_apps_service, desktop_documents_service,
    desktop_file_service, desktop_launcher_service, desktop_search_service,
    desktop_session_service, desktop_settings_service, desktop_shortcuts_service,
    desktop_status_service, desktop_surface_service, desktop_user_service,
    shared_file_manager_settings, shared_types,
};

pub use app::{apply_native_appearance, configure_native_context, RobcoNativeApp};
