# NetRail v1.6.3 — Reproducibility & supply chain

**Date:** 2026-08-02
**Type:** Patch — strictly scoped to reproducibility, test coverage, and supply chain. No architecture changes, no new APIs, no security-model changes.

## Added

- **SBOM pinned in bundle (E2)** — the Rust dependency inventory is now embedded in every binary at build time (`build.rs` derives it from `Cargo.lock`; `netrail-api --sbom` prints it, verified byte-identical to the Rust section of the shipped `SBOM.txt`), and the full `SBOM.txt` is packaged into the `.deb` / `.rpm` / AppImage at `/usr/share/netrail/SBOM.txt`. Release CI generates it before the bundle build and asserts presence in both deb and rpm (`dpkg-deb -c` / `rpm -qlp`). Also fixed a latent generator bug: the lockfile's top-level `version =` line was emitted as a bare `@4` entry in past `SBOM.txt` assets. Script: `scripts/generate-sbom.sh` (single source for the release asset and the bundled copy).
- **Golden fixture growth (E5)** — `tests/fixtures/url_policy.json` (the policy SSOT driving Rust, Python, and the live parity harness) grew from 43 to 68 vectors: IPv6 loopback/link-local/ULA/IPv4-mapped forms, percent-encoded and uppercase-scheme loopbacks, `localhost` hostname (+ trailing dot), `0.0.0.0`, `ftp:`/`file:` schemes, xip.io subdomain rebinding, double-encoded DDG unwrap, cloud-metadata IP, plus IPv6 backend vectors including strict mode. Each vector was verified consistent on both stacks before committing. Fixed a real dual-stack divergence found during growth: `ftp://` (and any non-http(s) scheme) returned `OPEN_URL_INVALID` in Python but `OPEN_URL_INVALID_SCHEME` in Rust — Python now matches Rust (empty URL still `OPEN_URL_INVALID`). The live parity harness now also probes `backend_url` vectors against the running Rust binary via the settings-update path.
- **CSS regression guard (E3)** — `tests/test_ui_css.py` pins the `.result-card` grid contract (`minmax(0, 1fr) auto` desktop, `96px minmax(0, 1fr) auto` image-card, action column always `auto`, `720px` collapse to `1fr`) as a CI-gated structural check. Verified non-vacuous.
- **Release assurance doc** — `docs/RELEASE_ASSURANCE.md`: a non-technical map of "what guarantees does NetRail offer and where are they backed" (security, resilience, concurrency, performance, quality, supply chain) plus the release-identity discipline table.

## Verify

```bash
bash scripts/check-versions.sh
source .venv/bin/activate && pytest tests/ -q        # 162 passed
cd src-tauri && cargo clippy --all-targets -- -D warnings && cargo test  # 113 passed
cargo build --release --bin netrail-api --no-default-features
./src-tauri/target/release/netrail-api --sbom          # embedded Rust inventory
bash scripts/e2e-api-smoke.sh
bash scripts/parity-api-smoke.sh                       # 48 open_url + 17 backend_url live vectors
```

## Backlog (deferred to later cycles — not part of this release's scope)

- DNS resolve-and-warn flag (C3), images-off flag (C4), multi-user/RBAC, egress proxy/TLS pinning, metrics/SLO, Windows/macOS ports, on-device LLM, MCP.
