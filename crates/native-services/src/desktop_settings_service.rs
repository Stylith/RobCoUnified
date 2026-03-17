use super::shared_file_manager_settings::{
    FileManagerDisplaySettingsUpdate, FileManagerSettingsUpdate,
};
use crate::config::{
    cycle_hacking_difficulty, get_settings, persist_settings, reload_settings, update_settings,
    DesktopFileManagerSettings, DesktopPtyProfileSettings, HackingDifficulty, Settings,
};
use std::path::Path;

fn persist_settings_change<F>(apply: F)
where
    F: FnOnce(&mut Settings),
{
    update_settings(apply);
    persist_settings();
}

pub fn load_settings_snapshot() -> Settings {
    get_settings()
}

pub fn load_desktop_file_manager_settings() -> DesktopFileManagerSettings {
    load_settings_snapshot().desktop_file_manager
}

pub fn load_hacking_difficulty() -> HackingDifficulty {
    load_settings_snapshot().hacking_difficulty
}

pub fn reload_settings_snapshot() -> Settings {
    reload_settings();
    get_settings()
}

pub fn persist_settings_draft(settings: &Settings) -> Settings {
    persist_settings_change(|current| *current = settings.clone());
    reload_settings_snapshot()
}

pub fn sync_live_file_manager_settings_to_draft(draft: &mut Settings) {
    draft.desktop_file_manager = load_settings_snapshot().desktop_file_manager;
}

pub fn apply_file_manager_display_settings_update(
    draft: &mut Settings,
    update: FileManagerDisplaySettingsUpdate,
) {
    persist_settings_change(|settings| match update {
        FileManagerDisplaySettingsUpdate::ToggleTreePanel => {
            settings.desktop_file_manager.show_tree_panel =
                !settings.desktop_file_manager.show_tree_panel;
        }
        FileManagerDisplaySettingsUpdate::ToggleHiddenFiles => {
            settings.desktop_file_manager.show_hidden_files =
                !settings.desktop_file_manager.show_hidden_files;
        }
        FileManagerDisplaySettingsUpdate::SetViewMode(mode) => {
            settings.desktop_file_manager.view_mode = mode;
        }
        FileManagerDisplaySettingsUpdate::SetSortMode(mode) => {
            settings.desktop_file_manager.sort_mode = mode;
        }
    });
    sync_live_file_manager_settings_to_draft(draft);
}

pub fn apply_file_manager_settings_update(draft: &mut Settings, update: FileManagerSettingsUpdate) {
    persist_settings_change(|settings| {
        update.apply(&mut settings.desktop_file_manager);
    });
    sync_live_file_manager_settings_to_draft(draft);
}

pub fn cycle_hacking_difficulty_in_settings(draft: &mut Settings) {
    persist_settings_change(|settings| {
        settings.hacking_difficulty = cycle_hacking_difficulty(settings.hacking_difficulty, true);
    });
    draft.hacking_difficulty = load_settings_snapshot().hacking_difficulty;
}

fn pty_profile_key(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    let base = Path::new(trimmed)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or(trimmed)
        .trim();
    if base.is_empty() {
        None
    } else {
        Some(base.to_ascii_lowercase())
    }
}

pub fn pty_profile_for_command(cmd: &[String]) -> DesktopPtyProfileSettings {
    let profiles = load_settings_snapshot().desktop_cli_profiles;
    let Some(base) = cmd.first().and_then(|program| pty_profile_key(program)) else {
        return profiles.default;
    };
    if let Some(custom) = profiles.custom.get(&base) {
        return custom.clone();
    }
    match base.as_str() {
        name if name.starts_with("calcurse") => profiles.calcurse,
        name if name.starts_with("myman") => DesktopPtyProfileSettings {
            live_resize: false,
            preferred_w: Some(96),
            preferred_h: Some(32),
            ..profiles.default
        },
        "spotify_player" => profiles.spotify_player,
        "ranger" => profiles.ranger,
        "tuir" | "rtv" => profiles.reddit,
        _ => profiles.default,
    }
}

pub fn pty_force_render_mode(cmd: &[String]) -> Option<bool> {
    let base = cmd.first().and_then(|program| pty_profile_key(program))?;
    match base.as_str() {
        name if name.starts_with("myman") => Some(false),
        "spotify_player" | "ranger" => Some(false),
        _ => None,
    }
}
