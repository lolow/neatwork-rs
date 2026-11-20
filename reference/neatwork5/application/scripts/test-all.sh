#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

echo
echo "========================================"
echo " NeatWork automated test suite"
echo "========================================"
echo

echo "[1/3] Frontend"
echo "----------------------------------------"

cd "$ROOT_DIR/frontend"

npm run typecheck
npm test

echo
echo "[2/3] Backend"
echo "----------------------------------------"

cd "$ROOT_DIR/backend"
./mvnw test

echo
echo "[3/3] Python Core"
echo "----------------------------------------"

cd "$ROOT_DIR/python-core"
python -m pytest

echo
echo "========================================"
echo " All NeatWork tests passed."
echo "========================================"