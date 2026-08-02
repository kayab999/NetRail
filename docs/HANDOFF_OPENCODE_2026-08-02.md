# NetRail — Enterprise Analysis & OpenCode Handoff (2026-08-02)

| Field | Value |
|-------|--------|
| **Audience** | OpenCode / any zero-context coding agent |
| **Product** | NetRail — local privacy-first research console (Linux) |
| **Version (SSOT)** | **1.6.1** (`scripts/check-versions.sh`, 5 files) |
| **License** | AGPL-3.0 |
| **Repo** | https://github.com/kayab999/NetRail |
| **Handoff date** | 2026-08-02 (supersedes `HANDOFF_OPENCODE_2026-08-01.md`) |
| **HEAD (committed)** | `abaf661` — `enterprise: read-only mode (NETRAIL_READONLY) + systemd unit + DB backup + audit docs` |
| **Branch** | `main` **in sync with `origin/main`** (all pushed) |
| **Working tree** | **Clean** |
| **Releases** | `v1.6.0` + `v1.6.1` published on GitHub (assets: AppImage/deb/rpm/netrail-api/SBOM.txt/SHA256SUMS + sigstore `.sig`/`.pem`, signature verified) |
| **Primary path** | Rust Axum API + Tauri 2 desktop; Python FastAPI for Docker/Flatpak/tests |
| **API bind** | `127.0.0.1:7421` only |
| **UI** | Vanilla HTML/CSS/JS in `netrail/static/` (no React/Vue; no PySide6/Qt) |
| **api_contract** | **1.4** |

**Companion docs (read in order if time-boxed):**

1. This handoff  
2. [HANDOVER.md](../HANDOVER.md) — freeze invariants + resume prompt  
3. [docs/AUDIT_ARCH_2026-08-01.md](AUDIT_ARCH_2026-08-01.md) — architecture/reliability audit (A1–A15, all closed)  
4. [docs/AUDIT_OPENCODE_ADVERSARIAL_2026-08-01.md](AUDIT_OPENCODE_ADVERSARIAL_2026-08-01.md) — adversarial findings (N1–N4 closed) + residuals register  
5. [SECURITY.md](../SECURITY.md) — threat model  
6. [docs/API_ERRORS.md](API_ERRORS.md) — typed error contract (incl. `READONLY_MODE`)  
7. Previous handoff: `docs/HANDOFF_OPENCODE_2026-08-01.md` (historical session record)

---

## 0. OpenCode mission briefing (30 seconds)

You are continuing **NetRail**, a single-user Linux research console: **query → fanout search → link rail → user opens chosen URL**.

- **Not** multi-tenant SaaS, remote-auth product, Google replacement, or owned crawl corpus (roadmap only).  
- **Not** a Qt/PySide GUI — UI is **static web** inside Tauri webview (or browser against the API).  
- **Production engine:** Rust. **Python:** parity surface for tests/Docker/Flatpak/`install.sh`.  
- **Do not** force-push. **Do not** amend published history. Push is expected when the human asks for a change batch (recent batches were committed + pushed in-session).  
- Prefer durable, minimal, dual-stack-aware changes. No scope creep.

---

## 1. Executive verdict (enterprise-grade)

### 1.1 Product posture (post-1.6.1)

| Dimension | Score (0–10) | Assessment |
|-----------|-------------:|------------|
| Core job (search → rail → open) | **9** | End-to-end path solid on both stacks |
| Threat-model fit (single-user local) | **9** | Localhost API by design; DNS pin on open (A15) |
| Security vs local attacker | **7.5** | Token (1.4.0) + DNS pin (1.6.1) + read-only gate (1.6.2-unreleased); default still open on loopback by design |
| Dual-stack parity | **9** | N1–N4 fixture + live parity harness drive both stacks from one SSOT |
| Correctness / tests | **9** | 162 pytest, 113 cargo tests, clippy `-D warnings`, webview E2E (matrix #9), CSS layout guard (E3) |
| Ops / packaging | **8.5** | deb/rpm/AppImage/CI + cosign-signed releases; systemd unit + backup script + read-only mode (unreleased) |
| Docs truth | **9** | Audit passes re-verified claims; residuals honestly documented |
| Enterprise multi-user / SOC2-ish | **3.5** | Out of stated model; token + audit + read-only are optional footholds only |
| Desktop UX polish | **8** | Keyboard rail, recovery UX, Spotlight focus, tray/hotkey |
| **Overall (v1 desktop product)** | **~8.8** | Soft GA reached; remaining work is release hygiene + backlog, not a rewrite |

**Verdict:** NetRail is a **credible single-user localhost research console** with production-quality open-URL controls (including resolve-and-pin before browser spawn), typed errors, rate limits, optional auth/audit/read-only gates, and a Rust-primary engine. Residual risk clusters around (1) unauthenticated default localhost API (by design, token optional), (2) dual-stack maintenance cost, (3) DDGS/provider fragility, (4) time-based DNS rebinding (documented), (5) remote image CDN loads (R7).

### 1.2 Release / git posture (critical for agents)

| Fact | Detail |
|------|--------|
| Committed product version | **1.6.1** on `main`, released + published (tag `v1.6.1` → `807bd51`) |
| Remote | `main` == `origin/main` (**everything pushed**) |
| Working tree | **Clean** |
| CI pins (verified live on v1.6.1) | `cargo-audit 0.22.2` (0.21.x can't parse CVSS 4.0), cosign verify needs `--certificate-identity-regexp` + `--certificate-oidc-issuer` |
| Policy | No force-push; no amend of published history; version bumps touch all 5 SSOT files or CI fails |

### 1.3 What “enterprise” means here

Enterprise *readiness for the stated threat model* ≠ multi-tenant SaaS. For NetRail:

| Control | Status |
|---------|--------|
| Localhost-only bind | ✅ Hard invariant |
| Open-URL SSRF-class guards | ✅ Encoded loopback, private IP, rebinding apex, DDG unwrap, metadata hosts, trailing-dot normalization (N1–N3), **DNS pin on open (A15)** |
| Typed API errors `{code,detail,status}` | ✅ Dual-stack; Rust 422→typed mapping (A1) |
| Rate limits (search/open/mutate) | ✅ Per-identity buckets since 1.6.0 (A9) |
| Optional API token | ✅ `NETRAIL_API_TOKEN` (default off) |
| Audit log (JSONL, rotated) | ✅ Opt-in; rotation + JSON logs since 1.6.0 (A5) |
| Strict backend URLs | ✅ Opt-in (homelab vs cloud split) |
| **Read-only mode** | ✅ `NETRAIL_READONLY=1` → `403 READONLY_MODE` on all mutations (unreleased, dual-stack + tests) |
| Systemd unit + DB backup | ✅ `packaging/netrail-api.service`, `scripts/backup-db.sh` (unreleased) |
| SBOM / dep audit in CI | ✅ SBOM.txt per release; cargo-audit/npm-audit/pip-audit gated; **pinned** in every binary (`--sbom`) + in the deb/rpm/AppImage (E2, 2026-08-02) |
| Formal SDL / multi-user RBAC | ❌ Out of scope |

---

## 2. Architecture (authoritative)

### 2.1 Runtime topology

```
┌──────────────────────────────────────────────────────────────────────────┐
│  ENTRY                                                                   │
│  • netrail          Tauri 2 desktop (feature desktop)                    │
│  • netrail-api      Headless Rust binary (--no-default-features OK)      │
│  • systemd          packaging/netrail-api.service (hardened unit)        │
│  • python -m netrail  FastAPI fallback (Docker/Flatpak/tests)            │
└────────────────────────────────┬───────────────────────────────────────┘
                                 ▼
                    HTTP 127.0.0.1:7421
         Axum (Rust primary)  or  FastAPI (Python)
                                 │
     ┌───────────┬───────────────┼────────────────┬────────────────┐
     ▼           ▼               ▼                ▼                ▼
 static UI   /api/search    /api/open      history SQLite    settings
 app.js      fanout+merge   browsers.*     Fernet/keyring    XDG JSON
 markdown    ddgs|searxng   pin_open_host  FTS5 tokens       config.*
             brave|wiki     (A15)          (plaintext FTS)
```

Read-only gate (`ensure_mutable` Rust / `_ensure_mutable` Python) sits at the top of the 5 mutation handlers; read endpoints are unaffected.

### 2.2 Frontend truth (do not invent stacks)

| Layer | Technology | Path |
|-------|------------|------|
| UI | **Vanilla** `index.html` + `app.js` + `style.css` + `markdown.js` | `netrail/static/` |
| Desktop shell | **Tauri 2** webview → loads `http://127.0.0.1:7421` | `src-tauri/` |
| Global Tauri JS | **`withGlobalTauri: false`** | Prefer `window.eval` bridges (A7 removed dead emit paths) |
| Native menus | Rust menu + tray | `src-tauri/src/desktop.rs` |
| **Not used** | PySide6, PyQt, Electron, React, Vue, Angular | — |

**UI ↔ backend:** plain `fetch` to local HTTP API. Docs/donate/focus use `window.eval("window.netrail…")` from Rust.

### 2.3 Module map

| Area | Rust | Python |
|------|------|--------|
| HTTP server | `src-tauri/src/server/mod.rs` | `netrail/main.py` |
| Fanout / backends | `src-tauri/src/backends/` | `netrail/backends/` |
| URL policy | `src-tauri/src/security.rs` | `netrail/security.py` |
| History | `src-tauri/src/history/` | `netrail/history/` |
| Auth (optional) | `src-tauri/src/auth.rs` | `netrail/auth.py` |
| Rate limit | `src-tauri/src/rate_limit.rs` | `netrail/rate_limit.py` |
| Audit | `src-tauri/src/audit.rs` | `netrail/audit.py` |
| JSON logging | `src-tauri/src/logging.rs` | `netrail/logging.py` (if any) |
| Desktop / tray | `src-tauri/src/desktop.rs` | — |
| Shared UI | `netrail/static/` | same |

**Config:** `~/.config/netrail/settings.json`  
**DB:** `~/.local/share/netrail/netrail.db` (`NETRAIL_DB_PATH`)  
**Static:** `static_dir()` / `NETRAIL_STATIC_DIR` / packaged `/usr/share/netrail/static/`

### 2.4 Version SSOT (must stay aligned)

When bumping version, update **all** of:

- `package.json`
- `src-tauri/Cargo.toml`
- `src-tauri/tauri.conf.json`
- `netrail/__init__.py`
- Rust `config` VERSION constant (see `scripts/check-versions.sh`)

CI fails on drift: `bash scripts/check-versions.sh`.

---

## 3. Hard invariants (never break)

1. **Bind `127.0.0.1` only** — no `0.0.0.0` without redesign + auth + docs.  
2. **Zero product telemetry** — no analytics, accounts, or Brave key on disk (env only).  
3. **Open-URL validation before browser spawn**  
   - http(s) only; no credentials  
   - block localhost + **encoded loopback** (decimal/hex/octal/short IPv4)  
   - block private / non-public (`OPEN_URL_PRIVATE`)  
   - block rebinding apex + suffixes  
   - unwrap DDG `uddg=` (host set includes `duck.com`, `www`, `r.`) before checks  
   - cloud metadata hostnames blocked  
   - IPv4-mapped IPv6 unmapped before policy  
   - normalize host: percent-decode, lowercase, strip trailing dots (N1–N3)  
   - **DNS pin on open (A15):** resolve host via system resolver and re-run the IP blocklist on every answer before spawn; unresolvable → `OPEN_URL_DNS_UNRESOLVABLE` fail closed  
4. **Backend URLs** may be localhost/LAN for SearXNG **unless** `strict_backend_urls` / `NETRAIL_STRICT_BACKEND_URLS`.  
5. **Env backend URLs** (`NETRAIL_SEARXNG_URL` / `SEARXNG_URL`) validated on load; invalid ignored.  
6. **Partial fanout failure** → HTTP 200 + `errors[]`; **total failure** → `FANOUT_TOTAL_FAILURE` 502.  
7. **Empty web fanout** → Wikipedia fallback (both stacks).  
8. **Fanout deadline** 20s both stacks.  
9. **History:** encrypt when key available; degrade + banner if keyring missing; FTS tokens plaintext (documented).  
10. **Rate limits:** search 90/min · open 120/min · mutate 60/min per identity (since 1.6.0); `NETRAIL_RATE_LIMIT=0` disables.  
11. **Typed errors:** `{ code, detail, status }` — see `docs/API_ERRORS.md`.  
12. **Dual-stack security changes land in both languages same PR** when possible.  
13. **Packaged UI must ship** under `/usr/share/netrail/static/` (deb/AppImage). Missing static = broken desktop.  
14. **Read-only mode:** `NETRAIL_READONLY=1` must block every mutating endpoint with `403 READONLY_MODE` (never silently skip the gate when adding handlers — mirror `ensure_mutable`/`_ensure_mutable`).

**Must not break:** search/open happy path, static packaging, Fernet Rust↔Python history interop, error shape, version SSOT.

---

## 4. Security model (agent-facing)

### 4.1 What we protect

| Asset | Protection |
|-------|------------|
| Query text / snippets at rest | Fernet when keyring or `NETRAIL_DB_KEY` available |
| Browser open SSRF to loopback/LAN | Open-URL policy + DDG unwrap + **DNS pin at open (A15)** |
| Accidental remote exposure | Localhost bind; Docker compose publishes `127.0.0.1:7421` |
| Mutation of state | Rate limits + optional token + **read-only mode** (`NETRAIL_READONLY=1`) |

### 4.2 What we do **not** claim

| Residual | Why |
|----------|-----|
| Same-machine malware | Unauthenticated API by default = full product control |
| Query privacy from backends | Queries **egress** to DDGS / SearXNG / Brave / Wikipedia |
| FTS full-disk secrecy | FTS5 tokens + URLs plaintext by design |
| Protection vs **time-based DNS rebinding** | TTL-flip after pin: browser re-resolves independently; same cost class as the original issue; documented (Q16 residual) |
| Protection vs remote image loads | CSP `img-src https:` still lets CDN thumbnails see requests (`no-referrer` mitigates; residual R7) |
| “Everything stays on 127.0.0.1” | **False** for query egress (fixed in Wave 0 docs) |

### 4.3 Optional enterprise controls

| Control | Env / setting | Notes |
|---------|---------------|-------|
| API token | `NETRAIL_API_TOKEN` | Bearer or `X-NetRail-Token`; health exempt |
| UI token inject | `NETRAIL_INJECT_UI_TOKEN` | CSP-safe via `script-src 'sha256-…'` (A2) |
| Strict backends | `strict_backend_urls` / `NETRAIL_STRICT_BACKEND_URLS=1` | Reject private/loopback backend URLs |
| Audit log | `NETRAIL_AUDIT_LOG=1` / `NETRAIL_AUDIT_LOG_PATH` | JSON lines; rotation via `_MAX_BYTES` (10 MiB) / `_MAX_FILES` (3) |
| JSON logs | `NETRAIL_LOG_JSON=1` | Structured tracing logs |
| **Read-only** | `NETRAIL_READONLY=1` | 403 on PUT settings / history delete/purge / collection create-add |
| Rate limit off | `NETRAIL_RATE_LIMIT=0` | Smoke/CI only; not default for users |

---

## 5. Session delta (recent sessions — all landed, committed, pushed)

### 5.1 Batch 1 — A15 + P3 audit batch (commit `feb2bbd`)

- **A15 DNS pin on open** (dual-stack): `pin_open_host` resolves + re-blocks before browser spawn; `OPEN_URL_DNS_UNRESOLVABLE`; `block_ip`/`_block_ip` extracted as shared policy; injectable resolver; 8 Rust + 8 Python unit tests + 2 Python API monkeypatch tests.
- **A8** Python handlers off the event loop (`async def` → `def`).
- **A10** CSP `upgrade-insecure-requests`.
- **A12** `API_CONTRACT = "1.4"` consts.
- **A13** FTS5 sync: real bug found (contentless FTS5 rejects `DELETE`, SQLite 3.46) → `rebuild_fts_index()` on every delete path; lifecycle tests.

### 5.2 Batch 2 — v1.6.1 release (`4ccc9db` → `182301f` + tag `v1.6.1` = `807bd51`)

- Version SSOT bumped 1.6.0 → 1.6.1; CHANGELOG retitled; release published with assets incl. `netrail-api`, `SBOM.txt`, `SHA256SUMS` + `.sig`/`.pem`.
- CI fixes found live: `cargo-audit` pinned 0.22.2; cosign verify identity flags added (`5749f97`, `807bd51`).
- Matrix #10 (dep audits + sigstore signing) verified green live on the release run; signature re-verified offline (`Verified OK`).

### 5.3 Batch 3 — enterprise residuals (commit `abaf661`, HEAD)

- **Read-only mode** (`NETRAIL_READONLY=1`): Rust `NetRailError::Readonly` (403/`READONLY_MODE`), `ensure_mutable()` + 5 gated handlers; Python `_ensure_mutable()` same 5; read endpoints unaffected.
- **`packaging/netrail-api.service`** — hardened systemd unit (User=netrail, ProtectSystem=strict, NoNewPrivileges, PrivateTmp, ReadWritePaths=/var/lib/netrail).
- **`scripts/backup-db.sh`** — WAL-safe `sqlite3 .backup`; restore via `.restore` (service stopped); cron example in DISTRIBUTION.md.
- Tests: `src-tauri/tests/readonly_mode.rs` (6 tests, separate binary so the env can't race `api_error_codes.rs`) + 2 Python API tests. Gates: 162 pytest, 113 cargo tests, clippy clean, parity + E2E smokes green on rebuilt release binary.
- Docs: `API_ERRORS.md` `READONLY_MODE`, DISTRIBUTION.md env table + systemd/backup sections, CHANGELOG [Unreleased], audit docs closed-state updates.

### 5.4 Batch 4 — backlog E2/E3/E4/E5 + v1.6.2 release (PRs #1–#4, main)

- **PR #1 → v1.6.2** (`307d92b`, tag `v1.6.2`): Sprints 2–4 hardening merged (chaos suite `aa2400a`, load/benchmarks `12fd13c`, hardening report `065e110`) + the PR-review CI fixes (`7ec8f69` chaos-test filter split, `a9b1bf1` `pip-audit`→`pip_audit`). Release workflow green; cosign keyless verified.
- **E2 SBOM in bundle** (PR #2, `abfdca0`): `build.rs` derives Rust inventory from `Cargo.lock` → `src/sbom.rs` `include_str!`; `netrail-api --sbom` (byte-identical to `SBOM.txt` Rust section); `tauri.conf.json` ships `SBOM.txt` at `/usr/share/netrail/SBOM.txt` in deb/rpm/AppImage; release.yml regenerates + `cmp` + dpkg/rpm presence step; `scripts/generate-sbom.sh` unified (fixed bare-`@4` bug).
- **E5 fixture growth** (PR #3, `ca35210`): `url_policy.json` 43→68 vectors; found + fixed Python `ftp://` → `OPEN_URL_INVALID_SCHEME` divergence; parity harness now probes `backend_url` live via settings PUT.
- **E3 CSS guard** (PR #4, `54ce3be`): `tests/test_ui_css.py` pins the `.result-card` minmax grid contract (CI-gated).
- **E4** — decision (no code): kept `show_menu_on_left_click(true)`.
- Docs refresh `536d32e`: HANDOVER HEAD note + HANDOFF tables; gates now **162 pytest · 113 cargo**.

**The 08-01 handoff's Spotlight/CSS WIP shipped in the 1.5.0 series** (tray focus places caret in `#query`; result-card grid `minmax(0,1fr) auto`) — no longer a continuity surface.

---

## 6. Findings backlog (audit residuals)

Source of truth: [AUDIT_ARCH_2026-08-01.md](AUDIT_ARCH_2026-08-01.md) (A1–A15 all closed) + [AUDIT_OPENCODE_ADVERSARIAL_2026-08-01.md](AUDIT_OPENCODE_ADVERSARIAL_2026-08-01.md) (N1–N4 closed). Do **not** re-open closed items without new evidence.

| Status | Items |
|--------|--------|
| **Closed (1.4.x–1.6.1)** | SEC-01/02/03/06/07/08/13, PAR-01/02/03/04/07, A1–A15, N1–N4, R8/OPS-03 (webview E2E), matrix #10 (CI audits + signing), enterprise gaps (systemd, backup, read-only, log-json, audit rotation, schema versioning) |
| **Still residual (accepted, documented)** | Time-based TTL-flip DNS rebinding (Q16); image CDN loads R7 / SEC-2026-12; unauth localhost default SEC-2026-09 (token optional); FTS plaintext SEC-2026-10; fixed-window rate limits SEC-2026-11 (per-identity since 1.6.0); dual-stack drift cost R3 (parity harness mitigates) |
| **Backlog (don't build unless asked)** | C3 DNS "resolve-and-warn" flag; C4 images-off flag; multi-user/RBAC; egress proxy/TLS pinning for backends; metrics/SLO; Windows/macOS ports |

**Do not expand scope into:** multi-user OAuth, owned crawl corpus, local LLM, non-Linux ports — unless human explicitly asks.

---

## 7. How to run / test / package (bootstrap)

```bash
# Version gate
bash scripts/check-versions.sh

# Python tests
source .venv/bin/activate
pytest tests/ -q

# Rust
cd src-tauri
cargo clippy --all-targets -- -D warnings
cargo test
cd ..

# Headless API
cargo build --release --manifest-path src-tauri/Cargo.toml \
  --bin netrail-api --no-default-features
./src-tauri/target/release/netrail-api
curl -s http://127.0.0.1:7421/api/health

# Smokes (rate limit off)
NETRAIL_RATE_LIMIT=0 bash scripts/e2e-api-smoke.sh
NETRAIL_RATE_LIMIT=0 bash scripts/parity-api-smoke.sh   # needs built binary
bash scripts/package-smoke.sh

# Webview E2E (matrix #9; needs tauri-driver + WebKitWebDriver + selenium)
bash scripts/webview-e2e.sh

# Desktop dev
npm ci && npm run dev

# Desktop release (AppImage needs patchelf)
APPIMAGE_EXTRACT_AND_RUN=1 npm run build
```

Gates as of 2026-08-02 (post backlog E2/E3/E5): **162 pytest · 113 cargo tests · clippy `-D warnings` clean · parity + E2E smokes green**.

### Useful env

| Variable | Purpose |
|----------|---------|
| `NETRAIL_STATIC_DIR` | UI assets |
| `NETRAIL_DB_PATH` | SQLite path |
| `NETRAIL_DB_KEY` | Fernet key (headless/Docker) |
| `NETRAIL_READONLY` | `1` = read-only mode (403 on mutations) |
| `BRAVE_SEARCH_API_KEY` | Brave backend |
| `NETRAIL_SEARXNG_URL` / settings | SearXNG |
| `NETRAIL_RATE_LIMIT=0` | Disable limits (CI/smoke) |
| `NETRAIL_API_TOKEN` | Optional auth |
| `NETRAIL_INJECT_UI_TOKEN` | UI header inject |
| `NETRAIL_STRICT_BACKEND_URLS` | Strict backend mode |
| `NETRAIL_AUDIT_LOG` / `_PATH` / `_MAX_BYTES` / `_MAX_FILES` | Audit JSONL + rotation |
| `NETRAIL_LOG_JSON` | Structured JSON logs |
| `APPIMAGE_EXTRACT_AND_RUN=1` | AppImage without FUSE |

---

## 8. Desktop UX surface (for UI work)

| Control | Behavior |
|---------|----------|
| Query input | `#query` — primary Spotlight target |
| Results | `.result-card` grid; `.result-body` + `.result-actions` |
| Open | `.open-btn` → `POST /api/open` |
| Save | ★ → collection dialog |
| Keyboard | Highlight rail; copy; open (see MANUAL) |
| Tray | Show / Quit; hide-on-close |
| Hotkey | `Ctrl+Shift+S` |
| Help | Native menu + web dropdown → docs dialog |

**IPC patterns:**

| Pattern | When |
|---------|------|
| `window.eval("window.netrailX()")` | Docs, donate, **focus-search** (reliable) |
| Health-driven UI | Encryption-degrade banner reads `/api/health` (A7 removed dead emits) |
| HTTP `fetch` | All product API |

---

## 9. Remaining work points (established 2026-08-02)

### P1 — Release hygiene (next release)

| # | Item | Acceptance |
|---|------|------------|
| ~~R1~~ | ~~Cut the next release from [Unreleased] delta~~ | **✅ Done 2026-08-02** — v1.6.2 cut (read-only mode + systemd unit + backup script) with the standard SSOT bump + tag; workflow green, cosign verified |
| ~~R2~~ | ~~Post-release doc refresh~~ | **✅ Done 2026-08-02** — DISTRIBUTION version headers; HANDOVER HEAD note; CHANGELOG [1.6.2] |
| R3 | **Cut v1.6.3** from the current [Unreleased] delta (E2 SBOM-in-bundle + E5 fixture growth + E3 CSS guard) | Bump SSOT 1.6.2 → **1.6.3** in 5 files (`package.json`, `Cargo.toml`, `tauri.conf.json`, `netrail/__init__.py`, `src-tauri/src/config.rs`); `check-versions.sh` green; CHANGELOG [Unreleased] → [1.6.3]; tag; release workflow — **first live exercise of the new "SBOM embedded in bundles" verify step** (dpkg-deb/rpm) and the pre-bundle SBOM generation ordering; verify cosign + offline signature like v1.6.1 |
| R4 | Post-1.6.3 doc refresh | DISTRIBUTION.md version headers; doc index; this handoff's release row; README screenshots/version badges if touched |

### P2 — Engineering backlog (not blocking)

| # | Item | Notes |
|---|------|-------|
| ~~E1~~ | ~~Load / performance check~~ | **✅ Done 2026-08-02** (Sprints 3–4): `scripts/load/{run,slope}.py` + `scripts/load-10k.sh` resource-stability harness and `scripts/bench/bench.py` + `report.py` + `scripts/bench-dual.sh` dual-stack benchmarks → `docs/sprint3-slope.md` + `docs/bench-dual.md` (Rust ≈573 rps / p50 23 ms / 14% CPU / 10.4 MiB vs Python ≈295 rps / p50 39 ms / 74% CPU / 64.1 MiB; no knee ≤ C=512). Also surfaced + fixed the Python `HistoryStore` concurrency race |
| ~~E2~~ | ~~SBOM pinned in bundle~~ | **✅ Done 2026-08-02** (PR #2): Rust inventory embedded in every binary at build time (`build.rs` reads `Cargo.lock` → `sbom.rs` `include_str!`; `netrail-api --sbom` prints it, byte-identical to the shipped `SBOM.txt` Rust section) + full `SBOM.txt` packaged in `.deb`/`.rpm`/AppImage at `/usr/share/netrail/SBOM.txt`; release CI asserts both. Generator unified in `scripts/generate-sbom.sh` (fixed the bare-`@4` awk bug) |
| ~~E3~~ | ~~CSS regression snapshot~~ | **✅ Done 2026-08-02** (PR #4): `tests/test_ui_css.py` — structural guard pinning `.result-card` desktop `minmax(0,1fr) auto`, image-card `96px minmax(0,1fr) auto`, action column stays `auto`, `720px` collapse to `1fr`. CI-gated via `pytest tests/`; verified non-vacuous |
| ~~E4~~ | ~~Alfred-style tray left-click~~ | **✅ Closed 2026-08-02** — kept `show_menu_on_left_click(true)` (left-click opens menu + focuses window); decision recorded, no change |
| ~~E5~~ | ~~Golden fixture growth~~ | **✅ Done 2026-08-02** (PR #3): `tests/fixtures/url_policy.json` 43→68 vectors (IPv6 forms, percent/uppercase-scheme loopbacks, `localhost`, `0.0.0.0`, ftp/file schemes, xip.io subdomain, double-encoded DDG unwrap, metadata IP; IPv6 backend + strict). Fixed Python `ftp://` code divergence (now `OPEN_URL_INVALID_SCHEME` like Rust). Parity harness now probes `backend_url` live too |

### P3 — Accepted residuals (documented; do not build unless asked)

| Item | Where documented |
|------|------------------|
| Time-based TTL-flip DNS rebinding | Q16 / SEC-2026-04 residual (C3 "resolve-and-warn" flag = backlog) |
| Image CDN loads (`img-src https:`) | R7 / SEC-2026-12 (C4 images-off flag = backlog) |
| Unauth localhost default (token optional) | SEC-2026-09 |
| FTS tokens plaintext | SEC-2026-10 |
| Fixed-window rate limits | SEC-2026-11 |
| Dual-stack drift cost | R3 (parity harness + fixture mitigate) |
| Multi-user/RBAC, egress proxy/TLS pin, metrics/SLO, Windows/macOS | Out of scope |

---

## 10. Anti-patterns for coding agents

| Do **not** | Why |
|------------|-----|
| Introduce React/Vue “because frontend” | Architecture is vanilla static by design |
| Rewrite UI in PySide6/Qt | Explicitly not the stack |
| Bind API to `0.0.0.0` “for Docker ease” | Breaks threat model |
| Store Brave key in settings.json | Policy: env only |
| Silent empty search without errors[] | Tribunal / recovery UX regression |
| Fix security only on Rust | Dual-stack drift = PAR findings |
| Disable open-URL private blocks for “LAN collections” | Intentional; use backends policy separately |
| Add a mutation handler without the read-only gate | Invariant 14 — must mirror `ensure_mutable`/`_ensure_mutable` |
| Force-push / amend published history | Human policy |
| Bump version in one file only | SSOT script fails CI |
| Assume `__TAURI__` always present | `withGlobalTauri: false` |
| Re-open closed audit findings without new evidence | A1–A15, N1–N4 all closed; residuals are documented |

---

## 11. Scorecard (post-1.6.1 + enterprise batch)

| Criterion | Score | One-liner |
|-----------|------:|-----------|
| Core job completeness | 9 | Search → rail → open solid |
| Safety / data-loss | 9 | Purge confirm; WAL; backup script; read-only gate |
| Correctness of mutations | 9 | ETag/If-Match settings; typed collection errors; readonly gate tested |
| Performance | 8 | Load harness + slope + dual-stack benchmarks (Rust ≈573 rps/p50 23 ms; Python ≈295 rps/p50 39 ms) |
| Usability | 8.5 | Spotlight focus; keyboard rail; recovery UX |
| Recoverability | 8.5 | Partial fanout, wiki fallback, hide-to-tray, DB backup/restore |
| Architecture | 8.5 | Clear modules; dual-stack residual cost |
| Code quality | 8.5 | Small surface; typed errors; clippy clean |
| Tests | 9 | 162 pytest + 113 cargo + CSS layout guard + webview E2E + live parity smokes |
| Security (model-fit) | 9 | Open-URL + DNS pin + optional token/audit/readonly |
| Docs / claims | 9 | Audit truth waves landed; residuals honest |
| Packaging | 8.5 | CI primary; cosign-signed releases; systemd unit |
| **Overall** | **~8.8** | Soft GA + active hardening |

---

## 12. Copy-paste resume prompt (OpenCode)

```
You are continuing NetRail (Linux local research console).

READ FIRST:
  docs/HANDOFF_OPENCODE_2026-08-02.md
  HANDOVER.md
  docs/AUDIT_ARCH_2026-08-01.md (closed findings) + docs/AUDIT_OPENCODE_ADVERSARIAL_2026-08-01.md (residuals only)

Version: 1.6.2 SSOT. HEAD = 536d32e, main == origin/main (everything pushed), tree CLEAN.
Releases: v1.6.1, v1.6.2 published (cosign-signed). [Unreleased] holds E2 SBOM-in-bundle,
  E5 fixture growth, E3 CSS guard → next cut is 1.6.3 (see handoff §9 R3).

Stack:
  Rust Axum primary + Tauri 2 desktop; Python FastAPI for Docker/Flatpak/tests.
  UI = netrail/static vanilla JS (NO React, NO PySide6).
  withGlobalTauri: false → prefer window.eval bridges.

Invariants:
  localhost-only :7421; no telemetry; open-URL blocks (encoded loopback, private, rebinding, DDG uddg,
  trailing-dot normalization, DNS pin at open A15); no Brave key on disk; version SSOT via
  scripts/check-versions.sh; dual-stack security in both languages; typed errors {code,detail,status}
  (incl. READONLY_MODE); NETRAIL_READONLY=1 gates ALL mutations; packaged static under /usr/share/netrail/static/.

Bootstrap:
  bash scripts/check-versions.sh
  source .venv/bin/activate && pytest tests/ -q
  cd src-tauri && cargo clippy --all-targets -- -D warnings && cargo test
  cargo build --release --bin netrail-api --no-default-features
  NETRAIL_RATE_LIMIT=0 bash scripts/e2e-api-smoke.sh && bash scripts/parity-api-smoke.sh

Remaining work: see handoff §9 — P1 release cut 1.6.2; P2 load test / SBOM-in-bundle / CSS snapshot /
  tray left-click / fixture growth; P3 accepted residuals (do not build without being asked).

Must not break: search/open, static packaging, Fernet interop, error shape, read-only gate coverage.

Out of scope unless asked: owned corpus, local AI, multi-user auth productization, non-Linux.

Do not force-push. Do not amend published history. Commit + push when the human asks for the batch.
```

---

## 13. Doc index (quick)

| Doc | Role |
|-----|------|
| [README.md](../README.md) | Install + pitch |
| [HANDOVER.md](../HANDOVER.md) | Zero-context freeze resume |
| [SECURITY.md](../SECURITY.md) | Threat model |
| [CHANGELOG.md](../CHANGELOG.md) | Semver history (1.6.2 released; [Unreleased] = E2/E3/E5) |
| [docs/ARCHITECTURE.md](ARCHITECTURE.md) | Lifecycle / design roadmap (current state + next milestones) |
| [docs/DISTRIBUTION.md](DISTRIBUTION.md) | Packaging + env table + systemd + backup/restore |
| [docs/MANUAL.md](MANUAL.md) | End-user manual |
| [docs/API_ERRORS.md](API_ERRORS.md) | Error codes (incl. `READONLY_MODE`) |
| [docs/RELEASE_v1.6.0.md](RELEASE_v1.6.0.md) | 1.6.0 notes |
| [docs/AUDIT_ARCH_2026-08-01.md](AUDIT_ARCH_2026-08-01.md) | Architecture audit (A1–A15 closed) |
| [docs/AUDIT_OPENCODE_ADVERSARIAL_2026-08-01.md](AUDIT_OPENCODE_ADVERSARIAL_2026-08-01.md) | Adversarial audit (N1–N4 closed, residuals register) |
| [docs/HANDOFF_OPENCODE_2026-08-01.md](HANDOFF_OPENCODE_2026-08-01.md) | Previous handoff (historical) |
| **This file** | OpenCode handoff (current) |

---

## 14. Honest closing

NetRail 1.6.1 is a **mature single-user product** for its model: link-first search, hardened open-URL pipeline (syntax + DNS pin), optional operator controls (token, audit, strict backends, read-only, JSON logs), cosign-signed releases, and dual entrypoints (desktop + headless). The two audit passes closed every P1/P2 security finding; what remains is a short, explicit list: one release cut (1.6.2), four concrete backlog items (load test, SBOM-in-bundle, CSS snapshot, tray left-click, fixture growth), and accepted residuals that are documented and should not be built without an explicit request.

OpenCode should treat **§9 as the work plan**, preserve dual-stack discipline on any security touch, keep the read-only gate on every new mutation handler, and refuse scope that turns NetRail into multi-tenant SaaS.

---

*Handoff for OpenCode / human continuity — NetRail 1.6.1 — 2026-08-02 — be honest, no scope creep, prefer durable repo state.*
