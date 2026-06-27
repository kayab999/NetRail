# NetRail v1.1.0 — Release notes & architecture clarifications

Thanks to everyone who reviewed the v1.0 architecture. v1.1.0 closes the most important refactor cycle we had pending.

## What's in v1.1.0

- **`NetRailError`** — typed errors with stable `code`, HTTP `status`, and `thiserror` messages across the Rust API
- **Shared HTTP client** — connection pooling for fanout backends (v1.0.1)
- **Keyring degradation** — history opens unencrypted when Secret Service is unavailable, with UI banner (v1.0.1)
- **Native Tauri CSP** — aligned with Axum `security::CSP` (v1.0.1)
- **Wiremock test** — CI documents partial fanout merge behavior (v1.0.1)

## Architecture clarifications (from external review)

### Partial fanout failures

`search_with_fanout` already merges results and errors. If SearXNG returns 15 results and another backend times out, the API responds with HTTP 200, the results, and a populated `errors[]` array (rendered in `#fanout-errors`). HTTP error only fires on **total failure** (zero results from all backends). CI now includes a wiremock test that locks this behavior.

### CSP

CSP is injected on Axum HTTP responses (`security.rs`); the webview inherits it from `http://127.0.0.1:7421`. v1.0.1+ also hardens `tauri.conf.json` natively as a second layer.

### Error typing

v1.1.0 replaces `Result<T, String>` with `NetRailResult<T>` in security, config, backends, history, search, and server. API errors now return:

```json
{ "code": "OPEN_URL_LOCALHOST", "detail": "...", "status": 400 }
```

`InvalidOpenUrl` is separate from `InvalidBackendUrl` — different attack surfaces, different stable codes.

## Roadmap (honest)

| Item | ROI | Plan |
|------|-----|------|
| Tests per error code | High | v1.2 candidate (~2h) |
| `HashMap` errors in fanout | Low | Only if a real use case appears |
| Deprecate Python backend | Zero for now | Rust path is primary; Python stays for Docker/Flatpak |

---

**Download:** [github.com/kayab999/NetRail/releases/tag/v1.1.0](https://github.com/kayab999/NetRail/releases/tag/v1.1.0)