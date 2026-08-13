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
use std::sync::Mutex;

struct RuntimeState {
    base_path: Mutex<String>,
    license_info: Mutex<String>,
    startup_time_ms: Mutex<i64>,
}

impl Default for RuntimeState {
    fn default() -> Self {
        Self {
            base_path: Mutex::new(String::new()),
            license_info: Mutex::new(String::new()),
            startup_time_ms: Mutex::new(0),
        }
    }
}

fn build_paths(base_path: &str) -> serde_json::Value {
    serde_json::json!({
        "base_path": base_path,
        "exe_path": format!("{}\\Game\\Lords Mobile PC.exe", base_path),
        "process_name": "Lords Mobile PC.exe",
        "accounts_path": format!("{}\\Accounts", base_path),
        "userdata_path": format!("{}\\userdata", base_path),
    })
}

#[tauri::command]
fn get_app_paths(state: tauri::State<RuntimeState>) -> Result<serde_json::Value, String> {
    let base_path = state.base_path.lock().map_err(|e| e.to_string())?;
    Ok(build_paths(&base_path))
}

#[tauri::command]
fn set_base_path(state: tauri::State<RuntimeState>, path: String) -> Result<(), String> {
    *state.base_path.lock().map_err(|e| e.to_string())? = path;
    Ok(())
}

#[tauri::command]
fn get_license_info(state: tauri::State<RuntimeState>) -> Result<String, String> {
    Ok(state.license_info.lock().map_err(|e| e.to_string())?.clone())
}

#[tauri::command]
fn set_license_info(state: tauri::State<RuntimeState>, info: String) -> Result<(), String> {
    *state.license_info.lock().map_err(|e| e.to_string())? = info;
    Ok(())
}

#[tauri::command]
fn get_startup_time_ms(state: tauri::State<RuntimeState>) -> Result<i64, String> {
    Ok(*state.startup_time_ms.lock().map_err(|e| e.to_string())?)
}

#[tauri::command]
fn set_startup_time_ms(state: tauri::State<RuntimeState>, time_ms: i64) -> Result<(), String> {
    *state.startup_time_ms.lock().map_err(|e| e.to_string())? = time_ms;
    Ok(())
}

#[tauri::command]
fn get_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(RuntimeState::default())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, shortcut, event| {
                    use tauri::Emitter;
                    use tauri_plugin_global_shortcut::ShortcutState;

                    if event.state == ShortcutState::Pressed {
                        let key = shortcut.to_string();
                        if let Some(action) = crate::hotkeys::action_for_shortcut(&key) {
                            let _ = app.emit("lmplus-hotkey", action);
                        }
                    }
                })
                .build(),
        )
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
            process::launch_lm_updater,
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
