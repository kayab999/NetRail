#!/usr/bin/env bash
# Dual-stack security/contract probes: Python TestClient vectors + optional live Rust binary.
# Usage:
#   bash scripts/parity-api-smoke.sh
#   bash scripts/parity-api-smoke.sh /path/to/netrail-api
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

export NETRAIL_RATE_LIMIT=0
export NETRAIL_HISTORY_ENCRYPT=false
export NETRAIL_AUTO_OPEN=false
# Dry-run open mode: `/api/open` allow vectors report success without
# discovering or spawning a browser, so the harness is headless-safe
# (no fake-browser PATH needed).
export NETRAIL_NO_OPEN=1
unset NETRAIL_API_TOKEN || true

echo "== Python golden security probes (pytest) =="
if [[ -x "$ROOT/.venv/bin/python" ]]; then
  PY="$ROOT/.venv/bin/python"
else
  PY=python3
fi
# Isolate the pytest section too: netrail.config now resolves $HOME lazily,
# so no test may read or write the developer's real ~/.config/netrail.
export HOME="$(mktemp -d)"
"$PY" -m pytest tests/test_url_policy.py tests/test_security.py tests/test_api.py -q --tb=line

BIN="${1:-$ROOT/src-tauri/target/release/netrail-api}"
if [[ ! -x "$BIN" ]]; then
  BIN="$ROOT/src-tauri/target/debug/netrail-api"
fi
if [[ ! -x "$BIN" ]]; then
  echo "note: netrail-api binary not found — skipping live Rust parity probes"
  echo "PARITY SMOKE OK (Python only)"
  exit 0
fi

export NETRAIL_STATIC_DIR="${NETRAIL_STATIC_DIR:-$ROOT/netrail/static}"
EXPECTED_VERSION="$("$PY" -c "import json; print(json.load(open('package.json'))['version'])")"

# Isolate the live binary from real user state: settings writes, history DB
# (SharedStore opens at startup and runs the TTL purge), and audit log.
export XDG_CONFIG_HOME="$(mktemp -d)"
export NETRAIL_DB_PATH="$(mktemp -d)/netrail.db"
unset NETRAIL_AUDIT_LOG_PATH || true
unset NETRAIL_AUDIT_LOG || true

if curl -sf http://127.0.0.1:7421/api/health >/dev/null 2>&1; then
  echo "error: port 7421 already in use" >&2
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

"$BIN" >"$LOG" 2>&1 &
echo $! >"$PID_FILE"
ready=0
for _ in $(seq 1 40); do
  if curl -sf http://127.0.0.1:7421/api/health >/dev/null 2>&1; then
    ready=1
    break
  fi
  if ! kill -0 "$(cat "$PID_FILE")" 2>/dev/null; then
    echo "error: netrail-api exited early" >&2
    cat "$LOG" >&2 || true
    exit 1
  fi
  sleep 0.25
done
[[ "$ready" -eq 1 ]] || { echo "timeout health" >&2; cat "$LOG" >&2; exit 1; }

probe() {
  local method="$1" path="$2" body="${3:-}" expect_code="$4" expect_status="$5"
  local args=(-s -o /tmp/nr-parity.json -w '%{http_code}' -X "$method" "http://127.0.0.1:7421$path")
  if [[ -n "$body" ]]; then
    args+=(-H 'Content-Type: application/json' -d "$body")
  fi
  local status
  status="$(curl "${args[@]}")"
  "$PY" -c "
import json
status=int('$status')
with open('/tmp/nr-parity.json') as f:
    j=json.load(f)
assert status == int('$expect_status'), (status, j)
assert j.get('code') == '$expect_code', j
print('$method', '$path', j['code'], status)
"
}

echo "== Rust live parity probes (fixture-driven) =="
health="$(curl -sf http://127.0.0.1:7421/api/health)"
echo "$health" | "$PY" -c "
import sys, json
h=json.load(sys.stdin)
assert h.get('status')=='ok'
assert h.get('version')=='$EXPECTED_VERSION', h
assert h.get('api_contract')=='1.4', h
assert 'mutate_per_minute' in h.get('rate_limit', {}), h
# A-06: canonical cipher-state field must be present and consistent with the
# /api/health flags (this run boots with NETRAIL_HISTORY_ENCRYPT=false).
hist = h['history']
state = hist['encryption_state']
if hist['encrypt_requested'] and hist['encryption_active']:
    expected = 'encrypted'
elif hist['encrypt_requested']:
    expected = 'degraded'
else:
    expected = 'plaintext'
assert state == expected, (state, expected)
assert state == 'plaintext', ('smoke boots plaintext', state)
print('health ok', h['version'], '| cipher_state', state)
"

# CSP parity (A-18): the live Rust server must serve the exact CSP the Python
# stack serves (incl. the inline splash failsafe script hash) and must never
# restore `upgrade-insecure-requests` (WebKitGTK white-screen pin).
"$PY" - <<'PYEOF'
import sys, urllib.request
sys.path.insert(0, ".")
import netrail.main as main
with urllib.request.urlopen("http://127.0.0.1:7421/") as resp:
    rust_csp = resp.headers.get("Content-Security-Policy", "")
assert rust_csp == main.CSP, (
    "CSP parity divergence (A-18):\n"
    f"rust={rust_csp!r}\npy  ={main.CSP!r}"
)
assert "upgrade-insecure-requests" not in rust_csp
print("CSP parity OK (failsafe hash present, no upgrade-insecure-requests)")
PYEOF

probe POST /api/search '{"query":"","mode":"web"}' QUERY_INVALID 400

# Typed-error contract on malformed bodies (A1): both stacks must return
# {code, detail, status} — never plain-text extractor output.
probe POST /api/search '{}' QUERY_INVALID 400
probe POST /api/search '{"query":123}' QUERY_INVALID 400
probe POST /api/search '{bad' REQUEST_INVALID 400
probe POST /api/search '{"query":"x","max_results":999}' CONFIG_MAX_RESULTS 400
probe POST /api/open '{}' OPEN_URL_INVALID 400

# Every open_url vector in the shared golden fixture must behave identically
# on the live Rust binary. Python parity is covered by test_url_policy.py.
"$PY" - "$ROOT/tests/fixtures/url_policy.json" <<'PYEOF'
import json, subprocess, sys, urllib.request

fixture = json.load(open(sys.argv[1]))
vecs = [c for c in fixture["open_url"]]
fails = []
for c in vecs:
    url = c["url"]
    req = urllib.request.Request(
        "http://127.0.0.1:7421/api/open",
        data=json.dumps({"url": url}).encode(),
        headers={"Content-Type": "application/json"},
        method="POST",
    )
    try:
        with urllib.request.urlopen(req) as resp:
            status, body = resp.status, json.loads(resp.read())
    except urllib.error.HTTPError as e:
        status, body = e.code, json.loads(e.read())
    expect = "allow" if c["expect"] == "allow" else "block"
    got = "allow" if status == 200 else f"block:{body.get('code')}"
    if expect == "block":
        ok = status != 200 and (not c.get("code") or body.get("code") == c["code"])
    else:
        ok = status == 200
    tag = "ok " if ok else "FAIL"
    print(f"{tag} {c['id']:42s} {got}")
    if not ok:
        fails.append((c["id"], url, status, body))
if fails:
    print("FIXTURE-DRIVEN PARITY FAILURES:")
    for f in fails:
        print("  ", f)
    sys.exit(1)
print(f"fixture open_url vectors: {len(vecs)} passed")
PYEOF

# Backend-URL vectors must behave identically on the live Rust binary. They are
# exercised through the settings update path (validate_settings) since there is
# no dedicated backend-URL endpoint. Strict-mode vectors are covered by the
# Rust/Python unit tests, so only non-strict ones are probed live.
"$PY" - "$ROOT/tests/fixtures/url_policy.json" <<'PYEOF'
import json, sys, urllib.request

base = "http://127.0.0.1:7421"
fixture = json.load(open(sys.argv[1]))

def call(method, path, data=None):
    req = urllib.request.Request(
        base + path,
        data=json.dumps(data).encode() if data is not None else None,
        headers={"Content-Type": "application/json"},
        method=method,
    )
    try:
        with urllib.request.urlopen(req, timeout=5) as resp:
            return resp.status, json.loads(resp.read())
    except urllib.error.HTTPError as e:
        return e.code, json.loads(e.read())

settings = {
    "search_strategy": "fanout",
    "backend_order": ["ddgs"],
    "ddgs_enabled": True,
    "searxng_url": None,
    "brave_enabled": False,
    "private_mode": False,
    "history_enabled": True,
    "history_encrypt": False,
    "history_ttl_days": 90,
    "max_results": 25,
}
fails = []
for c in fixture["backend_url"]:
    # Strict-mode vectors are covered by unit tests; resolved_ips vectors
    # need a resolver injection the live API cannot provide (A-05 fetch-time
    # guard is exercised by the gold tests on both stacks).
    if c.get("strict") or "resolved_ips" in c:
        continue
    body = dict(settings, searxng_url=c["url"])
    status, resp = call("PUT", "/api/settings", body)
    if c["expect"] == "allow":
        ok = status == 200
    else:
        ok = status == 400 and (not c.get("code") or resp.get("code") == c["code"])
    tag = "ok " if ok else "FAIL"
    print(f"{tag} {c['id']:44s} {status} {resp.get('code')}")
    if not ok:
        fails.append((c["id"], c["url"], status, resp))
if fails:
    print("BACKEND LIVE PARITY FAILURES:")
    for f in fails:
        print("  ", f)
    sys.exit(1)
print(
    "fixture backend_url live vectors: "
    f"{sum(1 for c in fixture['backend_url'] if not c.get('strict') and 'resolved_ips' not in c)} passed"
)
PYEOF

probe GET /api/docs/nope '' DOC_NOT_FOUND 404

# Settings concurrency contract (A6): ETag on GET, 409 SETTINGS_CONFLICT on a
# stale If-Match PUT, 200 on a fresh one. Python side covered in test_api.py.
"$PY" - <<'PYEOF'
import json, urllib.request

base = "http://127.0.0.1:7421"

def call(method, path, data=None, headers=None):
    req = urllib.request.Request(
        base + path,
        data=json.dumps(data).encode() if data is not None else None,
        headers={"Content-Type": "application/json", **(headers or {})},
        method=method,
    )
    try:
        with urllib.request.urlopen(req) as resp:
            return resp.status, json.loads(resp.read()), resp.headers.get("ETag")
    except urllib.error.HTTPError as e:
        return e.code, json.loads(e.read()), e.headers.get("ETag")

settings = {
    "search_strategy": "fanout",
    "backend_order": ["ddgs"],
    "ddgs_enabled": True,
    "searxng_url": None,
    "brave_enabled": False,
    "private_mode": False,
    "history_enabled": True,
    "history_encrypt": False,
    "history_ttl_days": 90,
    "max_results": 25,
}
status, _, etag = call("GET", "/api/settings")
assert status == 200 and etag and etag.startswith('"'), (status, etag)
status, body, _ = call("PUT", "/api/settings", settings, {"If-Match": '"stale"'})
assert status == 409 and body.get("code") == "SETTINGS_CONFLICT", (status, body)
status, _, new_etag = call("PUT", "/api/settings", settings, {"If-Match": etag})
assert status == 200 and new_etag, (status, new_etag)
status, body, _ = call("PUT", "/api/settings", settings)
assert status == 200, (status, body)
print("settings ETag/If-Match probes: passed")
PYEOF

echo "== Browser-discovery live parity (QA-09 T2) =="
PYTHONPATH="$ROOT" "$PY" - <<'PYEOF'
import json, urllib.request

base = "http://127.0.0.1:7421"
with urllib.request.urlopen(base + "/api/browsers") as resp:
    rust = json.loads(resp.read())

from netrail.browsers import discover_browsers

py = [
    {
        "id": b.id,
        "name": b.name,
        "executable": b.executable,
        "supports_private": b.private_flag is not None,
    }
    for b in discover_browsers()
]
rust = sorted(rust, key=lambda b: b["name"].lower())
py = sorted(py, key=lambda b: b["name"].lower())
assert rust == py, ("browser-discovery parity divergence (QA-09):\n"
                    f"rust={json.dumps(rust, indent=2)}\n"
                    f"py  ={json.dumps(py, indent=2)}")
print(f"browser-discovery parity OK ({len(rust)} browsers)")
PYEOF

echo "== Cipher-state directivity live probe (A-11) =="
DB_KEY="$("$PY" -c 'from cryptography.fernet import Fernet; print(Fernet.generate_key().decode())')"
kill -TERM "$(cat "$PID_FILE")" 2>/dev/null || true
wait "$(cat "$PID_FILE")" 2>/dev/null || true
# Reboot WITHOUT the NETRAIL_HISTORY_ENCRYPT pin so PUT /api/settings can flip
# the effective cipher mode live (the store must rebind on every access).
NETRAIL_DB_KEY="$DB_KEY" env -u NETRAIL_HISTORY_ENCRYPT "$BIN" >>"$LOG" 2>&1 &
echo $! >"$PID_FILE"
ready=0
for _ in $(seq 1 40); do
  if curl -sf http://127.0.0.1:7421/api/health >/dev/null 2>&1; then
    ready=1
    break
  fi
  sleep 0.25
done
[[ "$ready" -eq 1 ]] || { echo "timeout health (directivity boot)" >&2; exit 1; }

"$PY" - <<'PYEOF'
import json, urllib.request

base = "http://127.0.0.1:7421"


def call(method, path, data=None):
    req = urllib.request.Request(
        base + path,
        data=json.dumps(data).encode() if data is not None else None,
        headers={"Content-Type": "application/json"},
        method=method,
    )
    try:
        with urllib.request.urlopen(req, timeout=5) as resp:
            return resp.status, json.loads(resp.read())
    except urllib.error.HTTPError as e:
        return e.code, json.loads(e.read())


settings = {
    "search_strategy": "fanout",
    "backend_order": ["ddgs"],
    "ddgs_enabled": True,
    "searxng_url": None,
    "brave_enabled": False,
    "private_mode": False,
    "history_enabled": True,
    "history_encrypt": True,
    "history_ttl_days": 90,
    "max_results": 25,
}


def state():
    status, health = call("GET", "/api/health")
    assert status == 200, (status, health)
    return health["history"]


# The binary boots unencrypted-requested only via PUT below, so establish the
# plaintext mode first, then drive encrypted -> plaintext -> disabled -> enabled.
assert call("PUT", "/api/settings", settings)[0] == 200
assert state()["encrypt_requested"] is True
assert state()["encryption_state"] == "encrypted"

settings["history_encrypt"] = False
assert call("PUT", "/api/settings", settings)[0] == 200
assert state()["encryption_state"] == "plaintext", state()

settings["history_encrypt"] = True
assert call("PUT", "/api/settings", settings)[0] == 200
assert state()["encryption_state"] == "encrypted", state()

settings["history_enabled"] = False
assert call("PUT", "/api/settings", settings)[0] == 200
assert state()["enabled"] is False, state()

settings["history_enabled"] = True
assert call("PUT", "/api/settings", settings)[0] == 200
assert state()["encryption_state"] == "encrypted", state()
print("cipher-state directivity live probe: passed")
PYEOF
kill -TERM "$(cat "$PID_FILE")" 2>/dev/null || true
wait "$(cat "$PID_FILE")" 2>/dev/null || true

echo "PARITY SMOKE OK (Python + Rust $BIN)"
