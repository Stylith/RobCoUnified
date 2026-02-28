use anyhow::{anyhow, Result};
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone)]
pub struct TerminalLaunchPlan {
    pub program: String,
    pub args: Vec<String>,
    pub display: String,
}

fn sibling_tui_binary() -> PathBuf {
    let current = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("robcos-native"));
    let dir = current
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    #[cfg(windows)]
    let tui = dir.join("robcos.exe");
    #[cfg(not(windows))]
    let tui = dir.join("robcos");
    tui
}

fn shell_command_for_tui() -> String {
    let tui = sibling_tui_binary();
    format!(
        "cd '{}' && '{}'",
        tui.parent().unwrap_or(Path::new(".")).display(),
        tui.display()
    )
}

fn launch_plan_for_command(command: String, display: String) -> TerminalLaunchPlan {
    #[cfg(target_os = "macos")]
    {
        TerminalLaunchPlan {
            program: "osascript".to_string(),
            args: vec![
                "-e".to_string(),
                format!(
                    "tell application \"Terminal\" to do script \"{}\"",
                    command.replace('\\', "\\\\").replace('\"', "\\\"")
                ),
            ],
            display,
        }
    }
    #[cfg(target_os = "windows")]
    {
        TerminalLaunchPlan {
            program: "cmd".to_string(),
            args: vec![
                "/C".to_string(),
                "start".to_string(),
                "cmd".to_string(),
                "/K".to_string(),
                command,
            ],
            display,
        }
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        let terminals = [
            (
                "x-terminal-emulator",
                vec!["-e".to_string(), command.clone()],
            ),
            (
                "gnome-terminal",
                vec![
                    "--".to_string(),
                    "sh".to_string(),
                    "-lc".to_string(),
                    command.clone(),
                ],
            ),
            (
                "konsole",
                vec![
                    "-e".to_string(),
                    "sh".to_string(),
                    "-lc".to_string(),
                    command.clone(),
                ],
            ),
            ("xfce4-terminal", vec!["-e".to_string(), command.clone()]),
            (
                "kitty",
                vec!["sh".to_string(), "-lc".to_string(), command.clone()],
            ),
        ];
        for (program, args) in terminals {
            if which(program) {
                return TerminalLaunchPlan {
                    program: program.to_string(),
                    args,
                    display,
                };
            }
        }
        TerminalLaunchPlan {
            program: "sh".to_string(),
            args: vec!["-lc".to_string(), command],
            display,
        }
    }
}

pub fn launch_plan() -> TerminalLaunchPlan {
    launch_plan_for_command(shell_command_for_tui(), "terminal -> robcos".to_string())
}

pub fn launch_terminal_mode() -> Result<TerminalLaunchPlan> {
    let plan = launch_plan();
    let status = Command::new(&plan.program).args(&plan.args).spawn();
    match status {
        Ok(_) => Ok(plan),
        Err(err) => Err(anyhow!("launch failed via {}: {err}", plan.program)),
    }
}

#[cfg(all(unix, not(target_os = "macos")))]
fn which(program: &str) -> bool {
    let Some(path_var) = std::env::var_os("PATH") else {
        return false;
    };
    std::env::split_paths(&path_var).any(|dir| dir.join(program).is_file())
}
