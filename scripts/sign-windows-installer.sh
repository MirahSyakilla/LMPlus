#!/usr/bin/env bash
set -euo pipefail

target="${1:-}"
if [ -z "$target" ]; then
  echo "Usage: sign-windows-installer.sh <file-to-sign>" >&2
  exit 2
fi

if [ ! -f "$target" ]; then
  echo "File to sign not found: $target" >&2
  exit 1
fi

if ! command -v openssl >/dev/null 2>&1; then
  echo "openssl is required for self-signed Authenticode certificate generation." >&2
  exit 1
fi

if ! command -v osslsigncode >/dev/null 2>&1; then
  echo "osslsigncode is required for Authenticode signing." >&2
  exit 1
fi

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
app_dir="$(cd "$script_dir/.." && pwd)"
cert_dir="${LMPLUS_AUTHENTICODE_DIR:-$app_dir/src-tauri/target/authenticode}"
cert_pem="$cert_dir/lmplus-self-signed-code-signing.crt"
key_pem="$cert_dir/lmplus-self-signed-code-signing.key"

mkdir -p "$cert_dir"
chmod 700 "$cert_dir"

if [ ! -f "$cert_pem" ] || [ ! -f "$key_pem" ]; then
  openssl req -x509 -newkey rsa:3072 -sha256 -nodes -days 3650 \
    -subj "/C=MY/O=LMPlus/OU=Beta/CN=LMPlus Self-Signed Code Signing" \
    -addext "basicConstraints=critical,CA:FALSE" \
    -addext "keyUsage=critical,digitalSignature" \
    -addext "extendedKeyUsage=codeSigning" \
    -addext "subjectKeyIdentifier=hash" \
    -keyout "$key_pem" \
    -out "$cert_pem" >/dev/null 2>&1
  chmod 600 "$key_pem"
fi

signed_target="${target}.signed.$$"
sign_args=(
  sign
  -certs "$cert_pem"
  -key "$key_pem"
  -n "LMPlus"
  -i "https://lmp.nobullypls.site"
  -h sha256
  -in "$target"
  -out "$signed_target"
)

if [ -n "${LMPLUS_AUTHENTICODE_TIMESTAMP_URL:-}" ]; then
  sign_args+=(-t "$LMPLUS_AUTHENTICODE_TIMESTAMP_URL")
fi

osslsigncode "${sign_args[@]}"
mv -f "$signed_target" "$target"
osslsigncode verify -CAfile "$cert_pem" -in "$target" >/dev/null

echo "Authenticode self-signed: $target"
