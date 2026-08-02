#!/usr/bin/env bash
# Sprint 3 (Resource Stability) — load + slope analysis, one command per stack.
#
#   bash scripts/load-10k.sh rust     # Rust netrail-api (default)
#   bash scripts/load-10k.sh python   # Python FastAPI/uvicorn
#
# Writes docs/assets/sprint3-<stack>.{csv,svg} and docs/sprint3-slope.md.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
STACK="${1:-both}"
[ "${STACK}" = "rust" ] || [ "${STACK}" = "python" ] || [ "${STACK}" = "both" ] || {
  echo "usage: $0 [rust|python|both]" >&2
  exit 2
}

if [ -x "${ROOT}/.venv/bin/python" ]; then
  PY="${ROOT}/.venv/bin/python"
else
  PY="python3"
fi

mkdir -p "${ROOT}/docs/assets"

if [ "${STACK}" = "rust" ] || [ "${STACK}" = "both" ]; then
  echo ">>> building release netrail-api"
  cargo build --release --manifest-path "${ROOT}/src-tauri/Cargo.toml" --bin netrail-api --no-default-features
fi

[ "${STACK}" = "both" ] && STACK="rust python"
rc=0
for S in ${STACK}; do
  case "${S}" in
    rust) LABEL="Rust (Axum)";;
    python) LABEL="Python (FastAPI/uvicorn)";;
  esac
  CSV="${ROOT}/docs/assets/sprint3-${S}.csv"
  echo ">>> running load (10k sequential + 1k concurrent) for ${S}..."
  if ! "${PY}" "${ROOT}/scripts/load/run.py" --stack "${S}" --out "${CSV}"; then
    echo "!!! load run for ${S} did not reach 100% completeness" >&2
    rc=1
  fi
  echo ">>> slope analysis..."
  "${PY}" "${ROOT}/scripts/load/slope.py" --csv "${CSV}" --label "${LABEL}"
done
exit "${rc}"
