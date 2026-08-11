# NetRail v1.6.6 — Release-Readiness Baseline

**Date:** 2026-08-11
**Type:** Patch — RC closure: security convergence (A-05/A-06/A-11) on top of the v1.6.5 release-readiness baseline.

## Fixed

- **A-10 (canonical IPv6/v4 semantics, both stacks):** Python 3.13 `is_site_local` is the deprecated RFC 3513 site-local (`fec0::/10`), not ULA; `2001::/23` and `3fff::/20` boundary bit-math corrected; Rust `Ipv4Addr::is_shared()` is unstable (E0658) → explicit CGNAT check (`100.64.0.0/10`). Final classifiers anchored on the IANA special registry (is_private, incl. `2001:db8::/32`, `2002::/16`, `3fff::/20`, ORCHIDv2 exception) + RFC 4291 reserved table.
- **A-05 (fetch-time backend SSRF guard):** backend URLs are re-validated at fetch time in both stacks — cloud metadata/link-local/unspecified always blocked, other non-public targets strict-only, empty resolution fails closed (`BACKEND_URL_DNS_UNRESOLVABLE`). Rust fetch fanout (SearXNG) gated too; `BackendKind::Searxng(url, strict)`.
- **A-06 (cipher-state model):** canonical `history.encryption_state` (`encrypted`/`degraded`/`plaintext`) on `/api/health`, both stacks, pinned by `tests/fixtures/cipher_state.json`.
- **A-11 (settings directivity):** Python `get_store()` now rebinds on `(history_enabled, history_encrypt)` change like Rust `SharedStore::ensure` (was effective only after restart). Pinned by `tests/fixtures/settings_transitions.json`.
- **UX:** footer cipher-state chip; static "encrypted history" claim removed (misleading while degraded).
- **Dead code:** write-only `_encryption_enabled` removed; clippy `-D warnings` clean.

## Verify

```bash
bash scripts/check-versions.sh                                        # all five SSOT sources + prose spots
source .venv/bin/activate && pytest tests/ -q                        # 279 passed, 1 warning
cd src-tauri && cargo test                                           # 145 passed, 0 failed
cd .. && cargo clippy --all-targets                                  # 0 warnings
.venv/bin/python scripts/fuzz-parity.py --binary src-tauri/target/release/netrail-api --full --min-urls 3000 --ci   # EXIT=0, 12 160 URLs, code_diff=0, residual 50 (known 0xzz)
bash scripts/parity-api-smoke.sh src-tauri/target/release/netrail-api  # PARITY SMOKE OK (incl. cipher-state live + directivity probes)
```

## Environment notes (local verification)

- Python 3.13.3 (ipaddress semantics verified by the A-10 probe) and Python 3.12.7 (CI runtime) — `is_private`/`is_reserved` parity probed vector-by-vector on both for the full fixture-critical set (NAT64, ORCHIDv2, `2001:db8`, `2002::`, `3fff::`, `fec0`, multicast, link-local). `is_site_local` is never used (RFC 3513 semantics, deprecated).
- Golden counts above were produced on a clean checkout state; the smoke script isolates `$HOME` (temp) and boots the release binary twice (main probes + A-11 directivity boot with an injected Fernet key).
- Known residual family pinned at 50: `0xzz` open-URL divergent codes at the DNS stage (behavioral difference between the two stacks' resolvers, documented in `scripts/fuzz-parity.py`).