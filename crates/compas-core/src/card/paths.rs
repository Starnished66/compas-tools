use std::fs;
use std::io::{self, ErrorKind};
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

/// Creates `dir` if missing and fails if it is anything but a real directory.
pub(super) fn ensure_real_dir(dir: &Path) -> io::Result<()> {
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

#[cfg(all(test, unix))]
mod tests {
    use super::*;

    #[test]
    fn creates_parents_and_refuses_symlinks_and_escapes() {
        let dir = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        let root = dir.path();

        let path = resolve(root, "a/b/c.txt", true).unwrap();
        assert!(root.join("a/b").is_dir());
        assert_eq!(path, root.join("a/b/c.txt"));
        assert_eq!(
            resolve(root, "x/y.txt", false).unwrap(),
            root.join("x/y.txt")
        );
        assert!(!root.join("x").exists());

        std::os::unix::fs::symlink(outside.path(), root.join("link")).unwrap();
        assert!(resolve(root, "link/f.txt", true).is_err());
        assert!(resolve(root, "../f.txt", true).is_err());
        assert!(resolve(root, "/etc/passwd", true).is_err());
    }
}
