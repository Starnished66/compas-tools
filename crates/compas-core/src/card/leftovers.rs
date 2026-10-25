use std::fs;
use std::path::Path;

use super::Card;

impl Card {
    /// Removes what earlier runs that crashed left behind: partial downloads
    /// named like our temporary files, and staging folders that hold no
    /// backups (a folder with backups may hold the only copy of a user's file).
    /// Best effort; call it only while no operation is running.
    pub fn clean_leftovers(&self) {
        for dir in [self.root(), &self.compas_dir(), &self.plugins_dir()] {
            remove_temp_files(dir);
        }
        remove_stages(&self.compas_dir().join("store"));
    }
}

/// `.<name>.tmp.<pid>.<n>`, as made by `fs::temp_sibling`, from another process.
fn is_foreign_temp(name: &str) -> bool {
    let Some((_, tail)) = name.strip_prefix('.').and_then(|n| n.rsplit_once(".tmp.")) else {
        return false;
    };
    let Some((pid, counter)) = tail.split_once('.') else {
        return false;
    };
    let digits = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
    digits(pid) && digits(counter) && pid != std::process::id().to_string()
}

fn remove_temp_files(dir: &Path) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let is_file = entry.file_type().is_ok_and(|t| t.is_file());
        if is_file && is_foreign_temp(&entry.file_name().to_string_lossy()) {
            let _ = fs::remove_file(entry.path());
        }
    }
}

fn remove_stages(store: &Path) {
    let Ok(entries) = fs::read_dir(store) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let real_dir = entry.file_type().is_ok_and(|t| t.is_dir());
        let has_backup = fs::read_dir(&path).is_ok_and(|files| {
            files
                .flatten()
                .any(|f| f.file_name().to_string_lossy().ends_with(".backup"))
        });
        if real_dir && !has_backup {
            let _ = fs::remove_dir_all(&path);
        }
    }
    let _ = fs::remove_dir(store);
}

#[cfg(test)]
mod tests {
    use crate::test_support::{fake_card, write_file};

    #[test]
    fn removes_old_partials_and_empty_stages_but_keeps_backups_and_user_files() {
        let (dir, card) = fake_card();
        let root = dir.path();
        write_file(root, ".r1.upt.tmp.999999.0", b"partial");
        write_file(root, ".compas/.compas_player.tmp.999999.3", b"partial");
        write_file(root, ".plugins/.store_installed.tmp.999999.1", b"partial");
        let ours = format!(".plugins/.mine.tmp.{}.0", std::process::id());
        write_file(root, &ours, b"in use");
        write_file(root, ".plugins/.tmp.notes", b"user file");
        write_file(root, "r1.upt", b"image");
        write_file(root, ".compas/store/a.1.0/0.part", b"half");
        write_file(root, ".compas/store/b.1.1/0.backup", b"original");

        card.clean_leftovers();

        assert!(!root.join(".r1.upt.tmp.999999.0").exists());
        assert!(!root.join(".compas/.compas_player.tmp.999999.3").exists());
        assert!(!root.join(".plugins/.store_installed.tmp.999999.1").exists());
        assert!(root.join(&ours).exists());
        assert!(root.join(".plugins/.tmp.notes").exists());
        assert!(root.join("r1.upt").exists());
        assert!(!root.join(".compas/store/a.1.0").exists());
        assert!(root.join(".compas/store/b.1.1/0.backup").exists());
    }
}
