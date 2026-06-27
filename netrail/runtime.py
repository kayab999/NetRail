from __future__ import annotations

import os
import sys
from pathlib import Path


def is_flatpak() -> bool:
    return os.path.exists("/.flatpak-info")


def is_frozen() -> bool:
    return getattr(sys, "frozen", False)


def static_dir() -> Path:
    if is_frozen():
        return Path(sys._MEIPASS) / "netrail" / "static"
    return Path(__file__).resolve().parent / "static"


def package_root() -> Path:
    if is_frozen():
        return Path(sys._MEIPASS)
    return Path(__file__).resolve().parent.parent