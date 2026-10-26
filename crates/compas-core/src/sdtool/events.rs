//! The JSON lines `compas-sdtool` prints (`docs/SDTOOL.md` in compas-page).

use serde::{Deserialize, Serialize};

use crate::progress::Phase;

/// Result of `info`; also tells which database layout this build writes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LibraryInfo {
    pub sdtool_version: String,
    pub database_format: u32,
    /// 0 when the card has no database yet.
    pub generation: u64,
    pub songs: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScanDone {
    pub files: u64,
    pub parsed: u64,
    pub unchanged: u64,
    /// Files whose tags could not be read; they get no row.
    pub failed: u64,
    pub skipped: u64,
    pub songs: u64,
    pub generation: u64,
    /// False when the library was already up to date and the card was not touched.
    pub written: bool,
    /// Seconds added to the card's file times to match the player's clock.
    pub mtime_offset: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtDone {
    pub board: String,
    pub albums: u64,
    pub written: u64,
    pub unchanged: u64,
    pub no_art: u64,
    /// Albums whose cover could not be written.
    #[serde(default)]
    pub failed: u64,
    pub mtime_offset: i64,
    /// Set when the tool finished but some covers could not be written (`art_failed`).
    #[serde(default)]
    pub warning: Option<String>,
}

/// Both steps of a library update. Only a failed scan is a failed update.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LibraryUpdate {
    pub scan: ScanDone,
    /// `None` when the covers failed completely; the library itself is fine.
    pub art: Option<ArtDone>,
    pub art_error: Option<String>,
}

/// The tool's `progress` phase name, as one of our phases.
pub(super) fn phase(name: &str) -> Option<Phase> {
    match name {
        "walk" => Some(Phase::Scanning),
        "tags" => Some(Phase::ReadingTags),
        "save" => Some(Phase::Saving),
        "art" => Some(Phase::BuildingArt),
        _ => None,
    }
}
