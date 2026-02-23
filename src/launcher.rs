use anyhow::Result;
use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::io::stdout;
use std::path::Path;
use std::process::Command;

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

    result
}

pub fn launch_command(terminal: &mut Term, cmd: &[String]) -> Result<()> {
    if cmd.is_empty() { return Ok(()); }
    with_suspended(terminal, || {
        Command::new(&cmd[0]).args(&cmd[1..]).status()?;
        Ok(())
    })
}

pub fn launch_epy(terminal: &mut Term, path: &Path) -> Result<()> {
    with_suspended(terminal, || {
        Command::new("epy").arg(path).status()?;
        Ok(())
    })
}

/// Parse a JSON array of strings into a Vec<String> command.
pub fn json_to_cmd(val: &serde_json::Value) -> Vec<String> {
    val.as_array()
        .map(|a| a.iter().filter_map(|v| v.as_str().map(str::to_string)).collect())
        .unwrap_or_default()
}
