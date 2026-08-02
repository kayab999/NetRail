#!/usr/bin/env bash
# Sprint 4 (dual-stack benchmarks) — one command.
#
#   bash scripts/bench-dual.sh
#
# Runs the async httpx benchmark against the Rust netrail-api release binary
# and the Python FastAPI/uvicorn server (3 steady-state runs + saturation knee
# scan), then writes docs/bench-dual.md and docs/assets/bench-{rust,python}.json.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
if [ -x "${ROOT}/.venv/bin/python" ]; then
  PY="${ROOT}/.venv/bin/python"
else
  PY="python3"
fi

mkdir -p "${ROOT}/docs/assets"

echo ">>> building release netrail-api"
cargo build --release --manifest-path "${ROOT}/src-tauri/Cargo.toml" --bin netrail-api --no-default-features

"${PY}" "${ROOT}/scripts/bench/bench.py" --stack rust --out "${ROOT}/docs/assets/bench-rust.json"
"${PY}" "${ROOT}/scripts/bench/bench.py" --stack python --out "${ROOT}/docs/assets/bench-python.json"

echo ">>> report..."
"${PY}" "${ROOT}/scripts/bench/report.py"
