use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::Path;

use serde::Serialize;
use sha2::{Digest, Sha256};

use super::plan::{Plan, PlanItem};
use crate::card::Card;
use crate::error::{Error, IoContext, Result};
use crate::fs::{ensure_free, persist, sha256_file, temp_sibling};
use crate::progress::{OnProgress, Phase, Progress};

const CHUNK: usize = 256 * 1024;

#[derive(Debug, Clone, Serialize)]
pub struct Failure {
    pub path: String,
    pub message: String,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct CopyReport {
    pub copied: u64,
    /// Of `copied`, how many replaced a different file.
    pub replaced: u64,
    pub skipped_identical: u64,
    pub deleted: u64,
    pub failed: Vec<Failure>,
}

/// Copies every planned file, checking each one by reading it back from the
/// card, then (for a mirror) removes the files the plan lists, but only when
/// everything was copied. A file that fails does not stop the others.
pub fn copy_plan(card: &Card, plan: &Plan, on_progress: OnProgress<'_>) -> Result<CopyReport> {
    let transient = plan
        .items
        .iter()
        .filter(|i| i.exists)
        .map(|i| i.size)
        .max()
        .unwrap_or(0);
    let fresh: u64 = plan
        .items
        .iter()
        .filter(|i| !i.exists)
        .map(|i| i.size)
        .sum();
    ensure_free(card.root(), fresh + transient)?;

    let total = plan.total_bytes();
    let mut done = 0;
    let mut report = CopyReport::default();
    for item in &plan.items {
        let before = done;
        let outcome = copy_item(card, item, &mut |n| {
            done += n;
            on_progress(Progress {
                phase: Phase::Copying,
                done,
                total,
            });
        });
        match outcome {
            Ok(Outcome::Copied) => {
                report.copied += 1;
                report.replaced += u64::from(item.exists);
            }
            Ok(Outcome::Identical) => report.skipped_identical += 1,
            Err(error) => report.failed.push(Failure {
                path: item.source.display().to_string(),
                message: error.to_string(),
            }),
        }
        // Whatever happened, this file's share of the bar is spent.
        done = before + item.size;
        on_progress(Progress {
            phase: Phase::Copying,
            done,
            total,
        });
    }

    if report.failed.is_empty() {
        report.deleted = remove_deletions(card, plan);
    }
    Ok(report)
}

enum Outcome {
    Copied,
    Identical,
}

fn copy_item(card: &Card, item: &PlanItem, on_chunk: &mut dyn FnMut(u64)) -> Result<Outcome> {
    let dest = card.resolve(&item.dest, true)?;
    let source_hash = sha256_file(&item.source)?;
    let same_size = fs::symlink_metadata(&dest).is_ok_and(|m| m.is_file() && m.len() == item.size);
    if same_size && sha256_file(&dest)? == source_hash {
        return Ok(Outcome::Identical);
    }

    let tmp = temp_sibling(&dest);
    let result = copy_verified(&item.source, &tmp, item.size, &source_hash, on_chunk)
        .and_then(|()| persist(&tmp, &dest));
    if result.is_err() {
        let _ = fs::remove_file(&tmp);
    }
    result.map(|()| Outcome::Copied)
}

/// Writes `source` to `tmp`, then reads `tmp` back and compares its hash.
fn copy_verified(
    source: &Path,
    tmp: &Path,
    size: u64,
    source_hash: &str,
    on_chunk: &mut dyn FnMut(u64),
) -> Result<()> {
    let mut input = File::open(source).at(source)?;
    let mut output = File::create_new(tmp).at(tmp)?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0u8; CHUNK];
    let mut written = 0u64;
    loop {
        let n = input.read(&mut buffer).at(source)?;
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
        output.write_all(&buffer[..n]).at(tmp)?;
        written += n as u64;
        on_chunk(n as u64);
    }
    output.sync_all().at(tmp)?;
    drop(output);

    if written != size || format!("{:x}", hasher.finalize()) != source_hash {
        return Err(Error::Verification(format!(
            "{} changed while it was copied",
            source.display()
        )));
    }
    if sha256_file(tmp)? != source_hash {
        return Err(Error::Verification(format!(
            "{} was not written correctly",
            source.display()
        )));
    }
    Ok(())
}

/// Removes mirrored-away files, then folders they leave empty.
fn remove_deletions(card: &Card, plan: &Plan) -> u64 {
    let mut deleted = 0;
    for relative in &plan.deletions {
        let Ok(path) = card.resolve(relative, false) else {
            continue;
        };
        if !fs::symlink_metadata(&path).is_ok_and(|m| m.is_file())
            || fs::remove_file(&path).is_err()
        {
            continue;
        }
        deleted += 1;
        let mut parent = path.parent();
        while let Some(dir) = parent.filter(|d| *d != card.root()) {
            if fs::remove_dir(dir).is_err() {
                break; // not empty, or the mirror's own folder is still in use
            }
            parent = dir.parent();
        }
    }
    deleted
}

#[cfg(test)]
mod tests;
