pub mod config;
pub mod crypto;
pub mod hlog;
pub mod license;
#[cfg(target_os = "windows")]
pub mod lmagent;
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

use serde::Serialize;
use std::sync::Mutex;
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{Emitter, Manager, WebviewUrl, WebviewWindowBuilder, Wry};

const LICENSE_HEARTBEAT_TRANSIENT_GRACE_MS: i64 = 3 * 60 * 1000;
const LICENSE_HEARTBEAT_TERMINAL_GRACE_MS: i64 = 60 * 1000;
const LICENSE_HEARTBEAT_TRANSIENT_FAILURE_ALLOWANCE: u32 = 3;
const LICENSE_HEARTBEAT_TERMINAL_FAILURE_ALLOWANCE: u32 = 2;

pub(crate) struct RuntimeState {
    base_path: Mutex<String>,
    license_verified: Mutex<bool>,
    license_info: Mutex<String>,
    license_heartbeat: Mutex<LicenseHeartbeatGate>,
    startup_time_ms: Mutex<i64>,
    account_hotkeys_enabled: Mutex<bool>,
    formation_hotkeys_enabled: Mutex<bool>,
}

#[derive(Default)]
struct LicenseHeartbeatGate {
    consecutive_failures: u32,
    first_failure_ms: Option<i64>,
    blocked: bool,
}

#[derive(Serialize)]
struct LicenseHeartbeatStatus {
    status: String,
    locked: bool,
    terminal: bool,
    failure_count: u32,
    grace_remaining_ms: i64,
    reason: Option<String>,
}

#[derive(Default)]
struct TrayState(Mutex<Option<tauri::tray::TrayIcon<Wry>>>);

impl Default for RuntimeState {
    fn default() -> Self {
        Self {
            base_path: Mutex::new(String::new()),
            license_verified: Mutex::new(false),
            license_info: Mutex::new(String::new()),
            license_heartbeat: Mutex::new(LicenseHeartbeatGate::default()),
            startup_time_ms: Mutex::new(0),
            account_hotkeys_enabled: Mutex::new(true),
            formation_hotkeys_enabled: Mutex::new(true),
        }
    }
}

impl RuntimeState {
    fn mark_license_verified(&self, info: String) -> Result<(), String> {
        *self.license_verified.lock().map_err(|e| e.to_string())? = true;
        *self.license_info.lock().map_err(|e| e.to_string())? = info;
        *self.license_heartbeat.lock().map_err(|e| e.to_string())? =
            LicenseHeartbeatGate::default();
        Ok(())
    }

    fn is_license_verified(&self) -> Result<bool, String> {
        Ok(*self.license_verified.lock().map_err(|e| e.to_string())?)
    }

    fn is_license_blocked(&self) -> Result<bool, String> {
        Ok(self
            .license_heartbeat
            .lock()
            .map_err(|e| e.to_string())?
            .blocked)
    }

    fn require_verified_license(&self) -> Result<(), String> {
        if self.is_license_verified()? {
            Ok(())
        } else {
            Err("License verification required".to_string())
        }
    }

    fn require_license(&self) -> Result<(), String> {
        self.require_verified_license()?;
        if self.is_license_blocked()? {
            Err("License reconnect required".to_string())
        } else {
            Ok(())
        }
    }

    fn record_license_heartbeat_failure(
        &self,
        terminal: bool,
        reason: String,
    ) -> Result<LicenseHeartbeatStatus, String> {
        let now_ms = chrono::Utc::now().timestamp_millis();
        let mut gate = self.license_heartbeat.lock().map_err(|e| e.to_string())?;
        if gate.first_failure_ms.is_none() {
            gate.first_failure_ms = Some(now_ms);
        }
        let first_failure_ms = gate.first_failure_ms.unwrap_or(now_ms);
        gate.consecutive_failures = gate.consecutive_failures.saturating_add(1);

        let grace_ms = if terminal {
            LICENSE_HEARTBEAT_TERMINAL_GRACE_MS
        } else {
            LICENSE_HEARTBEAT_TRANSIENT_GRACE_MS
        };
        let failure_allowance = if terminal {
            LICENSE_HEARTBEAT_TERMINAL_FAILURE_ALLOWANCE
        } else {
            LICENSE_HEARTBEAT_TRANSIENT_FAILURE_ALLOWANCE
        };
        let elapsed_ms = now_ms.saturating_sub(first_failure_ms);
        if gate.consecutive_failures >= failure_allowance || elapsed_ms >= grace_ms {
            gate.blocked = true;
        }

        Ok(LicenseHeartbeatStatus {
            status: if gate.blocked { "locked" } else { "grace" }.to_string(),
            locked: gate.blocked,
            terminal,
            failure_count: gate.consecutive_failures,
            grace_remaining_ms: if gate.blocked {
                0
            } else {
                grace_ms.saturating_sub(elapsed_ms)
            },
            reason: Some(reason),
        })
    }
}

fn is_terminal_license_failure(reason: &str) -> bool {
    let lower = reason.to_lowercase();
    lower.contains("banned")
        || lower.contains("expired")
        || lower.contains("not found")
        || lower.contains("unknown key")
        || lower.contains("invalid")
        || lower.contains("does not match")
        || lower.contains("mismatch")
        || lower.contains("device")
        || lower.contains("fingerprint")
        || lower.contains("signature")
        || lower.contains("parse json")
        || lower.contains("saved license")
        || lower.contains("decryption")
        || lower.contains("utf-8")
        || lower.contains("data too short")
}

/// Small top-right toast in the LMPlus window.
fn toast(app: &tauri::AppHandle, message: &str) {
    use tauri::Emitter;
    let _ = app.emit("lmplus-toast", message);
}

fn pretty_spec(spec: &str) -> &str {
    match spec {
        "formation_inf_phalanx" => "Formation: Infantry Phalanx",
        "formation_range_phalanx" => "Formation: Ranged Phalanx",
        "formation_cav_phalanx" => "Formation: Cavalry Phalanx",
        "formation_inf_wedge" => "Formation: Infantry Wedge",
        "formation_range_wedge" => "Formation: Ranged Wedge",
        "formation_cav_wedge" => "Formation: Cavalry Wedge",
        "map3dview_full" => "Map 3D View: Full",
        "map3dview_balanced" => "Map 3D View: Balanced",
        "map3dview_none" => "Map 3D View: None",
        "switch_account_direct" => "Account Switch",
        s if s.starts_with("zoom:") => "Map Zoom",
        _ => spec,
    }
}

/// True when the foreground window belongs to the game or LMPlus.
fn is_game_or_lmplus_focused() -> bool {
    #[cfg(windows)]
    unsafe {
        use winapi::um::processthreadsapi::OpenProcess;
        use winapi::um::handleapi::CloseHandle;
        use winapi::um::winbase::QueryFullProcessImageNameW;
        use winapi::um::winnt::PROCESS_QUERY_LIMITED_INFORMATION;
        use winapi::um::winuser::GetForegroundWindow;
        use winapi::um::winuser::GetWindowThreadProcessId;

        let hwnd = GetForegroundWindow();
        if hwnd.is_null() {
            return false;
        }
        let mut pid: u32 = 0;
        GetWindowThreadProcessId(hwnd, &mut pid);
        if pid == 0 {
            return false;
        }
        // LMPlus itself?
        let our_pid = winapi::um::processthreadsapi::GetCurrentProcessId();
        if pid == our_pid {
            return true;
        }
        // The game? Compare image path with <basePath>\Game\Lords Mobile PC.exe.
        // Use QUERY_LIMITED_INFORMATION + QueryFullProcessImageNameW: works even
        // when the game runs elevated (GetModuleFileNameExW can fail there).
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if handle.is_null() {
            hlog::warn("focus check: OpenProcess(limited) failed");
            return false;
        }
        let mut path: [u16; 1024] = [0; 1024];
        let mut size: u32 = 1024;
        let ok = QueryFullProcessImageNameW(handle, 0, path.as_mut_ptr(), &mut size);
        CloseHandle(handle);
        if ok == 0 || size == 0 {
            hlog::warn("focus check: QueryFullProcessImageNameW failed");
            return false;
        }
        let exe = String::from_utf16_lossy(&path[..size as usize]);
        let base = current_base_path();
        let game_exe = format!(
            "{}\\Game\\Lords Mobile PC.exe",
            base.trim_end_matches(['\\', '/'])
        );
        let matched = exe.eq_ignore_ascii_case(&game_exe);
        if !matched {
            hlog::info(&format!(
                "focus check: foreground \"{}\" != game \"{}\"",
                exe, game_exe
            ));
        }
        matched
    }
    #[cfg(not(windows))]
    {
        true
    }
}

/// Base path for direct actions triggered from the global-shortcut handler.
fn current_base_path() -> String {
    config::get_settings()
        .ok()
        .and_then(|v| v.get("basePath").and_then(|b| b.as_str()).map(String::from))
        .unwrap_or_default()
}

fn current_exe_path_for_direct(_app: &tauri::AppHandle) -> String {
    let base = current_base_path();
    if base.is_empty() {
        return String::new();
    }
    format!("{}\\Game\\Lords Mobile PC.exe", base.trim_end_matches(['\\', '/']))
}

/// Execute a `direct:<spec>` action from any thread. `spec` may be either a
/// named id (formation_inf_phalanx, map3dview_full, switch_account_direct) or
/// a parameterized one (formation:3, map3dview:2, zoom:0.5).
fn execute_direct_spec(exe_path: &str, spec: &str) -> Result<String, String> {
    let action = lmagent::action::resolve_named_or_spec(spec)
        .ok_or_else(|| format!("unknown direct action {}", spec))?;
    lmagent::execute(exe_path, &action)
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

fn require_login_window_or_license(
    window: &tauri::Window<Wry>,
    state: &RuntimeState,
) -> Result<(), String> {
    if window.label() == "login" || state.is_license_verified()? {
        Ok(())
    } else {
        Err("Login window required".to_string())
    }
}

#[tauri::command(rename = "get_app_paths")]
fn cmd_get_app_paths(state: tauri::State<RuntimeState>) -> Result<serde_json::Value, String> {
    state.require_verified_license()?;
    let base_path = state.base_path.lock().map_err(|e| e.to_string())?;
    Ok(build_paths(&base_path))
}

#[tauri::command(rename = "set_base_path")]
fn cmd_set_base_path(state: tauri::State<RuntimeState>, path: String) -> Result<(), String> {
    state.require_license()?;
    let persisted_path = path.clone();
    *state.base_path.lock().map_err(|e| e.to_string())? = path;
    crate::config::set_base_path_setting(&persisted_path)?;
    Ok(())
}

#[tauri::command(rename = "get_license_info")]
fn cmd_get_license_info(state: tauri::State<RuntimeState>) -> Result<String, String> {
    Ok(state
        .license_info
        .lock()
        .map_err(|e| e.to_string())?
        .clone())
}

#[tauri::command(rename = "set_license_info")]
fn cmd_set_license_info(state: tauri::State<RuntimeState>, info: String) -> Result<(), String> {
    state.require_license()?;
    *state.license_info.lock().map_err(|e| e.to_string())? = info;
    Ok(())
}

#[tauri::command(rename = "get_startup_time_ms")]
fn cmd_get_startup_time_ms(state: tauri::State<RuntimeState>) -> Result<i64, String> {
    state.require_license()?;
    Ok(*state.startup_time_ms.lock().map_err(|e| e.to_string())?)
}

#[tauri::command(rename = "set_startup_time_ms")]
fn cmd_set_startup_time_ms(state: tauri::State<RuntimeState>, time_ms: i64) -> Result<(), String> {
    state.require_license()?;
    *state.startup_time_ms.lock().map_err(|e| e.to_string())? = time_ms;
    Ok(())
}

#[tauri::command(rename = "get_version")]
fn cmd_get_version() -> String {
    option_env!("LMPLUS_RELEASE_VERSION")
        .unwrap_or(env!("CARGO_PKG_VERSION"))
        .to_string()
}

#[tauri::command(rename = "is_license_verified")]
fn cmd_is_license_verified(state: tauri::State<RuntimeState>) -> Result<bool, String> {
    state.is_license_verified()
}

#[tauri::command(rename = "exit_app")]
fn cmd_exit_app(app: tauri::AppHandle) {
    app.exit(0);
}

#[tauri::command(rename = "show_authorized_main_window")]
fn cmd_show_authorized_main_window(
    app: tauri::AppHandle,
    state: tauri::State<RuntimeState>,
) -> Result<(), String> {
    state.require_verified_license()?;
    if let Some(main) = app.get_webview_window("main") {
        let _ = main.show();
        let _ = main.unminimize();
        let _ = main.set_focus();
    }
    Ok(())
}

#[tauri::command(rename = "verify_license")]
async fn cmd_verify_license(
    app: tauri::AppHandle,
    window: tauri::Window<Wry>,
    state: tauri::State<'_, RuntimeState>,
    key: String,
    fingerprint: String,
) -> Result<serde_json::Value, String> {
    require_login_window_or_license(&window, &state)?;
    let result = license::verify_license(key, fingerprint).await?;
    let info = result
        .get("license_info")
        .and_then(|v| v.as_str())
        .unwrap_or("Unknown")
        .to_string();
    state.mark_license_verified(info)?;

    if let Some(main) = app.get_webview_window("main") {
        let _ = main.emit("lmplus-license-ok", ());
    }

    Ok(result)
}

#[tauri::command(rename = "check_license_heartbeat")]
async fn cmd_check_license_heartbeat(
    state: tauri::State<'_, RuntimeState>,
    fingerprint: String,
) -> Result<LicenseHeartbeatStatus, String> {
    state.require_verified_license()?;

    let result: Result<serde_json::Value, String> = async {
        if !license::get_config_signature_status(fingerprint.clone())? {
            return Err("Saved license data could not be verified.".to_string());
        }

        let saved = license::load_saved_license(fingerprint.clone())?;
        let key = saved
            .get("key")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim()
            .to_string();
        if key.is_empty() {
            return Err("Saved license key is missing.".to_string());
        }

        license::verify_license(key, fingerprint).await
    }
    .await;

    match result {
        Ok(value) => {
            let info = value
                .get("license_info")
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown")
                .to_string();
            state.mark_license_verified(info)?;
            Ok(LicenseHeartbeatStatus {
                status: "ok".to_string(),
                locked: false,
                terminal: false,
                failure_count: 0,
                grace_remaining_ms: 0,
                reason: None,
            })
        }
        Err(reason) => {
            let terminal = is_terminal_license_failure(&reason);
            state.record_license_heartbeat_failure(terminal, reason)
        }
    }
}

#[tauri::command(rename = "load_saved_license")]
fn cmd_load_saved_license(
    window: tauri::Window<Wry>,
    state: tauri::State<RuntimeState>,
    fingerprint: String,
) -> Result<serde_json::Value, String> {
    require_login_window_or_license(&window, &state)?;
    license::load_saved_license(fingerprint)
}

#[tauri::command(rename = "get_config_signature_status")]
fn cmd_get_config_signature_status(
    window: tauri::Window<Wry>,
    state: tauri::State<RuntimeState>,
    fingerprint: String,
) -> Result<bool, String> {
    require_login_window_or_license(&window, &state)?;
    license::get_config_signature_status(fingerprint)
}

#[tauri::command(rename = "get_device_fingerprint")]
fn cmd_get_device_fingerprint(
    window: tauri::Window<Wry>,
    state: tauri::State<RuntimeState>,
) -> Result<String, String> {
    require_login_window_or_license(&window, &state)?;
    fingerprint::get_device_fingerprint()
}

#[tauri::command(rename = "suggest_directories")]
fn cmd_suggest_directories(
    state: tauri::State<RuntimeState>,
    input: String,
) -> Result<Vec<String>, String> {
    state.require_license()?;
    config::suggest_directories(input)
}

#[tauri::command(rename = "load_accounts")]
fn cmd_load_accounts(
    state: tauri::State<RuntimeState>,
    base_path: String,
) -> Result<Vec<String>, String> {
    state.require_license()?;
    config::load_accounts(base_path)
}

#[tauri::command(rename = "get_settings")]
fn cmd_get_settings(state: tauri::State<RuntimeState>) -> Result<serde_json::Value, String> {
    state.require_license()?;
    config::get_settings()
}

#[tauri::command(rename = "get_login_settings")]
fn cmd_get_login_settings(
    window: tauri::Window<Wry>,
    state: tauri::State<RuntimeState>,
) -> Result<serde_json::Value, String> {
    require_login_window_or_license(&window, &state)?;
    config::get_login_settings()
}

#[tauri::command(rename = "set_settings")]
fn cmd_set_settings(
    state: tauri::State<RuntimeState>,
    settings: serde_json::Value,
) -> Result<(), String> {
    state.require_license()?;
    config::set_settings(settings)
}

#[tauri::command(rename = "get_hotkey_settings")]
fn cmd_get_hotkey_settings(state: tauri::State<RuntimeState>) -> Result<serde_json::Value, String> {
    state.require_license()?;
    config::get_hotkey_settings()
}

#[tauri::command(rename = "set_hotkey")]
fn cmd_set_hotkey(
    state: tauri::State<RuntimeState>,
    category: String,
    name: String,
    hotkey: String,
) -> Result<(), String> {
    state.require_license()?;
    config::set_hotkey(category, name, hotkey)
}

#[tauri::command(rename = "remove_hotkey")]
fn cmd_remove_hotkey(
    state: tauri::State<RuntimeState>,
    category: String,
    name: String,
) -> Result<(), String> {
    state.require_license()?;
    config::remove_hotkey(category, name)
}

#[tauri::command(rename = "get_account_order")]
fn cmd_get_account_order(state: tauri::State<RuntimeState>) -> Result<Vec<String>, String> {
    state.require_license()?;
    config::get_account_order()
}

#[tauri::command(rename = "set_account_order")]
fn cmd_set_account_order(
    state: tauri::State<RuntimeState>,
    order: Vec<String>,
) -> Result<(), String> {
    state.require_license()?;
    config::set_account_order(order)
}

#[tauri::command(rename = "load_misc_cfg")]
fn cmd_load_misc_cfg(state: tauri::State<RuntimeState>) -> Result<serde_json::Value, String> {
    state.require_license()?;
    config::load_misc_cfg()
}

#[tauri::command(rename = "save_misc_cfg")]
fn cmd_save_misc_cfg(
    state: tauri::State<RuntimeState>,
    macros: Vec<serde_json::Value>,
) -> Result<(), String> {
    state.require_license()?;
    config::save_misc_cfg(macros)
}

#[tauri::command(rename = "switch_account")]
fn cmd_switch_account(
    state: tauri::State<RuntimeState>,
    base_path: String,
    account: String,
) -> Result<(), String> {
    state.require_license()?;
    config::switch_account(base_path, account)
}

#[tauri::command(rename = "add_account")]
fn cmd_add_account(
    state: tauri::State<RuntimeState>,
    base_path: String,
    name: String,
) -> Result<(), String> {
    state.require_license()?;
    config::add_account(base_path, name)
}

#[tauri::command(rename = "rename_account")]
fn cmd_rename_account(
    state: tauri::State<RuntimeState>,
    base_path: String,
    old_name: String,
    new_name: String,
) -> Result<(), String> {
    state.require_license()?;
    config::rename_account(base_path, old_name, new_name)
}

#[tauri::command(rename = "delete_account")]
fn cmd_delete_account(
    state: tauri::State<RuntimeState>,
    base_path: String,
    name: String,
) -> Result<(), String> {
    state.require_license()?;
    config::delete_account(base_path, name)
}

#[tauri::command(rename = "launch_game")]
fn cmd_launch_game(
    state: tauri::State<RuntimeState>,
    exe_path: String,
    process_name: String,
) -> Result<(), String> {
    state.require_license()?;
    process::launch_game(exe_path, process_name)
}

#[tauri::command(rename = "kill_game")]
fn cmd_kill_game(
    state: tauri::State<RuntimeState>,
    exe_path: String,
    process_name: String,
) -> Result<bool, String> {
    state.require_verified_license()?;
    process::kill_game(exe_path, process_name)
}

#[tauri::command(rename = "restart_game")]
fn cmd_restart_game(
    state: tauri::State<RuntimeState>,
    exe_path: String,
    process_name: String,
    relaunch: bool,
) -> Result<(), String> {
    state.require_license()?;
    process::restart_game(exe_path, process_name, relaunch)
}

#[tauri::command(rename = "is_game_running")]
fn cmd_is_game_running(
    state: tauri::State<RuntimeState>,
    exe_path: String,
    process_name: String,
) -> Result<bool, String> {
    state.require_license()?;
    process::is_game_running(exe_path, process_name)
}

#[tauri::command(rename = "is_another_instance_running")]
fn cmd_is_another_instance_running() -> Result<bool, String> {
    process::is_another_instance_running()
}

#[tauri::command(rename = "launch_lm_updater")]
fn cmd_launch_lm_updater(state: tauri::State<RuntimeState>) -> Result<(), String> {
    state.require_license()?;
    process::launch_lm_updater()
}

#[tauri::command(rename = "register_hotkeys")]
fn cmd_register_hotkeys(
    app: tauri::AppHandle,
    state: tauri::State<RuntimeState>,
    accounts_path: String,
    settings_json: String,
) -> Result<serde_json::Value, String> {
    state.require_license()?;
    hotkeys::register_hotkeys(app, accounts_path, settings_json)
}

#[tauri::command(rename = "unregister_hotkeys")]
fn cmd_unregister_hotkeys(app: tauri::AppHandle) -> Result<(), String> {
    hotkeys::unregister_hotkeys(app)
}

#[tauri::command(rename = "execute_macro")]
fn cmd_execute_macro(
    state: tauri::State<RuntimeState>,
    macro_name: String,
    exe_path: String,
    process_name: String,
) -> Result<(), String> {
    state.require_license()?;
    macros::execute_macro(macro_name, exe_path, process_name)
}

#[tauri::command(rename = "parse_misc_cfg")]
fn cmd_parse_misc_cfg(state: tauri::State<RuntimeState>) -> Result<serde_json::Value, String> {
    state.require_license()?;
    macros::parse_misc_cfg()
}

#[tauri::command(rename = "perform_map_zoom")]
fn cmd_perform_map_zoom(
    state: tauri::State<RuntimeState>,
    exe_path: String,
    persistent: bool,
) -> Result<(), String> {
    state.require_license()?;
    map_zoom::perform_map_zoom(exe_path, persistent)
}

#[tauri::command(rename = "execute_direct_action")]
fn cmd_execute_direct_action(
    state: tauri::State<RuntimeState>,
    exe_path: String,
    action: serde_json::Value,
) -> Result<String, String> {
    state.require_license()?;
    #[cfg(target_os = "windows")]
    {
        let parsed: lmagent::action::LMPlusAction = serde_json::from_value(action)
            .map_err(|e| format!("invalid action: {}", e))?;
        lmagent::execute(&exe_path, &parsed)
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = (exe_path, action);
        Err("direct actions are Windows-only".into())
    }
}

#[tauri::command(rename = "stop_persistent_zoom")]
fn cmd_stop_persistent_zoom(state: tauri::State<RuntimeState>) -> Result<(), String> {
    state.require_license()?;
    map_zoom::stop_persistent_zoom()
}

#[tauri::command(rename = "collect_and_send_telemetry")]
async fn cmd_collect_and_send_telemetry(
    state: tauri::State<'_, RuntimeState>,
    base_path: String,
    startup_time: i64,
    hide_count: i64,
    launch_count: i64,
    total_uptime: i64,
    fingerprint: String,
) -> Result<(), String> {
    state.require_license()?;
    telemetry::collect_and_send_telemetry(
        base_path,
        startup_time,
        hide_count,
        launch_count,
        total_uptime,
        fingerprint,
    )
    .await
}

#[tauri::command(rename = "increment_hide_to_tray")]
fn cmd_increment_hide_to_tray(state: tauri::State<RuntimeState>) -> Result<i64, String> {
    state.require_license()?;
    telemetry::increment_hide_to_tray()
}

#[tauri::command(rename = "check_for_update")]
async fn cmd_check_for_update(
    state: tauri::State<'_, RuntimeState>,
) -> Result<serde_json::Value, String> {
    state.require_license()?;
    updater::check_for_update().await
}

#[tauri::command(rename = "perform_update")]
async fn cmd_perform_update(
    state: tauri::State<'_, RuntimeState>,
    app: tauri::AppHandle,
    download_url: String,
) -> Result<(), String> {
    state.require_license()?;
    updater::perform_update(app, download_url).await
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    hlog::init();
    tauri::Builder::default()
        .manage(RuntimeState::default())
        .manage(TrayState::default())
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
                        hlog::info(&format!("global hotkey pressed: {}", key));
                        // Focus gate: hotkeys are only honored when the game or
                        // LMPlus is the foreground window.
                        if !is_game_or_lmplus_focused() {
                            hlog::info("ignored: neither game nor LMPlus focused");
                            return;
                        }
                        let action = crate::hotkeys::action_for_shortcut_id(shortcut.id())
                            .or_else(|| crate::hotkeys::action_for_shortcut(&key));
                        if let Some(action) = action {
                            hlog::info(&format!("hotkey {} (id {:x}) -> action {}", key, shortcut.id(), action));
                            if let Some(spec) = action.strip_prefix("direct:").map(String::from) {
                                // Direct actions run fully in Rust — no webview
                                // needed, so hotkeys work while the game has focus.
                                let exe_path = current_exe_path_for_direct(app);
                                hlog::info(&format!(
                                    "direct action {} with exe_path {:?}",
                                    spec, exe_path
                                ));
                                let app2 = app.clone();
                                std::thread::spawn(move || {
                                    match execute_direct_spec(&exe_path, &spec) {
                                        Ok(detail) => {
                                            hlog::info(&format!("direct {} ok: {}", spec, detail));
                                            toast(&app2, &format!("{}: {}", pretty_spec(&spec), detail));
                                        }
                                        Err(e) => {
                                            hlog::error(&format!("direct {} failed: {}", spec, e));
                                            toast(&app2, &format!("{} failed: {}", pretty_spec(&spec), e));
                                        }
                                    }
                                });
                            } else {
                                let _ = app.emit("lmplus-hotkey", action.clone());
                                toast(app, &format!("Switching: {}", action));
                            }
                        } else {
                            hlog::warn(&format!("hotkey {} has no mapped action", key));
                            toast(app, &format!("No action mapped to {}", key));
                        }
                    }
                })
                .build(),
        )
        .invoke_handler(tauri::generate_handler![
            cmd_get_app_paths,
            cmd_set_base_path,
            cmd_get_license_info,
            cmd_set_license_info,
            cmd_get_startup_time_ms,
            cmd_set_startup_time_ms,
            cmd_get_version,
            cmd_is_license_verified,
            cmd_exit_app,
            cmd_show_authorized_main_window,
            cmd_check_license_heartbeat,
            cmd_suggest_directories,
            cmd_load_accounts,
            cmd_get_settings,
            cmd_get_login_settings,
            cmd_set_settings,
            cmd_get_hotkey_settings,
            cmd_set_hotkey,
            cmd_remove_hotkey,
            cmd_get_account_order,
            cmd_set_account_order,
            cmd_load_misc_cfg,
            cmd_save_misc_cfg,
            cmd_switch_account,
            cmd_add_account,
            cmd_rename_account,
            cmd_delete_account,
            cmd_verify_license,
            cmd_load_saved_license,
            cmd_get_config_signature_status,
            cmd_launch_game,
            cmd_kill_game,
            cmd_restart_game,
            cmd_is_game_running,
            cmd_is_another_instance_running,
            cmd_launch_lm_updater,
            cmd_register_hotkeys,
            cmd_unregister_hotkeys,
            cmd_execute_macro,
            cmd_parse_misc_cfg,
            cmd_perform_map_zoom,
            cmd_stop_persistent_zoom,
            cmd_execute_direct_action,
            cmd_collect_and_send_telemetry,
            cmd_increment_hide_to_tray,
            cmd_check_for_update,
            cmd_perform_update,
            cmd_get_device_fingerprint,
        ])
        .setup(|app| {
            let window = app.get_webview_window("main").unwrap();
            let _ = window.set_title("LMPlus");
            let _ = window.hide();
            let mut tray = TrayIconBuilder::with_id("lmplus")
                .tooltip("LMPlus")
                .show_menu_on_left_click(false)
                .on_tray_icon_event(|tray, event| {
                    let should_restore = matches!(
                        event,
                        TrayIconEvent::Click {
                            button: MouseButton::Left,
                            button_state: MouseButtonState::Up,
                            ..
                        } | TrayIconEvent::DoubleClick {
                            button: MouseButton::Left,
                            ..
                        }
                    );
                    if should_restore {
                        let app = tray.app_handle();
                        let state = app.state::<RuntimeState>();
                        let verified = state.is_license_verified().unwrap_or(false);
                        if verified {
                            if let Some(window) = app.get_webview_window("main") {
                                let _ = window.show();
                                let _ = window.unminimize();
                                let _ = window.set_focus();
                            }
                        } else if let Some(window) = app.get_webview_window("login") {
                            let _ = window.show();
                            let _ = window.unminimize();
                            let _ = window.set_focus();
                        }
                    }
                });
            if let Some(icon) = app.default_window_icon().cloned() {
                tray = tray.icon(icon);
            }
            let tray_icon = tray.build(app)?;
            let tray_state = app.state::<TrayState>();
            if let Ok(mut tray_slot) = tray_state.0.lock() {
                *tray_slot = Some(tray_icon);
            }

            let login_url = WebviewUrl::App("index.html".into());
            let _login = WebviewWindowBuilder::new(app, "login", login_url)
                .title("LMPlus Login")
                .inner_size(460.0, 220.0)
                .resizable(false)
                .maximizable(false)
                .minimizable(true)
                .closable(true)
                .center()
                .decorations(true)
                .devtools(false)
                .visible(true)
                .build()?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
