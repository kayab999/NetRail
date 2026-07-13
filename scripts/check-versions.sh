#!/usr/bin/env bash
# Fail if product version strings drift across package manifests.
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
if [[ "$ok" -ne 1 ]]; then
  exit 1
fi
echo "OK: all versions are $expected"
