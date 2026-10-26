//! Player releases and placing update files on the card.

mod github;
mod place;

use std::collections::HashMap;

use serde::Serialize;

use crate::card::Model;
use crate::error::{Error, Result};
use crate::version;

pub use github::{fetch_latest, fetch_latest_from, API_BASE};
pub use place::{other_upts, place};

/// The first player release that writes `.compas/device.json`. A player that has run
/// without writing one is older than this (v1.2.0 and earlier).
pub const DEVICE_INFO_SINCE: &str = "v1.3.0";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    /// Just the player binary, placed at `.compas/compas_player`.
    PlayerOnly,
    /// The whole firmware image, placed at `<board>.upt` in the card root.
    FullImage,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Freshness {
    /// The card has no `device.json`, so its version is not known.
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
            (Kind::FullImage, other) => format!("{}.upt", other.board()),
        }
    }

    /// Whether the release ships a file of this kind for the model.
    pub fn supports(&self, kind: Kind, model: Model) -> bool {
        let name = Self::asset_name(kind, model);
        self.assets.iter().any(|a| a.name == name)
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

    /// For a player that has no `device.json` although it runs: it predates the file, so it is
    /// older than any release from [`DEVICE_INFO_SINCE`] on. Older releases say nothing.
    pub fn freshness_without_device_info(&self) -> Freshness {
        let since = version::strip_v(DEVICE_INFO_SINCE);
        let latest = version::strip_v(&self.tag);
        if version::is_valid(latest) && version::compare(latest, since).is_ge() {
            Freshness::UpdateAvailable
        } else {
            Freshness::Unknown
        }
    }

    /// Compares the player version on the card with this release.
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
    async fn a_player_without_device_json_is_older_than_the_file_itself() {
        let (_server, firmware_only) = release_with("v1.3.0", false).await;
        assert_eq!(
            firmware_only.freshness_without_device_info(),
            Freshness::UpdateAvailable
        );
        let (_server, older) = release("v1.2.0").await;
        assert_eq!(older.freshness_without_device_info(), Freshness::Unknown);
    }

    #[tokio::test]
    async fn a_firmware_only_release_is_valid_without_player_files() {
        let (_server, release) = release_with("v1.3.0", false).await;
        assert!(release.player_only_boards.is_empty());
        assert!(!release.supports(Kind::PlayerOnly, Model::R1));
        assert!(release.supports(Kind::FullImage, Model::R1));
        assert!(release.file(Kind::FullImage, Model::R3ProII).is_ok());
    }

    #[tokio::test]
    async fn a_release_with_player_files_lists_its_boards() {
        let (_server, release) = release("v1.2.0").await;
        assert_eq!(release.player_only_boards, ["r1", "r3proii", "r3ii_2025"]);
        assert!(release.supports(Kind::PlayerOnly, Model::R3ProII));
    }
}
