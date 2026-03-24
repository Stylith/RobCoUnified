use super::super::desktop_app::DesktopWindow;
use super::super::desktop_file_service::FileManagerLocation;
use super::super::desktop_settings_service::apply_file_manager_display_settings_update as apply_desktop_file_manager_display_settings_update;
use super::super::file_manager::FileEntryRow;
use super::super::file_manager_app::FileManagerDisplaySettingsUpdate;
use super::RobcoNativeApp;
use eframe::egui::{Context, TextureHandle};
use robcos_native_settings_app::NativeSettingsPanel;
use std::path::{Path, PathBuf};

impl RobcoNativeApp {
    pub(super) fn file_manager_home_path(&self) -> PathBuf {
        if let Some(session) = &self.session {
            super::super::data::word_processor_dir(&session.username)
        } else {
            super::super::data::home_dir_fallback()
        }
    }

    pub(super) fn apply_file_manager_location(&mut self, location: FileManagerLocation) {
        self.file_manager.set_cwd(location.cwd);
        if let Some(selected) = location.selected {
            self.file_manager.select(Some(selected));
        }
        self.open_desktop_window(DesktopWindow::FileManager);
    }

    pub(super) fn apply_file_manager_display_settings_update(
        &mut self,
        update: FileManagerDisplaySettingsUpdate,
    ) {
        apply_desktop_file_manager_display_settings_update(&mut self.settings.draft, update);
        self.sync_runtime_settings_cache();
        self.file_manager.ensure_selection_valid();
    }

    pub(super) fn settings_panel_texture(
        &mut self,
        ctx: &Context,
        panel: NativeSettingsPanel,
    ) -> Option<TextureHandle> {
        let cache = self.asset_cache.as_mut()?;
        let texture = match panel {
            NativeSettingsPanel::General => cache.icon_general.get_or_insert_with(|| {
                Self::load_svg_icon(
                    ctx,
                    "icon_general",
                    include_bytes!("../../Icons/pixel--home-solid.svg"),
                    Some(64),
                )
            }),
            NativeSettingsPanel::Appearance => cache.icon_appearance.get_or_insert_with(|| {
                Self::load_svg_icon(
                    ctx,
                    "icon_appearance",
                    include_bytes!("../../Icons/pixel--image-solid.svg"),
                    Some(64),
                )
            }),
            NativeSettingsPanel::DefaultApps => cache.icon_default_apps.get_or_insert_with(|| {
                Self::load_svg_icon(
                    ctx,
                    "icon_default_apps",
                    include_bytes!("../../Icons/pixel--external-link-solid.svg"),
                    Some(64),
                )
            }),
            NativeSettingsPanel::Connections => &mut cache.icon_connections,
            NativeSettingsPanel::CliProfiles => cache.icon_cli_profiles.get_or_insert_with(|| {
                Self::load_svg_icon(
                    ctx,
                    "icon_cli_profiles",
                    include_bytes!("../../Icons/pixel--code-solid.svg"),
                    Some(64),
                )
            }),
            NativeSettingsPanel::EditMenus => cache.icon_edit_menus.get_or_insert_with(|| {
                Self::load_svg_icon(
                    ctx,
                    "icon_edit_menus",
                    include_bytes!("../../Icons/pixel--bullet-list-solid.svg"),
                    Some(64),
                )
            }),
            NativeSettingsPanel::UserManagement => {
                cache.icon_user_management.get_or_insert_with(|| {
                    Self::load_svg_icon(
                        ctx,
                        "icon_user_management",
                        include_bytes!("../../Icons/pixel--user-solid.svg"),
                        Some(64),
                    )
                })
            }
            NativeSettingsPanel::About => cache.icon_about.get_or_insert_with(|| {
                Self::load_svg_icon(
                    ctx,
                    "icon_about",
                    include_bytes!("../../Icons/pixel--info-circle-solid.svg"),
                    Some(64),
                )
            }),
            _ => return None,
        };
        Some(texture.clone())
    }

    pub(super) fn installer_games_texture(&mut self, ctx: &Context) -> Option<TextureHandle> {
        let cache = self.asset_cache.as_mut()?;
        Some(
            cache
                .icon_gaming
                .get_or_insert_with(|| {
                    Self::load_svg_icon(
                        ctx,
                        "icon_gaming",
                        include_bytes!("../../Icons/pixel--gaming.svg"),
                        Some(64),
                    )
                })
                .clone(),
        )
    }

    pub(super) fn file_manager_texture_for_row(
        &mut self,
        ctx: &Context,
        row: &FileEntryRow,
    ) -> Option<TextureHandle> {
        let cache = self.asset_cache.as_mut()?;
        if row.is_parent_dir() {
            return Some(Self::ensure_cached_svg_icon(
                &mut cache.icon_folder_open,
                ctx,
                "icon_folder_open",
                include_bytes!("../../Icons/pixel--folder-open-solid.svg"),
                Some(64),
            ));
        }
        if row.is_dir {
            return Some(Self::ensure_cached_svg_icon(
                &mut cache.icon_folder,
                ctx,
                "icon_folder",
                include_bytes!("../../Icons/pixel--folder-solid.svg"),
                Some(64),
            ));
        }
        let extension = row
            .path
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        Some(match extension.as_str() {
            "txt" | "md" | "log" | "toml" | "yaml" | "yml" | "json" | "cfg" | "ini" | "conf"
            | "ron" | "rs" | "py" | "js" | "ts" | "c" | "cpp" | "h" | "hpp" | "sh" | "bash"
            | "fish" | "lua" | "rb" => Self::ensure_cached_svg_icon(
                &mut cache.icon_text,
                ctx,
                "icon_text",
                include_bytes!("../../Icons/pixel--newspaper-solid.svg"),
                Some(64),
            ),
            "png" | "jpg" | "jpeg" | "gif" | "bmp" | "webp" | "svg" | "ico" => {
                Self::ensure_cached_svg_icon(
                    &mut cache.icon_image,
                    ctx,
                    "icon_image",
                    include_bytes!("../../Icons/pixel--image-solid.svg"),
                    Some(64),
                )
            }
            "mp3" | "wav" | "ogg" | "flac" | "aac" | "m4a" => Self::ensure_cached_svg_icon(
                &mut cache.icon_audio,
                ctx,
                "icon_audio",
                include_bytes!("../../Icons/pixel--music-solid.svg"),
                Some(64),
            ),
            "mp4" | "mkv" | "avi" | "mov" | "webm" => Self::ensure_cached_svg_icon(
                &mut cache.icon_video,
                ctx,
                "icon_video",
                include_bytes!("../../Icons/pixel--media.svg"),
                Some(64),
            ),
            "zip" | "tar" | "gz" | "bz2" | "xz" | "7z" | "rar" => Self::ensure_cached_svg_icon(
                &mut cache.icon_archive,
                ctx,
                "icon_archive",
                include_bytes!("../../Icons/pixel--save-solid.svg"),
                Some(64),
            ),
            "exe" | "bin" | "appimage" | "dmg" | "deb" | "rpm" | "app" | "bat" | "cmd" => {
                Self::ensure_cached_svg_icon(
                    &mut cache.icon_app,
                    ctx,
                    "icon_app",
                    include_bytes!("../../Icons/pixel--programming.svg"),
                    Some(64),
                )
            }
            _ => Self::ensure_cached_svg_icon(
                &mut cache.icon_file,
                ctx,
                "icon_file",
                include_bytes!("../../Icons/pixel--clipboard-solid.svg"),
                Some(64),
            ),
        })
    }

    pub(super) fn file_manager_selected_entries(&self) -> Vec<FileEntryRow> {
        self.file_manager.selected_rows_for_action()
    }

    pub(super) fn file_manager_selection_count(&self) -> usize {
        self.file_manager_selected_entries().len()
    }

    pub(super) fn file_manager_select_path(
        &mut self,
        path: PathBuf,
        ctrl_toggle: bool,
        allow_multi: bool,
    ) {
        if allow_multi && ctrl_toggle {
            self.file_manager.toggle_selected_path(&path);
        } else {
            self.file_manager.select(Some(path));
        }
    }

    pub(super) fn svg_preview_texture(
        &mut self,
        ctx: &Context,
        row: &FileEntryRow,
    ) -> Option<TextureHandle> {
        let is_svg = row
            .path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.eq_ignore_ascii_case("svg"))
            .unwrap_or(false);
        if is_svg {
            let key = row.path.to_string_lossy().to_string();
            return self.load_cached_shortcut_icon(ctx, &key, &row.path, 32);
        }
        self.file_manager_texture_for_row(ctx, row)
    }

    pub(super) fn clear_file_manager_preview_texture(&mut self) {
        self.file_manager_preview_texture = None;
        self.file_manager_preview_loaded_for.clear();
    }

    pub(super) fn path_supports_file_manager_image_preview(path: &Path) -> bool {
        matches!(
            path.extension()
                .and_then(|ext| ext.to_str())
                .unwrap_or("")
                .to_ascii_lowercase()
                .as_str(),
            "png" | "jpg" | "jpeg" | "gif" | "bmp" | "webp" | "ico" | "svg"
        )
    }

    pub(super) fn file_manager_preview_texture(
        &mut self,
        ctx: &Context,
        row: &FileEntryRow,
    ) -> Option<TextureHandle> {
        if row.is_dir
            || row.is_parent_dir()
            || !Self::path_supports_file_manager_image_preview(&row.path)
        {
            self.clear_file_manager_preview_texture();
            return None;
        }
        let is_svg = row
            .path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.eq_ignore_ascii_case("svg"))
            .unwrap_or(false);
        if is_svg {
            let key = format!("{}#preview", row.path.to_string_lossy());
            return self.load_cached_shortcut_icon(ctx, &key, &row.path, 192);
        }
        let loaded_for = row.path.to_string_lossy().to_string();
        if self.file_manager_preview_loaded_for != loaded_for {
            self.file_manager_preview_texture = Self::load_tinted_image_texture(
                ctx,
                format!("file_manager_preview::{loaded_for}"),
                &row.path,
                Some(192),
            );
            self.file_manager_preview_loaded_for.clear();
            self.file_manager_preview_loaded_for.push_str(&loaded_for);
        }
        self.file_manager_preview_texture.clone()
    }

    pub(super) fn split_file_name(name: &str) -> (&str, &str) {
        if let Some((stem, _ext)) = name.rsplit_once('.') {
            if !stem.is_empty() {
                return (stem, &name[stem.len()..]);
            }
        }
        (name, "")
    }
}
