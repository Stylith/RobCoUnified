use robcos_native_services::desktop_default_apps_service::{
    custom_command_input_for_slot, DefaultAppSlot,
};
use robcos_native_services::desktop_user_service::sorted_usernames;
use robcos_native_terminal_app::{SettingsChoiceKind, SettingsChoiceOverlay};
use robcos_shared::config::{
    CliAcsMode, DesktopCliProfiles, DesktopPtyProfileSettings, NativeStartupWindowMode, OpenMode,
    Settings, CUSTOM_THEME_NAME, THEMES,
};
use robcos_shared::connections::macos_connections_disabled;
use robcos_shared::core::auth::AuthMethod;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalSettingsEvent {
    None,
    Persist,
    Back,
    OpenPanel(TerminalSettingsPanel),
    OpenConnections,
    OpenEditMenus,
    OpenDefaultApps,
    OpenAbout,
    EnterUserManagement,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalSettingsPanel {
    Home,
    General,
    Appearance,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeSettingsPanel {
    Home,
    General,
    Appearance,
    DefaultApps,
    Connections,
    ConnectionsNetwork,
    ConnectionsBluetooth,
    CliProfiles,
    EditMenus,
    UserManagement,
    UserManagementViewUsers,
    UserManagementCreateUser,
    UserManagementEditUsers,
    UserManagementEditCurrentUser,
    About,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuiCliProfileSlot {
    Default,
    Calcurse,
    SpotifyPlayer,
    Ranger,
    Reddit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsHomeTileAction {
    OpenPanel(NativeSettingsPanel),
    CloseWindow,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SettingsHomeTile {
    pub action: SettingsHomeTileAction,
    pub label: &'static str,
    pub icon: &'static str,
    pub enabled: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SettingsPanelNavItem {
    pub label: &'static str,
    pub panel: NativeSettingsPanel,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DesktopSettingsUiDefaults {
    pub panel: NativeSettingsPanel,
    pub default_app_custom_text_code: String,
    pub default_app_custom_ebook: String,
    pub cli_profile_slot: GuiCliProfileSlot,
    pub user_selected: String,
    pub user_selected_loaded_for: String,
    pub user_create_auth: AuthMethod,
    pub user_edit_auth: AuthMethod,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SettingsRowId {
    OpenGeneral,
    OpenAppearance,
    Sound,
    SystemSoundVolume,
    Bootup,
    NavigationHints,
    Theme,
    CustomThemeRed,
    CustomThemeGreen,
    CustomThemeBlue,
    BorderGlyphs,
    DefaultOpenMode,
    WindowMode,
    Connections,
    EditMenus,
    DefaultApps,
    About,
    UserManagement,
    Back,
}

pub fn settings_panel_title(panel: NativeSettingsPanel) -> &'static str {
    match panel {
        NativeSettingsPanel::Home => "Settings",
        NativeSettingsPanel::General => "General",
        NativeSettingsPanel::Appearance => "Appearance",
        NativeSettingsPanel::DefaultApps => "Default Apps",
        NativeSettingsPanel::Connections => "Connections",
        NativeSettingsPanel::ConnectionsNetwork => "Network",
        NativeSettingsPanel::ConnectionsBluetooth => "Bluetooth",
        NativeSettingsPanel::CliProfiles => "CLI Profiles",
        NativeSettingsPanel::EditMenus => "Edit Menus",
        NativeSettingsPanel::UserManagement => "User Management",
        NativeSettingsPanel::UserManagementViewUsers => "View Users",
        NativeSettingsPanel::UserManagementCreateUser => "Create User",
        NativeSettingsPanel::UserManagementEditUsers => "Edit Users",
        NativeSettingsPanel::UserManagementEditCurrentUser => "Edit Current User",
        NativeSettingsPanel::About => "About",
    }
}

pub fn desktop_settings_default_panel() -> NativeSettingsPanel {
    NativeSettingsPanel::Home
}

pub fn desktop_settings_back_target(panel: NativeSettingsPanel) -> NativeSettingsPanel {
    match panel {
        NativeSettingsPanel::ConnectionsNetwork | NativeSettingsPanel::ConnectionsBluetooth => {
            NativeSettingsPanel::Connections
        }
        NativeSettingsPanel::UserManagementViewUsers
        | NativeSettingsPanel::UserManagementCreateUser
        | NativeSettingsPanel::UserManagementEditUsers
        | NativeSettingsPanel::UserManagementEditCurrentUser => NativeSettingsPanel::UserManagement,
        NativeSettingsPanel::Home => NativeSettingsPanel::Home,
        _ => desktop_settings_default_panel(),
    }
}

pub fn build_desktop_settings_ui_defaults(
    draft: &Settings,
    session_username: Option<&str>,
) -> DesktopSettingsUiDefaults {
    let users = sorted_usernames();
    let user_selected = session_username
        .map(str::to_string)
        .filter(|name| users.iter().any(|user| user == name))
        .or_else(|| users.first().cloned())
        .unwrap_or_default();
    DesktopSettingsUiDefaults {
        panel: desktop_settings_default_panel(),
        default_app_custom_text_code: custom_command_input_for_slot(
            draft,
            DefaultAppSlot::TextCode,
        ),
        default_app_custom_ebook: custom_command_input_for_slot(draft, DefaultAppSlot::Ebook),
        cli_profile_slot: GuiCliProfileSlot::Default,
        user_selected,
        user_selected_loaded_for: String::new(),
        user_create_auth: AuthMethod::Password,
        user_edit_auth: AuthMethod::Password,
    }
}

pub fn desktop_settings_home_rows(is_admin: bool) -> Vec<Vec<SettingsHomeTile>> {
    vec![
        vec![
            SettingsHomeTile {
                action: SettingsHomeTileAction::OpenPanel(NativeSettingsPanel::General),
                label: "General",
                icon: "[*]",
                enabled: true,
            },
            SettingsHomeTile {
                action: SettingsHomeTileAction::OpenPanel(NativeSettingsPanel::Appearance),
                label: "Appearance",
                icon: "[A]",
                enabled: true,
            },
            SettingsHomeTile {
                action: SettingsHomeTileAction::OpenPanel(NativeSettingsPanel::DefaultApps),
                label: "Default Apps",
                icon: "[D]",
                enabled: true,
            },
            SettingsHomeTile {
                action: SettingsHomeTileAction::OpenPanel(NativeSettingsPanel::Connections),
                label: "Connections",
                icon: "[C]",
                enabled: true,
            },
        ],
        vec![
            SettingsHomeTile {
                action: SettingsHomeTileAction::OpenPanel(NativeSettingsPanel::CliProfiles),
                label: "CLI Profiles",
                icon: "[=]",
                enabled: true,
            },
            SettingsHomeTile {
                action: SettingsHomeTileAction::OpenPanel(NativeSettingsPanel::EditMenus),
                label: "Edit Menus",
                icon: "[M]",
                enabled: true,
            },
            SettingsHomeTile {
                action: SettingsHomeTileAction::OpenPanel(NativeSettingsPanel::UserManagement),
                label: "User Management",
                icon: "[U]",
                enabled: is_admin,
            },
            SettingsHomeTile {
                action: SettingsHomeTileAction::OpenPanel(NativeSettingsPanel::About),
                label: "About",
                icon: "[i]",
                enabled: true,
            },
        ],
        vec![SettingsHomeTile {
            action: SettingsHomeTileAction::CloseWindow,
            label: "Close",
            icon: "[X]",
            enabled: true,
        }],
    ]
}

pub fn desktop_settings_connections_nav_items() -> [SettingsPanelNavItem; 2] {
    [
        SettingsPanelNavItem {
            label: "Network",
            panel: NativeSettingsPanel::ConnectionsNetwork,
        },
        SettingsPanelNavItem {
            label: "Bluetooth",
            panel: NativeSettingsPanel::ConnectionsBluetooth,
        },
    ]
}

pub fn desktop_settings_user_management_nav_items() -> [SettingsPanelNavItem; 4] {
    [
        SettingsPanelNavItem {
            label: "View Users",
            panel: NativeSettingsPanel::UserManagementViewUsers,
        },
        SettingsPanelNavItem {
            label: "Create User",
            panel: NativeSettingsPanel::UserManagementCreateUser,
        },
        SettingsPanelNavItem {
            label: "Edit Users",
            panel: NativeSettingsPanel::UserManagementEditUsers,
        },
        SettingsPanelNavItem {
            label: "Edit Current User",
            panel: NativeSettingsPanel::UserManagementEditCurrentUser,
        },
    ]
}

pub fn gui_cli_profile_slot_label(slot: GuiCliProfileSlot) -> &'static str {
    match slot {
        GuiCliProfileSlot::Default => "Default",
        GuiCliProfileSlot::Calcurse => "Calcurse",
        GuiCliProfileSlot::SpotifyPlayer => "Spotify Player",
        GuiCliProfileSlot::Ranger => "Ranger",
        GuiCliProfileSlot::Reddit => "Reddit",
    }
}

pub fn gui_cli_profile_slots() -> [GuiCliProfileSlot; 5] {
    [
        GuiCliProfileSlot::Default,
        GuiCliProfileSlot::Calcurse,
        GuiCliProfileSlot::SpotifyPlayer,
        GuiCliProfileSlot::Ranger,
        GuiCliProfileSlot::Reddit,
    ]
}

pub fn gui_cli_profile_mut(
    profiles: &mut DesktopCliProfiles,
    slot: GuiCliProfileSlot,
) -> &mut DesktopPtyProfileSettings {
    match slot {
        GuiCliProfileSlot::Default => &mut profiles.default,
        GuiCliProfileSlot::Calcurse => &mut profiles.calcurse,
        GuiCliProfileSlot::SpotifyPlayer => &mut profiles.spotify_player,
        GuiCliProfileSlot::Ranger => &mut profiles.ranger,
        GuiCliProfileSlot::Reddit => &mut profiles.reddit,
    }
}

pub fn terminal_settings_rows(draft: &Settings, is_admin: bool) -> Vec<String> {
    terminal_settings_rows_with_ids(TerminalSettingsPanel::Home, draft, is_admin)
        .into_iter()
        .map(|(label, _)| label)
        .collect()
}

pub fn handle_settings_activation(
    panel: TerminalSettingsPanel,
    draft: &mut Settings,
    idx: usize,
    choice_overlay: &mut Option<SettingsChoiceOverlay>,
    is_admin: bool,
) -> TerminalSettingsEvent {
    let rows = terminal_settings_rows_with_ids(panel, draft, is_admin);
    let Some((_, row_id)) = rows.get(idx) else {
        return TerminalSettingsEvent::Back;
    };
    match row_id {
        SettingsRowId::OpenGeneral => {
            TerminalSettingsEvent::OpenPanel(TerminalSettingsPanel::General)
        }
        SettingsRowId::OpenAppearance => {
            TerminalSettingsEvent::OpenPanel(TerminalSettingsPanel::Appearance)
        }
        SettingsRowId::Sound => {
            draft.sound = !draft.sound;
            TerminalSettingsEvent::Persist
        }
        SettingsRowId::SystemSoundVolume => TerminalSettingsEvent::None,
        SettingsRowId::Bootup => {
            draft.bootup = !draft.bootup;
            TerminalSettingsEvent::Persist
        }
        SettingsRowId::NavigationHints => {
            draft.show_navigation_hints = !draft.show_navigation_hints;
            TerminalSettingsEvent::Persist
        }
        SettingsRowId::Theme => {
            *choice_overlay = Some(open_settings_choice(draft, SettingsChoiceKind::Theme));
            TerminalSettingsEvent::None
        }
        SettingsRowId::CustomThemeRed
        | SettingsRowId::CustomThemeGreen
        | SettingsRowId::CustomThemeBlue => TerminalSettingsEvent::None,
        SettingsRowId::BorderGlyphs => {
            draft.cli_acs_mode = match draft.cli_acs_mode {
                CliAcsMode::Ascii => CliAcsMode::Unicode,
                CliAcsMode::Unicode => CliAcsMode::Ascii,
            };
            TerminalSettingsEvent::Persist
        }
        SettingsRowId::DefaultOpenMode => {
            *choice_overlay = Some(open_settings_choice(
                draft,
                SettingsChoiceKind::DefaultOpenMode,
            ));
            TerminalSettingsEvent::None
        }
        SettingsRowId::WindowMode => {
            *choice_overlay = Some(open_settings_choice(draft, SettingsChoiceKind::WindowMode));
            TerminalSettingsEvent::None
        }
        SettingsRowId::Connections => TerminalSettingsEvent::OpenConnections,
        SettingsRowId::EditMenus => TerminalSettingsEvent::OpenEditMenus,
        SettingsRowId::DefaultApps => TerminalSettingsEvent::OpenDefaultApps,
        SettingsRowId::About => TerminalSettingsEvent::OpenAbout,
        SettingsRowId::UserManagement => {
            if is_admin {
                TerminalSettingsEvent::EnterUserManagement
            } else {
                TerminalSettingsEvent::Back
            }
        }
        SettingsRowId::Back => TerminalSettingsEvent::Back,
    }
}

pub fn open_settings_choice(draft: &Settings, kind: SettingsChoiceKind) -> SettingsChoiceOverlay {
    let selected = match kind {
        SettingsChoiceKind::Theme => THEMES
            .iter()
            .position(|(name, _)| *name == draft.theme)
            .unwrap_or(0),
        SettingsChoiceKind::DefaultOpenMode => match draft.default_open_mode {
            OpenMode::Terminal => 0,
            OpenMode::Desktop => 1,
        },
        SettingsChoiceKind::WindowMode => match draft.native_startup_window_mode {
            NativeStartupWindowMode::Windowed => 0,
            NativeStartupWindowMode::Maximized => 1,
            NativeStartupWindowMode::BorderlessFullscreen => 2,
            NativeStartupWindowMode::Fullscreen => 3,
        },
    };
    SettingsChoiceOverlay { kind, selected }
}

pub fn settings_choice_items(kind: SettingsChoiceKind) -> Vec<String> {
    match kind {
        SettingsChoiceKind::Theme => THEMES.iter().map(|(name, _)| (*name).to_string()).collect(),
        SettingsChoiceKind::DefaultOpenMode => vec!["Terminal".to_string(), "Desktop".to_string()],
        SettingsChoiceKind::WindowMode => vec![
            "Windowed".to_string(),
            "Maximized".to_string(),
            "Borderless Fullscreen".to_string(),
            "Fullscreen".to_string(),
        ],
    }
}

pub fn apply_settings_choice(draft: &mut Settings, kind: SettingsChoiceKind, selected: usize) {
    match kind {
        SettingsChoiceKind::Theme => {
            if let Some((name, _)) = THEMES.get(selected) {
                draft.theme = (*name).to_string();
            }
        }
        SettingsChoiceKind::DefaultOpenMode => {
            draft.default_open_mode = if selected == 0 {
                OpenMode::Terminal
            } else {
                OpenMode::Desktop
            };
        }
        SettingsChoiceKind::WindowMode => {
            draft.native_startup_window_mode = match selected {
                1 => NativeStartupWindowMode::Maximized,
                2 => NativeStartupWindowMode::BorderlessFullscreen,
                3 => NativeStartupWindowMode::Fullscreen,
                _ => NativeStartupWindowMode::Windowed,
            };
        }
    }
}

pub fn adjust_settings_slider(
    panel: TerminalSettingsPanel,
    draft: &mut Settings,
    idx: usize,
    is_admin: bool,
    delta: i16,
) -> bool {
    let rows = terminal_settings_rows_with_ids(panel, draft, is_admin);
    let Some((_, row_id)) = rows.get(idx) else {
        return false;
    };
    match row_id {
        SettingsRowId::CustomThemeRed => {
            adjust_rgb_component(&mut draft.custom_theme_rgb[0], delta);
            if draft.theme != CUSTOM_THEME_NAME {
                draft.theme = CUSTOM_THEME_NAME.to_string();
            }
            true
        }
        SettingsRowId::SystemSoundVolume => {
            adjust_percent(&mut draft.system_sound_volume, delta * 5);
            true
        }
        SettingsRowId::CustomThemeGreen => {
            adjust_rgb_component(&mut draft.custom_theme_rgb[1], delta);
            if draft.theme != CUSTOM_THEME_NAME {
                draft.theme = CUSTOM_THEME_NAME.to_string();
            }
            true
        }
        SettingsRowId::CustomThemeBlue => {
            adjust_rgb_component(&mut draft.custom_theme_rgb[2], delta);
            if draft.theme != CUSTOM_THEME_NAME {
                draft.theme = CUSTOM_THEME_NAME.to_string();
            }
            true
        }
        _ => false,
    }
}

fn terminal_settings_rows_with_ids(
    panel: TerminalSettingsPanel,
    draft: &Settings,
    is_admin: bool,
) -> Vec<(String, SettingsRowId)> {
    match panel {
        TerminalSettingsPanel::Home => {
            let mut rows = vec![
                ("General".to_string(), SettingsRowId::OpenGeneral),
                ("Appearance".to_string(), SettingsRowId::OpenAppearance),
                ("Default Apps".to_string(), SettingsRowId::DefaultApps),
            ];
            if !macos_connections_disabled() {
                rows.push(("Connections".to_string(), SettingsRowId::Connections));
            }
            rows.push(("Edit Menus".to_string(), SettingsRowId::EditMenus));
            if is_admin {
                rows.push(("User Management".to_string(), SettingsRowId::UserManagement));
            }
            rows.push(("About".to_string(), SettingsRowId::About));
            rows.push(("Back".to_string(), SettingsRowId::Back));
            rows
        }
        TerminalSettingsPanel::General => vec![
            (
                format!(
                    "Default Open Mode: {} [choose]",
                    match draft.default_open_mode {
                        OpenMode::Terminal => "Terminal",
                        OpenMode::Desktop => "Desktop",
                    }
                ),
                SettingsRowId::DefaultOpenMode,
            ),
            (
                format!("Sound: {} [toggle]", if draft.sound { "ON" } else { "OFF" }),
                SettingsRowId::Sound,
            ),
            (
                format!(
                    "System Sound Volume: {}% [adjust]",
                    draft.system_sound_volume
                ),
                SettingsRowId::SystemSoundVolume,
            ),
            (
                format!(
                    "Bootup: {} [toggle]",
                    if draft.bootup { "ON" } else { "OFF" }
                ),
                SettingsRowId::Bootup,
            ),
            (
                format!(
                    "Navigation Hints: {} [toggle]",
                    if draft.show_navigation_hints {
                        "ON"
                    } else {
                        "OFF"
                    }
                ),
                SettingsRowId::NavigationHints,
            ),
            ("Back".to_string(), SettingsRowId::Back),
        ],
        TerminalSettingsPanel::Appearance => {
            let mut rows = vec![
                (
                    format!(
                        "Window Mode: {} [choose]",
                        draft.native_startup_window_mode.label()
                    ),
                    SettingsRowId::WindowMode,
                ),
                (
                    format!("Theme: {} [choose]", draft.theme),
                    SettingsRowId::Theme,
                ),
            ];
            if draft.theme == CUSTOM_THEME_NAME {
                let [r, g, b] = draft.custom_theme_rgb;
                rows.extend([
                    (
                        format!("Custom Theme Red: {r} [adjust]"),
                        SettingsRowId::CustomThemeRed,
                    ),
                    (
                        format!("Custom Theme Green: {g} [adjust]"),
                        SettingsRowId::CustomThemeGreen,
                    ),
                    (
                        format!("Custom Theme Blue: {b} [adjust]"),
                        SettingsRowId::CustomThemeBlue,
                    ),
                ]);
            }
            rows.push((
                format!(
                    "Border Glyphs: {} [toggle]",
                    match draft.cli_acs_mode {
                        CliAcsMode::Ascii => "ASCII",
                        CliAcsMode::Unicode => "Unicode Smooth",
                    }
                ),
                SettingsRowId::BorderGlyphs,
            ));
            rows.push(("Back".to_string(), SettingsRowId::Back));
            rows
        }
    }
}

fn adjust_rgb_component(value: &mut u8, delta: i16) {
    let next = (*value as i16 + delta).clamp(0, 255);
    *value = next as u8;
}

fn adjust_percent(value: &mut u8, delta: i16) {
    let next = (*value as i16 + delta).clamp(0, 100);
    *value = next as u8;
}

#[cfg(test)]
mod tests {
    use super::*;
    use robcos_shared::config::get_settings;

    #[test]
    fn terminal_settings_rows_include_default_apps_and_about() {
        let draft = get_settings();
        let user_rows = terminal_settings_rows(&draft, false);
        assert!(user_rows.iter().any(|label| label == "General"));
        assert!(user_rows.iter().any(|label| label == "Appearance"));
        assert!(user_rows.iter().any(|label| label == "Edit Menus"));
        assert!(user_rows.iter().any(|label| label == "Default Apps"));
        assert!(user_rows.iter().any(|label| label == "About"));
        assert_eq!(user_rows.last().map(|label| label.as_str()), Some("Back"));

        let admin_rows = terminal_settings_rows(&draft, true);
        assert!(admin_rows.iter().any(|label| label == "User Management"));
        assert_eq!(admin_rows.last().map(|label| label.as_str()), Some("Back"));
    }

    #[test]
    fn handle_settings_activation_routes_new_rows_correctly() {
        let mut draft = get_settings();
        let mut overlay = None;
        let user_rows = terminal_settings_rows_with_ids(TerminalSettingsPanel::Home, &draft, false);
        if let Some(connections_idx) = user_rows
            .iter()
            .position(|(_, id)| *id == SettingsRowId::Connections)
        {
            assert!(matches!(
                handle_settings_activation(
                    TerminalSettingsPanel::Home,
                    &mut draft,
                    connections_idx,
                    &mut overlay,
                    false,
                ),
                TerminalSettingsEvent::OpenConnections
            ));
        }
        let general_idx = user_rows
            .iter()
            .position(|(_, id)| *id == SettingsRowId::OpenGeneral)
            .unwrap();
        let default_apps_idx = user_rows
            .iter()
            .position(|(_, id)| *id == SettingsRowId::DefaultApps)
            .unwrap();
        let edit_menus_idx = user_rows
            .iter()
            .position(|(_, id)| *id == SettingsRowId::EditMenus)
            .unwrap();
        let about_idx = user_rows
            .iter()
            .position(|(_, id)| *id == SettingsRowId::About)
            .unwrap();
        let admin_rows = terminal_settings_rows_with_ids(TerminalSettingsPanel::Home, &draft, true);
        let user_mgmt_idx = admin_rows
            .iter()
            .position(|(_, id)| *id == SettingsRowId::UserManagement)
            .unwrap();

        assert!(matches!(
            handle_settings_activation(
                TerminalSettingsPanel::Home,
                &mut draft,
                general_idx,
                &mut overlay,
                false,
            ),
            TerminalSettingsEvent::OpenPanel(TerminalSettingsPanel::General)
        ));
        assert!(matches!(
            handle_settings_activation(
                TerminalSettingsPanel::Home,
                &mut draft,
                edit_menus_idx,
                &mut overlay,
                false,
            ),
            TerminalSettingsEvent::OpenEditMenus
        ));
        assert!(matches!(
            handle_settings_activation(
                TerminalSettingsPanel::Home,
                &mut draft,
                default_apps_idx,
                &mut overlay,
                false,
            ),
            TerminalSettingsEvent::OpenDefaultApps
        ));
        assert!(matches!(
            handle_settings_activation(
                TerminalSettingsPanel::Home,
                &mut draft,
                about_idx,
                &mut overlay,
                false,
            ),
            TerminalSettingsEvent::OpenAbout
        ));
        assert!(matches!(
            handle_settings_activation(
                TerminalSettingsPanel::Home,
                &mut draft,
                user_mgmt_idx,
                &mut overlay,
                true,
            ),
            TerminalSettingsEvent::EnterUserManagement
        ));
    }

    #[test]
    fn connections_row_respects_platform_capability() {
        let draft = get_settings();
        let rows = terminal_settings_rows_with_ids(TerminalSettingsPanel::Home, &draft, false);
        let has_connections = rows.iter().any(|(_, id)| *id == SettingsRowId::Connections);
        assert_eq!(has_connections, !macos_connections_disabled());
    }

    #[test]
    fn window_mode_row_opens_choice_overlay() {
        let mut draft = get_settings();
        let mut overlay = None;
        let rows =
            terminal_settings_rows_with_ids(TerminalSettingsPanel::Appearance, &draft, false);
        let idx = rows
            .iter()
            .position(|(_, id)| *id == SettingsRowId::WindowMode)
            .expect("window mode row");

        assert!(matches!(
            handle_settings_activation(
                TerminalSettingsPanel::Appearance,
                &mut draft,
                idx,
                &mut overlay,
                false,
            ),
            TerminalSettingsEvent::None
        ));
        assert!(matches!(
            overlay,
            Some(SettingsChoiceOverlay {
                kind: SettingsChoiceKind::WindowMode,
                ..
            })
        ));
    }

    #[test]
    fn window_mode_choice_items_include_borderless_fullscreen() {
        assert_eq!(
            settings_choice_items(SettingsChoiceKind::WindowMode),
            vec![
                "Windowed".to_string(),
                "Maximized".to_string(),
                "Borderless Fullscreen".to_string(),
                "Fullscreen".to_string(),
            ]
        );
    }

    #[test]
    fn border_glyphs_row_toggles_acs_mode() {
        let mut draft = get_settings();
        let mut overlay = None;
        let rows =
            terminal_settings_rows_with_ids(TerminalSettingsPanel::Appearance, &draft, false);
        let idx = rows
            .iter()
            .position(|(_, id)| *id == SettingsRowId::BorderGlyphs)
            .expect("border glyph row");
        let before = draft.cli_acs_mode;
        assert!(matches!(
            handle_settings_activation(
                TerminalSettingsPanel::Appearance,
                &mut draft,
                idx,
                &mut overlay,
                false,
            ),
            TerminalSettingsEvent::Persist
        ));
        assert_ne!(draft.cli_acs_mode, before);
    }

    #[test]
    fn custom_rgb_rows_show_only_for_custom_theme() {
        let mut draft = get_settings();
        draft.theme = "Green (Default)".to_string();
        let base_rows =
            terminal_settings_rows_with_ids(TerminalSettingsPanel::Appearance, &draft, false);
        assert!(!base_rows
            .iter()
            .any(|(_, id)| matches!(id, SettingsRowId::CustomThemeRed)));
        assert!(!base_rows
            .iter()
            .any(|(_, id)| matches!(id, SettingsRowId::CustomThemeGreen)));
        assert!(!base_rows
            .iter()
            .any(|(_, id)| matches!(id, SettingsRowId::CustomThemeBlue)));

        draft.theme = CUSTOM_THEME_NAME.to_string();
        let custom_rows =
            terminal_settings_rows_with_ids(TerminalSettingsPanel::Appearance, &draft, false);
        assert!(custom_rows
            .iter()
            .any(|(_, id)| matches!(id, SettingsRowId::CustomThemeRed)));
        assert!(custom_rows
            .iter()
            .any(|(_, id)| matches!(id, SettingsRowId::CustomThemeGreen)));
        assert!(custom_rows
            .iter()
            .any(|(_, id)| matches!(id, SettingsRowId::CustomThemeBlue)));
    }

    #[test]
    fn custom_rgb_row_adjusts_value_and_keeps_custom_theme() {
        let mut draft = get_settings();
        draft.theme = CUSTOM_THEME_NAME.to_string();
        draft.custom_theme_rgb = [10, 20, 30];
        let rows =
            terminal_settings_rows_with_ids(TerminalSettingsPanel::Appearance, &draft, false);
        let red_idx = rows
            .iter()
            .position(|(_, id)| matches!(id, SettingsRowId::CustomThemeRed))
            .expect("red row");
        assert!(adjust_settings_slider(
            TerminalSettingsPanel::Appearance,
            &mut draft,
            red_idx,
            false,
            5,
        ));
        assert_eq!(draft.custom_theme_rgb[0], 15);
        assert_eq!(draft.theme, CUSTOM_THEME_NAME);
    }

    #[test]
    fn desktop_settings_home_rows_disable_user_management_for_non_admin() {
        let rows = desktop_settings_home_rows(false);
        let tile = rows[1]
            .iter()
            .find(|tile| tile.label == "User Management")
            .unwrap();
        assert!(!tile.enabled);
        assert_eq!(
            tile.action,
            SettingsHomeTileAction::OpenPanel(NativeSettingsPanel::UserManagement)
        );
    }

    #[test]
    fn gui_cli_profile_slot_labels_cover_all_slots() {
        let labels: Vec<_> = gui_cli_profile_slots()
            .into_iter()
            .map(gui_cli_profile_slot_label)
            .collect();
        assert_eq!(
            labels,
            vec!["Default", "Calcurse", "Spotify Player", "Ranger", "Reddit",]
        );
    }

    #[test]
    fn desktop_settings_ui_defaults_pick_current_user_when_present() {
        let draft = get_settings();
        let username = sorted_usernames().into_iter().next().unwrap_or_default();
        let defaults = build_desktop_settings_ui_defaults(&draft, Some(&username));
        assert_eq!(defaults.panel, NativeSettingsPanel::Home);
        assert_eq!(defaults.user_selected, username);
        assert_eq!(defaults.cli_profile_slot, GuiCliProfileSlot::Default);
        assert_eq!(defaults.user_create_auth, AuthMethod::Password);
        assert_eq!(defaults.user_edit_auth, AuthMethod::Password);
    }

    #[test]
    fn desktop_settings_back_target_routes_nested_panels_to_parent() {
        assert_eq!(
            desktop_settings_back_target(NativeSettingsPanel::ConnectionsNetwork),
            NativeSettingsPanel::Connections
        );
        assert_eq!(
            desktop_settings_back_target(NativeSettingsPanel::UserManagementEditUsers),
            NativeSettingsPanel::UserManagement
        );
        assert_eq!(
            desktop_settings_back_target(NativeSettingsPanel::Appearance),
            NativeSettingsPanel::Home
        );
    }

    #[test]
    fn desktop_settings_nav_items_cover_connections_and_user_management() {
        let connections = desktop_settings_connections_nav_items();
        assert_eq!(connections[0].label, "Network");
        assert_eq!(
            connections[1].panel,
            NativeSettingsPanel::ConnectionsBluetooth
        );

        let user_management = desktop_settings_user_management_nav_items();
        assert_eq!(user_management[0].label, "View Users");
        assert_eq!(
            user_management[3].panel,
            NativeSettingsPanel::UserManagementEditCurrentUser
        );
    }
}
