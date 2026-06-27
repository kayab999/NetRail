# 🛡️ Tribunal v12.0 — NetRail Audit-Fix Loop (3 Cycles)

**Date:** 2026-06-27  
**Scope:** Search regression, backend fanout, observability, blast radius  
**Stack:** Rust (Tauri) + Axum + static JS  
**Tier:** Sovereign (single-node desktop)

## Fase 0 — Context

| Field | Value |
|-------|-------|
| Software | NetRail v1.2.0 |
| Domain | Desktop metasearch console |
| State | Live / GitHub published |
| Modules | CORE, VAR-SCALE (partial), VAR-OBS |

**Non-negotiables applied:** No silent failures. Evidence before verdict. Tests precede merge.

---

## Cycle 1 — Discovery (Truth-Seeking)

### Evidence (reproducible)

```bash
# Before fix — running v1.1.1 binary
curl -sf -X POST http://127.0.0.1:7421/api/search \
  -H 'Content-Type: application/json' \
  -d '{"query":"music","mode":"web","max_results":10}'
# → results: 0, errors: [], strategy: fanout  (SILENT FAILURE)

# DDG HTML endpoint from same host
curl -sA 'Mozilla/5.0' 'https://html.duckduckgo.com/html/?q=music' | rg 'anomaly-modal'
# → bot challenge CAPTCHA HTML, zero .result nodes

# Python ddgs library (same machine)
python -c "from netrail.backends.ddgs import DDGSBackend; ..."
# → 5 results (wikipedia/grokipedia engines — not Rust HTML scraper)
```

### Findings

| ID | Sev | Finding | Blast Radius |
|----|-----|---------|--------------|
| **C01** | 🔴 P0 | DDGS HTML scraper returns 0 results when DDG serves bot challenge; fanout reports **no errors** | User sees empty UI — search appears broken |
| **R01** | 🟠 P1 | `Ok(batch)` with empty results swallowed in fanout loop | Errors invisible in logs and API |
| **R02** | 🟠 P1 | Protocol-relative DDG hrefs (`//duckduckgo.com/l/...`) not unwrapped by `url_resolve` | Dedupe/title cleanup fails when DDG works |
| **M01** | 🟡 P2 | No observability when backends return empty | Debugging requires manual curl |

### Cycle 1 Fixes

- `DDGS_BOT_CHALLENGE` error when captcha HTML detected (POST + heuristic)
- `url_resolve.rs` — `//` protocol-relative URLs
- `wikipedia.rs` — OpenSearch fallback when fanout empty (web mode)
- `tracing::warn!` on backend empty/failure paths

---

## Cycle 2 — Re-Audit + Validation

### Tests executed

```bash
cd src-tauri && cargo test          # 38+ unit/integration — PASS
cargo build --release --bin netrail # PASS
```

### Post-fix evidence

```bash
curl -sf -X POST http://127.0.0.1:7421/api/search \
  -d '{"query":"music","mode":"web","max_results":10}' ...
# → results: 10, backends: ["wikipedia"]
# → errors: ["ddgs: DuckDuckGo blocked automated HTML search (bot challenge)..."]
```

| ID | Sev | Finding | Status |
|----|-----|---------|--------|
| **C01** | 🔴 P0 | Silent zero results | ✅ Fixed — Wikipedia fallback + surfaced DDGS error |
| **R01** | 🟠 P1 | Empty batch swallowed | ✅ Fixed — `returned no results` in errors[] |
| **R02** | 🟠 P1 | Protocol-relative URLs | ✅ Fixed |
| **M01** | 🟡 P2 | No tracing | ✅ Fixed — `tracing::info/warn` on fanout paths |

### Cycle 2 Fix

- Duplicate `ddgs: ddgs:` prefix in error strings (`query_backend` used `format!("{name}: {e}")` while `NetRailError::BackendFailure` already prefixes backend name)

---

## Cycle 3 — Final Tribunal

### Scoring Matrix

| Domain | Before | After | Target |
|--------|--------|-------|--------|
| Blast Radius | 2 (silent UX break) | 0 (degraded but usable) | 0 |
| Fiabilidad | 4/10 | 7/10 | 8/10 |
| Observability | 3/10 | 6/10 | 7/10 |
| IPC / Frame-Length | N/A (HTTP JSON) | N/A | — |
| **OVERALL** | **4.5** | **7.0** | **8.0** |

### Remaining (not P0 for v1.2.0)

| ID | Sev | Item | Notes |
|----|-----|------|-------|
| **M02** | 🟡 P2 | Multi-engine parity with Python `ddgs` package | Rust uses DDGS HTML only; Python fans out to bing/wikipedia/grokipedia |
| **M03** | 🟡 P2 | Brave/SearXNG enable path in settings UI | User has brave disabled, searxng URL unset |
| **R03** | 🟠 P1 | VAR-IPC Frame-Length / Arena Reset | Out of scope — desktop app, no worker IPC |

### Destructive tests (applicable)

| Test | Result |
|------|--------|
| DDG bot challenge HTML | Detected → `DDGS_BOT_CHALLENGE` |
| Fanout total failure (all backends down) | `FANOUT_TOTAL_FAILURE` — existing test PASS |
| Partial fanout (SearXNG + dead peer) | Wiremock test PASS |
| Wikipedia fallback under DDG block | Live curl — 10 results PASS |

---

## Fase 5 — Verdict

**Estado operativo:** *Degraded-sovereign.* Search works via Wikipedia fallback when DuckDuckGo blocks the HTML scraper. Users see results **and** a visible backend error — no silent empty state.

**Narrativa:** The v1.2.0 result-UX work did not break rendering — it exposed a pre-existing fragility: Rust DDGS depended on HTML scraping that DuckDuckGo now rate-limits with CAPTCHA. The frontend was innocent; the API returned `{results: [], errors: []}` which is a P0 silent failure under Tribunal rules. Fix: fail loud on captcha, fallback to Wikipedia OpenSearch, trace all empty backend paths.

**Merge criterion met:** `cargo test` green, live `music` query returns 10 results, errors surfaced.

---

## CI Hook (recommended)

```yaml
# Add to release.yml after cargo test:
- name: Smoke search (wikipedia fallback tolerant)
  run: |
    ./src-tauri/target/release/netrail-api &
    sleep 2
    curl -sf -X POST http://127.0.0.1:7421/api/search \
      -H 'Content-Type: application/json' \
      -d '{"query":"music","mode":"web","max_results":3}' \
      | jq -e '.results | length > 0'
```

*NetRail Tribunal v12.0 — 3 cycles complete.*