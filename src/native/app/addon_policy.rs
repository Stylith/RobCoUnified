use super::super::desktop_launcher_service::{catalog_names, ProgramCatalog};
use crate::native::installed_hosted_application_names;
use super::launch_registry::{
    about_launch_target, connections_launch_target, default_apps_launch_target,
    desktop_launch_target_available_for_profile, edit_menus_launch_target, editor_launch_target,
    file_manager_launch_target,
    terminal_launch_target_available_for_profile,
};
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

    pub(super) fn visible_application_builtins(&self) -> (bool, bool) {
        let profile = crate::config::install_profile();
        let show_file_manager =
            desktop_launch_target_available_for_profile(&file_manager_launch_target(), profile);
        let show_text_editor = self.settings.draft.builtin_menu_visibility.text_editor
            && desktop_launch_target_available_for_profile(&editor_launch_target(), profile);
        (show_file_manager, show_text_editor)
    }

    pub(super) fn terminal_settings_visibility(&self) -> TerminalSettingsVisibility {
        let profile = crate::config::install_profile();
        TerminalSettingsVisibility {
            default_apps: terminal_launch_target_available_for_profile(
                &default_apps_launch_target(),
                profile,
            ),
            connections: terminal_launch_target_available_for_profile(
                &connections_launch_target(),
                profile,
            ),
            edit_menus: terminal_launch_target_available_for_profile(
                &edit_menus_launch_target(),
                profile,
            ),
            about: terminal_launch_target_available_for_profile(&about_launch_target(), profile),
        }
    }

    pub(super) fn desktop_settings_visibility(&self) -> DesktopSettingsVisibility {
        let profile = crate::config::install_profile();
        DesktopSettingsVisibility {
            default_apps: desktop_launch_target_available_for_profile(
                &default_apps_launch_target(),
                profile,
            ),
            connections: desktop_launch_target_available_for_profile(
                &connections_launch_target(),
                profile,
            ),
            edit_menus: desktop_launch_target_available_for_profile(
                &edit_menus_launch_target(),
                profile,
            ),
            about: desktop_launch_target_available_for_profile(&about_launch_target(), profile),
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
        let (show_file_manager, show_text_editor) =
            self.visible_application_builtins();
        let needs_rebuild = self
            .desktop_applications_sections_cache
            .as_ref()
            .is_none_or(|cache| {
                cache.show_file_manager != show_file_manager
                    || cache.show_text_editor != show_text_editor
            });
        if needs_rebuild {
            let mut configured_names = catalog_names(ProgramCatalog::Applications);
            for name in installed_hosted_application_names() {
                if !configured_names.iter().any(|existing| existing == &name) {
                    configured_names.push(name);
                }
            }
            configured_names.sort();
            let sections = Arc::new(build_desktop_applications_sections(
                show_file_manager,
                show_text_editor,
                &configured_names,
                BUILTIN_TEXT_EDITOR_APP,
            ));
            self.desktop_applications_sections_cache = Some(DesktopApplicationsSectionsCache {
                show_file_manager,
                show_text_editor,
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
