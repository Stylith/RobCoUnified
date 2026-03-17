mod about_screen;
pub mod app;
mod connections_screen;
mod data;
mod default_apps_screen;
mod desktop_app;
#[path = "../../crates/native-services/src/desktop_connections_service.rs"]
mod desktop_connections_service;
#[path = "../../crates/native-services/src/desktop_default_apps_service.rs"]
mod desktop_default_apps_service;
#[path = "../../crates/native-services/src/desktop_documents_service.rs"]
mod desktop_documents_service;
#[path = "../../crates/native-services/src/desktop_file_service.rs"]
mod desktop_file_service;
#[path = "../../crates/native-services/src/desktop_launcher_service.rs"]
mod desktop_launcher_service;
#[path = "../../crates/native-services/src/desktop_search_service.rs"]
mod desktop_search_service;
#[path = "../../crates/native-services/src/desktop_session_service.rs"]
mod desktop_session_service;
#[path = "../../crates/native-services/src/desktop_settings_service.rs"]
mod desktop_settings_service;
#[path = "../../crates/native-services/src/desktop_shortcuts_service.rs"]
mod desktop_shortcuts_service;
#[path = "../../crates/native-services/src/desktop_status_service.rs"]
mod desktop_status_service;
#[path = "../../crates/native-services/src/desktop_surface_service.rs"]
mod desktop_surface_service;
#[path = "../../crates/native-services/src/desktop_user_service.rs"]
mod desktop_user_service;
mod document_browser;
mod donkey_kong;
mod edit_menus_screen;
mod editor_app;
mod file_manager;
mod file_manager_app;
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
#[path = "../../crates/native-services/src/shared_file_manager_settings.rs"]
mod shared_file_manager_settings;
#[path = "../../crates/native-services/src/shared_types.rs"]
mod shared_types;
mod shell_actions;
mod shell_screen;
mod user_management;

pub use app::{apply_native_appearance, configure_native_context, RobcoNativeApp};
