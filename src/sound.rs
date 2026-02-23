/// RobcOS Sound System
///
/// Drop your audio files into `src/sounds/`, then swap `None` with
/// `Some(include_bytes!("sounds/yourfile.wav"))` for each slot.
/// Files are compiled directly into the binary.
/// Supported: WAV, MP3, OGG/Vorbis.

use cpal::traits::{DeviceTrait, HostTrait};
use rodio::{Decoder, OutputStream, Sink, Source};
use std::io::Cursor;

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

// ── Device sample rate detection ──────────────────────────────────────────────

/// Query the default output device's preferred sample rate.
/// Falls back to 44100 if unavailable.
fn device_sample_rate() -> u32 {
    let host   = cpal::default_host();
    let device = match host.default_output_device() {
        Some(d) => d,
        None    => return 44100,
    };
    let config = match device.default_output_config() {
        Ok(c) => c,
        Err(_) => return 44100,
    };
    config.sample_rate().0
}

// ── Playback ──────────────────────────────────────────────────────────────────

pub fn play(sound: Sound) {
    if !get_settings().sound { return; }
    if let Some(bytes) = bytes_for(sound) {
        std::thread::spawn(move || { let _ = play_bytes(bytes); });
    }
}

fn play_bytes(bytes: &'static [u8]) -> anyhow::Result<()> {
    let (_stream, stream_handle) = OutputStream::try_default()?;
    let sink = Sink::try_new(&stream_handle)?;

    let decoder     = Decoder::new(Cursor::new(bytes))?;
    let file_rate   = decoder.sample_rate();
    let device_rate = device_sample_rate();

    if file_rate != device_rate {
        // Explicitly correct the playback speed so the audio isn't pitched up/down.
        // speed() factor: < 1.0 slows down, > 1.0 speeds up.
        // We want to play file_rate samples at device_rate, so:
        //   factor = file_rate / device_rate
        // e.g. file=22050, device=44100 → factor=0.5 → plays at correct pitch.
        let factor = file_rate as f32 / device_rate as f32;
        sink.append(decoder.speed(factor));
    } else {
        sink.append(decoder);
    }

    sink.sleep_until_end();
    Ok(())
}

// ── Convenience wrappers ──────────────────────────────────────────────────────

pub fn play_startup()  { play(Sound::Startup);  }
pub fn play_login()    { play(Sound::Login);     }
pub fn play_logout()   { play(Sound::Logout);    }
pub fn play_error()    { play(Sound::Error);     }
pub fn play_navigate() { play(Sound::Navigate);  }

#[allow(dead_code)]
pub fn play_keypress() { play(Sound::Keypress);  }
