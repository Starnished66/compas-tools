use std::path::PathBuf;

use serde::Serialize;
use tauri::ipc::Channel;
use tauri::State;

use compas_core::music::{self, CopyReport, PlanOptions, PlanSummary, TidyReport};
use compas_core::sdtool::LibraryUpdate;
use compas_core::transfer::{self, LibraryOutcome};
use compas_core::Progress;

use super::reporter;
use crate::error::{CommandError, CommandResult};
use crate::sidecar::sdtool;
use crate::state::AppState;

#[derive(Serialize)]
pub struct PlanView {
    #[serde(flatten)]
    summary: PlanSummary,
    free_bytes: u64,
    max_file_bytes: Option<u64>,
}

#[derive(Serialize)]
pub struct CopyView {
    #[serde(flatten)]
    report: CopyReport,
    tidy: TidyReport,
    library: LibraryOutcome,
}

fn options(
    card: &compas_core::card::Card,
    dest_dir: String,
    delete_extra: bool,
) -> CommandResult<PlanOptions> {
    Ok(PlanOptions {
        dest_dir,
        delete_extra,
        max_file_bytes: music::max_file_bytes(card)?,
    })
}

fn blocking_failed(error: tauri::Error) -> CommandError {
    CommandError::new("internal", error.to_string())
}

#[tauri::command]
pub async fn music_plan(
    state: State<'_, AppState>,
    sources: Vec<PathBuf>,
    dest_dir: String,
    delete_extra: bool,
) -> CommandResult<PlanView> {
    let card = state.card()?;
    let options = options(&card, dest_dir, delete_extra)?;
    let free_bytes = card.info()?.free_bytes;
    let max_file_bytes = options.max_file_bytes;
    let plan = tauri::async_runtime::spawn_blocking(move || music::plan(&card, &sources, &options))
        .await
        .map_err(blocking_failed)??;
    Ok(PlanView {
        summary: plan.summary(),
        free_bytes,
        max_file_bytes,
    })
}

/// Plans again and copies, so the result always matches the card as it is now.
#[tauri::command]
pub async fn music_copy(
    state: State<'_, AppState>,
    sources: Vec<PathBuf>,
    dest_dir: String,
    delete_extra: bool,
    on_progress: Channel<Progress>,
) -> CommandResult<CopyView> {
    let _busy = state.try_busy()?;
    let card = state.card()?;
    let options = options(&card, dest_dir, delete_extra)?;
    let progress = std::sync::Arc::new(reporter(on_progress));
    let (copy_card, copy_progress) = (card.clone(), progress.clone());
    let (report, tidy) = tauri::async_runtime::spawn_blocking(move || {
        let plan = music::plan(&copy_card, &sources, &options)?;
        let report = music::copy_plan(&copy_card, &plan, &*copy_progress)?;
        Ok::<_, compas_core::Error>((report, music::tidy(&copy_card)?))
    })
    .await
    .map_err(blocking_failed)??;

    // The copied files stay whatever happens to the library update.
    let library = transfer::update_after_copy(&sdtool(), &card, &report, &*progress).await;
    Ok(CopyView {
        report,
        tidy,
        library,
    })
}

/// Repeats just the library update, e.g. after it failed when music was copied.
#[tauri::command]
pub async fn music_update_library(
    state: State<'_, AppState>,
    on_progress: Channel<Progress>,
) -> CommandResult<LibraryUpdate> {
    let _busy = state.try_busy()?;
    let card = state.card()?;
    sdtool().ensure_library_ready(&card).await?;
    Ok(sdtool()
        .update_library(&card, &reporter(on_progress))
        .await?)
}

#[tauri::command]
pub async fn music_tidy(state: State<'_, AppState>) -> CommandResult<TidyReport> {
    let _busy = state.try_busy()?;
    let card = state.card()?;
    tauri::async_runtime::spawn_blocking(move || music::tidy(&card))
        .await
        .map_err(blocking_failed)?
        .map_err(Into::into)
}
