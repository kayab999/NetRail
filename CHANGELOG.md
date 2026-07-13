# Changelog

All notable changes to NetRail are documented here. The project follows [Semantic Versioning](https://semver.org/).

## [Unreleased]

## [1.2.2] — 2026-07-12

### Fixed

- **CI clippy** — allow intentional `TrayState` retention of tray icon (Linux); restores green `main`
- **Version single source of truth** — Python, README, MANUAL, and package metadata aligned to 1.2.2
- **Open-URL encoded loopback** — block decimal/hex/octal/short IPv4 forms browsers resolve to localhost (`OPEN_URL_LOCALHOST`)
- **Open-URL private nets** — block RFC1918 / ULA / non-public IPs from result opens (`OPEN_URL_PRIVATE`); SearXNG backends may still use LAN
- **Python validation errors** — FastAPI 422 mapped to `{code, detail, status}` (`QUERY_INVALID`, etc.)

### Changed

- **Release workflow** — runs `cargo clippy --all-targets -- -D warnings` before tests (same gate as CI)
- **SECURITY.md** — supported versions include 1.2.x; open-URL and encryption-degrade docs expanded
- **DISTRIBUTION.md** — AppImage requires `patchelf` locally; CI already installs it

### Added

- **RC / adversarial audits** — `docs/AUDIT_RC_2026-07-12.md`, `AUDIT_POSTFIX_*`, `AUDIT_ADVERSARIAL_QA_*`, `docs/RELEASE_v1.2.2.md`
- **Python Wikipedia fallback** — OpenSearch + extracts when fanout is empty (parity with Rust)
- **Python `search_recovery`** — `/api/health` hints for SearXNG / Brave
- **Empty backend errors (Python)** — zero-result batches surface in `errors[]` like Rust
- **Local packaging** — `.deb` / `.rpm` / `netrail-api` with static UI; AppImage via CI when `patchelf` available
- **Local rate limits** — 90 searches / 120 opens per minute (`RATE_LIMITED` 429); disable with `NETRAIL_RATE_LIMIT=0`
- **A11y polish** — skip link, tab `aria-selected`, focus-visible, reduced-motion, labeled controls
- **Image privacy** — `referrerpolicy=no-referrer` on result thumbnails
- **CI version SSOT** — `scripts/check-versions.sh` fails the pipeline on drift
- **Health** — `api_contract` + `rate_limit` status; UI footer shows app version
- **API E2E smoke** — `scripts/e2e-api-smoke.sh` (search + open-url + UI assets); run on CI and release
- **Release** — AppImage/deb required artifacts; publish non-draft on tag; dual-stack support policy documented

## [1.2.1] — 2026-06-27

### Added

- **Wikipedia intro extracts** — OpenSearch + `prop=extracts` for readable fallback snippets
- **`search_recovery` on `/api/health`** — hints for SearXNG / Brave when DDGS is blocked
- **`docs/RELEASE_v1.2.1.md`** — patch release notes

### Fixed

- **Empty Wikipedia cards** — no more blank / “No description available” on Wikipedia-only results
- **Splash / startup** — `RESULTS_PAGE_SIZE` TDZ crash; 2.5s HTML failsafe
- **Error banner** — shorter DDGS message, dismiss control, recovery tips

### Changed

- **Fanout error UX** — dismissible banner; Wikipedia-only recovery messaging

## [1.2.0] — 2026-06-27

### Added

- **`url_resolve.rs`** — unwrap DuckDuckGo redirect URLs (`uddg`) for clean results and fanout dedupe
- **Search result UX** — snippets (3 lines), result counter, pagination (10 + “Show more”)
- **`docs/RELEASE_v1.2.0.md`** — release notes for packaging workflow

### Changed

- **Frontend** — decoded/truncated display URLs (~72 chars), improved result card CSS
- **Version** — 1.2.0 across Rust, Tauri, npm, and Python

### Fixed

- **DDGS backend** — resolve redirect hrefs to destination URLs; cleaner titles from `.result__url`
- **Fanout dedupe** — normalize resolved URLs before deduplication (Python + Rust)
- **Packaged UI** — bundle `netrail/static/` in `.deb` / AppImage; runtime `static_dir` (fixes `index.html not found`)
- **Tray menu** — Show / Quit on Linux; remove duplicate config tray icon; drop `prevent_exit()` blocking quit

## [1.1.1] — 2026-06-27

### Added

- **Error-code regression tests** — 8 HTTP integration tests (`tests/api_error_codes.rs`) plus unit coverage for `error`, `config`, and fanout total failure
- **`build_router(state)`** — extracted Axum router for testable API surface
- **Technical audit** — `docs/AUDIT_TECHNICAL_2026-06-27.md`
- **`docs/API_ERRORS.md`** — stable error code reference for API consumers
- **Python `NetRailError`** — FastAPI handler returns `{code, detail, status}` (parity with Rust)

### Changed

- **`search::search`** — accepts injected `Settings` from `AppState` (testable fanout total failure)
- **Docs sync** — `MANUAL.md`, `DISTRIBUTION.md`, `ARCHITECTURE.md` aligned to current release

### Fixed

- **CI clippy** — `unnecessary_sort_by` in `browsers.rs` (Rust 1.96)
- **README / package-lock** — install artifact names aligned to release version
- **GitHub** — v1.0.0 draft release published as historical release

## [1.1.0] — 2026-06-27

### Added

- **`NetRailError`** — typed errors with stable `code`, HTTP `status`, and `thiserror` messages across Rust API
- API JSON errors now include `code`, `detail`, and `status` (frontend can branch on `code`)

### Changed

- Migrated `security`, `config`, backends, `history`, `search`, and `server` from `Result<T, String>` to `NetRailResult<T>`
- Fanout partial backend failures still surface as human-readable strings in `errors[]`; total failure uses `FANOUT_TOTAL_FAILURE`

## [1.0.1] — 2026-06-27

### Added

- **Shared HTTP client** — single pooled `reqwest::Client` in API state for fanout backends
- **Keyring degradation** — history opens unencrypted when Secret Service is unavailable (WSL/i3/headless), with UI banner and Tauri event
- **Native Tauri CSP** — aligned with Axum `security::CSP` in `tauri.conf.json`
- **Wiremock test** — documents partial fanout (results + backend errors)

### Changed

- Invalid search `mode` values log `tracing::warn!` before defaulting to web

## [1.0.0] — 2026-06-27

### Added

- **Multi-backend fanout** — concurrent queries to all enabled backends via `tokio::join!` / thread pool
- **Merge & dedupe** — URL normalization (strip `www.`, tracking params), richer-snippet wins, round-robin interleave
- **Brave Search API** — BYO key via `BRAVE_SEARCH_API_KEY` env (never stored in settings)
- **Structured `backends` config** — optional array in `settings.json` alongside legacy `backend_order`
- **`search_strategy`** — `fanout` (default) or `fallback` for legacy sequential behavior
- **`netrail-api` binary** — headless server (`cargo build --bin netrail-api --no-default-features`)
- **UI: keyboard navigation** — ↑/↓ highlight, Enter open, Shift+Enter private, Ctrl+C copy URL
- **UI: export rail** — JSON export (Shift+click for CSV) from header button
- **UI: backend pills** — `[DDGS]` / `[SearXNG]` / `[Brave]` provenance badges
- **GitHub Actions** — release workflow builds AppImage, `.deb`, and `netrail-api` on tag push

### Changed

- Sovereignty step 3 when Brave or SearXNG contributes results
- README rewritten for production launch positioning
- OPEN_LETTER postscript for v1.0

## [0.5.0] — 2026-06-27

### Added

- **Rust port** — Axum HTTP server inside Tauri 2 binary; no Python sidecar
- **`src-tauri/`** — Full API parity: search, history, collections, browsers, settings
- **Fernet compatibility** — Reads v0.4 encrypted SQLite databases via OS keyring / `NETRAIL_DB_KEY`
- **DDGS HTML scraper** — `reqwest` + `scraper` backend (same provenance chain as Python)
- **SearXNG backend** — JSON API client with health check
- **Tauri desktop shell** — System tray, `Ctrl+Shift+S` global hotkey, single-instance lock
- **`--api-only`** — Headless mode for scripting (`curl http://127.0.0.1:7421/api/health`)
- **`npm run build`** — Tauri AppImage / `.deb` / `.rpm` via GitHub Actions-ready toolchain

### Changed

- UI (`netrail/static/`) unchanged — webview loads `http://127.0.0.1:7421`
- Python `netrail/` retained as optional headless fallback (`install.sh` auto-detects Tauri binary)
- Cold start: native binary reaches API in &lt;100ms vs ~2s Python cold start

### Technical

- Crate mapping: `axum`, `rusqlite` (bundled), `fernet`, `keyring`, `reqwest`, `scraper`
- Rust unit tests for Fernet roundtrip and encrypted history migration

## [0.4.0] — 2026-06-27

### Added

- **Flatpak** packaging with `flatpak-spawn --host` browser launches
- **Docker** image + `docker-compose.yml` (strict `127.0.0.1` bind, SearXNG profile)
- **AppImage** build via PyInstaller + `appimagetool`
- **`install.sh`** — one-command local desktop install
- SVG icon and `.desktop` file for application menu integration
- Auto-open UI on startup (`NETRAIL_AUTO_OPEN`, default `true`)
- Env config: `SEARXNG_URL`, `NETRAIL_*` overrides for Docker/homelab
- [docs/DISTRIBUTION.md](docs/DISTRIBUTION.md) — packaging and sandbox guide

### Changed

- `browsers.py` detects Flatpak sandbox and routes host browser spawns
- `main.py` uses `runtime.static_dir()` for PyInstaller compatibility

## [0.3.0] — 2026-06-27

### Added

- Local SQLite history at `~/.local/share/netrail/netrail.db`
- Field-level Fernet encryption (OS keyring or `NETRAIL_DB_KEY` env)
- FTS5 full-text search over past queries
- Visit tracking with revisit metadata on search results
- Research collections with save-to-collection UI and CSV/JSON export
- History tab: local search, re-run, per-entry delete, purge all
- Auto-purge via `history_ttl_days` (default 90)
- API: `/api/history`, `/api/collections`, collection items and export
- `result_id` on search results; visits recorded on `/api/open`
- Sovereignty step 4 when local history is active

### Settings

- `history_enabled` (default `true`)
- `history_encrypt` (default `true`)
- `history_ttl_days` (default `90`)

## [0.2.0] — 2026-06-27

### Added

- `SearchBackend` protocol and `netrail/backends/` package
- SearXNG backend (configure `searxng_url` in settings)
- Backend fallback chaining and result deduplication
- Backend provenance in API responses and UI badges
- Sovereignty step indicator (1–5) in header
- `GET /api/backends` endpoint
- Content-Security-Policy and security headers
- Stricter URL validation (`netrail/security.py`)
- Test suite: API, backends, security
- [docs/VIABILITY.md](docs/VIABILITY.md) — product assessment and strategic response

### Changed

- Open Letter rewritten for radical honesty about default index chain
- Architecture roadmap restructured (credibility → retention → distribution → Rust shell)
- Tagline: *Search first. Browse second. On your terms.*

## [0.1.0] — 2026-06-27

### Added

- Local FastAPI server bound to `127.0.0.1:7421`
- Web and image metasearch via `ddgs` with operator passthrough
- Link rail UI with browser picker and private/incognito mode
- REST API: `/api/search`, `/api/open`, `/api/browsers`, `/api/settings`, `/api/health`
- XDG settings persistence at `~/.config/netrail/settings.json`
- AGPL-3.0 license and open letter manifesto
- Documentation: README, user manual, architecture blueprint

### Security

- No telemetry, analytics, or accounts
- URL open restricted to `http://` and `https://` schemes
- Localhost-only server bind in v0.1

[1.2.2]: https://github.com/kayab999/NetRail/releases/tag/v1.2.2
[1.2.1]: https://github.com/kayab999/NetRail/releases/tag/v1.2.1
[1.2.0]: https://github.com/kayab999/NetRail/releases/tag/v1.2.0
[1.1.1]: https://github.com/kayab999/NetRail/releases/tag/v1.1.1
[1.1.0]: https://github.com/kayab999/NetRail/releases/tag/v1.1.0
[1.0.1]: https://github.com/kayab999/NetRail/releases/tag/v1.0.1
[1.0.0]: https://github.com/kayab999/NetRail/releases/tag/v1.0.0
[0.1.0]: https://github.com/kayab999/NetRail/releases/tag/v0.1.0