//! PTY session — runs a child process in a pseudo-terminal and renders its
//! output inside the ratatui TUI using vt100 for terminal emulation.
//!
//! The child process thinks it has a real terminal: correct size, SIGWINCH on
//! resize, readline/colors/cursor movement all work. Output is captured into a
//! vt100::Parser on a background reader thread and rendered each frame.
//!
//! Usage:
//!   run_pty_session(terminal, "/bin/bash", &[])
//!   launch_in_pty(terminal, &["vim", "file.txt"])

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use portable_pty::{native_pty_system, CommandBuilder, PtySize};
use ratatui::{
    layout::{Alignment, Rect},
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

#[derive(Debug, Clone, Default)]
pub struct PtyLaunchOptions {
    pub env: Vec<(String, String)>,
    pub top_bar: Option<String>,
}

static SUSPENDED_PTY: OnceLock<Mutex<HashMap<usize, PtySession>>> = OnceLock::new();

fn suspended_pty_map() -> &'static Mutex<HashMap<usize, PtySession>> {
    SUSPENDED_PTY.get_or_init(|| Mutex::new(HashMap::new()))
}

fn park_active_session_pty(session: PtySession) {
    let idx = crate::session::active_idx();
    if let Ok(mut map) = suspended_pty_map().lock() {
        if let Some(mut old) = map.insert(idx, session) {
            old.terminate();
        }
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
        for (_, mut session) in map.drain() {
            session.terminate();
        }
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
        Err(_) => std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open("robcos_keys.log")
            .ok(),
    }
}

fn append_marker_line(line: &str) {
    let Some(mut file) = open_key_debug_file() else {
        return;
    };
    let _ = writeln!(file, "{line}");
}

fn append_key_debug_line(line: &str) {
    if std::env::var_os("ROBCOS_KEY_DEBUG").is_none() {
        return;
    }
    let Some(mut file) = open_key_debug_file() else {
        return;
    };
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

fn try_tilde_session_chord(code: KeyCode, mods: KeyModifiers, state: &mut TildeChordState) -> bool {
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

#[derive(Debug, Clone, Copy)]
enum PtyRenderMode {
    Plain,
    Styled,
}

fn render_mode_for_program(program: &str) -> PtyRenderMode {
    match std::env::var("ROBCOS_PTY_RENDER")
        .ok()
        .map(|v| v.to_ascii_lowercase())
        .as_deref()
    {
        Some("styled") | Some("style") | Some("cell") => PtyRenderMode::Styled,
        Some("plain") | Some("raw") => PtyRenderMode::Plain,
        _ if crate::config::get_settings().cli_styled_render => PtyRenderMode::Styled,
        _ if is_ranger_program(program) => PtyRenderMode::Styled,
        _ => PtyRenderMode::Plain,
    }
}

#[derive(Debug, Clone, Copy)]
enum PtyColorMode {
    ThemeLock,
    PaletteMap,
    Monochrome,
    Ansi,
}

fn pty_color_mode() -> PtyColorMode {
    match std::env::var("ROBCOS_PTY_COLOR")
        .ok()
        .map(|v| v.to_ascii_lowercase())
        .as_deref()
    {
        Some("ansi") | Some("color") | Some("colours") | Some("colors") => PtyColorMode::Ansi,
        Some("mono") | Some("monochrome") | Some("plain") => PtyColorMode::Monochrome,
        Some("palette") | Some("palette-map") | Some("palettemap") => PtyColorMode::PaletteMap,
        Some("theme") | Some("theme-lock") | Some("themelock") | Some("lock") => {
            PtyColorMode::ThemeLock
        }
        _ => match crate::config::get_settings().cli_color_mode {
            crate::config::CliColorMode::ThemeLock => PtyColorMode::ThemeLock,
            crate::config::CliColorMode::PaletteMap => PtyColorMode::PaletteMap,
            crate::config::CliColorMode::Color => PtyColorMode::Ansi,
            crate::config::CliColorMode::Monochrome => PtyColorMode::Monochrome,
        },
    }
}

#[derive(Debug, Clone, Copy)]
enum AcsGlyphMode {
    Ascii,
    Unicode,
}

impl AcsGlyphMode {
    fn from_config() -> Self {
        match std::env::var("ROBCOS_ACS")
            .ok()
            .map(|v| v.to_ascii_lowercase())
            .as_deref()
        {
            Some("unicode") | Some("utf8") | Some("utf-8") => Self::Unicode,
            Some("ascii") | Some("plain") => Self::Ascii,
            _ => match crate::config::get_settings().cli_acs_mode {
                crate::config::CliAcsMode::Ascii => Self::Ascii,
                crate::config::CliAcsMode::Unicode => Self::Unicode,
            },
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
enum EscPending {
    #[default]
    None,
    Esc,
    EscParen,
    EscParenRight,
    EscCsi,
}

#[derive(Debug)]
struct DecSpecialGraphics {
    g0_special: bool,
    g1_special: bool,
    use_g1: bool,
    pending: EscPending,
    csi_buf: Vec<u8>,
    last_glyph: Vec<u8>,
    glyph_mode: AcsGlyphMode,
}

impl Default for DecSpecialGraphics {
    fn default() -> Self {
        Self {
            g0_special: false,
            g1_special: false,
            use_g1: false,
            pending: EscPending::None,
            csi_buf: Vec::new(),
            last_glyph: Vec::new(),
            glyph_mode: AcsGlyphMode::from_config(),
        }
    }
}

impl DecSpecialGraphics {
    fn active_is_special(&self) -> bool {
        if self.use_g1 {
            self.g1_special
        } else {
            self.g0_special
        }
    }

    fn emit_char(&mut self, out: &mut Vec<u8>, c: char) {
        let mut buf = [0u8; 4];
        let bytes = c.encode_utf8(&mut buf).as_bytes();
        out.extend_from_slice(bytes);
        self.last_glyph.clear();
        self.last_glyph.extend_from_slice(bytes);
    }

    fn emit_byte(&mut self, out: &mut Vec<u8>, b: u8) {
        out.push(b);
        if b >= 0x20 && b != 0x7f {
            self.last_glyph.clear();
            self.last_glyph.push(b);
        }
    }

    fn repeat_last_glyph(&self, out: &mut Vec<u8>, count: usize) {
        if self.last_glyph.is_empty() {
            return;
        }
        for _ in 0..count {
            out.extend_from_slice(&self.last_glyph);
        }
    }

    fn parse_rep_count(params: &[u8]) -> usize {
        if params.is_empty() {
            return 1;
        }
        let first = params.split(|&b| b == b';').next().unwrap_or(params);
        if first.is_empty() {
            return 1;
        }
        std::str::from_utf8(first)
            .ok()
            .and_then(|s| s.parse::<usize>().ok())
            .filter(|&n| n > 0)
            .unwrap_or(1)
    }

    fn map_special(&self, c: char) -> Option<char> {
        Some(match (self.glyph_mode, c) {
            (AcsGlyphMode::Unicode, '`') => '◆',
            (AcsGlyphMode::Unicode, 'a') => '▒',
            (AcsGlyphMode::Unicode, 'f') => '°',
            (AcsGlyphMode::Unicode, 'g') => '±',
            (AcsGlyphMode::Unicode, 'j') => '┘',
            (AcsGlyphMode::Unicode, 'k') => '┐',
            (AcsGlyphMode::Unicode, 'l') => '┌',
            (AcsGlyphMode::Unicode, 'm') => '└',
            (AcsGlyphMode::Unicode, 'n') => '┼',
            (AcsGlyphMode::Unicode, 'q') => '─',
            (AcsGlyphMode::Unicode, 't') => '├',
            (AcsGlyphMode::Unicode, 'u') => '┤',
            (AcsGlyphMode::Unicode, 'v') => '┴',
            (AcsGlyphMode::Unicode, 'w') => '┬',
            (AcsGlyphMode::Unicode, 'x') => '│',
            (AcsGlyphMode::Unicode, 'y') => '≤',
            (AcsGlyphMode::Unicode, 'z') => '≥',
            (AcsGlyphMode::Unicode, '{') => 'π',
            (AcsGlyphMode::Unicode, '|') => '≠',
            (AcsGlyphMode::Unicode, '}') => '£',
            (AcsGlyphMode::Unicode, '~') => '·',

            // ASCII fallback for fonts that do not render Unicode box chars.
            (AcsGlyphMode::Ascii, '`') => '*',
            (AcsGlyphMode::Ascii, 'a') => ':',
            (AcsGlyphMode::Ascii, 'f') => '*',
            (AcsGlyphMode::Ascii, 'g') => '#',
            (AcsGlyphMode::Ascii, 'j' | 'k' | 'l' | 'm' | 'n' | 't' | 'u' | 'v' | 'w') => '+',
            (AcsGlyphMode::Ascii, 'q') => '-',
            (AcsGlyphMode::Ascii, 'x') => '|',
            (AcsGlyphMode::Ascii, 'y') => '<',
            (AcsGlyphMode::Ascii, 'z') => '>',
            (AcsGlyphMode::Ascii, '{') => '*',
            (AcsGlyphMode::Ascii, '|') => '!',
            (AcsGlyphMode::Ascii, '}') => '#',
            (AcsGlyphMode::Ascii, '~') => '.',

            _ => return None,
        })
    }

    fn map_cp437(&self, b: u8) -> Option<char> {
        Some(match (self.glyph_mode, b) {
            (AcsGlyphMode::Unicode, 0xB3) => '│',
            (AcsGlyphMode::Unicode, 0xC4) => '─',
            (AcsGlyphMode::Unicode, 0xDA) => '┌',
            (AcsGlyphMode::Unicode, 0xBF) => '┐',
            (AcsGlyphMode::Unicode, 0xC0) => '└',
            (AcsGlyphMode::Unicode, 0xD9) => '┘',
            (AcsGlyphMode::Unicode, 0xC3) => '├',
            (AcsGlyphMode::Unicode, 0xB4) => '┤',
            (AcsGlyphMode::Unicode, 0xC2) => '┬',
            (AcsGlyphMode::Unicode, 0xC1) => '┴',
            (AcsGlyphMode::Unicode, 0xC5) => '┼',

            (AcsGlyphMode::Ascii, 0xB3) => '|',
            (AcsGlyphMode::Ascii, 0xC4) => '-',
            (AcsGlyphMode::Ascii, 0xDA | 0xBF | 0xC0 | 0xD9 | 0xC3 | 0xB4 | 0xC2 | 0xC1 | 0xC5) => {
                '+'
            }

            _ => return None,
        })
    }

    fn process(&mut self, input: &[u8]) -> Vec<u8> {
        let mut out = Vec::with_capacity(input.len());

        for &b in input {
            if let Some(mapped) = self.map_cp437(b) {
                self.emit_char(&mut out, mapped);
                continue;
            }

            // Complete ESC-designate sequences that may span read chunks.
            match self.pending {
                EscPending::Esc => match b {
                    b'(' => {
                        self.pending = EscPending::EscParen;
                        continue;
                    }
                    b')' => {
                        self.pending = EscPending::EscParenRight;
                        continue;
                    }
                    b'[' => {
                        self.pending = EscPending::EscCsi;
                        self.csi_buf.clear();
                        continue;
                    }
                    _ => {
                        out.push(0x1b);
                        self.emit_byte(&mut out, b);
                        self.pending = EscPending::None;
                        continue;
                    }
                },
                EscPending::EscParen => {
                    match b {
                        b'0' => self.g0_special = true,
                        b'B' => self.g0_special = false,
                        _ => {
                            out.extend_from_slice(&[0x1b, b'(', b]);
                        }
                    }
                    self.pending = EscPending::None;
                    continue;
                }
                EscPending::EscParenRight => {
                    match b {
                        b'0' => self.g1_special = true,
                        b'B' => self.g1_special = false,
                        _ => {
                            out.extend_from_slice(&[0x1b, b')', b]);
                        }
                    }
                    self.pending = EscPending::None;
                    continue;
                }
                EscPending::EscCsi => {
                    if (0x40..=0x7e).contains(&b) {
                        if b == b'b' {
                            let n = Self::parse_rep_count(&self.csi_buf);
                            self.repeat_last_glyph(&mut out, n);
                        } else {
                            out.extend_from_slice(&[0x1b, b'[']);
                            out.extend_from_slice(&self.csi_buf);
                            out.push(b);
                        }
                        self.csi_buf.clear();
                        self.pending = EscPending::None;
                    } else {
                        self.csi_buf.push(b);
                        if self.csi_buf.len() > 64 {
                            out.extend_from_slice(&[0x1b, b'[']);
                            out.extend_from_slice(&self.csi_buf);
                            self.csi_buf.clear();
                            self.pending = EscPending::None;
                        }
                    }
                    continue;
                }
                EscPending::None => {}
            }

            match b {
                0x1b => {
                    self.pending = EscPending::Esc;
                }
                // SO / SI: switch active charset between G1 and G0.
                0x0e => {
                    self.use_g1 = true;
                }
                0x0f => {
                    self.use_g1 = false;
                }
                _ => {
                    if self.active_is_special() && b.is_ascii() {
                        let ch = b as char;
                        if let Some(mapped) = self.map_special(ch) {
                            self.emit_char(&mut out, mapped);
                            continue;
                        }
                    }
                    self.emit_byte(&mut out, b);
                }
            }
        }

        out
    }
}

// ── PTY Session ───────────────────────────────────────────────────────────────

pub struct PtySession {
    /// Write end — send keyboard input to the child
    writer: Box<dyn Write + Send>,
    /// Shared vt100 parser — updated by reader thread, read by render loop
    parser: Arc<Mutex<vt100::Parser>>,
    /// Child handle — check if still alive
    child: Box<dyn portable_pty::Child + Send + Sync>,
    /// Current PTY dimensions
    cols: u16,
    rows: u16,
    /// Selected render mode for this command.
    render_mode: PtyRenderMode,
    /// Selected color mode for this command.
    color_mode: PtyColorMode,
    /// Selected border glyph mode for this command.
    acs_mode: AcsGlyphMode,
    /// Optional top banner shown above PTY content.
    top_bar: Option<String>,
    /// Master — kept alive so the PTY stays open; also used for resize
    master: Box<dyn portable_pty::MasterPty + Send>,
}

impl PtySession {
    pub fn spawn(
        program: &str,
        args: &[&str],
        cols: u16,
        rows: u16,
        options: &PtyLaunchOptions,
    ) -> Result<Self> {
        let pty_system = native_pty_system();
        let pair = pty_system.openpty(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        })?;

        let mut cmd = CommandBuilder::new(program);
        for arg in args {
            cmd.arg(arg);
        }
        for (key, value) in &options.env {
            cmd.env(key, value);
        }
        // Calcurse renders more reliably in Fixedsys/embedded PTY with ASCII ACS.
        if needs_ncurses_ascii_acs(program) && cmd.get_env("NCURSES_NO_UTF8_ACS").is_none() {
            cmd.env("NCURSES_NO_UTF8_ACS", "1");
        }
        if cmd.get_env("TERM").is_none() {
            cmd.env("TERM", "xterm-256color");
        }
        let render_mode = render_mode_for_program(program);
        let color_mode = pty_color_mode();
        let acs_mode = AcsGlyphMode::from_config();

        let child = pair.slave.spawn_command(cmd)?;
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
                let mut dec_special = DecSpecialGraphics {
                    glyph_mode: acs_mode,
                    ..DecSpecialGraphics::default()
                };
                loop {
                    match std::io::Read::read(&mut reader, &mut buf) {
                        Ok(0) | Err(_) => break,
                        Ok(n) => {
                            let bytes = dec_special.process(&buf[..n]);
                            if bytes.is_empty() {
                                continue;
                            }
                            if let Ok(mut p) = parser_clone.lock() {
                                p.process(&bytes);
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
            render_mode,
            color_mode,
            acs_mode,
            top_bar: options.top_bar.clone(),
            master: pair.master,
        })
    }

    /// Send raw bytes to the child's stdin (keyboard input)
    pub fn write(&mut self, data: &[u8]) {
        let _ = self.writer.write_all(data);
        let _ = self.writer.flush();
    }

    /// Translate a terminal key event and send it to the PTY child.
    pub fn send_key(&mut self, code: KeyCode, mods: KeyModifiers) {
        let application_cursor = self
            .parser
            .lock()
            .map(|p| p.screen().application_cursor())
            .unwrap_or(false);
        if let Some(bytes) = key_to_bytes(code, mods, application_cursor) {
            self.write(&bytes);
        }
    }

    /// Resize the PTY and notify the child via SIGWINCH
    pub fn resize(&mut self, cols: u16, rows: u16) {
        if cols == self.cols && rows == self.rows {
            return;
        }
        self.cols = cols;
        self.rows = rows;
        let _ = self.master.resize(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        });
        if let Ok(mut p) = self.parser.lock() {
            p.set_size(rows, cols);
        }
    }

    /// Is the child process still running?
    pub fn is_alive(&mut self) -> bool {
        matches!(self.child.try_wait(), Ok(None))
    }

    /// Force-stop the PTY child process and release its resources.
    pub fn terminate(&mut self) {
        if matches!(self.child.try_wait(), Ok(None)) {
            let _ = self.child.kill();
            let _ = self.child.try_wait();
        }
    }

    /// Render the current vt100 screen into `area` of the ratatui frame.
    pub fn render(&self, f: &mut ratatui::Frame, area: Rect) {
        let Ok(parser) = self.parser.lock() else {
            return;
        };
        let screen = parser.screen();

        let rows = area.height as usize;
        let cols = area.width as usize;

        if matches!(self.render_mode, PtyRenderMode::Plain) {
            let mut lines: Vec<Line> = screen
                .rows(0, area.width)
                .take(rows)
                .map(Line::from)
                .collect();
            while lines.len() < rows {
                lines.push(Line::from(""));
            }
            let para = match self.color_mode {
                PtyColorMode::ThemeLock | PtyColorMode::PaletteMap => Paragraph::new(lines).style(
                    Style::default()
                        .fg(crate::config::current_theme_color())
                        .bg(Color::Black),
                ),
                _ => Paragraph::new(lines),
            };
            f.render_widget(para, area);
            return;
        }

        let lines: Vec<Line> = (0..rows)
            .map(|row| {
                let spans: Vec<Span> = (0..cols)
                    .map(|col| {
                        let row_u16 = row as u16;
                        let col_u16 = col as u16;
                        let cell = screen.cell(row_u16, col_u16);
                        let ch = cell
                            .and_then(|c| c.contents().chars().next())
                            .unwrap_or(' ');
                        let ch = if matches!(self.acs_mode, AcsGlyphMode::Unicode) {
                            smooth_ascii_border_char(screen, row_u16, col_u16, ch)
                        } else {
                            ch
                        };
                        let text = ch.to_string();

                        let style = cell
                            .map(|c| vt100_style(c, self.color_mode))
                            .unwrap_or_else(|| vt100_default_style(self.color_mode));
                        Span::styled(text, style)
                    })
                    .collect();
                Line::from(spans)
            })
            .collect();

        f.render_widget(Paragraph::new(lines), area);
    }
}

#[derive(Debug, Clone, Copy, Default)]
struct LineConnections {
    up: bool,
    down: bool,
    left: bool,
    right: bool,
}

fn line_connections(ch: char) -> LineConnections {
    match ch {
        '-' | '─' => LineConnections {
            left: true,
            right: true,
            ..LineConnections::default()
        },
        '|' | '│' => LineConnections {
            up: true,
            down: true,
            ..LineConnections::default()
        },
        '+' | '┼' => LineConnections {
            up: true,
            down: true,
            left: true,
            right: true,
        },
        '┌' => LineConnections {
            down: true,
            right: true,
            ..LineConnections::default()
        },
        '┐' => LineConnections {
            down: true,
            left: true,
            ..LineConnections::default()
        },
        '└' => LineConnections {
            up: true,
            right: true,
            ..LineConnections::default()
        },
        '┘' => LineConnections {
            up: true,
            left: true,
            ..LineConnections::default()
        },
        '├' => LineConnections {
            up: true,
            down: true,
            right: true,
            ..LineConnections::default()
        },
        '┤' => LineConnections {
            up: true,
            down: true,
            left: true,
            ..LineConnections::default()
        },
        '┬' => LineConnections {
            down: true,
            left: true,
            right: true,
            ..LineConnections::default()
        },
        '┴' => LineConnections {
            up: true,
            left: true,
            right: true,
            ..LineConnections::default()
        },
        _ => LineConnections::default(),
    }
}

fn connection_map_to_unicode(c: LineConnections) -> Option<char> {
    Some(match (c.up, c.down, c.left, c.right) {
        (true, true, true, true) => '┼',
        (true, true, true, false) => '┤',
        (true, true, false, true) => '├',
        (true, false, true, true) => '┴',
        (false, true, true, true) => '┬',
        (true, true, false, false) => '│',
        (false, false, true, true) => '─',
        (false, true, false, true) => '┌',
        (false, true, true, false) => '┐',
        (true, false, false, true) => '└',
        (true, false, true, false) => '┘',
        (true, false, false, false) | (false, true, false, false) => '│',
        (false, false, true, false) | (false, false, false, true) => '─',
        _ => return None,
    })
}

fn screen_char(screen: &vt100::Screen, row: i32, col: i32) -> char {
    if row < 0 || col < 0 {
        return ' ';
    }
    screen
        .cell(row as u16, col as u16)
        .and_then(|c| c.contents().chars().next())
        .unwrap_or(' ')
}

fn smooth_ascii_border_char(screen: &vt100::Screen, row: u16, col: u16, ch: char) -> char {
    if !matches!(ch, '+' | '-' | '|') {
        return ch;
    }

    let row = i32::from(row);
    let col = i32::from(col);

    let left = line_connections(screen_char(screen, row, col - 1)).right;
    let right = line_connections(screen_char(screen, row, col + 1)).left;
    let up = line_connections(screen_char(screen, row - 1, col)).down;
    let down = line_connections(screen_char(screen, row + 1, col)).up;
    let conn = LineConnections {
        up,
        down,
        left,
        right,
    };

    match ch {
        '-' if !(left || right) => '-',
        '|' if !(up || down) => '|',
        '+' if !(left || right || up || down) => '+',
        '-' => connection_map_to_unicode(conn).unwrap_or('─'),
        '|' => connection_map_to_unicode(conn).unwrap_or('│'),
        '+' => connection_map_to_unicode(conn).unwrap_or('┼'),
        _ => ch,
    }
}

fn needs_ncurses_ascii_acs(program: &str) -> bool {
    let Some(name) = command_basename(program) else {
        return false;
    };

    name.starts_with("calcurse")
}

fn command_basename(program: &str) -> Option<String> {
    std::path::Path::new(program)
        .file_name()
        .and_then(|name| name.to_str())
        .map(|name| name.to_ascii_lowercase())
}

fn is_ranger_program(program: &str) -> bool {
    command_basename(program)
        .map(|name| name.starts_with("ranger"))
        .unwrap_or(false)
}

// ── vt100 cell → ratatui Style ────────────────────────────────────────────────

fn vt100_default_style(mode: PtyColorMode) -> Style {
    match mode {
        PtyColorMode::ThemeLock | PtyColorMode::PaletteMap => Style::default()
            .fg(crate::config::current_theme_color())
            .bg(Color::Black),
        _ => Style::default(),
    }
}

fn vt100_style(cell: &vt100::Cell, mode: PtyColorMode) -> Style {
    let mut style = vt100_default_style(mode);

    match mode {
        PtyColorMode::Ansi => {
            style = style.fg(vt100_color(cell.fgcolor(), Color::Reset));
            style = style.bg(vt100_color(cell.bgcolor(), Color::Reset));
        }
        PtyColorMode::PaletteMap => {
            if !matches!(cell.fgcolor(), vt100::Color::Default) {
                style = style.fg(palette_map_vt100_color(cell.fgcolor(), false));
            }
            if !matches!(cell.bgcolor(), vt100::Color::Default) {
                style = style.bg(palette_map_vt100_color(cell.bgcolor(), true));
            }
        }
        PtyColorMode::ThemeLock | PtyColorMode::Monochrome => {}
    }

    if cell.bold() {
        style = style.add_modifier(Modifier::BOLD);
    }
    if cell.italic() {
        style = style.add_modifier(Modifier::ITALIC);
    }
    if cell.underline() {
        style = style.add_modifier(Modifier::UNDERLINED);
    }
    if cell.inverse() {
        match mode {
            PtyColorMode::ThemeLock => {
                style = style
                    .fg(Color::Black)
                    .bg(crate::config::current_theme_color());
            }
            PtyColorMode::PaletteMap | PtyColorMode::Monochrome | PtyColorMode::Ansi => {
                style = style.add_modifier(Modifier::REVERSED);
            }
        }
    }

    style
}

fn vt100_color(c: vt100::Color, default: Color) -> Color {
    match c {
        vt100::Color::Default => default,
        vt100::Color::Idx(i) => ansi_idx(i),
        vt100::Color::Rgb(r, g, b) => Color::Rgb(r, g, b),
    }
}

fn ansi_idx(i: u8) -> Color {
    match i {
        0 => Color::Black,
        1 => Color::Red,
        2 => Color::Green,
        3 => Color::Yellow,
        4 => Color::Blue,
        5 => Color::Magenta,
        6 => Color::Cyan,
        7 => Color::White,
        8 => Color::DarkGray,
        9 => Color::LightRed,
        10 => Color::LightGreen,
        11 => Color::LightYellow,
        12 => Color::LightBlue,
        13 => Color::LightMagenta,
        14 => Color::LightCyan,
        15 => Color::White,
        n => Color::Indexed(n),
    }
}

fn palette_map_vt100_color(c: vt100::Color, is_background: bool) -> Color {
    let Some((r, g, b)) = vt100_color_rgb(c) else {
        return if is_background {
            Color::Black
        } else {
            crate::config::current_theme_color()
        };
    };

    let luma = (0.2126 * (r as f32) + 0.7152 * (g as f32) + 0.0722 * (b as f32)) / 255.0;
    let scale = if is_background {
        if luma < 0.33 {
            0.16
        } else if luma < 0.66 {
            0.22
        } else {
            0.30
        }
    } else if luma < 0.20 {
        0.40
    } else if luma < 0.40 {
        0.58
    } else if luma < 0.60 {
        0.74
    } else if luma < 0.80 {
        0.88
    } else {
        1.00
    };

    let (tr, tg, tb) = theme_base_rgb();
    Color::Rgb(
        ((tr as f32 * scale).round() as u8).max(1),
        ((tg as f32 * scale).round() as u8).max(1),
        ((tb as f32 * scale).round() as u8).max(1),
    )
}

fn vt100_color_rgb(c: vt100::Color) -> Option<(u8, u8, u8)> {
    match c {
        vt100::Color::Default => None,
        vt100::Color::Rgb(r, g, b) => Some((r, g, b)),
        vt100::Color::Idx(i) => color_to_rgb(ansi_idx(i)),
    }
}

fn theme_base_rgb() -> (u8, u8, u8) {
    color_to_rgb(crate::config::current_theme_color()).unwrap_or((0, 255, 0))
}

fn color_to_rgb(c: Color) -> Option<(u8, u8, u8)> {
    Some(match c {
        Color::Reset => return None,
        Color::Black => (0, 0, 0),
        Color::Red => (205, 0, 0),
        Color::Green => (0, 205, 0),
        Color::Yellow => (205, 205, 0),
        Color::Blue => (0, 0, 238),
        Color::Magenta => (205, 0, 205),
        Color::Cyan => (0, 205, 205),
        Color::Gray => (180, 180, 180),
        Color::DarkGray => (120, 120, 120),
        Color::LightRed => (255, 85, 85),
        Color::LightGreen => (85, 255, 85),
        Color::LightYellow => (255, 255, 85),
        Color::LightBlue => (85, 85, 255),
        Color::LightMagenta => (255, 85, 255),
        Color::LightCyan => (85, 255, 255),
        Color::White => (245, 245, 245),
        Color::Rgb(r, g, b) => (r, g, b),
        Color::Indexed(i) => indexed_ansi_rgb(i),
    })
}

fn indexed_ansi_rgb(i: u8) -> (u8, u8, u8) {
    if i < 16 {
        return color_to_rgb(ansi_idx(i)).unwrap_or((255, 255, 255));
    }
    if (16..=231).contains(&i) {
        let n = i - 16;
        let r = n / 36;
        let g = (n % 36) / 6;
        let b = n % 6;
        let step = |v: u8| if v == 0 { 0 } else { 55 + v * 40 };
        return (step(r), step(g), step(b));
    }
    // 232..=255 grayscale ramp
    let g = 8 + (i.saturating_sub(232) * 10);
    (g, g, g)
}

// ── Key → bytes ───────────────────────────────────────────────────────────────

pub fn key_to_bytes(
    code: KeyCode,
    mods: KeyModifiers,
    application_cursor: bool,
) -> Option<Vec<u8>> {
    // Ctrl+<letter>
    if mods.contains(KeyModifiers::CONTROL) {
        if let KeyCode::Char(c) = code {
            let lc = c.to_ascii_lowercase();
            let byte = (lc as u8).wrapping_sub(b'a').wrapping_add(1);
            if byte < 32 {
                return Some(vec![byte]);
            }
        }
    }

    Some(match code {
        KeyCode::Char(c) => c.to_string().into_bytes(),
        KeyCode::Enter => b"\r".to_vec(),
        KeyCode::Backspace => b"\x7f".to_vec(),
        KeyCode::Tab => b"\t".to_vec(),
        KeyCode::Esc => b"\x1b".to_vec(),
        KeyCode::Up => {
            if application_cursor {
                b"\x1bOA".to_vec()
            } else {
                b"\x1b[A".to_vec()
            }
        }
        KeyCode::Down => {
            if application_cursor {
                b"\x1bOB".to_vec()
            } else {
                b"\x1b[B".to_vec()
            }
        }
        KeyCode::Right => {
            if application_cursor {
                b"\x1bOC".to_vec()
            } else {
                b"\x1b[C".to_vec()
            }
        }
        KeyCode::Left => {
            if application_cursor {
                b"\x1bOD".to_vec()
            } else {
                b"\x1b[D".to_vec()
            }
        }
        KeyCode::Home => {
            if application_cursor {
                b"\x1bOH".to_vec()
            } else {
                b"\x1b[H".to_vec()
            }
        }
        KeyCode::End => {
            if application_cursor {
                b"\x1bOF".to_vec()
            } else {
                b"\x1b[F".to_vec()
            }
        }
        KeyCode::PageUp => b"\x1b[5~".to_vec(),
        KeyCode::PageDown => b"\x1b[6~".to_vec(),
        KeyCode::Delete => b"\x1b[3~".to_vec(),
        KeyCode::Insert => b"\x1b[2~".to_vec(),
        KeyCode::F(1) => b"\x1bOP".to_vec(),
        KeyCode::F(2) => b"\x1bOQ".to_vec(),
        KeyCode::F(3) => b"\x1bOR".to_vec(),
        KeyCode::F(4) => b"\x1bOS".to_vec(),
        KeyCode::F(n) => format!("\x1b[{}~", n + 10).into_bytes(),
        _ => return None,
    })
}

// ── Interactive run loop ──────────────────────────────────────────────────────

fn pty_content_rows(total_height: u16, has_top_bar: bool) -> u16 {
    let reserved = 1 + u16::from(has_top_bar); // bottom status + optional top bar
    total_height.saturating_sub(reserved).max(1)
}

fn render_top_bar(f: &mut ratatui::Frame, area: Rect, label: &str) {
    let text = format!(" {label} ");
    let style = Style::default()
        .fg(Color::Black)
        .bg(crate::config::current_theme_color())
        .add_modifier(Modifier::BOLD);
    f.render_widget(
        Paragraph::new(text)
            .alignment(Alignment::Center)
            .style(style),
        area,
    );
}

/// Run a program in a PTY inside the ratatui TUI.
/// Exits when the child process exits, shell exits, or a global session switch is requested.
pub fn run_pty_session(terminal: &mut Term, program: &str, args: &[&str]) -> Result<()> {
    run_pty_session_with_options(terminal, program, args, PtyLaunchOptions::default())
}

/// Run a PTY session with custom environment and optional top banner.
pub fn run_pty_session_with_options(
    terminal: &mut Term,
    program: &str,
    args: &[&str],
    options: PtyLaunchOptions,
) -> Result<()> {
    let size = terminal.size()?;
    let pty_rows = pty_content_rows(size.height, options.top_bar.is_some());
    let pty_cols = size.width;

    let mut session = PtySession::spawn(program, args, pty_cols, pty_rows, &options)?;
    init_key_debug_log();
    let outcome = run_pty_loop(terminal, &mut session)?;
    if matches!(outcome, PtyLoopOutcome::SuspendedForSwitch) {
        park_active_session_pty(session);
    }
    Ok(())
}

/// Convenience wrapper: launch an arbitrary command in a PTY session.
pub fn launch_in_pty(terminal: &mut Term, cmd: &[String]) -> Result<()> {
    if cmd.is_empty() {
        return Ok(());
    }
    let program = &cmd[0];
    let args: Vec<&str> = cmd[1..].iter().map(String::as_str).collect();
    run_pty_session(terminal, program, &args)
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
        let pr = pty_content_rows(sz.height, session.top_bar.is_some());
        let pc = sz.width;
        session.resize(pc, pr);

        // Render
        terminal.draw(|f| {
            let area = f.area();
            let show_top_bar = session.top_bar.is_some() && area.height > 1;
            let top_h = if show_top_bar { 1 } else { 0 };
            let pty_area = Rect {
                x: 0,
                y: top_h,
                width: area.width,
                height: pty_content_rows(area.height, show_top_bar),
            };
            let status_area = Rect {
                x: 0,
                y: area.height.saturating_sub(1),
                width: area.width,
                height: 1,
            };

            if let Some(label) = session.top_bar.as_deref() {
                render_top_bar(
                    f,
                    Rect {
                        x: 0,
                        y: 0,
                        width: area.width,
                        height: 1,
                    },
                    label,
                );
            }
            session.render(f, pty_area);
            render_status_bar(f, status_area);
        })?;

        // Check if child exited
        if !session.is_alive() {
            return Ok(PtyLoopOutcome::ProcessExited);
        }

        // Input
        if event::poll(Duration::from_millis(16))? {
            if let Event::Key(key) = event::read()? {
                debug_log_key(key.code, key.modifiers, key.kind);
                if key.kind == KeyEventKind::Release {
                    continue;
                }

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
                let application_cursor = session
                    .parser
                    .lock()
                    .map(|p| p.screen().application_cursor())
                    .unwrap_or(false);
                if let Some(bytes) = key_to_bytes(key.code, key.modifiers, application_cursor) {
                    session.write(&bytes);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        key_to_bytes, needs_ncurses_ascii_acs, smooth_ascii_border_char, DecSpecialGraphics,
    };
    use crossterm::event::{KeyCode, KeyModifiers};

    #[test]
    fn app_cursor_arrows_use_ss3_sequences() {
        assert_eq!(
            key_to_bytes(KeyCode::Up, KeyModifiers::NONE, true).unwrap(),
            b"\x1bOA".to_vec()
        );
        assert_eq!(
            key_to_bytes(KeyCode::Down, KeyModifiers::NONE, true).unwrap(),
            b"\x1bOB".to_vec()
        );
        assert_eq!(
            key_to_bytes(KeyCode::Right, KeyModifiers::NONE, true).unwrap(),
            b"\x1bOC".to_vec()
        );
        assert_eq!(
            key_to_bytes(KeyCode::Left, KeyModifiers::NONE, true).unwrap(),
            b"\x1bOD".to_vec()
        );
    }

    #[test]
    fn dec_special_graphics_translates_box_chars() {
        let mut d = DecSpecialGraphics::default();
        let out = d.process(b"\x1b(0lqqqk\x1b(B");
        let line = String::from_utf8(out).unwrap();
        assert!(
            line == "+---+" || line == "┌───┐",
            "unexpected line mapping: {line:?}"
        );
    }

    #[test]
    fn cp437_box_chars_map_in_ascii_mode() {
        let mut d = DecSpecialGraphics::default();
        let out = d.process(&[0xDA, 0xC4, 0xC4, 0xBF, 0x0d, 0x0a, 0xB3, b' ', 0xB3]);
        let text = String::from_utf8(out).unwrap();
        assert!(
            text == "+--+\r\n| |" || text == "┌──┐\r\n│ │",
            "unexpected cp437 mapping: {text:?}"
        );
    }

    #[test]
    fn ascii_acs_env_only_for_calcurse() {
        assert!(needs_ncurses_ascii_acs("calcurse"));
        assert!(needs_ncurses_ascii_acs("/opt/homebrew/bin/calcurse"));
        assert!(!needs_ncurses_ascii_acs("ranger"));
        assert!(!needs_ncurses_ascii_acs("/usr/bin/vim"));
    }

    #[test]
    fn vt100_rows_keep_acs_after_translation() {
        let mut d = DecSpecialGraphics::default();
        let bytes = d.process(b"\x1b(0lq\x1b[5bk\x1b(B");
        let mut p = vt100::Parser::new(4, 40, 0);
        p.process(&bytes);
        let line = p
            .screen()
            .rows(0, 20)
            .next()
            .unwrap_or_default()
            .to_string();
        assert!(
            line.contains("+------+") || line.contains("┌──────┐"),
            "translated line was: {line:?}"
        );
    }

    #[test]
    fn ascii_box_smoothing_maps_to_unicode_lines() {
        let mut p = vt100::Parser::new(4, 16, 0);
        p.process(b"+----+\r\n|    |\r\n+----+");
        let s = p.screen();

        assert_eq!(smooth_ascii_border_char(s, 0, 0, '+'), '┌');
        assert_eq!(smooth_ascii_border_char(s, 0, 5, '+'), '┐');
        assert_eq!(smooth_ascii_border_char(s, 2, 0, '+'), '└');
        assert_eq!(smooth_ascii_border_char(s, 2, 5, '+'), '┘');
        assert_eq!(smooth_ascii_border_char(s, 0, 2, '-'), '─');
        assert_eq!(smooth_ascii_border_char(s, 1, 0, '|'), '│');
    }

    #[test]
    fn smoothing_does_not_touch_standalone_plus() {
        let mut p = vt100::Parser::new(2, 8, 0);
        p.process(b"1+2");
        let s = p.screen();
        assert_eq!(smooth_ascii_border_char(s, 0, 1, '+'), '+');
    }
}
