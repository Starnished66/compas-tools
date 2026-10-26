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
    installed: Option<String>,
    without_device_info: Option<bool>,
) -> CommandResult<LatestRelease> {
    let release = match state.recent_release() {
        Some(release) if !refresh => release,
        _ => {
            let release = Arc::new(releases::fetch_latest().await?);
            state.set_release(release.clone());
            release
        }
    };
    let freshness = if installed.is_some() {
        // The Device screen passes what it read over adb; no selected player is consulted.
        match installed.as_deref().filter(|v| !v.is_empty()) {
            Some(version) => release.freshness(Some(version)),
            None if without_device_info == Some(true) => release.freshness_older_than_device_info(),
            None => Freshness::Unknown,
        }
    } else {
        match state.selected_card() {
            None => Freshness::Unknown,
            Some(card) => match card.device()? {
                Some(device) => release.freshness(Some(&device.player_version)),
                // No file: v1.2.0 and older, but also a newer player that has not written it.
                None if !card.compas_dir().join("device.json").exists() => {
                    release.freshness_probably_older()
                }
                None => Freshness::Unknown,
            },
        }
    };
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
