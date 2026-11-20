#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

"$ROOT_DIR/scripts/test-all.sh"

echo
echo "[Build] Frontend"

cd "$ROOT_DIR/frontend"
npm run build

echo
echo "[Compile] Python Core"

cd "$ROOT_DIR/python-core"
python -m compileall app

echo
echo "Full verification passed."