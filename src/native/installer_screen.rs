use super::menu::draw_terminal_menu_screen;
use crate::config::get_current_user;
use crate::native::{
    installed_addon_inventory, installed_addon_inventory_sections, remove_installed_addon,
    repository_sync_action_for_manifest, set_addon_enabled_override,
    InstalledAddonInventorySections, InstalledAddonRecord, RepositoryAddonRecord,
};
pub use nucleon_native_installer_app::{
    add_package_to_menu, apply_filter, apply_search_query, available_runtime_tools,
    build_package_command, runtime_tool_action_for_selection, runtime_tool_actions,
    runtime_tool_description, runtime_tool_menu_label, runtime_tool_pkg, runtime_tool_title,
    settle_view_after_package_command, DesktopInstallerConfirm, DesktopInstallerEvent,
    DesktopInstallerNotice, DesktopInstallerState, DesktopInstallerView, InstallerCategory,
    InstallerEvent, InstallerMenuTarget, InstallerPackageAction, InstallerView, RuntimeTool,
    TerminalInstallerState,
};
#[cfg(test)]
use nucleon_native_installer_app::{PackageManager, SearchResult};

fn installer_page_size(menu_start_row: usize, status_row: usize) -> usize {
    status_row
        .saturating_sub(menu_start_row)
        // Keep room for separators/navigation rows so "Back" never collides
        // with the shell-status line at the bottom.
        .saturating_sub(6)
        .max(6)
}

fn is_admin(username: String) -> bool {
    crate::core::auth::load_users()
        .get(&username)
        .map(|u| u.is_admin)
        .unwrap_or(false)
}

pub fn cached_package_description(state: &DesktopInstallerState, pkg: &str) -> Option<String> {
    state.cached_package_description(pkg)
}

pub fn runtime_tool_installed_cached(state: &mut DesktopInstallerState, tool: RuntimeTool) -> bool {
    state.runtime_tool_installed_cached(tool)
}

#[allow(clippy::too_many_arguments)]
pub fn draw_installer_screen(
    ctx: &eframe::egui::Context,
    state: &mut TerminalInstallerState,
    shell_status: &str,
    cols: usize,
    rows: usize,
    header_start_row: usize,
    separator_top_row: usize,
    title_row: usize,
    separator_bottom_row: usize,
    subtitle_row: usize,
    menu_start_row: usize,
    status_row: usize,
    content_col: usize,
    header_lines: &[String],
) -> InstallerEvent {
    if !is_admin(get_current_user().unwrap_or_default()) {
        return InstallerEvent::Status("Access denied. Admin only.".to_string());
    }

    match state.view.clone() {
        InstallerView::Root => draw_root(
            ctx,
            state,
            shell_status,
            cols,
            rows,
            header_start_row,
            separator_top_row,
            title_row,
            separator_bottom_row,
            subtitle_row,
            menu_start_row,
            status_row,
            content_col,
            header_lines,
        ),
        InstallerView::PackageManagerSelect => draw_package_manager_select(
            ctx,
            state,
            shell_status,
            cols,
            rows,
            header_start_row,
            separator_top_row,
            title_row,
            separator_bottom_row,
            subtitle_row,
            menu_start_row,
            status_row,
            content_col,
            header_lines,
        ),
        InstallerView::SearchResults => draw_search_results(
            ctx,
            state,
            shell_status,
            cols,
            rows,
            header_start_row,
            separator_top_row,
            title_row,
            separator_bottom_row,
            subtitle_row,
            menu_start_row,
            status_row,
            content_col,
            header_lines,
        ),
        InstallerView::RuntimeTools => draw_runtime_tools(
            ctx,
            state,
            shell_status,
            cols,
            rows,
            header_start_row,
            separator_top_row,
            title_row,
            separator_bottom_row,
            subtitle_row,
            menu_start_row,
            status_row,
            content_col,
            header_lines,
        ),
        InstallerView::AddonInventory => draw_addon_inventory(
            ctx,
            state,
            shell_status,
            cols,
            rows,
            header_start_row,
            separator_top_row,
            title_row,
            separator_bottom_row,
            subtitle_row,
            menu_start_row,
            status_row,
            content_col,
            header_lines,
        ),
        InstallerView::AddonActions { addon_id } => draw_addon_actions(
            ctx,
            state,
            &addon_id,
            shell_status,
            cols,
            rows,
            header_start_row,
            separator_top_row,
            title_row,
            separator_bottom_row,
            subtitle_row,
            menu_start_row,
            status_row,
            content_col,
            header_lines,
        ),
        InstallerView::RuntimeToolActions { tool } => draw_runtime_tool_actions(
            ctx,
            state,
            tool,
            shell_status,
            cols,
            rows,
            header_start_row,
            separator_top_row,
            title_row,
            separator_bottom_row,
            subtitle_row,
            menu_start_row,
            status_row,
            content_col,
            header_lines,
        ),
        InstallerView::Installed => draw_installed(
            ctx,
            state,
            shell_status,
            cols,
            rows,
            header_start_row,
            separator_top_row,
            title_row,
            separator_bottom_row,
            subtitle_row,
            menu_start_row,
            status_row,
            content_col,
            header_lines,
        ),
        InstallerView::SearchActions { pkg } => draw_search_actions(
            ctx,
            state,
            &pkg,
            shell_status,
            cols,
            rows,
            header_start_row,
            separator_top_row,
            title_row,
            separator_bottom_row,
            subtitle_row,
            menu_start_row,
            status_row,
            content_col,
            header_lines,
        ),
        InstallerView::InstalledActions { pkg } => draw_installed_actions(
            ctx,
            state,
            &pkg,
            shell_status,
            cols,
            rows,
            header_start_row,
            separator_top_row,
            title_row,
            separator_bottom_row,
            subtitle_row,
            menu_start_row,
            status_row,
            content_col,
            header_lines,
        ),
        InstallerView::AddToMenu { pkg } => draw_add_to_menu(
            ctx,
            state,
            &pkg,
            shell_status,
            cols,
            rows,
            header_start_row,
            separator_top_row,
            title_row,
            separator_bottom_row,
            subtitle_row,
            menu_start_row,
            status_row,
            content_col,
            header_lines,
        ),
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_root(
    ctx: &eframe::egui::Context,
    state: &mut TerminalInstallerState,
    shell_status: &str,
    cols: usize,
    rows: usize,
    header_start_row: usize,
    separator_top_row: usize,
    title_row: usize,
    separator_bottom_row: usize,
    subtitle_row: usize,
    menu_start_row: usize,
    status_row: usize,
    content_col: usize,
    header_lines: &[String],
) -> InstallerEvent {
    state.ensure_available_pms();
    let pm_label = state.pm_label().to_string();
    let mut items = vec![
        "Search".to_string(),
        "Installed Apps".to_string(),
        "Runtime Tools".to_string(),
        "Installed Addons".to_string(),
    ];
    if state.available_pms.len() > 1 {
        items.push("Package Manager".to_string());
    }
    items.push("---".to_string());
    items.push("Back".to_string());
    let activated = draw_terminal_menu_screen(
        ctx,
        "Program Installer",
        Some(&format!("Package Manager: {pm_label}")),
        &items,
        &mut state.root_idx,
        cols,
        rows,
        header_start_row,
        separator_top_row,
        title_row,
        separator_bottom_row,
        subtitle_row,
        menu_start_row,
        status_row,
        content_col,
        shell_status,
        header_lines,
    );
    match activated {
        Some(0) => InstallerEvent::OpenSearchPrompt,
        Some(1) => {
            state.installed_packages = state
                .selected_pm()
                .map(|p| p.list_installed())
                .unwrap_or_default();
            state.installed_idx = 0;
            state.installed_page = 0;
            state.view = InstallerView::Installed;
            InstallerEvent::Status(format!(
                "Loaded {} installed package(s).",
                state.installed_packages.len()
            ))
        }
        Some(2) => {
            state.clear_runtime_tool_caches();
            state.view = InstallerView::RuntimeTools;
            state.runtime_tools_idx = 0;
            InstallerEvent::None
        }
        Some(3) => {
            state.view = InstallerView::AddonInventory;
            state.addons_idx = 0;
            InstallerEvent::None
        }
        Some(4) if state.available_pms.len() > 1 => {
            state.pm_select_idx = state.selected_pm_idx;
            state.view = InstallerView::PackageManagerSelect;
            InstallerEvent::None
        }
        Some(_) => InstallerEvent::BackToMainMenu,
        None => InstallerEvent::None,
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_package_manager_select(
    ctx: &eframe::egui::Context,
    state: &mut TerminalInstallerState,
    shell_status: &str,
    cols: usize,
    rows: usize,
    header_start_row: usize,
    separator_top_row: usize,
    title_row: usize,
    separator_bottom_row: usize,
    subtitle_row: usize,
    menu_start_row: usize,
    status_row: usize,
    content_col: usize,
    header_lines: &[String],
) -> InstallerEvent {
    state.ensure_available_pms();
    let mut items: Vec<String> = state
        .available_pms
        .iter()
        .enumerate()
        .map(|(idx, pm)| {
            if idx == state.selected_pm_idx {
                format!("[selected] {}", pm.name())
            } else {
                pm.name().to_string()
            }
        })
        .collect();
    items.push("---".to_string());
    items.push("Back".to_string());
    let subtitle = format!("Current: {}", state.pm_label());
    let activated = draw_terminal_menu_screen(
        ctx,
        "Package Manager",
        Some(&subtitle),
        &items,
        &mut state.pm_select_idx,
        cols,
        rows,
        header_start_row,
        separator_top_row,
        title_row,
        separator_bottom_row,
        subtitle_row,
        menu_start_row,
        status_row,
        content_col,
        shell_status,
        header_lines,
    );
    match activated {
        Some(idx) if idx < state.available_pms.len() => {
            let changed = state.select_package_manager(idx);
            state.view = InstallerView::Root;
            if changed {
                InstallerEvent::Status(format!("Package manager set to {}.", state.pm_label()))
            } else {
                InstallerEvent::None
            }
        }
        Some(_) => {
            state.view = InstallerView::Root;
            InstallerEvent::None
        }
        None => InstallerEvent::None,
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_addon_inventory(
    ctx: &eframe::egui::Context,
    state: &mut TerminalInstallerState,
    shell_status: &str,
    cols: usize,
    rows: usize,
    header_start_row: usize,
    separator_top_row: usize,
    title_row: usize,
    separator_bottom_row: usize,
    subtitle_row: usize,
    menu_start_row: usize,
    status_row: usize,
    content_col: usize,
    header_lines: &[String],
) -> InstallerEvent {
    #[derive(Clone)]
    enum AddonRow {
        Install,
        Essential(usize),
        Optional(usize),
        Repository(usize),
        Prev,
        Next,
        Back,
        Ignore,
    }

    let sections = installed_addon_inventory_sections();
    let repository_addon_indices = sections
        .repository_available
        .iter()
        .enumerate()
        .filter_map(|(idx, record)| {
            (record.manifest.kind != crate::platform::AddonKind::Theme).then_some(idx)
        })
        .collect::<Vec<_>>();
    let repository_theme_indices = sections
        .repository_available
        .iter()
        .enumerate()
        .filter_map(|(idx, record)| {
            (record.manifest.kind == crate::platform::AddonKind::Theme).then_some(idx)
        })
        .collect::<Vec<_>>();
    let installed_total = sections.essential.len() + sections.optional.len();
    let page_size = installer_page_size(menu_start_row, status_row);
    let total_pages = paged_addon_total(
        &sections,
        &repository_addon_indices,
        &repository_theme_indices,
    )
    .div_ceil(page_size)
    .max(1);
    state.addons_page = state.addons_page.min(total_pages.saturating_sub(1));
    let (paged_rows, total_rows, end) = paged_addon_rows(
        &sections,
        &repository_addon_indices,
        &repository_theme_indices,
        state.addons_page,
        page_size,
    );

    let mut items = Vec::new();
    let mut row_actions = Vec::new();
    items.push("Install Addon From Path".to_string());
    row_actions.push(AddonRow::Install);
    items.push("---".to_string());
    row_actions.push(AddonRow::Ignore);
    for row in paged_rows {
        match row {
            AddonDisplayRow::SectionHeader(label) => {
                items.push(format!("### {label}"));
                row_actions.push(AddonRow::Ignore);
            }
            AddonDisplayRow::Essential(idx) => {
                items.push(addon_inventory_menu_label(&sections.essential[idx]));
                row_actions.push(AddonRow::Essential(idx));
            }
            AddonDisplayRow::Optional(idx) => {
                items.push(addon_inventory_menu_label(&sections.optional[idx]));
                row_actions.push(AddonRow::Optional(idx));
            }
            AddonDisplayRow::Repository(idx) => {
                items.push(repository_addon_menu_label(
                    &sections.repository_available[idx],
                ));
                row_actions.push(AddonRow::Repository(idx));
            }
            AddonDisplayRow::RepositoryTheme(idx) => {
                items.push(repository_addon_menu_label(
                    &sections.repository_available[idx],
                ));
                row_actions.push(AddonRow::Repository(idx));
            }
        }
    }
    if !sections.issues.is_empty() {
        items.push("### Discovery Issues".to_string());
        row_actions.push(AddonRow::Ignore);
        if let Some(issue) = sections.issues.first() {
            items.push(addon_issue_menu_label(issue));
            row_actions.push(AddonRow::Ignore);
        }
        if sections.issues.len() > 1 {
            items.push(format!("... {} more issue(s)", sections.issues.len() - 1));
            row_actions.push(AddonRow::Ignore);
        }
    }
    if let Some(issue) = &sections.repository_issue {
        items.push("### Repository Feed".to_string());
        row_actions.push(AddonRow::Ignore);
        let source = sections
            .repository_source
            .as_ref()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| "unknown".to_string());
        items.push(format!("{}: {}", source, issue));
        row_actions.push(AddonRow::Ignore);
    }
    if state.addons_page > 0 {
        items.push("< Prev Page".to_string());
        row_actions.push(AddonRow::Prev);
    }
    if end < total_rows {
        items.push("> Next Page".to_string());
        row_actions.push(AddonRow::Next);
    }
    items.push("---".to_string());
    row_actions.push(AddonRow::Ignore);
    items.push("Back".to_string());
    row_actions.push(AddonRow::Back);

    let selectable_rows: Vec<usize> = items
        .iter()
        .enumerate()
        .filter_map(|(idx, _)| match row_actions.get(idx) {
            Some(AddonRow::Ignore) => None,
            _ => Some(idx),
        })
        .collect();
    let subtitle = selectable_rows
        .get(state.addons_idx)
        .copied()
        .and_then(|raw_idx| match row_actions.get(raw_idx) {
            Some(AddonRow::Install) => Some(
                "Install a manifest, addon folder, or .zip/.tar(.gz) addon archive into the user addons root."
                    .to_string(),
            ),
            Some(AddonRow::Essential(idx)) => {
                Some(addon_inventory_subtitle(&sections.essential[*idx]))
            }
            Some(AddonRow::Optional(idx)) => {
                Some(addon_inventory_subtitle(&sections.optional[*idx]))
            }
            Some(AddonRow::Repository(idx)) => {
                Some(repository_addon_subtitle(&sections.repository_available[*idx]))
            }
            _ => None,
        });
    let addon_status = format!(
        "{} installed   {} repository addon(s)   {} repository theme(s)   {} issue(s)   Page {}/{}",
        installed_total,
        repository_addon_indices.len(),
        repository_theme_indices.len(),
        sections.issues.len() + usize::from(sections.repository_issue.is_some()),
        state.addons_page + 1,
        total_pages
    );
    let status_line = if shell_status.is_empty() {
        addon_status
    } else {
        format!("{addon_status} | {shell_status}")
    };

    let activated = draw_terminal_menu_screen(
        ctx,
        "Installed Addons",
        subtitle.as_deref(),
        &items,
        &mut state.addons_idx,
        cols,
        rows,
        header_start_row,
        separator_top_row,
        title_row,
        separator_bottom_row,
        subtitle_row,
        menu_start_row,
        status_row,
        content_col,
        &status_line,
        header_lines,
    );

    match activated {
        Some(idx) => match row_actions.get(idx) {
            Some(AddonRow::Install) => InstallerEvent::OpenAddonInstallPrompt,
            Some(AddonRow::Essential(idx)) => {
                state.action_idx = 0;
                state.view = InstallerView::AddonActions {
                    addon_id: sections.essential[*idx].manifest.id.to_string(),
                };
                InstallerEvent::None
            }
            Some(AddonRow::Optional(idx)) => {
                state.action_idx = 0;
                state.view = InstallerView::AddonActions {
                    addon_id: sections.optional[*idx].manifest.id.to_string(),
                };
                InstallerEvent::None
            }
            Some(AddonRow::Repository(idx)) => InstallerEvent::StartRepositoryAddonInstall {
                addon_id: sections.repository_available[*idx].manifest.id.to_string(),
                action_label: "Install".to_string(),
            },
            Some(AddonRow::Prev) => {
                state.addons_page = state.addons_page.saturating_sub(1);
                state.addons_idx = 0;
                InstallerEvent::None
            }
            Some(AddonRow::Next) => {
                state.addons_page = (state.addons_page + 1).min(total_pages.saturating_sub(1));
                state.addons_idx = 0;
                InstallerEvent::None
            }
            Some(AddonRow::Back) => {
                state.view = InstallerView::Root;
                InstallerEvent::None
            }
            _ => InstallerEvent::None,
        },
        None => InstallerEvent::None,
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_addon_actions(
    ctx: &eframe::egui::Context,
    state: &mut TerminalInstallerState,
    addon_id: &str,
    shell_status: &str,
    cols: usize,
    rows: usize,
    header_start_row: usize,
    separator_top_row: usize,
    title_row: usize,
    separator_bottom_row: usize,
    subtitle_row: usize,
    menu_start_row: usize,
    status_row: usize,
    content_col: usize,
    header_lines: &[String],
) -> InstallerEvent {
    #[derive(Clone, Copy)]
    enum AddonAction {
        Toggle,
        Remove,
        RepositorySync(&'static str),
        Back,
        Ignore,
    }

    let Some(record) = installed_addon_inventory()
        .into_iter()
        .find(|record| record.manifest.id.as_str() == addon_id)
    else {
        state.view = InstallerView::AddonInventory;
        return InstallerEvent::Status("Addon is no longer installed.".to_string());
    };

    let mut items = Vec::new();
    let mut actions = Vec::new();
    if !record.manifest.essential {
        items.push(if record.effective_enabled {
            "Disable".to_string()
        } else {
            "Enable".to_string()
        });
        actions.push(AddonAction::Toggle);
    }
    if addon_can_be_removed(&record) {
        items.push("Remove".to_string());
        actions.push(AddonAction::Remove);
    }
    if let Ok(Some(action)) = repository_sync_action_for_manifest(&record.manifest) {
        items.push(action.label().to_string());
        actions.push(AddonAction::RepositorySync(action.label()));
    }
    items.push("---".to_string());
    actions.push(AddonAction::Ignore);
    items.push("Back".to_string());
    actions.push(AddonAction::Back);

    let subtitle = addon_inventory_subtitle(&record);
    let activated = draw_terminal_menu_screen(
        ctx,
        &record.manifest.display_name,
        Some(&subtitle),
        &items,
        &mut state.action_idx,
        cols,
        rows,
        header_start_row,
        separator_top_row,
        title_row,
        separator_bottom_row,
        subtitle_row,
        menu_start_row,
        status_row,
        content_col,
        shell_status,
        header_lines,
    );

    match activated {
        Some(idx) => match actions.get(idx) {
            Some(AddonAction::Toggle) => {
                let next_enabled = !record.effective_enabled;
                let override_value = addon_override_value(&record, next_enabled);
                match set_addon_enabled_override(record.manifest.id.clone(), override_value) {
                    Ok(()) => InstallerEvent::Status(format!(
                        "{} {}.",
                        record.manifest.display_name,
                        if next_enabled { "enabled" } else { "disabled" }
                    )),
                    Err(err) => InstallerEvent::Status(err),
                }
            }
            Some(AddonAction::Remove) => match remove_installed_addon(record.manifest.id.clone()) {
                Ok(message) => {
                    state.view = InstallerView::AddonInventory;
                    state.action_idx = 0;
                    InstallerEvent::Status(message)
                }
                Err(err) => InstallerEvent::Status(err),
            },
            Some(AddonAction::RepositorySync(label)) => {
                InstallerEvent::StartRepositoryAddonInstall {
                    addon_id: record.manifest.id.to_string(),
                    action_label: label.to_string(),
                }
            }
            Some(AddonAction::Back) => {
                state.view = InstallerView::AddonInventory;
                state.action_idx = 0;
                InstallerEvent::None
            }
            _ => InstallerEvent::None,
        },
        None => InstallerEvent::None,
    }
}

enum AddonDisplayRow {
    SectionHeader(&'static str),
    Essential(usize),
    Optional(usize),
    Repository(usize),
    RepositoryTheme(usize),
}

fn paged_addon_total(
    sections: &InstalledAddonInventorySections,
    repository_addon_indices: &[usize],
    repository_theme_indices: &[usize],
) -> usize {
    sections.essential.len()
        + sections.optional.len()
        + repository_addon_indices.len()
        + repository_theme_indices.len()
}

fn paged_addon_rows(
    sections: &InstalledAddonInventorySections,
    repository_addon_indices: &[usize],
    repository_theme_indices: &[usize],
    page: usize,
    page_size: usize,
) -> (Vec<AddonDisplayRow>, usize, usize) {
    let installed_total = sections.essential.len() + sections.optional.len();
    let repository_total = repository_addon_indices.len() + repository_theme_indices.len();
    let total = paged_addon_total(sections, repository_addon_indices, repository_theme_indices);
    let start = page * page_size;
    let end = (start + page_size).min(total);

    let essential_start = start.min(sections.essential.len());
    let essential_end = end.min(sections.essential.len());
    let optional_start = start
        .saturating_sub(sections.essential.len())
        .min(sections.optional.len());
    let optional_end = end
        .saturating_sub(sections.essential.len())
        .min(sections.optional.len());
    let repository_addon_start = start
        .saturating_sub(installed_total)
        .min(repository_addon_indices.len());
    let repository_addon_end = end
        .saturating_sub(installed_total)
        .min(repository_addon_indices.len());
    let repository_theme_start = start
        .saturating_sub(installed_total + repository_addon_indices.len())
        .min(repository_theme_indices.len());
    let repository_theme_end = end
        .saturating_sub(installed_total + repository_addon_indices.len())
        .min(repository_theme_indices.len());

    let mut visible = Vec::new();
    if essential_start < essential_end {
        visible.push(AddonDisplayRow::SectionHeader("Essential Addons"));
        visible.extend((essential_start..essential_end).map(AddonDisplayRow::Essential));
    }
    if optional_start < optional_end {
        visible.push(AddonDisplayRow::SectionHeader("Optional Addons"));
        visible.extend((optional_start..optional_end).map(AddonDisplayRow::Optional));
    }
    if repository_addon_start < repository_addon_end {
        visible.push(AddonDisplayRow::SectionHeader("Repository Addons"));
        visible.extend(
            repository_addon_indices[repository_addon_start..repository_addon_end]
                .iter()
                .copied()
                .map(AddonDisplayRow::Repository),
        );
    }
    if repository_theme_start < repository_theme_end {
        visible.push(AddonDisplayRow::SectionHeader("Repository Themes"));
        visible.extend(
            repository_theme_indices[repository_theme_start..repository_theme_end]
                .iter()
                .copied()
                .map(AddonDisplayRow::RepositoryTheme),
        );
    }

    (visible, installed_total + repository_total, end)
}

fn addon_inventory_menu_label(record: &InstalledAddonRecord) -> String {
    let state = if record.manifest.essential {
        "[required]"
    } else if record.effective_enabled {
        "[on]"
    } else {
        "[off]"
    };
    format!(
        "{state} {} ({})",
        record.manifest.display_name,
        addon_scope_label(record)
    )
}

fn addon_inventory_subtitle(record: &InstalledAddonRecord) -> String {
    let source = addon_source_label(record);
    format!(
        "{} | id={} | source={}",
        addon_enabled_label(record),
        record.manifest.id,
        source
    )
}

fn addon_source_label(record: &InstalledAddonRecord) -> String {
    record
        .manifest_path
        .as_ref()
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "static fallback manifest".to_string())
}

fn addon_issue_menu_label(issue: &crate::platform::AddonManifestLoadIssue) -> String {
    format!(
        "[{}] {}: {}",
        addon_scope_name(issue.scope),
        issue.manifest_path.display(),
        issue.detail
    )
}

fn repository_addon_menu_label(record: &RepositoryAddonRecord) -> String {
    let version = record
        .release
        .as_ref()
        .map(|release| release.version.as_str())
        .unwrap_or(record.manifest.version.as_str());
    format!("[feed] {} (v{version})", record.manifest.display_name)
}

fn repository_addon_subtitle(record: &RepositoryAddonRecord) -> String {
    let channel = record
        .release
        .as_ref()
        .and_then(|release| release.channel.as_deref())
        .unwrap_or("default");
    format!(
        "available from repository | id={} | channel={} | source={}",
        record.manifest.id,
        channel,
        record.repository_source.display()
    )
}

fn addon_scope_name(scope: crate::platform::AddonScope) -> &'static str {
    match scope {
        crate::platform::AddonScope::Bundled => "bundled",
        crate::platform::AddonScope::System => "system",
        crate::platform::AddonScope::User => "user",
    }
}

fn addon_enabled_label(record: &InstalledAddonRecord) -> &'static str {
    if record.manifest.essential {
        "required"
    } else if record.effective_enabled {
        "enabled"
    } else {
        "disabled"
    }
}

fn addon_scope_label(record: &InstalledAddonRecord) -> &'static str {
    addon_scope_name(record.manifest.scope)
}

fn addon_override_value(record: &InstalledAddonRecord, enabled: bool) -> Option<bool> {
    if enabled == record.manifest.enabled_by_default {
        None
    } else {
        Some(enabled)
    }
}

fn addon_can_be_removed(record: &InstalledAddonRecord) -> bool {
    record.manifest.scope == crate::platform::AddonScope::User && record.manifest_path.is_some()
}

#[allow(clippy::too_many_arguments)]
fn draw_runtime_tools(
    ctx: &eframe::egui::Context,
    state: &mut TerminalInstallerState,
    shell_status: &str,
    cols: usize,
    rows: usize,
    header_start_row: usize,
    separator_top_row: usize,
    title_row: usize,
    separator_bottom_row: usize,
    subtitle_row: usize,
    menu_start_row: usize,
    status_row: usize,
    content_col: usize,
    header_lines: &[String],
) -> InstallerEvent {
    #[derive(Clone, Copy)]
    enum RuntimeRow {
        Tool(RuntimeTool),
        Back,
        Ignore,
    }
    let tools = available_runtime_tools();
    let mut items = Vec::new();
    let mut runtime_rows = Vec::new();
    for tool in tools.iter().copied() {
        items.push(runtime_tool_menu_label(
            tool,
            state.runtime_tool_installed_cached(tool),
        ));
        runtime_rows.push(RuntimeRow::Tool(tool));
    }
    items.push("---".to_string());
    runtime_rows.push(RuntimeRow::Ignore);
    items.push("Back".to_string());
    runtime_rows.push(RuntimeRow::Back);
    let subtitle = runtime_rows
        .get(state.runtime_tools_idx)
        .and_then(|row| match row {
            RuntimeRow::Tool(tool) => Some(runtime_tool_description(*tool)),
            _ => None,
        })
        .unwrap_or("Choose a runtime tool");
    let activated = draw_terminal_menu_screen(
        ctx,
        "Runtime Tools",
        Some(subtitle),
        &items,
        &mut state.runtime_tools_idx,
        cols,
        rows,
        header_start_row,
        separator_top_row,
        title_row,
        separator_bottom_row,
        subtitle_row,
        menu_start_row,
        status_row,
        content_col,
        shell_status,
        header_lines,
    );
    match activated {
        Some(idx) => match runtime_rows.get(idx) {
            Some(RuntimeRow::Tool(tool)) => {
                state.action_idx = 0;
                state.view = InstallerView::RuntimeToolActions { tool: *tool };
                InstallerEvent::None
            }
            Some(RuntimeRow::Back) => {
                state.view = InstallerView::Root;
                InstallerEvent::None
            }
            _ => InstallerEvent::None,
        },
        None => InstallerEvent::None,
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_runtime_tool_actions(
    ctx: &eframe::egui::Context,
    state: &mut TerminalInstallerState,
    tool: RuntimeTool,
    shell_status: &str,
    cols: usize,
    rows: usize,
    header_start_row: usize,
    separator_top_row: usize,
    title_row: usize,
    separator_bottom_row: usize,
    subtitle_row: usize,
    menu_start_row: usize,
    status_row: usize,
    content_col: usize,
    header_lines: &[String],
) -> InstallerEvent {
    let installed = state.runtime_tool_installed_cached(tool);
    let mut items: Vec<String> = runtime_tool_actions(installed)
        .iter()
        .map(|action| match action {
            InstallerPackageAction::Install => "Install".to_string(),
            InstallerPackageAction::Update => "Update".to_string(),
            InstallerPackageAction::Reinstall => "Reinstall".to_string(),
            InstallerPackageAction::Uninstall => "Uninstall".to_string(),
        })
        .collect();
    items.push("---".to_string());
    items.push("Back".to_string());
    let subtitle = format!(
        "{} | {}",
        runtime_tool_description(tool),
        if installed {
            "Installed"
        } else {
            "Not installed"
        }
    );
    let activated = draw_terminal_menu_screen(
        ctx,
        runtime_tool_title(tool),
        Some(&subtitle),
        &items,
        &mut state.action_idx,
        cols,
        rows,
        header_start_row,
        separator_top_row,
        title_row,
        separator_bottom_row,
        subtitle_row,
        menu_start_row,
        status_row,
        content_col,
        shell_status,
        header_lines,
    );
    let pkg = runtime_tool_pkg(tool).to_string();
    match activated {
        Some(idx) if runtime_tool_action_for_selection(installed, idx).is_some() => {
            InstallerEvent::OpenConfirmAction {
                pkg,
                action: runtime_tool_action_for_selection(installed, idx)
                    .expect("validated runtime tool action index"),
            }
        }
        Some(_) => {
            state.view = InstallerView::RuntimeTools;
            InstallerEvent::None
        }
        None => InstallerEvent::None,
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_search_results(
    ctx: &eframe::egui::Context,
    state: &mut TerminalInstallerState,
    shell_status: &str,
    cols: usize,
    rows: usize,
    header_start_row: usize,
    separator_top_row: usize,
    title_row: usize,
    separator_bottom_row: usize,
    subtitle_row: usize,
    menu_start_row: usize,
    status_row: usize,
    content_col: usize,
    header_lines: &[String],
) -> InstallerEvent {
    #[derive(Clone)]
    enum SearchRow {
        Package(usize),
        Prev,
        Next,
        Back,
        Ignore,
    }
    let total = state.search_results.len();
    let page_size = installer_page_size(menu_start_row, status_row);
    let total_pages = total.div_ceil(page_size).max(1);
    state.search_page = state.search_page.min(total_pages.saturating_sub(1));
    let start = state.search_page * page_size;
    let end = (start + page_size).min(total);
    let mut items: Vec<String> = Vec::new();
    let mut row_actions: Vec<SearchRow> = Vec::new();
    for idx in start..end {
        let result = &state.search_results[idx];
        items.push(format!(
            "{} {}",
            if result.installed {
                "[installed]"
            } else {
                "[get]"
            },
            result.raw
        ));
        row_actions.push(SearchRow::Package(idx));
    }
    if state.search_page > 0 {
        items.push("< Prev Page".to_string());
        row_actions.push(SearchRow::Prev);
    }
    if end < total {
        items.push("> Next Page".to_string());
        row_actions.push(SearchRow::Next);
    }
    items.push("---".to_string());
    row_actions.push(SearchRow::Ignore);
    items.push("Back".to_string());
    row_actions.push(SearchRow::Back);
    let subtitle = format!(
        "Query: {}  Page {}/{}",
        state.search_query,
        state.search_page + 1,
        total_pages
    );
    let activated = draw_terminal_menu_screen(
        ctx,
        "Search Results",
        Some(&subtitle),
        &items,
        &mut state.search_idx,
        cols,
        rows,
        header_start_row,
        separator_top_row,
        title_row,
        separator_bottom_row,
        subtitle_row,
        menu_start_row,
        status_row,
        content_col,
        shell_status,
        header_lines,
    );
    match activated {
        Some(idx) => match row_actions.get(idx) {
            Some(SearchRow::Package(pkg_idx)) => {
                let pkg = state.search_results[*pkg_idx].pkg.clone();
                state.action_idx = 0;
                state.view = InstallerView::SearchActions { pkg };
                InstallerEvent::None
            }
            Some(SearchRow::Prev) => {
                state.search_page = state.search_page.saturating_sub(1);
                state.search_idx = 0;
                InstallerEvent::None
            }
            Some(SearchRow::Next) => {
                state.search_page = (state.search_page + 1).min(total_pages.saturating_sub(1));
                state.search_idx = 0;
                InstallerEvent::None
            }
            Some(SearchRow::Back) => {
                state.view = InstallerView::Root;
                InstallerEvent::None
            }
            _ => InstallerEvent::None,
        },
        None => InstallerEvent::None,
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_installed(
    ctx: &eframe::egui::Context,
    state: &mut TerminalInstallerState,
    shell_status: &str,
    cols: usize,
    rows: usize,
    header_start_row: usize,
    separator_top_row: usize,
    title_row: usize,
    separator_bottom_row: usize,
    subtitle_row: usize,
    menu_start_row: usize,
    status_row: usize,
    content_col: usize,
    header_lines: &[String],
) -> InstallerEvent {
    #[derive(Clone)]
    enum InstalledRow {
        Filter,
        Package(String),
        Prev,
        Next,
        Back,
        Ignore,
    }
    let filter_label = if state.installed_filter.is_empty() {
        "Filter...".to_string()
    } else {
        format!("Filter: {}", state.installed_filter)
    };
    let filtered: Vec<String> = state
        .installed_packages
        .iter()
        .filter(|p| {
            state.installed_filter.is_empty()
                || p.to_lowercase()
                    .contains(&state.installed_filter.to_lowercase())
        })
        .cloned()
        .collect();
    let total = filtered.len();
    let page_size = installer_page_size(menu_start_row, status_row);
    let total_pages = total.div_ceil(page_size).max(1);
    state.installed_page = state.installed_page.min(total_pages.saturating_sub(1));
    let start = state.installed_page * page_size;
    let end = (start + page_size).min(total);

    let mut items = vec![filter_label.clone(), "---".to_string()];
    let mut row_actions = vec![InstalledRow::Filter, InstalledRow::Ignore];
    for pkg in &filtered[start..end] {
        items.push(pkg.clone());
        row_actions.push(InstalledRow::Package(pkg.clone()));
    }
    if state.installed_page > 0 {
        items.push("< Prev Page".to_string());
        row_actions.push(InstalledRow::Prev);
    }
    if end < total {
        items.push("> Next Page".to_string());
        row_actions.push(InstalledRow::Next);
    }
    items.push("---".to_string());
    row_actions.push(InstalledRow::Ignore);
    items.push("Back".to_string());
    row_actions.push(InstalledRow::Back);
    let selectable_rows: Vec<usize> = items
        .iter()
        .enumerate()
        .filter_map(|(idx, item)| if item == "---" { None } else { Some(idx) })
        .collect();
    let subtitle = selectable_rows
        .get(state.installed_idx)
        .copied()
        .and_then(|raw_idx| match row_actions.get(raw_idx) {
            Some(InstalledRow::Package(pkg)) => state.cached_package_description(pkg),
            _ => None,
        });
    let installed_status = format!(
        "{} packages installed   Page {}/{}",
        total,
        state.installed_page + 1,
        total_pages
    );
    let status_line = if shell_status.is_empty() {
        installed_status
    } else {
        format!("{installed_status} | {shell_status}")
    };
    let activated = draw_terminal_menu_screen(
        ctx,
        "Installed Apps",
        subtitle.as_deref(),
        &items,
        &mut state.installed_idx,
        cols,
        rows,
        header_start_row,
        separator_top_row,
        title_row,
        separator_bottom_row,
        subtitle_row,
        menu_start_row,
        status_row,
        content_col,
        &status_line,
        header_lines,
    );
    match activated {
        Some(idx) => match row_actions.get(idx) {
            Some(InstalledRow::Filter) => InstallerEvent::OpenFilterPrompt,
            Some(InstalledRow::Package(pkg)) => {
                state.action_idx = 0;
                state.view = InstallerView::InstalledActions { pkg: pkg.clone() };
                InstallerEvent::None
            }
            Some(InstalledRow::Prev) => {
                state.installed_page = state.installed_page.saturating_sub(1);
                state.installed_idx = 0;
                InstallerEvent::None
            }
            Some(InstalledRow::Next) => {
                state.installed_page =
                    (state.installed_page + 1).min(total_pages.saturating_sub(1));
                state.installed_idx = 0;
                InstallerEvent::None
            }
            Some(InstalledRow::Back) => {
                state.view = InstallerView::Root;
                InstallerEvent::None
            }
            _ => InstallerEvent::None,
        },
        None => InstallerEvent::None,
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_search_actions(
    ctx: &eframe::egui::Context,
    state: &mut TerminalInstallerState,
    pkg: &str,
    shell_status: &str,
    cols: usize,
    rows: usize,
    header_start_row: usize,
    separator_top_row: usize,
    title_row: usize,
    separator_bottom_row: usize,
    subtitle_row: usize,
    menu_start_row: usize,
    status_row: usize,
    content_col: usize,
    header_lines: &[String],
) -> InstallerEvent {
    let items = vec!["Install".to_string(), "---".to_string(), "Back".to_string()];
    let subtitle = state
        .package_description(pkg)
        .unwrap_or_else(|| "Search result actions".to_string());
    let activated = draw_terminal_menu_screen(
        ctx,
        pkg,
        Some(&subtitle),
        &items,
        &mut state.action_idx,
        cols,
        rows,
        header_start_row,
        separator_top_row,
        title_row,
        separator_bottom_row,
        subtitle_row,
        menu_start_row,
        status_row,
        content_col,
        shell_status,
        header_lines,
    );
    match activated {
        Some(0) => InstallerEvent::OpenConfirmAction {
            pkg: pkg.to_string(),
            action: InstallerPackageAction::Install,
        },
        Some(_) => {
            state.view = InstallerView::SearchResults;
            InstallerEvent::None
        }
        None => InstallerEvent::None,
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_installed_actions(
    ctx: &eframe::egui::Context,
    state: &mut TerminalInstallerState,
    pkg: &str,
    shell_status: &str,
    cols: usize,
    rows: usize,
    header_start_row: usize,
    separator_top_row: usize,
    title_row: usize,
    separator_bottom_row: usize,
    subtitle_row: usize,
    menu_start_row: usize,
    status_row: usize,
    content_col: usize,
    header_lines: &[String],
) -> InstallerEvent {
    let items = vec![
        "Update".to_string(),
        "Reinstall".to_string(),
        "Uninstall".to_string(),
        "Add to Menu".to_string(),
        "---".to_string(),
        "Back".to_string(),
    ];
    let subtitle = state
        .package_description(pkg)
        .unwrap_or_else(|| "Installed package actions".to_string());
    let activated = draw_terminal_menu_screen(
        ctx,
        pkg,
        Some(&subtitle),
        &items,
        &mut state.action_idx,
        cols,
        rows,
        header_start_row,
        separator_top_row,
        title_row,
        separator_bottom_row,
        subtitle_row,
        menu_start_row,
        status_row,
        content_col,
        shell_status,
        header_lines,
    );
    match activated {
        Some(0) => InstallerEvent::OpenConfirmAction {
            pkg: pkg.to_string(),
            action: InstallerPackageAction::Update,
        },
        Some(1) => InstallerEvent::OpenConfirmAction {
            pkg: pkg.to_string(),
            action: InstallerPackageAction::Reinstall,
        },
        Some(2) => InstallerEvent::OpenConfirmAction {
            pkg: pkg.to_string(),
            action: InstallerPackageAction::Uninstall,
        },
        Some(3) => {
            state.add_menu_idx = 0;
            state.view = InstallerView::AddToMenu {
                pkg: pkg.to_string(),
            };
            InstallerEvent::None
        }
        Some(_) => {
            state.view = InstallerView::Installed;
            InstallerEvent::None
        }
        None => InstallerEvent::None,
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_add_to_menu(
    ctx: &eframe::egui::Context,
    state: &mut TerminalInstallerState,
    pkg: &str,
    shell_status: &str,
    cols: usize,
    rows: usize,
    header_start_row: usize,
    separator_top_row: usize,
    title_row: usize,
    separator_bottom_row: usize,
    subtitle_row: usize,
    menu_start_row: usize,
    status_row: usize,
    content_col: usize,
    header_lines: &[String],
) -> InstallerEvent {
    let items = vec![
        "Applications".to_string(),
        "Games".to_string(),
        "Network".to_string(),
        "---".to_string(),
        "Back".to_string(),
    ];
    let activated = draw_terminal_menu_screen(
        ctx,
        "Add to Menu",
        Some(pkg),
        &items,
        &mut state.add_menu_idx,
        cols,
        rows,
        header_start_row,
        separator_top_row,
        title_row,
        separator_bottom_row,
        subtitle_row,
        menu_start_row,
        status_row,
        content_col,
        shell_status,
        header_lines,
    );
    match activated {
        Some(0) => InstallerEvent::OpenDisplayNamePrompt {
            pkg: pkg.to_string(),
            target: InstallerMenuTarget::Applications,
        },
        Some(1) => InstallerEvent::OpenDisplayNamePrompt {
            pkg: pkg.to_string(),
            target: InstallerMenuTarget::Games,
        },
        Some(2) => InstallerEvent::OpenDisplayNamePrompt {
            pkg: pkg.to_string(),
            target: InstallerMenuTarget::Network,
        },
        Some(_) => {
            state.view = InstallerView::InstalledActions {
                pkg: pkg.to_string(),
            };
            InstallerEvent::None
        }
        None => InstallerEvent::None,
    }
}

// ─── Desktop Installer GUI ──────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn back_from_search_actions_returns_to_search_results() {
        let mut state = TerminalInstallerState {
            view: InstallerView::SearchActions {
                pkg: "pkg".to_string(),
            },
            ..Default::default()
        };
        assert!(!state.back());
        assert!(matches!(state.view, InstallerView::SearchResults));
    }

    #[test]
    fn empty_search_reports_cancelled() {
        let mut state = TerminalInstallerState::default();
        let event = apply_search_query(&mut state, "   ");
        assert!(matches!(
            event,
            InstallerEvent::Status(ref s) if s == "Search cancelled."
        ));
    }

    #[test]
    fn back_from_pm_selection_returns_to_root() {
        let mut state = TerminalInstallerState {
            view: InstallerView::PackageManagerSelect,
            ..Default::default()
        };
        assert!(!state.back());
        assert!(matches!(state.view, InstallerView::Root));
    }

    #[test]
    fn changing_pm_clears_terminal_installer_results() {
        let mut state = TerminalInstallerState {
            available_pms: vec![PackageManager::Brew, PackageManager::Apt],
            selected_pm_idx: 0,
            pm_select_idx: 0,
            search_query: "ripgrep".to_string(),
            search_results: vec![SearchResult {
                raw: "ripgrep".to_string(),
                pkg: "ripgrep".to_string(),
                description: Some("fast grep".to_string()),
                installed: false,
            }],
            installed_packages: vec!["fd".to_string()],
            installed_filter: "fd".to_string(),
            ..Default::default()
        };

        assert!(state.select_package_manager(1));
        assert_eq!(state.selected_pm(), Some(PackageManager::Apt));
        assert!(state.search_query.is_empty());
        assert!(state.search_results.is_empty());
        assert!(state.installed_packages.is_empty());
        assert!(state.installed_filter.is_empty());
    }
}
