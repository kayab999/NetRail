# NetRail

**Search first. Browse second. On your terms.**

NetRail is a local, privacy-first research console for Linux. It fans out your query to every search backend you enable, merges results on your machine, and shows them in a link rail — **nothing opens in a browser until you choose**. No accounts. No analytics. No telemetry.

> *v0.1 was a promise. v1.0 is the receipt.*  
> Read the manifesto in [OPEN_LETTER.md](OPEN_LETTER.md).

**Version:** 1.0.0 · **License:** [AGPL-3.0](LICENSE)

---

## Why NetRail

| Problem | NetRail answer |
|---------|----------------|
| Search is a funnel | Link rail — you choose what to open |
| One fragile index | **Fanout** to SearXNG + DDGS + Brave concurrently |
| Opaque provenance | `[DDGS]` / `[SearXNG]` / `[Brave]` pill on every result |
| Cloud history | Encrypted SQLite + FTS5, local only |
| Slow Python startup | Native Rust engine, **&lt;100ms** API cold start |
| Surveillance economics | Zero telemetry — audit the source |

---

## Quick Start

### Native desktop (recommended)

```bash
git clone <your-repo-url> NetRail && cd NetRail
npm install && npm run build
./install.sh
netrail-launch
```

The Tauri shell loads `http://127.0.0.1:7421` — same UI, Rust engine underneath.

### Headless API (homelab / scripting)

```bash
cd src-tauri
cargo build --release --bin netrail-api --no-default-features
./target/release/netrail-api
curl http://127.0.0.1:7421/api/health
```

### Python fallback

```bash
./install.sh          # without Tauri build
netrail-launch
```

### Docker

```bash
cp .env.example .env
docker compose up -d netrail
```

---

## Fanout & backends

Enable backends in `~/.config/netrail/settings.json`:

```json
{
  "search_strategy": "fanout",
  "searxng_url": "http://127.0.0.1:8080",
  "brave_enabled": true,
  "backend_order": ["searxng", "ddgs", "brave"]
}
```

**Brave API key** — never stored on disk:

```bash
export BRAVE_SEARCH_API_KEY="your-key"
```

Set `search_strategy` to `"fallback"` for legacy sequential behavior.

---

## Keyboard workflow

| Key | Action |
|-----|--------|
| `↑` / `↓` | Highlight result in link rail |
| `Enter` | Open highlighted result |
| `Shift+Enter` | Open in private/incognito |
| `Ctrl+C` (search focused) | Copy highlighted URL |
| `Ctrl+Shift+S` (Tauri) | Focus NetRail from anywhere |

---

## Local API

All endpoints bind to `127.0.0.1:7421` only.

```bash
curl -s http://127.0.0.1:7421/api/health
curl -s -X POST http://127.0.0.1:7421/api/search \
  -H 'Content-Type: application/json' \
  -d '{"query":"rust programming","mode":"web","max_results":10}'
```

Full API: see [docs/MANUAL.md](docs/MANUAL.md).

---

## Project structure

```
NetRail/
├── src-tauri/          # Rust + Tauri (primary engine)
├── netrail/static/     # Web UI (unchanged contract)
├── netrail/            # Python headless fallback
├── .github/workflows/  # Release CI (AppImage + netrail-api)
└── docs/
```

---

## Documentation

| Document | Description |
|----------|-------------|
| [User Manual](docs/MANUAL.md) | Search, operators, browsers, troubleshooting |
| [Architecture](docs/ARCHITECTURE.md) | Design, lifecycle roadmap |
| [Distribution](docs/DISTRIBUTION.md) | Flatpak, Docker, AppImage, install |
| [Open Letter](OPEN_LETTER.md) | Philosophy and v1.0 postscript |

---

## Releases

Tagged releases (`v1.0.0`) publish via GitHub Actions:

- Linux AppImage + `.deb`
- `netrail-api` headless binary

Download from [GitHub Releases](https://github.com/your-org/NetRail/releases).

---

## Development

```bash
# Native
npm run dev

# Headless API only
cargo run --manifest-path src-tauri/Cargo.toml --bin netrail-api --no-default-features

# Python fallback + tests
python -m venv .venv && source .venv/bin/activate
pip install -r requirements.txt
pytest
```

---

## License

AGPL-3.0 — fork it, improve it, ship it.

---

*Built with spite and hope. For everyone who remembers when the web felt like yours.*