use crate::config::{get_settings, persist_settings, reload_settings, update_settings, Settings};

pub fn load_settings_snapshot() -> Settings {
    get_settings()
}

pub fn reload_settings_snapshot() -> Settings {
    reload_settings();
    get_settings()
}

pub fn persist_settings_draft(settings: &Settings) -> Settings {
    update_settings(|current| *current = settings.clone());
    persist_settings();
    reload_settings_snapshot()
}
