//! Updates of Compás Tools itself. Compiled in only with the `updater` feature, which needs
//! the signing key described in `docs/RELEASING.md`; without it the app never looks.

use serde::{Deserialize, Serialize};
#[cfg(feature = "updater")]
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::ipc::Channel;
use tauri::AppHandle;

use crate::error::CommandResult;

#[cfg(feature = "updater")]
static UPDATE_INSTALLING: AtomicBool = AtomicBool::new(false);

#[cfg(feature = "updater")]
struct UpdateInstallGuard;

#[cfg(feature = "updater")]
impl UpdateInstallGuard {
    fn acquire() -> Result<Self, crate::error::CommandError> {
        UPDATE_INSTALLING
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .map(|_| Self)
            .map_err(|_| {
                crate::error::CommandError::new(
                    "update_busy",
                    "A Compás Tools update is already in progress.",
                )
            })
    }
}

#[cfg(feature = "updater")]
impl Drop for UpdateInstallGuard {
    fn drop(&mut self) {
        UPDATE_INSTALLING.store(false, Ordering::Release);
    }
}

#[derive(Serialize)]
pub struct AppUpdate {
    /// Whether this build can update itself at all.
    enabled: bool,
    version: Option<String>,
    notes: Option<String>,
    current_version: String,
    channel: UpdateChannel,
}

#[derive(Serialize)]
pub struct AppUpdateProgress {
    phase: &'static str,
    done: u64,
    total: u64,
}

#[derive(Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum UpdateChannel {
    Stable,
    Beta,
}

impl UpdateChannel {
    #[cfg(feature = "updater")]
    pub(crate) fn endpoint(self) -> &'static str {
        match self {
            Self::Stable => {
                "https://github.com/Starnished66/compas-tools/releases/latest/download/latest.json"
            }
            Self::Beta => {
                "https://github.com/Starnished66/compas-tools/releases/download/updates-beta/latest.json"
            }
        }
    }
}

#[cfg(not(feature = "updater"))]
#[tauri::command]
pub async fn app_update_check(app: AppHandle, channel: UpdateChannel) -> CommandResult<AppUpdate> {
    Ok(AppUpdate {
        enabled: false,
        version: None,
        notes: None,
        current_version: app.package_info().version.to_string(),
        channel,
    })
}

#[cfg(not(feature = "updater"))]
#[tauri::command]
pub async fn app_update_install(
    app: AppHandle,
    channel: UpdateChannel,
    _expected_version: String,
    _on_progress: Channel<AppUpdateProgress>,
) -> CommandResult<AppUpdate> {
    Ok(AppUpdate {
        enabled: false,
        version: None,
        notes: None,
        current_version: app.package_info().version.to_string(),
        channel,
    })
}

#[cfg(feature = "updater")]
#[tauri::command]
pub async fn app_update_check(app: AppHandle, channel: UpdateChannel) -> CommandResult<AppUpdate> {
    let update = super::app_update_feed::check(&app, channel).await?;
    Ok(AppUpdate {
        enabled: true,
        version: update.as_ref().map(|u| u.version.clone()),
        notes: update.and_then(|u| u.body),
        current_version: app.package_info().version.to_string(),
        channel,
    })
}

/// Downloads and installs the newer version; the app has to be restarted afterwards.
#[cfg(feature = "updater")]
#[tauri::command]
pub async fn app_update_install(
    app: AppHandle,
    channel: UpdateChannel,
    expected_version: String,
    on_progress: Channel<AppUpdateProgress>,
) -> CommandResult<AppUpdate> {
    let _install_guard = UpdateInstallGuard::acquire()?;
    let update = super::app_update_feed::check(&app, channel)
        .await?
        .ok_or_else(|| {
            crate::error::CommandError::new(
                "update_changed",
                "The update is no longer available. Check for updates again.",
            )
        })?;
    if update.version != expected_version {
        return Err(crate::error::CommandError::new(
            "update_changed",
            "The available update changed. Check for updates again before installing.",
        ));
    }

    let bytes = super::app_update_feed::download_artifact(&update, |done, total| {
        let _ = on_progress.send(AppUpdateProgress {
            phase: "downloading",
            done,
            total: total.unwrap_or(0),
        });
    })
    .await?;
    let _ = on_progress.send(AppUpdateProgress {
        phase: "installing",
        done: 1,
        total: 1,
    });
    let installed_version = update.version.clone();
    tauri::async_runtime::spawn_blocking(move || update.install(bytes))
        .await
        .map_err(|error| crate::error::CommandError::new("update", error.to_string()))?
        .map_err(|error| crate::error::CommandError::new("update", error.to_string()))?;
    // On Windows the updater launches its installer and exits the app itself. On macOS and
    // Linux the installed bundle needs the app process to restart to pick up the new version.
    #[cfg(not(target_os = "windows"))]
    app.request_restart();
    Ok(AppUpdate {
        enabled: true,
        version: Some(installed_version),
        notes: None,
        current_version: app.package_info().version.to_string(),
        channel,
    })
}
