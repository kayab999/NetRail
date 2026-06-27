#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")"

if [[ ! -d .venv ]]; then
  python3 -m venv .venv
  .venv/bin/pip install -r requirements.txt
fi

echo "NetRail starting at http://127.0.0.1:7421"
echo "No telemetry. Press Ctrl+C to stop."
exec .venv/bin/python -m netrail