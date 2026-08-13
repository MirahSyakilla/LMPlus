use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

fn settings_path() -> Result<PathBuf, String> {
    let exe = std::env::current_exe().map_err(|e| format!("Failed to get exe path: {}", e))?;
    let dir = exe.parent().ok_or("Failed to get exe directory")?;
    Ok(dir.join("settings.ini"))
}

fn read_ini() -> Result<BTreeMap<String, BTreeMap<String, String>>, String> {
    let path = settings_path()?;
    let content = fs::read_to_string(&path).unwrap_or_default();
    let mut map: BTreeMap<String, BTreeMap<String, String>> = BTreeMap::new();
    let mut current_section = String::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with(';') || trimmed.starts_with('#') {
            continue;
        }
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            current_section = trimmed[1..trimmed.len() - 1].to_string();
            map.entry(current_section.clone()).or_default();
            continue;
        }
        if let Some(eq) = trimmed.find('=') {
            let key = trimmed[..eq].trim().to_string();
            let val = trimmed[eq + 1..].trim().to_string();
            map.entry(current_section.clone()).or_default().insert(key, val);
        }
    }
    Ok(map)
}

fn write_ini(map: &BTreeMap<String, BTreeMap<String, String>>) -> Result<(), String> {
    let path = settings_path()?;
    let mut content = String::new();
    for (section, keys) in map {
        content.push_str(&format!("[{}]\n", section));
        for (k, v) in keys {
            content.push_str(&format!("{}={}\n", k, v));
        }
        content.push('\n');
    }
    fs::write(&path, content).map_err(|e| format!("Failed to write settings: {}", e))
}

fn exe_dir() -> Result<PathBuf, String> {
    let exe = std::env::current_exe().map_err(|e| format!("Failed to get exe path: {}", e))?;
    exe.parent()
        .map(|p| p.to_path_buf())
        .ok_or("Failed to get exe directory".to_string())
}

#[tauri::command]
pub fn load_accounts(base_path: String) -> Result<Vec<String>, String> {
    let accounts_dir = PathBuf::from(&base_path).join("Accounts");
    let mut accounts: Vec<String> = Vec::new();
    if accounts_dir.exists() {
        if let Ok(entries) = fs::read_dir(&accounts_dir) {
            for entry in entries.flatten() {
                if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                    accounts.push(entry.file_name().to_string_lossy().to_string());
                }
            }
        }
    }
    accounts.sort();

    let order = get_account_order_impl()?;
    let mut ordered: Vec<String> = Vec::new();
    for name in &order {
        if accounts.contains(name) {
            ordered.push(name.clone());
        }
    }
    for name in &accounts {
        if !ordered.contains(name) {
            ordered.push(name.clone());
        }
    }
    Ok(ordered)
}

#[tauri::command]
pub fn get_settings() -> Result<serde_json::Value, String> {
    let ini = read_ini()?;
    let general = ini.get("General");
    let base_path = general
        .and_then(|g| g.get("basePath"))
        .cloned()
        .unwrap_or_default();

    let account_order = get_account_order_impl()?;

    Ok(serde_json::json!({
        "basePath": base_path,
        "accountOrder": account_order,
    }))
}

#[tauri::command]
pub fn set_settings(settings: serde_json::Value) -> Result<(), String> {
    let mut ini = read_ini()?;
    if let Some(base_path) = settings.get("basePath").and_then(|v| v.as_str()) {
        ini.entry("General".to_string())
            .or_default()
            .insert("basePath".to_string(), base_path.to_string());
    }
    write_ini(&ini)
}

#[tauri::command]
pub fn get_hotkey_settings() -> Result<serde_json::Value, String> {
    let ini = read_ini()?;
    let hotkeys = ini.get("Hotkeys").cloned().unwrap_or_default();
    let map: serde_json::Map<String, serde_json::Value> = hotkeys
        .into_iter()
        .map(|(k, v)| (k, serde_json::Value::String(v)))
        .collect();
    Ok(serde_json::Value::Object(map))
}

#[tauri::command]
pub fn set_hotkey(category: String, name: String, hotkey: String) -> Result<(), String> {
    let mut ini = read_ini()?;
    let key = format!("{}.{}", category, name);
    ini.entry("Hotkeys".to_string())
        .or_default()
        .insert(key, hotkey);
    write_ini(&ini)
}

#[tauri::command]
pub fn remove_hotkey(category: String, name: String) -> Result<(), String> {
    let mut ini = read_ini()?;
    let key = format!("{}.{}", category, name);
    if let Some(section) = ini.get_mut("Hotkeys") {
        section.remove(&key);
    }
    write_ini(&ini)
}

fn get_account_order_impl() -> Result<Vec<String>, String> {
    let ini = read_ini()?;
    let order_str = ini
        .get("General")
        .and_then(|g| g.get("accountOrder"))
        .cloned()
        .unwrap_or_default();
    if order_str.is_empty() {
        return Ok(Vec::new());
    }
    Ok(order_str.split(',').map(|s| s.trim().to_string()).collect())
}

#[tauri::command]
pub fn get_account_order() -> Result<Vec<String>, String> {
    get_account_order_impl()
}

#[tauri::command]
pub fn set_account_order(order: Vec<String>) -> Result<(), String> {
    let mut ini = read_ini()?;
    ini.entry("General".to_string())
        .or_default()
        .insert("accountOrder".to_string(), order.join(","));
    write_ini(&ini)
}

#[tauri::command]
pub fn load_misc_cfg() -> Result<serde_json::Value, String> {
    let path = exe_dir()?.join("misc.cfg");
    let content = fs::read_to_string(&path).unwrap_or_default();
    let mut macros: Vec<serde_json::Value> = Vec::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let parts: Vec<&str> = trimmed.splitn(3, ';').collect();
        if parts.len() < 3 {
            continue;
        }
        let name = parts[0].to_string();
        let hotkey = parts[1].to_string();
        let points: Vec<serde_json::Value> = parts[2]
            .split(';')
            .filter(|s| !s.is_empty())
            .map(|coord| {
                let xy: Vec<&str> = coord.split(',').collect();
                serde_json::json!({
                    "x": xy.first().and_then(|v| v.parse::<i32>().ok()).unwrap_or(0),
                    "y": xy.get(1).and_then(|v| v.parse::<i32>().ok()).unwrap_or(0),
                })
            })
            .collect();
        macros.push(serde_json::json!({
            "name": name,
            "hotkey": hotkey,
            "points": points,
        }));
    }
    Ok(serde_json::json!(macros))
}

#[tauri::command]
pub fn save_misc_cfg(macros: Vec<serde_json::Value>) -> Result<(), String> {
    let path = exe_dir()?.join("misc.cfg");
    let mut content = String::new();
    for m in &macros {
        let name = m.get("name").and_then(|v| v.as_str()).unwrap_or("");
        let hotkey = m.get("hotkey").and_then(|v| v.as_str()).unwrap_or("");
        let points_str = m
            .get("points")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .map(|p| {
                        let x = p.get("x").and_then(|v| v.as_i64()).unwrap_or(0);
                        let y = p.get("y").and_then(|v| v.as_i64()).unwrap_or(0);
                        format!("{},{}", x, y)
                    })
                    .collect::<Vec<_>>()
                    .join(";")
            })
            .unwrap_or_default();
        content.push_str(&format!("{};{};{}\n", name, hotkey, points_str));
    }
    fs::write(&path, content).map_err(|e| format!("Failed to write misc.cfg: {}", e))
}

fn copy_dir_recursive(src: &std::path::Path, dst: &std::path::Path) -> Result<(), String> {
    if !src.exists() {
        return Err(format!("Source does not exist: {}", src.display()));
    }
    fs::create_dir_all(dst).map_err(|e| format!("Failed to create dir {}: {}", dst.display(), e))?;
    for entry in fs::read_dir(src).map_err(|e| format!("Failed to read dir {}: {}", src.display(), e))? {
        let entry = entry.map_err(|e| e.to_string())?;
        let ty = entry.file_type().map_err(|e| e.to_string())?;
        let dest_path = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_dir_recursive(&entry.path(), &dest_path)?;
        } else if ty.is_file() {
            fs::copy(entry.path(), &dest_path)
                .map_err(|e| format!("Failed to copy {}: {}", entry.path().display(), e))?;
        }
    }
    Ok(())
}

#[tauri::command]
pub fn switch_account(base_path: String, account: String) -> Result<(), String> {
    let base = PathBuf::from(&base_path);
    let account_folder = base.join("Accounts").join(&account);
    if !account_folder.exists() {
        return Err(format!("Account folder not found: {}", account_folder.display()));
    }
    let userdata = base.join("userdata");
    if userdata.exists() {
        fs::remove_dir_all(&userdata)
            .map_err(|e| format!("Failed to remove userdata: {}", e))?;
    }
    copy_dir_recursive(&account_folder, &userdata)
        .map_err(|e| format!("Failed to swap account: {}", e))?;
    Ok(())
}

#[tauri::command]
pub fn add_account(base_path: String, name: String) -> Result<(), String> {
    if name.trim().is_empty() {
        return Err("Account name cannot be empty".to_string());
    }
    let dst = PathBuf::from(&base_path).join("Accounts").join(&name);
    if dst.exists() {
        return Err(format!("Account already exists: {}", name));
    }
    let userdata = PathBuf::from(&base_path).join("userdata");
    copy_dir_recursive(&userdata, &dst).map_err(|e| format!("Failed to save account: {}", e))?;
    Ok(())
}

#[tauri::command]
pub fn rename_account(base_path: String, old_name: String, new_name: String) -> Result<(), String> {
    if new_name.trim().is_empty() {
        return Err("Account name cannot be empty".to_string());
    }
    let accounts = PathBuf::from(&base_path).join("Accounts");
    let src = accounts.join(&old_name);
    let dst = accounts.join(&new_name);
    if !src.exists() {
        return Err(format!("Account not found: {}", old_name));
    }
    if dst.exists() {
        return Err("An account with that name already exists".to_string());
    }
    fs::rename(&src, &dst).map_err(|e| format!("Failed to rename account: {}", e))?;

    let mut ini = read_ini()?;
    let old_hotkey_key = format!("hotkeys.{}", old_name);
    let new_hotkey_key = format!("hotkeys.{}", new_name);
    if let Some(hotkeys) = ini.get_mut("Hotkeys") {
        if let Some(value) = hotkeys.remove(&old_hotkey_key) {
            hotkeys.insert(new_hotkey_key, value);
        }
    }
    write_ini(&ini)?;

    let order = get_account_order_impl()?;
    let updated: Vec<String> = order
        .into_iter()
        .map(|a| if a == old_name { new_name.clone() } else { a })
        .collect();
    set_account_order(updated)
}

#[tauri::command]
pub fn delete_account(base_path: String, name: String) -> Result<(), String> {
    let folder = PathBuf::from(&base_path).join("Accounts").join(&name);
    if !folder.exists() {
        return Err(format!("Account not found: {}", name));
    }
    fs::remove_dir_all(&folder).map_err(|e| format!("Failed to delete account: {}", e))?;

    let mut ini = read_ini()?;
    if let Some(hotkeys) = ini.get_mut("Hotkeys") {
        hotkeys.remove(&format!("hotkeys.{}", name));
    }
    write_ini(&ini)?;

    let order: Vec<String> = get_account_order_impl()?
        .into_iter()
        .filter(|a| a != &name)
        .collect();
    set_account_order(order)
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_base(label: &str) -> PathBuf {
        let nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let path = std::env::temp_dir().join(format!("lmplus_rs_{}_{}", label, nanos));
        fs::create_dir_all(&path).unwrap();
        path
    }

    #[test]
    fn copy_dir_recursive_copies_nested_files() {
        let root = temp_base("copy");
        let src = root.join("src");
        let dst = root.join("dst");
        fs::create_dir_all(src.join("nested")).unwrap();
        fs::write(src.join("a.txt"), "a").unwrap();
        fs::write(src.join("nested").join("b.txt"), "b").unwrap();

        copy_dir_recursive(&src, &dst).unwrap();

        assert_eq!(fs::read_to_string(dst.join("a.txt")).unwrap(), "a");
        assert_eq!(fs::read_to_string(dst.join("nested").join("b.txt")).unwrap(), "b");
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn switch_account_replaces_userdata() {
        let root = temp_base("switch");
        fs::create_dir_all(root.join("Accounts").join("main")).unwrap();
        fs::create_dir_all(root.join("userdata")).unwrap();
        fs::write(root.join("Accounts").join("main").join("state.dat"), "new").unwrap();
        fs::write(root.join("userdata").join("old.dat"), "old").unwrap();

        switch_account(root.to_string_lossy().to_string(), "main".into()).unwrap();

        assert!(!root.join("userdata").join("old.dat").exists());
        assert_eq!(fs::read_to_string(root.join("userdata").join("state.dat")).unwrap(), "new");
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn add_rename_delete_account_flow() {
        let root = temp_base("accounts");
        fs::create_dir_all(root.join("userdata")).unwrap();
        fs::write(root.join("userdata").join("save.bin"), "save").unwrap();

        add_account(root.to_string_lossy().to_string(), "alpha".into()).unwrap();
        assert_eq!(fs::read_to_string(root.join("Accounts").join("alpha").join("save.bin")).unwrap(), "save");

        rename_account(root.to_string_lossy().to_string(), "alpha".into(), "beta".into()).unwrap();
        assert!(!root.join("Accounts").join("alpha").exists());
        assert!(root.join("Accounts").join("beta").exists());

        delete_account(root.to_string_lossy().to_string(), "beta".into()).unwrap();
        assert!(!root.join("Accounts").join("beta").exists());
        let _ = fs::remove_dir_all(root);
    }
}
