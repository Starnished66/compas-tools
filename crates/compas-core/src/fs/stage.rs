use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::error::{IoContext, Result};
use crate::fs::ensure_real_dir;

static COUNTER: AtomicU32 = AtomicU32::new(0);

/// A staging folder with a name of its own, deleted on drop. Operations
/// download and back up into it so a failure never leaves half-written files
/// at their destination.
pub struct StageDir {
    path: PathBuf,
    keep: bool,
}

impl StageDir {
    pub fn create(parent: &Path, label: &str) -> Result<Self> {
        let secs = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_secs());
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = parent.join(format!("{label}.{secs}.{n}"));
        for dir in [parent, path.as_path()] {
            ensure_real_dir(dir).at(dir)?;
        }
        Ok(Self { path, keep: false })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Leaves the folder in place; it holds the only copy of a user's files.
    pub fn keep(mut self) {
        self.keep = true;
    }
}

impl Drop for StageDir {
    fn drop(&mut self) {
        if !self.keep {
            let _ = fs::remove_dir_all(&self.path);
            // The parent is shared; this only succeeds once it is empty.
            if let Some(parent) = self.path.parent() {
                let _ = fs::remove_dir(parent);
            }
        }
    }
}

#[cfg(test)]
mod tests;
