# NetRail Distribution Guide (v1.1)

NetRail ships as a Rust/Tauri desktop app and a headless API binary. Python sources remain for development parity and legacy packaging paths.

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

Download the AppImage or `.deb` from the [Releases](https://github.com/kayab999/NetRail/releases) page. Verify checksums against `SHA256SUMS` in the release assets.

```bash
chmod +x NetRail_*_amd64.AppImage
APPIMAGE_EXTRACT_AND_RUN=1 ./NetRail_*_amd64.AppImage
```

### Build locally

```bash
npm ci
cd src-tauri && cargo build --release --bin netrail-api --no-default-features && cd ..
APPIMAGE_EXTRACT_AND_RUN=1 npm run build
```

Artifacts land in `src-tauri/target/release/bundle/`.

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
docker compose up -d netrail
curl -s http://127.0.0.1:7421/api/health
```

### Run with SearXNG profile

```bash
docker compose --profile searxng up -d
```

Set in `.env`:

```
SEARXNG_URL=http://searxng:8080
```

### Security warning

The compose file binds `127.0.0.1:7421:7421`. **Do not** change this to `7421:7421` unless you intend to expose NetRail to your entire LAN.

Docker has no OS keyring — `NETRAIL_DB_KEY` is **required** for encrypted history.

---

## Environment variables

| Variable | Purpose |
|----------|---------|
| `NETRAIL_DB_PATH` | SQLite database location |
| `NETRAIL_DB_KEY` | Fernet key (Docker / headless) |
| `NETRAIL_AUTO_OPEN` | Open browser on start (`true`/`false`) |
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

*NetRail v1.1.0 — The Sovereign Search Console*