use super::{NativeSettingsPanel, NucleonNativeApp};
use eframe::egui::{self, Color32, Context};

const STANDALONE_SETTINGS_DEFAULT_SIZE: [f32; 2] = [760.0, 500.0];

pub struct NucleonNativeSettingsApp {
    inner: NucleonNativeApp,
}

impl NucleonNativeSettingsApp {
    pub fn new(session_username: Option<String>, panel: Option<NativeSettingsPanel>) -> Self {
        let mut inner = NucleonNativeApp::default();
        inner.prepare_standalone_settings_window(session_username, panel);
        Self { inner }
    }

    pub fn default_window_size() -> [f32; 2] {
        STANDALONE_SETTINGS_DEFAULT_SIZE
    }

    pub fn min_window_size() -> [f32; 2] {
        STANDALONE_SETTINGS_DEFAULT_SIZE
    }
}

impl eframe::App for NucleonNativeSettingsApp {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        Color32::from_rgb(0, 0, 0).to_normalized_gamma_f32()
    }

    fn save(&mut self, _storage: &mut dyn eframe::Storage) {}

    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        self.inner.update_standalone_settings_window(ctx);
    }
}

pub fn standalone_settings_panel_arg(panel: NativeSettingsPanel) -> &'static str {
    match panel {
        NativeSettingsPanel::Home => "home",
        NativeSettingsPanel::General => "general",
        NativeSettingsPanel::Appearance => "appearance",
        NativeSettingsPanel::DefaultApps => "default-apps",
        NativeSettingsPanel::Connections => "connections",
        NativeSettingsPanel::ConnectionsNetwork => "connections-network",
        NativeSettingsPanel::ConnectionsBluetooth => "connections-bluetooth",
        NativeSettingsPanel::CliProfiles => "cli-profiles",
        NativeSettingsPanel::EditMenus => "edit-menus",
        NativeSettingsPanel::UserManagement => "user-management",
        NativeSettingsPanel::UserManagementViewUsers => "user-management-view-users",
        NativeSettingsPanel::UserManagementCreateUser => "user-management-create-user",
        NativeSettingsPanel::UserManagementEditUsers => "user-management-edit-users",
        NativeSettingsPanel::UserManagementEditCurrentUser => "user-management-edit-current-user",
        NativeSettingsPanel::About => "about",
    }
}

pub fn standalone_settings_panel_from_arg(arg: &str) -> Option<NativeSettingsPanel> {
    match arg.trim().to_ascii_lowercase().as_str() {
        "home" => Some(NativeSettingsPanel::Home),
        "general" => Some(NativeSettingsPanel::General),
        "appearance" => Some(NativeSettingsPanel::Appearance),
        "default-apps" | "default_apps" | "defaultapps" => Some(NativeSettingsPanel::DefaultApps),
        "connections" => Some(NativeSettingsPanel::Connections),
        "connections-network" | "connections_network" | "network" => {
            Some(NativeSettingsPanel::ConnectionsNetwork)
        }
        "connections-bluetooth" | "connections_bluetooth" | "bluetooth" => {
            Some(NativeSettingsPanel::ConnectionsBluetooth)
        }
        "cli-profiles" | "cli_profiles" | "profiles" => Some(NativeSettingsPanel::CliProfiles),
        "edit-menus" | "edit_menus" => Some(NativeSettingsPanel::EditMenus),
        "user-management" | "user_management" => Some(NativeSettingsPanel::UserManagement),
        "user-management-view-users" | "user_management_view_users" => {
            Some(NativeSettingsPanel::UserManagementViewUsers)
        }
        "user-management-create-user" | "user_management_create_user" => {
            Some(NativeSettingsPanel::UserManagementCreateUser)
        }
        "user-management-edit-users" | "user_management_edit_users" => {
            Some(NativeSettingsPanel::UserManagementEditUsers)
        }
        "user-management-edit-current-user" | "user_management_edit_current_user" => {
            Some(NativeSettingsPanel::UserManagementEditCurrentUser)
        }
        "about" => Some(NativeSettingsPanel::About),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn standalone_settings_panel_args_round_trip() {
        for panel in [
            NativeSettingsPanel::Home,
            NativeSettingsPanel::General,
            NativeSettingsPanel::Appearance,
            NativeSettingsPanel::DefaultApps,
            NativeSettingsPanel::Connections,
            NativeSettingsPanel::ConnectionsNetwork,
            NativeSettingsPanel::ConnectionsBluetooth,
            NativeSettingsPanel::CliProfiles,
            NativeSettingsPanel::EditMenus,
            NativeSettingsPanel::UserManagement,
            NativeSettingsPanel::UserManagementViewUsers,
            NativeSettingsPanel::UserManagementCreateUser,
            NativeSettingsPanel::UserManagementEditUsers,
            NativeSettingsPanel::UserManagementEditCurrentUser,
            NativeSettingsPanel::About,
        ] {
            assert_eq!(
                standalone_settings_panel_from_arg(standalone_settings_panel_arg(panel)),
                Some(panel)
            );
        }
    }

    #[test]
    fn standalone_settings_panel_parser_accepts_short_aliases() {
        assert_eq!(
            standalone_settings_panel_from_arg("network"),
            Some(NativeSettingsPanel::ConnectionsNetwork)
        );
        assert_eq!(
            standalone_settings_panel_from_arg("bluetooth"),
            Some(NativeSettingsPanel::ConnectionsBluetooth)
        );
        assert_eq!(
            standalone_settings_panel_from_arg("profiles"),
            Some(NativeSettingsPanel::CliProfiles)
        );
    }
}
