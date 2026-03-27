mod about_screen;
mod addons;
pub mod app;
mod applications_standalone;
mod background;
mod command_layer;
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
mod hosted_addon_runtime;
mod hacking_screen;
mod installer_screen;
mod installer_standalone;
pub mod ipc;
mod menu;
mod programs_screen;
mod prompt;
mod prompt_flow;
mod pty_screen;
mod retro_ui;
mod settings_screen;
mod settings_standalone;
mod shell_slots;
mod shell_screen;
mod standalone_launcher;
mod terminal_open_with_picker;
mod terminal_slots;
mod tweaks_standalone;
mod wasm_addon_runtime;

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
    first_party_addon_manifests, first_party_addon_registry, install_repository_addon,
    install_user_addon, installed_addon_bundle_dir, installed_addon_bundle_path,
    installed_addon_inventory, installed_hosted_addon_process, installed_hosted_application_names,
    installed_hosted_game_names,
    installed_theme_packs, apply_theme_pack,
    installed_wasm_addon_module, installed_wasm_addon_module_by_display_name,
    installed_addon_inventory_sections,
    installed_addon_manifest_registry, installed_enabled_addon_manifest_registry,
    is_installed_hosted_game, remove_installed_addon, repository_addon_for_id,
    repository_sync_action_for_manifest,
    set_addon_enabled_override, InstalledAddonInventorySections, InstalledAddonRecord,
    InstalledHostedAddonProcess, InstalledWasmAddonModule,
    RepositoryAddonAction, RepositoryAddonRecord,
};
pub(crate) use addons::{
    first_party_addon_disabled_reason, first_party_addon_enabled,
    first_party_addon_registry_for_profile, first_party_addon_runtime,
    FirstPartyAddonDisabledReason, NativeDesktopRoute, NativeTerminalRoute,
};
pub use app::{apply_native_appearance, configure_native_context, RobcoNativeApp};
pub use applications_standalone::RobcoNativeApplicationsApp;
pub use editor_standalone::RobcoNativeEditorApp;
pub use file_manager_standalone::RobcoNativeFileManagerApp;
pub use installer_standalone::RobcoNativeInstallerApp;
pub use settings_standalone::{
    standalone_settings_panel_arg, standalone_settings_panel_from_arg, RobcoNativeSettingsApp,
};
pub use standalone_launcher::{
    LEGACY_ROBCOS_NATIVE_IPC_SOCKET_ENV, LEGACY_ROBCOS_NATIVE_STANDALONE_USER_ENV,
    NUCLEON_NATIVE_IPC_SOCKET_ENV, NUCLEON_NATIVE_STANDALONE_USER_ENV, standalone_env_value,
};
pub use tweaks_standalone::RobcoNativeTweaksApp;
