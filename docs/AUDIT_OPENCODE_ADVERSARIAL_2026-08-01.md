# NetRail — Adversarial Q&A & Post-1.4.0 Workplan (OpenCode session)

| Field | Value |
|-------|--------|
| **Date** | 2026-08-01 |
| **Tree** | 1.4.0 (`main` HEAD `7d0daef`, 5 ahead of origin, WIP desktop/UI uncommitted) |
| **Method** | Hostile re-probe of both stacks against audit findings + new adversarial vectors |
| **Evidence base** | `check-versions.sh` OK · pytest 91 · Rust lib 59 · `api_error_codes` 10 · clippy `-D warnings` clean · e2e-api-smoke OK · parity-api-smoke OK · live API probes Rust+Python (trailing-dot, zone-id, mapped, strict-backend, malformed IPv6) |
| **Prior doc** | [AUDIT_ENTERPRISE_2026-07-31.md](AUDIT_ENTERPRISE_2026-07-31.md) — waves 0–5 landed in 1.2.3→1.4.0; this pass re-verifies closure and adds **N1–N4** |

**Bottom line:** the audit's Waves 1–5 are confirmed closed in code and tests, **except** one harness gap that shipped: the shared golden fixture contains **no trailing-dot (FQDN-root) IP vectors**, and Python's parser does not normalize them. This pass proved a **live Python-only open-URL policy bypass** (browser actually spawned to `127.0.0.1.`), a **strict-backend bypass**, and an **untyped 500** — none of which Rust has.

---

## 1. Adversarial Q&A (hostile, evidence-based)

| # | Question | Truthful answer | Evidence |
|---|----------|-----------------|----------|
| Q1 | Can a search result open `javascript:` / `file:` / `data:`? | **No.** Both stacks block non-http(s) schemes with typed `OPEN_URL_INVALID_SCHEME` / `OPEN_URL_INVALID`. | `security.rs:34-40`, `security.py:221-230`, fixture `reject_javascript` |
| Q2 | Can classic loopback forms open? (decimal, hex, octal, short, mapped) | **No** on both stacks. `2130706433`, `0x7f000001`, `0177.0.0.1`, `127.1`, `[::ffff:127.0.0.1]` all blocked. | Rust `parse_browser_ipv4` + `effective_ip`; Python equivalents; fixture vectors |
| Q3 | Can `duck.com` / `www.` DDG wrappers hide a loopback target? | **No.** Unified host set `{duckduckgo.com, duck.com}` + suffix match in both stacks; inner URL re-validated. | `security.rs:7`, `security.py:11`, tests `unwraps_duck_com…` both stacks |
| Q4 | Can rebinding-helper **apex** domains open? | **No.** `localtest.me`, `nip.io`, `sslip.io`, `xip.io` blocked as exact host and suffix. | `security.rs:8`, tests both stacks, fixture apex vectors |
| Q5 | Can RFC1918 / link-local / metadata targets open from results? | **No.** Private → `OPEN_URL_PRIVATE`; link-local → `OPEN_URL_LINK_LOCAL`; metadata hostnames + `169.254.169.254` + `fd00:ec2::254` → `OPEN_URL_CLOUD_METADATA`. | `security.rs:183-262`, `security.py`, live probes |
| **Q6** | **Can `http://127.0.0.1./` open? (NEW — FIXED)** | **Originally: Python YES — browser spawned to loopback. Rust: NO.** Python's `urlparse` kept the trailing dot, so `_parse_browser_ipv4` saw 5 parts → `None` → host passed. Browsers strip the trailing dot at DNS resolution → loopback. Same for `192.168.1.1.`, `10.0.0.1.`, `0x7f.0.0.1.`, `127.000.000.001.`, `127.0.0.1.:8080/admin`, and via DDG `uddg` unwrap. **Fixed 2026-08-01** in both stacks; `POST /api/open` now returns `400 OPEN_URL_LOCALHOST` / `OPEN_URL_PRIVATE`. | Live probe (pre-fix): Python `200 {"browser":"Brave","url":"http://127.0.0.1./"}`; post-fix both stacks `400`. → **N1 (closed)** |
| **Q7** | **Can `strict_backend_urls` be bypassed with a trailing dot? (NEW — FIXED)** | **Originally: Yes, Python. No, Rust.** `validate_backend_url("http://127.0.0.1.:8080", strict=True)` → allowed in Python (literal check misses `127.0.0.1.`; IP parse returns `None`). Rust: live `PUT /api/settings` with strict env → `BACKEND_URL_STRICT_PRIVATE` (url crate normalizes host). **Fixed 2026-08-01**: Python normalizes host before policy; both stacks now `BACKEND_URL_STRICT_PRIVATE`. | Live probes both stacks. → **N2 (closed)** |
| Q8 | **Does malformed IPv6 return a typed error? (NEW — FIXED)** | **Originally: No, Python.** `http://[::ffff:7f00:1]./` raised raw `ValueError: Invalid IPv6 URL` from `urlparse` → unhandled 500, no `{code,detail,status}`. **Fixed 2026-08-01**: `urlparse` wrapped → typed `OPEN_URL_INVALID`; parity with Rust. | Live probe + traceback (pre-fix at `security.py:219`). → **N3 (closed)** |
| Q9 | Can malware as my user purge history / rewrite settings? | **Yes** by default — documented design (R1/SEC-2026-09). `NETRAIL_API_TOKEN` + `NETRAIL_INJECT_UI_TOKEN` now exist, but **default off**. Mutations are rate-limited (60/min) and audit-logged when enabled. | `auth.rs`, `rate_limit.rs:13`, `server/mod.rs` audit calls |
| Q10 | Does the API token protect against local readers? | **Partially.** `/api/*` (except health) requires Bearer / `X-NetRail-Token` when set — but the **unauthenticated `/` page gets the token injected into HTML** when `NETRAIL_INJECT_UI_TOKEN` is on (default when token set). Any local HTTP client can `GET /` and read it. Honest scope: token is an accidental-cross-process / Docker guard, **not** a same-user malware defense (env is readable anyway). | `auth.rs:57-65` (only `/api/*`), `server/mod.rs:129-157` (index injection) |
| Q11 | Is history fully encrypted at rest? | **No.** FTS5 tokens + URLs + collections plaintext by design; Fernet only when keyring or `NETRAIL_DB_KEY` present; degrade + banner otherwise. Documented. | `SECURITY.md`, handoff §4.2 |
| Q12 | Does "everything stays on 127.0.0.1" hold? | **No** — queries egress to DDGS/SearXNG/Brave/Wikipedia. Docs fixed in Wave 0. | `README.md` (fixed), handoff §4.2 |
| Q13 | Are Docker and desktop the same engine? | **No.** Docker/Flatpak/`install.sh` = Python; desktop/`netrail-api` = Rust. Parity harness exists; **this pass found the harness can still miss policy drift (N1–N3)**. | `docker-compose.yml`, `Dockerfile`, parity smoke |
| Q14 | Is the rate limit a security boundary? | **No** — soft UX anti-abuse (fixed window, disableable). Now covers search/open/mutate (settings, purge, collections), but bursts at window boundaries remain. | `rate_limit.rs`, `server/mod.rs` call sites |
| Q15 | Does Images mode expose thumbnails to trackers? | **Partially** — `no-referrer` set, but CSP `img-src 'self' https: data:` still loads remote images → CDN sees request. Accepted residual R7. | `CSP` const, `security.py` |
| Q16 | Can a public result URL redirect to localhost after validation? | **Yes** — no DNS pin (SEC-2026-04 class). Backend client disables redirects (`http_client.rs:15`), but the **browser** follows any redirect from the validated public URL. Documented residual; expensive to fix. | `http_client.rs` redirect none; open path has no resolve |
| Q17 | Do both stacks agree on the golden vectors? | **Yes for the 30 shipped vectors** — parity smoke green. **No for trailing-dot/odd-host forms** (missing from fixture). | `parity-api-smoke.sh`, fixture contents |
| Q18 | Are process-memory / config-file secrets safe? | Brave key: env only (never settings.json). Fernet key: env or keyring. UI token: page HTML when injected (see Q10). | `config.rs`, `SECURITY.md` |

---

## 2. Findings register (this pass)

Severity key: **P0** ship/stop · **P1** real bypass/privacy/contract break · **P2** meaningful residual · **P3** polish.

### New findings (2026-08-01, not in 07-31 audit)

| ID | Sev | Title | Evidence | Attack scenario | Fix direction |
|----|-----|-------|----------|-----------------|---------------|
| **N1** | **P1** | Open-URL policy allows **trailing-dot IP literals** (Python) and **trailing-dot DDG hosts** (Rust+Python) | Python: `validate_open_url("http://127.0.0.1./")` → allowed; live `/api/open` **spawned Brave to `127.0.0.1.`**. Rust: `https://duckduckgo.com./l/?uddg=http://127.0.0.1/` → ALLOW (host `duckduckgo.com.` fails `is_ddg_host` suffix match → no unwrap → browser follows redirect to loopback). Fixture had no trailing-dot vectors. | Malicious search result / DDG-unwrapped URL `http://127.0.0.1.:8080/admin` → browser hits localhost/LAN service (SSRF-class open); trailing-dot DDG wrapper defeats unwrap | Python: normalize host (percent-decode, lowercase, strip trailing dots) before policy — `_normalize_host` in `security.py`. Rust: `trim_end_matches('.')` in `is_ddg_host` call + `block_unsafe_host`/`block_backend_host`. Both: fixture vectors + parity probe |
| **N2** | **P1** | Python `strict_backend_urls` bypass via trailing dot | `validate_backend_url("http://127.0.0.1.:8080", strict=True)` → allowed | Strict mode (cloud/homelab split) claims to reject loopback backends but doesn't → settings PUT can point SearXNG at localhost in strict mode | Same normalization fix as N1 for backend path; fixture vector `strict` note |
| **N3** | **P2** | Python untyped 500 on malformed bracketed IPv6 | `http://[::ffff:7f00:1]./` → raw `ValueError: Invalid IPv6 URL` (from `urlparse`), no typed error body | Any local process (or result URL) can force 500s; breaks `{code,detail,status}` invariant 11 | Catch `ValueError` in `_validate_open_url_inner` → typed `OPEN_URL_INVALID`; same for `_block_backend_host` caller |
| **N4** | **P3** | Golden fixture lacked trailing-dot / malformed-IPv6 vectors | Fixture had 22 open + 8 backend vectors; none covered FQDN-root forms | Harness "green" while N1–N3 existed → drift-proofing claim overstated | ✅ **Closed 2026-08-01** — 13 vectors added (9 open, 2 strict backend, 1 malformed IPv6, 1 non-strict allow); `strict` field supported by both harnesses; parity smoke green |

### Audit residuals — re-verified status (2026-08-01)

| Item (audit ID) | Status on 1.4.0 | Verified by |
|-----------------|-----------------|-------------|
| SEC-2026-01 DDG `duck.com` | ✅ Closed | code both stacks + tests + fixture |
| SEC-2026-02 env backend validation | ✅ Closed | `config.rs:179`, `config.py:51`, `test_config_env.py` |
| SEC-2026-03 rebinding apex | ✅ Closed | tests + fixture apex vectors |
| SEC-2026-06 metadata hostnames | ✅ Closed | `security.rs:10-15`, tests |
| SEC-2026-07 AWS IMDS IPv6 `fd00:ec2::254` | ✅ Closed | `security.rs:381`, test |
| SEC-2026-08 IPv4-mapped IPv6 | ✅ Closed | `effective_ip` both stacks, tests |
| SEC-2026-09 unauth localhost API | 🔶 Mitigated — optional token, default off (by design) | `auth.rs`; **Q10 nuance** |
| SEC-2026-10 history crypto partial | 🔶 Residual — documented | `SECURITY.md` |
| SEC-2026-11 soft rate limits | 🔶 Mostly closed — mutate caps added (60/min); still fixed-window, disableable | `rate_limit.rs` |
| SEC-2026-12 image CDN privacy | 🔶 Residual R7 | CSP `img-src https:` |
| SEC-2026-13 no dep audit | ✅ Closed — `cargo audit` / `pip-audit` CI | handoff §1.3, CI files |
| PAR-01 open DDG hosts differ | ✅ Closed | unified sets + fixture |
| PAR-02 history no-key behavior | ✅ Closed (handoff claims parity; Rust↔Python Fernet tests green) | `history` tests |
| PAR-03 error detail/mode/collection | ✅ Closed per handoff | `api_error_codes` 10 green |
| PAR-04 fanout 20s | ✅ Closed | handoff §6 |
| PAR-07 dual golden harness | 🔶 **Gap exposed by N1–N4** | this pass |
| SEC-2026-04 DNS pin on open | 🔶 Residual (documented, expensive) | no resolve in open path |
| SEC-2026-05 process SSRF via settings | 🔶 Mitigated — token + strict (but **N2 weakens strict**) | this pass |
| R8 / OPS-03 no Tauri E2E | 🔶 Residual (accepted) | — |
| R3 dual-stack cost | 🔶 Residual — **this pass shows drift is still reachable** | N1–N3 |

---

## 3. Workplan (post-1.4.0, risk/effort ordered)

All security items ship **dual-stack + golden vectors** (invariant 12). Rust remains production SSOT.

### Wave A — Close N1–N3 + fixture gap (0.5–1 day) — **patch candidate 1.4.1** ✅ Landed 2026-08-01

| # | Item | Files | Acceptance |
|---|------|-------|------------|
| A1 | Normalize host (percent-decode, lowercase, strip trailing dots) before open **and** backend policy — both stacks | `netrail/security.py` (`_normalize_host`, `_block_unsafe_host`, `_block_backend_host`, validators), `src-tauri/src/security.rs` (`trim_end_matches('.')` in DDG unwrap + block fns) | ✅ `127.0.0.1.` / `192.168.1.1.` / `0x7f.0.0.1.` / `127.000.000.001.` / `127.0.0.1.:8080` → same typed codes as before; trailing-dot DDG hosts (`duckduckgo.com.`, `duck.com.`) unwrap and block inner loopback — both stacks |
| A2 | Python: catch `ValueError` from `urlparse` on malformed bracketed hosts → typed `OPEN_URL_INVALID` | `netrail/security.py:215-250` | ✅ `http://[::ffff:7f00:1]./` → `400 OPEN_URL_INVALID` (no untyped 500); parity with Rust |
| A3 | Extend golden fixture with N1/N2/N3 vectors (trailing-dot open ×9, strict trailing-dot backend ×2, malformed IPv6 ×1) + `strict` field support in harnesses | `tests/fixtures/url_policy.json`, `tests/test_url_policy.py`, `security.rs` golden test | ✅ Fixture drives Rust table test + pytest + parity smoke; CI green (pytest 104, Rust 59+10, clippy clean) |
| A4 | Rust regression guard: trailing-dot DDG unwrap + backend strict vectors | `security.rs` tests + fixture | ✅ Live probe: `duckduckgo.com.` / `duck.com.` unwrap → `OPEN_URL_LOCALHOST` |
| A5 | Confirmed Rust blocks trailing-dot backend strict (`BACKEND_URL_STRICT_PRIVATE`, live PUT probe) — no Rust change needed beyond normalization; regression vectors in fixture | `security.rs` tests | ✅ Rust behavior locked in by fixture vectors |

### Wave B — Harden the harness (0.5 day) ✅ Landed 2026-08-01

| # | Item | Acceptance |
|---|------|------------|
| B1 | Drive Rust live parity probes from the shared fixture (was hardcoded list) | ✅ `scripts/parity-api-smoke.sh` now loops every `open_url` vector in `url_policy.json` against the live Rust binary (32 vectors), asserting code+status; Python side covered by `test_url_policy.py`. Any fixture addition automatically gates both stacks |
| B2 | Fixture = policy SSOT (documented in Wave A notes + this file) | ✅ Doc note; no code |

### Wave C — Residual risk documentation & operator hardening (backlog, not blocking) ✅ Doc items landed 2026-08-01

| # | Item | Notes |
|---|------|-------|
| C1 | Document token-in-`/`-page tradeoff (Q10) in `SECURITY.md` + `docs/DISTRIBUTION.md` | ✅ New "Optional API token" section in SECURITY.md (inject-on default, local readers can fetch it, same-user malware reads env anyway); DISTRIBUTION env table row expanded |
| C2 | Strengthen Docker guidance: token **on by default** in compose example; strict backends on | ✅ compose: token/strict/audit recommended + rust profile pass-throughs; `.env` example updated; pre-existing YAML break fixed (quoted DB_KEY guard) |
| C3 | DNS-pin on open (SEC-2026-04): keep documented residual; add "resolve-and-warn" experimental flag if ever funded | Expensive/UX-sensitive; do not build now |
| C4 | Images mode: optional `images:off` / proxy as product decision (R7) | Product call, not security fix |

### Wave D — Process hygiene (already in handoff §9 P0; keep ordered)

1. Commit or discard uncommitted desktop/UI WIP (Spotlight + card CSS) after visual verify.
2. Decide 1.4.0-delta vs patch 1.4.1 (Wave A is natural 1.4.1 content if shipping).
3. Push 5 local commits **only on human request**.
4. Confirm GitHub release/tag alignment for 1.4.0.

### Suggested release mapping

| Release | Scope |
|---------|--------|
| **1.4.1 (patch)** | Wave A + B + C — landed on `main` (commits `f3fd306`, `4a43935`, `b091b1c`); tag/push only on human request |
| **1.5** | Remaining residuals: C3 DNS-pin flag, C4 images-off, dual-stack golden growth |
| **2.x** | Only with owned-corpus / multi-user redesign (bind+auth) — out of model unless human asks |

### Explicit non-goals (this workplan)

- Multi-tenant SaaS auth, remote bind by default
- DNS-pin enforcement, local LLM, owned corpus
- Tauri webview E2E driver (accepted residual R8)
- Rewrites of the vanilla UI or Rust-primary architecture

---

## 4. Reproduce (commands used for this pass)

```bash
bash scripts/check-versions.sh
source .venv/bin/activate && pytest tests/ -q
cd src-tauri && cargo clippy --all-targets -- -D warnings && cargo test
NETRAIL_RATE_LIMIT=0 bash scripts/e2e-api-smoke.sh
bash scripts/parity-api-smoke.sh

# N1/N2 probes (Python)
python3 - <<'EOF'
from netrail.security import validate_open_url, validate_backend_url
for u in ["http://127.0.0.1./admin","http://192.168.1.1./","http://127.0.0.1.:8080/admin"]:
    try: print("ALLOW", u, validate_open_url(u))
    except Exception as e: print("BLOCK", u, getattr(e,"code",e))
try: validate_backend_url("http://127.0.0.1.:8080", strict=True); print("STRICT-ALLOW 127.0.0.1.:8080")
except Exception as e: print("STRICT-BLOCK", getattr(e,"code",e))
EOF

# N3 probe: POST /api/open {"url":"http://[::ffff:7f00:1]./"} → untyped ValueError (Python)
```

---

*Adversarial Q&A + workplan — NetRail 1.4.0 — 2026-08-01 — live-probed, dual-stack honest, no scope creep.*
