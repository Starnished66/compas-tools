use std::path::PathBuf;
use std::sync::Arc;

use serde::Serialize;
use tauri::ipc::Channel;
use tauri::{AppHandle, Manager, State};

use compas_core::adb::{
    self, Adb, BackupReport, DeviceInfo, InstallReport, InstallRequest, LogBundle, ToolsSource,
    WaitOptions,
};
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
    /// The platform-tools release the download button installs.
    download_version: &'static str,
}

#[derive(Serialize)]
pub struct DeviceView {
    serial: String,
    /// `device`, `unauthorized`, `offline`, ...
    state: String,
    product: Option<String>,
    model: Option<String>,
    /// An Ingenic serial or the player's USB product; others are only probed when asked.
    likely_player: bool,
    /// Only for devices that answered.
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
        download_version: adb::PLATFORM_TOOLS_VERSION,
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
    let archive = adb::tools_archive(std::env::consts::OS);
    adb::download_platform_tools(&archive, &dir(&app, "data")?, &reporter(on_progress)).await?;
    status(&app).await
}

/// Lists connected devices. Only those that look like the player are queried; anything
/// else (somebody's phone) is left alone until the user asks.
#[tauri::command]
pub async fn adb_devices(app: AppHandle) -> CommandResult<Vec<DeviceView>> {
    let adb = adb(&app)?;
    let mut views = Vec::new();
    for device in adb.devices().await? {
        let likely_player = device.looks_like_player();
        let info = if device.state == "device" && likely_player {
            adb::identify(&adb, &device.serial).await.ok()
        } else {
            None
        };
        views.push(DeviceView {
            serial: device.serial,
            state: device.state,
            product: device.product,
            model: device.model,
            likely_player,
            info,
        });
    }
    Ok(views)
}

/// Looks at a device that did not look like the player, because the user said to.
#[tauri::command]
pub async fn adb_probe(app: AppHandle, serial: String) -> CommandResult<DeviceInfo> {
    Ok(adb::identify(&adb(&app)?, &serial).await?)
}

/// Deletes the update image an install left at the root of the card.
#[tauri::command]
pub async fn adb_remove_update_file(
    app: AppHandle,
    state: State<'_, AppState>,
    serial: String,
    remote_path: String,
) -> CommandResult<()> {
    let _busy = state.try_busy()?;
    Ok(adb::remove_staged_image(&adb(&app)?, &serial, &remote_path).await?)
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
