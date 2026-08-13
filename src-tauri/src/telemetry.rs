use std::process::Command;
use std::time::SystemTime;
use once_cell::sync::Lazy;
use std::sync::Mutex;
use sha2::Digest;

static LAST_SENT_TIME: Lazy<Mutex<u64>> = Lazy::new(|| Mutex::new(0));
static UPTIME_START: Lazy<Mutex<u64>> = Lazy::new(|| {
    Mutex::new(
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
    )
});

fn get_os_version() -> String {
    let output = Command::new("cmd")
        .args(["/c", "ver"])
        .output()
        .ok();
    output
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "windows".to_string())
}

fn run_powershell(command: &str) -> Option<String> {
    let output = Command::new("powershell")
        .args(["-Command", command])
        .output()
        .ok()?;
    let s = String::from_utf8(output.stdout).ok()?;
    let trimmed = s.trim().to_string();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed)
    }
}

fn get_cpu_model() -> String {
    run_powershell("Get-CimInstance Win32_Processor | Select-Object -ExpandProperty Name")
        .unwrap_or_else(|| "unknown".to_string())
}

fn get_ram_mb() -> i64 {
    run_powershell("Get-CimInstance Win32_OperatingSystem | Select-Object -ExpandProperty TotalVisibleMemorySize")
        .and_then(|s| s.parse::<i64>().ok())
        .map(|kb| kb / 1024)
        .unwrap_or(-1)
}

fn get_gpu_model() -> String {
    run_powershell("Get-CimInstance Win32_VideoController | Select-Object -ExpandProperty Name")
        .unwrap_or_else(|| "unknown".to_string())
}

fn get_timezone() -> String {
    let output = Command::new("cmd")
        .args(["/c", "tzutil", "/g"])
        .output()
        .ok();
    output
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "unknown".to_string())
}

fn anonymize_path(path: &str) -> String {
    let username = std::env::var("USERNAME").unwrap_or_default();
    let home = std::env::var("USERPROFILE").unwrap_or_default();
    let mut result = path.to_string();
    if !username.is_empty() {
        result = result.replace(&username, "[user]");
    }
    if !home.is_empty() {
        result = result.replace(&home, "[home]");
    }
    result = result.replace("C:\\", "[drive]\\").replace("c:\\", "[drive]\\");
    result
}

fn get_license_key_last10(fingerprint: &str) -> String {
    use std::fs;
    use std::path::PathBuf;
    let exe = std::env::current_exe().unwrap_or_default();
    let dir = exe.parent().unwrap_or(std::path::Path::new("."));
    let path: PathBuf = dir.join("config.cfg");
    let content = fs::read_to_string(&path).unwrap_or_default();
    let mut ek = String::new();
    for line in content.lines() {
        let t = line.trim();
        if t.starts_with("ek=") {
            ek = t[3..].trim().to_string();
            break;
        }
    }
    if ek.is_empty() {
        return "unknown".to_string();
    }

    let key = crate::crypto::decrypt_aes256_cbc(ek, fingerprint.to_string()).unwrap_or_default();
    if key.is_empty() {
        return "unknown".to_string();
    }
    if key.len() >= 10 {
        key[key.len() - 10..].to_string()
    } else {
        key
    }
}

#[tauri::command]
pub async fn collect_and_send_telemetry(
    base_path: String,
    startup_time: i64,
    hide_count: i64,
    launch_count: i64,
    total_uptime: i64,
    fingerprint: String,
) -> Result<(), String> {
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    {
        let mut last = LAST_SENT_TIME.lock().map_err(|e| e.to_string())?;
        if now - *last < 1800 {
            return Ok(());
        }
        *last = now;
    }

    let app_path = std::env::current_exe()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();

    let accounts_dir = std::path::PathBuf::from(&base_path).join("Accounts");
    let total_accounts = if accounts_dir.exists() {
        std::fs::read_dir(&accounts_dir)
            .map(|entries| entries.filter(|e| e.as_ref().map(|x| x.file_type().map(|t| t.is_dir()).unwrap_or(false)).unwrap_or(false)).count())
            .unwrap_or(0u64)
    } else {
        0u64
    };

    let country = reqwest::get("http://ip-api.com/json")
        .await
        .ok()
        .and_then(|r| r.json::<serde_json::Value>().ok())
        .and_then(|v| v.get("countryCode").and_then(|c| c.as_str()).map(|s| s.to_string()))
        .unwrap_or_else(|| "unknown".to_string());

    let json = serde_json::json!({
        "launch_count": launch_count,
        "startup_time": startup_time,
        "hide_to_tray_count": hide_count,
        "os_version": get_os_version(),
        "app_version": env!("CARGO_PKG_VERSION"),
        "hardware_info": {
            "cpu_model": get_cpu_model(),
            "ram_size_mb": get_ram_mb(),
            "gpu_model": get_gpu_model(),
        },
        "app_full_path": anonymize_path(&app_path),
        "base_path": anonymize_path(&base_path),
        "total_accounts": total_accounts,
        "total_assigned_hotkeys": 0,
        "network_info": country,
        "app_total_uptime": total_uptime,
        "user_license_key_last10": get_license_key_last10(&fingerprint),
        "pc_timezone": get_timezone(),
    });

    let json_str = serde_json::to_string(&json).unwrap_or_default();
    let url = format!(
        "http://lmp.nobullypls.site/clt?data={}",
        urlencoding::encode(&json_str)
    );

    let _ = reqwest::get(&url).await;
    Ok(())
}

#[tauri::command]
pub fn increment_hide_to_tray() -> Result<i64, String> {
    Ok(0)
}