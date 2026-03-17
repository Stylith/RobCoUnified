use crate::config::{DesktopFileManagerSettings, FileManagerSortMode, FileManagerViewMode};

const FILE_MANAGER_OPEN_WITH_HISTORY_LIMIT: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileManagerDisplaySettingsUpdate {
    ToggleTreePanel,
    ToggleHiddenFiles,
    SetViewMode(FileManagerViewMode),
    SetSortMode(FileManagerSortMode),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileManagerSettingsUpdate {
    RecordOpenWithCommand {
        ext_key: String,
        command: String,
    },
    SetOpenWithDefaultCommand {
        ext_key: String,
        command: Option<String>,
    },
    ReplaceOpenWithCommand {
        ext_key: String,
        old_command: String,
        new_command: String,
    },
    RemoveOpenWithCommand {
        ext_key: String,
        command: String,
    },
}

pub fn push_open_with_history(history: &mut Vec<String>, command: &str) {
    let normalized = command.trim();
    if normalized.is_empty() {
        return;
    }
    history.retain(|entry| entry.trim() != normalized);
    history.insert(0, normalized.to_string());
    if history.len() > FILE_MANAGER_OPEN_WITH_HISTORY_LIMIT {
        history.truncate(FILE_MANAGER_OPEN_WITH_HISTORY_LIMIT);
    }
}

pub fn set_open_with_default_in_settings(
    fm: &mut DesktopFileManagerSettings,
    ext_key: &str,
    command: Option<&str>,
) {
    match command.map(str::trim).filter(|value| !value.is_empty()) {
        Some(normalized) => {
            let history = fm
                .open_with_by_extension
                .entry(ext_key.to_string())
                .or_default();
            push_open_with_history(history, normalized);
            fm.open_with_default_by_extension
                .insert(ext_key.to_string(), normalized.to_string());
        }
        None => {
            fm.open_with_default_by_extension.remove(ext_key);
        }
    }
}

#[cfg(test)]
pub fn sync_open_with_settings_to_draft(
    live: &DesktopFileManagerSettings,
    draft: &mut DesktopFileManagerSettings,
) {
    draft.open_with_by_extension = live.open_with_by_extension.clone();
    draft.open_with_default_by_extension = live.open_with_default_by_extension.clone();
}

pub fn open_with_history_for_extension(
    fm: &DesktopFileManagerSettings,
    ext_key: &str,
) -> Vec<String> {
    fm.open_with_by_extension
        .get(ext_key)
        .cloned()
        .unwrap_or_default()
}

pub fn open_with_default_for_extension(
    fm: &DesktopFileManagerSettings,
    ext_key: &str,
) -> Option<String> {
    fm.open_with_default_by_extension
        .get(ext_key)
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

pub fn record_open_with_command_in_settings(
    fm: &mut DesktopFileManagerSettings,
    ext_key: &str,
    command: &str,
) {
    let normalized = command.trim();
    if normalized.is_empty() {
        return;
    }
    let history = fm
        .open_with_by_extension
        .entry(ext_key.to_string())
        .or_default();
    push_open_with_history(history, normalized);
}

impl FileManagerSettingsUpdate {
    pub fn apply(&self, fm: &mut DesktopFileManagerSettings) {
        match self {
            Self::RecordOpenWithCommand { ext_key, command } => {
                record_open_with_command_in_settings(fm, ext_key, command);
            }
            Self::SetOpenWithDefaultCommand { ext_key, command } => {
                set_open_with_default_in_settings(fm, ext_key, command.as_deref());
            }
            Self::ReplaceOpenWithCommand {
                ext_key,
                old_command,
                new_command,
            } => {
                let old_normalized = old_command.trim();
                let new_normalized = new_command.trim();
                if old_normalized.is_empty() || new_normalized.is_empty() {
                    return;
                }
                replace_open_with_command_in_settings(fm, ext_key, old_normalized, new_normalized);
            }
            Self::RemoveOpenWithCommand { ext_key, command } => {
                let normalized = command.trim();
                if normalized.is_empty() {
                    return;
                }
                remove_open_with_command_in_settings(fm, ext_key, normalized);
            }
        }
    }
}

pub fn replace_open_with_command_in_settings(
    fm: &mut DesktopFileManagerSettings,
    ext_key: &str,
    old_normalized: &str,
    new_normalized: &str,
) {
    let was_default = fm
        .open_with_default_by_extension
        .get(ext_key)
        .is_some_and(|current| current.trim() == old_normalized);

    let remove_bucket = {
        let history = fm
            .open_with_by_extension
            .entry(ext_key.to_string())
            .or_default();
        history.retain(|entry| entry.trim() != old_normalized);
        push_open_with_history(history, new_normalized);
        history.is_empty()
    };
    if remove_bucket {
        fm.open_with_by_extension.remove(ext_key);
    }

    if was_default {
        fm.open_with_default_by_extension
            .insert(ext_key.to_string(), new_normalized.to_string());
    }
}

pub fn remove_open_with_command_in_settings(
    fm: &mut DesktopFileManagerSettings,
    ext_key: &str,
    normalized: &str,
) {
    let mut remove_bucket = false;
    if let Some(history) = fm.open_with_by_extension.get_mut(ext_key) {
        history.retain(|entry| entry.trim() != normalized);
        remove_bucket = history.is_empty();
    }
    if remove_bucket {
        fm.open_with_by_extension.remove(ext_key);
    }
    if fm
        .open_with_default_by_extension
        .get(ext_key)
        .is_some_and(|current| current.trim() == normalized)
    {
        fm.open_with_default_by_extension.remove(ext_key);
    }
}
