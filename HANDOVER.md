# NetRail — Handover (zero-context resume)

| Field | Value |
|-------|--------|
| **Product** | Local privacy-first research console for Linux |
| **Version** | **1.6.1** (`scripts/check-versions.sh`; [Unreleased] = read-only mode + systemd unit + backup script) |
| **Primary path** | Rust Axum API + Tauri 2 desktop; Python for Docker/Flatpak/tests |
| **License** | AGPL-3.0 |
| **Repo** | https://github.com/kayab999/NetRail |
| **Freeze date** | 2026-07-12 (invariants; state refreshed 2026-08-02) |
| **HEAD note** | **1.6.1** released (DNS pin A15, webview E2E matrix #9, cosign-signed CI matrix #10, FTS sync fix A13, typed 422s A1, CSP-safe token A2, ETag settings A6, per-identity rate limits A9, audit rotation A5, WAL + graceful shutdown A4, schema versioning A11). HEAD `abaf661`, main == origin/main, tree clean. Audit: [docs/AUDIT_ARCH_2026-08-01.md](docs/AUDIT_ARCH_2026-08-01.md) + [docs/AUDIT_OPENCODE_ADVERSARIAL_2026-08-01.md](docs/AUDIT_OPENCODE_ADVERSARIAL_2026-08-01.md) |

---

## 1. What it is

NetRail fans out a search query to enabled backends (DDGS, optional SearXNG, optional Brave), merges/dedupes results on the machine, and shows a **link rail**. Nothing opens in a browser until the user clicks Open. History is local SQLite (+ optional Fernet encryption). Zero telemetry. API binds **only** `127.0.0.1:7421`.

**Core job:** search → review links → open chosen URL in a chosen browser (optionally private).

**Not:** Google replacement, multi-user SaaS, authenticated remote API, owned crawl corpus (roadmap).

---

## 2. Architecture map

```
Entry: netrail (Tauri) | netrail-api | python -m netrail | Docker/Flatpak
                │
                ▼
        HTTP 127.0.0.1:7421  (Axum Rust primary / FastAPI Python)
                │
    ┌───────────┼───────────┬──────────────┐
    ▼           ▼           ▼              ▼
 static UI   /api/search  /api/open    history SQLite
 app.js      fanout+merge  browsers    Fernet/keyring
             wikipedia fb  validate_url FTS5 (plaintext tokens)
```

| Area | Rust | Python |
|------|------|--------|
| Server | `src-tauri/src/server/mod.rs` | `netrail/main.py` |
| Fanout | `src-tauri/src/backends/` | `netrail/backends/` |
| Security | `src-tauri/src/security.rs` | `netrail/security.py` |
| History | `src-tauri/src/history/mod.rs` | `netrail/history/` |
| UI | `netrail/static/` (shared) | same |
| Desktop | `src-tauri/src/desktop.rs` | — |
| Rate limit | `src-tauri/src/rate_limit.rs` | `netrail/rate_limit.py` |

**Config:** `~/.config/netrail/settings.json`  
**DB:** `~/.local/share/netrail/netrail.db` (override `NETRAIL_DB_PATH`)  
**Static UI:** runtime `static_dir()` / `NETRAIL_STATIC_DIR` / packaged `/usr/share/netrail/static/`

---

## 3. Safe defaults & invariants

1. **Bind localhost only** — never listen on `0.0.0.0` without explicit redesign + auth.
2. **No telemetry / no accounts / no Brave key on disk** — key via env only.
3. **Open-URL validation** before browser spawn:
   - http(s) only; no credentials
   - block localhost + **encoded loopback** (decimal/hex/octal/short IPv4)
   - block private/non-public IPs (`OPEN_URL_PRIVATE`)
   - block DNS rebinding helper domains
   - unwrap DDG `uddg=` before checks
4. **Backend URLs** may be localhost/LAN (SearXNG); block metadata + rebinding.
5. **Partial fanout failure** → 200 + `errors[]`; total failure → `FANOUT_TOTAL_FAILURE` 502.
6. **Empty web fanout** → Wikipedia fallback (both stacks).
7. **History:** encrypt when key available; degrade + banner if keyring missing; FTS tokens plaintext (documented).
8. **Rate limits:** 90 searches / 120 opens per minute (`RATE_LIMITED` 429); `NETRAIL_RATE_LIMIT=0` disables.
9. **Version SSOT:** package.json ≡ Cargo.toml ≡ tauri.conf ≡ `netrail/__init__.py` ≡ `config.rs` VERSION — enforced by `scripts/check-versions.sh`.

**Primary risks:** privacy (queries leave machine to backends), local-process API abuse (no auth), history integrity/encryption degrade.  
**Must not break:** search/open happy path, static UI packaging, Fernet Rust↔Python history, typed error `{code,detail,status}`.

---

## 4. How to run

```bash
# Dev desktop
npm ci && npm run dev

# Release desktop binary
npm ci && npm run build
./src-tauri/target/release/netrail

# Headless API
cargo build --release --manifest-path src-tauri/Cargo.toml \
  --bin netrail-api --no-default-features
./src-tauri/target/release/netrail-api --api-only
curl -s http://127.0.0.1:7421/api/health

# Python fallback
python3 -m venv .venv && source .venv/bin/activate
pip install -r requirements.txt
python -m netrail
```

Useful env:

| Var | Purpose |
|-----|---------|
| `NETRAIL_STATIC_DIR` | UI asset directory |
| `NETRAIL_DB_PATH` | SQLite path |
| `NETRAIL_DB_KEY` | Fernet key (Docker/headless) |
| `BRAVE_SEARCH_API_KEY` | Brave backend |
| `NETRAIL_SEARXNG_URL` / settings | SearXNG |
| `NETRAIL_RATE_LIMIT=0` | Disable rate limits |
| `APPIMAGE_EXTRACT_AND_RUN=1` | AppImage without FUSE |

---

## 5. How to test

```bash
bash scripts/check-versions.sh
source .venv/bin/activate && pytest tests/ -q
cd src-tauri && cargo clippy --all-targets -- -D warnings && cargo test
bash scripts/package-smoke.sh   # needs built netrail-api
```

**Expected (freeze):** Python ~47 · Rust ~49 unit + ~10 integration · clippy clean · CI green on `main`.

---

## 6. How to package

```bash
# Headless
cargo build --release --manifest-path src-tauri/Cargo.toml \
  --bin netrail-api --no-default-features

# Desktop bundles (.deb, .rpm, AppImage)
# AppImage needs: sudo apt install patchelf
APPIMAGE_EXTRACT_AND_RUN=1 npm run build
# → src-tauri/target/release/bundle/
```

| Artifact | Notes |
|----------|--------|
| `.deb` | Ships `usr/share/netrail/static/` — critical for UI |
| `.rpm` | Same idea |
| AppImage | CI installs `patchelf`; local fails without it |
| `netrail-api` | Headless; smoke with `scripts/package-smoke.sh` |

Release CI: `.github/workflows/release.yml` on tag `v*` (clippy + tests + AppImage/deb + SHA256SUMS, draft release).

---

## 7. Residual risks (honest)

| ID | Residual | Why not fixed / next step |
|----|----------|---------------------------|
| R1 | No API auth on localhost | Design v1; optional token later |
| R2 | DDGS HTML scrape / captcha | External; Wikipedia + recovery UX |
| R3 | Dual Rust/Python surface | **Policy:** Rust production; Python compatibility (see DISTRIBUTION) |
| R4 | Public GitHub Latest may lag | ✅ **v1.2.2 is Latest** (2026-07-13) |
| R5 | Draft releases v1.2.0/1.2.1 | ✅ Removed after 1.2.2 publish |
| R6 | Local AppImage needs patchelf | Documented; CI **requires** AppImage on release |
| R7 | Image CDN privacy (Images mode) | `no-referrer` set; still loads remote URLs |
| R8 | No Tauri webview E2E | **API E2E smoke** in CI (`scripts/e2e-api-smoke.sh`); no GTK driver |
| R9 | Collection add uses open-URL policy | Private LAN URLs cannot be saved via API — intentional safety |

---

## 8. Findings freeze snapshot (P0–P3)

| Sev | Item | Status |
|-----|------|--------|
| P0 | CI red (clippy TrayState) | ✅ Fixed |
| P0 | Silent empty search | ✅ Fixed + Wikipedia |
| P0 | Packaged UI 404 | ✅ Fixed (static bundle + runtime path) |
| P1 | Encoded loopback open | ✅ Fixed |
| P1 | Private IP open | ✅ Blocked on open |
| P1 | Python error shape / Wikipedia | ✅ Fixed |
| P2 | Rate limit | ✅ Added |
| P2 | A11y basics | ✅ Improved |
| P3 | Rate limit off for smoke | `NETRAIL_RATE_LIMIT=0` |
| P3 | Node 20 GH Actions deprecation | Residual CI warning |

Non-happy-path covered: empty query, bad mode/settings, open localhost/private/encoded, static path traversal (404), second bind EADDRINUSE, fanout empty → wiki/errors, encryption degrade banner, purge confirm in UI.

---

## 9. Scorecard (freeze)

| Criterion | Score | One-liner |
|-----------|------:|-----------|
| Core job completeness | 9 | Search → rail → open works end-to-end |
| Safety / data-loss | 8 | Purge confirms; encrypt/degrade honest; no cloud |
| Correctness of mutations | 8 | Settings validate; history parameterized |
| Performance | 7 | Unverified load tests; cold API &lt;100ms claim historical |
| Usability | 8.5 | Keyboard nav, recovery UX, a11y basics |
| Recoverability | 8 | Partial fanout, wiki fallback, restart-safe |
| Architecture | 8.5 | Clear modules; dual-stack residual cost |
| Code quality | 8.5 | Typed errors; small surface |
| Tests | 8.5 | Strong API/security; no E2E |
| Security | 9 | Model-fit strong after open-URL + rate limit |
| Docs / claims | 9 | Threat model + audits + this handover |
| Packaging | 8 | Deb/rpm/api solid; AppImage CI-primary |
| **Overall** | **~8.8** | **Usable RC / soft GA** pending public tag |

**Release readiness:** **usable RC / soft release** for Linux desktop + headless. **Polished GA** after tag publish + draft cleanup + optional soak.

---

## 10. Freeze checklist

- [x] Tests green (local)
- [x] Clippy `-D warnings`
- [x] Version SSOT script
- [x] Security open-URL hardening
- [x] Rate limits + a11y polish
- [x] CI green on push (as of last score lift)
- [x] HANDOVER.md written
- [x] Tag `v1.2.2`
- [x] Publish GitHub Release + SHA256SUMS (non-draft, Latest)
- [x] Close draft 1.2.0 / 1.2.1
- [x] `bash scripts/e2e-api-smoke.sh` green in CI + release

---

## 11. Copy-paste resume prompt

```
You are continuing NetRail (Linux local research console, Rust-primary + Python fallback).

Read HANDOVER.md first. Repo: NetRail. Version 1.6.1 (HEAD `abaf661`, tree clean, all pushed). See docs/HANDOFF_OPENCODE_2026-08-02.md for the remaining work plan.

Invariants: localhost-only API, no telemetry, open-URL blocks (incl. encoded loopback + private IPs + trailing-dot normalization + DNS pin at open), no Brave key on disk, version SSOT via scripts/check-versions.sh, typed errors {code,detail,status}, NETRAIL_READONLY=1 gates all mutations.

Bootstrap:
  bash scripts/check-versions.sh
  source .venv/bin/activate && pytest tests/ -q
  cd src-tauri && cargo clippy --all-targets -- -D warnings && cargo test
  cargo build --release --bin netrail-api --no-default-features
  NETRAIL_RATE_LIMIT=0 bash scripts/e2e-api-smoke.sh
  NETRAIL_RATE_LIMIT=0 bash scripts/parity-api-smoke.sh

Primary risks: privacy of queries to backends; local process abuse of :7421; history encryption degrade.

Must not break: search/open, static UI packaging, Fernet interop, typed errors.

Out of scope unless asked: owned corpus, local AI, multi-user auth, non-Linux.

Last freeze residual: publish tag v1.2.2; AppImage needs patchelf locally; dual-stack maintenance.

Do not force-push. Do not push unless I ask.
```

---

## 12. Key docs index

| Doc | Role |
|-----|------|
| [docs/HANDOFF_OPENCODE_2026-08-02.md](docs/HANDOFF_OPENCODE_2026-08-02.md) | **OpenCode handoff** — enterprise analysis + remaining work plan (release cut 1.6.2, backlog, residuals) |
| [README.md](README.md) | Install + pitch |
| [SECURITY.md](SECURITY.md) | Threat model |
| [docs/API_ERRORS.md](docs/API_ERRORS.md) | Error codes |
| [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) | Lifecycle roadmap |
| [docs/DISTRIBUTION.md](docs/DISTRIBUTION.md) | Packaging |
| [docs/MANUAL.md](docs/MANUAL.md) | User manual |
| [docs/AUDIT_ENTERPRISE_2026-07-31.md](docs/AUDIT_ENTERPRISE_2026-07-31.md) | Post-GA enterprise audit + workplan |
| [docs/AUDIT_ARCH_2026-08-01.md](docs/AUDIT_ARCH_2026-08-01.md) | Architecture-level audit (code-as-built, both stacks, enterprise readiness) |
| [docs/AUDIT_ADVERSARIAL_QA_2026-07-12.md](docs/AUDIT_ADVERSARIAL_QA_2026-07-12.md) | Hostile Q&A (historical) |
| [docs/AUDIT_RC_2026-07-12.md](docs/AUDIT_RC_2026-07-12.md) | RC audit |
| [CHANGELOG.md](CHANGELOG.md) | Semver history |
| [docs/RELEASE_v1.2.2.md](docs/RELEASE_v1.2.2.md) | Release notes |

---

*Handover for human/AI continuity — NetRail 1.4.0 — be honest, no scope creep, prefer durable repo state.*
