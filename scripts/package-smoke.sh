#!/usr/bin/env bash
# Smoke-test a built netrail-api binary (or PATH netrail-api).
# Usage: scripts/package-smoke.sh [path/to/netrail-api]
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
BIN="${1:-$ROOT/src-tauri/target/release/netrail-api}"
if [[ ! -x "$BIN" ]]; then
  BIN="${1:-$ROOT/dist/release/netrail-api}"
fi
if [[ ! -x "$BIN" ]]; then
  echo "error: netrail-api not found or not executable: $BIN" >&2
  echo "build: cargo build --release --manifest-path src-tauri/Cargo.toml --bin netrail-api --no-default-features" >&2
  exit 1
fi

export NETRAIL_AUTO_OPEN=false
export NETRAIL_HISTORY_ENCRYPT=false
export NETRAIL_RATE_LIMIT=0
# Prefer repo static assets when testing a raw binary outside install paths
export NETRAIL_STATIC_DIR="${NETRAIL_STATIC_DIR:-$ROOT/netrail/static}"

LOG="$(mktemp)"
PID_FILE="$(mktemp)"
cleanup() {
  if [[ -f "$PID_FILE" ]]; then
    kill "$(cat "$PID_FILE")" 2>/dev/null || true
    wait "$(cat "$PID_FILE")" 2>/dev/null || true
    rm -f "$PID_FILE"
  fi
  rm -f "$LOG"
}
trap cleanup EXIT

"$BIN" --api-only >"$LOG" 2>&1 &
echo $! >"$PID_FILE"
sleep 1.5

echo "== health =="
health="$(curl -sf http://127.0.0.1:7421/api/health)"
echo "$health" | python3 -c "import sys,json; h=json.load(sys.stdin); assert h.get('status')=='ok'; assert h.get('version'); print('version', h['version'], 'ok')"

echo "== UI assets =="
code="$(curl -sf -o /tmp/nr-index.html -w '%{http_code}' http://127.0.0.1:7421/)"
test "$code" = "200"
grep -q 'search-form' /tmp/nr-index.html
code="$(curl -sf -o /tmp/nr-app.js -w '%{http_code}' http://127.0.0.1:7421/static/app.js)"
test "$code" = "200"
echo "index+app.js 200"

echo "== open-url blocks =="
curl -sf -X POST http://127.0.0.1:7421/api/open \
  -H 'Content-Type: application/json' \
  -d '{"url":"http://127.0.0.1/"}' && { echo "expected localhost block"; exit 1; } || true
body="$(curl -s -X POST http://127.0.0.1:7421/api/open \
  -H 'Content-Type: application/json' \
  -d '{"url":"http://127.0.0.1/"}')"
echo "$body" | python3 -c "import sys,json; j=json.load(sys.stdin); assert j.get('code')=='OPEN_URL_LOCALHOST'; print(j['code'])"

echo "SMOKE OK: $BIN"
