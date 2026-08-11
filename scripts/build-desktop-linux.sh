#!/usr/bin/env bash
# Build NetRail Linux desktop + headless release artifacts locally.
# Primary path: Tauri 2 → AppImage + .deb + .rpm + netrail-api
#
# Usage:
#   bash scripts/build-desktop-linux.sh
#   bash scripts/build-desktop-linux.sh --skip-tests
#
# Output:
#   dist/release/NetRail_*_amd64.AppImage   (requires patchelf)
#   dist/release/NetRail_*_amd64.deb
#   dist/release/NetRail-*-1.x86_64.rpm
#   dist/release/netrail-api
#   dist/release/SBOM.txt
#   dist/release/SHA256SUMS
#
# Environment assumptions (Ubuntu 24.04-class):
#   - rustc / cargo (1.84+), node 20+, npm, python3
#   - sudo apt install: libwebkit2gtk-4.1-dev libayatana-appindicator3-dev \
#       librsvg2-dev libssl-dev libgtk-3-dev patchelf
#   - APPIMAGE_EXTRACT_AND_RUN=1 recommended (no FUSE required to run)
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

SKIP_TESTS=0
for arg in "$@"; do
  case "$arg" in
    --skip-tests) SKIP_TESTS=1 ;;
    -h|--help)
      sed -n '2,25p' "$0"
      exit 0
      ;;
  esac
done

echo "==> NetRail Linux desktop packaging"
echo "    root: $ROOT"

bash scripts/check-versions.sh

if ! command -v npm >/dev/null; then
  echo "ERROR: npm required" >&2
  exit 1
fi
if ! command -v cargo >/dev/null; then
  echo "ERROR: cargo required" >&2
  exit 1
fi

if ! command -v patchelf >/dev/null; then
  echo "WARN: patchelf not found — AppImage bundling will fail."
  echo "      Install: sudo apt install patchelf"
  echo "      .deb / netrail-api may still build."
fi

if [[ ! -d node_modules/@tauri-apps/cli ]]; then
  echo "==> npm ci"
  npm ci
fi

if [[ "$SKIP_TESTS" -eq 0 ]]; then
  echo "==> Tests + clippy"
  if [[ -f .venv/bin/activate ]]; then
    # shellcheck disable=SC1091
    source .venv/bin/activate
  fi
  if command -v pytest >/dev/null; then
    pytest tests/ -q
  else
    echo "WARN: pytest not found; skipping Python tests"
  fi
  (
    cd src-tauri
    cargo clippy --all-targets -- -D warnings
    cargo test
  )
else
  echo "==> Skipping tests (--skip-tests)"
fi

echo "==> Headless netrail-api (release)"
cargo build --release --manifest-path src-tauri/Cargo.toml \
  --bin netrail-api --no-default-features

echo "==> SBOM for package embeds"
mkdir -p src-tauri/sbom
bash scripts/generate-sbom.sh src-tauri/sbom/SBOM.txt

echo "==> Tauri desktop bundle (AppImage / deb / rpm)"
export APPIMAGE_EXTRACT_AND_RUN="${APPIMAGE_EXTRACT_AND_RUN:-1}"
# linuxdeploy strip fails on .relr.dyn sections on ubuntu-24.04+ libs
# ("failed to run linuxdeploy" — NR-16; tauri-apps/tauri#14796/#8929/#13113).
export NO_STRIP="${NO_STRIP:-true}"
npm run build

echo "==> Collect dist/release"
mkdir -p dist/release
cp -f src-tauri/target/release/netrail-api dist/release/
find src-tauri/target/release/bundle -type f \
  \( -name '*.AppImage' -o -name '*.deb' -o -name '*.rpm' \) \
  -exec cp -f {} dist/release/ \;

bash scripts/generate-sbom.sh dist/release/SBOM.txt
if [[ -f src-tauri/sbom/SBOM.txt ]]; then
  cmp -s dist/release/SBOM.txt src-tauri/sbom/SBOM.txt \
    || echo "WARN: SBOM drift between embed path and release path"
fi

(
  cd dist/release
  # Exclude the previous SHA256SUMS from the glob so re-runs stay idempotent
  # (a self-referenced checksum line can never verify).
  sha256sum -- $(find . -maxdepth 1 -type f ! -name SHA256SUMS | sort) >SHA256SUMS
)

echo ""
echo "==> Artifacts in dist/release/"
ls -lh dist/release/ || true

APPIMAGE="$(find dist/release -maxdepth 1 -name '*.AppImage' -print -quit || true)"
DEB="$(find dist/release -maxdepth 1 -name '*.deb' -print -quit || true)"

if [[ -z "${APPIMAGE}" ]]; then
  echo "WARN: AppImage missing (install patchelf and re-run)"
else
  echo "AppImage: ${APPIMAGE}"
  echo "  chmod +x \"${APPIMAGE}\""
  echo "  APPIMAGE_EXTRACT_AND_RUN=1 \"${APPIMAGE}\""
fi
if [[ -z "${DEB}" ]]; then
  echo "WARN: .deb missing"
else
  echo "deb:     ${DEB}"
fi
echo "API:     dist/release/netrail-api"
echo "sums:    dist/release/SHA256SUMS"
echo ""
echo "Done."
