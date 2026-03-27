use super::desktop_app::{
    build_active_desktop_menu_section, build_shared_desktop_menu_section, DesktopHostedApp,
    DesktopMenuAction, DesktopMenuBuildContext, DesktopMenuItem, DesktopMenuSection,
};
use super::retro_ui::{current_palette_for_surface, ShellSurfaceKind};
use eframe::egui::{self, Context, Id, RichText};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandLayerTarget {
    Editor,
    FileManager,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandLayerSection {
    pub section: DesktopMenuSection,
    pub items: Vec<DesktopMenuItem>,
}

#[derive(Debug, Clone)]
pub struct CommandLayerState {
    pub open: bool,
    pub target: CommandLayerTarget,
    pub selected_section: usize,
    pub selection_path: Vec<usize>,
}

impl Default for CommandLayerState {
    fn default() -> Self {
        Self {
            open: false,
            target: CommandLayerTarget::Editor,
            selected_section: 0,
            selection_path: Vec::new(),
        }
    }
}

impl CommandLayerTarget {
    pub fn hosted_app(self) -> DesktopHostedApp {
        match self {
            CommandLayerTarget::Editor => DesktopHostedApp::Editor,
            CommandLayerTarget::FileManager => DesktopHostedApp::FileManager,
        }
    }
}

impl CommandLayerState {
    pub fn open_for_target(
        &mut self,
        target: CommandLayerTarget,
        sections: &[CommandLayerSection],
    ) {
        self.open = true;
        self.target = target;
        self.selected_section = 0;
        self.selection_path =
            first_selection_path(sections.first().map(|section| section.items.as_slice()));
    }

    fn clamp_to_sections(&mut self, sections: &[CommandLayerSection]) {
        if sections.is_empty() {
            self.open = false;
            self.selection_path.clear();
            self.selected_section = 0;
            return;
        }
        self.selected_section = self.selected_section.min(sections.len().saturating_sub(1));
        if self.selection_path.is_empty() {
            self.selection_path =
                first_selection_path(Some(&sections[self.selected_section].items));
        }
    }
}

pub fn build_command_layer_sections(
    target: CommandLayerTarget,
    context: &DesktopMenuBuildContext<'_>,
) -> Vec<CommandLayerSection> {
    let hosted_app = target.hosted_app();
    hosted_app
        .menu_sections()
        .iter()
        .copied()
        .filter(|section| {
            !matches!(
                section,
                DesktopMenuSection::View | DesktopMenuSection::Window
            )
        })
        .filter_map(|section| {
            let mut items = build_active_desktop_menu_section(hosted_app, section, context);
            if section == DesktopMenuSection::Help {
                items.extend(build_shared_desktop_menu_section(DesktopMenuSection::Help));
            }
            if items.is_empty() {
                None
            } else {
                Some(CommandLayerSection { section, items })
            }
        })
        .collect()
}

fn is_selectable(item: &DesktopMenuItem) -> bool {
    matches!(
        item,
        DesktopMenuItem::Action { .. } | DesktopMenuItem::Submenu { .. }
    )
}

fn selectable_indices(items: &[DesktopMenuItem]) -> Vec<usize> {
    items
        .iter()
        .enumerate()
        .filter_map(|(index, item)| is_selectable(item).then_some(index))
        .collect()
}

fn first_selection_path(items: Option<&[DesktopMenuItem]>) -> Vec<usize> {
    items
        .and_then(|items| selectable_indices(items).into_iter().next())
        .map(|index| vec![index])
        .unwrap_or_default()
}

fn selected_item<'a>(
    items: &'a [DesktopMenuItem],
    selection_path: &[usize],
) -> Option<&'a DesktopMenuItem> {
    let (&index, rest) = selection_path.split_first()?;
    let item = items.get(index)?;
    if rest.is_empty() {
        Some(item)
    } else if let DesktopMenuItem::Submenu { items, .. } = item {
        selected_item(items, rest)
    } else {
        None
    }
}

fn current_items<'a>(
    items: &'a [DesktopMenuItem],
    selection_path: &[usize],
) -> &'a [DesktopMenuItem] {
    if selection_path.len() <= 1 {
        return items;
    }
    let index = selection_path[0];
    match items.get(index) {
        Some(DesktopMenuItem::Submenu { items, .. }) => current_items(items, &selection_path[1..]),
        _ => items,
    }
}

fn selection_index_at_depth(selection_path: &[usize], depth: usize) -> Option<usize> {
    selection_path.get(depth).copied()
}

fn set_selection_index_at_depth(selection_path: &mut Vec<usize>, depth: usize, index: usize) {
    selection_path.truncate(depth + 1);
    if let Some(slot) = selection_path.get_mut(depth) {
        *slot = index;
    } else {
        selection_path.push(index);
    }
}

fn move_within_current_menu(
    selection_path: &mut Vec<usize>,
    root_items: &[DesktopMenuItem],
    delta: isize,
) {
    if selection_path.is_empty() {
        *selection_path = first_selection_path(Some(root_items));
        return;
    }
    let depth = selection_path.len().saturating_sub(1);
    let items = current_items(root_items, selection_path);
    let selectable = selectable_indices(items);
    if selectable.is_empty() {
        selection_path.clear();
        return;
    }
    let current = selection_index_at_depth(selection_path, depth)
        .and_then(|current| selectable.iter().position(|index| *index == current))
        .unwrap_or(0);
    let next =
        (current as isize + delta).clamp(0, selectable.len().saturating_sub(1) as isize) as usize;
    set_selection_index_at_depth(selection_path, depth, selectable[next]);
}

fn open_selected_submenu(selection_path: &mut Vec<usize>, root_items: &[DesktopMenuItem]) -> bool {
    let Some(DesktopMenuItem::Submenu { items, .. }) = selected_item(root_items, selection_path)
    else {
        return false;
    };
    let Some(first) = selectable_indices(items).into_iter().next() else {
        return false;
    };
    selection_path.push(first);
    true
}

fn close_selected_submenu(selection_path: &mut Vec<usize>) -> bool {
    if selection_path.len() > 1 {
        selection_path.pop();
        true
    } else {
        false
    }
}

fn panel_width_for_items(items: &[DesktopMenuItem]) -> f32 {
    let mut width: f32 = 220.0;
    for item in items {
        let text = match item {
            DesktopMenuItem::Action { label, .. }
            | DesktopMenuItem::Disabled { label }
            | DesktopMenuItem::Label { label }
            | DesktopMenuItem::Submenu { label, .. } => label.len(),
            DesktopMenuItem::Separator => 0,
        };
        width = width.max((text as f32) * 8.0 + 40.0);
    }
    width.min(420.0)
}

fn render_menu_panel(
    ctx: &Context,
    panel_id: &'static str,
    position: egui::Pos2,
    items: &[DesktopMenuItem],
    selected_index: Option<usize>,
) -> (Option<usize>, Option<DesktopMenuAction>, Option<egui::Rect>) {
    let mut hovered_index = None;
    let mut activated_action = None;
    let mut submenu_anchor = None;
    let palette = current_palette_for_surface(ShellSurfaceKind::Terminal);
    let panel_width = panel_width_for_items(items);

    egui::Area::new(Id::new(panel_id))
        .order(egui::Order::Foreground)
        .fixed_pos(position)
        .interactable(true)
        .show(ctx, |ui| {
            let frame = egui::Frame::none()
                .fill(palette.panel)
                .stroke(egui::Stroke::new(2.0, palette.fg))
                .inner_margin(egui::Margin::same(8.0));
            frame.show(ui, |ui| {
                ui.set_min_width(panel_width);
                ui.set_max_width(panel_width);
                for (index, item) in items.iter().enumerate() {
                    match item {
                        DesktopMenuItem::Action { label, action } => {
                            let selected = selected_index == Some(index);
                            let response = render_menu_row(ui, label, selected);
                            if response.hovered() {
                                hovered_index = Some(index);
                            }
                            if response.clicked() {
                                activated_action = Some(action.clone());
                            }
                        }
                        DesktopMenuItem::Submenu { label, .. } => {
                            let selected = selected_index == Some(index);
                            let response = render_menu_row(ui, label, selected);
                            if response.hovered() {
                                hovered_index = Some(index);
                            }
                            if selected {
                                submenu_anchor = Some(response.rect);
                            }
                        }
                        DesktopMenuItem::Disabled { label } => {
                            let response = render_disabled_row(ui, label);
                            if response.hovered() {
                                hovered_index = Some(index);
                            }
                        }
                        DesktopMenuItem::Label { label } => {
                            ui.label(RichText::new(label).small().color(palette.dim));
                        }
                        DesktopMenuItem::Separator => {
                            ui.add_space(2.0);
                            ui.separator();
                            ui.add_space(2.0);
                        }
                    }
                }
            });
        });

    (hovered_index, activated_action, submenu_anchor)
}

fn render_menu_row(ui: &mut egui::Ui, label: &str, selected: bool) -> egui::Response {
    let palette = current_palette_for_surface(ShellSurfaceKind::Terminal);
    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), 24.0), egui::Sense::click());
    let active = selected || response.hovered();
    let fill = if active { palette.fg } else { palette.panel };
    let text_color = if active {
        egui::Color32::BLACK
    } else {
        palette.fg
    };
    ui.painter().rect_filled(rect, 0.0, fill);
    ui.painter().text(
        egui::pos2(rect.left() + 8.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        label,
        egui::FontId::new(16.0, egui::FontFamily::Monospace),
        text_color,
    );
    response
}

fn render_disabled_row(ui: &mut egui::Ui, label: &str) -> egui::Response {
    let palette = current_palette_for_surface(ShellSurfaceKind::Terminal);
    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), 24.0), egui::Sense::hover());
    ui.painter().text(
        egui::pos2(rect.left() + 8.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        label,
        egui::FontId::new(16.0, egui::FontFamily::Monospace),
        palette.dim,
    );
    response
}

pub fn draw_command_layer(
    ctx: &Context,
    state: &mut CommandLayerState,
    sections: &[CommandLayerSection],
    block_rect: egui::Rect,
    bar_pos: egui::Pos2,
) -> Option<DesktopMenuAction> {
    state.clamp_to_sections(sections);
    if !state.open || sections.is_empty() {
        return None;
    }

    let no_mods = egui::Modifiers::NONE;
    if ctx.input(|i| i.key_pressed(egui::Key::F1) || i.key_pressed(egui::Key::Escape)) {
        state.open = false;
        ctx.input_mut(|i| {
            i.consume_key(no_mods, egui::Key::F1);
            i.consume_key(no_mods, egui::Key::Escape);
        });
        return None;
    }

    if ctx.input(|i| i.key_pressed(egui::Key::ArrowLeft)) {
        if !close_selected_submenu(&mut state.selection_path) {
            let next = state.selected_section.saturating_sub(1);
            if next != state.selected_section {
                state.selected_section = next;
                state.selection_path =
                    first_selection_path(Some(&sections[state.selected_section].items));
            }
        }
    }
    if ctx.input(|i| i.key_pressed(egui::Key::ArrowRight)) {
        if !open_selected_submenu(
            &mut state.selection_path,
            &sections[state.selected_section].items,
        ) {
            let next = (state.selected_section + 1).min(sections.len().saturating_sub(1));
            if next != state.selected_section {
                state.selected_section = next;
                state.selection_path =
                    first_selection_path(Some(&sections[state.selected_section].items));
            }
        }
    }
    if ctx.input(|i| i.key_pressed(egui::Key::ArrowUp)) {
        move_within_current_menu(
            &mut state.selection_path,
            &sections[state.selected_section].items,
            -1,
        );
    }
    if ctx.input(|i| i.key_pressed(egui::Key::ArrowDown)) {
        move_within_current_menu(
            &mut state.selection_path,
            &sections[state.selected_section].items,
            1,
        );
    }

    let activated =
        ctx.input(|i| i.key_pressed(egui::Key::Enter) || i.key_pressed(egui::Key::Space));
    ctx.input_mut(|i| {
        i.consume_key(no_mods, egui::Key::ArrowLeft);
        i.consume_key(no_mods, egui::Key::ArrowRight);
        i.consume_key(no_mods, egui::Key::ArrowUp);
        i.consume_key(no_mods, egui::Key::ArrowDown);
        i.consume_key(no_mods, egui::Key::Enter);
        i.consume_key(no_mods, egui::Key::Space);
        i.consume_key(no_mods, egui::Key::Tab);
        i.events.retain(|event| {
            !matches!(
                event,
                egui::Event::Key { .. } | egui::Event::Text(_) | egui::Event::Paste(_)
            )
        });
    });

    if activated {
        let root_items = &sections[state.selected_section].items;
        if !open_selected_submenu(&mut state.selection_path, root_items) {
            if let Some(DesktopMenuItem::Action { action, .. }) =
                selected_item(root_items, &state.selection_path)
            {
                state.open = false;
                return Some(action.clone());
            }
        }
    }

    let palette = current_palette_for_surface(ShellSurfaceKind::Terminal);
    egui::Area::new(Id::new("command_layer_blocker"))
        .order(egui::Order::Foreground)
        .fixed_pos(block_rect.min)
        .show(ctx, |ui| {
            let response = ui.allocate_rect(block_rect, egui::Sense::click());
            ui.painter()
                .rect_filled(block_rect, 0.0, egui::Color32::from_black_alpha(16));
            if response.clicked() {
                // Intentionally eat clicks outside the menu layer.
            }
        });

    let mut section_rects = Vec::with_capacity(sections.len());
    egui::Area::new(Id::new("command_layer_bar"))
        .order(egui::Order::Foreground)
        .fixed_pos(bar_pos)
        .show(ctx, |ui| {
            let frame = egui::Frame::none()
                .fill(palette.selected_bg)
                .stroke(egui::Stroke::new(1.0, palette.fg))
                .inner_margin(egui::Margin::symmetric(10.0, 6.0));
            frame.show(ui, |ui| {
                ui.horizontal(|ui| {
                    for (index, section) in sections.iter().enumerate() {
                        let selected = index == state.selected_section;
                        let text = if selected {
                            RichText::new(section.section.label())
                                .strong()
                                .color(egui::Color32::BLACK)
                        } else {
                            RichText::new(section.section.label())
                                .strong()
                                .color(egui::Color32::BLACK)
                        };
                        let button = egui::Button::new(text)
                            .fill(if selected {
                                palette.fg
                            } else {
                                palette.selected_bg
                            })
                            .stroke(egui::Stroke::new(1.0, palette.fg));
                        let response = ui.add(button);
                        if response.hovered() {
                            state.selected_section = index;
                            if state.selection_path.is_empty() {
                                state.selection_path =
                                    first_selection_path(Some(&sections[index].items));
                            }
                        }
                        if response.clicked() {
                            state.selected_section = index;
                            state.selection_path =
                                first_selection_path(Some(&sections[index].items));
                        }
                        section_rects.push(response.rect);
                    }
                    ui.add_space(12.0);
                    ui.label(
                        RichText::new("F1 close")
                            .small()
                            .color(egui::Color32::BLACK),
                    );
                });
            });
        });

    let Some(section_rect) = section_rects.get(state.selected_section).copied() else {
        return None;
    };

    let mut panel_position = egui::pos2(section_rect.left(), section_rect.bottom() - 1.0);
    let mut submenu_prefix = Vec::new();
    let mut submenu_anchor = None;

    for depth in 0..state.selection_path.len().max(1) {
        let items = if depth == 0 {
            &sections[state.selected_section].items
        } else if let Some(DesktopMenuItem::Submenu { items, .. }) =
            selected_item(&sections[state.selected_section].items, &submenu_prefix)
        {
            items
        } else {
            break;
        };

        if items.is_empty() {
            break;
        }

        let selected_index = selection_index_at_depth(&state.selection_path, depth);
        let panel_id = match depth {
            0 => "command_layer_menu_root",
            1 => "command_layer_menu_sub_1",
            2 => "command_layer_menu_sub_2",
            _ => "command_layer_menu_sub_extra",
        };
        let (hovered_index, action, next_anchor) =
            render_menu_panel(ctx, panel_id, panel_position, items, selected_index);
        if let Some(index) = hovered_index {
            set_selection_index_at_depth(&mut state.selection_path, depth, index);
            submenu_prefix = state.selection_path[..=depth].to_vec();
            if matches!(items.get(index), Some(DesktopMenuItem::Submenu { .. })) {
                if state.selection_path.len() == depth + 1 {
                    let _ = open_selected_submenu(
                        &mut state.selection_path,
                        &sections[state.selected_section].items,
                    );
                }
            } else {
                state.selection_path.truncate(depth + 1);
            }
        } else {
            submenu_prefix =
                state.selection_path[..state.selection_path.len().min(depth + 1)].to_vec();
        }

        if let Some(action) = action {
            state.open = false;
            return Some(action);
        }
        let Some(anchor) = next_anchor.or(submenu_anchor) else {
            break;
        };
        submenu_anchor = Some(anchor);
        panel_position = egui::pos2(anchor.right() - 1.0, anchor.top());
        if !matches!(
            selected_item(&sections[state.selected_section].items, &submenu_prefix),
            Some(DesktopMenuItem::Submenu { .. })
        ) {
            break;
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::DesktopFileManagerSettings;
    use crate::native::editor_app::EditorWindow;
    use crate::native::file_manager::NativeFileManagerState;
    use crate::native::file_manager_app::FileManagerEditRuntime;
    use std::path::PathBuf;

    fn build_test_context() -> DesktopMenuBuildContext<'static> {
        let editor = Box::leak(Box::new(EditorWindow::default()));
        let recent_files = Box::leak(Box::new(Vec::<String>::new()));
        let file_manager = Box::leak(Box::new(NativeFileManagerState::new(PathBuf::from("/"))));
        let runtime = Box::leak(Box::new(FileManagerEditRuntime::default()));
        let settings = Box::leak(Box::new(DesktopFileManagerSettings::default()));
        DesktopMenuBuildContext {
            editor,
            editor_recent_files: recent_files,
            file_manager,
            file_manager_runtime: runtime,
            file_manager_settings: settings,
        }
    }

    #[test]
    fn editor_command_layer_hides_view_and_window_sections() {
        let context = build_test_context();
        let sections = build_command_layer_sections(CommandLayerTarget::Editor, &context);

        assert!(sections.iter().all(|section| !matches!(
            section.section,
            DesktopMenuSection::View | DesktopMenuSection::Window
        )));
        assert!(sections
            .iter()
            .any(|section| matches!(section.section, DesktopMenuSection::File)));
        assert!(sections
            .iter()
            .any(|section| matches!(section.section, DesktopMenuSection::Edit)));
    }

    #[test]
    fn file_manager_command_layer_starts_with_selection_path() {
        let context = build_test_context();
        let sections = build_command_layer_sections(CommandLayerTarget::FileManager, &context);
        let mut state = CommandLayerState::default();

        state.open_for_target(CommandLayerTarget::FileManager, &sections);

        assert!(state.open);
        assert_eq!(state.selected_section, 0);
        assert_eq!(state.selection_path.len(), 1);
    }
}
