"""Structural CSS regression guard for the result-card grid contract.

The .result-card layout relies on explicit minmax tracks so the action column
(star / Open) stays content-sized instead of stretching to fill leftover width
on short snippets. A past regression silently changed these tracks and broke
short-result cards; this module pins the contract so a CSS refactor can't
reintroduce it. It is a structural check, not a full visual snapshot, because
a byte-for-byte snapshot of style.css would fail on every legitimate restyle.

The parser only looks at top-level rules, so declarations nested inside
@media blocks are captured separately (the mobile collapse to a single column
is itself part of the contract).
"""

from __future__ import annotations

import re
from pathlib import Path

STYLE_CSS = Path(__file__).parent.parent / "netrail" / "static" / "style.css"

RESULT_CARD_DESKTOP = "minmax(0, 1fr) auto"
RESULT_CARD_IMAGE_DESKTOP = "96px minmax(0, 1fr) auto"
MOBILE_BREAKPOINT = "max-width: 720px"


def _top_level_blocks(css: str) -> dict[str, str]:
    """Return {selector: body} for every top-level rule, honoring nesting."""
    blocks: dict[str, str] = {}
    depth = 0
    selector_chars: list[str] = []
    cur_selector: str | None = None
    buf_start = 0
    i = 0
    n = len(css)
    while i < n:
        ch = css[i]
        if depth == 0:
            if ch == "{":
                cur_selector = " ".join("".join(selector_chars).split())
                selector_chars = []
                depth = 1
                buf_start = i + 1
            else:
                selector_chars.append(ch)
        else:
            if ch == "{":
                depth += 1
            elif ch == "}":
                depth -= 1
                if depth == 0 and cur_selector is not None:
                    blocks[cur_selector] = css[buf_start:i]
                    cur_selector = None
        i += 1
    return blocks


def _decl(body: str, prop: str) -> str | None:
    m = re.search(rf"{re.escape(prop)}\s*:\s*([^;]+);", body)
    return m.group(1).strip() if m else None


def _test_blocks():
    return _top_level_blocks(STYLE_CSS.read_text(encoding="utf-8"))


def test_result_card_grid_stays_minmax():
    blocks = _test_blocks()
    body = blocks[".result-card"]
    assert _decl(body, "grid-template-columns") == RESULT_CARD_DESKTOP


def test_result_card_image_grid_stays_minmax():
    blocks = _test_blocks()
    body = blocks[".result-card.image-card"]
    assert _decl(body, "grid-template-columns") == RESULT_CARD_IMAGE_DESKTOP


def test_action_column_never_stretches():
    blocks = _test_blocks()
    body = blocks[".result-card"]
    cols = _decl(body, "grid-template-columns")
    assert cols is not None
    tracks = [t.strip() for t in cols.split()]
    assert tracks[-1] == "auto", f"action column must stay auto, got: {tracks}"


def test_mobile_collapses_result_card_to_single_column():
    blocks = _test_blocks()
    media = blocks[f"@media ({MOBILE_BREAKPOINT})"]
    assert _decl(media, "grid-template-columns") == "1fr"
