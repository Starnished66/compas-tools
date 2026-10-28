//! Player settings over ADB: the player's HTTP API through `adb forward`. The PIN the user types
//! stays in this process's memory for the session; it is never written, logged or sent back to
//! the screen. The session lock is never held across adb or network calls, so nothing here can
//! stall the app's exit.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use serde::Serialize;
use tauri::{AppHandle, Manager, State};
use tokio::sync::Mutex;

use compas_core::adb::{self, Adb};
use compas_core::settings::{
    self, Accepted, Applied, ApplyRequest, Connection, Pin, Player, Reply, Snapshot, Status,
};

use crate::error::{CommandError, CommandResult};

/// How long leaving may wait for adb to drop the forward.
const CLOSE_LIMIT: Duration = Duration::from_secs(5);

/// The player the settings screen is talking to.
struct Session {
    id: u64,
    adb: Adb,
    serial: String,
    port: u16,
    pin: Pin,
    player: Arc<Player>,
}

/// Held in Tauri's state; empty until the screen connects.
#[derive(Default)]
pub struct SettingsSession {
    slot: Mutex<Option<Session>>,
    next: AtomicU64,
}

/// What `settings_connect` answers: the session to quote in later calls, and what was found.
#[derive(Serialize)]
pub struct Opened {
    session: u64,
    #[serde(flatten)]
    connection: Connection,
}

async fn drop_forward(session: Session) {
    let _ = tokio::time::timeout(
        CLOSE_LIMIT,
        session.adb.remove_forward(&session.serial, session.port),
    )
    .await;
}

impl SettingsSession {
    /// Takes the forward away, whichever session it is. Used when the app exits.
    pub async fn close(&self) {
        let session = self.slot.lock().await.take();
        if let Some(session) = session {
            drop_forward(session).await;
        }
    }

    /// Closes the session only if it is still `id`: an old view cannot end a newer one.
    async fn close_if(&self, id: u64) {
        let session = {
            let mut slot = self.slot.lock().await;
            if slot.as_ref().is_some_and(|s| s.id == id) {
                slot.take()
            } else {
                None
            }
        };
        if let Some(session) = session {
            drop_forward(session).await;
        }
    }

    async fn player(&self, id: u64) -> CommandResult<Arc<Player>> {
        self.slot
            .lock()
            .await
            .as_ref()
            .filter(|s| s.id == id)
            .map(|s| s.player.clone())
            .ok_or_else(|| CommandError::new("no_session", "The player's settings are not open."))
    }
}

fn adb(app: &AppHandle) -> CommandResult<Adb> {
    let data = app
        .path()
        .app_data_dir()
        .map_err(|e| CommandError::new("io", e.to_string()))?;
    adb::locate(&data)
        .map(|found| found.adb)
        .ok_or_else(|| CommandError::new("no_adb", "adb is not installed yet."))
}

/// Opens the player's settings. `pin` is what the user typed; `None` reuses the one already held
/// for this player (Reload, Retry). The answer says what is missing, or carries the schema and
/// values, and always the id of the new session.
#[tauri::command]
pub async fn settings_connect(
    app: AppHandle,
    state: State<'_, SettingsSession>,
    serial: String,
    pin: Option<String>,
) -> CommandResult<Opened> {
    let adb = adb(&app)?;
    let id = state.next.fetch_add(1, Ordering::Relaxed) + 1;
    // Take the old session out and release the lock before any adb or network call.
    let previous = state.slot.lock().await.take();
    let kept = previous
        .as_ref()
        .filter(|s| s.serial == serial)
        .map(|s| s.pin.clone());
    if let Some(old) = previous {
        drop_forward(old).await;
    }
    let pin = match pin {
        Some(typed) => Pin::new(typed),
        None => kept.unwrap_or_else(|| Pin::new("")),
    };
    // On failure `open` has already taken its forward away; nothing is stored.
    let opened = settings::open(&adb, &serial, pin.clone()).await?;
    // A wrong PIN is not kept.
    let keep_pin = if matches!(opened.connection, Connection::PinRequired { .. }) {
        Pin::new("")
    } else {
        pin
    };
    let session = Session {
        id,
        adb,
        serial,
        port: opened.port,
        pin: keep_pin,
        player: opened.player,
    };
    let displaced = state.slot.lock().await.replace(session);
    if let Some(other) = displaced {
        // A newer connect finished in between; its forward is no longer wanted.
        drop_forward(other).await;
    }
    Ok(Opened {
        session: id,
        connection: opened.connection,
    })
}

/// The player's settings revision and epoch, to notice a change made on the device.
#[tauri::command]
pub async fn settings_status(
    state: State<'_, SettingsSession>,
    session: u64,
) -> CommandResult<Reply<Status>> {
    Ok(state.player(session).await?.status().await?)
}

/// Every value, or with `since` (`[revision, epoch]`) only what changed after it.
#[tauri::command]
pub async fn settings_values(
    state: State<'_, SettingsSession>,
    session: u64,
    since: Option<(u64, u64)>,
) -> CommandResult<Reply<Snapshot>> {
    Ok(state.player(session).await?.settings(since).await?)
}

#[tauri::command]
pub async fn settings_apply(
    state: State<'_, SettingsSession>,
    session: u64,
    request: ApplyRequest,
) -> CommandResult<Reply<Applied>> {
    Ok(state.player(session).await?.apply(&request).await?)
}

#[tauri::command]
pub async fn settings_rescan(
    state: State<'_, SettingsSession>,
    session: u64,
) -> CommandResult<Reply<Accepted>> {
    Ok(state.player(session).await?.rescan().await?)
}

/// Removes the forward and forgets the PIN, if `session` is still the current one.
#[tauri::command]
pub async fn settings_disconnect(
    state: State<'_, SettingsSession>,
    session: u64,
) -> CommandResult<()> {
    state.close_if(session).await;
    Ok(())
}
