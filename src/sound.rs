/// RobcOS Sound System
///
/// Plays audio by writing embedded bytes to a temp file and handing it to
/// the OS native player — exactly like Python's playsound. No resampling,
/// no pitch issues. The OS handles everything.
///
/// macOS  → afplay
/// Linux  → aplay (WAV) or ffplay (MP3/OGG) or paplay
/// Windows → powershell Media.SoundPlayer
///
/// To add a sound: drop your file into src/sounds/, then swap `None` with
/// `Some(include_bytes!("sounds/yourfile.wav"))` for the matching slot.

use std::io::Write;
use std::process::Command;

use crate::config::get_settings;

// ── Sound catalogue ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sound {
    Startup,
    Login,
    Logout,
    Error,
    Navigate,
    Keypress,
}

fn bytes_for(sound: Sound) -> Option<&'static [u8]> {
    match sound {
        Sound::Startup  => None, // Some(include_bytes!("sounds/startup.wav")),
        Sound::Login    => Some(include_bytes!("sounds/ui_hacking_passgood.wav")),
        Sound::Logout   => Some(include_bytes!("sounds/ui_hacking_passbad.wav")),
        Sound::Error    => Some(include_bytes!("sounds/ui_hacking_passbad.wav")),
        Sound::Navigate => Some(include_bytes!("sounds/ui_hacking_charenter_01.wav")),
        Sound::Keypress => None, // Some(include_bytes!("sounds/keypress.wav")),
    }
}

// ── Extension helper ──────────────────────────────────────────────────────────

fn ext_for(sound: Sound) -> &'static str {
    // Match the extension to whatever file you embed above.
    // Change these if you use mp3/ogg instead of wav.
    match sound {
        Sound::Startup  => "wav",
        Sound::Login    => "wav",
        Sound::Logout   => "wav",
        Sound::Error    => "wav",
        Sound::Navigate => "wav",
        Sound::Keypress => "wav",
    }
}

// ── Playback ──────────────────────────────────────────────────────────────────

pub fn play(sound: Sound) {
    if !get_settings().sound { return; }
    if let Some(bytes) = bytes_for(sound) {
        let ext = ext_for(sound);
        std::thread::spawn(move || { let _ = play_bytes(bytes, ext); });
    }
}

fn play_bytes(bytes: &'static [u8], ext: &str) -> anyhow::Result<()> {
    // Write to a named temp file so the OS player can open it by path.
    let tmp = tempfile_path(ext);
    {
        let mut f = std::fs::File::create(&tmp)?;
        f.write_all(bytes)?;
    }

    play_file(&tmp);

    // Clean up after playback (best-effort).
    let _ = std::fs::remove_file(&tmp);
    Ok(())
}

fn tempfile_path(ext: &str) -> std::path::PathBuf {
    let id = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.subsec_nanos())
        .unwrap_or(0);
    std::env::temp_dir().join(format!("robcos_sfx_{id}.{ext}"))
}

fn play_file(path: &std::path::Path) {
    let path_str = path.to_string_lossy();

    #[cfg(target_os = "macos")]
    {
        // afplay is the same backend playsound uses on macOS.
        let _ = Command::new("afplay")
            .arg(path_str.as_ref())
            .output();
    }

    #[cfg(target_os = "linux")]
    {
        // Try aplay first (ALSA, WAV only), then paplay (PulseAudio), then ffplay.
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if ext == "wav" {
            if Command::new("aplay")
                .arg("-q")
                .arg(path_str.as_ref())
                .output()
                .is_err()
            {
                let _ = Command::new("paplay").arg(path_str.as_ref()).output();
            }
        } else {
            // MP3/OGG — use ffplay (silent, no window)
            let _ = Command::new("ffplay")
                .args(["-nodisp", "-autoexit", "-loglevel", "quiet"])
                .arg(path_str.as_ref())
                .output();
        }
    }

    #[cfg(target_os = "windows")]
    {
        // PowerShell SoundPlayer — same as playsound on Windows for WAV.
        let script = format!(
            "(New-Object Media.SoundPlayer '{}').PlaySync()",
            path_str.replace('\'', "''")
        );
        let _ = Command::new("powershell")
            .args(["-NoProfile", "-Command", &script])
            .output();
    }
}

// ── Convenience wrappers ──────────────────────────────────────────────────────

pub fn play_startup()  { play(Sound::Startup);  }
pub fn play_login()    { play(Sound::Login);     }
pub fn play_logout()   { play(Sound::Logout);    }
pub fn play_error()    { play(Sound::Error);     }
pub fn play_navigate() { play(Sound::Navigate);  }

#[allow(dead_code)]
pub fn play_keypress() { play(Sound::Keypress);  }
