use nucleon_native_services::desktop_default_apps_service::{
    binding_label_for_slot, default_app_binding_matches, default_app_choices_for_slot,
    default_app_slot_label, DefaultAppChoiceAction, DefaultAppSlot,
};
use nucleon_shared::config::{DefaultAppBinding, Settings};

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
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TerminalDefaultAppsRequest {
    None,
    BackToSettings,
    OpenSlot(DefaultAppSlot),
    CloseSlotPicker,
    ApplyBinding {
        slot: DefaultAppSlot,
        binding: DefaultAppBinding,
    },
    PromptCustom {
        slot: DefaultAppSlot,
        prompt_label: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DefaultAppSettingsChoice {
    pub label: String,
    pub binding: DefaultAppBinding,
    pub selected: bool,
}

pub fn build_default_apps_root_items(draft: &Settings) -> Vec<String> {
    vec![
        format!(
            "Text/Code Files: {} [choose]",
            binding_label_for_slot(draft, DefaultAppSlot::TextCode)
        ),
        format!(
            "Ebook Files: {} [choose]",
            binding_label_for_slot(draft, DefaultAppSlot::Ebook)
        ),
        "---".to_string(),
        "Back".to_string(),
    ]
}

pub fn default_app_slot_description(slot: DefaultAppSlot) -> &'static str {
    match slot {
        DefaultAppSlot::TextCode => "Used when opening text documents and code files.",
        DefaultAppSlot::Ebook => "Used when opening ebook and reader-oriented documents.",
    }
}

pub fn build_default_app_choice_items(slot: DefaultAppSlot) -> Vec<String> {
    let mut items: Vec<String> = default_app_choices_for_slot(slot)
        .into_iter()
        .map(|choice| choice.label)
        .collect();
    items.push("---".to_string());
    items.push("Back".to_string());
    items
}

pub fn build_default_app_settings_choices(
    settings: &Settings,
    slot: DefaultAppSlot,
) -> Vec<DefaultAppSettingsChoice> {
    default_app_choices_for_slot(slot)
        .into_iter()
        .filter_map(|choice| match choice.action {
            DefaultAppChoiceAction::Set(binding) => Some(DefaultAppSettingsChoice {
                selected: default_app_binding_matches(settings, slot, &binding),
                label: choice.label,
                binding,
            }),
            DefaultAppChoiceAction::PromptCustom => None,
        })
        .collect()
}

pub fn resolve_default_apps_root_event(activated: Option<usize>) -> DefaultAppsEvent {
    match activated {
        Some(0) => DefaultAppsEvent::OpenSlot(DefaultAppSlot::TextCode),
        Some(1) => DefaultAppsEvent::OpenSlot(DefaultAppSlot::Ebook),
        Some(_) => DefaultAppsEvent::Back,
        None => DefaultAppsEvent::None,
    }
}

pub fn resolve_default_apps_choice_event(
    slot: DefaultAppSlot,
    activated: Option<usize>,
) -> DefaultAppsEvent {
    let choices = default_app_choices_for_slot(slot);
    match activated {
        Some(idx) if idx < choices.len() => match &choices[idx].action {
            DefaultAppChoiceAction::Set(binding) => DefaultAppsEvent::SetBinding {
                slot,
                binding: binding.clone(),
            },
            DefaultAppChoiceAction::PromptCustom => DefaultAppsEvent::PromptCustom(slot),
        },
        Some(_) => DefaultAppsEvent::CloseSlotPicker,
        None => DefaultAppsEvent::None,
    }
}

pub fn resolve_terminal_default_apps_request(
    event: DefaultAppsEvent,
) -> TerminalDefaultAppsRequest {
    match event {
        DefaultAppsEvent::None => TerminalDefaultAppsRequest::None,
        DefaultAppsEvent::Back => TerminalDefaultAppsRequest::BackToSettings,
        DefaultAppsEvent::OpenSlot(slot) => TerminalDefaultAppsRequest::OpenSlot(slot),
        DefaultAppsEvent::CloseSlotPicker => TerminalDefaultAppsRequest::CloseSlotPicker,
        DefaultAppsEvent::SetBinding { slot, binding } => {
            TerminalDefaultAppsRequest::ApplyBinding { slot, binding }
        }
        DefaultAppsEvent::PromptCustom(slot) => TerminalDefaultAppsRequest::PromptCustom {
            slot,
            prompt_label: default_app_slot_label(slot).to_string(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nucleon_shared::config::get_settings;

    #[test]
    fn build_default_apps_root_items_contains_two_slots_and_back() {
        let items = build_default_apps_root_items(&get_settings());
        assert_eq!(items.len(), 4);
        assert!(items[0].starts_with("Text/Code Files: "));
        assert!(items[1].starts_with("Ebook Files: "));
        assert_eq!(items[3], "Back");
    }

    #[test]
    fn resolve_default_apps_root_event_routes_slots_and_back() {
        assert!(matches!(
            resolve_default_apps_root_event(Some(0)),
            DefaultAppsEvent::OpenSlot(DefaultAppSlot::TextCode)
        ));
        assert!(matches!(
            resolve_default_apps_root_event(Some(1)),
            DefaultAppsEvent::OpenSlot(DefaultAppSlot::Ebook)
        ));
        assert!(matches!(
            resolve_default_apps_root_event(Some(3)),
            DefaultAppsEvent::Back
        ));
    }

    #[test]
    fn resolve_default_apps_choice_event_handles_back_and_custom() {
        let items = build_default_app_choice_items(DefaultAppSlot::TextCode);
        assert_eq!(items.last().map(String::as_str), Some("Back"));
        assert!(matches!(
            resolve_default_apps_choice_event(DefaultAppSlot::TextCode, Some(items.len() - 1)),
            DefaultAppsEvent::CloseSlotPicker
        ));
    }

    #[test]
    fn build_default_app_settings_choices_marks_selected_binding() {
        let settings = get_settings();
        let choices = build_default_app_settings_choices(&settings, DefaultAppSlot::TextCode);

        assert!(!choices.is_empty());
        assert!(choices.iter().any(|choice| choice.selected));
    }

    #[test]
    fn resolve_terminal_default_apps_request_formats_custom_prompt() {
        assert_eq!(
            resolve_terminal_default_apps_request(DefaultAppsEvent::PromptCustom(
                DefaultAppSlot::Ebook
            )),
            TerminalDefaultAppsRequest::PromptCustom {
                slot: DefaultAppSlot::Ebook,
                prompt_label: "Ebook Files".to_string(),
            }
        );
    }
}
