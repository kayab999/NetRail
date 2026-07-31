# NetRail — Enterprise Codebase & Adversarial Q&A Audit

| Field | Value |
|-------|--------|
| **Date** | 2026-07-31 |
| **Tree** | 1.2.2 (`main` + uncommitted `HANDOVER.md` publish checklist edits) |
| **Method** | Hostile re-audit of code, dual-stack parity, docs claims, live unit probes, full test/clippy green |
| **Evidence base** | Source review · `pytest` 48 · Rust lib 49 · `api_error_codes` 10 · clippy `-D warnings` · adversarial URL matrix |
| **Prior audits** | Point-in-time only; several narratives superseded by 1.2.2 code (ADV-01/02/04/09) |

**Stance:** Assume a curious local process, a tired packager, a dual-stack Docker operator, and a user who trusts marketing copy. Prefer falsification over freeze scores.

---

## 1. Executive verdict

NetRail remains a **strong single-user localhost research console** for its stated threat model: no remote bind, solid scheme/credential/encoded-loopback/private-IP open controls (post-1.2.2), honest history encryption boundaries, typed errors, and green CI surface.

It is **not** enterprise multi-user software. Residual risk is concentrated in:

1. **Unauthenticated full-product control on `127.0.0.1:7421`** (design).
2. **Open-URL host-list / redirect residuals** (new material finding: `duck.com` DDG unwrap gap).
3. **Dual-stack drift** (Rust production vs Python Docker/Flatpak).
4. **Doc/marketing truth debt** (one privacy overclaim; stale architecture/viability/audit narratives).
5. **Process hygiene** (uncommitted HANDOVER; historical freeze docs still say “publish open”).

| Dimension | Score (0–10) | One-liner |
|-----------|-------------:|-----------|
| Core job (search → rail → open) | 9 | End-to-end path solid |
| Threat-model fit (single-user local) | 8.5 | Honest SECURITY.md; residual open-URL/SSRF edges |
| Security vs local attacker | 5 | By design: full API = full product |
| Dual-stack parity | 7 | Security core close; contract/lifecycle gaps remain |
| Correctness / tests | 8.5 | Strong unit/security; weak dual golden harness |
| Ops / packaging | 8 | Compose localhost correct; Docker Python path residual |
| Docs truth | 7 | SECURITY/API good; README overclaim; stale lifecycle |
| Enterprise readiness (multi-user / SOC2-ish) | 3 | Not in scope; would need auth, audit log, SBOM, formal SDL |
| **Overall product (v1 desktop)** | **~8.2** | Soft GA for stated model; not multi-tenant |

**Release posture:** 1.2.2 is already published Latest. Treat this audit as **post-GA hardening backlog**, not a ship gate for 1.2.2.

---

## 2. What still holds under hostile reading

Do **not** re-litigate these without new evidence:

| Control | Evidence |
|---------|----------|
| Bind `127.0.0.1:7421` only | `server/mod.rs`, `netrail/main.py` — no bind-host env |
| Encoded loopback open blocked | Browser-style IPv4 parse + tests both stacks |
| RFC1918 / non-public open blocked | `OPEN_URL_PRIVATE` + parametrize tests |
| DDG `duckduckgo.com` / `r.` / (Python) `www.` `uddg` unwrap | Blocks inner localhost |
| CSP + nosniff + no-referrer | Middleware both stacks |
| Browser spawn argv, no shell; `LD_PRELOAD` stripped | `browsers.rs` / `browsers.py` |
| Doc asset path traversal blocked | basename + reject `..` `/` `\` |
| Tauri capabilities minimal | No shell plugin; UI talks HTTP |
| Version SSOT 1.2.2 | `scripts/check-versions.sh` OK |
| Rate limits 90/120 | Both stacks; disable via `NETRAIL_RATE_LIMIT=0` |
| Zero product telemetry | No analytics SDKs; health `telemetry: none` |
| Tests/clippy green (this pass) | 48 py · 49 rust lib · 10 integration · clippy clean |

---

## 3. Findings register (adversarial)

Severity key: **P0** ship/stop or active exploit class · **P1** real bypass or privacy/contract break · **P2** meaningful residual · **P3** polish / debt.

### SEC — Security

| ID | Sev | Title | Evidence | Attack scenario | Fix direction |
|----|-----|-------|----------|-----------------|---------------|
| **SEC-2026-01** | **P1** | DDG unwrap host set incomplete for **open** | Open validators: Rust `duckduckgo.com`,`r.duckduckgo.com`; Python +`www`. Merge resolve includes **`duck.com`**. Live probe: `https://duck.com/l/?uddg=http://127.0.0.1/` → **ALLOW** | Malicious/result URL uses `duck.com` redirect wrapper → browser follows to loopback/LAN after open | Unify DDG host set for open **and** merge (`duck.com`, `www`, `r.`); regression test |
| **SEC-2026-02** | **P1** | Env backend URL skips validation | `apply_env_overrides` sets `NETRAIL_SEARXNG_URL` / `SEARXNG_URL` without `validate_backend_url` (Rust `config.rs`, Python `config.py`). Validation only on save | Compromised env / bad Docker env points SearXNG at metadata hostname or rebinding host | Validate on load; fail start or ignore + log |
| **SEC-2026-03** | **P1** | Rebinding helper **apex** domains allowed | Code uses `ends_with(".localtest.me")` etc. Live: `http://localtest.me/`, `http://nip.io/` **ALLOW** while `foo.localtest.me` blocks | Apex domains that resolve to loopback/private used as open targets | Block exact apex **and** suffix; add tests |
| **SEC-2026-04** | **P2** | Open-URL is host-string only (no DNS pin) | No resolve in `validate_open_url` | Attacker-controlled DNS / open-redirect on public host → browser lands on private after click | Document residual; optional resolve-and-reject private; refuse redirects on backend HTTP client |
| **SEC-2026-05** | **P2** | Process SSRF via unauth settings → SearXNG | Backend URL **allows** localhost/LAN by design; settings PUT unauthenticated | Local process sets `searxng_url` to internal service, triggers search | Optional token; “strict backend” mode; disable HTTP redirects on client |
| **SEC-2026-06** | **P2** | Cloud metadata hostname allow | `metadata.google.internal` **ALLOW** open + backend; IP literal blocked | Cloud desktop/container uses name-based IMDS | Block known metadata hostnames; expand IMDS coverage |
| **SEC-2026-07** | **P2** | Rust IPv6 IMDS check wrong address | `is_cloud_metadata_ip` compares to `fd00:ec2::` (last segment 0), not `::254` | Backend URL `http://[fd00:ec2::254]/` may not match intended block on Rust | Fix segments + dual-stack test |
| **SEC-2026-08** | **P2** | Rust IPv4-mapped IPv6 open residual | Rust `Ipv6Addr::is_loopback` is `::1` only; no unmap of `::ffff:x.x.x.x`. Python 3.13 often blocks mapped forms via `is_private`/`is_loopback` | Stack-only open of mapped private/loopback on Rust | Unmap `to_ipv4_mapped` and re-run v4 policy |
| **SEC-2026-09** | **P2** | Unauthenticated localhost API (design) | All mutating routes open | Same-user malware: purge history, open phishing (public URLs), rewrite settings, export collections | Optional `NETRAIL_API_TOKEN`; document multi-user hosts out of model |
| **SEC-2026-10** | **P2** | History crypto partial + Rust degrade-open | FTS tokens + URLs plaintext (documented). Rust plaintext if keyring missing; Python may disable history | Disk forensics / silent plaintext session | Align policies; optional encrypt URLs; force `NETRAIL_DB_KEY` in headless |
| **SEC-2026-11** | **P3** | Rate limits soft-only | Search/open only; fixed window; disableable | Abuse of purge/settings unlimited; 90 searches/min still provider-hostile | Document as UX anti-spam; optional lower caps |
| **SEC-2026-12** | **P3** | Image CDN privacy | CSP allows `https:` images; `no-referrer` set | Tracking via thumbnail load | Optional proxy / images-off default |
| **SEC-2026-13** | **Info** | No dep audit in CI | Unpinned Python floors; Cargo.lock present | Supply chain | `cargo audit` / `pip-audit` job |

### PAR — Dual-stack parity

| ID | Sev | Title | Notes |
|----|-----|-------|-------|
| **PAR-01** | **P1** | Open DDG hosts differ open vs resolve vs stacks | Security fix must land **both** languages same PR |
| **PAR-02** | **P1** | History no-key behavior diverges | Rust degrade+banner; Python store often `None` |
| **PAR-03** | **P2** | Error `detail` / validation codes | Rust prefixes `detail`; collection `name` → `REQUEST_INVALID` on Python; invalid `mode` silent default on Rust |
| **PAR-04** | **P2** | Fanout 20s deadline Rust-only | Python can hang longer on slow backends |
| **PAR-05** | **P2** | Open browser failure shape | Python bare HTTPException path residual |
| **PAR-06** | **P3** | Collection notes max length Python-only | Rust unbounded notes |
| **PAR-07** | **P3** | No dual golden harness | Separate security tests; easy re-drift |

### DOC — Q&A / claims

| ID | Sev | Title | Verdict |
|----|-----|-------|---------|
| **DOC-01** | **P0** | README: “Every query stays on `127.0.0.1`” | **FALSE** — API is local; queries egress to backends |
| **DOC-02** | **P1** | MANUAL copy shortcut `Ctrl+Shift+C` | **FALSE** — code is `Ctrl+C` |
| **DOC-03** | **P1** | CONTINUITY_FREEZE / mid-AUDIT process “publish blocked” | **STALE** vs HANDOVER + GitHub Latest 1.2.2 |
| **DOC-04** | **P2** | VIABILITY / ARCHITECTURE lifecycle still “planned” for shipped features | **STALE** (v0.2 / Phase 1 tables) |
| **DOC-05** | **P2** | MANUAL/DISTRIBUTION omit local rate limits + several env vars | **GAP** |
| **DOC-06** | **P2** | `install.sh` says Tauri binary “**(v1.0)**” | **STALE** |
| **DOC-07** | **P2** | Prior ADV narrative still describes open encoded IPs / no rate limit in body | **STALE** mid-doc; summary later fixed |
| **DOC-08** | **P3** | Cold start &lt;100ms / binary sizes | **UNVERIFIED** marketing |
| **DOC-09** | **P3** | `netrail-api --api-only` implied required | **PARTIAL** — flag is desktop-meaningful |

### OPS / QA

| ID | Sev | Title | Notes |
|----|-----|-------|-------|
| **OPS-01** | **P2** | Docker path is Python, not Rust | Higher dual-stack cost for “production-like” containers |
| **OPS-02** | **P2** | No HTTP rate-limit integration test | Unit only |
| **OPS-03** | **P3** | No Tauri webview E2E | API smoke only (accepted residual R8) |
| **OPS-04** | **P3** | Uncommitted HANDOVER publish edits | Working tree dirty vs origin |
| **OPS-05** | **P3** | Starlette TestClient deprecation warning | pytest noise |

---

## 4. Adversarial Q&A (hostile answers)

| # | Question | Truthful answer |
|---|----------|-----------------|
| Q1 | Can a search result open `javascript:` / `file:`? | **No.** Scheme blocked both stacks. |
| Q2 | Can classic `127.0.0.1` / decimal/hex/octal loopback open? | **No** (fixed ADV-01). |
| Q3 | Can `https://duck.com/l/?uddg=http://127.0.0.1/` pass open validation? | **Yes today** — **SEC-2026-01**. |
| Q4 | Can `http://localtest.me/` open? | **Yes** (apex gap) — **SEC-2026-03**. |
| Q5 | Can RFC1918 open from results? | **No** (`OPEN_URL_PRIVATE`). LAN research via Open is blocked; SearXNG on LAN still allowed. |
| Q6 | Can malware as my user purge history / change settings? | **Yes** — no auth. |
| Q7 | Does encrypt history hide all research from disk forensics? | **No** — FTS tokens + URLs + collections plaintext. |
| Q8 | Does “query stays on 127.0.0.1” hold? | **No** — marketing false; backends + Wikipedia see the query. |
| Q9 | Are Docker and desktop the same engine? | **No** — Docker/Flatpak Python; desktop/API Rust. |
| Q10 | Is 1.2.2 published? | **Yes** (Latest). Older freeze docs may still say no. |
| Q11 | Does Images mode Wikipedia-fallback? | **No** — web mode only. |
| Q12 | Is rate limit a security boundary? | **No** — soft UX cap; not on purge/settings. |
| Q13 | Can env set unsafe SearXNG URL? | **Yes** — validation skipped on load — **SEC-2026-02**. |
| Q14 | Copy URL shortcut? | **Ctrl+C** with query focused (not Ctrl+Shift+C). |

---

## 5. Live verification snapshot (2026-07-31)

```text
scripts/check-versions.sh     → OK all 1.2.2
pytest tests/                 → 48 passed
cargo test --lib              → 49 passed
cargo test --test api_error_codes → 10 tests listed
cargo clippy --all-targets -- -D warnings → clean

Open-URL probes (Python):
  duck.com?uddg=http://127.0.0.1/     ALLOW   ← P1
  localtest.me / nip.io apex          ALLOW   ← P1
  2130706433 / 0x7f000001 / 127.1     BLOCK
  192.168.1.1 / 10.0.0.1              BLOCK OPEN_URL_PRIVATE
  [::ffff:127.0.0.1]                  BLOCK (Python 3.13)
  metadata.google.internal            ALLOW   ← P2
```

---

## 6. Workplan (enterprise backlog)

Ordered for **risk reduction per unit effort**. Rust remains production SSOT; every security policy change ships **dual-stack + shared golden vectors**.

### Wave 0 — Truth & hygiene (0.5–1 day)

| Item | Owner area | Acceptance |
|------|------------|------------|
| **W0.1** Fix README privacy line (DOC-01) | Docs | No claim that queries stay on loopback; distinguish API bind vs query egress |
| **W0.2** Fix MANUAL Ctrl+C (DOC-02) | Docs | Matches `app.js` |
| **W0.3** Stamp CONTINUITY_FREEZE + old audits as historical | Docs | Banner: superseded by this audit + 1.2.2 code |
| **W0.4** Commit or discard HANDOVER publish checklist edits | Process | Clean `git status` or intentional WIP note |
| **W0.5** `install.sh` drop “v1.0” | Packaging | Neutral wording |

### Wave 1 — Security P1 (1–2 days) — **next release candidate 1.2.3**

| Item | Files | Acceptance |
|------|-------|------------|
| **W1.1** Unified DDG host set for open+merge | `security.rs`, `security.py`, `url_resolve.rs`, `merge.py` | `duck.com` / `www` / `r.` unwrap; localhost inner → `OPEN_URL_LOCALHOST` both stacks |
| **W1.2** Block rebinding **apex** + suffix | security both | `localtest.me`, `nip.io`, `sslip.io`, `xip.io` blocked exactly and as suffixes |
| **W1.3** Validate backend URL on env load / startup | config both | Invalid `NETRAIL_SEARXNG_URL` ignored or process fails with typed error; never silent use of metadata/rebinding |
| **W1.4** Shared golden vectors | `tests/fixtures/url_policy.json` (or similar) + Rust table + pytest | Same file drives both; CI fails on drift |
| **W1.5** Regression tests for W1.1–W1.3 | unit + optional HTTP | Green clippy/pytest |

### Wave 2 — Security P2 + parity (2–4 days)

| Item | Acceptance |
|------|------------|
| **W2.1** Rust unmap IPv4-mapped IPv6 for open (+ backend metadata) | Mapped loopback/private rejected; tests |
| **W2.2** Fix Rust `fd00:ec2::254` metadata match | Matches Python + API_ERRORS |
| **W2.3** Block known metadata hostnames | At least `metadata.google.internal`, `metadata`, AWS classic names as policy |
| **W2.4** Backend HTTP client: no redirects (or deny private final hop) | Documented; unit/integration |
| **W2.5** Align history encrypt-no-key policy | Same health fields + store behavior both stacks |
| **W2.6** Python fanout overall deadline (~20s) | Matches ARCHITECTURE / Rust |
| **W2.7** Error contract cleanup | Collection name → `COLLECTION_NAME_INVALID`; open browser → `BROWSER_NOT_FOUND`; prefer raw `detail` message on Rust **or** document prefix |
| **W2.8** Invalid search `mode` → 400 both (no silent default) | Explicit |

### Wave 3 — Hardening optional / enterprise path (backlog)

| Item | Notes |
|------|-------|
| **W3.1** Optional `NETRAIL_API_TOKEN` (header) | Default off for desktop UX; recommended Docker |
| **W3.2** `strict_backend_urls` / confirm private SearXNG | Homelab vs cloud split |
| **W3.3** Rate-limit settings/purge lightly | Abuse resistance |
| **W3.4** Docker image based on `netrail-api` | Shrink dual-stack production surface |
| **W3.5** `cargo audit` + `pip-audit` CI | Supply chain |
| **W3.6** SBOM + signed releases | Enterprise packaging bar |
| **W3.7** Audit log of local API mutating actions | Optional file under XDG |
| **W3.8** Encrypt visit/collection URLs or document “not full disk privacy” deeper | Product decision |

### Wave 4 — Docs & ops completeness (1 day)

| Item | Acceptance |
|------|------------|
| **W4.1** MANUAL: local rate limits, Wikipedia egress, Images no-wiki, private open blocked vs LAN SearXNG | User can self-serve |
| **W4.2** DISTRIBUTION env table: `NETRAIL_RATE_LIMIT`, `NETRAIL_STATIC_DIR`, token if added | Complete |
| **W4.3** Refresh or archive VIABILITY + ARCHITECTURE lifecycle | Current = 1.2.x reality |
| **W4.4** Clarify `netrail` vs `netrail-api` flags | No `--api-only` confusion |
| **W4.5** Soften size/latency claims or measure in CI note | Evidence-backed |

### Wave 5 — Test harness (ongoing)

| Item | Acceptance |
|------|------------|
| **W5.1** `scripts/parity-api-smoke.sh` | Same probes → Rust binary + Python; equal `code`+status for security cases |
| **W5.2** Python `test_api.py` mirrors `api_error_codes.rs` | FANOUT, HISTORY_*, CONFIG_*, COLLECTION_* |
| **W5.3** HTTP 429 test with test seam / env | Both stacks |
| **W5.4** Keep e2e-api-smoke as release gate | Already in CI |

---

## 7. Suggested release mapping

| Release | Scope |
|---------|--------|
| **1.2.3 (patch)** | Wave 0 + Wave 1 (docs truth + open-URL/env validation P1s) |
| **1.3.0 (minor)** | Wave 2 parity/hardening + optional token behind flag |
| **2.x** | Owned corpus / multi-user — only with redesign of bind+auth (HANDOVER out-of-scope preserved) |

---

## 8. Explicit non-goals (this workplan)

- Multi-tenant SaaS auth, remote bind by default  
- Replacing DDGS scrape reliability (external)  
- Full E2E GTK/Tauri driver (cost/benefit poor vs API smoke)  
- Claiming “malware-proof” or “enterprise zero-trust” without Wave 3  

---

## 9. Scorecard delta vs freeze (~8.8)

| Criterion | Freeze | This audit | Why |
|-----------|-------:|-----------:|-----|
| Security | 9 | **8** | New open-URL host-list/apex/env residuals under re-probe |
| Docs / claims | 9 | **7** | Privacy overclaim + stale lifecycle/audit docs |
| Dual-stack | (in arch 8.5) | **7** | Contract/history/deadline drift still real |
| Tests | 8.5 | **8** | Strong units; no shared golden + thin Python HTTP error suite |
| Overall | ~8.8 | **~8.2** | Still soft-GA for model; honest post-publish backlog |

---

## 10. Immediate operator checklist

```bash
bash scripts/check-versions.sh
source .venv/bin/activate && pytest tests/ -q
cd src-tauri && cargo clippy --all-targets -- -D warnings && cargo test
# After Wave 1:
# add golden vectors; re-run security matrix including duck.com + apex rebinding
```

**First code PR:** W1.1 + W1.2 + W1.3 + tests (dual-stack).  
**First docs PR:** W0.1 + W0.2 (can land same day).

---

*Enterprise adversarial audit — NetRail 1.2.2 — 2026-07-31 — evidence over freeze mythology.*
