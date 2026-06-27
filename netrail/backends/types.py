from __future__ import annotations

from dataclasses import dataclass, field
from typing import Literal

SearchMode = Literal["web", "images"]

OPERATORS = frozenset({"site:", "filetype:", "intitle:", "inurl:", '"', "-"})


@dataclass(frozen=True)
class SearchResult:
    title: str
    url: str
    snippet: str = ""
    image: str | None = None
    source: str = ""
    backend: str = ""
    provenance: str = ""

    def as_dict(self) -> dict[str, str | None]:
        return {
            "title": self.title,
            "url": self.url,
            "snippet": self.snippet,
            "image": self.image,
            "source": self.source,
            "backend": self.backend,
            "provenance": self.provenance,
        }


@dataclass
class SearchResponse:
    query: str
    mode: SearchMode
    results: list[SearchResult] = field(default_factory=list)
    backends_used: list[str] = field(default_factory=list)
    provenance_chain: list[str] = field(default_factory=list)
    sovereignty_step: int = 1
    sovereignty_total: int = 5
    errors: list[str] = field(default_factory=list)
    search_strategy: str = "fanout"

    def as_dict(self) -> dict:
        return {
            "query": self.query,
            "mode": self.mode,
            "count": len(self.results),
            "results": [r.as_dict() for r in self.results],
            "backends_used": self.backends_used,
            "provenance_chain": self.provenance_chain,
            "sovereignty": {
                "step": self.sovereignty_step,
                "total": self.sovereignty_total,
                "label": _sovereignty_label(self.sovereignty_step),
            },
            "errors": self.errors,
            "search_strategy": self.search_strategy,
        }


def _sovereignty_label(step: int) -> str:
    labels = {
        1: "Local console — borrowed indexes",
        2: "Pluggable backends enabled",
        3: "Self-hosted discovery (SearXNG)",
        4: "Local history and corpus",
        5: "Owned index — full sovereignty",
    }
    return labels.get(step, labels[1])