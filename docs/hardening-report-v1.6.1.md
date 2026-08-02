# NetRail Hardening Report — v1.6.1 (Sprints 2–4)

Date: 2026-08-02 · Branch: `sprint2/chaos-fault-injection` · Baseline: `bc3f176`

Hardening program covering chaos/fault injection (Sprint 2), resource stability
(Sprint 3), and dual-stack benchmarks (Sprint 4). Everything below was executed
live against the release `netrail-api` binary and the Python FastAPI/uvicorn
stack.

## 1. Sprint 2 — Chaos / fault injection

Deliverables: `scripts/chaos/harness.sh` (`gate`, `live-busy`, `live-kill`),
`src-tauri/tests/chaos_db.rs`, `src-tauri/tests/chaos_process.rs`,
`tests/test_chaos.py`, plus a dedicated `chaos` job in CI
(`.github/workflows/ci.yml`).

| Fault injected | Expected | Verified |
|---|---|---|
| SQLite `SQLITE_BUSY` (writer held) | Readers unaffected (WAL); writes return typed `500 DB_ERROR`; recovery **without restart** after unlock | Pass |
| Unwritable DB directory | Graceful `HISTORY_DISABLED` degrade (typed 400), auto-recovery when writable again | Pass |
| `SIGKILL` mid-session | WAL data intact after restart | Pass |
| `SIGINT` | Clean exit 0 | Pass |
| External audit-log rotation (logrotate-style) | No JSONL loss | Pass |

Fixes surfaced by the suite:

- **Python typed DB errors** — `sqlite3.Error` now maps to the `{code, detail, status}` contract (`DB_ERROR`, 500) instead of an untyped 500 (`netrail/main.py` exception handler).
- **Python store degrade** — `get_store()` catches `sqlite3.Error` on open and returns history-disabled (typed `HISTORY_DISABLED`) instead of crashing (Rust `SharedStore` parity).
- **Read-only test flake** — `readonly_mode` tests serialized (env-var race).

## 2. Sprint 3 — Resource stability

Deliverables: `scripts/load-10k.sh`, `scripts/load/run.py`,
`scripts/load/slope.py`. 10k sequential + 1k concurrent `GET /api/health`,
sampling the server PID's RSS / FD count / established sockets every 500
requests, with a linear-regression slope analysis (two-sided p + practical
budget: RSS < 1 MiB / 10k req, FDs and sockets within ±2 / 10k req).

Result: **both stacks stable** — full report in `docs/sprint3-slope.md`.

**Finding (fixed):** the Python stack dropped concurrent requests
(`ConnectionResetError`) under the load phase. Root cause: the shared
`sqlite3.Connection` (`check_same_thread=False`) was used from FastAPI's
threadpool threads with no lock, so `fetchone()` could return `None` → untyped
TypeError. Fix: all `HistoryStore` methods now take a reentrant lock
(`netrail/history/store.py`); regression test hammers 16 threads × 100 mixed
read/write ops. After the fix the Python stack reached 100% completeness.

## 3. Sprint 4 — Dual-stack benchmarks

Deliverables: `scripts/bench-dual.sh`, `scripts/bench/bench.py`,
`scripts/bench/report.py`. asyncio httpx keep-alive client, 3 steady-state
runs (C=16, N=2000) + saturation scan doubling concurrency until the error
rate exceeds 1% (sampling server CPU/RSS). Full report: `docs/bench-dual.md`.

| Metric (median) | Rust (Axum) | Python (FastAPI/uvicorn) |
|---|---:|---:|
| Throughput | ~573 rps | ~295 rps |
| p50 | 23 ms | 39 ms |
| p95 | 58 ms | 127 ms |
| errors @ C=16 | 0% | 0% |
| Server CPU | ~14% | ~74% |
| Peak RSS | ~10.4 MiB | ~64.1 MiB |
| Saturation knee | none ≤ C=512 (p95 degrades) | none ≤ C=512 (p95 degrades) |

Neither stack exceeds the 1% error budget through C=512; p95 latency
degradation is the practical knee signal. Figures are single-core-local and
health-endpoint-specific (settings/keyring/DB work included).

## 4. D-tasks / regression gates

- `cargo clippy --all-targets -- -D warnings` — clean.
- `cargo test` — 109 passed / 0 failed.
- `pytest tests/` — 133 passed (incl. new concurrency + S1 property tests).
- `scripts/parity-api-smoke.sh` — PARITY SMOKE OK.
- `scripts/e2e-api-smoke.sh` — E2E API SMOKE OK.
- CI: new `chaos` job added (Rust chaos tests, Python chaos tests, live
  `live-busy` / `live-kill` scenarios).

## 5. Scorecard delta

The audit matrix "Performance 7 — no formal load tests" (E1) is now closed:
this report plus `docs/sprint3-slope.md` and `docs/bench-dual.md` provide
load/leak/perf coverage on both stacks.
