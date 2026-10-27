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
        && dest.split('/').all(is_plain_segment)
        && lower != ".compas"
        && !lower.starts_with(".compas/")
        && !lower.starts_with(".plugins/.")
        && !lower.ends_with(".upt")
}

/// Not empty, `.` or `..`, and not ending in a dot or space: vfat and Windows
/// drop those, so `.compas./x` would reach the player's `.compas/x`. This is
/// deliberately stricter than the player.
fn is_plain_segment(segment: &str) -> bool {
    !matches!(segment, "" | "." | "..") && !segment.ends_with(['.', ' '])
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
    let (short, long) = if a.len() <= b.len() {
        (&a, &b)
    } else {
        (&b, &a)
    };
    short == long || (long.starts_with(short.as_str()) && long[short.len()..].starts_with('/'))
}

#[cfg(test)]
mod tests;
