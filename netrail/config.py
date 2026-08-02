from __future__ import annotations

import json
import os
from pathlib import Path
from typing import Any

def config_dir() -> Path:
    # Resolved lazily: tests (and processes) may legitimately change $HOME
    # after import, and module-level Path.home() would freeze the wrong path.
    return Path.home() / ".config" / "netrail"


def config_file() -> Path:
    return config_dir() / "settings.json"

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
    "strict_backend_urls": False,
}


def _as_bool(value: str) -> bool:
    return value.lower() in {"1", "true", "yes", "on"}


def strict_backend_urls_from_env() -> bool:
    return _as_bool(os.environ.get("NETRAIL_STRICT_BACKEND_URLS", "0"))


def readonly_mode() -> bool:
    """NETRAIL_READONLY=1 rejects all mutating API calls with
    403 READONLY_MODE; read endpoints (search, open, docs, export) keep
    working."""
    return _as_bool(os.environ.get("NETRAIL_READONLY", "0"))


def _apply_env_overrides(settings: dict[str, Any]) -> dict[str, Any]:
    if raw := os.environ.get("NETRAIL_STRICT_BACKEND_URLS"):
        settings["strict_backend_urls"] = _as_bool(raw)

    if url := os.environ.get("NETRAIL_SEARXNG_URL") or os.environ.get("SEARXNG_URL"):
        # Same gate as settings save — never apply metadata/rebinding/etc. from env.
        from netrail.errors import NetRailError
        from netrail.security import validate_backend_url

        strict = bool(settings.get("strict_backend_urls")) or strict_backend_urls_from_env()
        try:
            settings["searxng_url"] = validate_backend_url(url, strict=strict)
        except NetRailError:
            # Leave prior settings value; invalid env must not enable a hostile backend.
            pass

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
    if config_file().exists():
        try:
            data = json.loads(config_file().read_text(encoding="utf-8"))
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


def validate_settings(settings: dict[str, Any]) -> None:
    from netrail.errors import NetRailError
    from netrail.security import validate_backend_url

    max_results = int(settings.get("max_results", DEFAULTS["max_results"]))
    if max_results < 1 or max_results > 50:
        raise NetRailError(
            "CONFIG_MAX_RESULTS",
            "max_results must be between 1 and 50.",
        )

    ttl = int(settings.get("history_ttl_days", DEFAULTS["history_ttl_days"]))
    if ttl < 0 or ttl > 3650:
        raise NetRailError(
            "CONFIG_HISTORY_TTL",
            "history_ttl_days must be at most 3650.",
        )

    strategy = settings.get("search_strategy", DEFAULTS["search_strategy"])
    if strategy not in {"fanout", "fallback"}:
        raise NetRailError(
            "CONFIG_SEARCH_STRATEGY",
            "search_strategy must be 'fanout' or 'fallback'.",
        )

    strict = bool(settings.get("strict_backend_urls")) or strict_backend_urls_from_env()
    if url := settings.get("searxng_url"):
        validate_backend_url(url, strict=strict)

    for entry in settings.get("backends") or []:
        if entry_url := entry.get("url"):
            validate_backend_url(entry_url, strict=strict)


def save_settings(settings: dict[str, Any]) -> dict[str, Any]:
    validate_settings(settings)
    config_dir().mkdir(parents=True, exist_ok=True)
    payload = DEFAULTS.copy()
    for key in DEFAULTS:
        if key in settings:
            payload[key] = settings[key]
    config_file().write_text(json.dumps(payload, indent=2) + "\n", encoding="utf-8")
    return _apply_env_overrides(payload)