# Release notes 1.8.0

## SearXNG enrichment (P0)

- Optional search params on `SearchRequest`: `language`, `time_range` (day/month/year), `pageno` (1-100), `safesearch` (0/1/2). Propagated to SearXNG only.
- Dynamic discovery via SearXNG `/config`: `GET /api/searxng/config` returns status (`not_configured`/`ok`/`unavailable`/`invalid`), categories, enabled engines, and overlap warnings.
- Non-blocking overlap diagnostics when a SearXNG engine (DuckDuckGo/Brave) overlaps an enabled direct backend.

## Notes

- SearXNG remains optional; NetRail works without it.
- No UI changes in this release.