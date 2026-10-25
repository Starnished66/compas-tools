use std::sync::Arc;

use tauri::ipc::{Channel, Response};
use tauri::{AppHandle, Manager, State};

use compas_core::catalog::{self, Catalog, Entry, Outcome, UpdateAllReport};
use compas_core::Progress;

use super::reporter;
use crate::error::{CommandError, CommandResult};
use crate::state::AppState;

/// The catalog evaluated against the selected card. Fetches the index when
/// asked to, or when none is loaded yet.
#[tauri::command]
pub async fn catalog_entries(
    state: State<'_, AppState>,
    refresh: bool,
) -> CommandResult<Vec<Entry>> {
    let card = state.card()?;
    let catalog = match state.catalog() {
        Ok(catalog) if !refresh => catalog,
        _ => {
            let catalog = Arc::new(Catalog::fetch().await?);
            state.set_catalog(catalog.clone());
            catalog
        }
    };
    let plugin_api = card.device()?.map(|d| d.plugin_api);
    Ok(catalog::list(&catalog, &card, plugin_api)?)
}

#[tauri::command]
pub async fn catalog_install(
    state: State<'_, AppState>,
    plugin_id: String,
    force: bool,
    on_progress: Channel<Progress>,
) -> CommandResult<Outcome> {
    let _busy = state.try_busy()?;
    let (card, catalog) = (state.card()?, state.catalog()?);
    Ok(catalog::install(&card, &catalog, &plugin_id, force, &reporter(on_progress)).await?)
}

#[tauri::command]
pub async fn catalog_update(
    state: State<'_, AppState>,
    plugin_id: String,
    force: bool,
    on_progress: Channel<Progress>,
) -> CommandResult<Outcome> {
    let _busy = state.try_busy()?;
    let (card, catalog) = (state.card()?, state.catalog()?);
    Ok(catalog::update(&card, &catalog, &plugin_id, force, &reporter(on_progress)).await?)
}

#[tauri::command]
pub async fn catalog_update_all(
    state: State<'_, AppState>,
    on_progress: Channel<Progress>,
) -> CommandResult<UpdateAllReport> {
    let _busy = state.try_busy()?;
    let (card, catalog) = (state.card()?, state.catalog()?);
    Ok(catalog::update_all(&card, &catalog, &reporter(on_progress)).await?)
}

#[tauri::command]
pub async fn catalog_uninstall(
    state: State<'_, AppState>,
    plugin_id: String,
    on_progress: Channel<Progress>,
) -> CommandResult<()> {
    let _busy = state.try_busy()?;
    let card = state.card()?;
    Ok(catalog::uninstall(
        &card,
        &plugin_id,
        &reporter(on_progress),
    )?)
}

#[tauri::command]
pub async fn catalog_set_enabled(
    state: State<'_, AppState>,
    script: String,
    enabled: bool,
) -> CommandResult<()> {
    let _busy = state.try_busy()?;
    Ok(catalog::set_enabled(&state.card()?, &script, enabled)?)
}

/// Renames a damaged installed-plugins record out of the way.
#[tauri::command]
pub async fn catalog_move_record_aside(state: State<'_, AppState>) -> CommandResult<()> {
    let _busy = state.try_busy()?;
    Ok(catalog::move_record_aside(&state.card()?)?)
}

/// The verified preview image as raw bytes.
#[tauri::command]
pub async fn catalog_preview(
    app: AppHandle,
    state: State<'_, AppState>,
    plugin_id: String,
) -> CommandResult<Response> {
    let cache = app
        .path()
        .app_cache_dir()
        .map_err(|e| CommandError::new("io", e.to_string()))?
        .join("previews");
    let catalog = state.catalog()?;
    let path = catalog::ensure_preview(&catalog, &plugin_id, &cache).await?;
    let bytes = tokio::fs::read(&path)
        .await
        .map_err(|e| CommandError::new("io", format!("{}: {e}", path.display())))?;
    Ok(Response::new(bytes))
}
