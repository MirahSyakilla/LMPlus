use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;

use crate::config::{self, ReleaseChannel};
use futures_util::StreamExt;
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

const GITHUB_RELEASES_API: &str =
    "https://api.github.com/repos/MirahSyakilla/LMPlus/releases?per_page=20";
const ZIP_PASSWORD: &str = "lmp25nbp";
const UPDATE_PROGRESS_EVENT: &str = "lmplus-update-progress";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum AssetKind {
    Zip,
    Installer,
}

impl AssetKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Zip => "zip",
            Self::Installer => "installer",
        }
    }
}

struct ReleaseAsset {
    version: String,
    download_url: String,
    kind: AssetKind,
}

#[derive(Clone, Serialize)]
struct UpdateProgressPayload {
    phase: &'static str,
    unit: &'static str,
    downloaded: u64,
    total: Option<u64>,
    speed_bps: Option<u64>,
    message: Option<String>,
}

fn hidden_command(program: &str) -> Command {
    #[allow(unused_mut)]
    let mut command = Command::new(program);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        use winapi::um::winbase::CREATE_NO_WINDOW;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    command
}

fn exe_dir() -> Result<PathBuf, String> {
    let exe = env::current_exe().map_err(|e| format!("Failed to get exe path: {}", e))?;
    exe.parent()
        .map(|p| p.to_path_buf())
        .ok_or("Failed to get exe directory".to_string())
}

#[tauri::command]
pub async fn check_for_update() -> Result<serde_json::Value, String> {
    let current_version = normalized_current_version();
    let desired_channel = config::get_release_channel()?;
    let applied_channel = config::get_applied_release_channel().unwrap_or(desired_channel);

    let client = reqwest::Client::builder()
        .user_agent("LMPlus/2.0")
        .timeout(Duration::from_secs(15))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    let resp = client
        .get(GITHUB_RELEASES_API)
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("GitHub API error: {}", resp.status()));
    }

    let releases: Vec<serde_json::Value> = resp
        .json()
        .await
        .map_err(|e| format!("Parse JSON: {}", e))?;

    let release = find_latest_channel_asset(&releases, desired_channel)
        .ok_or_else(|| format!("No {} release asset found", desired_channel.as_str()))?;

    let channel_switch_required = desired_channel != applied_channel;
    let update_available =
        channel_switch_required || compare_versions(&release.version, &current_version);

    Ok(serde_json::json!({
        "update_available": update_available,
        "latest_version": release.version,
        "current_version": current_version,
        "download_url": release.download_url,
        "zip_url": release.download_url,
        "asset_kind": release.kind.as_str(),
        "release_channel": desired_channel.as_str(),
        "current_channel": applied_channel.as_str(),
        "channel_switch_required": channel_switch_required,
    }))
}

#[tauri::command]
pub async fn perform_update(app: AppHandle, download_url: String) -> Result<(), String> {
    emit_update_progress(
        &app,
        UpdateProgressPayload {
            phase: "preparing",
            unit: "none",
            downloaded: 0,
            total: None,
            speed_bps: None,
            message: Some("Preparing update...".to_string()),
        },
    );
    match infer_asset_kind(&download_url)? {
        AssetKind::Zip => perform_zip_update(&app, download_url).await,
        AssetKind::Installer => perform_installer_update(&app, download_url).await,
    }
}

async fn perform_zip_update(app: &AppHandle, download_url: String) -> Result<(), String> {
    let desired_channel = config::get_release_channel()?;
    let dir = exe_dir()?;
    let exe_name = env::current_exe()
        .ok()
        .and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string()))
        .unwrap_or_else(|| "lmplus.exe".to_string());
    let tmp_dir = dir.join("tmp");

    if tmp_dir.exists() {
        let _ = fs::remove_dir_all(&tmp_dir);
    }
    fs::create_dir_all(&tmp_dir).map_err(|e| format!("Failed to create tmp dir: {}", e))?;

    let data = download_update_asset(app, &download_url).await?;

    let zip_path = tmp_dir.join("LMPlus_latest.zip");
    fs::write(&zip_path, &data).map_err(|e| format!("Failed to save zip: {}", e))?;

    let cursor = std::io::Cursor::new(&data);
    let mut archive =
        zip::ZipArchive::new(cursor).map_err(|e| format!("Failed to open zip: {}", e))?;
    let total_entries = archive.len() as u64;
    emit_update_progress(
        app,
        UpdateProgressPayload {
            phase: "extracting",
            unit: "files",
            downloaded: 0,
            total: Some(total_entries),
            speed_bps: None,
            message: Some("Extracting update files...".to_string()),
        },
    );

    for i in 0..archive.len() {
        let mut file = archive
            .by_index_decrypt(i, ZIP_PASSWORD.as_bytes())
            .map_err(|e| format!("Failed to read zip entry {}: {}", i, e))?;

        let name = file.name().to_string();
        let out_path = tmp_dir.join(&name);

        if file.is_dir() {
            fs::create_dir_all(&out_path).ok();
            continue;
        }

        if let Some(parent) = out_path.parent() {
            fs::create_dir_all(parent).ok();
        }

        let mut out_file = fs::File::create(&out_path)
            .map_err(|e| format!("Failed to create file {}: {}", out_path.display(), e))?;
        std::io::copy(&mut file, &mut out_file)
            .map_err(|e| format!("Failed to write file {}: {}", out_path.display(), e))?;

        emit_update_progress(
            app,
            UpdateProgressPayload {
                phase: "extracting",
                unit: "files",
                downloaded: (i + 1) as u64,
                total: Some(total_entries),
                speed_bps: None,
                message: Some("Extracting update files...".to_string()),
            },
        );
    }

    let ps_path = dir.join("update.ps1");
    let ps_script = r#"$ErrorActionPreference = 'Stop'
$folder = Split-Path -Parent $MyInvocation.MyCommand.Path
$tmp = Join-Path $folder "tmp"
$zip = Join-Path $tmp "LMPlus_latest.zip"
$maxAttempts = 3
$attempt = 0
while (Get-Process -Name LMPlus -ErrorAction SilentlyContinue) {{
  $attempt++
  if ($attempt -gt $maxAttempts) {{
    Write-Warning 'Failed to terminate LMPlus after $maxAttempts attempts'
    break
  }}
  Stop-Process -Name LMPlus -Force -ErrorAction SilentlyContinue
  Start-Sleep -Seconds 2
}}
Get-ChildItem $tmp -Recurse | ForEach-Object {{
  $relativePath = $_.FullName.Substring($tmp.Length + 1)
  $dest = Join-Path $folder $relativePath
  try {{
    if (-not $_.PSIsContainer) {{
      $destDir = Split-Path $dest -Parent
      if (-not (Test-Path $destDir)) {{ New-Item -ItemType Directory -Path $destDir -Force | Out-Null }}
      Move-Item -Path $_.FullName -Destination $dest -Force -ErrorAction Stop
    }} else {{
      if (-not (Test-Path $dest)) {{ New-Item -ItemType Directory -Path $dest -Force | Out-Null }}
    }}
  }} catch {{
    Write-Error "Failed to process $($_.FullName): $_"
    exit 1
  }}
}}
Remove-Item (Join-Path $folder "*.log") -Force -ErrorAction SilentlyContinue
Remove-Item $tmp -Recurse -Force -ErrorAction SilentlyContinue
Remove-Item (Join-Path $folder "*.zip") -Force -ErrorAction SilentlyContinue
Remove-Item (Join-Path $folder "Updater.exe") -Force -ErrorAction SilentlyContinue
try {{
  Start-Process -FilePath (Join-Path $folder "__EXE_NAME__") -ErrorAction Stop
}} catch {{
  Write-Error "Failed to start __EXE_NAME__: $_"
  exit 1
}}
Remove-Item $MyInvocation.MyCommand.Path -Force -ErrorAction SilentlyContinue
"#
    .replace("__EXE_NAME__", &exe_name);

    fs::write(&ps_path, ps_script)
        .map_err(|e| format!("Failed to write PowerShell script: {}", e))?;

    emit_update_progress(
        app,
        UpdateProgressPayload {
            phase: "installing",
            unit: "none",
            downloaded: 0,
            total: None,
            speed_bps: None,
            message: Some("Applying update...".to_string()),
        },
    );

    hidden_command("powershell.exe")
        .args([
            "-ExecutionPolicy",
            "Bypass",
            "-File",
            &ps_path.to_string_lossy(),
        ])
        .spawn()
        .map_err(|e| format!("Failed to launch PowerShell: {}", e))?;

    emit_update_progress(
        app,
        UpdateProgressPayload {
            phase: "restarting",
            unit: "none",
            downloaded: 0,
            total: None,
            speed_bps: None,
            message: Some("Restarting LMPlus...".to_string()),
        },
    );

    let _ = config::set_release_channel(desired_channel);
    let _ = config::set_applied_release_channel(desired_channel);
    std::process::exit(0);
}

async fn perform_installer_update(app: &AppHandle, download_url: String) -> Result<(), String> {
    let desired_channel = config::get_release_channel()?;
    let dir = exe_dir()?;
    let exe_name = env::current_exe()
        .ok()
        .and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string()))
        .unwrap_or_else(|| "lmplus.exe".to_string());
    let tmp_dir = dir.join("tmp");

    if tmp_dir.exists() {
        let _ = fs::remove_dir_all(&tmp_dir);
    }
    fs::create_dir_all(&tmp_dir).map_err(|e| format!("Failed to create tmp dir: {}", e))?;

    let data = download_update_asset(app, &download_url).await?;
    let installer_name =
        file_name_from_url(&download_url).unwrap_or_else(|| "LMPlus_latest_setup.exe".to_string());
    let installer_path = tmp_dir.join(&installer_name);
    fs::write(&installer_path, &data).map_err(|e| format!("Failed to save installer: {}", e))?;

    let ps_path = dir.join("update.ps1");
    let ps_script = r#"$ErrorActionPreference = 'Stop'
$folder = Split-Path -Parent $MyInvocation.MyCommand.Path
$tmp = Join-Path $folder "tmp"
$installer = Join-Path $tmp "__INSTALLER_NAME__"
$maxAttempts = 3
$attempt = 0
while (Get-Process -Name LMPlus -ErrorAction SilentlyContinue) {
  $attempt++
  if ($attempt -gt $maxAttempts) {
    Write-Warning 'Failed to terminate LMPlus after $maxAttempts attempts'
    break
  }
  Stop-Process -Name LMPlus -Force -ErrorAction SilentlyContinue
  Start-Sleep -Seconds 2
}
Start-Process -FilePath $installer -ArgumentList '/S' -Wait -ErrorAction Stop
Remove-Item $tmp -Recurse -Force -ErrorAction SilentlyContinue
try {
  Start-Process -FilePath (Join-Path $folder "__EXE_NAME__") -ErrorAction Stop
} catch {
  Write-Warning "Failed to start __EXE_NAME__: $_"
}
Remove-Item $MyInvocation.MyCommand.Path -Force -ErrorAction SilentlyContinue
"#
    .replace("__INSTALLER_NAME__", &installer_name)
    .replace("__EXE_NAME__", &exe_name);

    fs::write(&ps_path, ps_script)
        .map_err(|e| format!("Failed to write PowerShell script: {}", e))?;

    emit_update_progress(
        app,
        UpdateProgressPayload {
            phase: "installing",
            unit: "none",
            downloaded: 0,
            total: None,
            speed_bps: None,
            message: Some("Launching installer...".to_string()),
        },
    );

    hidden_command("powershell.exe")
        .args([
            "-ExecutionPolicy",
            "Bypass",
            "-File",
            &ps_path.to_string_lossy(),
        ])
        .spawn()
        .map_err(|e| format!("Failed to launch PowerShell: {}", e))?;

    emit_update_progress(
        app,
        UpdateProgressPayload {
            phase: "restarting",
            unit: "none",
            downloaded: 0,
            total: None,
            speed_bps: None,
            message: Some("Restarting LMPlus...".to_string()),
        },
    );

    let _ = config::set_release_channel(desired_channel);
    let _ = config::set_applied_release_channel(desired_channel);
    std::process::exit(0);
}

async fn download_update_asset(app: &AppHandle, download_url: &str) -> Result<Vec<u8>, String> {
    let client = reqwest::Client::builder()
        .user_agent("LMPlus/2.0")
        .timeout(Duration::from_secs(120))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    let resp = client
        .get(download_url)
        .send()
        .await
        .map_err(|e| format!("Download failed: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("Download failed with status: {}", resp.status()));
    }

    let total = resp.content_length();
    let mut stream = resp.bytes_stream();
    let mut data = Vec::new();
    let mut downloaded = 0u64;
    let started = std::time::Instant::now();

    emit_update_progress(
        app,
        UpdateProgressPayload {
            phase: "downloading",
            unit: "bytes",
            downloaded: 0,
            total,
            speed_bps: Some(0),
            message: Some("Downloading update package...".to_string()),
        },
    );

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| format!("Read response: {}", e))?;
        downloaded += chunk.len() as u64;
        data.extend_from_slice(&chunk);
        let elapsed = started.elapsed().as_secs_f64();
        let speed_bps = if elapsed > 0.0 {
            Some((downloaded as f64 / elapsed) as u64)
        } else {
            Some(0)
        };
        emit_update_progress(
            app,
            UpdateProgressPayload {
                phase: "downloading",
                unit: "bytes",
                downloaded,
                total,
                speed_bps,
                message: Some("Downloading update package...".to_string()),
            },
        );
    }

    Ok(data)
}

fn emit_update_progress(app: &AppHandle, payload: UpdateProgressPayload) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.emit(UPDATE_PROGRESS_EVENT, payload);
    } else {
        let _ = app.emit(UPDATE_PROGRESS_EVENT, payload);
    }
}

fn find_latest_channel_asset(
    releases: &[serde_json::Value],
    channel: ReleaseChannel,
) -> Option<ReleaseAsset> {
    for release in releases {
        if release
            .get("draft")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
        {
            continue;
        }

        let prerelease = release
            .get("prerelease")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let tag = release
            .get("tag_name")
            .and_then(|v| v.as_str())
            .unwrap_or("0.0.0");

        let matches_channel = match channel {
            ReleaseChannel::Stable => !prerelease && !tag.to_ascii_lowercase().contains("beta"),
            ReleaseChannel::Beta => prerelease || tag.to_ascii_lowercase().contains("beta"),
        };
        if !matches_channel {
            continue;
        }

        let Some(assets) = release.get("assets").and_then(|v| v.as_array()) else {
            continue;
        };
        if let Some((download_url, kind)) = select_update_asset(assets) {
            return Some(ReleaseAsset {
                version: tag.trim_start_matches('v').to_string(),
                download_url,
                kind,
            });
        }
    }

    None
}

fn select_update_asset(assets: &[serde_json::Value]) -> Option<(String, AssetKind)> {
    let mut installer = None;
    let mut zip = None;

    for asset in assets {
        let name = asset.get("name").and_then(|v| v.as_str()).unwrap_or("");
        let url = asset
            .get("browser_download_url")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        if url.is_empty() {
            continue;
        }

        if is_installer_asset(name) {
            installer = Some((url.to_string(), AssetKind::Installer));
        } else if is_zip_asset(name) {
            zip = Some((url.to_string(), AssetKind::Zip));
        }
    }

    installer.or(zip)
}

fn is_installer_asset(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    lower.ends_with(".exe")
        && (lower == "lmplus_installer.exe"
            || lower.starts_with("lmplus_") && lower.ends_with("_x64-setup.exe"))
}

fn is_zip_asset(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    lower == "lmplus.zip" || lower.ends_with(".zip")
}

fn infer_asset_kind(download_url: &str) -> Result<AssetKind, String> {
    let path = reqwest::Url::parse(download_url)
        .ok()
        .map(|url| url.path().to_ascii_lowercase())
        .unwrap_or_else(|| download_url.to_ascii_lowercase());

    if path.ends_with(".exe") {
        Ok(AssetKind::Installer)
    } else if path.ends_with(".zip") {
        Ok(AssetKind::Zip)
    } else {
        Err("Unsupported update asset type".to_string())
    }
}

fn file_name_from_url(download_url: &str) -> Option<String> {
    reqwest::Url::parse(download_url)
        .ok()
        .and_then(|url| {
            url.path_segments()
                .and_then(|mut segments| segments.next_back().map(|s| s.to_string()))
        })
        .filter(|name| !name.is_empty())
}

fn compare_versions(latest: &str, current: &str) -> bool {
    let l_parts = version_parts(latest);
    let c_parts = version_parts(current);
    let len = l_parts.len().max(c_parts.len());
    for i in 0..len {
        let l = l_parts.get(i).copied().unwrap_or(0);
        let c = c_parts.get(i).copied().unwrap_or(0);
        if l > c {
            return true;
        }
        if l < c {
            return false;
        }
    }
    false
}

fn version_parts(version: &str) -> Vec<u32> {
    version
        .trim()
        .trim_start_matches('v')
        .split('.')
        .filter_map(|part| {
            let digits: String = part.chars().take_while(|c| c.is_ascii_digit()).collect();
            if digits.is_empty() {
                None
            } else {
                digits.parse::<u32>().ok()
            }
        })
        .collect()
}

fn normalized_current_version() -> String {
    let raw = option_env!("LMPLUS_RELEASE_VERSION").unwrap_or(env!("CARGO_PKG_VERSION"));
    let mut parts: Vec<&str> = raw.trim().trim_start_matches('v').split('.').collect();
    while parts.len() < 4 {
        parts.push("0");
    }
    parts.truncate(4);
    parts.join(".")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compare_versions_accepts_four_part_and_suffix_tags() {
        assert!(compare_versions("2.5.0.0-beta", "2.4.0.2"));
        assert!(!compare_versions("2.5.0.0-beta", "2.5.0"));
        assert!(compare_versions("2.5.0.1", "2.5.0"));
    }

    #[test]
    fn detects_asset_kinds() {
        assert!(matches!(
            infer_asset_kind("https://example.com/LMPlus_2.5.0.0_x64-setup.exe"),
            Ok(AssetKind::Installer)
        ));
        assert!(matches!(
            infer_asset_kind("https://example.com/LMPlus.zip"),
            Ok(AssetKind::Zip)
        ));
    }
}
