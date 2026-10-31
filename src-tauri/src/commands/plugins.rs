//! Plugin Store and player-layout commands on the existing authenticated settings session.
use super::settings::SettingsSession;
use crate::error::CommandResult;
use compas_core::settings::{
    Capabilities, InstalledSnapshot, LayoutSnapshot, PluginAction, PreviewResult, Reply,
    StoreSnapshot,
};
use tauri::State;

#[tauri::command]
pub async fn player_plugins_capabilities(
    state: State<'_, SettingsSession>,
    session: u64,
) -> CommandResult<Capabilities> {
    Ok(state.player(session).await?.plugin_capabilities().await?)
}
#[tauri::command]
pub async fn player_plugin_store(
    state: State<'_, SettingsSession>,
    session: u64,
    offset: u64,
    limit: u64,
) -> CommandResult<Reply<StoreSnapshot>> {
    Ok(state
        .player(session)
        .await?
        .plugin_store(offset, limit)
        .await?)
}
#[tauri::command]
pub async fn player_plugin_store_preview(
    state: State<'_, SettingsSession>,
    session: u64,
    id: String,
) -> CommandResult<Reply<PreviewResult>> {
    Ok(state
        .player(session)
        .await?
        .store_plugin_preview(&id)
        .await?)
}
#[tauri::command]
pub async fn player_plugin_store_refresh(
    state: State<'_, SettingsSession>,
    session: u64,
) -> CommandResult<Reply<PluginAction>> {
    Ok(state.player(session).await?.plugin_store_refresh().await?)
}
#[tauri::command]
pub async fn player_plugin_install(
    state: State<'_, SettingsSession>,
    session: u64,
    id: String,
    force: Option<bool>,
) -> CommandResult<Reply<PluginAction>> {
    Ok(state
        .player(session)
        .await?
        .plugin_install(&id, force.unwrap_or(false))
        .await?)
}
#[tauri::command]
pub async fn player_plugin_update(
    state: State<'_, SettingsSession>,
    session: u64,
    id: String,
    force: Option<bool>,
) -> CommandResult<Reply<PluginAction>> {
    Ok(state
        .player(session)
        .await?
        .plugin_update(&id, force.unwrap_or(false))
        .await?)
}
#[tauri::command]
pub async fn player_plugin_update_all(
    state: State<'_, SettingsSession>,
    session: u64,
) -> CommandResult<Reply<PluginAction>> {
    Ok(state.player(session).await?.plugin_update_all().await?)
}
#[tauri::command]
pub async fn player_plugin_uninstall(
    state: State<'_, SettingsSession>,
    session: u64,
    id: String,
) -> CommandResult<Reply<PluginAction>> {
    Ok(state.player(session).await?.plugin_uninstall(&id).await?)
}
#[tauri::command]
pub async fn player_plugins_installed(
    state: State<'_, SettingsSession>,
    session: u64,
    offset: u64,
    limit: u64,
) -> CommandResult<Reply<InstalledSnapshot>> {
    Ok(state
        .player(session)
        .await?
        .installed_plugins(offset, limit)
        .await?)
}
#[tauri::command]
pub async fn player_plugin_set_enabled(
    state: State<'_, SettingsSession>,
    session: u64,
    id: String,
    enabled: bool,
) -> CommandResult<Reply<PluginAction>> {
    Ok(state
        .player(session)
        .await?
        .plugin_set_enabled(&id, enabled)
        .await?)
}
#[tauri::command]
pub async fn player_plugins_refresh(
    state: State<'_, SettingsSession>,
    session: u64,
) -> CommandResult<Reply<PluginAction>> {
    Ok(state.player(session).await?.plugins_refresh().await?)
}
#[tauri::command]
pub async fn player_plugin_open_settings(
    state: State<'_, SettingsSession>,
    session: u64,
    id: String,
) -> CommandResult<Reply<PluginAction>> {
    Ok(state
        .player(session)
        .await?
        .open_plugin_settings(&id)
        .await?)
}
#[tauri::command]
pub async fn player_layouts(
    state: State<'_, SettingsSession>,
    session: u64,
    offset: u64,
    limit: u64,
) -> CommandResult<Reply<LayoutSnapshot>> {
    Ok(state
        .player(session)
        .await?
        .player_layouts(offset, limit)
        .await?)
}
#[tauri::command]
pub async fn player_layout_preview(
    state: State<'_, SettingsSession>,
    session: u64,
    id: String,
) -> CommandResult<Reply<PreviewResult>> {
    Ok(state
        .player(session)
        .await?
        .player_layout_preview(&id)
        .await?)
}
#[tauri::command]
pub async fn player_layout_select(
    state: State<'_, SettingsSession>,
    session: u64,
    id: String,
) -> CommandResult<Reply<PluginAction>> {
    Ok(state
        .player(session)
        .await?
        .select_player_layout(&id)
        .await?)
}
