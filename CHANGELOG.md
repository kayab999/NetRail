# Changelog

All notable changes to NetRail are documented here. The project follows [Semantic Versioning](https://semver.org/).

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