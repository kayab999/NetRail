# Security Policy

## Supported versions

| Version | Supported |
|---------|-----------|
| 1.6.x   | Yes (current: 1.6.6) |
| 1.5.x   | Yes (security fixes) |
| 1.4.x   | Yes (security fixes) |
| 1.3.x   | Yes (security fixes) |
| 1.2.x   | Yes (incl. 1.2.3) |
| 1.1.x   | Yes (security fixes) |
| 1.0.x   | Yes (security fixes) |
| < 1.0   | No        |

## Threat model (summary)

NetRail is a **localhost-only** research console. The API binds to `127.0.0.1:7421` and has **no authentication**. Any process on your machine can call it. Do not expose port 7421 to your LAN or the public internet.

NetRail protects you from **cloud surveillance and careless open-from-results** behavior. It does **not** protect against malware already running as your user on the same machine.

Report issues that break this model or enable remote exploitation without explicit user configuration.

## What open-URL validation blocks

Search results and `/api/open` reject:

- Non-`http`/`https` schemes (`javascript:`, `data:`, `file:`, …)
- Embedded credentials
- Loopback / localhost (including **decimal, hex, octal, short IPv4, and FQDN-root trailing-dot** forms browsers may resolve to `127.0.0.1`, e.g. `2130706433`, `0x7f000001`, `127.1`, `127.0.0.1.`)
- Link-local and **private / non-public** addresses (RFC1918, ULA, multicast, …)
- Known DNS-rebinding helper domains — apex and subdomains (`nip.io`, `sslip.io`, `xip.io`, `localtest.me`)
- Cloud metadata hostnames (`metadata.google.internal`, `metadata`, `instance-data`) and IMDS IPs (`169.254.169.254`, `fd00:ec2::254`)
- IPv4-mapped IPv6 forms (`::ffff:127.0.0.1`, etc.) after unmap
- DuckDuckGo redirect wrappers (`duckduckgo.com`, `duck.com`, and subdomains) are unwrapped via `uddg=` before checks

Backend HTTP clients **do not follow redirects**, so a SearXNG/Brave hop cannot bounce NetRail onto private targets via 30x.

**Backend URLs** (e.g. self-hosted SearXNG) still **allow** localhost and private LAN hosts so operators can point at home instances. Cloud metadata and rebinding hostnames remain blocked.

**Fetch-time re-validation (A-05):** save-time URL checks are reapplied at **fetch time** in both stacks. Each SearXNG search/health call resolves the hostname and evaluates every resolved IP: cloud metadata (`169.254.169.254`, `fd00:ec2::254`) and link-local/unspecified are always blocked; other private/loopback targets are blocked only under `strict_backend_urls`; an empty resolution fails closed (`BACKEND_URL_DNS_UNRESOLVABLE`). IP-literal backends never go through DNS. This closes the TOCTOU gap where a backend hostname could be re-pointed after validation.

## Read-only mode

`NETRAIL_READONLY=1` rejects **administrative mutations** with HTTP `403 READONLY_MODE`:

- `PUT /api/settings`
- history delete / purge
- collection create / add-item

**By design, search and open still record local history/visits** so the console remains useful as a kiosk/archive viewer with an intact local audit trail. Read endpoints (settings GET, history list, collections list/export, docs, health) keep working. See `docs/API_ERRORS.md` and `docs/DISTRIBUTION.md`.

## Optional API token

`NETRAIL_API_TOKEN` requires `Authorization: Bearer …` or `X-NetRail-Token: …` on `/api/*` (health is exempt). It is a guard against **accidental cross-process access** (other users' processes, browser extensions, containers sharing the loopback) — **not** a defense against malware already running as your user.

Important tradeoff: when the token is set, `NETRAIL_INJECT_UI_TOKEN` (default **on**) injects the token into the HTML of the **unauthenticated** `/` page so the web UI can authenticate. Any local HTTP client can `GET http://127.0.0.1:7421/` and read it. Since same-user malware can read `NETRAIL_API_TOKEN` from the environment anyway, this does not weaken the threat model — but do not treat the token as a secret that survives local readers. For Docker/multi-process hosts, set `NETRAIL_INJECT_UI_TOKEN=0` and supply the token to the UI via `localStorage` only if you understand the consequence (the UI cannot authenticate until you do).

## History encryption

- Query text and result titles/snippets are encrypted with Fernet when a key is available (`NETRAIL_DB_KEY` or OS keyring).
- The FTS5 index stores **plaintext tokens** of queries (required for local search).
- If encryption is enabled but the keyring is unavailable (WSL, some window managers, headless), NetRail **degrades** to unencrypted history for the session and shows a **security banner** (Rust and Python). Prefer setting `NETRAIL_DB_KEY` in those environments.
- `/api/health` reports a canonical `history.encryption_state` (`encrypted` / `degraded` / `plaintext`) derived from `encrypt_requested` + `encryption_active`; the web UI footer shows the same state as a chip. Setting changes via `PUT /api/settings` take effect immediately — the store rebinds on the next access (settings directivity, A-11).

## Reporting a vulnerability

1. **Do not** open a public GitHub issue for exploitable security bugs.
2. Email the maintainer via the contact on [github.com/kayab999](https://github.com/kayab999) or open a private security advisory on the repository once it exists.
3. Include: affected version, reproduction steps, impact, and suggested fix if you have one.

We aim to acknowledge reports within **72 hours** and ship fixes for confirmed issues on supported versions as soon as practical.

## Out of scope

- Metasearch provider rate limits, CAPTCHAs, or HTML layout changes (DDGS scraping)
- User-configured SearXNG instances on private networks (intentional for self-hosters)
- Lack of API token auth on localhost (documented design choice for v1.x)
- Remote image loads in Images mode (HTTPS thumbnails; privacy residual)

## Safe defaults

- URL open validation as above
- Backend URL validation blocks cloud metadata and rebinding hostnames
- CSP, `nosniff`, and `no-referrer` on API responses
- Local rate limits on search/open/mutations (90 / 120 / 60 per minute); set `NETRAIL_RATE_LIMIT=0` to disable
- Optional `NETRAIL_API_TOKEN` (Bearer / `X-NetRail-Token`) for Docker or multi-process hosts
- Optional `NETRAIL_STRICT_BACKEND_URLS` to forbid private/loopback SearXNG URLs
- Optional `NETRAIL_AUDIT_LOG` JSON lines for search/open/settings/history mutations
- Optional `NETRAIL_READONLY=1` to lock settings/history/collections mutations (search/visit logging stays active)
- Zero telemetry
- Image result thumbnails request `referrerpolicy=no-referrer`
