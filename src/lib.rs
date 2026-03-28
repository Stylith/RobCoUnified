pub use nucleon_shared::{
    config, connections, core, default_apps, diag, launcher, platform, pty, session, sound, status,
    theme, ui,
};

pub mod legacy;
pub use legacy::{
    apps, auth, boot, checks, desktop, docedit, documents, hacking, installer, settings,
    shell_terminal,
};

pub mod native;
