use std::fs;

use super::commit::{restore, Applied};
use super::record::Record;
use crate::card::Card;
use crate::error::{Error, IoContext, Result};
use crate::fs::StageDir;
use crate::progress::{OnProgress, Phase, Progress};

/// Removes a store-installed plugin: its files (except `keep` ones) are moved
/// into a staging folder, the record is rewritten without it, and only then
/// are the files discarded. A failure puts everything back.
pub fn uninstall(card: &Card, id: &str, on_progress: OnProgress<'_>) -> Result<()> {
    let record = Record::read(card);
    let plugin = record
        .get(id)
        .ok_or_else(|| Error::NotFound(format!("{id} is not installed by the store")))?;
    let stage = StageDir::create(&card.compas_dir().join("store"), &format!("uninstall-{id}"))?;

    let files: Vec<_> = plugin.files.iter().filter(|f| !f.keep).collect();
    let mut moved = Vec::new();
    let mut result = Ok(());
    for (i, file) in files.iter().enumerate() {
        on_progress(Progress { phase: Phase::Removing, done: i as u64, total: files.len() as u64 });
        let Ok(target) = card.resolve(&file.dest, false) else { continue };
        if !fs::symlink_metadata(&target).is_ok_and(|m| m.is_file()) {
            continue;
        }
        let backup = stage.path().join(format!("{i}.backup"));
        match fs::rename(&target, &backup).at(&target) {
            Ok(()) => moved.push(Applied { target, backup: Some(backup) }),
            Err(e) => {
                result = Err(e);
                break;
            }
        }
    }
    if result.is_ok() {
        result = record.without(id).write(card);
    }
    if let Err(error) = result {
        if !restore(&moved) {
            stage.keep();
        }
        return Err(error);
    }
    on_progress(Progress { phase: Phase::Removing, done: files.len() as u64, total: files.len() as u64 });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::super::install::install;
    use super::super::test_fixtures::{publish, spec};
    use super::*;
    use crate::test_support::fake_card;

    #[tokio::test]
    async fn removes_files_and_record_but_keeps_keep_files() {
        let (dir, card) = fake_card();
        let s = spec("p", "1.0", &[
            (".plugins/P.lua", "lua", false),
            ("Data/p.txt", "data", false),
            ("Data/user.txt", "mine", true),
        ]);
        let (_server, catalog) = publish(&[s]).await;
        install(&card, &catalog, "p", false, &|_| {}).await.unwrap();

        uninstall(&card, "p", &|_| {}).unwrap();
        assert!(!dir.path().join(".plugins/P.lua").exists());
        assert!(!dir.path().join("Data/p.txt").exists());
        assert!(dir.path().join("Data/user.txt").exists());
        assert!(Record::read(&card).plugins.is_empty());
        assert!(!dir.path().join(".compas/store").exists());
    }

    #[test]
    fn unknown_plugin_is_not_found() {
        let (_dir, card) = fake_card();
        assert!(matches!(uninstall(&card, "x", &|_| {}), Err(Error::NotFound(_))));
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn a_failed_move_puts_earlier_files_back() {
        use std::os::unix::fs::PermissionsExt;

        let (dir, card) = fake_card();
        let s = spec("p", "1.0", &[("Data/p.txt", "data", false), (".plugins/P.lua", "lua", false)]);
        let (_server, catalog) = publish(&[s]).await;
        install(&card, &catalog, "p", false, &|_| {}).await.unwrap();

        let plugins = dir.path().join(".plugins");
        std::fs::set_permissions(&plugins, std::fs::Permissions::from_mode(0o555)).unwrap();
        let result = uninstall(&card, "p", &|_| {});
        std::fs::set_permissions(&plugins, std::fs::Permissions::from_mode(0o755)).unwrap();

        if result.is_ok() {
            return; // running as root: permissions do not apply
        }
        assert!(dir.path().join("Data/p.txt").exists());
        assert!(plugins.join("P.lua").exists());
        assert!(Record::read(&card).get("p").is_some());
    }
}
