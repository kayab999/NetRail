# NetRail Distribution Guide (v0.4)

NetRail ships in four formats. Choose based on how you work.

| Format | Best for | Browser open | History encryption |
|--------|----------|--------------|-------------------|
| **install.sh** | Developers / git clone | Native | OS keyring |
| **Flatpak** | Desktop users (GNOME/KDE) | `flatpak-spawn --host` | OS keyring (`org.freedesktop.secrets`) |
| **AppImage** | Portable zero-install | Native (no sandbox) | OS keyring |
| **Docker** | Headless / homelab / API | N/A (API only) | `NETRAIL_DB_KEY` env var |

---

## Quick install (60 seconds)

```bash
git clone <repo-url> NetRail && cd NetRail
chmod +x install.sh && ./install.sh
netrail-launch
```

Your browser opens to `http://127.0.0.1:7421` automatically.

---

## Flatpak

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
- Browser launches route through `flatpak-spawn --host` (see `netrail/browsers.py`)
- Desktop entries are read from `/usr/share/applications` (read-only mount)

---

## Docker

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

## AppImage

```bash
pip install pyinstaller
chmod +x packaging/appimage/build.sh
./packaging/appimage/build.sh
chmod +x build/appimage/NetRail-x86_64.AppImage
./build/appimage/NetRail-x86_64.AppImage
```

AppImage runs outside Flatpak sandbox — browser integration works like a native install.

---

## Environment variables

| Variable | Purpose |
|----------|---------|
| `NETRAIL_DB_PATH` | SQLite database location |
| `NETRAIL_DB_KEY` | Fernet key (Docker / headless) |
| `NETRAIL_AUTO_OPEN` | Open browser on start (`true`/`false`) |
| `SEARXNG_URL` / `NETRAIL_SEARXNG_URL` | Self-hosted SearXNG base URL |
| `NETRAIL_HISTORY_ENABLED` | Enable/disable history |
| `NETRAIL_HISTORY_ENCRYPT` | Field encryption on/off |
| `NETRAIL_HISTORY_TTL_DAYS` | Auto-purge age |

---

## Desktop integration

- Icon: `assets/netrail.svg`
- Desktop entry: `assets/netrail.desktop`
- Data: `~/.local/share/netrail/netrail.db`
- Config: `~/.config/netrail/settings.json`

Flatpak uses XDG paths under `~/.var/app/io.netrail.NetRail/`.

---

## Flatpak sandbox troubleshooting

| Symptom | Fix |
|---------|-----|
| Open button does nothing | Confirm `flatpak-spawn` in PATH inside sandbox; check `io.netrail.NetRail` has session-bus |
| No browsers listed | Verify `/usr/share/applications` mount; install a browser on the **host** |
| History won't encrypt | Grant `org.freedesktop.secrets` talk permission; or set `NETRAIL_DB_KEY` |
| SearXNG unreachable | Use full URL in settings; for Docker use service hostname |

---

*NetRail v0.4 — Adoption phase*