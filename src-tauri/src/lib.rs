pub mod config;
pub mod crypto;
pub mod fingerprint;
pub mod hotkeys;
pub mod license;
pub mod macros;
pub mod map_zoom;
pub mod process;
pub mod telemetry;
pub mod updater;

use std::sync::Mutex;
use tauri::Manager;
use once_cell::sync::Lazy;

pub struct AppState {
    pub base_path: Mutex<String>,
    pub exe_path: Mutex<String>,
    pub process_name: Mutex<String>,
    pub accounts_path: Mutex<String>,
    pub userdata_path: Mutex<String>,
    pub startup_time_ms: Mutex<i64>,
    pub hotkey_map: Mutex<std::collections::HashMap<i32, String>>,
    pub next_hotkey_id: Mutex<i32>,
    pub license_info: Mutex<String>,
}

static APP_STATE: Lazy<AppState> = Lazy::new(|| AppState {
    base_path: Mutex::new(String::new()),
    exe_path: Mutex::new(String::new()),
    process_name: Mutex::new(String::new()),
    accounts_path: Mutex::new(String::new()),
    userdata_path: Mutex::new(String::new()),
    startup_time_ms: Mutex::new(0),
    hotkey_map: Mutex::new(std::collections::HashMap::new()),
    next_hotkey_id: Mutex::new(1),
    license_info: Mutex::new(String::new()),
});

#[tauri::command]
fn get_app_paths() -> Result<serde_json::Value, String> {
    let base = APP_STATE.base_path.lock().map_err(|e| e.to_string())?;
    let exe = APP_STATE.exe_path.lock().map_err(|e| e.to_string())?;
    let proc = APP_STATE.process_name.lock().map_err(|e| e.to_string())?;
    let accounts = APP_STATE.accounts_path.lock().map_err(|e| e.to_string())?;
    let userdata = APP_STATE.userdata_path.lock().map_err(|e| e.to_string())?;

    Ok(serde_json::json!({
        "base_path": *base,
        "exe_path": *exe,
        "process_name": *proc,
        "accounts_path": *accounts,
        "userdata_path": *userdata,
    }))
}

#[tauri::command]
fn set_base_path(path: String) -> Result<(), String> {
    let mut base = APP_STATE.base_path.lock().map_err(|e| e.to_string())?;
    *base = path.clone();
    drop(base);

    let mut exe = APP_STATE.exe_path.lock().map_err(|e| e.to_string())?;
    let mut proc = APP_STATE.process_name.lock().map_err(|e| e.to_string())?;
    let mut accounts = APP_STATE.accounts_path.lock().map_err(|e| e.to_string())?;
    let mut userdata = APP_STATE.userdata_path.lock().map_err(|e| e.to_string())?;

    *exe = format!("{}\\Game\\Lords Mobile PC.exe", path);
    *proc = "Lords Mobile PC.exe".to_string();
    *accounts = format!("{}\\Accounts", path);
    *userdata = format!("{}\\userdata", path);

    Ok(())
}

#[tauri::command]
fn get_license_info() -> Result<String, String> {
    let info = APP_STATE.license_info.lock().map_err(|e| e.to_string())?;
    Ok(info.clone())
}

#[tauri::command]
fn set_license_info(info: String) -> Result<(), String> {
    let mut lic = APP_STATE.license_info.lock().map_err(|e| e.to_string())?;
    *lic = info;
    Ok(())
}

#[tauri::command]
fn get_startup_time_ms() -> Result<i64, String> {
    let t = APP_STATE.startup_time_ms.lock().map_err(|e| e.to_string())?;
    Ok(*t)
}

#[tauri::command]
fn set_startup_time_ms(time_ms: i64) -> Result<(), String> {
    let mut t = APP_STATE.startup_time_ms.lock().map_err(|e| e.to_string())?;
    *t = time_ms;
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
        .manage(APP_STATE)
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
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::Destroyed = event {
                let _ = hotkeys::unregister_hotkeys(window);
            }
        })
        .setup(|app| {
            let window = app.get_webview_window("main").unwrap();
            let _ = window.set_title("LMPlus");
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}