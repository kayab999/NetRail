# NetRail v1.4.1 — Trailing-dot security fix + parity harness hardening

**Date:** 2026-08-01  
**Type:** Patch

## Security

- **Trailing-dot (FQDN-root) host bypass closed (dual-stack)** — `127.0.0.1.`, `192.168.1.1.`, `10.0.0.1.`, `0x7f.0.0.1.`, `127.000.000.001.`, `127.0.0.1.:8080` are normalized (percent-decode, lowercase, strip trailing dots) before open-URL and backend-URL policy. Browsers strip the final dot at DNS resolution, so these previously reached loopback/private hosts from search results — live-probed on the Python API (browser spawned to `127.0.0.1.`). Found by the 2026-08-01 adversarial pass (`docs/AUDIT_OPENCODE_ADVERSARIAL_2026-08-01.md`).
- **Trailing-dot DDG wrappers unwrapped** — `duckduckgo.com.` / `duck.com.` redirect hosts now unwrap via `uddg=` before checks (Rust + Python).
- **Typed IPv6 parse errors (Python)** — malformed bracketed hosts (`[::ffff:7f00:1].`) return `OPEN_URL_INVALID` 400 instead of an untyped 500.
- **`strict_backend_urls` trailing-dot bypass closed** — `http://127.0.0.1.:8080` now rejected in strict mode (Python).

## Changed

- **Parity harness is fixture-driven** — `scripts/parity-api-smoke.sh` probes every `open_url` vector in `tests/fixtures/url_policy.json` against the live Rust binary (32 vectors) in addition to Python pytest coverage; fixture additions now gate both stacks automatically.
- **Golden fixture extended** — 13 new vectors (trailing-dot open ×9, strict trailing-dot backend ×2, malformed IPv6 ×1, non-strict allow ×1) plus a `strict` field honored by both harnesses.
- **Docs** — `SECURITY.md` documents the optional-token/UI-inject tradeoff; `docs/DISTRIBUTION.md` recommends token + strict backends + audit for Docker; `docker-compose.yml` passes through strict/audit env on the rust profile and fixes a pre-existing YAML break.
- **Desktop UX** — tray/hotkey/second-instance focus places the caret in `#query` (skipped when modal dialogs are open); result-card grid freezes the ★ / Open action column so short one-line snippets no longer stretch the buttons.

## Verify

```bash
bash scripts/check-versions.sh
source .venv/bin/activate && pytest tests/ -q
cd src-tauri && cargo clippy --all-targets -- -D warnings && cargo test
cargo build --release --bin netrail-api --no-default-features
bash scripts/e2e-api-smoke.sh
bash scripts/parity-api-smoke.sh
```
