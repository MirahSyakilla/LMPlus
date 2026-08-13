#[tauri::command]
pub fn get_device_fingerprint() -> Result<String, String> {
    Ok("stub-fingerprint".to_string())
}