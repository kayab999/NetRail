from __future__ import annotations

import json
import os
from pathlib import Path
from typing import Any

CONFIG_DIR = Path.home() / ".config" / "netrail"
CONFIG_FILE = CONFIG_DIR / "settings.json"

DEFAULT_BACKENDS: list[dict[str, Any]] = [
    {"id": "searxng", "enabled": True, "url": None},
    {"id": "ddgs", "enabled": True},
    {
        "id": "brave",
        "enabled": False,
        "api_key_env": "BRAVE_SEARCH_API_KEY",
    },
]

DEFAULTS: dict[str, Any] = {
    "browser_id": None,
    "private_mode": False,
    "max_results": 25,
    "backend_order": ["searxng", "ddgs", "brave"],
    "ddgs_enabled": True,
    "searxng_url": None,
    "brave_enabled": False,
    "search_strategy": "fanout",
    "backends": DEFAULT_BACKENDS,
    "history_enabled": True,
    "history_encrypt": True,
    "history_ttl_days": 90,
}


def _as_bool(value: str) -> bool:
    return value.lower() in {"1", "true", "yes", "on"}


def _apply_env_overrides(settings: dict[str, Any]) -> dict[str, Any]:
    if url := os.environ.get("NETRAIL_SEARXNG_URL") or os.environ.get("SEARXNG_URL"):
        settings["searxng_url"] = url

    if os.environ.get("BRAVE_SEARCH_API_KEY") or os.environ.get("NETRAIL_BRAVE_API_KEY"):
        settings["brave_enabled"] = True
        for backend in settings.get("backends", []):
            if backend.get("id") == "brave":
                backend["enabled"] = True
        order = settings.setdefault("backend_order", [])
        if "brave" not in order:
            order.append("brave")

    if raw := os.environ.get("NETRAIL_BRAVE_ENABLED"):
        settings["brave_enabled"] = _as_bool(raw)

    if raw := os.environ.get("NETRAIL_SEARCH_STRATEGY"):
        lower = raw.lower()
        if lower in {"fanout", "fallback"}:
            settings["search_strategy"] = lower

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
    for key, value in data.items():
        if key in DEFAULTS:
            merged[key] = value
    if not merged.get("backends"):
        merged["backends"] = [dict(item) for item in DEFAULT_BACKENDS]
    return _apply_env_overrides(merged)


def save_settings(settings: dict[str, Any]) -> dict[str, Any]:
    CONFIG_DIR.mkdir(parents=True, exist_ok=True)
    payload = DEFAULTS.copy()
    for key in DEFAULTS:
        if key in settings:
            payload[key] = settings[key]
    CONFIG_FILE.write_text(json.dumps(payload, indent=2) + "\n", encoding="utf-8")
    return _apply_env_overrides(payload)