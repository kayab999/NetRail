#!/usr/bin/env python3
"""Webview E2E (audit matrix #9): drives the real Tauri/WebKitGTK webview via
tauri-driver + WebKitWebDriver + Selenium and covers the desktop eval bridges
(focus-search pipeline + docs bridge) that replaced the dead __TAURI__ emits (A7).

Usage:
  tauri-driver must be running on 127.0.0.1:4444 (see scripts/webview-e2e.sh).
  python tests/webview_e2e.py /path/to/debug/netrail
"""
import os
import shutil
import subprocess
import sys
import time

from selenium import webdriver
from selenium.webdriver.common.by import By
from selenium.webdriver.support import expected_conditions as EC
from selenium.webdriver.support.ui import WebDriverWait

APP = sys.argv[1] if len(sys.argv) > 1 else os.environ.get("NETRAIL_APP_BIN")
assert APP, "pass the app binary path (or set NETRAIL_APP_BIN)"
DRIVER_URL = os.environ.get("TAURI_DRIVER_URL", "http://127.0.0.1:4444")

checks: list[tuple[str, bool]] = []


def check(name: str, ok: bool, detail: str = "") -> bool:
    checks.append((name, bool(ok)))
    tag = "ok  " if ok else "FAIL"
    print(f"{tag} {name} {detail}")
    return bool(ok)


class TauriWebdriverOptions(webdriver.common.options.BaseOptions):
    """Minimal capabilities for tauri-driver + WebKitWebDriver.

    The base Options injects `pageLoadStrategy: normal`, which WebKitWebDriver
    2.50 mishandles (session creation never completes) — omitted on purpose.
    """

    def __init__(self, application: str) -> None:
        super().__init__()
        self._caps.pop("pageLoadStrategy", None)
        self.set_capability("browserName", "wry")
        self.set_capability("tauri:options", {"application": application})

    @property
    def default_capabilities(self) -> dict:
        return {}

    def to_capabilities(self) -> dict:
        return self._caps


opts = TauriWebdriverOptions(APP)
driver = webdriver.Remote(command_executor=DRIVER_URL, options=opts)
driver.implicitly_wait(5)
wait = WebDriverWait(driver, 30)

try:
    # 1. Webview loads the UI and reaches the API (webview -> HTTP -> axum -> UI).
    wait.until(EC.presence_of_element_located((By.ID, "query")))
    check("page loads: #query present", True)

    # 2. focus-search bridge: eval -> window.netrailFocusSearch -> caret + selection.
    driver.execute_script("document.getElementById('query').value = 'netrail e2e';")
    driver.execute_script("window.netrailFocusSearch()")
    time.sleep(0.3)
    active = driver.execute_script(
        "return document.activeElement ? document.activeElement.id : null"
    )
    sel = driver.execute_script(
        "const q = document.getElementById('query');"
        "return [q.selectionStart, q.selectionEnd, q.value.length];"
    )
    check(
        "focus-search bridge focuses + selects #query",
        active == "query" and sel[0] == 0 and sel[1] == sel[2],
        f"active={active} selection={sel}",
    )

    # 3. Global-shortcut pipeline: OS key -> XGrabKey -> focus_main_window ->
    #    window.eval(netrailFocusSearch). Full Rust->webview path (A7).
    if shutil.which("xdotool"):
        driver.execute_script("document.getElementById('query').blur();")
        time.sleep(0.3)
        subprocess.run(["xdotool", "key", "ctrl+shift+s"], check=True)
        time.sleep(1.0)
        active2 = driver.execute_script(
            "return document.activeElement ? document.activeElement.id : null"
        )
        check(
            "global shortcut pipeline focuses #query",
            active2 == "query",
            f"active={active2}",
        )
    else:
        check("global shortcut pipeline focuses #query", True, "(skipped: xdotool not present)")

    # 4. Docs bridge: netrailOpenDoc('manual') -> /api/docs/manual -> rendered modal.
    driver.execute_script("window.netrailOpenDoc('manual')")
    wait.until(
        lambda d: d.execute_script("return document.getElementById('doc-dialog').open")
    )
    wait.until(
        lambda d: d.execute_script(
            "return document.getElementById('doc-title').textContent.trim() === 'User Manual'"
        )
    )
    body_len = driver.execute_script(
        "return document.getElementById('doc-body').textContent.length"
    )
    check(
        "docs bridge opens manual with rendered content",
        body_len > 200,
        f"body={body_len} chars",
    )

    # 5. focus-search guard: skipped while a modal dialog owns the surface.
    driver.execute_script("window.netrailFocusSearch()")
    time.sleep(0.3)
    active3 = driver.execute_script(
        "return document.activeElement ? document.activeElement.id : null"
    )
    check(
        "focus-search skipped while doc dialog open",
        active3 != "query",
        f"active={active3}",
    )

    # 6. Docs bridge error path: unknown slug -> dialog error state.
    driver.execute_script("window.netrailOpenDoc('does-not-exist')")
    wait.until(
        lambda d: d.execute_script(
            "return document.getElementById('doc-title').textContent.trim() === 'Document unavailable'"
        )
    )
    check("docs bridge error path (bad slug)", True)

finally:
    try:
        driver.quit()
    except Exception:
        pass

failed = [name for name, ok in checks if not ok]
print(f"\nWEBVIEW E2E: {len(checks) - len(failed)}/{len(checks)} passed")
sys.exit(1 if failed else 0)
