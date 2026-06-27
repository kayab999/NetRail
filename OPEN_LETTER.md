# An Open Letter Against Big-Tech Dependency

*NetRail — June 2026*

---

We were told the internet is free. It isn't. We pay with attention, identity, and data — mined by default, sold by design.

A handful of companies built the maps we use to navigate a decentralized web. Then they became the toll booths. Search stopped being a tool and became a funnel. Results optimized for engagement, not truth. Privacy policies longer than novels. "Free" products that cost you yourself.

**We reject the premise that discovery must be centralized.**

You have an ISP. You have a computer. That is enough to request any page on the open web. What you lack is not permission from Silicon Valley — it is a **console** that respects you: local, operator-aware, link-first, silent.

NetRail is that console — a desktop search workflow that shows you links and lets you choose **before** you ever open a browser.

## Radical honesty about v0.2

We will not pretend independence we have not built yet.

**Today, unless you configure your own SearXNG instance**, NetRail's default path is:

```
Your query → NetRail (localhost) → ddgs library → DuckDuckGo metasearch → primarily Bing's index
```

That is borrowed infrastructure. We do not call it sovereign. We call it **Step 1 of 5** on a visible gradient toward control you own.

What *is* sovereign today:

- NetRail sends **zero analytics** — audit `netrail/main.py`, `netrail/search.py`, and `netrail/backends/`
- Results render locally; **nothing opens until you click**
- Settings stay in `~/.config/netrail/` — no account, no sync
- Backend provenance is **shown on every result** — we do not hide the chain

What becomes sovereign next (and in what order):

1. **Local console** — no NetRail telemetry *(now)*
2. **Pluggable backends + fallback** — not one fragile provider *(now)*
3. **Self-hosted SearXNG** — your instance, your engines *(configure `searxng_url`)*
4. **Local history and corpus** — your research accumulates on your machine
5. **Owned index** — crawl allowlists you define; search without borrowed maps

Independence is incremental. Honesty is not optional.

## What we believe

1. **Your queries are yours.** They should not train models, feed ad auctions, or populate behavioral profiles.
2. **Results are suggestions, not commands.** Show links; let humans choose.
3. **Software should be inspectable.** Open source is not a feature — it is the contract.
4. **Independence is incremental.** Start with metasearch you control; grow toward caches and crawls you own.
5. **Professionals deserve pro tools.** Operators, exports, browser discipline — not infinite scroll dopamine.

## What NetRail is not

- Not a claim of a Google-free index in v0.2 (unless you bring your own backend).
- Not a venture-backed "AI search" with a chatbox and a billing page.
- Not a product that phones home.

## What you can do

- Run it. Break it. Fix it. Fork it.
- Point `searxng_url` at an instance you control.
- Tell others that **dependency is a choice**.

The web is still out there — messy, magnificent, bigger than any single company. We intend to look at it on our terms.

**Search first. Browse second. Take back the rail.**

— The NetRail contributors