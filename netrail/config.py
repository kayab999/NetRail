from __future__ import annotations

import json
import os
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

def _as_bool(value: str) -> bool:
    return value.lower() in {"1", "true", "yes", "on"}


def _apply_env_overrides(settings: dict[str, Any]) -> dict[str, Any]:
    if url := os.environ.get("NETRAIL_SEARXNG_URL") or os.environ.get("SEARXNG_URL"):
        settings["searxng_url"] = url

    if raw := os.environ.get("NETRAIL_HISTORY_ENABLED"):
        settings["history_enabled"] = _as_bool(raw)

    if raw := os.environ.get("NETRAIL_HISTORY_ENCRYPT"):
        settings["history_encrypt"] = _as_bool(raw)

    if raw := os.environ.get("NETRAIL_HISTORY_TTL_DAYS"):
        try:
            settings["history_ttl_days"] = int(raw)
        except ValueError:
            pass

    if raw := os.environ.get("NETRAIL_MAX_RESULTS"):
        try:
            settings["max_results"] = int(raw)
        except ValueError:
            pass

    return settings


def load_settings() -> dict[str, Any]:
    if CONFIG_FILE.exists():
        try:
            data = json.loads(CONFIG_FILE.read_text(encoding="utf-8"))
        except (json.JSONDecodeError, OSError):
            data = {}
    else:
        data = {}

    merged = DEFAULTS.copy()
    merged.update({k: v for k, v in data.items() if k in DEFAULTS})
    return _apply_env_overrides(merged)


def save_settings(settings: dict[str, Any]) -> dict[str, Any]:
    CONFIG_DIR.mkdir(parents=True, exist_ok=True)
    payload = DEFAULTS.copy()
    payload.update({k: settings[k] for k in DEFAULTS if k in settings})
    CONFIG_FILE.write_text(json.dumps(payload, indent=2) + "\n", encoding="utf-8")
    return _apply_env_overrides(payload)