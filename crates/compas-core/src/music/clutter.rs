//! What macOS leaves on a FAT card, and the marker that stops Spotlight.

use std::fs;
use std::path::Path;

use serde::Serialize;

use crate::card::Card;
use crate::error::Result;
use crate::fs::write_atomic;

const NEVER_INDEX: &str = ".metadata_never_index";
/// Folders macOS creates at the root of a volume.
const ROOT_FOLDERS: [&str; 4] = [
    ".Spotlight-V100",
    ".fseventsd",
    ".Trashes",
    ".TemporaryItems",
];

/// Names never worth copying to the card.
pub fn is_clutter_name(name: &str) -> bool {
    name.starts_with("._")
        || name == ".DS_Store"
        || name == ".apdisk"
        || ROOT_FOLDERS.contains(&name)
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct TidyReport {
    pub removed: u64,
    /// Whether `.metadata_never_index` had to be created.
    pub marker_written: bool,
}

/// Removes macOS clutter (`._*`, `.DS_Store`, and the root folders above),
/// never inside `.compas` or `.plugins`,
/// and makes sure `.metadata_never_index` exists. Links are never followed.
pub fn tidy(card: &Card) -> Result<TidyReport> {
    let mut report = TidyReport::default();
    sweep(card.root(), true, &mut report);

    let marker = card.root().join(NEVER_INDEX);
    if fs::symlink_metadata(&marker).is_err() {
        write_atomic(&marker, b"")?;
        report.marker_written = true;
    }
    Ok(report)
}

fn sweep(dir: &Path, is_root: bool, report: &mut TidyReport) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        let Ok(kind) = entry.file_type() else {
            continue;
        };
        let path = entry.path();
        if kind.is_dir() {
            if is_root && (name == ".compas" || name == ".plugins") {
                continue;
            }
            if is_root && ROOT_FOLDERS.contains(&name.as_str()) {
                report.removed += u64::from(fs::remove_dir_all(&path).is_ok());
            } else {
                sweep(&path, false, report);
            }
        } else if kind.is_file() && (name.starts_with("._") || name == ".DS_Store") {
            report.removed += u64::from(fs::remove_file(&path).is_ok());
        }
    }
}

#[cfg(test)]
mod tests;
