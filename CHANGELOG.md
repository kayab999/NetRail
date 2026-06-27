# Changelog

All notable changes to NetRail are documented here. The project follows [Semantic Versioning](https://semver.org/).

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

[0.1.0]: https://github.com/your-org/NetRail/releases/tag/v0.1.0