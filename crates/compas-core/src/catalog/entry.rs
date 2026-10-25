use serde::Serialize;

use super::classify::classify;
use super::record::{Record, RecordedPlugin};
use super::state::{self, PluginState};
use super::{dest, disabled, Catalog};
use crate::card::Card;
use crate::error::Result;

/// One row of the UI: a catalog plugin (or a recorded one the catalog no
/// longer offers) evaluated against the card.
#[derive(Debug, Clone, Serialize)]
pub struct Entry {
    pub id: String,
    pub name: String,
    /// Catalog version, or the recorded one for a removed plugin.
    pub version: String,
    pub installed_version: Option<String>,
    pub state: PluginState,
    pub incompatible: bool,
    pub api_min: Option<u32>,
    pub description: String,
    pub category: String,
    pub author: String,
    pub size: Option<u64>,
    pub player_layout: bool,
    pub theme_pack: bool,
    pub has_preview: bool,
    /// Key of the `.disabled` list; absent when the plugin has no script.
    pub script: Option<String>,
    pub enabled: bool,
}

pub fn list(catalog: &Catalog, card: &Card, plugin_api: Option<u32>) -> Result<Vec<Entry>> {
    let record = Record::read(card);
    let disabled = disabled::read(card)?;
    let is_enabled = |script: &Option<String>| {
        script
            .as_ref()
            .is_none_or(|s| !disabled.iter().any(|d| d == s))
    };

    let mut entries: Vec<Entry> = catalog
        .plugins
        .iter()
        .map(|p| {
            let installed = record.get(&p.id).map(|r| r.version.as_str());
            let script = p.script().map(str::to_string);
            let class = classify(p.dests());
            Entry {
                id: p.id.clone(),
                name: p.name.clone(),
                version: p.version.clone(),
                installed_version: installed.map(str::to_string),
                state: state::compute(p, installed, card, plugin_api),
                incompatible: state::is_incompatible(p, plugin_api),
                api_min: Some(p.api_min),
                description: p.description.clone(),
                category: p.category.clone(),
                author: p.author.clone(),
                size: Some(p.size),
                player_layout: class.player_layout,
                theme_pack: class.theme_pack,
                has_preview: p.preview.is_some(),
                enabled: is_enabled(&script),
                script,
            }
        })
        .collect();

    entries.extend(
        record
            .plugins
            .iter()
            .filter(|r| catalog.get(&r.id).is_err())
            .map(|r| removed_entry(r, is_enabled)),
    );
    Ok(entries)
}

fn removed_entry(r: &RecordedPlugin, is_enabled: impl Fn(&Option<String>) -> bool) -> Entry {
    let class = classify(r.files.iter().map(|f| f.dest.as_str()));
    let script = r
        .files
        .iter()
        .find_map(|f| dest::script_name(&f.dest))
        .map(str::to_string);
    Entry {
        id: r.id.clone(),
        name: r.id.clone(),
        version: r.version.clone(),
        installed_version: Some(r.version.clone()),
        state: PluginState::Removed,
        incompatible: false,
        api_min: None,
        description: String::new(),
        category: String::new(),
        author: String::new(),
        size: None,
        player_layout: class.player_layout,
        theme_pack: class.theme_pack,
        has_preview: false,
        enabled: is_enabled(&script),
        script,
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_fixtures::{publish, spec};
    use super::*;
    use crate::test_support::{fake_card, write_file};

    #[tokio::test]
    async fn lists_catalog_and_removed_rows_with_flags() {
        let (dir, card) = fake_card();
        let layout = spec(
            "lay",
            "1.0",
            &[
                (".plugins/L.lua", "l", false),
                (".plugins/L/player_layouts/l.xml", "x", false),
            ],
        );
        let theme = spec(
            "thm",
            "1.0",
            &[
                (".plugins/T.lua", "t", false),
                ("Themes/Night.theme", "n", false),
            ],
        );
        let (_server, catalog) = publish(&[layout, theme]).await;

        let gone = spec("gone", "0.9", &[(".plugins/Gone.lua", "g", false)]).plugin;
        crate::catalog::record::Record::default()
            .with_installed(&gone)
            .write(&card)
            .unwrap();
        write_file(dir.path(), ".plugins/.disabled", b"Gone.lua\n");
        write_file(dir.path(), ".plugins/T.lua", b"mine");

        let rows = list(&catalog, &card, Some(17)).unwrap();
        let by_id = |id: &str| rows.iter().find(|r| r.id == id).unwrap();
        assert_eq!(rows.len(), 3);
        assert!(by_id("lay").player_layout && !by_id("lay").theme_pack);
        assert!(by_id("thm").theme_pack && !by_id("thm").player_layout);
        assert_eq!(by_id("thm").state, PluginState::Manual);
        assert_eq!(by_id("gone").state, PluginState::Removed);
        assert!(!by_id("gone").enabled);
        assert!(by_id("lay").enabled);
    }
}
