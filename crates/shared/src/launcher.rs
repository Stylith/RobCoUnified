use anyhow::Result;
use crossterm::{
    event::{poll, read},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::io::stdout;
use std::process::Command;
use std::time::{Duration, Instant};

use crate::config::{get_settings, update_settings};
use crate::ui::Term;

/// Suspend the TUI, run a closure in normal terminal mode, then resume.
pub fn with_suspended<F: FnOnce() -> Result<()>>(terminal: &mut Term, f: F) -> Result<()> {
    // Give a small buffer so the TUI render finishes
    std::thread::sleep(std::time::Duration::from_millis(80));
    disable_raw_mode()?;
    execute!(stdout(), LeaveAlternateScreen)?;

    let result = f();

    enable_raw_mode()?;
    execute!(stdout(), EnterAlternateScreen)?;
    terminal.clear()?;
    drain_pending_input(Duration::from_millis(80));

    result
}

fn drain_pending_input(max_for: Duration) {
    let deadline = Instant::now() + max_for;
    while Instant::now() < deadline {
        match poll(Duration::from_millis(0)) {
            Ok(true) => {
                let _ = read();
            }
            _ => break,
        }
    }
}

const FAST_EXIT_RETRY_MS: u64 = 300;

pub fn fast_exit_retry_window() -> Duration {
    Duration::from_millis(FAST_EXIT_RETRY_MS)
}

pub fn should_probe_fast_exit(cmd: &[String]) -> bool {
    cmd.first()
        .is_some_and(|program| !program.is_empty() && !program.contains('/'))
        && cmd.len() <= 1
}

pub fn should_retry_with_shell_after_fast_exit(cmd: &[String], elapsed: Duration) -> bool {
    should_probe_fast_exit(cmd) && elapsed <= fast_exit_retry_window()
}

fn command_launch_key(cmd: &[String]) -> Option<String> {
    let program = cmd.first()?;
    let base = std::path::Path::new(program)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or(program)
        .trim();
    if base.is_empty() {
        None
    } else {
        Some(base.to_ascii_lowercase())
    }
}

pub fn is_shell_preferred(cmd: &[String]) -> bool {
    let Some(key) = command_launch_key(cmd) else {
        return false;
    };
    get_settings()
        .pty_shell_preferred
        .get(&key)
        .copied()
        .unwrap_or(false)
}

pub fn remember_shell_preferred(cmd: &[String]) {
    let Some(key) = command_launch_key(cmd) else {
        return;
    };
    let mut changed = false;
    update_settings(|s| {
        let prev = s.pty_shell_preferred.insert(key.clone(), true);
        changed = prev != Some(true);
    });
    if changed {
        #[cfg(not(test))]
        crate::config::persist_settings();
    }
}

pub fn command_exists(name: &str) -> bool {
    if name.is_empty() {
        return false;
    }
    if name.contains('/') {
        return std::path::Path::new(name).is_file();
    }
    std::env::var_os("PATH")
        .is_some_and(|path| std::env::split_paths(&path).any(|dir| dir.join(name).is_file()))
}

pub fn normalize_command_aliases(cmd: &[String]) -> Vec<String> {
    if cmd.is_empty() {
        return Vec::new();
    }
    let mut out = cmd.to_vec();
    let program = out[0].clone();
    if program.contains('/') || command_exists(&program) {
        return out;
    }
    if program.contains('-') {
        let alt = program.replace('-', "_");
        if command_exists(&alt) {
            out[0] = alt;
            return out;
        }
    }
    if program.contains('_') {
        let alt = program.replace('_', "-");
        if command_exists(&alt) {
            out[0] = alt;
            return out;
        }
    }
    out
}

pub fn build_shell_fallback_command(cmd: &[String]) -> Option<Vec<String>> {
    if cmd.is_empty() || cmd[0].contains('/') {
        return None;
    }
    let shell = std::env::var("SHELL")
        .ok()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "/bin/sh".to_string());
    let line = cmd
        .iter()
        .map(|part| shell_quote(part))
        .collect::<Vec<_>>()
        .join(" ");
    Some(vec![shell, "-ic".to_string(), line])
}

fn shell_quote(value: &str) -> String {
    if value.is_empty() {
        return "''".to_string();
    }
    if value
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || "-_./:=".contains(ch))
    {
        return value.to_string();
    }
    format!("'{}'", value.replace('\'', "'\\''"))
}

/// Launch a command in a PTY session inside the TUI (future use).
#[allow(dead_code)]
pub fn launch_in_pty(terminal: &mut Term, cmd: &[String]) -> Result<()> {
    crate::pty::launch_in_pty(terminal, cmd)
}

pub fn launch_argv(terminal: &mut Term, cmd: &[String]) -> Result<()> {
    if cmd.is_empty() {
        return Ok(());
    }
    with_suspended(terminal, || {
        Command::new(&cmd[0]).args(&cmd[1..]).status()?;
        Ok(())
    })
}

/// Parse a JSON array of strings into a Vec<String> command.
pub fn json_to_cmd(val: &serde_json::Value) -> Vec<String> {
    val.as_array()
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Settings;

    fn sv(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| (*s).to_string()).collect()
    }

    #[test]
    fn fast_exit_probe_rules() {
        assert!(should_probe_fast_exit(&sv(&["spotify-player"])));
        assert!(!should_probe_fast_exit(&sv(&["/usr/bin/spotify-player"])));
        assert!(!should_probe_fast_exit(&sv(&["python3", "script.py"])));
        assert!(!should_probe_fast_exit(&Vec::new()));
    }

    #[test]
    fn fast_exit_retry_window_rules() {
        let cmd = sv(&["spotify-player"]);
        assert!(should_retry_with_shell_after_fast_exit(
            &cmd,
            Duration::from_millis(50)
        ));
        assert!(!should_retry_with_shell_after_fast_exit(
            &cmd,
            Duration::from_secs(2)
        ));
    }

    #[test]
    fn shell_preference_roundtrip_uses_normalized_key() {
        update_settings(|s| *s = Settings::default());
        let cmd = sv(&["/usr/bin/Spotify-Player"]);
        assert!(!is_shell_preferred(&cmd));
        remember_shell_preferred(&cmd);
        assert!(is_shell_preferred(&cmd));
        let alias = sv(&["spotify-player"]);
        assert!(is_shell_preferred(&alias));
    }

    #[test]
    fn shell_fallback_builder_rejects_abs_program() {
        assert!(build_shell_fallback_command(&sv(&["/usr/bin/vim"])).is_none());
        let built = build_shell_fallback_command(&sv(&["vim", "file.txt"]));
        assert!(built.is_some());
    }
}
