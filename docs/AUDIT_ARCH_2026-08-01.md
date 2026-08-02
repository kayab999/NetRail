# NetRail — Architecture-Level Technical Audit (Enterprise Grade)

| Field | Value |
|-------|--------|
| **Date** | 2026-08-01 |
| **Tree** | 1.4.1 (`main` HEAD `72f2621`, clean, in sync with `origin/main`; tag `v1.4.1`) |
| **Method** | Full-source walk (every production module, both stacks, UI, shell, CI), line-level evidence, live runtime probes against the 1.4.1 `netrail-api` release binary, test/CI gate review |
| **Evidence base** | 100 % of Rust production sources (server, security, history, backends, config, error, browsers, search, rate_limit, auth, audit, crypto, docs, http_client, desktop, lib, main, bin), 100 % of Python parity modules (main, security, config, errors, history/store+db, backends/registry+merge, auth, rate_limit, audit, runtime), UI (app.js, index.html, style.css), tauri.conf.json, release.yml/ci.yml; live probes (typed-error contract, token+CSP interaction, auth bypass paths); release CI run `30690333556` verified green through headless-build step |
| **Prior audits** | AUDIT_ENTERPRISE_2026-07-31 (waves 0–5), AUDIT_OPENCODE_ADVERSARIAL_2026-08-01 (N1–N4) — this audit is the first **architecture-level** pass over the code as built |
| **Contract reference** | api_contract `1.4`; typed-error invariant `{code, detail, status}` (docs/API_ERRORS.md) |

**Bottom line:** the codebase is a well-engineered single-user research console with an unusually strong local-security posture (browser-faithful IP parsing, redirect discipline, no-telemetry, typed errors, golden-fixture policy tests, dual-stack parity harness). At enterprise scale it is **not yet an enterprise product**: it has no observability, no graceful shutdown, per-request DB connection churn, an untested typed-error hole on Rust (HTTP 422), a token-mode UI defect (CSP blocks the injected token script), and a long tail of single-user assumptions (per-process rate limits, no multi-client identity, no schema migrations). All findings below are evidence-anchored; severity register at §13.

---

## 1. Architecture map

### 1.1 Runtime topologies (four deployment shapes, one HTTP surface)

```
                    ┌──────────────────────────────────────────────────────────┐
                    │                 loopback HTTP · 127.0.0.1:7421           │
                    │  ┌────────────────────────────────────────────────────┐  │
   Tauri desktop ───┤  │  static UI  (index.html + app.js + style.css)      │  │
   (webview + tray) │  │  JSON API   (/api/*)                               │  │
                    │  └────────────────────────────────────────────────────┘  │
   netrail-api ─────┤  server (Rust/axum) · middlewares: auth → headers        │
   (headless bin)   │  AppState { http_client, settings_fn, rate_limiter }     │
                    └──────────────────────────────────────────────────────────┘
   Python/FastAPI ───┤  parity surface (Docker, Flatpak, install.sh)           │
                     └──── egress: DDGS · SearXNG · Brave · Wikipedia (fanout) │
```

- **Entry points:** Tauri desktop (`src-tauri/src/main.rs` → `desktop.rs` spawns the API server as a background task, `desktop.rs:52-56`); headless `netrail-api` (`src-tauri/src/bin/netrail-api.rs`, `--no-default-features`); Python `uvicorn netrail.main:app` (`netrail/main.py:514-521`).
- **Single bind:** `127.0.0.1:7421` (config.rs:8-9) — loopback-only by construction; no TLS (correct for loopback single-user).
- **Client model:** one UI (vanilla JS, no framework, no build step) shared by every topology; UI is static assets served by the same server (`ServeDir` nest + index handler with token injection).

### 1.2 Module inventory (all production files, as built)

| Module | Lines | Responsibility | Key anchors |
|--------|-------|----------------|-------------|
| `server/mod.rs` | 637 | Router, middlewares, handlers, error mapping | routes mod.rs:44-66; auth middleware :111; headers :93; `ApiError` :612-637 |
| `security.rs` | 605 | open/backend URL policy engine, CSP | browser-IP parse :81-145; block :190-266; backend policy :276-376; CSP :386 |
| `history/mod.rs` | 762 | SQLite schema, store, FTS5, TTL, collections | schema :33-92; record :178; list+FTS :294; collections :404-560 |
| `backends/mod.rs` | 515 | Backend enum, availability, fanout+deadline, Wikipedia fallback | fanout :200-356; sovereignty :154-170 |
| `backends/{ddgs,searxng,brave,wikipedia}.rs` | — | Adapters: HTML scrape / JSON API / REST / Wikipedia | DDG bot-challenge :59-67; SearXNG health cache :17-61 |
| `backends/{types,merge,url_resolve}.rs` | — | Result model, dedupe/interleave, DDG unwrap | tracking-param strip merge.rs:6-59 |
| `config.rs` | 360 | Settings struct, env overrides, static-dir resolution | load :112-122; validate :124-155; static_dir :244-291 |
| `error.rs` | 289 | Typed `NetRailError`, status/code mapping | status map :102-129; From impls :185-231 |
| `browsers.rs` | 241 | Desktop-file + known-binary discovery, spawn | flatpak wrap :174-193; LD_PRELOAD strip :175 |
| `search.rs` | 120 | Search orchestration, history enrichment, sovereignty | :8-52 |
| `rate_limit.rs` | 158 | 3 fixed-window counters (search/open/mutate) | :10-15 |
| `auth.rs` | 87 | Optional bearer token, path exemption | :57-65 |
| `audit.rs` | 60 | Optional JSON-lines audit log | :39-56 |
| `crypto.rs` | 78 | Fernet key mgmt (env + keyring), degrade | :8-27 |
| `docs.rs` | 49 | Embedded manual/about, asset path guard | :30-43 |
| `http_client.rs` | 18 | Shared reqwest client, redirects OFF | :9-17 |
| `desktop.rs` | 169 | Tray, menu, global shortcut, focus bridge | :130-149 |
| `netrail/main.py` | 525 | FastAPI parity surface | typed 422 mapping :72-129 |
| `netrail/{security,config,errors,history/store+db,backends/*}.py` | — | Python parity of the above | schema db.py:13-72 |
| `netrail/static/{index,app.js,style.css}` | — | Vanilla-JS UI | state obj app.js:3-19; api() :92 |

### 1.3 Request flows (as coded, verified by reading, not assumptions)

**Search** `POST /api/search`: rate-limit `check_search` → query trim/length (1–500) → mode whitelist → `search::search` → `get_enabled_backends` (structured `backends[]` first, legacy `backend_order` fallback, DDG default) → availability pre-check (`backend_available`, SearXNG `healthz` TTL-cached 60 s) → fanout `join_all` under a hard **20 s deadline** → per-backend outcomes captured as strings in `errors[]` (never fatal) → `merge_fanout` (normalize+dedupe keeping richer snippet, round-robin interleave) or `fallback` strategy → empty-web Wikipedia fallback → sovereignty step → history `record_search` + visit-metadata enrichment → typed JSON. Full stack: `server/mod.rs:323-354`, `backends/mod.rs:200-356`, `search.rs:8-52`.

**Open** `POST /api/open`: rate-limit `check_open` → `validate_open_url` (scheme, embedded creds, DDG `uddg` unwrap depth ≤ 5, then host policy) → browser discovery (`desktop-files` dirs + `which`/`flatpak-spawn` fallback, `browsers.rs:97-155`) → spawn with `LD_PRELOAD` stripped and flatpak host wrap → `record_visit`. Notably **no DNS resolution** at validation time — the browser is the resolver (residual SEC-2026-04, §11).

**History/collections/docs**: SQLite CRUD with FTS5 (`MATCH` on externally-stored `queries_fts`), TTL purge (`datetime('now', '-N days')`), collections upsert + JSON/CSV export, embedded docs with path-traversal guard.

**Settings** `PUT /api/settings`: rate-limit `check_mutate` → `validate_settings` (max_results 1–50, TTL ≤ 3650, strategy whitelist, backend URL policy incl. strict mode) → atomic file write → **state is re-read from disk on every request** (`settings_fn` = `load_settings`, `server/mod.rs:73`) → immediate pickup, no in-memory mutation. Env overrides applied at load (config.rs:172-230; Python config.py:46-99).

### 1.4 Concurrency & state model

- **Rust:** tokio multi-thread runtime; `AppState` holds only immutable/Arc state; no request-global mutable state; per-request SQLite connection (finding A3); `OnceCell<Mutex<Option<HistoryStore>>>` singleton exists but is **write-only dead code** (history/mod.rs:31, 598-604 — never read; `get_store` reopens per call); parking_lot mutexes for rate limiter and SearXNG health cache.
- **Python:** process singleton store (`history/store.py:15-16`), `check_same_thread=False` (db.py:82); fanout via `ThreadPoolExecutor`; **sync work inside `async def` endpoints blocks the event loop** (main.py:366-382, finding A8).
- **Cross-process:** Docker service and desktop instance can run simultaneously on one host; nothing coordinates settings, DB (SQLite locking is the only arbiter), or rate limits (A9).

### 1.5 Error contract

`NetRailError` (Rust enum, error.rs:5-97) ↔ `NetRailError` (Python class, errors.py:4-12): stable codes, mapped HTTP statuses (400/404/429/502/500), JSON body `{code, detail, status}`. Golden fixture `tests/fixtures/url_policy.json` (32 open-url + backend-url vectors) drives Rust unit tests, Python tests, and the live parity harness. **Hole:** axum extractor rejections bypass this contract entirely → plain-text HTTP 422 (finding A1).

---

## 2. Security architecture assessment

### 2.1 What is genuinely strong (verified in code)

1. **Browser-faithful host parsing** — `parse_browser_ipv4` handles decimal integer, hex, octal, short forms (`127.1`), plus IPv4-mapped IPv6 unmap (`effective_ip`); RFC1918/link-local/metadata/broadcast/unspecified all rejected for opens (security.rs:81-176, 190-266). N1–N3 (trailing-dot, strict bypass, malformed IPv6) closed dual-stack in 1.4.1 with 13 new fixture vectors.
2. **Redirect discipline** — backend client `redirect(Policy::none())` (http_client.rs:15) prevents a backend response from bouncing the process onto private/metadata targets; DDG `uddg` unwraps are re-validated recursively (depth cap 5).
3. **Cloud-metadata defenses** — hostnames (`metadata.google.internal`, `metadata`, `instance-data`) + `169.254.169.254` + AWS IMDSv2 IPv6 `fd00:ec2::254` blocked for both open and backend URLs (security.rs:10-15, 184-188, 378-384).
4. **Hardening on responses** — CSP with `frame-ancestors 'none'`, `base-uri 'self'`, `form-action 'self'`, no `unsafe-eval`; `X-Content-Type-Options: nosniff`; `Referrer-Policy: no-referrer` (mod.rs:93-109; security.rs:386).
5. **Secrets hygiene** — API keys env-only, never serialized to settings.json (`api_key_env` stores the *name*, not the key, config.rs:18-20); audit log records hosts, not queries or tokens; no telemetry, no crash reporter, no analytics anywhere.
6. **Browser spawn hardening** — `LD_PRELOAD` stripped (browsers.rs:175); flatpak `--host` wrap; private-mode flags per browser; `Type=`/`NoDisplay` filtered from desktop-file discovery.
7. **Auth** — optional bearer/`X-NetRail-Token` middleware on `/api/*` except `/api/health` (auth.rs:57-65); typed `AUTH_REQUIRED` 401; works identically on both stacks.
8. **Fanout resilience** — a dead backend is an error string, never a failure; 20 s hard deadline; Wikipedia fallback; typed `FANOUT_TOTAL_FAILURE` only when zero results and errors exist (search.rs:27-32).

### 2.2 Weaknesses found (detail in §13)

| ID | Sev | One-line | Live evidence |
|----|-----|----------|---------------|
| **A1** | P1 | Rust 422s are plain text, not `{code,detail,status}` | `curl -d '{}' /api/search` → `Failed to deserialize the JSON body into the target type: missing field 'query'` (text/plain 422); Python maps the same input to typed `400 QUERY_INVALID` (main.py:72-129). Not covered by api_error_codes.rs, test_api.py, or parity harness. |
| **A2** | P1 | Token-mode UI injection defeated by the page's own CSP | With `NETRAIL_API_TOKEN` set: `GET /` returns `<script>window.NETRAIL_API_TOKEN=…</script>` **and** `Content-Security-Policy: script-src 'self'` → inline script blocked → UI cannot authenticate → every API call fails `401 AUTH_REQUIRED`. SECURITY.md/DISTRIBUTION.md document token mode as functional. |
| **A3** | P2 | Rust opens a fresh SQLite connection per request | `get_store` → `HistoryStore::open` (history/mod.rs:591-596) on every call (search record, history list, delete, collections, visit); each open runs the full `SCHEMA_SQL` DDL batch + TTL purge (mod.rs:109-117, 143-144). `STORE`/`with_store` (mod.rs:31,598-611) are dead code. No WAL, no `busy_timeout` in either stack → `SQLITE_BUSY` risk under concurrent writes (two rapid searches; search+visit). Python mitigates with a singleton (store.py:337-357). |
| **A4** | P2 | No graceful shutdown or signal handling | `axum::serve` without `with_graceful_shutdown` (mod.rs:88); desktop spawn has no cancellation (desktop.rs:52-56); `netrail-api` has no SIGTERM hook (bin/netrail-api.rs:12-18). Kill mid-write relies on SQLite rollback journal. Enterprise `systemctl stop` → possible lost last history entries. |
| **A5** | P2 | No observability; audit log unbounded | Tracing to stderr, no metrics endpoint, no request IDs, no structured log sink (tracing_subscriber fmt everywhere). `audit.rs:49-56`: append-only file, no rotation, no size cap, no per-action filtering. |
| **A6** | P2 | Settings: per-request disk read + last-writer-wins | `load_settings()` parses settings.json on every request (config.rs:112-122); `PUT /api/settings` has no version/ETag → two writers clobber silently. OK for single user; enterprise needs concurrency control. |
| **A9** | P2 | Rate limiting is per-process and disableable | Fixed-window counters in memory (rate_limit.rs:17-47), `NETRAIL_RATE_LIMIT=0` documented (rate_limit.rs:2). Docker + desktop on one host share nothing; not a security boundary (acknowledged), but no per-client/token dimension exists. |
| **A11** | P2 | No schema migration framework | Schema versioned by nothing; `CREATE TABLE IF NOT EXISTS` batch (history/mod.rs:33-92) duplicated verbatim in Python db.py:13-72. Cross-stack drift is mitigated only by the `decrypts_python_generated_database` test (history/mod.rs:667-724) and the parity harness — no `PRAGMA user_version`. |
| **A7** | P3 | Dead desktop-bridge path | `withGlobalTauri: false` (tauri.conf.json:14) → `window.__TAURI__` is never injected → `focus-search` emit (desktop.rs:145) + app.js:949-955 listener are inert; only the `eval` fallback (desktop.rs:146-148) works. |
| **A8** | P3 | Python blocks the event loop | Sync `search()` (ThreadPoolExecutor fanout) + sync sqlite IO inside `async def` handlers (main.py:366-382, 428-436); one slow backend blocks all other requests for up to 20 s. |
| **A10** | P3 | CSP hardening gaps | No `upgrade-insecure-requests`; `img-src https:` permits remote image loads (accepted residual R7 — thumbnails still hit CDNs, `no-referrer` mitigates); inline style `unsafe-inline` needed by vanilla UI. |
| **A12** | P3 | API contract versioning is a constant string | `api_contract: "1.4"` literal in both health endpoints (mod.rs:239, main.py:304); no Accept-versioning, no deprecation mechanism; forward-compat relies on serde/pydantic defaults. |
| **A13** | P3 | External-content FTS5 sync is manual | Every insert/delete path must mirror `queries_fts` writes (record_search mod.rs:193-197, purge/delete :171-173, :387-389). Correct today; any future path forgetting sync silently drifts search results. |
| **A15** | P1(open) | DNS pin on open remains unresolved (SEC-2026-04) | Validation is syntactic only — no resolve-before-open; validated public URL can redirect to loopback/private after validation (browser follows). Acknowledged residual since 2026-07; recommended roadmap item before multi-user/enterprise. |

---

## 3. Dual-stack parity analysis (Rust vs Python)

| Surface | Rust | Python | Parity |
|---------|------|--------|--------|
| Open URL policy | `validate_open_url` (security.rs:17-65) | `validate_open_url` (security.py:264) | ✅ golden fixture, 32 vectors |
| Backend URL policy | `validate_backend_url_with_options` (security.rs:276) | `validate_backend_url` (security.py:315) | ✅ incl. `strict` |
| Typed errors | `NetRailError` enum (error.rs) | `NetRailError` class (errors.py) | ✅ contract; **✗ 422 path** (A1) |
| Settings model | struct + serde (config.rs:24-45) | pydantic model (main.py:175-198) | ✅ fields align |
| Backend selection | `get_enabled_backends` (backends/mod.rs:86) | `get_enabled_backends` (registry.py:21) | ✅ same precedence |
| Fanout + deadline | `join_all` + 20 s (mod.rs:200-241) | ThreadPool + 20 s (registry.py:102-123) | ✅ same deadline const |
| Merge | dedupe+interleave (merge.rs:72-159) | same (merge.py:76-119) | ✅ |
| Wikipedia fallback | web-empty only (mod.rs:314-333) | same (registry.py:131-146) | ✅ |
| Sovereignty | 5-step + history bump (search.rs:54-65) | `_sovereignty_with_history` (search.py:12) | ✅ |
| History schema | mod.rs:33-92 | db.py:13-72 | ✅ verbatim |
| Encryption | Fernet + keyring (crypto.rs) | Fernet + keyring (history/crypto.py) | ✅ cross-stack blob tests |
| FTS query strip | `[^\w\s-]` (mod.rs:30,404) | `[^\w\s-]` (main.py:221-225) | ✅ identical regex |
| Auth/token | auth.rs | auth.py | ✅ |
| Audit | audit.rs | audit.py | ✅ |
| Rate limits | 90/120/60 per min (rate_limit.rs:10-13) | same consts (rate_limit.py:11-13) | ✅ |
| CSP | security.rs:386 | main.py:39-48 | ✅ identical |
| Request validation | axum extractors | pydantic | **✗ A1 (drift)** |
| Store lifecycle | shared in AppState (WAL + busy_timeout) | process singleton (WAL + busy_timeout) | ✅ (A3 closed) |
| Graceful shutdown | none | none | ✗ (shared gap A4) |

---

## 4. Reliability, operations, enterprise readiness

**What exists:** single loopback process with restartable server task; `init_history_on_startup`; encryption degrade banner surfaced in UI (`security:encryption-degraded` event, desktop.rs:45-50; `/api/health` flags); search recovery hints on health; golden-fixture regression safety; release CI with version-SSOT gate, parity smoke, clippy `-D warnings`.

**Enterprise gaps (beyond §2 findings):** no systemd/launchd units or healthcheck semantics beyond `/api/health`; no backup/restore of the SQLite DB; no read-only mode; no multi-user/role model; no egress proxy/TLS pinning config for backends; no metrics (SLO/SLI impossible); no structured logs for ingestion (SIEM); audit log has no rotation/retention policy; no webview E2E (desktop UX changes, e.g. the pending focus-search visual QA, ship without automated verification); Windows/macOS unsupported (Linux-only discovery, CI, packaging); no DB schema versioning (A11).

**Rate-limit semantics:** fixed-window with window-reset burst at boundaries; three independent buckets; `NETRAIL_RATE_LIMIT=0` disable. Documented as UX anti-abuse, not security — correct call, but enterprises must plan a per-identity dimension.

---

## 5. CI/CD & release engineering (as built)

- **ci.yml:** PR/main gates (check-versions, pytest, clippy `-D warnings`, cargo test, smoke).
- **release.yml** (repaired in 1.4.1, commit `72f2621`): tag `v*` → ubuntu-24.04 → dtolnay stable → npm ci → version SSOT gate → system deps → pytest + clippy + cargo test → `netrail-api --no-default-features` release → Tauri AppImage/deb/rpm → E2E smoke on the released binary → SBOM + SHA256SUMS (now awk-derived from Cargo.lock — previously a heredoc that broke YAML and killed every tag push) → release notes from `docs/RELEASE_${TAG}.md` → `gh release upload`.
- **Observed state during this audit:** run `30690333556` green through the headless-build step; Tauri bundle in progress at close of evidence collection. The YAML repair is confirmed effective (the run progressed past the step where all prior tag pushes failed in 0 s).
- **Verdict:** strong release discipline for a 1.x solo project; version SSOT across 5 files with a CI gate is exemplary. Gaps: no artifact attestation/signing, no SBOM pinning for the Tauri bundle, no caching for the ~11 min build, no Windows/macOS matrix, no dependency-audit step (`cargo audit`/`npm audit`).

---

## 6. Findings register

Severity key: **P0** ship/stop · **P1** real bypass/contract break/feature-defect · **P2** meaningful residual · **P3** polish/debt.

| ID | Sev | Title | Evidence | Fix direction |
|----|-----|-------|----------|---------------|
| A1 | P1 | Rust 422 responses break the typed-error contract | Live: missing/`Content-Type`-less/type-mismatched bodies → plain-text 422 (`Failed to deserialize…`, `Expected request with Content-Type…`); Python maps to typed 400 codes (main.py:72-129); neither api_error_codes.rs nor parity harness covers it | **✅ Closed 2026-08-01** — axum `JsonRejection`/`QueryRejection` → `ApiError` mapping (field-aware, mirrors Python), explicit `CONFIG_MAX_RESULTS` range check on search, 7 new integration tests + 5 parity probes |
| A2 | P1 | Token injection defeats itself via CSP `script-src 'self'` | Live: `/` with `NETRAIL_API_TOKEN` serves inline token script under a CSP that blocks inline scripts; UI then fails all API calls with 401 | **✅ Closed 2026-08-01** — `'sha256-…'` hash of the exact script added to `script-src` on the index response (Rust + Python); middleware inserts defaults only when absent; verified live (hash matches header) + tests |
| A15 | P1 | DNS pin on open unresolved (SEC-2026-04) | security.rs validates syntax only; browser resolves after validation | **✅ Closed 2026-08-01** — resolve-at-open before spawn in both stacks: `pin_open_host` resolves the validated host via the system resolver and re-runs the IP blocklist (`block_ip` extracted from the literal-IP check) on every answer; unresolvable hosts fail closed with new code `OPEN_URL_DNS_UNRESOLVABLE`; IP literals skip (already checked); resolver injectable → deterministic tests (loopback/private/link-local/empty answers), API-level monkeypatch tests, parity via shared codes; `http_client` redirect `Policy::none()` unchanged. DNS-over-HTTPS deliberately not used: the spawned browser re-resolves with the same system resolver, so DoH would not pin the browser; system-resolver check closes the attacker-DNS vector (hostname → private IP) that the browser would honor |
| A3 | P2 | Per-request SQLite reopen + dead singleton; no WAL/busy_timeout | history/mod.rs:31, 109-117, 143-144, 591-611; db.py:82 | **✅ Closed 2026-08-01** — `SharedStore` in `AppState` (one connection for process lifetime, reopened only on history/encryption settings change); `journal_mode=WAL` + 5 s `busy_timeout` in both stacks; dead `STORE` cell/`get_store`/`with_store` removed; visit recording moved to `open_link` handler (Python parity); TTL purge now runs at store open (startup/settings change) |
| A4 | P2 | No graceful shutdown / signal handling | mod.rs:88; desktop.rs:52-56; bin/netrail-api.rs:12-18 | **✅ Closed 2026-08-01** — `with_graceful_shutdown` + SIGINT/SIGTERM tokio signal in `server::start` (drains in-flight requests, clean exit verified live); SQLite WAL checkpoints on close; systemd unit pattern stays documented in DISTRIBUTION |
| A5 | P2 | No observability; audit log unbounded | audit.rs:39-56; tracing fmt everywhere | **✅ Closed 2026-08-01** — audit rotation (`NETRAIL_AUDIT_MAX_BYTES` default 10 MiB, `NETRAIL_AUDIT_MAX_FILES` default 3, 0 disables) in both stacks with rotation tests; optional structured JSON logs via `NETRAIL_LOG_JSON=1` (new `logging.rs`, wired into all three entrypoints) |
| A6 | P2 | Settings last-writer-wins, per-request re-parse | config.rs:112-122; put_settings mod.rs:295-304 | **✅ Closed 2026-08-01** — strong `ETag` on `GET`/`PUT /api/settings`; optional `If-Match` on PUT (mismatch → 409 `SETTINGS_CONFLICT`, absent → back-compat); stateless re-read kept; parity probes + integration tests on both stacks (Python `config_file()` now resolves `$HOME` lazily so tests isolate correctly) |
| A9 | P2 | Rate limits per-process, fixed-window, disableable | rate_limit.rs:10-47 | **✅ Closed 2026-08-01** — per-identity buckets: `anonymous` without a token, else `token:<base64(sha256(token))>`; 1024-identity cap with idle sweep; health/status reports `mode: per-token|process`; multi-process limits still OS-level (documented) |
| A11 | P2 | No schema migrations | history/mod.rs:33-92 vs db.py:13-72 | **✅ Closed 2026-08-01** — `PRAGMA user_version` framework in both stacks (`SCHEMA_VERSION = 1`, ordered `if current < N` steps); existing DBs migrate idempotently on open; tests assert version stamp + WAL mode |
| A7 | P3 | Dead `focus-search` emit path (`withGlobalTauri:false`) | tauri.conf.json:14; app.js:949-955; desktop.rs:145 | **✅ Closed 2026-08-01** — dead `emit` calls (focus-search, security:encryption-degraded) and the inert `__TAURI__` listeners removed; eval bridge + health-driven banner kept; webview E2E still open (see matrix #9) |
| A8 | P3 | Python event-loop blocking in sync handlers | main.py:366-382, 428-436 | **✅ Closed 2026-08-01** — all sync-bodied route handlers converted from `async def` to `def` (FastAPI threadpool): index, health, backends, browsers, settings, docs, search, open (incl. the new DNS resolve), history, collections, export; only middleware/error handlers stay async |
| A10 | P3 | CSP gaps (`upgrade-insecure-requests`, img-src) | security.rs:386 | **✅ Closed 2026-08-01** — `upgrade-insecure-requests` added to the shared CSP const in both stacks (browser auto-upgrades http subresources to https); `img-src https:` kept as documented residual R7 (thumbnails from CDNs, `no-referrer` mitigates); exact-string CSP assertions updated in Rust/Python |
| A12 | P3 | api_contract is a literal, not versioning | mod.rs:239; main.py:304 | **✅ Closed 2026-08-01** — contract centralized as `API_CONTRACT = "1.4"` (netrail/`__init__.py`) and `pub const API_CONTRACT` (config.rs), referenced by both health endpoints and tests; evolution rule documented: additive changes only, bump only on breaking change |
| A13 | P3 | Manual FTS5 external-content sync | mod.rs:193-197, 387-389 | **✅ Closed 2026-08-01** — sync test coverage exposed a real bug: contentless FTS5 tables reject `DELETE` statements (verified on SQLite 3.46), so `delete_history_entry`/`purge_expired`/`purge_all_history` failed in both stacks; fixed via `rebuild_fts_index()` (drop + recreate + reindex from `queries`) used by every delete path; tests assert `COUNT(queries) == COUNT(queries_fts)` with zero orphans through record/delete/purge/expire lifecycles on both stacks |

---

## 7. Recommendation matrix (risk × effort)

| # | Action | Fixes | Effort | Risk |
|---|--------|-------|--------|------|
| 1 | Typed 422 mapping + harness vectors | A1 | S | H |
| 2 | CSP nonce for injected token + token-mode E2E | A2 | S | H |
| 3 | Persistent store in AppState + WAL + busy_timeout | A3 | M | M | **done 2026-08-01** |
| 4 | Graceful shutdown + signal handling | A4 | S | M | **done 2026-08-01** |
| 5 | Audit log rotation + structured logging option | A5 | M | M | **done 2026-08-01** |
| 6 | Settings ETag/version | A6 | S | L | **done 2026-08-01** |
| 7 | Schema `user_version` migrations | A11 | S | M | **done 2026-08-01** |
| 8 | DNS-pin roadmap (resolve+verify before spawn) | A15 | L | H | **done 2026-08-01** — `pin_open_host` in both stacks: resolve via system resolver + blocklist on every answer before spawn; `OPEN_URL_DNS_UNRESOLVABLE` fails closed; parity-tested |
| 9 | Webview E2E for focus-search/docs bridge | A7 + handoff P1 | M | M | **done 2026-08-01** — `scripts/webview-e2e.sh` + `tests/webview_e2e.py`: tauri-driver + WebKitWebDriver + selenium driving the real debug webview; 6/6 checks green (page load, `netrailFocusSearch` bridge, xdotool global-shortcut pipeline, `netrailOpenDoc('manual')`, dialog guard, docs error path); `XDG_DATA_HOME`/`XDG_CACHE_HOME` isolated because the WebKit HTTP cache otherwise serves a stale cached `app.js` (verified: old blob matched commit `0072d34`, `transferSize: 0` cache hit) |
| 10 | CI: `cargo audit` + `npm audit` + artifact signing | — | S–M | L | **done 2026-08-01** — release workflow gates on `cargo audit` (0 vulnerabilities after `plist` 1.9→1.10 / `quick-xml` 0.39→0.41 upgrade for RUSTSEC-2026-0194/0195) and `npm audit --audit-level=high` (clean); release `SHA256SUMS` is sigstore keyless-signed (`cosign sign-blob` via GitHub OIDC, verified in-job) — `SHA256SUMS.sig`/`SHA256SUMS.pem` ship as release assets; `Swatinem/rust-cache` added. **First live run verified 2026-08-01 on the v1.6.1 tag** (release run `30727671725`, build-linux green in 14m1s; assets: AppImage/deb/rpm/netrail-api/SBOM + signed SHA256SUMS, signature re-verified offline with pinned identity `release.yml@refs/tags/v` + actions OIDC issuer). Two toolchain pins were required to get there: `cargo-audit 0.22.2` (0.21.2 cannot parse CVSS 4.0 entries in the current RustSec advisory DB — RUSTSEC-2026-0109) and `cosign verify-blob --certificate-identity-regexp/--certificate-oidc-issuer` (keyless verify refuses to run without them) |

**Recommended sequence:** 1–2 (1.4.2 hotfix) — **done 2026-08-01**, see closure notes in §6 — then 3–4–7 (1.5.0 hardening) — **done 2026-08-01** — then 5–6 (1.6.0 ops) — **done 2026-08-01** — then 9 (webview E2E) — **done 2026-08-01** — then 8 (enterprise readiness gate, before any multi-user story).

---

## 8. Evidence appendix

- Live probes against `src-tauri/target/release/netrail-api` (1.4.1): health schema; missing body / missing Content-Type / wrong type on `/api/search` (A1); `/` token injection + CSP header pair (A2); unauthenticated `/api/search` → `401 AUTH_REQUIRED` (auth confirmed).
- Static: every file listed in §1.2 read in full; line anchors above.
- Release CI `30690333556` (v1.4.1): steps `version SSOT`, `tests and lint`, `build headless API` completed successfully at evidence close; Tauri bundle in progress.
- Test gates at 1.4.1 (from release cycle): pytest 104, Rust lib 59, api_error_codes 10, clippy clean, parity smoke OK (32 fixture vectors live), e2e smoke OK, compose config OK.
