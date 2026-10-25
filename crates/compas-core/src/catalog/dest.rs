//! Destination paths inside a store plugin (`plugin_store_dest_valid`).

/// A card-relative path the store may write.
pub fn is_valid(dest: &str) -> bool {
    let lower = dest.to_ascii_lowercase();
    dest.len() <= 255
        && !dest.is_empty()
        && !dest.starts_with('/')
        && dest
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b" ._-/".contains(&b))
        && dest.split('/').all(|s| !s.is_empty() && s != "." && s != "..")
        && lower != ".compas"
        && !lower.starts_with(".compas/")
        && !lower.starts_with(".plugins/.")
        && !lower.ends_with(".upt")
}

/// `script_dest`: a `.lua` file directly inside `.plugins/`.
pub fn script_name(dest: &str) -> Option<&str> {
    let (dir, name) = dest.split_at_checked(9)?;
    (dir.eq_ignore_ascii_case(".plugins/")
        && !name.contains('/')
        && name.len() > 4
        && name.to_ascii_lowercase().ends_with(".lua"))
    .then_some(name)
}

/// Two destinations that cannot both exist: equal, or one holds the other.
pub fn collide(a: &str, b: &str) -> bool {
    let (a, b) = (a.to_ascii_lowercase(), b.to_ascii_lowercase());
    let (short, long) = if a.len() <= b.len() { (&a, &b) } else { (&b, &a) };
    short == long || (long.starts_with(short.as_str()) && long[short.len()..].starts_with('/'))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validity() {
        for ok in [".plugins/a.lua", "Themes/x/y.theme", "Data/some file.txt"] {
            assert!(is_valid(ok), "{ok}");
        }
        for bad in [
            "", "/abs", "a//b", "a/./b", "../x", "a/..", ".compas", ".COMPAS/x",
            ".plugins/.store_installed", ".Plugins/.disabled", "r1.UPT", "a\\b", "é.lua",
            &"a".repeat(256),
        ] {
            assert!(!is_valid(bad), "{bad}");
        }
    }

    #[test]
    fn script() {
        assert_eq!(script_name(".plugins/A.lua"), Some("A.lua"));
        assert_eq!(script_name(".PLUGINS/A.LUA"), Some("A.LUA"));
        assert_eq!(script_name(".plugins/.lua"), None);
        assert_eq!(script_name(".plugins/sub/a.lua"), None);
        assert_eq!(script_name(".plugins/a.xml"), None);
        assert_eq!(script_name("é"), None);
    }

    #[test]
    fn collisions() {
        assert!(collide("Data/a", "data/A"));
        assert!(collide("Data/a", "Data/a/b"));
        assert!(!collide("Data/a", "Data/ab"));
    }
}
