# NetRail v1.6.4 — Security Remediations

**Date:** 2026-08-02
**Type:** Patch — addressing open items from the security and correctness audit report (NR-01..NR-07).

## Added / Fixed

- **NR-01 (P2):** Mutation rate limiting was added to the `POST /api/collections/{collection_id}/items` endpoint on both the Rust and Python stacks.
- **NR-02 / NR-08 (P2):** Atomic settings write via unique same-dir temp + POSIX `rename`/`os.replace` (Python `tempfile.mkstemp`, Rust `pid.seq.nanos`). Concurrent multi-thread saves no longer race on a PID-only temp path. I/O failure code `CONFIG_SAVE_FAILED` is HTTP 500.
- **NR-03 (P3):** Added `collection.item.add` audit log event recording the collection ID and safe hostname, matching the open-URL audit policy.
- **NR-04 (P3):** Documented product decision on read-only mode (`NETRAIL_READONLY=1`) in `SECURITY.md`, `DISTRIBUTION.md`, and `API_ERRORS.md`: mutation endpoints locked; search/visit history remains active by design.
- **NR-05 (P3):** API token comparison is now constant-time (Python `hmac.compare_digest`, Rust SHA-256 digest XOR folding).
- **NR-06 (P3):** Refreshed `CONTEXT_DUMP_2026-08-02.md` SSOT to **1.6.4** and completion status of S1–S4 hardening metrics.
- **NR-07 (P3):** Homoglyph/Unicode hostname validation edge cases remain accepted residuals covered by the DNS pinning guard.

## Verify

```bash
bash scripts/check-versions.sh
source .venv/bin/activate && pytest tests/ -q                     # 165 passed
cd src-tauri && cargo clippy --all-targets -- -D warnings && cargo test  # 113 passed
```
