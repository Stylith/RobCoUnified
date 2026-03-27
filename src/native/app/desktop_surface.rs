use super::super::data::desktop_surface_dir;
use super::super::desktop_app::DesktopWindow;
use super::super::desktop_search_service::NativeStartLeafAction;
use super::super::desktop_shortcuts_service::{
    create_shortcut_from_start_action, delete_shortcut as delete_desktop_shortcut,
    shortcut_launch_command as desktop_shortcut_launch_command, sort_shortcuts,
    toggle_snap_to_grid as toggle_desktop_snap_to_grid,
    update_shortcut_properties as update_desktop_shortcut_properties, ShortcutPropertiesUpdate,
};
use super::super::desktop_surface_service::{
    build_default_desktop_icon_positions, desktop_builtin_icons, finalize_dragged_icon_position,
    icon_position, load_desktop_surface_entries, update_dragged_icon_position,
    DesktopBuiltinIconKind, DesktopIconDragGrid, DesktopIconGridLayout, DesktopSurfaceEntry,
};
use super::super::file_manager::FileEntryRow;
use super::super::file_manager_app::{FileManagerPromptRequest, NativeFileManagerDragPayload};
use super::super::retro_ui::{current_palette, RetroPalette};
use super::super::shared_file_manager_settings::FileManagerSettingsUpdate;
use super::RobcoNativeApp;
use super::{
    AssetCache, ContextMenuAction, DesktopIconLayoutCache, DesktopIconSelection,
    DesktopSurfaceEntriesCache, ShortcutPropertiesState, StartMenuRenameState,
};
use crate::config::{DesktopIconSortMode, DesktopIconStyle, WallpaperSizeMode};
use eframe::egui::{self, Align2, Color32, Context, FontFamily, FontId, RichText, TextureHandle};
use robcos_native_settings_app::NativeSettingsPanel;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

impl RobcoNativeApp {
    pub(super) fn invalidate_desktop_icon_layout_cache(&mut self) {
        self.desktop_icon_layout_cache = None;
    }

    pub(super) fn desktop_surface_entries(&mut self) -> Arc<Vec<DesktopSurfaceEntry>> {
        let dir = desktop_surface_dir();
        let modified = std::fs::metadata(&dir)
            .and_then(|meta| meta.modified())
            .ok();
        let needs_reload = self
            .desktop_surface_entries_cache
            .as_ref()
            .is_none_or(|cache| cache.dir != dir || cache.modified != modified);
        if needs_reload {
            let entries = Arc::new(load_desktop_surface_entries(&dir));
            self.desktop_surface_entries_cache = Some(DesktopSurfaceEntriesCache {
                dir,
                modified,
                entries,
            });
            self.invalidate_desktop_icon_layout_cache();
        }
        self.desktop_surface_entries_cache
            .as_ref()
            .expect("desktop surface cache initialized")
            .entries
            .clone()
    }

    pub(super) fn default_desktop_icon_positions(
        &mut self,
        layout: DesktopIconGridLayout,
        desktop_entries: &[DesktopSurfaceEntry],
    ) -> Arc<HashMap<String, [f32; 2]>> {
        let desktop_entry_keys = Arc::new(
            desktop_entries
                .iter()
                .map(|entry| entry.key.clone())
                .collect::<Vec<_>>(),
        );
        let needs_rebuild = self.desktop_icon_layout_cache.as_ref().is_none_or(|cache| {
            cache.layout != layout
                || cache.desktop_entry_keys.as_ref() != desktop_entry_keys.as_ref()
        });
        if needs_rebuild {
            let positions = Arc::new(build_default_desktop_icon_positions(
                layout,
                self.settings.draft.desktop_icon_sort,
                &self.settings.draft.desktop_hidden_builtin_icons,
                desktop_entries,
                &self.settings.draft.desktop_shortcuts,
            ));
            self.desktop_icon_layout_cache = Some(DesktopIconLayoutCache {
                layout,
                desktop_entry_keys,
                positions,
            });
        }
        self.desktop_icon_layout_cache
            .as_ref()
            .expect("desktop icon layout cache initialized")
            .positions
            .clone()
    }

    pub(super) fn load_tinted_image_texture(
        ctx: &Context,
        texture_id: impl Into<String>,
        path: &Path,
        max_side_px: Option<u32>,
    ) -> Option<TextureHandle> {
        let bytes = std::fs::read(path).ok()?;
        let mut image = image::load_from_memory(&bytes).ok()?.into_rgba8();
        if let Some(max_side_px) = max_side_px {
            let longest_side = image.width().max(image.height());
            if longest_side > max_side_px {
                image = image::imageops::thumbnail(&image, max_side_px, max_side_px);
            }
        }
        let (width, height) = image.dimensions();
        let mut rgba = Vec::with_capacity((width * height * 4) as usize);
        for pixel in image.pixels() {
            let luma =
                ((pixel[0] as u16 * 77 + pixel[1] as u16 * 150 + pixel[2] as u16 * 29) / 256) as u8;
            rgba.extend_from_slice(&[luma, luma, luma, pixel[3]]);
        }
        let color_image =
            egui::ColorImage::from_rgba_unmultiplied([width as usize, height as usize], &rgba);
        Some(ctx.load_texture(texture_id.into(), color_image, egui::TextureOptions::LINEAR))
    }

    pub(super) fn load_wallpaper_texture(ctx: &Context, path: &str) -> Option<TextureHandle> {
        if path.trim().is_empty() {
            return None;
        }
        Self::load_tinted_image_texture(ctx, "desktop_wallpaper", Path::new(path), None)
    }

    pub(super) fn build_asset_cache(ctx: &Context) -> AssetCache {
        const ICON_SIZE: u32 = 64;

        AssetCache {
            icon_settings: Self::load_svg_icon(
                ctx,
                "icon_settings",
                include_bytes!("../../Icons/pixel--cog-solid.svg"),
                Some(ICON_SIZE),
            ),
            icon_file_manager: Self::load_svg_icon(
                ctx,
                "icon_file_manager",
                include_bytes!("../../Icons/pixel--folder-solid.svg"),
                Some(ICON_SIZE),
            ),
            icon_terminal: Self::load_svg_icon(
                ctx,
                "icon_terminal",
                include_bytes!("../../Icons/pixel--code-block-solid.svg"),
                Some(ICON_SIZE),
            ),
            icon_applications: Self::load_svg_icon(
                ctx,
                "icon_applications",
                include_bytes!("../../Icons/pixel--grid.svg"),
                Some(ICON_SIZE),
            ),
            icon_installer: Self::load_svg_icon(
                ctx,
                "icon_installer",
                include_bytes!("../../Icons/pixel--file-import-solid.svg"),
                Some(ICON_SIZE),
            ),
            icon_editor: Self::load_svg_icon(
                ctx,
                "icon_editor",
                include_bytes!("../../Icons/pixel--pen-solid.svg"),
                Some(ICON_SIZE),
            ),
            icon_general: None,
            icon_appearance: None,
            icon_default_apps: None,
            icon_connections: Self::load_svg_icon(
                ctx,
                "icon_connections",
                include_bytes!("../../Icons/pixel--globe.svg"),
                Some(ICON_SIZE),
            ),
            icon_cli_profiles: None,
            icon_edit_menus: None,
            icon_user_management: None,
            icon_about: None,
            icon_folder: None,
            icon_folder_open: None,
            icon_file: None,
            icon_text: None,
            icon_image: None,
            icon_audio: None,
            icon_video: None,
            icon_archive: None,
            icon_app: None,
            icon_shortcut_badge: None,
            icon_gaming: None,
            wallpaper: None,
            wallpaper_loaded_for: String::new(),
        }
    }

    pub(super) fn sync_wallpaper(&mut self, ctx: &Context) {
        let wallpaper_path = self.settings.draft.desktop_wallpaper.as_str();
        if let Some(cache) = &mut self.asset_cache {
            if cache.wallpaper_loaded_for != wallpaper_path {
                cache.wallpaper = Self::load_wallpaper_texture(ctx, wallpaper_path);
                cache.wallpaper_loaded_for.clear();
                cache.wallpaper_loaded_for.push_str(wallpaper_path);
            }
        }
    }

    pub(super) fn draw_wallpaper(
        &self,
        painter: &egui::Painter,
        screen: egui::Rect,
        palette: &RetroPalette,
    ) -> bool {
        let Some(cache) = &self.asset_cache else {
            return false;
        };
        let Some(texture) = &cache.wallpaper else {
            return false;
        };

        let image_size = egui::vec2(texture.size()[0] as f32, texture.size()[1] as f32);
        let uv = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
        let tint = palette.fg;
        match self.settings.draft.desktop_wallpaper_size_mode {
            WallpaperSizeMode::FitToScreen | WallpaperSizeMode::Stretch => {
                painter.image(texture.id(), screen, uv, tint);
            }
            WallpaperSizeMode::Centered => {
                painter.rect_filled(screen, 0.0, palette.bg);
                let origin = screen.center() - image_size * 0.5;
                painter.image(
                    texture.id(),
                    egui::Rect::from_min_size(origin, image_size),
                    uv,
                    tint,
                );
            }
            WallpaperSizeMode::DefaultSize => {
                painter.rect_filled(screen, 0.0, palette.bg);
                painter.image(
                    texture.id(),
                    egui::Rect::from_min_size(screen.min, image_size),
                    uv,
                    tint,
                );
            }
            WallpaperSizeMode::Tile => {
                painter.rect_filled(screen, 0.0, palette.bg);
                let mut y = screen.top();
                while y < screen.bottom() {
                    let mut x = screen.left();
                    while x < screen.right() {
                        painter.image(
                            texture.id(),
                            egui::Rect::from_min_size(egui::pos2(x, y), image_size),
                            uv,
                            tint,
                        );
                        x += image_size.x.max(1.0);
                    }
                    y += image_size.y.max(1.0);
                }
            }
        }
        true
    }

    pub(super) fn load_cached_shortcut_icon(
        &mut self,
        ctx: &Context,
        cache_key: &str,
        path: &Path,
        size_px: u32,
    ) -> Option<TextureHandle> {
        if let Some(tex) = self.shortcut_icon_cache.get(cache_key) {
            return Some(tex.clone());
        }
        if self.shortcut_icon_missing.contains(cache_key) {
            return None;
        }
        let bytes = match std::fs::read(path) {
            Ok(bytes) => bytes,
            Err(_) => {
                self.shortcut_icon_missing.insert(cache_key.to_string());
                return None;
            }
        };
        let tex = Self::load_svg_icon(ctx, cache_key, &bytes, Some(size_px));
        self.shortcut_icon_cache
            .insert(cache_key.to_string(), tex.clone());
        Some(tex)
    }

    pub(super) fn import_paths_to_desktop(&mut self, paths: Vec<PathBuf>) {
        let desktop_dir = desktop_surface_dir();
        self.shell_status = match self
            .file_manager_runtime
            .copy_paths_into_dir(paths, &desktop_dir)
        {
            Ok((count, last_dst)) => {
                self.invalidate_desktop_surface_cache();
                if count == 1 {
                    format!(
                        "Imported {} to the desktop.",
                        last_dst
                            .as_ref()
                            .and_then(|path| path.file_name())
                            .and_then(|name| name.to_str())
                            .unwrap_or("item")
                    )
                } else {
                    format!("Imported {count} items to the desktop.")
                }
            }
            Err(err) => format!("Desktop import failed: {err}"),
        };
    }

    pub(super) fn dispatch_context_menu_action(&mut self, _ctx: &Context) {
        let Some(action) = self.context_menu_action.take() else {
            return;
        };
        match action {
            ContextMenuAction::Open => self.run_file_manager_command(
                super::super::file_manager::FileManagerCommand::OpenSelected,
            ),
            ContextMenuAction::OpenWith => {
                if let Some(entry) = self.file_manager_selected_file() {
                    let ext_key =
                        super::super::file_manager_app::open_with_extension_key(&entry.path);
                    self.open_file_manager_prompt(FileManagerPromptRequest::open_with_new_command(
                        entry.path, ext_key, false,
                    ));
                } else {
                    self.shell_status = "Open With requires a file.".to_string();
                }
            }
            ContextMenuAction::OpenWithCommand(command) => {
                if let Some(entry) = self.file_manager_selected_file() {
                    let ext_key =
                        super::super::file_manager_app::open_with_extension_key(&entry.path);
                    match super::super::file_manager_app::prepare_open_with_launch(
                        &entry.path,
                        &command,
                    ) {
                        Ok(launch) => {
                            self.shell_status = self.launch_open_with_request(launch);
                            self.apply_file_manager_settings_update(
                                FileManagerSettingsUpdate::RecordOpenWithCommand {
                                    ext_key,
                                    command,
                                },
                            );
                        }
                        Err(err) => {
                            self.shell_status = format!("Open failed: {err}");
                        }
                    }
                } else {
                    self.shell_status = "Open With requires a file.".to_string();
                }
            }
            ContextMenuAction::Rename => self
                .run_file_manager_command(super::super::file_manager::FileManagerCommand::Rename),
            ContextMenuAction::Cut => {
                self.run_file_manager_command(super::super::file_manager::FileManagerCommand::Cut)
            }
            ContextMenuAction::Copy => {
                self.run_file_manager_command(super::super::file_manager::FileManagerCommand::Copy)
            }
            ContextMenuAction::Paste => {
                self.run_file_manager_command(super::super::file_manager::FileManagerCommand::Paste)
            }
            ContextMenuAction::Duplicate => self.run_file_manager_command(
                super::super::file_manager::FileManagerCommand::Duplicate,
            ),
            ContextMenuAction::Delete => self
                .run_file_manager_command(super::super::file_manager::FileManagerCommand::Delete),
            ContextMenuAction::Properties => {
                self.shell_status = "Properties dialog is not implemented yet.".to_string();
            }
            ContextMenuAction::PasteToDesktop => {
                self.paste_to_desktop();
            }
            ContextMenuAction::NewFolder => {
                self.create_desktop_folder();
            }
            ContextMenuAction::ChangeAppearance => {
                self.launch_settings_panel_via_registry(NativeSettingsPanel::Appearance);
            }
            ContextMenuAction::OpenSettings => {
                self.launch_settings_via_registry();
            }
            ContextMenuAction::GenericCopy => {}
            ContextMenuAction::GenericPaste => {}
            ContextMenuAction::GenericSelectAll => {}
            ContextMenuAction::CreateShortcut { label, action } => {
                create_shortcut_from_start_action(&mut self.settings.draft, label, &action);
                self.persist_native_settings();
            }
            ContextMenuAction::RenameStartMenuEntry { target, name } => {
                self.start_menu_rename = Some(StartMenuRenameState {
                    target,
                    original_name: name.clone(),
                    name_input: name,
                });
            }
            ContextMenuAction::RemoveStartMenuEntry { target, name } => {
                self.delete_program_entry(target, &name);
                self.close_start_menu();
            }
            ContextMenuAction::DeleteShortcut(idx) => {
                if delete_desktop_shortcut(&mut self.settings.draft, idx) {
                    self.persist_native_settings();
                }
            }
            ContextMenuAction::SortDesktopIcons(mode) => {
                sort_shortcuts(&mut self.settings.draft, mode);
                self.persist_native_settings();
            }
            ContextMenuAction::ToggleSnapToGrid => {
                toggle_desktop_snap_to_grid(&mut self.settings.draft);
                self.persist_native_settings();
            }
            ContextMenuAction::LaunchShortcut(name) => {
                let custom_cmd = desktop_shortcut_launch_command(&self.settings.draft, &name);
                if let Some(cmd) = custom_cmd {
                    let args: Vec<String> = cmd.split_whitespace().map(|s| s.to_string()).collect();
                    self.launch_shell_command_in_desktop_surface(&name, &args);
                } else {
                    self.run_start_leaf_action(NativeStartLeafAction::LaunchConfiguredApp(name));
                }
            }
            ContextMenuAction::OpenShortcutProperties(idx) => {
                if let Some(sc) = self.settings.draft.desktop_shortcuts.get(idx) {
                    self.shortcut_properties = Some(ShortcutPropertiesState {
                        shortcut_idx: idx,
                        name_draft: sc.label.clone(),
                        command_draft: sc
                            .launch_command
                            .clone()
                            .unwrap_or_else(|| sc.app_name.clone()),
                        icon_path_draft: sc.icon_path.clone(),
                    });
                }
            }
            ContextMenuAction::OpenDesktopItem(path) => {
                self.open_desktop_surface_path(path);
            }
            ContextMenuAction::OpenDesktopItemWith(path) => {
                self.open_desktop_surface_with_prompt(path);
            }
            ContextMenuAction::RenameDesktopItem(path) => {
                self.rename_desktop_item(path);
            }
            ContextMenuAction::DeleteDesktopItem(path) => {
                self.delete_desktop_item(path);
            }
            ContextMenuAction::OpenDesktopItemProperties(path) => {
                self.open_desktop_item_properties(path);
            }
        }
    }

    pub(super) fn attach_desktop_empty_context_menu(
        action: &mut Option<ContextMenuAction>,
        response: &egui::Response,
        snap_to_grid: bool,
        sort_mode: DesktopIconSortMode,
    ) {
        response.context_menu(|ui| {
            Self::apply_context_menu_style(ui);
            ui.set_min_width(136.0);
            ui.set_max_width(180.0);

            ui.menu_button("View", |ui| {
                Self::apply_context_menu_style(ui);
                ui.set_min_width(140.0);
                ui.set_max_width(180.0);
                let name_label = if sort_mode == DesktopIconSortMode::ByName {
                    "✓ Sort by Name"
                } else {
                    "  Sort by Name"
                };
                let type_label = if sort_mode == DesktopIconSortMode::ByType {
                    "✓ Sort by Type"
                } else {
                    "  Sort by Type"
                };
                if ui.button(name_label).clicked() {
                    *action = Some(ContextMenuAction::SortDesktopIcons(
                        DesktopIconSortMode::ByName,
                    ));
                    ui.close_menu();
                }
                if ui.button(type_label).clicked() {
                    *action = Some(ContextMenuAction::SortDesktopIcons(
                        DesktopIconSortMode::ByType,
                    ));
                    ui.close_menu();
                }
                Self::retro_separator(ui);
                let snap_label = if snap_to_grid {
                    "✓ Snap to Grid"
                } else {
                    "  Snap to Grid"
                };
                if ui.button(snap_label).clicked() {
                    *action = Some(ContextMenuAction::ToggleSnapToGrid);
                    ui.close_menu();
                }
            });

            Self::retro_separator(ui);

            if ui.button("Paste").clicked() {
                *action = Some(ContextMenuAction::PasteToDesktop);
                ui.close_menu();
            }

            Self::retro_separator(ui);

            if ui.button("New Folder").clicked() {
                *action = Some(ContextMenuAction::NewFolder);
                ui.close_menu();
            }

            Self::retro_separator(ui);

            if ui.button("Change Appearance...").clicked() {
                *action = Some(ContextMenuAction::ChangeAppearance);
                ui.close_menu();
            }
            if ui.button("Settings...").clicked() {
                *action = Some(ContextMenuAction::OpenSettings);
                ui.close_menu();
            }
        });
    }

    pub(super) fn desktop_icon_label_lines(label: &str) -> Vec<String> {
        const MAX_LINE_CHARS: usize = 14;

        if label.chars().count() <= MAX_LINE_CHARS {
            return vec![label.to_string()];
        }
        let words: Vec<&str> = label.split_whitespace().collect();
        if words.len() < 2 {
            return vec![Self::truncate_file_manager_label(label, MAX_LINE_CHARS)];
        }

        let mut first_line = String::new();
        let mut split_idx = 0usize;
        for (idx, word) in words.iter().enumerate() {
            let candidate = if first_line.is_empty() {
                (*word).to_string()
            } else {
                format!("{first_line} {word}")
            };
            if candidate.chars().count() > MAX_LINE_CHARS {
                break;
            }
            first_line = candidate;
            split_idx = idx + 1;
        }
        if first_line.is_empty() {
            return vec![Self::truncate_file_manager_label(label, MAX_LINE_CHARS)];
        }
        if split_idx >= words.len() {
            return vec![first_line];
        }

        let second_line = words[split_idx..].join(" ");
        vec![
            first_line,
            Self::truncate_file_manager_label(&second_line, MAX_LINE_CHARS),
        ]
    }

    pub(super) fn paint_desktop_icon_label(
        ui: &mut egui::Ui,
        rect: egui::Rect,
        label: &str,
        color: Color32,
    ) {
        let lines = Self::desktop_icon_label_lines(label);
        if lines.len() == 1 {
            ui.painter().text(
                rect.center(),
                Align2::CENTER_CENTER,
                &lines[0],
                FontId::new(13.0, FontFamily::Monospace),
                color,
            );
            return;
        }

        let line_height = 11.0;
        let total_height = line_height * lines.len() as f32;
        let start_y = rect.center().y - total_height * 0.5 + line_height * 0.5;
        for (idx, line) in lines.iter().enumerate() {
            ui.painter().text(
                egui::pos2(rect.center().x, start_y + idx as f32 * line_height),
                Align2::CENTER_CENTER,
                line,
                FontId::new(11.0, FontFamily::Monospace),
                color,
            );
        }
    }

    pub(super) fn paint_desktop_icon_selection(
        ui: &mut egui::Ui,
        rect: egui::Rect,
        palette: RetroPalette,
        selected: bool,
        hovered: bool,
    ) {
        if !(selected || hovered) {
            return;
        }
        let fill = if selected {
            palette.selected_bg
        } else {
            palette.panel
        };
        let stroke = if selected { palette.fg } else { palette.dim };
        ui.painter().rect_filled(rect.expand(2.0), 0.0, fill);
        ui.painter()
            .rect_stroke(rect.expand(2.0), 0.0, egui::Stroke::new(1.0, stroke));
    }

    pub(super) fn desktop_icon_foreground(palette: RetroPalette, selected: bool) -> Color32 {
        if selected {
            palette.bg
        } else {
            palette.fg
        }
    }

    pub(super) fn apply_context_menu_style(ui: &mut egui::Ui) {
        let palette = current_palette();
        let mut style = ui.style().as_ref().clone();
        let stroke = egui::Stroke::new(2.0, palette.fg);
        style.visuals.button_frame = true;
        style.visuals.window_fill = palette.panel;
        style.visuals.window_stroke = stroke;
        style.visuals.window_rounding = egui::Rounding::ZERO;
        style.visuals.menu_rounding = egui::Rounding::ZERO;
        style.visuals.window_shadow = egui::epaint::Shadow::NONE;
        style.visuals.popup_shadow = egui::epaint::Shadow::NONE;
        style.visuals.override_text_color = None;
        style.spacing.item_spacing = egui::vec2(0.0, 0.0);
        style.spacing.button_padding = egui::vec2(5.0, 2.0);
        style.spacing.menu_margin = egui::Margin::same(2.0);
        style.spacing.interact_size.y = 18.0;
        style.visuals.widgets.noninteractive.bg_fill = Color32::TRANSPARENT;
        style.visuals.widgets.noninteractive.weak_bg_fill = Color32::TRANSPARENT;
        style.visuals.widgets.noninteractive.bg_stroke = egui::Stroke::NONE;
        style.visuals.widgets.noninteractive.fg_stroke.color = palette.fg;
        style.visuals.widgets.noninteractive.rounding = egui::Rounding::ZERO;
        style.visuals.widgets.noninteractive.expansion = 0.0;
        style.visuals.widgets.inactive.bg_fill = Color32::TRANSPARENT;
        style.visuals.widgets.inactive.weak_bg_fill = Color32::TRANSPARENT;
        style.visuals.widgets.inactive.bg_stroke = egui::Stroke::NONE;
        style.visuals.widgets.inactive.fg_stroke.color = palette.fg;
        style.visuals.widgets.inactive.rounding = egui::Rounding::ZERO;
        style.visuals.widgets.inactive.expansion = 0.0;
        for visuals in [
            &mut style.visuals.widgets.hovered,
            &mut style.visuals.widgets.active,
            &mut style.visuals.widgets.open,
        ] {
            visuals.bg_fill = palette.fg;
            visuals.weak_bg_fill = palette.fg;
            visuals.bg_stroke = egui::Stroke::NONE;
            visuals.fg_stroke.color = Color32::BLACK;
            visuals.rounding = egui::Rounding::ZERO;
            visuals.expansion = 0.0;
        }
        ui.set_style(style);
    }

    pub(super) fn draw_desktop_icons(&mut self, ui: &mut egui::Ui) {
        let (
            tex_file_manager,
            tex_editor,
            tex_installer,
            tex_settings,
            tex_terminal,
            tex_connections,
        ) = {
            let Some(cache) = self.asset_cache.as_ref() else {
                return;
            };
            (
                cache.icon_file_manager.clone(),
                cache.icon_editor.clone(),
                cache.icon_installer.clone(),
                cache.icon_settings.clone(),
                cache.icon_terminal.clone(),
                cache.icon_connections.clone(),
            )
        };
        let tex_shortcut_badge = Self::ensure_cached_svg_icon(
            &mut self
                .asset_cache
                .as_mut()
                .expect("desktop asset cache")
                .icon_shortcut_badge,
            ui.ctx(),
            "icon_shortcut_badge",
            include_bytes!("../../Icons/pixel--external-link-solid.svg"),
            Some(16),
        );
        let tex_app = Self::ensure_cached_svg_icon(
            &mut self
                .asset_cache
                .as_mut()
                .expect("desktop asset cache")
                .icon_app,
            ui.ctx(),
            "icon_app",
            include_bytes!("../../Icons/pixel--programming.svg"),
            Some(64),
        );

        let palette = current_palette();
        let style = self.settings.draft.desktop_icon_style;
        let snap = self.settings.draft.desktop_snap_to_grid;
        let workspace = self.active_desktop_workspace_rect(ui.ctx());
        let (icon_size, label_height, item_height, column_width): (f32, f32, f32, f32) = match style
        {
            DesktopIconStyle::Minimal => (34.0, 0.0, 46.0, 48.0),
            DesktopIconStyle::Win95 | DesktopIconStyle::Dos => (48.0, 28.0, 84.0, 100.0),
            DesktopIconStyle::NoIcons => return,
        };

        let drag_grid = DesktopIconDragGrid {
            cell_w: column_width,
            cell_h: item_height,
            snap_to_grid: snap,
        };

        let hidden_icons = self.settings.draft.desktop_hidden_builtin_icons.clone();
        let desktop_entries = self.desktop_surface_entries();
        let shortcuts = self.settings.draft.desktop_shortcuts.clone();
        let builtin_entries = desktop_builtin_icons();
        let default_positions = self.default_desktop_icon_positions(
            DesktopIconGridLayout {
                left: workspace.left(),
                top: workspace.top(),
                height: workspace.height(),
                item_height,
                column_width,
            },
            &desktop_entries,
        );
        let mut open_window: Option<DesktopWindow> = None;
        let mut open_terminal = false;
        let mut open_desktop_path: Option<PathBuf> = None;
        let mut desktop_action: Option<ContextMenuAction> = None;
        let mut needs_persist = false;

        for (index, entry) in builtin_entries.iter().enumerate() {
            if hidden_icons.contains(entry.key) {
                continue;
            }
            let texture = match entry.kind {
                DesktopBuiltinIconKind::FileManager => &tex_file_manager,
                DesktopBuiltinIconKind::Editor => &tex_editor,
                DesktopBuiltinIconKind::Installer => &tex_installer,
                DesktopBuiltinIconKind::Settings => &tex_settings,
                DesktopBuiltinIconKind::Terminal => &tex_terminal,
            };
            let top_left = {
                let [x, y] = icon_position(
                    &self.settings.draft,
                    entry.key,
                    [
                        workspace.left() + 4.0,
                        workspace.top() + 16.0 + index as f32 * item_height,
                    ],
                    &default_positions,
                );
                egui::pos2(x, y)
            };

            let icon_rect = egui::Rect::from_min_size(
                top_left + egui::vec2((column_width - icon_size) * 0.5, 0.0),
                egui::vec2(icon_size, icon_size),
            );
            let label_rect = egui::Rect::from_min_size(
                top_left + egui::vec2(0.0, icon_size + 2.0),
                egui::vec2(column_width, label_height.max(16.0)),
            );
            let hit_rect = if label_height > 0.0 {
                egui::Rect::from_min_size(
                    top_left,
                    egui::vec2(column_width, icon_size + label_height + 2.0),
                )
            } else {
                icon_rect
            };

            let response = ui.allocate_rect(hit_rect, egui::Sense::click_and_drag());
            let selected =
                self.desktop_selected_icon == Some(DesktopIconSelection::Builtin(entry.key));
            Self::paint_desktop_icon_selection(ui, hit_rect, palette, selected, response.hovered());
            let icon_fg = Self::desktop_icon_foreground(palette, selected);

            match style {
                DesktopIconStyle::Dos => {
                    ui.painter().text(
                        icon_rect.center(),
                        Align2::CENTER_CENTER,
                        entry.ascii,
                        FontId::new(18.0, FontFamily::Monospace),
                        icon_fg,
                    );
                }
                DesktopIconStyle::Minimal | DesktopIconStyle::Win95 => {
                    Self::paint_tinted_texture(ui.painter(), texture, icon_rect, icon_fg);
                }
                DesktopIconStyle::NoIcons => {}
            }

            if label_height > 0.0 {
                Self::paint_desktop_icon_label(ui, label_rect, entry.label, icon_fg);
            }

            if response.dragged() {
                update_dragged_icon_position(
                    &mut self.settings.draft,
                    entry.key,
                    [top_left.x, top_left.y],
                    [response.drag_delta().x, response.drag_delta().y],
                );
            }
            if response.drag_stopped() {
                needs_persist |=
                    finalize_dragged_icon_position(&mut self.settings.draft, entry.key, drag_grid);
            }

            if response.clicked() || response.secondary_clicked() {
                self.desktop_selected_icon = Some(DesktopIconSelection::Builtin(entry.key));
            }
            if response.double_clicked() {
                if let Some(window) = entry.target_window {
                    open_window = Some(window);
                } else {
                    open_terminal = true;
                }
            }
        }

        for (entry_idx, entry) in desktop_entries.iter().enumerate() {
            let entry_key = entry.key.clone();
            let entry_path = entry.path.clone();
            let entry_label = entry.label.clone();
            let entry_is_dir = entry.is_dir();
            let row = Self::desktop_entry_row(entry);
            let top_left = {
                let [x, y] = icon_position(
                    &self.settings.draft,
                    &entry_key,
                    [
                        workspace.left() + 4.0 + column_width,
                        workspace.top()
                            + 16.0
                            + (builtin_entries.len() + entry_idx) as f32 * item_height,
                    ],
                    &default_positions,
                );
                egui::pos2(x, y)
            };

            let icon_rect = egui::Rect::from_min_size(
                top_left + egui::vec2((column_width - icon_size) * 0.5, 0.0),
                egui::vec2(icon_size, icon_size),
            );
            let label_rect = egui::Rect::from_min_size(
                top_left + egui::vec2(0.0, icon_size + 2.0),
                egui::vec2(column_width, label_height.max(16.0)),
            );
            let hit_rect = if label_height > 0.0 {
                egui::Rect::from_min_size(
                    top_left,
                    egui::vec2(column_width, icon_size + label_height + 2.0),
                )
            } else {
                icon_rect
            };

            let response = ui.allocate_rect(hit_rect, egui::Sense::click_and_drag());
            let selected = self.desktop_selected_icon
                == Some(DesktopIconSelection::Surface(entry_key.clone()));
            Self::paint_desktop_icon_selection(ui, hit_rect, palette, selected, response.hovered());
            let icon_fg = Self::desktop_icon_foreground(palette, selected);
            response.dnd_set_drag_payload(NativeFileManagerDragPayload {
                paths: vec![entry_path.clone()],
            });
            let file_manager_drop_hover = entry_is_dir
                && response
                    .dnd_hover_payload::<NativeFileManagerDragPayload>()
                    .is_some_and(|payload| {
                        Self::file_manager_drop_allowed(&payload.paths, &entry_path)
                    });

            match style {
                DesktopIconStyle::Dos => {
                    ui.painter().text(
                        icon_rect.center(),
                        Align2::CENTER_CENTER,
                        row.icon(),
                        FontId::new(18.0, FontFamily::Monospace),
                        icon_fg,
                    );
                }
                DesktopIconStyle::Minimal | DesktopIconStyle::Win95 => {
                    if let Some(texture) = self.file_manager_texture_for_row(ui.ctx(), &row) {
                        Self::paint_tinted_texture(ui.painter(), &texture, icon_rect, icon_fg);
                    }
                }
                DesktopIconStyle::NoIcons => {}
            }

            if file_manager_drop_hover {
                ui.painter().rect_stroke(
                    hit_rect.expand(2.0),
                    0.0,
                    egui::Stroke::new(1.5, palette.fg),
                );
            }

            if label_height > 0.0 {
                Self::paint_desktop_icon_label(ui, label_rect, &entry_label, icon_fg);
            }

            if response.dragged() {
                update_dragged_icon_position(
                    &mut self.settings.draft,
                    &entry_key,
                    [top_left.x, top_left.y],
                    [response.drag_delta().x, response.drag_delta().y],
                );
            }
            if response.drag_stopped() {
                needs_persist |=
                    finalize_dragged_icon_position(&mut self.settings.draft, &entry_key, drag_grid);
            }

            if response.clicked() || response.secondary_clicked() {
                self.desktop_selected_icon = Some(DesktopIconSelection::Surface(entry_key.clone()));
            }

            response.context_menu(|ui| {
                Self::apply_context_menu_style(ui);
                ui.set_min_width(140.0);
                ui.set_max_width(190.0);
                if ui.button("Open").clicked() {
                    desktop_action = Some(ContextMenuAction::OpenDesktopItem(entry_path.clone()));
                    ui.close_menu();
                }
                if !entry_is_dir {
                    if ui.button("Open With...").clicked() {
                        desktop_action =
                            Some(ContextMenuAction::OpenDesktopItemWith(entry_path.clone()));
                        ui.close_menu();
                    }
                }
                Self::retro_separator(ui);
                if ui.button("Rename").clicked() {
                    desktop_action = Some(ContextMenuAction::RenameDesktopItem(entry_path.clone()));
                    ui.close_menu();
                }
                if ui.button("Properties").clicked() {
                    desktop_action = Some(ContextMenuAction::OpenDesktopItemProperties(
                        entry_path.clone(),
                    ));
                    ui.close_menu();
                }
                Self::retro_separator(ui);
                if ui.button("Delete").clicked() {
                    desktop_action = Some(ContextMenuAction::DeleteDesktopItem(entry_path.clone()));
                    ui.close_menu();
                }
            });

            if entry_is_dir {
                if let Some(payload) =
                    response.dnd_release_payload::<NativeFileManagerDragPayload>()
                {
                    if Self::file_manager_drop_allowed(&payload.paths, &entry_path) {
                        self.file_manager_handle_drop_to_dir(
                            payload.paths.clone(),
                            entry_path.clone(),
                        );
                    }
                }
            }

            if response.double_clicked() {
                open_desktop_path = Some(entry_path.clone());
            }
        }

        for (sidx, shortcut) in shortcuts.iter().enumerate() {
            let key = format!("shortcut_{}", sidx);
            let top_left = {
                let [x, y] = icon_position(
                    &self.settings.draft,
                    &key,
                    [
                        workspace.left() + 4.0 + column_width * 2.0,
                        workspace.top() + 16.0 + sidx as f32 * item_height,
                    ],
                    &default_positions,
                );
                egui::pos2(x, y)
            };

            let icon_rect = egui::Rect::from_min_size(
                top_left + egui::vec2((column_width - icon_size) * 0.5, 0.0),
                egui::vec2(icon_size, icon_size),
            );
            let label_rect = egui::Rect::from_min_size(
                top_left + egui::vec2(0.0, icon_size + 2.0),
                egui::vec2(column_width, label_height.max(16.0)),
            );
            let hit_rect = if label_height > 0.0 {
                egui::Rect::from_min_size(
                    top_left,
                    egui::vec2(column_width, icon_size + label_height + 2.0),
                )
            } else {
                icon_rect
            };

            let response = ui.allocate_rect(hit_rect, egui::Sense::click_and_drag());
            let selected = self.desktop_selected_icon == Some(DesktopIconSelection::Shortcut(sidx));
            Self::paint_desktop_icon_selection(ui, hit_rect, palette, selected, response.hovered());
            let icon_fg = Self::desktop_icon_foreground(palette, selected);

            match style {
                DesktopIconStyle::Dos => {
                    ui.painter().text(
                        icon_rect.center(),
                        Align2::CENTER_CENTER,
                        "[LNK]",
                        FontId::new(18.0, FontFamily::Monospace),
                        icon_fg,
                    );
                }
                DesktopIconStyle::Minimal | DesktopIconStyle::Win95 => {
                    // Try to use a custom icon texture if icon_path is set
                    let icon_path_clone = shortcut.icon_path.clone();
                    let icon_tex: Option<egui::TextureHandle> =
                        if let Some(ref path) = icon_path_clone {
                            self.load_cached_shortcut_icon(ui.ctx(), path, Path::new(path), 48)
                        } else {
                            None
                        };
                    if let Some(tex) = icon_tex {
                        Self::paint_tinted_texture(ui.painter(), &tex, icon_rect, icon_fg);
                    } else {
                        let kind_tex = match shortcut.shortcut_kind.as_str() {
                            "network" => &tex_connections,
                            "editor" => &tex_editor,
                            _ => &tex_app,
                        };
                        Self::paint_tinted_texture(ui.painter(), kind_tex, icon_rect, icon_fg);
                    }
                    let badge_size = (icon_size * 0.35).max(10.0);
                    let badge_rect = egui::Rect::from_min_size(
                        icon_rect.min + egui::vec2(0.0, icon_size - badge_size),
                        egui::vec2(badge_size, badge_size),
                    );
                    let badge_bg = if selected {
                        palette.panel
                    } else {
                        Color32::BLACK
                    };
                    ui.painter().rect_filled(badge_rect, 0.0, badge_bg);
                    Self::paint_tinted_texture(
                        ui.painter(),
                        &tex_shortcut_badge,
                        badge_rect,
                        icon_fg,
                    );
                }
                DesktopIconStyle::NoIcons => {}
            }

            if label_height > 0.0 {
                Self::paint_desktop_icon_label(ui, label_rect, &shortcut.label, icon_fg);
            }

            if response.dragged() {
                update_dragged_icon_position(
                    &mut self.settings.draft,
                    &key,
                    [top_left.x, top_left.y],
                    [response.drag_delta().x, response.drag_delta().y],
                );
            }
            if response.drag_stopped() {
                needs_persist |=
                    finalize_dragged_icon_position(&mut self.settings.draft, &key, drag_grid);
            }

            if response.clicked() || response.secondary_clicked() {
                self.desktop_selected_icon = Some(DesktopIconSelection::Shortcut(sidx));
            }

            let app_name_for_menu = shortcut.app_name.clone();
            response.context_menu(|ui| {
                Self::apply_context_menu_style(ui);
                ui.set_min_width(136.0);
                ui.set_max_width(180.0);
                if ui.button("Open").clicked() {
                    desktop_action =
                        Some(ContextMenuAction::LaunchShortcut(app_name_for_menu.clone()));
                    ui.close_menu();
                }
                Self::retro_separator(ui);
                if ui.button("Properties").clicked() {
                    desktop_action = Some(ContextMenuAction::OpenShortcutProperties(sidx));
                    ui.close_menu();
                }
                Self::retro_separator(ui);
                if ui.button("Delete Shortcut").clicked() {
                    desktop_action = Some(ContextMenuAction::DeleteShortcut(sidx));
                    ui.close_menu();
                }
            });

            if response.double_clicked() {
                desktop_action = Some(ContextMenuAction::LaunchShortcut(shortcut.app_name.clone()));
            }
        }

        if needs_persist {
            self.persist_native_settings();
        }

        if let Some(action) = desktop_action {
            match action {
                ContextMenuAction::DeleteShortcut(idx) => {
                    if delete_desktop_shortcut(&mut self.settings.draft, idx) {
                        if self.desktop_selected_icon == Some(DesktopIconSelection::Shortcut(idx)) {
                            self.desktop_selected_icon = None;
                        }
                        self.persist_native_settings();
                    }
                }
                _ => {
                    self.context_menu_action = Some(action);
                }
            }
        }

        if open_terminal {
            self.launch_desktop_terminal_shell_via_registry();
        } else if let Some(path) = open_desktop_path {
            self.open_desktop_surface_path(path);
        } else if let Some(window) = open_window {
            self.open_or_spawn_desktop_window(window);
        }
    }

    pub(super) fn draw_shortcut_properties_window(&mut self, ctx: &egui::Context) {
        let Some(props) = self.shortcut_properties.clone() else {
            return;
        };
        let palette = current_palette();
        let props_idx = props.shortcut_idx;
        let mut name_draft = props.name_draft.clone();
        let mut command_draft = props.command_draft.clone();
        let icon_path_draft = props.icon_path_draft.clone();
        let mut action: Option<&'static str> = None;

        egui::Window::new("shortcut_properties_window")
            .title_bar(false)
            .resizable(false)
            .collapsible(false)
            .frame(Self::desktop_window_frame())
            .fixed_size(egui::vec2(360.0, 260.0))
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
            .show(ctx, |ui| {
                Self::apply_settings_control_style(ui);

                // Header
                let header_action =
                    Self::draw_desktop_window_header(ui, "Shortcut Properties", false);
                if matches!(
                    header_action,
                    super::desktop_window_mgmt::DesktopHeaderAction::Close
                ) {
                    action = Some("cancel");
                }

                ui.add_space(12.0);

                // Icon preview + shortcut label
                ui.horizontal(|ui| {
                    // Icon preview box
                    let icon_size = 48.0;
                    let (rect, _) = ui.allocate_exact_size(
                        egui::vec2(icon_size, icon_size),
                        egui::Sense::hover(),
                    );
                    // Draw current icon
                    let icon_tex: Option<egui::TextureHandle> = icon_path_draft
                        .as_ref()
                        .and_then(|p| self.load_cached_shortcut_icon(ctx, p, Path::new(p), 48));
                    if let Some(tex) = icon_tex {
                        Self::paint_tinted_texture(ui.painter(), &tex, rect, palette.fg);
                    } else if let Some(cache) = &self.asset_cache {
                        let icon = cache.icon_applications.clone();
                        Self::paint_tinted_texture(ui.painter(), &icon, rect, palette.fg);
                    }
                    ui.add_space(8.0);
                    ui.label(
                        RichText::new(&name_draft)
                            .strong()
                            .monospace()
                            .color(palette.fg),
                    );
                });

                ui.add_space(8.0);
                Self::retro_separator(ui);
                ui.add_space(8.0);

                // Name field
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Name:   ").monospace().color(palette.fg));
                    let name_edit = egui::TextEdit::singleline(&mut name_draft)
                        .font(egui::TextStyle::Monospace)
                        .desired_width(220.0);
                    ui.add(name_edit);
                });

                ui.add_space(6.0);

                // Target field
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Target: ").monospace().color(palette.fg));
                    let cmd_edit = egui::TextEdit::singleline(&mut command_draft)
                        .font(egui::TextStyle::Monospace)
                        .desired_width(220.0);
                    ui.add(cmd_edit);
                });

                ui.add_space(6.0);

                // Change Icon button
                ui.horizontal(|ui| {
                    ui.add_space(80.0);
                    if ui.button("Change Icon...").clicked() {
                        action = Some("change_icon");
                    }
                    if let Some(path) = &icon_path_draft {
                        let filename = std::path::Path::new(path)
                            .file_name()
                            .map(|n| n.to_string_lossy().to_string())
                            .unwrap_or_default();
                        ui.label(
                            RichText::new(filename)
                                .small()
                                .monospace()
                                .color(palette.dim),
                        );
                    }
                });

                ui.add_space(12.0);
                Self::retro_separator(ui);
                ui.add_space(8.0);

                // OK / Cancel
                ui.horizontal(|ui| {
                    ui.add_space(ui.available_width() / 2.0 - 70.0);
                    if ui.button("  OK  ").clicked() {
                        action = Some("ok");
                    }
                    ui.add_space(8.0);
                    if ui.button("Cancel").clicked() {
                        action = Some("cancel");
                    }
                });

                // Sync drafts back to state
                if let Some(props) = &mut self.shortcut_properties {
                    props.name_draft = name_draft;
                    props.command_draft = command_draft;
                }
            });

        // Handle deferred actions OUTSIDE the window closure (to avoid double-borrow)
        match action {
            Some("ok") => {
                if let Some(props) = &self.shortcut_properties {
                    let update = ShortcutPropertiesUpdate {
                        label: props.name_draft.clone(),
                        command_draft: props.command_draft.clone(),
                        icon_path: props.icon_path_draft.clone(),
                    };
                    update_desktop_shortcut_properties(
                        &mut self.settings.draft,
                        props_idx,
                        &update,
                    );
                }
                self.persist_native_settings();
                self.shortcut_properties = None;
            }
            Some("cancel") => {
                self.shortcut_properties = None;
            }
            Some("change_icon") => {
                let icons_dir =
                    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/Icons");
                self.picking_icon_for_shortcut = Some(props_idx);
                self.open_embedded_file_manager_at(icons_dir);
            }
            _ => {}
        }
    }

    pub(super) fn draw_desktop_item_properties_window(&mut self, ctx: &egui::Context) {
        let Some(props) = self.desktop_item_properties.clone() else {
            return;
        };
        let palette = current_palette();
        let mut name_draft = props.name_draft.clone();
        let mut action: Option<&'static str> = None;
        let item_name = props
            .path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("item")
            .to_string();
        let item_type = if props.is_dir { "Folder" } else { "File" };
        let path_display = props.path.display().to_string();

        egui::Window::new("desktop_item_properties_window")
            .title_bar(false)
            .resizable(false)
            .collapsible(false)
            .frame(Self::desktop_window_frame())
            .fixed_size(egui::vec2(440.0, 250.0))
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
            .show(ctx, |ui| {
                Self::apply_settings_control_style(ui);
                let header_action =
                    Self::draw_desktop_window_header(ui, "Desktop Item Properties", false);
                if matches!(
                    header_action,
                    super::desktop_window_mgmt::DesktopHeaderAction::Close
                ) {
                    action = Some("cancel");
                }

                ui.add_space(12.0);
                ui.label(
                    RichText::new(&item_name)
                        .strong()
                        .monospace()
                        .color(palette.fg),
                );
                ui.add_space(6.0);
                ui.label(RichText::new(format!("Type: {item_type}")).color(palette.dim));
                ui.add_space(6.0);
                ui.label(RichText::new("Path:").color(palette.dim));
                ui.label(RichText::new(path_display).monospace().color(palette.fg));

                ui.add_space(12.0);
                Self::retro_separator(ui);
                ui.add_space(10.0);

                ui.horizontal(|ui| {
                    ui.label(RichText::new("Name:").monospace().color(palette.fg));
                    let edit = egui::TextEdit::singleline(&mut name_draft)
                        .font(egui::TextStyle::Monospace)
                        .desired_width(260.0);
                    ui.add(edit);
                });

                ui.add_space(16.0);
                ui.horizontal(|ui| {
                    if ui.button("Open").clicked() {
                        action = Some("open");
                    }
                    if !props.is_dir && ui.button("Open With...").clicked() {
                        action = Some("open_with");
                    }
                    if ui.button("Delete").clicked() {
                        action = Some("delete");
                    }
                });

                ui.add_space(12.0);
                Self::retro_separator(ui);
                ui.add_space(10.0);

                ui.horizontal(|ui| {
                    ui.add_space(ui.available_width() / 2.0 - 70.0);
                    if ui.button(" Save ").clicked() {
                        action = Some("save");
                    }
                    ui.add_space(8.0);
                    if ui.button("Cancel").clicked() {
                        action = Some("cancel");
                    }
                });
            });

        if let Some(props) = &mut self.desktop_item_properties {
            props.name_draft = name_draft.clone();
        }

        match action {
            Some("open") => {
                self.open_desktop_surface_path(props.path.clone());
                self.desktop_item_properties = None;
            }
            Some("open_with") => {
                self.open_desktop_surface_with_prompt(props.path.clone());
                self.desktop_item_properties = None;
            }
            Some("delete") => {
                self.delete_desktop_item(props.path.clone());
            }
            Some("save") => {
                let entry = FileEntryRow {
                    path: props.path.clone(),
                    label: item_name,
                    is_dir: props.is_dir,
                };
                self.shell_status = match self.file_manager_runtime.rename_entry(entry, name_draft)
                {
                    Ok(new_path) => {
                        self.desktop_selected_icon = Some(DesktopIconSelection::Surface(format!(
                            "desktop_item:{}",
                            new_path
                                .file_name()
                                .and_then(|name| name.to_str())
                                .unwrap_or("item")
                        )));
                        self.desktop_item_properties = None;
                        self.invalidate_desktop_surface_cache();
                        "Desktop item renamed.".to_string()
                    }
                    Err(err) => format!("Desktop rename failed: {err}"),
                };
            }
            Some("cancel") => {
                self.desktop_item_properties = None;
            }
            _ => {}
        }
    }

    pub(super) fn draw_desktop(&mut self, ctx: &Context) {
        if self.asset_cache.is_none() {
            self.asset_cache = Some(Self::build_asset_cache(ctx));
        }
        self.sync_wallpaper(ctx);
        let palette = current_palette();
        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(palette.bg).inner_margin(0.0))
            .show(ctx, |ui| {
                let rect = ui.max_rect();
                let response = ui.allocate_rect(rect, egui::Sense::click());
                let desktop_dir = desktop_surface_dir();
                let file_manager_drop_hover = response
                    .dnd_hover_payload::<NativeFileManagerDragPayload>()
                    .is_some_and(|payload| {
                        Self::file_manager_drop_allowed(&payload.paths, &desktop_dir)
                    });
                if !self.draw_wallpaper(ui.painter(), rect, &palette) {
                    ui.painter().rect_filled(rect, 0.0, palette.bg);
                }
                if file_manager_drop_hover {
                    ui.painter().rect_stroke(
                        rect.shrink(6.0),
                        0.0,
                        egui::Stroke::new(2.0, palette.fg),
                    );
                }
                if !matches!(
                    self.settings.draft.desktop_icon_style,
                    DesktopIconStyle::NoIcons
                ) {
                    self.draw_desktop_icons(ui);
                }
                if let Some(payload) =
                    response.dnd_release_payload::<NativeFileManagerDragPayload>()
                {
                    if Self::file_manager_drop_allowed(&payload.paths, &desktop_dir) {
                        self.file_manager_handle_drop_to_dir(payload.paths.clone(), desktop_dir);
                    }
                }
                Self::attach_desktop_empty_context_menu(
                    &mut self.context_menu_action,
                    &response,
                    self.settings.draft.desktop_snap_to_grid,
                    self.settings.draft.desktop_icon_sort,
                );
                let dropped_paths: Vec<PathBuf> = ctx.input(|input| {
                    let hovered = input
                        .pointer
                        .hover_pos()
                        .is_some_and(|pos| rect.contains(pos));
                    if !hovered {
                        return Vec::new();
                    }
                    input
                        .raw
                        .dropped_files
                        .iter()
                        .filter_map(|file| file.path.clone())
                        .collect()
                });
                if !dropped_paths.is_empty() {
                    self.import_paths_to_desktop(dropped_paths);
                }
                if response.clicked() {
                    self.close_desktop_overlays();
                    self.desktop_selected_icon = None;
                }
            });
    }
}
