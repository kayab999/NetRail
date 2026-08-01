# NetRail v1.5.0 — Persistence hardening (shared store, WAL, graceful shutdown, schema versioning)

**Date:** 2026-08-01
**Type:** Minor

## Changed

- **Persistent history store (Rust, A3)** — one SQLite connection now lives in `AppState` (`SharedStore`) for the process lifetime instead of reopening per request (each open previously re-ran the full DDL batch and TTL purge). Reopened only when history/encryption settings change. The dead `STORE` singleton cell and `get_store`/`with_store` helpers were removed; visit recording moved from `browsers::open_url` into the `open_link` handler (Python parity).
- **WAL + busy_timeout (dual-stack, A3)** — `journal_mode=WAL` and a 5 s busy timeout on both `connect()` implementations. `SQLITE_BUSY` risk under concurrent writes (two rapid searches, search + visit) is gone.
- **Graceful shutdown (Rust, A4)** — `server::start` installs SIGINT/SIGTERM handlers and drains in-flight requests before exit (`with_graceful_shutdown`). Verified live: SIGTERM → clean exit 0, WAL checkpoints on close. Docker/systemd `stop` no longer risks losing the last writes.
- **Schema versioning (dual-stack, A11)** — `PRAGMA user_version` migration framework (`SCHEMA_VERSION = 1`, ordered steps). Existing databases migrate idempotently on open; tests assert the version stamp + WAL mode on both stacks.
- **Dead desktop-bridge code removed (A7)** — the inert `focus-search` / `security:encryption-degraded` `emit` calls (unreachable with `withGlobalTauri: false`) and the `__TAURI__` listeners in app.js were removed. The eval bridge and the health-driven encryption banner remain the supported paths.
- **Tests** — 2 new Rust history unit tests (WAL/version stamp, shared-store settings toggle), 1 Python db test; `SharedStore` wired into the `api_error_codes` harness.

## Verify

```bash
bash scripts/check-versions.sh
source .venv/bin/activate && pytest tests/ -q
cd src-tauri && cargo clippy --all-targets -- -D warnings && cargo test
cargo build --release --bin netrail-api
bash scripts/e2e-api-smoke.sh
bash scripts/parity-api-smoke.sh
```

## Backlog (from `docs/AUDIT_ARCH_2026-08-01.md`)

- 1.6.0 ops: A5 audit rotation + structured logs, A6 settings ETag, A9 rate-limit documentation
- Roadmap: A15 DNS pin on open (P1), webview E2E (matrix #9), CI dependency audits + signing (matrix #10)
