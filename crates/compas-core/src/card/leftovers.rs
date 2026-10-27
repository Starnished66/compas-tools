use std::fs;
use std::path::Path;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use super::Card;

/// Younger leftovers may belong to another running instance.
const MIN_AGE: Duration = Duration::from_secs(60 * 60);

impl Card {
    /// Removes what earlier runs that crashed left behind: partial downloads
    /// named like our temporary files, and staging folders that hold no
    /// backups (a folder with backups may hold the only copy of a user's file).
    /// Only exact name patterns older than an hour are touched, and only
    /// inside real directories, so links can never lead it off the card.
    /// Best effort.
    pub fn clean_leftovers(&self) {
        let now = SystemTime::now();
        for dir in [
            self.root().to_path_buf(),
            self.compas_dir(),
            self.plugins_dir(),
        ] {
            remove_temp_files(&dir, now);
        }
        remove_stages(&self.compas_dir().join("store"), now);
    }
}

fn is_real_dir(path: &Path) -> bool {
    fs::symlink_metadata(path).is_ok_and(|m| m.is_dir())
}

fn all_digits(s: &str) -> bool {
    !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit())
}

/// `.<name>.tmp.<pid>.<n>`, as made by `fs::temp_sibling`, or `.<name>.<pid>-<millis>.part`, an
/// upload to the player over adb that was cut short.
fn is_temp_name(name: &str) -> bool {
    let adb_part = name
        .strip_prefix('.')
        .and_then(|n| n.strip_suffix(".part"))
        .and_then(|n| n.rsplit_once('.'))
        .and_then(|(base, tail)| Some((base, tail.split_once('-')?)))
        .is_some_and(|(base, (pid, millis))| {
            !base.is_empty() && all_digits(pid) && all_digits(millis)
        });
    adb_part
        || name
            .strip_prefix('.')
            .and_then(|n| n.rsplit_once(".tmp."))
            .and_then(|(base, tail)| Some((base, tail.split_once('.')?)))
            .is_some_and(|(base, (pid, n))| !base.is_empty() && all_digits(pid) && all_digits(n))
}

/// The creation time in a `<label>.<secs>.<n>` stage folder name; the label
/// is a plugin id, optionally prefixed `uninstall-`.
fn stage_secs(name: &str) -> Option<u64> {
    let mut parts = name.rsplitn(3, '.');
    let (n, secs, label) = (parts.next()?, parts.next()?, parts.next()?);
    let label_ok = !label.is_empty()
        && label
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b"_.-".contains(&b));
    (label_ok && all_digits(n) && all_digits(secs)).then(|| secs.parse().ok())?
}

fn older_than_min_age(modified: Option<SystemTime>, now: SystemTime) -> bool {
    modified
        .and_then(|m| now.duration_since(m).ok())
        .is_some_and(|age| age >= MIN_AGE)
}

fn remove_temp_files(dir: &Path, now: SystemTime) {
    if !is_real_dir(dir) {
        return;
    }
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let is_file = entry.file_type().is_ok_and(|t| t.is_file());
        let modified = entry.metadata().and_then(|m| m.modified()).ok();
        if is_file
            && is_temp_name(&entry.file_name().to_string_lossy())
            && older_than_min_age(modified, now)
        {
            let _ = fs::remove_file(entry.path());
        }
    }
}

fn remove_stages(store: &Path, now: SystemTime) {
    if !is_real_dir(store) {
        return;
    }
    let now_secs = now.duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs());
    let Ok(entries) = fs::read_dir(store) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let old = stage_secs(&entry.file_name().to_string_lossy())
            .is_some_and(|secs| now_secs.saturating_sub(secs) >= MIN_AGE.as_secs());
        let has_backup = fs::read_dir(&path).is_ok_and(|files| {
            files
                .flatten()
                .any(|f| f.file_name().to_string_lossy().ends_with(".backup"))
        });
        if old && is_real_dir(&path) && !has_backup {
            let _ = fs::remove_dir_all(&path);
        }
    }
    let _ = fs::remove_dir(store);
}

#[cfg(test)]
mod tests;
