use compas_core::sdtool::SdTool;

/// The bundled `compas-sdtool`, which Tauri places next to the app executable
/// (and next to the debug binary during development).
pub fn sdtool() -> SdTool {
    let name = if cfg!(windows) {
        "compas-sdtool.exe"
    } else {
        "compas-sdtool"
    };
    let dir = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(std::path::Path::to_path_buf))
        .unwrap_or_default();
    SdTool::new(dir.join(name))
}
