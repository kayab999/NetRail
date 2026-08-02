# NetRail — Comprehensive Context Dump (2026-08-02)

| Field | Value |
| :--- | :--- |
| **Product** | NetRail — Sovereign Local Research Console (Linux) |
| **Version SSOT** | **1.6.1** (Enforced across 5 configuration files) |
| **License** | AGPL-3.0 |
| **Repository** | https://github.com/kayab999/NetRail |
| **Primary Stack** | Rust Axum API + Tauri 2 Desktop Shell (`src-tauri/`) |
| **Secondary Stack** | Python FastAPI Engine (`netrail/`) for Docker / Flatpak / Tests |
| **API Bind** | `127.0.0.1:7421` (Localhost-only invariant) |
| **UI Stack** | Vanilla HTML / CSS / JS (`netrail/static/`) — No heavy SPA frameworks |
| **API Contract** | **1.4** |

---

## 1. System Invariants & Security Architecture

1. **Localhost Binding Invariant:** Server binds strictly to `127.0.0.1:7421`. Exposing to external interfaces without authorization is prohibited.
2. **Zero Telemetry Policy:** No remote logging, user tracking, or telemetry endpoints exist within the codebase.
3. **SSRF & Open-URL Guard (Pre-Spawn DNS Pinning — A15):**
   - Protocol restriction: `http://` and `https://` only.
   - Rejection of embedded credentials (`user:pass@host`).
   - Host normalization: WHATWG-compliant percent-decoding, lowercasing, and trailing-dot stripping (`127.0.0.1.` $\rightarrow$ `127.0.0.1`).
   - Rejection of loopback addresses across formats (dotted decimal, hex `0x7f000001`, octal `0177.0.0.1`, short IPv4 `127.1`, IPv4-mapped IPv6 `[::ffff:127.0.0.1]`).
   - Rejection of private IP ranges (RFC 1918), link-local (`169.254.0.0/16`), and AWS/GCP cloud metadata endpoints (`169.254.169.254`, `fd00:ec2::254`, `metadata.google.internal`).
   - Rejection of DNS rebinding apex domains (`nip.io`, `sslip.io`, `xip.io`, `localtest.me`).
   - Pre-spawn resolution (`pin_open_host`): System DNS lookup occurs *before* launching the browser to verify that resolved IPs pass all blocklists. Fail-closed on empty resolution (`OPEN_URL_DNS_UNRESOLVABLE`).
4. **Typed Error Schema:** All API errors follow the uniform contract:
   ```json
   {
     "code": "OPEN_URL_LOCALHOST",
     "detail": "Localhost URLs cannot be opened from search results.",
     "status": 400
   }
   ```
5. **Read-Only Mode (`NETRAIL_READONLY=1`):** Gates all state mutations (settings, history deletion, collections) with HTTP `403 READONLY_MODE`.
6. **Audit Logging & Rotation (A5):** JSONL audit log capped at 10 MiB with automatic rotation (`audit.log.1..3`).

---

## 2. Hardening & Verification Program (Sprint Status)

```
[S1: Invariants & Fuzzing] (COMPLETE) ──> [S2: Chaos & Faults] (NEXT) ──> [S3: Resource Stability] ──> [S4: Latency & Benchmarks]
```

### Sprint 1: Property & Invariant Verification (COMPLETED)
- **Host Normalization Idempotency:** $\text{normalize}(\text{normalize}(h)) == \text{normalize}(h)$ verified in Rust and Python.
- **IPv4 Loose Parser Zero-Panic Guarantee:** `parse_browser_ipv4` fuzzed against invalid byte sequences and edge cases without panics.
- **Pre-spawn DNS Pinning Invariants:** Verified fail-closed logic on empty DNS responses and automatic block on private IP responses.
- **Test Metrics:**
  - **Rust:** 105 tests passed (76 lib + 19 API + 6 readonly + 4 property tests).
  - **Python:** 129 tests passed (`pytest`).
  - **Clippy:** Clean (`-D warnings`).

### Sprint 2: Chaos & Fault Injection Testing (UP NEXT)
- Verification of typed error degradation under SQLite locks (`SQLITE_BUSY`), write-restricted filesystems, and abrupt SIGINT signals.

### Sprint 3: Resource Stability & Slope Analysis
- Leak detection across 10,000 requests measuring RSS, File Descriptors (`/proc/self/fd`), Tokio task count, and active sockets.

### Sprint 4: Dual-Stack Capacity & Latency Benchmarks
- Quantitative benchmarking of Rust (Axum) vs Python (FastAPI) covering p50/p95/p99 latency, throughput, and CPU/memory utilization under saturation.

---

## 3. Version Single Source of Truth (SSOT)

To maintain version alignment, any release bump must update all 5 SSOT locations simultaneously:
1. `package.json`
2. `src-tauri/Cargo.toml`
3. `src-tauri/tauri.conf.json`
4. `netrail/__init__.py`
5. `src-tauri/src/config.rs` (`VERSION` constant)

Validation Command:
```bash
bash scripts/check-versions.sh
```

---

## 4. Operational Commands & Test Execution

```bash
# 1. Verify Version SSOT
bash scripts/check-versions.sh

# 2. Run Rust Property & Unit Tests
cd src-tauri
cargo clippy --all-targets -- -D warnings
cargo test
cd ..

# 3. Run Python Test Suite
source .venv/bin/activate
pytest tests/ -q

# 4. Build Headless Rust API & Run E2E Smoke Tests
cargo build --release --manifest-path src-tauri/Cargo.toml --bin netrail-api --no-default-features
NETRAIL_RATE_LIMIT=0 bash scripts/e2e-api-smoke.sh
NETRAIL_RATE_LIMIT=0 bash scripts/parity-api-smoke.sh
```
