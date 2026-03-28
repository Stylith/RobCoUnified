use super::super::desktop_documents_service::{
    add_document_category as add_desktop_document_category,
    delete_document_category as delete_desktop_document_category,
    rename_document_category as rename_desktop_document_category,
};
use super::super::desktop_launcher_service::{
    add_catalog_entry, catalog_names, delete_catalog_entry, parse_catalog_command_line,
    rename_catalog_entry, ProgramCatalog,
};
use super::super::edit_menus_screen::EditMenuTarget;
use super::NucleonNativeApp;

impl NucleonNativeApp {
    pub(super) fn edit_program_entries(&self, target: EditMenuTarget) -> Vec<String> {
        match target {
            EditMenuTarget::Applications => catalog_names(ProgramCatalog::Applications),
            EditMenuTarget::Documents => {
                super::super::desktop_documents_service::document_category_names()
            }
            EditMenuTarget::Network => catalog_names(ProgramCatalog::Network),
            EditMenuTarget::Games => catalog_names(ProgramCatalog::Games),
        }
    }

    pub(super) fn program_catalog_for_edit_target(
        target: EditMenuTarget,
    ) -> Option<ProgramCatalog> {
        match target {
            EditMenuTarget::Applications => Some(ProgramCatalog::Applications),
            EditMenuTarget::Network => Some(ProgramCatalog::Network),
            EditMenuTarget::Games => Some(ProgramCatalog::Games),
            EditMenuTarget::Documents => None,
        }
    }

    pub(super) fn add_program_entry(
        &mut self,
        target: EditMenuTarget,
        name: String,
        command: String,
    ) {
        let Ok(argv) = parse_catalog_command_line(command.trim()) else {
            self.shell_status = "Error: invalid command line".to_string();
            return;
        };
        match target {
            EditMenuTarget::Documents => {
                self.shell_status = "Error: invalid target for command entry.".to_string();
                return;
            }
            other => {
                let Some(catalog) = Self::program_catalog_for_edit_target(other) else {
                    self.shell_status = "Error: invalid target for command entry.".to_string();
                    return;
                };
                self.shell_status = add_catalog_entry(catalog, name, argv);
                self.invalidate_program_catalog_cache();
                self.invalidate_edit_menu_entries_cache(other);
            }
        }
    }

    pub(super) fn delete_program_entry(&mut self, target: EditMenuTarget, name: &str) {
        match target {
            EditMenuTarget::Documents => {
                self.delete_document_category(name);
                return;
            }
            other => {
                let Some(catalog) = Self::program_catalog_for_edit_target(other) else {
                    return;
                };
                self.shell_status = delete_catalog_entry(catalog, name);
                self.invalidate_program_catalog_cache();
                self.invalidate_edit_menu_entries_cache(other);
            }
        }
    }

    pub(super) fn rename_program_entry(
        &mut self,
        target: EditMenuTarget,
        old_name: &str,
        new_name: &str,
    ) {
        let new_name = new_name.trim();
        if new_name.is_empty() {
            self.shell_status = "Name cannot be empty.".to_string();
            return;
        }
        if new_name == old_name {
            self.shell_status = "Name unchanged.".to_string();
            return;
        }

        match target {
            EditMenuTarget::Documents => match rename_desktop_document_category(old_name, new_name)
            {
                Ok(status) => {
                    self.shell_status = status;
                    self.invalidate_edit_menu_entries_cache(EditMenuTarget::Documents);
                }
                Err(err) => self.shell_status = err,
            },
            other => {
                let Some(catalog) = Self::program_catalog_for_edit_target(other) else {
                    return;
                };
                match rename_catalog_entry(catalog, old_name, new_name) {
                    Ok(status) => {
                        self.shell_status = status;
                        self.invalidate_program_catalog_cache();
                        self.invalidate_edit_menu_entries_cache(other);
                    }
                    Err(err) => self.shell_status = err,
                }
            }
        }
    }

    pub(super) fn add_document_category(&mut self, name: String, path_raw: String) {
        match add_desktop_document_category(name, &path_raw) {
            Ok(status) => {
                self.shell_status = status;
                self.invalidate_edit_menu_entries_cache(EditMenuTarget::Documents);
            }
            Err(err) => self.shell_status = err,
        }
    }

    pub(super) fn delete_document_category(&mut self, name: &str) {
        self.shell_status = delete_desktop_document_category(name);
        self.invalidate_edit_menu_entries_cache(EditMenuTarget::Documents);
    }
}
