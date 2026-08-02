# NetRail — Linux packaging & distribution

This document is the **packaging SSOT** for public GitHub Releases. Product features are out of scope here.

## Approach (why Tauri AppImage)

| Choice | Rationale |
|--------|-----------|
| **Primary: Tauri 2 AppImage** | One double-click binary; bundles WebKit/GTK runtime bits needed by the desktop shell; works without a system-wide Rust/Python install |
| **Secondary: `.deb` / `.rpm`** | Native package managers; same Tauri build (`targets: all`) |
| **Also: `netrail-api`** | ~static headless binary for servers/homelabs (no GUI toolkit required at runtime) |
| **Not primary: Python PyInstaller AppImage** | Legacy path in `packaging/appimage/` — kept for historical Flatpak-adjacent experiments; **do not ship** as the GitHub desktop artifact |
| **Optional: Flatpak (Python)** | Sandboxed compatibility path; not the production desktop engine |

User data stays **outside** the bundle (XDG):

| Path | Contents |
|------|----------|
| `~/.config/netrail/settings.json` | Settings |
| `~/.local/share/netrail/netrail.db` | History / collections |
| `NETRAIL_DB_KEY` / OS keyring | Encryption key (never baked into the image) |

No telemetry in packaging or runtime.

## Filled packaging template

| Field | Value |
|-------|--------|
| Application name | **NetRail** |
| Language stack | **Rust** (Axum + Tauri 2 primary), Python (Docker/Flatpak/tests) |
| UI framework | **Tauri 2** webview + vanilla `netrail/static` HTML/CSS/JS |
| Target | Linux desktop (x86_64) |
| Default entry | GUI (`netrail`); CLI/headless via `netrail-api` or `netrail --api-only` |
| Primary artifact | **AppImage** |
| Secondary | `.deb`, `.rpm`, `netrail-api` |
| Exec | `netrail` |
| Icon | `netrail` (from `src-tauri/icons/`) |
| Categories | Utility (+ Network keywords); FreeDesktop template in `packaging/linux/netrail.desktop.hbs` |

## One-command local release build

```bash
# System deps (Ubuntu 24.04 example)
sudo apt-get install -y \
  libwebkit2gtk-4.1-dev libayatana-appindicator3-dev \
  librsvg2-dev libssl-dev libgtk-3-dev patchelf

# Optional: Node 20+, Rust stable, Python venv with requirements.txt
npm ci
bash scripts/build-desktop-linux.sh
# or skip gates while iterating packaging only:
# bash scripts/build-desktop-linux.sh --skip-tests
```

**Artifacts:** `dist/release/`

| File (examples) | Role |
|-----------------|------|
| `NetRail_<ver>_amd64.AppImage` | Double-click desktop app |
| `NetRail_<ver>_amd64.deb` | Debian/Ubuntu install |
| `NetRail-<ver>-1.x86_64.rpm` | Fedora/RHEL-class install |
| `netrail-api` | Headless API only |
| `SBOM.txt` / `SHA256SUMS` | Supply chain |

GitHub Release CI (`.github/workflows/release.yml`) runs the same pipeline on tags `v*` and cosign-signs `SHA256SUMS`.

## Run the AppImage

```bash
chmod +x NetRail_*_amd64.AppImage
# Ubuntu 24.04+ without FUSE:
APPIMAGE_EXTRACT_AND_RUN=1 ./NetRail_*_amd64.AppImage
```

- **GUI** starts by default (tray + webview → `http://127.0.0.1:7421`).
- **Single-instance** is enforced by the Tauri desktop shell.
- **CLI / headless:** `./netrail-api` or `./netrail --api-only` (desktop binary).

## Desktop integration

| Item | Source |
|------|--------|
| `.desktop` template (deb/rpm) | `packaging/linux/netrail.desktop.hbs` |
| Category / descriptions | `src-tauri/tauri.conf.json` → `bundle.category`, `shortDescription`, `longDescription` |
| Icons | `src-tauri/icons/*` |
| install.sh local desktop entry | `assets/netrail.desktop` → `Exec=netrail-launch` |

## Known caveats

1. **patchelf** is required to produce the AppImage; without it, prefer `.deb` from CI.
2. **FUSE** optional: use `APPIMAGE_EXTRACT_AND_RUN=1`.
3. **WebKitGTK** is the UI surface — GPU/driver quirks are host/WebKit issues, not NetRail product bugs.
4. **Flatpak/Python** is compatibility only; production desktop is Rust/Tauri.
5. **Legacy** `packaging/appimage/build.sh` (PyInstaller) is **not** the release path.

## Verify a release tree

```bash
cd dist/release
sha256sum -c SHA256SUMS
# if signed:
# cosign verify-blob --certificate SHA256SUMS.pem --signature SHA256SUMS.sig \
#   --certificate-identity-regexp '^https://github.com/kayab999/NetRail/.github/workflows/release.yml@refs/tags/v' \
#   --certificate-oidc-issuer 'https://token.actions.githubusercontent.com' \
#   SHA256SUMS
./netrail-api --sbom | head
NETRAIL_RATE_LIMIT=0 bash ../../scripts/e2e-api-smoke.sh ./netrail-api
```

## Related docs

- End-user install: [README.md](../README.md)
- Operator / Docker / env: [docs/DISTRIBUTION.md](../docs/DISTRIBUTION.md)
- Release trust map: [docs/RELEASE_ASSURANCE.md](../docs/RELEASE_ASSURANCE.md)
