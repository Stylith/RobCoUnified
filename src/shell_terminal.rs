use anyhow::Result;

use crate::launcher::with_suspended;
use crate::ui::Term;

/// Launch an embedded shell by temporarily suspending the TUI.
/// Uses the system shell and resumes the TUI on exit.
/// For a full in-TUI PTY we'd use `portable-pty`; this keeps dependencies lean.
pub fn embedded_terminal(terminal: &mut Term) -> Result<()> {
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".into());
    with_suspended(terminal, || {
        std::process::Command::new(&shell)
            .env("PS1", "> ")
            .status()?;
        Ok(())
    })
}
