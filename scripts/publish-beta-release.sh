#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
APP="$ROOT/LMPlus-RS"
RELEASE_DIR="$ROOT/LMPlus-Release"
REPO="${GITHUB_REPOSITORY:-MirahSyakilla/LMPlus}"

version="$(node -e "const c=require('$APP/src-tauri/tauri.conf.json'); const p=String(c.version).split('.'); while (p.length < 4) p.push('0'); console.log(p.slice(0,4).join('.'))")"
if [ -f "$APP/src-tauri/release-version.txt" ]; then
  version="$(tr -d '\r\n' < "$APP/src-tauri/release-version.txt")"
fi
tag="${1:-v${version}-beta}"
title="LMPlus ${version} Beta"

installer="$RELEASE_DIR/LMPlus_${version}_x64-setup.exe"
sig="$installer.sig"

if [ ! -f "$installer" ]; then
  echo "Installer not found: $installer" >&2
  echo "Build/sign the four-part beta installer before publishing." >&2
  exit 1
fi

if [ ! -f "$sig" ]; then
  echo "Signature not found: $sig" >&2
  exit 1
fi

if [ "$sig" -ot "$installer" ]; then
  echo "Signature is older than installer: $sig" >&2
  echo "Re-sign the installer before publishing." >&2
  exit 1
fi

if gh release view "$tag" --repo "$REPO" >/dev/null 2>&1; then
  gh release edit "$tag" --repo "$REPO" --title "$title" --prerelease >/dev/null
  gh release edit "$tag" --repo "$REPO" --latest=false >/dev/null 2>&1 || true
  gh release upload "$tag" "$installer" "$sig" --repo "$REPO" --clobber
else
  gh release create "$tag" "$installer" "$sig" \
    --repo "$REPO" \
    --title "$title" \
    --notes "Beta build for LMPlus ${version}. This prerelease is excluded from production update checks." \
    --prerelease \
    --latest=false
fi
