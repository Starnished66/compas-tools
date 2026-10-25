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
mod tests {
    use super::*;
    use crate::test_support::{fake_card, write_file};

    #[test]
    fn disables_and_enables_idempotently() {
        let (dir, card) = fake_card();
        set_enabled(&card, "A.lua", true).unwrap();
        assert!(
            !dir.path().join(".plugins/.disabled").exists(),
            "no-op writes nothing"
        );

        set_enabled(&card, "A.lua", false).unwrap();
        set_enabled(&card, "B.lua", false).unwrap();
        set_enabled(&card, "A.lua", false).unwrap();
        let text = fs::read_to_string(dir.path().join(".plugins/.disabled")).unwrap();
        assert_eq!(text, "A.lua\nB.lua\n");

        set_enabled(&card, "A.lua", true).unwrap();
        assert_eq!(read(&card).unwrap(), [b"B.lua".to_vec()]);
    }

    #[test]
    fn reads_crlf_and_blank_lines_and_rejects_paths() {
        let (dir, card) = fake_card();
        write_file(dir.path(), ".plugins/.disabled", b"A.lua\r\n\r\nB.lua\n");
        assert_eq!(read(&card).unwrap(), [b"A.lua".to_vec(), b"B.lua".to_vec()]);
        assert!(set_enabled(&card, ".plugins/A.lua", false).is_err());
    }

    #[test]
    fn only_the_first_names_count_but_all_survive_a_rewrite() {
        let (dir, card) = fake_card();
        let body: String = (0..70).map(|i| format!("p{i}.lua\n")).collect();
        write_file(dir.path(), ".plugins/.disabled", body.as_bytes());
        let names = read(&card).unwrap();
        assert!(contains(&names, "p63.lua"));
        assert!(!contains(&names, "p64.lua"));

        set_enabled(&card, "p0.lua", true).unwrap();
        let text = fs::read_to_string(dir.path().join(".plugins/.disabled")).unwrap();
        assert_eq!(text.lines().count(), 69);
        assert!(text.contains("p69.lua"));
    }

    #[test]
    fn caps_the_list() {
        let (_dir, card) = fake_card();
        for i in 0..MAX_NAMES {
            set_enabled(&card, &format!("p{i}.lua"), false).unwrap();
        }
        assert!(set_enabled(&card, "extra.lua", false).is_err());
    }

    #[test]
    fn keeps_lines_that_are_not_utf8_when_rewriting() {
        let (dir, card) = fake_card();
        write_file(dir.path(), ".plugins/.disabled", b"caf\xe9.lua\nA.lua\n");
        assert!(contains(&read(&card).unwrap(), "A.lua"));
        set_enabled(&card, "B.lua", false).unwrap();
        let bytes = fs::read(dir.path().join(".plugins/.disabled")).unwrap();
        assert_eq!(bytes, b"caf\xe9.lua\nA.lua\nB.lua\n");
    }
}
