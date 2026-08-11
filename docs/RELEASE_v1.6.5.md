# NetRail v1.6.5 — Release-Readiness Baseline

**Date:** 2026-08-10
**Type:** Patch — release-readiness fixes and gates (QA-01..QA-15), SHIP-GRADE 4.65 baseline (Baseline #2).

## Fixed

- **QA-01 (P0):** clippy `-D warnings` gate green as-built.
- **QA-02 (P2):** flaky `settings_put_with_fresh_if_match…` integration test — rate-limit state isolated; 8/8 full-suite runs.
- **QA-13 (P0, found by the QA-05 webview gate):** UI dead in the desktop webview — CSP `upgrade-insecure-requests` removed (Rust + Python). WebKitGTK upgrades loopback http subresources to https; the plain-http UI server then fails every asset (white screen, no browser detection, dead eval bridges). Proven with a 4-variant A/B in a real WebKit engine; regression pinned in `tests/test_api.py`.
- **QA-09 (P2):** browser-discovery parity — Rust/Python aligned on canonical 13-browser fixture SSOT.
- **QA-10 (P2):** fanout deadline symmetry (Rust `JoinSet`+`select!`, Python cancel+bounded wait); 5-property contract.
- **QA-12 (P3):** cross-doc link-integrity checker (40 docs), CI-gated.
- **QA-14 (P3):** `api_error_codes` flake (QA-02 family) — `empty_collection_name…`/`missing_history_entry…` raced the env vars of the serialized env group; a mid-run env steal made `HistoryStore::open` fall back to the real user DB → `locked`/`disk I/O error` → spurious `HISTORY_DISABLED` 400. Both now serial; was 1/6–1/15 of runs, now 0/200 + 0/8 full-suite.
- **QA-15 (P3):** `build-desktop-linux.sh` SHA256SUMS idempotency (self-referenced checksum file broke re-run verification).
- **QA-06 (P3):** prose-version drift closed to SSOT (5 docs) + `check-versions.sh` prose spot-lists.

## Added

- **qa-03 (P2):** differential open-URL fuzz CI-gated (`scripts/fuzz-parity.py --ci --binary`); `code_diff=0` on 7 600 URLs.
- **QA-04/T5:** coverage observability per merge (Python 77%, Rust 57.5% lines).
- **QA-05 (T6):** webview E2E as explicit manual pre-tag release gate.
- **QA-11 (P3):** `check-versions.sh` covers CHANGELOG top entry + HEAD-tag advisory.
- **Baseline #2:** 4.12 → **4.65 SHIP-GRADE**, release gate UNBLOCKED.

## Verify

```bash
bash scripts/check-versions.sh                                  # 5/5 sources + prose spots
source .venv/bin/activate && pytest tests/ -q                   # 212 passed
cd src-tauri && cargo test                                      # 8/8 full-suite passes (QA-14 fixed)
cd .. && bash scripts/package-smoke.sh dist/release/netrail-api # E2E ok (search 5/0, open-url blocks 2)
bash scripts/webview-e2e.sh                                     # WEBVIEW E2E: 6/6 passed (QA-05 gate, incl. QA-13)
# artifact gates (local, full pass exit 0, 110M in dist/release/):
#   sha256sum -c SHA256SUMS      — all coincide, idempotent across re-runs (QA-15)
#   AppImage: desktop-file-validate 0, zero repo-path leaks, no RUNPATH
#   self-containment: ./AppImage --api-only from clean dir + clean HOME serves static from bundle, DB under XDG
```

## Environment notes (local verification)

- `librsvg-2.0.pc` is only needed by the linuxdeploy gtk plugin; shipped via `librsvg2-dev` in CI. On machines without it, a userspace shim (`PKG_CONFIG_PATH = <dir>/librsvg-2.0.pc` with `libdir=/lib/x86_64-linux-gnu`) reproduces the CI copy source exactly.
- The webview pre-tag gate (`scripts/webview-e2e.sh` + `tests/webview_e2e.py`) requires `webkitgtk-webdriver` (WebKitWebDriver), `tauri-driver`, and a desktop session — executed locally on 2026-08-10: **`WEBVIEW E2E: 6/6 passed`** (page loads, focus-search bridge, global-shortcut pipeline, docs bridge, dialog guard, error path). This gate caught the QA-13 UI regression on first run.