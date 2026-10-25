//! `.plugins/.store_installed`: what the store installed, in the player's format.
//!
//! ```text
//! P <id> <version>
//! F<TAB><dest><TAB><sha256><TAB><keep:0|1>
//! ```

use std::collections::HashSet;
use std::fs;

use super::index::{Plugin, MAX_FILES};
use super::{dest, validate};
use crate::card::Card;
use crate::error::Result;
use crate::fs::write_atomic;
use crate::version;

/// Most plugins the record holds; larger than the catalog so retired plugins
/// stay manageable.
pub const MAX_RECORDED: usize = 400;
const MAX_LINE: usize = 511;
const FILE_NAME: &str = ".store_installed";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordedFile {
    pub dest: String,
    pub sha256: String,
    pub keep: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordedPlugin {
    pub id: String,
    pub version: String,
    pub files: Vec<RecordedFile>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Record {
    pub plugins: Vec<RecordedPlugin>,
}

impl Record {
    /// An absent, unsafe or malformed record reads as empty, like the player.
    pub fn read(card: &Card) -> Self {
        let path = card.plugins_dir().join(FILE_NAME);
        let is_regular = |p: &std::path::Path| fs::symlink_metadata(p).is_ok_and(|m| m.is_file());
        if !fs::symlink_metadata(card.plugins_dir()).is_ok_and(|m| m.is_dir()) || !is_regular(&path)
        {
            return Self::default();
        }
        fs::read_to_string(&path)
            .ok()
            .and_then(|text| Self::parse(&text))
            .unwrap_or_default()
    }

    pub fn write(&self, card: &Card) -> Result<()> {
        let path = card.ensure_plugins_dir()?.join(FILE_NAME);
        write_atomic(&path, self.render().as_bytes())
    }

    pub fn get(&self, id: &str) -> Option<&RecordedPlugin> {
        self.plugins.iter().find(|p| p.id == id)
    }

    /// The record after installing `plugin`: it replaces any earlier entry of
    /// the same id and takes over destinations other entries listed; an entry
    /// left without files is dropped (`write_record_after`).
    pub fn with_installed(&self, plugin: &Plugin) -> Self {
        let mut plugins: Vec<RecordedPlugin> = self
            .plugins
            .iter()
            .filter(|p| p.id != plugin.id)
            .map(|p| RecordedPlugin {
                files: p
                    .files
                    .iter()
                    .filter(|f| !plugin.dests().any(|d| d.eq_ignore_ascii_case(&f.dest)))
                    .cloned()
                    .collect(),
                ..p.clone()
            })
            .filter(|p| !p.files.is_empty())
            .collect();
        plugins.push(RecordedPlugin {
            id: plugin.id.clone(),
            version: plugin.version.clone(),
            files: plugin
                .files
                .iter()
                .map(|f| RecordedFile {
                    dest: f.dest.clone(),
                    sha256: f.sha256.clone(),
                    keep: f.keep,
                })
                .collect(),
        });
        Self { plugins }
    }

    pub fn without(&self, id: &str) -> Self {
        Self {
            plugins: self.plugins.iter().filter(|p| p.id != id).cloned().collect(),
        }
    }

    /// Whether `dest` with this hash was installed by plugin `owner`.
    pub fn tracks(&self, owner: &str, dest: &str, sha256: &str) -> bool {
        self.get(owner).is_some_and(|p| {
            p.files
                .iter()
                .any(|f| f.dest.eq_ignore_ascii_case(dest) && f.sha256 == sha256)
        })
    }

    fn render(&self) -> String {
        let mut out = String::new();
        for p in &self.plugins {
            out.push_str(&format!("P {} {}\n", p.id, p.version));
            for f in &p.files {
                out.push_str(&format!("F\t{}\t{}\t{}\n", f.dest, f.sha256, u8::from(f.keep)));
            }
        }
        out
    }

    fn parse(text: &str) -> Option<Self> {
        let mut plugins: Vec<RecordedPlugin> = Vec::new();
        let mut dests = HashSet::new();
        for line in text.split_inclusive('\n') {
            let line = line.strip_suffix('\n').unwrap_or(line);
            if line.len() > MAX_LINE {
                return None;
            }
            if let Some(rest) = line.strip_prefix("P ") {
                if plugins.last().is_some_and(|p| p.files.is_empty()) || plugins.len() >= MAX_RECORDED
                {
                    return None;
                }
                let fields: Vec<&str> = rest.split_whitespace().collect();
                let [id, ver] = fields[..] else { return None };
                if !validate::is_id(id)
                    || !version::is_valid(ver)
                    || plugins.iter().any(|p| p.id == id)
                {
                    return None;
                }
                plugins.push(RecordedPlugin {
                    id: id.into(),
                    version: ver.into(),
                    files: Vec::new(),
                });
            } else if let Some(rest) = line.strip_prefix("F\t") {
                let [d, sha, keep] = rest.split('\t').collect::<Vec<_>>()[..] else {
                    return None;
                };
                let keep = match keep {
                    "0" => false,
                    "1" => true,
                    _ => return None,
                };
                let plugin = plugins.last_mut()?;
                if !dest::is_valid(d)
                    || !validate::is_sha256(sha)
                    || plugin.files.len() >= MAX_FILES
                    || !dests.insert(d.to_ascii_lowercase())
                {
                    return None;
                }
                plugin.files.push(RecordedFile {
                    dest: d.into(),
                    sha256: sha.into(),
                    keep,
                });
            } else {
                return None;
            }
        }
        plugins
            .last()
            .map_or(true, |p| !p.files.is_empty())
            .then_some(Self { plugins })
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_fixtures::spec;
    use super::*;
    use crate::test_support::{fake_card, write_file};

    fn sha(c: char) -> String {
        c.to_string().repeat(64)
    }

    fn sample() -> String {
        format!(
            "P a.one 1.0\nF\t.plugins/A.lua\t{}\t0\nF\tData/a.bin\t{}\t1\nP b 2\nF\t.plugins/B.lua\t{}\t0\n",
            sha('a'),
            sha('b'),
            sha('c')
        )
    }

    #[test]
    fn round_trips_through_the_card() {
        let (dir, card) = fake_card();
        write_file(dir.path(), ".plugins/.store_installed", sample().as_bytes());
        let record = Record::read(&card);
        assert_eq!(record.plugins.len(), 2);
        assert_eq!(record.plugins[0].files[1].keep, true);
        record.write(&card).unwrap();
        let text = std::fs::read_to_string(dir.path().join(".plugins/.store_installed")).unwrap();
        assert_eq!(text, sample());
    }

    #[test]
    fn any_violation_reads_as_empty() {
        let (dir, card) = fake_card();
        let a = format!("F\t.plugins/A.lua\t{}\t0\n", sha('a'));
        let bad = [
            "P a 1\n".to_string(),                                   // no files
            format!("{a}P a 1\n"),                                    // F before P
            format!("P a 1\n{a}P a 2\n{a}"),                          // duplicate id
            format!("P A 1\n{a}"),                                    // bad id
            format!("P a 1.x\n{a}"),                                  // bad version
            format!("P a 1\n{a}{a}"),                                 // repeated dest
            format!("P a 1\n{a}P b 1\n{a}"),                          // dest shared across plugins
            format!("P a 1\nF\t.plugins/A.lua\t{}\t2\n", sha('a')),   // bad keep
            format!("P a 1\nF\t.compas/x\t{}\t0\n", sha('a')),        // bad dest
            format!("P a 1 extra\n{a}"),                              // extra field
            format!("X\nP a 1\n{a}"),                                 // unknown line
        ];
        for text in bad {
            write_file(dir.path(), ".plugins/.store_installed", text.as_bytes());
            assert!(Record::read(&card).plugins.is_empty(), "{text:?}");
        }
    }

    #[test]
    fn last_line_may_lack_a_newline() {
        let text = format!("P a 1\nF\t.plugins/A.lua\t{}\t0", sha('a'));
        assert_eq!(Record::parse(&text).unwrap().plugins.len(), 1);
    }

    #[test]
    fn installing_takes_over_destinations_and_drops_empty_entries() {
        let (dir, card) = fake_card();
        write_file(dir.path(), ".plugins/.store_installed", sample().as_bytes());
        let mut plugin = spec("new", "1.0", &[(".plugins/B.lua", "x", false)]).plugin;
        let next = Record::read(&card).with_installed(&plugin);
        let ids: Vec<_> = next.plugins.iter().map(|p| p.id.as_str()).collect();
        assert_eq!(ids, ["a.one", "new"], "b lost its only file");

        plugin.id = "a.one".into();
        plugin.version = "2.0".into();
        let next = Record::read(&card).with_installed(&plugin);
        assert_eq!(next.get("a.one").unwrap().version, "2.0");
        assert_eq!(next.plugins.last().unwrap().id, "a.one");
        assert!(next.tracks("a.one", ".PLUGINS/b.lua", &plugin.files[0].sha256));
    }
}
