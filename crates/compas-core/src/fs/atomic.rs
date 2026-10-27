use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use crate::error::{IoContext, Result};

static COUNTER: AtomicU32 = AtomicU32::new(0);

/// A unique hidden temporary name next to `dest`, so the final rename stays on
/// one filesystem and a leftover from another run is never reused.
pub fn temp_sibling(dest: &Path) -> PathBuf {
    let name = dest
        .file_name()
        .map(|n| n.to_string_lossy())
        .unwrap_or_default();
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
mod tests;
