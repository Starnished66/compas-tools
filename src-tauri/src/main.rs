// Hides the console window on Windows release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod error;
mod state;

use commands::{card, catalog, releases};
use tauri::Manager;

fn main() {
    tauri::Builder::default()
        // Two instances would write to the same card; focus the first instead.
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.unminimize();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .manage(state::AppState::default())
        .invoke_handler(tauri::generate_handler![
            card::card_list,
            card::card_select,
            card::card_current,
            card::card_clear,
            catalog::catalog_entries,
            catalog::catalog_install,
            catalog::catalog_update,
            catalog::catalog_update_all,
            catalog::catalog_uninstall,
            catalog::catalog_set_enabled,
            catalog::catalog_move_record_aside,
            catalog::catalog_preview,
            releases::releases_latest,
            releases::releases_place,
            releases::releases_other_upts,
        ])
        .run(tauri::generate_context!())
        .expect("failed to run Compás Tools");
}
