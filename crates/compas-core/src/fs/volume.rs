use std::path::{Path, PathBuf};

use sysinfo::Disks;

use super::label::volume_label;
use crate::error::{Error, Result};

/// Slack kept free on top of what an operation writes, as the player does.
const MARGIN: u64 = 2 << 20;

#[derive(Debug, Clone)]
pub struct Volume {
    pub name: String,
    pub mount_point: PathBuf,
    pub file_system: String,
    pub total_bytes: u64,
    pub available_bytes: u64,
}

pub fn volumes() -> Vec<Volume> {
    Disks::new_with_refreshed_list()
        .iter()
        .map(|d| {
            let device = d.name().to_string_lossy().into_owned();
            Volume {
                // On Linux sysinfo names the device node; show the label instead.
                name: if device.starts_with("/dev/") {
                    volume_label(&device).unwrap_or_default()
                } else {
                    device
                },
                mount_point: d.mount_point().to_path_buf(),
                file_system: d.file_system().to_string_lossy().into_owned(),
                total_bytes: d.total_space(),
                available_bytes: d.available_space(),
            }
        })
        .collect()
}

/// The volume holding `path`: the one with the longest matching mount point.
pub fn volume_for(path: &Path) -> Option<Volume> {
    let path = dunce::canonicalize(path).ok()?;
    best_match(&path, volumes())
}

/// `canonicalize` yields `\\?\E:\` on Windows while mount points are `E:\`,
/// so both sides go through `dunce::simplified` before comparing.
fn best_match(path: &Path, volumes: Vec<Volume>) -> Option<Volume> {
    let path = dunce::simplified(path);
    volumes
        .into_iter()
        .filter(|v| path.starts_with(dunce::simplified(&v.mount_point)))
        .max_by_key(|v| v.mount_point.as_os_str().len())
}

/// Fails when `needed` bytes (plus a margin) do not fit. An unknown volume
/// passes: the write itself will report a full disk.
pub fn ensure_free(path: &Path, needed: u64) -> Result<()> {
    let Some(volume) = volume_for(path) else {
        return Ok(());
    };
    let needed = needed.saturating_add(MARGIN);
    if volume.available_bytes < needed {
        return Err(Error::NoSpace {
            needed,
            available: volume.available_bytes,
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_a_volume_and_rejects_absurd_sizes() {
        let dir = tempfile::tempdir().unwrap();
        assert!(volume_for(dir.path()).is_some());
        assert!(ensure_free(dir.path(), 0).is_ok());
        assert!(matches!(
            ensure_free(dir.path(), u64::MAX / 2),
            Err(Error::NoSpace { .. })
        ));
    }

    fn volume(mount: &str) -> Volume {
        Volume {
            name: String::new(),
            mount_point: mount.into(),
            file_system: String::new(),
            total_bytes: 0,
            available_bytes: 0,
        }
    }

    #[test]
    fn picks_the_longest_mount_point() {
        let all = vec![volume("/"), volume("/media/card"), volume("/media")];
        let found = best_match(Path::new("/media/card/.plugins"), all).unwrap();
        assert_eq!(found.mount_point, Path::new("/media/card"));
    }

    #[cfg(windows)]
    #[test]
    fn matches_verbatim_windows_paths() {
        let all = vec![volume(r"C:\"), volume(r"E:\")];
        let found = best_match(Path::new(r"\\?\E:\.plugins"), all).unwrap();
        assert_eq!(found.mount_point, Path::new(r"E:\"));
    }
}
