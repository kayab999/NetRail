# NetRail — Enterprise Analysis & OpenCode Handoff

| Field | Value |
|-------|--------|
| **Audience** | OpenCode / any zero-context coding agent |
| **Product** | NetRail — local privacy-first research console (Linux) |
| **Version (SSOT)** | **1.4.0** (`scripts/check-versions.sh`) |
| **License** | AGPL-3.0 |
| **Repo** | https://github.com/kayab999/NetRail |
| **Handoff date** | 2026-08-01 |
| **HEAD (committed)** | `7d0daef` — `feat(1.4.0): Waves 3–5 optional token, audit, Docker Rust, parity harness` |
| **Branch** | `main` **ahead of `origin/main` by 5 commits** (not pushed) |
| **Working tree** | **Dirty** — 3 files uncommitted (desktop Spotlight UX + result-card CSS) |
| **Primary path** | Rust Axum API + Tauri 2 desktop; Python FastAPI for Docker/Flatpak/tests |
| **API bind** | `127.0.0.1:7421` only |
| **UI** | Vanilla HTML/CSS/JS in `netrail/static/` (no React/Vue; no PySide6/Qt) |
| **api_contract** | **1.4** |

**Companion docs (read in order if time-boxed):**

1. This handoff  
2. [HANDOVER.md](../HANDOVER.md) — freeze invariants + resume prompt  
3. [docs/AUDIT_ENTERPRISE_2026-07-31.md](AUDIT_ENTERPRISE_2026-07-31.md) — adversarial findings + waves  
4. [SECURITY.md](../SECURITY.md) — threat model  
5. [docs/API_ERRORS.md](API_ERRORS.md) — typed error contract  

---

## 0. OpenCode mission briefing (30 seconds)

You are continuing **NetRail**, a single-user Linux research console: **query → fanout search → link rail → user opens chosen URL**.

- **Not** multi-tenant SaaS, remote-auth product, Google replacement, or owned crawl corpus (roadmap only).  
- **Not** a Qt/PySide GUI — UI is **static web** inside Tauri webview (or browser against the API).  
- **Production engine:** Rust. **Python:** parity surface for tests/Docker/Flatpak/`install.sh`.  
- **Do not** force-push. **Do not** push unless the human asks.  
- Prefer durable, minimal, dual-stack-aware changes. No scope creep.

---

## 1. Executive verdict (enterprise-grade)

### 1.1 Product posture

| Dimension | Score (0–10) | Assessment |
|-----------|-------------:|------------|
| Core job (search → rail → open) | **9** | End-to-end path solid on both stacks |
| Threat-model fit (single-user local) | **8.5** | Honest SECURITY.md; localhost API by design |
| Security vs local attacker | **6.5** | Optional token (1.4.0) improves posture; default still open on loopback |
| Dual-stack parity | **8** | Waves 1–2 closed major gaps; residual drift cost remains |
| Correctness / tests | **8.5** | Strong unit + URL fixtures + API smokes; no Tauri webview E2E |
| Ops / packaging | **8** | deb/rpm/AppImage/CI; Rust Docker profile added |
| Docs truth | **8.5** | Wave 0 + 4 fixed major false claims |
| Enterprise multi-user / SOC2-ish | **3.5** | Out of stated model; token + audit log are optional footholds only |
| Desktop UX polish | **8** | Keyboard rail, recovery UX; tray Spotlight + card layout mid-flight |
| **Overall (v1 desktop product)** | **~8.5** | Soft GA for stated model; post-1.4.0 hardening + UX polish |

**Verdict:** NetRail is a **credible single-user localhost research console** with production-quality open-URL controls, typed errors, rate limits, optional auth/audit, and a Rust-primary engine. It is **not** enterprise multi-user software. Residual risk clusters around (1) unauthenticated default localhost API, (2) dual-stack maintenance cost, (3) DDGS/provider fragility, (4) no webview E2E.

### 1.2 Release / git posture (critical for agents)

| Fact | Detail |
|------|--------|
| Committed product version | **1.4.0** on `main` |
| Remote | Local `main` is **5 commits ahead** of `origin/main` — **push not done** |
| Uncommitted work (this session) | `src-tauri/src/desktop.rs`, `netrail/static/app.js`, `netrail/static/style.css` |
| Tag status | Confirm on GitHub before assuming public `v1.4.0` ships; local tree claims 1.4.0 SSOT |
| Policy | No force-push; no unsolicited push/publish |

### 1.3 What “enterprise” means here

Enterprise *readiness for the stated threat model* ≠ multi-tenant SaaS. For NetRail:

| Control | Status |
|---------|--------|
| Localhost-only bind | ✅ Hard invariant |
| Open-URL SSRF-class guards | ✅ Encoded loopback, private IP, rebinding apex, DDG unwrap, metadata hosts |
| Typed API errors `{code,detail,status}` | ✅ Dual-stack |
| Rate limits (search/open/mutate) | ✅ |
| Optional API token | ✅ `NETRAIL_API_TOKEN` (default off) |
| Audit log (JSONL) | ✅ Opt-in |
| Strict backend URLs | ✅ Opt-in (homelab vs cloud split) |
| SBOM / dep audit in CI | ✅ Wave 5 |
| Formal SDL / multi-user RBAC | ❌ Out of scope |
| SOC2 evidence pack | ❌ Not applicable as product goal |

---

## 2. Architecture (authoritative)

### 2.1 Runtime topology

```
┌──────────────────────────────────────────────────────────────────────────┐
│  ENTRY                                                                   │
│  • netrail          Tauri 2 desktop (feature desktop)                    │
│  • netrail-api      Headless Rust binary (--no-default-features OK)      │
│  • python -m netrail  FastAPI fallback (Docker/Flatpak/tests)            │
│  • Docker Compose   Python default; profile rust → Dockerfile.rust       │
└────────────────────────────────┬───────────────────────────────────────┘
                                 ▼
                    HTTP 127.0.0.1:7421
         Axum (Rust primary)  or  FastAPI (Python)
                                 │
     ┌───────────┬───────────────┼────────────────┬────────────────┐
     ▼           ▼               ▼                ▼                ▼
 static UI   /api/search    /api/open      history SQLite    settings
 app.js      fanout+merge   browsers.*     Fernet/keyring    XDG JSON
 markdown    ddgs|searxng   validate_url   FTS5 tokens       config.*
             brave|wiki     argv spawn     (plaintext FTS)
```

### 2.2 Frontend truth (do not invent stacks)

| Layer | Technology | Path |
|-------|------------|------|
| UI | **Vanilla** `index.html` + `app.js` + `style.css` + `markdown.js` | `netrail/static/` |
| Desktop shell | **Tauri 2** webview → loads `http://127.0.0.1:7421` | `src-tauri/` |
| Global Tauri JS | **`withGlobalTauri: false`** | Prefer `window.eval` bridges; optional `__TAURI__` if present |
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
4. **Backend URLs** may be localhost/LAN for SearXNG **unless** `strict_backend_urls` / `NETRAIL_STRICT_BACKEND_URLS`.  
5. **Env backend URLs** (`NETRAIL_SEARXNG_URL` / `SEARXNG_URL`) validated on load; invalid ignored.  
6. **Partial fanout failure** → HTTP 200 + `errors[]`; **total failure** → `FANOUT_TOTAL_FAILURE` 502.  
7. **Empty web fanout** → Wikipedia fallback (both stacks).  
8. **Fanout deadline** 20s both stacks.  
9. **History:** encrypt when key available; degrade + banner if keyring missing; FTS tokens plaintext (documented).  
10. **Rate limits:** search 90/min · open 120/min · mutate 60/min; `NETRAIL_RATE_LIMIT=0` disables.  
11. **Typed errors:** `{ code, detail, status }` — see `docs/API_ERRORS.md`.  
12. **Dual-stack security changes land in both languages same PR** when possible.  
13. **Packaged UI must ship** under `/usr/share/netrail/static/` (deb/AppImage). Missing static = broken desktop.  

**Must not break:** search/open happy path, static packaging, Fernet Rust↔Python history interop, error shape, version SSOT.

---

## 4. Security model (agent-facing)

### 4.1 What we protect

| Asset | Protection |
|-------|------------|
| Query text / snippets at rest | Fernet when keyring or `NETRAIL_DB_KEY` available |
| Browser open SSRF to loopback/LAN | Open-URL policy + DDG unwrap |
| Accidental remote exposure | Localhost bind; Docker compose publishes `127.0.0.1:7421` |

### 4.2 What we do **not** claim

| Residual | Why |
|----------|-----|
| Same-machine malware | Unauthenticated API by default = full product control |
| Query privacy from backends | Queries **egress** to DDGS / SearXNG / Brave / Wikipedia |
| FTS full-disk secrecy | FTS5 tokens + URLs plaintext by design |
| Focus of marketing “everything stays on 127.0.0.1” | **False** for query egress (fixed in Wave 0 docs) |

### 4.3 Optional enterprise controls (1.4.0)

| Control | Env / setting | Notes |
|---------|---------------|-------|
| API token | `NETRAIL_API_TOKEN` | Bearer or `X-NetRail-Token`; health exempt |
| UI token inject | `NETRAIL_INJECT_UI_TOKEN` | Desktop/static inject for token mode |
| Strict backends | `strict_backend_urls` / `NETRAIL_STRICT_BACKEND_URLS=1` | Reject private/loopback backend URLs |
| Audit log | `NETRAIL_AUDIT_LOG=1` / `NETRAIL_AUDIT_LOG_PATH` | JSON lines for sensitive actions |
| Rate limit off | `NETRAIL_RATE_LIMIT=0` | Smoke/CI only; not default for users |

---

## 5. Session delta (uncommitted — **OpenCode must handle first**)

Work from the 2026-08-01 Grok session. **Not committed.** Diff summary:

### 5.1 Files

| File | Change |
|------|--------|
| `src-tauri/src/desktop.rs` | After OS focus, emit `focus-search` + `eval` → `window.netrailFocusSearch()` |
| `netrail/static/app.js` | `focusSearchInput()` / `window.netrailFocusSearch`; listen `focus-search` if `__TAURI__` |
| `netrail/static/style.css` | Result-card grid fix + frozen action column |

### 5.2 Spotlight / tray focus pipeline

**Already shipped (committed 1.x desktop):**

- `TrayIconBuilder` + Show/Quit menu  
- Left-click tray → `focus_main_window`  
- `CloseRequested` → `prevent_close` + `hide()` (Wayland-friendly warm window)  
- Global shortcut `Ctrl+Shift+S`  
- Single-instance → focus main  
- `always_on_top` pulse to beat Wayland focus stealing  

**Added uncommitted:**

```
tray | hotkey | second-instance | menu Show
            │
            ▼
    focus_main_window()
      unminimize + show + set_focus
      always_on_top true/false
      emit("focus-search")
      eval(netrailFocusSearch @ 50ms)
            │
            ▼
    #query focus() + select()
    (skip if doc/save dialog open)
```

**Design notes:**

- `withGlobalTauri: false` → **`eval` bridge is authoritative**; emit is best-effort.  
- Same bridge pattern as `window.netrailOpenDoc` / `window.netrailDonate`.  
- Linux tray left-click is historically flaky; **right-click → Show** and **Ctrl+Shift+S** remain solid.  
- Optional UX follow-up (not done): `show_menu_on_left_click(false)` for Alfred-style left-click = show only.

### 5.3 Result-card CSS bug (root cause)

**Symptom:** ★ / Open buttons stretch wide on **short one-line snippets**; look fine on long 3-line snippets.

**Root cause (grid, not “flex free space” alone):**

```css
/* OLD — wrong for 2-child web cards */
.result-card { grid-template-columns: auto 1fr auto; }
```

Without thumbnail, only two children exist: body → col1 `auto`, **actions → col2 `1fr`** (absorb free width). Short body ⇒ huge free track ⇒ fat buttons.

**Fix (uncommitted):**

```css
.result-card { grid-template-columns: minmax(0, 1fr) auto; }
.result-card.image-card { grid-template-columns: 96px minmax(0, 1fr) auto; }
.result-actions {
  width: max-content;
  min-width: 4.75rem;
  justify-self: end;
}
```

### 5.4 Suggested commit message (when human approves)

```
fix(desktop,ui): Spotlight focus-search on tray and freeze result actions

- After focus_main_window, emit focus-search and eval netrailFocusSearch
- Focus/select #query (skip when modals open)
- Fix result-card grid so actions never sit on the 1fr track without a thumb
```

Bump to **1.4.1** only if shipping a patch release; otherwise keep under Unreleased until tag policy is decided.

---

## 6. Findings backlog (enterprise audit residual)

Source of truth: [AUDIT_ENTERPRISE_2026-07-31.md](AUDIT_ENTERPRISE_2026-07-31.md). Waves 0–5 largely landed in **1.2.3 → 1.4.0**. Do **not** re-open closed SEC items without new evidence.

| Status | Items |
|--------|--------|
| **Closed in 1.2.3** | SEC-2026-01 DDG `duck.com`; SEC-2026-02 env backend validate; SEC-2026-03 rebinding apex; DOC-01 query privacy claim |
| **Closed in 1.3.0** | Mapped IPv6, AWS IMDS IPv6, metadata hostnames, redirect policy, history no-key parity, fanout 20s Python, error/detail/mode/collection parity |
| **Closed in 1.4.0** | Optional token, strict backends, mutate rate limits, audit log, Rust Docker, dep audits, SBOM, parity harness |
| **Still residual** | DNS pin on open (SEC-04 class); process SSRF via settings without token; soft rate limits; image CDN privacy; no Tauri E2E; dual-stack long-term cost |

**Do not expand scope into:** multi-user OAuth, owned crawl corpus, local LLM, non-Linux ports — unless human explicitly asks.

---

## 7. How to run / test / package (bootstrap)

```bash
# Version gate
bash scripts/check-versions.sh

# Python tests
python3 -m venv .venv && source .venv/bin/activate
pip install -r requirements.txt
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

# Smokes
NETRAIL_RATE_LIMIT=0 bash scripts/e2e-api-smoke.sh
NETRAIL_RATE_LIMIT=0 bash scripts/package-smoke.sh   # needs built binary
bash scripts/parity-api-smoke.sh

# Desktop dev
npm ci && npm run dev

# Desktop release (AppImage needs patchelf)
APPIMAGE_EXTRACT_AND_RUN=1 npm run build
```

### Useful env

| Variable | Purpose |
|----------|---------|
| `NETRAIL_STATIC_DIR` | UI assets |
| `NETRAIL_DB_PATH` | SQLite path |
| `NETRAIL_DB_KEY` | Fernet key (headless/Docker) |
| `BRAVE_SEARCH_API_KEY` | Brave backend |
| `NETRAIL_SEARXNG_URL` / settings | SearXNG |
| `NETRAIL_RATE_LIMIT=0` | Disable limits (CI/smoke) |
| `NETRAIL_API_TOKEN` | Optional auth |
| `NETRAIL_INJECT_UI_TOKEN` | UI header inject |
| `NETRAIL_STRICT_BACKEND_URLS` | Strict backend mode |
| `NETRAIL_AUDIT_LOG` / `_PATH` | Audit JSONL |
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
| `window.emit` + optional `__TAURI__.event.listen` | Encryption degrade; focus-search secondary |
| HTTP `fetch` | All product API |

---

## 9. Recommended next actions (priority ordered)

### P0 — Working tree hygiene

1. **Review & commit** uncommitted Spotlight + CSS fix (or discard if human rejects).  
2. Decide: stay **1.4.0 Unreleased delta** vs patch **1.4.1**.  
3. **Push 5 local commits** only when human requests (`main` ahead of origin).  
4. Confirm public GitHub release/tag alignment for 1.4.0.

### P1 — Verify this session’s UX

1. Manual desktop: tray Show / `Ctrl+Shift+S` → caret in `#query` + select.  
2. Search with short vs long snippets → action buttons same compact width.  
3. Image mode cards still layout thumb | body | actions.  
4. Modal open (docs/save) → focus-search must not steal caret.  
5. Wayland: close window → hides to tray; show restores without full restart.

### P2 — Optional UX productization

| Item | Notes |
|------|-------|
| Alfred left-click | `show_menu_on_left_click(false)`; menu on right-click only |
| Global shortcut discoverability | Footer / MANUAL / tray tooltip already partial |
| CSS regression | Snapshot or simple layout unit not present — visual QA only |

### P3 — Residual engineering (not blocking)

| Item | Notes |
|------|-------|
| Open-URL DNS pin | Documented residual; expensive / UX-sensitive |
| Token-on-by-default for Docker | Document strongly; default-off preserves desktop UX |
| Tauri webview E2E | Accepted residual R8; API smoke remains gate |
| Dual-stack golden expansion | `tests/fixtures/url_policy.json` + parity smoke exist; grow carefully |

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
| Force-push / amend published history | Human policy |
| Bump version in one file only | SSOT script fails CI |
| Assume `__TAURI__` always present | `withGlobalTauri: false` |

---

## 11. Scorecard (post-1.4.0 + session WIP)

| Criterion | Score | One-liner |
|-----------|------:|-----------|
| Core job completeness | 9 | Search → rail → open solid |
| Safety / data-loss | 8.5 | Purge confirm; encrypt/degrade honest |
| Correctness of mutations | 8.5 | Validated settings; typed collection errors |
| Performance | 7 | No formal load tests; fanout deadline present |
| Usability | 8.5 | Spotlight WIP; keyboard rail; recovery UX |
| Recoverability | 8.5 | Partial fanout, wiki fallback, hide-to-tray |
| Architecture | 8.5 | Clear modules; dual-stack residual cost |
| Code quality | 8.5 | Small surface; typed errors |
| Tests | 8.5 | Strong security/API; no webview E2E |
| Security (model-fit) | 9 | Open-URL + optional token/audit |
| Docs / claims | 8.5 | Truth waves landed |
| Packaging | 8 | CI primary for AppImage |
| **Overall** | **~8.6** | Soft GA + active UX polish |

---

## 12. Copy-paste resume prompt (OpenCode)

```
You are continuing NetRail (Linux local research console).

READ FIRST:
  docs/HANDOFF_OPENCODE_2026-08-01.md
  HANDOVER.md
  docs/AUDIT_ENTERPRISE_2026-07-31.md (residual only)

Version: 1.4.0 SSOT. Branch main is 5 commits ahead of origin/main (do not push unless asked).

WORKING TREE (uncommitted — handle first):
  src-tauri/src/desktop.rs  — Spotlight: emit focus-search + eval netrailFocusSearch after focus
  netrail/static/app.js     — focusSearchInput on #query (skip modals)
  netrail/static/style.css  — result-card grid 1fr|auto so ★/Open do not stretch on short snippets

Stack:
  Rust Axum primary + Tauri 2 desktop; Python FastAPI for Docker/Flatpak/tests.
  UI = netrail/static vanilla JS (NO React, NO PySide6).
  withGlobalTauri: false → prefer window.eval bridges.

Invariants:
  localhost-only :7421; no telemetry; open-URL blocks (encoded loopback, private, rebinding, DDG uddg incl duck.com);
  no Brave key on disk; version SSOT via scripts/check-versions.sh; dual-stack security in both languages;
  typed errors {code,detail,status}; packaged static under /usr/share/netrail/static/.

Bootstrap:
  bash scripts/check-versions.sh
  source .venv/bin/activate && pytest tests/ -q
  cd src-tauri && cargo clippy --all-targets -- -D warnings && cargo test
  cargo build --release --bin netrail-api --no-default-features
  NETRAIL_RATE_LIMIT=0 bash scripts/e2e-api-smoke.sh

Must not break: search/open, static packaging, Fernet interop, error shape.

Out of scope unless asked: owned corpus, local AI, multi-user auth productization, non-Linux.

Do not force-push. Do not push unless I ask. Prefer commit of current WIP after visual verify.
```

---

## 13. Doc index (quick)

| Doc | Role |
|-----|------|
| [README.md](../README.md) | Install + pitch |
| [HANDOVER.md](../HANDOVER.md) | Zero-context freeze resume |
| [SECURITY.md](../SECURITY.md) | Threat model |
| [CHANGELOG.md](../CHANGELOG.md) | Semver history |
| [docs/ARCHITECTURE.md](ARCHITECTURE.md) | Lifecycle / design |
| [docs/DISTRIBUTION.md](DISTRIBUTION.md) | Packaging + env table |
| [docs/MANUAL.md](MANUAL.md) | End-user manual |
| [docs/API_ERRORS.md](API_ERRORS.md) | Error codes |
| [docs/RELEASE_v1.4.0.md](RELEASE_v1.4.0.md) | 1.4.0 notes |
| [docs/AUDIT_ENTERPRISE_2026-07-31.md](AUDIT_ENTERPRISE_2026-07-31.md) | Enterprise adversarial audit |
| **This file** | OpenCode handoff + session delta + analysis |

---

## 14. Honest closing

NetRail 1.4.0 is a **mature single-user product** for its model: link-first search, solid open-URL hygiene, optional operator controls, and dual entrypoints (desktop + headless). The enterprise audit waves closed the highest-ROI security and truth gaps. Remaining work is **hygiene (push/commit), UX polish (this session’s WIP), and deliberate residual risk documentation** — not a rewrite.

OpenCode should treat the three dirty files as **the immediate continuity surface**, preserve dual-stack discipline on any security touch, and refuse scope that turns NetRail into multi-tenant SaaS.

---

*Handoff for OpenCode / human continuity — NetRail 1.4.0 — 2026-08-01 — be honest, no scope creep, prefer durable repo state.*
