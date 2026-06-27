# NetRail — User Manual

## Introduction

NetRail is a local research console for professionals who want to search the web without surrendering control to a surveillance-driven browser tab. It is inspired by the OG Web Ferret workflow: **query → link list → you choose what to open**.

NetRail does not replace your browser. It sits in front of it. Results appear in a slick local interface. Links open only when you click **Open** — in the browser you select, optionally in private or incognito mode.

**What NetRail is not:** a Google account, a cloud history service, or an AI chatbot that answers instead of showing sources.

---

## Launching NetRail

### Graphical use (v0.1)

1. Start the server:
   ```bash
   cd NetRail
   ./run.sh
   ```
2. Open **http://127.0.0.1:7421** in any browser.
3. Use the search bar and link rail. Your chosen browser is only used when you open a result.

> **Note:** v0.1 requires a browser to view the NetRail UI itself. A native desktop shell (Tauri) is planned for v1.0 so NetRail becomes fully self-contained.

### Stopping NetRail

Press **Ctrl+C** in the terminal where `./run.sh` or `python -m netrail` is running.

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

### What stays local

- Search queries as typed in the UI (not logged to disk in v0.1)
- Browser and privacy preferences (`~/.config/netrail/`)
- The link rail rendering and UI state

### What leaves your machine

- **Metasearch requests** — NetRail contacts public search providers through the `ddgs` library to retrieve results. This is the same class of traffic as typing a query into a search engine, but without Google accounts, cookies, or NetRail telemetry layered on top.
- **Browser navigation** — Only when **you** click Open.

### What NetRail never does

- No analytics SDKs
- No crash reporters that phone home
- No accounts or cloud history
- No ads
- No binding to `0.0.0.0` (LAN exposure) in v0.1

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

> **Modular integrations:** External tools (e.g. network diagnostics suites) may call this API without importing NetRail code. See [ARCHITECTURE.md](ARCHITECTURE.md#modular-integration-boundary).

---

## Troubleshooting

| Symptom | Likely cause | Solution |
|---------|--------------|----------|
| **Page won't load** | Server not running | Run `./run.sh` or `python -m netrail` |
| **Search failed (502)** | Network/DNS issue or provider timeout | Check internet connectivity; retry. If using NetMedic or similar tools, run network diagnostics separately. |
| **No browsers listed** | No `.desktop` browser entries found | Install a browser; ensure it has a Freedesktop entry |
| **Open does nothing** | Browser binary moved or permissions | Re-select browser in dropdown; verify `which firefox` (or your browser) works |
| **Private mode ignored** | Browser lacks known private flag | Browser opens in normal mode; try Firefox or Chromium |
| **Few or no results** | Provider rate limit or query too narrow | Simplify query; wait and retry |
| **Port already in use** | Another NetRail instance on 7421 | Stop the other process or change port in `netrail/main.py` (advanced) |

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
| Focus search bar | Click or tab to `#query` (browser-dependent) |
| Submit search | **Enter** in the search field |
| Switch mode | Click **Web** or **Images** tab |

Native keyboard shortcuts (Ctrl+K focus, Ctrl+Enter open, etc.) are planned for the Tauri desktop shell.

---

## Updating

```bash
cd NetRail
git pull
source .venv/bin/activate   # if using venv
pip install -r requirements.txt
```

Review [CHANGELOG.md](../CHANGELOG.md) when available for breaking changes.

---

## Getting Help & Contributing

- **Manifesto:** [OPEN_LETTER.md](../OPEN_LETTER.md)
- **Architecture:** [ARCHITECTURE.md](ARCHITECTURE.md)
- **License:** AGPL-3.0 — contributions welcome; fork-friendly by design

---

*NetRail contributors — 2026*