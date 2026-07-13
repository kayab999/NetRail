# NetRail — Post-fix audit (Phase A + hardening + packaging)

| Field | Value |
|-------|--------|
| **Date** | 2026-07-12 |
| **Version** | **1.2.2** |
| **Prior audit** | [AUDIT_RC_2026-07-12.md](AUDIT_RC_2026-07-12.md) |
| **Scope** | Verify Phase A fixes; close P1 parity; build RC artifacts |

---

## 1. Post-fix gate matrix

| Gate (from RC audit) | Pre-fix | Post-fix | Evidence |
|----------------------|----------|------------|----------|
| Clippy `-D warnings` | ❌ `TrayState` | ✅ Pass | `cargo clippy --all-targets -- -D warnings` |
| Rust tests | ✅ 48 | ✅ 48 | `cargo test` |
| Python tests | ✅ 35 | ✅ **37** | + Wikipedia fallback + health recovery |
| Version SSOT | ❌ 1.2.0/1.2.1 mix | ✅ **1.2.2** all product paths | package.json, Cargo.toml, `__version__`, README, MANUAL |
| CHANGELOG 1.2.1 + 1.2.2 | ❌ | ✅ | `CHANGELOG.md` |
| SECURITY supported | ❌ 1.0 only | ✅ 1.2.x + security for 1.1/1.0 | `SECURITY.md` |
| Release clippy gate | ❌ | ✅ | `.github/workflows/release.yml` |
| Python Wikipedia | ❌ | ✅ | `netrail/backends/wikipedia.py` + registry |
| Python `search_recovery` | ❌ | ✅ | `netrail/main.py` health |
| Empty-batch errors (Python) | ❌ silent | ✅ in `errors[]` | registry parity with Rust |
| Packaged static UI | was fixed in 1.2.0 | ✅ in **1.2.2 deb** | `usr/share/netrail/static/{index,app.js,style.css}` |
| `netrail-api` smoke | — | ✅ | health `1.2.2`, UI 200, open localhost 400 |

**Verdict:** Phase A is **closed**. Hardening (P1 dual-stack) is **closed**. Packaging is **RC-usable** via `.deb` / `.rpm` / `netrail-api`; local AppImage bundling still fails (see §4).

---

## 2. What changed since pre-fix audit

### Process / CI

- `TrayState` dead_code allow — restores green clippy/CI
- Release workflow mirrors CI clippy gate

### Product parity

| Feature | Rust | Python (now) |
|---------|------|----------------|
| Wikipedia fallback on empty web fanout | ✅ | ✅ |
| Empty backend → `errors[]` | ✅ | ✅ |
| `/api/health` → `search_recovery` | ✅ | ✅ |

### Docs / versions

- Single product version **1.2.2**
- `docs/RELEASE_v1.2.2.md` updated for parity + packaging

---

## 3. Runtime smoke (local, 2026-07-12)

```text
GET /api/health
  status=ok
  version=1.2.2
  search_recovery.hints present

GET /
  200, 6813 bytes, contains #search-form

GET /static/app.js
  200, 29107 bytes

POST /api/open {"url":"http://127.0.0.1/"}
  400 OPEN_URL_LOCALHOST
```

Binary: `src-tauri/target/release/netrail-api` (headless, ~6.7–6.8M).

---

## 4. Packaging results

| Artifact | Path | Status |
|----------|------|--------|
| **netrail-api** | `dist/release/netrail-api` | ✅ built + smoked |
| **Desktop binary** | `dist/release/netrail-desktop` | ✅ built |
| **.deb** | `dist/release/NetRail_1.2.2_amd64.deb` | ✅ static assets present |
| **.rpm** | `dist/release/NetRail-1.2.2-1.x86_64.rpm` | ✅ bundled |
| **AppImage** | — | ❌ local `linuxdeploy` failure |
| **SHA256SUMS** | `dist/release/SHA256SUMS` | ✅ |

### AppImage note

Local `tauri build --bundles appimage` fails. Root causes observed:

1. **`patchelf` missing** — `ERROR: Could not find patchelf: no such file` (linuxdeploy abort)
2. Secondary (manual re-run): missing WebKit helper path under AppDir

This host cannot install packages without sudo. GitHub Release runners install `patchelf` and produce AppImages (v1.2.1 draft succeeded). **RC packaging policy:**

1. Prefer **CI-built AppImage** on tag push for the official artifact.
2. Ship **.deb** + **.rpm** + **netrail-api** from local builds without waiting on AppImage.
3. Treat local AppImage as non-blocking if CI AppImage is green on tag.
4. For local AppImage later: `sudo apt install patchelf` then re-run `npm run build`.

### Deb verification

```text
usr/share/netrail/static/index.html
usr/share/netrail/static/app.js
usr/share/netrail/static/style.css
Package: net-rail  Version: 1.2.2
Depends: libwebkit2gtk-4.1-0, libgtk-3-0, …
```

---

## 5. Residual risks (unchanged or lowered)

| ID | Risk | Status |
|----|------|--------|
| SEC-01 | No localhost API auth | Accepted for v1 |
| SEC-03 | Keyring degrade | Bannered; unchanged |
| OPS-DDG | Captcha / empty results | Mitigated: Wikipedia + recovery on **both** stacks |
| PKG-AppImage | Local linuxdeploy fail | Mitigate via CI bundle |
| DRAFT-rel | v1.2.0/1.2.1 drafts still open | **Open** — close on publish of 1.2.2 |

---

## 6. RC readiness score

| Area | Score | Notes |
|------|-------|-------|
| Code quality gates | 9/10 | Clippy+tests green; CI not re-run on remote until push |
| Product completeness | 9/10 | 1.2.1 features + Python parity |
| Packaging | 8/10 | deb/rpm/api solid; AppImage via CI |
| Docs / version hygiene | 9/10 | Aligned |
| **Overall** | **8.5–9 / 10** | Ready for **commit → push → tag v1.2.2** |

---

## 7. Recommended next steps (Phase C)

1. **Commit** all Phase A + hardening + docs (exclude `screenshot*.png`, `src-tauri/rust_out` if untracked noise).
2. **Push `main`** — confirm GitHub CI green.
3. **Tag `v1.2.2`** — let Release workflow build AppImage + deb + netrail-api.
4. Publish release (or pre-release RC), verify SHA256SUMS.
5. Supersede/delete draft **v1.2.0** / **v1.2.1** or leave as historical drafts closed without “latest”.

---

## 8. Test counts

```text
Rust:   48 passed
Python: 37 passed  (+2: wikipedia fallback, health search_recovery)
```

---

*Post-fix audit — NetRail 1.2.2 — 2026-07-12*
