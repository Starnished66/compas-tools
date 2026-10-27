//! `.plugins/.disabled`: script file names the player must not load.

use std::fs;
use std::io::ErrorKind;

use crate::card::Card;
use crate::error::{Error, IoContext, Result};
use crate::fs::write_atomic;

/// The player's cap on the list (`PLUGIN_DISABLED_LIST_MAX`).
const MAX_NAMES: usize = 64;
const FILE_NAME: &str = ".disabled";

/// Lines of the file as raw bytes, as the player compares them. Anything we
/// do not understand is kept so a rewrite never drops it.
pub(super) fn read(card: &Card) -> Result<Vec<Vec<u8>>> {
    let path = card.plugins_dir().join(FILE_NAME);
    match fs::read(&path) {
        Ok(bytes) => Ok(bytes
            .split(|&b| b == b'\n')
            .map(|line| line.strip_suffix(b"\r").unwrap_or(line))
            .filter(|line| !line.is_empty())
            .map(<[u8]>::to_vec)
            .collect()),
        Err(e) if e.kind() == ErrorKind::NotFound => Ok(Vec::new()),
        Err(e) => Err(e).at(&path),
    }
}

pub(super) fn contains(names: &[Vec<u8>], script: &str) -> bool {
    // The player loads only the first lines; later ones have no effect.
    names.iter().take(MAX_NAMES).any(|n| n == script.as_bytes())
}

/// Enables or disables the plugin whose script is `script` (a file name).
pub fn set_enabled(card: &Card, script: &str, enabled: bool) -> Result<()> {
    if script.is_empty() || script.contains(['/', '\\', '\n', '\r']) {
        return Err(Error::Rejected(format!(
            "invalid plugin file name {script}"
        )));
    }
    let mut names = read(card)?;
    if enabled == !contains(&names, script) {
        return Ok(());
    }
    if enabled {
        names.retain(|n| n != script.as_bytes());
    } else if names.len() >= MAX_NAMES {
        return Err(Error::Rejected("too many disabled plugins".into()));
    } else {
        names.push(script.as_bytes().to_vec());
    }
    let body: Vec<u8> = names
        .iter()
        .flat_map(|n| [n.as_slice(), b"\n"].concat())
        .collect();
    write_atomic(&card.ensure_plugins_dir()?.join(FILE_NAME), &body)
}

#[cfg(test)]
mod tests;
