use super::addons::{AddonId, CapabilityId, PermissionId};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "target")]
pub enum LaunchTarget {
    Addon { addon_id: AddonId },
    Capability { capability: CapabilityId },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum LaunchSurface {
    Desktop,
    Terminal,
    Background,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "kind")]
pub enum ShellAction {
    Launch {
        target: LaunchTarget,
        surface: LaunchSurface,
        payload: Option<Value>,
    },
    OpenPath {
        path: PathBuf,
        preferred_capability: Option<CapabilityId>,
    },
    FocusWindow {
        window_id: String,
    },
    CloseWindow {
        window_id: String,
    },
    DispatchAddonAction {
        addon_id: AddonId,
        action: String,
        payload: Value,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "kind")]
pub enum ShellEvent {
    AddonLaunched {
        addon_id: AddonId,
        surface: LaunchSurface,
    },
    WindowOpened {
        window_id: String,
        addon_id: AddonId,
    },
    WindowClosed {
        window_id: String,
        addon_id: AddonId,
    },
    ActionRequested {
        source_addon: AddonId,
        action: String,
        payload: Value,
    },
    PermissionRequested {
        addon_id: AddonId,
        permission: PermissionId,
    },
    Status {
        addon_id: Option<AddonId>,
        code: String,
        message: String,
    },
}
