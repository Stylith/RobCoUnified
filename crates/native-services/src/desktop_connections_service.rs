use crate::config::{get_settings, ConnectionKind, SavedConnection, Settings};
use crate::connections::{
    bluetooth_installer_hint, connect_connection, disconnect_connection, discovered_row_label,
    filter_discovered_connections, filter_network_discovered_group, filter_network_saved_group,
    forget_saved_connection, kind_plural_label, macos_blueutil_missing, macos_connections_disabled,
    macos_connections_disabled_hint, network_group_label, network_menu_groups,
    network_requires_password, refresh_discovered_connections, saved_connections, saved_row_label,
    NetworkMenuGroup,
};
use std::collections::HashSet;

pub use crate::connections::DiscoveredConnection;

pub fn bluetooth_installer_status_hint() -> &'static str {
    bluetooth_installer_hint()
}

pub fn connection_kind_plural_label(kind: ConnectionKind) -> &'static str {
    kind_plural_label(kind)
}

pub fn connection_network_group_label(group: NetworkMenuGroup) -> &'static str {
    network_group_label(group)
}

pub fn connection_network_groups() -> [NetworkMenuGroup; 5] {
    network_menu_groups()
}

pub fn connections_macos_blueutil_missing() -> bool {
    macos_blueutil_missing()
}

pub fn connections_macos_disabled() -> bool {
    macos_connections_disabled()
}

pub fn connections_macos_disabled_hint() -> &'static str {
    macos_connections_disabled_hint()
}

pub fn connection_requires_password(detail: &str) -> bool {
    network_requires_password(detail)
}

pub fn discovered_connection_label(item: &DiscoveredConnection) -> String {
    discovered_row_label(item)
}

pub fn saved_connection_label(item: &SavedConnection) -> String {
    saved_row_label(item)
}

pub fn discovered_connections(kind: ConnectionKind) -> Vec<DiscoveredConnection> {
    refresh_discovered_connections(kind)
}

pub fn saved_connections_for_kind(kind: ConnectionKind) -> Vec<SavedConnection> {
    saved_connections(kind)
}

pub fn filter_discovered_connection_list(
    discovered: &[DiscoveredConnection],
    query: &str,
) -> Vec<DiscoveredConnection> {
    filter_discovered_connections(discovered, query)
}

pub fn filter_network_group_discovered_connections(
    discovered: &[DiscoveredConnection],
    group: NetworkMenuGroup,
) -> Vec<DiscoveredConnection> {
    filter_network_discovered_group(discovered, group)
}

pub fn filter_network_group_saved_connections(
    saved: &[SavedConnection],
    group: NetworkMenuGroup,
) -> Vec<SavedConnection> {
    filter_network_saved_group(saved, group)
}

pub fn disconnect_connection_status(
    kind: ConnectionKind,
    name: Option<&str>,
    detail: Option<&str>,
) -> String {
    disconnect_connection(kind, name, detail)
}

pub fn bluetooth_disconnect_targets(
    discovered: &[DiscoveredConnection],
) -> Vec<DiscoveredConnection> {
    let mut out = Vec::new();
    let mut seen = HashSet::new();

    for item in discovered {
        let name = item.name.trim();
        if name.is_empty() {
            continue;
        }
        let key = name.to_ascii_lowercase();
        if seen.insert(key) {
            out.push(item.clone());
        }
    }

    for entry in saved_connections_for_kind(ConnectionKind::Bluetooth) {
        let name = entry.name.trim();
        if name.is_empty() {
            continue;
        }
        let key = name.to_ascii_lowercase();
        if seen.insert(key) {
            out.push(DiscoveredConnection {
                name: entry.name,
                detail: entry.detail,
            });
        }
    }

    out
}

pub fn scan_discovered_connections(kind: ConnectionKind) -> (Vec<DiscoveredConnection>, String) {
    let discovered = discovered_connections(kind);
    let status = format!("Found {} items.", discovered.len());
    (discovered, status)
}

pub fn forget_saved_connection_and_refresh_settings(
    kind: ConnectionKind,
    name: &str,
) -> Option<(Settings, String)> {
    forget_saved_connection(kind, name).then(|| (get_settings(), format!("Forgot '{}'.", name)))
}

pub fn connect_connection_and_refresh_settings(
    kind: ConnectionKind,
    target: &DiscoveredConnection,
    password: Option<&str>,
) -> Result<(Settings, String), String> {
    let status = connect_connection(kind, &target.name, Some(target.detail.as_str()), password)
        .map_err(|err| err.to_string())?;
    Ok((get_settings(), status))
}
