//! `.plugins/.disabled`: script file names the player must not load.

use std::fs;
use std::io::ErrorKind;

use crate::card::Card;
use crate::error::{Error, IoContext, Result};
use crate::fs::write_atomic;

/// The player's cap on the list (`PLUGIN_DISABLED_LIST_MAX`).
const MAX_NAMES: usize = 64;
const FILE_NAME: &str = ".disabled";

pub(super) fn read(card: &Card) -> Result<Vec<String>> {
    let path = card.plugins_dir().join(FILE_NAME);
    match fs::read_to_string(&path) {
        Ok(text) => Ok(text
            .lines()
            .filter(|l| !l.is_empty())
            .take(MAX_NAMES)
            .map(str::to_string)
            .collect()),
        Err(e) if e.kind() == ErrorKind::NotFound => Ok(Vec::new()),
        Err(e) => Err(e).at(&path),
    }
}

/// Enables or disables the plugin whose script is `script` (a file name).
pub fn set_enabled(card: &Card, script: &str, enabled: bool) -> Result<()> {
    if script.is_empty() || script.contains(['/', '\\', '\n', '\r']) {
        return Err(Error::Rejected(format!("invalid plugin file name {script}")));
    }
    let mut names = read(card)?;
    let listed = names.iter().any(|n| n == script);
    if enabled == !listed {
        return Ok(());
    }
    if enabled {
        names.retain(|n| n != script);
    } else if names.len() >= MAX_NAMES {
        return Err(Error::Rejected("too many disabled plugins".into()));
    } else {
        names.push(script.to_string());
    }
    let body: String = names.iter().map(|n| format!("{n}\n")).collect();
    write_atomic(&card.ensure_plugins_dir()?.join(FILE_NAME), body.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{fake_card, write_file};

    #[test]
    fn disables_and_enables_idempotently() {
        let (dir, card) = fake_card();
        set_enabled(&card, "A.lua", true).unwrap();
        assert!(!dir.path().join(".plugins/.disabled").exists(), "no-op writes nothing");

        set_enabled(&card, "A.lua", false).unwrap();
        set_enabled(&card, "B.lua", false).unwrap();
        set_enabled(&card, "A.lua", false).unwrap();
        let text = fs::read_to_string(dir.path().join(".plugins/.disabled")).unwrap();
        assert_eq!(text, "A.lua\nB.lua\n");

        set_enabled(&card, "A.lua", true).unwrap();
        assert_eq!(read(&card).unwrap(), ["B.lua"]);
    }

    #[test]
    fn reads_crlf_and_blank_lines_and_rejects_paths() {
        let (dir, card) = fake_card();
        write_file(dir.path(), ".plugins/.disabled", b"A.lua\r\n\r\nB.lua\n");
        assert_eq!(read(&card).unwrap(), ["A.lua", "B.lua"]);
        assert!(set_enabled(&card, ".plugins/A.lua", false).is_err());
    }

    #[test]
    fn caps_the_list() {
        let (_dir, card) = fake_card();
        for i in 0..MAX_NAMES {
            set_enabled(&card, &format!("p{i}.lua"), false).unwrap();
        }
        assert!(set_enabled(&card, "extra.lua", false).is_err());
    }
}
