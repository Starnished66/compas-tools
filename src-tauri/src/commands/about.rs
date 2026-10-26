use tauri::path::BaseDirectory;
use tauri::{AppHandle, Manager};

use crate::error::{CommandError, CommandResult};

/// The notices for the app's Rust and npm dependencies, as installed beside it. `None` in a
/// build that was not made by the release workflow (it writes the file).
#[tauri::command]
pub fn about_notices(app: AppHandle) -> CommandResult<Option<String>> {
    const LIMIT: u64 = 8 << 20;
    let path = app
        .path()
        .resolve("licenses/THIRD_PARTY_NOTICES.txt", BaseDirectory::Resource)
        .map_err(|e| CommandError::new("io", e.to_string()))?;
    match std::fs::metadata(&path) {
        Ok(meta) if meta.is_file() && meta.len() <= LIMIT => std::fs::read_to_string(&path)
            .map(Some)
            .map_err(|e| CommandError::new("io", e.to_string())),
        _ => Ok(None),
    }
}
