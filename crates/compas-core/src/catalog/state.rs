//! A catalog plugin's state against a card (`plugin_store_compute_state`).

use std::fs;

use serde::Serialize;

use super::index::Plugin;
use crate::card::Card;
use crate::version;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginState {
    Available,
    Installed,
    Update,
    /// Its script is on the card but the store did not install it.
    Manual,
    Incompatible,
    /// Recorded as installed but no longer in the catalog.
    Removed,
}

/// `plugin_api` is the card's API level when `device.json` tells us.
pub fn is_incompatible(plugin: &Plugin, plugin_api: Option<u32>) -> bool {
    plugin_api.is_some_and(|api| plugin.api_min > api)
}

pub fn compute(
    plugin: &Plugin,
    installed_version: Option<&str>,
    card: &Card,
    plugin_api: Option<u32>,
) -> PluginState {
    let incompatible = is_incompatible(plugin, plugin_api);
    if let Some(installed) = installed_version {
        let newer = version::compare(&plugin.version, installed).is_gt();
        return if newer && !incompatible {
            PluginState::Update
        } else {
            PluginState::Installed
        };
    }
    let script_present = plugin
        .script()
        .is_some_and(|name| fs::symlink_metadata(card.plugins_dir().join(name)).is_ok_and(|m| m.is_file()));
    if script_present {
        PluginState::Manual
    } else if incompatible {
        PluginState::Incompatible
    } else {
        PluginState::Available
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_fixtures::spec;
    use super::*;
    use crate::test_support::{fake_card, write_file};

    #[test]
    fn states() {
        let (dir, card) = fake_card();
        let mut p = spec("a", "1.1", &[(".plugins/A.lua", "x", false)]).plugin;

        assert_eq!(compute(&p, None, &card, Some(17)), PluginState::Available);
        assert_eq!(compute(&p, Some("1.0"), &card, Some(17)), PluginState::Update);
        assert_eq!(compute(&p, Some("1.1"), &card, Some(17)), PluginState::Installed);
        assert_eq!(compute(&p, Some("2.0"), &card, Some(17)), PluginState::Installed);

        p.api_min = 18;
        assert_eq!(compute(&p, None, &card, Some(17)), PluginState::Incompatible);
        assert_eq!(compute(&p, Some("1.0"), &card, Some(17)), PluginState::Installed);
        assert_eq!(compute(&p, None, &card, None), PluginState::Available, "unknown api");

        write_file(dir.path(), ".plugins/A.lua", b"hand made");
        assert_eq!(compute(&p, None, &card, None), PluginState::Manual);
    }
}
