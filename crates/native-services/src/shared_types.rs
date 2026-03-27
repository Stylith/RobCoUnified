use crate::core::auth::UserRecord;

#[derive(Debug, Clone)]
pub enum FlashAction {
    Noop,
    ExitApp,
    FinishLogout,
    FinishLogin {
        username: String,
        user: UserRecord,
    },
    StartHacking {
        username: String,
    },
    LaunchPty {
        title: String,
        argv: Vec<String>,
        return_screen: TerminalScreen,
        status: String,
        completion_message: Option<String>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DesktopWindow {
    FileManager,
    Editor,
    Settings,
    Tweaks,
    Applications,
    TerminalMode,
    PtyApp,
    Installer,
}

/// Unique identifier for a desktop window instance.
///
/// `kind` identifies the application type, `instance` distinguishes
/// multiple windows of the same kind (0 = primary, 1+ = additional).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WindowInstanceId {
    pub kind: DesktopWindow,
    pub instance: u32,
}

impl WindowInstanceId {
    pub fn primary(kind: DesktopWindow) -> Self {
        Self { kind, instance: 0 }
    }
}

impl From<DesktopWindow> for WindowInstanceId {
    fn from(kind: DesktopWindow) -> Self {
        Self::primary(kind)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalScreen {
    MainMenu,
    Applications,
    Documents,
    Network,
    Games,
    PtyApp,
    ProgramInstaller,
    Logs,
    DocumentBrowser,
    Settings,
    EditMenus,
    Connections,
    DefaultApps,
    About,
    UserManagement,
}
