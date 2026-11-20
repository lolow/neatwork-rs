#!/usr/bin/env bash

set -euo pipefail


ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

cd "$ROOT_DIR/application/python-core"

python -m pytest \
    tests/reference_validation \
    -m reference \
    -vv