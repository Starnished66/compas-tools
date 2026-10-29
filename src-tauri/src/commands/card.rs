use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

use tauri::State;

use compas_core::card::{self, Card, CardInfo};

use crate::error::{CommandError, CommandResult};
use crate::state::AppState;

#[tauri::command]
pub async fn card_list() -> CommandResult<Vec<CardInfo>> {
    tauri::async_runtime::spawn_blocking(card::discover)
        .await
        .map_err(|e| CommandError::new("internal", e.to_string()))
}

#[tauri::command]
pub fn card_select(state: State<'_, AppState>, path: String) -> CommandResult<CardInfo> {
    let card = Card::open(path)?;
    let info = card.info()?;
    // Leftovers of crashed runs; skipped while an operation is writing.
    if let Ok(_busy) = state.try_busy() {
        card.clean_leftovers();
    }
    state.set_card(Some(card));
    Ok(info)
}

/// Re-reads the selected card (free space and `device.json` change under us).
#[tauri::command]
pub fn card_current(state: State<'_, AppState>) -> CommandResult<Option<CardInfo>> {
    state
        .selected_card()
        .map(|card| card.info())
        .transpose()
        .map_err(Into::into)
}

#[tauri::command]
pub fn card_clear(state: State<'_, AppState>) {
    state.set_card(None);
}

/// How long a folder may take to answer before it is assumed to be there: a hung network
/// share or wedged USB drive must neither freeze the app nor look like an unplugged card.
const PRESENT_TIMEOUT: Duration = Duration::from_secs(3);

/// Folders being opened right now. A hung mount keeps its blocking thread until the system
/// gives up, so a folder is not asked about again while an earlier check is still pending.
fn pending() -> &'static Mutex<HashSet<PathBuf>> {
    static PENDING: OnceLock<Mutex<HashSet<PathBuf>>> = OnceLock::new();
    PENDING.get_or_init(Mutex::default)
}

/// Takes a folder off [`pending`] when the check ends, however it ends.
struct Pending(PathBuf);

impl Drop for Pending {
    fn drop(&mut self) {
        pending()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .remove(&self.0);
    }
}

/// Opens a card's folder off the main thread. `None` when it did not answer in time, or an
/// earlier check of the same folder has not answered yet.
async fn opens(root: PathBuf) -> Option<bool> {
    let fresh = pending()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .insert(root.clone());
    if !fresh {
        return None;
    }
    let open = tauri::async_runtime::spawn_blocking(move || {
        let _done = Pending(root.clone());
        Card::open(root).is_ok()
    });
    tokio::time::timeout(PRESENT_TIMEOUT, open)
        .await
        .ok()
        .and_then(Result::ok)
}

/// Whether the selected player is still there: its folder and `.compas` can still be opened.
/// A drive picked by hand is not in the discovered list, so this is the way to tell. A folder
/// that does not answer in time counts as present.
#[tauri::command]
pub async fn card_present(state: State<'_, AppState>) -> CommandResult<bool> {
    match state.selected_card() {
        Some(card) => Ok(opens(card.root().to_path_buf()).await.unwrap_or(true)),
        None => Ok(true),
    }
}

/// Deselects the card if it is still the one at `root` and its folder cannot be opened.
/// The check and the change are one step, so a card chosen meanwhile is never dropped.
/// True when the card was dropped.
#[tauri::command]
pub async fn card_forget_if_missing(
    state: State<'_, AppState>,
    root: PathBuf,
) -> CommandResult<bool> {
    if opens(root.clone()).await != Some(false) {
        return Ok(false);
    }
    Ok(state.clear_card_if(&root))
}
