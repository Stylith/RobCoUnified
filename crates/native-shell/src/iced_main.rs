//! Entry point for the iced-based RobCoOS binary (`robcos-iced`).
//!
//! The existing eframe binary (`robcos-native`) is not affected.
//! This binary will eventually replace it once feature parity is reached.

use robcos::native::shell::RobcoShell;
use robcos::core::auth::ensure_default_admin;
use robcos::config::reload_settings;

fn main() -> iced::Result {
    // Ensure there is at least one admin user and load settings from disk.
    ensure_default_admin();
    reload_settings();

    iced::application("RobCoOS", RobcoShell::update, RobcoShell::view)
        .theme(RobcoShell::theme)
        .subscription(RobcoShell::subscription)
        .window(iced::window::Settings {
            size: iced::Size::new(1360.0, 840.0),
            min_size: Some(iced::Size::new(960.0, 600.0)),
            ..iced::window::Settings::default()
        })
        .run_with(RobcoShell::new)
}
