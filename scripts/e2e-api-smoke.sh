#!/usr/bin/env bash
# Product E2E over HTTP + static UI (no Tauri webview driver).
# Usage: scripts/e2e-api-smoke.sh [path/to/netrail-api]
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

EXPECTED_VERSION="$(python3 -c "import json; print(json.load(open('$ROOT/package.json'))['version'])")"

export NETRAIL_AUTO_OPEN=false
export NETRAIL_HISTORY_ENCRYPT=false
export NETRAIL_RATE_LIMIT=0
export NETRAIL_STATIC_DIR="${NETRAIL_STATIC_DIR:-$ROOT/netrail/static}"

if curl -sf http://127.0.0.1:7421/api/health >/dev/null 2>&1; then
  echo "error: port 7421 already in use (stop other NetRail instances first)" >&2
  exit 1
fi

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
ready=0
for _ in $(seq 1 40); do
  if curl -sf http://127.0.0.1:7421/api/health >/dev/null 2>&1; then
    ready=1
    break
  fi
  # bind failure → process exits early
  if ! kill -0 "$(cat "$PID_FILE")" 2>/dev/null; then
    echo "error: netrail-api exited before becoming ready" >&2
    cat "$LOG" >&2 || true
    exit 1
  fi
  sleep 0.25
done
if [[ "$ready" -ne 1 ]]; then
  echo "error: timeout waiting for /api/health" >&2
  cat "$LOG" >&2 || true
  exit 1
fi

echo "== health =="
health="$(curl -sf http://127.0.0.1:7421/api/health)"
echo "$health" | python3 -c "
import sys, json
h = json.load(sys.stdin)
assert h.get('status') == 'ok', h
assert h.get('version') == '$EXPECTED_VERSION', (h.get('version'), '$EXPECTED_VERSION')
assert h.get('telemetry') == 'none'
assert 'search_recovery' in h
assert 'rate_limit' in h
print('version', h['version'], 'ok')
"

echo "== UI assets =="
code="$(curl -sf -o /tmp/nr-e2e-index.html -w '%{http_code}' http://127.0.0.1:7421/)"
test "$code" = "200"
grep -q 'search-form' /tmp/nr-e2e-index.html
code="$(curl -sf -o /tmp/nr-e2e-app.js -w '%{http_code}' http://127.0.0.1:7421/static/app.js)"
test "$code" = "200"
echo "index+app.js 200"

echo "== search empty =="
body="$(curl -s -X POST http://127.0.0.1:7421/api/search \
  -H 'Content-Type: application/json' \
  -d '{"query":"","mode":"web"}')"
echo "$body" | python3 -c "
import sys, json
j = json.load(sys.stdin)
assert j.get('code') == 'QUERY_INVALID', j
assert j.get('status') == 400
print(j['code'])
"

echo "== search product path =="
# Allow network: 200 with results and/or errors, or 502 FANOUT_TOTAL_FAILURE.
# Forbid silent empty: 200 + empty results + empty errors.
http_code="$(curl -s -o /tmp/nr-e2e-search.json -w '%{http_code}' \
  -X POST http://127.0.0.1:7421/api/search \
  -H 'Content-Type: application/json' \
  -d '{"query":"netrail e2e","mode":"web","max_results":5}')"
python3 -c "
import json
code = int('$http_code')
with open('/tmp/nr-e2e-search.json') as f:
    j = json.load(f)
if code == 200:
    results = j.get('results') or []
    errors = j.get('errors') or []
    assert results or errors, f'silent empty search forbidden: {j!r}'
    print('search 200 results=', len(results), 'errors=', len(errors))
elif code == 502:
    assert j.get('code') == 'FANOUT_TOTAL_FAILURE', j
    print('search 502 FANOUT_TOTAL_FAILURE')
else:
    raise SystemExit(f'unexpected search status {code}: {j!r}')
"

echo "== open-url blocks =="
body="$(curl -s -X POST http://127.0.0.1:7421/api/open \
  -H 'Content-Type: application/json' \
  -d '{"url":"http://127.0.0.1/"}')"
echo "$body" | python3 -c "
import sys, json
j = json.load(sys.stdin)
assert j.get('code') == 'OPEN_URL_LOCALHOST', j
print(j['code'])
"
body="$(curl -s -X POST http://127.0.0.1:7421/api/open \
  -H 'Content-Type: application/json' \
  -d '{"url":"http://192.168.1.1/"}')"
echo "$body" | python3 -c "
import sys, json
j = json.load(sys.stdin)
assert j.get('code') == 'OPEN_URL_PRIVATE', j
print(j['code'])
"

echo "E2E API SMOKE OK: $BIN"
