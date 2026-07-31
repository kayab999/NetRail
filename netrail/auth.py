"""Optional API token (NETRAIL_API_TOKEN). Parity with Rust auth.rs."""

from __future__ import annotations

import os

from netrail.errors import NetRailError


def api_token_from_env() -> str | None:
    raw = os.environ.get("NETRAIL_API_TOKEN", "").strip()
    return raw or None


def token_required() -> bool:
    return api_token_from_env() is not None


def inject_ui_token() -> bool:
    if not token_required():
        return False
    raw = os.environ.get("NETRAIL_INJECT_UI_TOKEN", "1")
    return raw not in {"0", "false", "False", "FALSE"}


def check_request_token(authorization: str | None, x_token: str | None) -> None:
    expected = api_token_from_env()
    if not expected:
        return
    if authorization:
        auth = authorization.strip()
        for prefix in ("Bearer ", "bearer "):
            if auth.startswith(prefix) and auth[len(prefix) :].strip() == expected:
                return
    if x_token and x_token.strip() == expected:
        return
    raise NetRailError(
        "AUTH_REQUIRED",
        "Valid NETRAIL_API_TOKEN required (Authorization: Bearer or X-NetRail-Token).",
        status=401,
    )


def path_requires_token(path: str) -> bool:
    if not token_required():
        return False
    if path == "/api/health":
        return False
    return path.startswith("/api/")
