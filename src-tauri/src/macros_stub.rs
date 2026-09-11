#[tauri::command]
pub fn execute_macro(
    _macro_name: String,
    _exe_path: String,
    _process_name: String,
) -> Result<(), String> {
    Ok(())
}

#[tauri::command]
pub fn parse_misc_cfg() -> Result<serde_json::Value, String> {
    Ok(serde_json::json!([]))
}
