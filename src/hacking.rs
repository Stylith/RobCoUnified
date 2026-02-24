use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::{
    layout::{Alignment, Rect},
    style::Style,
    widgets::Paragraph,
};
use rand::Rng;
use std::collections::HashSet;
use std::time::Duration;

use crate::config::current_theme_color;
use crate::status::render_status_bar;
use crate::ui::{Term, sel_style, dim_style};

const H_PAD: u16 = 3;

// ── Constants ─────────────────────────────────────────────────────────────────

const WORD_LEN:   usize = 5;
const COLS:       usize = 2;
const ROWS:       usize = 16;
const COL_WIDTH:  usize = 12;
const NUM_WORDS:  usize = 10;
const MAX_TRIES:  usize = 4;

const JUNK: &[char] = &[
    '!','@','#','$','%','^','&','*','-','+','=','[',']','{','}','|',';',':',
    '\'',',','.','<','>','?','/','\\','~','`',
];

const WORD_BANK: &[&str] = &[
    "CRANE","FLAME","BLADE","SHORE","GRIME","BRUTE","STALE","PRIME",
    "GRIND","PLANK","FLASK","CRAMP","BLAZE","SCORN","TROVE","PHASE",
    "CLAMP","SNARE","GROAN","FLINT","BRICK","CRAVE","DRONE","SCALP",
    "BLUNT","CRISP","PROWL","SLICK","KNAVE","FRAIL","STOVE","GRASP",
    "CLEFT","BRAND","SMIRK","TRAMP","GLARE","SPOUT","DWARF","BRAID",
    "TANKS","THIRD","TRIES","TIRES","TERMS","TEXAS","TRITE","TRIBE",
    "VAULT","POWER","STEEL","LASER","NERVE","FORCE","GUARD","WATCH",
    "DEATH","BLOOD","GHOST","STORM","NIGHT","FLESH","SKULL","WASTE",
];

const OPENERS: &[char] = &['(', '[', '<', '{'];
const CLOSERS: &[char] = &[')', ']', '>', '}'];

// ── Grid state ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
struct WordPos { start: usize, word: String }

#[derive(Debug, Clone)]
struct BracketPair { open: usize, close: usize }

struct Grid {
    chars:          Vec<char>,
    word_positions: Vec<WordPos>,
    bracket_pairs:  Vec<BracketPair>,
}

fn junk_char(rng: &mut impl Rng) -> char { JUNK[rng.gen_range(0..JUNK.len())] }

fn likeness(guess: &str, answer: &str) -> usize {
    guess.chars().zip(answer.chars()).filter(|(a, b)| a == b).count()
}

fn build_grid(answer: &str) -> Grid {
    let mut rng = rand::thread_rng();
    let total   = COLS * ROWS * COL_WIDTH;
    let mut chars: Vec<char> = (0..total).map(|_| junk_char(&mut rng)).collect();

    let mut pool: Vec<&str> = WORD_BANK.iter().copied().filter(|&w| w != answer).collect();
    shuffle_vec(&mut pool, &mut rng);
    let mut words: Vec<String> = pool[..NUM_WORDS.saturating_sub(1)]
        .iter().map(|s| s.to_string()).collect();
    words.push(answer.to_string());
    shuffle_vec(&mut words, &mut rng);

    let mut used: HashSet<usize> = HashSet::new();
    let mut word_positions: Vec<WordPos> = Vec::new();

    for word in &words {
        let mut placed = false;
        for _ in 0..200 {
            let row    = rng.gen_range(0..ROWS * COLS);
            let col_s  = rng.gen_range(0..COL_WIDTH - WORD_LEN + 1);
            let start  = row * COL_WIDTH + col_s;
            let idxs: Vec<usize> = (start..start + WORD_LEN).collect();
            if !idxs.iter().any(|i| used.contains(i)) {
                for (i, ch) in idxs.iter().zip(word.chars()) { chars[*i] = ch; }
                used.extend(idxs);
                word_positions.push(WordPos { start, word: word.clone() });
                placed = true;
                break;
            }
        }
        if !placed {
            let start = rng.gen_range(0..total - WORD_LEN);
            for (i, ch) in word.chars().enumerate() { chars[start + i] = ch; }
            word_positions.push(WordPos { start, word: word.clone() });
        }
    }

    let mut bracket_pairs: Vec<BracketPair> = Vec::new();
    for row in 0..ROWS * COLS {
        if rng.gen_bool(0.3) {
            let row_start = row * COL_WIDTH;
            let max_span  = COL_WIDTH - 2;
            let span = rng.gen_range(1..=max_span.max(1));
            let op   = rng.gen_range(row_start..row_start + COL_WIDTH - span - 1);
            let cl   = op + span + 1;
            if !used.contains(&op) && !used.contains(&cl) {
                let kind = rng.gen_range(0..4);
                chars[op] = OPENERS[kind];
                chars[cl] = CLOSERS[kind];
                bracket_pairs.push(BracketPair { open: op, close: cl });
            }
        }
    }

    Grid { chars, word_positions, bracket_pairs }
}

fn shuffle_vec<T>(v: &mut [T], rng: &mut impl Rng) {
    use rand::seq::SliceRandom;
    v.shuffle(rng);
}

// ── Index ↔ screen position ───────────────────────────────────────────────────

fn idx_to_cell(idx: usize) -> (usize, usize) {
    let col_block   = idx / (ROWS * COL_WIDTH);
    let within      = idx % (ROWS * COL_WIDTH);
    let row_in_col  = within / COL_WIDTH;
    let char_in_row = within % COL_WIDTH;
    let scr_col = 7 + col_block * (COL_WIDTH + 14) + char_in_row;
    (row_in_col, scr_col)
}

fn find_word_at(idx: usize, positions: &[WordPos]) -> Option<&WordPos> {
    positions.iter().find(|wp| idx >= wp.start && idx < wp.start + WORD_LEN)
}

fn find_bracket_at(idx: usize, pairs: &[BracketPair]) -> Option<&BracketPair> {
    pairs.iter().find(|bp| idx == bp.open || idx == bp.close)
}

// ── Overlay screens ───────────────────────────────────────────────────────────

fn draw_security_override(terminal: &mut Term) -> Result<()> {
    terminal.draw(|f| {
        let size = f.area();
        let fg = current_theme_color();
        let ns = Style::default().fg(fg);
        f.render_widget(Paragraph::new("").style(ns), size);
        let mid = size.height / 2;
        let line = Paragraph::new("SECURITY OVERRIDE")
            .alignment(Alignment::Center)
            .style(ns);
        f.render_widget(line, Rect { x: H_PAD, y: mid.saturating_sub(1), width: size.width.saturating_sub(H_PAD*2), height: 1 });
    })?;
    std::thread::sleep(Duration::from_millis(1200));
    Ok(())
}

/// "TERMINAL LOCKED" screen — plain themed text, no highlight.
/// Waits for Enter, then returns (user goes back to user select).
pub fn draw_terminal_locked(terminal: &mut Term) -> Result<()> {
    loop {
        terminal.draw(|f| {
            let size = f.area();
            let fg = current_theme_color();
            let ns = Style::default().fg(fg);
            let ds = dim_style();

            f.render_widget(Paragraph::new("").style(ns), size);

            let mid = size.height / 2;

            let line1 = Paragraph::new("TERMINAL LOCKED")
                .alignment(Alignment::Center)
                .style(ns);
            f.render_widget(line1, Rect { x: H_PAD, y: mid.saturating_sub(2), width: size.width.saturating_sub(H_PAD*2), height: 1 });

            let line2 = Paragraph::new("PLEASE CONTACT AN ADMINISTRATOR")
                .alignment(Alignment::Center)
                .style(ns);
            f.render_widget(line2, Rect { x: H_PAD, y: mid, width: size.width.saturating_sub(H_PAD*2), height: 1 });

            let line3 = Paragraph::new("[ Press ENTER to Exit ]")
                .alignment(Alignment::Center)
                .style(ds);
            f.render_widget(line3, Rect { x: H_PAD, y: mid + 2, width: size.width.saturating_sub(H_PAD*2), height: 1 });

            render_status_bar(f, Rect { x: 0, y: size.height.saturating_sub(1), width: size.width, height: 1 });
        })?;

        if event::poll(Duration::from_millis(200))? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press { continue; }
                if matches!(key.code, KeyCode::Enter | KeyCode::Char(' ')) {
                    crate::sound::play_navigate();
                    break;
                }
            }
        }
    }
    Ok(())
}

// ── Draw the hacking grid (shared between loop and success delay) ─────────────

fn draw_grid_frame(
    terminal: &mut Term,
    grid: &Grid,
    log: &[String],
    cursor: usize,
    attempts: usize,
    removed_duds: &HashSet<String>,
    base_addr: u16,
) -> Result<()> {
    terminal.draw(|f| {
        let size = f.area();
        let fg   = current_theme_color();
        let ns   = Style::default().fg(fg);
        let ss   = sel_style();
        let ds   = dim_style();

        // Header
        let hdr = Paragraph::new("ROBCO INDUSTRIES (TM) TERMLINK PROTOCOL")
            .alignment(Alignment::Center)
            .style(ns);
        f.render_widget(hdr, Rect { x:H_PAD, y:0, width:size.width.saturating_sub(H_PAD*2), height:1 });

        // Attempts
        let boxes: String = "■ ".repeat(attempts) + &"□ ".repeat(MAX_TRIES - attempts);
        let warn = if attempts <= 1 {
            format!("!!! WARNING: LOCKOUT IMMINENT !!!  {}", boxes.trim())
        } else {
            format!("{} ATTEMPT(S) LEFT:  {}", attempts, boxes.trim())
        };
        f.render_widget(Paragraph::new(warn).style(ns), Rect { x:H_PAD, y:2, width:size.width.saturating_sub(H_PAD*2), height:1 });

        // Hint
        f.render_widget(
            Paragraph::new("TAB=Next Column  q=cancel").style(ds),
            Rect { x:H_PAD, y:size.height.saturating_sub(2), width:size.width.saturating_sub(H_PAD*2), height:1 },
        );

        let hover_word    = find_word_at(cursor, &grid.word_positions);
        let hover_bracket = find_bracket_at(cursor, &grid.bracket_pairs);

        let base_row: u16 = 5;
        // Address labels
        for col_block in 0..COLS {
            for row in 0..ROWS {
                let addr = base_addr + ((col_block * ROWS + row) * COL_WIDTH) as u16;
                let sx = H_PAD + (col_block * (COL_WIDTH + 14)) as u16;
                let sy = base_row + row as u16;
                if sy >= size.height.saturating_sub(2) { continue; }
                f.render_widget(
                    Paragraph::new(format!("0x{addr:04X}")).style(ds),
                    Rect { x:sx, y:sy, width:6, height:1 },
                );
            }
        }

        // Grid characters
        for (i, &ch) in grid.chars.iter().enumerate() {
            let (row_off, col_off) = idx_to_cell(i);
            let sy = base_row + row_off as u16;
            let sx = H_PAD + col_off as u16;
            if sy >= size.height.saturating_sub(2) || sx >= size.width { continue; }

            let is_removed = grid.word_positions.iter().any(|wp| {
                removed_duds.contains(&wp.word) && i >= wp.start && i < wp.start + WORD_LEN
            });

            let highlighted =
                hover_word.is_some_and(|hw| i >= hw.start && i < hw.start + WORD_LEN)
                || hover_bracket.is_some_and(|hb| i >= hb.open && i <= hb.close)
                || i == cursor;

            let (style, display) = if is_removed {
                (ds, '.')
            } else if highlighted {
                (ss, ch)
            } else {
                (ns, ch)
            };

            f.render_widget(
                Paragraph::new(display.to_string()).style(style),
                Rect { x:sx, y:sy, width:1, height:1 },
            );
        }

        // Right panel log
        let panel_col = H_PAD + (7 + (COL_WIDTH + 14) + COL_WIDTH + 4) as u16;
        for (li, entry) in log.iter().rev().take(ROWS).enumerate() {
            let sy = base_row + (ROWS - 1 - li) as u16;
            if panel_col < size.width && sy < size.height.saturating_sub(1) {
                f.render_widget(
                    Paragraph::new(entry.as_str()).style(ns),
                    Rect { x:panel_col, y:sy, width:size.width.saturating_sub(panel_col), height:1 },
                );
            }
        }

        render_status_bar(f, Rect { x:0, y:size.height.saturating_sub(1), width:size.width, height:1 });
    })?;
    Ok(())
}

// ── Minigame entry point ──────────────────────────────────────────────────────

pub fn run_hacking(terminal: &mut Term) -> Result<bool> {
    draw_security_override(terminal)?;

    let mut rng    = rand::thread_rng();
    let answer_idx = rng.gen_range(0..WORD_BANK.len());
    let answer     = WORD_BANK[answer_idx].to_string();

    let mut grid         = build_grid(&answer);
    let total            = COLS * ROWS * COL_WIDTH;
    let mut cursor       = 0usize;
    let mut attempts     = MAX_TRIES;
    let mut log: Vec<String> = Vec::new();
    let mut removed_duds: HashSet<String> = HashSet::new();
    let mut duds_left: Vec<String> = grid.word_positions.iter()
        .filter(|wp| wp.word != answer)
        .map(|wp| wp.word.clone())
        .collect();

    let base_addr: u16 = 0xF964;

    loop {
        draw_grid_frame(terminal, &grid, &log, cursor, attempts, &removed_duds, base_addr)?;

        if !event::poll(Duration::from_millis(100))? { continue; }
        let ev = event::read()?;
        let Event::Key(key) = ev else { continue };
        if key.kind != KeyEventKind::Press { continue; }

        if crate::ui::check_session_switch_pub(key.code, key.modifiers) {
            if crate::session::has_switch_request() {
                return Ok(false);
            }
            continue;
        }

        match key.code {
            KeyCode::Right | KeyCode::Char('d') => {
                cursor = (cursor + 1) % total;
                crate::sound::play_navigate();
            }
            KeyCode::Left | KeyCode::Char('a') => {
                cursor = cursor.checked_sub(1).unwrap_or(total - 1);
                crate::sound::play_navigate();
            }
            KeyCode::Down | KeyCode::Char('s') => {
                let (cb, within) = (cursor / (ROWS * COL_WIDTH), cursor % (ROWS * COL_WIDTH));
                let row = (within / COL_WIDTH + 1) % ROWS;
                let chr = within % COL_WIDTH;
                cursor = cb * ROWS * COL_WIDTH + row * COL_WIDTH + chr;
                crate::sound::play_navigate();
            }
            KeyCode::Up | KeyCode::Char('w') => {
                let (cb, within) = (cursor / (ROWS * COL_WIDTH), cursor % (ROWS * COL_WIDTH));
                let row = (within / COL_WIDTH + ROWS - 1) % ROWS;
                let chr = within % COL_WIDTH;
                cursor = cb * ROWS * COL_WIDTH + row * COL_WIDTH + chr;
                crate::sound::play_navigate();
            }
            KeyCode::Tab => {
                let cb     = cursor / (ROWS * COL_WIDTH);
                let within = cursor % (ROWS * COL_WIDTH);
                let new_cb = (cb + 1) % COLS;
                cursor = new_cb * ROWS * COL_WIDTH + within % (ROWS * COL_WIDTH);
                crate::sound::play_navigate();
            }
            KeyCode::Enter | KeyCode::Char(' ') => {
                crate::sound::play_navigate();
                let sel_word    = find_word_at(cursor, &grid.word_positions).cloned();
                let sel_bracket = find_bracket_at(cursor, &grid.bracket_pairs).cloned();

                if let Some(wp) = sel_word {
                    if !removed_duds.contains(&wp.word) {
                        log.push(format!(">{}", wp.word));
                        if wp.word == answer {
                            // Show ">Exact match!" in the log panel and redraw
                            log.push(">Exact match!".to_string());
                            crate::sound::play_login();
                            draw_grid_frame(terminal, &grid, &log, cursor, attempts, &removed_duds, base_addr)?;
                            std::thread::sleep(Duration::from_millis(900));
                            // Then add ">Please wait..." and redraw again
                            log.push(">Please wait...".to_string());
                            draw_grid_frame(terminal, &grid, &log, cursor, attempts, &removed_duds, base_addr)?;
                            std::thread::sleep(Duration::from_millis(1400));
                            return Ok(true);
                        } else {
                            let lk = likeness(&wp.word, &answer);
                            log.push(">Entry denied.".to_string());
                            log.push(format!(">{lk}/{WORD_LEN} correct."));
                            attempts = attempts.saturating_sub(1);
                            if attempts == 0 {
                                log.push(">LOCKED OUT.".to_string());
                                crate::sound::play_error();
                                draw_terminal_locked(terminal)?;
                                return Ok(false);
                            }
                        }
                    }
                } else if let Some(bp) = sel_bracket {
                    grid.bracket_pairs.retain(|b| b.open != bp.open);
                    let mut rng = rand::thread_rng();
                    if !duds_left.is_empty() && rng.gen_bool(0.5) {
                        let idx = rng.gen_range(0..duds_left.len());
                        let dud = duds_left.remove(idx);
                        removed_duds.insert(dud);
                        log.push(">Dud removed.".to_string());
                    } else if attempts < MAX_TRIES {
                        attempts = (attempts + 1).min(MAX_TRIES);
                        log.push(">Tries reset.".to_string());
                    } else {
                        log.push(">No effect.".to_string());
                    }
                }
            }
            KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                crate::sound::play_navigate();
                return Ok(false);
            }
            _ => {}
        }
    }
}
