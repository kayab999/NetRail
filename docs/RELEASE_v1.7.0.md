# NetRail v1.7.0 — Headless Token Gate + Parity Batch

**Date:** 2026-09-16
**Type:** Minor — one breaking change (headless token requirement) plus the
request-timeout, merge-parity and coverage batch. Desktop Tauri behavior is
unchanged except the shared fanout/merge improvements.

## Breaking

- **Headless API token required:** `netrail-api` and `python -m netrail`
  (incl. Docker) exit(1) with setup instructions when `NETRAIL_API_TOKEN`
  is unset. Set a token (`openssl rand -hex 32`) or `NETRAIL_API_TOKEN=""`
  explicitly to preserve the old unauthenticated behavior (runs with a
  stderr warning). Migration: `.env` (see `.env.example`), systemd
  `EnvironmentFile` (`/etc/netrail/netrail.env`, see
  `packaging/netrail-api.service`), CI scripts (`NETRAIL_API_TOKEN=""`).
  Desktop Tauri is unaffected (token stays optional there).

## Added

- Request timeout middleware (30s default, `NETRAIL_REQUEST_TIMEOUT_SECS`
  override, `0` disables) with typed `408 REQUEST_TIMEOUT` — both stacks.
- `NETRAIL_FANOUT_DEADLINE_SECS` env override for the Rust fanout budget.
- Merge ordering follows configured backend order (was completion-ordered).
- Query `+` → `%20` normalization for URL dedupe keys (byte-exact parity).
- Case-preserving URL normalization (scheme/host lowercased, path/query
  preserved) with fragment/default-port/root-slash alignment.
- Images mode CDN privacy note in `docs/MANUAL.md`.

## Fixed

- Merge fanout completion-order bug (both callers, found by
  verification-first inspection — the audit had blamed the wrong layer).
- URL dedup over-normalization (`/Page` vs `/page`).
- Missing Rust fanout-deadline coverage (QA-10 behavior was unpinned).
- `chaos_process.rs` + all smoke/bench/load harnesses updated for the
  headless gate (token escape hatch).

## Changed

- Coverage: Rust 71.8% → 80.0% lines (`llvm-cov --all-targets`), Python
  80% → 85% — brave/ddgs/wikipedia fetch+parse matrix via
  wiremock/`MockTransport`/patched clients.
- Docs: DISTRIBUTION/SECURITY/API_ERRORS updated for the token requirement.

## Residual risks (update to HANDOVER.md §7)

- R1 (unauthenticated localhost API) is now **resolved for headless**:
  `netrail-api`/Docker require a token decision at startup. Desktop
  remains opt-in by design.

## Verify

```bash
bash scripts/check-versions.sh                                        # 1.7.0 everywhere
source .venv/bin/activate && pytest tests/ -q                        # 346 passed, 1 warning
cd src-tauri && cargo clippy --all-targets -- -D warnings && cargo test
bash scripts/e2e-api-smoke.sh src-tauri/target/debug/netrail-api     # E2E OK
bash scripts/parity-api-smoke.sh                                     # PARITY SMOKE OK
```
