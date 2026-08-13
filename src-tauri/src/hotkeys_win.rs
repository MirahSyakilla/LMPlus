use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::Mutex;
use tauri_plugin_global_shortcut::GlobalShortcutExt;

static HOTKEY_ACTIONS: Lazy<Mutex<HashMap<String, String>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

fn normalize_shortcut(shortcut: &str) -> String {
    shortcut
        .split('+')
        .map(|p| p.trim().to_ascii_lowercase())
        .collect::<Vec<_>>()
        .join("+")
}

pub fn action_for_shortcut(shortcut: &str) -> Option<String> {
    let actions = HOTKEY_ACTIONS.lock().ok()?;
    actions
        .get(&normalize_shortcut(shortcut))
        .cloned()
        .or_else(|| actions.get(shortcut).cloned())
}

fn collect_shortcuts(settings: &serde_json::Value) -> Vec<(String, String, String)> {
    let mut items = Vec::new();

    if let Some(hotkeys) = settings.get("hotkeys").and_then(|v| v.as_object()) {
        for (name, value) in hotkeys {
            if let Some(hotkey) = value.as_str().filter(|v| !v.trim().is_empty()) {
                items.push((hotkey.to_string(), name.clone(), "account".to_string()));
            }
        }
    }

    if let Some(swap_hotkeys) = settings.get("swap_hotkeys").and_then(|v| v.as_object()) {
        for (name, value) in swap_hotkeys {
            if let Some(hotkey) = value.as_str().filter(|v| !v.trim().is_empty()) {
                items.push((hotkey.to_string(), format!("swap_{}", name), "swap".to_string()));
            }
        }
    }

    if let Some(misc_hotkeys) = settings.get("misc_hotkeys").and_then(|v| v.as_array()) {
        for item in misc_hotkeys {
            let name = item.get("name").and_then(|v| v.as_str()).unwrap_or_default();
            let hotkey = item.get("hotkey").and_then(|v| v.as_str()).unwrap_or_default();
            if !name.is_empty() && !hotkey.trim().is_empty() {
                items.push((hotkey.to_string(), format!("misc_{}", name), "misc".to_string()));
            }
        }
    }

    items
}

#[tauri::command]
pub fn register_hotkeys(
    app: tauri::AppHandle,
    _accounts_path: String,
    settings_json: String,
) -> Result<serde_json::Value, String> {
    unregister_hotkeys(app.clone())?;

    let settings: serde_json::Value =
        serde_json::from_str(&settings_json).map_err(|e| format!("JSON parse: {}", e))?;

    let shortcuts = collect_shortcuts(&settings);
    let manager = app.global_shortcut();
    let mut actions = HOTKEY_ACTIONS.lock().map_err(|e| e.to_string())?;
    let mut registered = Vec::new();

    for (hotkey, action, kind) in shortcuts {
        manager
            .register(hotkey.as_str())
            .map_err(|e| format!("Failed to register {}: {}", hotkey, e))?;
        actions.insert(normalize_shortcut(&hotkey), action.clone());
        actions.insert(hotkey.clone(), action.clone());
        registered.push(serde_json::json!({
            "name": action.trim_start_matches("swap_").trim_start_matches("misc_"),
            "type": kind,
            "hotkey": hotkey,
        }));
    }

    Ok(serde_json::json!(registered))
}

#[tauri::command]
pub fn unregister_hotkeys(app: tauri::AppHandle) -> Result<(), String> {
    let keys: Vec<String> = HOTKEY_ACTIONS
        .lock()
        .map_err(|e| e.to_string())?
        .keys()
        .cloned()
        .collect();

    let manager = app.global_shortcut();
    for key in keys {
        let _ = manager.unregister(key.as_str());
    }
    HOTKEY_ACTIONS.lock().map_err(|e| e.to_string())?.clear();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_shortcut_is_case_insensitive() {
        assert_eq!(normalize_shortcut("Ctrl + Shift + A"), "ctrl+shift+a");
    }

    #[test]
    fn collect_shortcuts_builds_actions() {
        let settings = serde_json::json!({
            "hotkeys": { "Acc1": "Ctrl+1" },
            "swap_hotkeys": { "Inf Phal": "Ctrl+2" },
            "misc_hotkeys": [{ "name": "Speed", "hotkey": "Ctrl+3" }]
        });
        let items = collect_shortcuts(&settings);
        assert_eq!(items.len(), 3);
        assert!(items.iter().any(|(_, action, _)| action == "Acc1"));
        assert!(items.iter().any(|(_, action, _)| action == "swap_Inf Phal"));
        assert!(items.iter().any(|(_, action, _)| action == "misc_Speed"));
    }
}
