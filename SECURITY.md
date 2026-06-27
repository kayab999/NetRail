# Security Policy

## Supported versions

| Version | Supported |
|---------|-----------|
| 1.0.x   | Yes       |
| < 1.0   | No        |

## Threat model (summary)

NetRail is a **localhost-only** research console. The API binds to `127.0.0.1:7421` and has **no authentication**. Any process on your machine can call it. Do not expose port 7421 to your LAN or the public internet.

Report issues that break this model or enable remote exploitation without explicit user configuration.

## Reporting a vulnerability

1. **Do not** open a public GitHub issue for exploitable security bugs.
2. Email the maintainer via the contact on [github.com/kayab999](https://github.com/kayab999) or open a private security advisory on the repository once it exists.
3. Include: affected version, reproduction steps, impact, and suggested fix if you have one.

We aim to acknowledge reports within **72 hours** and ship fixes for confirmed issues on supported versions as soon as practical.

## Out of scope

- Metasearch provider rate limits, CAPTCHAs, or HTML layout changes (DDGS scraping)
- User-configured SearXNG instances reaching private network hosts (intentional for self-hosters)
- Lack of API token auth on localhost (documented design choice for v1.0)

## Safe defaults

- URL open validation blocks `javascript:`, `data:`, localhost, and DNS rebinding hosts in search results
- Backend URL validation blocks cloud metadata and rebinding hostnames
- History encryption fails closed when no key is available
- CSP, `nosniff`, and `no-referrer` on API responses