use anyhow::Result;
use crossterm::{
    event::{KeyboardEnhancementFlags, PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags},
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
mod desktop;
mod docedit;
mod documents;
mod hacking;
mod installer;
mod launcher;
mod nuke_codes;
mod pty;
mod session;
mod settings;
mod shell_terminal;
mod sound;
mod status;
mod ui;

use auth::{clear_session, ensure_default_admin, login_screen};
use checks::{print_preflight, run_preflight};
use config::{get_settings, set_current_user, OpenMode};
use ui::{flash_message, run_menu, MenuResult, Term};

fn apply_pending_switch() {
    if let Some(target) = session::take_switch_request() {
        let count = session::session_count();
        if target < count {
            session::set_active(target);
        } else if target == count && count < session::MAX_SESSIONS {
            // Open a new session for the current user only.
            if let Some(current_user) = session::active_username() {
                let idx = session::push_session(&current_user);
                session::set_active(idx);
            }
        }
        // Out-of-range target: ignore, current session resumes.
    }
}

fn write_key_debug_startup_marker() {
    let path = std::env::var_os("ROBCOS_KEY_DEBUG_PATH")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::path::PathBuf::from("/tmp/robcos_keys.log"));

    let mut f = match std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
    {
        Ok(f) => f,
        Err(_) => match std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open("robcos_keys.log")
        {
            Ok(f) => f,
            Err(_) => return,
        },
    };

    use std::io::Write;
    let _ = writeln!(
        f,
        "--- app startup pid={} cwd={} debug_path={} ---",
        std::process::id(),
        std::env::current_dir()
            .ok()
            .and_then(|p| p.into_os_string().into_string().ok())
            .unwrap_or_else(|| "<unknown>".into()),
        path.display()
    );
}

// ── Terminal setup / teardown ─────────────────────────────────────────────────

fn init_terminal() -> Result<Term> {
    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen)?;
    // Best effort: this helps terminals disambiguate Ctrl+number combinations.
    let _ = execute!(
        stdout,
        PushKeyboardEnhancementFlags(
            KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES
                | KeyboardEnhancementFlags::REPORT_ALL_KEYS_AS_ESCAPE_CODES
        )
    );
    let backend = CrosstermBackend::new(stdout);
    Ok(ratatui::Terminal::new(backend)?)
}

fn restore_terminal(terminal: &mut Term) -> Result<()> {
    disable_raw_mode()?;
    let _ = execute!(terminal.backend_mut(), PopKeyboardEnhancementFlags);
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}

// ── Main application loop ─────────────────────────────────────────────────────

fn run(terminal: &mut Term, show_bootup: bool) -> Result<()> {
    config::reload_settings();

    if get_settings().bootup && show_bootup {
        sound::play_startup();
        boot::bootup(terminal)?;
    }

    'main: loop {
        // ── Ensure at least one active session ───────────────────────────────
        if session::session_count() == 0 {
            // Clear any stale switch request before showing login
            session::take_switch_request();

            match login_screen(terminal)? {
                Some(u) => {
                    let idx = session::push_session(&u);
                    session::set_active(idx);
                    set_current_user(Some(&u));
                }
                None => {
                    // None could mean: user chose Exit, or a switch key was
                    // pressed while on the login screen. If a switch request
                    // was registered, ignore the exit and loop back.
                    if session::take_switch_request().is_some() {
                        continue 'main;
                    }
                    sound::play_logout();
                    std::thread::sleep(std::time::Duration::from_millis(450));
                    break 'main;
                }
            }
        }

        // ── Activate the correct user ─────────────────────────────────────────
        let username = match session::active_username() {
            Some(u) => u,
            None => {
                session::set_active(0);
                continue 'main;
            }
        };
        set_current_user(Some(&username));

        // If this session has a suspended PTY command (e.g. vim), resume it
        // immediately so switching behaves like tmux-style windows.
        if pty::has_suspended_for_active() {
            pty::resume_suspended_for_active(terminal)?;
            if session::has_switch_request() {
                apply_pending_switch();
                continue 'main;
            }
        }

        // ── Session main menu loop ────────────────────────────────────────────
        let mut logged_out = false;
        let mut launch_default_desktop =
            matches!(get_settings().default_open_mode, OpenMode::Desktop);

        'menu: loop {
            if launch_default_desktop {
                launch_default_desktop = false;
                match desktop::desktop_mode(terminal, &username)? {
                    desktop::DesktopExit::ReturnToTerminal => {}
                    desktop::DesktopExit::Logout => {
                        sound::play_logout();
                        set_current_user(None);
                        clear_session();
                        flash_message(terminal, "Logging out...", 800)?;
                        logged_out = true;
                        break 'menu;
                    }
                    desktop::DesktopExit::Shutdown => {
                        return Ok(());
                    }
                }
            }

            if session::has_switch_request() {
                break 'menu;
            }

            let result = run_menu(
                terminal,
                "Main Menu",
                &[
                    "Applications",
                    "Documents",
                    "Network",
                    "Games",
                    "Program Installer",
                    "Terminal",
                    "Desktop Mode",
                    "---",
                    "Settings",
                    "Logout",
                ],
                Some("RobcOS v0.1.0"),
            )?;

            match result {
                MenuResult::Back => {
                    if session::has_switch_request() {
                        break 'menu;
                    }
                }
                MenuResult::Selected(s) => match s.as_str() {
                    "Applications" => apps::apps_menu(terminal)?,
                    "Documents" => documents::documents_menu(terminal)?,
                    "Network" => apps::network_menu(terminal)?,
                    "Games" => apps::games_menu(terminal)?,
                    "Program Installer" => installer::appstore_menu(terminal)?,
                    "Terminal" => shell_terminal::embedded_terminal(terminal)?,
                    "Desktop Mode" => match desktop::desktop_mode(terminal, &username)? {
                        desktop::DesktopExit::ReturnToTerminal => {}
                        desktop::DesktopExit::Logout => {
                            sound::play_logout();
                            set_current_user(None);
                            clear_session();
                            flash_message(terminal, "Logging out...", 800)?;
                            logged_out = true;
                            break 'menu;
                        }
                        desktop::DesktopExit::Shutdown => {
                            return Ok(());
                        }
                    },
                    "Settings" => settings::settings_menu(terminal, &username)?,
                    "Logout" => {
                        sound::play_logout();
                        set_current_user(None);
                        clear_session();
                        flash_message(terminal, "Logging out...", 800)?;
                        logged_out = true;
                        break 'menu;
                    }
                    _ => {}
                },
            }
        }

        // ── Handle logout ─────────────────────────────────────────────────────
        if logged_out {
            pty::clear_all_suspended();
            session::clear_sessions();
            session::take_switch_request();
            continue 'main; // always return to login
        }

        // ── Handle session switch ─────────────────────────────────────────────
        apply_pending_switch();
    }

    Ok(())
}

// ── Entry point ───────────────────────────────────────────────────────────────

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let no_preflight = args.contains(&"--no-preflight".to_string());
    write_key_debug_startup_marker();

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

    ensure_default_admin();

    let mut terminal = init_terminal()?;

    let result =
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| run(&mut terminal, true)));

    sound::stop_audio();
    std::thread::sleep(std::time::Duration::from_millis(50));
    restore_terminal(&mut terminal).ok();
    print!(
        "{}",
        crossterm::terminal::Clear(crossterm::terminal::ClearType::All)
    );

    match result {
        Ok(Ok(())) => Ok(()),
        Ok(Err(e)) => Err(e),
        Err(_) => {
            eprintln!("RobcOS crashed. Check /tmp/robcos_error.log");
            Ok(())
        }
    }
}
