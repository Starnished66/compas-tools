use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use crate::error::{IoContext, Result};

static COUNTER: AtomicU32 = AtomicU32::new(0);

/// A unique hidden temporary name next to `dest`, so the final rename stays on
/// one filesystem and a leftover from another run is never reused.
pub fn temp_sibling(dest: &Path) -> PathBuf {
    let name = dest.file_name().map(|n| n.to_string_lossy()).unwrap_or_default();
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    dest.with_file_name(format!(".{name}.tmp.{}.{n}", std::process::id()))
}

/// Flushes a finished temporary file and renames it over `dest`.
pub fn persist(tmp: &Path, dest: &Path) -> Result<()> {
    let file = OpenOptions::new().write(true).open(tmp).at(tmp)?;
    file.sync_all().at(tmp)?;
    drop(file);
    fs::rename(tmp, dest).at(dest)?;
    sync_parent(dest);
    Ok(())
}

/// Writes `data` to a temporary file, fsyncs it and renames it over `path`.
pub fn write_atomic(path: &Path, data: &[u8]) -> Result<()> {
    let tmp = temp_sibling(path);
    let written = File::create_new(&tmp)
        .and_then(|mut f| f.write_all(data))
        .at(&tmp)
        .and_then(|()| persist(&tmp, path));
    if written.is_err() {
        let _ = fs::remove_file(&tmp);
    }
    written
}

/// Makes the rename itself durable. Best effort: not every platform can
/// open a directory.
fn sync_parent(path: &Path) {
    if let Some(dir) = path.parent().and_then(|p| File::open(p).ok()) {
        let _ = dir.sync_all();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replaces_content_and_leaves_no_temp_files() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a.txt");
        write_atomic(&path, b"one").unwrap();
        write_atomic(&path, b"two").unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"two");
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1);
    }

    #[test]
    fn failure_keeps_the_old_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a.txt");
        write_atomic(&path, b"old").unwrap();
        fs::create_dir(dir.path().join("sub")).unwrap();
        assert!(write_atomic(&dir.path().join("sub"), b"x").is_err());
        assert_eq!(fs::read(&path).unwrap(), b"old");
    }
}
