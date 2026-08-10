#!/usr/bin/env python3
"""Differential open-URL parity fuzzer: Python validator vs live Rust binary.

Ground truth is the running Rust `netrail-api` (the production stack). The
corpus is seeded for reproducibility (QA baseline 2026-08-09 seed 20260809)
and densely covers non-canonical IPv4 literals, ports, schemes and paths.

Expected residual (documented in docs/QA_EVALUATION_2026-08-09.md D2.2):
only the `0xzz` single-label DNS-stage family — Rust fails DNS inside open,
Python in the later pin stage (same terminal state, no fail-open).

Usage:
    scripts/fuzz-parity.py [--base http://127.0.0.1:7421] [--seed 20260809]
                           [--min-urls 7600] [--full]

Checks that python-side `validate_open_url` never allows a URL the live Rust
binary blocks, and that blocked codes match (code_diff == 0).

Exit codes: 0 = parity holds (residual within documented classes),
1 = py-allow/rust-block fail-open found, 2 = code divergence found.
"""

from __future__ import annotations

import argparse
import json
import random
import sys
import urllib.error
import urllib.request

sys.path.insert(0, "/home/carlos/NetRail")
from netrail.errors import NetRailError
from netrail.security import validate_open_url

SCHEMES = ["http", "https", "HTTP", "HTTPS", "HtTp"]
HOST_POOL = [
    "127.0.0.1", "127.1", "0.0.0.0", "0.1.2.3", "00.1.2.3", "01.02.03.04",
    "08.02.01.1", "011.119.190.078", "0177.1", "0177.0.0.1", "016.1.2.3",
    "0296.1", "08.077", "1.2", "1.2.3", "1.2.3.4.5", "1.2.3.4.5.6", "1.2..3",
    "999.1.2.3", "1955950671", "2130706433", "4294967295", "4294967296",
    "0x7f000001", "0x1ddB5d6", "0x7f.1", "0x7f.2.3", "1.16777215",
    "1.16777216", "74.188", "0377.0377.0377.0377", "011778", "0119",
    "0xzz", "0x", "1x2.3.4.5", "139.241.", "123.191.258.", "120.51.87.",
    "114.40.175.19.", "101.80.35", "192.116.221.248", "215.37.231",
    "30.219", "38.62", "173.62", "1060011134", "573805379", "2981237992",
    "2157867145", "3258265571", "555915523", "221.13.193.249", "058.253.245.150",
    "028.148.127.107", "example.com", "duckduckgo.com", "duck.com",
    "münchen.de", "xn--mnchen-3ya.de", "%B4", "%89y", "084", "0", "01",
    "256.1.1.1", "1.256.1.1", "1.1.256.1", "1.1.1.256", "255.255.255.255",
    "240.0.0.1", "169.254.169.254", "10.0.0.1", "192.168.1.1", "172.16.0.1",
    "nip.io", "xip.io", "sslip.io", "localtest.me", "metadata.google.internal",
]
PORT_TAILS = ["", ":1", ":80", ":443", ":65535", ":65536", ":99999", ":0",
              ":8080.", ".:80", ".", ":80:9604", ":8080:43279", ":1:0",
              ":abc", ":-1", ":+80", ":080", ":00080"]
PATHS = ["", "/", "/%89y+=ZwD]w$vv", "/%8a;0(#Y", "/%F5T,9,N", "/$]gjag(w",
         "/hGGO$", "/%D", "/WB2VU", "/U", "/KN7n#qE=fzX", "/.n",
         "/%B2rZVzkCc@", "/%7AozAqkwJ", "/?q=1&uddg=https%3A%2F%2Fexample.com%2F",
         "/%f0:#", "/%14L#p8N:U", "/$%5cje", "/-%3E"]

# Documented residual: single-label `0x` prefix with a non-hex tail falls to
# the DNS path in both stacks; the DNS stage ordering difference is not a
# fail-open (terminal state identical). Allowed as the only residual class.
_KNOWN_DNS_STAGE_RESIDUAL = ("0xzz",)


def build_corpus(seed: int, min_urls: int, full: bool) -> list[str]:
    rng = random.Random(seed)
    urls: set[str] = set()
    pool = list(HOST_POOL)
    if full:
        pool = HOST_POOL
    for h in pool:
        for pt in PORT_TAILS:
            for s in SCHEMES:
                urls.add(f"{s}://{h}{pt}{rng.choice(PATHS)}")
    while len(urls) < min_urls:
        urls.add(
            f"{rng.choice(SCHEMES)}://{rng.choice(pool)}{rng.choice(PORT_TAILS)}"
            f"{rng.choice(PATHS)}"
        )
    return sorted(urls)


def rust_open(base: str, url: str) -> tuple[int, str | None]:
    req = urllib.request.Request(
        base + "/api/open",
        data=json.dumps({"url": url}).encode(),
        headers={"Content-Type": "application/json"},
        method="POST",
    )
    try:
        with urllib.request.urlopen(req, timeout=10) as r:
            body = json.loads(r.read())
            return r.status, body.get("code")
    except urllib.error.HTTPError as e:
        try:
            return e.code, json.loads(e.read()).get("code")
        except Exception:
            return e.code, None
    except Exception:
        return -1, None


def py_open(url: str) -> tuple[int, str | None]:
    try:
        validate_open_url(url)
        return 200, "ALLOW"
    except NetRailError as e:
        return 400, e.code


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--base", default="http://127.0.0.1:7421")
    ap.add_argument("--seed", type=int, default=20260809)
    ap.add_argument("--min-urls", type=int, default=2000)
    ap.add_argument("--full", action="store_true")
    ap.add_argument("--corpus-only", action="store_true")
    ap.add_argument("--dump-all", action="store_true")
    args = ap.parse_args()

    urls = build_corpus(args.seed, args.min_urls, args.full)

    if args.corpus_only:
        stats = {"total": 0, "allowed": 0, "blocked": 0, "blocked_codes": {},
                 "allowed_hosts": {}}
        for url in urls:
            stats["total"] += 1
            p = py_open(url)
            if p[0] == 200:
                stats["allowed"] += 1
                host = url.split("://", 1)[1].split("/", 1)[0].lower()
                stats["allowed_hosts"][host] = stats["allowed_hosts"].get(host, 0) + 1
            else:
                stats["blocked"] += 1
                stats["blocked_codes"][p[1]] = stats["blocked_codes"].get(p[1], 0) + 1
        print(json.dumps(stats, indent=2))
        print("CORPUS EXPLORATION OK")
        return 0
    stats = {"total": 0, "py_allow_rust_block": 0, "code_diff": 0}
    mismatches: dict[tuple[str | None, str | None], list[str]] = {}

    for url in urls:
        p = py_open(url)
        r = rust_open(args.base, url)
        stats["total"] += 1
        if p[0] == 200 and r[0] != 200:
            stats["py_allow_rust_block"] += 1
            mismatches.setdefault((p[1], r[1]), []).append(url)
        if p[0] != 200 and r[0] != 200 and p[1] != r[1]:
            stats["code_diff"] += 1
            mismatches.setdefault((p[1], r[1]), []).append(url)

    print(json.dumps(stats, indent=2))
    for (pc, rc), urls_in in sorted(mismatches.items(), key=lambda kv: -len(kv[1])):
        if stats["py_allow_rust_block"] and pc == "ALLOW" and all(
            u.split("://", 1)[1].split("/", 1)[0].lower().startswith(h)
            for u in urls_in for h in _KNOWN_DNS_STAGE_RESIDUAL
        ):
            print(f"known DNS-stage residual (rust={rc}) count={len(urls_in)}")
        else:
            print(f"rust={rc} py={pc} count={len(urls_in)}")
            if args.dump_all:
                for u in urls_in:
                    print(f"   {u[:90]}")
            else:
                for u in urls_in[:5]:
                    print(f"   {u[:90]}")

    if stats["py_allow_rust_block"]:
        print("FAIL-OPEN DIVERGENCE — Python allows, Rust blocks")
        return 1
    if stats["code_diff"]:
        print("CODE DIVERGENCE — both block with different codes")
        return 2
    print("PARITY OK")
    return 0


if __name__ == "__main__":
    sys.exit(main())
