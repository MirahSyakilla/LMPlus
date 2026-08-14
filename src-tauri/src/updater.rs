use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

const GITHUB_API: &str = "https://api.github.com/repos/MirahSyakilla/LMPlus/releases/latest";
const ZIP_PASSWORD: &str = "lmp25nbp";

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
    let current_version = env!("CARGO_PKG_VERSION");

    let client = reqwest::Client::builder()
        .user_agent("LMPlus/2.0")
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    let resp = client
        .get(GITHUB_API)
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    let json: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("Parse JSON: {}", e))?;

    let tag = json
        .get("tag_name")
        .and_then(|v| v.as_str())
        .unwrap_or("0.0.0")
        .trim_start_matches('v');

    let assets = json
        .get("assets")
        .and_then(|v| v.as_array())
        .ok_or("No assets in response")?;

    let mut zip_url = String::new();
    for asset in assets {
        let name = asset.get("name").and_then(|v| v.as_str()).unwrap_or("");
        if name.ends_with(".zip") {
            zip_url = asset
                .get("browser_download_url")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            break;
        }
    }

    let update_available = compare_versions(tag, current_version);

    Ok(serde_json::json!({
        "update_available": update_available,
        "latest_version": tag,
        "current_version": current_version,
        "zip_url": zip_url,
    }))
}

#[tauri::command]
pub async fn perform_update(zip_url: String) -> Result<(), String> {
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

    let client = reqwest::Client::new();
    let resp = client
        .get(&zip_url)
        .send()
        .await
        .map_err(|e| format!("Download failed: {}", e))?;

    let data = resp
        .bytes()
        .await
        .map_err(|e| format!("Read response: {}", e))?;

    let zip_path = tmp_dir.join("LMPlus_latest.zip");
    fs::write(&zip_path, &data).map_err(|e| format!("Failed to save zip: {}", e))?;

    let cursor = std::io::Cursor::new(&data);
    let mut archive =
        zip::ZipArchive::new(cursor).map_err(|e| format!("Failed to open zip: {}", e))?;

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

    hidden_command("powershell.exe")
        .args([
            "-ExecutionPolicy",
            "Bypass",
            "-File",
            &ps_path.to_string_lossy(),
        ])
        .spawn()
        .map_err(|e| format!("Failed to launch PowerShell: {}", e))?;

    std::process::exit(0);
}

fn compare_versions(latest: &str, current: &str) -> bool {
    let l_parts: Vec<u32> = latest.split('.').filter_map(|s| s.parse().ok()).collect();
    let c_parts: Vec<u32> = current.split('.').filter_map(|s| s.parse().ok()).collect();
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
