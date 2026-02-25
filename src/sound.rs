//! RobcOS Sound System
//!
//! Fire-and-forget playback using OS audio players.
//! Boot key clips are preprocessed to remove leading dead-space and keep
//! a short click window, which reduces random perceived gaps.

use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};
use std::time::{SystemTime, UNIX_EPOCH};

use rand::seq::SliceRandom;

use crate::config::get_settings;

static STOPPED: AtomicBool = AtomicBool::new(false);
static ACTIVE: AtomicUsize = AtomicUsize::new(0);
const MAX_CONCURRENT: usize = 6;

static LAST_NAVIGATE_MS: AtomicU64 = AtomicU64::new(0);
static LAST_NAV_REPEAT_MS: AtomicU64 = AtomicU64::new(0);
static LAST_KEYPRESS_MS: AtomicU64 = AtomicU64::new(0);
static LAST_BOOT_KEY_MS: AtomicU64 = AtomicU64::new(0);
static BOOT_SEQ_IDX: AtomicUsize = AtomicUsize::new(0);

const NAVIGATE_GAP_MS: u64 = 0;
const NAVIGATE_REPEAT_GAP_MS: u64 = 75;
const KEYPRESS_GAP_MS: u64 = 16;
const BOOT_KEY_GAP_MS: u64 = 0;

struct PythonHelper {
    child: Child,
    stdin: ChildStdin,
}

static PY_HELPER: OnceLock<Mutex<Option<PythonHelper>>> = OnceLock::new();
static PY_HELPER_READY: AtomicBool = AtomicBool::new(false);
static PY_HELPER_USABLE: AtomicBool = AtomicBool::new(false);

pub fn stop_audio() {
    STOPPED.store(true, Ordering::SeqCst);
    if let Some(lock) = PY_HELPER.get() {
        if let Ok(mut guard) = lock.lock() {
            if let Some(mut helper) = guard.take() {
                let _ = helper.child.kill();
                let _ = helper.child.wait();
            }
        }
    }
}

struct SoundPaths {
    login: PathBuf,
    logout: PathBuf,
    error: PathBuf,
    navigate: PathBuf,
    keypress: PathBuf,
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

fn write_temp_boot_key(name: &str, bytes: &[u8]) -> PathBuf {
    let processed = extract_boot_click_window_pcm16_mono(bytes, 550, 28, 2100);
    write_temp(name, &processed)
}

fn get_paths() -> &'static SoundPaths {
    PATHS.get_or_init(|| SoundPaths {
        login: write_temp("login", include_bytes!("sounds/ui_hacking_passgood.wav")),
        logout: write_temp("logout", include_bytes!("sounds/ui_hacking_passbad.wav")),
        error: write_temp("error", include_bytes!("sounds/ui_hacking_passbad.wav")),
        navigate: write_temp(
            "navigate",
            include_bytes!("sounds/ui_hacking_charenter_01.wav"),
        ),
        keypress: write_temp(
            "keypress",
            include_bytes!("sounds/ui_hacking_charscroll.wav"),
        ),
        boot_keys: vec![
            write_temp_boot_key(
                "boot0",
                include_bytes!("sounds/ui_hacking_charsingle_01.wav"),
            ),
            write_temp_boot_key(
                "boot1",
                include_bytes!("sounds/ui_hacking_charsingle_02.wav"),
            ),
            write_temp_boot_key(
                "boot2",
                include_bytes!("sounds/ui_hacking_charsingle_03.wav"),
            ),
            write_temp_boot_key(
                "boot3",
                include_bytes!("sounds/ui_hacking_charsingle_04.wav"),
            ),
            write_temp_boot_key(
                "boot4",
                include_bytes!("sounds/ui_hacking_charsingle_05.wav"),
            ),
        ],
    })
}

fn run_spawn(path: PathBuf) {
    #[cfg(target_os = "macos")]
    let child = Command::new("afplay").arg(&path).spawn();

    #[cfg(target_os = "linux")]
    let child = Command::new("aplay")
        .arg("-q")
        .arg(&path)
        .spawn()
        .or_else(|_| Command::new("paplay").arg(&path).spawn());

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

fn helper_lock() -> &'static Mutex<Option<PythonHelper>> {
    PY_HELPER.get_or_init(|| Mutex::new(None))
}

fn ensure_python_helper() {
    let Ok(mut guard) = helper_lock().lock() else {
        return;
    };
    if guard.is_some() {
        return;
    }

    let script = r#"import sys
try:
    from playsound import playsound
    backend_ok = True
except Exception:
    playsound = None
    backend_ok = False

if backend_ok:
    sys.stdout.write("__ROBCOS_READY__\n")
else:
    sys.stdout.write("__ROBCOS_NOBACKEND__\n")
sys.stdout.flush()

for line in sys.stdin:
    path = line.rstrip("\r\n")
    if not path or not backend_ok:
        continue
    try:
        playsound(path, False)
    except Exception:
        pass
"#;

    PY_HELPER_READY.store(false, Ordering::Release);
    PY_HELPER_USABLE.store(false, Ordering::Release);

    let spawn = Command::new("python3")
        .args(["-u", "-c", script])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn();

    let Ok(mut child) = spawn else {
        return;
    };
    let Some(stdin) = child.stdin.take() else {
        let _ = child.kill();
        let _ = child.wait();
        return;
    };

    if let Some(stdout) = child.stdout.take() {
        std::thread::spawn(move || {
            let mut reader = BufReader::new(stdout);
            let mut line = String::new();
            if reader.read_line(&mut line).is_ok() {
                let msg = line.trim();
                if msg == "__ROBCOS_READY__" {
                    PY_HELPER_USABLE.store(true, Ordering::Release);
                }
                if msg == "__ROBCOS_READY__" || msg == "__ROBCOS_NOBACKEND__" {
                    PY_HELPER_READY.store(true, Ordering::Release);
                }
            }
        });
    }

    *guard = Some(PythonHelper { child, stdin });
}

fn play_via_python(path: &std::path::Path) -> bool {
    ensure_python_helper();
    if !PY_HELPER_READY.load(Ordering::Acquire) || !PY_HELPER_USABLE.load(Ordering::Acquire) {
        return false;
    }
    let Ok(mut guard) = helper_lock().lock() else {
        return false;
    };
    let Some(helper) = guard.as_mut() else {
        return false;
    };

    let line = format!("{}\n", path.display());
    if helper.stdin.write_all(line.as_bytes()).is_ok() && helper.stdin.flush().is_ok() {
        return true;
    }

    // If helper died, drop it and fall back to native spawn for this event.
    if let Some(mut dead) = guard.take() {
        let _ = dead.child.kill();
        let _ = dead.child.wait();
    }
    false
}

fn has_helper_process() -> bool {
    helper_lock().lock().map(|g| g.is_some()).unwrap_or(false)
}

pub fn wait_boot_audio_ready(timeout_ms: u64) {
    if !get_settings().sound {
        return;
    }

    let _ = get_paths();
    ensure_python_helper();

    if !has_helper_process() {
        // Fallback to previous behavior when python helper is unavailable.
        std::thread::sleep(Duration::from_millis(180));
        return;
    }

    let start = Instant::now();
    while start.elapsed() < Duration::from_millis(timeout_ms) {
        if PY_HELPER_READY.load(Ordering::Acquire) {
            break;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
}

fn play_nonblocking(path: PathBuf) {
    if STOPPED.load(Ordering::SeqCst) {
        return;
    }

    if play_via_python(&path) {
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
    if !get_settings().sound {
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
    play_nonblocking(paths[idx].clone());
}

pub fn play_navigate() {
    if !get_settings().sound {
        return;
    }
    if !passes_gap(&LAST_NAVIGATE_MS, NAVIGATE_GAP_MS) {
        return;
    }
    play_nonblocking(get_paths().navigate.clone());
}

pub fn play_navigate_repeat() {
    if !get_settings().sound {
        return;
    }
    if !passes_gap(&LAST_NAV_REPEAT_MS, NAVIGATE_REPEAT_GAP_MS) {
        return;
    }
    play_nonblocking(get_paths().navigate.clone());
}

pub fn play_login() {
    if !get_settings().sound {
        return;
    }
    play_nonblocking(get_paths().login.clone());
}

pub fn play_logout() {
    if !get_settings().sound {
        return;
    }
    play_nonblocking(get_paths().logout.clone());
}

pub fn play_error() {
    if !get_settings().sound {
        return;
    }
    play_nonblocking(get_paths().error.clone());
}

pub fn play_startup() {
    // Warm up sound paths + helper once so first audible event has no cold-start delay.
    if !get_settings().sound {
        return;
    }
    let _ = get_paths();
    ensure_python_helper();
}

#[allow(dead_code)]
pub fn play_keypress() {
    if !get_settings().sound {
        return;
    }
    if !passes_gap(&LAST_KEYPRESS_MS, KEYPRESS_GAP_MS) {
        return;
    }
    play_nonblocking(get_paths().keypress.clone());
}
