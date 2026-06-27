from __future__ import annotations

from pathlib import Path

PROJECT_ROOT = Path(__file__).resolve().parent.parent


def _rewrite_asset_paths(markdown: str) -> str:
    return markdown.replace("](docs/assets/", "](/static/docs/assets/")


def load_doc(slug: str) -> dict[str, str]:
    mapping = {
        "manual": (PROJECT_ROOT / "docs" / "MANUAL.md", "User Manual"),
        "about": (PROJECT_ROOT / "README.md", "About NetRail"),
    }
    if slug not in mapping:
        from netrail.errors import NetRailError

        raise NetRailError("DOC_NOT_FOUND", f"document '{slug}'", status=404)

    path, title = mapping[slug]
    markdown = path.read_text(encoding="utf-8")
    return {
        "slug": slug,
        "title": title,
        "markdown": _rewrite_asset_paths(markdown),
    }


def asset_path(filename: str) -> Path | None:
    if ".." in filename or "/" in filename or "\\" in filename:
        return None

    static_asset = PROJECT_ROOT / "netrail" / "static" / "docs" / "assets" / filename
    if static_asset.exists():
        return static_asset

    project_asset = PROJECT_ROOT / "docs" / "assets" / filename
    if project_asset.exists():
        return project_asset

    return None