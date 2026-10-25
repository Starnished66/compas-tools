use serde::Serialize;
use tauri::ipc::Channel;
use tauri::State;

use compas_core::sdtool::{ArtDone, LibraryInfo, ScanDone, ScanOptions};
use compas_core::Progress;

use super::reporter;
use crate::error::CommandResult;
use crate::sidecar::sdtool;
use crate::state::AppState;

#[derive(Serialize)]
pub struct LibraryStatus {
    #[serde(flatten)]
    info: LibraryInfo,
    /// The player on the card writes another database layout than this tool.
    format_mismatch: bool,
}

#[tauri::command]
pub async fn library_info(state: State<'_, AppState>) -> CommandResult<LibraryStatus> {
    let card = state.card()?;
    let info = sdtool().info(&card).await?;
    let card_format = card.device()?.and_then(|d| d.database_format);
    Ok(LibraryStatus {
        format_mismatch: card_format.is_some_and(|f| f != info.database_format),
        info,
    })
}

#[tauri::command]
pub async fn library_scan(
    state: State<'_, AppState>,
    full: bool,
    on_progress: Channel<Progress>,
) -> CommandResult<ScanDone> {
    let _busy = state.try_busy()?;
    let card = state.card()?;
    let options = ScanOptions {
        full,
        mtime_offset: None,
    };
    Ok(sdtool()
        .scan(&card, options, &reporter(on_progress))
        .await?)
}

#[tauri::command]
pub async fn library_art(
    state: State<'_, AppState>,
    on_progress: Channel<Progress>,
) -> CommandResult<ArtDone> {
    let _busy = state.try_busy()?;
    let card = state.card()?;
    Ok(sdtool()
        .art(&card, None, None, &reporter(on_progress))
        .await?)
}
