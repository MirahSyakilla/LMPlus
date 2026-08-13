#[tauri::command]
pub fn launch_game(_exe_path: String, _process_name: String) -> Result<(), String> {
    Ok(())
}

#[tauri::command]
pub fn kill_game(_exe_path: String, _process_name: String) -> Result<bool, String> {
    Ok(false)
}

#[tauri::command]
pub fn restart_game(_exe_path: String, _process_name: String, _relaunch: bool) -> Result<(), String> {
    Ok(())
}

#[tauri::command]
pub fn is_game_running(_exe_path: String, _process_name: String) -> Result<bool, String> {
    Ok(false)
}

#[tauri::command]
pub fn is_another_instance_running() -> Result<bool, String> {
    Ok(false)
}