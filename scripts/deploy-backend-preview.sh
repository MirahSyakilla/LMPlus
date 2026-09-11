#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
BACKEND="$ROOT/lmplus-backend"

cd "$BACKEND"

if [ ! -d node_modules ]; then
  npm install
fi

npx wrangler deploy --env staging
