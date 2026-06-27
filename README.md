# NetRail

**A local research console — inspired by Web Ferret, built for the post-big-tech web.**

NetRail is a privacy-first search front-end that runs entirely on your machine. Results appear in a clean link rail; **nothing opens in a browser until you choose**. No accounts. No analytics. No telemetry.

> This is an open letter in code: you do not need a surveillance company to find things on the internet.  
> Read the full manifesto in [OPEN_LETTER.md](OPEN_LETTER.md).

**Version:** 0.1.0 · **License:** [AGPL-3.0](LICENSE)

---

## Features (v0.1)

| Area | Capabilities |
|------|-------------|
| **Web search** | Metasearch with operator passthrough (`site:`, `filetype:`, `intitle:`, `"phrase"`, `-exclude`) |
| **Image search** | Separate image tab with thumbnail previews |
| **Link rail** | Results in-app; you decide what to open |
| **Browser control** | Detect installed browsers; per-session private/incognito mode |
| **Privacy** | Binds to `127.0.0.1` only; no telemetry SDKs; settings in XDG config |
| **API** | Local REST API for scripting and future modular integrations |

---

## Quick Start

### Option A — Run script (recommended)

```bash
git clone <your-repo-url> NetRail
cd NetRail
chmod +x run.sh
./run.sh
```

Open **http://127.0.0.1:7421**

### Option B — Manual setup

```bash
python3 -m venv .venv
source .venv/bin/activate
pip install -r requirements.txt
python -m netrail
```

Press **Ctrl+C** to stop the server.

---

## Project Structure

```
NetRail/
├── netrail/              # Core application package
│   ├── main.py           # FastAPI server and REST API
│   ├── search.py         # Metasearch adapter (ddgs)
│   ├── browsers.py       # Browser discovery and launcher
│   ├── config.py         # XDG settings persistence
│   └── static/           # Web UI (HTML, CSS, JS)
├── docs/
│   ├── MANUAL.md         # User manual
│   └── ARCHITECTURE.md   # System design and lifecycle roadmap
├── OPEN_LETTER.md        # Project manifesto
├── requirements.txt
├── run.sh
└── LICENSE
```

---

## Documentation

| Document | Description |
|----------|-------------|
| [User Manual](docs/MANUAL.md) | How to search, use operators, configure browsers, troubleshoot |
| [Architecture & Roadmap](docs/ARCHITECTURE.md) | System design, privacy model, modular boundaries, long-term lifecycle |
| [Open Letter](OPEN_LETTER.md) | Philosophy and motivation |

---

## Philosophy (short)

The internet is a network of direct connections. Search indexes are **optional services** — useful, but not mandatory gatekeepers.

1. **Your machine** issues the search request.
2. **Results render locally** in the link rail.
3. **You choose** the link, browser, and whether to go private.

In v0.1, broad discovery uses open metasearch providers via the `ddgs` library — not Google accounts, Chrome sync, or proprietary apps. Future versions add pluggable backends and local indexes you control. See [ARCHITECTURE.md](docs/ARCHITECTURE.md) for the full technical path.

---

## Local API

All endpoints bind to `127.0.0.1:7421` only.

| Endpoint | Method | Purpose |
|----------|--------|---------|
| `/api/health` | GET | Status, version, telemetry declaration |
| `/api/search` | POST | `{ "query", "mode": "web"\|"images", "max_results" }` |
| `/api/open` | POST | `{ "url", "browser_id", "private_mode" }` |
| `/api/browsers` | GET | List detected browsers |
| `/api/settings` | GET/PUT | Read/write user preferences |

Example:

```bash
curl -s http://127.0.0.1:7421/api/health
curl -s -X POST http://127.0.0.1:7421/api/search \
  -H 'Content-Type: application/json' \
  -d '{"query":"site:wikipedia.org python","mode":"web","max_results":5}'
```

---

## System Requirements

- **OS:** Linux (primary target); macOS/Windows planned via Tauri shell
- **Python:** 3.10+
- **Network:** Outbound HTTPS for metasearch providers
- **Optional:** One or more desktop web browsers for the open-link workflow

---

## Modular Ecosystem

NetRail is designed to remain **standalone**. It shares a philosophy — not a codebase — with tools like [NetMedic](https://github.com/kayab999/netmedic-linux) (network diagnostics and repair). Optional future integration happens only through the local HTTP API documented above. No shared dependencies, no bundled services.

---

## Development

```bash
python3 -m venv .venv
source .venv/bin/activate
pip install -r requirements.txt
python -m netrail
```

Configuration is stored at `~/.config/netrail/settings.json`.

---

## Roadmap (summary)

| Phase | Focus |
|-------|-------|
| **v0.1** *(current)* | Web + image search, link rail, browser picker, zero telemetry |
| **v0.2–v0.5** | History, export, backend plugins, connectivity UX |
| **v1.0** | Native desktop shell (Tauri), no browser required to use NetRail |
| **v1.x** | Self-hosted backends (SearXNG), BYO API keys |
| **v2.x** | Local crawl cache, trusted-domain indexes |
| **v2.x+** | Local AI reranking and query assistance |
| **v3.x** | Optional modular integrations (IPC bridges, MCP tools) |

Full lifecycle detail: [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md#lifecycle-roadmap)

---

## License

AGPL-3.0 — fork it, improve it, ship it. If you run a modified version as a network service, share your source.

---

*Built with spite and hope. For everyone who remembers when the web felt like yours.*