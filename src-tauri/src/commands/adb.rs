use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard, OnceLock};
use std::time::{Duration, Instant};

use adb::Firmware;

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

fn locked<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// What was learned about each connected player, until it disconnects.
fn identified() -> &'static Mutex<HashMap<String, DeviceInfo>> {
    static CACHE: OnceLock<Mutex<HashMap<String, DeviceInfo>>> = OnceLock::new();
    CACHE.get_or_init(Mutex::default)
}

/// When and how often asking a player who it is has failed, per serial: a player whose shell
/// does not answer is not asked on every look.
fn failed_asks() -> &'static Mutex<HashMap<String, (u32, Instant)>> {
    static CACHE: OnceLock<Mutex<HashMap<String, (u32, Instant)>>> = OnceLock::new();
    CACHE.get_or_init(Mutex::default)
}

/// Wait before asking again after `failures` failed attempts: 5 s, 10 s, ... up to a minute.
fn ask_backoff(failures: u32) -> Duration {
    Duration::from_secs((5u64 << failures.saturating_sub(1).min(4)).min(60))
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
            let info = adb::identify(&adb, &device.serial).await.ok();
            if let Some(info) = &info {
                // Keeps the quick listing in step with what this full look found.
                locked(identified()).insert(device.serial.clone(), info.clone());
            }
            info
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

fn view(device: adb::AdbDevice, info: Option<DeviceInfo>) -> DeviceView {
    DeviceView {
        likely_player: device.looks_like_player(),
        serial: device.serial,
        state: device.state,
        product: device.product,
        model: device.model,
        info,
    }
}

/// The device list straight from `adb devices`, without asking any device anything: players
/// already identified carry what was learned, the others `info: None` until `adb_identify`
/// answers. Cheap enough to poll. Forgets players that are no longer online.
#[tauri::command]
pub async fn adb_devices_quick(app: AppHandle) -> CommandResult<Vec<DeviceView>> {
    let devices = adb(&app)?.devices_quick().await?;
    let online = |serial: &String| {
        devices
            .iter()
            .any(|d| d.state == "device" && &d.serial == serial)
    };
    locked(failed_asks()).retain(|serial, _| online(serial));
    let mut known = locked(identified());
    known.retain(|serial, _| online(serial));
    Ok(devices
        .into_iter()
        .map(|d| {
            let info = known.get(&d.serial).cloned();
            view(d, info)
        })
        .collect())
}

/// Identifies one player, once per connection: the answer is kept until `adb_devices_quick`
/// sees it go. A failed attempt is not kept.
#[tauri::command]
pub async fn adb_identify(app: AppHandle, serial: String) -> CommandResult<DeviceInfo> {
    if let Some(info) = locked(identified()).get(&serial) {
        return Ok(info.clone());
    }
    if let Some((failures, at)) = locked(failed_asks()).get(&serial) {
        if at.elapsed() < ask_backoff(*failures) {
            return Err(CommandError::new(
                "adb",
                "The player did not answer; asking again shortly.",
            ));
        }
    }
    match adb::try_identify(&adb(&app)?, &serial).await {
        Ok(info) => {
            if info.firmware == Firmware::Unknown {
                // Still starting up, or a firmware that shows no known program: not kept, and
                // asked again only after a growing wait, not on every look.
                let mut asks = locked(failed_asks());
                let failures = asks.get(&serial).map_or(0, |(n, _)| *n);
                asks.insert(serial, (failures.saturating_add(1), Instant::now()));
            } else {
                locked(failed_asks()).remove(&serial);
                locked(identified()).insert(serial, info.clone());
            }
            Ok(info)
        }
        Err(failure) => {
            let mut asks = locked(failed_asks());
            let failures = asks.get(&serial).map_or(0, |(n, _)| *n);
            asks.insert(serial, (failures.saturating_add(1), Instant::now()));
            Err(failure.into())
        }
    }
}

/// Restarts the adb server, only because the user pressed the button: it also drops whatever
/// else uses the server (a shell, scrcpy, forwards), and it must not cut an install or backup.
#[tauri::command]
pub async fn adb_restart_server(app: AppHandle, state: State<'_, AppState>) -> CommandResult<()> {
    let _busy = state.try_busy().map_err(|_| {
        CommandError::new(
            "busy",
            "An install or backup is running. Restart adb when it has finished.",
        )
    })?;
    locked(identified()).clear();
    locked(failed_asks()).clear();
    Ok(adb(&app)?.restart_server().await?)
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
    // What was learned about the player is out of date once an install starts.
    locked(identified()).clear();
    let report = match kind {
        Kind::FullImage => adb::install_full_image(&adb, &serial, &request, &progress).await,
        Kind::PlayerOnly => adb::install_player_only(&adb, &serial, &request, &progress).await,
    };
    locked(identified()).clear();
    Ok(report?)
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
