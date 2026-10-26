use serde::Serialize;
use tauri::ipc::Channel;
use tauri::State;

use compas_core::sdtool::{
    library_support, ArtDone, LibraryInfo, LibrarySupport, ScanDone, ScanOptions,
};
use compas_core::Progress;

use super::reporter;
use crate::error::CommandResult;
use crate::sidecar::sdtool;
use crate::state::AppState;

#[derive(Serialize)]
pub struct LibraryStatus {
    #[serde(flatten)]
    info: LibraryInfo,
    /// Whether the card's player writes the database layout this tool writes;
    /// otherwise the player builds the library itself.
    support: LibrarySupport,
}

#[tauri::command]
pub async fn library_info(state: State<'_, AppState>) -> CommandResult<LibraryStatus> {
    let card = state.card()?;
    let info = sdtool().info(&card).await?;
    Ok(LibraryStatus {
        support: library_support(&card, &info)?,
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
    sdtool().ensure_library_ready(&card).await?;
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
    sdtool().ensure_library_ready(&card).await?;
    Ok(sdtool()
        .art(&card, None, None, &reporter(on_progress))
        .await?)
}
