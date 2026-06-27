#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
MANIFEST="${ROOT}/packaging/flatpak/io.netrail.NetRail.yml"
BUILD_DIR="${ROOT}/build/flatpak"
REPO_DIR="${BUILD_DIR}/repo"
BUNDLE="${BUILD_DIR}/NetRail.flatpak"

mkdir -p "${BUILD_DIR}"

if ! command -v flatpak-builder >/dev/null 2>&1; then
  echo "flatpak-builder is required. Install: flatpak install flathub org.flatpak.Builder"
  exit 1
fi

flatpak-builder \
  --force-clean \
  --repo="${REPO_DIR}" \
  "${BUILD_DIR}/netrail-build" \
  "${MANIFEST}"

flatpak build-bundle "${REPO_DIR}" "${BUNDLE}" io.netrail.NetRail

echo "Built ${BUNDLE}"
echo "Install: flatpak install --bundle ${BUNDLE}"