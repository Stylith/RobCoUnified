use super::retro_ui::{current_palette, RetroScreen};
use eframe::egui::{self, Context, Id};
use robcos_native_file_manager_app::known_apps_for_extension;
use std::collections::HashSet;

#[derive(Debug, Clone)]
pub enum OpenWithPickerEntry {
    KnownApp { label: String, command: String },
    SavedCommand { command: String },
    Other,
}

impl OpenWithPickerEntry {
    pub fn display_label(&self) -> String {
        match self {
            Self::KnownApp { label, .. } => label.clone(),
            Self::SavedCommand { command } => command.clone(),
            Self::Other => "Other...".to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct OpenWithPickerState {
    pub open: bool,
    pub path: std::path::PathBuf,
    pub ext_key: String,
    pub entries: Vec<OpenWithPickerEntry>,
    pub selected: usize,
}

impl OpenWithPickerState {
    pub fn new(path: std::path::PathBuf, ext_key: String, saved_commands: Vec<String>) -> Self {
        let known_apps = known_apps_for_extension(&ext_key);
        let mut entries = Vec::new();
        let mut seen_commands: HashSet<String> = HashSet::new();

        for app in &known_apps {
            seen_commands.insert(app.command.clone());
            entries.push(OpenWithPickerEntry::KnownApp {
                label: app.label.clone(),
                command: app.command.clone(),
            });
        }

        for command in &saved_commands {
            if !seen_commands.contains(command) {
                entries.push(OpenWithPickerEntry::SavedCommand {
                    command: command.clone(),
                });
            }
        }

        entries.push(OpenWithPickerEntry::Other);

        Self {
            open: true,
            path,
            ext_key,
            entries,
            selected: 0,
        }
    }
}

#[derive(Debug, Clone)]
pub enum OpenWithPickerAction {
    LaunchCommand { command: String },
    OpenOtherPrompt,
}

pub fn draw_open_with_picker(
    ctx: &Context,
    state: &mut OpenWithPickerState,
    cols: usize,
    rows: usize,
) -> Option<OpenWithPickerAction> {
    let entry_count = state.entries.len();
    state.selected = state.selected.min(entry_count.saturating_sub(1));

    let no_mods = egui::Modifiers::NONE;
    if ctx.input(|i| i.key_pressed(egui::Key::ArrowUp)) {
        state.selected = state.selected.saturating_sub(1);
    }
    ctx.input_mut(|i| i.consume_key(no_mods, egui::Key::ArrowUp));
    if ctx.input(|i| i.key_pressed(egui::Key::ArrowDown)) {
        state.selected = (state.selected + 1).min(entry_count.saturating_sub(1));
    }
    ctx.input_mut(|i| i.consume_key(no_mods, egui::Key::ArrowDown));
    if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        state.open = false;
        ctx.input_mut(|i| {
            i.consume_key(no_mods, egui::Key::Escape);
        });
        return None;
    }
    let activated =
        ctx.input(|i| i.key_pressed(egui::Key::Enter) || i.key_pressed(egui::Key::Space));
    ctx.input_mut(|i| {
        i.consume_key(no_mods, egui::Key::Enter);
        i.consume_key(no_mods, egui::Key::Space);
        i.consume_key(no_mods, egui::Key::Tab);
        i.consume_key(no_mods, egui::Key::Q);
        i.consume_key(no_mods, egui::Key::O);
        i.consume_key(no_mods, egui::Key::F1);
    });

    let viewport = ctx.screen_rect();
    let mut result = None;

    let ext_label = robcos_native_file_manager_app::open_with_extension_label(&state.ext_key);
    let title = format!("Open With [{ext_label}]");

    egui::Area::new(Id::new("terminal_open_with_picker_overlay"))
        .order(egui::Order::Foreground)
        .fixed_pos(viewport.min)
        .show(ctx, |ui| {
            ui.set_min_size(viewport.size());
            let (screen, _) = RetroScreen::new(ui, cols, rows);
            let painter = ui.painter_at(screen.rect);
            let palette = current_palette();

            let max_visible = 12usize.min(entry_count);
            let box_h = max_visible + 4;
            let box_w = 50usize.min(cols.saturating_sub(4));
            let box_x = (cols.saturating_sub(box_w)) / 2;
            let box_y = rows.saturating_sub(box_h + 1);

            screen.boxed_panel(&painter, &palette, box_x, box_y, box_w, box_h);

            screen.text(&painter, box_x + 2, box_y + 1, &title, palette.fg);

            let scroll_offset = if state.selected >= max_visible {
                state.selected - max_visible + 1
            } else {
                0
            };

            let label_col = box_x + 2;
            let label_w = box_w.saturating_sub(4);

            for i in 0..max_visible {
                let entry_idx = scroll_offset + i;
                if entry_idx >= entry_count {
                    break;
                }
                let entry = &state.entries[entry_idx];
                let is_selected = entry_idx == state.selected;
                let row = box_y + 2 + i;

                let text = {
                    let label = entry.display_label();
                    if label.len() > label_w {
                        label[..label_w].to_string()
                    } else {
                        label
                    }
                };
                let response = screen.selectable_row(
                    ui,
                    &painter,
                    &palette,
                    label_col,
                    row,
                    &text,
                    is_selected,
                );

                if response.clicked() || (is_selected && activated) {
                    result = Some(match entry {
                        OpenWithPickerEntry::KnownApp { command, .. }
                        | OpenWithPickerEntry::SavedCommand { command } => {
                            OpenWithPickerAction::LaunchCommand {
                                command: command.clone(),
                            }
                        }
                        OpenWithPickerEntry::Other => OpenWithPickerAction::OpenOtherPrompt,
                    });
                }
            }

            screen.text(
                &painter,
                box_x + 2,
                box_y + box_h - 1,
                "Up/Down | Enter select | Esc close",
                palette.dim,
            );
        });

    if result.is_some() {
        state.open = false;
    }
    result
}
