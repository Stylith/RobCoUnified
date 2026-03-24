use super::super::data::logs_dir;
use super::super::desktop_app::DesktopWindow;
use super::super::desktop_documents_service::document_category_names;
use super::super::menu::TerminalScreen;
use super::RobcoNativeApp;
use chrono::Local;
use std::path::{Path, PathBuf};

impl RobcoNativeApp {
    pub(super) fn sorted_document_categories() -> Vec<String> {
        document_category_names()
    }

    pub(super) fn open_document_browser_at(&mut self, dir: PathBuf, return_screen: TerminalScreen) {
        if !dir.is_dir() {
            self.shell_status = format!("Error: '{}' not found.", dir.display());
            return;
        }
        self.file_manager.set_cwd(dir);
        self.file_manager.selected = None;
        self.terminal_nav.browser_idx = 0;
        self.terminal_nav.browser_return_screen = return_screen;
        self.navigate_to_screen(TerminalScreen::DocumentBrowser);
    }

    pub(super) fn open_log_view(&mut self) {
        self.open_document_browser_at(logs_dir(), TerminalScreen::Logs);
    }

    pub(super) fn normalize_new_file_name(raw: &str, default_stem: &str) -> Option<String> {
        let candidate = if raw.trim().is_empty() {
            default_stem.to_string()
        } else {
            raw.trim().to_string()
        };
        let mut normalized = String::new();
        let mut last_was_sep = false;
        for ch in candidate.chars() {
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.') {
                normalized.push(ch);
                last_was_sep = false;
            } else if ch.is_whitespace() && !normalized.is_empty() && !last_was_sep {
                normalized.push('_');
                last_was_sep = true;
            }
        }
        let normalized = normalized.trim_matches(['_', '.', ' ']).to_string();
        if normalized.is_empty() || normalized == "." || normalized == ".." {
            return None;
        }
        if Path::new(&normalized).extension().is_some() {
            Some(normalized)
        } else {
            Some(format!("{normalized}.txt"))
        }
    }

    pub(super) fn create_or_open_log(&mut self, raw_name: &str) {
        let default_stem = Local::now().format("%Y-%m-%d").to_string();
        let Some(name) = Self::normalize_new_file_name(raw_name, &default_stem) else {
            self.shell_status = "Error: Invalid document name.".to_string();
            return;
        };
        let path = logs_dir().join(name);
        let existing = if path.exists() {
            std::fs::read_to_string(&path).unwrap_or_default()
        } else {
            String::new()
        };
        self.editor.path = Some(path);
        self.editor.text = existing;
        self.editor.dirty = false;
        self.open_desktop_window(DesktopWindow::Editor);
        self.editor.status = "Opened log.".to_string();
        self.shell_status = "Opened log editor.".to_string();
    }
}
