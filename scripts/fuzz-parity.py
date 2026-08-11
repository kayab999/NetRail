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
import os
import subprocess
import sys
import time
import urllib.error
import urllib.request

_REPO_ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
sys.path.insert(0, _REPO_ROOT)
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
    "100.64.0.1", "100.127.255.254", "192.0.0.9", "192.0.2.1", "198.18.0.1",
    "198.19.255.255", "198.51.100.1", "203.0.113.1",
    "nip.io", "xip.io", "sslip.io", "localtest.me", "metadata.google.internal",
]
# IPv6 literal hosts (bracket form) covering the canonical A-10 policy:
# embedded-IPv4 decodes, NAT64, RFC 4291 reserved blocks, IANA special-registry
# "not globally reachable", ORCHIDv2 exception, ULA/link-local/multicast, GUA.
V6_HOST_POOL = [
    "[::1]", "[::]", "[::ffff:127.0.0.1]", "[::ffff:c0a8:101]",
    "[::127.0.0.1]", "[::1.1.1.1]", "[::1234:5678]",
    "[64:ff9b::101:101]", "[64:ff9b::7f00:1]", "[64:ff9b:1::7f00:1]",
    "[64:ff9b:2::1]", "[100::1]", "[100:1::1]", "[200::1]", "[400::1]",
    "[800::1]", "[1000::1]", "[4000::1]", "[5f00::1]", "[6000::1]",
    "[8000::1]", "[a000::1]", "[c000::1]", "[e000::1]", "[f000::1]",
    "[f800::1]", "[fe00::1]", "[fec0::1]", "[fe80::1]", "[fc00::1]",
    "[fd00::1]", "[ff00::1]", "[2001::1]", "[2001:10::1]", "[2001:20::1]",
    "[2001:db8::1]", "[2001:4860::1]", "[2002::1]", "[3fff::1]", "[3000::1]",
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

# Pinned CI expectation for the seed corpus: every divergence must belong to
# the known residual family, and the family size must stay at the Baseline #1
# value. A change here is a contract decision, not an incidental drift.
_EXPECTED_RESIDUAL_COUNT = 50


def spawn_server(binary: str, base: str) -> subprocess.Popen:
    """Boot a netrail-api with fully isolated state (harness-safe)."""
    import tempfile

    home = tempfile.mkdtemp(prefix="fuzz-home-")
    xdg = tempfile.mkdtemp(prefix="fuzz-xdg-")
    db = os.path.join(tempfile.mkdtemp(prefix="fuzz-db-"), "netrail.db")
    env = dict(
        os.environ,
        HOME=home,
        XDG_CONFIG_HOME=xdg,
        NETRAIL_DB_PATH=db,
        NETRAIL_RATE_LIMIT="0",
        NETRAIL_HISTORY_ENCRYPT="false",
        NETRAIL_AUTO_OPEN="false",
        NETRAIL_NO_OPEN="1",
        NETRAIL_STATIC_DIR=os.path.join(_REPO_ROOT, "netrail", "static"),
    )
    env.pop("NETRAIL_API_TOKEN", None)
    proc = subprocess.Popen([binary], env=env, stdout=subprocess.DEVNULL,
                            stderr=subprocess.DEVNULL)
    host = base.split("://")[1].rsplit(":", 1)[0]
    port = base.rsplit(":", 1)[1].split("/", 1)[0]
    for _ in range(80):
        if proc.poll() is not None:
            raise RuntimeError(f"netrail-api exited early (code {proc.returncode})")
        try:
            with urllib.request.urlopen(f"http://{host}:{port}/api/health",
                                        timeout=2) as r:
                if r.status == 200:
                    return proc
        except Exception:
            time.sleep(0.25)
    proc.terminate()
    raise RuntimeError("netrail-api did not become healthy in time")


def build_corpus(seed: int, min_urls: int, full: bool) -> list[str]:
    rng = random.Random(seed)
    urls: set[str] = set()
    pool = list(HOST_POOL) + V6_HOST_POOL
    if full:
        pool = list(HOST_POOL) + V6_HOST_POOL
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
    ap.add_argument("--binary", help="netrail-api binary to spawn (isolated env)")
    ap.add_argument("--ci", action="store_true",
                    help="gate mode: code_diff==0, all divergences in the known "
                         "residual family, residual count pinned")
    args = ap.parse_args()

    proc = None
    if args.binary:
        print(f"spawning {args.binary} (isolated state)")
        proc = spawn_server(args.binary, args.base)
    try:
        return run(args)
    finally:
        if proc is not None:
            proc.terminate()
            try:
                proc.wait(timeout=5)
            except subprocess.TimeoutExpired:
                proc.kill()


def run(args: argparse.Namespace) -> int:
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
    divergences: list[str] = []
    mismatches: dict[tuple[str | None, str | None], list[str]] = {}

    for url in urls:
        p = py_open(url)
        r = rust_open(args.base, url)
        stats["total"] += 1
        if p[0] == 200 and r[0] != 200:
            stats["py_allow_rust_block"] += 1
            divergences.append(url)
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
    if stats["code_diff"]:
        print("CODE DIVERGENCE — both block with different codes")

    if args.ci:
        residual = divergences
        known_only = all(u.split("://", 1)[1].split("/", 1)[0].lower()
                         .startswith(h)
                         for u in residual for h in _KNOWN_DNS_STAGE_RESIDUAL)
        if stats["code_diff"]:
            print("CI GATE FAILED — code divergence present")
            return 2
        if not known_only:
            print(f"CI GATE FAILED — unknown divergence family ({len(residual)} urls)")
            return 1
        if len(residual) != _EXPECTED_RESIDUAL_COUNT:
            print(f"CI GATE FAILED — known residual drifted: {len(residual)} "
                  f"!= pinned {_EXPECTED_RESIDUAL_COUNT} (contract decision required)")
            return 1
        print(f"CI GATE OK — parity holds; known residual pinned at {len(residual)}")
        return 0

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
