#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
BUILD="${ROOT}/build/appimage"
APPDIR="${BUILD}/NetRail.AppDir"

command -v pyinstaller >/dev/null || {
  echo "PyInstaller required: pip install pyinstaller"
  exit 1
}

rm -rf "${BUILD}"
mkdir -p "${APPDIR}"

cd "${ROOT}"
pyinstaller --noconfirm --clean packaging/appimage/netrail.spec --distpath "${BUILD}/dist" --workpath "${BUILD}/work"

install -Dm755 "${BUILD}/dist/netrail" "${APPDIR}/netrail"
install -Dm644 "${ROOT}/assets/netrail.desktop" "${APPDIR}/netrail.desktop"
install -Dm644 "${ROOT}/assets/netrail.svg" "${APPDIR}/netrail.svg"

cat > "${APPDIR}/AppRun" <<'EOF'
#!/bin/sh
HERE="$(dirname "$(readlink -f "$0")")"
export NETRAIL_AUTO_OPEN="${NETRAIL_AUTO_OPEN:-true}"
exec "${HERE}/netrail"
EOF
chmod +x "${APPDIR}/AppRun"

if command -v appimagetool >/dev/null 2>&1; then
  ARCH="$(uname -m)"
  OUT="${BUILD}/NetRail-${ARCH}.AppImage"
  appimagetool "${APPDIR}" "${OUT}"
  echo "Built ${OUT}"
else
  echo "AppDir ready at ${APPDIR}"
  echo "Install appimagetool to bundle: https://github.com/AppImage/AppImageKit"
fi