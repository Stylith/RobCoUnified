use anyhow::Result;
use std::path::Path;

use crate::ui::Term;

/// Launch an embedded shell in a PTY inside the TUI.
/// This keeps global shortcuts (session switching) available while the shell runs.
pub fn embedded_terminal(terminal: &mut Term) -> Result<()> {
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".into());
    let shell_name = Path::new(&shell)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("");

    let args: &[&str] = match shell_name {
        "bash" => &["--noprofile", "--norc"],
        "zsh" => &["-f"],
        _ => &[],
    };

    let options = crate::pty::PtyLaunchOptions {
        env: vec![
            ("PS1".into(), "> ".into()),
            ("PROMPT".into(), "> ".into()),
            ("ZDOTDIR".into(), "/dev/null".into()),
        ],
        top_bar: Some("ROBCO MAINTENANCE TERMLINK".into()),
    };

    crate::pty::run_pty_session_with_options(terminal, &shell, args, options)
}
