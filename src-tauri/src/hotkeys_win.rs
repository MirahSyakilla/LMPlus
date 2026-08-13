use std::collections::HashMap;
use std::sync::Mutex;
use once_cell::sync::Lazy;
use tauri::Manager;
use winapi::shared::minwindef::{BOOL, DWORD, FALSE, HINSTANCE, INT, LPARAM, LRESULT, UINT, WPARAM};
use winapi::um::winuser::{
    RegisterHotKey, UnregisterHotKey, MOD_ALT, MOD_CONTROL, MOD_SHIFT, MOD_WIN,
    VK_F1, VK_F2, VK_F3, VK_F4, VK_F5, VK_F6, VK_F7, VK_F8, VK_F9, VK_F10, VK_F11, VK_F12,
    VK_NUMPAD0, VK_NUMPAD1, VK_NUMPAD2, VK_NUMPAD3, VK_NUMPAD4, VK_NUMPAD5, VK_NUMPAD6,
    VK_NUMPAD7, VK_NUMPAD8, VK_NUMPAD9, WM_HOTKEY,
};

static HOTKEY_MAP: Lazy<Mutex<HashMap<i32, String>>> = Lazy::new(|| Mutex::new(HashMap::new()));
static NEXT_HOTKEY_ID: Lazy<Mutex<i32>> = Lazy::new(|| Mutex::new(1));

fn parse_key_name(name: &str) -> Option<UINT> {
    match name.to_uppercase().as_str() {
        "A" => Some(0x41), "B" => Some(0x42), "C" => Some(0x43), "D" => Some(0x44),
        "E" => Some(0x45), "F" => Some(0x46), "G" => Some(0x47), "H" => Some(0x48),
        "I" => Some(0x49), "J" => Some(0x4A), "K" => Some(0x4B), "L" => Some(0x4C),
        "M" => Some(0x4D), "N" => Some(0x4E), "O" => Some(0x4F), "P" => Some(0x50),
        "Q" => Some(0x51), "R" => Some(0x52), "S" => Some(0x53), "T" => Some(0x54),
        "U" => Some(0x55), "V" => Some(0x56), "W" => Some(0x57), "X" => Some(0x58),
        "Y" => Some(0x59), "Z" => Some(0x5A),
        "0" => Some(0x30), "1" => Some(0x31), "2" => Some(0x32), "3" => Some(0x33),
        "4" => Some(0x34), "5" => Some(0x35), "6" => Some(0x36), "7" => Some(0x37),
        "8" => Some(0x38), "9" => Some(0x39),
        "F1" => Some(VK_F1), "F2" => Some(VK_F2), "F3" => Some(VK_F3),
        "F4" => Some(VK_F4), "F5" => Some(VK_F5), "F6" => Some(VK_F6),
        "F7" => Some(VK_F7), "F8" => Some(VK_F8), "F9" => Some(VK_F9),
        "F10" => Some(VK_F10), "F11" => Some(VK_F11), "F12" => Some(VK_F12),
        "SPACE" => Some(0x20), "TAB" => Some(0x09), "BACKSPACE" => Some(0x08),
        "ENTER" => Some(0x0D), "ESCAPE" | "ESC" => Some(0x1B),
        "DELETE" | "DEL" => Some(0x2E), "INSERT" | "INS" => Some(0x2D),
        "HOME" => Some(0x24), "END" => Some(0x23),
        "PAGEUP" | "PGUP" => Some(0x21), "PAGEDOWN" | "PGDN" => Some(0x22),
        "UP" => Some(0x26), "DOWN" => Some(0x28), "LEFT" => Some(0x25), "RIGHT" => Some(0x27),
        "NUMPAD0" | "NUM0" => Some(VK_NUMPAD0), "NUMPAD1" | "NUM1" => Some(VK_NUMPAD1),
        "NUMPAD2" | "NUM2" => Some(VK_NUMPAD2), "NUMPAD3" | "NUM3" => Some(VK_NUMPAD3),
        "NUMPAD4" | "NUM4" => Some(VK_NUMPAD4), "NUMPAD5" | "NUM5" => Some(VK_NUMPAD5),
        "NUMPAD6" | "NUM6" => Some(VK_NUMPAD6), "NUMPAD7" | "NUM7" => Some(VK_NUMPAD7),
        "NUMPAD8" | "NUM8" => Some(VK_NUMPAD8), "NUMPAD9" | "NUM9" => Some(VK_NUMPAD9),
        _ => None,
    }
}

fn parse_hotkey_string(hotkey: &str) -> Option<(UINT, UINT)> {
    let parts: Vec<&str> = hotkey.split('+').map(|s| s.trim()).collect();
    if parts.is_empty() {
        return None;
    }

    let mut modifiers: UINT = 0;
    let mut key: Option<UINT> = None;

    for part in &parts {
        match part.to_uppercase().as_str() {
            "CTRL" | "CONTROL" => modifiers |= MOD_CONTROL,
            "ALT" => modifiers |= MOD_ALT,
            "SHIFT" => modifiers |= MOD_SHIFT,
            "WIN" | "META" | "WINDOWS" => modifiers |= MOD_WIN,
            other => {
                if key.is_some() {
                    return None;
                }
                key = parse_key_name(other);
            }
        }
    }

    key.map(|k| (modifiers, k))
}

#[tauri::command]
pub fn register_hotkeys(
    window: tauri::Window,
    accounts_path: String,
    settings_json: String,
) -> Result<serde_json::Value, String> {
    unregister_hotkeys_internal(&window)?;

    let hwnd = window.hwnd().map_err(|e| format!("Failed to get hwnd: {}", e))?;

    let settings: serde_json::Value =
        serde_json::from_str(&settings_json).map_err(|e| format!("JSON parse: {}", e))?;

    let mut map = HOTKEY_MAP.lock().map_err(|e| e.to_string())?;
    let mut id = {
        let mut next = NEXT_HOTKEY_ID.lock().map_err(|e| e.to_string())?;
        *next = 1;
        *next
    };

    let mut registered = Vec::new();

    let hotkeys = settings.get("hotkeys").and_then(|v| v.as_object());
    if let Some(hk) = hotkeys {
        for (name, value) in hk {
            if let Some(hotkey_str) = value.as_str() {
                if !hotkey_str.is_empty() {
                    if let Some((mods, vk)) = parse_hotkey_string(hotkey_str) {
                        let result = unsafe { RegisterHotKey(hwnd, id, mods, vk) };
                        if result != 0 {
                            map.insert(id, name.clone());
                            registered.push(serde_json::json!({
                                "id": id,
                                "name": name,
                                "type": "account",
                                "hotkey": hotkey_str,
                            }));
                            id += 1;
                        }
                    }
                }
            }
        }
    }

    let swap_hotkeys = settings.get("swap_hotkeys").and_then(|v| v.as_object());
    if let Some(sh) = swap_hotkeys {
        for (name, value) in sh {
            if let Some(hotkey_str) = value.as_str() {
                if !hotkey_str.is_empty() {
                    if let Some((mods, vk)) = parse_hotkey_string(hotkey_str) {
                        let result = unsafe { RegisterHotKey(hwnd, id, mods, vk) };
                        if result != 0 {
                            map.insert(id, format!("swap_{}", name));
                            registered.push(serde_json::json!({
                                "id": id,
                                "name": name,
                                "type": "swap",
                                "hotkey": hotkey_str,
                            }));
                            id += 1;
                        }
                    }
                }
            }
        }
    }

    let misc_hotkeys = settings.get("misc_hotkeys").and_then(|v| v.as_array());
    if let Some(mh) = misc_hotkeys {
        for item in mh {
            let name = item.get("name").and_then(|v| v.as_str()).unwrap_or("");
            let hotkey_str = item.get("hotkey").and_then(|v| v.as_str()).unwrap_or("");
            if !name.is_empty() && !hotkey_str.is_empty() {
                if let Some((mods, vk)) = parse_hotkey_string(hotkey_str) {
                    let result = unsafe { RegisterHotKey(hwnd, id, mods, vk) };
                    if result != 0 {
                        map.insert(id, format!("misc_{}", name));
                        registered.push(serde_json::json!({
                            "id": id,
                            "name": name,
                            "type": "misc",
                            "hotkey": hotkey_str,
                        }));
                        id += 1;
                    }
                }
            }
        }
    }

    Ok(serde_json::json!(registered))
}

#[tauri::command]
pub fn unregister_hotkeys(window: tauri::Window) -> Result<(), String> {
    unregister_hotkeys_internal(&window)
}

fn unregister_hotkeys_internal(window: &tauri::Window) -> Result<(), String> {
    let hwnd = window.hwnd().map_err(|e| format!("Failed to get hwnd: {}", e))?;
    let mut map = HOTKEY_MAP.lock().map_err(|e| e.to_string())?;
    let ids: Vec<i32> = map.keys().copied().collect();
    for id in ids {
        unsafe { UnregisterHotKey(hwnd, id) };
        map.remove(&id);
    }
    Ok(())
}

pub fn get_hotkey_map() -> std::collections::HashMap<i32, String> {
    HOTKEY_MAP.lock().map(|m| m.clone()).unwrap_or_default()
}