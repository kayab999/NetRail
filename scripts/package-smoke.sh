#!/usr/bin/env bash
# Package smoke = full product E2E over the headless binary.
# Usage: scripts/package-smoke.sh [path/to/netrail-api]
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
exec bash "$ROOT/scripts/e2e-api-smoke.sh" "$@"
