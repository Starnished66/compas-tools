//! Field validators from the player's `store_common.c` and `plugin_store.c`.

/// `regex_id`: `[a-z0-9][a-z0-9_.-]{0,62}`.
pub fn is_id(s: &str) -> bool {
    let mut bytes = s.bytes();
    s.len() <= 63
        && bytes
            .next()
            .is_some_and(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
        && bytes.all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b"_.-".contains(&b))
}

/// `store_token_valid` with dots allowed: `[A-Za-z0-9_.-]`, 1 to `max` bytes.
pub fn is_token(s: &str, max: usize) -> bool {
    !s.is_empty()
        && s.len() <= max
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"_.-".contains(&b))
}

pub use crate::fs::is_sha256;

/// `store_safe_text`: no control characters, at most `max` bytes.
pub fn is_safe_text(s: &str, max: usize, allow_empty: bool) -> bool {
    (allow_empty || !s.is_empty()) && s.len() <= max && s.bytes().all(|b| b >= 32 && b != 127)
}

#[cfg(test)]
mod tests;
