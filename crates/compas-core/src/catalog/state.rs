//! A catalog plugin's state against a card (`plugin_store_compute_state`).

use std::fs;

use serde::Serialize;

use super::index::Plugin;
use crate::card::Card;
use crate::fs::sha256_file;
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

/// What the card holds of a plugin's files, whoever put them there.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Local {
    /// Some file of the plugin is on the card, with whatever content.
    pub present: bool,
    /// Every file that carries the plugin's content is on the card with the catalog's exact
    /// bytes (the entry script alone does not decide it: a theme works from its `.theme` file).
    pub matches: bool,
}

/// A file on the card, asked for with the size it should have: its size and SHA-256, or `None`
/// when there is no such regular file. The hash may be left empty when the size differs.
pub type Probe<'a> = dyn Fn(&str, u64) -> Option<(u64, String)> + 'a;

/// Looks at the plugin's files through `probe`.
pub fn local_files(plugin: &Plugin, probe: &Probe<'_>) -> Local {
    let facts: Vec<(bool, bool)> = plugin
        .files
        .iter()
        .map(|file| {
            let found = probe(&file.dest, file.size);
            let matches = found
                .as_ref()
                .is_some_and(|(size, sha)| *size == file.size && *sha == file.sha256);
            (found.is_some(), matches)
        })
        .collect();
    let is_script = |i: usize| super::dest::script_name(&plugin.files[i].dest).is_some();
    let content: Vec<usize> = (0..facts.len()).filter(|i| !is_script(*i)).collect();
    let required = if content.is_empty() {
        (0..facts.len()).collect()
    } else {
        content
    };
    Local {
        present: facts.iter().any(|(found, _)| *found),
        matches: !required.is_empty() && required.iter().all(|i| facts[*i].1),
    }
}

/// The probe for a real card: size first, so a file of another size is never read.
pub fn card_probe(card: &Card) -> impl Fn(&str, u64) -> Option<(u64, String)> + '_ {
    move |dest, wanted| {
        let path = card.resolve(dest, false).ok()?;
        let meta = fs::symlink_metadata(&path).ok().filter(|m| m.is_file())?;
        let sha = if meta.len() == wanted {
            sha256_file(&path).unwrap_or_default()
        } else {
            String::new()
        };
        Some((meta.len(), sha))
    }
}

pub fn compute(
    plugin: &Plugin,
    installed_version: Option<&str>,
    local: Local,
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
    if local.present {
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

    fn local_of(plugin: &Plugin, card: &Card) -> Local {
        local_files(plugin, &card_probe(card))
    }

    #[test]
    fn states() {
        let (dir, card) = fake_card();
        let mut p = spec("a", "1.1", &[(".plugins/A.lua", "x", false)]).plugin;
        let none = local_of(&p, &card);

        assert_eq!(compute(&p, None, none, Some(17)), PluginState::Available);
        assert_eq!(
            compute(&p, Some("1.0"), none, Some(17)),
            PluginState::Update
        );
        assert_eq!(
            compute(&p, Some("1.1"), none, Some(17)),
            PluginState::Installed
        );
        assert_eq!(
            compute(&p, Some("2.0"), none, Some(17)),
            PluginState::Installed
        );

        p.api_min = 18;
        assert_eq!(compute(&p, None, none, Some(17)), PluginState::Incompatible);
        assert_eq!(
            compute(&p, Some("1.0"), none, Some(17)),
            PluginState::Installed
        );
        assert_eq!(
            compute(&p, None, none, None),
            PluginState::Available,
            "unknown api"
        );

        write_file(dir.path(), ".plugins/A.lua", b"hand made");
        let local = local_of(&p, &card);
        assert_eq!(compute(&p, None, local, None), PluginState::Manual);
        assert!(local.present && !local.matches, "other content");
        write_file(dir.path(), ".plugins/A.lua", b"x");
        assert!(local_of(&p, &card).matches, "the catalog's own bytes");
    }

    #[test]
    fn a_theme_counts_without_its_entry_script() {
        let (dir, card) = fake_card();
        let theme = spec(
            "t",
            "1.0",
            &[
                (".plugins/ThemeT.lua", "script", false),
                ("Themes/T.theme", "colours", false),
                ("Themes/T.png", "picture", false),
            ],
        )
        .plugin;
        write_file(dir.path(), "Themes/T.theme", b"colours");
        assert!(!local_of(&theme, &card).matches, "the picture is missing");
        write_file(dir.path(), "Themes/T.png", b"picture");
        let local = local_of(&theme, &card);
        assert!(local.matches && local.present);
        write_file(dir.path(), "Themes/T.png", b"another picture");
        let local = local_of(&theme, &card);
        assert!(
            !local.matches && local.present,
            "changed content is not a match"
        );
    }
}
