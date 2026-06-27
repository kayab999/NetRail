from __future__ import annotations

import configparser
import os
import shutil
import subprocess
from dataclasses import dataclass
from pathlib import Path

from netrail.runtime import is_flatpak


@dataclass(frozen=True)
class Browser:
    id: str
    name: str
    executable: str
    private_flag: str | None


DESKTOP_DIRS = [
    Path("/usr/share/applications"),
    Path.home() / ".local/share/applications",
]

KNOWN_BROWSERS: dict[str, tuple[str, str | None]] = {
    "firefox": ("Firefox", "--private-window"),
    "firefox-esr": ("Firefox ESR", "--private-window"),
    "google-chrome": ("Google Chrome", "--incognito"),
    "google-chrome-stable": ("Google Chrome", "--incognito"),
    "chromium": ("Chromium", "--incognito"),
    "chromium-browser": ("Chromium", "--incognito"),
    "brave-browser": ("Brave", "--incognito"),
    "microsoft-edge": ("Microsoft Edge", "--inprivate"),
    "microsoft-edge-stable": ("Microsoft Edge", "--inprivate"),
    "opera": ("Opera", "--private"),
    "vivaldi": ("Vivaldi", "--incognito"),
    "waterfox": ("Waterfox", "--private-window"),
    "librewolf": ("LibreWolf", "--private-window"),
}


def _parse_desktop_file(path: Path) -> tuple[str, str, list[str]] | None:
    parser = configparser.ConfigParser(interpolation=None)
    parser.optionxform = str
    try:
        parser.read(path, encoding="utf-8")
    except (configparser.Error, OSError):
        return None

    if "Desktop Entry" not in parser:
        return None

    section = parser["Desktop Entry"]
    if section.get("Type") != "Application":
        return None
    if section.get("NoDisplay", "false").lower() == "true":
        return None

    name = section.get("Name", path.stem)
    exec_line = section.get("Exec", "")
    executable = exec_line.split("%")[0].strip()
    if not executable:
        return None

    categories = [c.strip() for c in section.get("Categories", "").split(";") if c.strip()]
    mime_types = [m.strip() for m in section.get("MimeType", "").split(";") if m.strip()]
    return name, executable, categories + mime_types


def _is_browser(meta: list[str]) -> bool:
    joined = " ".join(meta).lower()
    return "webbrowser" in joined or "x-scheme-handler/http" in joined


def _host_which(command: str) -> str | None:
    token = Path(command.split()[0]).name
    try:
        result = subprocess.run(
            ["flatpak-spawn", "--host", "which", token],
            capture_output=True,
            text=True,
            timeout=5,
            check=False,
        )
    except (OSError, subprocess.TimeoutExpired):
        return None
    if result.returncode == 0 and result.stdout.strip():
        return result.stdout.strip()
    return None


def _resolve_executable(command: str) -> str | None:
    token = command.split()[0]
    if is_flatpak():
        if token.startswith("/"):
            return token
        return _host_which(command) or token

    resolved = shutil.which(Path(token).name) or shutil.which(token)
    return resolved


def _spawn_process(cmd: list[str], env: dict[str, str]) -> None:
    if is_flatpak():
        cmd = ["flatpak-spawn", "--host", *cmd]
    subprocess.Popen(
        cmd,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
        start_new_session=True,
        env=env,
    )


def discover_browsers() -> list[Browser]:
    seen: set[str] = set()
    browsers: list[Browser] = []

    for desktop_dir in DESKTOP_DIRS:
        if not desktop_dir.is_dir():
            continue
        for desktop_file in sorted(desktop_dir.glob("*.desktop")):
            parsed = _parse_desktop_file(desktop_file)
            if not parsed:
                continue
            name, command, meta = parsed
            if not _is_browser(meta):
                continue

            resolved = _resolve_executable(command)
            if not resolved or resolved in seen:
                continue

            stem = Path(resolved).name
            display_name, private_flag = KNOWN_BROWSERS.get(stem, (name, "--incognito"))
            browser_id = stem

            seen.add(resolved)
            browsers.append(
                Browser(
                    id=browser_id,
                    name=display_name,
                    executable=resolved,
                    private_flag=private_flag,
                )
            )

    for stem, (display_name, private_flag) in KNOWN_BROWSERS.items():
        if is_flatpak():
            resolved = _host_which(stem)
        else:
            resolved = shutil.which(stem)
        if resolved and resolved not in seen:
            seen.add(resolved)
            browsers.append(
                Browser(
                    id=stem,
                    name=display_name,
                    executable=resolved,
                    private_flag=private_flag,
                )
            )

    browsers.sort(key=lambda b: b.name.lower())
    return browsers


def find_browser(browser_id: str | None) -> Browser | None:
    browsers = discover_browsers()
    if not browsers:
        return None
    if browser_id:
        for browser in browsers:
            if browser.id == browser_id:
                return browser
    return browsers[0]


def open_url(url: str, browser_id: str | None = None, private_mode: bool = False) -> dict[str, str]:
    browser = find_browser(browser_id)
    if not browser:
        raise RuntimeError("No web browser found on this system.")

    cmd = [browser.executable]
    if private_mode and browser.private_flag:
        cmd.append(browser.private_flag)
    cmd.append(url)

    env = os.environ.copy()
    env.pop("LD_PRELOAD", None)

    _spawn_process(cmd, env)

    mode = "private" if private_mode and browser.private_flag else "normal"
    return {
        "browser": browser.name,
        "executable": browser.executable,
        "mode": mode,
        "url": url,
        "sandbox": "flatpak-host" if is_flatpak() else "native",
    }