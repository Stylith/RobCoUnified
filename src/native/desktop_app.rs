#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DesktopHostedApp {
    Desktop,
    FileManager,
    Editor,
    Settings,
    Applications,
    Game,
    Utility,
    Terminal,
    Installer,
    PtyApp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DesktopMenuSection {
    File,
    Edit,
    Format,
    View,
    Window,
    Help,
}

impl DesktopHostedApp {
    pub fn menu_sections(self) -> &'static [DesktopMenuSection] {
        match self {
            DesktopHostedApp::Editor => &[
                DesktopMenuSection::File,
                DesktopMenuSection::Edit,
                DesktopMenuSection::Format,
                DesktopMenuSection::View,
                DesktopMenuSection::Window,
                DesktopMenuSection::Help,
            ],
            _ => &[
                DesktopMenuSection::File,
                DesktopMenuSection::Edit,
                DesktopMenuSection::View,
                DesktopMenuSection::Window,
                DesktopMenuSection::Help,
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn editor_menu_profile_includes_format_menu() {
        assert!(DesktopHostedApp::Editor
            .menu_sections()
            .contains(&DesktopMenuSection::Format));
    }

    #[test]
    fn file_manager_menu_profile_omits_format_menu() {
        assert!(!DesktopHostedApp::FileManager
            .menu_sections()
            .contains(&DesktopMenuSection::Format));
    }
}
