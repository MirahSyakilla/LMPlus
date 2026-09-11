#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
BACKEND="$ROOT/lmplus-backend"
ENV_NAME="${1:-staging}"
SECRET_FILE="${2:-}"
SECRET_NAME="${LMPLUS_CF_PRIVATE_KEY_SECRET_NAME:-LMPLUS_PRIVATE_KEY_PEM}"
LOCAL_SECRET_FILE="${LMPLUS_PRIVATE_KEY_FILE:-$HOME/.lmplus-secrets/lmplus-backend-oneline_pem.txt}"

cd "$BACKEND"

if [ -n "$SECRET_FILE" ]; then
  npx wrangler secret bulk "$SECRET_FILE" --env "$ENV_NAME"
  exit 0
fi

if [ -n "${PRIVATE_KEY_PEM:-}" ]; then
  printf '%s' "$PRIVATE_KEY_PEM" | npx wrangler secret put "$SECRET_NAME" --env "$ENV_NAME"
  exit 0
fi

if [ -f "$LOCAL_SECRET_FILE" ]; then
  private_key="$(tr -d '\r' < "$LOCAL_SECRET_FILE")"
  if [ -n "$private_key" ]; then
    printf '%s' "$private_key" | npx wrangler secret put "$SECRET_NAME" --env "$ENV_NAME"
    exit 0
  fi
fi

private_key="$(
  LMPLUS_SECRET_ENV="$ENV_NAME" node <<'NODE'
const fs = require('fs');

function stripJsonComments(input) {
  let out = '';
  let inString = false;
  let escaped = false;
  let lineComment = false;
  let blockComment = false;

  for (let i = 0; i < input.length; i += 1) {
    const ch = input[i];
    const next = input[i + 1];

    if (lineComment) {
      if (ch === '\n') {
        lineComment = false;
        out += ch;
      }
      continue;
    }

    if (blockComment) {
      if (ch === '*' && next === '/') {
        blockComment = false;
        i += 1;
      }
      continue;
    }

    if (inString) {
      out += ch;
      if (escaped) {
        escaped = false;
      } else if (ch === '\\') {
        escaped = true;
      } else if (ch === '"') {
        inString = false;
      }
      continue;
    }

    if (ch === '"') {
      inString = true;
      out += ch;
      continue;
    }

    if (ch === '/' && next === '/') {
      lineComment = true;
      i += 1;
      continue;
    }

    if (ch === '/' && next === '*') {
      blockComment = true;
      i += 1;
      continue;
    }

    out += ch;
  }

  return out;
}

function removeTrailingCommas(input) {
  let out = '';
  let inString = false;
  let escaped = false;

  for (let i = 0; i < input.length; i += 1) {
    const ch = input[i];

    if (inString) {
      out += ch;
      if (escaped) {
        escaped = false;
      } else if (ch === '\\') {
        escaped = true;
      } else if (ch === '"') {
        inString = false;
      }
      continue;
    }

    if (ch === '"') {
      inString = true;
      out += ch;
      continue;
    }

    if (ch === ',') {
      let j = i + 1;
      while (j < input.length && /\s/.test(input[j])) j += 1;
      if (input[j] === '}' || input[j] === ']') {
        continue;
      }
    }

    out += ch;
  }

  return out;
}

const envName = process.env.LMPLUS_SECRET_ENV;
const raw = fs.readFileSync('wrangler.jsonc', 'utf8');
const parsed = JSON.parse(removeTrailingCommas(stripJsonComments(raw)));
const value = parsed?.env?.[envName]?.vars?.PRIVATE_KEY_PEM ?? '';
process.stdout.write(value);
NODE
)"

if [ -z "$private_key" ]; then
  echo "No PRIVATE_KEY_PEM found for env '$ENV_NAME'. Pass a secret JSON file, set PRIVATE_KEY_PEM, or provide $LOCAL_SECRET_FILE." >&2
  exit 1
fi

printf '%s' "$private_key" | npx wrangler secret put "$SECRET_NAME" --env "$ENV_NAME"
