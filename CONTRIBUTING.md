# Contributing to NetRail

## How to contribute

1. Fork the repository.
2. Create a feature branch (`git checkout -b feature/short-name`).
3. Make your change with tests (see gates below).
4. Run the full verification gate.
5. Push to your fork and open a pull request against `main`.

## Development setup

```bash
git clone https://github.com/kayab999/NetRail.git && cd NetRail
./install.sh            # Python fallback path; desktop needs npm + Tauri deps
netrail-launch
```

Headless API during development:

```bash
cargo build --manifest-path src-tauri/Cargo.toml --bin netrail-api --no-default-features
NETRAIL_API_TOKEN="" ./src-tauri/target/debug/netrail-api
```

## Required gates (every PR)

```bash
bash scripts/check-versions.sh
source .venv/bin/activate && pytest tests/ -q
cd src-tauri && cargo clippy --all-targets -- -D warnings && cargo test
```

A PR that fails any gate will not be merged. When adding user-visible
behavior, extend the relevant shared fixture in `tests/fixtures/` (URL
policy, merge normalization, cipher state) so both stacks stay in parity.

## Code style

- **Rust:** existing patterns; `cargo clippy --all-targets -- -D warnings` is the gate.
- **Python:** existing patterns; no formatter is enforced.
- **JavaScript:** vanilla JS only (no build step). Untrusted data must go
  through `escapeHtml()` / `encodeURIComponent` / `textContent` — see
  `tests/test_ui_xss.py`.
- **Docs:** keep `SECURITY.md`, `docs/DISTRIBUTION.md` and `docs/API_ERRORS.md`
  in sync with behavior changes, in the same commit.

## Version discipline

Product version is a single source of truth enforced by
`scripts/check-versions.sh` (`package.json`, `Cargo.toml`,
`tauri.conf.json`, `netrail/__init__.py`, `config.rs` plus prose spots).
Do not bump versions in feature PRs — releases do that.

## Security reports

Do **not** open a public issue for vulnerabilities. See `SECURITY.md` for
how to report privately.
