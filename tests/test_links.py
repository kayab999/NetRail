from __future__ import annotations

import re
from pathlib import Path

ROOT = Path(__file__).parent.parent
DOC_FILES = [ROOT / "README.md", ROOT / "CHANGELOG.md", ROOT / "SECURITY.md"]
DOC_FILES += sorted((ROOT / "docs").glob("*.md"))


def _github_slug(text: str) -> str:
    # GitHub anchor algorithm: lowercase, strip punctuation (dots included,
    # no replacement), collapse whitespace into single dashes.
    cleaned = re.sub(r"[^a-z0-9\s-]", "", text.lower())
    return re.sub(r"\s+", "-", cleaned.strip()).strip("-")


def _headings(path: Path) -> set[str]:
    headings = set()
    for line in path.read_text(errors="replace").splitlines():
        match = re.match(r"^(#{1,6})\s+(.*?)\s*#*\s*$", line)
        if match:
            headings.add(_github_slug(match.group(2)))
    return headings


def test_cross_doc_links_resolve():
    broken: list[str] = []
    for doc in DOC_FILES:
        text = doc.read_text(errors="replace")
        for match in re.finditer(r"\[([^\]]+)\]\(([^)\s]+)(?:\s+[\"'][^\"']*[\"'])?\s*\)", text):
            label, target = match.group(1), match.group(2)
            if target.startswith(("http://", "https://", "mailto:", "#")):
                continue
            path_part, _, anchor = target.partition("#")
            if not path_part:
                continue
            resolved = (doc.parent / path_part).resolve()
            if not resolved.is_file():
                broken.append(f"{doc.name}:{label} -> {target} (missing file)")
                continue
            if anchor:
                anchor_slug = _github_slug(anchor)
                if anchor_slug not in _headings(resolved):
                    broken.append(f"{doc.name}:{label} -> {target} (missing anchor)")
        for target in re.findall(r"<([^>]+)>", text):
            if target.startswith(("http://", "https://", "mailto:")):
                continue
            if target.endswith((".md", ".png", ".json", ".toml", ".txt", ".svg")):
                resolved = (doc.parent / target).resolve()
                if not resolved.is_file():
                    broken.append(f"{doc.name}:<{target}> (missing file)")
    assert not broken, "\n".join(broken[:20])