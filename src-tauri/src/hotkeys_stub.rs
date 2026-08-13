#[tauri::command]
pub fn register_hotkeys(_window: tauri::Window, _accounts_path: String, _settings_json: String) -> Result<serde_json::Value, String> {
    Ok(serde_json::json!([]))
}

#[tauri::command]
pub fn unregister_hotkeys(_window: tauri::Window) -> Result<(), String> {
    Ok(())
}