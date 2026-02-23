use anyhow::Result;
use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::backend::CrosstermBackend;
use std::io::stdout;

mod apps;
mod auth;
mod boot;
mod checks;
mod config;
mod docedit;
mod documents;
mod hacking;
mod installer;
mod launcher;
mod settings;
mod status;
mod shell_terminal;
mod sound;
mod ui;

use auth::{ensure_default_admin, login_screen, clear_session};
use checks::{run_preflight, print_preflight};
use config::{set_current_user, get_settings};
use ui::{Term, run_menu, flash_message, MenuResult};

// ── Terminal setup / teardown ─────────────────────────────────────────────────

fn init_terminal() -> Result<Term> {
    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    Ok(ratatui::Terminal::new(backend)?)
}

fn restore_terminal(terminal: &mut Term) -> Result<()> {
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}

// ── Main application loop ─────────────────────────────────────────────────────

fn run(terminal: &mut Term, show_bootup: bool) -> Result<()> {
    config::reload_settings();

    // Boot animation
    if get_settings().bootup && show_bootup {
        sound::play_startup();
        boot::bootup(terminal)?;
    }

    // Outer loop: login → main menu → logout → login again
    'login_loop: loop {
        // Login screen; None means user chose Exit
        let username = match login_screen(terminal)? {
            Some(u) => u,
            None    => {
                sound::play_logout();
                break 'login_loop;
            }
        };
        set_current_user(Some(&username));

        // Inner loop: main menu
        loop {
            let result = run_menu(
                terminal,
                "Main Menu",
                &[
                    "Applications", "Documents", "Network", "Games",
                    "Program Installer", "Terminal",
                    "---",
                    "Settings", "Logout",
                ],
                Some("RobcOS v0.1.0"),
            )?;

            match result {
                MenuResult::Back => {
                    // treat Back on main menu as nothing
                }
                MenuResult::Selected(s) => match s.as_str() {
                    "Applications"       => apps::apps_menu(terminal)?,
                    "Documents"         => documents::documents_menu(terminal)?,
                    "Network"           => apps::network_menu(terminal)?,
                    "Games"             => apps::games_menu(terminal)?,
                    "Program Installer" => installer::appstore_menu(terminal)?,
                    "Terminal"          => shell_terminal::embedded_terminal(terminal)?,
                    "Settings"          => settings::settings_menu(terminal, &username)?,
                    "Logout"            => {
                        sound::play_logout();
                        set_current_user(None);
                        clear_session();
                        flash_message(terminal, "Logging out...", 800)?;
                        continue 'login_loop;
                    }
                    _ => {}
                }
            }
        }
    }

    Ok(())
}

// ── Entry point ───────────────────────────────────────────────────────────────

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let no_preflight = args.contains(&"--no-preflight".to_string());

    // Preflight dependency check
    if !no_preflight {
        let report = run_preflight();
        if !report.ok {
            print_preflight(&report);
            eprintln!("\nPlease install missing dependencies and try again.");
            eprintln!("Run with --no-preflight to skip this check.");
            std::process::exit(1);
        }
        if !report.warnings.is_empty() {
            print_preflight(&report);
            eprintln!("\nSome optional features will be unavailable.");
            eprintln!("Press Enter to continue...");
            let mut buf = String::new();
            std::io::stdin().read_line(&mut buf)?;
        }
    }

    // Bootstrap default admin user if no users exist
    ensure_default_admin();

    let mut terminal = init_terminal()?;

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        run(&mut terminal, true)
    }));

    // Always restore terminal
    restore_terminal(&mut terminal).ok();
    print!("{}", crossterm::terminal::Clear(crossterm::terminal::ClearType::All));

    match result {
        Ok(Ok(())) => Ok(()),
        Ok(Err(e)) => Err(e),
        Err(_)     => {
            eprintln!("RobcOS crashed. Check /tmp/robcos_error.log");
            Ok(())
        }
    }
}
