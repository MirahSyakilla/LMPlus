#[tauri::command]
pub fn register_hotkeys(
    _app: tauri::AppHandle,
    _accounts_path: String,
    _settings_json: String,
) -> Result<serde_json::Value, String> {
    Ok(serde_json::json!([]))
}

#[tauri::command]
pub fn unregister_hotkeys(_app: tauri::AppHandle) -> Result<(), String> {
    Ok(())
}

pub fn action_for_shortcut(_shortcut: &str) -> Option<String> {
    None
}
