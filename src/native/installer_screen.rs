use super::menu::draw_terminal_menu_screen;
use crate::config::get_current_user;
pub use robcos_native_installer_app::{
    add_package_to_menu, apply_filter, apply_search_query, available_runtime_tools,
    build_package_command, runtime_tool_action_for_selection, runtime_tool_actions,
    runtime_tool_description, runtime_tool_menu_label, runtime_tool_pkg, runtime_tool_title,
    settle_view_after_package_command, DesktopInstallerConfirm, DesktopInstallerEvent,
    DesktopInstallerNotice, DesktopInstallerState, DesktopInstallerView, InstallerCategory,
    InstallerEvent, InstallerMenuTarget, InstallerPackageAction, InstallerView, RuntimeTool,
    TerminalInstallerState,
};
#[cfg(test)]
use robcos_native_installer_app::{PackageManager, SearchResult};

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
) -> InstallerEvent {
    let pm_label = state.pm_label().to_string();
    let mut items = vec![
        "Search".to_string(),
        "Installed Apps".to_string(),
        "Runtime Tools".to_string(),
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
        Some(3) if state.available_pms.len() > 1 => {
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
) -> InstallerEvent {
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
