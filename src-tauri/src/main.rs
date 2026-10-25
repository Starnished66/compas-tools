// Hides the console window on Windows release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod error;
mod sidecar;
mod state;

use commands::{adb, app_update, card, catalog, library, music, releases};
use tauri::Manager;

fn main() {
    let builder = tauri::Builder::default()
        // Two instances would write to the same card; focus the first instead.
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.unminimize();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .manage(state::AppState::default());
    #[cfg(feature = "updater")]
    let builder = builder.plugin(tauri_plugin_updater::Builder::new().build());
    builder
        .invoke_handler(tauri::generate_handler![
            app_update::app_update_check,
            app_update::app_update_install,
            adb::adb_status,
            adb::adb_download,
            adb::adb_devices,
            adb::adb_probe,
            adb::adb_remove_update_file,
            adb::adb_backup,
            adb::adb_install,
            adb::adb_collect_logs,
            adb::adb_save_logs,
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
            library::library_info,
            library::library_scan,
            library::library_art,
            music::music_plan,
            music::music_copy,
            music::music_tidy,
            music::music_update_library,
            releases::releases_latest,
            releases::releases_place,
            releases::releases_other_upts,
        ])
        .run(tauri::generate_context!())
        .expect("failed to run Compás Tools");
}
