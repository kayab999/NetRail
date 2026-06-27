# NetRail — Viability Assessment & Strategic Response

This document records an expert technical and product analysis of NetRail, the team's response, and tracked mitigation status. It complements [ARCHITECTURE.md](ARCHITECTURE.md) with honest market and sustainability framing.

---

## Executive Position

NetRail's **defensible innovation** is the link-rail workflow: strict separation of search → review → open. Privacy and local-first execution are necessary but not sufficient differentiators — SearXNG, Whoogle, and Brave overlap there.

**Tagline:** *Search first. Browse second. On your terms.*

The product is viable as an open-source professional tool and institutional offering. It is not viable as a mass-market Google replacement without years of index investment.

---

## Critical Risks & Mitigation Status

| Risk | Severity | Mitigation | Status |
|------|----------|------------|--------|
| Manifesto vs. borrowed index (ddgs→Bing) | P0 | Open Letter rewrite; provenance in UI/API | ✅ v0.2 |
| `ddgs` HTML scraping fragility | P0 | `SearchBackend` protocol; fallback chain; SearXNG | ✅ v0.2 |
| No user retention (no history) | P1 | History + collections in v0.3 | 🔲 Planned |
| Python Tauri sidecar trap | P1 | Rust port at v0.5, not Python sidecar | 📋 Decided |
| Browser-as-UI paradox | P2 | Tauri shell accelerated to v0.5 | 📋 Planned |
| No test suite | P1 | pytest on API, backends, security | ✅ v0.2 |
| Linux-only audience ceiling | P2 | macOS after v0.5; audience expansion docs | 🔲 Planned |
| No revenue model | P1 | Sponsorware → institutional licenses at v1.0 | 📋 Documented |

---

## The Sovereignty Gradient

NetRail exposes sovereignty as a **visible journey**, not a pretended finished state:

| Step | Label | Requirement |
|------|-------|-------------|
| 1 | Local console — borrowed indexes | Default `ddgs` path |
| 2 | Pluggable backends | Multiple backends configured |
| 3 | Self-hosted SearXNG | `searxng_url` set and reachable |
| 4 | Local history and corpus | v0.3+ features shipped |
| 5 | Owned index | v2.x crawler + FTS |

---

## Competitive Position

| Competitor | NetRail wins on | NetRail loses on |
|------------|-----------------|------------------|
| SearXNG | Desktop UX, browser control, link-rail discipline | Maturity, instance ecosystem |
| Kagi | Free, local, no account, operators | Index quality, polish |
| Brave Search | Open source, link-first, operator-native | Index quality, speed |
| Browser search bar | Privacy, control, workflow | Zero friction |

**Do not compete on index quality in v0.x.** Compete on **workflow discipline** and **inspectable trust**.

---

## Business Model (proposed)

AGPL-3.0 prevents traditional surveillance SaaS. Viable paths:

| Phase | Model |
|-------|-------|
| v0.1–v0.5 | GitHub Sponsors + donations |
| v1.0+ | Open core: AGPL core + commercial institutional tier (team collections, priority patches, SearXNG setup support) |
| Parallel | Consulting on self-hosted discovery stacks |

Premium must not compromise the privacy guarantees of the core product.

---

## Technical Decisions Locked In

### 1. SearchBackend protocol (implemented v0.2)

All discovery flows through `netrail/backends/`. No direct `ddgs` imports outside `ddgs.py`.

### 2. SearXNG in Phase 1, not Phase 3

Configure via `~/.config/netrail/settings.json`:

```json
{
  "searxng_url": "http://127.0.0.1:8080",
  "backend_order": ["searxng", "ddgs"],
  "ddgs_enabled": true
}
```

### 3. Rust port over Python sidecar

The Python codebase (~500 LOC across core modules) will be ported to Rust for the Tauri shell. Python remains the lightweight headless/API distribution.

### 4. Distribution before feature sprawl

Flatpak, AppImage, and Docker are Phase 3 (v0.4) — before v1.0 public launch marketing.

---

## Audience Strategy

**Primary (now):** Linux professionals — developers, OSINT analysts, journalists, legal researchers who read source code.

**Expansion (v1.0):** macOS developers; institutional site licenses for newsrooms and compliance-driven orgs.

**Realistic v0.x addressable users:** 50,000–200,000 globally. Sufficient for sustainable open source; requires institutional tier for full-time maintenance income.

---

## What We Agree With From External Review

1. The credibility gap was real — fixed with honesty, not marketing.
2. `ddgs` fragility is an existential operational risk — backend abstraction is mandatory.
3. Retention requires local state — history is the first v0.3 priority.
4. The link-rail workflow is the moat — positioning should lead with workflow, not nostalgia.
5. Installers matter as much as features for adoption.

## What We Modulate

1. **"Not built on Google's index"** — refined to *"not built on a Google account or Chrome sync; default index chain is disclosed."*
2. **Tauri timing** — moved to v0.5 (after distribution), but architecture commits to Rust port, not sidecar.
3. **Revenue** — documented but not implemented until v1.0; AGPL core remains unconditional.

---

## Review Cadence

Revisit this document each minor release. Update risk statuses. Add metrics when available:

- Install count (Flatpak Flathub, GitHub releases)
- Backend failure rates (local logs, opt-in)
- Retention proxy: history DB row count (v0.3+, local only)

---

*Last updated: v0.2.0 — June 2026*