# NetRail — User Manual

## Introduction

NetRail is a local research console for professionals who want to search the web without surrendering control to a surveillance-driven browser tab. It is inspired by the OG Web Ferret workflow: **query → link list → you choose what to open**.

NetRail does not replace your browser. It sits in front of it. Results appear in a slick local interface. Links open only when you click **Open** — in the browser you select, optionally in private or incognito mode.

**What NetRail is not:** a Google account, a cloud history service, or an AI chatbot that answers instead of showing sources.

---

## Launching NetRail

### Desktop app (v1.1 — recommended)

Download the AppImage or `.deb` from [GitHub Releases](https://github.com/kayab999/NetRail/releases/latest), or build from source:

```bash
npm install && npm run build
./src-tauri/target/release/netrail
```

The Tauri shell embeds the UI and starts the Rust API on `127.0.0.1:7421`. Use the system tray or `Ctrl+Shift+S` to focus the window.

On Ubuntu 24.04 without FUSE:

```bash
APPIMAGE_EXTRACT_AND_RUN=1 ./NetRail_1.1.0_amd64.AppImage
```

### Headless API

```bash
./netrail-api --api-only
```

Open **http://127.0.0.1:7421** in any browser if you prefer the web UI without the Tauri shell.

### Python fallback

```bash
./run.sh
# or: python -m netrail
```

### Help, About, and Donate

From the desktop app or web UI header:

- **Help → User Manual** — this document, rendered in-app
- **Help → About NetRail** — project README and version highlights
- **☕ Donate** — opens [buymeacoffee.com/kayabsoftware](https://buymeacoffee.com/kayabsoftware) in your configured browser

The native Tauri menu bar also exposes **Help** and **Donate…** with the same actions.

### Stopping NetRail

Close the Tauri window (it hides to the tray; API keeps running), use **Quit** from the tray menu to exit fully, or press **Ctrl+C** in the terminal for headless/Python mode.

### Headless / scripting use

NetRail exposes a local REST API on `127.0.0.1:7421`. See [API usage](#api-usage-for-power-users) below.

---

## The Link Rail Workflow

NetRail follows a deliberate three-step workflow:

```
┌──────────────┐     ┌──────────────┐     ┌──────────────┐
│  1. Query    │ ──▶ │  2. Review   │ ──▶ │  3. Open     │
│  (operators) │     │  (link rail) │     │  (your pick) │
└──────────────┘     └──────────────┘     └──────────────┘
```

1. **Query** — Type your search. Use operators for precision (see below).
2. **Review** — Scan titles, URLs, and snippets (or image thumbnails). Nothing leaves your machine except the metasearch request itself.
3. **Open** — Click **Open** or the result title. NetRail launches your chosen browser. Private mode is respected when enabled.

This is the core difference from default browser search: **you are not dropped into an engagement-optimized results page**. You get a rail of links and retain agency.

---

## Search Modes

### Web

Default mode. Returns titles, URLs, and text snippets. Best for research, documentation, news, and general discovery.

### Images

Switch to the **Images** tab before searching. Returns thumbnail previews with source URLs. Click **Open** to visit the page hosting the image.

---

## Search Operators

NetRail passes operators through to the underlying metasearch layer. These work in both Web and Images modes:

| Operator | Example | Effect |
|----------|---------|--------|
| `site:` | `site:gov climate policy` | Restrict to a domain or TLD |
| `filetype:` | `filetype:pdf battery regulations` | Prefer specific file types |
| `intitle:` | `intitle:CVE-2024` | Terms must appear in page title |
| `inurl:` | `inurl:documentation api` | Terms must appear in URL |
| `"phrase"` | `"semantic versioning"` | Exact phrase match |
| `-term` | `python -snake` | Exclude a term |
| `OR` | `debian OR ubuntu` | Either term (where supported) |

### Example queries for professionals

| Use case | Query |
|----------|-------|
| Government PDFs | `site:gov filetype:pdf "export controls"` |
| Academic sources | `site:edu OR site:arxiv.org transformer architecture` |
| Recent documentation | `site:docs.python.org intitle:asyncio` |
| Exclude junk | `"network security" -pinterest -facebook` |
| Image reference | `site:wikimedia.org cathedral floor plan` *(Images tab)* |

Operators are hints to the search provider, not guarantees. Different backends may interpret them differently. Future versions will show which backend honored each operator.

---

## Browser Settings

### Browser picker

The **Browser** dropdown lists web browsers detected on your system (via `.desktop` entries and known binary names). Supported private-mode flags include:

| Browser family | Private flag |
|----------------|--------------|
| Firefox, Waterfox, LibreWolf | `--private-window` |
| Chrome, Chromium, Brave, Vivaldi | `--incognito` |
| Microsoft Edge | `--inprivate` |
| Opera | `--private` |

If a browser does not support a known private flag, NetRail still opens it in normal mode.

### Private / incognito mode

Enable **Private / incognito** before opening links. When active:

- The **Open** button label changes to **Open private**.
- Each link opens in a new private window/tab (browser-dependent).
- Your choice is saved to settings automatically.

### Settings persistence

Preferences are stored locally at:

```
~/.config/netrail/settings.json
```

| Setting | Description | Default |
|---------|-------------|---------|
| `browser_id` | Selected browser identifier | First detected browser |
| `private_mode` | Open links in private/incognito | `false` |
| `max_results` | Results per query (1–50) | `25` |

NetRail never syncs these settings to the cloud. There is no account system.

---

## Privacy Model

### Threat model & encryption boundaries

NetRail is a **single-user localhost tool**. The API on `127.0.0.1:7421` has no authentication — any process on your machine can call it.

**Encrypted at rest** (when `history_encrypt` is enabled and a keyring key exists):

- Query text blobs, result titles, and snippets (Fernet)

**Plaintext by design:**

- FTS5 index tokens (SQLite cannot full-text search encrypted blobs)
- Visited URLs and collection URLs (needed for re-open and deduplication)

NetRail protects you from **cloud surveillance**, not from malware or untrusted local users. Use full-disk encryption (LUKS/FileVault) for that threat model.

### What stays local

- Browser and privacy preferences (`~/.config/netrail/`)
- Search history and collections (see encryption boundaries above)
- The link rail rendering and UI state

### What leaves your machine

- **Metasearch requests** — NetRail contacts public search providers through the `ddgs` library to retrieve results. This is the same class of traffic as typing a query into a search engine, but without Google accounts, cookies, or NetRail telemetry layered on top.
- **Browser navigation** — Only when **you** click Open.

### What NetRail never does

- No analytics SDKs
- No crash reporters that phone home
- No accounts or cloud history
- No ads
- No binding to `0.0.0.0` (LAN exposure) in v1.0

The health endpoint explicitly reports `"telemetry": "none"`.

---

## API Usage for Power Users

All endpoints are local only.

### Health check

```bash
curl -s http://127.0.0.1:7421/api/health
```

### Search

```bash
curl -s -X POST http://127.0.0.1:7421/api/search \
  -H 'Content-Type: application/json' \
  -d '{
    "query": "site:europa.eu filetype:pdf batteries",
    "mode": "web",
    "max_results": 10
  }'
```

### Open a link

```bash
curl -s -X POST http://127.0.0.1:7421/api/open \
  -H 'Content-Type: application/json' \
  -d '{
    "url": "https://example.com",
    "browser_id": "firefox",
    "private_mode": true
  }'
```

### List browsers

```bash
curl -s http://127.0.0.1:7421/api/browsers
```

### Read / write settings

```bash
curl -s http://127.0.0.1:7421/api/settings
curl -s -X PUT http://127.0.0.1:7421/api/settings \
  -H 'Content-Type: application/json' \
  -d '{"browser_id":"brave-browser","private_mode":true,"max_results":25}'
```

### History

```bash
curl -s 'http://127.0.0.1:7421/api/history?limit=50'
curl -s 'http://127.0.0.1:7421/api/history?q=battery&limit=20'
curl -s -X DELETE http://127.0.0.1:7421/api/history/42
curl -s -X DELETE http://127.0.0.1:7421/api/history
```

### Collections

```bash
curl -s http://127.0.0.1:7421/api/collections
curl -s -X POST http://127.0.0.1:7421/api/collections \
  -H 'Content-Type: application/json' \
  -d '{"name":"OSINT corpus"}'
curl -s -X POST http://127.0.0.1:7421/api/collections/1/items \
  -H 'Content-Type: application/json' \
  -d '{"url":"https://example.com","title":"Example","notes":"why it matters"}'
```

### In-app documentation

```bash
curl -s http://127.0.0.1:7421/api/docs/manual
curl -s http://127.0.0.1:7421/api/docs/about
```

> **Modular integrations:** External tools may call this API without importing NetRail code. See [ARCHITECTURE.md](ARCHITECTURE.md#modular-integration-boundary).

---

## Troubleshooting

| Symptom | Likely cause | Solution |
|---------|--------------|----------|
| **Page won't load** | Server not running | Launch the Tauri app, run `./netrail-api`, or use `./run.sh` / `python -m netrail` |
| **Search failed (502)** | Network/DNS issue, provider timeout, or `FANOUT_TOTAL_FAILURE` | Check internet connectivity; retry. Inspect API `code` field — see [API error codes](API_ERRORS.md). |
| **No browsers listed** | No `.desktop` browser entries found | Install a browser; ensure it has a Freedesktop entry |
| **Open does nothing** | Browser binary moved or permissions | Re-select browser in dropdown; verify `which firefox` (or your browser) works |
| **Private mode ignored** | Browser lacks known private flag | Browser opens in normal mode; try Firefox or Chromium |
| **Few or no results** | Provider rate limit or query too narrow | Simplify query; wait and retry |
| **Port already in use** | Another NetRail instance on 7421 | Stop the other process (`ss -tlnp | grep 7421`) |

### Search failed and your network

A 502 error often means NetRail could not reach a metasearch provider — not that NetRail itself is broken. Confirm basic connectivity:

```bash
curl -Is https://1.1.1.1
getent hosts example.com
```

NetRail is intentionally modular: network repair tools are a separate concern. Optional future versions may offer a "check connectivity" shortcut via documented APIs.

---

## Keyboard Tips

| Action | Shortcut |
|--------|----------|
| Focus NetRail window (Tauri) | **Ctrl+Shift+S** |
| Submit search | **Enter** in the search field |
| Move through results | **↑** / **↓** (when results are shown) |
| Open highlighted result | **Enter** (with result highlighted) |
| Open highlighted in private mode | **Shift+Enter** |
| Copy highlighted URL | **Ctrl+Shift+C** (with search field focused) |
| Export results | **Export** button (JSON); **Shift+click** for CSV |
| Switch mode | Click **Web**, **Images**, or **History** tab |

---

## Updating

```bash
cd NetRail
git pull

# Desktop (recommended)
npm ci && npm run build

# Headless API
cd src-tauri && cargo build --release --bin netrail-api --no-default-features

# Python fallback / tests
source .venv/bin/activate
pip install -r requirements.txt
```

Review [CHANGELOG.md](../CHANGELOG.md) for breaking changes.

---

## Getting Help & Contributing

- **In-app:** **Help → User Manual** / **About NetRail**
- **Repository:** [github.com/kayab999/NetRail](https://github.com/kayab999/NetRail)
- **Security:** [SECURITY.md](../SECURITY.md)
- **Manifesto:** [OPEN_LETTER.md](../OPEN_LETTER.md)
- **Architecture:** [ARCHITECTURE.md](ARCHITECTURE.md)
- **Support development:** [buymeacoffee.com/kayabsoftware](https://buymeacoffee.com/kayabsoftware)
- **License:** AGPL-3.0 — contributions welcome; fork-friendly by design

---

*NetRail — maintained by [kayab999](https://github.com/kayab999) — 2026*