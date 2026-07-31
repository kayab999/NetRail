# NetRail v1.4.0 — Waves 3–5 (enterprise hardening path)

**Date:** 2026-07-31  
**Type:** Minor

## Wave 3 — Optional enterprise controls

| Feature | How |
|---------|-----|
| API token | `NETRAIL_API_TOKEN` + Bearer / `X-NetRail-Token` |
| Strict backends | `strict_backend_urls` or `NETRAIL_STRICT_BACKEND_URLS=1` |
| Mutation rate limits | 60/min settings/history/collections |
| Audit log | `NETRAIL_AUDIT_LOG=1` or `NETRAIL_AUDIT_LOG_PATH` |
| Rust Docker | `Dockerfile.rust`, compose `--profile rust` |
| Dep audits | CI `cargo audit` + `pip-audit` |
| SBOM | `SBOM.txt` on release artifacts |

## Wave 4 — Docs

MANUAL headless vs desktop flags; DISTRIBUTION env table complete; privacy/rate-limit honesty; softened unverified size/latency claims.

## Wave 5 — Tests

- `scripts/parity-api-smoke.sh`
- Expanded `tests/test_api.py`
- Rate-limit test seam (`rate_limit.set_test_limits`)

## Verify

```bash
bash scripts/check-versions.sh
source .venv/bin/activate && pytest tests/ -q
cd src-tauri && cargo clippy --all-targets -- -D warnings && cargo test
cargo build --release --bin netrail-api --no-default-features
bash scripts/e2e-api-smoke.sh
bash scripts/parity-api-smoke.sh
```
