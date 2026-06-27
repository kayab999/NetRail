# NetRail v1.0.0 — The Sovereign Search Console

A local, privacy-first research console built in Rust. Search multiple indexes, review links locally, and open them on your terms.

## 🚀 v1.0 Highlights

- **Native Rust Engine:** Cold starts in &lt;100ms. 6.7MB headless binary.
- **Multi-Backend Fanout:** Query SearXNG and DDGS concurrently. Merge, dedupe, and interleave results.
- **Encrypted Local History:** FTS5 search over your past queries. Your data, encrypted with your OS keyring.
- **BYO API Keys:** Use Brave Search API with your own key. No vendor lock-in.
- **Zero Telemetry:** Still no analytics, accounts, or cloud sync. Ever.

## 📥 Downloads

| File | Use Case |
|------|----------|
| `NetRail_1.0.0_amd64.AppImage` | Desktop app (Ubuntu/Fedora/etc) |
| `NetRail_1.0.0_amd64.deb` | Debian/Ubuntu native package |
| `netrail-api` | Headless CLI / API server (homelabs, scripting) |

## 🛡️ The Sovereignty Gradient

NetRail doesn't pretend you can overthrow Google overnight. We show you exactly where your results come from, and give you a path to independence:

1. **Disclosed metasearch** (default) — `[DDGS]` pills show the chain
2. **Self-hosted SearXNG** — your instance, your engines
3. **BYO API keys** — Brave Search with `BRAVE_SEARCH_API_KEY`
4. **Local crawling** — coming in v2.x

## Quick start

```bash
chmod +x NetRail_1.0.0_amd64.AppImage
./NetRail_1.0.0_amd64.AppImage
```

Headless:

```bash
chmod +x netrail-api
./netrail-api --api-only
curl http://127.0.0.1:7421/api/health
```

## Keyboard workflow

| Key | Action |
|-----|--------|
| `↑` / `↓` | Highlight result |
| `Enter` | Open highlighted result |
| `Shift+Enter` | Open in private mode |
| `Ctrl+Shift+S` | Focus NetRail (Tauri) |

---

*v0.1 was the promise. v1.0 is the receipt.*