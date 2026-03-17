use super::*;
use crate::config::FileManagerViewMode;
use crate::native::file_manager_desktop::FILE_MANAGER_APP_TITLE;

impl RobcoNativeApp {
    fn attach_file_manager_context_menu(
        action: &mut Option<ContextMenuAction>,
        response: &egui::Response,
        has_selection: bool,
        has_file_selection: bool,
        has_clipboard: bool,
    ) {
        response.context_menu(|ui| {
            Self::apply_context_menu_style(ui);
            ui.set_min_width(136.0);
            ui.set_max_width(180.0);

            let open = if has_selection {
                ui.button("Open")
            } else {
                Self::retro_disabled_button(ui, "Open")
            };
            if open.clicked() {
                *action = Some(ContextMenuAction::Open);
                ui.close_menu();
            }
            let open_with = if has_file_selection {
                ui.button("Open with...")
            } else {
                Self::retro_disabled_button(ui, "Open with...")
            };
            if open_with.clicked() {
                *action = Some(ContextMenuAction::OpenWith);
                ui.close_menu();
            }

            Self::retro_separator(ui);

            let rename = if has_selection {
                ui.button("Rename")
            } else {
                Self::retro_disabled_button(ui, "Rename")
            };
            if rename.clicked() {
                *action = Some(ContextMenuAction::Rename);
                ui.close_menu();
            }

            Self::retro_separator(ui);

            let cut = if has_selection {
                ui.button("Cut")
            } else {
                Self::retro_disabled_button(ui, "Cut")
            };
            if cut.clicked() {
                *action = Some(ContextMenuAction::Cut);
                ui.close_menu();
            }
            let copy = if has_selection {
                ui.button("Copy")
            } else {
                Self::retro_disabled_button(ui, "Copy")
            };
            if copy.clicked() {
                *action = Some(ContextMenuAction::Copy);
                ui.close_menu();
            }
            let paste = if has_clipboard {
                ui.button("Paste")
            } else {
                Self::retro_disabled_button(ui, "Paste")
            };
            if paste.clicked() {
                *action = Some(ContextMenuAction::Paste);
                ui.close_menu();
            }
            if ui.button("New Folder").clicked() {
                *action = Some(ContextMenuAction::NewFolder);
                ui.close_menu();
            }
            let duplicate = if has_selection {
                ui.button("Duplicate")
            } else {
                Self::retro_disabled_button(ui, "Duplicate")
            };
            if duplicate.clicked() {
                *action = Some(ContextMenuAction::Duplicate);
                ui.close_menu();
            }

            Self::retro_separator(ui);

            let delete = if has_selection {
                ui.button("Delete")
            } else {
                Self::retro_disabled_button(ui, "Delete")
            };
            if delete.clicked() {
                *action = Some(ContextMenuAction::Delete);
                ui.close_menu();
            }

            Self::retro_separator(ui);

            let properties = if has_selection {
                ui.button("Properties")
            } else {
                Self::retro_disabled_button(ui, "Properties")
            };
            if properties.clicked() {
                *action = Some(ContextMenuAction::Properties);
                ui.close_menu();
            }
        });
    }

    pub(super) fn preload_file_manager_svg_previews(
        &mut self,
        ctx: &Context,
        rows: &[super::super::file_manager::FileEntryRow],
    ) {
        let svg_paths: Vec<PathBuf> = rows
            .iter()
            .filter(|row| {
                row.path
                    .extension()
                    .and_then(|e| e.to_str())
                    .map(|e| e.eq_ignore_ascii_case("svg"))
                    .unwrap_or(false)
            })
            .map(|row| row.path.clone())
            .collect();
        for path in svg_paths {
            let key = path.to_string_lossy().to_string();
            if !self.shortcut_icon_cache.contains_key(&key)
                && !self.shortcut_icon_missing.contains(&key)
            {
                let _ = self.load_cached_shortcut_icon(ctx, &key, &path, 32);
            }
        }
    }

    fn retro_file_manager_button(
        ui: &mut egui::Ui,
        label: impl Into<String>,
        desired: egui::Vec2,
        active: bool,
        stroked: bool,
    ) -> egui::Response {
        let palette = current_palette();
        let label = label.into();
        let (rect, response) = ui.allocate_exact_size(desired, egui::Sense::click());
        let hovered = response.hovered();
        let highlighted = active || hovered;
        let fill = if highlighted {
            palette.fg
        } else {
            Color32::TRANSPARENT
        };
        let text_color = if highlighted {
            Color32::BLACK
        } else {
            palette.fg
        };
        let stroke = if stroked {
            egui::Stroke::new(1.0, palette.fg)
        } else {
            egui::Stroke::NONE
        };
        let painter = ui.painter_at(rect.intersect(ui.clip_rect()));
        if highlighted {
            painter.rect_filled(rect, 0.0, fill);
        }
        if stroke != egui::Stroke::NONE {
            painter.rect_stroke(rect, 0.0, stroke);
        }
        painter.text(
            rect.left_top() + egui::vec2(8.0, 6.0),
            Align2::LEFT_TOP,
            label,
            FontId::new(18.0, FontFamily::Monospace),
            text_color,
        );
        response
    }

    fn retro_file_manager_item(
        ui: &mut egui::Ui,
        texture: Option<&TextureHandle>,
        fallback_icon: &str,
        label: &str,
        desired: egui::Vec2,
        active: bool,
        stroked: bool,
        grid_mode: bool,
    ) -> egui::Response {
        let palette = current_palette();
        let (rect, response) = ui.allocate_exact_size(desired, egui::Sense::click_and_drag());
        let hovered = response.hovered();
        let highlighted = active || hovered;
        let fill = if highlighted {
            palette.fg
        } else {
            Color32::TRANSPARENT
        };
        let text_color = if highlighted {
            Color32::BLACK
        } else {
            palette.fg
        };
        let stroke = if stroked {
            egui::Stroke::new(1.0, palette.fg)
        } else {
            egui::Stroke::NONE
        };
        let painter = ui.painter_at(rect.intersect(ui.clip_rect()));
        if highlighted {
            painter.rect_filled(rect, 0.0, fill);
        }
        if stroke != egui::Stroke::NONE {
            painter.rect_stroke(rect, 0.0, stroke);
        }

        if grid_mode {
            let icon_rect = egui::Rect::from_center_size(
                egui::pos2(rect.center().x, rect.top() + 18.0),
                egui::vec2(24.0, 24.0),
            );
            if let Some(texture) = texture {
                Self::paint_tinted_texture(&painter, texture, icon_rect, text_color);
            } else {
                painter.text(
                    icon_rect.center(),
                    Align2::CENTER_CENTER,
                    fallback_icon,
                    FontId::new(16.0, FontFamily::Monospace),
                    text_color,
                );
            }
            painter.text(
                egui::pos2(rect.center().x, rect.bottom() - 14.0),
                Align2::CENTER_CENTER,
                label,
                FontId::new(16.0, FontFamily::Monospace),
                text_color,
            );
        } else {
            let icon_rect = egui::Rect::from_min_size(
                rect.left_top() + egui::vec2(6.0, 4.0),
                egui::vec2(18.0, 18.0),
            );
            if let Some(texture) = texture {
                Self::paint_tinted_texture(&painter, texture, icon_rect, text_color);
            } else {
                painter.text(
                    icon_rect.center(),
                    Align2::CENTER_CENTER,
                    fallback_icon,
                    FontId::new(14.0, FontFamily::Monospace),
                    text_color,
                );
            }
            painter.text(
                rect.left_top() + egui::vec2(30.0, 6.0),
                Align2::LEFT_TOP,
                label,
                FontId::new(18.0, FontFamily::Monospace),
                text_color,
            );
        }
        response
    }

    fn handle_file_manager_row_interaction(
        &mut self,
        ctx: &Context,
        ui: &egui::Ui,
        response: &egui::Response,
        row: &super::super::file_manager::FileEntryRow,
        save_picker_mode: bool,
        action_selection_paths: &[PathBuf],
        has_editable_selection: bool,
        has_single_file_selection: bool,
        has_clipboard: bool,
    ) {
        let allow_multi = !save_picker_mode
            && self.picking_icon_for_shortcut.is_none()
            && !self.picking_wallpaper;
        let ctrl_toggle = allow_multi && ctx.input(|i| i.modifiers.ctrl);

        if response.secondary_clicked() && !self.file_manager.is_path_selected(&row.path) {
            self.file_manager.select(Some(row.path.clone()));
        }
        if response.clicked() {
            self.file_manager_select_path(row.path.clone(), ctrl_toggle, allow_multi);
        }
        if response.drag_started() && !save_picker_mode && !row.is_parent_dir() {
            if !self.file_manager.is_path_selected(&row.path) {
                self.file_manager.select(Some(row.path.clone()));
            }
        }
        if !save_picker_mode && !row.is_parent_dir() {
            let drag_paths = if action_selection_paths.iter().any(|path| path == &row.path) {
                action_selection_paths.to_vec()
            } else {
                vec![row.path.clone()]
            };
            if !drag_paths.is_empty() {
                response.dnd_set_drag_payload(NativeFileManagerDragPayload { paths: drag_paths });
            }
        }
        if row.is_dir && !save_picker_mode {
            let drop_hover = response
                .dnd_hover_payload::<NativeFileManagerDragPayload>()
                .is_some_and(|payload| Self::file_manager_drop_allowed(&payload.paths, &row.path));
            if drop_hover {
                ui.painter().rect_stroke(
                    response.rect,
                    0.0,
                    egui::Stroke::new(2.0, current_palette().fg),
                );
            }
            if let Some(payload) = response.dnd_release_payload::<NativeFileManagerDragPayload>() {
                if Self::file_manager_drop_allowed(&payload.paths, &row.path) {
                    self.file_manager_handle_drop_to_dir(payload.paths.clone(), row.path.clone());
                }
            }
        }
        if response.double_clicked() {
            self.file_manager.select(Some(row.path.clone()));
            self.file_manager_activate_or_pick();
        }
        Self::attach_file_manager_context_menu(
            &mut self.context_menu_action,
            response,
            has_editable_selection,
            has_single_file_selection,
            has_clipboard,
        );
    }

    pub(super) fn draw_file_manager_top_panel(
        &mut self,
        ctx: &Context,
        ui: &mut egui::Ui,
        generation: u64,
        maximized: bool,
        save_picker_mode: bool,
        desktop_model: &file_manager_desktop::FileManagerDesktopViewModel,
        search_id: &Id,
        header_action: &mut DesktopHeaderAction,
    ) {
        egui::TopBottomPanel::top(Id::new(("fm_top", generation)))
            .frame(egui::Frame::none())
            .exact_height(136.0)
            .show_inside(ui, |ui| {
                *header_action =
                    Self::draw_desktop_window_header(ui, FILE_MANAGER_APP_TITLE, maximized);

                if let Some(banner) = desktop_model.action_mode.banner() {
                    let banner_color = current_palette().fg;
                    ui.colored_label(banner_color, banner);
                }

                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new("Tabs").strong());
                    for (idx, tab) in desktop_model.tabs.iter().enumerate() {
                        let title = Self::truncate_file_manager_label(&tab.title, 12);
                        let response = Self::retro_file_manager_button(
                            ui,
                            format!(
                                "[{}:{}{}]",
                                idx + 1,
                                title,
                                if tab.active { "*" } else { "" }
                            ),
                            egui::vec2(132.0, 28.0),
                            tab.active,
                            false,
                        );
                        if response.clicked() {
                            let _ = self.file_manager.switch_to_tab(idx);
                        }
                    }
                    if ui.button("+").clicked() {
                        self.run_file_manager_command(FileManagerCommand::NewTab);
                    }
                    let close_tab = if desktop_model.close_tab_enabled() {
                        ui.button("x")
                    } else {
                        Self::retro_disabled_button(ui, "x")
                    };
                    if close_tab.clicked() {
                        self.run_file_manager_command(FileManagerCommand::CloseTab);
                    }
                });

                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new("Drives").strong());
                    for drive in &desktop_model.drives {
                        let response = Self::retro_file_manager_button(
                            ui,
                            drive.label.clone(),
                            egui::vec2(120.0, 26.0),
                            drive.active,
                            true,
                        );
                        let drop_hover = !save_picker_mode
                            && response
                                .dnd_hover_payload::<NativeFileManagerDragPayload>()
                                .is_some_and(|payload| {
                                    Self::file_manager_drop_allowed(&payload.paths, &drive.path)
                                });
                        if drop_hover {
                            ui.painter().rect_stroke(
                                response.rect,
                                0.0,
                                egui::Stroke::new(2.0, current_palette().fg),
                            );
                        }
                        if response.clicked() {
                            self.file_manager
                                .open_selected_tree_path(drive.path.clone());
                        }
                        if let Some(payload) =
                            response.dnd_release_payload::<NativeFileManagerDragPayload>()
                        {
                            if !save_picker_mode
                                && Self::file_manager_drop_allowed(&payload.paths, &drive.path)
                            {
                                self.file_manager_handle_drop_to_dir(
                                    payload.paths.clone(),
                                    drive.path.clone(),
                                );
                            }
                        }
                    }
                });

                let search_requested = self.desktop_active_window
                    == Some(DesktopWindow::FileManager)
                    && ctx.input(|i| i.modifiers.ctrl && i.key_pressed(Key::F));
                if search_requested {
                    ui.memory_mut(|mem| mem.request_focus(search_id.clone()));
                }

                let mut search_query = desktop_model.search_query.clone();
                ui.horizontal(|ui| {
                    ui.label(RichText::new(desktop_model.search_label()).strong());
                    let search_width = (ui.available_width() - 240.0).max(180.0);
                    let search = ui.add_sized(
                        [search_width, 0.0],
                        TextEdit::singleline(&mut search_query)
                            .id(search_id.clone())
                            .hint_text("filter files and folders"),
                    );
                    if search.changed() {
                        self.file_manager.update_search_query(search_query.clone());
                    }
                    ui.add_space(8.0);
                    let tree_on = desktop_model.show_tree_panel;
                    let list_on = desktop_model.view_mode == FileManagerViewMode::List;
                    let grid_on = desktop_model.view_mode == FileManagerViewMode::Grid;
                    let fg = current_palette().fg;
                    let sel_color = |on: bool| if on { Color32::BLACK } else { fg };
                    if ui
                        .selectable_label(tree_on, RichText::new("Tree").color(sel_color(tree_on)))
                        .clicked()
                    {
                        self.run_file_manager_command(FileManagerCommand::ToggleTreePanel);
                    }
                    if ui
                        .selectable_label(list_on, RichText::new("List").color(sel_color(list_on)))
                        .clicked()
                    {
                        self.run_file_manager_command(FileManagerCommand::SetViewMode(
                            FileManagerViewMode::List,
                        ));
                    }
                    if ui
                        .selectable_label(grid_on, RichText::new("Grid").color(sel_color(grid_on)))
                        .clicked()
                    {
                        self.run_file_manager_command(FileManagerCommand::SetViewMode(
                            FileManagerViewMode::Grid,
                        ));
                    }
                });

                if let Some(drive) = &desktop_model.current_drive_label {
                    ui.label(RichText::new(format!("Drive: {drive}")).strong());
                }
                ui.label(RichText::new(&desktop_model.path_label).strong());
                ui.separator();
            });
    }

    pub(super) fn draw_file_manager_footer_panel(
        &mut self,
        ui: &mut egui::Ui,
        generation: u64,
        save_picker_mode: bool,
        footer_model: &file_manager_desktop::FileManagerDesktopFooterModel,
    ) {
        egui::TopBottomPanel::bottom(Id::new(("fm_bottom", generation)))
            .frame(egui::Frame::none())
            .exact_height(if save_picker_mode { 56.0 } else { 44.0 })
            .show_inside(ui, |ui| {
                ui.painter().hline(
                    ui.max_rect().x_range(),
                    ui.max_rect().top() + 1.0,
                    egui::Stroke::new(1.0, current_palette().fg),
                );
                ui.add_space(4.0);
                if let Some(current_name) = &footer_model.file_name {
                    let mut file_name = current_name.clone();
                    let mut triggered_action = None;
                    ui.horizontal(|ui| {
                        for (idx, item) in footer_model.status_items.iter().enumerate() {
                            if idx > 0 {
                                ui.separator();
                            }
                            ui.small(item);
                        }
                        if !footer_model.status_items.is_empty() {
                            ui.separator();
                        }
                        for button in &footer_model.leading_buttons {
                            if ui.button(button.label).clicked() {
                                triggered_action = Some(button.action);
                            }
                        }
                        ui.separator();
                        ui.label("Name");
                        ui.add_sized(
                            [280.0, 28.0],
                            TextEdit::singleline(&mut file_name).hint_text("document.txt"),
                        );
                        ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                            for button in &footer_model.trailing_buttons {
                                let size = if matches!(
                                    button.action,
                                    FileManagerDesktopFooterAction::SaveHere
                                ) {
                                    [132.0, 28.0]
                                } else {
                                    [100.0, 28.0]
                                };
                                if ui
                                    .add_sized(size, egui::Button::new(button.label))
                                    .clicked()
                                {
                                    triggered_action = Some(button.action);
                                }
                            }
                        });
                    });
                    self.editor.save_as_input = Some(file_name);
                    if let Some(action) = triggered_action {
                        self.apply_file_manager_desktop_footer_action(action);
                    }
                } else {
                    ui.horizontal(|ui| {
                        for (idx, item) in footer_model.status_items.iter().enumerate() {
                            if idx > 0 {
                                ui.separator();
                            }
                            ui.small(item);
                        }
                        ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                            for button in &footer_model.trailing_buttons {
                                if ui.button(button.label).clicked() {
                                    self.apply_file_manager_desktop_footer_action(button.action);
                                }
                            }
                        });
                    });
                }
            });
    }

    pub(super) fn draw_file_manager_tree_panel(
        &mut self,
        ui: &mut egui::Ui,
        generation: u64,
        save_picker_mode: bool,
        desktop_model: &file_manager_desktop::FileManagerDesktopViewModel,
    ) {
        if !desktop_model.show_tree_panel {
            return;
        }
        egui::SidePanel::left(Id::new(("fm_tree", generation)))
            .frame(egui::Frame::none())
            .width_range(140.0..=280.0)
            .default_width(200.0)
            .show_inside(ui, |ui| {
                ui.label(RichText::new("Locations").strong());
                egui::ScrollArea::vertical()
                    .id_salt(("native_file_manager_tree", generation))
                    .show(ui, |ui| {
                        for item in &desktop_model.tree_items {
                            if item.path.is_none() {
                                ui.add_space(4.0);
                                ui.label(RichText::new(&item.line).strong());
                                continue;
                            }
                            let Some(path) = item.path.as_ref() else {
                                continue;
                            };
                            let selected = Some(path) == self.file_manager.tree_selected.as_ref();
                            let response = Self::retro_file_manager_button(
                                ui,
                                item.line.clone(),
                                egui::vec2(ui.available_width(), 26.0),
                                selected,
                                false,
                            );
                            let drop_hover = !save_picker_mode
                                && response
                                    .dnd_hover_payload::<NativeFileManagerDragPayload>()
                                    .is_some_and(|payload| {
                                        Self::file_manager_drop_allowed(&payload.paths, path)
                                    });
                            if drop_hover {
                                ui.painter().rect_stroke(
                                    response.rect,
                                    0.0,
                                    egui::Stroke::new(2.0, current_palette().fg),
                                );
                            }
                            if response.clicked() {
                                self.file_manager.open_selected_tree_path(path.clone());
                            }
                            if let Some(payload) =
                                response.dnd_release_payload::<NativeFileManagerDragPayload>()
                            {
                                if !save_picker_mode
                                    && Self::file_manager_drop_allowed(&payload.paths, path)
                                {
                                    self.file_manager_handle_drop_to_dir(
                                        payload.paths.clone(),
                                        path.clone(),
                                    );
                                }
                            }
                        }
                    });
            });
    }

    pub(super) fn draw_file_manager_content_panel(
        &mut self,
        ctx: &Context,
        ui: &mut egui::Ui,
        generation: u64,
        save_picker_mode: bool,
        desktop_model: &file_manager_desktop::FileManagerDesktopViewModel,
        action_selection_paths: &[PathBuf],
        has_editable_selection: bool,
        has_single_file_selection: bool,
        has_clipboard: bool,
    ) {
        egui::CentralPanel::default()
            .frame(egui::Frame::none())
            .show_inside(ui, |ui| {
                if desktop_model.rows.is_empty() {
                    ui.label("No files match the current search.");
                    return;
                }
                match desktop_model.view_mode {
                    FileManagerViewMode::List => {
                        egui::ScrollArea::vertical()
                            .id_salt(("native_file_manager_list", generation))
                            .show(ui, |ui| {
                                for row in &desktop_model.rows {
                                    let selected = self.file_manager.is_path_selected(&row.path);
                                    let preview = self.svg_preview_texture(row);
                                    let response = Self::retro_file_manager_item(
                                        ui,
                                        preview.as_ref(),
                                        row.icon(),
                                        &row.label,
                                        egui::vec2(ui.available_width(), 28.0),
                                        selected,
                                        false,
                                        false,
                                    );
                                    self.handle_file_manager_row_interaction(
                                        ctx,
                                        ui,
                                        &response,
                                        row,
                                        save_picker_mode,
                                        action_selection_paths,
                                        has_editable_selection,
                                        has_single_file_selection,
                                        has_clipboard,
                                    );
                                }
                                let background = ui.allocate_rect(
                                    ui.available_rect_before_wrap(),
                                    egui::Sense::click(),
                                );
                                if background.clicked() && !save_picker_mode {
                                    self.file_manager.clear_multi_selection();
                                }
                                Self::attach_file_manager_context_menu(
                                    &mut self.context_menu_action,
                                    &background,
                                    has_editable_selection,
                                    has_single_file_selection,
                                    has_clipboard,
                                );
                            });
                    }
                    FileManagerViewMode::Grid => {
                        let tile_width = 150.0;
                        let available_w = ui.available_width();
                        let cols = desktop_model.grid_columns(available_w, tile_width);
                        egui::ScrollArea::vertical()
                            .id_salt(("native_file_manager_grid", generation))
                            .show(ui, |ui| {
                                for chunk in desktop_model.rows.chunks(cols) {
                                    ui.allocate_ui_with_layout(
                                        egui::vec2(available_w, 64.0),
                                        Layout::left_to_right(egui::Align::Min),
                                        |ui| {
                                            for row in chunk {
                                                let selected =
                                                    self.file_manager.is_path_selected(&row.path);
                                                let label = Self::truncate_file_manager_label(
                                                    &row.label, 16,
                                                );
                                                let preview = self.svg_preview_texture(row);
                                                let response = Self::retro_file_manager_item(
                                                    ui,
                                                    preview.as_ref(),
                                                    row.icon(),
                                                    &label,
                                                    egui::vec2(tile_width - 8.0, 60.0),
                                                    selected,
                                                    true,
                                                    true,
                                                );
                                                self.handle_file_manager_row_interaction(
                                                    ctx,
                                                    ui,
                                                    &response,
                                                    row,
                                                    save_picker_mode,
                                                    action_selection_paths,
                                                    has_editable_selection,
                                                    has_single_file_selection,
                                                    has_clipboard,
                                                );
                                            }
                                        },
                                    );
                                }
                                let background = ui.allocate_rect(
                                    ui.available_rect_before_wrap(),
                                    egui::Sense::click(),
                                );
                                if background.clicked() && !save_picker_mode {
                                    self.file_manager.clear_multi_selection();
                                }
                                Self::attach_file_manager_context_menu(
                                    &mut self.context_menu_action,
                                    &background,
                                    has_editable_selection,
                                    has_single_file_selection,
                                    has_clipboard,
                                );
                            });
                    }
                }
            });
    }
}
