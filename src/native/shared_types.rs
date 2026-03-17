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
    Applications,
    DonkeyKong,
    NukeCodes,
    TerminalMode,
    PtyApp,
    Installer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalScreen {
    MainMenu,
    Applications,
    Documents,
    Network,
    Games,
    DonkeyKong,
    NukeCodes,
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
