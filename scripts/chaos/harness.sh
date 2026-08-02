#!/usr/bin/env bash
# NetRail Sprint 2 chaos harness — one-command reproducibility for the
# fault-injection scenarios in src-tauri/tests/chaos_{db,process}.rs and
# tests/test_chaos.py, plus live fault-injection drivers against a running
# netrail-api binary.
#
# Usage:
#   bash scripts/chaos/harness.sh            # run the full automated chaos gate
#   bash scripts/chaos/harness.sh live-busy  # lock SQLite, observe typed 500, recover
#   bash scripts/chaos/harness.sh live-kill  # SIGKILL mid-session, verify WAL survives
#
# Requirements: cargo, a Python venv (.venv) with pytest, curl, sqlite3 CLI.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
PORT="${NETRAIL_CHAOS_PORT:-7421}"
BASE="http://127.0.0.1:${PORT}"
BIN="${ROOT}/src-tauri/target/release/netrail-api"
VENV_PY="${ROOT}/.venv/bin/python"

log() { printf '\033[1;36m[chaos]\033[0m %s\n' "$*"; }
fail() { printf '\033[1;31m[chaos FAIL]\033[0m %s\n' "$*"; exit 1; }

wait_healthy() {
  for _ in $(seq 1 50); do
    if curl -fsS "${BASE}/api/health" >/dev/null 2>&1; then return 0; fi
    sleep 0.2
  done
  fail "netrail-api never became healthy on port ${PORT}"
}

SERVER_PID=""
purge_stale() {
  if curl -fsS "${BASE}/api/health" >/dev/null 2>&1; then
    log "stale server on port ${PORT}; terminating..."
    pkill -f 'netrail-api' 2>/dev/null || true
    sleep 0.5
  fi
}
cleanup() { [ -n "${SERVER_PID}" ] && kill "${SERVER_PID}" 2>/dev/null || true; }
trap cleanup EXIT

start_server() {
  purge_stale
  NETRAIL_DB_PATH="$1" NETRAIL_DB_KEY="$2" "${BIN}" >/dev/null 2>&1 &
  SERVER_PID=$!
  wait_healthy
}

api() { curl -fsS -H 'Content-Type: application/json' "$@"; }

run_gate() {
  log "Rust chaos tests (chaos_db, chaos_process, audit rotation)..."
  (cd "${ROOT}/src-tauri" && cargo test --test chaos_db --test chaos_process --lib audit::tests --quiet)
  log "Python chaos tests (test_chaos.py)..."
  "${VENV_PY}" -m pytest "${ROOT}/tests/test_chaos.py" -q
  log "chaos gate OK"
}

live_busy() {
  local db
  db="$(mktemp -d)/netrail.db"
  local key
  key="$("${VENV_PY}" -c 'from cryptography.fernet import Fernet; print(Fernet.generate_key().decode())')"
  log "starting netrail-api (NETRAIL_DB_PATH=${db})..."
  start_server "${db}" "${key}"
  api -X POST "${BASE}/api/collections" -d '{"name":"seed"}' >/dev/null || fail "seed write"

  # Hold the SQLite write lock from a second connection for ~30s.
  "${VENV_PY}" -c "
import sqlite3, sys, time
conn = sqlite3.connect(sys.argv[1])
conn.execute('BEGIN IMMEDIATE')
conn.execute(\"INSERT INTO collections (name) VALUES ('lock-holder')\")
print('write lock held', flush=True)
time.sleep(30)
" "${db}" &
  local lockpid=$!
  sleep 0.5
  log "write lock held; mutation must now return a typed 500 DB_ERROR..."
  local out status
  out="$(mktemp)"
  status="$(curl -sS -o "${out}" -w '%{http_code}' -H 'Content-Type: application/json' -X POST "${BASE}/api/collections" -d '{"name":"blocked"}')"
  local body
  body="$(cat "${out}"; rm -f "${out}")"
  [ "${status}" = "500" ] || fail "expected 500, got ${status} (${body})"
  echo "${body}" | grep -q '"code":"DB_ERROR"' || fail "expected typed DB_ERROR, got: ${body}"

  kill "${lockpid}" 2>/dev/null || true
  wait "${lockpid}" 2>/dev/null || true
  log "lock released; mutation must recover without restart..."
  api -X POST "${BASE}/api/collections" -d '{"name":"blocked"}' | grep -q '"name":"blocked"' || fail "recovery failed"
  log "live-busy OK"
}

live_kill() {
  local db
  db="$(mktemp -d)/netrail.db"
  local key
  key="$("${VENV_PY}" -c 'from cryptography.fernet import Fernet; print(Fernet.generate_key().decode())')"
  log "starting netrail-api (NETRAIL_DB_PATH=${db})..."
  start_server "${db}" "${key}"
  api -X POST "${BASE}/api/collections" -d '{"name":"pre"}' >/dev/null || fail "seed write"

  log "SIGKILL mid-session..."
  kill -9 "${SERVER_PID}"
  wait "${SERVER_PID}" 2>/dev/null || true
  SERVER_PID=""

  log "restart on the same database file..."
  start_server "${db}" "${key}"
  api "${BASE}/api/collections" | grep -q '"pre"' || fail "WAL data lost across SIGKILL"
  log "live-kill OK"
}

case "${1:-}" in
  live-busy) live_busy ;;
  live-kill) live_kill ;;
  "" | gate) run_gate ;;
  *) echo "usage: $0 [gate|live-busy|live-kill]" >&2; exit 2 ;;
esac
