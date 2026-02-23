/// RobcOS Sound System
///
/// Mirrors Python's playsound(path, False) exactly:
/// spawn the OS player and return immediately — no waiting, no blocking.
/// A concurrent-sound counter caps simultaneous afplay processes to avoid
/// unlimited spawning from held-down keys.

use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::OnceLock;

use crate::config::get_settings;

static STOPPED: AtomicBool  = AtomicBool::new(false);
static ACTIVE:  AtomicUsize = AtomicUsize::new(0);
const  MAX_CONCURRENT: usize = 3;

pub fn stop_audio() {
    STOPPED.store(true, Ordering::SeqCst);
}

// ── Temp paths (written once) ─────────────────────────────────────────────────

struct SoundPaths {
    login:     PathBuf,
    logout:    PathBuf,
    error:     PathBuf,
    navigate:  PathBuf,
    boot_keys: Vec<PathBuf>,
}

static PATHS: OnceLock<SoundPaths> = OnceLock::new();

fn write_temp(name: &str, bytes: &[u8]) -> PathBuf {
    let p = std::env::temp_dir().join(format!("robcos_{name}.wav"));
    let _ = std::fs::write(&p, bytes);
    p
}

fn get_paths() -> &'static SoundPaths {
    PATHS.get_or_init(|| SoundPaths {
        login:    write_temp("login",    include_bytes!("sounds/ui_hacking_passgood.wav")),
        logout:   write_temp("logout",   include_bytes!("sounds/ui_hacking_passbad.wav")),
        error:    write_temp("error",    include_bytes!("sounds/ui_hacking_passbad.wav")),
        navigate: write_temp("navigate", include_bytes!("sounds/ui_hacking_charenter_01.wav")),
        boot_keys: vec![
            write_temp("boot0", include_bytes!("sounds/ui_hacking_charsingle_01.wav")),
            write_temp("boot1", include_bytes!("sounds/ui_hacking_charsingle_02.wav")),
            write_temp("boot2", include_bytes!("sounds/ui_hacking_charsingle_03.wav")),
            write_temp("boot3", include_bytes!("sounds/ui_hacking_charsingle_04.wav")),
            write_temp("boot4", include_bytes!("sounds/ui_hacking_charsingle_05.wav")),
        ],
    })
}

// ── Fire-and-forget spawn (mirrors playsound non-blocking) ────────────────────

fn play_nonblocking(path: PathBuf) {
    if STOPPED.load(Ordering::SeqCst) { return; }
    if ACTIVE.load(Ordering::Relaxed) >= MAX_CONCURRENT { return; }

    ACTIVE.fetch_add(1, Ordering::Relaxed);
    std::thread::spawn(move || {
        #[cfg(target_os = "macos")]
        { let _ = Command::new("afplay").arg(&path).output(); }

        #[cfg(target_os = "linux")]
        {
            if Command::new("aplay").arg("-q").arg(&path).output().is_err() {
                let _ = Command::new("paplay").arg(&path).output();
            }
        }

        #[cfg(target_os = "windows")]
        {
            let script = format!(
                "(New-Object Media.SoundPlayer '{}').PlaySync()",
                path.to_string_lossy().replace('\'', "''")
            );
            let _ = Command::new("powershell")
                .args(["-NoProfile", "-Command", &script])
                .output();
        }

        ACTIVE.fetch_sub(1, Ordering::Relaxed);
    });
}

// ── Public API ────────────────────────────────────────────────────────────────

pub fn play_boot_key() {
    if !get_settings().sound { return; }
    let paths = &get_paths().boot_keys;
    if paths.is_empty() { return; }
    let idx = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.subsec_nanos() as usize)
        .unwrap_or(0) % paths.len();
    play_nonblocking(paths[idx].clone());
}

pub fn play_navigate() {
    if !get_settings().sound { return; }
    play_nonblocking(get_paths().navigate.clone());
}

pub fn play_login() {
    if !get_settings().sound { return; }
    play_nonblocking(get_paths().login.clone());
}

pub fn play_logout() {
    if !get_settings().sound { return; }
    play_nonblocking(get_paths().logout.clone());
}

pub fn play_error() {
    if !get_settings().sound { return; }
    play_nonblocking(get_paths().error.clone());
}

pub fn play_startup()     {}
pub fn play_boot_header() {}
#[allow(dead_code)]
pub fn play_keypress() { play_navigate(); }
