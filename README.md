# NetRail

**A local research console — inspired by Web Ferret, built for the post-big-tech web.**

NetRail is a privacy-first search front-end that runs entirely on your machine. Results appear in a clean link rail; **nothing opens in a browser until you choose**. No accounts. No analytics. No telemetry.

> This is an open letter in code: you do not need a surveillance company to find things on the internet.

## What it does (v0.1)

- **Web & image search** with operator support (`site:`, `filetype:`, `intitle:`, `"phrase"`, etc.)
- **Link-first GUI** — scan results, then open what matters
- **Browser picker** — choose Firefox, Chrome, Brave, Chromium, Edge, and others detected on your system
- **Private / incognito mode** per session
- **100% local** — settings stored in `~/.config/netrail/`; queries never logged to NetRail servers (there are none)

## Quick start

```bash
git clone <your-repo-url> NetRail
cd NetRail
chmod +x run.sh
./run.sh
```

Open **http://127.0.0.1:7421** in any browser (ironic, we know — or curl the API).

### Manual setup

```bash
python3 -m venv .venv
source .venv/bin/activate
pip install -r requirements.txt
python -m netrail
```

## Philosophy

The internet is still a network of direct connections. Search indexes are optional services — useful, but not mandatory gatekeepers. NetRail keeps **you** in the loop:

1. Your machine issues the search request.
2. Results render locally.
3. You pick the link, the browser, and whether to go private.

We depend on open metasearch providers for broad discovery in v0.1 — not on Google accounts, Chrome sync, or proprietary apps. Future versions will add local crawl caches and pluggable backends you control.

Read the full manifesto in [OPEN_LETTER.md](OPEN_LETTER.md).

## API (local only)

| Endpoint | Method | Purpose |
|---|---|---|
| `/api/search` | POST | `{ "query", "mode": "web"\|"images", "max_results" }` |
| `/api/open` | POST | `{ "url", "browser_id", "private_mode" }` |
| `/api/browsers` | GET | List detected browsers |
| `/api/settings` | GET/PUT | Persist preferences |

## License

AGPL-3.0 — fork it, improve it, ship it. If you run a modified version as a network service, share your source.

## Roadmap

- [ ] Desktop shell (Tauri) — no browser needed to use NetRail
- [ ] Local search history (encrypted, optional)
- [ ] Pluggable backends (SearXNG, self-hosted, bring-your-own API keys)
- [ ] AI reranking — fully local models
- [ ] Export results (CSV/JSON)

---

*Built with spite and hope. For everyone who remembers when the web felt like yours.*