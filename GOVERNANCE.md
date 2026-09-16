# Project Governance

## Current state

NetRail is maintained by a single developer (Carlos Hernández,
https://github.com/kayab999). All commits to date are from the primary
maintainer. There is no team to pretend otherwise — this document records
that honestly and what follows from it.

## Decision process

- Technical direction: `HANDOVER.md` and `CHANGELOG.md`.
- Security model and tradeoffs: `SECURITY.md`.
- Roadmap and lifecycle: `docs/ARCHITECTURE.md`.
- Internal QA documents in `docs/AUDIT_*.md` carry a provenance note:
  they are self-generated development artifacts, not independent audits.

## Continuity guarantee

If the maintainer becomes unavailable, the project remains usable and
forkable:

- **License:** AGPL-3.0 — the code stays free and forkable, no relicensing risk.
- **CI/CD is in-repo:** `.github/workflows/` builds, tests, audits
  dependencies and publishes releases with no external services beyond GitHub.
- **No external services required** to build, test or run NetRail.
- **Docs are the handover:** `HANDOVER.md` is maintained as a zero-context
  resume file; `docs/RELEASE_ASSURANCE.md` maps every guarantee to its backing.

## Becoming a co-maintainer

Sustained, reviewed contributions (see `CONTRIBUTING.md`) over several
release cycles are the path. There is no formal process beyond that — open
an issue to start the conversation.
