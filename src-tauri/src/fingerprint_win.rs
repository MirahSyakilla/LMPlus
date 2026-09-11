use sha2::{Digest, Sha256};
use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use std::os::windows::process::CommandExt;
use std::process::Command;
use winapi::shared::minwindef::{DWORD, HKEY};
use winapi::um::winbase::CREATE_NO_WINDOW;
use winapi::um::winnt::{
    KEY_QUERY_VALUE, KEY_READ, KEY_SET_VALUE, REG_OPTION_NON_VOLATILE, REG_SZ,
};
use winapi::um::winreg::{
    RegCloseKey, RegCreateKeyExW, RegOpenKeyExW, RegQueryValueExW, RegSetValueExW,
    HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE,
};

const PIN_SUBKEY: &str = "Software\\LMPlus";
const PIN_VALUE: &str = "DeviceFingerprint";

fn to_wide(s: &str) -> Vec<u16> {
    OsStr::new(s).encode_wide().chain(Some(0)).collect()
}

fn reg_read_string(root: HKEY, subkey: &str, value_name: &str) -> Option<String> {
    unsafe {
        let mut handle: HKEY = std::ptr::null_mut();
        if RegOpenKeyExW(root, to_wide(subkey).as_ptr(), 0, KEY_READ | KEY_QUERY_VALUE, &mut handle) != 0 {
            return None;
        }
        let mut len: DWORD = 0;
        let mut kind: DWORD = 0;
        let status = RegQueryValueExW(
            handle,
            to_wide(value_name).as_ptr(),
            std::ptr::null_mut(),
            &mut kind,
            std::ptr::null_mut(),
            &mut len,
        );
        if status != 0 || kind != REG_SZ || len == 0 {
            RegCloseKey(handle);
            return None;
        }
        let mut buf: Vec<u16> = vec![0; (len as usize + 1) / 2];
        let status = RegQueryValueExW(
            handle,
            to_wide(value_name).as_ptr(),
            std::ptr::null_mut(),
            &mut kind,
            buf.as_mut_ptr() as *mut u8,
            &mut len,
        );
        RegCloseKey(handle);
        if status != 0 {
            return None;
        }
        while buf.last() == Some(&0) {
            buf.pop();
        }
        Some(String::from_utf16_lossy(&buf))
    }
}

fn reg_write_string(root: HKEY, subkey: &str, value_name: &str, data: &str) -> bool {
    unsafe {
        let mut handle: HKEY = std::ptr::null_mut();
        if RegCreateKeyExW(
            root,
            to_wide(subkey).as_ptr(),
            0,
            std::ptr::null_mut(),
            REG_OPTION_NON_VOLATILE,
            KEY_SET_VALUE,
            std::ptr::null_mut(),
            &mut handle,
            std::ptr::null_mut(),
        ) != 0
        {
            return false;
        }
        let mut data_wide = to_wide(data);
        let size = (data_wide.len() * 2) as DWORD;
        let status = RegSetValueExW(
            handle,
            to_wide(value_name).as_ptr(),
            0,
            REG_SZ,
            data_wide.as_mut_ptr() as *const u8,
            size,
        );
        RegCloseKey(handle);
        status == 0
    }
}

fn run_wmic_query(class: &str, property: &str) -> Result<String, String> {
    let mut command = Command::new("wmic");
    let output = command
        .args([class, "get", property])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map_err(|e| format!("Failed to run wmic: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in stdout.lines() {
        let trimmed = line.trim();
        if !trimmed.is_empty() && !trimmed.eq_ignore_ascii_case(property) {
            return Ok(trimmed.to_string());
        }
    }

    Ok(String::new())
}

fn hash_parts(parts: &[String]) -> String {
    let joined = parts.join("|");
    let hash = Sha256::digest(joined.as_bytes());
    hex::encode(hash)
}

fn compute_legacy_fingerprint() -> Option<String> {
    let motherboard_uuid = run_wmic_query("csproduct", "UUID").ok()?;
    if motherboard_uuid.is_empty() {
        return None;
    }
    let cpu_id = run_wmic_query("cpu", "ProcessorId").unwrap_or_default();
    let disk_serial = run_wmic_query("diskdrive", "SerialNumber").unwrap_or_default();
    Some(hash_parts(&[motherboard_uuid, cpu_id, disk_serial]))
}

fn compute_stable_fingerprint() -> Option<String> {
    let machine_guid = reg_read_string(
        HKEY_LOCAL_MACHINE,
        r"SOFTWARE\Microsoft\Cryptography",
        "MachineGuid",
    )?;
    if machine_guid.is_empty() {
        return None;
    }
    let cpu_id = reg_read_string(
        HKEY_LOCAL_MACHINE,
        r"HARDWARE\DESCRIPTION\System\CentralProcessor\0",
        "Identifier",
    )
    .unwrap_or_default();
    let cpu_name = reg_read_string(
        HKEY_LOCAL_MACHINE,
        r"HARDWARE\DESCRIPTION\System\CentralProcessor\0",
        "ProcessorNameString",
    )
    .unwrap_or_default();
    let bios_version = reg_read_string(
        HKEY_LOCAL_MACHINE,
        r"HARDWARE\DESCRIPTION\System\BIOS",
        "SystemBiosVersion",
    )
    .unwrap_or_default();
    Some(hash_parts(&[machine_guid, cpu_id, cpu_name, bios_version]))
}

fn read_pinned_fingerprint() -> Option<String> {
    let pinned = reg_read_string(HKEY_CURRENT_USER, PIN_SUBKEY, PIN_VALUE)?;
    if pinned.len() == 64 && pinned.chars().all(|c| c.is_ascii_hexdigit()) {
        Some(pinned)
    } else {
        None
    }
}

#[tauri::command]
pub fn get_device_fingerprint() -> Result<String, String> {
    if let Some(pinned) = read_pinned_fingerprint() {
        return Ok(pinned);
    }

    let fingerprint = compute_legacy_fingerprint()
        .or_else(compute_stable_fingerprint)
        .ok_or("Failed to compute device fingerprint")?;

    reg_write_string(HKEY_CURRENT_USER, PIN_SUBKEY, PIN_VALUE, &fingerprint);

    Ok(fingerprint)
}
