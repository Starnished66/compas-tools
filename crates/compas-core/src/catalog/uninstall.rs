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
    let record = Record::read(card)?;
    let plugin = record
        .get(id)
        .ok_or_else(|| Error::NotFound(format!("{id} is not installed by the store")))?;
    let stage = StageDir::create(&card.compas_dir().join("store"), &format!("uninstall-{id}"))?;

    let files: Vec<_> = plugin.files.iter().filter(|f| !f.keep).collect();
    let mut moved = Vec::new();
    let mut result = Ok(());
    for (i, file) in files.iter().enumerate() {
        on_progress(Progress {
            phase: Phase::Removing,
            done: i as u64,
            total: files.len() as u64,
        });
        let Ok(target) = card.resolve(&file.dest, false) else {
            continue;
        };
        if !fs::symlink_metadata(&target).is_ok_and(|m| m.is_file()) {
            continue;
        }
        let backup = stage.path().join(format!("{i}.backup"));
        match fs::rename(&target, &backup).at(&target) {
            Ok(()) => moved.push(Applied {
                target,
                backup: Some(backup),
            }),
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
    on_progress(Progress {
        phase: Phase::Removing,
        done: files.len() as u64,
        total: files.len() as u64,
    });
    Ok(())
}

#[cfg(test)]
mod tests;
