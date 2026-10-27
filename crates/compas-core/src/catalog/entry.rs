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

    #[tokio::test]
    async fn a_renamed_script_keeps_the_installed_name_as_disabled_key() {
        use super::super::install::{install, update};

        let (_dir, card) = fake_card();
        let (_s1, c1) = publish(&[spec("p", "1.0", &[(".plugins/OldName.lua", "a", false)])]).await;
        install(&card, &c1, "p", false, &|_| {}).await.unwrap();
        crate::catalog::set_enabled(&card, "OldName.lua", false).unwrap();

        let (_s2, c2) = publish(&[spec("p", "1.1", &[(".plugins/NewName.lua", "b", false)])]).await;
        let rows = list(&c2, &card, None).unwrap();
        assert_eq!(rows[0].script.as_deref(), Some("OldName.lua"));
        assert!(!rows[0].enabled);

        update(&card, &c2, "p", false, &|_| {}).await.unwrap();
        let rows = list(&c2, &card, None).unwrap();
        assert_eq!(rows[0].script.as_deref(), Some("NewName.lua"));
        assert!(
            rows[0].enabled,
            "the new file name is not in the disabled list"
        );
    }

    /// The metadata of a real player card (record and file facts, no music): its themes were
    /// installed through the old "Themes" bundle, one through the individual theme.
    #[test]
    fn themes_installed_through_a_bundle_show_as_present_and_not_as_available() {
        use std::collections::HashMap;

        let index = include_bytes!("fixtures/index-2026-10-26.json");
        let catalog = Catalog::parse(index, "https://example.test").unwrap();
        let record = Record::parse(include_str!("fixtures/store_installed")).unwrap();
        let facts: HashMap<&str, (u64, &str)> = include_str!("fixtures/card_files.tsv")
            .lines()
            .map(|l| {
                let mut f = l.split('\t');
                (
                    f.next().unwrap(),
                    (f.next().unwrap().parse().unwrap(), f.next().unwrap()),
                )
            })
            .collect();
        let probe = |dest: &str, _| {
            facts
                .get(dest)
                .map(|(size, sha)| (*size, (*sha).to_string()))
        };

        let rows = build(&catalog, &record, &[], &probe, Some(17));
        let themes: Vec<&Entry> = rows
            .iter()
            .filter(|r| r.id.starts_with("org.compas.theme."))
            .collect();
        assert_eq!(themes.len(), 21);
        for t in &themes {
            if t.id == "org.compas.theme.cherry_noir" {
                assert_eq!(t.state, PluginState::Installed, "installed on its own");
                assert_eq!(t.installed_by, None);
            } else {
                assert_eq!(t.state, PluginState::Manual, "{}", t.id);
                assert!(t.files_match, "{} has the catalog's exact files", t.id);
                assert_eq!(
                    t.installed_by.as_deref(),
                    Some("example.themes"),
                    "{}",
                    t.id
                );
            }
            assert_ne!(t.state, PluginState::Available, "{} is on the card", t.id);
        }
        let bundle = rows.iter().find(|r| r.id == "example.themes").unwrap();
        assert_eq!(bundle.state, PluginState::Removed);
        // No duplicates: every id appears once.
        let mut ids: Vec<_> = rows.iter().map(|r| r.id.as_str()).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), rows.len());
    }

    #[tokio::test]
    async fn a_bundle_over_individually_installed_packages_is_present_once() {
        use super::super::install::install;

        let (_dir, card) = fake_card();
        let single = |id: &str, name: &str| {
            spec(
                id,
                "1.0",
                &[
                    (&format!(".plugins/{name}.lua"), "s", false),
                    (&format!("Themes/{name}.theme"), name, false),
                ],
            )
        };
        let bundle = spec(
            "bundle",
            "1.0",
            &[
                (".plugins/Bundle.lua", "b", false),
                ("Themes/A.theme", "A", false),
                ("Themes/B.theme", "B", false),
            ],
        );
        // The catalog never lists a bundle next to its parts, but it can swap one for the other.
        let (_s1, parts) = publish(&[single("a", "A"), single("b", "B")]).await;
        install(&card, &parts, "a", false, &|_| {}).await.unwrap();
        install(&card, &parts, "b", false, &|_| {}).await.unwrap();

        let (_s2, catalog) = publish(&[bundle]).await;
        let rows = list(&catalog, &card, None).unwrap();
        let by_id = |id: &str| rows.iter().find(|r| r.id == id).unwrap();
        assert_eq!(
            rows.len(),
            3,
            "the bundle and the two recorded parts, once each"
        );
        assert_eq!(by_id("a").state, PluginState::Removed);
        assert_eq!(by_id("b").state, PluginState::Removed);
        let bundle = by_id("bundle");
        assert_eq!(bundle.state, PluginState::Manual);
        assert!(bundle.files_match);
        assert_eq!(bundle.installed_by.as_deref(), Some("a"));
    }
}
