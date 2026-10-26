use std::sync::Arc;

use serde::Serialize;

use compas_core::releases::{self, Freshness, Release};
use tauri::State;

use crate::error::CommandResult;
use crate::state::AppState;

#[derive(Serialize)]
pub struct LatestRelease {
    release: Release,
    freshness: Freshness,
}

/// Fetches the latest release. `installed` is the version read from the player over adb;
/// without it the freshness is unknown, because nothing else tells the player's version.
#[tauri::command]
pub async fn releases_latest(
    state: State<'_, AppState>,
    refresh: bool,
    installed: Option<String>,
) -> CommandResult<LatestRelease> {
    let release = match state.recent_release() {
        Some(release) if !refresh => release,
        _ => {
            let release = Arc::new(releases::fetch_latest().await?);
            state.set_release(release.clone());
            release
        }
    };
    let freshness = release.freshness(installed.as_deref().filter(|v| !v.is_empty()));
    Ok(LatestRelease {
        release: (*release).clone(),
        freshness,
    })
}
