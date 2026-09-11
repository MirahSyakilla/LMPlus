#[tauri::command]
pub fn perform_map_zoom(_exe_path: String, _persistent: bool) -> Result<(), String> {
    Ok(())
}

#[tauri::command]
pub fn stop_persistent_zoom() -> Result<(), String> {
    Ok(())
}
