# Sprint 4 — Dual-stack benchmarks

Endpoint: `http://127.0.0.1:7421/api/health`. Client is an asyncio httpx keep-alive pool; the
1% error budget defines the saturation knee. Note: `tokio::runtime::metrics`
is not enabled in shipped binaries.

Medians across the 3 steady-state runs (C = 16, N = 2000):

| stack | rps | p50 (ms) | p95 (ms) | errors % | CPU % | peak RSS |
|---|---|---|---|---|---|---|
| Rust (Axum) | 573 | 22.89 | 58.09 | 0.00 | 14 | 10.4 MiB |
| Python (FastAPI/uvicorn) | 295 | 39.19 | 127.47 | 0.00 | 74 | 64.1 MiB |

Neither stack exceeded the 1% error budget at C = 512 (largest scanned); p95 latency degradation is the practical knee indicator. These are single-core-local figures — the health endpoint also performs settings/keyring/DB work, so absolute rps is workload-specific, not a ceiling.
## Rust (Axum)

Endpoint: `http://127.0.0.1:7421/api/health` — client: asyncio httpx keep-alive (C = 16,
N = 2000 per run, 3 runs). CPU/RSS sampled from the server PID.

| C | rps | p50 (ms) | p95 (ms) | p99 (ms) | errors % | CPU % | peak RSS (KiB) |
|---|---|---|---|---|---|---|---|
| 16 | 507 | 25.43 | 71.20 | 95.09 | 0.00 | 14 | 10652 |
| 16 | 599 | 22.69 | 53.87 | 87.87 | 0.00 | 14 | 10700 |
| 16 | 573 | 22.89 | 58.09 | 93.23 | 0.00 | 16 | 10840 |

Saturation scan (stop at >1% errors):

| C | rps | p95 (ms) | errors % |
|---|---|---|---|
| 16 | 491 | 61.98 | 0.00 |
| 32 | 447 | 198.43 | 0.00 |
| 64 | 486 | 365.98 | 0.00 |
| 128 | 303 | 1294.38 | 0.00 |
| 256 | 384 | 1935.09 | 0.00 |
| 512 | 248 | 5943.16 | 0.00 |

No knee within the scanned range: 512 concurrent stayed under 1% errors.

## Python (FastAPI/uvicorn)

Endpoint: `http://127.0.0.1:7421/api/health` — client: asyncio httpx keep-alive (C = 16,
N = 2000 per run, 3 runs). CPU/RSS sampled from the server PID.

| C | rps | p50 (ms) | p95 (ms) | p99 (ms) | errors % | CPU % | peak RSS (KiB) |
|---|---|---|---|---|---|---|---|
| 16 | 353 | 34.84 | 105.25 | 149.14 | 0.00 | 74 | 65264 |
| 16 | 295 | 39.19 | 127.47 | 238.28 | 0.00 | 78 | 65600 |
| 16 | 277 | 42.50 | 134.34 | 261.51 | 0.00 | 70 | 65772 |

Saturation scan (stop at >1% errors):

| C | rps | p95 (ms) | errors % |
|---|---|---|---|
| 16 | 322 | 111.02 | 0.00 |
| 32 | 235 | 384.61 | 0.00 |
| 64 | 208 | 1004.38 | 0.00 |
| 128 | 234 | 1726.04 | 0.00 |
| 256 | 251 | 2840.01 | 0.00 |
| 512 | 247 | 5620.30 | 0.00 |

No knee within the scanned range: 512 concurrent stayed under 1% errors.
