//! Names a music transfer must never write to or delete from.

/// Top-level names the player (or the plugin store) owns besides hidden ones.
const PLAYER_ENTRIES: [&str; 2] = ["themes", "radio.txt"];

/// `.compas`, `.plugins`, `.DS_Store`, ...: hidden names are never copied or removed.
pub fn is_hidden(name: &str) -> bool {
    name.starts_with('.')
}

/// Files that steer the player's library scan; a mirror never lists or removes them.
pub fn is_scan_control_file(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    lower == "database.ignore" || lower == "database.unignore"
}

/// A `.upt` file is a firmware image the player installs; music transfers
/// neither copy nor delete one.
pub fn is_firmware_image(name: &str) -> bool {
    name.to_ascii_lowercase().ends_with(".upt")
}

/// Whether a card-relative destination (as path segments) is off limits:
/// a hidden segment anywhere, a player-owned top-level entry, or a firmware image.
pub fn reserved_destination(segments: &[String]) -> bool {
    segments.iter().any(|s| is_hidden(s))
        || segments
            .first()
            .is_some_and(|s| PLAYER_ENTRIES.contains(&s.to_ascii_lowercase().as_str()))
        || segments.last().is_some_and(|s| is_firmware_image(s))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reserved(path: &str) -> bool {
        reserved_destination(&path.split('/').map(str::to_string).collect::<Vec<_>>())
    }

    #[test]
    fn scan_control_files_are_recognised_in_any_case() {
        assert!(is_scan_control_file("database.ignore"));
        assert!(is_scan_control_file("Database.UNIGNORE"));
        assert!(!is_scan_control_file("database.ignore.bak"));
    }

    #[test]
    fn player_owned_hidden_and_firmware_paths_are_reserved() {
        for path in [
            ".compas",
            ".compas/device.json",
            ".plugins/x.lua",
            ".PLUGINS",
            "Music/.hidden/a.mp3",
            "Themes",
            "themes/Night.theme",
            "THEMES/a",
            "radio.txt",
            "r1.upt",
            "Music/R1.UPT",
        ] {
            assert!(reserved(path), "{path}");
        }
        for path in [
            "Music/Themes",
            "Artist/Album/01.flac",
            "Radio/radio.txt.mp3",
            "Upt/a.mp3",
        ] {
            assert!(!reserved(path), "{path}");
        }
    }
}
