use anyhow::Result;
use crossterm::{
    event::{poll, read},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::io::stdout;
use std::process::Command;
use std::time::{Duration, Instant};

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
