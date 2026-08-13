pub mod config;
pub mod crypto;
pub mod license;
pub mod telemetry;
pub mod updater;

#[cfg(target_os = "windows")]
#[path = "fingerprint_win.rs"]
pub mod fingerprint;

#[cfg(not(target_os = "windows"))]
#[path = "fingerprint_stub.rs"]
pub mod fingerprint;

#[cfg(target_os = "windows")]
#[path = "hotkeys_win.rs"]
pub mod hotkeys;

#[cfg(not(target_os = "windows"))]
#[path = "hotkeys_stub.rs"]
pub mod hotkeys;

#[cfg(target_os = "windows")]
#[path = "macros_win.rs"]
pub mod macros;

#[cfg(not(target_os = "windows"))]
#[path = "macros_stub.rs"]
pub mod macros;

#[cfg(target_os = "windows")]
#[path = "map_zoom_win.rs"]
pub mod map_zoom;

#[cfg(not(target_os = "windows"))]
#[path = "map_zoom_stub.rs"]
pub mod map_zoom;

#[cfg(target_os = "windows")]
#[path = "process_win.rs"]
pub mod process;

#[cfg(not(target_os = "windows"))]
#[path = "process_stub.rs"]
pub mod process;

use tauri::Manager;

#[tauri::command]
fn get_app_paths() -> Result<serde_json::Value, String> {
    Ok(serde_json::json!({
        "base_path": "",
        "exe_path": "",
        "process_name": "",
        "accounts_path": "",
        "userdata_path": "",
    }))
}

#[tauri::command]
fn set_base_path(_path: String) -> Result<(), String> {
    Ok(())
}

#[tauri::command]
fn get_license_info() -> Result<String, String> {
    Ok(String::new())
}

#[tauri::command]
fn set_license_info(_info: String) -> Result<(), String> {
    Ok(())
}

#[tauri::command]
fn get_startup_time_ms() -> Result<i64, String> {
    Ok(0)
}

#[tauri::command]
fn set_startup_time_ms(_time_ms: i64) -> Result<(), String> {
    Ok(())
}

#[tauri::command]
fn get_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .invoke_handler(tauri::generate_handler![
            get_app_paths,
            set_base_path,
            get_license_info,
            set_license_info,
            get_startup_time_ms,
            set_startup_time_ms,
            get_version,
            config::load_accounts,
            config::get_settings,
            config::set_settings,
            config::get_hotkey_settings,
            config::set_hotkey,
            config::remove_hotkey,
            config::get_account_order,
            config::set_account_order,
            config::load_misc_cfg,
            config::save_misc_cfg,
            config::switch_account,
            config::add_account,
            config::rename_account,
            config::delete_account,
            license::verify_license,
            license::load_saved_license,
            license::get_config_signature_status,
            process::launch_game,
            process::kill_game,
            process::restart_game,
            process::is_game_running,
            process::is_another_instance_running,
            hotkeys::register_hotkeys,
            hotkeys::unregister_hotkeys,
            macros::execute_macro,
            macros::parse_misc_cfg,
            map_zoom::perform_map_zoom,
            map_zoom::stop_persistent_zoom,
            telemetry::collect_and_send_telemetry,
            telemetry::increment_hide_to_tray,
            updater::check_for_update,
            updater::perform_update,
            fingerprint::get_device_fingerprint,
        ])
        .setup(|app| {
            let window = app.get_webview_window("main").unwrap();
            let _ = window.set_title("LMPlus");
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}