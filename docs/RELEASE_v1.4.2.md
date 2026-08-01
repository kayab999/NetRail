# NetRail v1.4.2 — Typed-error contract (Rust) + CSP-safe token mode

**Date:** 2026-08-01  
**Type:** Patch

## Fixed

- **Typed errors on malformed request bodies (Rust)** — axum extractor rejections (missing fields, wrong types, broken JSON, bad query params) previously returned plain-text HTTP 422 with no `{code, detail, status}` body. They now map to typed 400s with field-aware codes mirroring Python: `QUERY_INVALID`, `OPEN_URL_INVALID`, `CONFIG_MAX_RESULTS`, `COLLECTION_NAME_INVALID`, `COLLECTION_ITEM_TITLE_INVALID`, `COLLECTION_ITEM_NOTES_INVALID`, `REQUEST_INVALID`. Found by the architecture audit (`docs/AUDIT_ARCH_2026-08-01.md`, finding A1).
- **`max_results` out-of-range parity (Rust)** — `/api/search` with `max_results` outside 1–50 now returns `CONFIG_MAX_RESULTS` 400 instead of silently clamping (Python already errored).
- **Token mode UI broken by CSP (dual-stack)** — with `NETRAIL_API_TOKEN` + `NETRAIL_INJECT_UI_TOKEN` set, the injected `window.NETRAIL_API_TOKEN` inline script was blocked by the page's own `script-src 'self'`, so the UI could not authenticate (every API call 401). The index response CSP now carries the exact `sha256-…` hash of the injected script — token mode works while all other inline scripts remain blocked. Found by the architecture audit (A2).

## Changed

- **Architecture audit published** — `docs/AUDIT_ARCH_2026-08-01.md`: first code-as-built architecture audit of both stacks — module inventory with line anchors, request flows, concurrency/state model, dual-stack parity table, security assessment, enterprise-readiness gaps, 15 findings (A1–A13, A15) with severities and recommendation matrix. Both P1s closed in this release; P2/P3 backlog tracked for 1.5.0/1.6.0.
- **Parity harness hardened** — 5 new live probes (missing field, wrong type, broken JSON, out-of-range `max_results`, missing `url`) asserting typed 400s on both stacks.
- **Tests** — 7 new Rust integration tests (`api_error_codes.rs`), 2 CSP unit tests, Python typed-body + token/CSP tests.

## Verify

```bash
bash scripts/check-versions.sh
source .venv/bin/activate && pytest tests/ -q
cd src-tauri && cargo clippy --all-targets -- -D warnings && cargo test
cargo build --release --bin netrail-api --no-default-features
bash scripts/e2e-api-smoke.sh
bash scripts/parity-api-smoke.sh
```
