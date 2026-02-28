use super::menu::draw_terminal_menu_screen;
use crate::config::{DefaultAppBinding, Settings};
use crate::default_apps::{
    binding_for_slot, binding_label, default_app_choices, parse_custom_command_line, slot_label,
    DefaultAppChoiceAction, DefaultAppSlot,
};
use eframe::egui::Context;

#[derive(Debug, Clone)]
pub enum DefaultAppsEvent {
    None,
    Back,
    OpenSlot(DefaultAppSlot),
    CloseSlotPicker,
    SetBinding {
        slot: DefaultAppSlot,
        binding: DefaultAppBinding,
    },
    PromptCustom(DefaultAppSlot),
    Status(String),
}

#[allow(clippy::too_many_arguments)]
pub fn draw_default_apps_screen(
    ctx: &Context,
    draft: &Settings,
    root_idx: &mut usize,
    choice_idx: &mut usize,
    active_slot: &mut Option<DefaultAppSlot>,
    shell_status: &str,
    cols: usize,
    rows: usize,
    header_start_row: usize,
    separator_top_row: usize,
    title_row: usize,
    separator_bottom_row: usize,
    subtitle_row: usize,
    menu_start_row: usize,
    status_row: usize,
    content_col: usize,
) -> DefaultAppsEvent {
    match active_slot {
        Some(slot) => {
            let choices = default_app_choices(*slot);
            let mut items: Vec<String> = choices.iter().map(|c| c.label.clone()).collect();
            items.push("---".to_string());
            items.push("Back".to_string());
            let activated = draw_terminal_menu_screen(
                ctx,
                &format!("Default App: {}", slot_label(*slot)),
                None,
                &items,
                choice_idx,
                cols,
                rows,
                header_start_row,
                separator_top_row,
                title_row,
                separator_bottom_row,
                subtitle_row,
                menu_start_row,
                status_row,
                content_col,
                shell_status,
            );
            if let Some(idx) = activated {
                let label = &items[idx];
                if label == "Back" {
                    *active_slot = None;
                    return DefaultAppsEvent::CloseSlotPicker;
                }
                let Some(choice) = choices.iter().find(|choice| choice.label == *label) else {
                    return DefaultAppsEvent::None;
                };
                return match &choice.action {
                    DefaultAppChoiceAction::Set(binding) => DefaultAppsEvent::SetBinding {
                        slot: *slot,
                        binding: binding.clone(),
                    },
                    DefaultAppChoiceAction::PromptCustom => DefaultAppsEvent::PromptCustom(*slot),
                };
            }
            DefaultAppsEvent::None
        }
        None => {
            let items = vec![
                format!(
                    "Text/Code Files: {} [choose]",
                    binding_label(&binding_for_slot(draft, DefaultAppSlot::TextCode))
                ),
                format!(
                    "Ebook Files: {} [choose]",
                    binding_label(&binding_for_slot(draft, DefaultAppSlot::Ebook))
                ),
                "---".to_string(),
                "Back".to_string(),
            ];
            let activated = draw_terminal_menu_screen(
                ctx,
                "Default Apps",
                Some("Set default apps for your files."),
                &items,
                root_idx,
                cols,
                rows,
                header_start_row,
                separator_top_row,
                title_row,
                separator_bottom_row,
                subtitle_row,
                menu_start_row,
                status_row,
                content_col,
                shell_status,
            );
            if let Some(idx) = activated {
                return match idx {
                    0 => DefaultAppsEvent::OpenSlot(DefaultAppSlot::TextCode),
                    1 => DefaultAppsEvent::OpenSlot(DefaultAppSlot::Ebook),
                    _ => DefaultAppsEvent::Back,
                };
            }
            DefaultAppsEvent::None
        }
    }
}

pub fn apply_custom_command(slot: DefaultAppSlot, raw: &str) -> DefaultAppsEvent {
    let Some(argv) = parse_custom_command_line(raw.trim()) else {
        return DefaultAppsEvent::Status("Error: invalid command line".to_string());
    };
    DefaultAppsEvent::SetBinding {
        slot,
        binding: DefaultAppBinding::CustomArgv { argv },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn apply_custom_command_returns_custom_binding() {
        let event = apply_custom_command(DefaultAppSlot::TextCode, "epy --foo");
        match event {
            DefaultAppsEvent::SetBinding { slot, binding } => {
                assert_eq!(slot, DefaultAppSlot::TextCode);
                assert_eq!(
                    binding,
                    DefaultAppBinding::CustomArgv {
                        argv: vec!["epy".to_string(), "--foo".to_string()]
                    }
                );
            }
            other => panic!("unexpected event: {other:?}"),
        }
    }

    #[test]
    fn apply_custom_command_rejects_invalid_command_line() {
        let event = apply_custom_command(DefaultAppSlot::Ebook, "\"unterminated");
        assert!(matches!(
            event,
            DefaultAppsEvent::Status(ref status) if status == "Error: invalid command line"
        ));
    }
}
