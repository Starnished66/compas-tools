use std::sync::Arc;

use serde::Serialize;
use tauri::ipc::Channel;
use tauri::State;

use compas_core::card::Model;
use compas_core::releases::{self, Freshness, Kind, Release};
use compas_core::Progress;

use super::reporter;
use crate::error::CommandResult;
use crate::state::AppState;

#[derive(Serialize)]
pub struct LatestRelease {
    release: Release,
    freshness: Freshness,
}

#[derive(Serialize)]
pub struct PlacedFile {
    path: String,
    other_upts: Vec<String>,
}

/// Fetches the latest release and compares it with the selected card.
#[tauri::command]
pub async fn releases_latest(
    state: State<'_, AppState>,
    refresh: bool,
) -> CommandResult<LatestRelease> {
    let release = match state.recent_release() {
        Some(release) if !refresh => release,
        _ => {
            let release = Arc::new(releases::fetch_latest().await?);
            state.set_release(release.clone());
            release
        }
    };
    let device = state
        .selected_card()
        .map(|card| card.device())
        .transpose()?
        .flatten();
    let freshness = release.freshness(device.as_ref().map(|d| d.player_version.as_str()));
    Ok(LatestRelease {
        release: (*release).clone(),
        freshness,
    })
}

#[tauri::command]
pub async fn releases_place(
    state: State<'_, AppState>,
    kind: Kind,
    model: Model,
    on_progress: Channel<Progress>,
) -> CommandResult<PlacedFile> {
    let _busy = state.try_busy()?;
    let (card, release) = (state.card()?, state.release()?);
    let path = releases::place(&card, &release, kind, model, &reporter(on_progress)).await?;
    Ok(PlacedFile {
        path: path.display().to_string(),
        other_upts: releases::other_upts(&card, Some((kind, model)))?,
    })
}

#[tauri::command]
pub fn releases_other_upts(
    state: State<'_, AppState>,
    kind: Kind,
    model: Model,
) -> CommandResult<Vec<String>> {
    Ok(releases::other_upts(&state.card()?, Some((kind, model)))?)
}
