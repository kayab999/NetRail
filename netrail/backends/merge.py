from __future__ import annotations

from urllib.parse import parse_qsl, urlencode, urlparse, urlunparse

from netrail.backends.types import SearchResult

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


def normalize_url_key(raw: str) -> str:
    trimmed = raw.strip()
    try:
        parsed = urlparse(trimmed)
        host = (parsed.hostname or "").lower().removeprefix("www.")
        path = parsed.path.rstrip("/") or "/"
        pairs = [
            (k, v)
            for k, v in parse_qsl(parsed.query, keep_blank_values=True)
            if k.lower() not in TRACKING_PARAMS
        ]
        pairs.sort()
        query = urlencode(pairs)
        rebuilt = urlunparse(
            (
                parsed.scheme.lower(),
                host + (f":{parsed.port}" if parsed.port else ""),
                path,
                "",
                query,
                "",
            )
        )
        return rebuilt.lower()
    except Exception:  # noqa: BLE001
        return trimmed.rstrip("/").lower()


def _richer(a: SearchResult, b: SearchResult) -> SearchResult:
    a_score = len(a.snippet) + len(a.title)
    b_score = len(b.snippet) + len(b.title)
    return b if b_score > a_score else a


def dedupe_results(results: list[SearchResult]) -> list[SearchResult]:
    seen: dict[str, SearchResult] = {}
    for item in results:
        key = normalize_url_key(item.url)
        if key in seen:
            seen[key] = _richer(seen[key], item)
        else:
            seen[key] = item
    return list(seen.values())


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