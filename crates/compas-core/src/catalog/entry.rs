use serde::Serialize;

use super::classify::classify;
use super::index::Plugin;
use super::record::{Record, RecordedPlugin};
use super::state::{self, PluginState};
use super::{disabled, Catalog};
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
    /// The card holds this plugin's content byte for byte, whoever put it there.
    pub files_match: bool,
    /// Another installed package (a bundle, say) owns files of this plugin; its name.
    pub installed_by: Option<String>,
}

pub fn list(catalog: &Catalog, card: &Card, plugin_api: Option<u32>) -> Result<Vec<Entry>> {
    let record = Record::read(card)?;
    let disabled_names = disabled::read(card)?;
    Ok(build(
        catalog,
        &record,
        &disabled_names,
        &state::card_probe(card),
        plugin_api,
    ))
}

/// The rows for a catalog, a record, the `.disabled` list and a view of the card's files.
fn build(
    catalog: &Catalog,
    record: &Record,
    disabled_names: &[Vec<u8>],
    probe: &state::Probe<'_>,
    plugin_api: Option<u32>,
) -> Vec<Entry> {
    let is_enabled = |script: &Option<String>| {
        script
            .as_ref()
            .is_none_or(|s| !disabled::contains(disabled_names, s))
    };
    let owner_of = |p: &Plugin| {
        record
            .plugins
            .iter()
            .filter(|r| r.id != p.id)
            .find(|r| {
                r.files
                    .iter()
                    .any(|f| p.dests().any(|d| d.eq_ignore_ascii_case(&f.dest)))
            })
            .map(|r| {
                catalog
                    .get(&r.id)
                    .map_or_else(|_| r.id.clone(), |o| o.name.clone())
            })
    };

    let mut entries: Vec<Entry> = catalog
        .plugins
        .iter()
        .map(|p| {
            let recorded = record.get(&p.id);
            let installed = recorded.map(|r| r.version.as_str());
            // What is on the card is what the player sees; a renamed script
            // in a newer catalog version must not change the key.
            let script = recorded
                .map_or_else(|| p.script(), RecordedPlugin::script)
                .map(str::to_string);
            let class = classify(p.dests());
            let local = state::local_files(p, probe);
            Entry {
                id: p.id.clone(),
                name: p.name.clone(),
                version: p.version.clone(),
                installed_version: installed.map(str::to_string),
                state: state::compute(p, installed, local, plugin_api),
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
                files_match: local.matches,
                installed_by: if recorded.is_none() {
                    owner_of(p)
                } else {
                    None
                },
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
    entries
}

fn removed_entry(r: &RecordedPlugin, is_enabled: impl Fn(&Option<String>) -> bool) -> Entry {
    let class = classify(r.files.iter().map(|f| f.dest.as_str()));
    let script = r.script().map(str::to_string);
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
        files_match: false,
        installed_by: None,
    }
}

#[cfg(test)]
mod tests;
