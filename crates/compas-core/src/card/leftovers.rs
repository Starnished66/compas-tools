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

/// `.<name>.tmp.<pid>.<n>`, as made by `fs::temp_sibling`.
fn is_temp_name(name: &str) -> bool {
    name.strip_prefix('.')
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
mod tests {
    use std::fs::File;
    use std::time::{Duration, SystemTime};

    use super::*;
    use crate::test_support::{fake_card, write_file};

    fn age(path: &Path, hours: u64) {
        let when = SystemTime::now() - Duration::from_secs(hours * 3600);
        File::options()
            .write(true)
            .open(path)
            .unwrap()
            .set_modified(when)
            .unwrap();
    }

    fn now_secs() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
    }

    #[test]
    fn recognises_only_our_own_names() {
        assert!(is_temp_name(".r1.upt.tmp.123.4"));
        assert!(!is_temp_name(".tmp.notes"));
        assert!(!is_temp_name("r1.upt.tmp.123.4"));
        assert!(!is_temp_name(".x.tmp.12a.4"));
        assert_eq!(
            stage_secs("example.plugin.1700000000.3"),
            Some(1_700_000_000)
        );
        assert_eq!(stage_secs("uninstall-a.1700000000.0"), Some(1_700_000_000));
        assert_eq!(stage_secs("Music.1700000000.3"), None);
        assert_eq!(stage_secs("a.b"), None);
    }

    #[test]
    fn removes_old_partials_and_stages_but_keeps_recent_ones_backups_and_user_files() {
        let (dir, card) = fake_card();
        let root = dir.path();
        let old = ".plugins/.store_installed.tmp.999999.1";
        write_file(root, old, b"partial");
        age(&root.join(old), 2);
        write_file(root, ".plugins/.fresh.tmp.999999.2", b"maybe in use");
        write_file(root, ".plugins/.tmp.notes", b"user file");
        age(&root.join(".plugins/.tmp.notes"), 2);
        write_file(root, "r1.upt", b"image");
        write_file(root, ".compas/store/a.1.0/0.part", b"half");
        write_file(root, ".compas/store/b.1.1/0.backup", b"original");
        write_file(
            root,
            &format!(".compas/store/c.{}.2/0.part", now_secs()),
            b"fresh",
        );
        write_file(root, ".compas/store/Music/song.mp3", b"not ours");

        card.clean_leftovers();

        assert!(!root.join(old).exists());
        assert!(root.join(".plugins/.fresh.tmp.999999.2").exists());
        assert!(root.join(".plugins/.tmp.notes").exists());
        assert!(root.join("r1.upt").exists());
        assert!(!root.join(".compas/store/a.1.0").exists());
        assert!(root.join(".compas/store/b.1.1/0.backup").exists());
        assert!(root.join(".compas/store/Music/song.mp3").exists());
        assert_eq!(fs::read_dir(root.join(".compas/store")).unwrap().count(), 3);
    }

    #[cfg(unix)]
    #[test]
    fn never_follows_links_off_the_card() {
        use std::os::unix::fs::symlink;

        let (dir, card) = fake_card();
        let outside = tempfile::tempdir().unwrap();
        write_file(outside.path(), "a.1.0/0.part", b"not yours");
        write_file(outside.path(), ".old.tmp.999999.1", b"not yours");
        age(&outside.path().join(".old.tmp.999999.1"), 2);

        symlink(outside.path(), dir.path().join(".compas/store")).unwrap();
        fs::remove_dir(dir.path().join(".plugins")).unwrap();
        symlink(outside.path(), dir.path().join(".plugins")).unwrap();
        // A stage-shaped link inside a real store folder is not followed either.
        card.clean_leftovers();
        assert!(outside.path().join("a.1.0/0.part").exists());
        assert!(outside.path().join(".old.tmp.999999.1").exists());

        fs::remove_file(dir.path().join(".compas/store")).unwrap();
        fs::create_dir(dir.path().join(".compas/store")).unwrap();
        symlink(
            outside.path().join("a.1.0"),
            dir.path().join(".compas/store/b.1.0"),
        )
        .unwrap();
        card.clean_leftovers();
        assert!(outside.path().join("a.1.0/0.part").exists());
    }
}
