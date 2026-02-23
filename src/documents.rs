use anyhow::Result;
use chrono::Local;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    text::{Line, Span},
    widgets::Paragraph,
};
use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::config::{load_categories, get_current_user, is_allowed_extension};
use crate::launcher::launch_epy;
use crate::status::render_status_bar;
use crate::ui::{
    Term, title_style, normal_style, dim_style,
    run_menu, confirm, flash_message, pager,
    render_header, render_separator,
    MenuResult,
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
            if p.is_dir() { dirs.push(p); }
        }
    }
    dirs.sort_by_key(|d| d.file_name().unwrap_or_default().to_string_lossy().to_lowercase().to_string());
    dirs
}

fn sort_key(f: &Path) -> String {
    let name = f.file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .replace('_', " ")
        .to_lowercase();
    if name.starts_with("the ") { name[4..].to_string() } else { name }
}

// ── Inline text editor (journal) ──────────────────────────────────────────────

struct Editor {
    lines: Vec<String>,
    row:   usize,
    col:   usize,
}

impl Editor {
    fn new(text: &str) -> Self {
        let lines: Vec<String> = if text.is_empty() {
            vec![String::new()]
        } else {
            text.lines().map(str::to_string).collect()
        };
        Self { lines, row: 0, col: 0 }
    }

    fn text(&self) -> String { self.lines.join("\n") }

    fn key(&mut self, code: KeyCode) -> EditorAction {
        match code {
            KeyCode::Char('\x17') | KeyCode::F(2) => return EditorAction::Save,
            KeyCode::Char('\x18') | KeyCode::Esc  => return EditorAction::Cancel,
            KeyCode::Enter => {
                let rest = self.lines[self.row][self.col..].to_string();
                self.lines[self.row].truncate(self.col);
                self.row += 1;
                self.lines.insert(self.row, rest);
                self.col = 0;
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
            }
            KeyCode::Up    => { if self.row > 0 { self.row -= 1; self.col = self.col.min(self.lines[self.row].len()); } }
            KeyCode::Down  => { if self.row < self.lines.len()-1 { self.row += 1; self.col = self.col.min(self.lines[self.row].len()); } }
            KeyCode::Left  => { if self.col > 0 { self.col -= 1; } }
            KeyCode::Right => { let max = self.lines[self.row].len(); if self.col < max { self.col += 1; } }
            KeyCode::Char(c) => {
                self.lines[self.row].insert(self.col, c);
                self.col += 1;
            }
            _ => {}
        }
        EditorAction::None
    }
}

#[derive(PartialEq)]
enum EditorAction { None, Save, Cancel }

fn run_editor(terminal: &mut Term, title: &str, initial: &str) -> Result<Option<String>> {
    let mut ed = Editor::new(initial);

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
                    Constraint::Min(1),
                    Constraint::Length(1),
                    Constraint::Length(1),
                ])
                .split(size);

            render_header(f, chunks[0]);
            render_separator(f, chunks[1]);

            let tp = Paragraph::new(title).alignment(Alignment::Center).style(title_style());
            f.render_widget(tp, chunks[2]);
            crate::ui::render_separator(f, chunks[3]);

            let lines: Vec<Line> = ed.lines.iter().map(|l| Line::from(Span::styled(l.as_str(), normal_style()))).collect();
            f.render_widget(Paragraph::new(lines), chunks[4]);

            let hint = Paragraph::new("Ctrl+W = save   Ctrl+X / Esc = cancel").style(dim_style());
            f.render_widget(hint, chunks[5]);
            render_status_bar(f, chunks[6]);
        })?;

        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press { continue; }
                // Map Ctrl+W and Ctrl+X
                let code = match key.code {
                    KeyCode::Char('w') if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) => {
                        KeyCode::Char('\x17') // Ctrl+W
                    }
                    KeyCode::Char('x') if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) => {
                        KeyCode::Char('\x18') // Ctrl+X
                    }
                    other => other,
                };
                match ed.key(code) {
                    EditorAction::Save   => return Ok(Some(ed.text())),
                    EditorAction::Cancel => return Ok(None),
                    EditorAction::None   => {}
                }
            }
        }
    }
}

// ── Journal ───────────────────────────────────────────────────────────────────

fn journal_dir() -> PathBuf {
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

pub fn journal_new(terminal: &mut Term) -> Result<()> {
    let today = Local::now().format("%Y-%m-%d").to_string();
    let title = format!("New Entry — {today}");
    if let Some(text) = run_editor(terminal, &title, "")? {
        if !text.trim().is_empty() {
            let path = journal_dir().join(format!("{today}.txt"));
            let mut existing = std::fs::read_to_string(&path).unwrap_or_default();
            existing.push_str(&text);
            existing.push('\n');
            std::fs::write(&path, existing)?;
            flash_message(terminal, "Entry saved.", 800)?;
        }
    }
    Ok(())
}

pub fn journal_view(terminal: &mut Term) -> Result<()> {
    let dir = journal_dir();
    if !dir.exists() {
        return flash_message(terminal, "Error: journal_entries folder not found.", 800);
    }
    let mut logs: Vec<PathBuf> = std::fs::read_dir(&dir)?
        .flatten()
        .filter(|e| e.path().is_file())
        .map(|e| e.path())
        .collect();
    if logs.is_empty() {
        return flash_message(terminal, "Error: No entries found.", 800);
    }
    logs.sort_by(|a, b| b.cmp(a)); // newest first

    loop {
        let mut keys: Vec<String> = logs.iter()
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
        if !path.exists() { continue; }

        loop {
            match run_menu(terminal, &sel, &["View", "Edit", "Delete", "---", "Back"], None)? {
                MenuResult::Back => break,
                MenuResult::Selected(s) => match s.as_str() {
                    "View" => {
                        let text = std::fs::read_to_string(&path).unwrap_or_default();
                        pager(terminal, &text, &sel)?;
                    }
                    "Edit" => {
                        let text = std::fs::read_to_string(&path).unwrap_or_default();
                        if let Some(new_text) = run_editor(terminal, &sel, &text)? {
                            std::fs::write(&path, new_text + "\n")?;
                            flash_message(terminal, "Saved.", 800)?;
                        }
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
                }
            }
        }
    }
    Ok(())
}

pub fn logs_menu(terminal: &mut Term) -> Result<()> {
    loop {
        match run_menu(terminal, "Logs", &["Create New Log", "View Logs", "---", "Back"], None)? {
            MenuResult::Back => break,
            MenuResult::Selected(s) => match s.as_str() {
                "Create New Log" => journal_new(terminal)?,
                "View Logs"      => journal_view(terminal)?,
                _                => break,
            }
        }
    }
    Ok(())
}

// ── Folder browser ─────────────────────────────────────────────────────────────

fn browse_folder(terminal: &mut Term, folder: &Path, title: &str) -> Result<()> {
    loop {
        let subfolders = scan_subfolders(folder);
        let files      = scan_documents(folder);

        if subfolders.is_empty() && files.is_empty() {
            flash_message(terminal, "No documents or subfolders found.", 800)?;
            return Ok(());
        }

        let mut choices: Vec<String> = Vec::new();
        for sf in &subfolders {
            let name = sf.file_name().unwrap_or_default().to_string_lossy().to_string() + "/";
            choices.push(name);
        }
        for f in &files {
            let name = f.file_stem().unwrap_or_default().to_string_lossy().replace('_', " ").to_string();
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
                        p.file_name().map(|n| n.to_string_lossy() == sf_name).unwrap_or(false)
                    }) {
                        browse_folder(terminal, sf, sf_name)?;
                    }
                } else {
                    // File
                    let fname = s.replace(' ', "_");
                    if let Some(f) = files.iter().find(|p| {
                        p.file_stem().map(|n| {
                            n.to_string_lossy().replace('_', " ") == s ||
                            n.to_string_lossy() == fname.as_str()
                        }).unwrap_or(false)
                    }) {
                        launch_epy(terminal, f)?;
                    }
                }
            }
        }
    }
    Ok(())
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
                    if !path.exists() || !path.is_dir() {
                        flash_message(terminal, &format!("Error: '{path_str}' not found."), 1000)?;
                        continue;
                    }
                    browse_folder(terminal, &path, &s)?;
                }
            }
        }
    }
    Ok(())
}
