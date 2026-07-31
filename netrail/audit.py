"""Optional local audit log (NETRAIL_AUDIT_LOG / NETRAIL_AUDIT_LOG_PATH)."""

from __future__ import annotations

import json
import os
from datetime import datetime, timezone
from pathlib import Path

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
        with path.open("a", encoding="utf-8") as handle:
            handle.write(json.dumps(line, ensure_ascii=False) + "\n")
    except OSError:
        pass


def reset_for_tests() -> None:
    global _resolved
    _resolved = False
