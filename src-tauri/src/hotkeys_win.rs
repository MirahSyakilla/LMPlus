use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::Mutex;

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
                items.push((
                    hotkey.to_string(),
                    format!("swap_{}", name),
                    "swap".to_string(),
                ));
            }
        }
    }

    if let Some(misc_hotkeys) = settings.get("misc_hotkeys").and_then(|v| v.as_array()) {
        for item in misc_hotkeys {
            let name = item
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or_default();
            let hotkey = item
                .get("hotkey")
                .and_then(|v| v.as_str())
                .unwrap_or_default();
            if !name.is_empty() && !hotkey.trim().is_empty() {
                items.push((
                    hotkey.to_string(),
                    format!("misc_{}", name),
                    "misc".to_string(),
                ));
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
    use tauri_plugin_global_shortcut::GlobalShortcutExt;

    let settings: serde_json::Value =
        serde_json::from_str(&settings_json).map_err(|e| format!("JSON parse: {}", e))?;

    let shortcuts = collect_shortcuts(&settings);
    let mut actions = HOTKEY_ACTIONS.lock().map_err(|e| e.to_string())?;
    actions.clear();

    // Re-register every shortcut as a GLOBAL OS hotkey so they fire while the
    // game (or anything else) has focus, not just when LMPlus is focused.
    let gs = app.global_shortcut();
    let _ = gs.unregister_all();
    let mut registered = Vec::new();

    for (hotkey, action, kind) in shortcuts {
        match gs.register(hotkey.as_str()) {
            Ok(()) => {
                actions.insert(normalize_shortcut(&hotkey), action.clone());
                actions.insert(hotkey.clone(), action.clone());
                registered.push(serde_json::json!({
                    "name": action.trim_start_matches("swap_").trim_start_matches("misc_"),
                    "type": kind,
                    "hotkey": hotkey,
                }));
            }
            Err(e) => {
                // Keep the action mapped so the in-window fallback still works,
                // but surface the failure.
                actions.insert(normalize_shortcut(&hotkey), action.clone());
                actions.insert(hotkey.clone(), action.clone());
                registered.push(serde_json::json!({
                    "name": action.trim_start_matches("swap_").trim_start_matches("misc_"),
                    "type": kind,
                    "hotkey": hotkey,
                    "global": false,
                    "error": e.to_string(),
                }));
            }
        }
    }

    Ok(serde_json::json!(registered))
}

#[tauri::command]
pub fn unregister_hotkeys(app: tauri::AppHandle) -> Result<(), String> {
    use tauri_plugin_global_shortcut::GlobalShortcutExt;
    let _ = app.global_shortcut().unregister_all();
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

    #[test]
    fn collect_shortcuts_includes_direct_actions() {
        let settings = serde_json::json!({
            "hotkeys": {
                "formation_inf_phalanx": "Ctrl+1",
                "formation_cav_wedge": "Ctrl+2",
                "map3dview_full": "Ctrl+3",
                "map3dview_balanced": "Ctrl+4",
                "map3dview_none": "Ctrl+5",
                "switch_account_direct": "Ctrl+6",
                "zoom:12.5": "Ctrl+7",
                "Acc1": "Ctrl+8",
            },
            "swap_hotkeys": {},
            "misc_hotkeys": []
        });
        let items = collect_shortcuts(&settings);
        assert_eq!(items.len(), 8);
        // Direct entries ride in the plain "hotkeys" group; the frontend's
        // reregisterHotkeys adds the "direct:" prefix when building the
        // action map, so the backend just needs to pass the raw ids through.
        assert!(items.iter().any(|(h, action, kind)| action == "formation_inf_phalanx" && kind == "account" && h == "Ctrl+1"));
        assert!(items.iter().any(|(_, action, _)| action == "formation_cav_wedge"));
        assert!(items.iter().any(|(_, action, _)| action == "map3dview_full"));
        assert!(items.iter().any(|(_, action, _)| action == "map3dview_balanced"));
        assert!(items.iter().any(|(_, action, _)| action == "map3dview_none"));
        assert!(items.iter().any(|(_, action, _)| action == "switch_account_direct"));
        assert!(items.iter().any(|(_, action, _)| action == "zoom:12.5"));
    }
}
