# NetRail v1.2.1 — Wikipedia Snippets & Search Recovery

Patch release fixing empty Wikipedia result cards, improving backend error UX, and bundling splash/Tribunal fixes from the v1.2.0 cycle.

## Highlights

- **Wikipedia intro extracts** — result cards show article summaries (OpenSearch + `prop=extracts`)
- **Cleaner error banner** — shorter DDGS message, dismiss button, recovery hints for Brave/SearXNG
- **Empty-snippet polish** — no more "No description available." on Wikipedia-only fallback
- **Splash fix** — `RESULTS_PAGE_SIZE` TDZ crash resolved; 2.5s HTML failsafe

## Search recovery

When DuckDuckGo serves a bot challenge, NetRail falls back to Wikipedia and surfaces actionable hints:

```bash
# Self-hosted metasearch
export NETRAIL_SEARXNG_URL=https://your-searxng.example

# Brave Search API (free tier)
export BRAVE_SEARCH_API_KEY=your_key_here
```

Edit `~/.config/netrail/settings.json` to enable backends. `/api/health` now includes `search_recovery` hints.

## Downloads

| File | Use Case |
|------|----------|
| `NetRail_1.2.1_amd64.AppImage` | Desktop app |
| `NetRail_1.2.1_amd64.deb` | Debian/Ubuntu package |
| `netrail-api` | Headless API server |
| `SHA256SUMS` | Verify integrity |

## Since v1.2.0

No breaking API changes. Wikipedia fallback now returns readable snippets; fanout errors are shorter and dismissible.