use crate::config::ConnectionKind;
use super::super::desktop_connections_service::{
    connect_connection_and_refresh_settings, connection_requires_password,
    connections_macos_disabled, connections_macos_disabled_hint, discovered_connection_label,
    forget_saved_connection_and_refresh_settings, saved_connection_label,
    scan_discovered_connections,
};
use super::super::desktop_default_apps_service::{
    apply_default_app_binding, binding_label_for_slot, default_app_slot_label,
    resolve_custom_default_app_binding, DefaultAppSlot,
};
use super::super::desktop_status_service::{
    invalid_input_settings_status, mirror_shell_to_settings, settings_status,
};
use super::super::desktop_user_service::{
    create_user as create_desktop_user, delete_user as delete_desktop_user,
    toggle_user_admin as toggle_desktop_user_admin, update_user_auth_method,
    user_auth_method_label,
};
use super::super::edit_menus_screen::EditMenuTarget;
use super::super::editor_app::EDITOR_APP_TITLE;
use super::super::retro_ui::current_palette;
use super::RobcoNativeApp;
use crate::config::Settings;
use crate::core::auth::AuthMethod;
use eframe::egui::{self, RichText, TextEdit};
use robcos_native_default_apps_app::{
    build_default_app_settings_choices, default_app_slot_description,
};
use robcos_native_settings_app::{
    gui_cli_profile_mut, gui_cli_profile_slot_label, gui_cli_profile_slots,
};

impl RobcoNativeApp {
    pub(super) fn draw_settings_default_apps_panel(&mut self, ui: &mut egui::Ui) -> bool {
        let mut changed = false;
        egui::ScrollArea::vertical().show(ui, |ui| {
            for slot in [DefaultAppSlot::TextCode, DefaultAppSlot::Ebook] {
                let current_label = binding_label_for_slot(&self.settings.draft, slot);
                let custom_buffer = match slot {
                    DefaultAppSlot::TextCode => &mut self.settings.default_app_custom_text_code,
                    DefaultAppSlot::Ebook => &mut self.settings.default_app_custom_ebook,
                };

                ui.group(|ui| {
                    Self::settings_two_columns(ui, |left, right| {
                        Self::settings_section(
                            left,
                            &format!("Default App For {}", default_app_slot_label(slot)),
                            |left| {
                                left.label(format!("Currently selected: {current_label}"));
                                left.small(default_app_slot_description(slot));
                            },
                        );

                        Self::settings_section(right, "Selection", |right| {
                            let field_width =
                                Self::responsive_input_width(right, 0.85, 220.0, 620.0);
                            right.horizontal(|ui| {
                                ui.label("Chooser");
                                egui::ComboBox::from_id_salt(format!(
                                    "native_default_app_slot_{slot:?}"
                                ))
                                .selected_text(
                                    RichText::new(current_label.clone())
                                        .color(current_palette().fg),
                                )
                                .show_ui(ui, |ui| {
                                    Self::apply_settings_control_style(ui);
                                    for choice in build_default_app_settings_choices(
                                        &self.settings.draft,
                                        slot,
                                    ) {
                                        if Self::retro_choice_button(
                                            ui,
                                            choice.label,
                                            choice.selected,
                                        )
                                        .clicked()
                                        {
                                            apply_default_app_binding(
                                                &mut self.settings.draft,
                                                slot,
                                                choice.binding,
                                            );
                                            changed = true;
                                            ui.close_menu();
                                        }
                                    }
                                });
                            });
                            right.add_space(6.0);
                            right.label("Custom Command");
                            right.add(
                                TextEdit::singleline(custom_buffer)
                                    .desired_width(field_width)
                                    .hint_text("epy"),
                            );
                            if Self::retro_full_width_button(right, "Apply Custom Command")
                                .clicked()
                            {
                                match resolve_custom_default_app_binding(custom_buffer.trim()) {
                                    Ok(binding) => {
                                        apply_default_app_binding(
                                            &mut self.settings.draft,
                                            slot,
                                            binding,
                                        );
                                        changed = true;
                                    }
                                    Err(_) => {
                                        self.settings.status =
                                            "Error: invalid command line".to_string();
                                    }
                                }
                            }
                        });
                    });
                });
                ui.add_space(10.0);
            }
        });
        changed
    }

    pub(super) fn draw_settings_connections_kind_panel(&mut self, ui: &mut egui::Ui, kind: ConnectionKind) {
        if connections_macos_disabled() {
            ui.small(connections_macos_disabled_hint());
            return;
        }

        let saved_connections = self.saved_connections_cached(kind);

        let (scan_label, saved_title, discovered_title, scanned_items) = match kind {
            ConnectionKind::Network => (
                "Scan Networks",
                "Saved Networks",
                "Discovered Networks",
                &mut self.settings.scanned_networks,
            ),
            ConnectionKind::Bluetooth => (
                "Scan Bluetooth",
                "Saved Bluetooth",
                "Discovered Bluetooth",
                &mut self.settings.scanned_bluetooth,
            ),
        };

        if Self::retro_full_width_button(ui, scan_label).clicked() {
            let (discovered, status) = scan_discovered_connections(kind);
            *scanned_items = discovered;
            self.settings.status = status;
        }
        if matches!(kind, ConnectionKind::Network) {
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.label("Network Password");
                let field_width = Self::responsive_input_width(ui, 0.65, 220.0, 520.0);
                ui.add(
                    TextEdit::singleline(&mut self.settings.connection_password)
                        .desired_width(field_width)
                        .password(true),
                );
            });
            ui.small("Used only when connecting to secured networks.");
        }
        ui.add_space(8.0);
        let mut pending_settings: Option<Settings> = None;
        let mut pending_status: Option<String> = None;
        Self::settings_two_columns(ui, |left, right| {
            Self::settings_section(left, saved_title, |left| {
                if saved_connections.is_empty() {
                    left.small("No saved items.");
                } else {
                    egui::ScrollArea::vertical()
                        .max_height((left.available_height() * 0.85).clamp(180.0, 420.0))
                        .show(left, |ui| {
                            for entry in saved_connections.iter() {
                                ui.horizontal(|ui| {
                                    ui.label(saved_connection_label(entry));
                                    if ui.button("Forget").clicked() {
                                        if let Some((settings, status)) =
                                            forget_saved_connection_and_refresh_settings(
                                                kind,
                                                &entry.name,
                                            )
                                        {
                                            pending_settings = Some(settings);
                                            pending_status = Some(status);
                                        }
                                    }
                                });
                            }
                        });
                }
            });

            Self::settings_section(right, discovered_title, |right| {
                if scanned_items.is_empty() {
                    right.small("Run a scan to populate this list.");
                } else {
                    egui::ScrollArea::vertical()
                        .max_height((right.available_height() * 0.85).clamp(180.0, 420.0))
                        .show(right, |ui| {
                            for entry in scanned_items.clone() {
                                ui.horizontal(|ui| {
                                    ui.label(discovered_connection_label(&entry));
                                    if ui.button("Connect").clicked() {
                                        let password = if matches!(kind, ConnectionKind::Network)
                                            && connection_requires_password(&entry.detail)
                                            && !self.settings.connection_password.trim().is_empty()
                                        {
                                            Some(self.settings.connection_password.clone())
                                        } else {
                                            None
                                        };
                                        match connect_connection_and_refresh_settings(
                                            kind,
                                            &entry,
                                            password.as_deref(),
                                        ) {
                                            Ok((settings, status)) => {
                                                pending_settings = Some(settings);
                                                pending_status = Some(status);
                                            }
                                            Err(err) => {
                                                self.settings.status =
                                                    format!("Connect failed: {err}");
                                            }
                                        }
                                    }
                                });
                            }
                        });
                }
            });
        });
        if let Some(settings) = pending_settings {
            self.replace_settings_draft(settings);
        }
        if let Some(status) = pending_status {
            self.settings.status = status;
        }
    }

    pub(super) fn draw_settings_cli_profiles_panel(&mut self, ui: &mut egui::Ui) -> bool {
        let mut changed = false;
        let custom_profile_count = self.settings.draft.desktop_cli_profiles.custom.len();
        let profile = gui_cli_profile_mut(
            &mut self.settings.draft.desktop_cli_profiles,
            self.settings.cli_profile_slot,
        );
        let mut min_w = profile.min_w;
        let mut min_h = profile.min_h;
        Self::settings_two_columns(ui, |left, right| {
            Self::settings_section(left, "Profile", |left| {
                left.horizontal(|ui| {
                    ui.label("Profile");
                    egui::ComboBox::from_id_salt("native_settings_cli_profile_slot")
                        .selected_text(
                            RichText::new(gui_cli_profile_slot_label(
                                self.settings.cli_profile_slot,
                            ))
                            .color(current_palette().fg),
                        )
                        .show_ui(ui, |ui| {
                            Self::apply_settings_control_style(ui);
                            for slot in gui_cli_profile_slots() {
                                if Self::retro_choice_button(
                                    ui,
                                    gui_cli_profile_slot_label(slot),
                                    self.settings.cli_profile_slot == slot,
                                )
                                .clicked()
                                {
                                    self.settings.cli_profile_slot = slot;
                                    ui.close_menu();
                                }
                            }
                        });
                });
                left.add_space(8.0);
                changed |= left
                    .add(
                        egui::DragValue::new(&mut min_w)
                            .range(20..=240)
                            .prefix("Min W "),
                    )
                    .changed();
                changed |= left
                    .add(
                        egui::DragValue::new(&mut min_h)
                            .range(10..=120)
                            .prefix("Min H "),
                    )
                    .changed();

                let mut use_pref_w = profile.preferred_w.is_some();
                if Self::retro_checkbox_row(left, &mut use_pref_w, "Use Preferred Width").clicked()
                {
                    profile.preferred_w = if use_pref_w {
                        Some(profile.min_w)
                    } else {
                        None
                    };
                    changed = true;
                }
                if let Some(preferred) = profile.preferred_w.as_mut() {
                    changed |= left
                        .add(
                            egui::DragValue::new(preferred)
                                .range(profile.min_w..=280)
                                .prefix("Preferred W "),
                        )
                        .changed();
                }
            });

            Self::settings_section(right, "Behavior", |right| {
                let mut use_pref_h = profile.preferred_h.is_some();
                if Self::retro_checkbox_row(right, &mut use_pref_h, "Use Preferred Height")
                    .clicked()
                {
                    profile.preferred_h = if use_pref_h {
                        Some(profile.min_h)
                    } else {
                        None
                    };
                    changed = true;
                }
                if let Some(preferred) = profile.preferred_h.as_mut() {
                    changed |= right
                        .add(
                            egui::DragValue::new(preferred)
                                .range(profile.min_h..=140)
                                .prefix("Preferred H "),
                        )
                        .changed();
                }
                if Self::retro_checkbox_row(
                    right,
                    &mut profile.mouse_passthrough,
                    "Mouse passthrough",
                )
                .clicked()
                {
                    changed = true;
                }
                if Self::retro_checkbox_row(right, &mut profile.open_fullscreen, "Open fullscreen")
                    .clicked()
                {
                    changed = true;
                }
                if Self::retro_checkbox_row(right, &mut profile.live_resize, "Live resize")
                    .clicked()
                {
                    changed = true;
                }
                right.add_space(8.0);
                right.small(format!(
                    "Custom profiles currently stored: {}",
                    custom_profile_count
                ));
            });
        });
        if min_w != profile.min_w {
            profile.min_w = min_w;
            if let Some(preferred) = profile.preferred_w.as_mut() {
                *preferred = (*preferred).max(profile.min_w);
            }
        }
        if min_h != profile.min_h {
            profile.min_h = min_h;
            if let Some(preferred) = profile.preferred_h.as_mut() {
                *preferred = (*preferred).max(profile.min_h);
            }
        }
        changed
    }

    pub(super) fn draw_settings_edit_menus_panel(&mut self, ui: &mut egui::Ui) -> bool {
        let mut changed = false;
        ui.horizontal(|ui| {
            ui.label("Menu");
            egui::ComboBox::from_id_salt("native_settings_edit_target")
                .selected_text(
                    RichText::new(self.settings.edit_target.title()).color(current_palette().fg),
                )
                .show_ui(ui, |ui| {
                    Self::apply_settings_control_style(ui);
                    for target in [
                        EditMenuTarget::Applications,
                        EditMenuTarget::Documents,
                        EditMenuTarget::Network,
                        EditMenuTarget::Games,
                    ] {
                        if Self::retro_choice_button(
                            ui,
                            target.title(),
                            self.settings.edit_target == target,
                        )
                        .clicked()
                        {
                            self.settings.edit_target = target;
                            ui.close_menu();
                        }
                    }
                });
        });
        ui.add_space(8.0);

        Self::settings_two_columns(ui, |left, right| {
            Self::settings_section(left, "Current Entries", |left| {
                if matches!(self.settings.edit_target, EditMenuTarget::Applications) {
                    if Self::retro_checkbox_row(
                        left,
                        &mut self.settings.draft.builtin_menu_visibility.text_editor,
                        &format!("Show {EDITOR_APP_TITLE}"),
                    )
                    .clicked()
                    {
                        changed = true;
                    }
                    if Self::retro_checkbox_row(
                        left,
                        &mut self.settings.draft.builtin_menu_visibility.nuke_codes,
                        "Show Nuke Codes",
                    )
                    .clicked()
                    {
                        changed = true;
                    }
                }
                egui::ScrollArea::vertical()
                    .max_height((left.available_height() * 0.7).clamp(180.0, 380.0))
                    .show(left, |ui| {
                        let entries = self.edit_menu_entries_cached(self.settings.edit_target);
                        for name in entries.iter() {
                            ui.horizontal(|ui| {
                                ui.label(name.as_str());
                                if ui.button("Delete").clicked() {
                                    self.delete_program_entry(self.settings.edit_target, &name);
                                    self.apply_status_update(mirror_shell_to_settings(
                                        &self.shell_status,
                                    ));
                                }
                            });
                        }
                    });
            });

            Self::settings_section(right, "Add Entry", |right| {
                let name_width = Self::responsive_input_width(right, 0.9, 220.0, 420.0);
                let value_width = Self::responsive_input_width(right, 0.95, 320.0, 760.0);
                right.label("Name");
                right.add(
                    TextEdit::singleline(&mut self.settings.edit_name_input)
                        .desired_width(name_width),
                );
                right.add_space(6.0);
                let value_label = if matches!(self.settings.edit_target, EditMenuTarget::Documents)
                {
                    "Folder Path"
                } else {
                    "Command"
                };
                right.label(value_label);
                right.add(
                    TextEdit::singleline(&mut self.settings.edit_value_input)
                        .desired_width(value_width),
                );
                right.add_space(8.0);
                if Self::retro_full_width_button(right, "Add Entry").clicked() {
                    let name = self.settings.edit_name_input.trim().to_string();
                    let value = self.settings.edit_value_input.trim().to_string();
                    if name.is_empty() || value.is_empty() {
                        self.apply_status_update(invalid_input_settings_status());
                    } else {
                        match self.settings.edit_target {
                            EditMenuTarget::Documents => self.add_document_category(name, value),
                            target => self.add_program_entry(target, name, value),
                        }
                        self.apply_status_update(mirror_shell_to_settings(&self.shell_status));
                        if !self
                            .settings
                            .status
                            .to_ascii_lowercase()
                            .starts_with("error")
                        {
                            self.settings.edit_name_input.clear();
                            self.settings.edit_value_input.clear();
                        }
                    }
                }
            });
        });
        changed
    }

    pub(super) fn draw_settings_user_view_panel(&mut self, ui: &mut egui::Ui) {
        let users = self.sorted_user_records_cached();
        egui::ScrollArea::vertical().show(ui, |ui| {
            for (name, record) in users.iter() {
                ui.label(format!(
                    "{} | auth: {} | admin: {}",
                    name,
                    user_auth_method_label(&record.auth_method),
                    if record.is_admin { "yes" } else { "no" }
                ));
            }
        });
    }

    pub(super) fn draw_settings_user_create_panel(&mut self, ui: &mut egui::Ui) {
        let users = self.sorted_usernames_cached();
        ui.group(|ui| {
            Self::settings_two_columns(ui, |left, right| {
                let field_width = Self::responsive_input_width(left, 0.85, 180.0, 420.0);
                Self::settings_section(left, "Account", |left| {
                    left.label("Username");
                    left.add(
                        TextEdit::singleline(&mut self.settings.user_create_username)
                            .desired_width(field_width),
                    );
                    left.add_space(6.0);
                    left.label("Authentication");
                    egui::ComboBox::from_id_salt("native_settings_user_create_auth")
                        .selected_text(
                            RichText::new(user_auth_method_label(&self.settings.user_create_auth))
                                .color(current_palette().fg),
                        )
                        .show_ui(left, |ui| {
                            Self::apply_settings_control_style(ui);
                            for auth in [
                                AuthMethod::Password,
                                AuthMethod::NoPassword,
                                AuthMethod::HackingMinigame,
                            ] {
                                if Self::retro_choice_button(
                                    ui,
                                    user_auth_method_label(&auth),
                                    self.settings.user_create_auth == auth,
                                )
                                .clicked()
                                {
                                    self.settings.user_create_auth = auth;
                                    ui.close_menu();
                                }
                            }
                        });
                });

                let pw_width = Self::responsive_input_width(right, 0.85, 180.0, 420.0);
                let create_clicked = Self::settings_section(right, "Credentials", |right| {
                    if matches!(self.settings.user_create_auth, AuthMethod::Password) {
                        right.label("Password");
                        right.add(
                            TextEdit::singleline(&mut self.settings.user_create_password)
                                .desired_width(pw_width)
                                .password(true),
                        );
                        right.add_space(6.0);
                        right.label("Confirm");
                        right.add(
                            TextEdit::singleline(&mut self.settings.user_create_password_confirm)
                                .desired_width(pw_width)
                                .password(true),
                        );
                    } else {
                        right.small("No password fields required for this auth method.");
                    }
                    right.add_space(8.0);
                    Self::retro_full_width_button(right, "Create User").clicked()
                });
                if !create_clicked {
                    return;
                }
                let username = self.settings.user_create_username.trim().to_string();
                if username.is_empty() {
                    self.apply_status_update(settings_status("Username cannot be empty."));
                } else if users.iter().any(|name| name == &username) {
                    self.apply_status_update(settings_status("User already exists."));
                } else {
                    match self.settings.user_create_auth {
                        AuthMethod::Password => {
                            if self.settings.user_create_password.is_empty() {
                                self.apply_status_update(settings_status(
                                    "Password cannot be empty.",
                                ));
                            } else if self.settings.user_create_password
                                != self.settings.user_create_password_confirm
                            {
                                self.apply_status_update(settings_status(
                                    "Passwords do not match.",
                                ));
                            } else {
                                match create_desktop_user(
                                    &username,
                                    AuthMethod::Password,
                                    Some(&self.settings.user_create_password),
                                ) {
                                    Ok(status) => {
                                        self.invalidate_user_cache();
                                        self.apply_status_update(settings_status(status));
                                        self.settings.user_create_username.clear();
                                        self.settings.user_create_password.clear();
                                        self.settings.user_create_password_confirm.clear();
                                        self.settings.user_selected = username;
                                        self.settings.user_selected_loaded_for.clear();
                                    }
                                    Err(status) => {
                                        self.apply_status_update(settings_status(status));
                                    }
                                }
                            }
                        }
                        AuthMethod::NoPassword | AuthMethod::HackingMinigame => {
                            match create_desktop_user(
                                &username,
                                self.settings.user_create_auth.clone(),
                                None,
                            ) {
                                Ok(status) => {
                                    self.invalidate_user_cache();
                                    self.apply_status_update(settings_status(status));
                                    self.settings.user_create_username.clear();
                                    self.settings.user_selected = username;
                                    self.settings.user_selected_loaded_for.clear();
                                }
                                Err(status) => {
                                    self.apply_status_update(settings_status(status));
                                }
                            }
                        }
                    }
                }
            });
        });
    }

    pub(super) fn draw_settings_user_edit_panel(&mut self, ui: &mut egui::Ui, current_only: bool) {
        let current_username = self.session.as_ref().map(|s| s.username.clone());
        let users = self.sorted_user_records_cached();
        let names: Vec<String> = users.iter().map(|(name, _)| name.clone()).collect();
        if names.is_empty() {
            ui.small("No users found.");
            return;
        }
        if current_only {
            self.settings.user_selected = current_username.clone().unwrap_or_default();
        } else if !names
            .iter()
            .any(|name| name == &self.settings.user_selected)
        {
            self.settings.user_selected = names[0].clone();
        }
        if self.settings.user_selected_loaded_for != self.settings.user_selected {
            if let Some((_, record)) = users
                .iter()
                .find(|(name, _)| name == &self.settings.user_selected)
            {
                self.settings.user_edit_auth = record.auth_method.clone();
                self.settings.user_edit_password.clear();
                self.settings.user_edit_password_confirm.clear();
                self.settings.user_selected_loaded_for = self.settings.user_selected.clone();
            }
        }

        ui.group(|ui| {
            Self::settings_two_columns(ui, |left, right| {
                let field_width = Self::responsive_input_width(left, 0.85, 180.0, 420.0);
                Self::settings_section(
                    left,
                    if current_only {
                        "Edit Current User"
                    } else {
                        "Edit User"
                    },
                    |left| {
                        left.label("User");
                        if current_only {
                            left.label(&self.settings.user_selected);
                        } else {
                            egui::ComboBox::from_id_salt("native_settings_user_selected")
                                .selected_text(
                                    RichText::new(self.settings.user_selected.clone())
                                        .color(current_palette().fg),
                                )
                                .show_ui(left, |ui| {
                                    Self::apply_settings_control_style(ui);
                                    for name in &names {
                                        if Self::retro_choice_button(
                                            ui,
                                            name,
                                            self.settings.user_selected == *name,
                                        )
                                        .clicked()
                                        {
                                            self.settings.user_selected = name.clone();
                                            ui.close_menu();
                                        }
                                    }
                                });
                        }
                        if let Some((_, record)) = users
                            .iter()
                            .find(|(name, _)| name == &self.settings.user_selected)
                        {
                            left.small(format!(
                                "Current auth: {} | Admin: {}",
                                user_auth_method_label(&record.auth_method),
                                if record.is_admin { "yes" } else { "no" }
                            ));
                        }
                        left.add_space(8.0);
                        left.label("New Auth");
                        egui::ComboBox::from_id_salt("native_settings_user_edit_auth")
                            .selected_text(
                                RichText::new(user_auth_method_label(
                                    &self.settings.user_edit_auth,
                                ))
                                .color(current_palette().fg),
                            )
                            .show_ui(left, |ui| {
                                Self::apply_settings_control_style(ui);
                                for auth in [
                                    AuthMethod::Password,
                                    AuthMethod::NoPassword,
                                    AuthMethod::HackingMinigame,
                                ] {
                                    if Self::retro_choice_button(
                                        ui,
                                        user_auth_method_label(&auth),
                                        self.settings.user_edit_auth == auth,
                                    )
                                    .clicked()
                                    {
                                        self.settings.user_edit_auth = auth;
                                        ui.close_menu();
                                    }
                                }
                            });
                    },
                );

                let apply_auth = Self::settings_section(right, "Actions", |right| {
                    if matches!(self.settings.user_edit_auth, AuthMethod::Password) {
                        right.label("Password");
                        right.add(
                            TextEdit::singleline(&mut self.settings.user_edit_password)
                                .desired_width(field_width)
                                .password(true),
                        );
                        right.add_space(6.0);
                        right.label("Confirm");
                        right.add(
                            TextEdit::singleline(&mut self.settings.user_edit_password_confirm)
                                .desired_width(field_width)
                                .password(true),
                        );
                        right.add_space(8.0);
                    }
                    Self::retro_full_width_button(right, "Apply Auth Method").clicked()
                });
                if apply_auth {
                    let username = self.settings.user_selected.clone();
                    match self.settings.user_edit_auth {
                        AuthMethod::Password => {
                            if self.settings.user_edit_password.is_empty() {
                                self.apply_status_update(settings_status(
                                    "Password cannot be empty.",
                                ));
                            } else if self.settings.user_edit_password
                                != self.settings.user_edit_password_confirm
                            {
                                self.apply_status_update(settings_status(
                                    "Passwords do not match.",
                                ));
                            } else {
                                match update_user_auth_method(
                                    &username,
                                    AuthMethod::Password,
                                    Some(&self.settings.user_edit_password),
                                ) {
                                    Ok(status) => {
                                        self.invalidate_user_cache();
                                        self.apply_status_update(settings_status(status));
                                        self.settings.user_edit_password.clear();
                                        self.settings.user_edit_password_confirm.clear();
                                        self.settings.user_selected_loaded_for.clear();
                                    }
                                    Err(status) => {
                                        self.apply_status_update(settings_status(status));
                                    }
                                }
                            }
                        }
                        AuthMethod::NoPassword | AuthMethod::HackingMinigame => {
                            match update_user_auth_method(
                                &username,
                                self.settings.user_edit_auth.clone(),
                                None,
                            ) {
                                Ok(status) => {
                                    self.invalidate_user_cache();
                                    self.apply_status_update(settings_status(status));
                                    self.settings.user_selected_loaded_for.clear();
                                }
                                Err(status) => {
                                    self.apply_status_update(settings_status(status));
                                }
                            }
                        }
                    }
                }

                if Self::retro_full_width_button(right, "Toggle Admin").clicked() {
                    if !current_only {
                        let username = self.settings.user_selected.clone();
                        match toggle_desktop_user_admin(&username) {
                            Ok(status) => {
                                self.invalidate_user_cache();
                                self.apply_status_update(settings_status(status));
                                self.settings.user_selected_loaded_for.clear();
                            }
                            Err(status) => {
                                self.apply_status_update(settings_status(status));
                            }
                        }
                    }
                }
                right.add_space(8.0);

                if !current_only {
                    let can_delete = current_username
                        .as_ref()
                        .is_none_or(|name| name != &self.settings.user_selected);
                    let delete_user = if can_delete {
                        right.button("Delete User")
                    } else {
                        Self::retro_disabled_button(right, "Delete User")
                    };
                    if delete_user.clicked() {
                        if self.settings.user_delete_confirm == self.settings.user_selected {
                            let username = self.settings.user_selected.clone();
                            match delete_desktop_user(&username) {
                                Ok(status) => {
                                    self.invalidate_user_cache();
                                    self.apply_status_update(settings_status(status));
                                    self.settings.user_delete_confirm.clear();
                                    self.settings.user_selected_loaded_for.clear();
                                }
                                Err(status) => {
                                    self.apply_status_update(settings_status(status));
                                }
                            }
                        } else {
                            self.settings.user_delete_confirm = self.settings.user_selected.clone();
                            self.apply_status_update(settings_status(
                                "Click Delete User again to confirm.",
                            ));
                        }
                    }
                    if current_username
                        .as_ref()
                        .is_some_and(|name| name == &self.settings.user_selected)
                    {
                        right.small("You cannot delete the current user.");
                    }
                }
            });
        });
    }
}
