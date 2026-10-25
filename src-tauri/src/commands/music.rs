use std::path::PathBuf;

use serde::Serialize;
use tauri::ipc::Channel;
use tauri::State;

use compas_core::music::{self, CopyReport, PlanOptions, PlanSummary, TidyReport};
use compas_core::Progress;

use super::reporter;
use crate::error::{CommandError, CommandResult};
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
    let report = reporter(on_progress);
    tauri::async_runtime::spawn_blocking(move || {
        let plan = music::plan(&card, &sources, &options)?;
        let report = music::copy_plan(&card, &plan, &report)?;
        Ok::<_, compas_core::Error>(CopyView {
            report,
            tidy: music::tidy(&card)?,
        })
    })
    .await
    .map_err(blocking_failed)?
    .map_err(Into::into)
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
