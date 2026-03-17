pub mod legacy;
pub use legacy::{
    apps, auth, boot, checks, desktop, docedit, documents, hacking, installer, nuke_codes,
    settings, shell_terminal,
};

pub mod config;
pub mod connections;
pub mod core;
pub mod default_apps;
pub mod diag;
pub mod launcher;
pub mod native;
pub mod pty;
pub mod session;
pub mod sound;
pub mod status;
pub mod ui;
