# NetRail v1.1.0 — Typed errors, Linux hardening

A local, privacy-first research console built in Rust. Search multiple indexes, review links locally, and open them on your terms.

## 🚀 v1.1 Highlights

- **`NetRailError`** — stable error `code`, HTTP `status`, and human-readable `detail` across the Rust API
- **Connection pooling** — shared `reqwest::Client` for fanout backends (DNS cache + TCP reuse)
- **Keyring degradation** — WSL/i3/headless no longer fail closed; UI banner when history runs unencrypted
- **Native Tauri CSP** — double layer with Axum headers for Link Rail hardening
- **Wiremock CI test** — documents partial fanout (results + backend errors)

## 📥 Downloads

| File | Use Case |
|------|----------|
| `NetRail_1.1.0_amd64.AppImage` | Desktop app (Ubuntu/Fedora/etc) |
| `NetRail_1.1.0_amd64.deb` | Debian/Ubuntu native package |
| `netrail-api` | Headless CLI / API server (homelabs, scripting) |
| `SHA256SUMS` | Verify artifact integrity |

## API error contract (new)

```json
{
  "code": "FANOUT_TOTAL_FAILURE",
  "detail": "Fanout failed: searxng: HTTP 503",
  "status": 502
}
```

Frontend consumers can branch on `code` without parsing error strings.

## Quick start

```bash
chmod +x NetRail_1.1.0_amd64.AppImage
./NetRail_1.1.0_amd64.AppImage
```

Headless:

```bash
chmod +x netrail-api
./netrail-api --api-only
curl http://127.0.0.1:7421/api/health
```

## Since v1.0.0

**v1.0.1** — pooled HTTP client, keyring degradation, native CSP, wiremock fanout test

**v1.1.0** — `NetRailError` typed errors across security, config, backends, history, search, and server

---

**Repository:** [github.com/kayab999/NetRail](https://github.com/kayab999/NetRail)

*v1.0 was the receipt. v1.1 is the infrastructure.*