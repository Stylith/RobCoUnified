use crate::config::{DefaultAppBinding, Settings};
use crate::default_apps::{
    binding_for_slot, binding_label, parse_custom_command_line, set_binding_for_slot,
    DefaultAppSlot,
};

pub fn binding_label_for_slot(settings: &Settings, slot: DefaultAppSlot) -> String {
    binding_label(&binding_for_slot(settings, slot))
}

pub fn custom_command_input_for_slot(settings: &Settings, slot: DefaultAppSlot) -> String {
    match binding_for_slot(settings, slot) {
        DefaultAppBinding::CustomArgv { argv } => argv.join(" "),
        _ => String::new(),
    }
}

pub fn apply_default_app_binding(
    settings: &mut Settings,
    slot: DefaultAppSlot,
    binding: DefaultAppBinding,
) {
    set_binding_for_slot(settings, slot, binding);
}

pub fn resolve_custom_default_app_binding(raw: &str) -> Result<DefaultAppBinding, String> {
    let Some(argv) = parse_custom_command_line(raw.trim()) else {
        return Err("Error: invalid command line".to_string());
    };
    if argv.is_empty() {
        return Err("Error: invalid command line".to_string());
    }
    Ok(DefaultAppBinding::CustomArgv { argv })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Settings;

    #[test]
    fn custom_command_input_for_slot_returns_custom_argv_text() {
        let mut settings = Settings::default();
        apply_default_app_binding(
            &mut settings,
            DefaultAppSlot::TextCode,
            DefaultAppBinding::CustomArgv {
                argv: vec!["epy".to_string(), "--foo".to_string()],
            },
        );

        assert_eq!(
            custom_command_input_for_slot(&settings, DefaultAppSlot::TextCode),
            "epy --foo"
        );
    }

    #[test]
    fn resolve_custom_default_app_binding_rejects_invalid_command_line() {
        let err =
            resolve_custom_default_app_binding("\"unterminated").expect_err("invalid command");

        assert_eq!(err, "Error: invalid command line");
    }
}
