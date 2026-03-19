pub mod desktop_wm_widget;
pub mod message;
pub mod retro_iced_theme;
pub mod retro_theme;
pub mod shell;
mod start_menu_model;
pub mod terminal_canvas;
mod builtin_icons;
mod data;
mod desktop_app;
mod editor_app;
mod file_manager;
mod file_manager_app;
mod file_manager_desktop;
mod file_manager_menu;
mod file_manager_prompt;
mod prompt;
mod prompt_flow;

pub use robcos_native_services::{
    desktop_connections_service, desktop_default_apps_service, desktop_documents_service,
    desktop_file_service, desktop_launcher_service, desktop_search_service,
    desktop_session_service, desktop_settings_service, desktop_shortcuts_service,
    desktop_status_service, desktop_surface_service, desktop_user_service,
    shared_file_manager_settings, shared_types,
};
pub use robcos_native_settings_app::NativeSettingsPanel;

pub use builtin_icons::{builtin_icon, BUILTIN_ICON_NAMES, BUILTIN_ICON_SIZES};
