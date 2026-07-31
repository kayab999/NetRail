# NetRail v1.2.3 — Security + docs truth

**Date:** 2026-07-31  
**Type:** Patch (Wave 0 docs + Wave 1 open-URL/env hardening)

## Highlights

### Wave 0 — Documentation truth

- README no longer claims search queries stay on loopback; clarifies API bind vs backend egress
- MANUAL copy-URL shortcut corrected to **Ctrl+C**
- Architecture / viability / continuity freeze / adversarial Q&A stamped current vs historical
- `install.sh` no longer labels the Tauri binary as “v1.0”
- Troubleshooting notes for local rate limits, Wikipedia egress, and private-open policy

### Wave 1 — Security P1

- Unified DuckDuckGo host set for open validation and merge resolve (`duckduckgo.com`, `duck.com` + subdomains)
- DNS rebinding helpers blocked at **apex** and subdomain (`localtest.me`, `nip.io`, `sslip.io`, `xip.io`)
- `NETRAIL_SEARXNG_URL` / `SEARXNG_URL` must pass backend URL policy (same as settings save)
- Shared golden vectors: `tests/fixtures/url_policy.json` (Python + Rust)

## Verify

```bash
bash scripts/check-versions.sh
source .venv/bin/activate && pytest tests/ -q
cd src-tauri && cargo clippy --all-targets -- -D warnings && cargo test
```

## References

- [CHANGELOG.md](../CHANGELOG.md)
- [AUDIT_ENTERPRISE_2026-07-31.md](AUDIT_ENTERPRISE_2026-07-31.md)
- [SECURITY.md](../SECURITY.md)
