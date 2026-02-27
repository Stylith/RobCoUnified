use anyhow::Result;
use std::process::{Child, Command, ExitStatus, Stdio};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use crate::config::{
    get_settings, persist_settings, update_settings, ConnectionKind, SavedConnection, Settings,
};
use crate::ui::{input_prompt, run_menu, MenuResult, Term};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveredConnection {
    pub name: String,
    pub detail: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetworkMenuGroup {
    Wifi,
    Ethernet,
    Thunderbolt,
    Other,
    All,
}

pub fn network_menu_groups() -> [NetworkMenuGroup; 5] {
    [
        NetworkMenuGroup::Wifi,
        NetworkMenuGroup::Ethernet,
        NetworkMenuGroup::Thunderbolt,
        NetworkMenuGroup::Other,
        NetworkMenuGroup::All,
    ]
}

pub fn network_group_label(group: NetworkMenuGroup) -> &'static str {
    match group {
        NetworkMenuGroup::Wifi => "Wi-Fi",
        NetworkMenuGroup::Ethernet => "Ethernet",
        NetworkMenuGroup::Thunderbolt => "Thunderbolt",
        NetworkMenuGroup::Other => "Other",
        NetworkMenuGroup::All => "All",
    }
}

pub fn kind_label(kind: ConnectionKind) -> &'static str {
    match kind {
        ConnectionKind::Network => "Network",
        ConnectionKind::Bluetooth => "Bluetooth",
    }
}

pub fn kind_plural_label(kind: ConnectionKind) -> &'static str {
    match kind {
        ConnectionKind::Network => "Networks",
        ConnectionKind::Bluetooth => "Bluetooth Devices",
    }
}

pub fn macos_blueutil_missing() -> bool {
    cfg!(target_os = "macos") && !command_exists("blueutil")
}

pub fn bluetooth_installer_hint() -> &'static str {
    "Bluetooth tools require blueutil. Install it from Program Installer."
}

pub fn filter_network_discovered_group(
    discovered: &[DiscoveredConnection],
    group: NetworkMenuGroup,
) -> Vec<DiscoveredConnection> {
    if matches!(group, NetworkMenuGroup::All) {
        return discovered.to_vec();
    }
    discovered
        .iter()
        .filter(|entry| network_entry_group(entry.name.as_str(), entry.detail.as_str()) == group)
        .cloned()
        .collect()
}

pub fn filter_network_saved_group(
    saved: &[SavedConnection],
    group: NetworkMenuGroup,
) -> Vec<SavedConnection> {
    if matches!(group, NetworkMenuGroup::All) {
        return saved.to_vec();
    }
    saved
        .iter()
        .filter(|entry| network_entry_group(entry.name.as_str(), entry.detail.as_str()) == group)
        .cloned()
        .collect()
}

pub fn discovered_row_label(item: &DiscoveredConnection) -> String {
    if item.detail.trim().is_empty() {
        item.name.clone()
    } else {
        format!("{} ({})", item.name, item.detail)
    }
}

pub fn saved_row_label(item: &SavedConnection) -> String {
    if item.detail.trim().is_empty() {
        item.name.clone()
    } else {
        format!("{} ({})", item.name, item.detail)
    }
}

pub fn saved_connections(kind: ConnectionKind) -> Vec<SavedConnection> {
    let s = get_settings();
    match kind {
        ConnectionKind::Network => s.connections.network,
        ConnectionKind::Bluetooth => s.connections.bluetooth,
    }
}

fn saved_connections_mut(
    settings: &mut Settings,
    kind: ConnectionKind,
) -> &mut Vec<SavedConnection> {
    match kind {
        ConnectionKind::Network => &mut settings.connections.network,
        ConnectionKind::Bluetooth => &mut settings.connections.bluetooth,
    }
}

pub fn forget_saved_connection(kind: ConnectionKind, name: &str) -> bool {
    let target = name.trim().to_ascii_lowercase();
    if target.is_empty() {
        return false;
    }
    let mut changed = false;
    update_settings(|s| {
        let list = saved_connections_mut(s, kind);
        let before = list.len();
        list.retain(|entry| entry.name.trim().to_ascii_lowercase() != target);
        changed = list.len() != before;
    });
    if changed {
        persist_settings();
    }
    changed
}

pub fn connect_and_save_connection(
    kind: ConnectionKind,
    name: &str,
    detail: Option<&str>,
) -> Result<String> {
    let name = name.trim();
    if name.is_empty() {
        return Err(anyhow::anyhow!("empty target"));
    }
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let detail = detail.unwrap_or("").trim().to_string();
    let normalized = name.to_ascii_lowercase();
    update_settings(|s| {
        let list = saved_connections_mut(s, kind);
        list.retain(|entry| entry.name.trim().to_ascii_lowercase() != normalized);
        list.insert(
            0,
            SavedConnection {
                name: name.to_string(),
                detail: detail.clone(),
                last_connected_unix: now,
            },
        );
        if list.len() > 64 {
            list.truncate(64);
        }
    });
    persist_settings();
    Ok(format!("Connected to {name}"))
}

pub fn network_requires_password(detail: &str) -> bool {
    let d = detail.to_ascii_lowercase();
    (d.contains("wpa")
        || d.contains("wep")
        || d.contains("psk")
        || d.contains("802.1x")
        || d.contains("enterprise")
        || d.contains("security"))
        && !d.contains("open")
        && !d.contains("none")
}

pub fn connect_connection(
    kind: ConnectionKind,
    name: &str,
    detail: Option<&str>,
    password: Option<&str>,
) -> Result<String> {
    let name = name.trim();
    if name.is_empty() {
        return Err(anyhow::anyhow!("empty target"));
    }
    let detail_text = detail.unwrap_or("").trim().to_string();

    let connected = match kind {
        ConnectionKind::Network => connect_network(name, password),
        ConnectionKind::Bluetooth => connect_bluetooth(name, &detail_text),
    };
    let _ = connect_and_save_connection(kind, name, Some(detail_text.as_str()))?;
    Ok(if connected {
        format!("Connected to {name}")
    } else {
        format!("Saved connection for {name}")
    })
}

pub fn disconnect_connection(
    kind: ConnectionKind,
    name: Option<&str>,
    detail: Option<&str>,
) -> String {
    let disconnected = match kind {
        ConnectionKind::Network => disconnect_network(name),
        ConnectionKind::Bluetooth => disconnect_bluetooth(name, detail.unwrap_or("")),
    };

    match (disconnected, kind, name) {
        (true, ConnectionKind::Network, Some(target)) => format!("Disconnected {target}"),
        (true, ConnectionKind::Bluetooth, Some(target)) => format!("Disconnected {target}"),
        (true, ConnectionKind::Network, None) => "Disconnected active network".to_string(),
        (true, ConnectionKind::Bluetooth, None) => {
            "Disconnected active bluetooth device".to_string()
        }
        (false, ConnectionKind::Network, _) => "Disconnect failed or not supported".to_string(),
        (false, ConnectionKind::Bluetooth, _) => "Disconnect failed or not supported".to_string(),
    }
}

pub fn filter_discovered_connections(
    discovered: &[DiscoveredConnection],
    query: &str,
) -> Vec<DiscoveredConnection> {
    let q = query.trim().to_ascii_lowercase();
    if q.is_empty() {
        return discovered.to_vec();
    }
    discovered
        .iter()
        .filter(|item| {
            item.name.to_ascii_lowercase().contains(&q)
                || item.detail.to_ascii_lowercase().contains(&q)
        })
        .cloned()
        .collect()
}

pub fn choose_discovered_connection(
    terminal: &mut Term,
    kind: ConnectionKind,
    title: &str,
    discovered: &[DiscoveredConnection],
    allow_manual: bool,
) -> Result<Option<DiscoveredConnection>> {
    let mut rows: Vec<String> = discovered
        .iter()
        .enumerate()
        .map(|(idx, item)| format!("{}. {}", idx + 1, discovered_row_label(item)))
        .collect();
    if allow_manual {
        rows.push("Manual Entry...".to_string());
    }
    rows.push("---".to_string());
    rows.push("Back".to_string());
    let refs: Vec<&str> = rows.iter().map(String::as_str).collect();

    match run_menu(terminal, title, &refs, Some("Search, select, then connect"))? {
        MenuResult::Back => Ok(None),
        MenuResult::Selected(sel) if sel == "Back" => Ok(None),
        MenuResult::Selected(sel) if allow_manual && sel == "Manual Entry..." => {
            let prompt = format!("{} name:", kind_label(kind));
            let Some(raw) = input_prompt(terminal, &prompt)? else {
                return Ok(None);
            };
            let manual = raw.trim();
            if manual.is_empty() {
                return Ok(None);
            }
            Ok(Some(DiscoveredConnection {
                name: manual.to_string(),
                detail: "Manual".to_string(),
            }))
        }
        MenuResult::Selected(sel) => {
            let Some((idx, _)) = discovered
                .iter()
                .enumerate()
                .find(|(idx, item)| format!("{}. {}", idx + 1, discovered_row_label(item)) == sel)
            else {
                return Ok(None);
            };
            Ok(discovered.get(idx).cloned())
        }
    }
}

pub fn refresh_discovered_connections(kind: ConnectionKind) -> Vec<DiscoveredConnection> {
    let mut out = Vec::new();
    match kind {
        ConnectionKind::Network => {
            scan_network_nmcli(&mut out);
            scan_network_macos_airport(&mut out);
            scan_network_macos_system_profiler(&mut out);
            scan_network_setup_ports(&mut out);
            scan_network_interfaces(&mut out);
        }
        ConnectionKind::Bluetooth => {
            scan_macos_blueutil(&mut out);
            scan_bluetoothctl(&mut out);
            scan_macos_bluetooth(&mut out);
        }
    }
    out.sort_by_key(|item| item.name.to_ascii_lowercase());
    out
}

fn command_exists(bin: &str) -> bool {
    Command::new("which")
        .arg(bin)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

const COMMAND_TIMEOUT: Duration = Duration::from_secs(3);
const COMMAND_POLL_INTERVAL: Duration = Duration::from_millis(20);

fn wait_for_child_with_timeout(child: &mut Child, timeout: Duration) -> Option<ExitStatus> {
    let started = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return Some(status),
            Ok(None) => {
                if started.elapsed() >= timeout {
                    let _ = child.kill();
                    let _ = child.wait();
                    return None;
                }
                std::thread::sleep(COMMAND_POLL_INTERVAL);
            }
            Err(_) => {
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
        }
    }
}

fn command_status_success(bin: &str, args: &[String]) -> bool {
    if !command_exists(bin) {
        return false;
    }
    let mut child = match Command::new(bin)
        .args(args)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
    {
        Ok(child) => child,
        Err(_) => return false,
    };
    wait_for_child_with_timeout(&mut child, COMMAND_TIMEOUT)
        .map(|status| status.success())
        .unwrap_or(false)
}

fn command_status_success_path(bin_path: &str, args: &[&str]) -> bool {
    let mut child = match Command::new(bin_path)
        .args(args)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
    {
        Ok(child) => child,
        Err(_) => return false,
    };
    wait_for_child_with_timeout(&mut child, COMMAND_TIMEOUT)
        .map(|status| status.success())
        .unwrap_or(false)
}

fn command_output(bin: &str, args: &[&str]) -> Option<String> {
    if !command_exists(bin) {
        return None;
    }
    Command::new(bin)
        .args(args)
        .output()
        .ok()
        .map(|out| String::from_utf8_lossy(&out.stdout).to_string())
}

fn command_output_path(bin_path: &str, args: &[&str]) -> Option<String> {
    Command::new(bin_path)
        .args(args)
        .output()
        .ok()
        .map(|out| String::from_utf8_lossy(&out.stdout).to_string())
}

fn macos_wifi_device() -> Option<String> {
    let out = command_output("networksetup", &["-listallhardwareports"])?;
    let mut in_wifi = false;
    for line in out.lines() {
        let trimmed = line.trim();
        if let Some(port) = trimmed.strip_prefix("Hardware Port:") {
            let p = port.trim().to_ascii_lowercase();
            in_wifi = p == "wi-fi" || p == "airport";
            continue;
        }
        if in_wifi {
            if let Some(device) = trimmed.strip_prefix("Device:") {
                let d = device.trim();
                if !d.is_empty() {
                    return Some(d.to_string());
                }
            }
        }
    }
    None
}

fn macos_airport_bin() -> Option<&'static str> {
    let path =
        "/System/Library/PrivateFrameworks/Apple80211.framework/Versions/Current/Resources/airport";
    if std::path::Path::new(path).exists() {
        Some(path)
    } else {
        None
    }
}

fn find_bluetooth_mac_by_name(name: &str) -> Option<String> {
    let out = command_output("bluetoothctl", &["devices"])?;
    let target = name.trim().to_ascii_lowercase();
    for line in out.lines() {
        let trimmed = line.trim();
        let Some(rest) = trimmed.strip_prefix("Device ") else {
            continue;
        };
        let mut parts = rest.splitn(2, ' ');
        let mac = parts.next().unwrap_or("").trim();
        let device_name = parts.next().unwrap_or("").trim().to_ascii_lowercase();
        if device_name == target && !mac.is_empty() {
            return Some(mac.to_string());
        }
    }
    let target = name.trim().to_ascii_lowercase();
    for (device_name, mac, _) in macos_bluetooth_devices() {
        if !mac.is_empty() && device_name.to_ascii_lowercase() == target {
            return Some(mac);
        }
    }
    None
}

fn mac_from_detail(detail: &str) -> Option<String> {
    let marker = "MAC ";
    let idx = detail.find(marker)?;
    let tail = &detail[idx + marker.len()..];
    let raw = tail
        .split_whitespace()
        .next()
        .unwrap_or("")
        .trim_matches(|c: char| !c.is_ascii_hexdigit() && c != ':' && c != '-')
        .trim();
    if raw.is_empty() {
        return None;
    }
    let mac = normalize_mac(raw);
    if mac.is_empty() {
        return None;
    }
    let mac = mac
        .trim_matches(|c: char| !c.is_ascii_hexdigit() && c != ':' && c != '-')
        .to_string();
    if mac.is_empty() {
        None
    } else {
        Some(mac)
    }
}

fn normalize_mac(value: &str) -> String {
    value.trim().replace('-', ":").to_ascii_uppercase()
}

fn network_entry_group(name: &str, detail: &str) -> NetworkMenuGroup {
    let text = format!(
        "{} {}",
        name.to_ascii_lowercase(),
        detail.to_ascii_lowercase()
    );
    if text.contains("wi-fi") || text.contains("wifi") {
        return NetworkMenuGroup::Wifi;
    }
    if text.contains("thunderbolt") {
        return NetworkMenuGroup::Thunderbolt;
    }
    if text.contains("ethernet") {
        return NetworkMenuGroup::Ethernet;
    }
    if text.contains("interface") {
        return NetworkMenuGroup::Other;
    }
    NetworkMenuGroup::Other
}

fn is_bssid(candidate: &str) -> bool {
    if candidate.len() != 17 {
        return false;
    }
    let bytes = candidate.as_bytes();
    for (idx, b) in bytes.iter().enumerate() {
        if matches!(idx, 2 | 5 | 8 | 11 | 14) {
            if *b != b':' {
                return false;
            }
        } else if !(*b as char).is_ascii_hexdigit() {
            return false;
        }
    }
    true
}

fn is_mac_like(candidate: &str) -> bool {
    let upper = candidate.to_ascii_uppercase();
    if is_bssid(&upper) {
        return true;
    }
    let compact: String = upper.chars().filter(|c| *c != ':' && *c != '-').collect();
    compact.len() == 12 && compact.chars().all(|c| c.is_ascii_hexdigit())
}

fn mac_compact(candidate: &str) -> String {
    candidate
        .chars()
        .filter(|c| c.is_ascii_hexdigit())
        .collect::<String>()
        .to_ascii_uppercase()
}

fn blueutil_is_connected(id: &str) -> Option<bool> {
    let out = command_output("blueutil", &["--is-connected", id])?;
    let value = out.trim();
    if value == "1" {
        Some(true)
    } else if value == "0" {
        Some(false)
    } else {
        None
    }
}

fn find_bssid_in_line(line: &str) -> Option<usize> {
    let bytes = line.as_bytes();
    if bytes.len() < 17 {
        return None;
    }
    for idx in 0..=bytes.len() - 17 {
        let chunk = String::from_utf8_lossy(&bytes[idx..idx + 17]);
        if is_bssid(chunk.as_ref()) {
            return Some(idx);
        }
    }
    None
}

fn macos_bluetooth_devices() -> Vec<(String, String, bool)> {
    let Some(out) = command_output("system_profiler", &["SPBluetoothDataType"]) else {
        return Vec::new();
    };
    let mut devices = Vec::new();
    let mut in_devices = false;
    let mut current_name: Option<String> = None;
    let mut current_mac = String::new();
    let mut current_connected = false;

    let flush = |devices: &mut Vec<(String, String, bool)>,
                 name: &mut Option<String>,
                 mac: &mut String,
                 connected: &mut bool| {
        if let Some(device_name) = name.take() {
            devices.push((device_name, mac.clone(), *connected));
        }
        mac.clear();
        *connected = false;
    };

    let field_like = |key: &str| {
        matches!(
            key,
            "address"
                | "major type"
                | "minor type"
                | "services"
                | "paired"
                | "connected"
                | "manufacturer"
                | "firmware version"
                | "vendor id"
                | "product id"
                | "class of device"
                | "battery level"
                | "transport"
        )
    };

    for raw in out.lines() {
        let trimmed = raw.trim();
        if trimmed.starts_with("Devices (") {
            in_devices = true;
            continue;
        }
        if !in_devices {
            continue;
        }
        if trimmed.is_empty() {
            continue;
        }

        let indent = raw.chars().take_while(|c| *c == ' ').count();
        if indent <= 2 {
            break;
        }

        if trimmed.ends_with(':') {
            let key = trimmed.trim_end_matches(':').trim().to_ascii_lowercase();
            if indent >= 8 && !field_like(&key) {
                flush(
                    &mut devices,
                    &mut current_name,
                    &mut current_mac,
                    &mut current_connected,
                );
                current_name = Some(trimmed.trim_end_matches(':').trim().to_string());
                continue;
            }
        }

        if current_name.is_none() {
            continue;
        }
        if let Some(value) = trimmed.strip_prefix("Address:") {
            current_mac = normalize_mac(value);
        } else if let Some(value) = trimmed.strip_prefix("Connected:") {
            current_connected = value.trim().eq_ignore_ascii_case("yes");
        }
    }
    flush(
        &mut devices,
        &mut current_name,
        &mut current_mac,
        &mut current_connected,
    );
    devices
}

fn parse_blueutil_json_devices(raw: &str, connected_default: bool) -> Vec<(String, String, bool)> {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(raw) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    let mut push_obj = |obj: &serde_json::Map<String, serde_json::Value>| {
        let name = obj
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim()
            .to_string();
        let mac = obj
            .get("address")
            .and_then(|v| v.as_str())
            .map(normalize_mac)
            .unwrap_or_default();
        let connected = obj
            .get("connected")
            .and_then(|v| v.as_bool())
            .unwrap_or(connected_default);
        if name.is_empty() && mac.is_empty() {
            return;
        }
        out.push((name, mac, connected));
    };

    match value {
        serde_json::Value::Array(arr) => {
            for v in &arr {
                if let Some(obj) = v.as_object() {
                    push_obj(obj);
                }
            }
        }
        serde_json::Value::Object(obj) => {
            push_obj(&obj);
        }
        _ => {}
    }
    out
}

fn connect_network(name: &str, password: Option<&str>) -> bool {
    let pwd = password.unwrap_or("").trim();
    if command_exists("nmcli") {
        let mut args = vec![
            "device".to_string(),
            "wifi".to_string(),
            "connect".to_string(),
            name.to_string(),
        ];
        if !pwd.is_empty() {
            args.push("password".to_string());
            args.push(pwd.to_string());
        }
        if command_status_success("nmcli", &args) {
            return true;
        }
    }
    if let Some(dev) = macos_wifi_device() {
        let _ = command_status_success(
            "networksetup",
            &[
                "-setairportpower".to_string(),
                dev.clone(),
                "on".to_string(),
            ],
        );
        let mut args = vec!["-setairportnetwork".to_string(), dev, name.to_string()];
        if !pwd.is_empty() {
            args.push(pwd.to_string());
        }
        if command_status_success("networksetup", &args) {
            return true;
        }
    }
    false
}

fn connect_bluetooth(name: &str, detail: &str) -> bool {
    let target_mac = mac_from_detail(detail).or_else(|| find_bluetooth_mac_by_name(name));
    if command_exists("bluetoothctl") {
        let id = target_mac.clone().unwrap_or_else(|| name.to_string());
        let args = vec!["connect".to_string(), id];
        if command_status_success("bluetoothctl", &args) {
            return true;
        }
    }
    if command_exists("blueutil") {
        let id = target_mac.unwrap_or_else(|| name.to_string());
        let args = vec!["--connect".to_string(), id];
        if command_status_success("blueutil", &args) {
            return true;
        }
    }
    false
}

fn disconnect_network(name: Option<&str>) -> bool {
    if command_exists("nmcli") {
        if let Some(target) = name.map(str::trim).filter(|s| !s.is_empty()) {
            let down = vec![
                "connection".to_string(),
                "down".to_string(),
                "id".to_string(),
                target.to_string(),
            ];
            if command_status_success("nmcli", &down) {
                return true;
            }
            let dev_disconnect = vec![
                "device".to_string(),
                "disconnect".to_string(),
                target.to_string(),
            ];
            if command_status_success("nmcli", &dev_disconnect) {
                return true;
            }
        }

        if let Some(status) = command_output("nmcli", &["-t", "-f", "DEVICE,TYPE,STATE", "device"])
        {
            let mut any = false;
            for line in status.lines() {
                let mut cols = line.splitn(3, ':');
                let device = cols.next().unwrap_or("").trim();
                let kind = cols.next().unwrap_or("").trim();
                let state = cols.next().unwrap_or("").trim();
                if device.is_empty() || !matches!(kind, "wifi" | "ethernet") {
                    continue;
                }
                if !(state == "connected" || state == "connecting") {
                    continue;
                }
                let args = vec![
                    "device".to_string(),
                    "disconnect".to_string(),
                    device.to_string(),
                ];
                if command_status_success("nmcli", &args) {
                    any = true;
                }
            }
            if any {
                return true;
            }
        }
    }
    if let Some(airport) = macos_airport_bin() {
        if command_status_success_path(airport, &["-z"]) {
            return true;
        }
    }
    if let Some(dev) = macos_wifi_device() {
        let args = vec!["-setairportpower".to_string(), dev, "off".to_string()];
        if command_status_success("networksetup", &args) {
            return true;
        }
    }
    false
}

fn disconnect_bluetooth(name: Option<&str>, detail: &str) -> bool {
    let mut targets = Vec::new();
    if let Some(mac) = mac_from_detail(detail) {
        targets.push(mac);
    }
    if let Some(n) = name {
        if let Some(mac) = find_bluetooth_mac_by_name(n) {
            if !targets.iter().any(|m| m == &mac) {
                targets.push(mac);
            }
        }
    }
    if command_exists("bluetoothctl") {
        for mac in &targets {
            let args = vec!["disconnect".to_string(), mac.clone()];
            if command_status_success("bluetoothctl", &args) {
                return true;
            }
        }
    }
    if command_exists("blueutil") {
        if targets.is_empty() {
            if let Some(n) = name {
                if let Some(mac) = find_bluetooth_mac_by_name(n) {
                    targets.push(mac);
                } else {
                    let named = n.trim().to_string();
                    if !named.is_empty() {
                        targets.push(named);
                    }
                }
            }
            if targets.is_empty() {
                if let Some(raw) = command_output("blueutil", &["--connected", "--format", "json"])
                {
                    for (name, mac, _) in parse_blueutil_json_devices(&raw, true) {
                        let id = if !mac.is_empty() { mac } else { name };
                        if !id.is_empty() && !targets.iter().any(|m| m == &id) {
                            targets.push(id);
                        }
                    }
                }
            }
            if targets.is_empty() {
                for (name, mac, connected) in macos_bluetooth_devices() {
                    if !connected {
                        continue;
                    }
                    let id = if !mac.is_empty() { mac } else { name };
                    if !id.is_empty() && !targets.iter().any(|m| m == &id) {
                        targets.push(id);
                    }
                }
            }
        }
        if targets.is_empty() {
            // Nothing appears connected and there is no target to resolve.
            return true;
        }

        for target in &targets {
            let mut candidates = vec![target.clone()];
            if is_mac_like(target) {
                let compact = mac_compact(target);
                if !compact.is_empty() && !candidates.iter().any(|c| c == &compact) {
                    candidates.push(compact);
                }
            }

            for id in candidates {
                let args = vec!["--disconnect".to_string(), id.clone()];
                if command_status_success("blueutil", &args) {
                    return true;
                }
                if let Some(false) = blueutil_is_connected(id.as_str()) {
                    // Desired state already reached.
                    return true;
                }
            }
        }
    }
    false
}

fn push_unique(items: &mut Vec<DiscoveredConnection>, name: String, detail: String) {
    let key = name.trim().to_ascii_lowercase();
    if key.is_empty() {
        return;
    }
    if items
        .iter()
        .any(|existing| existing.name.trim().to_ascii_lowercase() == key)
    {
        return;
    }
    items.push(DiscoveredConnection { name, detail });
}

fn scan_network_nmcli(items: &mut Vec<DiscoveredConnection>) {
    let Some(wifi) = command_output(
        "nmcli",
        &[
            "-t",
            "-f",
            "IN-USE,SSID,SIGNAL,SECURITY",
            "device",
            "wifi",
            "list",
            "--rescan",
            "auto",
        ],
    ) else {
        return;
    };
    for line in wifi.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let mut cols = line.splitn(4, ':');
        let in_use = cols.next().unwrap_or("");
        let ssid = cols.next().unwrap_or("").trim();
        let signal = cols.next().unwrap_or("").trim();
        let security = cols.next().unwrap_or("").trim();
        if ssid.is_empty() {
            continue;
        }
        let mut detail = "Wi-Fi".to_string();
        if !signal.is_empty() {
            detail.push_str(&format!(", {signal}%"));
        }
        if !security.is_empty() {
            detail.push_str(&format!(", {security}"));
        }
        if in_use.trim() == "*" {
            detail.push_str(", connected");
        }
        push_unique(items, ssid.to_string(), detail);
    }

    if let Some(status) = command_output(
        "nmcli",
        &[
            "-t",
            "-f",
            "TYPE,DEVICE,STATE,CONNECTION",
            "device",
            "status",
        ],
    ) {
        for line in status.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let mut cols = line.splitn(4, ':');
            let kind = cols.next().unwrap_or("").trim();
            let device = cols.next().unwrap_or("").trim();
            let state = cols.next().unwrap_or("").trim();
            let connection = cols.next().unwrap_or("").trim();
            if !matches!(kind, "wifi" | "ethernet") {
                continue;
            }
            let name = if !connection.is_empty() && connection != "--" {
                connection
            } else {
                device
            };
            if name.is_empty() {
                continue;
            }
            let detail = format!(
                "{}{}, {}",
                if kind == "wifi" { "Wi-Fi" } else { "Ethernet" },
                if device.is_empty() {
                    String::new()
                } else {
                    format!(" ({device})")
                },
                state
            );
            push_unique(items, name.to_string(), detail);
        }
    }
}

fn scan_network_macos_airport(items: &mut Vec<DiscoveredConnection>) {
    let Some(airport) = macos_airport_bin() else {
        return;
    };
    let Some(out) = command_output_path(airport, &["-s"]) else {
        return;
    };
    for line in out.lines().skip(1) {
        let raw = line.trim_end();
        if raw.trim().is_empty() {
            continue;
        }
        let Some(idx) = find_bssid_in_line(raw) else {
            continue;
        };
        let bytes = raw.as_bytes();
        let ssid = String::from_utf8_lossy(&bytes[..idx]).trim().to_string();
        if ssid.is_empty() {
            continue;
        }
        let right = String::from_utf8_lossy(&bytes[idx + 17..])
            .trim()
            .to_string();
        let mut cols = right.split_whitespace();
        let rssi = cols.next().unwrap_or("");
        let _channel = cols.next().unwrap_or("");
        let _ht = cols.next().unwrap_or("");
        let _cc = cols.next().unwrap_or("");
        let security = cols.collect::<Vec<&str>>().join(" ");
        let mut detail = "Wi-Fi".to_string();
        if !rssi.is_empty() {
            detail.push_str(&format!(", RSSI {rssi}"));
        }
        if !security.is_empty() {
            detail.push_str(&format!(", {security}"));
        }
        push_unique(items, ssid, detail);
    }
}

fn scan_network_macos_system_profiler(items: &mut Vec<DiscoveredConnection>) {
    let Some(out) = command_output("system_profiler", &["SPAirPortDataType"]) else {
        return;
    };
    let mut in_preferred = false;
    let mut in_current = false;
    let mut in_other_local = false;
    let field_like = |key: &str| {
        matches!(
            key,
            "phy mode"
                | "bssid"
                | "channel"
                | "country code"
                | "network type"
                | "security"
                | "signal / noise"
                | "transmit rate"
                | "mcs index"
                | "locale"
                | "interface"
        )
    };
    for raw in out.lines() {
        let trimmed = raw.trim();
        if trimmed.eq_ignore_ascii_case("Preferred Networks:") {
            in_preferred = true;
            in_current = false;
            in_other_local = false;
            continue;
        }
        if trimmed.eq_ignore_ascii_case("Current Network Information:") {
            in_current = true;
            in_preferred = false;
            in_other_local = false;
            continue;
        }
        if trimmed.eq_ignore_ascii_case("Other Local Wi-Fi Networks:") {
            in_other_local = true;
            in_preferred = false;
            in_current = false;
            continue;
        }
        if trimmed.is_empty() {
            continue;
        }
        let indent = raw.chars().take_while(|c| *c == ' ').count();
        if indent < 8 {
            in_preferred = false;
            in_current = false;
            in_other_local = false;
            continue;
        }
        if !(in_preferred || in_current || in_other_local) {
            continue;
        }
        if trimmed.ends_with(':') {
            let candidate = trimmed.trim_end_matches(':').trim();
            let key = candidate.to_ascii_lowercase();
            if !candidate.is_empty() && !field_like(&key) {
                let detail = if in_current {
                    "Wi-Fi, connected".to_string()
                } else if in_preferred {
                    "Wi-Fi, preferred network".to_string()
                } else {
                    "Wi-Fi, nearby".to_string()
                };
                push_unique(items, candidate.to_string(), detail);
            }
        }
    }
}

fn scan_network_setup_ports(items: &mut Vec<DiscoveredConnection>) {
    let Some(out) = command_output("networksetup", &["-listallhardwareports"]) else {
        return;
    };
    let mut current_port: Option<String> = None;
    let mut current_device: Option<String> = None;

    let mut flush_block = |port: &mut Option<String>, device: &mut Option<String>| {
        let Some(port_name) = port.take() else {
            return;
        };
        let Some(device_name) = device.take() else {
            return;
        };
        if device_name.eq_ignore_ascii_case("lo0") {
            return;
        }
        push_unique(
            items,
            format!("{port_name} ({device_name})"),
            "Interface".to_string(),
        );
    };

    for line in out.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            flush_block(&mut current_port, &mut current_device);
            continue;
        }
        if let Some(value) = trimmed.strip_prefix("Hardware Port:") {
            current_port = Some(value.trim().to_string());
        } else if let Some(value) = trimmed.strip_prefix("Device:") {
            current_device = Some(value.trim().to_string());
        }
    }
    flush_block(&mut current_port, &mut current_device);
}

fn interface_kind(name: &str) -> Option<&'static str> {
    let lowered = name.to_ascii_lowercase();
    if lowered.starts_with("lo") {
        return None;
    }
    if lowered.starts_with("bridge") || lowered.starts_with("thunderbolt") {
        return Some("Thunderbolt");
    }
    if lowered.starts_with("wl") || lowered.starts_with("wlan") || lowered.contains("wifi") {
        return Some("Wi-Fi");
    }
    if cfg!(target_os = "macos") && lowered == "en0" {
        return Some("Network");
    }
    if lowered.starts_with("eth")
        || lowered.starts_with("en")
        || lowered.starts_with("eno")
        || lowered.starts_with("enp")
    {
        return Some("Ethernet");
    }
    Some("Network")
}

fn scan_network_interfaces(items: &mut Vec<DiscoveredConnection>) {
    if let Some(list) = command_output("ifconfig", &["-l"]) {
        for iface in list.split_whitespace() {
            let Some(kind) = interface_kind(iface) else {
                continue;
            };
            push_unique(items, format!("{kind} ({iface})"), "Interface".to_string());
        }
    }

    if let Some(list) = command_output("ip", &["-o", "link", "show"]) {
        for line in list.lines() {
            let Some(raw_name) = line.split(':').nth(1) else {
                continue;
            };
            let iface = raw_name.trim();
            let Some(kind) = interface_kind(iface) else {
                continue;
            };
            push_unique(items, format!("{kind} ({iface})"), "Interface".to_string());
        }
    }
}

fn scan_bluetoothctl(items: &mut Vec<DiscoveredConnection>) {
    let Some(out) = command_output("bluetoothctl", &["devices"]) else {
        return;
    };
    for line in out.lines() {
        let trimmed = line.trim();
        let Some(rest) = trimmed.strip_prefix("Device ") else {
            continue;
        };
        let mut parts = rest.splitn(2, ' ');
        let mac = parts.next().unwrap_or("").trim();
        let name = parts.next().unwrap_or("").trim();
        if name.is_empty() {
            continue;
        }
        let detail = if mac.is_empty() {
            "Paired".to_string()
        } else {
            format!("MAC {mac}")
        };
        push_unique(items, name.to_string(), detail);
    }
}

fn scan_macos_blueutil(items: &mut Vec<DiscoveredConnection>) {
    if let Some(raw) = command_output("blueutil", &["--paired", "--format", "json"]) {
        for (name, mac, connected) in parse_blueutil_json_devices(&raw, false) {
            let mut detail = if mac.is_empty() {
                "Paired device".to_string()
            } else {
                format!("MAC {mac}")
            };
            if connected {
                detail.push_str(", connected");
            }
            push_unique(items, name, detail);
        }
    }
    if let Some(raw) = command_output("blueutil", &["--connected", "--format", "json"]) {
        for (name, mac, _) in parse_blueutil_json_devices(&raw, true) {
            let mut detail = if mac.is_empty() {
                "Paired device".to_string()
            } else {
                format!("MAC {mac}")
            };
            detail.push_str(", connected");
            push_unique(items, name, detail);
        }
    }
}

fn scan_macos_bluetooth(items: &mut Vec<DiscoveredConnection>) {
    for (name, mac, connected) in macos_bluetooth_devices() {
        let mut detail = if mac.is_empty() {
            "Paired device".to_string()
        } else {
            format!("MAC {mac}")
        };
        if connected {
            detail.push_str(", connected");
        }
        push_unique(items, name, detail);
    }
}
