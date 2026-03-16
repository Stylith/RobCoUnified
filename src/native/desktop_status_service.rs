#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeStatusValue {
    Set(String),
    Clear,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct NativeStatusUpdate {
    pub shell: Option<NativeStatusValue>,
    pub settings: Option<NativeStatusValue>,
}

pub fn shell_status(message: impl Into<String>) -> NativeStatusUpdate {
    NativeStatusUpdate {
        shell: Some(NativeStatusValue::Set(message.into())),
        settings: None,
    }
}

pub fn clear_shell_status() -> NativeStatusUpdate {
    NativeStatusUpdate {
        shell: Some(NativeStatusValue::Clear),
        settings: None,
    }
}

pub fn settings_status(message: impl Into<String>) -> NativeStatusUpdate {
    NativeStatusUpdate {
        shell: None,
        settings: Some(NativeStatusValue::Set(message.into())),
    }
}

pub fn clear_settings_status() -> NativeStatusUpdate {
    NativeStatusUpdate {
        shell: None,
        settings: Some(NativeStatusValue::Clear),
    }
}

pub fn mirror_shell_to_settings(shell: &str) -> NativeStatusUpdate {
    settings_status(shell.to_string())
}

pub fn saved_shell_status() -> NativeStatusUpdate {
    shell_status("Settings saved.")
}

pub fn saved_settings_status() -> NativeStatusUpdate {
    settings_status("Settings saved.")
}

pub fn cancelled_shell_status() -> NativeStatusUpdate {
    shell_status("Cancelled.")
}

pub fn invalid_input_shell_status() -> NativeStatusUpdate {
    shell_status("Error: Invalid input.")
}

pub fn invalid_input_settings_status() -> NativeStatusUpdate {
    settings_status("Error: Invalid input.")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mirror_shell_to_settings_sets_settings_channel() {
        let update = mirror_shell_to_settings("hello");
        assert_eq!(
            update,
            NativeStatusUpdate {
                shell: None,
                settings: Some(NativeStatusValue::Set("hello".to_string())),
            }
        );
    }

    #[test]
    fn clear_shell_status_marks_shell_clear() {
        let update = clear_shell_status();
        assert_eq!(update.shell, Some(NativeStatusValue::Clear));
        assert_eq!(update.settings, None);
    }
}
