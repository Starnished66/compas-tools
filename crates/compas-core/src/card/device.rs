use std::fs;
use std::io::ErrorKind;

use serde::{Deserialize, Serialize};

use super::Card;
use crate::error::{IoContext, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Model {
    #[serde(rename = "r1")]
    R1,
    #[serde(rename = "r3proii")]
    R3ProII,
    #[serde(rename = "r3ii_2025")]
    R3II2025,
}

impl Model {
    pub const ALL: [Model; 3] = [Model::R1, Model::R3ProII, Model::R3II2025];

    /// Name used for update files on the card and in the player's releases.
    pub fn board(self) -> &'static str {
        match self {
            Model::R1 => "r1",
            Model::R3ProII => "r3proii",
            Model::R3II2025 => "r3ii_2025",
        }
    }
}

/// `.compas/device.json`, written by the player.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Device {
    pub schema: u32,
    pub model: Model,
    pub player_version: String,
    pub plugin_api: u32,
    pub database_format: Option<u32>,
    pub art_cache: Option<String>,
}

/// A missing or unreadable-as-schema-1 file means the device is unknown.
pub(super) fn read(card: &Card) -> Result<Option<Device>> {
    let path = card.compas_dir().join("device.json");
    let bytes = match fs::read(&path) {
        Ok(bytes) => bytes,
        Err(e) if e.kind() == ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e).at(&path),
    };
    Ok(serde_json::from_slice::<Device>(&bytes)
        .ok()
        .filter(|d| d.schema == 1))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{fake_card, write_file};

    #[test]
    fn reads_player_written_file() {
        let (dir, card) = fake_card();
        write_file(
            dir.path(),
            ".compas/device.json",
            br#"{"schema":1,"model":"r3ii_2025","player_version":"1.2.0","plugin_api":17,"database_format":3,"art_cache":"v4"}"#,
        );
        let device = card.device().unwrap().unwrap();
        assert_eq!(device.model, Model::R3II2025);
        assert_eq!(device.plugin_api, 17);
        assert_eq!(device.art_cache.as_deref(), Some("v4"));
    }

    #[test]
    fn unknown_when_missing_malformed_or_other_schema() {
        let (dir, card) = fake_card();
        assert!(card.device().unwrap().is_none());
        for body in [
            "not json",
            r#"{"schema":2,"model":"r1","player_version":"1","plugin_api":1}"#,
        ] {
            write_file(dir.path(), ".compas/device.json", body.as_bytes());
            assert!(card.device().unwrap().is_none());
        }
    }
}
