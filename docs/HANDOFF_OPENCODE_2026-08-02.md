# NetRail — Enterprise Analysis & OpenCode Handoff (2026-08-02)

| Field | Value |
|-------|--------|
| **Audience** | OpenCode / any zero-context coding agent |
| **Product** | NetRail — local privacy-first research console (Linux) |
| **Version (SSOT)** | **1.6.1** (`scripts/check-versions.sh`, 5 files) |
| **License** | AGPL-3.0 |
| **Repo** | https://github.com/kayab999/NetRail |
| **Handoff date** | 2026-08-02 (supersedes `HANDOFF_OPENCODE_2026-08-01.md`) |
| **Audit refresh** | 2026-08-02 post-v1.6.3 — findings **NR-01..NR-07** in §6A (remediations deferred) |
| **HEAD (at audit)** | `bc63068` — `docs: post-1.6.3 refresh — release closed (R3/R4), backlog now don't-build only` |
| **Branch** | `main` **in sync with `origin/main`** (at audit start) |
| **Working tree** | Audit wrote **only** this handoff (§6A / §9 P0 / resume prompt); remediations not started |
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
| **Overall (v1 desktop product)** | **~8.6** | Soft GA; post-1.6.3 audit found 2×P2 (NR-01 rate-limit hole, NR-02 non-atomic settings) — see §6A |

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

### 5.5 Batch 5 — v1.6.3 release (tag `v1.6.3`)

- **Scope discipline:** strictly reproducibility & supply chain (E2 SBOM-in-bundle, E5 fixtures, E3 CSS guard) — no architecture, no new APIs, no security-model changes.
- **`docs/RELEASE_ASSURANCE.md`** (new): non-technical trust map (security / resilience / concurrency / performance / quality / supply chain with evidence anchors) + release-identity table.
- SSOT 1.6.2→1.6.3 in 5 files; CHANGELOG [1.6.3]; `docs/RELEASE_v1.6.3.md`; commit `f8645c6`.
- **Live finding on first tag:** the new SBOM-in-bundle verify step failed — `scripts/generate-sbom.sh` emitted `generated=<date>` so the two SBOM generations in the job differed by a timestamp (`cmp` failed). Fixed: deterministic commit-SHA provenance (SOURCE_DATE_EPOCH first), `b68e4d9`; re-tagged; workflow green.
- **Post-verify:** assets rpm/deb/AppImage/netrail-api/SBOM.txt/SHA256SUMS(.sig/.pem); deb + AppImage bundled `SBOM.txt` byte-identical to the release asset; `netrail-api --sbom` == SBOM Rust section (570 pkgs, `netrail@1.6.3`); SHA256SUMS integrity checked; cosign keyless **"Verified OK"** in-job.
- Docs refresh `7a47510`/this edit: HANDOVER HEAD note + doc index + `RELEASE_ASSURANCE.md`; handoff R3/R4 closed; gates **162 pytest · 113 cargo**.

**The 08-01 handoff's Spotlight/CSS WIP shipped in the 1.5.0 series** (tray focus places caret in `#query`; result-card grid `minmax(0,1fr) auto`) — no longer a continuity surface.

---

## 6. Findings backlog (audit residuals)

Source of truth: [AUDIT_ARCH_2026-08-01.md](AUDIT_ARCH_2026-08-01.md) (A1–A15 all closed) + [AUDIT_OPENCODE_ADVERSARIAL_2026-08-01.md](AUDIT_OPENCODE_ADVERSARIAL_2026-08-01.md) (N1–N4 closed) + **§6A below (fresh enterprise audit 2026-08-02 post-1.6.3)**. Do **not** re-open closed A/N items without new evidence.

| Status | Items |
|--------|--------|
| **Closed (1.4.x–1.6.3)** | SEC-01/02/03/06/07/08/13, PAR-01/02/03/04/07, A1–A15, N1–N4, R8/OPS-03 (webview E2E), matrix #10 (CI audits + signing), enterprise gaps (systemd, backup, read-only, log-json, audit rotation, schema versioning), E2/E3/E5 |
| **NEW open (this audit — §6A)** | **NR-01** (P2 rate-limit hole on collection item add), **NR-02** (P2 non-atomic settings write + false “atomic” claim), **NR-03** (P3 no audit on collection item add), **NR-04** (P3 readonly still writes history on search/open), **NR-05** (P3 token `==` not constant-time), **NR-06** (P3 doc drift CONTEXT_DUMP), **NR-07** (P3 Unicode host residual class) |
| **Still residual (accepted, documented)** | Time-based TTL-flip DNS rebinding (Q16); image CDN loads R7 / SEC-2026-12; unauth localhost default SEC-2026-09 (token optional); FTS plaintext SEC-2026-10; fixed-window rate limits SEC-2026-11 (per-identity since 1.6.0); dual-stack drift cost R3 (parity harness mitigates) |
| **Backlog (don't build unless asked)** | C3 DNS "resolve-and-warn" flag; C4 images-off flag; multi-user/RBAC; egress proxy/TLS pinning for backends; metrics/SLO; Windows/macOS ports |

**Do not expand scope into:** multi-user OAuth, owned crawl corpus, local LLM, non-Linux ports — unless human explicitly asks.

---

## 6A. Enterprise audit + Q&A — 2026-08-02 (post-v1.6.3) — OPEN findings for OpenCode

| Field | Value |
|-------|--------|
| **Tree** | SSOT **1.6.3**, HEAD `bc63068`, `main` == `origin/main`, clean |
| **Method** | Full security-critical re-walk (auth, open-URL, DNS pin, rate limit, readonly, settings, history, UI XSS sinks, dual-stack parity) + live unit gates + adversarial URL probes |
| **Gates re-run** | `check-versions.sh` OK · **162 pytest passed** · **cargo test --lib 83 passed** |
| **Verdict** | Soft-GA product still holds. **No P0/P1 security regression** vs closed A1–A15/N1–N4. **Two P2 correctness/ops bugs** (rate-limit hole; non-atomic settings) need dual-stack fix. Remainder P3 hygiene/docs/residuals. |
| **Scope of this pass** | Audit only — remediations deferred (OpenCode / next session) |

### 6A.1 Scorecard (delta vs handoff §11)

| Dimension | Score | Notes |
|-----------|------:|-------|
| Core job | 9 | Unchanged |
| Security (model-fit) | 8.5 | −0.5: mutate rate-limit incomplete; settings durability overstated |
| Correctness / tests | 8.5 | Gates green; gap not covered by tests |
| Ops / packaging | 8 | Non-atomic settings is ops reliability |
| Docs truth | 8 | CONTEXT_DUMP stale; AUDIT_ARCH “atomic write” claim false |
| Enterprise multi-user | 3.5 | Out of model |
| **Overall** | **~8.6** | Still soft GA; short fix list |

### 6A.2 Open findings (actionable)

| ID | Sev | Finding | Evidence | Impact | Remediation (OpenCode) | Acceptance |
|----|-----|---------|----------|--------|------------------------|------------|
| **NR-01** | **P2** | **`POST /api/collections/{id}/items` skips mutate rate limit (dual-stack)** | Rust `server/mod.rs` `add_collection_item` (~646–679): has `ensure_mutable` + URL/title validation, **no** `check_mutate`, **no** `HeaderMap`/`request_identity`. Python `main.py` `add_collection_item` (~562–572): `_ensure_mutable` only — **no** `rate_limit.check_mutate`, no `Request`. Contrast: `create_collection` / purge / settings all call `check_mutate`. | Localhost client (or same-user malware) can flood collection inserts beyond 60/min mutate budget; rate-limit invariant incomplete; audit story incomplete (see NR-03). | Dual-stack: add `check_mutate(request_identity)` before store write; Python take `Request` like create. Add API tests that force mutate limit=1 then second add → 429 `RATE_LIMITED`. | Both stacks return 429 after mutate cap; create still works; readonly still 403 |
| **NR-02** | **P2** | **Settings persist is not atomic; docs claim atomic** | Rust `config.rs` `save_settings`: `fs::write(config_file(), …)` in place. Python `config.py` `save_settings`: `Path.write_text(...)`. AUDIT_ARCH §1.3 says “atomic file write”. | Crash/power loss mid-write can leave truncated/corrupt `settings.json` → next load falls back to defaults (or partial JSON parse fail → defaults), silent config loss. | Write temp file in same dir + `rename`/`os.replace` (POSIX atomic). Dual-stack. Fix AUDIT_ARCH / any “atomic” claim. Optional chaos: kill mid-write. | Kill during save cannot leave unreadable partial as final path; rename-replace pattern present both stacks |
| **NR-03** | **P3** | **No audit log event on collection item add** | `collection.create` audited; `add_collection_item` returns without `audit::log_event` / `audit.log_event`. | Incomplete audit trail when `NETRAIL_AUDIT_LOG=1`. | Add `collection.item.add` with host/name lens only (no full URL if policy prefers hosts; match open-event style). Dual-stack. | Audit line appears when audit enabled; still no secrets/queries |
| **NR-04** | **P3** | **`NETRAIL_READONLY=1` still mutates DB on search/open** | Docs correctly list gated endpoints (settings, history delete/purge, collection create/add). `run_search` still `record_search`; `open_link` still `record_visit`. Readonly tests only assert GET health/settings/history/collections. | Operator expectation “read-only = no writes” is wrong; disk grows under “readonly”. Product decision, not code bug vs docs. | Either document explicitly in SECURITY/DISTRIBUTION/API_ERRORS (“history still recorded”) **or** optionally skip history writes when readonly (product call). | Doc sentence present, or search/open no longer write when readonly + tests |
| **NR-05** | **P3** | **API token compare is not constant-time** | Rust/Python auth: `bearer.trim() == expected` / `==`. | Theoretical timing leak of token on localhost; low practical risk. | `subtle`/`hmac.compare_digest` dual-stack. | Uses constant-time compare; tests still pass |
| **NR-06** | **P3** | **Doc drift: `CONTEXT_DUMP_2026-08-02.md`** | Claims SSOT **1.6.1**, Sprint 2 “UP NEXT”; product is **1.6.3**, sprints 2–4 shipped. | Agents bootstrap wrong residual map. | Refresh dump or mark superseded by this handoff §6A. | Version + sprint status match 1.6.3 reality |
| **NR-07** | **P3** residual | **Unicode / homoglyph hosts pass `validate_open_url`; rely on DNS pin** | Live Python: `http://127。0。0。1/`, `http://①②⑦.0.0.1/` → ALLOW at validate; pin with empty → `OPEN_URL_DNS_UNRESOLVABLE`; pin with 127.0.0.1 → `OPEN_URL_LOCALHOST`. | Same residual class as Q16 if browser normalizes differently than system resolver. | Do **not** expand scope unless asked; document next to Q16 if touching SECURITY.md. | Documented residual only |

### 6A.3 Q&A evaluation (enterprise checklist)

| # | Question | Answer | Evidence |
|---|----------|--------|----------|
| Q1 | Localhost-only bind preserved? | **Yes** | `TcpListener::bind(127.0.0.1:7421)`; compose publishes `127.0.0.1:7421` |
| Q2 | Open-URL SSRF class closed (encoded loopback, private, rebinding, DDG, metadata)? | **Yes** (syntax + pin) | Fixture 68 vectors; DNS pin A15; 162 pytest green |
| Q3 | Typed errors on 422 / validation? | **Yes** (closed A1) | Python `RequestValidationError` mapper; Rust JsonRejection path (prior closed) |
| Q4 | Rate limits complete on all mutates? | **No — NR-01** | collection item add missing |
| Q5 | Readonly gates all documented mutations? | **Yes** for listed endpoints | 6 Rust + 2 Python tests; search/open still write (NR-04 semantic) |
| Q6 | Dual-stack security parity on open policy? | **Yes** for fixture set | Percent-encoded loopback, trailing-dot, ftp scheme aligned |
| Q7 | Settings durable under crash? | **No — NR-02** | non-atomic write |
| Q8 | Secrets on disk? | **No** Brave key env-only; Fernet key keyring/env | config + crypto modules |
| Q9 | UI XSS from results/history? | **Mostly safe** | `escapeHtml` on titles/snippets/history; docs markdown escaped; images load remote HTTPS (R7) |
| Q10 | Telemetry? | **None** | no analytics code paths |
| Q11 | Auth default? | **Off** (by design SEC-2026-09) | token optional |
| Q12 | Test gates healthy? | **Yes** | 162 pytest / 83 lib cargo this pass; handoff claims 113 total cargo incl. integration |
| Q13 | Prior P1 findings reopened? | **No** | A1–A15, N1–N4 still closed with evidence |
| Q14 | Supply chain / SBOM? | **Yes** for 1.6.3 | deterministic SBOM, cosign-signed releases (handoff §5.5) |

### 6A.4 Explicit non-findings (do not re-fix as bugs)

- Trailing-dot / percent-encoded loopback / DDG unwrap / metadata hosts — closed and re-probed.
- CSP token inject (A2) — hash path present; tests for CSP hash.
- SharedStore single connection (A3) — present.
- Graceful shutdown SIGTERM (A4) — present.
- FTS rebuild on delete (A13) — present + tests.
- Read-only gate missing on create/purge/settings — **not** missing; only item-add rate limit missing.

### 6A.5 OpenCode remediation order (when quota allows)

1. **NR-01** dual-stack rate limit + tests (smallest, highest correctness value).  
2. **NR-02** atomic settings write dual-stack + doc claim fix.  
3. **NR-03** audit event (trivial).  
4. **NR-04** doc clarification (or product flag — ask human).  
5. **NR-05** constant-time token (optional hygiene).  
6. **NR-06** CONTEXT_DUMP refresh.  
7. Leave **NR-07** / Q16 / R7 as accepted residuals unless asked.

**Must not break:** search/open, static packaging, Fernet interop, error shape, readonly gate coverage, version SSOT.

---

## 6B. Post-remediation verification — 2026-08-02 (Antigravity + full sweep)

| Field | Value |
|-------|--------|
| **Tree** | SSOT **1.6.4**. Working tree **dirty / uncommitted** until human asks to commit |
| **Full sweep** | NR-01..NR-14 closed in tree (NR-07 residual; NR-15 = commit/release process) |
| **Verdict** | Ready to commit; cut **1.6.4** after commit + CI green. No known open P2. |

### 6B.1 NR checklist (final)

| ID | Status | Evidence |
|----|--------|----------|
| **NR-01** | ✅ | Dual-stack `check_mutate` on collection item add; Python + Rust 429 tests |
| **NR-02/08** | ✅ | Unique temps: Python `mkstemp`; Rust `pid.seq.nanos` + rename; concurrent tests both stacks |
| **NR-03** | ✅ | `collection.item.add` audit dual-stack + Python test |
| **NR-04** | ✅ | SECURITY.md + DISTRIBUTION + API_ERRORS (history still logged under readonly) |
| **NR-05** | ✅ | Constant-time token compare dual-stack; `hmac` top-level import |
| **NR-06** | ✅ | CONTEXT_DUMP SSOT **1.6.4** |
| **NR-07** | ✅ residual | DNS-pin class; documented |
| **NR-09..14** | ✅ | SECURITY, dump version, hygiene, CONFIG_SAVE→500 Internal, Rust save + rate-limit tests |
| **NR-15** | open process | Commit + tag when human requests |

### 6B.2 Residual (accepted / process only)

| Item | Notes |
|------|-------|
| Q16 TTL rebinding, R7 images, unauth localhost default, FTS plaintext | Unchanged accepted residuals |
| 1.6.4 not yet committed/tagged | Process — not a code defect |

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

## 9. Remaining work points (established 2026-08-02; **audit refresh same day**)

### P0 — Full sweep status (see §6B)

| # | Item | Status |
|---|------|--------|
| NR-01..NR-14 | Code + docs + tests | ✅ closed in working tree |
| NR-15 | Commit + push + tag **1.6.4** | ⏳ human request |

### P1 — Release hygiene (next release)

| # | Item | Acceptance |
|---|------|------------|
| ~~R1~~ | ~~Cut the next release from [Unreleased] delta~~ | **✅ Done 2026-08-02** — v1.6.2 cut (read-only mode + systemd unit + backup script) with the standard SSOT bump + tag; workflow green, cosign verified |
| ~~R2~~ | ~~Post-release doc refresh~~ | **✅ Done 2026-08-02** — DISTRIBUTION version headers; HANDOVER HEAD note; CHANGELOG [1.6.2] |
| ~~R3~~ | ~~Cut v1.6.3~~ | **✅ Done 2026-08-02** — reproducibility & supply chain: E2 SBOM-in-bundle + E5 fixture growth + E3 CSS guard + `docs/RELEASE_ASSURANCE.md`. SSOT 1.6.2→1.6.3 in 5 files; CHANGELOG [1.6.3]; tag `v1.6.3`. First live run of the SBOM-in-bundle verify step **failed** on a real finding (generator emitted a `generated=<timestamp>` → the two job generations differed) → fixed to deterministic commit-provenance (`b68e4d9`), re-tagged; green. Assets: rpm/deb/AppImage/netrail-api/SBOM.txt/SHA256SUMS(.sig/.pem); cosign "Verified OK"; deb/AppImage bundled SBOM byte-identical to the asset (rpm verified in CI); binary `--sbom` == SBOM Rust section (570 pkgs, `netrail@1.6.3`) |
| ~~R4~~ | ~~Post-1.6.3 doc refresh~~ | **✅ Done 2026-08-02** — DISTRIBUTION (no version headers needed); HANDOVER HEAD note + doc index; handoff release row + resume prompt; CHANGELOG [1.6.3] |
| R5 | Cut patch **1.6.4** after full-sweep commit + CI green | NR-08 fixed; gates must pass |

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
  docs/HANDOFF_OPENCODE_2026-08-02.md  ← especially §6A (new audit findings NR-01..07) and §9 P0
  HANDOVER.md
  docs/AUDIT_ARCH_2026-08-01.md (closed A1–A15) + docs/AUDIT_OPENCODE_ADVERSARIAL_2026-08-01.md (residuals)

Version: 1.6.3 SSOT. HEAD at audit time = bc63068 (or later if remediations landed).
main == origin/main when clean. Releases: v1.6.1–v1.6.3 cosign-signed.

Stack:
  Rust Axum primary + Tauri 2 desktop; Python FastAPI for Docker/Flatpak/tests.
  UI = netrail/static vanilla JS (NO React, NO PySide6).
  withGlobalTauri: false → prefer window.eval bridges.

Invariants:
  localhost-only :7421; no telemetry; open-URL blocks (encoded loopback, private, rebinding, DDG uddg,
  trailing-dot normalization, DNS pin at open A15); no Brave key on disk; version SSOT via
  scripts/check-versions.sh; dual-stack security in both languages; typed errors {code,detail,status}
  (incl. READONLY_MODE); NETRAIL_READONLY=1 gates ALL mutations; packaged static under /usr/share/netrail/static/.

THIS SESSION / NEXT WORK (audit-only pass already done — remediations):
  P0 from handoff §9 / §6A:
    NR-01 P2: rate-limit POST /api/collections/{id}/items (Rust+Python) + 429 tests
    NR-02 P2: atomic settings write (temp+rename) dual-stack; fix false "atomic" doc claim
    NR-03 P3: audit event on collection item add
    NR-04 P3: document or change readonly vs history writes (ask human before behavior change)
    NR-05 P3: constant-time token compare (optional)
    NR-06 P3: refresh CONTEXT_DUMP version/sprint drift
  Do NOT re-open closed A1–A15 / N1–N4 without new evidence.
  Do NOT build accepted residuals (Q16 rebinding, R7 images, multi-user) unless asked.

Bootstrap:
  bash scripts/check-versions.sh
  source .venv/bin/activate && pytest tests/ -q
  cd src-tauri && cargo clippy --all-targets -- -D warnings && cargo test
  cargo build --release --bin netrail-api --no-default-features
  NETRAIL_RATE_LIMIT=0 bash scripts/e2e-api-smoke.sh && bash scripts/parity-api-smoke.sh

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
| [CHANGELOG.md](../CHANGELOG.md) | Semver history (1.6.3 released) |
| [docs/ARCHITECTURE.md](ARCHITECTURE.md) | Lifecycle / design roadmap (current state + next milestones) |
| [docs/RELEASE_ASSURANCE.md](RELEASE_ASSURANCE.md) | Non-technical trust map |
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

NetRail **1.6.3** is a **mature single-user product** for its model: link-first search, hardened open-URL pipeline (syntax + DNS pin), optional operator controls (token, audit, strict backends, read-only, JSON logs), cosign-signed releases, SBOM-in-bundle, and dual entrypoints (desktop + headless). Prior audits closed A1–A15 and N1–N4.

**This audit (2026-08-02 post-1.6.3)** found **no P0/P1 regressions**, but **two P2 bugs** still open for remediation: incomplete mutate rate-limit on collection item add (**NR-01**), and non-atomic settings persistence despite docs claiming atomic (**NR-02**). P3 hygiene (audit event, readonly semantics docs, constant-time token, CONTEXT_DUMP drift) and accepted residuals (Q16, R7, unauth default) remain.

OpenCode should treat **§6A + §9 P0 as the work plan**, preserve dual-stack discipline, keep the read-only gate on every new mutation handler, and refuse scope that turns NetRail into multi-tenant SaaS.

---

*Handoff for OpenCode / human continuity — NetRail 1.6.3 — audit refresh 2026-08-02 — be honest, no scope creep, prefer durable repo state.*
