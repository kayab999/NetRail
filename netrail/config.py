from __future__ import annotations

import json
from pathlib import Path
from typing import Any

CONFIG_DIR = Path.home() / ".config" / "netrail"
CONFIG_FILE = CONFIG_DIR / "settings.json"

DEFAULTS: dict[str, Any] = {
    "browser_id": None,
    "private_mode": False,
    "max_results": 25,
    "backend_order": ["searxng", "ddgs"],
    "ddgs_enabled": True,
    "searxng_url": None,
    "history_enabled": True,
    "history_encrypt": True,
    "history_ttl_days": 90,
}


def load_settings() -> dict[str, Any]:
    if not CONFIG_FILE.exists():
        return DEFAULTS.copy()
    try:
        data = json.loads(CONFIG_FILE.read_text(encoding="utf-8"))
    except (json.JSONDecodeError, OSError):
        return DEFAULTS.copy()
    merged = DEFAULTS.copy()
    merged.update({k: v for k, v in data.items() if k in DEFAULTS})
    return merged


def save_settings(settings: dict[str, Any]) -> dict[str, Any]:
    CONFIG_DIR.mkdir(parents=True, exist_ok=True)
    payload = DEFAULTS.copy()
    payload.update({k: settings[k] for k in DEFAULTS if k in settings})
    CONFIG_FILE.write_text(json.dumps(payload, indent=2) + "\n", encoding="utf-8")
    return payload