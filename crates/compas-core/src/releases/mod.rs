//! Player releases: what the latest one is and which files it ships.

mod github;

use std::collections::HashMap;

use serde::Serialize;

use crate::card::Model;
use crate::error::{Error, Result};
use crate::version;

pub use github::{fetch_latest, fetch_latest_from, API_BASE};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    /// Just the player binary, placed at `.compas/compas_player`.
    PlayerOnly,
    /// The whole firmware image, placed at `<stem>.upt` in the card root (`v1.upt` for the V1).
    FullImage,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Freshness {
    /// The player's version is not known.
    Unknown,
    UpToDate,
    UpdateAvailable,
    /// The card runs something newer than the latest release.
    Newer,
}

#[derive(Debug, Clone)]
pub(crate) struct Asset {
    pub name: String,
    pub size: u64,
    pub url: String,
}

/// A downloadable file of a release, with the hash from `SHA256SUMS`.
#[derive(Debug, Clone)]
pub struct ReleaseFile {
    pub name: String,
    pub size: u64,
    pub url: String,
    pub sha256: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct Release {
    pub tag: String,
    pub name: String,
    pub notes: String,
    pub published_at: String,
    pub url: String,
    /// Boards this release ships a player-only file for; a firmware-only release has none.
    pub player_only_boards: Vec<String>,
    #[serde(skip)]
    assets: Vec<Asset>,
    #[serde(skip)]
    checksums: HashMap<String, String>,
}

impl Release {
    /// Name of the release asset for `kind` on `model`: the R1 player is
    /// plain `compas_player`, other boards add their name.
    pub fn asset_name(kind: Kind, model: Model) -> String {
        match (kind, model) {
            (Kind::PlayerOnly, Model::R1) => "compas_player".into(),
            (Kind::PlayerOnly, other) => format!("compas_player_{}", other.board()),
            (Kind::FullImage, other) => format!("{}.upt", other.update_stem()),
        }
    }

    /// Whether the release ships a file of this kind for the model.
    pub fn supports(&self, kind: Kind, model: Model) -> bool {
        let name = Self::asset_name(kind, model);
        self.checksums.contains_key(&name) && self.assets.iter().any(|a| a.name == name)
    }

    pub(crate) fn list_player_only_boards(&mut self) {
        self.player_only_boards = Model::ALL
            .into_iter()
            .filter(|m| self.supports(Kind::PlayerOnly, *m))
            .map(|m| m.board().to_string())
            .collect();
    }

    pub fn file(&self, kind: Kind, model: Model) -> Result<ReleaseFile> {
        let name = Self::asset_name(kind, model);
        let asset = self
            .assets
            .iter()
            .find(|a| a.name == name)
            .ok_or_else(|| Error::InvalidRelease(format!("{} has no {name}", self.tag)))?;
        let sha256 = self
            .checksums
            .get(&name)
            .ok_or_else(|| Error::InvalidRelease(format!("SHA256SUMS has no {name}")))?;
        Ok(ReleaseFile {
            name,
            size: asset.size,
            url: asset.url.clone(),
            sha256: sha256.clone(),
        })
    }

    /// Compares the player's version with this release.
    pub fn freshness(&self, installed: Option<&str>) -> Freshness {
        let latest = version::strip_v(&self.tag);
        match installed {
            Some(installed) => {
                let installed = version::strip_v(installed);
                if !version::is_valid(installed) || !version::is_valid(latest) {
                    return Freshness::Unknown;
                }
                match version::compare(installed, latest) {
                    std::cmp::Ordering::Less => Freshness::UpdateAvailable,
                    std::cmp::Ordering::Equal => Freshness::UpToDate,
                    std::cmp::Ordering::Greater => Freshness::Newer,
                }
            }
            None => Freshness::Unknown,
        }
    }
}

#[cfg(test)]
pub(crate) mod test_fixtures;

#[cfg(test)]
mod tests {
    use super::test_fixtures::{release, release_with};
    use super::*;

    #[test]
    fn asset_names_per_board() {
        assert_eq!(
            Release::asset_name(Kind::PlayerOnly, Model::R1),
            "compas_player"
        );
        assert_eq!(
            Release::asset_name(Kind::PlayerOnly, Model::R3ProII),
            "compas_player_r3proii"
        );
        assert_eq!(
            Release::asset_name(Kind::FullImage, Model::R3II2025),
            "r3ii_2025.upt"
        );
        assert_eq!(
            Release::asset_name(Kind::FullImage, Model::TempotecV1),
            "v1.upt"
        );
        assert_eq!(
            Release::asset_name(Kind::PlayerOnly, Model::TempotecV1),
            "compas_player_tempotec_v1"
        );
    }

    #[tokio::test]
    async fn fetches_release_with_checksums_and_picks_files() {
        let (_server, release) = release("v1.2.0").await;
        assert_eq!(release.tag, "v1.2.0");
        assert_eq!(release.notes, "Notes");
        let file = release.file(Kind::FullImage, Model::R1).unwrap();
        assert_eq!(file.name, "r1.upt");
        assert_eq!(file.size, "image r1".len() as u64);
        assert_eq!(file.sha256.len(), 64);
    }

    #[tokio::test]
    async fn missing_asset_or_checksum_is_an_error() {
        let (_server, mut release) = release("v1.2.0").await;
        release.checksums.remove("r1.upt");
        assert!(matches!(
            release.file(Kind::FullImage, Model::R1),
            Err(Error::InvalidRelease(_))
        ));
        release.assets.clear();
        assert!(matches!(
            release.file(Kind::PlayerOnly, Model::R1),
            Err(Error::InvalidRelease(_))
        ));
    }

    #[tokio::test]
    async fn freshness_against_the_card() {
        let (_server, release) = release("v1.2.0").await;
        assert_eq!(release.freshness(None), Freshness::Unknown);
        assert_eq!(release.freshness(Some("dev")), Freshness::Unknown);
        assert_eq!(release.freshness(Some("1.1.9")), Freshness::UpdateAvailable);
        assert_eq!(release.freshness(Some("1.2.0")), Freshness::UpToDate);
        assert_eq!(release.freshness(Some("1.10")), Freshness::Newer);
    }

    #[tokio::test]
    async fn a_leading_v_on_either_side_is_ignored() {
        let (_server, release) = release("v1.3.0").await;
        assert_eq!(release.freshness(Some("v1.3.0")), Freshness::UpToDate);
        assert_eq!(release.freshness(Some("V1.3.0")), Freshness::UpToDate);
        assert_eq!(release.freshness(Some("1.3.0")), Freshness::UpToDate);
        assert_eq!(
            release.freshness(Some("v1.2.0")),
            Freshness::UpdateAvailable
        );
        assert_eq!(release.freshness(Some("v1.3.1")), Freshness::Newer);
        assert_eq!(release.freshness(Some("v")), Freshness::Unknown);
        assert_eq!(release.freshness(Some("vdev")), Freshness::Unknown);
    }

    #[tokio::test]
    async fn a_firmware_only_release_is_valid_without_player_files() {
        let (_server, release) = release_with("v1.3.0", false).await;
        assert!(release.player_only_boards.is_empty());
        assert!(!release.supports(Kind::PlayerOnly, Model::R1));
        assert!(release.supports(Kind::FullImage, Model::R1));
        assert!(release.file(Kind::FullImage, Model::R3ProII).is_ok());
        // The V1 is offered its own image even though no player-only file exists.
        assert_eq!(
            release
                .file(Kind::FullImage, Model::TempotecV1)
                .unwrap()
                .name,
            "v1.upt"
        );
        assert!(!release.supports(Kind::PlayerOnly, Model::TempotecV1));
    }

    #[tokio::test]
    async fn an_asset_without_a_checksum_is_not_offered() {
        let (_server, mut release) = release("v1.2.0").await;
        release.checksums.remove("compas_player");
        release.list_player_only_boards();
        assert_eq!(
            release.player_only_boards,
            ["r3proii", "r3ii_2025", "tempotec_v1"]
        );
        assert!(!release.supports(Kind::PlayerOnly, Model::R1));
    }

    #[tokio::test]
    async fn a_release_with_player_files_lists_its_boards() {
        let (_server, release) = release("v1.2.0").await;
        assert_eq!(
            release.player_only_boards,
            ["r1", "r3proii", "r3ii_2025", "tempotec_v1"]
        );
        assert!(release.supports(Kind::PlayerOnly, Model::R3ProII));
    }
}
