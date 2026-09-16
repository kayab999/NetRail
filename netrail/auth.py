"""API token (NETRAIL_API_TOKEN). Parity with Rust auth.rs.

Desktop: optional — when unset, behavior is unchanged (v1 single-user
model). Headless (`python -m netrail`/Docker): required — `main()` exits
before serving unless the operator sets a token or explicitly opts out with
`NETRAIL_API_TOKEN=""` (see `headless_token_gate`).
"""

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


def _tokens_match(presented: str, expected: str) -> bool:
    """Constant-time compare that stays on the 401 path for non-ASCII input.

    hmac.compare_digest(str, str) raises TypeError on non-ASCII; Rust hashes
    bytes and never leaves the AUTH_REQUIRED path.
    """
    try:
        return hmac.compare_digest(presented.encode("utf-8"), expected.encode("utf-8"))
    except (TypeError, ValueError):
        return False


def check_request_token(authorization: str | None, x_token: str | None) -> None:
    expected = api_token_from_env()
    if not expected:
        return
    if authorization:
        auth = authorization.strip()
        for prefix in ("Bearer ", "bearer "):
            if auth.startswith(prefix) and _tokens_match(auth[len(prefix) :].strip(), expected):
                return
    if x_token and _tokens_match(x_token.strip(), expected):
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


HEADLESS_TOKEN_REQUIRED_MSG = """ERROR: netrail-api requires an API token for authentication.

Generate one:
  openssl rand -hex 32

Then set it:
  export NETRAIL_API_TOKEN="your-generated-token"
  # or in Docker:
  docker run -e NETRAIL_API_TOKEN="your-token" ...

For testing/CI where auth is unnecessary (localhost-only):
  export NETRAIL_API_TOKEN=""
  # WARNING: runs without authentication — any local process can access the API

See docs/DISTRIBUTION.md for persistent token setup (systemd EnvironmentFile, Docker Compose)."""

HEADLESS_TOKEN_EMPTY_WARNING = (
    "WARNING: NETRAIL_API_TOKEN is explicitly empty — running WITHOUT "
    "authentication. Any local process can read/write the API. "
    "Set a token for any shared host."
)


def headless_token_gate() -> None:
    """Headless startup gate (parity with Rust `headless_token_gate`).

    Unset → `SystemExit(1)` after printing setup instructions to stderr.
    Explicitly empty/whitespace → warn to stderr and run (testing/CI
    escape hatch). Non-empty → run with token auth enforced.
    """
    import sys

    raw = os.environ.get("NETRAIL_API_TOKEN")
    if raw is None:
        print(HEADLESS_TOKEN_REQUIRED_MSG, file=sys.stderr)
        raise SystemExit(1)
    if not raw.strip():
        print(HEADLESS_TOKEN_EMPTY_WARNING, file=sys.stderr)
        return
    return
