# NetRail v1.6.0 — Ops batch (settings ETag/If-Match, per-identity rate limits, audit rotation, JSON logs)

**Date:** 2026-08-01
**Type:** Minor

## Changed

- **Settings ETag / If-Match (dual-stack, A6)** — `GET /api/settings` and successful `PUT`s return a strong `ETag` header. `PUT /api/settings` accepts an optional `If-Match`: mismatch → 409 `SETTINGS_CONFLICT` (`{code, detail, status}` contract, documented in API_ERRORS.md); absent `If-Match` keeps the old unconditional behavior. Rust etag = serde struct-order JSON → sha256 → base64; Python = sorted-key JSON → sha256 hex (each stack self-consistent for If-Match).
- **Per-identity rate limits (dual-stack, A9)** — buckets keyed by client identity: `anonymous` when no token is configured, else `token:<base64(sha256(token))>` (identical in both stacks). 1024 identities, idle sweep, `rate_limit.mode: per-token|process` in health/status. Defaults 90/120/60 per minute; `NETRAIL_RATE_LIMIT=0` disables.
- **Audit log rotation (dual-stack, A5)** — JSONL audit rotates by size: `NETRAIL_AUDIT_MAX_BYTES` (default 10 MiB), `NETRAIL_AUDIT_MAX_FILES` (default 3, `0` disables). Files shift to `<path>.1`, `.2`, …
- **Structured JSON logs (Rust, A5)** — `NETRAIL_LOG_JSON=1` → tracing-subscriber `json()` from the new `logging.rs` in all entrypoints.
- **Test isolation fix (Python)** — `config_file()`/`config_dir()` resolve lazily; tests + smoke scripts no longer read/write the developer's real `~/.config/netrail/settings.json` (or the real history DB — both smoke scripts now isolate `XDG_CONFIG_HOME`, `NETRAIL_DB_PATH`, and `$HOME`).
- **Parity smoke** — live probes for the ETag/If-Match round trip (GET etag → stale If-Match 409 → fresh If-Match 200) run against the release binary.

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

- Roadmap: A15 DNS pin on open (P1), webview E2E (matrix #9), CI dependency audits + signing (matrix #10)
