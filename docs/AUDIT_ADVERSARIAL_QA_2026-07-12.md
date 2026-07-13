# NetRail — Adversarial Q&A Audit (post-hardening)

| Field | Value |
|-------|--------|
| **Date** | 2026-07-12 |
| **Tree** | Working tree **1.2.2** (uncommitted Phase A + Python parity) |
| **Method** | Hostile Q&A · live probes · packaging · process |
| **Tests at audit** | Rust clippy+48 · Python 37 · all green |

**Stance:** Assume a curious attacker, a flaky network, a tired packager, and a dual-stack client. Prefer evidence over vibes.

---

## Verdict (one paragraph)

NetRail **survives a local-desktop adversarial pass** for its stated threat model (single-user, `127.0.0.1`, no remote surface). Schema-level open-URL defenses work against classic schemes, DDG unwrap to localhost, credentials, and cloud-metadata backend URLs. **Residual real issues:** encoded-loopback open bypasses, RFC1918 open allowed by design, Python FastAPI 422 shape ≠ Rust typed errors, AppImage local build broken without `patchelf`, and **release process still blocked** (uncommitted tree, drafts 1.2.0/1.2.1, public latest = 1.1.1). **No ship-stopping RCE/SSRF-to-LAN-as-server** found. **Do not tag until commit + CI green.**

**Score:** product **8/10** · process **5/10** · combined RC gate **hold until process closes**.

---

## Q&A register

### Q1. Can a search result open `javascript:` / `data:` / `file:` and own the machine?

**A. No.** Both stacks reject non-http(s) schemes.

| Probe | Result |
|-------|--------|
| `javascript:alert(1)` | `OPEN_URL_INVALID_SCHEME` |
| `data:text/html,…` | `OPEN_URL_INVALID_SCHEME` |
| `file:///etc/passwd` | `OPEN_URL_INVALID_SCHEME` |

Browser spawn uses argv list (no shell). **Pass.**

---

### Q2. Can a result force-open localhost / loopback services?

**A. Partially blocked — classic hosts yes; encoded forms no.**

| Probe | Result | Severity |
|-------|--------|----------|
| `http://127.0.0.1/` | BLOCK `OPEN_URL_LOCALHOST` | — |
| `http://localhost/` | BLOCK | — |
| `http://[::1]/` | BLOCK | — |
| DDG `uddg=` → `127.0.0.1` | BLOCK (unwrap works) | — |
| `http://169.254.169.254/…` | BLOCK link-local | — |
| `http://192.168.1.1/` | **ALLOW** + open 200 | P2 design |
| `http://10.0.0.1/` | **ALLOW** | P2 design |
| `http://2130706433/` (decimal 127.0.0.1) | **ALLOW** | **P1** |
| `http://0x7f000001/` | **ALLOW** | **P1** |
| `http://0177.0.0.1/` | **ALLOW** | **P1** |
| `http://127.1/` | **ALLOW** | **P1** |

**Finding ADV-01 (P1):** Host string is only checked as literal names / `ipaddress`/`IpAddr` parse. Browsers often still resolve decimal/hex/octal/short IPv4 to loopback. **Recommendation:** normalize host via a full IPv4 parser (or reject hosts that are not DNS-looking and fail strict parse), then re-check `is_loopback` / private as policy.

**Finding ADV-02 (P2 / accepted):** RFC1918 open is intentional for “research console”; document in SECURITY + MANUAL as residual. Optional `strict_open_urls` later.

---

### Q3. Can a user-configured SearXNG URL hit cloud metadata or rebinding hosts?

**A. No for metadata/rebinding; localhost and LAN allowed intentionally.**

| Probe | Result |
|-------|--------|
| `http://169.254.169.254/` | BLOCK `BACKEND_URL_CLOUD_METADATA` |
| `http://evil.nip.io/` | BLOCK rebinding |
| `http://127.0.0.1:8080` | ALLOW |
| `http://192.168.0.5:8080` | ALLOW |

**Pass** for stated self-host model.

---

### Q4. Is the UI safe against XSS from titles/snippets?

**A. Largely yes.** Result cards use `escapeHtml` for title, URL, snippet; `data-url` uses `encodeURIComponent`. Image `src` is provider URL under CSP `img-src 'self' https: data:` — **privacy leak / tracking pixel possible**, not script execution. Docs markdown → `innerHTML` is local trusted content only.

**Finding ADV-03 (P3):** Malicious image CDN can observe loads when Images mode is used. Accept or add optional image proxy off-by-default.

---

### Q5. Can SQL/FTS injection corrupt history?

**A. Parameterized SQLite queries; FTS query is token-scrubbed.** Adversarial search strings like `'; DROP TABLE` are treated as search text (200 if backends answer), not executed as SQL. FTS strips non-word chars into quoted tokens. **Pass** for injection. **Residual:** FTS still stores **plaintext** query tokens (documented honesty).

---

### Q6. Does “all backends empty” still silently fail?

**A. No (both stacks, post-hardening).**

- Empty batch → `errors[]` (`returned no results`)
- Web mode empty → Wikipedia fallback
- Still empty + errors → `FANOUT_TOTAL_FAILURE` (502) on search layer
- Images mode empty → **no** Wikipedia (correct); errors only

**Pass** (Tribunal C01 closed and Python-parity closed).

---

### Q7. Dual-stack: do clients get the same error contract?

**A. No — residual shape gap.**

| Case | Rust | Python |
|------|------|--------|
| Empty query | 400 `{code:QUERY_INVALID,…}` | **422** FastAPI validation list (no `code`) |
| `max_results` overflow body | 400 typed / clamp | **422** validation |
| Invalid mode | default web + warn | **422** (Literal) |
| Open bad scheme | 400 + `code` | 400 + `code` ✅ |

**Finding ADV-04 (P2):** Docker/Flatpak Python clients cannot branch on `code` for validation failures. **Fix options:** custom exception handler mapping 422 → NetRailError, or document “prefer Rust binary for stable error codes.”

---

### Q8. Is localhost API abusable by other local processes?

**A. Yes — by design.** Any UID that can connect to `127.0.0.1:7421` can search, purge history, open URLs (within open-URL policy), change settings. No auth token. Documented in README/SECURITY.

**Finding ADV-05 (accepted v1):** Not a bug for threat model; block GA marketing claims that imply “malware-proof.” Future: optional local token.

Second instance: bind fails cleanly (`Address already in use`) — **Pass**.

---

### Q9. Can browser spawn be hijacked via settings `browser_id`?

**A. Constrained.** Discovery only from `.desktop` / known stems; open uses resolved executable path + argv. Malicious `.desktop` on the user machine is out of scope (same as any desktop). `LD_PRELOAD` stripped on spawn. **Pass.**

---

### Q10. History encryption — fail closed or open?

**A. Degrades open with banner** when keyring missing (`encryption_degraded`). FTS plaintext always. **Pass** only if RC notes keep honesty; **ADV-06 (P2):** users who enable encrypt may still get plaintext session without reading banner.

---

### Q11. Packaging: does the shippable binary include UI assets?

**A. Yes for `.deb` (and rpm) built 1.2.2.**

- `usr/share/netrail/static/{index.html,app.js,style.css,…}` present  
- `usr/bin/netrail` + `usr/bin/netrail-api` present  
- Live `netrail-api`: `/` and `/static/app.js` 200; health `version=1.2.2`

**AppImage local:** fail — missing **`patchelf`**. CI historically succeeds. **ADV-07 (P1 process):** do not claim local AppImage for this host; gate AppImage on CI.

**Note:** Local `dist/*.deb` timestamp is **before** last Wikipedia Python file change; **desktop deb is Rust** so Wikipedia in binary depends on Rust rebuild (done). Python Docker image must be rebuilt from current tree for parity.

---

### Q12. Version / release process — can a user get 1.2.2 today?

**A. No.**

| Fact | State |
|------|--------|
| Public GitHub **Latest** | **v1.1.1** |
| v1.2.0 / v1.2.1 | **Draft** only |
| Working tree 1.2.2 | **Uncommitted** on `main` |
| CI on remote `main` | Still red until push of clippy fix |

**Finding ADV-08 (P0 process):** Shipping risk is process, not product. Tagging uncommitted work or publishing old drafts without hygiene reopens CI failure narrative.

---

### Q13. Denial of service / resource abuse?

| Vector | Behavior | Note |
|--------|----------|------|
| Concurrent `/api/health` (40) | All 200 | OK |
| Fanout 20s deadline | Caps wait | OK |
| Unbounded history growth | TTL purge | OK if TTL set |
| No rate limit on search | Accepted localhost | Local malware can thrash providers |

**ADV-09 (P3):** Optional local rate limit is nice-to-have only.

---

### Q14. Does clippy/CI still block after hardening?

**A. Local: green.** Clippy `-D warnings` pass; 48 Rust + 37 Python tests pass. Remote green **only after push**.

---

### Q15. Wikipedia fallback — abuse / leakage?

Calls `en.wikipedia.org` with user query and titles. No API key. User query leaves the machine for Wikimedia (and for DDGS/Brave/SearX when enabled). **Honest** for metasearch product. **ADV-10 (P3):** document Wikipedia as network egress in MANUAL recovery section.

---

## Findings summary (prioritized)

| ID | Sev | Title | Action |
|----|-----|-------|--------|
| **ADV-08** | **P0** | Uncommitted 1.2.2 + drafts + latest still 1.1.1 | ✅ Code committed in 1.2.2; push/tag still operator |
| **ADV-01** | **P1** | Encoded loopback open-URL bypass | ✅ Fixed (browser-style IPv4 parse) |
| **ADV-07** | **P1** | Local AppImage needs `patchelf` | ✅ Documented; CI already installs patchelf |
| **ADV-04** | **P2** | Python 422 ≠ Rust `code` for validation | ✅ FastAPI handler → NetRailError shape |
| **ADV-02** | **P2** | RFC1918 open allowed | ✅ Blocked on open (`OPEN_URL_PRIVATE`); backends may use LAN |
| **ADV-06** | **P2** | Encryption degrade | ✅ Documented in SECURITY.md |
| **ADV-03** | **P3** | Image tracking pixels | ✅ Documented residual in SECURITY |
| **ADV-09** | **P3** | No local rate limit | Accepted backlog (localhost model) |
| **ADV-10** | **P3** | Wikipedia egress | ✅ SECURITY / recovery UX |

---

## What still passes hard (do not re-litigate)

- Localhost bind only  
- No telemetry  
- CSP + nosniff + no-referrer  
- DDG redirect unwrap for SSRF-ish localhost  
- Typed errors on Rust path + Python `NetRailError` for domain errors  
- Silent empty fanout fixed + Python parity  
- Tray/clippy hygiene for 1.2.2  
- Deb static asset regression (1.2.0 class) fixed in packages  

---

## Adversarial RC checklist (go/no-go)

| Check | Go? |
|-------|-----|
| Clippy + tests local | ✅ |
| ADV-01 encoded IP fix or explicit residual in SECURITY | ⚠️ open |
| Commit + push + CI green | ❌ not done |
| Tag + published release | ❌ |
| AppImage via CI or deb-only notes | ⚠️ policy only |
| Dual-stack 422 documented or fixed | ⚠️ open |

**No-go for “published latest” until ADV-08 closed.**  
**Go for “developer RC candidate tree”** if team accepts ADV-01 as documented residual for 1.2.2 and schedules fix for 1.2.3.

---

## Recommended response (ordered)

1. **Commit** 1.2.2 tree (exclude screenshots / `rust_out`).  
2. **Push**; wait for CI green.  
3. Either **(a)** quick ADV-01 fix in same release, or **(b)** add SECURITY residual bullet and ship 1.2.2, patch 1.2.3.  
4. Tag **v1.2.2**; publish; close drafts 1.2.0/1.2.1.  
5. Optional: Python validation → NetRailError mapper (ADV-04).

---

## Evidence appendix (commands run)

```text
pytest → 37 passed
cargo clippy --all-targets -- -D warnings → pass
cargo test → 48 passed
Python validate_open_url matrix → ADV-01 ALLOW cases
API probes → 422 empty query (Python); open RFC1918 200
Second netrail-api → bind EADDRINUSE
dpkg-deb static assets → present
gh release list → Latest v1.1.1; 1.2.x drafts
```

---

*Adversarial Q&A audit — NetRail 1.2.2 working tree — 2026-07-12*
