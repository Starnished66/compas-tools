//! The catalog `index.json` and its validation (`plugin_store_parse_index`).

use std::collections::HashSet;

use serde::{Deserialize, Deserializer, Serialize};

use super::{dest, validate};
use crate::error::{Error, Result};
use crate::version;

pub const MAX_PLUGINS: usize = 200;
pub const MAX_FILES: usize = 32;
const MAX_FILE_BYTES: u64 = 1 << 20;
const MAX_PREVIEW_BYTES: u64 = 65536;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Plugin {
    pub id: String,
    pub name: String,
    pub version: String,
    #[serde(deserialize_with = "uint")]
    pub api_min: u32,
    pub description: String,
    pub category: String,
    pub author: String,
    #[serde(deserialize_with = "uint")]
    pub size: u64,
    #[serde(default)]
    pub preview: Option<Preview>,
    pub files: Vec<PluginFile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginFile {
    pub asset: String,
    pub dest: String,
    pub sha256: String,
    #[serde(deserialize_with = "uint")]
    pub size: u64,
    #[serde(default)]
    pub keep: bool,
}

/// Preview image metadata. Only the publisher's default image is used.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Preview {
    pub asset: String,
    pub sha256: String,
    #[serde(deserialize_with = "uint")]
    pub size: u64,
    #[serde(deserialize_with = "uint")]
    pub width: u32,
    #[serde(deserialize_with = "uint")]
    pub height: u32,
    #[serde(default)]
    pub(super) variants: Vec<Preview>,
}

#[derive(Deserialize)]
pub(super) struct RawIndex {
    #[serde(deserialize_with = "uint")]
    schema: u32,
    pub(super) tag: String,
    pub(super) plugins: Vec<Plugin>,
}

impl Plugin {
    pub fn dests(&self) -> impl Iterator<Item = &str> + Clone {
        self.files.iter().map(|f| f.dest.as_str())
    }

    /// File name of the plugin script, the key of the `.disabled` list.
    pub fn script(&self) -> Option<&str> {
        self.dests().find_map(dest::script_name)
    }
}

pub(super) fn parse(bytes: &[u8]) -> Result<RawIndex> {
    let value = super::json::parse(bytes).map_err(invalid)?;
    let index: RawIndex = serde_json::from_value(value).map_err(invalid)?;
    if index.schema != 1 {
        return Err(invalid("unsupported schema"));
    }
    if !validate::is_token(&index.tag, 63) {
        return Err(invalid("bad tag"));
    }
    if index.plugins.len() > MAX_PLUGINS {
        return Err(invalid("too many plugins"));
    }
    let mut ids = HashSet::new();
    for (i, plugin) in index.plugins.iter().enumerate() {
        check_plugin(plugin).map_err(|why| invalid(format!("{}: {why}", plugin.id)))?;
        if !ids.insert(plugin.id.as_str()) {
            return Err(invalid(format!("duplicate id {}", plugin.id)));
        }
        for other in &index.plugins[..i] {
            if plugin
                .dests()
                .any(|a| other.dests().any(|b| dest::collide(a, b)))
            {
                return Err(invalid(format!("{} overlaps {}", plugin.id, other.id)));
            }
        }
    }
    Ok(index)
}

/// Unsigned 32-bit integers, also written as `15.0` (`store_json_uint`).
fn uint<'de, D, T>(deserializer: D) -> std::result::Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: TryFrom<u64>,
{
    let number = f64::deserialize(deserializer)?;
    let integral = number.fract() == 0.0 && (0.0..=f64::from(u32::MAX)).contains(&number);
    let value = if integral {
        T::try_from(number as u64).ok()
    } else {
        None
    };
    value.ok_or_else(|| serde::de::Error::custom("expected an unsigned 32-bit integer"))
}

fn invalid(why: impl ToString) -> Error {
    Error::InvalidIndex(why.to_string())
}

fn check_plugin(p: &Plugin) -> std::result::Result<(), &'static str> {
    let ok = validate::is_id(&p.id)
        && validate::is_safe_text(&p.name, 64, false)
        && version::is_valid(&p.version)
        && validate::is_safe_text(&p.description, 400, true)
        && validate::is_safe_text(&p.category, 32, true)
        && validate::is_safe_text(&p.author, 64, true)
        && p.api_min >= 1;
    if !ok {
        return Err("invalid metadata");
    }
    if let Some(preview) = &p.preview {
        check_preview(preview)?;
    }
    if p.files.is_empty() || p.files.len() > MAX_FILES {
        return Err("bad file count");
    }
    let mut scripts = 0;
    for (i, f) in p.files.iter().enumerate() {
        let ok = validate::is_token(&f.asset, 127)
            && dest::is_valid(&f.dest)
            && validate::is_sha256(&f.sha256)
            && (1..=MAX_FILE_BYTES).contains(&f.size);
        if !ok {
            return Err("invalid file entry");
        }
        scripts += usize::from(dest::script_name(&f.dest).is_some());
        if p.files[..i].iter().any(|o| dest::collide(&f.dest, &o.dest)) {
            return Err("colliding destinations");
        }
    }
    if scripts != 1 {
        return Err("needs exactly one script");
    }
    if p.files.iter().map(|f| f.size).sum::<u64>() != p.size {
        return Err("size does not match files");
    }
    Ok(())
}

fn check_preview(p: &Preview) -> std::result::Result<(), &'static str> {
    let ok = validate::is_token(&p.asset, 127)
        && validate::is_sha256(&p.sha256)
        && (1..=MAX_PREVIEW_BYTES).contains(&p.size)
        && (1..=240).contains(&p.width)
        && (1..=400).contains(&p.height);
    if !ok || p.variants.len() > 3 || p.variants.iter().any(|v| check_preview(v).is_err()) {
        return Err("invalid preview");
    }
    Ok(())
}

#[cfg(test)]
pub(super) mod tests {
    use serde_json::{json, Value};

    use super::*;

    pub fn sha(c: char) -> String {
        c.to_string().repeat(64)
    }

    pub fn plugin_json(id: &str, dest: &str, size: u64) -> Value {
        json!({
            "id": id, "name": "Name", "version": "1.0", "api_min": 10,
            "description": "d", "category": "c", "author": "a", "size": size,
            "files": [{ "asset": format!("{id}.bin"), "dest": dest, "sha256": sha('a'), "size": size }]
        })
    }

    fn index(plugins: Vec<Value>) -> Vec<u8> {
        json!({ "schema": 1, "tag": "v1", "plugins": plugins })
            .to_string()
            .into_bytes()
    }

    #[test]
    fn accepts_a_valid_index() {
        let mut p = plugin_json("a", ".plugins/A.lua", 10);
        p["preview"] =
            json!({ "asset": "p.jpg", "sha256": sha('b'), "size": 5, "width": 10, "height": 20 });
        let parsed = parse(&index(vec![p, plugin_json("b", ".plugins/B.lua", 3)])).unwrap();
        assert_eq!(parsed.tag, "v1");
        assert_eq!(parsed.plugins[0].script(), Some("A.lua"));
        assert!(!parsed.plugins[0].files[0].keep);
    }

    #[test]
    fn rejects_each_rule_violation() {
        let good = || plugin_json("a", ".plugins/A.lua", 10);
        let with = |f: &dyn Fn(&mut Value)| {
            let mut p = good();
            f(&mut p);
            index(vec![p])
        };
        let cases: Vec<(&str, Vec<u8>)> = vec![
            ("bad id", with(&|p| p["id"] = json!("Bad Id"))),
            ("empty name", with(&|p| p["name"] = json!(""))),
            ("bad version", with(&|p| p["version"] = json!("1.x"))),
            ("api_min zero", with(&|p| p["api_min"] = json!(0))),
            ("control char", with(&|p| p["description"] = json!("a\nb"))),
            ("size mismatch", with(&|p| p["size"] = json!(11))),
            ("no files", with(&|p| p["files"] = json!([]))),
            (
                "bad dest",
                with(&|p| p["files"][0]["dest"] = json!(".plugins/.disabled")),
            ),
            (
                "no script",
                with(&|p| p["files"][0]["dest"] = json!("Data/x.bin")),
            ),
            ("bad sha", with(&|p| p["files"][0]["sha256"] = json!("xyz"))),
            (
                "zero size file",
                with(&|p| {
                    p["files"][0]["size"] = json!(0);
                    p["size"] = json!(0)
                }),
            ),
            (
                "keep not bool",
                with(&|p| p["files"][0]["keep"] = json!("yes")),
            ),
            (
                "bad preview",
                with(
                    &|p| p["preview"] = json!({ "asset": "p", "sha256": sha('b'), "size": 5, "width": 241, "height": 1 }),
                ),
            ),
            ("duplicate id", index(vec![good(), good()])),
            (
                "overlap",
                index(vec![good(), plugin_json("b", ".plugins/A.lua", 3)]),
            ),
            (
                "schema",
                br#"{"schema":2,"tag":"v1","plugins":[]}"#.to_vec(),
            ),
            ("tag", br#"{"schema":1,"tag":"v 1","plugins":[]}"#.to_vec()),
            ("not json", b"<html>".to_vec()),
        ];
        for (name, bytes) in cases {
            assert!(parse(&bytes).is_err(), "{name} should be rejected");
        }
    }

    #[test]
    fn accepts_integral_floats_and_repeated_keys() {
        let mut p = plugin_json("a", ".plugins/A.lua", 10);
        p["api_min"] = json!(15.0);
        let text = index(vec![p]);
        let text = String::from_utf8(text)
            .unwrap()
            .replace("\"name\":", "\"name\":\"first\",\"name\":");
        let parsed = parse(text.as_bytes()).unwrap();
        assert_eq!(parsed.plugins[0].api_min, 15);
        assert_eq!(parsed.plugins[0].name, "first");

        let mut bad = plugin_json("a", ".plugins/A.lua", 10);
        bad["api_min"] = json!(15.5);
        assert!(parse(&index(vec![bad])).is_err());
        let mut bad = plugin_json("a", ".plugins/A.lua", 10);
        bad["api_min"] = json!(-1);
        assert!(parse(&index(vec![bad])).is_err());
    }

    #[test]
    fn rejects_two_scripts_and_too_many_plugins() {
        let mut p = plugin_json("a", ".plugins/A.lua", 10);
        p["files"].as_array_mut().unwrap().push(json!(
            { "asset": "b", "dest": ".plugins/B.lua", "sha256": sha('a'), "size": 1 }));
        p["size"] = json!(11);
        assert!(parse(&index(vec![p])).is_err());

        let many = (0..=MAX_PLUGINS)
            .map(|i| plugin_json(&format!("p{i}"), &format!(".plugins/P{i}.lua"), 1))
            .collect();
        assert!(parse(&index(many)).is_err());
    }
}
