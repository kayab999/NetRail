# NetRail — Handover (zero-context resume)

| Field | Value |
|-------|--------|
| **Product** | Local privacy-first research console for Linux |
| **Version** | **1.6.4** (`scripts/check-versions.sh`) |
| **Primary path** | Rust Axum API + Tauri 2 desktop; Python for Docker/Flatpak/tests |
| **License** | AGPL-3.0 |
| **Repo** | https://github.com/kayab999/NetRail |
| **Freeze date** | 2026-07-12 (invariants; state refreshed 2026-08-02) |
| **HEAD note** | **1.6.4 on main** (`a12dbed` packaging + `e436e6d` NR remediations; tag `v1.6.4` pushed). Audit NR-01..NR-14 closed (collection item rate-limit, concurrent-safe atomic settings, audit event, constant-time token, readonly docs, etc.). Official distribution: **AppImage-first** — [packaging/README.md](packaging/README.md), `scripts/build-desktop-linux.sh`. **GitHub Release assets for v1.6.4 not yet published** — Release CI run failed at AppImage/`linuxdeploy` (see handoff §9 NR-16). Prior: 1.6.3 SBOM-in-bundle + fixture growth + CSS guard; 1.6.2 chaos/load/bench; 1.6.1 DNS pin A15 + E2E. Full agent handoff: [docs/HANDOFF_OPENCODE_2026-08-02.md](docs/HANDOFF_OPENCODE_2026-08-02.md). Audits: [AUDIT_ARCH](docs/AUDIT_ARCH_2026-08-01.md) + [AUDIT_OPENCODE](docs/AUDIT_OPENCODE_ADVERSARIAL_2026-08-01.md). |

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
| R4 | Public GitHub Latest may lag | ✅ **v1.6.6 is Latest** (release-readiness RC, 2026-08-10; published Latest still v1.6.4 until RC lands) |
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

## 8b. Findings freeze snapshot (P0–P3) — differential probes 2026-08-02, re-verified 2026-08-03 on 1.6.4, P0/P2 fixed 2026-08-03

Original probes ran against `netrail-api` 1.6.2 + Python. Re-verification 2026-08-03 (full adversarial sweep + live probes on **1.6.4**, both stacks, fake-browser sandbox): 10,000 fuzzed open-URLs differential, targeted SSRF encodings, corrupt-settings boot, partial-read, double-instance, audit, browser-discovery parity, full source sweep of both stacks + frontend + build configs.

| Sev | Item | Evidence | Status |
|-----|------|----------|--------|
| P0 | **Rust SSRF via IPv4-embedded IPv6 literals** | `http://[64:ff9b::7f00:1]/` (→127.0.0.1) and `[64:ff9b::c0a8:0101]` (→192.168.1.1), `[::ffff:0:10.0.0.1]`, `[::ffff:0:7f00:1]` all returned **200** from Rust on 1.6.4; Python blocked `OPEN_URL_PRIVATE`. Cause: `effective_ip` (src-tauri/src/security.rs:156) only unmapped `::ffff:x.x.x.x`; `is_non_public_v6` (:176) never decoded IPv4 embedded in `64:ff9b::/96` (RFC 6052) or `::ffff:0:a.b.c.d`. | **FIXED 2026-08-03:** Rust `decode_embedded_v4` (security.rs:156) now decodes `64:ff9b::/96`, `::ffff:0:0:0/96` (`::ffff:0:a.b.c.d`) and deprecated `::/96` (`::a.b.c.d`) and applies the v4 policy; `is_non_public_v6` also blocks the RFC 6052 local-use prefix `64:ff9b:1::/48`. Python `_effective_ip` (security.py:140) mirrors the decode and the public-embedded over-block is relaxed (`64:ff9b::1.1.1.1` now allows). **Regression:** 9 new `open_url` + 2 new `backend_url` fixture vectors green on both stacks; live probe 12/12 vectors → identical codes (`OPEN_URL_LOCALHOST`/`OPEN_URL_PRIVATE`/allow) on both stacks. |
| P1 | **Non-atomic settings write** | **FIXED (NR-08):** both stacks now tmp+rename (Rust config.rs:174–213 unique temp + rename; Python config.py:166–192 mkstemp + `os.replace`). | Fixed |
| P1 | **Silent settings reset** | Boot with truncated/corrupt `settings.json` served defaults with **zero log output** in both stacks (Rust config.rs:114–124 `.ok()`/`unwrap_or_default()`; Python config.py:114–129 `except (json.JSONDecodeError, OSError): pass`). Config loss was invisible. | **FIXED 2026-08-03:** both stacks now emit a WARN at load time — Rust `load_settings` (config.rs) logs `settings.json is corrupt/unreadable; falling back to defaults` with `path` + `err`; Python logs via `netrail.config` logger with the parse error. **Regression:** `corrupt_settings_falls_back_with_warning` (Rust, captures events via `tracing-subscriber`; new dev-dep) + `test_load_settings_warns_on_corrupt_file` (Python, caplog) green; live-verified both stacks (WARN on stderr at boot). |
| P2 | Python `UnicodeEncodeError` escapes pin | `http://%B4.3511866278/x` → `/api/open` **500 "Internal Server Error"** (plain text, non-contract body) on 1.6.4; Rust 400 `OPEN_URL_INVALID`. `resolve_host_ips` caught only `OSError` (netrail/security.py:234–235); `getaddrinfo` raises `UnicodeEncodeError` (subclass of `ValueError`). | **FIXED 2026-08-03:** `_normalize_host` IDNA-encodes valid internationalized hosts (Rust `url` parity — `xn--bcher-kva.de` allows), invalid IDN is rejected `OPEN_URL_INVALID`/`BACKEND_URL_INVALID` (netrail/security.py), and `resolve_host_ips` catches `(OSError, UnicodeError)`. **Regression:** `reject_invalid_idn_host` + `reject_invalid_idn_backend` fixture vectors green both stacks; live `%B4` → 400 `OPEN_URL_INVALID`. |
| P2 | Error-code divergence on malformed URLs | Both block, different codes: `http:\\192.168.1.1\x` (Rust `PRIVATE` vs Python `NO_HOST`), `\\`-network-path public hosts (Rust 200 browser-compatible / Python `NO_HOST`), leading-zero/u32/hex octets (both block — safe), port >65535 (Python allow, Rust `INVALID`), `1.1.1.1\@2.2.2.2` (Rust 200 / Python `CREDENTIALS`), `[2001:db8::1]` (Python block / Rust allow), `[fec0::1]` (both allow). All core SSRF encodings (hex/octal/u32/short-form/v4-mapped/%-encoded/trailing-dot/nip.io) blocked by **both**. **Post-fix re-fuzz (10,000 URLs, live, 2026-08-03): 2,747 mismatches = 189 semantic + 2,558 code-only.** Semantic class breakdown of all 189: 58 Python `NO_HOST` vs Rust allow (`\\`-network-path, fail-closed), 57 Python `OPEN_URL_PRIVATE` vs Rust allow (IPv6 `::/8`-reserved via `is_reserved`, fail-closed), 74 Python allow vs Rust block (non-canonical IPv4 literals — integer/hex/leading-zero/malformed-port — **all verified NOT loopback/private**; parity gap only). **Zero `UnicodeEncodeError` 500s and zero NAT64-class divergences remain.** Golden-fixture live parity (sandboxed, post-fix): **57/57 open_url + 18/18 non-strict backend_url + ETag/If-Match contract all pass** on the rebuilt Rust binary. | **FIXED 2026-08-09:** Python mirrors the Rust `url` crate WHATWG rules, probed empirically live against the rebuilt binary (30+ vector matrices). (1) `_parse_whatwg_ipv4` (netrail/security.py) — single label parses as IPv4 iff all-digits or `0x`-hex (`0x` alone = 0 → 0.0.0.0; `0xzz` → DNS domain); dotted hosts with an all-digit/`0x` last label take the strict parse (octal 8/9 digits, mid-label `x`, part > 255, > 4 parts, empty parts, last-part bound `256^(5-parts)`, fold `a<<24|b`) — failure = `OPEN_URL_INVALID`/`BACKEND_URL_INVALID`; (2) port: `urlparse.port` `ValueError` (multi-colon `:80:9604`, `:8080.`) and port > 65535 → `INVALID`, port 0 allowed (Rust allows); (3) `_is_non_public_v4` adds 0.0.0.0/8 (Rust blocks `31` → 0.0.0.31 private). **Re-fuzz (7,600 URLs, live, same corpus classes + new): 50 residual py-allow/rust-block (0.66%), all the single `0xzz` DNS-family — Rust fails DNS inside open, Python in the later pin stage (same terminal state, no fail-open); `py_block_rust_allow` = 0, `code_diff` = 0 across all 7,600.** Regression: 19 new pytest vectors (tests/test_security.py → 54 pass). Known residual by design: DNS-dependent stage ordering (validate vs pin), `\\`-network-path, IPv6 reserved-class codes. |
| P2 | **Smoke harness opens real browser** | `scripts/parity-api-smoke.sh` POSTs allow vectors to `/api/open` → spawned a real browser tab per vector and failed headless (BROWSER_NOT_FOUND→500). No dry-run support existed in either stack. | **FIXED 2026-08-03:** new `NETRAIL_NO_OPEN` env (any value except `0`/`false`/empty) makes `open_url` report `{browser/executable/sandbox: "dry-run"}` **without browser discovery or spawn** — Rust `browsers.rs:204`, Python `browsers.py:180`. `parity-api-smoke.sh` now exports it, so the harness is headless-safe with **no fake-browser PATH required** (verified: 57/57 open_url + 18/18 backend_url + ETag pass without fakebin). **Regression:** `netrail_no_open_returns_dry_run_without_discovery` + `dry_run_env_parsing` (Rust) and `test_open_dry_run_returns_without_browser` (Python) green. |
| P2 | Blocked opens not audit-logged | Audit (opt-in `NETRAIL_AUDIT_LOG=1`) recorded only successful `open` (netrail/main.py, src-tauri/src/server/mod.rs); validation-failure and BROWSER_NOT_FOUND paths exited before `audit::log_event`. Attempted-block record missing. | **FIXED 2026-08-03:** both stacks now emit `open.blocked` (`{url_host, code, detail}`) for every rejected `/api/open` attempt (validation, DNS pin, browser spawn), alongside the success `open` event — Rust via closure wrap in `open_link` (server/mod.rs) + `audit_open_blocked`; Python via try/except `NetRailError` in `open_link` (main.py) + `_open_link_impl`. 429 rate-limit rejections intentionally **not** audited (pre-open, per-design). **Regression:** `open_blocked_audit_tests` (Rust ×2) + `test_audit_log_open_blocked` (Python) green; live verified both stacks (`OPEN_URL_LOCALHOST`, `OPEN_URL_INVALID` with correct `url_host`). Minor log-field divergence: unparseable-URL host is `null` in Rust, raw host string in Python (log detail only). |
| P3 | **Browser-discovery parity divergence (new)** | Live on 1.6.4, same machine: `brave-browser-stable` → Rust `{name: "New Incognito Window", supports_private: false}` vs Python `{name: "Brave Web Browser", supports_private: true}`; `torbrowser-launcher` → Rust `supports_private: false` vs Python `true`. Root causes: (1) Rust `known_browsers()` has 7 entries (browsers.rs:23–33) vs Python 13 (browsers.py:26–40); (2) Python **defaults unknown stems to `--incognito`** (browsers.py:136) while Rust defaults to no flag (browsers.rs:128) — Python can append a flag the browser doesn't support (e.g. torbrowser-launcher); (3) .desktop `Name=` parsing differs: Rust takes the first raw `Name=` line (browsers.rs:75), Python configparser keeps the last. Same `browser_id` + `private_mode` → different actual behavior per stack. | Open |
| P4 | `build_http_client` fallback (new, observation) | http_client.rs:17 `unwrap_or_else(|_| Client::new())` silently drops 15s timeout, `Policy::none` redirects, UA, keepalive if the builder ever fails (currently hard to trigger). | **Fixed 2026-08-03:** fallback now `tracing::warn!`s with the builder error, naming the dropped protections (http_client.rs:17). |
| P4 | Fanout deadline asymmetry (observation) | Rust hard-aborts at 20s (`tokio::time::timeout`, backends/mod.rs:214–241); Python's `as_completed(timeout=20)` then blocks in `ThreadPoolExecutor.__exit__` (shutdown wait) for stragglers — bounded by the 12–15s backend timeouts, so the 20s deadline is effectively dead code. Response may still exceed 20s when a backend exceeds its own timeout. | Open (trivial) |
| P4 | Inline failsafe script blocked by CSP (observation) | index.html:158–169 inline splash failsafe had no CSP hash (only the token script did) → dead under `script-src 'self'`. Harmless, but dead code. | **Fixed 2026-08-03:** the failsafe script's `sha256-aN9klVksJOk4OThOcI2OMlo7DsWPc+W7cPY4E+ODbD8=` is now whitelisted in `security::CSP`; regression test `csp_includes_failsafe_script_hash` (server/mod.rs) pins the hash to the actual index.html content so they cannot drift. Live header verified. |
| P4 | Docker runs as root (observation) | Both Dockerfiles had no `USER`; container binds 127.0.0.1 so exposure was limited. | **Fixed 2026-08-03:** both `Dockerfile` and `Dockerfile.rust` create `appuser` and run with `USER appuser`; `/app/data`, `$HOME/.config` (settings) and `$HOME/.local/share/netrail` (audit log) are owned by appuser. |
| — | Partial-read / double-instance / corruption | Content-Length mismatch holds slot until client timeout (no crash, no poison; no request timeout in either stack). Second instance exits clean (exit 1, EADDRINUSE). Corrupt DB not reached — WAL behavior unchanged from prior freeze. | No finding |

**Fixture gaps (regression blind spots) for the open findings:** `tests/fixtures/url_policy.json` (57 open_url + 22 backend_url vectors after the P0/P2 regression additions) now covers `64:ff9b::`/`64:ff9b:1::`/`::ffff:0:`/`::`-embedded vectors (P0, 8 vectors) and non-ASCII/`%B4` hosts (P2, 2 vectors) on both stacks. Remaining open findings without dedicated vectors: leading-zero/u32/hex/port>65535 literal parsing parity (P2 divergence), `\\`-network-path and IPv6 `::/8`-reserved code parity.

Browser incident note: initial fuzz runs (pre-fake-browser sandbox) opened real browser tabs — harness bug, now mitigated (fake browser scripts in PATH + empty XDG + verify `OpenResult.executable` is the fake). **Never probe `/api/open` with allow vectors without the sandbox.**

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

Read first:
  docs/HANDOFF_OPENCODE_2026-08-02.md  (§6B closed work, §9 P0 NR-16)
  packaging/README.md
  HANDOVER.md

Version 1.6.4 SSOT. HEAD a12dbed on main (pushed). Tag v1.6.4 pushed.
GitHub Release assets for v1.6.4 NOT published — CI failed at linuxdeploy/AppImage (NR-16).

Invariants: localhost-only API, no telemetry, open-URL + DNS pin, no Brave key on disk,
version SSOT, typed errors, NETRAIL_READONLY gates admin mutations (history still on search/open),
XDG data outside bundle, AppImage-first ship path (not PyInstaller).

NEXT: fix Release CI AppImage/linuxdeploy and publish v1.6.4 assets. Then post-release doc refresh.
Do not re-open closed audits without evidence. Don't-build backlog only if asked (Q16, R7, multi-user).

Bootstrap:
  bash scripts/check-versions.sh
  source .venv/bin/activate && pytest tests/ -q
  cd src-tauri && cargo clippy --all-targets -- -D warnings && cargo test
  bash scripts/build-desktop-linux.sh --skip-tests
  gh run view 30771210870 --log-failed

Must not break: search/open, static packaging, Fernet interop, typed errors, read-only gate.
Do not force-push. Do not amend published history.
```

---

## 12. Key docs index

| Doc | Role |
|-----|------|
| [docs/HANDOFF_OPENCODE_2026-08-02.md](docs/HANDOFF_OPENCODE_2026-08-02.md) | **OpenCode handoff** — 1.6.4 state + NR-16 release CI |
| [packaging/README.md](packaging/README.md) | **Packaging SSOT** (AppImage-first) |
| [README.md](README.md) | Install + pitch |
| [SECURITY.md](SECURITY.md) | Threat model |
| [docs/API_ERRORS.md](docs/API_ERRORS.md) | Error codes |
| [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) | Lifecycle roadmap |
| [docs/RELEASE_ASSURANCE.md](docs/RELEASE_ASSURANCE.md) | Non-technical trust map |
| [docs/DISTRIBUTION.md](docs/DISTRIBUTION.md) | Ops packaging + env |
| [docs/RELEASE_v1.6.4.md](docs/RELEASE_v1.6.4.md) | 1.6.4 notes |
| [docs/MANUAL.md](docs/MANUAL.md) | User manual |
| [docs/AUDIT_ENTERPRISE_2026-07-31.md](docs/AUDIT_ENTERPRISE_2026-07-31.md) | Post-GA enterprise audit + workplan |
| [docs/AUDIT_ARCH_2026-08-01.md](docs/AUDIT_ARCH_2026-08-01.md) | Architecture-level audit (code-as-built, both stacks, enterprise readiness) |
| [docs/AUDIT_ADVERSARIAL_QA_2026-07-12.md](docs/AUDIT_ADVERSARIAL_QA_2026-07-12.md) | Hostile Q&A (historical) |
| [docs/AUDIT_RC_2026-07-12.md](docs/AUDIT_RC_2026-07-12.md) | RC audit |
| [CHANGELOG.md](CHANGELOG.md) | Semver history |
| [docs/RELEASE_v1.2.2.md](docs/RELEASE_v1.2.2.md) | Release notes |

---

*Handover for human/AI continuity — NetRail 1.6.4 — be honest, no scope creep, prefer durable repo state.*
