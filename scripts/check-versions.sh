#!/usr/bin/env bash
# Fail if product version strings drift across package manifests, prose
# spot-lists, the CHANGELOG top entry, and the git tag of HEAD (QA-06, QA-11).
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

expected="$(python3 -c "import json; print(json.load(open('package.json'))['version'])")"
rust="$(python3 -c "import re; t=open('src-tauri/Cargo.toml').read(); print(re.search(r'^version = \"([^\"]+)\"', t, re.M).group(1))")"
tauri="$(python3 -c "import json; print(json.load(open('src-tauri/tauri.conf.json'))['version'])")"
py="$(python3 -c "import re; t=open('netrail/__init__.py').read(); print(re.search(r'__version__ = \"([^\"]+)\"', t).group(1))")"
cfg="$(python3 -c "import re; t=open('src-tauri/src/config.rs').read(); print(re.search(r'VERSION: &str = \"([^\"]+)\"', t).group(1))")"

echo "package.json     $expected"
echo "Cargo.toml       $rust"
echo "tauri.conf.json  $tauri"
echo "netrail/__init__ $py"
echo "config.rs        $cfg"

ok=1
for v in "$rust" "$tauri" "$py" "$cfg"; do
  if [[ "$v" != "$expected" ]]; then
    echo "ERROR: version drift — expected $expected, got $v" >&2
    ok=0
  fi
done

# Prose spot-lists (QA-06): "file:needle" pairs that must reference $expected.
spot_ok=1
while IFS= read -r spot; do
  file="${spot%%:*}"
  needle="${spot#*:}"
  if ! grep -qF "$needle" "$file"; then
    echo "ERROR: $file missing prose spot '$needle'" >&2
    spot_ok=0
  fi
done <<EOF
docs/ARCHITECTURE.md:NetRail **$expected**
docs/DISTRIBUTION.md:parity matrix ($expected)
docs/MANUAL.md:NetRail_${expected}_amd64.AppImage
SECURITY.md:current: $expected
HANDOVER.md:**v$expected is Latest**
EOF

# CHANGELOG latest entry + git tag of HEAD (QA-11).
changelog_ok=1
if ! grep -qF "## [$expected]" CHANGELOG.md; then
  echo "ERROR: CHANGELOG.md has no entry for v$expected" >&2
  changelog_ok=0
fi
if [[ "$(git tag --points-at HEAD 2>/dev/null | head -1)" != "v$expected" ]]; then
  # Only advisory on non-release commits: main is usually untagged between releases.
  echo "note: HEAD is not tagged v$expected (expected on non-release commits)"
fi

if [[ "$ok" -ne 1 || "$spot_ok" -ne 1 || "$changelog_ok" -ne 1 ]]; then
  exit 1
fi
echo "OK: all versions are $expected"
