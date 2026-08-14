use sha2::{Digest, Sha256};
use std::process::Command;
use std::os::windows::process::CommandExt;
use winapi::um::winbase::CREATE_NO_WINDOW;

fn run_wmic_query(class: &str, property: &str) -> Result<String, String> {
    let mut command = Command::new("wmic");
    let output = command
        .args([class, "get", property])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map_err(|e| format!("Failed to run wmic: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.lines().collect();

    for line in &lines {
        let trimmed = line.trim();
        if !trimmed.is_empty() && !trimmed.eq_ignore_ascii_case(property) {
            return Ok(trimmed.to_string());
        }
    }

    Ok(String::new())
}

#[tauri::command]
pub fn get_device_fingerprint() -> Result<String, String> {
    let motherboard_uuid = run_wmic_query("csproduct", "UUID").unwrap_or_default();
    let cpu_id = run_wmic_query("cpu", "ProcessorId").unwrap_or_default();
    let disk_serial = run_wmic_query("diskdrive", "SerialNumber").unwrap_or_default();

    let joined = format!("{}|{}|{}", motherboard_uuid, cpu_id, disk_serial);
    let hash = Sha256::digest(joined.as_bytes());
    Ok(hex::encode(hash))
}
