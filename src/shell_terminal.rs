use anyhow::Result;

use crate::ui::Term;

/// Launch an embedded shell in a PTY inside the TUI.
/// This keeps global shortcuts (session switching) available while the shell runs.
pub fn embedded_terminal(terminal: &mut Term) -> Result<()> {
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".into());
    crate::pty::run_pty_session(terminal, &shell, &[])
}
