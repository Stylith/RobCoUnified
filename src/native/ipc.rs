//! Lightweight Unix domain socket IPC for communication between the
//! RobCoOS shell and standalone apps.
//!
//! The shell starts a listener thread that accepts one-shot JSON messages.
//! Standalone apps connect, send a single [`IpcMessage`], and disconnect.
//! The shell polls received messages each frame via an mpsc channel.

use crate::config::base_dir;
use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;
use std::sync::mpsc;

/// Messages that standalone apps can send to the shell.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum IpcMessage {
    /// A standalone app saved settings — shell should reload from disk.
    SettingsChanged,
    /// Request the shell to open a file in the editor.
    OpenInEditor { path: String },
    /// Request the shell to reveal a path in the file manager.
    RevealInFileManager { path: String },
    /// Request the shell to open a specific settings panel.
    OpenSettings { panel: Option<String> },
    /// A standalone app is closing.
    AppClosed { app: String },
    /// Ping — used to check if the shell is running.
    Ping,
}

/// Handle for the shell to receive IPC messages.
pub struct IpcReceiver {
    rx: mpsc::Receiver<IpcMessage>,
    socket_path: PathBuf,
}

impl IpcReceiver {
    /// Drain all pending messages. Call once per frame.
    pub fn poll(&self) -> Vec<IpcMessage> {
        let mut msgs = Vec::new();
        while let Ok(msg) = self.rx.try_recv() {
            msgs.push(msg);
        }
        msgs
    }
}

impl Drop for IpcReceiver {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.socket_path);
    }
}

/// Returns the socket path under the RobCoOS data directory.
pub fn socket_path() -> PathBuf {
    base_dir().join("shell.sock")
}

/// Start the IPC listener on a background thread. Returns a receiver
/// that the shell polls each frame. Safe to call once at startup.
pub fn start_listener() -> IpcReceiver {
    let path = socket_path();

    // Remove stale socket from a previous run.
    let _ = std::fs::remove_file(&path);

    let listener = match UnixListener::bind(&path) {
        Ok(l) => l,
        Err(err) => {
            eprintln!("[ipc] failed to bind {}: {err}", path.display());
            let (_, rx) = mpsc::channel();
            return IpcReceiver {
                rx,
                socket_path: path,
            };
        }
    };

    let (tx, rx) = mpsc::channel();

    std::thread::spawn(move || {
        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    let tx = tx.clone();
                    // Handle each connection on a short-lived thread so
                    // a misbehaving client can't block the listener.
                    std::thread::spawn(move || {
                        handle_connection(stream, &tx);
                    });
                }
                Err(err) => {
                    eprintln!("[ipc] accept error: {err}");
                }
            }
        }
    });

    IpcReceiver {
        rx,
        socket_path: path,
    }
}

fn handle_connection(stream: UnixStream, tx: &mpsc::Sender<IpcMessage>) {
    let _ = stream.set_read_timeout(Some(std::time::Duration::from_secs(2)));
    let reader = BufReader::new(&stream);
    for line in reader.lines() {
        let Ok(line) = line else { break };
        let line = line.trim().to_string();
        if line.is_empty() {
            continue;
        }
        match serde_json::from_str::<IpcMessage>(&line) {
            Ok(msg) => {
                let _ = tx.send(msg);
            }
            Err(err) => {
                eprintln!("[ipc] bad message: {err}");
            }
        }
    }
}

// ── Client side (used by standalone apps) ────────────────────────────────────

/// Send a single IPC message to the running shell. Returns Ok(()) on
/// success, Err with description on failure. Non-blocking-ish — connects
/// with a 1-second timeout.
pub fn send_to_shell(msg: &IpcMessage) -> Result<(), String> {
    let path = socket_path();
    let mut stream =
        UnixStream::connect(&path).map_err(|e| format!("connect to shell: {e}"))?;
    stream
        .set_write_timeout(Some(std::time::Duration::from_secs(1)))
        .ok();
    let json = serde_json::to_string(msg).map_err(|e| format!("serialize: {e}"))?;
    stream
        .write_all(json.as_bytes())
        .map_err(|e| format!("write: {e}"))?;
    stream
        .write_all(b"\n")
        .map_err(|e| format!("write newline: {e}"))?;
    Ok(())
}

/// Check if the shell is running by attempting a Ping.
pub fn shell_is_running() -> bool {
    send_to_shell(&IpcMessage::Ping).is_ok()
}

/// Notify the shell that settings were changed on disk.
/// Called by standalone apps after persisting settings.
/// Silently ignored if the shell isn't running.
pub fn notify_settings_changed() {
    let _ = send_to_shell(&IpcMessage::SettingsChanged);
}

/// Ask the shell to open a file in the editor.
pub fn request_open_in_editor(path: &std::path::Path) {
    let _ = send_to_shell(&IpcMessage::OpenInEditor {
        path: path.display().to_string(),
    });
}

/// Ask the shell to reveal a path in the file manager.
pub fn request_reveal_in_file_manager(path: &std::path::Path) {
    let _ = send_to_shell(&IpcMessage::RevealInFileManager {
        path: path.display().to_string(),
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_ipc_message() {
        let msg = IpcMessage::OpenInEditor {
            path: "/tmp/test.txt".to_string(),
        };
        let json = serde_json::to_string(&msg).unwrap();
        let parsed: IpcMessage = serde_json::from_str(&json).unwrap();
        assert!(matches!(parsed, IpcMessage::OpenInEditor { path } if path == "/tmp/test.txt"));
    }

    #[test]
    fn settings_changed_serializes() {
        let msg = IpcMessage::SettingsChanged;
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("settings_changed"));
    }
}
