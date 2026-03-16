//! RobcOS Sound System
//!
//! Fire-and-forget playback using OS audio players.
//! Boot key clips are preprocessed to remove leading dead-space and keep
//! a short click window, which reduces random perceived gaps.

use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

use rand::seq::SliceRandom;

use crate::config::get_settings;

static STOPPED: AtomicBool = AtomicBool::new(false);
static ACTIVE: AtomicUsize = AtomicUsize::new(0);
const MAX_CONCURRENT: usize = 10;

static LAST_NAVIGATE_MS: AtomicU64 = AtomicU64::new(0);
static LAST_NAV_REPEAT_MS: AtomicU64 = AtomicU64::new(0);
static LAST_KEYPRESS_MS: AtomicU64 = AtomicU64::new(0);
static LAST_BOOT_KEY_MS: AtomicU64 = AtomicU64::new(0);
static BOOT_SEQ_IDX: AtomicUsize = AtomicUsize::new(0);

const SOUND_TEMP_DISABLED: bool = false;

const NAVIGATE_REPEAT_GAP_MS: u64 = 80;
const NAVIGATE_HOLD_WINDOW_MS: u64 = 120;
const KEYPRESS_GAP_MS: u64 = 16;
const BOOT_KEY_GAP_MS: u64 = 0;

pub fn stop_audio() {
    STOPPED.store(true, Ordering::SeqCst);
}

struct SoundPaths {
    login: SoundClip,
    logout: SoundClip,
    error: SoundClip,
    navigate: SoundClip,
    keypress: SoundClip,
    boot_keys: Vec<SoundClip>,
}

static PATHS: OnceLock<SoundPaths> = OnceLock::new();
static CLIP_CACHE: OnceLock<Mutex<HashMap<String, PathBuf>>> = OnceLock::new();

struct SoundClip {
    name: &'static str,
    bytes: Vec<u8>,
}

#[derive(Default)]
struct BootShuffleState {
    order: Vec<usize>,
    pos: usize,
    last: Option<usize>,
}

static BOOT_SHUFFLE: OnceLock<Mutex<BootShuffleState>> = OnceLock::new();

fn sound_enabled() -> bool {
    !SOUND_TEMP_DISABLED && get_settings().sound
}

fn system_sound_volume() -> u8 {
    get_settings().system_sound_volume.clamp(0, 100)
}

fn extract_boot_click_window_pcm16_mono(
    bytes: &[u8],
    threshold: i16,
    pre_roll_samples: usize,
    max_samples: usize,
) -> Vec<u8> {
    if bytes.len() < 44 {
        return bytes.to_vec();
    }
    if &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        return bytes.to_vec();
    }
    if &bytes[12..16] != b"fmt " || &bytes[36..40] != b"data" {
        return bytes.to_vec();
    }

    let audio_format = u16::from_le_bytes([bytes[20], bytes[21]]);
    let channels = u16::from_le_bytes([bytes[22], bytes[23]]);
    let bits_per_sample = u16::from_le_bytes([bytes[34], bytes[35]]);
    let data_len = u32::from_le_bytes([bytes[40], bytes[41], bytes[42], bytes[43]]) as usize;
    let data_start = 44usize;

    if audio_format != 1 || channels != 1 || bits_per_sample != 16 {
        return bytes.to_vec();
    }
    if bytes.len() < data_start + data_len {
        return bytes.to_vec();
    }

    let data = &bytes[data_start..data_start + data_len];
    let threshold = threshold as i32;
    let total_samples = data.len() / 2;
    let mut first_sample = None;

    for i in (0..data.len()).step_by(2) {
        if i + 1 >= data.len() {
            break;
        }
        let s = i16::from_le_bytes([data[i], data[i + 1]]) as i32;
        if s.abs() > threshold {
            first_sample = Some(i / 2);
            break;
        }
    }

    let Some(first_sample) = first_sample else {
        return bytes.to_vec();
    };

    let start_sample = first_sample.saturating_sub(pre_roll_samples);
    let mut end_sample = start_sample.saturating_add(max_samples);
    if end_sample > total_samples {
        end_sample = total_samples;
    }
    if end_sample <= start_sample {
        return bytes.to_vec();
    }

    let start_byte = start_sample * 2;
    let end_byte = end_sample * 2;
    let trimmed = &data[start_byte..end_byte];
    let target_bytes = max_samples.saturating_mul(2);

    let mut out = Vec::with_capacity(44 + target_bytes);
    out.extend_from_slice(&bytes[..44]);
    out[4..8].copy_from_slice(&((36 + target_bytes) as u32).to_le_bytes());
    out[40..44].copy_from_slice(&(target_bytes as u32).to_le_bytes());
    out.extend_from_slice(trimmed);
    if trimmed.len() < target_bytes {
        out.resize(44 + target_bytes, 0);
    }
    out
}

fn scale_pcm16_wav(bytes: &[u8], volume: u8) -> Vec<u8> {
    if volume >= 100 || bytes.len() < 44 {
        return bytes.to_vec();
    }
    if &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        return bytes.to_vec();
    }
    if &bytes[12..16] != b"fmt " || &bytes[36..40] != b"data" {
        return bytes.to_vec();
    }

    let audio_format = u16::from_le_bytes([bytes[20], bytes[21]]);
    let bits_per_sample = u16::from_le_bytes([bytes[34], bytes[35]]);
    let data_len = u32::from_le_bytes([bytes[40], bytes[41], bytes[42], bytes[43]]) as usize;
    let data_start = 44usize;

    if audio_format != 1 || bits_per_sample != 16 || bytes.len() < data_start + data_len {
        return bytes.to_vec();
    }

    let mut out = bytes.to_vec();
    for sample in out[data_start..data_start + data_len].chunks_exact_mut(2) {
        let raw = i16::from_le_bytes([sample[0], sample[1]]) as i32;
        let scaled = (raw * volume as i32 / 100).clamp(i16::MIN as i32, i16::MAX as i32) as i16;
        sample.copy_from_slice(&scaled.to_le_bytes());
    }
    out
}

fn clip_path(clip: &SoundClip) -> PathBuf {
    let volume = system_sound_volume();
    let key = format!("{}_{}", clip.name, volume);
    let cache = CLIP_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    if let Ok(guard) = cache.lock() {
        if let Some(existing) = guard.get(&key) {
            return existing.clone();
        }
    }

    let data = scale_pcm16_wav(&clip.bytes, volume);
    let path = std::env::temp_dir().join(format!("robcos_{}_{}.wav", clip.name, volume));
    let _ = std::fs::write(&path, data);
    if let Ok(mut guard) = cache.lock() {
        guard.insert(key, path.clone());
    }
    path
}

fn get_paths() -> &'static SoundPaths {
    PATHS.get_or_init(|| SoundPaths {
        login: SoundClip {
            name: "login",
            bytes: include_bytes!("sounds/ui_hacking_passgood.wav").to_vec(),
        },
        logout: SoundClip {
            name: "logout",
            bytes: include_bytes!("sounds/ui_hacking_passbad.wav").to_vec(),
        },
        error: SoundClip {
            name: "error",
            bytes: include_bytes!("sounds/ui_hacking_passbad.wav").to_vec(),
        },
        navigate: SoundClip {
            name: "navigate",
            bytes: include_bytes!("sounds/ui_hacking_charenter_01.wav").to_vec(),
        },
        keypress: SoundClip {
            name: "keypress",
            bytes: include_bytes!("sounds/ui_hacking_charscroll.wav").to_vec(),
        },
        boot_keys: vec![
            SoundClip {
                name: "boot0",
                bytes: extract_boot_click_window_pcm16_mono(
                    include_bytes!("sounds/ui_hacking_charsingle_01.wav"),
                    550,
                    28,
                    2100,
                ),
            },
            SoundClip {
                name: "boot1",
                bytes: extract_boot_click_window_pcm16_mono(
                    include_bytes!("sounds/ui_hacking_charsingle_02.wav"),
                    550,
                    28,
                    2100,
                ),
            },
            SoundClip {
                name: "boot2",
                bytes: extract_boot_click_window_pcm16_mono(
                    include_bytes!("sounds/ui_hacking_charsingle_03.wav"),
                    550,
                    28,
                    2100,
                ),
            },
            SoundClip {
                name: "boot3",
                bytes: extract_boot_click_window_pcm16_mono(
                    include_bytes!("sounds/ui_hacking_charsingle_04.wav"),
                    550,
                    28,
                    2100,
                ),
            },
            SoundClip {
                name: "boot4",
                bytes: extract_boot_click_window_pcm16_mono(
                    include_bytes!("sounds/ui_hacking_charsingle_05.wav"),
                    550,
                    28,
                    2100,
                ),
            },
        ],
    })
}

fn run_spawn(path: PathBuf) {
    #[cfg(target_os = "macos")]
    let child = Command::new("afplay").arg(&path).spawn();

    #[cfg(target_os = "linux")]
    let child = Command::new("pw-play")
        .arg(&path)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .or_else(|_| {
            Command::new("paplay")
                .arg(&path)
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
        })
        .or_else(|_| {
            Command::new("aplay")
                .arg("-q")
                .arg(&path)
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
        });

    #[cfg(target_os = "windows")]
    let child = {
        let script = format!(
            "(New-Object Media.SoundPlayer '{}').PlaySync()",
            path.to_string_lossy().replace('\'', "''")
        );
        Command::new("powershell")
            .args(["-NoProfile", "-Command", &script])
            .spawn()
    };

    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    let child: std::io::Result<std::process::Child> = Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "audio playback unsupported",
    ));

    match child {
        Ok(mut child) => {
            // Reap the child asynchronously.
            std::thread::spawn(move || {
                let _ = child.wait();
                ACTIVE.fetch_sub(1, Ordering::Relaxed);
            });
        }
        Err(_) => {
            ACTIVE.fetch_sub(1, Ordering::Relaxed);
        }
    }
}

pub fn wait_boot_audio_ready(_timeout_ms: u64) {
    if !sound_enabled() {
        return;
    }
    // Warm up paths so temp files are written before first play call.
    let _ = get_paths();
}

fn play_nonblocking(path: PathBuf) {
    if STOPPED.load(Ordering::SeqCst) {
        return;
    }

    // Soft cap to avoid runaway process spawning on held keys.
    let prev = ACTIVE.fetch_add(1, Ordering::Relaxed);
    if prev >= MAX_CONCURRENT {
        ACTIVE.fetch_sub(1, Ordering::Relaxed);
        return;
    }

    run_spawn(path);
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
        return BOOT_SEQ_IDX.fetch_add(1, Ordering::Relaxed) % len;
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

pub fn play_boot_key() {
    if !sound_enabled() {
        return;
    }
    if !passes_gap(&LAST_BOOT_KEY_MS, BOOT_KEY_GAP_MS) {
        return;
    }
    let paths = &get_paths().boot_keys;
    if paths.is_empty() {
        return;
    }
    let idx = next_boot_key_index(paths.len());
    play_nonblocking(clip_path(&paths[idx]));
}

pub fn play_navigate() {
    if !sound_enabled() {
        return;
    }
    let now = now_ms();
    let prev = LAST_NAVIGATE_MS.swap(now, Ordering::Relaxed);

    // Many callers emit only play_navigate() during key-hold auto-repeat.
    // When calls bunch up, keep the same nav sound but gate it to avoid overlap.
    if prev != 0 && now.saturating_sub(prev) <= NAVIGATE_HOLD_WINDOW_MS {
        if !passes_gap(&LAST_NAV_REPEAT_MS, NAVIGATE_REPEAT_GAP_MS) {
            return;
        }
        play_nonblocking(clip_path(&get_paths().navigate));
        return;
    }

    play_nonblocking(clip_path(&get_paths().navigate));
}

pub fn play_navigate_repeat() {
    if !sound_enabled() {
        return;
    }
    if !passes_gap(&LAST_NAV_REPEAT_MS, NAVIGATE_REPEAT_GAP_MS) {
        return;
    }
    play_nonblocking(clip_path(&get_paths().navigate));
}

pub fn play_login() {
    if !sound_enabled() {
        return;
    }
    play_nonblocking(clip_path(&get_paths().login));
}

pub fn play_logout() {
    if !sound_enabled() {
        return;
    }
    play_nonblocking(clip_path(&get_paths().logout));
}

pub fn play_error() {
    if !sound_enabled() {
        return;
    }
    play_nonblocking(clip_path(&get_paths().error));
}

pub fn play_startup() {
    // Warm up sound paths so first audible event has no cold-start delay.
    if !sound_enabled() {
        return;
    }
    let _ = get_paths();
}

#[allow(dead_code)]
pub fn play_keypress() {
    if !sound_enabled() {
        return;
    }
    if !passes_gap(&LAST_KEYPRESS_MS, KEYPRESS_GAP_MS) {
        return;
    }
    play_nonblocking(clip_path(&get_paths().keypress));
}
