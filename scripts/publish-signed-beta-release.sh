#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
APP="$ROOT/LMPlus-RS"
RELEASE_DIR="$ROOT/LMPlus-Release"

tauri_version="$(node -e "const c=require('$APP/src-tauri/tauri.conf.json'); console.log(String(c.version))")"
release_version="$(node -e "const c=require('$APP/src-tauri/tauri.conf.json'); const p=String(c.version).split('.'); while (p.length < 4) p.push('0'); console.log(p.slice(0,4).join('.'))")"
if [ -f "$APP/src-tauri/release-version.txt" ]; then
  release_version="$(tr -d '\r\n' < "$APP/src-tauri/release-version.txt")"
fi

source_installer="$APP/src-tauri/target/x86_64-pc-windows-msvc/release/bundle/nsis/LMPlus_${tauri_version}_x64-setup.exe"
source_sig="$source_installer.sig"
target_installer="$RELEASE_DIR/LMPlus_${release_version}_x64-setup.exe"
target_sig="$target_installer.sig"
authenticode_cert="${LMPLUS_AUTHENTICODE_CERT:-$APP/src-tauri/target/authenticode/lmplus-self-signed-code-signing.crt}"

if [ ! -f "$source_installer" ]; then
  echo "Signed installer not found: $source_installer" >&2
  echo "Build it first with the Windows Tauri target before publishing." >&2
  exit 1
fi

if [ ! -f "$source_sig" ]; then
  echo "Updater signature not found: $source_sig" >&2
  echo "Build with Tauri updater signing enabled before publishing." >&2
  exit 1
fi

if [ "$source_sig" -ot "$source_installer" ]; then
  echo "Updater signature is older than installer: $source_sig" >&2
  echo "Rebuild so the .sig matches the signed installer." >&2
  exit 1
fi

mkdir -p "$RELEASE_DIR"
cp -f "$source_installer" "$target_installer"
cp -f "$source_sig" "$target_sig"

if [ -f "$authenticode_cert" ] && command -v osslsigncode >/dev/null 2>&1; then
  osslsigncode verify -CAfile "$authenticode_cert" -in "$target_installer" >/dev/null
  echo "Authenticode verification passed for $target_installer"
else
  echo "Skipping Authenticode verification; certificate or osslsigncode was not found." >&2
fi

"$APP/scripts/publish-beta-release.sh" "$@"
