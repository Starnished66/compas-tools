//! Updates of Compás Tools itself. Compiled in only with the `updater` feature, which needs
//! the signing key described in `docs/RELEASING.md`; without it the app never looks.

use serde::Serialize;
use tauri::AppHandle;

use crate::error::CommandResult;

#[derive(Serialize)]
pub struct AppUpdate {
    /// Whether this build can update itself at all.
    enabled: bool,
    version: Option<String>,
    notes: Option<String>,
}

#[cfg(not(feature = "updater"))]
const DISABLED: AppUpdate = AppUpdate {
    enabled: false,
    version: None,
    notes: None,
};

#[cfg(not(feature = "updater"))]
#[tauri::command]
pub async fn app_update_check(_app: AppHandle) -> CommandResult<AppUpdate> {
    Ok(DISABLED)
}

#[cfg(not(feature = "updater"))]
#[tauri::command]
pub async fn app_update_install(_app: AppHandle) -> CommandResult<AppUpdate> {
    Ok(DISABLED)
}

#[cfg(feature = "updater")]
fn failed(error: impl std::fmt::Display) -> crate::error::CommandError {
    crate::error::CommandError::new("update", error.to_string())
}

/// Asks the update server (`latest.json` of the latest GitHub release) for a newer version.
#[cfg(feature = "updater")]
#[tauri::command]
pub async fn app_update_check(app: AppHandle) -> CommandResult<AppUpdate> {
    use tauri_plugin_updater::UpdaterExt;

    let update = app
        .updater()
        .map_err(failed)?
        .check()
        .await
        .map_err(failed)?;
    Ok(AppUpdate {
        enabled: true,
        version: update.as_ref().map(|u| u.version.clone()),
        notes: update.and_then(|u| u.body),
    })
}

/// Downloads and installs the newer version; the app has to be restarted afterwards.
#[cfg(feature = "updater")]
#[tauri::command]
pub async fn app_update_install(app: AppHandle) -> CommandResult<AppUpdate> {
    use tauri_plugin_updater::UpdaterExt;

    let update = app
        .updater()
        .map_err(failed)?
        .check()
        .await
        .map_err(failed)?;
    if let Some(update) = &update {
        update
            .download_and_install(|_, _| {}, || {})
            .await
            .map_err(failed)?;
    }
    Ok(AppUpdate {
        enabled: true,
        version: update.map(|u| u.version),
        notes: None,
    })
}
