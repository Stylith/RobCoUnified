mod about_screen;
mod addons;
mod addons_standalone;
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
mod hacking_screen;
mod hosted_addon_runtime;
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
mod shell_screen;
mod shell_slots;
mod standalone_launcher;
mod terminal_open_with_picker;
mod terminal_slots;
mod tweaks_standalone;
mod wasm_addon_runtime;

pub use nucleon_native_services::{
    desktop_connections_service, desktop_default_apps_service, desktop_documents_service,
    desktop_file_service, desktop_launcher_service, desktop_search_service,
    desktop_session_service, desktop_settings_service, desktop_shortcuts_service,
    desktop_status_service, desktop_surface_service, desktop_user_service,
    shared_file_manager_settings, shared_types,
};
pub use nucleon_native_settings_app::NativeSettingsPanel;

pub use addons::{
    addon_state_overrides, apply_theme_pack, discovered_addon_manifest_catalog,
    effective_addon_enabled, first_party_addon_manifests, first_party_addon_registry,
    install_repository_addon, install_user_addon, installed_addon_bundle_dir,
    installed_addon_bundle_path, installed_addon_inventory, installed_addon_inventory_sections,
    installed_addon_manifest_registry, installed_color_themes, installed_cursor_packs,
    installed_desktop_styles, installed_enabled_addon_manifest_registry, installed_font_packs,
    installed_hosted_addon_process, installed_hosted_application_names,
    installed_hosted_game_names, installed_icon_packs, installed_sound_packs,
    installed_terminal_themes,
    installed_theme_packs, installed_wasm_addon_module,
    installed_wasm_addon_module_by_display_name, is_installed_hosted_game, remove_installed_addon,
    repository_addon_for_id, repository_sync_action_for_manifest, set_addon_enabled_override,
    InstalledAddonInventorySections, InstalledAddonRecord, InstalledHostedAddonProcess,
    InstalledWasmAddonModule, RepositoryAddonAction, RepositoryAddonRecord,
};
pub(crate) use addons::{
    first_party_addon_disabled_reason, first_party_addon_enabled,
    first_party_addon_registry_for_profile, first_party_addon_runtime,
    FirstPartyAddonDisabledReason, NativeDesktopRoute, NativeTerminalRoute,
};
pub use addons_standalone::NucleonNativeAddonsApp;
pub use app::{apply_native_appearance, configure_native_context, NucleonNativeApp};
pub use applications_standalone::NucleonNativeApplicationsApp;
pub use editor_standalone::NucleonNativeEditorApp;
pub use file_manager_standalone::NucleonNativeFileManagerApp;
pub use installer_standalone::NucleonNativeInstallerApp;
pub use settings_standalone::{
    standalone_settings_panel_arg, standalone_settings_panel_from_arg, NucleonNativeSettingsApp,
};
pub use standalone_launcher::{
    standalone_env_value, NUCLEON_NATIVE_IPC_SOCKET_ENV, NUCLEON_NATIVE_STANDALONE_USER_ENV,
};
pub use tweaks_standalone::NucleonNativeTweaksApp;
