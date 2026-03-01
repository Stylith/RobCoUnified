use crate::config::HackingDifficulty;
use rand::Rng;
use std::collections::HashSet;

pub const COLS: usize = 2;
pub const ROWS: usize = 16;
pub const COL_WIDTH: usize = 12;
pub const BASE_ADDR: u16 = 0xF964;

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

#[derive(Debug, Clone)]
pub struct WordPos {
    pub start: usize,
    pub word: String,
}

#[derive(Debug, Clone)]
pub struct BracketPair {
    pub open: usize,
    pub close: usize,
}

#[derive(Debug, Clone)]
pub struct Grid {
    pub chars: Vec<char>,
    pub word_positions: Vec<WordPos>,
    pub bracket_pairs: Vec<BracketPair>,
}

#[derive(Debug, Clone, Copy)]
pub struct HackingProfile {
    pub difficulty: HackingDifficulty,
    pub word_len: usize,
    pub num_words: usize,
    pub max_tries: usize,
    pub bracket_chance: f64,
    pub dud_remove_chance: f64,
}

#[derive(Debug, Clone)]
pub struct HackingGame {
    pub profile: HackingProfile,
    pub answer: String,
    pub grid: Grid,
    pub cursor: usize,
    pub attempts: usize,
    pub log: Vec<String>,
    pub removed_duds: HashSet<String>,
    pub base_addr: u16,
    duds_left: Vec<String>,
    solved: bool,
    locked_out: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SelectOutcome {
    NoEffect,
    WordRejected {
        likeness: usize,
        attempts_left: usize,
    },
    DudRemoved,
    TriesReset,
    Success,
    LockedOut,
}

pub fn hacking_profile(difficulty: HackingDifficulty) -> HackingProfile {
    match difficulty {
        HackingDifficulty::Easy => HackingProfile {
            difficulty,
            word_len: 4,
            num_words: 8,
            max_tries: 4,
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
            max_tries: 4,
            bracket_chance: 0.22,
            dud_remove_chance: 0.35,
        },
    }
}

impl HackingGame {
    pub fn new(difficulty: HackingDifficulty) -> Self {
        let profile = hacking_profile(difficulty);
        let mut rng = rand::thread_rng();
        let word_bank = word_bank_for_len(profile.word_len);
        let answer = word_bank[rng.gen_range(0..word_bank.len())].to_string();
        let grid = build_grid(&answer, profile);
        let duds_left = grid
            .word_positions
            .iter()
            .filter(|wp| wp.word != answer)
            .map(|wp| wp.word.clone())
            .collect();
        Self {
            profile,
            answer,
            grid,
            cursor: 0,
            attempts: profile.max_tries,
            log: Vec::new(),
            removed_duds: HashSet::new(),
            base_addr: BASE_ADDR,
            duds_left,
            solved: false,
            locked_out: false,
        }
    }

    pub fn is_solved(&self) -> bool {
        self.solved
    }

    pub fn is_locked_out(&self) -> bool {
        self.locked_out
    }

    pub fn move_right(&mut self) {
        let col_size = ROWS * COL_WIDTH;
        let cb = self.cursor / col_size;
        let within = self.cursor % col_size;
        let row = within / COL_WIDTH;
        let chr = within % COL_WIDTH;
        let (next_cb, next_chr) = if chr + 1 < COL_WIDTH {
            (cb, chr + 1)
        } else {
            ((cb + 1) % COLS, 0)
        };
        self.cursor = next_cb * col_size + row * COL_WIDTH + next_chr;
    }

    pub fn move_left(&mut self) {
        let col_size = ROWS * COL_WIDTH;
        let cb = self.cursor / col_size;
        let within = self.cursor % col_size;
        let row = within / COL_WIDTH;
        let chr = within % COL_WIDTH;
        let (prev_cb, prev_chr) = if chr > 0 {
            (cb, chr - 1)
        } else {
            ((cb + COLS - 1) % COLS, COL_WIDTH - 1)
        };
        self.cursor = prev_cb * col_size + row * COL_WIDTH + prev_chr;
    }

    pub fn move_down(&mut self) {
        let cb = self.cursor / (ROWS * COL_WIDTH);
        let within = self.cursor % (ROWS * COL_WIDTH);
        let row = (within / COL_WIDTH + 1) % ROWS;
        let chr = within % COL_WIDTH;
        self.cursor = cb * ROWS * COL_WIDTH + row * COL_WIDTH + chr;
    }

    pub fn move_up(&mut self) {
        let cb = self.cursor / (ROWS * COL_WIDTH);
        let within = self.cursor % (ROWS * COL_WIDTH);
        let row = (within / COL_WIDTH + ROWS - 1) % ROWS;
        let chr = within % COL_WIDTH;
        self.cursor = cb * ROWS * COL_WIDTH + row * COL_WIDTH + chr;
    }

    pub fn select(&mut self) -> SelectOutcome {
        if self.solved || self.locked_out {
            return SelectOutcome::NoEffect;
        }

        if let Some(wp) = find_word_at(self.cursor, &self.grid.word_positions).cloned() {
            if self.removed_duds.contains(&wp.word) {
                return SelectOutcome::NoEffect;
            }
            self.log.push(format!(">{}", wp.word));
            if wp.word == self.answer {
                self.log.push(">Exact match!".to_string());
                self.solved = true;
                return SelectOutcome::Success;
            }
            let lk = likeness(&wp.word, &self.answer);
            self.log.push(">Entry denied.".to_string());
            self.log
                .push(format!(">{lk}/{} correct.", self.profile.word_len));
            self.attempts = self.attempts.saturating_sub(1);
            if self.attempts == 0 {
                self.log.push(">LOCKED OUT.".to_string());
                self.locked_out = true;
                return SelectOutcome::LockedOut;
            }
            return SelectOutcome::WordRejected {
                likeness: lk,
                attempts_left: self.attempts,
            };
        }

        if let Some(bp) = find_bracket_at(self.cursor, &self.grid.bracket_pairs).cloned() {
            self.grid.bracket_pairs.retain(|b| b.open != bp.open);
            let mut rng = rand::thread_rng();
            if !self.duds_left.is_empty() && rng.gen_bool(self.profile.dud_remove_chance) {
                let idx = rng.gen_range(0..self.duds_left.len());
                let dud = self.duds_left.remove(idx);
                self.removed_duds.insert(dud);
                self.log.push(">Dud removed.".to_string());
                return SelectOutcome::DudRemoved;
            }
            if self.attempts < self.profile.max_tries {
                self.attempts = (self.attempts + 1).min(self.profile.max_tries);
                self.log.push(">Tries reset.".to_string());
                return SelectOutcome::TriesReset;
            }
            self.log.push(">No effect.".to_string());
            return SelectOutcome::NoEffect;
        }

        SelectOutcome::NoEffect
    }
}

pub fn idx_to_cell(idx: usize) -> (usize, usize) {
    let col_block = idx / (ROWS * COL_WIDTH);
    let within = idx % (ROWS * COL_WIDTH);
    let row_in_col = within / COL_WIDTH;
    let char_in_row = within % COL_WIDTH;
    let scr_col = 7 + col_block * (COL_WIDTH + 14) + char_in_row;
    (row_in_col, scr_col)
}

pub fn find_word_at(idx: usize, positions: &[WordPos]) -> Option<&WordPos> {
    positions
        .iter()
        .find(|wp| idx >= wp.start && idx < wp.start + wp.word.chars().count())
}

pub fn find_bracket_at(idx: usize, pairs: &[BracketPair]) -> Option<&BracketPair> {
    pairs.iter().find(|bp| idx == bp.open || idx == bp.close)
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
            if start == answer_start || chars[start..start + word_len] != target[..] {
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
    let mut word_positions = Vec::new();

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

    let mut bracket_pairs = Vec::new();
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
                assert!(right.start > left.start + left.word.chars().count());
                assert!(JUNK.contains(&grid.chars[left.start + left.word.chars().count()]));
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

    #[test]
    fn max_tries_is_always_four_across_difficulties() {
        assert_eq!(hacking_profile(HackingDifficulty::Easy).max_tries, 4);
        assert_eq!(hacking_profile(HackingDifficulty::Normal).max_tries, 4);
        assert_eq!(hacking_profile(HackingDifficulty::Hard).max_tries, 4);
    }
}
