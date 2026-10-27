//! File name rules for a FAT card read by the player.

/// `TAGCACHE_PATH_MAX`: the player's path buffer, terminator included.
const PLAYER_PATH_MAX: usize = 600;
/// `/data/mnt/sd_0/`, which the player prepends to a card-relative path.
const PLAYER_ROOT_LEN: usize = "/data/mnt/sd_0/".len();
/// FAT long file names hold 255 UTF-16 code units.
const NAME_MAX_UNITS: usize = 255;
const RESERVED: [&str; 22] = [
    "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
    "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
];

/// A name FAT (and Windows) accepts: forbidden characters become `_`,
/// trailing dots and spaces go, and reserved device names get a `_` prefix.
pub fn fat_safe_name(name: &str) -> String {
    let replaced: String = name
        .chars()
        .map(|c| {
            if c < ' ' || "\\/:*?\"<>|".contains(c) {
                '_'
            } else {
                c
            }
        })
        .collect();
    let trimmed = replaced.trim_end_matches(['.', ' ']);
    if trimmed.is_empty() {
        return "_".into();
    }
    let stem = trimmed
        .split('.')
        .next()
        .unwrap_or(trimmed)
        .to_ascii_uppercase();
    if RESERVED.contains(&stem.as_str()) {
        format!("_{trimmed}")
    } else {
        trimmed.to_string()
    }
}

pub fn name_too_long(name: &str) -> bool {
    name.encode_utf16().count() > NAME_MAX_UNITS
}

/// Whether the player can hold the card-relative path (`/`-separated, UTF-8).
pub fn player_path_fits(relative: &str) -> bool {
    PLAYER_ROOT_LEN + relative.len() < PLAYER_PATH_MAX
}

#[cfg(test)]
mod tests;
