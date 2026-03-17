use robcos_native_services::desktop_connections_service::{
    bluetooth_disconnect_targets, bluetooth_installer_status_hint,
    connection_kind_plural_label, connection_network_group_label, connection_network_groups,
    connection_requires_password, connections_macos_blueutil_missing,
    disconnect_connection_status, discovered_connection_label, discovered_connections,
    filter_discovered_connection_list, filter_network_group_discovered_connections,
    filter_network_group_saved_connections, forget_saved_connection_and_refresh_settings,
    saved_connection_label, saved_connections_for_kind,
};
use robcos_shared::config::{ConnectionKind, SavedConnection};
use robcos_shared::connections::{DiscoveredConnection, NetworkMenuGroup};

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

#[derive(Debug, Clone)]
pub enum TerminalConnectionsRequest {
    None,
    BackToSettings,
    OpenPromptSearch {
        kind: ConnectionKind,
        group: Option<NetworkMenuGroup>,
        title: String,
        prompt: String,
    },
    OpenPasswordPrompt {
        kind: ConnectionKind,
        target: DiscoveredConnection,
        title: String,
        prompt: String,
    },
    ConnectImmediate {
        kind: ConnectionKind,
        target: DiscoveredConnection,
    },
    Status {
        status: String,
        back_to_settings: bool,
    },
    NavigateToView {
        view: ConnectionsView,
        clear_status: bool,
        reset_kind_idx: bool,
        reset_picker_idx: bool,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectionsMenuModel {
    pub title: String,
    pub subtitle: Option<String>,
    pub items: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct KindMenuModel {
    pub menu: ConnectionsMenuModel,
    pub discovered: Vec<DiscoveredConnection>,
    pub disconnect_targets: Vec<DiscoveredConnection>,
}

#[derive(Debug, Clone)]
pub struct SavedMenuModel {
    pub menu: ConnectionsMenuModel,
    pub saved: Vec<SavedConnection>,
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

pub fn build_connections_root_menu() -> ConnectionsMenuModel {
    let mut items = vec!["Network".to_string()];
    let subtitle = if connections_macos_blueutil_missing() {
        Some(bluetooth_installer_status_hint().to_string())
    } else {
        items.push("Bluetooth".to_string());
        None
    };
    items.push("---".to_string());
    items.push("Back".to_string());
    ConnectionsMenuModel {
        title: "Connections".to_string(),
        subtitle,
        items,
    }
}

pub fn resolve_connections_root_activation(activated: Option<usize>) -> ConnectionsEvent {
    match activated {
        Some(0) => ConnectionsEvent::OpenNetworkGroups,
        Some(1) if !connections_macos_blueutil_missing() => ConnectionsEvent::OpenBluetooth,
        Some(_) => ConnectionsEvent::BackToSettings,
        None => ConnectionsEvent::None,
    }
}

pub fn build_network_groups_menu() -> ConnectionsMenuModel {
    let mut items: Vec<String> = connection_network_groups()
        .iter()
        .map(|g| format!("{} Networks", connection_network_group_label(*g)))
        .collect();
    items.push("---".to_string());
    items.push("Back".to_string());
    ConnectionsMenuModel {
        title: "Connections - Network".to_string(),
        subtitle: None,
        items,
    }
}

pub fn apply_network_groups_activation(
    state: &mut TerminalConnectionsState,
    activated: Option<usize>,
) -> ConnectionsEvent {
    if let Some(idx) = activated {
        if idx < connection_network_groups().len() {
            state.view = ConnectionsView::Kind {
                kind: ConnectionKind::Network,
                group: Some(connection_network_groups()[idx]),
            };
        } else {
            state.view = ConnectionsView::Root;
        }
    }
    ConnectionsEvent::None
}

pub fn build_kind_menu_model(kind: ConnectionKind, group: Option<NetworkMenuGroup>) -> KindMenuModel {
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
    let disconnect_targets = if matches!(kind, ConnectionKind::Bluetooth) {
        bluetooth_disconnect_targets(&discovered_view)
    } else {
        Vec::new()
    };
    let items = vec![
        "Search and Connect".to_string(),
        format!(
            "Refresh Available {} ({})",
            connection_kind_plural_label(kind),
            discovered_view.len()
        ),
        "Connect to Available".to_string(),
        if matches!(kind, ConnectionKind::Bluetooth) {
            "Disconnect Device...".to_string()
        } else {
            "Disconnect Active".to_string()
        },
        format!(
            "Saved {} ({})",
            connection_kind_plural_label(kind),
            saved_view.len()
        ),
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
    KindMenuModel {
        menu: ConnectionsMenuModel {
            title,
            subtitle: Some("Search, refresh, connect, manage saved".to_string()),
            items,
        },
        discovered: discovered_view,
        disconnect_targets,
    }
}

pub fn apply_kind_menu_activation(
    state: &mut TerminalConnectionsState,
    kind: ConnectionKind,
    group: Option<NetworkMenuGroup>,
    activated: Option<usize>,
    model: &KindMenuModel,
) -> ConnectionsEvent {
    match activated {
        Some(0) => ConnectionsEvent::OpenPromptSearch { kind, group },
        Some(1) => ConnectionsEvent::Status(format!("Found {} target(s).", model.discovered.len())),
        Some(2) => {
            if model.discovered.is_empty() {
                ConnectionsEvent::Status("No available targets found.".to_string())
            } else {
                state.view = ConnectionsView::Picker {
                    kind,
                    group,
                    title: format!("Available {}", connection_kind_plural_label(kind)),
                    items: model.discovered.clone(),
                    mode: PickerMode::Connect,
                };
                ConnectionsEvent::None
            }
        }
        Some(3) => {
            if matches!(kind, ConnectionKind::Bluetooth) {
                if model.disconnect_targets.is_empty() {
                    ConnectionsEvent::Status("No bluetooth devices available.".to_string())
                } else {
                    state.view = ConnectionsView::Picker {
                        kind,
                        group,
                        title: "Disconnect Bluetooth Device".to_string(),
                        items: model.disconnect_targets.clone(),
                        mode: PickerMode::DisconnectBluetooth,
                    };
                    ConnectionsEvent::None
                }
            } else {
                ConnectionsEvent::Status(disconnect_connection_status(kind, None, None))
            }
        }
        Some(4) => {
            state.view = ConnectionsView::Saved { kind, group };
            ConnectionsEvent::None
        }
        Some(_) => {
            state.view = if matches!(kind, ConnectionKind::Network) && group.is_some() {
                ConnectionsView::NetworkGroups
            } else {
                ConnectionsView::Root
            };
            ConnectionsEvent::None
        }
        None => ConnectionsEvent::None,
    }
}

pub fn build_saved_menu_model(
    kind: ConnectionKind,
    group: Option<NetworkMenuGroup>,
) -> Result<SavedMenuModel, String> {
    let saved_all = saved_connections_for_kind(kind);
    let saved = if matches!(kind, ConnectionKind::Network) {
        filter_network_group_saved_connections(&saved_all, group.unwrap_or(NetworkMenuGroup::All))
    } else {
        saved_all
    };
    if saved.is_empty() {
        return Err(format!(
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
    Ok(SavedMenuModel {
        menu: ConnectionsMenuModel {
            title: format!("Saved {}", connection_kind_plural_label(kind)),
            subtitle: Some("Connect or forget previous targets".to_string()),
            items,
        },
        saved,
    })
}

pub fn apply_saved_menu_activation(
    state: &mut TerminalConnectionsState,
    kind: ConnectionKind,
    group: Option<NetworkMenuGroup>,
    activated: Option<usize>,
    model: &SavedMenuModel,
) -> ConnectionsEvent {
    if let Some(idx) = activated {
        let triple = model.saved.len() * 3;
        if idx >= triple {
            state.view = ConnectionsView::Kind { kind, group };
            return ConnectionsEvent::None;
        }
        let slot = idx / 3;
        let action = idx % 3;
        let entry = &model.saved[slot];
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

pub fn build_picker_menu(title: &str, items: &[DiscoveredConnection]) -> ConnectionsMenuModel {
    let mut rows: Vec<String> = items.iter().map(discovered_connection_label).collect();
    rows.push("---".to_string());
    rows.push("Back".to_string());
    ConnectionsMenuModel {
        title: title.to_string(),
        subtitle: None,
        items: rows,
    }
}

pub fn apply_picker_activation(
    state: &mut TerminalConnectionsState,
    kind: ConnectionKind,
    group: Option<NetworkMenuGroup>,
    items: &[DiscoveredConnection],
    mode: PickerMode,
    activated: Option<usize>,
) -> ConnectionsEvent {
    if let Some(idx) = activated {
        if idx >= items.len() {
            state.view = ConnectionsView::Kind { kind, group };
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
            PickerMode::DisconnectBluetooth => ConnectionsEvent::Status(
                disconnect_connection_status(
                    kind,
                    Some(target.name.as_str()),
                    Some(target.detail.as_str()),
                ),
            ),
        };
    }
    ConnectionsEvent::None
}

pub fn resolve_terminal_connections_request(
    state: &mut TerminalConnectionsState,
    event: ConnectionsEvent,
    disabled_hint: &str,
) -> TerminalConnectionsRequest {
    match event {
        ConnectionsEvent::None => TerminalConnectionsRequest::None,
        ConnectionsEvent::BackToSettings => TerminalConnectionsRequest::BackToSettings,
        ConnectionsEvent::OpenNetworkGroups => {
            state.view = ConnectionsView::NetworkGroups;
            TerminalConnectionsRequest::NavigateToView {
                view: state.view.clone(),
                clear_status: true,
                reset_kind_idx: false,
                reset_picker_idx: false,
            }
        }
        ConnectionsEvent::OpenBluetooth => {
            state.view = ConnectionsView::Kind {
                kind: ConnectionKind::Bluetooth,
                group: None,
            };
            TerminalConnectionsRequest::NavigateToView {
                view: state.view.clone(),
                clear_status: true,
                reset_kind_idx: true,
                reset_picker_idx: false,
            }
        }
        ConnectionsEvent::OpenPromptSearch { kind, group } => {
            TerminalConnectionsRequest::OpenPromptSearch {
                kind,
                group,
                title: "Connections".to_string(),
                prompt: "Search query:".to_string(),
            }
        }
        ConnectionsEvent::OpenPasswordPrompt { kind, target } => {
            TerminalConnectionsRequest::OpenPasswordPrompt {
                kind,
                prompt: format!("Password for {} (blank cancels)", target.name),
                title: "Connections".to_string(),
                target,
            }
        }
        ConnectionsEvent::ConnectImmediate { kind, target } => {
            TerminalConnectionsRequest::ConnectImmediate { kind, target }
        }
        ConnectionsEvent::Status(status) => TerminalConnectionsRequest::Status {
            back_to_settings: status == disabled_hint,
            status,
        },
    }
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

    #[test]
    fn root_menu_contains_back() {
        let menu = build_connections_root_menu();
        assert_eq!(menu.items.first().map(String::as_str), Some("Network"));
        assert_eq!(menu.items.last().map(String::as_str), Some("Back"));
    }

    #[test]
    fn network_group_activation_opens_network_kind_view() {
        let mut state = TerminalConnectionsState::default();
        let event = apply_network_groups_activation(&mut state, Some(0));

        assert!(matches!(event, ConnectionsEvent::None));
        assert!(matches!(
            state.view,
            ConnectionsView::Kind {
                kind: ConnectionKind::Network,
                group: Some(_)
            }
        ));
    }

    #[test]
    fn resolve_terminal_connections_request_marks_disabled_status_for_back() {
        let mut state = TerminalConnectionsState::default();
        let request = resolve_terminal_connections_request(
            &mut state,
            ConnectionsEvent::Status("disabled".to_string()),
            "disabled",
        );

        assert!(matches!(
            request,
            TerminalConnectionsRequest::Status {
                back_to_settings: true,
                ..
            }
        ));
    }
}
