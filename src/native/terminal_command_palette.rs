use super::retro_ui::{current_palette, RetroScreen};
use eframe::egui::{self, Context, Id};

/// Which app the command palette is serving.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandPaletteTarget {
    Editor,
    DocumentBrowser,
}

/// A single entry in the command palette.
#[derive(Debug, Clone)]
pub struct CommandPaletteEntry {
    pub label: String,
    pub shortcut: &'static str,
    pub action: CommandPaletteAction,
}

/// Actions dispatched when a command palette entry is activated.
#[derive(Debug, Clone)]
pub enum CommandPaletteAction {
    // Editor commands
    EditorSave,
    EditorSaveAs,
    EditorNewDocument,
    EditorFind,
    EditorFindReplace,
    EditorUndo,
    EditorRedo,
    EditorCut,
    EditorCopy,
    EditorPaste,
    EditorSelectAll,
    EditorToggleWordWrap,
    EditorFontLarger,
    EditorFontSmaller,
    EditorFontReset,
    EditorAlignLeft,
    EditorAlignCenter,
    EditorAlignRight,
    EditorToggleLineNumbers,
    EditorClose,
    // File manager commands
    FmOpenSelected,
    FmNewFolder,
    FmHome,
    FmCopy,
    FmCut,
    FmPaste,
    FmDuplicate,
    FmRename,
    FmMoveTo,
    FmDelete,
    FmUndo,
    FmRedo,
    FmClearSearch,
    FmNewDocument,
    FmToggleHiddenFiles,
    FmClose,
}

/// Transient state for the command palette overlay.
#[derive(Debug, Clone)]
pub struct CommandPaletteState {
    pub open: bool,
    pub target: CommandPaletteTarget,
    pub selected: usize,
    /// Action deferred to the next frame (when ctx is available).
    pub pending_action: Option<CommandPaletteAction>,
}

impl Default for CommandPaletteState {
    fn default() -> Self {
        Self {
            open: false,
            target: CommandPaletteTarget::Editor,
            selected: 0,
            pending_action: None,
        }
    }
}

fn format_entry(label: &str, shortcut: &str, max_w: usize) -> String {
    if shortcut.is_empty() {
        label.to_string()
    } else {
        let gap = max_w.saturating_sub(label.len() + shortcut.len());
        format!("{}{}{}", label, " ".repeat(gap.max(1)), shortcut)
    }
}

pub fn editor_palette_entries() -> Vec<CommandPaletteEntry> {
    vec![
        CommandPaletteEntry {
            label: "Save".into(),
            shortcut: "Ctrl+S",
            action: CommandPaletteAction::EditorSave,
        },
        CommandPaletteEntry {
            label: "Save As".into(),
            shortcut: "",
            action: CommandPaletteAction::EditorSaveAs,
        },
        CommandPaletteEntry {
            label: "New Document".into(),
            shortcut: "Ctrl+N",
            action: CommandPaletteAction::EditorNewDocument,
        },
        CommandPaletteEntry {
            label: "Undo".into(),
            shortcut: "Ctrl+Z",
            action: CommandPaletteAction::EditorUndo,
        },
        CommandPaletteEntry {
            label: "Redo".into(),
            shortcut: "Ctrl+Y",
            action: CommandPaletteAction::EditorRedo,
        },
        CommandPaletteEntry {
            label: "Cut".into(),
            shortcut: "Ctrl+X",
            action: CommandPaletteAction::EditorCut,
        },
        CommandPaletteEntry {
            label: "Copy".into(),
            shortcut: "Ctrl+C",
            action: CommandPaletteAction::EditorCopy,
        },
        CommandPaletteEntry {
            label: "Paste".into(),
            shortcut: "Ctrl+V",
            action: CommandPaletteAction::EditorPaste,
        },
        CommandPaletteEntry {
            label: "Select All".into(),
            shortcut: "Ctrl+A",
            action: CommandPaletteAction::EditorSelectAll,
        },
        CommandPaletteEntry {
            label: "Find".into(),
            shortcut: "Ctrl+F",
            action: CommandPaletteAction::EditorFind,
        },
        CommandPaletteEntry {
            label: "Find & Replace".into(),
            shortcut: "Ctrl+H",
            action: CommandPaletteAction::EditorFindReplace,
        },
        CommandPaletteEntry {
            label: "Toggle Word Wrap".into(),
            shortcut: "",
            action: CommandPaletteAction::EditorToggleWordWrap,
        },
        CommandPaletteEntry {
            label: "Font Larger".into(),
            shortcut: "Ctrl++",
            action: CommandPaletteAction::EditorFontLarger,
        },
        CommandPaletteEntry {
            label: "Font Smaller".into(),
            shortcut: "Ctrl+-",
            action: CommandPaletteAction::EditorFontSmaller,
        },
        CommandPaletteEntry {
            label: "Font Reset".into(),
            shortcut: "",
            action: CommandPaletteAction::EditorFontReset,
        },
        CommandPaletteEntry {
            label: "Align Left".into(),
            shortcut: "",
            action: CommandPaletteAction::EditorAlignLeft,
        },
        CommandPaletteEntry {
            label: "Align Center".into(),
            shortcut: "",
            action: CommandPaletteAction::EditorAlignCenter,
        },
        CommandPaletteEntry {
            label: "Align Right".into(),
            shortcut: "",
            action: CommandPaletteAction::EditorAlignRight,
        },
        CommandPaletteEntry {
            label: "Toggle Line Numbers".into(),
            shortcut: "",
            action: CommandPaletteAction::EditorToggleLineNumbers,
        },
        CommandPaletteEntry {
            label: "Close Editor".into(),
            shortcut: "Esc/Tab",
            action: CommandPaletteAction::EditorClose,
        },
    ]
}

pub fn file_manager_palette_entries() -> Vec<CommandPaletteEntry> {
    vec![
        CommandPaletteEntry {
            label: "Open Selected".into(),
            shortcut: "Enter",
            action: CommandPaletteAction::FmOpenSelected,
        },
        CommandPaletteEntry {
            label: "New Folder".into(),
            shortcut: "Ctrl+Shift+N",
            action: CommandPaletteAction::FmNewFolder,
        },
        CommandPaletteEntry {
            label: "Home".into(),
            shortcut: "",
            action: CommandPaletteAction::FmHome,
        },
        CommandPaletteEntry {
            label: "Copy".into(),
            shortcut: "Ctrl+C",
            action: CommandPaletteAction::FmCopy,
        },
        CommandPaletteEntry {
            label: "Cut".into(),
            shortcut: "Ctrl+X",
            action: CommandPaletteAction::FmCut,
        },
        CommandPaletteEntry {
            label: "Paste".into(),
            shortcut: "Ctrl+V",
            action: CommandPaletteAction::FmPaste,
        },
        CommandPaletteEntry {
            label: "Duplicate".into(),
            shortcut: "",
            action: CommandPaletteAction::FmDuplicate,
        },
        CommandPaletteEntry {
            label: "Rename".into(),
            shortcut: "F2",
            action: CommandPaletteAction::FmRename,
        },
        CommandPaletteEntry {
            label: "Move To".into(),
            shortcut: "",
            action: CommandPaletteAction::FmMoveTo,
        },
        CommandPaletteEntry {
            label: "Delete".into(),
            shortcut: "Del",
            action: CommandPaletteAction::FmDelete,
        },
        CommandPaletteEntry {
            label: "Undo".into(),
            shortcut: "Ctrl+Z",
            action: CommandPaletteAction::FmUndo,
        },
        CommandPaletteEntry {
            label: "Redo".into(),
            shortcut: "Ctrl+Y",
            action: CommandPaletteAction::FmRedo,
        },
        CommandPaletteEntry {
            label: "New Document".into(),
            shortcut: "",
            action: CommandPaletteAction::FmNewDocument,
        },
        CommandPaletteEntry {
            label: "Clear Search".into(),
            shortcut: "",
            action: CommandPaletteAction::FmClearSearch,
        },
        CommandPaletteEntry {
            label: "Toggle Hidden Files".into(),
            shortcut: "",
            action: CommandPaletteAction::FmToggleHiddenFiles,
        },
        CommandPaletteEntry {
            label: "Close File Manager".into(),
            shortcut: "Q",
            action: CommandPaletteAction::FmClose,
        },
    ]
}

/// Draw the command palette as an overlay at the bottom of the terminal screen.
/// Returns the action if a command was selected, or None.
pub fn draw_command_palette(
    ctx: &Context,
    state: &mut CommandPaletteState,
    cols: usize,
    rows: usize,
) -> Option<CommandPaletteAction> {
    let entries = match state.target {
        CommandPaletteTarget::Editor => editor_palette_entries(),
        CommandPaletteTarget::DocumentBrowser => file_manager_palette_entries(),
    };
    let entry_count = entries.len();
    state.selected = state.selected.min(entry_count.saturating_sub(1));

    // Keyboard: navigate and select — consume keys so underlying app doesn't see them
    let no_mods = egui::Modifiers::NONE;
    if ctx.input(|i| i.key_pressed(egui::Key::ArrowUp)) {
        state.selected = state.selected.saturating_sub(1);
    }
    ctx.input_mut(|i| i.consume_key(no_mods, egui::Key::ArrowUp));
    if ctx.input(|i| i.key_pressed(egui::Key::ArrowDown)) {
        state.selected = (state.selected + 1).min(entry_count.saturating_sub(1));
    }
    ctx.input_mut(|i| i.consume_key(no_mods, egui::Key::ArrowDown));
    if ctx.input(|i| i.key_pressed(egui::Key::Escape) || i.key_pressed(egui::Key::F1)) {
        state.open = false;
        ctx.input_mut(|i| {
            i.consume_key(no_mods, egui::Key::Escape);
            i.consume_key(no_mods, egui::Key::F1);
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
    });

    let viewport = ctx.screen_rect();
    let mut result = None;

    egui::Area::new(Id::new("terminal_command_palette_overlay"))
        .order(egui::Order::Foreground)
        .fixed_pos(viewport.min)
        .show(ctx, |ui| {
            ui.set_min_size(viewport.size());
            let (screen, _) = RetroScreen::new(ui, cols, rows);
            let painter = ui.painter_at(screen.rect);
            let palette = current_palette();

            // Compute box dimensions — show up to 12 entries, scrolled
            let max_visible = 12usize.min(entry_count);
            let box_h = max_visible + 4; // 1 title + 1 blank + entries + 1 blank + 1 hint
            let box_w = 50usize.min(cols.saturating_sub(4));
            let box_x = (cols.saturating_sub(box_w)) / 2;
            let box_y = rows.saturating_sub(box_h + 1);

            screen.boxed_panel(&painter, &palette, box_x, box_y, box_w, box_h);

            // Title
            screen.text(&painter, box_x + 2, box_y + 1, "Commands", palette.fg);

            // Scrolling window
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
                let entry = &entries[entry_idx];
                let is_selected = entry_idx == state.selected;
                let row = box_y + 2 + i;

                let text = format_entry(&entry.label, entry.shortcut, label_w);
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
                    result = Some(entry.action.clone());
                }
            }

            // Bottom hint
            screen.text(
                &painter,
                box_x + 2,
                box_y + box_h - 1,
                "Up/Down | Enter select | F1 close",
                palette.dim,
            );
        });

    if result.is_some() {
        state.open = false;
    }
    result
}
