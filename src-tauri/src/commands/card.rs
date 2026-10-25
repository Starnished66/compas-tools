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
