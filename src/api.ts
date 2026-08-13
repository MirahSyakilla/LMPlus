import { invoke } from "@tauri-apps/api/core";

export interface AppPaths {
  base_path: string;
  exe_path: string;
  process_name: string;
  accounts_path: string;
  userdata_path: string;
}

export interface HotkeyEntry {
  id: number;
  name: string;
  type: string;
  hotkey: string;
}

export interface MiscMacro {
  name: string;
  hotkey: string;
  points: { x: number; y: number }[];
}

export interface UpdateInfo {
  update_available: boolean;
  latest_version: string;
  current_version: string;
  zip_url: string;
}

export interface LicenseResult {
  status: string;
  license_info?: string;
}

// --- App paths ---
export async function getAppPaths(): Promise<AppPaths> {
  return invoke("get_app_paths");
}

export async function setBasePath(path: string): Promise<void> {
  return invoke("set_base_path", { path });
}

// --- Version ---
export async function getVersion(): Promise<string> {
  return invoke("get_version");
}

// --- License ---
export async function verifyLicense(key: string, fingerprint: string): Promise<LicenseResult> {
  return invoke("verify_license", { key, fingerprint });
}

export async function loadSavedLicense(fingerprint: string): Promise<{ key: string | null }> {
  return invoke("load_saved_license", { fingerprint });
}

export async function getConfigSignatureStatus(fingerprint: string): Promise<boolean> {
  return invoke("get_config_signature_status", { fingerprint });
}

export async function getLicenseInfo(): Promise<string> {
  return invoke("get_license_info");
}

export async function setLicenseInfo(info: string): Promise<void> {
  return invoke("set_license_info", { info });
}

// --- Fingerprint ---
export async function getDeviceFingerprint(): Promise<string> {
  return invoke("get_device_fingerprint");
}

// --- Accounts ---
export async function loadAccounts(basePath: string): Promise<string[]> {
  return invoke("load_accounts", { basePath });
}

export async function getSettings(): Promise<{ basePath: string; accountOrder: string[] }> {
  return invoke("get_settings");
}

export async function setSettings(settings: { basePath: string }): Promise<void> {
  return invoke("set_settings", { settings });
}

export async function getAccountOrder(): Promise<string[]> {
  return invoke("get_account_order");
}

export async function setAccountOrder(order: string[]): Promise<void> {
  return invoke("set_account_order", { order });
}

// --- Account operations ---
export async function switchAccount(basePath: string, account: string): Promise<void> {
  return invoke("switch_account", { basePath, account });
}

export async function addAccount(basePath: string, name: string): Promise<void> {
  return invoke("add_account", { basePath, name });
}

export async function renameAccount(basePath: string, oldName: string, newName: string): Promise<void> {
  return invoke("rename_account", { basePath, oldName, newName });
}

export async function deleteAccount(basePath: string, name: string): Promise<void> {
  return invoke("delete_account", { basePath, name });
}

// --- Hotkeys ---
export async function getHotkeySettings(): Promise<Record<string, string>> {
  return invoke("get_hotkey_settings");
}

export async function setHotkey(category: string, name: string, hotkey: string): Promise<void> {
  return invoke("set_hotkey", { category, name, hotkey });
}

export async function removeHotkey(category: string, name: string): Promise<void> {
  return invoke("remove_hotkey", { category, name });
}

export async function registerHotkeys(accountsPath: string, settingsJson: string): Promise<HotkeyEntry[]> {
  return invoke("register_hotkeys", { accountsPath, settingsJson });
}

export async function unregisterHotkeys(): Promise<void> {
  return invoke("unregister_hotkeys");
}

// --- Misc config ---
export async function loadMiscCfg(): Promise<MiscMacro[]> {
  return invoke("load_misc_cfg");
}

export async function saveMiscCfg(macros: MiscMacro[]): Promise<void> {
  return invoke("save_misc_cfg", { macros });
}

export async function parseMiscCfg(): Promise<MiscMacro[]> {
  return invoke("parse_misc_cfg");
}

// --- Process ---
export async function launchGame(exePath: string, processName: string): Promise<void> {
  return invoke("launch_game", { exePath, processName });
}

export async function killGame(exePath: string, processName: string): Promise<boolean> {
  return invoke("kill_game", { exePath, processName });
}

export async function restartGame(exePath: string, processName: string, relaunch: boolean): Promise<void> {
  return invoke("restart_game", { exePath, processName, relaunch });
}

export async function isGameRunning(exePath: string, processName: string): Promise<boolean> {
  return invoke("is_game_running", { exePath, processName });
}

export async function isAnotherInstanceRunning(): Promise<boolean> {
  return invoke("is_another_instance_running");
}

export async function launchLmUpdater(): Promise<void> {
  return invoke("launch_lm_updater");
}

// --- Macros ---
export async function executeMacro(macroName: string, exePath: string, processName: string): Promise<void> {
  return invoke("execute_macro", { macroName, exePath, processName });
}

// --- Map Zoom ---
export async function performMapZoom(exePath: string, persistent: boolean): Promise<void> {
  return invoke("perform_map_zoom", { exePath, persistent });
}

export async function stopPersistentZoom(): Promise<void> {
  return invoke("stop_persistent_zoom");
}

// --- Telemetry ---
export async function collectAndSendTelemetry(
  basePath: string,
  startupTime: number,
  hideCount: number,
  launchCount: number,
  totalUptime: number,
  fingerprint: string,
): Promise<void> {
  return invoke("collect_and_send_telemetry", {
    basePath,
    startupTime,
    hideCount,
    launchCount,
    totalUptime,
    fingerprint,
  });
}

export async function incrementHideToTray(): Promise<number> {
  return invoke("increment_hide_to_tray");
}

// --- Updater ---
export async function checkForUpdate(): Promise<UpdateInfo> {
  return invoke("check_for_update");
}

export async function performUpdate(zipUrl: string): Promise<void> {
  return invoke("perform_update", { zipUrl });
}

// --- Crypto ---
export async function encryptAes256Cbc(data: string, keyHex: string): Promise<string> {
  return invoke("encrypt_aes256_cbc", { data, keyHex });
}

export async function decryptAes256Cbc(dataB64: string, keyHex: string): Promise<string> {
  return invoke("decrypt_aes256_cbc", { dataB64, keyHex });
}

export async function sha256Hex(data: string): Promise<string> {
  return invoke("sha256_hex", { data });
}

export async function hmacSha256(data: string, keyHex: string): Promise<string> {
  return invoke("hmac_sha256", { data, keyHex });
}
