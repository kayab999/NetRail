# NetRail v1.2.0 — Search Results UX

Minor release focused on making packaged installs and search results usable for real demos.

## Highlights

- **Clean search results** — decoded URLs, visible snippets, result counter, pagination
- **DDG redirect resolution** — fanout dedupe works across backends (`url_resolve.rs`)
- **Packaged UI fixed** — static assets bundled in `.deb` / AppImage; runtime `static_dir` resolution
- **Tray menu** — Show / Quit work on Linux (right-click tray)

## Downloads

| File | Use Case |
|------|----------|
| `NetRail_1.2.0_amd64.AppImage` | Desktop app (Ubuntu/Fedora/etc) |
| `NetRail_1.2.0_amd64.deb` | Debian/Ubuntu native package |
| `netrail-api` | Headless CLI / API server |
| `SHA256SUMS` | Verify artifact integrity |

## Quick start

```bash
chmod +x NetRail_1.2.0_amd64.AppImage
./NetRail_1.2.0_amd64.AppImage
```

Headless:

```bash
chmod +x netrail-api
./netrail-api --api-only
curl http://127.0.0.1:7421/api/health
```

## Search results UX

### Fixed

- DDGS URLs now resolve redirects (no more `duckduckgo.com/l/?uddg=...`)
- Fanout dedupe now works correctly across backends
- URLs displayed are clean, decoded, and truncated (~72 chars)

### Added

- Search result snippets (3 lines visible)
- Result counter: `12 results for "query" via DDGS, Brave`
- Pagination: first 10 results + **Show 10 more** button
- Improved CSS: larger titles, monospace URLs with ellipsis

### Changed

- Backend: new `url_resolve.rs` module for DDG redirect unwrapping
- Frontend: cleaner result cards with backend badges

## Desktop shell (since v1.1.1)

- Static UI bundled in Linux packages; no more `index.html not found`
- Tray contextual menu: **Show NetRail** and **Quit** (right-click on Linux)
- Wayland window focus improved (`Ctrl+Shift+S` global shortcut)

## Since v1.1.1

No breaking API changes. UX and packaging fixes only.

---

*v1.1.1 hardened the API. v1.2.0 makes the product legible.*