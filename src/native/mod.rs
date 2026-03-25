mod about_screen;
mod addons;
pub mod app;
mod applications_standalone;
mod background;
mod connections_screen;
mod data;
mod default_apps_screen;
mod desktop_app;
mod document_browser;
mod edit_menus_screen;
mod editor_app;
mod editor_standalone;
mod file_manager;
mod file_manager_app;
mod file_manager_desktop;
mod file_manager_menu;
mod file_manager_prompt;
mod file_manager_standalone;
mod hacking_screen;
mod installer_screen;
mod installer_standalone;
pub mod ipc;
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
mod terminal_command_palette;
mod terminal_open_with_picker;

pub use robcos_native_services::{
    desktop_connections_service, desktop_default_apps_service, desktop_documents_service,
    desktop_file_service, desktop_launcher_service, desktop_search_service,
    desktop_session_service, desktop_settings_service, desktop_shortcuts_service,
    desktop_status_service, desktop_surface_service, desktop_user_service,
    shared_file_manager_settings, shared_types,
};
pub use robcos_native_settings_app::NativeSettingsPanel;

pub use addons::{
    addon_state_overrides, discovered_addon_manifest_catalog, effective_addon_enabled,
    first_party_addon_manifests, first_party_addon_registry, installed_addon_inventory,
    installed_addon_inventory_sections, installed_addon_manifest_registry,
    installed_enabled_addon_manifest_registry, remove_installed_addon, set_addon_enabled_override,
    InstalledAddonInventorySections, InstalledAddonRecord,
};
pub(crate) use addons::{
    first_party_addon_disabled_reason, first_party_addon_enabled,
    first_party_addon_registry_for_profile, first_party_addon_runtime,
    first_party_capability_enabled_str, FirstPartyAddonDisabledReason, NativeDesktopRoute,
    NativeTerminalRoute,
};
pub use app::{apply_native_appearance, configure_native_context, RobcoNativeApp};
pub use applications_standalone::RobcoNativeApplicationsApp;
pub use editor_standalone::RobcoNativeEditorApp;
pub use file_manager_standalone::RobcoNativeFileManagerApp;
pub use installer_standalone::RobcoNativeInstallerApp;
pub use nuke_codes_standalone::RobcoNativeNukeCodesApp;
pub use settings_standalone::{
    standalone_settings_panel_arg, standalone_settings_panel_from_arg, RobcoNativeSettingsApp,
};
pub use standalone_launcher::{ROBCOS_NATIVE_IPC_SOCKET_ENV, ROBCOS_NATIVE_STANDALONE_USER_ENV};
