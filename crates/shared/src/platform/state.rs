use super::addons::AddonId;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AddonStateOverrides {
    #[serde(default)]
    pub addons: BTreeMap<AddonId, AddonStateOverride>,
}

impl AddonStateOverrides {
    pub fn enabled_for(&self, addon_id: &AddonId) -> Option<bool> {
        self.addons.get(addon_id).and_then(|state| state.enabled)
    }

    pub fn set_enabled(&mut self, addon_id: AddonId, enabled: Option<bool>) {
        match enabled {
            Some(enabled) => {
                self.addons.insert(
                    addon_id,
                    AddonStateOverride {
                        enabled: Some(enabled),
                    },
                );
            }
            None => {
                self.addons.remove(&addon_id);
            }
        }
    }

    pub fn is_empty(&self) -> bool {
        self.addons.is_empty()
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AddonStateOverride {
    #[serde(default)]
    pub enabled: Option<bool>,
}

#[cfg(test)]
mod tests {
    use super::AddonStateOverrides;
    use crate::platform::AddonId;

    #[test]
    fn state_overrides_set_and_clear_enablement() {
        let addon_id = AddonId::from("shell.editor");
        let mut overrides = AddonStateOverrides::default();

        assert_eq!(overrides.enabled_for(&addon_id), None);

        overrides.set_enabled(addon_id.clone(), Some(false));
        assert_eq!(overrides.enabled_for(&addon_id), Some(false));

        overrides.set_enabled(addon_id.clone(), Some(true));
        assert_eq!(overrides.enabled_for(&addon_id), Some(true));

        overrides.set_enabled(addon_id.clone(), None);
        assert_eq!(overrides.enabled_for(&addon_id), None);
        assert!(overrides.is_empty());
    }
}
