/// PTY session — runs a child process in a pseudo-terminal and renders its
/// output inside the ratatui TUI using vt100 for terminal emulation.
///
/// The child process thinks it has a real terminal: correct size, SIGWINCH on
/// resize, readline/colors/cursor movement all work. Output is captured into a
/// vt100::Parser on a background reader thread and rendered each frame.
///
/// Usage:
///   run_pty_session(terminal, "/bin/bash", &[])
///   launch_in_pty(terminal, &["vim", "file.txt"])

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use portable_pty::{native_pty_system, CommandBuilder, PtySize};
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
};
use std::collections::HashMap;
use std::io::Write;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

use crate::status::render_status_bar;
use crate::ui::Term;

static SUSPENDED_PTY: OnceLock<Mutex<HashMap<usize, PtySession>>> = OnceLock::new();

fn suspended_pty_map() -> &'static Mutex<HashMap<usize, PtySession>> {
    SUSPENDED_PTY.get_or_init(|| Mutex::new(HashMap::new()))
}

fn park_active_session_pty(session: PtySession) {
    let idx = crate::session::active_idx();
    if let Ok(mut map) = suspended_pty_map().lock() {
        map.insert(idx, session);
    }
}

fn take_active_session_pty() -> Option<PtySession> {
    let idx = crate::session::active_idx();
    suspended_pty_map().lock().ok()?.remove(&idx)
}

pub fn has_suspended_for_active() -> bool {
    let idx = crate::session::active_idx();
    suspended_pty_map()
        .lock()
        .map(|map| map.contains_key(&idx))
        .unwrap_or(false)
}

pub fn clear_all_suspended() {
    if let Ok(mut map) = suspended_pty_map().lock() {
        map.clear();
    }
}

fn key_debug_path() -> std::path::PathBuf {
    std::env::var_os("ROBCOS_KEY_DEBUG_PATH")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::path::PathBuf::from("/tmp/robcos_keys.log"))
}

fn open_key_debug_file() -> Option<std::fs::File> {
    let primary = key_debug_path();
    match std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&primary)
    {
        Ok(f) => Some(f),
        Err(_) => match std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open("robcos_keys.log")
        {
            Ok(f) => Some(f),
            Err(_) => None,
        },
    }
}

fn append_marker_line(line: &str) {
    let Some(mut file) = open_key_debug_file() else { return };
    let _ = writeln!(file, "{line}");
}

fn append_key_debug_line(line: &str) {
    if std::env::var_os("ROBCOS_KEY_DEBUG").is_none() {
        return;
    }
    let Some(mut file) = open_key_debug_file() else { return };
    let _ = writeln!(file, "{line}");
}

fn init_key_debug_log() {
    append_marker_line(&format!(
        "--- pty session start pid={} path={} ---",
        std::process::id(),
        key_debug_path().display()
    ));
}

fn debug_log_key(code: KeyCode, mods: KeyModifiers, kind: KeyEventKind) {
    append_key_debug_line(&format!("kind={kind:?} code={code:?} mods={mods:?}"));
}

const TILDE_CHORD_WINDOW: Duration = Duration::from_millis(1200);

#[derive(Debug, Clone, Copy)]
enum TildeChordState {
    None,
    One(Instant),
    Two(Instant),
}

fn flush_tilde_state(state: &mut TildeChordState, session: &mut PtySession) {
    match *state {
        TildeChordState::None => {}
        TildeChordState::One(_) => session.write(b"~"),
        TildeChordState::Two(_) => session.write(b"~~"),
    }
    *state = TildeChordState::None;
}

fn try_tilde_session_chord(
    code: KeyCode,
    mods: KeyModifiers,
    state: &mut TildeChordState,
) -> bool {
    let plain_or_shift = mods.is_empty() || mods == KeyModifiers::SHIFT;
    let now = Instant::now();
    match code {
        KeyCode::Char('~') if plain_or_shift => {
            *state = match *state {
                TildeChordState::None => TildeChordState::One(now),
                TildeChordState::One(t) if now.duration_since(t) <= TILDE_CHORD_WINDOW => {
                    TildeChordState::Two(now)
                }
                _ => TildeChordState::One(now),
            };
            true
        }
        KeyCode::Char(c @ '1'..='9') if mods.is_empty() => {
            if let TildeChordState::Two(t) = *state {
                if now.duration_since(t) <= TILDE_CHORD_WINDOW {
                    *state = TildeChordState::None;
                    let idx = (c as usize) - ('1' as usize);
                    let count = crate::session::session_count();
                    if idx < count || (idx == count && count < crate::session::MAX_SESSIONS) {
                        crate::session::request_switch(idx);
                    }
                    return true;
                }
            }
            false
        }
        _ => false,
    }
}

fn maybe_flush_expired_tilde_state(state: &mut TildeChordState, session: &mut PtySession) {
    let now = Instant::now();
    let expired = match *state {
        TildeChordState::None => false,
        TildeChordState::One(t) | TildeChordState::Two(t) => {
            now.duration_since(t) > TILDE_CHORD_WINDOW
        }
    };
    if expired {
        flush_tilde_state(state, session);
    }
}

enum PtyLoopOutcome {
    ProcessExited,
    SuspendedForSwitch,
}

// ── PTY Session ───────────────────────────────────────────────────────────────

pub struct PtySession {
    /// Write end — send keyboard input to the child
    writer:  Box<dyn Write + Send>,
    /// Shared vt100 parser — updated by reader thread, read by render loop
    parser:  Arc<Mutex<vt100::Parser>>,
    /// Child handle — check if still alive
    child:   Box<dyn portable_pty::Child + Send + Sync>,
    /// Current PTY dimensions
    cols:    u16,
    rows:    u16,
    /// Master — kept alive so the PTY stays open; also used for resize
    master:  Box<dyn portable_pty::MasterPty + Send>,
}

impl PtySession {
    pub fn spawn(program: &str, args: &[&str], cols: u16, rows: u16) -> Result<Self> {
        let pty_system = native_pty_system();
        let pair = pty_system.openpty(PtySize {
            rows,
            cols,
            pixel_width:  0,
            pixel_height: 0,
        })?;

        let mut cmd = CommandBuilder::new(program);
        for arg in args { cmd.arg(arg); }

        let child  = pair.slave.spawn_command(cmd)?;
        let writer = pair.master.take_writer()?;
        let reader = pair.master.try_clone_reader()?;

        // vt100 parser — shared with reader thread
        let parser = Arc::new(Mutex::new(vt100::Parser::new(rows, cols, 0)));
        let parser_clone = Arc::clone(&parser);

        // Reader thread: pump PTY output into the vt100 parser continuously
        std::thread::Builder::new()
            .name("robcos-pty-reader".into())
            .spawn(move || {
                let mut reader = reader;
                let mut buf = [0u8; 4096];
                loop {
                    match std::io::Read::read(&mut reader, &mut buf) {
                        Ok(0) | Err(_) => break,
                        Ok(n) => {
                            if let Ok(mut p) = parser_clone.lock() {
                                p.process(&buf[..n]);
                            }
                        }
                    }
                }
            })?;

        Ok(Self {
            writer,
            parser,
            child,
            cols,
            rows,
            master: pair.master,
        })
    }

    /// Send raw bytes to the child's stdin (keyboard input)
    pub fn write(&mut self, data: &[u8]) {
        let _ = self.writer.write_all(data);
        let _ = self.writer.flush();
    }

    /// Resize the PTY and notify the child via SIGWINCH
    pub fn resize(&mut self, cols: u16, rows: u16) {
        if cols == self.cols && rows == self.rows { return; }
        self.cols = cols;
        self.rows = rows;
        let _ = self.master.resize(PtySize { rows, cols, pixel_width: 0, pixel_height: 0 });
        if let Ok(mut p) = self.parser.lock() {
            p.set_size(rows, cols);
        }
    }

    /// Is the child process still running?
    pub fn is_alive(&mut self) -> bool {
        matches!(self.child.try_wait(), Ok(None))
    }

    /// Render the current vt100 screen into `area` of the ratatui frame.
    pub fn render(&self, f: &mut ratatui::Frame, area: Rect) {
        let Ok(parser) = self.parser.lock() else { return };
        let screen = parser.screen();

        let rows = area.height as usize;
        let cols = area.width  as usize;

        let lines: Vec<Line> = (0..rows).map(|row| {
            let spans: Vec<Span> = (0..cols).map(|col| {
                let cell = screen.cell(row as u16, col as u16);
                let ch   = cell.map(|c| c.contents().to_string())
                               .filter(|s| !s.is_empty())
                               .unwrap_or_else(|| " ".to_string());

                let style = cell.map(|c| vt100_style(c)).unwrap_or_default();
                Span::styled(ch, style)
            }).collect();
            Line::from(spans)
        }).collect();

        f.render_widget(Paragraph::new(lines), area);
    }
}

// ── vt100 cell → ratatui Style ────────────────────────────────────────────────

fn vt100_style(cell: &vt100::Cell) -> Style {
    let mut style = Style::default();

    style = style.fg(vt100_color(cell.fgcolor(), Color::Reset));
    style = style.bg(vt100_color(cell.bgcolor(), Color::Reset));

    if cell.bold()       { style = style.add_modifier(Modifier::BOLD);          }
    if cell.italic()     { style = style.add_modifier(Modifier::ITALIC);         }
    if cell.underline()  { style = style.add_modifier(Modifier::UNDERLINED);     }
    if cell.inverse()    { style = style.add_modifier(Modifier::REVERSED);       }

    style
}

fn vt100_color(c: vt100::Color, default: Color) -> Color {
    match c {
        vt100::Color::Default         => default,
        vt100::Color::Idx(i)          => ansi_idx(i),
        vt100::Color::Rgb(r, g, b)   => Color::Rgb(r, g, b),
    }
}

fn ansi_idx(i: u8) -> Color {
    match i {
        0  => Color::Black,
        1  => Color::Red,
        2  => Color::Green,
        3  => Color::Yellow,
        4  => Color::Blue,
        5  => Color::Magenta,
        6  => Color::Cyan,
        7  => Color::White,
        8  => Color::DarkGray,
        9  => Color::LightRed,
        10 => Color::LightGreen,
        11 => Color::LightYellow,
        12 => Color::LightBlue,
        13 => Color::LightMagenta,
        14 => Color::LightCyan,
        15 => Color::White,
        n  => Color::Indexed(n),
    }
}

// ── Key → bytes ───────────────────────────────────────────────────────────────

pub fn key_to_bytes(code: KeyCode, mods: KeyModifiers) -> Option<Vec<u8>> {
    // Ctrl+<letter>
    if mods.contains(KeyModifiers::CONTROL) {
        if let KeyCode::Char(c) = code {
            let byte = (c as u8).wrapping_sub(b'a').wrapping_add(1);
            if byte < 32 { return Some(vec![byte]); }
        }
    }

    Some(match code {
        KeyCode::Char(c)   => c.to_string().into_bytes(),
        KeyCode::Enter     => b"\r".to_vec(),
        KeyCode::Backspace => b"\x7f".to_vec(),
        KeyCode::Tab       => b"\t".to_vec(),
        KeyCode::Esc       => b"\x1b".to_vec(),
        KeyCode::Up        => b"\x1b[A".to_vec(),
        KeyCode::Down      => b"\x1b[B".to_vec(),
        KeyCode::Right     => b"\x1b[C".to_vec(),
        KeyCode::Left      => b"\x1b[D".to_vec(),
        KeyCode::Home      => b"\x1b[H".to_vec(),
        KeyCode::End       => b"\x1b[F".to_vec(),
        KeyCode::PageUp    => b"\x1b[5~".to_vec(),
        KeyCode::PageDown  => b"\x1b[6~".to_vec(),
        KeyCode::Delete    => b"\x1b[3~".to_vec(),
        KeyCode::Insert    => b"\x1b[2~".to_vec(),
        KeyCode::F(1)      => b"\x1bOP".to_vec(),
        KeyCode::F(2)      => b"\x1bOQ".to_vec(),
        KeyCode::F(3)      => b"\x1bOR".to_vec(),
        KeyCode::F(4)      => b"\x1bOS".to_vec(),
        KeyCode::F(n)      => format!("\x1b[{}~", n + 10).into_bytes(),
        _                  => return None,
    })
}

// ── Interactive run loop ──────────────────────────────────────────────────────

/// Run a program in a PTY inside the ratatui TUI.
/// Exits when the child process exits, shell exits, or a global session switch is requested.
pub fn run_pty_session(terminal: &mut Term, program: &str, args: &[&str]) -> Result<()> {
    let size = terminal.size()?;
    // Leave one row for a status hint at the bottom
    let pty_rows = size.height.saturating_sub(1);
    let pty_cols = size.width;

    let mut session = PtySession::spawn(program, args, pty_cols, pty_rows)?;
    init_key_debug_log();
    let outcome = run_pty_loop(terminal, &mut session)?;
    if matches!(outcome, PtyLoopOutcome::SuspendedForSwitch) {
        park_active_session_pty(session);
    }
    Ok(())
}

pub fn resume_suspended_for_active(terminal: &mut Term) -> Result<bool> {
    let Some(mut session) = take_active_session_pty() else {
        return Ok(false);
    };
    append_marker_line(&format!(
        "--- pty session resume pid={} ---",
        std::process::id()
    ));
    let outcome = run_pty_loop(terminal, &mut session)?;
    if matches!(outcome, PtyLoopOutcome::SuspendedForSwitch) {
        park_active_session_pty(session);
    }
    Ok(true)
}

fn run_pty_loop(terminal: &mut Term, session: &mut PtySession) -> Result<PtyLoopOutcome> {
    let mut tilde_state = TildeChordState::None;

    loop {
        maybe_flush_expired_tilde_state(&mut tilde_state, session);

        // Resize if terminal changed
        let sz = terminal.size()?;
        let pr = sz.height.saturating_sub(1);
        let pc = sz.width;
        session.resize(pc, pr);

        // Render
        terminal.draw(|f| {
            let area    = f.area();
            let pty_area = Rect { x: 0, y: 0, width: area.width, height: area.height.saturating_sub(1) };
            let status_area = Rect { x: 0, y: area.height.saturating_sub(1), width: area.width, height: 1 };

            session.render(f, pty_area);
            render_status_bar(f, status_area);
        })?;

        // Check if child exited
        if !session.is_alive() { return Ok(PtyLoopOutcome::ProcessExited); }

        // Input
        if event::poll(Duration::from_millis(16))? {
            if let Event::Key(key) = event::read()? {
                debug_log_key(key.code, key.modifiers, key.kind);
                if key.kind == KeyEventKind::Release { continue; }

                if !matches!(tilde_state, TildeChordState::None)
                    && !matches!(key.code, KeyCode::Char('~') | KeyCode::Char('1'..='9'))
                {
                    flush_tilde_state(&mut tilde_state, session);
                }

                if try_tilde_session_chord(key.code, key.modifiers, &mut tilde_state) {
                    if crate::session::has_switch_request() {
                        return Ok(PtyLoopOutcome::SuspendedForSwitch);
                    }
                    continue;
                }

                if crate::ui::check_session_switch_pty_pub(key.code, key.modifiers) {
                    if crate::session::has_switch_request() {
                        return Ok(PtyLoopOutcome::SuspendedForSwitch);
                    }
                    continue;
                }
                if let Some(bytes) = key_to_bytes(key.code, key.modifiers) {
                    session.write(&bytes);
                }
            }
        }
    }
}

/// Convenience wrapper: launch an arbitrary command in a PTY session.
pub fn launch_in_pty(terminal: &mut Term, cmd: &[String]) -> Result<()> {
    if cmd.is_empty() { return Ok(()); }
    let program = &cmd[0];
    let args: Vec<&str> = cmd[1..].iter().map(String::as_str).collect();
    run_pty_session(terminal, program, &args)
}
