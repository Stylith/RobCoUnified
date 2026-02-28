use anyhow::Result;
use chrono::Local;
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    text::{Line, Span},
    widgets::Paragraph,
};
use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::config::{get_current_user, is_allowed_extension, load_categories};
use crate::default_apps::{resolve_document_open, ResolvedDocumentOpen};
use crate::launcher::launch_argv;
use crate::status::render_status_bar;
use crate::ui::{
    confirm, dim_style, flash_message, input_prompt, normal_style, pad_horizontal, pager,
    render_header, render_separator, run_menu, sel_style, title_style, MenuResult, Term,
};

// ── Document scanning ─────────────────────────────────────────────────────────

pub fn scan_documents(folder: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    if let Ok(rd) = std::fs::read_dir(folder) {
        for entry in rd.flatten() {
            let p = entry.path();
            if p.is_file() && is_allowed_extension(&p) {
                files.push(p);
            }
        }
    }
    files.sort_by_key(|f| sort_key(f));
    files
}

pub fn scan_subfolders(folder: &Path) -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Ok(rd) = std::fs::read_dir(folder) {
        for entry in rd.flatten() {
            let p = entry.path();
            if p.is_dir() {
                dirs.push(p);
            }
        }
    }
    dirs.sort_by_key(|d| {
        d.file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_lowercase()
            .to_string()
    });
    dirs
}

fn sort_key(f: &Path) -> String {
    let name = f
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .replace('_', " ")
        .to_lowercase();
    if let Some(rest) = name.strip_prefix("the ") {
        rest.to_string()
    } else {
        name
    }
}

// ── Inline text editor ────────────────────────────────────────────────────────

struct Editor {
    lines: Vec<String>,
    row: usize,
    col: usize,
    scroll_y: usize,
    scroll_x: usize,
    dirty: bool,
    path: PathBuf,
    search_query: String,
    search_matches: Vec<(usize, usize)>,
    search_index: usize,
}

impl Editor {
    fn new(text: &str, path: PathBuf) -> Self {
        let lines: Vec<String> = if text.is_empty() {
            vec![String::new()]
        } else {
            text.lines().map(str::to_string).collect()
        };
        Self {
            lines,
            row: 0,
            col: 0,
            scroll_y: 0,
            scroll_x: 0,
            dirty: false,
            path,
            search_query: String::new(),
            search_matches: Vec::new(),
            search_index: 0,
        }
    }

    fn text(&self) -> String {
        self.lines.join("\n")
    }

    fn line_len(&self, row: usize) -> usize {
        self.lines.get(row).map(|line| line.len()).unwrap_or(0)
    }

    fn file_name(&self) -> String {
        self.path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("document.txt")
            .to_string()
    }

    fn ensure_visible(&mut self, visible_rows: usize, visible_cols: usize) {
        if visible_rows == 0 || visible_cols == 0 {
            self.scroll_y = 0;
            self.scroll_x = 0;
            return;
        }
        if self.row < self.scroll_y {
            self.scroll_y = self.row;
        } else if self.row >= self.scroll_y.saturating_add(visible_rows) {
            self.scroll_y = self.row.saturating_sub(visible_rows.saturating_sub(1));
        }
        let max_scroll = self.lines.len().saturating_sub(visible_rows);
        self.scroll_y = self.scroll_y.min(max_scroll);
        if self.col < self.scroll_x {
            self.scroll_x = self.col;
        } else if self.col >= self.scroll_x.saturating_add(visible_cols) {
            self.scroll_x = self.col.saturating_sub(visible_cols.saturating_sub(1));
        }
    }

    fn move_vertical(&mut self, delta: isize) {
        if delta < 0 {
            self.row = self.row.saturating_sub(delta.unsigned_abs());
        } else {
            self.row = (self.row + delta as usize).min(self.lines.len().saturating_sub(1));
        }
        self.col = self.col.min(self.line_len(self.row));
    }

    fn refresh_search(&mut self) {
        let query = self.search_query.to_ascii_lowercase();
        self.search_matches.clear();
        self.search_index = 0;
        if query.is_empty() {
            return;
        }
        for (row, line) in self.lines.iter().enumerate() {
            let lower = line.to_ascii_lowercase();
            let mut start = 0;
            while let Some(idx) = lower[start..].find(&query) {
                self.search_matches.push((row, start + idx));
                start += idx + query.len().max(1);
                if start >= lower.len() {
                    break;
                }
            }
        }
    }

    fn jump_to_match(&mut self, idx: usize, visible_rows: usize, visible_cols: usize) {
        if let Some((row, col)) = self.search_matches.get(idx).copied() {
            self.search_index = idx;
            self.row = row;
            self.col = col;
            self.ensure_visible(visible_rows, visible_cols);
        }
    }

    fn find_next(&mut self, visible_rows: usize, visible_cols: usize) {
        if self.search_matches.is_empty() {
            return;
        }
        let next = (self.search_index + 1) % self.search_matches.len();
        self.jump_to_match(next, visible_rows, visible_cols);
    }

    fn key(
        &mut self,
        code: KeyCode,
        modifiers: KeyModifiers,
        visible_rows: usize,
        visible_cols: usize,
    ) -> EditorAction {
        if modifiers.contains(KeyModifiers::CONTROL) {
            match code {
                KeyCode::Char('s') | KeyCode::Char('S') => return EditorAction::Save,
                KeyCode::Char('a') | KeyCode::Char('A') => return EditorAction::SaveAs,
                KeyCode::Char('r') | KeyCode::Char('R') => return EditorAction::Rename,
                KeyCode::Char('f') | KeyCode::Char('F') => return EditorAction::Search,
                KeyCode::Char('g') | KeyCode::Char('G') => return EditorAction::FindNext,
                KeyCode::Char('q') | KeyCode::Char('Q') => return EditorAction::ForceClose,
                KeyCode::Home => {
                    self.scroll_x = 0;
                    return EditorAction::None;
                }
                _ => {}
            }
        }

        match code {
            KeyCode::Esc | KeyCode::Tab => return EditorAction::RequestClose,
            KeyCode::Enter => {
                let rest = self.lines[self.row][self.col..].to_string();
                self.lines[self.row].truncate(self.col);
                self.row += 1;
                self.lines.insert(self.row, rest);
                self.col = 0;
                self.dirty = true;
                self.refresh_search();
            }
            KeyCode::Backspace => {
                if self.col > 0 {
                    self.lines[self.row].remove(self.col - 1);
                    self.col -= 1;
                } else if self.row > 0 {
                    let cur = self.lines.remove(self.row);
                    self.row -= 1;
                    self.col = self.lines[self.row].len();
                    self.lines[self.row].push_str(&cur);
                }
                self.dirty = true;
                self.refresh_search();
            }
            KeyCode::Up => {
                self.move_vertical(-1);
            }
            KeyCode::Down => {
                self.move_vertical(1);
            }
            KeyCode::PageUp => {
                let step = visible_rows.max(1) as isize;
                self.move_vertical(-step);
            }
            KeyCode::PageDown => {
                let step = visible_rows.max(1) as isize;
                self.move_vertical(step);
            }
            KeyCode::Home => {
                self.col = 0;
            }
            KeyCode::End => {
                self.col = self.line_len(self.row);
            }
            KeyCode::Left => {
                if self.col > 0 {
                    self.col -= 1;
                } else if self.row > 0 {
                    self.row -= 1;
                    self.col = self.line_len(self.row);
                }
            }
            KeyCode::Right => {
                let max = self.line_len(self.row);
                if self.col < max {
                    self.col += 1;
                } else if self.row + 1 < self.lines.len() {
                    self.row += 1;
                    self.col = 0;
                }
            }
            KeyCode::Char(c)
                if !modifiers.contains(KeyModifiers::CONTROL)
                    && !modifiers.contains(KeyModifiers::ALT)
                    && !modifiers.contains(KeyModifiers::SUPER)
                    && (c as u32) >= 32 =>
            {
                self.lines[self.row].insert(self.col, c);
                self.col += 1;
                self.dirty = true;
                self.refresh_search();
            }
            _ => {}
        }
        self.ensure_visible(visible_rows, visible_cols);
        EditorAction::None
    }
}

#[derive(Debug, PartialEq)]
enum EditorAction {
    None,
    Save,
    SaveAs,
    Rename,
    Search,
    FindNext,
    RequestClose,
    ForceClose,
}

fn resolve_editor_target_path(current_path: &Path, raw: &str) -> Option<PathBuf> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    let mut out = PathBuf::from(trimmed);
    if !out.is_absolute() {
        let parent = current_path
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."));
        out = parent.join(out);
    }
    if out.extension().is_none() {
        out.set_extension("txt");
    }
    Some(out)
}

fn prompt_editor_target(
    terminal: &mut Term,
    current_path: &Path,
    label: &str,
) -> Result<Option<PathBuf>> {
    let seed = current_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("document.txt");
    let prompt = format!("{label} (blank cancels, default {seed}):");
    let Some(raw) = input_prompt(terminal, &prompt)? else {
        return Ok(None);
    };
    Ok(resolve_editor_target_path(
        current_path,
        if raw.trim().is_empty() { seed } else { &raw },
    ))
}

fn prompt_editor_search(terminal: &mut Term, current: &str) -> Result<Option<String>> {
    let prompt = if current.trim().is_empty() {
        "Find (blank cancels):".to_string()
    } else {
        format!("Find (blank keeps {current}):")
    };
    let Some(raw) = input_prompt(terminal, &prompt)? else {
        return Ok(None);
    };
    let next = if raw.trim().is_empty() {
        current.to_string()
    } else {
        raw
    };
    if next.trim().is_empty() {
        return Ok(None);
    }
    Ok(Some(next))
}

enum EditorCloseDecision {
    Save,
    Discard,
    Cancel,
}

fn prompt_editor_close(terminal: &mut Term, file_name: &str) -> Result<EditorCloseDecision> {
    match run_menu(
        terminal,
        "Unsaved Changes",
        &["Save", "Discard", "Cancel"],
        Some(&format!("Save changes to {file_name}?")),
    )? {
        MenuResult::Selected(s) if s == "Save" => Ok(EditorCloseDecision::Save),
        MenuResult::Selected(s) if s == "Discard" => Ok(EditorCloseDecision::Discard),
        _ => Ok(EditorCloseDecision::Cancel),
    }
}

#[derive(Clone)]
struct TerminalSaveAsEntry {
    label: String,
    path: PathBuf,
    is_dir: bool,
}

struct TerminalSaveAsState {
    cwd: PathBuf,
    entries: Vec<TerminalSaveAsEntry>,
    selected: usize,
    scroll: usize,
    file_name: String,
    input_mode: bool,
}

impl TerminalSaveAsState {
    fn new(current_path: &Path) -> Self {
        let cwd = current_path
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(text_editor_dir);
        let file_name = current_path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("document.txt")
            .to_string();
        let mut state = Self {
            cwd,
            entries: Vec::new(),
            selected: 0,
            scroll: 0,
            file_name,
            input_mode: false,
        };
        state.refresh();
        state
    }

    fn refresh(&mut self) {
        self.entries = terminal_save_as_entries(&self.cwd);
        if self.entries.is_empty() {
            self.selected = 0;
            self.scroll = 0;
        } else {
            self.selected = self.selected.min(self.entries.len().saturating_sub(1));
            self.scroll = self.scroll.min(self.entries.len().saturating_sub(1));
        }
    }

    fn parent(&mut self) {
        if let Some(parent) = self.cwd.parent() {
            self.cwd = parent.to_path_buf();
            self.selected = 0;
            self.scroll = 0;
            self.refresh();
        }
    }
}

fn terminal_save_as_entries(dir: &Path) -> Vec<TerminalSaveAsEntry> {
    let mut entries = Vec::new();
    if let Some(parent) = dir.parent() {
        entries.push(TerminalSaveAsEntry {
            label: "../".to_string(),
            path: parent.to_path_buf(),
            is_dir: true,
        });
    }

    let mut dirs = Vec::new();
    let mut files = Vec::new();
    if let Ok(read_dir) = std::fs::read_dir(dir) {
        for item in read_dir.flatten() {
            let path = item.path();
            let name = item.file_name().to_string_lossy().to_string();
            if path.is_dir() {
                dirs.push(TerminalSaveAsEntry {
                    label: format!("{name}/"),
                    path,
                    is_dir: true,
                });
            } else if path.is_file() {
                files.push(TerminalSaveAsEntry {
                    label: name,
                    path,
                    is_dir: false,
                });
            }
        }
    }
    dirs.sort_by_key(|entry| entry.label.to_ascii_lowercase());
    files.sort_by_key(|entry| entry.label.to_ascii_lowercase());
    entries.extend(dirs);
    entries.extend(files);
    entries
}

fn normalize_editor_file_name(raw: &str, default_name: &str) -> Option<String> {
    let trimmed = raw.trim();
    let candidate = if trimmed.is_empty() {
        default_name.trim()
    } else {
        trimmed
    };
    if candidate.is_empty() || candidate.contains('/') || candidate.contains('\\') {
        return None;
    }
    let mut out = candidate.to_string();
    if Path::new(&out).extension().is_none() {
        out.push_str(".txt");
    }
    Some(out)
}

fn terminal_save_as_list_visible_rows(terminal: &mut Term) -> usize {
    terminal
        .size()
        .map(|size| size.height.saturating_sub(11) as usize)
        .unwrap_or(1)
        .max(1)
}

fn terminal_save_as_ensure_selection_visible(state: &mut TerminalSaveAsState, visible_rows: usize) {
    if state.entries.is_empty() {
        state.selected = 0;
        state.scroll = 0;
        return;
    }
    state.selected = state.selected.min(state.entries.len().saturating_sub(1));
    if state.selected < state.scroll {
        state.scroll = state.selected;
    } else if state.selected >= state.scroll.saturating_add(visible_rows) {
        state.scroll = state
            .selected
            .saturating_sub(visible_rows.saturating_sub(1));
    }
    let max_scroll = state.entries.len().saturating_sub(visible_rows);
    state.scroll = state.scroll.min(max_scroll);
}

fn run_terminal_save_as_browser(
    terminal: &mut Term,
    current_path: &Path,
) -> Result<Option<PathBuf>> {
    let mut state = TerminalSaveAsState::new(current_path);

    loop {
        terminal.draw(|f| {
            let size = f.area();
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3),
                    Constraint::Length(1),
                    Constraint::Length(1),
                    Constraint::Length(1),
                    Constraint::Length(2),
                    Constraint::Min(1),
                    Constraint::Length(1),
                    Constraint::Length(1),
                    Constraint::Length(1),
                ])
                .split(size);

            render_header(f, chunks[0]);
            render_separator(f, chunks[1]);
            f.render_widget(
                Paragraph::new("Save Document As")
                    .alignment(Alignment::Center)
                    .style(title_style()),
                pad_horizontal(chunks[2]),
            );
            render_separator(f, chunks[3]);
            f.render_widget(
                Paragraph::new(vec![
                    Line::from(Span::styled(
                        format!("Folder: {}", state.cwd.display()),
                        normal_style(),
                    )),
                    Line::from(Span::styled(
                        if state.input_mode {
                            format!("File name: {}_", state.file_name)
                        } else {
                            format!("File name: {}", state.file_name)
                        },
                        if state.input_mode {
                            sel_style()
                        } else {
                            normal_style()
                        },
                    )),
                ]),
                pad_horizontal(chunks[4]),
            );

            let visible_rows = chunks[5].height as usize;
            terminal_save_as_ensure_selection_visible(&mut state, visible_rows);
            let lines: Vec<Line> = state
                .entries
                .iter()
                .enumerate()
                .skip(state.scroll)
                .take(visible_rows)
                .map(|(idx, entry)| {
                    let style = if !state.input_mode && idx == state.selected {
                        sel_style()
                    } else {
                        normal_style()
                    };
                    Line::from(Span::styled(entry.label.clone(), style))
                })
                .collect();
            f.render_widget(Paragraph::new(lines), pad_horizontal(chunks[5]));

            f.render_widget(
                Paragraph::new(Line::from(Span::styled(
                    "Arrows move | Enter open folder/select file | Backspace parent | Tab name | Ctrl+S save | Esc cancel",
                    dim_style(),
                ))),
                pad_horizontal(chunks[6]),
            );
            f.render_widget(
                Paragraph::new(Line::from(Span::styled(
                    "Files fill the name bar. Folders change the target location.",
                    dim_style(),
                ))),
                pad_horizontal(chunks[7]),
            );
            render_status_bar(f, chunks[8]);
        })?;

        if !event::poll(Duration::from_millis(50))? {
            continue;
        }
        let Event::Key(key) = event::read()? else {
            continue;
        };
        if key.kind != KeyEventKind::Press {
            continue;
        }
        if crate::ui::check_session_switch_pub(key.code, key.modifiers) {
            if crate::session::has_switch_request() {
                return Ok(None);
            }
            continue;
        }

        if key.modifiers.contains(KeyModifiers::CONTROL)
            && matches!(key.code, KeyCode::Char('s') | KeyCode::Char('S'))
        {
            let default_name = current_path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("document.txt");
            let Some(file_name) = normalize_editor_file_name(&state.file_name, default_name) else {
                flash_message(terminal, "Invalid file name.", 900)?;
                continue;
            };
            return Ok(Some(state.cwd.join(file_name)));
        }

        if state.input_mode {
            match key.code {
                KeyCode::Esc => return Ok(None),
                KeyCode::Enter => {
                    let default_name = current_path
                        .file_name()
                        .and_then(|name| name.to_str())
                        .unwrap_or("document.txt");
                    let Some(file_name) =
                        normalize_editor_file_name(&state.file_name, default_name)
                    else {
                        flash_message(terminal, "Invalid file name.", 900)?;
                        continue;
                    };
                    return Ok(Some(state.cwd.join(file_name)));
                }
                KeyCode::Tab | KeyCode::BackTab => state.input_mode = false,
                KeyCode::Backspace => {
                    let _ = state.file_name.pop();
                }
                KeyCode::Char(c)
                    if !key.modifiers.contains(KeyModifiers::ALT)
                        && !key.modifiers.contains(KeyModifiers::SUPER) =>
                {
                    state.file_name.push(c);
                }
                _ => {}
            }
            continue;
        }

        match key.code {
            KeyCode::Esc => return Ok(None),
            KeyCode::Tab | KeyCode::BackTab => state.input_mode = true,
            KeyCode::Up => {
                state.selected = state.selected.saturating_sub(1);
            }
            KeyCode::Down => {
                if !state.entries.is_empty() {
                    state.selected =
                        (state.selected + 1).min(state.entries.len().saturating_sub(1));
                }
            }
            KeyCode::PageUp => {
                let step = terminal_save_as_list_visible_rows(terminal);
                state.selected = state.selected.saturating_sub(step);
            }
            KeyCode::PageDown => {
                let step = terminal_save_as_list_visible_rows(terminal);
                if !state.entries.is_empty() {
                    state.selected =
                        (state.selected + step).min(state.entries.len().saturating_sub(1));
                }
            }
            KeyCode::Home => state.selected = 0,
            KeyCode::End => {
                state.selected = state.entries.len().saturating_sub(1);
            }
            KeyCode::Backspace => state.parent(),
            KeyCode::Enter => {
                if let Some(entry) = state.entries.get(state.selected).cloned() {
                    if entry.is_dir {
                        state.cwd = entry.path;
                        state.selected = 0;
                        state.scroll = 0;
                        state.refresh();
                    } else {
                        state.file_name = entry
                            .path
                            .file_name()
                            .and_then(|name| name.to_str())
                            .unwrap_or("document.txt")
                            .to_string();
                        state.input_mode = true;
                    }
                }
            }
            _ => {}
        }
    }
}

fn save_editor(editor: &mut Editor) -> Result<String> {
    if let Some(parent) = editor.path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    std::fs::write(&editor.path, editor.text())?;
    editor.dirty = false;
    Ok(format!("Saved {}.", editor.file_name()))
}

fn run_editor(terminal: &mut Term, title: &str, initial: &str, path: PathBuf) -> Result<()> {
    let mut ed = Editor::new(initial, path);

    loop {
        terminal.draw(|f| {
            let size = f.area();
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3),
                    Constraint::Length(1),
                    Constraint::Length(1),
                    Constraint::Length(1),
                    Constraint::Length(3),
                    Constraint::Min(1),
                    Constraint::Length(1),
                    Constraint::Length(1),
                ])
                .split(size);

            render_header(f, chunks[0]);
            render_separator(f, chunks[1]);

            let tp = Paragraph::new(title)
                .alignment(Alignment::Center)
                .style(title_style());
            f.render_widget(tp, pad_horizontal(chunks[2]));
            crate::ui::render_separator(f, chunks[3]);

            let path_line = format!(
                "{}{}",
                ed.path.display(),
                if ed.dirty { " *" } else { "" }
            );
            let status_line = format!("Ln {} Col {}", ed.row + 1, ed.col + 1);
            let search_line = if ed.search_query.is_empty() {
                "Search: Ctrl+F".to_string()
            } else if ed.search_matches.is_empty() {
                format!("Search: {} (0 matches)", ed.search_query)
            } else {
                format!(
                    "Search: {} ({}/{})",
                    ed.search_query,
                    ed.search_index + 1,
                    ed.search_matches.len()
                )
            };
            f.render_widget(
                Paragraph::new(vec![
                    Line::from(Span::styled(
                        path_line,
                        normal_style(),
                    )),
                    Line::from(Span::styled(status_line, dim_style())),
                    Line::from(Span::styled(
                        search_line,
                        if ed.search_query.is_empty() {
                            dim_style()
                        } else {
                            normal_style()
                        },
                    )),
                ]),
                pad_horizontal(chunks[4]),
            );

            let visible_rows = chunks[5].height as usize;
            let gutter = ed.lines.len().max(1).to_string().len() + 2;
            let body_width = chunks[5].width.saturating_sub(2) as usize;
            let visible_cols = body_width.saturating_sub(gutter).max(1);
            ed.ensure_visible(visible_rows, visible_cols);
            let lines: Vec<Line> = ed
                .lines
                .iter()
                .enumerate()
                .skip(ed.scroll_y)
                .take(visible_rows)
                .map(|(idx, l)| {
                    let prefix = format!("{:>width$} ", idx + 1, width = gutter.saturating_sub(1));
                    let mut visible: String = l.chars().skip(ed.scroll_x).take(visible_cols).collect();
                    if idx == ed.row {
                        let cursor_idx = ed.col.saturating_sub(ed.scroll_x).min(visible_cols);
                        if cursor_idx >= visible.chars().count() {
                            visible.push('_');
                        } else {
                            let mut chars: Vec<char> = visible.chars().collect();
                            chars.insert(cursor_idx, '_');
                            visible = chars.into_iter().take(visible_cols).collect();
                        }
                    }
                    let text = format!("{prefix}{visible}");
                    Line::from(Span::styled(text, normal_style()))
                })
                .collect();
            f.render_widget(Paragraph::new(lines), pad_horizontal(chunks[5]));

            let hint = Paragraph::new(
                "Ctrl+S save | Ctrl+A save as | Ctrl+R rename | Ctrl+F find | Ctrl+G next | Ctrl+Q discard | Tab close",
            )
                .style(dim_style());
            f.render_widget(hint, pad_horizontal(chunks[6]));
            render_status_bar(f, chunks[7]);
        })?;

        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }
                // Session switch cancels the edit
                if crate::ui::check_session_switch_pub(key.code, key.modifiers) {
                    if crate::session::has_switch_request() {
                        return Ok(());
                    }
                    continue;
                }
                let (visible_rows, visible_cols) = terminal
                    .size()
                    .map(|size| {
                        (
                            size.height.saturating_sub(10) as usize,
                            size.width.saturating_sub(6) as usize,
                        )
                    })
                    .unwrap_or((1, 20));
                match ed.key(key.code, key.modifiers, visible_rows, visible_cols) {
                    EditorAction::Save => {
                        let msg = save_editor(&mut ed)?;
                        flash_message(terminal, &msg, 900)?;
                    }
                    EditorAction::SaveAs => {
                        if let Some(path) = run_terminal_save_as_browser(terminal, &ed.path)? {
                            ed.path = path;
                            save_editor(&mut ed)?;
                            flash_message(terminal, &format!("Saved as {}.", ed.file_name()), 900)?;
                        }
                    }
                    EditorAction::Rename => {
                        if let Some(target) = prompt_editor_target(terminal, &ed.path, "Rename")? {
                            if target != ed.path {
                                if target.exists() {
                                    flash_message(terminal, "Target already exists.", 1000)?;
                                } else {
                                    if ed.path.exists() {
                                        if let Some(parent) = target.parent() {
                                            let _ = std::fs::create_dir_all(parent);
                                        }
                                        std::fs::rename(&ed.path, &target)?;
                                    }
                                    ed.path = target;
                                    flash_message(
                                        terminal,
                                        &format!("Renamed to {}.", ed.file_name()),
                                        900,
                                    )?;
                                }
                            }
                        }
                    }
                    EditorAction::Search => {
                        if let Some(query) = prompt_editor_search(terminal, &ed.search_query)? {
                            ed.search_query = query;
                            ed.refresh_search();
                            if ed.search_matches.is_empty() {
                                flash_message(terminal, "No matches found.", 900)?;
                            } else {
                                ed.jump_to_match(0, visible_rows, visible_cols);
                            }
                        }
                    }
                    EditorAction::FindNext => {
                        if ed.search_matches.is_empty() {
                            flash_message(terminal, "No matches found.", 900)?;
                        } else {
                            ed.find_next(visible_rows, visible_cols);
                        }
                    }
                    EditorAction::RequestClose => {
                        if ed.dirty {
                            match prompt_editor_close(terminal, &ed.file_name())? {
                                EditorCloseDecision::Save => {
                                    let msg = save_editor(&mut ed)?;
                                    flash_message(terminal, &msg, 900)?;
                                    return Ok(());
                                }
                                EditorCloseDecision::Discard => return Ok(()),
                                EditorCloseDecision::Cancel => {}
                            }
                        } else {
                            return Ok(());
                        }
                    }
                    EditorAction::ForceClose => return Ok(()),
                    EditorAction::None => {}
                }
            }
        }
    }
}

pub fn view_text_file(terminal: &mut Term, path: &Path) -> Result<()> {
    let title = path
        .file_name()
        .and_then(|n| n.to_str())
        .map(|n| format!("View: {n}"))
        .unwrap_or_else(|| "View File".to_string());
    match std::fs::read(path) {
        Ok(bytes) => {
            let text = String::from_utf8_lossy(&bytes).to_string();
            pager(terminal, &text, &title)?;
        }
        Err(_) => {
            flash_message(terminal, "Could not open file", 1000)?;
        }
    }
    Ok(())
}

pub fn edit_text_file(terminal: &mut Term, path: &Path) -> Result<()> {
    let text = match std::fs::read_to_string(path) {
        Ok(t) => t,
        Err(_) => {
            flash_message(terminal, "File is not UTF-8 text", 1000)?;
            return Ok(());
        }
    };
    run_editor(terminal, "Text Editor", &text, path.to_path_buf())
}

// ── Journal ───────────────────────────────────────────────────────────────────

fn log_dir() -> PathBuf {
    let base = std::path::PathBuf::from("journal_entries");
    if let Some(u) = get_current_user() {
        let d = base.join(&u);
        let _ = std::fs::create_dir_all(&d);
        d
    } else {
        let _ = std::fs::create_dir_all(&base);
        base
    }
}

fn text_editor_dir() -> PathBuf {
    let base = std::path::PathBuf::from("text_editor_documents");
    if let Some(u) = get_current_user() {
        let d = base.join(&u);
        let _ = std::fs::create_dir_all(&d);
        d
    } else {
        let _ = std::fs::create_dir_all(&base);
        base
    }
}

fn normalize_new_text_document_name(raw: &str, default_stem: &str) -> Option<String> {
    let trimmed = raw.trim();
    let candidate = if trimmed.is_empty() {
        default_stem.trim()
    } else {
        trimmed
    };

    let mut normalized = String::new();
    let mut last_was_sep = false;
    for ch in candidate.chars() {
        if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.') {
            normalized.push(ch);
            last_was_sep = false;
        } else if ch.is_whitespace() {
            if !normalized.is_empty() && !last_was_sep {
                normalized.push('_');
                last_was_sep = true;
            }
        }
    }

    let normalized = normalized.trim_matches(['_', '.', ' ']).to_string();
    if normalized.is_empty() || normalized == "." || normalized == ".." {
        return None;
    }

    let has_extension = Path::new(&normalized).extension().is_some();
    if has_extension {
        Some(normalized)
    } else {
        Some(format!("{normalized}.txt"))
    }
}

fn prompt_new_text_document_path_in_dir(
    terminal: &mut Term,
    dir: &Path,
) -> Result<Option<PathBuf>> {
    let default_stem = Local::now().format("%Y-%m-%d").to_string();
    loop {
        let prompt = format!("Document name (.txt default, blank for {default_stem}.txt):");
        let Some(raw) = input_prompt(terminal, &prompt)? else {
            return Ok(None);
        };
        let Some(name) = normalize_new_text_document_name(&raw, &default_stem) else {
            flash_message(terminal, "Error: Invalid document name.", 900)?;
            continue;
        };
        return Ok(Some(dir.join(name)));
    }
}

pub fn prompt_new_text_document_path(terminal: &mut Term) -> Result<Option<PathBuf>> {
    prompt_new_text_document_path_in_dir(terminal, &text_editor_dir())
}

pub fn prompt_new_log_path(terminal: &mut Term) -> Result<Option<PathBuf>> {
    prompt_new_text_document_path_in_dir(terminal, &log_dir())
}

pub fn new_text_document(terminal: &mut Term) -> Result<()> {
    let dir = text_editor_dir();
    let Some(path) = prompt_new_text_document_path_in_dir(terminal, &dir)? else {
        return Ok(());
    };
    let existing = if path.exists() {
        match std::fs::read_to_string(&path) {
            Ok(text) => text,
            Err(_) => {
                flash_message(terminal, "File is not UTF-8 text", 1000)?;
                return Ok(());
            }
        }
    } else {
        String::new()
    };
    run_editor(terminal, "Text Editor", &existing, path)
}

fn new_log(terminal: &mut Term) -> Result<()> {
    let dir = log_dir();
    let Some(path) = prompt_new_text_document_path_in_dir(terminal, &dir)? else {
        return Ok(());
    };
    let existing = if path.exists() {
        std::fs::read_to_string(&path).unwrap_or_default()
    } else {
        String::new()
    };
    run_editor(terminal, "Log Editor", &existing, path)
}

fn saved_document_paths(dir: &Path) -> Result<Vec<PathBuf>> {
    if !dir.exists() {
        anyhow::bail!("saved documents folder not found");
    }
    let mut logs: Vec<PathBuf> = std::fs::read_dir(&dir)?
        .flatten()
        .filter(|e| e.path().is_file())
        .map(|e| e.path())
        .collect();
    if logs.is_empty() {
        anyhow::bail!("no saved documents found");
    }
    logs.sort_by(|a, b| b.cmp(a)); // newest first
    Ok(logs)
}

fn pick_saved_document(terminal: &mut Term, title: &str, dir: &Path) -> Result<Option<PathBuf>> {
    let logs = match saved_document_paths(dir) {
        Ok(paths) => paths,
        Err(_) => {
            flash_message(terminal, "Error: No saved documents found.", 800)?;
            return Ok(None);
        }
    };
    loop {
        let mut keys: Vec<String> = logs
            .iter()
            .filter_map(|p| p.file_stem().map(|s| s.to_string_lossy().to_string()))
            .collect();
        keys.push("Back".to_string());
        let opts: Vec<&str> = keys.iter().map(String::as_str).collect();

        let sel = match run_menu(terminal, title, &opts, None)? {
            MenuResult::Back => return Ok(None),
            MenuResult::Selected(s) if s == "Back" => return Ok(None),
            MenuResult::Selected(s) => s,
        };

        let path = dir.join(format!("{sel}.txt"));
        if !path.exists() {
            continue;
        }
        return Ok(Some(path));
    }
}

pub fn journal_view(terminal: &mut Term) -> Result<()> {
    let dir = log_dir();
    let mut logs = match saved_document_paths(&dir) {
        Ok(paths) => paths,
        Err(_) => return flash_message(terminal, "Error: No entries found.", 800),
    };

    loop {
        let mut keys: Vec<String> = logs
            .iter()
            .filter_map(|p| p.file_stem().map(|s| s.to_string_lossy().to_string()))
            .collect();
        keys.push("Back".to_string());
        let opts: Vec<&str> = keys.iter().map(String::as_str).collect();

        let sel = match run_menu(terminal, "View Logs", &opts, None)? {
            MenuResult::Back => break,
            MenuResult::Selected(s) if s == "Back" => break,
            MenuResult::Selected(s) => s,
        };

        let path = dir.join(format!("{sel}.txt"));
        if !path.exists() {
            continue;
        }

        loop {
            match run_menu(
                terminal,
                &sel,
                &["View", "Edit", "Delete", "---", "Back"],
                None,
            )? {
                MenuResult::Back => break,
                MenuResult::Selected(s) => match s.as_str() {
                    "View" => {
                        let text = std::fs::read_to_string(&path).unwrap_or_default();
                        pager(terminal, &text, &sel)?;
                    }
                    "Edit" => {
                        edit_text_file(terminal, &path)?;
                    }
                    "Delete" => {
                        if confirm(terminal, &format!("Delete '{sel}'?"))? {
                            std::fs::remove_file(&path)?;
                            flash_message(terminal, &format!("Deleted {sel}."), 800)?;
                            logs.retain(|p| p != &path);
                            break;
                        }
                    }
                    _ => break,
                },
            }
        }
    }
    Ok(())
}

pub fn text_editor_menu(terminal: &mut Term) -> Result<()> {
    loop {
        match run_menu(
            terminal,
            "Text Editor",
            &["New Document", "Open Document", "---", "Back"],
            None,
        )? {
            MenuResult::Back => break,
            MenuResult::Selected(s) => match s.as_str() {
                "New Document" => new_text_document(terminal)?,
                "Open Document" => {
                    if let Some(path) =
                        pick_saved_document(terminal, "Open Document", &text_editor_dir())?
                    {
                        edit_text_file(terminal, &path)?;
                    }
                }
                _ => break,
            },
        }
    }
    Ok(())
}

pub fn logs_menu(terminal: &mut Term) -> Result<()> {
    loop {
        match run_menu(
            terminal,
            "Logs",
            &["New Log", "View Logs", "---", "Back"],
            None,
        )? {
            MenuResult::Back => break,
            MenuResult::Selected(s) => match s.as_str() {
                "New Log" => new_log(terminal)?,
                "View Logs" => journal_view(terminal)?,
                _ => break,
            },
        }
    }
    Ok(())
}

// ── Folder browser ─────────────────────────────────────────────────────────────

fn browse_folder(terminal: &mut Term, folder: &Path, title: &str) -> Result<()> {
    loop {
        let subfolders = scan_subfolders(folder);
        let files = scan_documents(folder);

        if subfolders.is_empty() && files.is_empty() {
            flash_message(terminal, "No documents or subfolders found.", 800)?;
            return Ok(());
        }

        let mut choices: Vec<String> = Vec::new();
        for sf in &subfolders {
            let name = sf
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string()
                + "/";
            choices.push(name);
        }
        for f in &files {
            let name = f
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .replace('_', " ")
                .to_string();
            choices.push(name);
        }
        choices.push("---".to_string());
        choices.push("Back".to_string());

        let opts: Vec<&str> = choices.iter().map(String::as_str).collect();
        let sub = folder.display().to_string();

        match run_menu(terminal, title, &opts, Some(&sub))? {
            MenuResult::Back => break,
            MenuResult::Selected(s) if s == "Back" => break,
            MenuResult::Selected(s) => {
                if s.ends_with('/') {
                    let sf_name = s.trim_end_matches('/');
                    if let Some(sf) = subfolders.iter().find(|p| {
                        p.file_name()
                            .map(|n| n.to_string_lossy() == sf_name)
                            .unwrap_or(false)
                    }) {
                        browse_folder(terminal, sf, sf_name)?;
                    }
                } else {
                    // File
                    let fname = s.replace(' ', "_");
                    if let Some(f) = files.iter().find(|p| {
                        p.file_stem()
                            .map(|n| {
                                n.to_string_lossy().replace('_', " ") == s
                                    || n.to_string_lossy() == fname.as_str()
                            })
                            .unwrap_or(false)
                    }) {
                        match resolve_document_open(f) {
                            Some(ResolvedDocumentOpen::BuiltinRobcoTerminalWriter) => {
                                view_text_file(terminal, f)?;
                            }
                            Some(ResolvedDocumentOpen::ExternalArgv(cmd)) => {
                                launch_argv(terminal, &cmd)?;
                            }
                            None => {
                                flash_message(terminal, "Error: No App for filetype", 1200)?;
                            }
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

pub fn open_documents_category(terminal: &mut Term, title: &str, path: &Path) -> Result<()> {
    if !path.exists() || !path.is_dir() {
        let path_str = path.display().to_string();
        flash_message(terminal, &format!("Error: '{path_str}' not found."), 1000)?;
        return Ok(());
    }
    browse_folder(terminal, path, title)
}

// ── Documents menu ─────────────────────────────────────────────────────────────

pub fn documents_menu(terminal: &mut Term) -> Result<()> {
    loop {
        let categories = load_categories();
        let mut choices = vec!["Logs".to_string()];
        choices.extend(categories.keys().cloned());
        choices.push("---".to_string());
        choices.push("Back".to_string());
        let opts: Vec<&str> = choices.iter().map(String::as_str).collect();

        match run_menu(terminal, "Documents", &opts, Some("Select Document Type"))? {
            MenuResult::Back => break,
            MenuResult::Selected(s) if s == "Back" => break,
            MenuResult::Selected(s) if s == "Logs" => logs_menu(terminal)?,
            MenuResult::Selected(s) => {
                if let Some(v) = categories.get(&s) {
                    let path_str = v.as_str().unwrap_or("");
                    let path = PathBuf::from(path_str);
                    open_documents_category(terminal, &s, &path)?;
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use crossterm::event::{KeyCode, KeyModifiers};
    use std::path::{Path, PathBuf};

    use super::{
        log_dir, normalize_editor_file_name, normalize_new_text_document_name,
        resolve_editor_target_path, text_editor_dir, Editor, EditorAction, TerminalSaveAsState,
    };

    #[test]
    fn new_text_document_name_uses_default_txt_extension() {
        assert_eq!(
            normalize_new_text_document_name("", "2077-10-23"),
            Some("2077-10-23.txt".to_string())
        );
    }

    #[test]
    fn new_text_document_name_preserves_existing_extension() {
        assert_eq!(
            normalize_new_text_document_name("vault_notes.md", "default"),
            Some("vault_notes.md".to_string())
        );
    }

    #[test]
    fn new_text_document_name_strips_path_chars_and_spaces() {
        assert_eq!(
            normalize_new_text_document_name(" ../Field Report 01 ", "default"),
            Some("Field_Report_01.txt".to_string())
        );
    }

    #[test]
    fn logs_and_text_editor_use_different_base_folders() {
        assert_ne!(log_dir(), text_editor_dir());
    }

    #[test]
    fn terminal_editor_ctrl_shortcuts_map_to_expected_actions() {
        let mut editor = Editor::new("", PathBuf::from("note.txt"));
        assert_eq!(
            editor.key(KeyCode::Char('s'), KeyModifiers::CONTROL, 10, 40),
            EditorAction::Save
        );
        assert_eq!(
            editor.key(KeyCode::Char('a'), KeyModifiers::CONTROL, 10, 40),
            EditorAction::SaveAs
        );
        assert_eq!(
            editor.key(KeyCode::Char('r'), KeyModifiers::CONTROL, 10, 40),
            EditorAction::Rename
        );
        assert_eq!(
            editor.key(KeyCode::Char('f'), KeyModifiers::CONTROL, 10, 40),
            EditorAction::Search
        );
        assert_eq!(
            editor.key(KeyCode::Char('g'), KeyModifiers::CONTROL, 10, 40),
            EditorAction::FindNext
        );
        assert_eq!(
            editor.key(KeyCode::Char('q'), KeyModifiers::CONTROL, 10, 40),
            EditorAction::ForceClose
        );
    }

    #[test]
    fn editor_target_path_defaults_to_txt_extension() {
        assert_eq!(
            resolve_editor_target_path(Path::new("text_editor_documents/note.txt"), "draft"),
            Some(PathBuf::from("text_editor_documents/draft.txt"))
        );
    }

    #[test]
    fn editor_file_name_defaults_to_txt_extension() {
        assert_eq!(
            normalize_editor_file_name("draft", "document.txt"),
            Some("draft.txt".to_string())
        );
    }

    #[test]
    fn terminal_editor_search_tracks_all_matches() {
        let mut editor = Editor::new("alpha beta alpha", PathBuf::from("note.txt"));
        editor.search_query = "alpha".to_string();
        editor.refresh_search();
        assert_eq!(editor.search_matches, vec![(0, 0), (0, 11)]);
        editor.jump_to_match(0, 5, 8);
        editor.find_next(5, 8);
        assert_eq!((editor.row, editor.col), (0, 11));
    }

    #[test]
    fn terminal_save_as_starts_in_browser_mode() {
        let state = TerminalSaveAsState::new(Path::new("text_editor_documents/note.txt"));
        assert!(!state.input_mode);
    }
}
