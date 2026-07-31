# Security Policy

## Supported versions

| Version | Supported |
|---------|-----------|
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
- Loopback / localhost (including **decimal, hex, octal, and short IPv4 forms** browsers may resolve to `127.0.0.1`)
- Link-local and **private / non-public** addresses (RFC1918, ULA, multicast, …)
- Known DNS-rebinding helper domains — apex and subdomains (`nip.io`, `sslip.io`, `xip.io`, `localtest.me`)
- Cloud metadata hostnames (`metadata.google.internal`, `metadata`, `instance-data`) and IMDS IPs (`169.254.169.254`, `fd00:ec2::254`)
- IPv4-mapped IPv6 forms (`::ffff:127.0.0.1`, etc.) after unmap
- DuckDuckGo redirect wrappers (`duckduckgo.com`, `duck.com`, and subdomains) are unwrapped via `uddg=` before checks

Backend HTTP clients **do not follow redirects**, so a SearXNG/Brave hop cannot bounce NetRail onto private targets via 30x.

**Backend URLs** (e.g. self-hosted SearXNG) still **allow** localhost and private LAN hosts so operators can point at home instances. Cloud metadata and rebinding hostnames remain blocked.

## History encryption

- Query text and result titles/snippets are encrypted with Fernet when a key is available (`NETRAIL_DB_KEY` or OS keyring).
- The FTS5 index stores **plaintext tokens** of queries (required for local search).
- If encryption is enabled but the keyring is unavailable (WSL, some window managers, headless), NetRail **degrades** to unencrypted history for the session and shows a **security banner** (Rust and Python). Prefer setting `NETRAIL_DB_KEY` in those environments.

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
- Local rate limits on search/open (90 / 120 per minute); set `NETRAIL_RATE_LIMIT=0` to disable
- Zero telemetry
- Image result thumbnails request `referrerpolicy=no-referrer`
