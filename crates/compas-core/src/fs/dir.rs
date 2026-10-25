use std::fs;
use std::io::{self, ErrorKind};
use std::path::Path;

/// Creates `dir` (one level) if missing and fails if it is anything but a
/// real directory; a symlink could lead off the card.
pub fn ensure_real_dir(dir: &Path) -> io::Result<()> {
    match fs::symlink_metadata(dir) {
        Ok(meta) if meta.is_dir() => Ok(()),
        Ok(_) => Err(io::Error::new(
            ErrorKind::AlreadyExists,
            "not a real folder",
        )),
        Err(e) if e.kind() == ErrorKind::NotFound => fs::create_dir(dir),
        Err(e) => Err(e),
    }
}
