# NetRail — Enterprise Analysis & OpenCode Handoff (2026-08-02)

| Field | Value |
|-------|--------|
| **Audience** | OpenCode / any zero-context coding agent |
| **Product** | NetRail — local privacy-first research console (Linux) |
| **Version (SSOT)** | **1.6.4** (`scripts/check-versions.sh`, 5 files) |
| **License** | AGPL-3.0 |
| **Repo** | https://github.com/kayab999/NetRail |
| **Handoff date** | 2026-08-02 (supersedes earlier same-day snapshots) |
| **HEAD** | `ed0603d` — `fix(release): unblock 1.6.4 AppImage (NR-16) — NO_STRIP + Categories fix` |
| **Prior commit** | `a12dbed` — `chore(release): consolidate AppImage-first Linux packaging for 1.6.4` |
| **Branch** | `main` — **ahead of `origin/main`** by 5 commits (A2 parity/fuzz/NR-16 fixes, unpushed) |
| **Working tree** | Clean (unless editing handoff now) |
| **Tag** | **`v1.6.4`** on `a12dbed` (pushed) — **needs re-point to `ed0603d`** (fix commit) once user authorizes |
| **GitHub Release assets** | ⚠️ **Not published yet** — AppImage blocker root-caused + fixed in `ed0603d` (see §9 P0): Categories empty-value bug in desktopTemplate + linuxdeploy strip/`.relr.dyn` on 24.04 (`NO_STRIP`). **Local full-pipeline proof:** `NetRail_1.6.4_amd64.AppImage` built. CI rerun pending tag re-point. |
| **Primary path** | Rust Axum API + Tauri 2 desktop; Python FastAPI for Docker/Flatpak/tests |
| **Official ship artifact** | **AppImage** (CI authority); secondaries `.deb` / `.rpm` / `netrail-api` |
| **Packaging SSOT** | [packaging/README.md](../packaging/README.md) + `scripts/build-desktop-linux.sh` |
| **API bind** | `127.0.0.1:7421` only |
| **UI** | Vanilla HTML/CSS/JS in `netrail/static/` (no React/Vue; no PySide6/Qt) |
| **api_contract** | **1.4** |

**Companion docs (read in order if time-boxed):**

1. This handoff  
2. [HANDOVER.md](../HANDOVER.md) — freeze invariants + resume prompt  
3. [packaging/README.md](../packaging/README.md) — AppImage-first distribution contract  
4. [docs/AUDIT_ARCH_2026-08-01.md](AUDIT_ARCH_2026-08-01.md) — A1–A15 closed  
5. [docs/AUDIT_OPENCODE_ADVERSARIAL_2026-08-01.md](AUDIT_OPENCODE_ADVERSARIAL_2026-08-01.md) — N1–N4 closed + residuals  
6. [SECURITY.md](../SECURITY.md) — threat model (incl. readonly)  
7. [docs/API_ERRORS.md](API_ERRORS.md) — typed errors (incl. `READONLY_MODE`, `CONFIG_SAVE_FAILED`)  
8. [docs/RELEASE_v1.6.4.md](RELEASE_v1.6.4.md) — 1.6.4 notes

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

### 1.1 Product posture (post-1.6.4)

| Dimension | Score (0–10) | Assessment |
|-----------|-------------:|------------|
| Core job (search → rail → open) | **9** | End-to-end path solid on both stacks |
| Threat-model fit (single-user local) | **9** | Localhost API; DNS pin (A15); readonly gate |
| Security / correctness | **9** | NR-01..NR-14 closed (rate-limit hole, atomic settings race, audit, constant-time token, docs) |
| Dual-stack parity | **9** | Fixture + live parity harness |
| Correctness / tests | **9** | 166 pytest, 85 lib + 31 integration cargo (approx), clippy `-D warnings` |
| Ops / packaging **contract** | **9** | AppImage-first SSOT, build script, desktop metadata, XDG documented |
| Ops / packaging **published assets** | **6** | **v1.6.4 GitHub Release not published** — CI AppImage/`linuxdeploy` failed (see §9) |
| Docs truth | **9** | Residuals honest; packaging SSOT live |
| Enterprise multi-user / SOC2-ish | **3.5** | Out of stated model |
| Desktop UX polish | **8** | Keyboard rail, recovery UX, Spotlight, tray/hotkey |
| **Overall (v1 desktop product)** | **~8.8 code / ~8.0 ship** | Product + pipeline defined; **finish release CI** to close the ship gap |

**Verdict:** NetRail 1.6.4 on `main` is a **distributable product contract**: audit closed, dual-stack remediations landed, single official packaging path (Tauri AppImage), XDG data outside the bundle, no telemetry. What remains operational is **making CI publish the v1.6.4 assets** after the `linuxdeploy` failure.

### 1.2 Release / git posture (critical for agents)

| Fact | Detail |
|------|--------|
| Committed product version | **1.6.4** on `main` |
| Commits (traceable stack) | `e436e6d` remediations → `a12dbed` packaging |
| Tag | `v1.6.4` → `a12dbed` (pushed) |
| Remote | `main` == `origin/main` |
| Working tree | Clean after handoff commit |
| GitHub Release page | **missing** until workflow succeeds |
| CI pins | `cargo-audit 0.22.2`; cosign verify needs identity-regexp + OIDC issuer |
| Policy | No force-push; no amend of published history; version SSOT 5 files |

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
| **Read-only mode** | ✅ `NETRAIL_READONLY=1` → `403 READONLY_MODE` on admin mutations; search/visit history still recorded (documented) |
| Systemd unit + DB backup | ✅ `packaging/netrail-api.service`, `scripts/backup-db.sh` |
| SBOM / dep audit in CI | ✅ Embedded `--sbom` + package path; audits gated |
| AppImage-first packaging SSOT | ✅ `packaging/README.md`, `scripts/build-desktop-linux.sh`, desktop template |
| v1.6.4 signed GitHub assets | ⚠️ **Fix landed** `ed0603d` (NR-16); CI rerun pending tag re-point |
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

## 6B. Post-remediation + packaging — **closed on main** (2026-08-02)

| Field | Value |
|-------|--------|
| **Tree** | SSOT **1.6.4**, HEAD `a12dbed`, `main` == `origin/main` |
| **Commits** | `e436e6d` NR-01..NR-14 · `a12dbed` AppImage-first packaging |
| **Tag** | `v1.6.4` pushed |
| **Verdict** | Code + packaging **contract** shipped to git. **GitHub Release assets not published** (CI fail). |

### 6B.1 NR checklist (final)

| ID | Status | Evidence |
|----|--------|----------|
| **NR-01** | ✅ on main | Dual-stack `check_mutate` on collection item add; Python + Rust 429 tests |
| **NR-02/08** | ✅ on main | Unique temps (`mkstemp` / `pid.seq.nanos`); concurrent tests |
| **NR-03** | ✅ on main | `collection.item.add` audit dual-stack |
| **NR-04** | ✅ on main | SECURITY + DISTRIBUTION + API_ERRORS (history still logged under readonly) |
| **NR-05** | ✅ on main | Constant-time token compare |
| **NR-06** | ✅ on main | CONTEXT_DUMP **1.6.4** |
| **NR-07** | ✅ residual | DNS-pin / homoglyph class; documented |
| **NR-09..14** | ✅ on main | Docs, hygiene, CONFIG_SAVE→500, Rust tests |
| **NR-15** | ✅ git | Committed + tagged + pushed |
| **NR-16** | ✅ fixed `ed0603d` (pending CI rerun) | AppImage blocker root-caused: desktopTemplate Categories empty-value + linuxdeploy strip/`.relr.dyn`; local pipeline proof — see §9 P0 |

### 6B.2 Packaging contract (official path)

| Item | Path / command |
|------|----------------|
| SSOT | `packaging/README.md` |
| One-shot build | `bash scripts/build-desktop-linux.sh` (`npm run package:linux`) |
| Desktop template | `packaging/linux/netrail.desktop.hbs` |
| Tauri metadata | `bundle.category: Utility`, short/longDescription |
| Legacy non-ship | `packaging/appimage/build.sh` (PyInstaller) — **do not ship** |
| User data | XDG only: `~/.config/netrail/`, `~/.local/share/netrail/` |

### 6B.3 Accepted residuals (unchanged)

Q16 TTL rebinding · R7 image CDN · unauth localhost default · FTS plaintext · fixed-window rate limits · dual-stack cost · multi-user out of scope.

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

# Official Linux release tree → dist/release/
bash scripts/build-desktop-linux.sh
# or: npm run package:linux
# AppImage needs patchelf; without it expect AppImage fail (deb/api may still build)
```

Gates at 1.6.4 land: **166 pytest · ~85 cargo lib · ~31 cargo integration · clippy `-D warnings`**.  
Release CI (tag `v1.6.4`): `failed to run linuxdeploy` — **fixed** `ed0603d` (NO_STRIP + Categories); local AppImage proof; CI rerun pending (tag re-point, §9 P0).

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

## 9. Remaining work points (post-1.6.4 push)

### P0 — Unblock GitHub Release for v1.6.4 (**do this next**)

| # | Item | Status / notes |
|---|------|----------------|
| **NR-16** | Fix Release workflow AppImage step | Run `30771210870` failed `failed to run linuxdeploy` after deb+rpm. **Root-caused + fixed** `ed0603d` (2026-08-09): (1) desktopTemplate rendered `Categories=Utility;;Network;` — hardcoded suffix + tauri's trailing-`;` category var = empty value → appimagetool/desktop-file-validate hard-rejects; template now concatenates `{{categories}}Network;` (`Utility;Network;` / `Network;` when empty). (2) linuxdeploy strip chokes on `.relr.dyn` sections on ubuntu-24.04 libs (upstream tauri#14796/#8929/#13113) → `NO_STRIP: true` added to release.yml build env + `scripts/build-desktop-linux.sh`. **Local proof:** first-ever `NetRail_1.6.4_amd64.AppImage` built (90.6 MB); Categories validated inside squashfs; deb/rpm/SHA256SUMS regenerated. |
| R5a | Re-run or re-tag after fix | Patch landed (`ed0603d`): push main → **re-point tag `v1.6.4` to the fix commit** (delete+recreate locally, `--force` push of the tag only; policy allows tag force as the release is unpublished) → Release workflow rebuilds all assets. **No force-push of main.** Target assets: AppImage, deb, rpm, netrail-api, SBOM, SHA256SUMS, cosign. |
| R5b | Post-release doc refresh | After green: HANDOVER HEAD + handoff “assets published”; optional README pin to exact artifact names. |

**Resolution record (2026-08-09, `ed0603d`):** reproduced `failed to run linuxdeploy`
locally via `bash scripts/build-desktop-linux.sh --skip-tests` (patchelf 0.18 + librsvg
`.pc` shim); `--verbosity 2` pinpointed both failures:

1. **AppImage `.desktop` invalid** — template `Categories={{categories}};Network;`
   rendered `Utility;;Network;` (tauri passes `Utility;` already trailing `;`) → empty
   value → appimagetool rejects. Fixed: `Categories={{categories}}Network;`.
2. **linuxdeploy strip on `.relr.dyn`** (ubuntu-24.04 libs, upstream
   tauri#14796/#8929/#13113) — fixed: `NO_STRIP: true` in release.yml + script.

Local proof: `NetRail_1.6.4_amd64.AppImage` (90.6 MB) — first-ever for 1.6.4 — built,
extracted, `Categories=Utility;Network;` verified inside squashfs; dist/release
regenerated (deb/rpm/SBOM/SHA256SUMS).

```bash
gh run view 30771210870 --log-failed     # deb+rpm OK, linuxdeploy ran 59s then generic error
# Rerun recipe: git push origin main && git tag -f v1.6.4 && git push -f origin v1.6.4
# (tag force OK — release unpublished; NO force-push of main)
```

### P1 — Release hygiene (history)

| # | Item | Acceptance |
|---|------|------------|
| ~~R1–R4~~ | 1.6.2 / 1.6.3 cuts + doc refresh | ✅ Done earlier 2026-08-02 |
| ~~R5 code~~ | 1.6.4 remediations + packaging on main + tag | ✅ `e436e6d` + `a12dbed` + tag pushed |
| R5 assets | Publish signed GitHub Release for `v1.6.4` | ⚠️ **open — NR-16** |

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

## 11. Scorecard (post-1.6.4 on main)

| Criterion | Score | One-liner |
|-----------|------:|-----------|
| Core job completeness | 9 | Search → rail → open solid |
| Safety / data-loss | 9 | Purge confirm; WAL; backup; readonly gate |
| Correctness of mutations | 9 | ETag; rate-limit on all mutates incl. collection items; concurrent-safe settings |
| Performance | 8 | Dual-stack benches on record |
| Usability | 8.5 | Spotlight; keyboard rail; recovery UX |
| Recoverability | 8.5 | Fanout partial, wiki fallback, tray, DB backup |
| Architecture | 8.5 | Clear modules; dual-stack residual cost |
| Code quality | 8.5 | Typed errors; clippy clean |
| Tests | 9 | Expanded pytest/cargo + concurrent settings tests |
| Security (model-fit) | 9 | Open-URL + DNS pin + token/audit/readonly |
| Docs / claims | 9 | Packaging SSOT + honest CI failure note |
| Packaging contract | 9 | AppImage-first defined; legacy path excluded |
| Packaging published | 6 | **v1.6.4 Release assets blocked on linuxdeploy** |
| **Overall** | **~8.7 code / ship pending** | Product ready; finish NR-16 |

---

## 12. Copy-paste resume prompt (OpenCode)

```
You are continuing NetRail (Linux local research console).

READ FIRST:
  docs/HANDOFF_OPENCODE_2026-08-02.md  ← §6B closed work + §9 P0 (NR-16 release CI)
  packaging/README.md                  ← AppImage-first SSOT
  HANDOVER.md
  docs/AUDIT_ARCH_2026-08-01.md + docs/AUDIT_OPENCODE_ADVERSARIAL_2026-08-01.md (residuals only)

Version: 1.6.4 SSOT. HEAD = a12dbed. main == origin/main. Tag v1.6.4 pushed.
Commits: e436e6d (NR-01..14 remediations) → a12dbed (packaging consolidation).
GitHub Release for v1.6.4: NOT published — workflow 30771210870 FAILED at
  "failed to run linuxdeploy" during AppImage bundle (deb/rpm had started OK).

Stack:
  Rust Axum primary + Tauri 2 desktop; Python FastAPI for Docker/Flatpak/tests.
  UI = netrail/static vanilla JS (NO React, NO PySide6).
  withGlobalTauri: false → prefer window.eval bridges.
  Official ship path: Tauri AppImage (NOT packaging/appimage PyInstaller).

Invariants:
  localhost-only :7421; no telemetry; open-URL + DNS pin A15; no Brave key on disk;
  version SSOT (5 files); dual-stack security; typed errors {code,detail,status};
  NETRAIL_READONLY gates admin mutations (history still recorded on search/open);
  XDG data outside bundle; packaged static under /usr/share/netrail/static/.

NEXT WORK (P0):
  NR-16: fix Release CI AppImage/linuxdeploy so v1.6.4 assets publish
    (AppImage, deb, rpm, netrail-api, SBOM, SHA256SUMS, cosign).
  Do NOT re-open closed A1–A15 / N1–N4 / NR-01..15 without new evidence.
  Do NOT build accepted residuals (Q16, R7, multi-user) unless asked.
  After green: post-release handoff/HANDOVER refresh only.

Bootstrap:
  bash scripts/check-versions.sh
  source .venv/bin/activate && pytest tests/ -q
  cd src-tauri && cargo clippy --all-targets -- -D warnings && cargo test
  bash scripts/build-desktop-linux.sh --skip-tests   # needs patchelf for AppImage
  gh run view 30771210870 --log-failed

Must not break: search/open, static packaging, Fernet interop, error shape, read-only gate.

Out of scope unless asked: owned corpus, local AI, multi-user, non-Linux.
Do not force-push. Do not amend published history.
```

---

## 13. Doc index (quick)

| Doc | Role |
|-----|------|
| [README.md](../README.md) | Install + pitch (AppImage-first 1.6.4) |
| [HANDOVER.md](../HANDOVER.md) | Zero-context freeze resume |
| [packaging/README.md](../packaging/README.md) | **Packaging SSOT** |
| [SECURITY.md](../SECURITY.md) | Threat model + readonly |
| [CHANGELOG.md](../CHANGELOG.md) | Semver history (**1.6.4** on main) |
| [docs/ARCHITECTURE.md](ARCHITECTURE.md) | Design roadmap |
| [docs/RELEASE_ASSURANCE.md](RELEASE_ASSURANCE.md) | Trust map |
| [docs/DISTRIBUTION.md](DISTRIBUTION.md) | Ops, env, systemd, backup |
| [docs/MANUAL.md](MANUAL.md) | End-user manual |
| [docs/API_ERRORS.md](API_ERRORS.md) | Error codes |
| [docs/RELEASE_v1.6.4.md](RELEASE_v1.6.4.md) | 1.6.4 notes |
| [docs/AUDIT_ARCH_2026-08-01.md](AUDIT_ARCH_2026-08-01.md) | A1–A15 closed |
| [docs/AUDIT_OPENCODE_ADVERSARIAL_2026-08-01.md](AUDIT_OPENCODE_ADVERSARIAL_2026-08-01.md) | N1–N4 + residuals |
| **This file** | OpenCode handoff (current) |

---

## 14. Honest closing

NetRail **1.6.4 on `main`** is no longer “a version with fixes only”: it is a **distributable product with an operational contract** — audit remediations closed (NR-01..NR-14), AppImage-first packaging SSOT, desktop metadata, XDG boundaries, CI as publish authority.

What is **not** done yet is the **published GitHub Release** for `v1.6.4`: workflow `30771210870` failed at AppImage/`linuxdeploy` after deb/rpm bundling started. Code and tag are on the remote; **assets and cosign signatures are not**.

Next agent work is **NR-16** (unblock release), not feature work. Keep dual-stack discipline; do not revive PyInstaller as the ship path; do not re-open closed audit items without new evidence.

---

*Handoff for OpenCode / human continuity — NetRail 1.6.4 on main — 2026-08-02 — be honest: code shipped, Release CI still red (NR-16).*
