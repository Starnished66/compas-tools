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
mod tests {
    use super::*;
    use crate::test_support::{fake_card, write_file};

    #[test]
    fn removes_clutter_everywhere_but_keeps_music_and_the_players_folder() {
        let (dir, card) = fake_card();
        let root = dir.path();
        write_file(root, "Artist/Album/01.flac", b"music");
        write_file(root, "Artist/Album/._01.flac", b"apple double");
        write_file(root, "Artist/.DS_Store", b"");
        write_file(root, ".Spotlight-V100/Store-V2/x", b"");
        write_file(root, ".fseventsd/0000", b"");
        write_file(root, ".compas/._keep", b"inside the player's folder");
        write_file(root, ".plugins/._keep", b"inside the plugins folder");
        write_file(root, ".plugins/Data/.DS_Store", b"plugin data");
        write_file(
            root,
            "Artist/Album/.Trashes/not-root",
            b"only the root has these",
        );

        let report = tidy(&card).unwrap();
        assert_eq!(report.removed, 4);
        assert!(report.marker_written);
        assert!(root.join("Artist/Album/01.flac").exists());
        assert!(!root.join("Artist/Album/._01.flac").exists());
        assert!(!root.join(".fseventsd").exists());
        assert!(root.join(".compas/._keep").exists());
        assert!(root.join(".plugins/._keep").exists());
        assert!(root.join(".plugins/Data/.DS_Store").exists());
        assert!(root.join("Artist/Album/.Trashes/not-root").exists());
        assert!(root.join(".metadata_never_index").exists());

        let again = tidy(&card).unwrap();
        assert_eq!(again, TidyReport::default());
    }

    #[cfg(unix)]
    #[test]
    fn never_follows_links() {
        let (dir, card) = fake_card();
        let outside = tempfile::tempdir().unwrap();
        write_file(outside.path(), "._precious", b"not ours");
        write_file(outside.path(), "x", b"x");
        std::os::unix::fs::symlink(outside.path(), dir.path().join(".fseventsd")).unwrap();
        std::os::unix::fs::symlink(outside.path(), dir.path().join("Linked")).unwrap();

        tidy(&card).unwrap();
        assert!(outside.path().join("._precious").exists());
        assert!(outside.path().join("x").exists());
    }

    #[test]
    fn recognises_clutter_names() {
        assert!(is_clutter_name("._song.mp3"));
        assert!(is_clutter_name(".DS_Store"));
        assert!(!is_clutter_name("song._.mp3"));
        assert!(!is_clutter_name(".hidden"));
    }
}
