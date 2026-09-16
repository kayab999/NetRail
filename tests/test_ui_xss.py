"""Structural XSS guard for the vanilla-JS frontend (no JS runner in CI).

Pins two properties of `escapeHtml` (duplicated in `app.js` and
`markdown.js`):
1. The replacement chain covers &, <, >, " and ' — with & first, so later
   entities are not double-escaped.
2. Both copies stay in sync (same entity set), so a fix in one cannot drift
   from the other.

Plus spot-checks that the known untrusted-data sinks (result cards,
history entries, open-state) route through `escapeHtml`/`encodeURIComponent`
rather than raw interpolation. This is intentionally static: proportional to
the risk (all current sinks are double-quoted attributes or text content).
"""

from __future__ import annotations

import re
from pathlib import Path

STATIC = Path(__file__).parent.parent / "netrail" / "static"
APP_JS = (STATIC / "app.js").read_text(encoding="utf-8")
MARKDOWN_JS = (STATIC / "markdown.js").read_text(encoding="utf-8")

# char -> required output entity
REQUIRED_ENTITIES = {
    "&": "&amp;",
    "<": "&lt;",
    ">": "&gt;",
    '"': "&quot;",
    "'": "&#39;",
}


def _escape_fn_body(source: str, filename: str) -> str:
    """Extract the `escapeHtml` function body via brace matching."""
    start = source.find("function escapeHtml(")
    assert start != -1, f"{filename} must define escapeHtml"
    brace = source.find("{", start)
    depth = 0
    for i in range(brace, len(source)):
        if source[i] == "{":
            depth += 1
        elif source[i] == "}":
            depth -= 1
            if depth == 0:
                return source[brace : i + 1]
    raise AssertionError(f"{filename}: unterminated escapeHtml")


def test_escape_chain_covers_all_five_chars():
    for source, filename in ((APP_JS, "app.js"), (MARKDOWN_JS, "markdown.js")):
        body = _escape_fn_body(source, filename)
        for char, entity in REQUIRED_ENTITIES.items():
            assert entity in body, f"{filename}: escapeHtml must emit {entity}"
        # `&` must be replaced first — otherwise `&lt;` becomes `&amp;lt;`.
        assert body.find("&amp;") < body.find("&lt;"), f"{filename}: & must come first"


def test_escape_copies_stay_in_sync():
    app_entities = {e for e in REQUIRED_ENTITIES.values() if e in _escape_fn_body(APP_JS, "app.js")}
    md_entities = {
        e for e in REQUIRED_ENTITIES.values() if e in _escape_fn_body(MARKDOWN_JS, "markdown.js")
    }
    assert app_entities == md_entities == set(REQUIRED_ENTITIES.values())


def test_result_card_sinks_are_escaped():
    assert "escapeHtml(title)" in APP_JS
    assert "escapeHtml(resolvedUrl)" in APP_JS
    assert "escapeHtml(displayUrl)" in APP_JS
    assert "escapeHtml(snippet)" in APP_JS
    assert "encodeURIComponent(resolvedUrl)" in APP_JS


def test_history_and_open_state_sinks_are_escaped():
    assert "escapeHtml(entry.query)" in APP_JS
    assert "escapeHtml(entry.mode)" in APP_JS
    assert "escapeHtml(entry.timestamp)" in APP_JS
    assert "escapeHtml(result.browser)" in APP_JS
    assert "escapeHtml(result.url)" in APP_JS


def test_markdown_link_href_is_escaped():
    assert "escapeHtml(href)" in MARKDOWN_JS
