# NetRail — Adversarial UI/UX audit (2026-06-27)

**Reporter symptom:** Desktop app shows only `index.html not found`. Native Help/About/Donate visible but inert. No search box.

**Severity:** P0 ship-blocker for packaged installs.

---

## Executive summary

The broken screen is **not** a CSS or JavaScript bug. The Tauri shell loads `http://127.0.0.1:7421`, but the embedded Axum server could not find `netrail/static/index.html` on disk. Users received a one-line plaintext 404 instead of the full Link Rail UI.

| Finding | Severity | Status |
|---------|----------|--------|
| Static UI not bundled in `.deb` / AppImage | **P0** | Fixed in tree |
| `static_dir()` used compile-time dev path | **P0** | Fixed — runtime resolution |
| Plaintext 404 indistinguishable from “app” | **P1** | Fixed — HTML diagnostic page |
| Tauri menu calls JS that never loaded | **P1** | Documented; fixed when UI loads |
| Duplicate Help/About (native menu + web header) | **P2** | Open |
| Footer still said “v1.0” | **P3** | Fixed |
| No install-time verification of UI assets | **P1** | Fixed — deb/appimage `files` + `install.sh` |

---

## Root cause chain

```
Tauri window → http://127.0.0.1:7421/
       ↓
Axum GET / → read static_dir()/index.html
       ↓
static_dir() = CARGO_MANIFEST_DIR/../netrail/static  (compile-time dev path)
       ↓
CI build path: /home/runner/work/NetRail/...  → does NOT exist on user PC
       ↓
404 body: "index.html not found"  (no CSS, no app.js)
```

**What the user still saw**

- **Tauri native menu bar** (Help ▸ User Manual, Donate…) — rendered by Rust, not the web UI.
- Menu actions call `window.netrailOpenDoc()` / `window.netrailDonate()` via `webview.eval()`.
- Because `app.js` never loaded, those globals are undefined → **silent no-op**.
- User may have misread native chrome as “buttons on the main screen.”

**What was missing**

- Entire `index.html` structure: search input, Search button, tabs, results rail, history panel.

---

## Adversarial test matrix

| Scenario | Expected | Actual (pre-fix) |
|----------|----------|------------------|
| Fresh `.deb` install | Full Link Rail | Plaintext 404 |
| `curl http://127.0.0.1:7421/` | HTML + form | `index.html not found` |
| `curl /static/app.js` | 200 | 404 / wrong path |
| Help (native menu) | Doc dialog | No-op |
| Help (web dropdown) | Doc dialog | Not rendered |
| Donate | Opens URL | No-op (native) |
| API `/api/health` | 200 JSON | ✅ worked (API independent of UI path) |

**Key insight:** API can be healthy while UI is completely absent — health check is a poor “app OK” signal for desktop QA.

---

## Fixes applied

### 1. Runtime `static_dir()` (`config.rs`)

Resolution order:

1. `$NETRAIL_STATIC_DIR` (override)
2. `$EXE/../share/netrail/static` (deb layout)
3. Dev checkout fallback

### 2. Package bundling (`tauri.conf.json`)

```json
"linux": {
  "deb": { "files": { "/usr/share/netrail/static/": "../netrail/static/" } },
  "appimage": { "files": { "usr/share/netrail/static/": "../netrail/static/" } }
}
```

### 3. Better 404 page (`server/mod.rs`)

HTML explains missing assets and `NETRAIL_STATIC_DIR` — avoids “is this the app?” confusion.

### 4. `install.sh`

Copies `netrail/static/` to `~/.local/share/netrail/static`.

### 5. Immediate user workaround

```bash
mkdir -p ~/.local/opt/netrail/usr/share/netrail/static
cp -r ~/NetRail/netrail/static/. ~/.local/opt/netrail/usr/share/netrail/static/
pkill netrail; netrail &
```

---

## Remaining UX recommendations (P2+)

1. **Startup self-test** — refuse tray icon / show modal if `index.html` missing before opening webview.
2. **Unify menus** — pick native Tauri menu *or* in-app header; dual Help confuses users when one layer fails.
3. **Native menu fallback** — if `eval` fails, open docs in system browser (`/api/docs/manual` JSON → external viewer).
4. **Release QA gate** — CI step: install `.deb` in container, `curl -f http://127.0.0.1:7421/ | grep search-form`.
5. **Consider `withGlobalTauri: true`** only if needed; current design correctly keeps UI as plain web for testability.
6. **Splash screen** — never visible if index missing; post-fix OK.

---

## Verdict

This was a **packaging/architecture defect**, not user error. v1.1.0 and v1.1.1 releases shipped binaries without the web UI directory. Any desktop user hitting this saw a non-functional shell with misleading native chrome.

**Ship criterion for next patch (v1.1.2):** packaged install must serve `index.html` and `/static/app.js` before tagging.

---

*Audit triggered by production install on Ubuntu GNOME — [kayab999/NetRail](https://github.com/kayab999/NetRail)*