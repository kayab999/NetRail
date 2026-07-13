# NetRail v1.2.2 — RC Hygiene

Patch release for **release-candidate readiness**: restore green CI, align version strings, and harden the release pipeline so lint failures cannot ship.

## Highlights

- **CI green** — clippy fix for intentional tray icon retention (`TrayState`)
- **Single version** — Rust, Tauri, npm, Python, README, and MANUAL all report **1.2.2**
- **Release gate** — `release.yml` now runs the same clippy `-D warnings` check as CI
- **Security policy** — supported versions include 1.2.x
- **Open-URL hardening** — encoded loopback + private IP blocks
- **Local rate limits** — abuse protection on search/open (disable with `NETRAIL_RATE_LIMIT=0`)
- **A11y / privacy UX** — skip link, ARIA tabs, no-referrer image thumbs, version in footer

## Downloads

| File | Use case |
|------|----------|
| `NetRail_1.2.2_amd64.AppImage` | Desktop app (prefer CI-built) |
| `NetRail_1.2.2_amd64.deb` | Debian/Ubuntu package |
| `NetRail-1.2.2-1.x86_64.rpm` | Fedora/RHEL-style package |
| `netrail-api` | Headless API server |
| `SHA256SUMS` | Verify integrity |

Local packaging smoke (2026-07-12): `.deb` includes `usr/share/netrail/static/{index.html,app.js,style.css}`; `netrail-api` health reports `1.2.2`. AppImage may require CI `linuxdeploy` environment.

```bash
chmod +x NetRail_1.2.2_amd64.AppImage
# Ubuntu 24.04+ without FUSE:
APPIMAGE_EXTRACT_AND_RUN=1 ./NetRail_1.2.2_amd64.AppImage
./NetRail_1.2.2_amd64.AppImage

sudo dpkg -i NetRail_1.2.2_amd64.deb
```

## Since v1.2.1

No breaking API changes. Also closes Python/Docker parity gaps from the RC audit:

- Wikipedia OpenSearch fallback when fanout is empty
- `search_recovery` on Python `/api/health`
- Empty backend batches reported in `errors[]`

## RC notes

- Preferred install path: **Rust desktop** (AppImage / `.deb`) or **`netrail-api`**
- Python/Docker path now matches Rust recovery behavior for empty fanout
- Full audit: [AUDIT_RC_2026-07-12.md](AUDIT_RC_2026-07-12.md)

## Smoke checklist

- [ ] `GET /api/health` → `status=ok`, `version` is `1.2.2`
- [ ] `GET /` and `/static/app.js` return 200
- [ ] Search returns results or Wikipedia + visible recovery/errors
- [ ] Open localhost URL rejected
- [ ] Tray Show / Quit on Linux
