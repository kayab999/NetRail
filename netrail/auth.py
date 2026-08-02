"""Optional API token (NETRAIL_API_TOKEN). Parity with Rust auth.rs."""

from __future__ import annotations

import base64
import hashlib
import hmac
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
            if auth.startswith(prefix) and hmac.compare_digest(
                auth[len(prefix) :].strip(), expected
            ):
                return
    if x_token and hmac.compare_digest(x_token.strip(), expected):
        return
    raise NetRailError(
        "AUTH_REQUIRED",
        "Valid NETRAIL_API_TOKEN required (Authorization: Bearer or X-NetRail-Token).",
        status=401,
    )


def client_identity(authorization: str | None, x_token: str | None) -> str:
    """Stable per-client rate-limit bucket key (A9).

    With token auth on, the key is the SHA-256 of the presented token — never
    the token itself — so each client gets its own per-minute budget. Without
    auth, everything shares one "anonymous" budget per process.
    """
    if not token_required():
        return "anonymous"
    token: str | None = None
    if authorization:
        auth = authorization.strip()
        for prefix in ("Bearer ", "bearer "):
            if auth.startswith(prefix):
                token = auth[len(prefix) :].strip()
                break
    if token is None and x_token:
        token = x_token.strip()
    if not token:
        return "anonymous"
    digest = hashlib.sha256(token.encode()).digest()
    return f"token:{base64.b64encode(digest).decode()}"


def path_requires_token(path: str) -> bool:
    if not token_required():
        return False
    if path == "/api/health":
        return False
    return path.startswith("/api/")
