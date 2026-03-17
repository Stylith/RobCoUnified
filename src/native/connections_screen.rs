use super::desktop_connections_service::{
    bluetooth_disconnect_targets, bluetooth_installer_status_hint, connection_kind_plural_label,
    connection_network_group_label, connection_network_groups, connection_requires_password,
    connections_macos_blueutil_missing, connections_macos_disabled,
    connections_macos_disabled_hint, disconnect_connection_status, discovered_connection_label,
    discovered_connections, filter_discovered_connection_list,
    filter_network_group_discovered_connections, filter_network_group_saved_connections,
    forget_saved_connection_and_refresh_settings, saved_connection_label,
    saved_connections_for_kind,
};
use super::menu::draw_terminal_menu_screen;
use crate::config::ConnectionKind;
use crate::connections::{DiscoveredConnection, NetworkMenuGroup};
use eframe::egui::Context;

#[derive(Debug, Clone)]
pub enum ConnectionsView {
    Root,
    NetworkGroups,
    Kind {
        kind: ConnectionKind,
        group: Option<NetworkMenuGroup>,
    },
    Saved {
        kind: ConnectionKind,
        group: Option<NetworkMenuGroup>,
    },
    Picker {
        kind: ConnectionKind,
        group: Option<NetworkMenuGroup>,
        title: String,
        items: Vec<DiscoveredConnection>,
        mode: PickerMode,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PickerMode {
    Connect,
    DisconnectBluetooth,
}

#[derive(Debug, Clone)]
pub struct TerminalConnectionsState {
    pub view: ConnectionsView,
    pub root_idx: usize,
    pub network_group_idx: usize,
    pub kind_idx: usize,
    pub saved_idx: usize,
    pub picker_idx: usize,
}

impl Default for TerminalConnectionsState {
    fn default() -> Self {
        Self {
            view: ConnectionsView::Root,
            root_idx: 0,
            network_group_idx: 0,
            kind_idx: 0,
            saved_idx: 0,
            picker_idx: 0,
        }
    }
}

#[derive(Debug, Clone)]
pub enum ConnectionsEvent {
    None,
    BackToSettings,
    OpenNetworkGroups,
    OpenBluetooth,
    OpenPromptSearch {
        kind: ConnectionKind,
        group: Option<NetworkMenuGroup>,
    },
    OpenPasswordPrompt {
        kind: ConnectionKind,
        target: DiscoveredConnection,
    },
    ConnectImmediate {
        kind: ConnectionKind,
        target: DiscoveredConnection,
    },
    Status(String),
}

impl TerminalConnectionsState {
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    pub fn is_at_root(&self) -> bool {
        matches!(self.view, ConnectionsView::Root)
    }

    pub fn back(&mut self) -> bool {
        match &self.view {
            ConnectionsView::Root => true,
            ConnectionsView::NetworkGroups => {
                self.view = ConnectionsView::Root;
                false
            }
            ConnectionsView::Kind { kind, group } => {
                self.view = if matches!(kind, ConnectionKind::Network) && group.is_some() {
                    ConnectionsView::NetworkGroups
                } else {
                    ConnectionsView::Root
                };
                false
            }
            ConnectionsView::Saved { kind, group }
            | ConnectionsView::Picker { kind, group, .. } => {
                self.view = ConnectionsView::Kind {
                    kind: *kind,
                    group: *group,
                };
                false
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn draw_connections_screen(
    ctx: &Context,
    state: &mut TerminalConnectionsState,
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
) -> ConnectionsEvent {
    if connections_macos_disabled() {
        return ConnectionsEvent::Status(connections_macos_disabled_hint().to_string());
    }

    match state.view.clone() {
        ConnectionsView::Root => draw_connections_root(
            ctx,
            &mut state.root_idx,
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
        ConnectionsView::NetworkGroups => draw_network_groups(
            ctx,
            &mut state.network_group_idx,
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
            &mut state.view,
        ),
        ConnectionsView::Kind { kind, group } => draw_kind_menu(
            ctx,
            kind,
            group,
            &mut state.kind_idx,
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
            &mut state.view,
        ),
        ConnectionsView::Saved { kind, group } => draw_saved_menu(
            ctx,
            kind,
            group,
            &mut state.saved_idx,
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
            &mut state.view,
        ),
        ConnectionsView::Picker {
            kind,
            group,
            title,
            items,
            mode,
        } => draw_picker(
            ctx,
            kind,
            group,
            &title,
            &items,
            mode,
            &mut state.picker_idx,
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
            &mut state.view,
        ),
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_connections_root(
    ctx: &Context,
    selected_idx: &mut usize,
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
) -> ConnectionsEvent {
    let mut items = vec!["Network".to_string()];
    let subtitle = if connections_macos_blueutil_missing() {
        Some(bluetooth_installer_status_hint())
    } else {
        items.push("Bluetooth".to_string());
        None
    };
    items.push("---".to_string());
    items.push("Back".to_string());
    let activated = draw_terminal_menu_screen(
        ctx,
        "Connections",
        subtitle,
        &items,
        selected_idx,
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
        Some(0) => ConnectionsEvent::OpenNetworkGroups,
        Some(1) if !connections_macos_blueutil_missing() => ConnectionsEvent::OpenBluetooth,
        Some(_) => ConnectionsEvent::BackToSettings,
        None => ConnectionsEvent::None,
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_network_groups(
    ctx: &Context,
    selected_idx: &mut usize,
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
    view: &mut ConnectionsView,
) -> ConnectionsEvent {
    let mut items: Vec<String> = connection_network_groups()
        .iter()
        .map(|g| format!("{} Networks", connection_network_group_label(*g)))
        .collect();
    items.push("---".to_string());
    items.push("Back".to_string());
    let activated = draw_terminal_menu_screen(
        ctx,
        "Connections - Network",
        None,
        &items,
        selected_idx,
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
    if let Some(idx) = activated {
        if idx < connection_network_groups().len() {
            *view = ConnectionsView::Kind {
                kind: ConnectionKind::Network,
                group: Some(connection_network_groups()[idx]),
            };
        } else {
            *view = ConnectionsView::Root;
        }
    }
    ConnectionsEvent::None
}

#[allow(clippy::too_many_arguments)]
fn draw_kind_menu(
    ctx: &Context,
    kind: ConnectionKind,
    group: Option<NetworkMenuGroup>,
    selected_idx: &mut usize,
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
    view: &mut ConnectionsView,
) -> ConnectionsEvent {
    let discovered = discovered_connections(kind);
    let discovered_view = if matches!(kind, ConnectionKind::Network) {
        filter_network_group_discovered_connections(
            &discovered,
            group.unwrap_or(NetworkMenuGroup::All),
        )
    } else {
        discovered.clone()
    };
    let saved_all = saved_connections_for_kind(kind);
    let saved_view = if matches!(kind, ConnectionKind::Network) {
        filter_network_group_saved_connections(&saved_all, group.unwrap_or(NetworkMenuGroup::All))
    } else {
        saved_all
    };
    let refresh_label = format!(
        "Refresh Available {} ({})",
        connection_kind_plural_label(kind),
        discovered_view.len()
    );
    let saved_label = format!(
        "Saved {} ({})",
        connection_kind_plural_label(kind),
        saved_view.len()
    );
    let disconnect_label = if matches!(kind, ConnectionKind::Bluetooth) {
        "Disconnect Device...".to_string()
    } else {
        "Disconnect Active".to_string()
    };
    let items = vec![
        "Search and Connect".to_string(),
        refresh_label,
        "Connect to Available".to_string(),
        disconnect_label.clone(),
        saved_label,
        "---".to_string(),
        "Back".to_string(),
    ];
    let title = format!(
        "Connections - {}{}",
        match kind {
            ConnectionKind::Network => "Network",
            ConnectionKind::Bluetooth => "Bluetooth",
        },
        group
            .filter(|_| matches!(kind, ConnectionKind::Network))
            .map(|g| format!(" ({})", connection_network_group_label(g)))
            .unwrap_or_default()
    );
    let activated = draw_terminal_menu_screen(
        ctx,
        &title,
        Some("Search, refresh, connect, manage saved"),
        &items,
        selected_idx,
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
        Some(0) => ConnectionsEvent::OpenPromptSearch { kind, group },
        Some(1) => ConnectionsEvent::Status(format!("Found {} target(s).", discovered_view.len())),
        Some(2) => {
            if discovered_view.is_empty() {
                ConnectionsEvent::Status("No available targets found.".to_string())
            } else {
                *view = ConnectionsView::Picker {
                    kind,
                    group,
                    title: format!("Available {}", connection_kind_plural_label(kind)),
                    items: discovered_view,
                    mode: PickerMode::Connect,
                };
                ConnectionsEvent::None
            }
        }
        Some(3) => {
            if matches!(kind, ConnectionKind::Bluetooth) {
                let targets = bluetooth_disconnect_targets(&discovered_view);
                if targets.is_empty() {
                    ConnectionsEvent::Status("No bluetooth devices available.".to_string())
                } else {
                    *view = ConnectionsView::Picker {
                        kind,
                        group,
                        title: "Disconnect Bluetooth Device".to_string(),
                        items: targets,
                        mode: PickerMode::DisconnectBluetooth,
                    };
                    ConnectionsEvent::None
                }
            } else {
                ConnectionsEvent::Status(disconnect_connection_status(kind, None, None))
            }
        }
        Some(4) => {
            *view = ConnectionsView::Saved { kind, group };
            ConnectionsEvent::None
        }
        Some(_) => {
            *view = if matches!(kind, ConnectionKind::Network) && group.is_some() {
                ConnectionsView::NetworkGroups
            } else {
                ConnectionsView::Root
            };
            ConnectionsEvent::None
        }
        None => ConnectionsEvent::None,
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_saved_menu(
    ctx: &Context,
    kind: ConnectionKind,
    group: Option<NetworkMenuGroup>,
    selected_idx: &mut usize,
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
    view: &mut ConnectionsView,
) -> ConnectionsEvent {
    let saved_all = saved_connections_for_kind(kind);
    let saved = if matches!(kind, ConnectionKind::Network) {
        filter_network_group_saved_connections(&saved_all, group.unwrap_or(NetworkMenuGroup::All))
    } else {
        saved_all
    };
    if saved.is_empty() {
        return ConnectionsEvent::Status(format!(
            "No saved {}.",
            connection_kind_plural_label(kind).to_ascii_lowercase()
        ));
    }
    let mut items = Vec::new();
    for (idx, entry) in saved.iter().enumerate() {
        items.push(format!(
            "Connect [{}]: {}",
            idx + 1,
            saved_connection_label(entry)
        ));
        items.push(format!("Disconnect [{}]: {}", idx + 1, entry.name));
        items.push(format!("Forget  [{}]: {}", idx + 1, entry.name));
    }
    items.push("---".to_string());
    items.push("Back".to_string());
    let activated = draw_terminal_menu_screen(
        ctx,
        &format!("Saved {}", connection_kind_plural_label(kind)),
        Some("Connect or forget previous targets"),
        &items,
        selected_idx,
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
    if let Some(idx) = activated {
        let triple = saved.len() * 3;
        if idx >= triple {
            *view = ConnectionsView::Kind { kind, group };
            return ConnectionsEvent::None;
        }
        let slot = idx / 3;
        let action = idx % 3;
        let entry = &saved[slot];
        return match action {
            0 => {
                let target = DiscoveredConnection {
                    name: entry.name.clone(),
                    detail: entry.detail.clone(),
                };
                if matches!(kind, ConnectionKind::Network)
                    && connection_requires_password(&target.detail)
                {
                    ConnectionsEvent::OpenPasswordPrompt { kind, target }
                } else {
                    ConnectionsEvent::ConnectImmediate { kind, target }
                }
            }
            1 => ConnectionsEvent::Status(disconnect_connection_status(
                kind,
                Some(entry.name.as_str()),
                Some(entry.detail.as_str()),
            )),
            _ => {
                if forget_saved_connection_and_refresh_settings(kind, &entry.name).is_some() {
                    ConnectionsEvent::Status("Removed.".to_string())
                } else {
                    ConnectionsEvent::Status("Nothing removed.".to_string())
                }
            }
        };
    }
    ConnectionsEvent::None
}

#[allow(clippy::too_many_arguments)]
fn draw_picker(
    ctx: &Context,
    kind: ConnectionKind,
    group: Option<NetworkMenuGroup>,
    title: &str,
    items: &[DiscoveredConnection],
    mode: PickerMode,
    selected_idx: &mut usize,
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
    view: &mut ConnectionsView,
) -> ConnectionsEvent {
    let mut rows_vec: Vec<String> = items.iter().map(discovered_connection_label).collect();
    rows_vec.push("---".to_string());
    rows_vec.push("Back".to_string());
    let activated = draw_terminal_menu_screen(
        ctx,
        title,
        None,
        &rows_vec,
        selected_idx,
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
    if let Some(idx) = activated {
        if idx >= items.len() {
            *view = ConnectionsView::Kind { kind, group };
            return ConnectionsEvent::None;
        }
        let target = items[idx].clone();
        return match mode {
            PickerMode::Connect => {
                if matches!(kind, ConnectionKind::Network)
                    && connection_requires_password(&target.detail)
                {
                    ConnectionsEvent::OpenPasswordPrompt { kind, target }
                } else {
                    ConnectionsEvent::ConnectImmediate { kind, target }
                }
            }
            PickerMode::DisconnectBluetooth => {
                ConnectionsEvent::Status(disconnect_connection_status(
                    kind,
                    Some(target.name.as_str()),
                    Some(target.detail.as_str()),
                ))
            }
        };
    }
    ConnectionsEvent::None
}

pub fn apply_search_query(
    state: &mut TerminalConnectionsState,
    kind: ConnectionKind,
    group: Option<NetworkMenuGroup>,
    query: &str,
) -> ConnectionsEvent {
    let query = query.trim();
    if query.is_empty() {
        return ConnectionsEvent::Status("Enter a search query.".to_string());
    }
    let discovered = discovered_connections(kind);
    let view = if matches!(kind, ConnectionKind::Network) {
        filter_network_group_discovered_connections(
            &discovered,
            group.unwrap_or(NetworkMenuGroup::All),
        )
    } else {
        discovered
    };
    let filtered = filter_discovered_connection_list(&view, query);
    if filtered.is_empty() {
        return ConnectionsEvent::Status("No matches found.".to_string());
    }
    state.view = ConnectionsView::Picker {
        kind,
        group,
        title: "Search Results".to_string(),
        items: filtered,
        mode: PickerMode::Connect,
    };
    state.picker_idx = 0;
    ConnectionsEvent::None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_search_query_returns_status() {
        let mut state = TerminalConnectionsState::default();
        let event = apply_search_query(&mut state, ConnectionKind::Network, None, "");
        assert!(matches!(
            event,
            ConnectionsEvent::Status(ref status) if status == "Enter a search query."
        ));
    }

    #[test]
    fn back_from_network_group_returns_to_root() {
        let mut state = TerminalConnectionsState {
            view: ConnectionsView::NetworkGroups,
            ..Default::default()
        };
        assert!(!state.back());
        assert!(matches!(state.view, ConnectionsView::Root));
    }

    #[test]
    fn back_from_picker_returns_to_kind_menu() {
        let mut state = TerminalConnectionsState {
            view: ConnectionsView::Picker {
                kind: ConnectionKind::Bluetooth,
                group: None,
                title: "Pick".to_string(),
                items: vec![],
                mode: PickerMode::Connect,
            },
            ..Default::default()
        };
        assert!(!state.back());
        assert!(matches!(
            state.view,
            ConnectionsView::Kind {
                kind: ConnectionKind::Bluetooth,
                group: None
            }
        ));
    }
}
