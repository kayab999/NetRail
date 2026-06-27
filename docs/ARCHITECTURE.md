# NetRail — Architecture & Lifecycle Blueprint

## Vision

NetRail aims to be a **sovereign research console** for Linux professionals: local, operator-aware, link-first, and free of surveillance economics. It revives the Web Ferret workflow for an era where discovery has been centralized behind a handful of indexes and engagement-optimized UIs.

NetRail does not try to rebuild Google's data centers on a laptop. It tries to put **you** back in control of the discovery-and-open loop — with a credible path toward greater independence over time.

---

## High-Level Design (v1.0 — current)

Production v1.0 runs a **Rust Axum API** on `127.0.0.1:7421`, shared with a static web UI (`netrail/static/`). The Tauri desktop shell (`src-tauri/`) embeds that UI and spawns the API in-process. A headless `netrail-api` binary ships the same engine without GTK/Tauri. Python (`netrail/main.py`) remains for tests, Docker, Flatpak, and `install.sh` fallback.

```
┌─────────────────────────────────────────────────────────────────────────┐
│  ENTRY: Tauri netrail │ netrail-api │ python -m netrail │ Docker/Flatpak │
└───────────────────────────────┬─────────────────────────────────────────┘
                                ▼
┌─────────────────────────────────────────────────────────────────────────┐
│  Axum (Rust) or FastAPI (Python) @ 127.0.0.1:7421                       │
│  /api/search  /api/open  /api/settings  /api/history  /api/collections   │
│  /api/docs/{manual,about}  /static/*                                   │
└───────┬─────────────────┬──────────────────────┬──────────────────────┘
        ▼                 ▼                      ▼
   static UI         backends fanout         ~/.config/netrail/
   (link rail)       ddgs │ searxng │ brave   settings.json
                     merge + dedupe            ~/.local/share/netrail/
                     20s deadline              netrail.db (SQLite + FTS5)
        │                 │
        └────────┬────────┘
                 ▼
        browsers.rs / browsers.py → subprocess (flatpak-spawn on Flatpak)
                 ▼
        External: metasearch providers + user-chosen desktop browser
```

| Component | Primary implementation | Notes |
|-----------|------------------------|-------|
| Search fanout | `src-tauri/src/backends/mod.rs` | `join_all`, merge, 20s timeout |
| History | `src-tauri/src/history/mod.rs` | Fernet encryption, FTS5 |
| URL safety | `src-tauri/src/security.rs` | Open vs backend URL policies |
| UI | `netrail/static/app.js` | Shared across all modes |
| Python parity | `netrail/` package | Kept for CI and legacy packaging |

Repository: [github.com/kayab999/netrail](https://github.com/kayab999/netrail)

---

## Design Principles

| Principle | Implementation |
|-----------|----------------|
| **Local-first** | Server binds `127.0.0.1`; settings in XDG config |
| **Zero telemetry** | No analytics, crash reporters, accounts, or ads |
| **Link-first** | Results in-app; browser opens only on explicit user action |
| **Operator-native** | Power-user syntax passthrough, not dumbed-down queries |
| **Modular** | Standalone product; optional integration via HTTP API only |
| **Inspectable** | AGPL-3.0; behavior auditable; no hidden network calls beyond search |
| **Incremental sovereignty** | v1.0 fanout metasearch; later phases add owned indexes and local AI |

---

## High-Level Design (v0.1 — historical Python baseline)

```
┌─────────────────────────────────────────────────────────────────┐
│                        Entry Points                              │
│           ./run.sh  │  python -m netrail  │  curl API            │
└────────────────────────────┬────────────────────────────────────┘
                             │
                             ▼
┌─────────────────────────────────────────────────────────────────┐
│                     netrail/main.py                              │
│              FastAPI @ 127.0.0.1:7421                            │
│   /  /api/search  /api/open  /api/browsers  /api/settings        │
└────────────┬───────────────────────────────┬────────────────────┘
             │                               │
             ▼                               ▼
┌────────────────────────┐      ┌────────────────────────────┐
│   static/ (Web UI)     │      │   Core modules              │
│   index.html           │      │   search.py   → ddgs        │
│   app.js  style.css    │      │   browsers.py → subprocess  │
│   Link rail + controls │      │   config.py   → ~/.config/  │
└────────────────────────┘      └──────────────┬─────────────┘
                                               │
                                               ▼
                              ┌────────────────────────────────┐
                              │   External (user-initiated)     │
                              │   Metasearch providers (ddgs)   │
                              │   Desktop browser (on Open)     │
                              └────────────────────────────────┘
```

### Request flow: search

```
User types query + operators
        │
        ▼
POST /api/search  { query, mode, max_results }
        │
        ▼
search.py  →  DDGS().text() | DDGS().images()
        │
        ▼
Normalized result list  { title, url, snippet, image? }
        │
        ▼
Link rail renders locally  (no auto-navigation)
```

### Request flow: open link

```
User clicks Open
        │
        ▼
POST /api/open  { url, browser_id?, private_mode? }
        │
        ▼
config.py  →  merge saved preferences
        │
        ▼
browsers.py  →  discover → build argv → subprocess.Popen
        │
        ▼
Chosen browser opens URL (private flag if supported)
```

---

## Core Modules

| Module | Responsibility |
|--------|----------------|
| `main.py` | HTTP server, routing, request validation, error surfacing |
| `search.py` | Metasearch adapter; normalizes provider responses |
| `browsers.py` | Freedesktop `.desktop` parsing; known-browser private flags |
| `config.py` | XDG settings load/save with safe defaults |
| `static/` | Self-contained UI; no external CDN dependencies |

### Search adapter boundary

`search.py` is the **only** module that talks to metasearch providers in v0.1. All future backends (SearXNG, Brave API, local index) plug in behind this interface:

```python
def search(query: str, mode: SearchMode, max_results: int) -> list[dict]:
    ...
```

Returning a stable shape:

```json
{
  "title": "string",
  "url": "string",
  "snippet": "string",
  "image": "string | null",
  "source": "string"
}
```

### Browser launcher boundary

`browsers.py` owns all process-spawn logic for opening URLs. Future desktop shell (Tauri) reuses this module from the Rust side via Python subprocess or a ported implementation.

---

## Privacy & Trust Architecture

### Threat model (v0.2)

| Threat | Mitigation |
|--------|------------|
| NetRail exfiltrating queries | No telemetry code; localhost-only bind; open source |
| LAN snooping on NetRail API | `127.0.0.1` bind, not `0.0.0.0` |
| Provider tracking | No NetRail cookies; user may choose privacy DNS/VPN externally |
| Malicious URL open | `validate_open_url()` rejects `javascript:`, `data:`, credentials, localhost |
| UI injection via results | Content-Security-Policy on all HTTP responses |
| Settings tampering | User-owned `~/.config/netrail/`; no cloud sync |

### What we explicitly do not promise in v0.2

- A Google-free or Bing-free index unless you configure your own backend (e.g. SearXNG)
- Anonymity against metasearch providers (use VPN/Tor at system level)
- Google-equivalent index coverage on the default `ddgs` path
- Query result consistency across time (providers change HTML/APIs without notice)

### Default discovery chain (disclosed)

```
User query → NetRail (127.0.0.1) → SearchBackend registry
    → [searxng if configured] → else ddgs → DuckDuckGo → primarily Bing
```

Provenance is returned in API responses and shown in the UI. See [VIABILITY.md](VIABILITY.md) for product risk analysis.

### Future: local history encryption

Planned for v0.2+. Optional encrypted SQLite at `~/.local/share/netrail/history.db` with user-held key or system keyring integration.

---

## Modular Integration Boundary

NetRail and tools like **NetMedic** (network diagnostics) remain **separate repositories and processes**. Integration is optional and contract-based.

### Integration contract (stable surface)

| Endpoint | Consumer use case |
|----------|-------------------|
| `GET /api/health` | Liveness probe before calling search |
| `POST /api/search` | Programmatic research queries |
| `POST /api/open` | Open evidence links in controlled browser |
| `GET /api/browsers` | Display browser capabilities in host app |
| `GET /api/backends` | List configured search backends and provenance |

### What integrators must not do

- Import NetRail Python internals directly (unstable)
- Bundle NetRail inside GPL-incompatible proprietary shells without license review
- Expose port 7421 beyond localhost without explicit user consent and hardening

### Example future bridge (not implemented)

```
External tool  →  GET /api/health
              →  POST /api/search (on user action)
              →  POST /api/open   (on user confirmation)
```

NetMedic IPC, MCP servers, or desktop launchers can wrap these calls in ~50 lines without merging codebases.

---

## Technology Stack

| Layer | v0.1 Choice | Rationale |
|-------|-------------|-----------|
| Language | Python 3.10+ | Fast iteration, ddgs ecosystem |
| HTTP | FastAPI + Uvicorn | Typed API, auto-validation, scriptable |
| Search | `SearchBackend` protocol | `ddgs` default; `searxng` optional; fallback chain |
| UI | Static HTML/CSS/JS | No build chain; auditable; offline-capable assets |
| Config | JSON in XDG | Standard Linux convention |
| Desktop (planned) | Tauri 2 + **Rust port** | Single binary; avoid Python sidecar (see VIABILITY.md) |
| Local AI (planned) | llama.cpp / GGUF | Aligns with local-model patterns used in adjacent tools |

---

## Lifecycle Roadmap

The roadmap is organized into **phases** with explicit goals, deliverables, exit criteria, and risk notes. Dates are indicative — scope beats calendar.

---

### Phase 0 — Genesis (complete)

**Version:** 0.1.0  
**Status:** Released

| Deliverable | State |
|-------------|-------|
| Web + image metasearch | ✅ |
| Operator passthrough | ✅ |
| Link rail UI | ✅ |
| Browser picker + private mode | ✅ |
| Local REST API | ✅ |
| Zero telemetry | ✅ |
| AGPL-3.0 + open letter | ✅ |

**Exit criteria:** User can search, review, and open links locally without accounts or analytics.

---

### Phase 1 — Credibility + Reliability (in progress)

**Version:** 0.2.0  
**Theme:** Close the manifesto–reality gap; survive `ddgs` breakage

| Deliverable | State |
|-------------|-------|
| `SearchBackend` protocol | ✅ |
| SearXNG backend (configure `searxng_url`) | ✅ |
| Fallback chaining across backends | ✅ |
| Backend provenance in API + UI | ✅ |
| Sovereignty step indicator | ✅ |
| CSP + stricter URL validation | ✅ |
| Test suite (API, backends, security) | ✅ |
| Open Letter honesty rewrite | ✅ |
| Result caching | 🔲 v0.2.1 |
| Async multi-backend fanout | 🔲 v0.2.1 |
| Brave Search API (BYO key) | 🔲 v0.2.2 |

**Exit criteria:** User sees where results come from; SearXNG works when configured; tests gate releases.

---

### Phase 2 — Retention + Utility (complete)

**Version:** 0.3.0  
**Status:** Released

| Deliverable | State |
|-------------|-------|
| SQLite schema (`queries`, `results`, `visits`, `collections`) | ✅ |
| Fernet field encryption + keyring / `NETRAIL_DB_KEY` | ✅ |
| FTS5 local history search | ✅ |
| Revisit badges + visit metadata in search API | ✅ |
| Collections + CSV/JSON export | ✅ |
| `history_ttl_days` auto-purge | ✅ |
| History tab UI | ✅ |

**Exit criteria:** User can search past queries faster than re-Googling them. ✅

---

### Phase 3 — Distribution (complete)

**Version:** 0.4.0  
**Status:** Released

| Deliverable | State |
|-------------|-------|
| Flatpak + `flatpak-spawn --host` browser fix | ✅ |
| Docker + Compose (localhost bind, SearXNG profile) | ✅ |
| AppImage / PyInstaller build script | ✅ |
| `install.sh` one-click local install | ✅ |
| `.desktop` + SVG icon + auto-open UI | ✅ |
| [DISTRIBUTION.md](DISTRIBUTION.md) | ✅ |

**Exit criteria:** Install in under 60 seconds without manual venv setup. ✅

---

### Phase 4 — Native Shell (Rust) ✅

**Version:** 0.5.0  
**Theme:** End the "open Chrome to use privacy search" paradox

| Item | Status |
|------|--------|
| Rust port of search + browser + history modules | ✅ `src-tauri/` — Axum on `127.0.0.1:7421` |
| Tauri 2 shell | ✅ Webview → local API; tray + `Ctrl+Shift+S` + single-instance |
| Fernet DB migration | ✅ v0.4 encrypted SQLite opens without data loss |
| Python variant retained | ✅ `install.sh` falls back to `python -m netrail` |
| `--api-only` headless mode | ✅ For Docker-style scripting without GUI |

**Architecture decision:** Full Rust port — no Python sidecar. UI in `netrail/static/` unchanged.

**Exit criteria:** ✅ `curl http://127.0.0.1:7421/api/health` returns 200 from native binary; encrypted history readable.

---

### Phase 5 — Public Launch

**Target version:** 1.0  
**Theme:** Multi-backend merge as technical moat

| Item | Status |
|------|--------|
| Async fanout + dedupe merge | ✅ `join_all` fanout; `merge.rs` normalize → dedupe → interleave |
| Brave BYO-key backend | ✅ `BRAVE_SEARCH_API_KEY` env; never stored in settings |
| Pro-console UI | ✅ Backend pills, keyboard nav, result export (JSON/CSV) |
| GitHub Release CI | ✅ AppImage + `.deb` + `netrail-api` on tag push |
| Polished onboarding | ⏳ Sovereignty wizard; SearXNG setup guide (post-1.0) |
| Institutional license tier | ⏳ Newsrooms, legal, government (open core) |

**Exit criteria:** ✅ Fanout search, BYO API keys, keyboard workflow, and distributable AppImage.

---

### Phase 6 — Owned Corpus

**Target versions:** 2.0 – 2.4  
**Theme:** Discovery without borrowing planetary indexes

| Item | Description |
|------|-------------|
| Trusted-domain crawler | User-defined allowlist (e.g. `*.gov`, `arxiv.org`) |
| Local full-text index | SQLite FTS5 or Tantivy (if Rust migration) |
| Crawl scheduler | Background refresh; depth and rate limits |
| Hybrid mode | Metasearch for discovery → cache → local re-search |
| Feed/sitemap ingest | RSS, Atom, sitemap.xml as seed sources |

**Architecture addition:**

```
┌──────────────┐     ┌──────────────┐     ┌──────────────┐
│  Metasearch  │     │  Crawler     │     │  Local index │
│  (optional)  │     │  (allowlist) │     │  (FTS)       │
└──────┬───────┘     └──────┬───────┘     └──────┬───────┘
       │                    │                    │
       └────────────────────┴────────────────────┘
                            │
                     search.py router
                            │
                       Link rail UI
```

**Exit criteria:** Professional user can search a curated corpus with no live metasearch call.

**Risks:** Crawl scope creep, robots.txt ethics, storage growth — mitigate with explicit allowlists and quotas.

---

### Phase 7 — Local Intelligence

**Target versions:** 2.5 – 2.9
**Theme:** AI as judgment layer, not replacement for fetch

| Item | Description |
|------|-------------|
| Query → operators | Local LLM suggests `site:` / `filetype:` from natural language |
| Result reranking | Embedding similarity reorders metasearch hits locally |
| Snippet summarization | One-line gist before click (on-device) |
| Spam/SEO filter | Heuristic + model scoring for content-farm patterns |
| Agent loop (advanced) | Search → fetch top N → refine query (OSINT mode) |

**Guardrail pattern (borrowed from mature local-AI tools):**

```
User intent  →  Local LLM (GBNF-constrained)
             →  Whitelisted actions only:
                 - refine_query
                 - rerank_results
                 - summarize_result
             →  Never auto-open URLs without user click
```

**Exit criteria:** AI runs fully offline; no query leaves machine for inference.

---

### Phase 8 — Ecosystem & Long-Term Lifecycle

**Target versions:** 3.0+
**Theme:** NetRail as a platform others can orbit

| Item | Description |
|------|-------------|
| MCP server | Expose search/open as MCP tools for local AI assistants |
| IPC bridge docs | Formal OpenAPI spec; versioned API (`/api/v1/`) |
| Plugin SDK | Third-party search backends as Python entry points |
| Optional NetMedic bridge | Documented HTTP client, not bundled dependency |
| i18n | UI translation support |
| Accessibility audit | WCAG pass for web UI and Tauri shell |

---

## Software Lifecycle Policy

### Versioning

[Semantic Versioning](https://semver.org/):

- **MAJOR** — API or config breaking changes, backend contract shifts
- **MINOR** — New features, new backends, UI capabilities
- **PATCH** — Bug fixes, provider adapters, security patches

### Release channels

| Channel | Audience |
|---------|----------|
| `main` | Development; may break |
| Tagged releases | General users (`v0.1.0`, `v1.0.0`) |
| LTS (future) | Enterprise/stable after v1.0; security backports |

### Deprecation rules

1. API endpoints deprecated for ≥2 minor versions before removal.
2. Backend plugins announce removal in CHANGELOG with migration guide.
3. Settings keys auto-migrate when possible; log one-time warnings otherwise.

### Security response

| Severity | Response target |
|----------|----------------|
| Localhost escape / RCE | Immediate patch release |
| Provider SSRF via search | Patch within 7 days |
| Dependency CVE | Assess; patch or pin within 14 days |

### End-of-life criteria

A major version enters maintenance when the next major ships. Maintenance duration: minimum 12 months or until two majors ahead — whichever is longer. EOL announced in README with upgrade path.

---

## Data Flow & Sovereignty Model

Understanding what NetRail is responsible for vs what the user supplies:

```
┌─────────────────────────────────────────────────────────────┐
│  USER OWNS                                                   │
│  • Queries typed                                            │
│  • Browser choice + private mode                            │
│  • Optional: VPN, DNS, Tor (external tools)                 │
│  • Optional: self-hosted SearXNG (Phase 3+)                 │
│  • Optional: crawl allowlists (Phase 4+)                    │
└─────────────────────────────────────────────────────────────┘
┌─────────────────────────────────────────────────────────────┐
│  NETRAIL OWNS                                                │
│  • Presentation (link rail)                               │
│  • Open-link orchestration                                  │
│  • Settings persistence                                     │
│  • Backend routing (future)                                 │
│  • Local index + AI (future)                                │
└─────────────────────────────────────────────────────────────┘
┌─────────────────────────────────────────────────────────────┐
│  THIRD PARTIES (user-initiated, Phase 0)                     │
│  • Metasearch providers via ddgs                            │
│  • Destination websites (on Open click)                     │
└─────────────────────────────────────────────────────────────┘
```

**Sovereignty grows with phase:** Phase 0 borrows indexes. Phase 4 can operate without them for defined corpora. Phase 5 adds local judgment without cloud inference.

---

## Comparison to Historical Web Ferret

| Web Ferret (~1997–2004) | NetRail (2026+) |
|-------------------------|-----------------|
| Desktop native app | Web UI → Tauri shell (Phase 2) |
| Multi-engine metasearch | ddgs → pluggable backends |
| Link list, user opens | Link rail, user opens |
| Pre-Google-open-web | Post-enclosure web; privacy default |
| No telemetry (era norm) | No telemetry (explicit guarantee) |
| Operators via engine syntax | Operator passthrough + future NL assist |

The **workflow** is the inheritance. The **infrastructure** is modernized for a hostile, centralized web.

---

## Open Questions (to resolve per phase)

| Question | Phase to decide |
|----------|-----------------|
| Python sidecar vs full Rust port in Tauri? | Phase 2 |
| Default encrypted history key storage? | Phase 1 |
| SearXNG bundled vs user-provided URL? | Phase 3 |
| Crawl ethics / robots.txt policy? | Phase 4 |
| Which local model size for reranking? | Phase 5 |
| API versioning strategy (`/api/v1`)? | Phase 6 |

---

## Summary

NetRail v1.0 is a **modular research console**: localhost API (Rust-primary), multi-backend fanout, encrypted history, browser launcher, zero telemetry. Python and Docker paths remain for packaging parity. The long-term blueprint moves sovereignty forward in four dimensions:

1. **Shell** — from browser-hosted UI to native app  
2. **Discovery** — from borrowed indexes to chosen backends to owned corpora  
3. **Intelligence** — from raw results to local reranking and operator assist  
4. **Ecosystem** — from standalone tool to documented platform others integrate with  

Each phase is shippable alone. No phase requires merging with external projects. The path is incremental — matching the honest technical reality that ISP + computer is enough to *reach* the web, and NetRail's job is to make *finding and opening* yours again.

---

*NetRail Architecture Working Document — v1.0.0 (Rust-primary) — maintained by [kayab999](https://github.com/kayab999) — 2026*