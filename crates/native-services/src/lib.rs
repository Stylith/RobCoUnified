pub use robcos::{config, connections, core, default_apps, launcher, session};

#[path = "../../../src/native/shared_file_manager_settings.rs"]
pub mod shared_file_manager_settings;
#[path = "../../../src/native/shared_types.rs"]
pub mod shared_types;

pub mod desktop_connections_service;
pub mod desktop_default_apps_service;
pub mod desktop_documents_service;
pub mod desktop_file_service;
pub mod desktop_launcher_service;
pub mod desktop_search_service;
pub mod desktop_session_service;
pub mod desktop_settings_service;
pub mod desktop_shortcuts_service;
pub mod desktop_status_service;
pub mod desktop_surface_service;
pub mod desktop_user_service;
