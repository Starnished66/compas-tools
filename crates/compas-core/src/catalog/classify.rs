//! Whether a plugin ships a player layout or a theme pack, from its
//! destinations (`plugin_contains_theme_pack`, `plugin_contains_player_layout`).

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Classification {
    pub player_layout: bool,
    pub theme_pack: bool,
}

/// A theme pack wins over a layout when a plugin has both.
pub fn classify<'a>(dests: impl IntoIterator<Item = &'a str> + Clone) -> Classification {
    let theme_pack = dests.clone().into_iter().any(is_theme_file);
    Classification {
        theme_pack,
        player_layout: !theme_pack && dests.into_iter().any(is_layout_file),
    }
}

/// `Themes/<up to two folders>/<name>.theme`, matching the player's theme scan.
fn is_theme_file(dest: &str) -> bool {
    let Some(rel) = dest.strip_prefix("Themes/") else {
        return false;
    };
    if dest.len() <= 13 || !dest.ends_with(".theme") || rel.len() >= 64 {
        return false;
    }
    if rel.contains("..") || rel.contains('\\') || rel.bytes().any(|b| b < 32 || b == 127) {
        return false;
    }
    let segments_ok = rel.split('/').all(|s| !s.is_empty() && !s.starts_with('.'));
    segments_ok && rel.matches('/').count() <= 2
}

/// An `.xml` file in `.plugins/`, `.plugins/player_layouts/` or
/// `.plugins/<bundle>/player_layouts/`.
fn is_layout_file(dest: &str) -> bool {
    let Some(rest) = dest.strip_prefix(".plugins/") else {
        return false;
    };
    let file = if !rest.contains('/') {
        rest
    } else if let Some(file) = rest.strip_prefix("player_layouts/") {
        file
    } else {
        match rest.split_once('/') {
            Some((_, tail)) => tail.strip_prefix("player_layouts/").unwrap_or(""),
            None => "",
        }
    };
    !file.contains('/') && file.len() > 4 && file.to_ascii_lowercase().ends_with(".xml")
}

#[cfg(test)]
mod tests;
