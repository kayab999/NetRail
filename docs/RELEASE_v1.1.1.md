# NetRail v1.1.1 — Tests, audit fixes, API parity

Patch release closing the post-v1.1.0 technical audit. Stronger regression coverage, documentation sync, and Python/Rust error contract parity.

## Highlights

- **38 Rust tests** — 8 HTTP integration tests asserting stable API `code` fields (including `FANOUT_TOTAL_FAILURE`)
- **`docs/API_ERRORS.md`** — full error code reference for API consumers
- **Python `NetRailError`** — FastAPI returns `{code, detail, status}` like Rust
- **`search::search`** — settings injected from `AppState` (testable, no hidden `load_settings`)
- **CI green** — clippy fix on `main`; docs aligned to current release artifacts
- **v1.0.0 release** — historical GitHub release published

## Downloads

| File | Use Case |
|------|----------|
| `NetRail_1.1.1_amd64.AppImage` | Desktop app (Ubuntu/Fedora/etc) |
| `NetRail_1.1.1_amd64.deb` | Debian/Ubuntu native package |
| `netrail-api` | Headless CLI / API server |
| `SHA256SUMS` | Verify artifact integrity |

## Quick start

```bash
chmod +x NetRail_1.1.1_amd64.AppImage
./NetRail_1.1.1_amd64.AppImage
```

Headless:

```bash
chmod +x netrail-api
./netrail-api --api-only
curl http://127.0.0.1:7421/api/health
```

## Since v1.1.0

No breaking API changes. Error responses gain regression test coverage; Python fallback now matches Rust error JSON shape.

See [API error codes](API_ERRORS.md) and [technical audit](AUDIT_TECHNICAL_2026-06-27.md).

---

**Repository:** [github.com/kayab999/NetRail](https://github.com/kayab999/NetRail)

*v1.1.0 typed the errors. v1.1.1 tests them.*