from __future__ import annotations

from typing import Protocol, runtime_checkable

from netrail.backends.types import OPERATORS, SearchMode, SearchResult


@runtime_checkable
class SearchBackend(Protocol):
    """Pluggable search provider. All discovery flows through this boundary."""

    @property
    def name(self) -> str:
        """Stable backend identifier, e.g. 'ddgs' or 'searxng'."""
        ...

    @property
    def provenance(self) -> str:
        """Human-readable chain, e.g. 'ddgs → DuckDuckGo → primarily Bing'."""
        ...

    @property
    def supports_operators(self) -> frozenset[str]:
        """Operator prefixes this backend honors."""
        ...

    def is_available(self) -> bool:
        """False skips this backend in fallback chains."""
        ...

    def search(self, query: str, mode: SearchMode, max_results: int) -> list[SearchResult]:
        """Return normalized results. Raise on hard failure."""
        ...


DEFAULT_OPERATORS = OPERATORS