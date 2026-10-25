use std::path::PathBuf;
use std::sync::Arc;

use serde::Serialize;
use tauri::ipc::Channel;
use tauri::{AppHandle, Manager, State};

use compas_core::adb::{
    self, Adb, BackupReport, DeviceInfo, InstallReport, InstallRequest, LogBundle, ToolsSource,
    WaitOptions,
};
use compas_core::card::Model;
use compas_core::releases::{self, Kind};
use compas_core::Progress;

use super::reporter;
use crate::error::{CommandError, CommandResult};
use crate::state::AppState;

#[derive(Serialize)]
pub struct ToolsStatus {
    found: bool,
    source: Option<ToolsSource>,
    version: Option<String>,
    /// `linux`, `macos` or `windows`.
    platform: &'static str,
}

#[derive(Serialize)]
pub struct DeviceView {
    serial: String,
    /// `device`, `unauthorized`, `offline`, ...
    state: String,
    /// Only for devices that answer.
    info: Option<DeviceInfo>,
}

fn dir(app: &AppHandle, kind: &str) -> CommandResult<PathBuf> {
    let base = match kind {
        "cache" => app.path().app_cache_dir(),
        _ => app.path().app_data_dir(),
    };
    base.map_err(|e| CommandError::new("io", e.to_string()))
}

fn adb(app: &AppHandle) -> CommandResult<Adb> {
    adb::locate(&dir(app, "data")?)
        .map(|found| found.adb)
        .ok_or_else(|| CommandError::new("no_adb", "adb is not installed yet."))
}

async fn status(app: &AppHandle) -> CommandResult<ToolsStatus> {
    let located = adb::locate(&dir(app, "data")?);
    let version = match &located {
        Some(found) => found
            .adb
            .version()
            .await
            .ok()
            .and_then(|v| v.lines().next().map(str::to_string)),
        None => None,
    };
    Ok(ToolsStatus {
        found: located.is_some(),
        source: located.map(|l| l.source),
        version,
        platform: std::env::consts::OS,
    })
}

#[tauri::command]
pub async fn adb_status(app: AppHandle) -> CommandResult<ToolsStatus> {
    status(&app).await
}

/// Downloads Google's platform-tools; the UI asks the user first.
#[tauri::command]
pub async fn adb_download(
    app: AppHandle,
    state: State<'_, AppState>,
    on_progress: Channel<Progress>,
) -> CommandResult<ToolsStatus> {
    let _busy = state.try_busy()?;
    let url = adb::tools_url(std::env::consts::OS);
    adb::download_platform_tools(&url, &dir(&app, "data")?, &reporter(on_progress)).await?;
    status(&app).await
}

#[tauri::command]
pub async fn adb_devices(app: AppHandle) -> CommandResult<Vec<DeviceView>> {
    let adb = adb(&app)?;
    let mut views = Vec::new();
    for device in adb.devices().await? {
        let info = if device.state == "device" {
            adb::identify(&adb, &device.serial).await.ok()
        } else {
            None
        };
        views.push(DeviceView {
            serial: device.serial,
            state: device.state,
            info,
        });
    }
    Ok(views)
}

#[tauri::command]
pub async fn adb_backup(
    app: AppHandle,
    state: State<'_, AppState>,
    serial: String,
    on_progress: Channel<Progress>,
) -> CommandResult<BackupReport> {
    let _busy = state.try_busy()?;
    let backups = dir(&app, "data")?.join("backups");
    Ok(adb::backup_user_data(&adb(&app)?, &serial, &backups, &reporter(on_progress)).await?)
}

#[tauri::command]
pub async fn adb_install(
    app: AppHandle,
    state: State<'_, AppState>,
    serial: String,
    kind: Kind,
    model: Model,
    on_progress: Channel<Progress>,
) -> CommandResult<InstallReport> {
    let _busy = state.try_busy()?;
    let release = match state.recent_release() {
        Some(release) => release,
        None => {
            let fetched = Arc::new(releases::fetch_latest().await?);
            state.set_release(fetched.clone());
            fetched
        }
    };
    let cache = dir(&app, "cache")?.join("releases");
    let request = InstallRequest {
        model,
        release: &release,
        cache_dir: &cache,
        wait: WaitOptions::default(),
    };
    let progress = reporter(on_progress);
    let adb = adb(&app)?;
    Ok(match kind {
        Kind::FullImage => adb::install_full_image(&adb, &serial, &request, &progress).await?,
        Kind::PlayerOnly => adb::install_player_only(&adb, &serial, &request, &progress).await?,
    })
}

/// Collects and masks the logs; the result is kept so saving writes exactly what was shown.
#[tauri::command]
pub async fn adb_collect_logs(
    app: AppHandle,
    state: State<'_, AppState>,
    serial: String,
    on_progress: Channel<Progress>,
) -> CommandResult<LogBundle> {
    let _busy = state.try_busy()?;
    let adb = adb(&app)?;
    let info = adb::identify(&adb, &serial).await?;
    let bundle = adb::collect_logs(&adb, &serial, &info, &reporter(on_progress)).await?;
    state.set_logs(Arc::new(bundle.clone()));
    Ok(bundle)
}

#[tauri::command]
pub fn adb_save_logs(state: State<'_, AppState>, path: PathBuf) -> CommandResult<()> {
    let bundle = state
        .logs()
        .ok_or_else(|| CommandError::new("no_logs", "No logs have been collected."))?;
    Ok(adb::write_zip(&bundle, &path)?)
}
