# NetRail — Enterprise QA Evaluation 2026-08-09

> **Scope:** dual-stack as-built (HEAD `a12dbed` + uncommitted 2026-08-09 working-tree changes). Rust 1.6.4 production path (`netrail-api`), Python 1.6.4 compatibility path, API contract `1.4`, golden fixture `57 open_url + 22 backend_url`.
> **Method:** as-built baseline → evaluation → findings → score → verdict → delta. No findings were fixed before this baseline was captured; this document is the longitudinal baseline #1 for release-to-release comparison.
> **Question this evaluation answers:** *Is the system consistent, reproducible, operable, and defensible as a product, regardless of which stack executes the implementation?*

---

## 0. Governance model

Two independent results are reported. The weighted score alone **cannot** authorize a release.

```
Weighted Score:       X.XX / 5.00
Enterprise Band:      <band>
Release Gate:         PASS | BLOCKED
```

**Hard-stop rule:** any open **P0** finding forces `Release Gate: BLOCKED` and caps the Enterprise Band at **action required**, regardless of the weighted score. P1 findings are release-blocking unless explicitly waived in the findings register.

### Rubric (per criterion, score 1–5)

| Score | Meaning |
|-------|---------|
| 5 | Implemented, tested, documented, and automated/CI-gated **when applicable** |
| 4 | Implemented, tested, documented; partial automation |
| 3 | Implemented with a verifiable gap |
| 2 | Partial or inconsistent |
| 1 | Absent or not demonstrable |

### Enterprise bands

| Band | Weighted score |
|------|----------------|
| Ship-grade | ≥ 4.50 |
| Good with debt | 3.50 – 4.49 |
| Action required | 2.75 – 3.49 |
| Not enterprise-ready | < 2.75 |

---

## D1 — Architectural consistency & design integrity (weight 15%)

| ID | Question | Verification | Threshold | Sev | Score |
|----|----------|--------------|-----------|-----|-------|
| D1.1 | Does the module map in `docs/ARCHITECTURE.md` match the as-built tree? | Read vs `netrail/` + `src-tauri/src/`; endpoint list vs `server/mod.rs:45–64`, `main.py:260–603` | Every claimed module/route exists | — | 5 |
| D1.2 | Is the dual-stack mirroring module-for-module (config, history, security, backends, server, rate_limit, audit, auth, browsers)? | Paired reads + `parity-api-smoke.sh` | Mirror present; asymmetries documented | P3 | 4 |
| D1.3 | Are API routes, method set, and error shape `{code, detail, status}` identical across stacks? | Route matrix (18 endpoints + `/static`), `docs/API_ERRORS.md`, live smoke | Identical surface, contract `1.4` | — | 5 |
| D1.4 | Is the documented error taxonomy exhaustive against emitted codes? | Emitted-code sweep vs `docs/API_ERRORS.md` (89 lines) | All codes documented, no orphans | P3 | 4 |
| D1.5 | Is ENV-variable parity complete and documented? | ENV diff: 23 shared; `NETRAIL_AUTO_OPEN` (py-only, `main.py:616`), `NETRAIL_STATIC_DIR` (`config.rs:316`), `NETRAIL_LOG_JSON` (`logging.rs:11`), `XDG_DATA_HOME` (py-only, `audit.py:35`) | Asymmetries documented in DISTRIBUTION env table | P3 | 3 |
| D1.6 | Is the version SSOT enforced? | `check-versions.sh` (5 code locations, exit 1 on drift) | All = 1.6.4; CHANGELOG/tag checked too | P3 | 4 |
| D1.7 | Is the SBOM deterministic? | `build.rs` parses Cargo.lock (sorted, deduped, no timestamps); release `cmp` byte-identity | Deterministic, asserted | — | 5 |
| D1.8 | Is the concurrency/state model coherent (WAL, single-instance, EADDRINUSE, SharedStore)? | `history/mod.rs`, `server/mod.rs`, chaos tests | Documented, tested | — | 5 |
| D1.9 | Is the frontend contract lock-stepped (CSP with inline-script hash, static bundle parity)? | `tauri.conf.json:26–27`, `security.rs:458`, `csp_includes_failsafe_script_hash` test | Hash test locks index.html ↔ CSP | — | 4 |

**D1 score: 4.3**

Evidence: 18/18 routes identical; parity smoke green; SBOM unit tests green; version SSOT gate green. Gaps: ENV asymmetries undocumented (D1.5), `check-versions.sh` does not cover CHANGELOG/git tag (D1.6), no automated proof of API_ERRORS.md completeness (D1.4). Known deliberate behavioral deltas (Python `webbrowser.open` fallback vs Rust `BROWSER_NOT_FOUND`; `supports_operators` sorted vs unsorted) are recorded in the findings register.

---

## D2 — Security consistency (weight 20%)

| ID | Question | Verification | Threshold | Sev | Score |
|----|----------|--------------|-----------|-----|-------|
| D2.1 | Is URL-policy behavior identical across stacks (WHATWG mirror)? | Golden fixture live parity: **57/57 open_url** + **18 non-strict backend vectors** (5 strict covered in unit tests); `_parse_whatwg_ipv4` mirror, port netloc checks | All vectors identical codes | — | 5 |
| D2.2 | Is the attack-shape coverage demonstrated and residual documented? | Differential fuzz 7,600 URLs (live Rust vs Python): `py_block_rust_allow` = 0, `code_diff` = 0; residual 50/7,600 `0xzz`-family = DNS stage-ordering (same terminal state) | No fail-open class remains | P2 | 4 |
| D2.3 | Is API auth parity sound (constant-time token, UI token injection, 401 shape)? | `auth.rs`/`auth.py` + tests | Constant-time compare, no default creds | — | 5 |
| D2.4 | Is history encryption consistent (Fernet, keyring fallback, documented plaintext degradation)? | `crypto.rs` (Fernet `gAAAAA` test parity), `history/crypto.py`, SECURITY.md boundaries | Key management and degrade path documented | — | 4 |
| D2.5 | Is CSP hardened with inline-script hash lock-step? | `security.rs:941–944` locks index.html inline script ↔ CSP hash | Test green, `dangerousDisableAssetCspModification` confined to webview | — | 5 |
| D2.6 | Is path traversal prevented in docs assets and static serving? | `docs.rs`/`docs_content.py` traversal rules + tests, `/static` serve | `..`/`/`/`\` blocked, 404 typed | — | 5 |
| D2.7 | Is audit coverage complete (open blocked/success, rotation, `open.blocked`)? | `audit.rs`/`audit.py`, server closure wrap; rotation chaos `cargo test --lib audit::tests` | Blocked opens audited; rotation bounded | P3 | 4 |
| D2.8 | Is rate limiting identical (90/120/60 caps, 429 contract, `NETRAIL_RATE_LIMIT=0` disable)? | `rate_limit.rs`/`rate_limit.py`, api_error_codes tests | Same caps and 429 shape | — | 5 |
| D2.9 | Is read-only mode enforced identically (403 `READONLY_MODE`)? | `readonly_mode.rs` integration + `ensure_mutable` mirror | All mutating routes gated | — | 5 |
| D2.10 | Is the supply chain defensible? | `cargo audit --file Cargo.lock`: **0 vulnerabilities** over 570 crates (19 allowed warnings: atk/glib/gtk3 unmaintained, release pipeline allows); `pip-audit -r requirements.txt`: **0**; npm audit gate; cosign keyless sign+verify pinned to workflow identity | 0 known vulnerabilities; signed artifacts | — | 5 |

**D2 score: 4.7**

Evidence: audits executed in this baseline (see evidence log); fuzz convergence measured in this baseline; parse-level parity incl. port/multi-colon/octal-8-9/0x-empty (previous session's remediation) is live-verified. Residual documented: `0xzz` ± DNS stage ordering; `\\`-network-path fail-closed class; IPv6 reserved-class code differences (both block).

---

## D3 — Robustness & fault tolerance (weight 15%)

| ID | Question | Verification | Threshold | Sev | Score |
|----|----------|--------------|-----------|-----|-------|
| D3.1 | Does the chaos suite exercise DB/process failure without data loss? | `chaos_db.rs` (SQLITE_BUSY, read-only FS, locked-then-released), `chaos_process.rs` (SIGKILL WAL survival, SIGINT graceful), live `harness.sh busy|kill` in CI | All green | — | 5 |
| D3.2 | Are partial-failure semantics correct (partial fanout → 200 + `errors[]`, total → 502 `FANOUT_TOTAL_FAILURE`)? | `backends/mod.rs` fanout + Python mirror + api_error_codes tests | Contract per API_ERRORS.md:59–68 | — | 5 |
| D3.3 | Are fanout deadlines symmetric? | Rust hard-abort 20s vs Python `as_completed(timeout=20)` then blocking `ThreadPoolExecutor.__exit__` (HANDOVER §8b P4) | Same hard-deadline semantics | P2 | 3 |
| D3.4 | Do timeout/fallback paths fail loudly? | `http_client.rs:17` fallback now `tracing::warn!`s dropped protections | Warning names dropped protections | — | 4 |
| D3.5 | Do corrupt settings fall back with a visible WARN in both stacks? | `config.rs`/`config.py` corrupt-settings paths + unit tests | WARN with path/err both stacks | — | 5 |
| D3.6 | Is the test suite deterministic under concurrent full runs? | `cargo test` ×5 full runs: **1/5 failure** of `settings_put_with_fresh_if_match_succeeds_and_returns_new_etag` (0/12 isolated). Suspected shared mutate rate-limit budget under test parallelism | 0 flakes | P2 | 2 |
| D3.7 | Is the differential fuzz reproducible and CI-gated? | Harness lives in `/tmp` (rebuilt during baseline), not committed; not in CI | Committed + CI job | P2 | 2 |
| D3.8 | Do malformed inputs produce the same typed errors in both stacks? | `JsonRejection` from-impl mirror (`server/mod.rs:788–837`) vs `request_validation_handler` (`main.py:92–149`); api_error_codes | Field-for-field code parity | — | 5 |
| D3.9 | Are DB migrations and WAL recovery robust? | Migration tests, SIGKILL survival, WAL journaling | Covered by chaos | — | 5 |
| D3.10 | Is double-instance handling hardened? | Second bind → typed EADDRINUSE path, single-instance plugin | No silent corruption | — | 5 |

**D3 score: 4.1**

Evidence: chaos suites green in this baseline; flake reproduced (QA-02); fuzz-not-committed (QA-03).

---

## D4 — Reliability & operational readiness (weight 8%)

| ID | Question | Verification | Threshold | Sev | Score |
|----|----------|--------------|-----------|-----|-------|
| D4.1 | Is the health endpoint a truthful operational probe? | `/api/health` reports status, version, `api_contract`, `telemetry: none`, rate-limit state, backends | Matches running state | — | 5 |
| D4.2 | Are containers hardened (non-root, localhost bind, HEALTHCHECK, secrets via env guard)? | `Dockerfile`/`Dockerfile.rust` (non-root, curl/urllib healthchecks, 127.0.0.1), `docker-compose.yml` `${NETRAIL_DB_KEY:?}` guard | No root, no LAN exposure, health-gated | — | 5 |
| D4.3 | Is ops tooling documented (systemd unit, `backup-db.sh`, restore)? | DISTRIBUTION.md + script present | Documented procedures | — | 5 |
| D4.4 | Is structured logging parity complete? | `NETRAIL_LOG_JSON` Rust-only (`logging.rs:11`); Python has no JSON-logging twin | Same ops surface both stacks | P3 | 3 |
| D4.5 | Are failure states surfaced to users (encryption degrade, 429/401/502 troubleshooting)? | MANUAL.md troubleshooting, degrade banner | Documented + visible | — | 4 |

**D4 score: 4.4**

---

## D5 — Test quality & coverage (weight 12%)

| ID | Question | Verification | Threshold | Sev | Score |
|----|----------|--------------|-----------|-----|-------|
| D5.1 | Does the Python suite cover every module with behavioral tests? | 199 passed (11 files: api 31, security 29+, url_policy golden, history 10, backends/merge/chaos/config/docs/css/flatpak) | Green, module-scoped | — | 5 |
| D5.2 | Does the Rust suite cover lib + integration + chaos + release invariants? | 91 lib + 20 api_error_codes + 6 chaos_db + 2 readonly + 2 sbom + 2 extra + 1 chaos_process ≈ 126; SBOM/readonly integration targets | Green | — | 5 |
| D5.3 | Are cross-stack golden fixtures shared? | 79 vectors in `tests/fixtures/url_policy.json` (57 open + 22 backend) + live parity both stacks | Same fixture, live-verified | — | 5 |
| D5.4 | Is chaos in CI for both stacks? | `ci.yml` chaos job (Rust chaos_db/chaos_process/audit rotation + Python test_chaos + live harness) | CI-gated | — | 5 |
| D5.5 | Is the full-suite run deterministic? | Flake D3.6 evidence (1/5 full `cargo test`) | 0 flakes | P2 | 3 |
| D5.6 | Is coverage % measured and reported? | No coverage step in any workflow; no `--coverage`/coveralls/kcov anywhere | Reported per merge | P3 | 1 |
| D5.7 | Is the webview E2E gated? | `webview-e2e.sh` exists (tauri-driver + selenium), display-dependent, **not in CI** | CI-gated or marked manual | P3 | 3 |
| D5.8 | Is the differential fuzz repeatable? | Harness reconstructed in `/tmp` for this baseline (previous run lost to tmp cleanup); not committed | Reproducible in CI | P2 | 2 |

**D5 score: 3.6**

---

## D6 — Frontend & UX consistency (weight 6%)

| ID | Question | Verification | Threshold | Sev | Score |
|----|----------|--------------|-----------|-----|-------|
| D6.1 | Are structural CSS contracts regression-guarded? | `test_ui_css.py` (`.result-card` grid guard) | Guard green | — | 4 |
| D6.2 | Are desktop↔webview bridges stable? | `desktop.rs` eval bridges (`netrailFocusSearch`, `netrailOpenDoc`, `netrailDonate`), single-instance, global shortcut | No webview E2E in CI (D5.7) | P3 | 4 |
| D6.3 | Does the served UI honor CSP? | CSP headers + inline-script hash lock-step (D2.5) | Verified | — | 5 |
| D6.4 | Are accessibility basics present and tested? | AUDIT_ARCH A-level a11y improvements; no automated a11y checks | Automated checks | P3 | 3 |
| D6.5 | Is the keyboard workflow documented and implemented? | MANUAL keyboard tips, focus-search bridge | Documented | — | 4 |

**D6 score: 4.0**

---

## D7 — Documentation & claims integrity (weight 8%)

| ID | Question | Verification | Threshold | Sev | Score |
|----|----------|--------------|-----------|-----|-------|
| D7.1 | Do prose docs carry the current version? | **Drift found (QA-06):** `ARCHITECTURE.md:3` (1.6.2), `:259` (v1.6.3); `DISTRIBUTION.md:15` ("parity matrix (1.2.2)"); `MANUAL.md:29` (`NetRail_1.4.0_amd64.AppImage`); `SECURITY.md:5–10` supported-versions omits 1.3.x–1.6.x; `HANDOVER.md:158` ("v1.2.2 is Latest") + footer (1.4.0) | Zero stale version labels | P3 | 2 |
| D7.2 | Do README/SECURITY/MANUAL claims verify against code? | Claim-to-code spot-check (127.0.0.1:7421, telemetry none, rate caps 90/120/60, token headers, READONLY set) | All verified true | — | 4 |
| D7.3 | Is cross-doc link integrity checked? | No link checker exists (`test_docs.py` only tests API exposure) | Automated link check | P3 | 2 |
| D7.4 | Is API_ERRORS.md completeness proven? | Manual only (D1.4) | Generated/checked coverage | P3 | 3 |
| D7.5 | Is CHANGELOG aligned with tags? | v1.6.4 entry ↔ tag v1.6.4 ↔ HEAD aligned; historical gaps: v1.2.3/v1.3.x present in CHANGELOG, absent from tag list | Aligned at HEAD | — | 4 |
| D7.6 | Is the findings ledger maintained honestly? | HANDOVER §8/§8b open items explicit (P3 browser parity, P4 fanout) | Open ≠ resolved; no silent closure | — | 4 |

**D7 score: 3.0**

Note: the five drifts above were **discovered by this evaluation** (baseline order: as-built → evaluate). They are intentionally *not fixed yet* — they are proof the protocol detects un-catalogued inconsistency, and they seed the findings register (QA-06).

---

## D8 — Release engineering & supply chain (weight 8%)

| ID | Question | Verification | Threshold | Sev | Score |
|----|----------|--------------|-----------|-----|-------|
| D8.1 | Do CI gates protect quality (version drift, clippy `-D warnings`, tests, audits)? | `ci.yml` jobs; `release.yml:47–53` | **Gate present but red in as-built: `cargo clippy --all-targets -- -D warnings` fails on unused import `std::io::Read` (`server/mod.rs:950`)** — QA-01 (P0) | Gate green | **P0** | 3 |
| D8.2 | Are release artifacts asserted (AppImage/deb mandatory, executability)? | `release.yml:80–93` hard-fail checks | Missing artifact = job fail | — | 5 |
| D8.3 | Is the supply chain attested (SBOM byte-identical `cmp`, SHA256SUMS, cosign keyless)? | `release.yml:96–134`; sbom in deb/rpm verified via `dpkg-deb -c`/`rpm -qlp` | Byte-identical + signed | — | 5 |
| D8.4 | Is the release pipeline healthy end-to-end? | **NR-16 open:** v1.6.4 assets unpublished — release CI failed at AppImage/linuxdeploy (HANDOVER §11) | v1.6.4 published + Latest | **P1** | 2 |
| D8.5 | Are platform/coverage/docker gaps conscious? | Linux-only CI, no Windows/macOS, no coverage job, no Docker build/push in CI | Documented decision or gap closed | P3 | 3 |
| D8.6 | Is build reproducibility enforced? | `SOURCE_DATE_EPOCH`/git-derived SBOM timestamp, deterministic inventory | Rebuild → identical artifact | — | 5 |

**D8 score: 3.8**

---

## D9 — Performance & resource stability (weight 4%)

| ID | Question | Verification | Threshold | Sev | Score |
|----|----------|--------------|-----------|-----|-------|
| D9.1 | Are steady-state and saturation characterized per stack? | `docs/bench-dual.md` (3 steady-state runs + knee; **no knee ≤ 512 concurrent, <1% errors**) | Evidence on record | — | 5 |
| D9.2 | Is load stability statistically demonstrated? | `docs/sprint3-slope.md` (10k/1k, slope p < 0.05, RSS/fd/socket verdicts stable) | Verdicts computed and green | — | 5 |
| D9.3 | Is the settings concurrency contract sound? | ETag/If-Match → 409 → fresh PUT 200 (live smoke) | Verified both stacks | — | 5 |
| D9.4 | Is encrypted-history overhead characterized? | Not benchmarked in bench-dual/slope | Benchmark row | P3 | 3 |

**D9 score: 4.5**

---

## D10 — Process, maintainability & tech debt (weight 4%)

| ID | Question | Verification | Threshold | Sev | Score |
|----|----------|--------------|-----------|-----|-------|
| D10.1 | Is the findings ledger kept honest across releases? | HANDOVER §8/§8b explicit open items; this eval extends it | Open items tracked with evidence | — | 4 |
| D10.2 | Is the intentional dual-stack duplication a documented policy? | HANDOVER R3 (policy: Rust production, Python compatibility) | Policy documented | — | 4 |
| D10.3 | Is production code free of TODO/FIXME/HACK debt markers? | `rg -c "TODO|FIXME|HACK|XXX" netrail/*.py src-tauri/src/*.rs` → **0 matches** | Zero | — | 5 |
| D10.4 | Are legacy/superseded paths documented as such? | packaging/README notes PyInstaller path is not the release path | Documented | — | 4 |
| D10.5 | Is commit hygiene consistent? | Conventional commits (`feat/fix/chore/docs/test/release`) across git log | Consistent | — | 4 |

**D10 score: 4.2**

---

## 11. Executive scorecard

| Domain | Weight | Score | Weighted |
|--------|--------|-------|----------|
| D1 Architectural consistency | 15% | 4.3 | 0.645 |
| D2 Security consistency | 20% | 4.7 | 0.940 |
| D3 Robustness & fault tolerance | 15% | 4.1 | 0.615 |
| D4 Reliability & operations | 8% | 4.4 | 0.352 |
| D5 Test quality & coverage | 12% | 3.6 | 0.432 |
| D6 Frontend & UX | 6% | 4.0 | 0.240 |
| D7 Documentation integrity | 8% | 3.0 | 0.240 |
| D8 Release & supply chain | 8% | 3.8 | 0.304 |
| D9 Performance & stability | 4% | 4.5 | 0.180 |
| D10 Process & debt | 4% | 4.2 | 0.168 |
| **Total** | 100% | | **4.12** |

## 12. Verdict

```
Weighted Score:       4.12 / 5.00
Enterprise Band:      Good with debt (3.50 – 4.49)
Release Gate:         BLOCKED — P0 QA-01 (clippy -D warnings red in as-built)
                      + P1 NR-16 (v1.6.4 release assets unpublished)
```

Hard-stop application: despite a **ship-adjacent weighted average**, the presence of an open **P0** (QA-01) and the open **NR-16** release failure cap the verdict at *action required*. Per the governance model in §0, **no release may be authorized from this baseline**.

The score that would hide QA-01 is exactly what the hard-stop rule exists to prevent. A 4.12 with a red CI gate is not a release authorization; it is the to-do list below.

*Post-baseline (2026-08-10): this gate was subsequently **UNBLOCKED** — QA-01, QA-02, QA-03, NR-16 and §8b P2 all closed with evidence; v1.6.4 published (8 assets, cosign-signed). Baseline #1 photo and score unchanged; closure record in §17, new score computed at Baseline #2.*

## 13. Findings register — QA-2026-08-09

| ID | Sev | Domain | Title | Evidence | Fix direction |
|----|-----|--------|-------|----------|---------------|
| QA-01 | **P0** | D8 | clippy `-D warnings` gate red in as-built | `error: unused import: std::io::Read` `server/mod.rs:950` | **Closed by A2** (2026-08-09): line removed; `cargo clippy --all-targets -- -D warnings` exit 0 |
| QA-02 | P2 | D3/D5 | Flaky integration test under full-suite concurrency | `settings_put_with_fresh_if_match_succeeds_and_returns_new_etag`: 1/5 full `cargo test` runs; shared mutate rate-limit budget suspected | **Closed by A2** (2026-08-09): rate-limit state isolated; 8/8 full-suite runs clean (was 1/5) |
| QA-03 | P2 | D5 | Differential fuzz harness not committed nor CI-gated | **Closed by B-wave** (2026-08-09): harness + seed committed (`scripts/fuzz-parity.py`, `1e2d36e`); parity `code_diff=0` on 7 600 URLs; residual pinned to 1 family (50 `0xzz`); §16 numbers + runbook. **T1 (2026-08-10):** CI-gated in ci.yml `test` job — `--corpus-only` + `--ci` (code_diff==0, all divergences in known family, residual pinned at 50; drift = contract decision, not silent) | — |
| QA-04 | P3 | D5 | No coverage % measurement anywhere | No coverage step in ci.yml/release.yml; no kcov/coveralls/`pytest --cov` config | Add coverage job (rust: llvm-cov/tarpaulin, py: pytest-cov) with branch gate |
| QA-05 | P3 | D5/D6 | webview E2E not in CI | `webview-e2e.sh` + `tests/webview_e2e.py` exist; display-dependent; ci.yml omits it | Document as manual release gate, or CI with xvfb |
| QA-06 | P3 | D7 | Five prose-version drifts (discovered by this eval) | `ARCHITECTURE.md:3` (1.6.2), `:259` (v1.6.3); `DISTRIBUTION.md:15` ("(1.2.2)"); `MANUAL.md:29` (`1.4.0` AppImage); `SECURITY.md:5–10` (1.3.x–1.6.x missing from supported versions); `HANDOVER.md:158` ("v1.2.2 is Latest") + footer | Refresh to 1.6.4 SSOT; extend `check-versions.sh` to prose spot-lists |
| QA-07 | P3 | D1 | ENV asymmetries undocumented (4 vars) | `NETRAIL_AUTO_OPEN` py-only (`main.py:616`), `NETRAIL_STATIC_DIR`/`NETRAIL_LOG_JSON` rust-only (`config.rs:316`, `logging.rs:11`), `XDG_DATA_HOME` py-only (`audit.py:35`); DISTRIBUTION env table is non-stack-aware | Annotate stack ownership in DISTRIBUTION |
| QA-08 | P3 | D4 | JSON log parity gap | `NETRAIL_LOG_JSON` has no Python twin | Python JSON logging handler or explicit non-goal note |
| QA-09 | P2 | D2/D8 | Carried: browser-discovery parity divergence (HANDOVER §8b P3, open) | Rust 7 known browsers vs Python 13; Python defaults unknown stems to `--incognito` (`browsers.py:136`) vs Rust no flag (`browsers.rs:128`); `.desktop` `Name=` parse order differs | Align known_browsers + flag default + Name= semantics |
| QA-10 | P2 | D3 | Carried: fanout deadline asymmetry (HANDOVER §8b P4, open, "trivial") | Rust hard-abort 20s vs Python `as_completed(20)` + blocking `ThreadPoolExecutor.__exit__` | Replace executor shutdown with explicit `cancel` + bounded wait |
| QA-11 | P3 | D1 | `check-versions.sh` does not cover CHANGELOG/tag | Script compares 5 code locations only | Add `git tag` + CHANGELOG latest-entry check |
| QA-12 | P3 | D7 | No link-integrity checker for cross-doc links | `tests/test_docs.py` covers API exposure only; ~40 markdown files in repo | Add link checker test |

## 14. Delta vs HANDOVER §7/§8b

| Input | This baseline | Sign |
|-------|---------------|------|
| §8b P2 "Error-code divergence on malformed URLs" | **Closed** (fixed 2026-08-09; fuzz code_diff = 0) | ➕ |
| §8b P3 "Browser-discovery parity" | Open, carried as QA-09 | ➖ |
| §8b P4 "Fanout deadline asymmetry" | Open, carried as QA-10 (trivial) | ➖ |
| NR-11/NR-16 release assets | **Closed & published 2026-08-10** — NR-16 root-caused (`ed0603d`): desktopTemplate empty Categories value + linuxdeploy strip/`.relr.dyn` (tauri#14796/#8929/#13113) → `NO_STRIP`; Release CI `31350607763` green; 8 assets live, cosign-signed | ➕ |
| §7 R4 "v1.2.2 is Latest" | Row itself now stale (doc drift, QA-06) | ➖ |
| New, not previously catalogued | QA-01 (clippy red), QA-02 (flake), QA-03 (fuzz not committed), QA-04 (no coverage), QA-06 (doc drift ×5), QA-07/08/11/12 | ➖ (process/documentation consistency) |

**Net delta: negative on process/documentation consistency, positive on security parity.** The evaluation discovered more inconsistency than it inherited — which is the correct property for a QA instrument (§0 baseline order).

## 15. Reproducible execution protocol (enterprise release gate)

Run in this exact order on a clean checkout + release build. Expected results are pinned to this baseline.

```bash
# 1. Version SSOT
bash scripts/check-versions.sh                      # exit 0, all 1.6.4
# 2. Python suite
.venv/bin/python -m pytest tests/ -q                # 199 passed
# 3. Rust suite + clippy (gate!)
cargo test                                          # ≈126 passed, 0 flakes expected
cargo clippy --all-targets -- -D warnings           # exit 0 (closed by A2, 2026-08-09)
# 4. Live contract parity (Rust binary required)
NETRAIL_NO_OPEN=1 bash scripts/parity-api-smoke.sh  # 57/57 open + 18 backend + ETag
bash scripts/e2e-api-smoke.sh                       # health/static/QUERY_INVALID/open-blocks
# 5. Supply chain
cargo audit --file Cargo.lock                       # 0 vulnerabilities (570 crates)
.venv/bin/pip-audit -r requirements.txt             # 0 vulnerabilities
npm audit --audit-level=high                        # CI gate
# 6. Performance evidence (release)
bash scripts/bench-dual.sh                          # steady-state + knee → docs/bench-dual.md
bash scripts/load-10k.sh rust                       # 100% completeness + slope verdicts
# 7. Chaos (CI)
cargo test --test chaos_db --test chaos_process     # + live harness (ci.yml)
# 8. Evaluation refresh
#    re-run scorecard; close findings; record delta in a new QA_EVALUATION_*.md
```

## 16. B-wave closure — differential open-URL fuzz (QA-03), 2026-08-09

Harness committed: `scripts/fuzz-parity.py` (commit `1e2d36e`), corpus seed `20260809`,
reproducible, `--corpus-only` + full parity modes.

**B1 — seeded corpus exploration (python validator):** 7 600 URLs (5 schemes × 62 hosts ×
24 port-tails × path tails). Allowed 1 550 (public IP literals, DNS-hosts) · blocked 6 050
distributed across all six documented codes:
`OPEN_URL_INVALID` 4 700 · `OPEN_URL_LOCALHOST` 550 · `OPEN_URL_PRIVATE` 500 ·
`OPEN_URL_DNS_REBINDING` 200 · `OPEN_URL_CLOUD_METADATA` 50 · `OPEN_URL_LINK_LOCAL` 50.
= 60.9% of blocked URLs carry a typed policy code, not a generic parse error.

**B2 — differential parity vs live Rust binary (release `netrail-api`, dry-open):**
`code_diff = 0` across all 7 600 URLs (blocked codes identical). Fail-open divergence
`py_allow_rust_block = 50`, 100% of them the single-label `0xzz` DNS-stage family
(established residual: Python fails DNS inside open's resolver; terminal behavior
identical — no practical fail-open). The `*x` and numeric-last-label families from the
QA-03 baseline **no longer diverge** (A2 dual-stack hardening closed them):
residual families 3 → 1, divergence 74 → 50.

Runbook: `python3 scripts/fuzz-parity.py --corpus-only && python3 scripts/fuzz-parity.py`
CI gate (T1): `python3 scripts/fuzz-parity.py --ci --binary <netrail-api>` — spawns the
binary with isolated state; refuses any divergence outside the `0xzz` family and any
residual count drift from the pinned 50.

## 17. Remediation closure record — 2026-08-10 (post-baseline)

Baseline #1 gate was **BLOCKED** (P0 QA-01 + P1 NR-16). End-of-cycle state:

| Finding | Status | Evidence |
|---------|--------|----------|
| QA-01 clippy `-D warnings` P0 | **FIXED** | line removed; `cargo clippy --all-targets -- -D warnings` exit 0 |
| QA-02 flaky integration test | **FIXED** | rate-limit state isolated; 8/8 full-suite runs (was 1/5) |
| NR-16 Release AppImage P1 | **FIXED** | root cause chain: observation (`30771210870`) → local reproduction → desktopTemplate Categories empty-value + strip/`.relr.dyn` → `ed0603d` → local AppImage → CI main green (`31350151123`) → Release CI green (`31350607763`) → v1.6.4 published, 8 assets, cosign keyless |
| QA-03 fuzz harness not committed | **FIXED** | `scripts/fuzz-parity.py` + seed committed; `code_diff=0` on 7 600 URLs; residual pinned to 1 family (§16); CI gate step (`--ci --binary`, isolated spawn) green on the gate commit — run `31353537400` (success) |
| §8b P2 error-code divergence | **FIXED** | fuzz `code_diff=0`; live Rust probes 57/57 + 18/18 |
| **QA-09 browser-discovery parity** | **FIXED** | 

`7e3a0f7` (T2): `tests/fixtures/browsers.json` canonical SSOT (13 known browsers, unknown flag null); Rust: known-browser table → `Vec` of 13 + section-scoped `.desktop` parse (kills `[Desktop Action …]` leakage: pre-fix live value "New Incognito Window" for `brave-browser-stable`) + `host_which` basename; Python: unknown default `--incognito` → `None`, `desktop_dirs()` per-call (was frozen at import), Exec resolution token-first; fixture equality asserted by both test suites; live `/api/browsers` parity probe in `parity-api-smoke.sh` green on dev box (4 browsers, 4 fields, deterministic sort). Evidence: CI run `31369955603` (success) — cargo 99/99 incl. 10 browsers tests, clippy `-D warnings` clean, pytest 202/202 incl. 3 parity tests, fuzz gate `code_diff=0` residual 50; local smoke parity OK. Closure discipline held: FIXED only after observable CI behavior |

| **QA-10 fanout deadline** | **FIXED** | Python `backends/registry.py`: explicit pool — on `FuturesTimeout` cancel queued futures + `shutdown(wait=False, cancel_futures=True)` (the old `with`-block `__exit__` blocked on hung threads, stretching wall time past the 20s deadline; running threads now finish under their own HTTP timeouts: brave/searxng 12s, wikipedia 15s, ddgs ~5s per engine). Rust `backends/mod.rs`: replaced `tokio::time::timeout` discard-all with `JoinSet` + `select!` on the shared deadline → completed tasks keep partial results, remainder aborted (partial → 200 + errors[]; all-empty + errors → 502 `FANOUT_TOTAL_FAILURE` both stacks). 5-property contract now holds on both sides: deadline ≤20s, cancelación, bounded wait, partial → 200+errors[], total → 502. Tests: `tests/test_fanout_deadline.py` 4/4 (partial kept + timed-out error; 502 total failure; hung backend does not stretch wall time; fast path joins). Evidence: CI run `31375935523` (success) — pytest 206/206, cargo 99/99, clippy `-D warnings` clean |
| **QA-12 link-integrity checker** | **FIXED** | `tests/test_links.py` walks README/CHANGELOG/SECURITY + `docs/*.md` (40 files), resolving relative file targets and GitHub-style anchors; title-syntax links `(url "title")` supported. Shakedown of the checker surfaced two of its own defects (stale report variable; non-GitHub anchor slugging of `1.6.4`-style headers) — both fixed — and caught one genuine cross-doc rot: ARCHITECTURE's §9 anchor pointed at a HANDOFF header since renamed ("post-1.6.4 push"); repaired. Initial "missing file" reports for `RELEASE_v1.3.0/1.4.0.md`, `AUDIT_ENTERPRISE`, `API_ERRORS` were checker false positives (title syntax), not real rot — the repaired checker resolves them. Full corrected run: 1 passed across all 40 docs (local); CI-gated automatically via the pytest step |
| QA-06 prose-version drifts | **FIXED** | `fa102e8`: ARCHITECTURE current-state → 1.6.4 release line, DISTRIBUTION parity matrix (1.6.4), MANUAL AppImage 1.6.4, SECURITY supported-versions adds 1.3.x–1.6.x, HANDOVER R4 + footer; `check-versions.sh` prose spot-lists |
| QA-11 check-versions scope | **FIXED** | `fa102e8`: script now checks CHANGELOG `## [1.6.4]` top entry (format-matched) + advisory HEAD-tag note; exit 0 on main |
| QA-07 ENV asymmetries | **FIXED** | `8ea44dd`: DISTRIBUTION env table gained a Stack column (Rust/Python/Both) — `NETRAIL_AUTO_OPEN` Python, `NETRAIL_STATIC_DIR`/`NETRAIL_LOG_JSON` Rust, `XDG_DATA_HOME` Python row added |
| QA-08 JSON log parity | **FIXED** | `8ea44dd`: explicit non-goal note in DISTRIBUTION (Python structured log parity is a deliberate non-goal; operational parity surface is the shared audit NDJSON schema) |
| QA-04 coverage observability (T5) | **FIXED** (instrumentation; gate deferred) | CI now reports coverage without enforcing a threshold — the gate decision is explicitly deferred to Baseline #2 (T5 discipline: observability first, no threshold racing, no artificial tests). **Python:** `python -m pytest --cov=netrail --cov-report=term --cov-config=.coveragerc` (pytest 9.1.1, coverage.py 7.15.4; exclusions explicit in `.coveragerc`, only the `__main__` entrypoint shim; branch accounting). CI `abe2bd6`/`31410699870`: 77% combined / 80% statement. **Rust:** `cargo llvm-cov --no-default-features --lib --summary-only` via taiki-e install-action (cargo-llvm-cov 0.8.7, rustc 1.90.0). CI: 57.50% lines (lib scope, unit tests). Stack strategies deliberately differ (`--lib` measures unit surface; server/integration surface is exercised by the other CI targets) — that difference is part of the dual-stack reality and is not hidden. Per-module tables print in CI logs on every run |
| **QA-05 webview E2E** | **FIXED** (policy: manual release gate) | Decision and rationale documented in `docs/RELEASE_ASSURANCE.md` ("Webview E2E — gate manual de release"): webview validation happens as a manual gate before tagging (6/6 checks incl. focus-search bridge, global-shortcut, docs bridge, modal guard), with exact prerequisites, commands and evidence-to-record; CI covers the API-level surface of the same bridges (`/api/docs`, parity smoke). xvfb CI explicitly rejected as policy: display/shortcut focus is fragile under xvfb and would make the gate intermittently blocking. `RELEASE_ASSURANCE.md` header/counts refreshed to v1.6.4 (207/130) |

Release gate at end of cycle: **UNBLOCKED**. The 4.12 weighted score remains the
historical photo of Baseline #1; the recompute with the same instrument is §18 below.
Sequence preserved: Baseline #1 → remediation → 1.6.4 released → Baseline #2.

**Release tag traceability.** `v1.6.4` permanently points at the release snapshot
commit `a32df00` (the commit the Release CI `31350607763` validated). The
post-release documentation refresh R5b (`20b47be`) landed on `main` **after** the
release and was deliberately **not** re-tagged (moving the tag would re-open the
release state and re-introduce the uncertainty just removed). R5b therefore belongs
to the post-release line of history, not to the published snapshot — so a future
reader who asks "why is NR-16=FIXED not in the v1.6.4 tree?" finds the answer here:
the closure evidence is the CI runs and the published assets, both recorded against
`a32df00`/`31350607763`.

## 18. Baseline #2 — recompute (2026-08-10)

Same instrument (§1–§10): same 10 dimensions, same weights, same score rubric, same
stopping rule. Recomputed against the as-built tree after all §17/T1–T5 closures.
Only criteria whose score changed are listed; everything else retains its Baseline #1
score. Verification re-run during this recompute: `cargo clippy --all-targets -- -D warnings`
exit 0 (the ex-P0 gate), `pytest tests/` 207 passed (was 199), `pytest tests/test_links.py`
1 passed, debt-marker sweep 0 hits.

| ID | Δ | Justification (evidence in §13/§16/§17) |
|----|----|------------------------------------------|
| D1.5 | 3→4 | ENV asymmetries now documented with stack ownership (QA-07, `8ea44dd`) |
| D1.6 | 4→5 | `check-versions.sh` now covers CHANGELOG top entry + HEAD-tag advisory (QA-11, `fa102e8`) |
| D2.2 | 4→5 | Attack-shape evidence now committed, reproducible and CI-gated (QA-03 T1): `code_diff`=0 on 7 600 URLs, residual pinned to one known family, CI refuses divergence — no fail-open class remains, and it is no longer reconstructable-only |
| D3.3 | 3→5 | Fanout deadline symmetric (QA-10, CI `31375935523`): Rust `JoinSet`+`select!` vs Python explicit cancel + bounded wait; 5-property contract holds both stacks; `tests/test_fanout_deadline.py` 4/4 |
| D3.6 | 2→5 | Full-suite deterministic (QA-02): 8/8 clean runs (was 1/5) |
| D3.7 | 2→5 | Fuzz reproducible + CI-gated (QA-03 T1): committed harness, `--ci --binary` gate green on the gate commit |
| D4.4 | 3→4 | JSON-log parity gap closed as an explicit documented non-goal (QA-08, `8ea44dd`): shared ops surface is the audit NDJSON schema; decision recorded, twin deliberately absent |
| D5.5 | 3→5 | Deterministic full runs (QA-02, evidence as D3.6) |
| D5.6 | 1→4 | Coverage measured and reported per merge (T5, CI `31410699870`): Python 77% (80% statement), Rust 57.50% lib; threshold gate deferred per T5 discipline, not raced — reported, not enforced |
| D5.7 | 3→4 | Webview E2E policy recorded (QA-05): manual release gate in `RELEASE_ASSURANCE.md` (6/6 checks, prerequisites + evidence-to-record); xvfb CI rejected as policy |
| D5.8 | 2→5 | Fuzz repeatable (QA-03 T1): same evidence as D3.7 |
| D7.1 | 2→5 | Five prose-version drifts closed to 1.6.4 SSOT (QA-06, `fa102e8`) + `check-versions.sh` prose spot-lists |
| D7.3 | 2→5 | Cross-doc link integrity automated (QA-12, `tests/test_links.py`): 40 docs, relative targets + GitHub anchors, CI-gated via pytest; two checker defects and one genuine cross-doc rot found and closed during shakedown |
| D7.5 | 4→5 | CHANGELOG↔tag alignment now checked by the version gate (QA-11) |
| D8.1 | 3→5 | Gates green in as-built (QA-01 closed): clippy `-D warnings` exit 0, re-verified in this recompute |
| D8.4 | 2→5 | Release pipeline healthy end-to-end (NR-16 closed): v1.6.4 published, 8 assets, cosign keyless, Release CI `31350607763` green |
| D8.5 | 3→4 | Coverage job added to CI (T5); remaining gaps (Docker build/push, Windows/macOS) are documented decisions |

Unchanged by evidence, so honestly passthrough: D1 4.3→**4.6**, D2 4.7→**4.8**, D3 4.1→**4.9**, D4 4.4→**4.6**, D5 3.6→**4.8**, D6 4.0→**4.0**, D7 3.0→**4.3**, D8 3.8→**4.8**, D9 4.5→**4.5**, D10 4.2→**4.2**.

### 18.1 Executive scorecard — Baseline #2

| Domain | Weight | Baseline #1 | Baseline #2 | Weighted |
|--------|--------|-------------|-------------|----------|
| D1 Architectural consistency | 15% | 4.3 | 4.6 | 0.690 |
| D2 Security consistency | 20% | 4.7 | 4.8 | 0.960 |
| D3 Robustness & fault tolerance | 15% | 4.1 | 4.9 | 0.735 |
| D4 Reliability & operations | 8% | 4.4 | 4.6 | 0.368 |
| D5 Test quality & coverage | 12% | 3.6 | 4.8 | 0.576 |
| D6 Frontend & UX | 6% | 4.0 | 4.0 | 0.240 |
| D7 Documentation integrity | 8% | 3.0 | 4.3 | 0.344 |
| D8 Release & supply chain | 8% | 3.8 | 4.8 | 0.384 |
| D9 Performance & stability | 4% | 4.5 | 4.5 | 0.180 |
| D10 Process & debt | 4% | 4.2 | 4.2 | 0.168 |
| **Total** | 100% | **4.12** | **4.65** | |

### 18.2 Verdict — Baseline #2

```
Weighted Score:       4.65 / 5.00   (Baseline #1: 4.12, Δ +0.53)
Enterprise Band:      Ship-grade (≥ 4.50)
Release Gate:         UNBLOCKED — no open P0/P1 on the register
```

Hard-stop re-check at recompute: the single P0 (QA-01, clippy) and the P1 (NR-16,
release assets) are both closed with CI-recorded evidence, re-verified as-built during
this recompute. No new P0/P1 surfaced. The gate that blocked Baseline #1 is green.

### 18.3 What keeps it at 4.65 and not higher (honest residue)

- D1.4/D7.4: API_ERRORS.md completeness remains manual-only — no emitted-code sweep
  is automated (open P3).
- D6.4: no automated accessibility checks; A-level a11y improvements human-verified
  only (open P3).
- D4.4: JSON-log twin is a documented non-goal, not an implementation — 4 by
  decision-recording, not by parity.
- D9.4: encrypted-history overhead still not benchmarked (open P3).
- D5.6: coverage is reported per merge but the threshold gate is deferred by policy
  (T5 discipline: observability before enforcement).
- D7.6/D10 ledger: the register itself is carried honestly, including these open P3s.

Next baseline (#3) must recompute with this same instrument and delta against 4.65,
including closing or consciously re-dating the P3 residue above.

### 18.4 Phase transition — release-readiness mode (product-lead decision, 2026-08-10)

**NetRail exits remediation phase and enters release-candidate mode. Baseline #2 is
the release-readiness baseline for 1.6.5.**

No T8 hardening wave: the +0.53 delta came from evidence, not from instrument
manipulation (D6/D9/D10 with no new evidence did not move). The next phase is
deliberately boring: **freeze → clean verification → build → inspect → RC → smoke →
release.**

**R1 — Scope freeze (from this point):**

- no large refactors, no architectural changes;
- no new P2/P3 finding is automatically converted into work;
- no API contract changes except critical defects;
- no reopening of closed findings without new evidence;
- permitted work is limited to release mechanics, version/documentation
  consistency, and the R2/R3 gates below.

**Residue classification (decision on §18.3):**

| Residue | Decision |
|---------|----------|
| API_ERRORS completeness manual-only (D1.4/D7.4) | **Accepted debt** |
| a11y not automated (D6.4) | **Accepted debt** |
| JSON log parity twin absent (D4.4) | **Non-goal** (recorded decision) |
| encrypted-history overhead unbenchmarked (D9.4) | **Measurement debt** — schedule with next performance pass |
| coverage threshold gate (D5.6) | **Policy decision pending** — revisit at Baseline #3 |

Ship-grade does not mean zero debt; it means the remaining debt is understood,
located, and does not compromise the release criterion.

**Tag and release policy:**

- `v1.6.4 → a32df00` is immutable history: it will not be moved, amended or
  re-pointed under any circumstance (§17 tag traceability).
- 1.6.5 gets its own snapshot: version SSOT bump → RC → release. The post-1.6.4
  line of work (T1–T5, §17 remediations, Baseline #2) belongs to 1.6.5, not to
  v1.6.4.
- Pre-tag human gate (QA-05 policy, `docs/RELEASE_ASSURANCE.md`): the webview E2E
  manual gate (6/6 checks) re-executes before tagging. It is display-dependent and
  not executable in this session — it remains on the human release checklist.

```
1.6.4 (a32df00)
  └── post-release remediation T1–T5 + §17 closures
        └── Baseline #2 = 4.65 SHIP-GRADE (release-readiness gate)
              └── R2 clean-checkout verification → RC → 1.6.5
```

### 18.5 R2 record — final verification from clean checkout (2026-08-10)

Per §15 protocol, executed from a **fresh local clone** (`git clone` → HEAD `135ba17`)
in `/tmp/opencode/netrail-rc`, own venv (Python 3.13.3), own `CARGO_TARGET_DIR`
(rustc 1.90.0). Not the dev session: every dependency resolved from scratch.

| Step | Result |
|------|--------|
| `check-versions.sh` | exit 0, 5/5 locations = 1.6.4 (HEAD-tag advisory note, non-release commit as expected) |
| `pytest tests/` | **207 passed** — reproduces recorded count |
| `cargo test` | **130 passed, 0 failed** — reproduces recorded count |
| `cargo clippy --all-targets -- -D warnings` | exit 0 (the ex-P0 gate) |
| `parity-api-smoke.sh <debug netrail-api>` | OK — golden probes + ETag/If-Match + browser parity (4 browsers) |
| `fuzz-parity.py --ci --binary` | GATE OK — code_diff=0, residual pinned at 50 (known `0xzz` family) |
| `e2e-api-smoke.sh` | OK — health/static/typed errors/open-blocks |
| `cargo audit --file Cargo.lock` | exit 0, 0 vulnerabilities + 19 allowed warnings |
| `pip-audit -r requirements.txt` | 0 known vulnerabilities |
| `npm audit --audit-level=high` | 0 vulnerabilities |
| `pytest --cov=netrail` | **77%** total (80% stmts) — reproduces CI figure |
| `test_links.py` + `test_docs.py` | 1 + 3 passed (40-doc link integrity) |
| debt-marker sweep | 0 hits |
| `cargo build --release --bin netrail-api` | Finished, clean (9m 51s from zero) |

Equivalence check: every number recorded in the remediation cycle (§13/§17,
RELEASE_ASSURANCE.md: pytest 207, cargo 130, coverage 77%, clippy 0) was
independently reproduced from a clean tree — the evaluated state is real.

Honest scope note — what R2 did **not** do here, by design:
- webview E2E manual gate (QA-05 policy): display-dependent, human action pre-tag;
- Rust coverage run + AppImage/.deb/.rpm + SBOM attestation + cosign:
  CI-owned (release.yml), pinned in history against future release runs;
- R2's purpose is reproducibility of the **evaluated state**, not discovery —
  no new findings were burnt, per R1.

Outcome: **clean-tree state = evaluated state. Ready for R3 (RC).**

## 19. Longitudinal note

This document is baseline #1. Each release should answer not just "did tests pass?" but
"is NetRail more or less enterprise-ready than the previous baseline?" — same instrument,
same protocol, delta required, negative deltas recorded honestly.