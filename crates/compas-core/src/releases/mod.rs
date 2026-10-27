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
    /// Boards this release ships a full firmware image for.
    pub full_image_boards: Vec<String>,
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

    pub(crate) fn list_full_image_boards(&mut self) {
        self.full_image_boards = Model::ALL
            .into_iter()
            .filter(|m| self.supports(Kind::FullImage, *m))
            .map(|m| m.board().to_string())
            .collect();
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
mod tests;
