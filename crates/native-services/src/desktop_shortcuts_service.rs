use super::desktop_launcher_service::{resolve_catalog_command_line, ProgramCatalog};
use super::desktop_search_service::NativeStartLeafAction;
use crate::config::{DesktopIconSortMode, DesktopShortcut, Settings};
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShortcutPropertiesUpdate {
    pub label: String,
    pub command_draft: String,
    pub icon_path: Option<String>,
}

pub fn create_shortcut_from_start_action(
    settings: &mut Settings,
    label: String,
    action: &NativeStartLeafAction,
) {
    let (app_name, launch_command, shortcut_kind) = match action {
        NativeStartLeafAction::LaunchConfiguredApp(name) => (
            name.clone(),
            resolve_catalog_command_line(name.as_str(), ProgramCatalog::Applications),
            "app".to_string(),
        ),
        NativeStartLeafAction::LaunchNetworkProgram(name) => (
            name.clone(),
            resolve_catalog_command_line(name.as_str(), ProgramCatalog::Network),
            "network".to_string(),
        ),
        NativeStartLeafAction::LaunchGameProgram(name) => (
            name.clone(),
            resolve_catalog_command_line(name.as_str(), ProgramCatalog::Games),
            "game".to_string(),
        ),
        NativeStartLeafAction::OpenTextEditor => (label.clone(), None, "editor".to_string()),
        _ => (label.clone(), None, "app".to_string()),
    };

    settings.desktop_shortcuts.push(DesktopShortcut {
        label,
        app_name,
        pos_x: None,
        pos_y: None,
        launch_command,
        icon_path: None,
        shortcut_kind,
    });
}

pub fn delete_shortcut(settings: &mut Settings, shortcut_idx: usize) -> bool {
    if shortcut_idx >= settings.desktop_shortcuts.len() {
        return false;
    }
    settings.desktop_shortcuts.remove(shortcut_idx);
    true
}

pub fn update_shortcut_properties(
    settings: &mut Settings,
    shortcut_idx: usize,
    update: &ShortcutPropertiesUpdate,
) -> bool {
    let Some(shortcut) = settings.desktop_shortcuts.get_mut(shortcut_idx) else {
        return false;
    };
    shortcut.label = update.label.clone();
    shortcut.launch_command = if update.command_draft == shortcut.app_name {
        None
    } else {
        Some(update.command_draft.clone())
    };
    shortcut.icon_path = update.icon_path.clone();
    true
}

pub fn set_shortcut_icon(
    settings: &mut Settings,
    shortcut_idx: usize,
    path: &Path,
) -> Option<String> {
    let path_str = path.to_string_lossy().to_string();
    let shortcut = settings.desktop_shortcuts.get_mut(shortcut_idx)?;
    shortcut.icon_path = Some(path_str.clone());
    Some(path_str)
}

pub fn shortcut_launch_command(settings: &Settings, app_name: &str) -> Option<String> {
    settings
        .desktop_shortcuts
        .iter()
        .find(|shortcut| shortcut.app_name == app_name)
        .and_then(|shortcut| shortcut.launch_command.clone())
}

pub fn sort_shortcuts(settings: &mut Settings, mode: DesktopIconSortMode) {
    settings.desktop_icon_sort = mode;
    match mode {
        DesktopIconSortMode::ByName => {
            settings
                .desktop_shortcuts
                .sort_by(|left, right| left.label.cmp(&right.label));
        }
        DesktopIconSortMode::ByType => {
            settings
                .desktop_shortcuts
                .sort_by(|left, right| left.app_name.cmp(&right.app_name));
        }
        DesktopIconSortMode::Custom => {}
    }
    settings.desktop_icon_custom_positions.clear();
}

pub fn toggle_snap_to_grid(settings: &mut Settings) {
    settings.desktop_snap_to_grid = !settings.desktop_snap_to_grid;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{DesktopIconSortMode, Settings};

    #[test]
    fn create_shortcut_uses_catalog_command_for_app_entries() {
        let mut settings = Settings::default();
        create_shortcut_from_start_action(
            &mut settings,
            "Helix".to_string(),
            &NativeStartLeafAction::LaunchConfiguredApp("Helix".to_string()),
        );

        let shortcut = settings.desktop_shortcuts.first().expect("shortcut");
        assert_eq!(shortcut.label, "Helix");
        assert_eq!(shortcut.app_name, "Helix");
        assert_eq!(shortcut.shortcut_kind, "app");
    }

    #[test]
    fn sort_shortcuts_clears_custom_positions() {
        let mut settings = Settings::default();
        settings.desktop_shortcuts = vec![
            DesktopShortcut {
                label: "Beta".to_string(),
                app_name: "b".to_string(),
                pos_x: None,
                pos_y: None,
                launch_command: None,
                icon_path: None,
                shortcut_kind: "app".to_string(),
            },
            DesktopShortcut {
                label: "Alpha".to_string(),
                app_name: "a".to_string(),
                pos_x: None,
                pos_y: None,
                launch_command: None,
                icon_path: None,
                shortcut_kind: "app".to_string(),
            },
        ];
        settings
            .desktop_icon_custom_positions
            .insert("foo".to_string(), [1.0, 2.0]);

        sort_shortcuts(&mut settings, DesktopIconSortMode::ByName);

        assert_eq!(settings.desktop_shortcuts[0].label, "Alpha");
        assert!(settings.desktop_icon_custom_positions.is_empty());
    }
}
