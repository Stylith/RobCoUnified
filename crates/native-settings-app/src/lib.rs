use nucleon_native_services::desktop_default_apps_service::{
    custom_command_input_for_slot, DefaultAppSlot,
};
use nucleon_native_services::desktop_user_service::sorted_usernames;
use nucleon_native_terminal_app::{SettingsChoiceKind, SettingsChoiceOverlay};
use nucleon_shared::config::{
    CliAcsMode, CrtPreset, DesktopCliProfiles, DesktopPtyProfileSettings, NativeStartupWindowMode,
    OpenMode, Settings, CUSTOM_THEME_NAME, THEMES,
};
use nucleon_shared::connections::macos_connections_disabled;
use nucleon_shared::core::auth::AuthMethod;
use nucleon_shared::platform::CapabilityId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TerminalSettingsEvent {
    None,
    Persist,
    Back,
    OpenPanel(TerminalSettingsPanel),
    OpenCapability(CapabilityId),
    EnterUserManagement,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalSettingsPanel {
    Home,
    General,
    Appearance,
    AppearanceEffects,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DesktopSettingsVisibility {
    pub default_apps: bool,
    pub connections: bool,
    pub edit_menus: bool,
    pub about: bool,
}

impl Default for DesktopSettingsVisibility {
    fn default() -> Self {
        Self {
            default_apps: true,
            connections: !macos_connections_disabled(),
            edit_menus: true,
            about: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TerminalSettingsVisibility {
    pub default_apps: bool,
    pub connections: bool,
    pub edit_menus: bool,
    pub about: bool,
}

impl Default for TerminalSettingsVisibility {
    fn default() -> Self {
        Self {
            default_apps: true,
            connections: !macos_connections_disabled(),
            edit_menus: true,
            about: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeSettingsPanel {
    Home,
    General,
    Appearance,
    Addons,
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
    OpenEffects,
    Sound,
    SystemSoundVolume,
    Bootup,
    NavigationHints,
    Theme,
    CustomThemeRed,
    CustomThemeGreen,
    CustomThemeBlue,
    BorderGlyphs,
    CrtEffectsEnabled,
    CrtPreset,
    CrtCurvature,
    CrtScanlines,
    CrtGlow,
    CrtBloom,
    CrtVignette,
    CrtNoise,
    CrtFlicker,
    CrtJitter,
    CrtBurnIn,
    CrtGlowLine,
    CrtGlowLineSpeed,
    CrtPhosphorSoftness,
    CrtBrightness,
    CrtContrast,
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
        NativeSettingsPanel::Addons => "Addons",
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
    desktop_settings_home_rows_with_visibility(is_admin, DesktopSettingsVisibility::default())
}

pub fn desktop_settings_home_rows_with_visibility(
    is_admin: bool,
    visibility: DesktopSettingsVisibility,
) -> Vec<Vec<SettingsHomeTile>> {
    let mut first_row = vec![
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
            action: SettingsHomeTileAction::OpenPanel(NativeSettingsPanel::Addons),
            label: "Addons",
            icon: "[+]",
            enabled: true,
        },
    ];
    if visibility.default_apps {
        first_row.push(SettingsHomeTile {
            action: SettingsHomeTileAction::OpenPanel(NativeSettingsPanel::DefaultApps),
            label: "Default Apps",
            icon: "[D]",
            enabled: true,
        });
    }

    let mut second_row = Vec::new();
    if visibility.connections {
        second_row.push(SettingsHomeTile {
            action: SettingsHomeTileAction::OpenPanel(NativeSettingsPanel::Connections),
            label: "Connections",
            icon: "[C]",
            enabled: true,
        });
    }
    second_row.push(SettingsHomeTile {
        action: SettingsHomeTileAction::OpenPanel(NativeSettingsPanel::CliProfiles),
        label: "CLI Profiles",
        icon: "[=]",
        enabled: true,
    });
    if visibility.edit_menus {
        second_row.push(SettingsHomeTile {
            action: SettingsHomeTileAction::OpenPanel(NativeSettingsPanel::EditMenus),
            label: "Edit Menus",
            icon: "[M]",
            enabled: true,
        });
    }
    second_row.push(SettingsHomeTile {
        action: SettingsHomeTileAction::OpenPanel(NativeSettingsPanel::UserManagement),
        label: "User Management",
        icon: "[U]",
        enabled: is_admin,
    });

    let mut rows = vec![first_row, second_row];
    if visibility.about {
        rows.push(vec![SettingsHomeTile {
            action: SettingsHomeTileAction::OpenPanel(NativeSettingsPanel::About),
            label: "About",
            icon: "[i]",
            enabled: true,
        }]);
    }
    rows
}

pub fn desktop_settings_panel_enabled(
    panel: NativeSettingsPanel,
    visibility: DesktopSettingsVisibility,
) -> bool {
    match panel {
        NativeSettingsPanel::DefaultApps => visibility.default_apps,
        NativeSettingsPanel::Connections
        | NativeSettingsPanel::ConnectionsNetwork
        | NativeSettingsPanel::ConnectionsBluetooth => visibility.connections,
        NativeSettingsPanel::EditMenus => visibility.edit_menus,
        NativeSettingsPanel::About => visibility.about,
        _ => true,
    }
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
    terminal_settings_panel_rows(TerminalSettingsPanel::Home, draft, is_admin)
}

pub fn terminal_settings_panel_rows(
    panel: TerminalSettingsPanel,
    draft: &Settings,
    is_admin: bool,
) -> Vec<String> {
    terminal_settings_panel_rows_with_visibility(
        panel,
        draft,
        is_admin,
        TerminalSettingsVisibility::default(),
    )
}

pub fn terminal_settings_panel_rows_with_visibility(
    panel: TerminalSettingsPanel,
    draft: &Settings,
    is_admin: bool,
    visibility: TerminalSettingsVisibility,
) -> Vec<String> {
    terminal_settings_rows_with_ids_for_visibility(panel, draft, is_admin, visibility)
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
    handle_settings_activation_with_visibility(
        panel,
        draft,
        idx,
        choice_overlay,
        is_admin,
        TerminalSettingsVisibility::default(),
    )
}

pub fn handle_settings_activation_with_visibility(
    panel: TerminalSettingsPanel,
    draft: &mut Settings,
    idx: usize,
    choice_overlay: &mut Option<SettingsChoiceOverlay>,
    is_admin: bool,
    visibility: TerminalSettingsVisibility,
) -> TerminalSettingsEvent {
    let rows = terminal_settings_rows_with_ids_for_visibility(panel, draft, is_admin, visibility);
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
        SettingsRowId::OpenEffects => {
            TerminalSettingsEvent::OpenPanel(TerminalSettingsPanel::AppearanceEffects)
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
        SettingsRowId::CrtEffectsEnabled => {
            draft.display_effects.enabled = !draft.display_effects.enabled;
            if draft.display_effects.enabled && draft.display_effects.preset == CrtPreset::Off {
                draft.display_effects.apply_preset(CrtPreset::Classic);
            }
            TerminalSettingsEvent::Persist
        }
        SettingsRowId::CrtPreset => {
            *choice_overlay = Some(open_settings_choice(draft, SettingsChoiceKind::CrtPreset));
            TerminalSettingsEvent::None
        }
        SettingsRowId::CrtCurvature
        | SettingsRowId::CrtScanlines
        | SettingsRowId::CrtGlow
        | SettingsRowId::CrtBloom
        | SettingsRowId::CrtVignette
        | SettingsRowId::CrtNoise
        | SettingsRowId::CrtFlicker
        | SettingsRowId::CrtJitter
        | SettingsRowId::CrtBurnIn
        | SettingsRowId::CrtGlowLine
        | SettingsRowId::CrtGlowLineSpeed
        | SettingsRowId::CrtPhosphorSoftness
        | SettingsRowId::CrtBrightness
        | SettingsRowId::CrtContrast => TerminalSettingsEvent::None,
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
        SettingsRowId::Connections => {
            TerminalSettingsEvent::OpenCapability(CapabilityId::from("connections-ui"))
        }
        SettingsRowId::EditMenus => {
            TerminalSettingsEvent::OpenCapability(CapabilityId::from("edit-menus-ui"))
        }
        SettingsRowId::DefaultApps => {
            TerminalSettingsEvent::OpenCapability(CapabilityId::from("default-apps-ui"))
        }
        SettingsRowId::About => {
            TerminalSettingsEvent::OpenCapability(CapabilityId::from("about-ui"))
        }
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
        SettingsChoiceKind::CrtPreset => match draft.display_effects.preset {
            CrtPreset::Off => 0,
            CrtPreset::Subtle => 1,
            CrtPreset::Classic => 2,
            CrtPreset::WornTerminal => 3,
            CrtPreset::ExtremeRetro => 4,
            CrtPreset::Custom => 5,
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
        SettingsChoiceKind::CrtPreset => vec![
            CrtPreset::Off.label().to_string(),
            CrtPreset::Subtle.label().to_string(),
            CrtPreset::Classic.label().to_string(),
            CrtPreset::WornTerminal.label().to_string(),
            CrtPreset::ExtremeRetro.label().to_string(),
            CrtPreset::Custom.label().to_string(),
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
        SettingsChoiceKind::CrtPreset => {
            let preset = match selected {
                1 => CrtPreset::Subtle,
                2 => CrtPreset::Classic,
                3 => CrtPreset::WornTerminal,
                4 => CrtPreset::ExtremeRetro,
                5 => CrtPreset::Custom,
                _ => CrtPreset::Off,
            };
            if preset == CrtPreset::Custom {
                draft.display_effects.preset = CrtPreset::Custom;
            } else {
                draft.display_effects.apply_preset(preset);
            }
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
        SettingsRowId::CrtCurvature => adjust_crt_value(
            &mut draft.display_effects.curvature,
            delta,
            0.0,
            0.2,
            0.01,
            &mut draft.display_effects.preset,
        ),
        SettingsRowId::CrtScanlines => adjust_crt_value(
            &mut draft.display_effects.scanlines,
            delta,
            0.0,
            1.0,
            0.05,
            &mut draft.display_effects.preset,
        ),
        SettingsRowId::CrtGlow => adjust_crt_value(
            &mut draft.display_effects.glow,
            delta,
            0.0,
            1.5,
            0.05,
            &mut draft.display_effects.preset,
        ),
        SettingsRowId::CrtBloom => adjust_crt_value(
            &mut draft.display_effects.bloom,
            delta,
            0.0,
            1.5,
            0.05,
            &mut draft.display_effects.preset,
        ),
        SettingsRowId::CrtVignette => adjust_crt_value(
            &mut draft.display_effects.vignette,
            delta,
            0.0,
            1.0,
            0.05,
            &mut draft.display_effects.preset,
        ),
        SettingsRowId::CrtNoise => adjust_crt_value(
            &mut draft.display_effects.noise,
            delta,
            0.0,
            0.35,
            0.01,
            &mut draft.display_effects.preset,
        ),
        SettingsRowId::CrtFlicker => adjust_crt_value(
            &mut draft.display_effects.flicker,
            delta,
            0.0,
            0.3,
            0.01,
            &mut draft.display_effects.preset,
        ),
        SettingsRowId::CrtJitter => adjust_crt_value(
            &mut draft.display_effects.jitter,
            delta,
            0.0,
            0.12,
            0.005,
            &mut draft.display_effects.preset,
        ),
        SettingsRowId::CrtBurnIn => adjust_crt_value(
            &mut draft.display_effects.burn_in,
            delta,
            0.0,
            1.0,
            0.05,
            &mut draft.display_effects.preset,
        ),
        SettingsRowId::CrtGlowLine => adjust_crt_value(
            &mut draft.display_effects.glow_line,
            delta,
            0.0,
            1.0,
            0.05,
            &mut draft.display_effects.preset,
        ),
        SettingsRowId::CrtGlowLineSpeed => adjust_crt_value(
            &mut draft.display_effects.glow_line_speed,
            delta,
            0.2,
            2.0,
            0.05,
            &mut draft.display_effects.preset,
        ),
        SettingsRowId::CrtPhosphorSoftness => adjust_crt_value(
            &mut draft.display_effects.phosphor_softness,
            delta,
            0.0,
            1.0,
            0.05,
            &mut draft.display_effects.preset,
        ),
        SettingsRowId::CrtBrightness => adjust_crt_value(
            &mut draft.display_effects.brightness,
            delta,
            0.7,
            1.4,
            0.05,
            &mut draft.display_effects.preset,
        ),
        SettingsRowId::CrtContrast => adjust_crt_value(
            &mut draft.display_effects.contrast,
            delta,
            0.8,
            1.5,
            0.05,
            &mut draft.display_effects.preset,
        ),
        _ => false,
    }
}

fn terminal_settings_rows_with_ids(
    panel: TerminalSettingsPanel,
    draft: &Settings,
    is_admin: bool,
) -> Vec<(String, SettingsRowId)> {
    terminal_settings_rows_with_ids_for_visibility(
        panel,
        draft,
        is_admin,
        TerminalSettingsVisibility::default(),
    )
}

fn terminal_settings_rows_with_ids_for_visibility(
    panel: TerminalSettingsPanel,
    draft: &Settings,
    is_admin: bool,
    visibility: TerminalSettingsVisibility,
) -> Vec<(String, SettingsRowId)> {
    match panel {
        TerminalSettingsPanel::Home => {
            let mut rows = vec![
                ("General".to_string(), SettingsRowId::OpenGeneral),
                ("Appearance".to_string(), SettingsRowId::OpenAppearance),
            ];
            if visibility.default_apps {
                rows.push(("Default Apps".to_string(), SettingsRowId::DefaultApps));
            }
            if visibility.connections {
                rows.push(("Connections".to_string(), SettingsRowId::Connections));
            }
            if visibility.edit_menus {
                rows.push(("Edit Menus".to_string(), SettingsRowId::EditMenus));
            }
            if is_admin {
                rows.push(("User Management".to_string(), SettingsRowId::UserManagement));
            }
            if visibility.about {
                rows.push(("About".to_string(), SettingsRowId::About));
            }
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
            rows.push(("CRT Effects".to_string(), SettingsRowId::OpenEffects));
            rows.push(("Back".to_string(), SettingsRowId::Back));
            rows
        }
        TerminalSettingsPanel::AppearanceEffects => {
            let mut rows = vec![(
                format!(
                    "CRT Effects: {} [toggle]",
                    if draft.display_effects.enabled {
                        "ON"
                    } else {
                        "OFF"
                    }
                ),
                SettingsRowId::CrtEffectsEnabled,
            )];
            rows.push((
                format!(
                    "CRT Preset: {} [choose]",
                    draft.display_effects.preset.label()
                ),
                SettingsRowId::CrtPreset,
            ));
            if draft.display_effects.enabled {
                rows.extend([
                    (
                        format!(
                            "CRT Curvature: {:.2} [adjust]",
                            draft.display_effects.curvature
                        ),
                        SettingsRowId::CrtCurvature,
                    ),
                    (
                        format!(
                            "CRT Scanlines: {:.2} [adjust]",
                            draft.display_effects.scanlines
                        ),
                        SettingsRowId::CrtScanlines,
                    ),
                    (
                        format!("CRT Glow: {:.2} [adjust]", draft.display_effects.glow),
                        SettingsRowId::CrtGlow,
                    ),
                    (
                        format!("CRT Bloom: {:.2} [adjust]", draft.display_effects.bloom),
                        SettingsRowId::CrtBloom,
                    ),
                    (
                        format!(
                            "CRT Vignette: {:.2} [adjust]",
                            draft.display_effects.vignette
                        ),
                        SettingsRowId::CrtVignette,
                    ),
                    (
                        format!("CRT Noise: {:.2} [adjust]", draft.display_effects.noise),
                        SettingsRowId::CrtNoise,
                    ),
                    (
                        format!("CRT Flicker: {:.2} [adjust]", draft.display_effects.flicker),
                        SettingsRowId::CrtFlicker,
                    ),
                    (
                        format!("CRT Jitter: {:.3} [adjust]", draft.display_effects.jitter),
                        SettingsRowId::CrtJitter,
                    ),
                    (
                        format!("CRT Burn-In: {:.2} [adjust]", draft.display_effects.burn_in),
                        SettingsRowId::CrtBurnIn,
                    ),
                    (
                        format!(
                            "CRT Glow Line: {:.2} [adjust]",
                            draft.display_effects.glow_line
                        ),
                        SettingsRowId::CrtGlowLine,
                    ),
                    (
                        format!(
                            "CRT Glow Line Speed: {:.2} [adjust]",
                            draft.display_effects.glow_line_speed
                        ),
                        SettingsRowId::CrtGlowLineSpeed,
                    ),
                    (
                        format!(
                            "CRT Softness: {:.2} [adjust]",
                            draft.display_effects.phosphor_softness
                        ),
                        SettingsRowId::CrtPhosphorSoftness,
                    ),
                    (
                        format!(
                            "CRT Brightness: {:.2} [adjust]",
                            draft.display_effects.brightness
                        ),
                        SettingsRowId::CrtBrightness,
                    ),
                    (
                        format!(
                            "CRT Contrast: {:.2} [adjust]",
                            draft.display_effects.contrast
                        ),
                        SettingsRowId::CrtContrast,
                    ),
                ]);
            }
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

fn adjust_crt_value(
    value: &mut f32,
    delta: i16,
    min: f32,
    max: f32,
    step: f32,
    preset: &mut CrtPreset,
) -> bool {
    let next = (*value + delta as f32 * step).clamp(min, max);
    if (next - *value).abs() < f32::EPSILON {
        return false;
    }
    *value = next;
    *preset = CrtPreset::Custom;
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use nucleon_shared::config::get_settings;

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
                TerminalSettingsEvent::OpenCapability(capability)
                    if capability == CapabilityId::from("connections-ui")
            ));
        }
        let general_idx = user_rows
            .iter()
            .position(|(_, id)| *id == SettingsRowId::OpenGeneral)
            .unwrap();
        let appearance_idx = user_rows
            .iter()
            .position(|(_, id)| *id == SettingsRowId::OpenAppearance)
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
                appearance_idx,
                &mut overlay,
                false,
            ),
            TerminalSettingsEvent::OpenPanel(TerminalSettingsPanel::Appearance)
        ));
        assert!(matches!(
            handle_settings_activation(
                TerminalSettingsPanel::Home,
                &mut draft,
                edit_menus_idx,
                &mut overlay,
                false,
            ),
            TerminalSettingsEvent::OpenCapability(capability)
                if capability == CapabilityId::from("edit-menus-ui")
        ));
        assert!(matches!(
            handle_settings_activation(
                TerminalSettingsPanel::Home,
                &mut draft,
                default_apps_idx,
                &mut overlay,
                false,
            ),
            TerminalSettingsEvent::OpenCapability(capability)
                if capability == CapabilityId::from("default-apps-ui")
        ));
        assert!(matches!(
            handle_settings_activation(
                TerminalSettingsPanel::Home,
                &mut draft,
                about_idx,
                &mut overlay,
                false,
            ),
            TerminalSettingsEvent::OpenCapability(capability)
                if capability == CapabilityId::from("about-ui")
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
    fn terminal_settings_rows_respect_explicit_visibility() {
        let draft = get_settings();
        let rows = terminal_settings_panel_rows_with_visibility(
            TerminalSettingsPanel::Home,
            &draft,
            false,
            TerminalSettingsVisibility {
                default_apps: false,
                connections: false,
                edit_menus: false,
                about: false,
            },
        );

        assert!(rows.iter().any(|label| label == "General"));
        assert!(rows.iter().any(|label| label == "Appearance"));
        assert!(!rows.iter().any(|label| label == "Default Apps"));
        assert!(!rows.iter().any(|label| label == "Connections"));
        assert!(!rows.iter().any(|label| label == "Edit Menus"));
        assert!(!rows.iter().any(|label| label == "About"));
        assert_eq!(rows.last().map(|label| label.as_str()), Some("Back"));
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
    fn appearance_rows_include_crt_controls_when_enabled() {
        let mut draft = get_settings();
        draft.display_effects.enabled = true;
        let rows =
            terminal_settings_rows_with_ids(TerminalSettingsPanel::Appearance, &draft, false);
        assert!(rows
            .iter()
            .any(|(_, id)| matches!(id, SettingsRowId::OpenEffects)));
        assert!(!rows
            .iter()
            .any(|(_, id)| matches!(id, SettingsRowId::CrtEffectsEnabled)));
    }

    #[test]
    fn appearance_effects_rows_include_crt_controls_when_enabled() {
        let mut draft = get_settings();
        draft.display_effects.enabled = true;
        let rows = terminal_settings_rows_with_ids(
            TerminalSettingsPanel::AppearanceEffects,
            &draft,
            false,
        );
        assert!(rows
            .iter()
            .any(|(_, id)| matches!(id, SettingsRowId::CrtEffectsEnabled)));
        assert!(rows
            .iter()
            .any(|(_, id)| matches!(id, SettingsRowId::CrtPreset)));
        assert!(rows
            .iter()
            .any(|(_, id)| matches!(id, SettingsRowId::CrtBloom)));
        assert!(rows
            .iter()
            .any(|(_, id)| matches!(id, SettingsRowId::CrtGlow)));
        assert!(rows
            .iter()
            .any(|(_, id)| matches!(id, SettingsRowId::CrtBurnIn)));
        assert!(rows
            .iter()
            .any(|(_, id)| matches!(id, SettingsRowId::CrtGlowLine)));
        assert!(rows
            .iter()
            .any(|(_, id)| matches!(id, SettingsRowId::CrtGlowLineSpeed)));
        assert!(rows
            .iter()
            .any(|(_, id)| matches!(id, SettingsRowId::CrtContrast)));
    }

    #[test]
    fn effects_row_opens_effects_panel() {
        let mut draft = get_settings();
        let mut overlay = None;
        let rows =
            terminal_settings_rows_with_ids(TerminalSettingsPanel::Appearance, &draft, false);
        let idx = rows
            .iter()
            .position(|(_, id)| *id == SettingsRowId::OpenEffects)
            .expect("effects row");

        assert!(matches!(
            handle_settings_activation(
                TerminalSettingsPanel::Appearance,
                &mut draft,
                idx,
                &mut overlay,
                false,
            ),
            TerminalSettingsEvent::OpenPanel(TerminalSettingsPanel::AppearanceEffects)
        ));
    }

    #[test]
    fn crt_preset_choice_applies_effects_preset() {
        let mut draft = get_settings();
        draft.display_effects.apply_preset(CrtPreset::Subtle);
        apply_settings_choice(&mut draft, SettingsChoiceKind::CrtPreset, 4);
        assert_eq!(draft.display_effects.preset, CrtPreset::ExtremeRetro);
        assert!(draft.display_effects.enabled);
        assert!(draft.display_effects.glow >= 0.9);
        assert!(draft.display_effects.bloom >= 0.8);
        assert!(draft.display_effects.burn_in >= 0.5);
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
    fn desktop_settings_home_rows_keep_addons_on_first_row_and_connections_on_second() {
        let rows = desktop_settings_home_rows_with_visibility(
            true,
            DesktopSettingsVisibility {
                default_apps: true,
                connections: true,
                edit_menus: true,
                about: true,
            },
        );

        assert_eq!(
            rows[0].iter().map(|tile| tile.label).collect::<Vec<_>>(),
            vec!["General", "Appearance", "Addons", "Default Apps"]
        );
        assert_eq!(rows[1][0].label, "Connections");
    }

    #[test]
    fn desktop_settings_home_rows_respect_visibility() {
        let rows = desktop_settings_home_rows_with_visibility(
            false,
            DesktopSettingsVisibility {
                default_apps: false,
                connections: false,
                edit_menus: false,
                about: false,
            },
        );

        assert!(!rows[0].iter().any(|tile| tile.label == "Default Apps"));
        assert!(!rows[0].iter().any(|tile| tile.label == "Connections"));
        assert!(!rows[1].iter().any(|tile| tile.label == "Edit Menus"));
        assert!(!rows[1].iter().any(|tile| tile.label == "About"));
        assert!(rows[1].iter().any(|tile| tile.label == "User Management"));
    }

    #[test]
    fn desktop_settings_panel_enabled_matches_visibility() {
        let visibility = DesktopSettingsVisibility {
            default_apps: false,
            connections: false,
            edit_menus: false,
            about: false,
        };

        assert!(!desktop_settings_panel_enabled(
            NativeSettingsPanel::DefaultApps,
            visibility
        ));
        assert!(!desktop_settings_panel_enabled(
            NativeSettingsPanel::ConnectionsBluetooth,
            visibility
        ));
        assert!(!desktop_settings_panel_enabled(
            NativeSettingsPanel::EditMenus,
            visibility
        ));
        assert!(!desktop_settings_panel_enabled(
            NativeSettingsPanel::About,
            visibility
        ));
        assert!(desktop_settings_panel_enabled(
            NativeSettingsPanel::Appearance,
            visibility
        ));
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
