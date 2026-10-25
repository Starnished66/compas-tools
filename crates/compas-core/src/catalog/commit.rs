//! Replacing files on the card with staged ones, all or nothing
//! (`plugin_store_commit_local`).

use std::fs;
use std::io::ErrorKind;
use std::path::PathBuf;

use super::index::Plugin;
use super::install::Outcome;
use super::record::Record;
use crate::card::Card;
use crate::error::{Error, IoContext, Result};
use crate::fs::{sha256_file, StageDir};

/// A file moved aside (or created) by an operation that can be undone.
pub(super) struct Applied {
    pub target: PathBuf,
    pub backup: Option<PathBuf>,
}

/// Whether installing `plugin` would replace a file the store does not own:
/// an existing destination must be a regular file whose hash is the one
/// recorded for this plugin. Files marked `keep` are never replaced.
pub(super) fn conflict(card: &Card, plugin: &Plugin, record: &Record) -> Result<bool> {
    for file in plugin.files.iter().filter(|f| !f.keep) {
        let target = card.resolve(&file.dest, false)?;
        match fs::symlink_metadata(&target) {
            Ok(meta) if meta.is_file() => {
                if !record.tracks(&plugin.id, &file.dest, &sha256_file(&target)?) {
                    return Ok(true);
                }
            }
            Ok(_) => return Ok(true),
            Err(e) if e.kind() == ErrorKind::NotFound => {}
            Err(e) => return Err(e).at(&target),
        }
    }
    Ok(false)
}

/// Moves the staged `<i>.part` files into place, records the plugin, then
/// drops files the previous version had and this one does not. Any failure
/// restores the card to how it was.
pub(super) fn apply(
    card: &Card,
    stage: StageDir,
    plugin: &Plugin,
    force: bool,
    record: &Record,
) -> Result<Outcome> {
    if !force && conflict(card, plugin, record)? {
        return Ok(Outcome::NeedsConfirmation);
    }
    let mut applied = Vec::new();
    let result = place_files(card, &stage, plugin, &mut applied)
        .and_then(|()| record.with_installed(plugin).write(card));
    if let Err(error) = result {
        if !restore(&applied) {
            stage.keep();
        }
        return Err(error);
    }
    remove_dropped(card, plugin, record);
    Ok(Outcome::Done)
}

fn place_files(
    card: &Card,
    stage: &StageDir,
    plugin: &Plugin,
    applied: &mut Vec<Applied>,
) -> Result<()> {
    for (i, file) in plugin.files.iter().enumerate() {
        let target = card.resolve(&file.dest, true)?;
        let existing = fs::symlink_metadata(&target);
        if file.keep && existing.is_ok() {
            continue;
        }
        let backup = match existing {
            Ok(meta) if meta.is_file() => Some(stage.path().join(format!("{i}.backup"))),
            Ok(_) => {
                return Err(Error::Rejected(format!(
                    "{} is not a regular file",
                    target.display()
                )))
            }
            Err(_) => None,
        };
        if let Some(backup) = &backup {
            fs::rename(&target, backup).at(&target)?;
        }
        applied.push(Applied {
            target: target.clone(),
            backup,
        });
        fs::rename(stage.path().join(format!("{i}.part")), &target).at(&target)?;
    }
    Ok(())
}

/// Undoes `applied` in reverse. False when some file could not be put back,
/// in which case the backups must stay where they are.
pub(super) fn restore(applied: &[Applied]) -> bool {
    let mut complete = true;
    for step in applied.iter().rev() {
        let removed = match fs::remove_file(&step.target) {
            Ok(()) => true,
            Err(e) => e.kind() == ErrorKind::NotFound,
        };
        let restored = match &step.backup {
            Some(backup) => removed && fs::rename(backup, &step.target).is_ok(),
            None => removed,
        };
        complete &= restored;
    }
    complete
}

/// Removes files of the old version that the new one no longer ships, unless
/// the user changed them or they are `keep` files.
fn remove_dropped(card: &Card, plugin: &Plugin, record: &Record) {
    let Some(old) = record.get(&plugin.id) else {
        return;
    };
    for file in old.files.iter().filter(|f| !f.keep) {
        if plugin.dests().any(|d| d.eq_ignore_ascii_case(&file.dest)) {
            continue;
        }
        let Ok(target) = card.resolve(&file.dest, false) else {
            continue;
        };
        let untouched = fs::symlink_metadata(&target).is_ok_and(|m| m.is_file())
            && sha256_file(&target).is_ok_and(|sha| sha == file.sha256);
        if untouched {
            let _ = fs::remove_file(&target);
        }
    }
}
