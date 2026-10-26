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
    /// TempoTec Variations V1 (240x320).
    #[serde(rename = "tempotec_v1")]
    TempotecV1,
}

impl Model {
    pub const ALL: [Model; 4] = [
        Model::R1,
        Model::R3ProII,
        Model::R3II2025,
        Model::TempotecV1,
    ];

    /// Name used for update files on the card and in the player's releases.
    pub fn board(self) -> &'static str {
        match self {
            Model::R1 => "r1",
            Model::R3ProII => "r3proii",
            Model::R3II2025 => "r3ii_2025",
            Model::TempotecV1 => "tempotec_v1",
        }
    }

    /// The model for a board id, as in `COMPAS_PLAYER_BOARD:<board>` or `device.json`.
    pub fn from_board(board: &str) -> Option<Model> {
        Model::ALL.into_iter().find(|m| m.board() == board)
    }

    /// Name of the firmware image: `<stem>.upt`. The V1's is `v1.upt`, not its board id.
    pub fn update_stem(self) -> &'static str {
        match self {
            Model::TempotecV1 => "v1",
            other => other.board(),
        }
    }

    /// The board name the library tool accepts for album art. Its caches (a 72 px thumbnail
    /// and a 480 px cover) are the same on every board, and the player's V1 build uses the
    /// same sizes, so the tool's closest known board stands in for the V1.
    pub fn art_board(self) -> &'static str {
        match self {
            Model::TempotecV1 => "r1",
            other => other.board(),
        }
    }
}

/// `.compas/device.json`, written by the player.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Device {
    pub schema: u32,
    /// `None` for a board this app does not know; the rest of the file stays usable.
    #[serde(deserialize_with = "lenient_model")]
    pub model: Option<Model>,
    pub player_version: String,
    pub plugin_api: u32,
    pub database_format: Option<u32>,
    pub art_cache: Option<String>,
}

fn lenient_model<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> std::result::Result<Option<Model>, D::Error> {
    let value = serde_json::Value::deserialize(deserializer)?;
    Ok(value.as_str().and_then(Model::from_board))
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
    fn boards_names_and_update_files() {
        assert_eq!(Model::from_board("tempotec_v1"), Some(Model::TempotecV1));
        assert_eq!(Model::from_board("r3proii"), Some(Model::R3ProII));
        assert_eq!(Model::from_board("v1"), None);
        assert_eq!(Model::TempotecV1.update_stem(), "v1");
        assert_eq!(Model::R3II2025.update_stem(), "r3ii_2025");
        // The library tool does not know the V1, but its art does not depend on the board.
        assert_eq!(Model::TempotecV1.art_board(), "r1");
        assert_eq!(Model::R3ProII.art_board(), "r3proii");
    }

    #[test]
    fn reads_player_written_file() {
        let (dir, card) = fake_card();
        write_file(
            dir.path(),
            ".compas/device.json",
            br#"{"schema":1,"model":"r3ii_2025","player_version":"1.2.0","plugin_api":17,"database_format":3,"art_cache":"v4"}"#,
        );
        let device = card.device().unwrap().unwrap();
        assert_eq!(device.model, Some(Model::R3II2025));
        assert_eq!(device.plugin_api, 17);
        assert_eq!(device.art_cache.as_deref(), Some("v4"));
    }

    #[test]
    fn an_unknown_board_keeps_the_rest_of_the_file() {
        let (dir, card) = fake_card();
        write_file(
            dir.path(),
            ".compas/device.json",
            br#"{"schema":1,"model":"future_board","player_version":"v2.0.0","plugin_api":21,"database_format":4}"#,
        );
        let device = card.device().unwrap().unwrap();
        assert_eq!(device.model, None);
        assert_eq!((device.plugin_api, device.database_format), (21, Some(4)));
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
