# NetRail v1.3.0 — Wave 2 security & dual-stack parity

**Date:** 2026-07-31  
**Type:** Minor (security hardening + API/error contract parity)

## Highlights

| Area | Change |
|------|--------|
| Open-URL | IPv4-mapped IPv6 unmap; cloud metadata hostnames blocked |
| Backend URL | `fd00:ec2::254` IMDS fixed on Rust; metadata hostnames blocked |
| HTTP clients | No redirects for SearXNG/Brave (Rust + Python) |
| History | Python encrypt-no-key degrades like Rust + health banner |
| Fanout | Python 20s overall deadline (matches Rust) |
| Search mode | Invalid mode → `QUERY_INVALID` 400 on Rust (no silent default) |
| Errors | Rust `detail` is bare message (Python parity); collection codes mapped |
| Collections | Notes max 2000 on Rust; typed open browser failure on Python |

## Verify

```bash
bash scripts/check-versions.sh
source .venv/bin/activate && pytest tests/ -q
cd src-tauri && cargo clippy --all-targets -- -D warnings && cargo test
```

## References

- [CHANGELOG.md](../CHANGELOG.md)
- [AUDIT_ENTERPRISE_2026-07-31.md](AUDIT_ENTERPRISE_2026-07-31.md) Wave 2
- [API_ERRORS.md](API_ERRORS.md)
