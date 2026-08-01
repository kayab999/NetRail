# NetRail Distribution Guide (v1.2)

NetRail ships as a **Rust/Tauri desktop app** and a **headless `netrail-api` binary**. Python remains for **Docker, Flatpak, tests, and install.sh fallback** — not the primary production path.

## Support policy (dual-stack)

| Path | Support | Notes |
|------|---------|--------|
| **Rust desktop** (AppImage / `.deb` / `.rpm`) | **Production** | Full UI + tray + fanout + Wikipedia fallback |
| **`netrail-api`** (Rust headless) | **Production** | Homelab, scripting, CI smoke |
| **Python** (`python -m netrail`, Docker, Flatpak) | **Compatibility** | API parity targeted; prefer Rust when packaging allows |

### Feature parity matrix (1.2.2)

| Feature | Rust | Python |
|---------|------|--------|
| `/api/search` fanout + merge | ✅ | ✅ |
| Wikipedia empty-fanout fallback | ✅ | ✅ |
| Typed `{code,detail,status}` | ✅ | ✅ |
| Open-URL private + encoded loopback | ✅ | ✅ |
| `search_recovery` on health | ✅ | ✅ |
| Rate limits | ✅ | ✅ |
| Tauri tray / global shortcut | ✅ | — |

| Format | Best for | Browser open | History encryption |
|--------|----------|--------------|-------------------|
| **Tauri AppImage / .deb** | Desktop users (primary) | Native | OS keyring |
| **netrail-api** | Headless / homelab / scripting | N/A (API only) | OS keyring or `NETRAIL_DB_KEY` |
| **install.sh** | Developers / git clone | Native | OS keyring |
| **Flatpak** | Sandboxed desktop (Python path) | `flatpak-spawn --host` | OS keyring |
| **Docker** | Headless API (Python path) | N/A | `NETRAIL_DB_KEY` env var |

---

## Quick install (60 seconds)

```bash
git clone git@github.com:kayab999/NetRail.git NetRail && cd NetRail
chmod +x install.sh && ./install.sh
netrail-launch
```

Your browser opens to `http://127.0.0.1:7421` automatically.

---

## Tauri desktop (recommended)

### From GitHub Release

Download the AppImage or `.deb` from the [Releases](https://github.com/kayab999/NetRail/releases) page. Verify checksums against `SHA256SUMS` in the release assets, and verify the sigstore (keyless) signature before installing:

```bash
sha256sum -c SHA256SUMS
cosign verify-blob --certificate SHA256SUMS.pem --signature SHA256SUMS.sig SHA256SUMS
```

```bash
chmod +x NetRail_*_amd64.AppImage
APPIMAGE_EXTRACT_AND_RUN=1 ./NetRail_*_amd64.AppImage
```

### Build locally

```bash
# AppImage bundling needs patchelf (also installed in GitHub Release CI):
#   sudo apt install patchelf
npm ci
cd src-tauri && cargo build --release --bin netrail-api --no-default-features && cd ..
APPIMAGE_EXTRACT_AND_RUN=1 npm run build
```

Artifacts land in `src-tauri/target/release/bundle/` (`.deb`, `.rpm`, AppImage when `patchelf` is present).

If AppImage fails with `Could not find patchelf`, install `patchelf` or use the **`.deb`** / CI-built AppImage from the GitHub Release.

**Release CI** fails the job if AppImage or `.deb` is missing after `tauri build` (AppImage is a required ship artifact). Local machines without `patchelf` should still ship `.deb` / `netrail-api`.

**Smoke / E2E (API):** after building the headless binary:

```bash
bash scripts/e2e-api-smoke.sh
# or: bash scripts/package-smoke.sh
```

---

## Headless API (`netrail-api`)

The release ships a ~7MB static binary with no Tauri/GTK dependency:

```bash
./netrail-api
curl -s http://127.0.0.1:7421/api/health | jq
```

Set `NETRAIL_DB_KEY` when running without an OS keyring (Docker, CI, remote servers).

---

## Flatpak (Python stack)

### Build

```bash
flatpak install flathub org.freedesktop.Platform//23.08 org.freedesktop.Sdk//23.08
flatpak install flathub org.freedesktop.Sdk.Extension.python3//23.08
chmod +x packaging/flatpak/build.sh
./packaging/flatpak/build.sh
```

### Install

```bash
flatpak install --bundle build/flatpak/NetRail.flatpak
flatpak run io.netrail.NetRail
```

### Sandbox notes

- Metasearch requires `--share=network`
- Fernet keys use `--talk-name=org.freedesktop.secrets`
- Browser launches route through `flatpak-spawn --host`
- Desktop entries are read from `/usr/share/applications` (read-only mount)

---

## Docker (Python stack)

### Generate encryption key

```bash
python -c "from cryptography.fernet import Fernet; print(Fernet.generate_key().decode())"
```

Copy to `.env`:

```bash
cp .env.example .env
# edit NETRAIL_DB_KEY=...
```

### Run API only

```bash
# Python compatibility image (default service)
docker compose up -d netrail
curl -s http://127.0.0.1:7421/api/health

# Preferred production path: Rust netrail-api
docker compose --profile rust up -d netrail-rust
```

### Run with SearXNG profile

```bash
docker compose --profile searxng up -d
```

Set in `.env`:

```
SEARXNG_URL=http://searxng:8080
NETRAIL_DB_KEY=...   # required for encrypted history
NETRAIL_API_TOKEN=...  # recommended for Docker: guards against other containers/processes
NETRAIL_STRICT_BACKEND_URLS=1  # recommended: forbid private/loopback backend URLs
NETRAIL_AUDIT_LOG=1            # optional: JSONL audit of search/open/settings/history
```

### Security warning

The compose file binds `127.0.0.1:7421:7421`. **Do not** change this to `7421:7421` unless you intend to expose NetRail to your entire LAN (then set `NETRAIL_API_TOKEN`).

Docker has no OS keyring — `NETRAIL_DB_KEY` is **required** for encrypted history.

Build Rust image directly: `docker build -f Dockerfile.rust -t netrail-api .`

---

## Environment variables

| Variable | Purpose |
|----------|---------|
| `NETRAIL_DB_PATH` | SQLite database location |
| `NETRAIL_DB_KEY` | Fernet key (Docker / headless) |
| `NETRAIL_STATIC_DIR` | Directory containing `index.html` / UI assets |
| `NETRAIL_AUTO_OPEN` | Open browser on start (`true`/`false`) |
| `NETRAIL_RATE_LIMIT` | `0` / `false` disables the per-identity 90/120/60 per-minute caps (defaults; when a token is configured, limits apply per token identity, otherwise to the anonymous bucket) |
| `NETRAIL_API_TOKEN` | Optional API token; require Bearer / `X-NetRail-Token` on `/api/*` (except health) |
| `NETRAIL_INJECT_UI_TOKEN` | When token set, inject into served HTML for UI (default on). Note: the injected page (`/`) is unauthenticated, so any local HTTP client can read the token from it — see SECURITY.md. Set `0` and supply via `localStorage` if you need tighter behavior |
| `NETRAIL_STRICT_BACKEND_URLS` | `1` rejects private/loopback SearXNG/backend URLs |
| `NETRAIL_AUDIT_LOG` | `1` appends JSON lines to XDG data `netrail/audit.log` |
| `NETRAIL_AUDIT_LOG_PATH` | Explicit audit log path |
| `NETRAIL_AUDIT_MAX_BYTES` | Audit rotation size cap (default 10 MiB); on write overflow the log is shifted to `<path>.1` (`.2`, …) |
| `NETRAIL_AUDIT_MAX_FILES` | Max rotated audit files kept (default 3; `0` disables rotation) |
| `NETRAIL_LOG_JSON` | `1` emits structured JSON logs (tracing-subscriber `json`) instead of plain text |
| `SEARXNG_URL` / `NETRAIL_SEARXNG_URL` | Self-hosted SearXNG base URL |
| `BRAVE_SEARCH_API_KEY` / `NETRAIL_BRAVE_API_KEY` | Brave Search API key (never stored in settings) |
| `NETRAIL_SEARCH_STRATEGY` | `fanout` or `fallback` |
| `NETRAIL_HISTORY_ENABLED` | Enable/disable history |
| `NETRAIL_HISTORY_ENCRYPT` | Field encryption on/off |
| `NETRAIL_HISTORY_TTL_DAYS` | Auto-purge age |
| `NETRAIL_MAX_RESULTS` | Default result cap (1–50) |

---

## Desktop integration

- Icon: `assets/netrail.png` (128px installed by `install.sh`)
- Desktop entry: `assets/netrail.desktop` (`Terminal=false`)
- Data: `~/.local/share/netrail/netrail.db`
- Config: `~/.config/netrail/settings.json`

Flatpak uses XDG paths under `~/.var/app/io.netrail.NetRail/`.

---

## Flatpak sandbox troubleshooting

| Symptom | Fix |
|---------|-----|
| Open button does nothing | Confirm `flatpak-spawn` in PATH inside sandbox; check session-bus |
| No browsers listed | Verify `/usr/share/applications` mount; install a browser on the **host** |
| History won't encrypt | Grant `org.freedesktop.secrets` talk permission; or set `NETRAIL_DB_KEY` |
| SearXNG unreachable | Use full URL in settings; for Docker use service hostname |

---

*NetRail v1.4.0 — dual-stack policy: Rust production · Python compatibility · optional API token / audit log*