/// RobcOS Sound System
///
/// Mirrors Python's playsound(path, False) exactly:
/// spawn the OS player and return immediately — no waiting, no blocking.
/// A concurrent-sound counter caps simultaneous afplay processes to avoid
/// unlimited spawning from held-down keys.

use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

use rand::seq::SliceRandom;

use crate::config::get_settings;

static STOPPED: AtomicBool  = AtomicBool::new(false);
static ACTIVE:  AtomicUsize = AtomicUsize::new(0);
const  MAX_CONCURRENT: usize = 6;

static LAST_NAVIGATE_MS:  AtomicU64 = AtomicU64::new(0);
static LAST_KEYPRESS_MS:  AtomicU64 = AtomicU64::new(0);
static LAST_BOOT_KEY_MS:  AtomicU64 = AtomicU64::new(0);
static LAST_BOOT_HEAD_MS: AtomicU64 = AtomicU64::new(0);

const NAVIGATE_GAP_MS:  u64 = 28;
const KEYPRESS_GAP_MS:  u64 = 16;
const BOOT_KEY_GAP_MS:  u64 = 0;
const BOOT_HEAD_GAP_MS: u64 = 120;

pub fn stop_audio() {
    STOPPED.store(true, Ordering::SeqCst);
}

// ── Temp paths (written once) ─────────────────────────────────────────────────

struct SoundPaths {
    login:     PathBuf,
    logout:    PathBuf,
    error:     PathBuf,
    navigate:  PathBuf,
    keypress:  PathBuf,
    boot_head: PathBuf,
    boot_keys: Vec<PathBuf>,
}

static PATHS: OnceLock<SoundPaths> = OnceLock::new();

#[derive(Default)]
struct BootShuffleState {
    order: Vec<usize>,
    pos: usize,
    last: Option<usize>,
}

static BOOT_SHUFFLE: OnceLock<Mutex<BootShuffleState>> = OnceLock::new();

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
        keypress: write_temp("keypress", include_bytes!("sounds/ui_hacking_charscroll.wav")),
        boot_head: write_temp("boot_head", include_bytes!("sounds/ui_hacking_charenter_01.wav")),
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
        { let _ = Command::new("afplay").arg(&path).status(); }

        #[cfg(target_os = "linux")]
        {
            if Command::new("aplay").arg("-q").arg(&path).status().is_err() {
                let _ = Command::new("paplay").arg(&path).status();
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
                .status();
        }

        ACTIVE.fetch_sub(1, Ordering::Relaxed);
    });
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn passes_gap(last_ms: &AtomicU64, min_gap_ms: u64) -> bool {
    if min_gap_ms == 0 {
        return true;
    }

    let now = now_ms();
    loop {
        let prev = last_ms.load(Ordering::Relaxed);
        if now.saturating_sub(prev) < min_gap_ms {
            return false;
        }
        if last_ms
            .compare_exchange_weak(prev, now, Ordering::Relaxed, Ordering::Relaxed)
            .is_ok()
        {
            return true;
        }
    }
}

fn next_boot_key_index(len: usize) -> usize {
    if len <= 1 {
        return 0;
    }

    let lock = BOOT_SHUFFLE.get_or_init(|| Mutex::new(BootShuffleState::default()));
    let Ok(mut state) = lock.lock() else {
        return 0;
    };

    if state.order.len() != len || state.pos >= state.order.len() {
        state.order = (0..len).collect();
        let mut rng = rand::thread_rng();
        state.order.shuffle(&mut rng);

        if state.order.len() > 1
            && state.last.is_some()
            && state.order.first().copied() == state.last
        {
            state.order.swap(0, 1);
        }

        state.pos = 0;
    }

    let idx = state.order[state.pos];
    state.pos += 1;
    state.last = Some(idx);
    idx
}

// ── Public API ────────────────────────────────────────────────────────────────

pub fn play_boot_key() {
    if !get_settings().sound { return; }
    if !passes_gap(&LAST_BOOT_KEY_MS, BOOT_KEY_GAP_MS) { return; }
    let paths = &get_paths().boot_keys;
    if paths.is_empty() { return; }
    let idx = next_boot_key_index(paths.len());
    play_nonblocking(paths[idx].clone());
}

pub fn play_navigate() {
    if !get_settings().sound { return; }
    if !passes_gap(&LAST_NAVIGATE_MS, NAVIGATE_GAP_MS) { return; }
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

pub fn play_startup() {
    // Startup is intentionally silent; boot typing handles the full effect.
}

pub fn play_boot_header() {
    if !get_settings().sound { return; }
    if !passes_gap(&LAST_BOOT_HEAD_MS, BOOT_HEAD_GAP_MS) { return; }
    play_nonblocking(get_paths().boot_head.clone());
}

#[allow(dead_code)]
pub fn play_keypress() {
    if !get_settings().sound { return; }
    if !passes_gap(&LAST_KEYPRESS_MS, KEYPRESS_GAP_MS) { return; }
    play_nonblocking(get_paths().keypress.clone());
}
