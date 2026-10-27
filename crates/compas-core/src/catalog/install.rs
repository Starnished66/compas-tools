use std::fs;

use serde::Serialize;

use super::commit;
use super::index::PluginFile;
use super::record::{Record, MAX_RECORDED};
use super::state::is_incompatible;
use super::Catalog;
use crate::card::Card;
use crate::error::{Error, Result};
use crate::fs::{ensure_free, StageDir};
use crate::net::download_verified;
use crate::progress::{OnProgress, Phase, Progress};
use crate::version;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    Done,
    /// These local files would be replaced; call again with `force` once the user agrees.
    NeedsConfirmation(Vec<String>),
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Install,
    Update,
}

pub async fn install(
    card: &Card,
    catalog: &Catalog,
    id: &str,
    force: bool,
    on_progress: OnProgress<'_>,
) -> Result<Outcome> {
    apply(Mode::Install, card, catalog, id, force, on_progress).await
}

pub async fn update(
    card: &Card,
    catalog: &Catalog,
    id: &str,
    force: bool,
    on_progress: OnProgress<'_>,
) -> Result<Outcome> {
    apply(Mode::Update, card, catalog, id, force, on_progress).await
}

async fn apply(
    mode: Mode,
    card: &Card,
    catalog: &Catalog,
    id: &str,
    force: bool,
    on_progress: OnProgress<'_>,
) -> Result<Outcome> {
    let plugin = catalog.get(id)?;
    if is_incompatible(plugin, card.device()?.map(|d| d.plugin_api)) {
        return Err(Error::Rejected(
            "this plugin needs a newer player version".into(),
        ));
    }
    let record = Record::read(card)?;
    let installed = record.get(id);
    match (mode, installed) {
        (Mode::Install, Some(_)) if !force => {
            return Err(Error::Rejected("this plugin is already installed".into()))
        }
        (Mode::Update, Some(old)) if version::compare(&plugin.version, &old.version).is_gt() => {}
        (Mode::Update, _) => return Err(Error::Rejected("this plugin has no update".into())),
        (Mode::Install, None) if record.plugins.len() >= MAX_RECORDED => {
            return Err(Error::Rejected("too many plugins are installed".into()))
        }
        _ => {}
    }
    if !force {
        let files = commit::conflicts(card, plugin, &record)?;
        if !files.is_empty() {
            return Ok(Outcome::NeedsConfirmation(files));
        }
    }

    let wanted: Vec<(usize, &PluginFile)> = plugin
        .files
        .iter()
        .enumerate()
        .filter(|(_, f)| !(f.keep && fs::symlink_metadata(card.root().join(&f.dest)).is_ok()))
        .collect();
    let total: u64 = wanted.iter().map(|(_, f)| f.size).sum();
    ensure_free(card.root(), total)?;

    let stage = StageDir::create(&card.compas_dir().join("store"), &plugin.id)?;
    let mut finished = 0;
    for (i, file) in wanted {
        let part = stage.path().join(format!("{i}.part"));
        let base = finished;
        download_verified(
            &catalog.asset_url(&file.asset),
            &part,
            file.size,
            &file.sha256,
            |done, _| {
                on_progress(Progress {
                    phase: Phase::Downloading,
                    done: base + done,
                    total,
                });
            },
        )
        .await?;
        finished += file.size;
    }

    on_progress(Progress {
        phase: Phase::Installing,
        done: 0,
        total: 1,
    });
    let outcome = commit::apply(card, stage, plugin, force, &record)?;
    on_progress(Progress {
        phase: Phase::Installing,
        done: 1,
        total: 1,
    });
    Ok(outcome)
}

#[cfg(test)]
mod tests;
