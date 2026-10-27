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
pub(super) mod tests;
