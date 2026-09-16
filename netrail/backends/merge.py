from __future__ import annotations

from urllib.parse import parse_qsl, quote, unquote, urlencode, urlparse, urlunparse

from netrail.backends.types import SearchResult

# Keep in sync with netrail.security._DDG_HOSTS (open unwrap + merge resolve).
DDG_HOSTS = frozenset({"duckduckgo.com", "duck.com"})


def resolve_result_url(raw: str, depth: int = 0) -> str:
    if depth > 4:
        return raw.strip()
    trimmed = raw.strip()
    parsed = urlparse(trimmed)
    host = (parsed.hostname or "").lower()
    if host in DDG_HOSTS or any(host.endswith(f".{h}") for h in DDG_HOSTS):
        for key, value in parse_qsl(parsed.query, keep_blank_values=True):
            if key == "uddg" and value:
                return resolve_result_url(unquote(value), depth + 1)
    return trimmed

TRACKING_PARAMS = frozenset(
    {
        "utm_source",
        "utm_medium",
        "utm_campaign",
        "utm_term",
        "utm_content",
        "fbclid",
        "gclid",
        "mc_cid",
        "mc_eid",
        "ref",
        "ref_src",
        "igshid",
        "si",
    }
)


def _quote_space20(s: str, safe: str = "", encoding: str = "utf-8", errors: str = "strict") -> str:
    """`urlencode` quoter emitting `%20` for spaces (parity with Rust
    `Url::set_query`). `safe` is forced to `"+"` so a literal `+` stays
    `+` (Rust also leaves it unencoded) while `/` stays `%2F`, matching
    the previous `quote_plus` behavior for everything except spaces."""
    return quote(str(s), safe="+", encoding=encoding, errors=errors)


def normalize_url_key(raw: str) -> str:
    """Normalize a URL for deduplication.

    Scheme + host are case-insensitive per RFC 3986 (lowercased); path and
    query are case-sensitive (preserved) so `/Page` and `/page` do NOT
    dedupe. Strips `www.`, tracking params, fragment and trailing slash;
    sorts query params; drops default ports. Falls back to
    the old lowercase behavior when unparseable.
    """
    trimmed = resolve_result_url(raw)
    try:
        parsed = urlparse(trimmed)
        scheme = parsed.scheme.lower()
        host = (parsed.hostname or "").lower().removeprefix("www.")
        if not scheme or not host:
            return trimmed.rstrip("/").lower()
        # Drop default ports (parity with Rust `Url::to_string()`); keep
        # non-default ones.
        port = parsed.port
        if port is not None and not (
            (scheme == "http" and port == 80) or (scheme == "https" and port == 443)
        ):
            host = f"{host}:{port}"
        # Path is case-sensitive — preserve as-is.
        path = parsed.path.rstrip("/") or "/"
        pairs = [
            (k, v)
            for k, v in parse_qsl(parsed.query, keep_blank_values=True)
            if k.lower() not in TRACKING_PARAMS
        ]
        pairs.sort()
        # `parse_qsl` decodes both `+` and `%20` to space; re-encode as `%20`
        # so `?q=a+b` and `?q=a%20b` share one key (Rust parity). `%2B`
        # decodes to a literal `+`, which re-encodes as `%2B` — stays distinct.
        query = urlencode(pairs, quote_via=_quote_space20)
        # Parity with Rust `Url::to_string()` + trailing-slash strip: a bare
        # host serializes without `/` (`https://example.com`, not `.../`).
        if not query and path == "/":
            return f"{scheme}://{host}"
        rebuilt = urlunparse((scheme, host, path, "", query, ""))
        # NOTE: no `.lower()` here — only scheme+host were lowercased above.
        return rebuilt
    except Exception:  # noqa: BLE001
        return trimmed.rstrip("/").lower()


def _richer(a: SearchResult, b: SearchResult) -> SearchResult:
    a_score = len(a.snippet) + len(a.title)
    b_score = len(b.snippet) + len(b.title)
    return b if b_score > a_score else a


def dedupe_results(results: list[SearchResult]) -> list[SearchResult]:
    seen: dict[str, SearchResult] = {}
    order: list[str] = []
    for item in results:
        key = normalize_url_key(item.url)
        if key in seen:
            seen[key] = _richer(seen[key], item)
        else:
            order.append(key)
            seen[key] = item
    return [seen[key] for key in order]


def interleave_batches(batches: list[list[SearchResult]], max_results: int) -> list[SearchResult]:
    if not batches:
        return []
    if len(batches) == 1:
        return batches[0][:max_results]

    indices = [0] * len(batches)
    output: list[SearchResult] = []
    seen: set[str] = set()

    while len(output) < max_results:
        advanced = False
        for i, batch in enumerate(batches):
            while indices[i] < len(batch):
                item = batch[indices[i]]
                indices[i] += 1
                key = normalize_url_key(item.url)
                if key in seen:
                    continue
                seen.add(key)
                output.append(item)
                advanced = True
                break
            if len(output) >= max_results:
                break
        if not advanced:
            break
    return output


def merge_fanout(batches: list[tuple[str, list[SearchResult]]], max_results: int) -> list[SearchResult]:
    flat = [item for _, batch in batches for item in batch]
    deduped = dedupe_results(flat)
    by_backend: dict[str, list[SearchResult]] = {}
    for item in deduped:
        by_backend.setdefault(item.backend, []).append(item)
    return interleave_batches(list(by_backend.values()), max_results)