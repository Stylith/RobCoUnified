use super::*;

impl RobcoNativeApp {
    fn draw_file_manager_open_with_menu(&mut self, ui: &mut egui::Ui) {
        let Some(entry) = self.file_manager_selected_file() else {
            return;
        };
        let settings = get_settings();
        let open_with =
            file_manager_app::open_with_state_for_path(&entry.path, &settings.desktop_file_manager);

        for command in &open_with.saved_commands {
            let is_default = open_with.current_default.as_deref() == Some(command.as_str());
            let label = if is_default {
                format!("Use: {command} [default]")
            } else {
                format!("Use: {command}")
            };
            if ui.button(label).clicked() {
                match self.launch_open_with_command(&entry.path, command) {
                    Ok(message) => {
                        self.apply_file_manager_settings_update(
                            FileManagerSettingsUpdate::RecordOpenWithCommand {
                                ext_key: open_with.ext_key.clone(),
                                command: command.clone(),
                            },
                        );
                        self.shell_status = message;
                    }
                    Err(err) => {
                        self.shell_status = format!("Open failed: {err}");
                    }
                }
                ui.close_menu();
            }
        }
        if !open_with.saved_commands.is_empty() {
            Self::retro_separator(ui);
        }
        if ui.button("New Command...").clicked() {
            self.open_file_manager_prompt(FileManagerPromptRequest::open_with_new_command(
                entry.path.clone(),
                open_with.ext_key.clone(),
                false,
            ));
            ui.close_menu();
        }
        if ui
            .button(format!(
                "New Command + Always Use for {}",
                open_with.ext_label
            ))
            .clicked()
        {
            self.open_file_manager_prompt(FileManagerPromptRequest::open_with_new_command(
                entry.path.clone(),
                open_with.ext_key.clone(),
                true,
            ));
            ui.close_menu();
        }
        if !open_with.saved_commands.is_empty() {
            ui.menu_button("Edit", |ui| {
                Self::apply_top_dropdown_menu_style(ui);
                for command in &open_with.saved_commands {
                    let is_default = open_with.current_default.as_deref() == Some(command.as_str());
                    ui.menu_button(command, |ui| {
                        Self::apply_top_dropdown_menu_style(ui);
                        let default_label = if is_default {
                            "Stop Always Using"
                        } else {
                            "Always Use"
                        };
                        if ui.button(default_label).clicked() {
                            if is_default {
                                self.apply_file_manager_settings_update(
                                    FileManagerSettingsUpdate::SetOpenWithDefaultCommand {
                                        ext_key: open_with.ext_key.clone(),
                                        command: None,
                                    },
                                );
                                self.shell_status =
                                    file_manager_app::open_with_cleared_default_status(
                                        &open_with.ext_key,
                                    );
                            } else {
                                self.apply_file_manager_settings_update(
                                    FileManagerSettingsUpdate::SetOpenWithDefaultCommand {
                                        ext_key: open_with.ext_key.clone(),
                                        command: Some(command.clone()),
                                    },
                                );
                                self.shell_status = file_manager_app::open_with_set_default_status(
                                    command,
                                    &open_with.ext_key,
                                );
                            }
                            ui.close_menu();
                        }
                        if ui.button("Edit Saved").clicked() {
                            self.open_file_manager_prompt(
                                FileManagerPromptRequest::open_with_edit_command(
                                    entry.path.clone(),
                                    open_with.ext_key.clone(),
                                    command.clone(),
                                ),
                            );
                            ui.close_menu();
                        }
                        if ui.button("Remove Saved").clicked() {
                            self.apply_file_manager_settings_update(
                                FileManagerSettingsUpdate::RemoveOpenWithCommand {
                                    ext_key: open_with.ext_key.clone(),
                                    command: command.clone(),
                                },
                            );
                            self.shell_status = file_manager_app::open_with_removed_saved_status(
                                &open_with.ext_key,
                            );
                            ui.close_menu();
                        }
                    });
                }
                if open_with.current_default.is_some() {
                    Self::retro_separator(ui);
                    if ui.button("Clear Always Use").clicked() {
                        self.apply_file_manager_settings_update(
                            FileManagerSettingsUpdate::SetOpenWithDefaultCommand {
                                ext_key: open_with.ext_key.clone(),
                                command: None,
                            },
                        );
                        self.shell_status =
                            file_manager_app::open_with_cleared_default_status(&open_with.ext_key);
                        ui.close_menu();
                    }
                }
            });
        }
    }

    pub(super) fn draw_file_manager_file_menu_section(&mut self, ui: &mut egui::Ui) {
        let has_extra_tabs = self.file_manager.tabs.len() > 1;
        let has_selection = self.file_manager.selected_row().is_some();
        let has_editable_selection = self.file_manager_has_editable_selection();
        let has_file_selection = self.file_manager_selected_file().is_some();

        if ui.button("New Folder   Ctrl+Shift+N").clicked() {
            self.run_file_manager_command(FileManagerCommand::NewFolder);
            ui.close_menu();
        }
        if ui.button("New Tab").clicked() {
            self.run_file_manager_command(FileManagerCommand::NewTab);
            ui.close_menu();
        }
        Self::retro_separator(ui);
        if has_extra_tabs {
            if ui.button("Previous Tab").clicked() {
                self.run_file_manager_command(FileManagerCommand::PreviousTab);
                ui.close_menu();
            }
            if ui.button("Next Tab").clicked() {
                self.run_file_manager_command(FileManagerCommand::NextTab);
                ui.close_menu();
            }
            if ui.button("Close Tab").clicked() {
                self.run_file_manager_command(FileManagerCommand::CloseTab);
                ui.close_menu();
            }
        }
        if has_selection && ui.button("Open Selected").clicked() {
            self.run_file_manager_command(FileManagerCommand::OpenSelected);
            ui.close_menu();
        }
        if has_file_selection {
            ui.menu_button("Open With", |ui| {
                Self::apply_top_dropdown_menu_style(ui);
                self.draw_file_manager_open_with_menu(ui);
            });
        }
        if ui.button("Home").clicked() {
            self.run_file_manager_command(FileManagerCommand::OpenHome);
            ui.close_menu();
        }
        Self::retro_separator(ui);
        if has_editable_selection {
            ui.label(
                RichText::new(format!("{} selected", self.file_manager_selection_count())).small(),
            );
        }
    }

    pub(super) fn draw_file_manager_edit_menu_section(&mut self, ui: &mut egui::Ui) {
        let has_selection = self.file_manager_has_editable_selection();
        let has_clipboard = self.file_manager_runtime.has_clipboard();
        let has_history =
            self.file_manager_runtime.can_undo() || self.file_manager_runtime.can_redo();
        let paste_label = if let Some(clip) = &self.file_manager_runtime.clipboard {
            let mode = if matches!(clip.mode, FileManagerClipboardMode::Cut) {
                "Move"
            } else {
                "Paste"
            };
            if clip.paths.len() == 1 {
                format!("{mode} {}", Self::path_display_name(&clip.paths[0]))
            } else {
                format!("{mode} {} items", clip.paths.len())
            }
        } else {
            "Paste".to_string()
        };

        if ui.button("Open Selected").clicked() {
            self.run_file_manager_command(FileManagerCommand::OpenSelected);
            ui.close_menu();
        }
        if ui.button("Clear Search").clicked() {
            self.run_file_manager_command(FileManagerCommand::ClearSearch);
            ui.close_menu();
        }
        if has_selection || has_clipboard || has_history {
            Self::retro_separator(ui);
        }
        if has_selection {
            if ui.button("Copy").clicked() {
                self.run_file_manager_command(FileManagerCommand::Copy);
                ui.close_menu();
            }
            if ui.button("Cut").clicked() {
                self.run_file_manager_command(FileManagerCommand::Cut);
                ui.close_menu();
            }
            if ui.button("Duplicate").clicked() {
                self.run_file_manager_command(FileManagerCommand::Duplicate);
                ui.close_menu();
            }
            if ui.button("Rename").clicked() {
                self.run_file_manager_command(FileManagerCommand::Rename);
                ui.close_menu();
            }
            if ui.button("Move To").clicked() {
                self.run_file_manager_command(FileManagerCommand::Move);
                ui.close_menu();
            }
            if ui.button("Delete").clicked() {
                self.run_file_manager_command(FileManagerCommand::Delete);
                ui.close_menu();
            }
        }
        if has_clipboard && ui.button(paste_label).clicked() {
            self.run_file_manager_command(FileManagerCommand::Paste);
            ui.close_menu();
        }
        if self.file_manager_runtime.can_undo() && ui.button("Undo").clicked() {
            self.run_file_manager_command(FileManagerCommand::Undo);
            ui.close_menu();
        }
        if self.file_manager_runtime.can_redo() && ui.button("Redo").clicked() {
            self.run_file_manager_command(FileManagerCommand::Redo);
            ui.close_menu();
        }
        if has_selection || has_clipboard || has_history {
            Self::retro_separator(ui);
        }
        if ui.button("New Document").clicked() {
            self.run_editor_command(EditorCommand::NewDocument);
            ui.close_menu();
        }
    }

    pub(super) fn draw_file_manager_view_menu_section(&mut self, ui: &mut egui::Ui) {
        let show_tree = get_settings().desktop_file_manager.show_tree_panel;
        let show_hidden = get_settings().desktop_file_manager.show_hidden_files;
        let view_mode = self.file_manager_view_mode();
        let sort_mode = self.file_manager_sort_mode();

        if ui
            .button(if show_tree {
                "Hide Folder Tree"
            } else {
                "Show Folder Tree"
            })
            .clicked()
        {
            self.run_file_manager_command(FileManagerCommand::ToggleTreePanel);
            ui.close_menu();
        }
        if ui
            .button(if view_mode == FileManagerViewMode::Grid {
                "Grid View [Active]"
            } else {
                "Grid View"
            })
            .clicked()
        {
            self.run_file_manager_command(FileManagerCommand::SetViewMode(
                FileManagerViewMode::Grid,
            ));
            ui.close_menu();
        }
        if ui
            .button(if view_mode == FileManagerViewMode::List {
                "List View [Active]"
            } else {
                "List View"
            })
            .clicked()
        {
            self.run_file_manager_command(FileManagerCommand::SetViewMode(
                FileManagerViewMode::List,
            ));
            ui.close_menu();
        }
        if ui
            .button(if sort_mode == FileManagerSortMode::Name {
                "Sort By Name [Active]"
            } else {
                "Sort By Name"
            })
            .clicked()
        {
            self.run_file_manager_command(FileManagerCommand::SetSortMode(
                FileManagerSortMode::Name,
            ));
            ui.close_menu();
        }
        if ui
            .button(if sort_mode == FileManagerSortMode::Type {
                "Sort By Type [Active]"
            } else {
                "Sort By Type"
            })
            .clicked()
        {
            self.run_file_manager_command(FileManagerCommand::SetSortMode(
                FileManagerSortMode::Type,
            ));
            ui.close_menu();
        }
        if ui
            .button(if show_hidden {
                "Hide Hidden Files"
            } else {
                "Show Hidden Files"
            })
            .clicked()
        {
            self.run_file_manager_command(FileManagerCommand::ToggleHiddenFiles);
            ui.close_menu();
        }
        Self::retro_separator(ui);
    }
}
