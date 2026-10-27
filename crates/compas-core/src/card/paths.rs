use std::fs;
use std::io::ErrorKind;
use std::path::{Component, Path, PathBuf};

use crate::error::{Error, IoContext, Result};

/// Joins a card-relative `dest` onto `root`, checking that every existing
/// parent folder is a real directory (a symlink could lead off the card) and
/// creating the missing ones when asked. Callers validate `dest` itself.
pub(super) fn resolve(root: &Path, dest: &str, create_parents: bool) -> Result<PathBuf> {
    let relative = Path::new(dest);
    if relative
        .components()
        .any(|c| !matches!(c, Component::Normal(_)))
    {
        return Err(Error::Rejected(format!("unsafe path {dest}")));
    }
    let full = root.join(relative);
    let mut current = root.to_path_buf();
    let parents = relative.parent().into_iter().flat_map(Path::components);
    for component in parents {
        current.push(component);
        match fs::symlink_metadata(&current) {
            Ok(meta) if meta.is_dir() => {}
            Ok(_) => {
                return Err(Error::Rejected(format!(
                    "{} is not a folder",
                    current.display()
                )))
            }
            Err(e) if e.kind() == ErrorKind::NotFound => {
                if !create_parents {
                    return Ok(full); // nothing below a missing folder can exist
                }
                fs::create_dir(&current).at(&current)?;
            }
            Err(e) => return Err(e).at(&current),
        }
    }
    Ok(full)
}

#[cfg(all(test, unix))]
mod tests;
