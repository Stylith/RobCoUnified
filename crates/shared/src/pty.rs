//! PTY session — runs a child process in a pseudo-terminal using vt100 for
//! terminal emulation. Output is captured into a vt100::Parser on a background
//! reader thread and exposed via `committed_frame()` / `snapshot_styled()` for
//! the iced renderer.

use anyhow::Result;
use crossterm::event::{KeyCode, KeyModifiers, MouseButton, MouseEventKind};
use portable_pty::{native_pty_system, CommandBuilder, ExitStatus, PtySize};
use std::io::Write;
use std::sync::atomic::{AtomicU16, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Default)]
pub struct PtyLaunchOptions {
    pub env: Vec<(String, String)>,
    pub top_bar: Option<String>,
    pub force_render_mode: Option<bool>, // Some(true)=plain, Some(false)=styled
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
        _ => match crate::config::with_settings(|settings| settings.cli_color_mode) {
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
        match crate::config::with_settings(|settings| settings.cli_acs_mode) {
            crate::config::CliAcsMode::Ascii => Self::Ascii,
            crate::config::CliAcsMode::Unicode => Self::Unicode,
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
            // Checkerboard often renders as heavy '#' in Fixedsys variants; use
            // a lighter glyph to preserve texture without hash-wall artifacts.
            (AcsGlyphMode::Unicode, 'a') => '·',
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
            (AcsGlyphMode::Unicode, 0xB0) => '·',
            (AcsGlyphMode::Unicode, 0xB1) => '·',
            (AcsGlyphMode::Unicode, 0xB2) => '·',

            (AcsGlyphMode::Ascii, 0xB3) => '|',
            (AcsGlyphMode::Ascii, 0xC4) => '-',
            (AcsGlyphMode::Ascii, 0xDA | 0xBF | 0xC0 | 0xD9 | 0xC3 | 0xB4 | 0xC2 | 0xC1 | 0xC5) => {
                '+'
            }
            (AcsGlyphMode::Ascii, 0xB0 | 0xB1 | 0xB2) => '.',

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
    /// Monotonic counter incremented by reader thread when new PTY output arrives.
    output_epoch: Arc<AtomicU64>,
    /// Last observed output epoch on UI/render side.
    last_seen_output_epoch: u64,
    /// Committed display buffer — updated by reader thread after each
    /// coalesced I/O batch.  The renderer reads from this, never from
    /// the parser directly.
    display: Arc<Mutex<CommittedFrame>>,
    /// Shared PTY dimensions so the reader thread can snapshot at the
    /// correct size.  Updated by resize().
    shared_cols: Arc<AtomicU16>,
    shared_rows: Arc<AtomicU16>,
    /// Cached child exit status once the process has terminated.
    last_exit_status: Option<ExitStatus>,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct PtyTextSnapshot {
    pub lines: Vec<String>,
    pub cursor_row: u16,
    pub cursor_col: u16,
    pub cursor_hidden: bool,
}

/// Framework-agnostic terminal cell color. Decouples PTY rendering from any
/// specific UI framework (ratatui, iced, etc.).
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CellColor {
    /// Use the terminal's default fg/bg (maps to theme color in renderer).
    Reset,
    Black,
    DarkGray,
    Gray,
    White,
    Red,
    LightRed,
    Green,
    LightGreen,
    Yellow,
    LightYellow,
    Blue,
    LightBlue,
    Magenta,
    LightMagenta,
    Cyan,
    LightCyan,
    Rgb(u8, u8, u8),
    Indexed(u8),
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub struct PtyStyledCell {
    pub ch: char,
    pub fg: CellColor,
    pub bg: CellColor,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub reversed: bool,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct PtyStyledSnapshot {
    pub cells: Vec<Vec<PtyStyledCell>>,
    pub cursor_row: u16,
    pub cursor_col: u16,
    pub cursor_hidden: bool,
}

/// A complete terminal frame committed by the reader thread after processing
/// a coalesced I/O batch.  The renderer reads exclusively from this buffer,
/// never touching the vt100 parser directly, which eliminates race conditions
/// that cause mid-update blank/torn frames in ncurses apps.
#[allow(dead_code)]
#[derive(Clone)]
pub struct CommittedFrame {
    pub styled: PtyStyledSnapshot,
    pub plain: PtyTextSnapshot,
    pub cols: u16,
    pub rows: u16,
}

impl CommittedFrame {
    fn blank(cols: u16, rows: u16) -> Self {
        let default_cell = PtyStyledCell {
            ch: ' ',
            fg: CellColor::Reset,
            bg: CellColor::Black,
            bold: false,
            italic: false,
            underline: false,
            reversed: false,
        };
        Self {
            styled: PtyStyledSnapshot {
                cells: vec![vec![default_cell; cols as usize]; rows as usize],
                cursor_row: 0,
                cursor_col: 0,
                cursor_hidden: true,
            },
            plain: PtyTextSnapshot {
                lines: vec![String::new(); rows as usize],
                cursor_row: 0,
                cursor_col: 0,
                cursor_hidden: true,
            },
            cols,
            rows,
        }
    }
}

/// Build a styled cell snapshot from a locked parser.
/// Called by the reader thread while holding the parser lock — guaranteed
/// to see a consistent post-batch state.
fn build_styled_snapshot(
    parser: &vt100::Parser,
    cols: u16,
    rows: u16,
    acs_mode: AcsGlyphMode,
    color_mode: PtyColorMode,
) -> PtyStyledSnapshot {
    let screen = parser.screen();
    let mut lines = Vec::with_capacity(rows as usize);
    for row in 0..rows {
        let mut out = Vec::with_capacity(cols as usize);
        for col in 0..cols {
            let cell = screen.cell(row, col);
            let ch = cell
                .and_then(|c| c.contents().chars().next())
                .unwrap_or(' ');
            let ch = if matches!(acs_mode, AcsGlyphMode::Unicode) {
                smooth_ascii_border_char(screen, row, col, ch)
            } else {
                ch
            };
            let (fg, bg, bold, italic, underline, reversed) = if let Some(c) = cell {
                let (fg, bg) = cell_colors_direct(c, color_mode);
                let reversed = c.inverse();
                let (fg, bg) = if reversed && matches!(color_mode, PtyColorMode::ThemeLock) {
                    (CellColor::Black, CellColor::Reset)
                } else {
                    (fg, bg)
                };
                (fg, bg, c.bold(), c.italic(), c.underline(), c.inverse())
            } else {
                (CellColor::Reset, CellColor::Black, false, false, false, false)
            };
            out.push(PtyStyledCell { ch, fg, bg, bold, italic, underline, reversed });
        }
        lines.push(out);
    }
    let (cursor_row, cursor_col) = screen.cursor_position();
    PtyStyledSnapshot {
        cells: lines,
        cursor_row: cursor_row.min(rows.saturating_sub(1)),
        cursor_col: cursor_col.min(cols.saturating_sub(1)),
        cursor_hidden: screen.hide_cursor(),
    }
}

/// Compute fg/bg `CellColor` directly from a vt100 cell, without going
/// through a ratatui Style intermediate. Used by the iced renderer path.
fn cell_colors_direct(cell: &vt100::Cell, mode: PtyColorMode) -> (CellColor, CellColor) {
    match mode {
        PtyColorMode::Ansi => {
            let fg = vt100_color_to_cell(cell.fgcolor());
            let bg = vt100_color_to_cell(cell.bgcolor());
            (fg, bg)
        }
        PtyColorMode::PaletteMap => {
            let fg = if matches!(cell.fgcolor(), vt100::Color::Default) {
                CellColor::Reset
            } else {
                palette_map_vt100_color_to_cell(cell.fgcolor(), false)
            };
            let bg = if matches!(cell.bgcolor(), vt100::Color::Default) {
                CellColor::Black
            } else {
                palette_map_vt100_color_to_cell(cell.bgcolor(), true)
            };
            (fg, bg)
        }
        PtyColorMode::ThemeLock | PtyColorMode::Monochrome => {
            (CellColor::Reset, CellColor::Black)
        }
    }
}

fn vt100_color_to_cell(c: vt100::Color) -> CellColor {
    match c {
        vt100::Color::Default => CellColor::Reset,
        vt100::Color::Idx(i) => ansi_idx_to_cell(i),
        vt100::Color::Rgb(r, g, b) => CellColor::Rgb(r, g, b),
    }
}

fn ansi_idx_to_cell(i: u8) -> CellColor {
    match i {
        0 => CellColor::Black,
        1 => CellColor::Red,
        2 => CellColor::Green,
        3 => CellColor::Yellow,
        4 => CellColor::Blue,
        5 => CellColor::Magenta,
        6 => CellColor::Cyan,
        7 => CellColor::White,
        8 => CellColor::DarkGray,
        9 => CellColor::LightRed,
        10 => CellColor::LightGreen,
        11 => CellColor::LightYellow,
        12 => CellColor::LightBlue,
        13 => CellColor::LightMagenta,
        14 => CellColor::LightCyan,
        15 => CellColor::White,
        n => CellColor::Indexed(n),
    }
}

fn palette_map_vt100_color_to_cell(c: vt100::Color, is_background: bool) -> CellColor {
    let Some((r, g, b)) = vt100_color_to_rgb(c) else {
        return if is_background { CellColor::Black } else { CellColor::Reset };
    };
    let luma = (0.2126 * (r as f32) + 0.7152 * (g as f32) + 0.0722 * (b as f32)) / 255.0;
    let scale = if is_background {
        if luma < 0.25 { 0.12 } else if luma < 0.50 { 0.18 } else if luma < 0.75 { 0.24 } else { 0.32 }
    } else if luma < 0.20 { 0.90 } else if luma < 0.40 { 0.95 } else if luma < 0.60 { 1.00 } else if luma < 0.80 { 1.05 } else { 1.10 };
    let (tr, tg, tb) = theme_color_to_rgb(crate::config::current_theme_color());
    CellColor::Rgb(
        (tr as f32 * scale).round().clamp(1.0, 255.0) as u8,
        (tg as f32 * scale).round().clamp(1.0, 255.0) as u8,
        (tb as f32 * scale).round().clamp(1.0, 255.0) as u8,
    )
}

fn vt100_color_to_rgb(c: vt100::Color) -> Option<(u8, u8, u8)> {
    match c {
        vt100::Color::Default => None,
        vt100::Color::Rgb(r, g, b) => Some((r, g, b)),
        vt100::Color::Idx(i) => ansi_idx_to_rgb(i),
    }
}

fn ansi_idx_to_rgb(i: u8) -> Option<(u8, u8, u8)> {
    Some(match i {
        0 => (0, 0, 0),
        1 => (205, 0, 0),
        2 => (0, 205, 0),
        3 => (205, 205, 0),
        4 => (0, 0, 238),
        5 => (205, 0, 205),
        6 => (0, 205, 205),
        7 => (180, 180, 180),
        8 => (120, 120, 120),
        9 => (255, 85, 85),
        10 => (85, 255, 85),
        11 => (255, 255, 85),
        12 => (85, 85, 255),
        13 => (255, 85, 255),
        14 => (85, 255, 255),
        15 => (245, 245, 245),
        n if n < 232 => {
            let n = n - 16;
            let r = n / 36;
            let g = (n % 36) / 6;
            let b = n % 6;
            let step = |v: u8| if v == 0 { 0 } else { 55 + v * 40 };
            (step(r), step(g), step(b))
        }
        n => {
            let g = 8 + (n.saturating_sub(232) * 10);
            (g, g, g)
        }
    })
}

fn theme_color_to_rgb(c: crate::config::ThemeColor) -> (u8, u8, u8) {
    use crate::config::ThemeColor as T;
    match c {
        T::Black => (0, 0, 0),
        T::DarkGray => (120, 120, 120),
        T::Gray => (180, 180, 180),
        T::White => (245, 245, 245),
        T::Red => (205, 0, 0),
        T::LightRed => (255, 85, 85),
        T::Green => (0, 205, 0),
        T::LightGreen => (85, 255, 85),
        T::Yellow => (205, 205, 0),
        T::LightYellow => (255, 255, 85),
        T::Blue => (0, 0, 238),
        T::LightBlue => (85, 85, 255),
        T::Magenta => (205, 0, 205),
        T::LightMagenta => (255, 85, 255),
        T::Cyan => (0, 205, 205),
        T::LightCyan => (85, 255, 255),
        T::Rgb(r, g, b) => (r, g, b),
    }
}

/// Build a plain text snapshot from a locked parser.
fn build_plain_snapshot(parser: &vt100::Parser, cols: u16, rows: u16) -> PtyTextSnapshot {
    let screen = parser.screen();
    let mut lines: Vec<String> = screen.rows(0, cols).take(rows as usize).collect();
    while lines.len() < rows as usize {
        lines.push(String::new());
    }
    let (cursor_row, cursor_col) = screen.cursor_position();
    PtyTextSnapshot {
        lines,
        cursor_row: cursor_row.min(rows.saturating_sub(1)),
        cursor_col: cursor_col.min(cols.saturating_sub(1)),
        cursor_hidden: screen.hide_cursor(),
    }
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
        let acs_mode = AcsGlyphMode::from_config();
        // Calcurse renders more reliably in Fixedsys/embedded PTY with ASCII ACS.
        if matches!(acs_mode, AcsGlyphMode::Ascii)
            && needs_ncurses_ascii_acs(program)
            && cmd.get_env("NCURSES_NO_UTF8_ACS").is_none()
        {
            cmd.env("NCURSES_NO_UTF8_ACS", "1");
        }
        if cmd.get_env("TERM").is_none() {
            // Use xterm (not xterm-256color) — our vt100 parser + custom
            // renderer may not faithfully support every xterm-256color
            // capability.  ncurses apps look up terminfo for TERM and will
            // use simpler, more compatible escape sequences with "xterm".
            cmd.env("TERM", "xterm");
        }
        let render_mode = match options.force_render_mode {
            Some(true) => PtyRenderMode::Plain,
            Some(false) => PtyRenderMode::Styled,
            None => render_mode_for_program(program),
        };
        let color_mode = pty_color_mode();

        let child = pair.slave.spawn_command(cmd)?;
        let writer = pair.master.take_writer()?;
        let reader = pair.master.try_clone_reader()?;

        // Grab the master fd for poll()-based coalescing in the reader thread.
        // The reader is a dup of this fd so polling it tells us whether the
        // reader has more data queued.
        #[cfg(unix)]
        let poll_fd = pair.master.as_raw_fd().unwrap_or(-1);

        // vt100 parser — shared with reader thread
        let parser = Arc::new(Mutex::new(vt100::Parser::new(rows, cols, 0)));
        let parser_clone = Arc::clone(&parser);
        let output_epoch = Arc::new(AtomicU64::new(0));
        let output_epoch_clone = Arc::clone(&output_epoch);

        // Committed display buffer — the reader thread builds snapshots
        // after each coalesced batch and commits here.
        let display = Arc::new(Mutex::new(CommittedFrame::blank(cols, rows)));
        let display_clone = Arc::clone(&display);
        let shared_cols = Arc::new(AtomicU16::new(cols));
        let shared_rows = Arc::new(AtomicU16::new(rows));
        let reader_cols = Arc::clone(&shared_cols);
        let reader_rows = Arc::clone(&shared_rows);
        let reader_acs_mode = acs_mode;
        let reader_color_mode = color_mode;

        // Reader thread: pump PTY output into the vt100 parser continuously.
        // Uses poll()-based I/O coalescing to prevent mid-frame tearing from
        // ncurses apps that send "clear screen" + "draw content" as separate
        // write() calls.  After the first blocking read returns data, we poll
        // to check if more bytes are queued and read them too, so the whole
        // update is parsed as a single batch.
        std::thread::Builder::new()
            .name("robcos-pty-reader".into())
            .spawn(move || {
                let mut reader = reader;
                let mut buf = [0u8; 16384];
                let mut dec_special = DecSpecialGraphics {
                    glyph_mode: acs_mode,
                    ..DecSpecialGraphics::default()
                };

                loop {
                    // Phase 1: blocking read — wait for first bytes
                    match std::io::Read::read(&mut reader, &mut buf) {
                        Ok(0) | Err(_) => break,
                        Ok(n) => {
                            let mut all_bytes = dec_special.process(&buf[..n]);

                            // Phase 2: poll + read loop — coalesce any queued data
                            // so "clear + redraw" arrives as one parser batch.
                            #[cfg(unix)]
                            if poll_fd >= 0 {
                                loop {
                                    let mut pfd = libc::pollfd {
                                        fd: poll_fd,
                                        events: libc::POLLIN,
                                        revents: 0,
                                    };
                                    // 1ms timeout gives ncurses apps time to
                                    // flush their full clear+redraw sequence
                                    // before we finalize the batch.
                                    let ready = unsafe { libc::poll(&mut pfd as *mut _, 1, 1) };
                                    if ready > 0 && (pfd.revents & libc::POLLIN) != 0 {
                                        match std::io::Read::read(&mut reader, &mut buf) {
                                            Ok(0) | Err(_) => break,
                                            Ok(extra_n) => {
                                                let extra = dec_special.process(&buf[..extra_n]);
                                                all_bytes.extend_from_slice(&extra);
                                            }
                                        }
                                    } else {
                                        break;
                                    }
                                }
                            }

                            if all_bytes.is_empty() {
                                continue;
                            }
                            if let Ok(mut p) = parser_clone.lock() {
                                p.process(&all_bytes);
                                // Build display frame while holding the parser lock.
                                // This guarantees the snapshot is consistent — taken
                                // after the full coalesced batch has been processed.
                                let snap_cols = reader_cols.load(Ordering::Relaxed);
                                let snap_rows = reader_rows.load(Ordering::Relaxed);
                                let frame = CommittedFrame {
                                    styled: build_styled_snapshot(
                                        &p,
                                        snap_cols,
                                        snap_rows,
                                        reader_acs_mode,
                                        reader_color_mode,
                                    ),
                                    plain: build_plain_snapshot(&p, snap_cols, snap_rows),
                                    cols: snap_cols,
                                    rows: snap_rows,
                                };
                                drop(p); // release parser lock before display lock
                                if let Ok(mut d) = display_clone.lock() {
                                    *d = frame;
                                }
                                output_epoch_clone.fetch_add(1, Ordering::Relaxed);
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
            output_epoch,
            last_seen_output_epoch: 0,
            display,
            shared_cols,
            shared_rows,
            last_exit_status: None,
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

    /// Send pasted text, honoring bracketed paste mode when enabled by child app.
    #[allow(dead_code)]
    pub fn send_paste(&mut self, text: &str) {
        if text.is_empty() {
            return;
        }
        let bracketed = self
            .parser
            .lock()
            .map(|p| p.screen().bracketed_paste())
            .unwrap_or(false);
        let bytes = format_paste_bytes(text, bracketed);
        self.write(&bytes);
    }

    /// Send a translated mouse event to the PTY child using xterm SGR mouse encoding.
    pub fn send_mouse_event(
        &mut self,
        kind: MouseEventKind,
        mods: KeyModifiers,
        col: u16,
        row: u16,
    ) {
        let (mode, encoding) = self
            .parser
            .lock()
            .map(|p| {
                let screen = p.screen();
                (
                    screen.mouse_protocol_mode(),
                    screen.mouse_protocol_encoding(),
                )
            })
            .unwrap_or((
                vt100::MouseProtocolMode::None,
                vt100::MouseProtocolEncoding::Default,
            ));
        if let Some(bytes) = mouse_to_bytes(kind, mods, col, row, mode, encoding) {
            self.write(&bytes);
        }
    }

    #[allow(dead_code)]
    pub fn mouse_mode_enabled(&self) -> bool {
        self.parser
            .lock()
            .map(|p| {
                !matches!(
                    p.screen().mouse_protocol_mode(),
                    vt100::MouseProtocolMode::None
                )
            })
            .unwrap_or(false)
    }

    /// Resize the PTY and notify the child via SIGWINCH.
    /// Also updates shared dimensions so the reader thread's next
    /// committed frame uses the new size.
    pub fn resize(&mut self, cols: u16, rows: u16) {
        if cols == self.cols && rows == self.rows {
            return;
        }
        self.cols = cols;
        self.rows = rows;
        self.shared_cols.store(cols, Ordering::Relaxed);
        self.shared_rows.store(rows, Ordering::Relaxed);
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
        if self.last_exit_status.is_some() {
            return false;
        }
        match self.child.try_wait() {
            Ok(None) => true,
            Ok(Some(status)) => {
                self.last_exit_status = Some(status);
                false
            }
            Err(_) => false,
        }
    }

    #[allow(dead_code)]
    pub fn exit_status(&mut self) -> Option<ExitStatus> {
        if self.last_exit_status.is_none() {
            let _ = self.is_alive();
        }
        self.last_exit_status.clone()
    }

    pub fn exited_within(&mut self, window: Duration) -> bool {
        let started = Instant::now();
        while started.elapsed() <= window {
            if !self.is_alive() {
                return true;
            }
            std::thread::sleep(Duration::from_millis(15));
        }
        false
    }

    /// Optional PTY banner label shown above terminal content.
    #[allow(dead_code)]
    pub fn top_bar_label(&self) -> Option<&str> {
        self.top_bar.as_deref()
    }

    /// Returns true if new PTY output arrived since the last check.
    pub fn take_output_activity(&mut self) -> bool {
        let epoch = self.output_epoch.load(Ordering::Relaxed);
        if epoch != self.last_seen_output_epoch {
            self.last_seen_output_epoch = epoch;
            true
        } else {
            false
        }
    }

    /// True when PTY should favor fast plain line rendering.
    #[allow(dead_code)]
    pub fn prefers_plain_render(&self) -> bool {
        matches!(self.render_mode, PtyRenderMode::Plain)
    }

    /// Returns a clone of the output epoch counter for subscription use.
    /// The iced renderer watches this to know when a new frame is available
    /// without polling `committed_frame()` on every animation tick.
    pub fn output_epoch_arc(&self) -> Arc<AtomicU64> {
        self.output_epoch.clone()
    }

    /// Get the latest committed display frame.
    /// This is the primary API for renderers — it returns a snapshot that
    /// was built by the reader thread after a complete coalesced I/O batch,
    /// so it's guaranteed to be in a consistent (non-mid-update) state.
    pub fn committed_frame(&self) -> CommittedFrame {
        self.display
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    /// Snapshot the current screen as plain text for non-ratatui renderers.
    #[allow(dead_code)]
    pub fn snapshot_plain(&self, cols: u16, rows: u16) -> PtyTextSnapshot {
        let Ok(parser) = self.parser.lock() else {
            return PtyTextSnapshot {
                lines: vec![String::new(); rows as usize],
                cursor_row: 0,
                cursor_col: 0,
                cursor_hidden: false,
            };
        };
        let screen = parser.screen();
        let mut lines: Vec<String> = screen.rows(0, cols).take(rows as usize).collect();
        while lines.len() < rows as usize {
            lines.push(String::new());
        }
        let (cursor_row, cursor_col) = screen.cursor_position();
        PtyTextSnapshot {
            lines,
            cursor_row: cursor_row.min(rows.saturating_sub(1)),
            cursor_col: cursor_col.min(cols.saturating_sub(1)),
            cursor_hidden: screen.hide_cursor(),
        }
    }

    #[allow(dead_code)]
    pub fn snapshot_styled(&self, cols: u16, rows: u16) -> PtyStyledSnapshot {
        let Ok(parser) = self.parser.lock() else {
            return PtyStyledSnapshot {
                cells: vec![vec![]; rows as usize],
                cursor_row: 0,
                cursor_col: 0,
                cursor_hidden: false,
            };
        };
        let screen = parser.screen();
        let mut lines = Vec::with_capacity(rows as usize);
        for row in 0..rows {
            let mut out = Vec::with_capacity(cols as usize);
            for col in 0..cols {
                let cell = screen.cell(row, col);
                let ch = cell
                    .and_then(|c| c.contents().chars().next())
                    .unwrap_or(' ');
                let ch = if matches!(self.acs_mode, AcsGlyphMode::Unicode) {
                    smooth_ascii_border_char(screen, row, col, ch)
                } else {
                    ch
                };
                let (fg, bg, bold, italic, underline, reversed) = if let Some(c) = cell {
                    let (fg, bg) = cell_colors_direct(c, self.color_mode);
                    (fg, bg, c.bold(), c.italic(), c.underline(), c.inverse())
                } else {
                    (CellColor::Reset, CellColor::Black, false, false, false, false)
                };
                out.push(PtyStyledCell { ch, fg, bg, bold, italic, underline, reversed });
            }
            lines.push(out);
        }
        let (cursor_row, cursor_col) = screen.cursor_position();
        PtyStyledSnapshot {
            cells: lines,
            cursor_row: cursor_row.min(rows.saturating_sub(1)),
            cursor_col: cursor_col.min(cols.saturating_sub(1)),
            cursor_hidden: screen.hide_cursor(),
        }
    }

    /// Force-stop the PTY child process and release its resources.
    pub fn terminate(&mut self) {
        if matches!(self.child.try_wait(), Ok(None)) {
            let _ = self.child.kill();
            self.last_exit_status = self.child.try_wait().ok().flatten();
        }
    }

}

#[allow(dead_code)]
fn format_paste_bytes(text: &str, bracketed: bool) -> Vec<u8> {
    if !bracketed {
        return text.as_bytes().to_vec();
    }
    let mut out = Vec::with_capacity(text.len() + 16);
    out.extend_from_slice(b"\x1b[200~");
    out.extend_from_slice(text.as_bytes());
    out.extend_from_slice(b"\x1b[201~");
    out
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

// ── Key → bytes ───────────────────────────────────────────────────────────────

fn mouse_mod_bits(mods: KeyModifiers) -> u16 {
    let mut bits = 0u16;
    if mods.contains(KeyModifiers::SHIFT) {
        bits |= 4;
    }
    if mods.contains(KeyModifiers::ALT) || mods.contains(KeyModifiers::META) {
        bits |= 8;
    }
    if mods.contains(KeyModifiers::CONTROL) {
        bits |= 16;
    }
    bits
}

fn mouse_button_code(btn: MouseButton) -> u16 {
    match btn {
        MouseButton::Left => 0,
        MouseButton::Middle => 1,
        MouseButton::Right => 2,
    }
}

fn mouse_event_cb(kind: MouseEventKind, mods: KeyModifiers) -> Option<(u16, char)> {
    let mut cb = mouse_mod_bits(mods);
    let suffix = match kind {
        MouseEventKind::Down(btn) => {
            cb |= mouse_button_code(btn);
            'M'
        }
        MouseEventKind::Up(_) => {
            // X10/SGR release is button code 3, not the released button id.
            cb |= 3;
            'm'
        }
        MouseEventKind::Drag(btn) => {
            cb |= mouse_button_code(btn) | 32;
            'M'
        }
        MouseEventKind::ScrollUp => {
            cb |= 64;
            'M'
        }
        MouseEventKind::ScrollDown => {
            cb |= 65;
            'M'
        }
        MouseEventKind::ScrollLeft => {
            cb |= 66;
            'M'
        }
        MouseEventKind::ScrollRight => {
            cb |= 67;
            'M'
        }
        MouseEventKind::Moved => {
            // Any-motion event with no button held.
            cb |= 35;
            'M'
        }
    };
    Some((cb, suffix))
}

fn mouse_mode_allows(kind: MouseEventKind, mode: vt100::MouseProtocolMode) -> bool {
    use vt100::MouseProtocolMode as M;
    match mode {
        M::None => false,
        M::Press => matches!(
            kind,
            MouseEventKind::Down(_)
                | MouseEventKind::ScrollUp
                | MouseEventKind::ScrollDown
                | MouseEventKind::ScrollLeft
                | MouseEventKind::ScrollRight
        ),
        M::PressRelease => matches!(
            kind,
            MouseEventKind::Down(_)
                | MouseEventKind::Up(_)
                | MouseEventKind::ScrollUp
                | MouseEventKind::ScrollDown
                | MouseEventKind::ScrollLeft
                | MouseEventKind::ScrollRight
        ),
        M::ButtonMotion => matches!(
            kind,
            MouseEventKind::Down(_)
                | MouseEventKind::Up(_)
                | MouseEventKind::Drag(_)
                | MouseEventKind::ScrollUp
                | MouseEventKind::ScrollDown
                | MouseEventKind::ScrollLeft
                | MouseEventKind::ScrollRight
        ),
        M::AnyMotion => true,
    }
}

fn push_x10_coord(out: &mut Vec<u8>, value: u16) {
    let v = value.max(1).saturating_add(32).min(255) as u8;
    out.push(v);
}

fn push_utf8_coord(out: &mut Vec<u8>, value: u16) {
    let v = value.max(1).saturating_add(32) as u32;
    if let Some(ch) = char::from_u32(v) {
        let mut buf = [0u8; 4];
        out.extend_from_slice(ch.encode_utf8(&mut buf).as_bytes());
    }
}

fn mouse_to_bytes(
    kind: MouseEventKind,
    mods: KeyModifiers,
    col: u16,
    row: u16,
    mode: vt100::MouseProtocolMode,
    encoding: vt100::MouseProtocolEncoding,
) -> Option<Vec<u8>> {
    if !mouse_mode_allows(kind, mode) {
        return None;
    }
    let (cb, suffix) = mouse_event_cb(kind, mods)?;
    let x = col.max(1);
    let y = row.max(1);
    match encoding {
        vt100::MouseProtocolEncoding::Sgr => {
            Some(format!("\x1b[<{cb};{x};{y}{suffix}").into_bytes())
        }
        vt100::MouseProtocolEncoding::Default => {
            let mut out = Vec::with_capacity(6);
            out.extend_from_slice(b"\x1b[M");
            out.push(cb.saturating_add(32).min(255) as u8);
            push_x10_coord(&mut out, x);
            push_x10_coord(&mut out, y);
            Some(out)
        }
        vt100::MouseProtocolEncoding::Utf8 => {
            let mut out = Vec::with_capacity(16);
            out.extend_from_slice(b"\x1b[M");
            push_utf8_coord(&mut out, cb.saturating_add(32));
            push_utf8_coord(&mut out, x);
            push_utf8_coord(&mut out, y);
            Some(out)
        }
    }
}

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

#[cfg(test)]
mod tests {
    use super::{
        format_paste_bytes, key_to_bytes, mouse_to_bytes, needs_ncurses_ascii_acs,
        smooth_ascii_border_char, DecSpecialGraphics,
    };
    use crossterm::event::{KeyCode, KeyModifiers, MouseButton, MouseEventKind};
    use vt100::{MouseProtocolEncoding, MouseProtocolMode};

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

    #[test]
    fn mouse_release_uses_release_code_in_sgr() {
        let bytes = mouse_to_bytes(
            MouseEventKind::Up(MouseButton::Left),
            KeyModifiers::NONE,
            10,
            5,
            MouseProtocolMode::PressRelease,
            MouseProtocolEncoding::Sgr,
        )
        .expect("mouse bytes");
        assert_eq!(String::from_utf8_lossy(&bytes), "\u{1b}[<3;10;5m");
    }

    #[test]
    fn mouse_events_are_suppressed_when_mode_is_none() {
        let bytes = mouse_to_bytes(
            MouseEventKind::Down(MouseButton::Left),
            KeyModifiers::NONE,
            1,
            1,
            MouseProtocolMode::None,
            MouseProtocolEncoding::Sgr,
        );
        assert!(bytes.is_none());
    }

    #[test]
    fn paste_bytes_are_wrapped_only_in_bracketed_mode() {
        assert_eq!(format_paste_bytes("abc\n", false), b"abc\n".to_vec());
        assert_eq!(
            format_paste_bytes("abc\n", true),
            b"\x1b[200~abc\n\x1b[201~".to_vec()
        );
    }
}
