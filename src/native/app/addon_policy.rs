use super::super::desktop_launcher_service::{catalog_names, ProgramCatalog};
use super::*;

impl RobcoNativeApp {
    pub(super) fn settings_home_rows_for_session(
        &mut self,
        is_admin: bool,
    ) -> Arc<Vec<Vec<SettingsHomeTile>>> {
        let visibility = self.desktop_settings_visibility();
        let cache = if is_admin {
            &mut self.settings_home_rows_cache_admin
        } else {
            &mut self.settings_home_rows_cache_standard
        };
        let needs_rebuild = cache
            .as_ref()
            .is_none_or(|cache| cache.visibility != visibility);
        if needs_rebuild {
            *cache = Some(SettingsHomeRowsCache {
                visibility,
                rows: Arc::new(desktop_settings_home_rows_with_visibility(
                    is_admin, visibility,
                )),
            });
        }
        cache
            .as_ref()
            .expect("settings home rows cache initialized")
            .rows
            .clone()
    }

    pub(super) fn visible_application_builtins(&self) -> (bool, bool, bool) {
        let profile = crate::config::install_profile();
        let show_file_manager = first_party_capability_enabled_str(profile, "file-browser");
        let show_text_editor = self.settings.draft.builtin_menu_visibility.text_editor
            && first_party_capability_enabled_str(profile, "text-editor");
        let show_nuke_codes = self.settings.draft.builtin_menu_visibility.nuke_codes
            && first_party_capability_enabled_str(profile, "code-reference");
        (show_file_manager, show_text_editor, show_nuke_codes)
    }

    pub(super) fn terminal_settings_visibility(&self) -> TerminalSettingsVisibility {
        let profile = crate::config::install_profile();
        TerminalSettingsVisibility {
            default_apps: first_party_capability_enabled_str(profile, "default-apps-ui"),
            connections: first_party_capability_enabled_str(profile, "connections-ui"),
            edit_menus: first_party_capability_enabled_str(profile, "edit-menus-ui"),
            about: first_party_capability_enabled_str(profile, "about-ui"),
        }
    }

    pub(super) fn desktop_settings_visibility(&self) -> DesktopSettingsVisibility {
        let profile = crate::config::install_profile();
        DesktopSettingsVisibility {
            default_apps: first_party_capability_enabled_str(profile, "default-apps-ui"),
            connections: first_party_capability_enabled_str(profile, "connections-ui"),
            edit_menus: first_party_capability_enabled_str(profile, "edit-menus-ui"),
            about: first_party_capability_enabled_str(profile, "about-ui"),
        }
    }

    pub(super) fn coerce_desktop_settings_panel(
        &self,
        panel: NativeSettingsPanel,
    ) -> NativeSettingsPanel {
        Self::coerce_desktop_settings_panel_for_visibility(
            panel,
            self.desktop_settings_visibility(),
        )
    }

    pub(super) fn coerce_desktop_settings_panel_for_visibility(
        panel: NativeSettingsPanel,
        visibility: DesktopSettingsVisibility,
    ) -> NativeSettingsPanel {
        if desktop_settings_panel_enabled(panel, visibility) {
            panel
        } else {
            desktop_settings_default_panel()
        }
    }

    pub(super) fn desktop_applications_sections(&mut self) -> Arc<DesktopApplicationsSections> {
        let (show_file_manager, show_text_editor, show_nuke_codes) =
            self.visible_application_builtins();
        let needs_rebuild = self
            .desktop_applications_sections_cache
            .as_ref()
            .is_none_or(|cache| {
                cache.show_file_manager != show_file_manager
                    || cache.show_text_editor != show_text_editor
                    || cache.show_nuke_codes != show_nuke_codes
            });
        if needs_rebuild {
            let configured_names = catalog_names(ProgramCatalog::Applications);
            let sections = Arc::new(build_desktop_applications_sections(
                show_file_manager,
                show_text_editor,
                show_nuke_codes,
                &configured_names,
                BUILTIN_TEXT_EDITOR_APP,
                BUILTIN_NUKE_CODES_APP,
            ));
            self.desktop_applications_sections_cache = Some(DesktopApplicationsSectionsCache {
                show_file_manager,
                show_text_editor,
                show_nuke_codes,
                sections,
            });
        }
        self.desktop_applications_sections_cache
            .as_ref()
            .expect("desktop applications sections cache initialized")
            .sections
            .clone()
    }
}
