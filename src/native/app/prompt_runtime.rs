use super::super::desktop_status_service::shell_status;
use super::super::file_manager_app::{self, FileManagerPromptAction, FileManagerPromptRequest};
use super::super::prompt::{TerminalPrompt, TerminalPromptAction, TerminalPromptKind};
use super::super::prompt_flow::PromptOutcome;
use super::RobcoNativeApp;

impl RobcoNativeApp {
    pub(super) fn open_password_prompt(
        &mut self,
        title: impl Into<String>,
        prompt: impl Into<String>,
    ) {
        crate::sound::play_navigate();
        self.terminal_prompt = Some(TerminalPrompt {
            kind: TerminalPromptKind::Password,
            title: title.into(),
            prompt: prompt.into(),
            buffer: String::new(),
            confirm_yes: true,
            action: TerminalPromptAction::LoginPassword,
        });
    }

    pub(super) fn open_input_prompt(
        &mut self,
        title: impl Into<String>,
        prompt: impl Into<String>,
        action: TerminalPromptAction,
    ) {
        self.open_input_prompt_with_buffer(title, prompt, String::new(), action);
    }

    pub(super) fn open_input_prompt_with_buffer(
        &mut self,
        title: impl Into<String>,
        prompt: impl Into<String>,
        buffer: String,
        action: TerminalPromptAction,
    ) {
        crate::sound::play_navigate();
        self.terminal_prompt = Some(TerminalPrompt {
            kind: TerminalPromptKind::Input,
            title: title.into(),
            prompt: prompt.into(),
            buffer,
            confirm_yes: true,
            action,
        });
    }

    pub(super) fn open_password_prompt_with_action(
        &mut self,
        title: impl Into<String>,
        prompt: impl Into<String>,
        action: TerminalPromptAction,
    ) {
        crate::sound::play_navigate();
        self.terminal_prompt = Some(TerminalPrompt {
            kind: TerminalPromptKind::Password,
            title: title.into(),
            prompt: prompt.into(),
            buffer: String::new(),
            confirm_yes: true,
            action,
        });
    }

    pub(super) fn open_confirm_prompt(
        &mut self,
        title: impl Into<String>,
        prompt: impl Into<String>,
        action: TerminalPromptAction,
    ) {
        crate::sound::play_navigate();
        self.terminal_prompt = Some(TerminalPrompt {
            kind: TerminalPromptKind::Confirm,
            title: title.into(),
            prompt: prompt.into(),
            buffer: String::new(),
            confirm_yes: true,
            action,
        });
    }

    pub(super) fn open_file_manager_prompt(&mut self, request: FileManagerPromptRequest) {
        self.terminal_prompt = Some(request.to_terminal_prompt());
    }

    pub(super) fn apply_shell_status_result(&mut self, result: Result<String, String>) {
        match result {
            Ok(status) | Err(status) => self.apply_status_update(shell_status(status)),
        }
    }

    pub(super) fn handle_file_manager_prompt_outcome(&mut self, outcome: &PromptOutcome) -> bool {
        let Some(actions) = file_manager_app::apply_prompt_outcome(
            outcome,
            &mut self.file_manager,
            &mut self.file_manager_runtime,
        ) else {
            return false;
        };
        self.terminal_prompt = None;
        for action in actions {
            match action {
                FileManagerPromptAction::Launch(launch) => {
                    self.shell_status = self.launch_open_with_request(launch);
                }
                FileManagerPromptAction::ApplySettingsUpdate(update) => {
                    self.apply_file_manager_settings_update(update);
                }
                FileManagerPromptAction::ReportStatus(status) => {
                    self.shell_status = status;
                }
            }
        }
        true
    }
}
