#!/usr/bin/env bash
# Generates the release SBOM.txt — a lightweight SPDX-ish component inventory:
# Rust packages from Cargo.lock + Python requirements + version header
# (no external syft dependency; keep in sync with build.rs's embedded subset).
#
# Usage: bash scripts/generate-sbom.sh [OUTPUT_PATH]
#   (default: dist/release/SBOM.txt)
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT="${1:-${ROOT}/dist/release/SBOM.txt}"

mkdir -p "$(dirname "${OUT}")"

{
  echo "# NetRail SBOM (component inventory)"
  # Deterministic provenance so a release SBOM is reproducible byte-for-byte
  # (the release job generates it twice and `cmp`s them). Prefer the
  # reproducible-builds convention, else the checkout commit, else omit.
  if [[ -n "${SOURCE_DATE_EPOCH:-}" ]]; then
    echo "generated=$(date -u -d "@${SOURCE_DATE_EPOCH}" +%Y-%m-%dT%H:%M:%SZ 2>/dev/null || date -u -r "${SOURCE_DATE_EPOCH}" +%Y-%m-%dT%H:%M:%SZ 2>/dev/null || echo "SOURCE_DATE_EPOCH=${SOURCE_DATE_EPOCH}")"
  elif commit="$(git -C "${ROOT}" rev-parse HEAD 2>/dev/null)"; then
    echo "commit=${commit}"
  fi
  echo "version=$(${PYTHON:-python3} -c "import json; print(json.load(open('${ROOT}/package.json'))['version'])")"
  echo ""
  echo "## Rust (Cargo.lock packages)"
  # Skip the top-level lockfile `version =` line (no package name precedes it);
  # reset n after each package so a version can't pair with a stale name.
  awk '/^name = /{n=$3} /^version = / && n != "" {print n "@" $3; n=""}' \
    "${ROOT}/src-tauri/Cargo.lock" | tr -d '"' | sort -u
  echo ""
  echo "## Python (requirements.txt)"
  cat "${ROOT}/requirements.txt"
} > "${OUT}"

echo "SBOM written to ${OUT}"
