use super::file_manager::{FileEntryRow, FileManagerCommand, FileTreeItem, NativeFileManagerState};
use crate::config::{DesktopFileManagerSettings, FileManagerViewMode};
use std::path::PathBuf;

pub const FILE_MANAGER_APP_TITLE: &str = "My Computer";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileManagerDesktopFooterAction {
    OpenHome,
    GoUp,
    NewFolder,
    NewDocument,
    OpenSelected,
    SaveHere,
    CancelSavePicker,
    ChooseIcon,
    CancelIconPicker,
    ChooseWallpaper,
    CancelWallpaperPicker,
    ImportTheme,
    CancelThemeImportPicker,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileManagerDesktopFooterRequest {
    RunCommand(FileManagerCommand),
    NewDocument,
    CompleteSaveAs,
    CancelSavePicker,
    CommitIconPicker,
    CancelIconPicker,
    CommitWallpaperPicker,
    CancelWallpaperPicker,
    CommitThemeImportPicker,
    CancelThemeImportPicker,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileManagerDesktopFooterButton {
    pub action: FileManagerDesktopFooterAction,
    pub label: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileManagerDesktopTab {
    pub path: PathBuf,
    pub title: String,
    pub active: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileManagerDesktopDrive {
    pub path: PathBuf,
    pub label: String,
    pub active: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileManagerDesktopActionMode {
    Normal,
    SavePicker { file_name: String },
    IconPicker,
    WallpaperPicker,
    ThemeImportPicker,
}

impl FileManagerDesktopActionMode {
    pub fn banner(&self) -> Option<&'static str> {
        match self {
            Self::Normal => None,
            Self::SavePicker { .. } => Some("[ Save As mode ]"),
            Self::IconPicker => Some("[ Pick icon mode ]"),
            Self::WallpaperPicker => Some("[ Pick wallpaper mode ]"),
            Self::ThemeImportPicker => Some("[ Import theme mode ]"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileManagerDesktopStatus {
    pub row_count: usize,
    pub selected_count: usize,
    pub view_label: String,
    pub tree_label: String,
    pub has_editable_selection: bool,
    pub has_single_file_selection: bool,
    pub has_clipboard: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileManagerDesktopFooterModel {
    pub status_items: Vec<String>,
    pub leading_buttons: Vec<FileManagerDesktopFooterButton>,
    pub trailing_buttons: Vec<FileManagerDesktopFooterButton>,
    pub file_name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileManagerDesktopViewModel {
    pub action_mode: FileManagerDesktopActionMode,
    pub tabs: Vec<FileManagerDesktopTab>,
    pub drives: Vec<FileManagerDesktopDrive>,
    pub rows: Vec<FileEntryRow>,
    pub tree_items: Vec<FileTreeItem>,
    pub current_drive_label: Option<String>,
    pub path_label: String,
    pub search_query: String,
    pub show_tree_panel: bool,
    pub view_mode: FileManagerViewMode,
    pub status: FileManagerDesktopStatus,
}

pub fn desktop_action_mode(
    save_as_input: Option<String>,
    picking_icon_for_shortcut: Option<usize>,
    picking_wallpaper: bool,
    picking_theme_import: bool,
) -> FileManagerDesktopActionMode {
    if let Some(file_name) = save_as_input {
        FileManagerDesktopActionMode::SavePicker { file_name }
    } else if picking_icon_for_shortcut.is_some() {
        FileManagerDesktopActionMode::IconPicker
    } else if picking_wallpaper {
        FileManagerDesktopActionMode::WallpaperPicker
    } else if picking_theme_import {
        FileManagerDesktopActionMode::ThemeImportPicker
    } else {
        FileManagerDesktopActionMode::Normal
    }
}

pub fn build_desktop_view_model(
    file_manager: &NativeFileManagerState,
    settings: &DesktopFileManagerSettings,
    rows: &[FileEntryRow],
    selected_count: usize,
    has_editable_selection: bool,
    has_single_file_selection: bool,
    has_clipboard: bool,
    save_as_input: Option<String>,
    picking_icon_for_shortcut: Option<usize>,
    picking_wallpaper: bool,
    picking_theme_import: bool,
) -> FileManagerDesktopViewModel {
    let action_mode = desktop_action_mode(
        save_as_input,
        picking_icon_for_shortcut,
        picking_wallpaper,
        picking_theme_import,
    );
    let path_label = file_manager.cwd.display().to_string();
    let current_drive = file_manager.current_drive_root();
    FileManagerDesktopViewModel {
        tabs: file_manager
            .tabs
            .iter()
            .enumerate()
            .map(|(idx, path)| FileManagerDesktopTab {
                path: path.clone(),
                title: NativeFileManagerState::tab_title(path),
                active: idx == file_manager.active_tab,
            })
            .collect(),
        drives: NativeFileManagerState::drive_roots()
            .into_iter()
            .map(|path| FileManagerDesktopDrive {
                label: NativeFileManagerState::drive_label(&path),
                active: current_drive.as_ref() == Some(&path),
                path,
            })
            .collect(),
        rows: rows.to_vec(),
        tree_items: file_manager.tree_items(),
        current_drive_label: current_drive
            .as_ref()
            .map(|drive| NativeFileManagerState::drive_label(drive)),
        path_label,
        search_query: file_manager.search_query.clone(),
        show_tree_panel: settings.show_tree_panel,
        view_mode: settings.view_mode,
        status: FileManagerDesktopStatus {
            row_count: rows.len(),
            selected_count,
            view_label: match settings.view_mode {
                FileManagerViewMode::Grid => "Grid View".to_string(),
                FileManagerViewMode::List => "List View".to_string(),
            },
            tree_label: if settings.show_tree_panel {
                "Tree On".to_string()
            } else {
                "Tree Off".to_string()
            },
            has_editable_selection,
            has_single_file_selection,
            has_clipboard,
        },
        action_mode,
    }
}

impl FileManagerDesktopViewModel {
    pub fn close_tab_enabled(&self) -> bool {
        self.tabs.len() > 1
    }

    pub fn grid_columns(&self, available_width: f32, tile_width: f32) -> usize {
        ((available_width / tile_width).floor() as usize).max(1)
    }
}

pub fn build_footer_model(model: &FileManagerDesktopViewModel) -> FileManagerDesktopFooterModel {
    match &model.action_mode {
        FileManagerDesktopActionMode::SavePicker { file_name } => FileManagerDesktopFooterModel {
            status_items: vec!["Save picker".to_string()],
            leading_buttons: vec![
                FileManagerDesktopFooterButton {
                    action: FileManagerDesktopFooterAction::OpenHome,
                    label: "Home",
                },
                FileManagerDesktopFooterButton {
                    action: FileManagerDesktopFooterAction::GoUp,
                    label: "Up",
                },
                FileManagerDesktopFooterButton {
                    action: FileManagerDesktopFooterAction::NewFolder,
                    label: "New Folder",
                },
            ],
            trailing_buttons: vec![
                FileManagerDesktopFooterButton {
                    action: FileManagerDesktopFooterAction::CancelSavePicker,
                    label: "Cancel",
                },
                FileManagerDesktopFooterButton {
                    action: FileManagerDesktopFooterAction::SaveHere,
                    label: "Save Here",
                },
            ],
            file_name: Some(file_name.clone()),
        },
        FileManagerDesktopActionMode::IconPicker => FileManagerDesktopFooterModel {
            status_items: vec![
                format!("{} item(s)", model.status.row_count),
                format!("{} selected", model.status.selected_count),
                model.status.view_label.clone(),
                model.status.tree_label.clone(),
            ],
            leading_buttons: vec![],
            trailing_buttons: vec![
                FileManagerDesktopFooterButton {
                    action: FileManagerDesktopFooterAction::CancelIconPicker,
                    label: "Cancel",
                },
                FileManagerDesktopFooterButton {
                    action: FileManagerDesktopFooterAction::ChooseIcon,
                    label: "Choose Icon",
                },
                FileManagerDesktopFooterButton {
                    action: FileManagerDesktopFooterAction::GoUp,
                    label: "Up",
                },
                FileManagerDesktopFooterButton {
                    action: FileManagerDesktopFooterAction::OpenHome,
                    label: "Home",
                },
            ],
            file_name: None,
        },
        FileManagerDesktopActionMode::WallpaperPicker => FileManagerDesktopFooterModel {
            status_items: vec![
                format!("{} item(s)", model.status.row_count),
                format!("{} selected", model.status.selected_count),
                model.status.view_label.clone(),
                model.status.tree_label.clone(),
            ],
            leading_buttons: vec![],
            trailing_buttons: vec![
                FileManagerDesktopFooterButton {
                    action: FileManagerDesktopFooterAction::CancelWallpaperPicker,
                    label: "Cancel",
                },
                FileManagerDesktopFooterButton {
                    action: FileManagerDesktopFooterAction::ChooseWallpaper,
                    label: "Choose Wallpaper",
                },
                FileManagerDesktopFooterButton {
                    action: FileManagerDesktopFooterAction::GoUp,
                    label: "Up",
                },
                FileManagerDesktopFooterButton {
                    action: FileManagerDesktopFooterAction::OpenHome,
                    label: "Home",
                },
            ],
            file_name: None,
        },
        FileManagerDesktopActionMode::ThemeImportPicker => FileManagerDesktopFooterModel {
            status_items: vec![
                format!("{} item(s)", model.status.row_count),
                format!("{} selected", model.status.selected_count),
                model.status.view_label.clone(),
                model.status.tree_label.clone(),
            ],
            leading_buttons: vec![],
            trailing_buttons: vec![
                FileManagerDesktopFooterButton {
                    action: FileManagerDesktopFooterAction::CancelThemeImportPicker,
                    label: "Cancel",
                },
                FileManagerDesktopFooterButton {
                    action: FileManagerDesktopFooterAction::ImportTheme,
                    label: "Import Theme",
                },
                FileManagerDesktopFooterButton {
                    action: FileManagerDesktopFooterAction::GoUp,
                    label: "Up",
                },
                FileManagerDesktopFooterButton {
                    action: FileManagerDesktopFooterAction::OpenHome,
                    label: "Home",
                },
            ],
            file_name: None,
        },
        FileManagerDesktopActionMode::Normal => FileManagerDesktopFooterModel {
            status_items: vec![
                format!("{} item(s)", model.status.row_count),
                format!("{} selected", model.status.selected_count),
                model.status.view_label.clone(),
                model.status.tree_label.clone(),
            ],
            leading_buttons: vec![],
            trailing_buttons: vec![
                FileManagerDesktopFooterButton {
                    action: FileManagerDesktopFooterAction::NewFolder,
                    label: "New Folder",
                },
                FileManagerDesktopFooterButton {
                    action: FileManagerDesktopFooterAction::NewDocument,
                    label: "New Document",
                },
                FileManagerDesktopFooterButton {
                    action: FileManagerDesktopFooterAction::OpenSelected,
                    label: "Open",
                },
                FileManagerDesktopFooterButton {
                    action: FileManagerDesktopFooterAction::GoUp,
                    label: "Up",
                },
                FileManagerDesktopFooterButton {
                    action: FileManagerDesktopFooterAction::OpenHome,
                    label: "Home",
                },
            ],
            file_name: None,
        },
    }
}

pub fn resolve_footer_action(
    action: FileManagerDesktopFooterAction,
) -> FileManagerDesktopFooterRequest {
    match action {
        FileManagerDesktopFooterAction::OpenHome => {
            FileManagerDesktopFooterRequest::RunCommand(FileManagerCommand::OpenHome)
        }
        FileManagerDesktopFooterAction::GoUp => {
            FileManagerDesktopFooterRequest::RunCommand(FileManagerCommand::GoUp)
        }
        FileManagerDesktopFooterAction::NewFolder => {
            FileManagerDesktopFooterRequest::RunCommand(FileManagerCommand::NewFolder)
        }
        FileManagerDesktopFooterAction::NewDocument => FileManagerDesktopFooterRequest::NewDocument,
        FileManagerDesktopFooterAction::OpenSelected => {
            FileManagerDesktopFooterRequest::RunCommand(FileManagerCommand::OpenSelected)
        }
        FileManagerDesktopFooterAction::SaveHere => FileManagerDesktopFooterRequest::CompleteSaveAs,
        FileManagerDesktopFooterAction::CancelSavePicker => {
            FileManagerDesktopFooterRequest::CancelSavePicker
        }
        FileManagerDesktopFooterAction::ChooseIcon => {
            FileManagerDesktopFooterRequest::CommitIconPicker
        }
        FileManagerDesktopFooterAction::CancelIconPicker => {
            FileManagerDesktopFooterRequest::CancelIconPicker
        }
        FileManagerDesktopFooterAction::ChooseWallpaper => {
            FileManagerDesktopFooterRequest::CommitWallpaperPicker
        }
        FileManagerDesktopFooterAction::CancelWallpaperPicker => {
            FileManagerDesktopFooterRequest::CancelWallpaperPicker
        }
        FileManagerDesktopFooterAction::ImportTheme => {
            FileManagerDesktopFooterRequest::CommitThemeImportPicker
        }
        FileManagerDesktopFooterAction::CancelThemeImportPicker => {
            FileManagerDesktopFooterRequest::CancelThemeImportPicker
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::DesktopFileManagerSettings;

    #[test]
    fn desktop_action_mode_prioritizes_save_picker() {
        let mode = desktop_action_mode(Some("note.txt".to_string()), Some(3), true, true);
        assert!(matches!(
            mode,
            FileManagerDesktopActionMode::SavePicker { ref file_name } if file_name == "note.txt"
        ));
        assert_eq!(mode.banner(), Some("[ Save As mode ]"));
    }

    #[test]
    fn build_desktop_view_model_uses_drive_path_and_status_labels() {
        let settings = DesktopFileManagerSettings::default();
        let cwd = if cfg!(windows) {
            PathBuf::from("C:\\")
        } else {
            PathBuf::from("/")
        };
        let mut file_manager = NativeFileManagerState::new(cwd.clone());
        file_manager.open_tab_here();
        let model = build_desktop_view_model(
            &file_manager,
            &settings,
            &[],
            0,
            false,
            false,
            false,
            None,
            None,
            false,
            false,
        );

        assert_eq!(model.path_label, cwd.display().to_string());
        assert_eq!(model.search_query, "");
        assert_eq!(model.status.view_label, "Grid View");
        assert_eq!(model.status.tree_label, "Tree On");
        assert!(model.close_tab_enabled());
        assert_eq!(model.grid_columns(301.0, 150.0), 2);
        assert!(!model.tabs.is_empty());
        assert!(!model.drives.is_empty());
        assert!(model.rows.is_empty());
        assert!(!model.tree_items.is_empty());
    }

    #[test]
    fn build_footer_model_for_save_picker_exposes_name_and_actions() {
        let settings = DesktopFileManagerSettings::default();
        let file_manager = NativeFileManagerState::new(PathBuf::from("/"));
        let model = build_desktop_view_model(
            &file_manager,
            &settings,
            &[],
            0,
            false,
            false,
            false,
            Some("note.txt".to_string()),
            None,
            false,
            false,
        );
        let footer = build_footer_model(&model);

        assert_eq!(footer.file_name.as_deref(), Some("note.txt"));
        assert_eq!(footer.status_items, vec!["Save picker".to_string()]);
        assert_eq!(footer.leading_buttons.len(), 3);
        assert_eq!(footer.trailing_buttons.len(), 2);
        assert_eq!(footer.trailing_buttons[0].label, "Cancel");
        assert_eq!(footer.trailing_buttons[1].label, "Save Here");
    }

    #[test]
    fn build_footer_model_for_normal_mode_exposes_status_and_actions() {
        let settings = DesktopFileManagerSettings::default();
        let file_manager = NativeFileManagerState::new(PathBuf::from("/"));
        let model = build_desktop_view_model(
            &file_manager,
            &settings,
            &[],
            2,
            true,
            true,
            true,
            None,
            None,
            false,
            false,
        );
        let footer = build_footer_model(&model);

        assert_eq!(footer.file_name, None);
        assert_eq!(
            footer.status_items,
            vec![
                "0 item(s)".to_string(),
                "2 selected".to_string(),
                "Grid View".to_string(),
                "Tree On".to_string()
            ]
        );
        assert_eq!(footer.trailing_buttons[0].label, "New Folder");
        assert_eq!(footer.trailing_buttons[2].label, "Open");
    }

    #[test]
    fn resolve_footer_action_routes_to_workflow_requests() {
        assert_eq!(
            resolve_footer_action(FileManagerDesktopFooterAction::OpenHome),
            FileManagerDesktopFooterRequest::RunCommand(FileManagerCommand::OpenHome)
        );
        assert_eq!(
            resolve_footer_action(FileManagerDesktopFooterAction::SaveHere),
            FileManagerDesktopFooterRequest::CompleteSaveAs
        );
        assert_eq!(
            resolve_footer_action(FileManagerDesktopFooterAction::ChooseIcon),
            FileManagerDesktopFooterRequest::CommitIconPicker
        );
    }
}
