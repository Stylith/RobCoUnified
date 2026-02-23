/// RobcOS Sound System
///
/// Drop your audio files into `src/sounds/`, then swap `None` with
/// `Some(include_bytes!("sounds/yourfile.wav"))` for each slot.
/// Files are compiled directly into the binary.
/// Supported: WAV, MP3, OGG/Vorbis.

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
    /// Arrow keys, Enter, Space, Q, Tab
    Navigate,
    /// Text input keypresses
    Keypress,
}

/// Swap `None` → `Some(include_bytes!("sounds/yourfile.ext"))` to enable a slot.
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

// ── Playback ──────────────────────────────────────────────────────────────────

/// Play a sound on a background thread. No-op if sound is disabled or not embedded.
pub fn play(sound: Sound) {
    if !get_settings().sound {
        return;
    }
    if let Some(bytes) = bytes_for(sound) {
        std::thread::spawn(move || {
            let _ = play_bytes(bytes);
        });
    }
}

fn play_bytes(bytes: &'static [u8]) -> anyhow::Result<()> {
    // Keep _stream alive for the full duration — dropping it stops audio.
    let (_stream, stream_handle) = OutputStream::try_default()?;
    let sink = Sink::try_new(&stream_handle)?;

    // UniformSourceIterator resamples to the device's native sample rate,
    // fixing the sped-up audio that occurs when sample rates don't match.
    // We decode first to get the file's sample rate and channel count,
    // then wrap in UniformSourceIterator targeting 44100 Hz stereo.
    let decoder = Decoder::new(Cursor::new(bytes))?;
    let sample_rate = decoder.sample_rate();
    let channels    = decoder.channels();
    let resampled: rodio::source::UniformSourceIterator<_, f32> =
        rodio::source::UniformSourceIterator::new(decoder, channels, sample_rate);
    sink.append(resampled);
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
