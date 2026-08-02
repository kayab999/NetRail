#!/usr/bin/env bash
# Webview E2E (audit matrix #9): drives the real Tauri/WebKitGTK webview via
# tauri-driver + WebKitWebDriver + Selenium to cover the desktop eval bridges
# (focus-search pipeline + docs bridge) that replaced the dead __TAURI__ emits.
#
# Requirements:
#   - tauri-driver on PATH (cargo install tauri-driver --locked)
#   - WebKitWebDriver on PATH, or WK_DRIVER=/path/to/WebKitWebDriver
#     (Debian/Ubuntu: webkit2gtk-driver; Ubuntu 25.04+: webkitgtk-webdriver)
#   - xdotool (optional — only for the global-shortcut pipeline check)
#   - python venv with selenium
#   - a running X/Wayland session; DISPLAY set
#   - port 7421 free — stop any running NetRail first (the single-instance
#     plugin would hand focus to the existing instance instead of the test app)
#
# The app window opens briefly on your desktop during the run.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

APP="${1:-$ROOT/src-tauri/target/debug/netrail}"
if [[ ! -x "$APP" ]]; then
  echo "error: desktop binary not found: $APP" >&2
  echo "build: cargo build --manifest-path src-tauri/Cargo.toml --bin netrail" >&2
  exit 1
fi

if ! command -v tauri-driver >/dev/null 2>&1; then
  echo "error: tauri-driver not found (cargo install tauri-driver --locked)" >&2
  exit 1
fi

WK_DRIVER="${WK_DRIVER:-$(command -v WebKitWebDriver || true)}"
if [[ -z "$WK_DRIVER" || ! -x "$WK_DRIVER" ]]; then
  echo "error: WebKitWebDriver not found — set WK_DRIVER=/path/to/WebKitWebDriver" >&2
  echo "       (Debian/Ubuntu: sudo apt install webkit2gtk-driver)" >&2
  exit 1
fi

PY="${PY:-$ROOT/.venv/bin/python}"
if [[ ! -x "$PY" ]]; then
  echo "error: python venv not found at $PY" >&2
  exit 1
fi
"$PY" -c "import selenium" 2>/dev/null || {
  echo "error: selenium not installed in $PY (pip install selenium)" >&2
  exit 1
}

# Isolate from real user state: settings, history DB, audit, and the WebKit
# HTTP cache (XDG_DATA_HOME) — otherwise the webview serves a stale cached
# app.js and the eval bridges under test are missing.
export XDG_CONFIG_HOME="$(mktemp -d)"
export XDG_DATA_HOME="$(mktemp -d)"
export XDG_CACHE_HOME="$(mktemp -d)"
export NETRAIL_DB_PATH="$(mktemp -d)/netrail.db"
unset NETRAIL_AUDIT_LOG_PATH NETRAIL_AUDIT_LOG NETRAIL_API_TOKEN || true

if curl -sf http://127.0.0.1:7421/api/health >/dev/null 2>&1; then
  echo "error: port 7421 already in use — stop the running NetRail instance first" >&2
  exit 1
fi
if curl -sf http://127.0.0.1:4444/status >/dev/null 2>&1; then
  echo "error: port 4444 already in use — stop the other tauri-driver" >&2
  exit 1
fi

LOG="$(mktemp)"
DRIVER_PID=""
cleanup() {
  [[ -n "$DRIVER_PID" ]] && kill "$DRIVER_PID" 2>/dev/null || true
  wait "$DRIVER_PID" 2>/dev/null || true
  rm -f "$LOG"
}
trap cleanup EXIT

tauri-driver --native-driver "$WK_DRIVER" >"$LOG" 2>&1 &
DRIVER_PID=$!
ready=0
for _ in $(seq 1 60); do
  if curl -sf http://127.0.0.1:4444/status >/dev/null 2>&1; then
    ready=1
    break
  fi
  if ! kill -0 "$DRIVER_PID" 2>/dev/null; then
    echo "error: tauri-driver exited early" >&2
    cat "$LOG" >&2 || true
    exit 1
  fi
  sleep 0.25
done
[[ "$ready" -eq 1 ]] || { echo "error: tauri-driver not ready on :4444" >&2; cat "$LOG" >&2; exit 1; }

"$PY" tests/webview_e2e.py "$APP"
