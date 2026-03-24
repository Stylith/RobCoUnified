use super::super::data::save_text_file;
use super::super::editor_app::EditorTextCommand;
use super::RobcoNativeApp;
use eframe::egui::{self, Context, Id};
use std::path::Path;

impl RobcoNativeApp {
    pub(super) fn run_editor_text_command(
        &mut self,
        ctx: &Context,
        text_edit_id: Id,
        command: EditorTextCommand,
    ) {
        let key = match command {
            EditorTextCommand::Undo => egui::Key::Z,
            EditorTextCommand::Redo => egui::Key::Y,
            EditorTextCommand::Cut => egui::Key::X,
            EditorTextCommand::Copy => egui::Key::C,
            EditorTextCommand::Paste => egui::Key::V,
            EditorTextCommand::SelectAll => egui::Key::A,
        };
        ctx.memory_mut(|m| m.request_focus(text_edit_id));
        ctx.input_mut(|i| {
            i.events.push(egui::Event::Key {
                key,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::COMMAND,
            })
        });
    }

    pub(super) fn editor_find_next(&mut self, ctx: &egui::Context, text_edit_id: egui::Id) {
        if self.editor.ui.find_query.is_empty() {
            return;
        }
        let text = self.editor.text.clone();
        let query = self.editor.ui.find_query.clone();
        let matches: Vec<usize> = text
            .char_indices()
            .filter_map(|(byte_idx, _)| {
                if text[byte_idx..].starts_with(query.as_str()) {
                    Some(byte_idx)
                } else {
                    None
                }
            })
            .collect();
        if matches.is_empty() {
            self.editor.status = format!("Not found: {}", query);
            return;
        }
        let idx = self.editor.ui.find_occurrence % matches.len();
        self.editor.ui.find_occurrence = idx + 1;
        let byte_start = matches[idx];
        let byte_end = byte_start + query.len();
        let char_start = text[..byte_start].chars().count();
        let char_end = text[..byte_end].chars().count();
        let mut state = egui::text_edit::TextEditState::load(ctx, text_edit_id).unwrap_or_default();
        state
            .cursor
            .set_char_range(Some(egui::text::CCursorRange::two(
                egui::text::CCursor::new(char_start),
                egui::text::CCursor::new(char_end),
            )));
        state.store(ctx, text_edit_id);
        ctx.memory_mut(|m| m.request_focus(text_edit_id));
        self.editor.status = format!("Match {} of {}", idx + 1, matches.len());
    }

    pub(super) fn editor_replace_one(&mut self, ctx: &egui::Context, text_edit_id: egui::Id) {
        if self.editor.ui.find_query.is_empty() {
            return;
        }
        let query = self.editor.ui.find_query.clone();
        let replacement = self.editor.ui.replace_query.clone();
        if let Some(pos) = self.editor.text.find(&query) {
            self.editor
                .text
                .replace_range(pos..pos + query.len(), &replacement);
            self.editor.dirty = true;
        }
        self.editor_find_next(ctx, text_edit_id);
    }

    pub(super) fn editor_replace_all(&mut self) {
        if self.editor.ui.find_query.is_empty() {
            return;
        }
        let query = self.editor.ui.find_query.clone();
        let replacement = self.editor.ui.replace_query.clone();
        let count = self.editor.text.matches(query.as_str()).count();
        if count > 0 {
            self.editor.text = self.editor.text.replace(query.as_str(), &replacement);
            self.editor.dirty = true;
            self.editor.status = format!("Replaced {} occurrences.", count);
        } else {
            self.editor.status = format!("Not found: {}", query);
        }
    }

    pub(super) fn push_editor_recent_file(&mut self, path: &Path) {
        if let Some(s) = path.to_str() {
            let s = s.to_string();
            self.settings.draft.editor_recent_files.retain(|p| p != &s);
            self.settings.draft.editor_recent_files.insert(0, s);
            self.settings.draft.editor_recent_files.truncate(10);
        }
    }

    pub(super) fn save_editor_to_current_path(&mut self) -> bool {
        let Some(path) = self.editor.path.clone() else {
            self.open_editor_save_as_picker();
            return false;
        };
        match save_text_file(&path, &self.editor.text) {
            Ok(()) => {
                self.editor.dirty = false;
                self.editor.status = format!(
                    "Saved {}.",
                    path.file_name()
                        .and_then(|name| name.to_str())
                        .unwrap_or("document")
                );
                self.editor.cancel_close_confirmation();
                self.push_editor_recent_file(&path);
                true
            }
            Err(err) => {
                self.editor.status = format!("Save failed: {err}");
                false
            }
        }
    }

    pub(super) fn save_editor(&mut self) {
        let _ = self.save_editor_to_current_path();
    }

    pub(super) fn confirm_editor_close_save(&mut self) {
        if self.editor.path.is_some() {
            if self.save_editor_to_current_path() {
                self.close_current_editor_window_unchecked();
            } else {
                self.editor.prompt_close_confirmation();
            }
            return;
        }

        self.editor.queue_close_after_save();
        self.open_editor_save_as_picker();
        if self.editor.save_as_input.is_none() {
            self.editor.prompt_close_confirmation();
        }
    }
}
