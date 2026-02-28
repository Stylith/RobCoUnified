use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use rand::Rng;
use ratatui::{
    layout::{Alignment, Rect},
    style::Style,
    widgets::Paragraph,
};
use std::collections::HashSet;
use std::time::Duration;

use crate::config::{
    current_theme_color, get_settings, hacking_difficulty_label, HackingDifficulty,
};
use crate::status::render_status_bar;
use crate::ui::{dim_style, sel_style, Term};

const H_PAD: u16 = 3;

// ── Constants ─────────────────────────────────────────────────────────────────

const COLS: usize = 2;
const ROWS: usize = 16;
const COL_WIDTH: usize = 12;

const JUNK: &[char] = &[
    '!', '@', '#', '$', '%', '^', '&', '*', '-', '+', '=', '[', ']', '{', '}', '|', ';', ':', '\'',
    ',', '.', '<', '>', '?', '/', '\\', '~', '`',
];

const WORD_BANK_4: &[&str] = &[
    "ATOM", "BOLT", "CODE", "DATA", "DOOM", "FUSE", "GATE", "GRID", "HACK", "LOCK", "MESH", "NODE",
    "OMEN", "PING", "SEAL", "WIRE",
];

const WORD_BANK_5: &[&str] = &[
    "CRANE", "FLAME", "BLADE", "SHORE", "GRIME", "BRUTE", "STALE", "PRIME", "GRIND", "PLANK",
    "FLASK", "CRAMP", "BLAZE", "SCORN", "TROVE", "PHASE", "CLAMP", "SNARE", "GROAN", "FLINT",
    "BRICK", "CRAVE", "DRONE", "SCALP", "BLUNT", "CRISP", "PROWL", "SLICK", "KNAVE", "FRAIL",
    "STOVE", "GRASP", "CLEFT", "BRAND", "SMIRK", "TRAMP", "GLARE", "SPOUT", "DWARF", "BRAID",
    "TANKS", "THIRD", "TRIES", "TIRES", "TERMS", "TEXAS", "TRITE", "TRIBE", "VAULT", "POWER",
    "STEEL", "LASER", "NERVE", "FORCE", "GUARD", "WATCH", "DEATH", "BLOOD", "GHOST", "STORM",
    "NIGHT", "FLESH", "SKULL", "WASTE",
];

const WORD_BANK_6: &[&str] = &[
    "ACCESS", "BATTLE", "CIPHER", "DANGER", "ENCLAV", "FUSION", "GLITCH", "HANDLE", "INJECT",
    "JAMMER", "KERNEL", "LEGION", "MATRIX", "MODULE", "PACKET", "QUARTZ", "RADIOS", "SECTOR",
    "TARGET", "VECTOR",
];

const OPENERS: &[char] = &['(', '[', '<', '{'];
const CLOSERS: &[char] = &[')', ']', '>', '}'];

// ── Grid state ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
struct WordPos {
    start: usize,
    word: String,
}

#[derive(Debug, Clone)]
struct BracketPair {
    open: usize,
    close: usize,
}

struct Grid {
    chars: Vec<char>,
    word_positions: Vec<WordPos>,
    bracket_pairs: Vec<BracketPair>,
}

#[derive(Debug, Clone, Copy)]
struct HackingProfile {
    difficulty: HackingDifficulty,
    word_len: usize,
    num_words: usize,
    max_tries: usize,
    bracket_chance: f64,
    dud_remove_chance: f64,
}

#[derive(Debug, Clone, Copy)]
struct HackingLayout {
    body_top: u16,
    hint_y: u16,
    status_y: u16,
    visible_rows: usize,
    row_window_start: usize,
    log_x: Option<u16>,
}

fn junk_char(rng: &mut impl Rng) -> char {
    JUNK[rng.gen_range(0..JUNK.len())]
}

fn likeness(guess: &str, answer: &str) -> usize {
    guess
        .chars()
        .zip(answer.chars())
        .filter(|(a, b)| a == b)
        .count()
}

fn hacking_profile(difficulty: HackingDifficulty) -> HackingProfile {
    match difficulty {
        HackingDifficulty::Easy => HackingProfile {
            difficulty,
            word_len: 4,
            num_words: 8,
            max_tries: 5,
            bracket_chance: 0.38,
            dud_remove_chance: 0.65,
        },
        HackingDifficulty::Normal => HackingProfile {
            difficulty,
            word_len: 5,
            num_words: 10,
            max_tries: 4,
            bracket_chance: 0.30,
            dud_remove_chance: 0.50,
        },
        HackingDifficulty::Hard => HackingProfile {
            difficulty,
            word_len: 6,
            num_words: 11,
            max_tries: 3,
            bracket_chance: 0.22,
            dud_remove_chance: 0.35,
        },
    }
}

fn word_bank_for_len(word_len: usize) -> &'static [&'static str] {
    match word_len {
        4 => WORD_BANK_4,
        5 => WORD_BANK_5,
        6 => WORD_BANK_6,
        _ => WORD_BANK_5,
    }
}

fn word_reserved_range(start: usize, word_len: usize) -> std::ops::Range<usize> {
    let col = start % COL_WIDTH;
    let left = if col > 0 { start - 1 } else { start };
    let right = if col + word_len < COL_WIDTH {
        start + word_len + 1
    } else {
        start + word_len
    };
    left..right
}

fn can_place_word_at(start: usize, word_len: usize, used: &HashSet<usize>) -> bool {
    word_reserved_range(start, word_len).all(|idx| !used.contains(&idx))
}

fn reserve_word_span(used: &mut HashSet<usize>, start: usize, word_len: usize) {
    used.extend(word_reserved_range(start, word_len));
}

fn candidate_word_starts(word_len: usize) -> Vec<usize> {
    let mut starts = Vec::with_capacity(ROWS * COLS * (COL_WIDTH - word_len + 1));
    for row in 0..ROWS * COLS {
        for col in 0..=COL_WIDTH - word_len {
            starts.push(row * COL_WIDTH + col);
        }
    }
    starts
}

fn count_word_occurrences(chars: &[char], word: &str) -> usize {
    let word_len = word.chars().count();
    let target: Vec<char> = word.chars().collect();
    let mut matches = 0usize;
    for row in 0..ROWS * COLS {
        let row_start = row * COL_WIDTH;
        for col in 0..=COL_WIDTH.saturating_sub(word_len) {
            let start = row_start + col;
            if chars[start..start + word_len] == target[..] {
                matches += 1;
            }
        }
    }
    matches
}

fn scrub_accidental_answer_matches(
    chars: &mut [char],
    protected: &HashSet<usize>,
    answer: &str,
    answer_start: usize,
    rng: &mut impl Rng,
) {
    let word_len = answer.chars().count();
    let target: Vec<char> = answer.chars().collect();
    for row in 0..ROWS * COLS {
        let row_start = row * COL_WIDTH;
        for col in 0..=COL_WIDTH.saturating_sub(word_len) {
            let start = row_start + col;
            if start == answer_start {
                continue;
            }
            if chars[start..start + word_len] != target[..] {
                continue;
            }
            if let Some(idx) = (start..start + word_len).find(|idx| !protected.contains(idx)) {
                let expected = target[idx - start];
                let mut replacement = junk_char(rng);
                while replacement == expected {
                    replacement = junk_char(rng);
                }
                chars[idx] = replacement;
            }
        }
    }
}

fn build_grid(answer: &str, profile: HackingProfile) -> Grid {
    let mut rng = rand::thread_rng();
    let total = COLS * ROWS * COL_WIDTH;
    let mut chars: Vec<char> = (0..total).map(|_| junk_char(&mut rng)).collect();

    let mut pool: Vec<&str> = word_bank_for_len(profile.word_len)
        .iter()
        .copied()
        .filter(|&w| w != answer)
        .collect();
    shuffle_vec(&mut pool, &mut rng);
    let mut words: Vec<String> = pool[..profile.num_words.saturating_sub(1)]
        .iter()
        .map(|s| s.to_string())
        .collect();
    words.push(answer.to_string());
    shuffle_vec(&mut words, &mut rng);

    let mut used: HashSet<usize> = HashSet::new();
    let mut word_positions: Vec<WordPos> = Vec::new();

    for word in &words {
        let word_len = word.chars().count();
        let mut starts = candidate_word_starts(word_len);
        shuffle_vec(&mut starts, &mut rng);
        let start = starts
            .into_iter()
            .find(|&start| can_place_word_at(start, word_len, &used))
            .expect("hacking grid should have space for separated words");
        for (i, ch) in word.chars().enumerate() {
            chars[start + i] = ch;
        }
        reserve_word_span(&mut used, start, word_len);
        word_positions.push(WordPos {
            start,
            word: word.clone(),
        });
    }

    let mut bracket_pairs: Vec<BracketPair> = Vec::new();
    for row in 0..ROWS * COLS {
        if rng.gen_bool(profile.bracket_chance) {
            let row_start = row * COL_WIDTH;
            let max_span = COL_WIDTH - 2;
            let span = rng.gen_range(1..=max_span.max(1));
            let op = rng.gen_range(row_start..row_start + COL_WIDTH - span - 1);
            let cl = op + span + 1;
            if !used.contains(&op) && !used.contains(&cl) {
                let kind = rng.gen_range(0..4);
                chars[op] = OPENERS[kind];
                chars[cl] = CLOSERS[kind];
                bracket_pairs.push(BracketPair {
                    open: op,
                    close: cl,
                });
            }
        }
    }

    let protected: HashSet<usize> = word_positions
        .iter()
        .flat_map(|wp| wp.start..wp.start + wp.word.chars().count())
        .collect();
    let answer_start = word_positions
        .iter()
        .find(|wp| wp.word == answer)
        .map(|wp| wp.start)
        .expect("answer word should exist in grid");
    scrub_accidental_answer_matches(&mut chars, &protected, answer, answer_start, &mut rng);
    debug_assert_eq!(count_word_occurrences(&chars, answer), 1);

    Grid {
        chars,
        word_positions,
        bracket_pairs,
    }
}

fn shuffle_vec<T>(v: &mut [T], rng: &mut impl Rng) {
    use rand::seq::SliceRandom;
    v.shuffle(rng);
}

// ── Index ↔ screen position ───────────────────────────────────────────────────

fn idx_to_cell(idx: usize) -> (usize, usize) {
    let col_block = idx / (ROWS * COL_WIDTH);
    let within = idx % (ROWS * COL_WIDTH);
    let row_in_col = within / COL_WIDTH;
    let char_in_row = within % COL_WIDTH;
    let scr_col = 7 + col_block * (COL_WIDTH + 14) + char_in_row;
    (row_in_col, scr_col)
}

fn find_word_at(idx: usize, positions: &[WordPos]) -> Option<&WordPos> {
    positions
        .iter()
        .find(|wp| idx >= wp.start && idx < wp.start + wp.word.chars().count())
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
        f.render_widget(
            line,
            Rect {
                x: H_PAD,
                y: mid.saturating_sub(1),
                width: size.width.saturating_sub(H_PAD * 2),
                height: 1,
            },
        );
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
            f.render_widget(
                line1,
                Rect {
                    x: H_PAD,
                    y: mid.saturating_sub(2),
                    width: size.width.saturating_sub(H_PAD * 2),
                    height: 1,
                },
            );

            let line2 = Paragraph::new("PLEASE CONTACT AN ADMINISTRATOR")
                .alignment(Alignment::Center)
                .style(ns);
            f.render_widget(
                line2,
                Rect {
                    x: H_PAD,
                    y: mid,
                    width: size.width.saturating_sub(H_PAD * 2),
                    height: 1,
                },
            );

            let line3 = Paragraph::new("[ Press ENTER to Exit ]")
                .alignment(Alignment::Center)
                .style(ds);
            f.render_widget(
                line3,
                Rect {
                    x: H_PAD,
                    y: size.height.saturating_sub(3),
                    width: size.width.saturating_sub(H_PAD * 2),
                    height: 1,
                },
            );

            render_status_bar(
                f,
                Rect {
                    x: 0,
                    y: size.height.saturating_sub(1),
                    width: size.width,
                    height: 1,
                },
            );
        })?;

        if event::poll(Duration::from_millis(200))? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }
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
    profile: HackingProfile,
    log: &[String],
    cursor: usize,
    attempts: usize,
    removed_duds: &HashSet<String>,
    base_addr: u16,
) -> Result<()> {
    terminal.draw(|f| {
        let size = f.area();
        let fg = current_theme_color();
        let ns = Style::default().fg(fg);
        let ss = sel_style();
        let ds = dim_style();
        let cursor_row = (cursor % (ROWS * COL_WIDTH)) / COL_WIDTH;
        let visible_rows = size.height.saturating_sub(7).clamp(1, ROWS as u16) as usize;
        let row_window_start = cursor_row
            .saturating_sub(visible_rows.saturating_sub(1) / 2)
            .min(ROWS.saturating_sub(visible_rows));
        let log_x = {
            let x = H_PAD + (7 + (COL_WIDTH + 14) + COL_WIDTH + 4) as u16;
            (x + 12 < size.width).then_some(x)
        };
        let layout = HackingLayout {
            body_top: 5,
            hint_y: size.height.saturating_sub(2),
            status_y: size.height.saturating_sub(1),
            visible_rows,
            row_window_start,
            log_x,
        };

        // Header
        let hdr = Paragraph::new("ROBCO INDUSTRIES (TM) TERMLINK PROTOCOL")
            .alignment(Alignment::Center)
            .style(ns);
        f.render_widget(
            hdr,
            Rect {
                x: H_PAD,
                y: 0,
                width: size.width.saturating_sub(H_PAD * 2),
                height: 1,
            },
        );

        // Attempts
        let boxes: String = "■ ".repeat(attempts) + &"□ ".repeat(profile.max_tries - attempts);
        let warn = if attempts <= 1 {
            format!("!!! WARNING: LOCKOUT IMMINENT !!!  {}", boxes.trim())
        } else {
            format!("{} ATTEMPT(S) LEFT:  {}", attempts, boxes.trim())
        };
        f.render_widget(
            Paragraph::new(warn).style(ns),
            Rect {
                x: H_PAD,
                y: 2,
                width: size.width.saturating_sub(H_PAD * 2),
                height: 1,
            },
        );

        let meta = format!(
            "Difficulty: {} | {} candidates | {}-letter words",
            hacking_difficulty_label(profile.difficulty),
            grid.word_positions.len(),
            profile.word_len
        );
        f.render_widget(
            Paragraph::new(meta).style(ds),
            Rect {
                x: H_PAD,
                y: 3,
                width: size.width.saturating_sub(H_PAD * 2),
                height: 1,
            },
        );

        // Footer hint
        f.render_widget(
            Paragraph::new("Enter select | Tab cancel | Arrows move").style(ds),
            Rect {
                x: H_PAD,
                y: layout.hint_y,
                width: size.width.saturating_sub(H_PAD * 2),
                height: 1,
            },
        );

        let hover_word = find_word_at(cursor, &grid.word_positions);
        let hover_bracket = find_bracket_at(cursor, &grid.bracket_pairs);

        let base_row = layout.body_top;
        let visible_end = layout.row_window_start + layout.visible_rows;
        // Address labels
        for col_block in 0..COLS {
            for row in layout.row_window_start..visible_end {
                let addr = base_addr + ((col_block * ROWS + row) * COL_WIDTH) as u16;
                let sx = H_PAD + (col_block * (COL_WIDTH + 14)) as u16;
                let sy = base_row + (row - layout.row_window_start) as u16;
                if sy >= layout.hint_y {
                    continue;
                }
                f.render_widget(
                    Paragraph::new(format!("0x{addr:04X}")).style(ds),
                    Rect {
                        x: sx,
                        y: sy,
                        width: 6,
                        height: 1,
                    },
                );
            }
        }

        // Grid characters
        for (i, &ch) in grid.chars.iter().enumerate() {
            let (row_off, col_off) = idx_to_cell(i);
            if row_off < layout.row_window_start || row_off >= visible_end {
                continue;
            }
            let sy = base_row + (row_off - layout.row_window_start) as u16;
            let sx = H_PAD + col_off as u16;
            if sy >= layout.hint_y || sx >= size.width {
                continue;
            }

            let is_removed = grid.word_positions.iter().any(|wp| {
                removed_duds.contains(&wp.word)
                    && i >= wp.start
                    && i < wp.start + wp.word.chars().count()
            });

            let highlighted = hover_word
                .is_some_and(|hw| i >= hw.start && i < hw.start + hw.word.chars().count())
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
                Rect {
                    x: sx,
                    y: sy,
                    width: 1,
                    height: 1,
                },
            );
        }

        // Right panel log
        if let Some(panel_col) = layout.log_x {
            f.render_widget(
                Paragraph::new("LOG").style(ds),
                Rect {
                    x: panel_col,
                    y: base_row.saturating_sub(1),
                    width: size.width.saturating_sub(panel_col),
                    height: 1,
                },
            );
            for (li, entry) in log.iter().rev().take(layout.visible_rows).enumerate() {
                let sy = base_row + (layout.visible_rows - 1 - li) as u16;
                f.render_widget(
                    Paragraph::new(entry.as_str()).style(ns),
                    Rect {
                        x: panel_col,
                        y: sy,
                        width: size.width.saturating_sub(panel_col),
                        height: 1,
                    },
                );
            }
        }

        render_status_bar(
            f,
            Rect {
                x: 0,
                y: layout.status_y,
                width: size.width,
                height: 1,
            },
        );
    })?;
    Ok(())
}

// ── Minigame entry point ──────────────────────────────────────────────────────

pub fn run_hacking(terminal: &mut Term) -> Result<bool> {
    draw_security_override(terminal)?;

    let profile = hacking_profile(get_settings().hacking_difficulty);
    let mut rng = rand::thread_rng();
    let word_bank = word_bank_for_len(profile.word_len);
    let answer_idx = rng.gen_range(0..word_bank.len());
    let answer = word_bank[answer_idx].to_string();

    let mut grid = build_grid(&answer, profile);
    let mut cursor = 0usize;
    let mut attempts = profile.max_tries;
    let mut log: Vec<String> = Vec::new();
    let mut removed_duds: HashSet<String> = HashSet::new();
    let mut duds_left: Vec<String> = grid
        .word_positions
        .iter()
        .filter(|wp| wp.word != answer)
        .map(|wp| wp.word.clone())
        .collect();

    let base_addr: u16 = 0xF964;

    loop {
        draw_grid_frame(
            terminal,
            &grid,
            profile,
            &log,
            cursor,
            attempts,
            &removed_duds,
            base_addr,
        )?;

        if !event::poll(Duration::from_millis(100))? {
            continue;
        }
        let ev = event::read()?;
        let Event::Key(key) = ev else { continue };
        if key.kind != KeyEventKind::Press {
            continue;
        }

        if crate::ui::check_session_switch_pub(key.code, key.modifiers) {
            if crate::session::has_switch_request() {
                return Ok(false);
            }
            continue;
        }

        match key.code {
            KeyCode::Right | KeyCode::Char('d') => {
                let col_size = ROWS * COL_WIDTH;
                let cb = cursor / col_size;
                let within = cursor % col_size;
                let row = within / COL_WIDTH;
                let chr = within % COL_WIDTH;
                let (next_cb, next_chr) = if chr + 1 < COL_WIDTH {
                    (cb, chr + 1)
                } else {
                    ((cb + 1) % COLS, 0)
                };
                cursor = next_cb * col_size + row * COL_WIDTH + next_chr;
                crate::sound::play_navigate();
            }
            KeyCode::Left | KeyCode::Char('a') => {
                let col_size = ROWS * COL_WIDTH;
                let cb = cursor / col_size;
                let within = cursor % col_size;
                let row = within / COL_WIDTH;
                let chr = within % COL_WIDTH;
                let (prev_cb, prev_chr) = if chr > 0 {
                    (cb, chr - 1)
                } else {
                    ((cb + COLS - 1) % COLS, COL_WIDTH - 1)
                };
                cursor = prev_cb * col_size + row * COL_WIDTH + prev_chr;
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
            KeyCode::Enter | KeyCode::Char(' ') => {
                crate::sound::play_navigate();
                let sel_word = find_word_at(cursor, &grid.word_positions).cloned();
                let sel_bracket = find_bracket_at(cursor, &grid.bracket_pairs).cloned();

                if let Some(wp) = sel_word {
                    if !removed_duds.contains(&wp.word) {
                        log.push(format!(">{}", wp.word));
                        if wp.word == answer {
                            // Show ">Exact match!" in the log panel and redraw
                            log.push(">Exact match!".to_string());
                            crate::sound::play_login();
                            draw_grid_frame(
                                terminal,
                                &grid,
                                profile,
                                &log,
                                cursor,
                                attempts,
                                &removed_duds,
                                base_addr,
                            )?;
                            std::thread::sleep(Duration::from_millis(900));
                            // Then add ">Please wait..." and redraw again
                            log.push(">Please wait...".to_string());
                            draw_grid_frame(
                                terminal,
                                &grid,
                                profile,
                                &log,
                                cursor,
                                attempts,
                                &removed_duds,
                                base_addr,
                            )?;
                            std::thread::sleep(Duration::from_millis(1400));
                            return Ok(true);
                        } else {
                            let lk = likeness(&wp.word, &answer);
                            log.push(">Entry denied.".to_string());
                            log.push(format!(">{lk}/{} correct.", profile.word_len));
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
                    if !duds_left.is_empty() && rng.gen_bool(profile.dud_remove_chance) {
                        let idx = rng.gen_range(0..duds_left.len());
                        let dud = duds_left.remove(idx);
                        removed_duds.insert(dud);
                        log.push(">Dud removed.".to_string());
                    } else if attempts < profile.max_tries {
                        attempts = (attempts + 1).min(profile.max_tries);
                        log.push(">Tries reset.".to_string());
                    } else {
                        log.push(">No effect.".to_string());
                    }
                }
            }
            KeyCode::Tab | KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                crate::sound::play_navigate();
                return Ok(false);
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn separated_words_always_have_junk_between_them() {
        let profile = hacking_profile(HackingDifficulty::Normal);
        let grid = build_grid("POWER", profile);

        for row in 0..ROWS * COLS {
            let row_start = row * COL_WIDTH;
            let row_end = row_start + COL_WIDTH;
            let mut row_words: Vec<&WordPos> = grid
                .word_positions
                .iter()
                .filter(|wp| wp.start >= row_start && wp.start < row_end)
                .collect();
            row_words.sort_by_key(|wp| wp.start);

            for pair in row_words.windows(2) {
                let left = pair[0];
                let right = pair[1];
                assert!(
                    right.start > left.start + left.word.chars().count(),
                    "words should not touch in row {row}: {} at {} and {} at {}",
                    left.word,
                    left.start,
                    right.word,
                    right.start
                );
                assert!(
                    JUNK.contains(&grid.chars[left.start + left.word.chars().count()]),
                    "separator after {} should be junk",
                    left.word
                );
            }
        }
    }

    #[test]
    fn words_fit_within_single_row() {
        let profile = hacking_profile(HackingDifficulty::Normal);
        let grid = build_grid("POWER", profile);

        for wp in &grid.word_positions {
            let len = wp.word.chars().count();
            assert_eq!(wp.start / COL_WIDTH, (wp.start + len - 1) / COL_WIDTH);
        }
    }

    #[test]
    fn bracket_pairs_do_not_overwrite_word_buffers() {
        let profile = hacking_profile(HackingDifficulty::Normal);
        let grid = build_grid("POWER", profile);

        for bp in &grid.bracket_pairs {
            for wp in &grid.word_positions {
                let reserved = word_reserved_range(wp.start, wp.word.chars().count());
                assert!(!reserved.contains(&bp.open));
                assert!(!reserved.contains(&bp.close));
            }
        }
    }

    #[test]
    fn answer_occurs_exactly_once_in_grid() {
        let profile = hacking_profile(HackingDifficulty::Normal);
        let grid = build_grid("POWER", profile);
        assert_eq!(count_word_occurrences(&grid.chars, "POWER"), 1);
    }
}
