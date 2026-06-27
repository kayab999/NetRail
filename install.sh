#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")" && pwd)"
PREFIX="${HOME}/.local"
BIN_DIR="${PREFIX}/bin"
APP_DIR="${PREFIX}/share/applications"
ICON_DIR="${PREFIX}/share/icons/hicolor/scalable/apps"

echo "==> NetRail local install"

if [[ ! -d "${ROOT}/.venv" ]]; then
  python3 -m venv "${ROOT}/.venv"
  "${ROOT}/.venv/bin/pip" install -r "${ROOT}/requirements.txt"
fi

mkdir -p "${BIN_DIR}" "${APP_DIR}" "${ICON_DIR}"

cat > "${BIN_DIR}/netrail-launch" <<EOF
#!/usr/bin/env bash
cd "${ROOT}"
export NETRAIL_AUTO_OPEN="\${NETRAIL_AUTO_OPEN:-true}"
exec "${ROOT}/.venv/bin/python" -m netrail "\$@"
EOF
chmod +x "${BIN_DIR}/netrail-launch"

install -Dm644 "${ROOT}/assets/netrail.desktop" "${APP_DIR}/netrail.desktop"
sed -i "s|Exec=netrail-launch|Exec=${BIN_DIR}/netrail-launch|" "${APP_DIR}/netrail.desktop"

install -Dm644 "${ROOT}/assets/netrail.svg" "${ICON_DIR}/netrail.svg"

if command -v update-desktop-database >/dev/null 2>&1; then
  update-desktop-database "${APP_DIR}" 2>/dev/null || true
fi

echo ""
echo "Installed netrail-launch to ${BIN_DIR}"
echo "Ensure ${BIN_DIR} is in your PATH."
echo "Launch: netrail-launch   or find NetRail in your application menu."
echo "UI:     http://127.0.0.1:7421"