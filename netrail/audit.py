"""Optional local audit log (NETRAIL_AUDIT_LOG / NETRAIL_AUDIT_LOG_PATH).

Rotation (A5): the active file is capped at NETRAIL_AUDIT_MAX_BYTES (default
10 MiB); on overflow it is rotated to <path>.1, shifting older generations up
to NETRAIL_AUDIT_MAX_FILES (default 3). Set max files to 0 to disable.
"""

from __future__ import annotations

import json
import os
from datetime import datetime, timezone
from pathlib import Path

_DEFAULT_MAX_BYTES = 10 * 1024 * 1024
_DEFAULT_MAX_FILES = 3

_resolved: Path | None | bool = False  # False = unset


def _path() -> Path | None:
    global _resolved
    if _resolved is not False:
        return _resolved if isinstance(_resolved, Path) else None

    if path := os.environ.get("NETRAIL_AUDIT_LOG_PATH", "").strip():
        _resolved = Path(path)
        return _resolved

    raw = os.environ.get("NETRAIL_AUDIT_LOG", "0")
    if raw in {"0", "false", "False", "FALSE", ""}:
        _resolved = None
        return None

    base = Path(os.environ.get("XDG_DATA_HOME", Path.home() / ".local" / "share"))
    directory = base / "netrail"
    directory.mkdir(parents=True, exist_ok=True)
    _resolved = directory / "audit.log"
    return _resolved


def enabled() -> bool:
    return _path() is not None


def _rotation_limits() -> tuple[int, int]:
    try:
        max_bytes = int(os.environ.get("NETRAIL_AUDIT_MAX_BYTES", str(_DEFAULT_MAX_BYTES)))
    except ValueError:
        max_bytes = _DEFAULT_MAX_BYTES
    try:
        max_files = int(os.environ.get("NETRAIL_AUDIT_MAX_FILES", str(_DEFAULT_MAX_FILES)))
    except ValueError:
        max_files = _DEFAULT_MAX_FILES
    return max_bytes, max_files


def _rotate_if_needed(path: Path, max_bytes: int, max_files: int) -> None:
    if max_files <= 0 or max_bytes <= 0:
        return
    try:
        size = path.stat().st_size
    except OSError:
        return
    if size < max_bytes:
        return
    for i in range(max_files - 1, 0, -1):
        src = Path(f"{path}.{i}")
        dst = Path(f"{path}.{i + 1}")
        if dst.exists():
            dst.unlink(missing_ok=True)
        if src.exists():
            src.rename(dst)
    first = Path(f"{path}.1")
    if first.exists():
        first.unlink(missing_ok=True)
    path.rename(first)


def log_event(action: str, detail: dict | None = None) -> None:
    path = _path()
    if path is None:
        return
    line = {
        "ts": datetime.now(timezone.utc).isoformat(),
        "action": action,
        "detail": detail or {},
    }
    try:
        max_bytes, max_files = _rotation_limits()
        _rotate_if_needed(path, max_bytes, max_files)
        with path.open("a", encoding="utf-8") as handle:
            handle.write(json.dumps(line, ensure_ascii=False) + "\n")
    except OSError:
        pass


def reset_for_tests() -> None:
    global _resolved
    _resolved = False
