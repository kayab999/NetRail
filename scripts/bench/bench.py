#!/usr/bin/env python3
"""Sprint 4 dual-stack benchmark: latency percentiles, throughput, error rate,
CPU/RSS, and a saturation knee scan (stop when error rate > 1%).

Usage:
  python3 scripts/bench/bench.py --stack rust   --out docs/assets/bench-rust
  python3 scripts/bench/bench.py --stack python --out docs/assets/bench-python
"""

from __future__ import annotations

import argparse
import asyncio
import base64
import json
import os
import signal
import statistics
import subprocess
import sys
import tempfile
import threading
import time
from pathlib import Path

import httpx

ROOT = Path(__file__).resolve().parents[2]
URL = "http://127.0.0.1:7421/api/health"


def read_cpu_rss(pid: int) -> tuple[float, int]:
    try:
        with open(f"/proc/{pid}/stat") as fh:
            rest = fh.read().rsplit(")", 1)[1].split()
        clk = os.sysconf(os.sysconf_names["SC_CLK_TCK"])
        return (int(rest[11]) + int(rest[12])) / clk, int(rest[19]) / clk
    except OSError:
        return 0.0, 0.0


def read_rss_kib(pid: int) -> int:
    try:
        status = Path(f"/proc/{pid}/status").read_text()
        for line in status.splitlines():
            if line.startswith("VmRSS:"):
                return int(line.split()[1])
    except OSError:
        pass
    return 0


class Sampler:
    """Tracks peak CPU% and peak RSS of the server during a batch."""

    def __init__(self, pid: int) -> None:
        self.pid = pid
        self.peak_cpu = 0.0
        self.peak_rss = 0
        self._stop = threading.Event()
        self._thread = threading.Thread(target=self._run, daemon=True)

    def start(self) -> None:
        self._thread.start()

    def stop(self) -> None:
        self._stop.set()
        self._thread.join()

    def _run(self) -> None:
        while not self._stop.is_set():
            cpu_total, cpu_start = read_cpu_rss(self.pid)
            rss = read_rss_kib(self.pid)
            time.sleep(0.5)
            cpu_total2, cpu_start2 = read_cpu_rss(self.pid)
            wall = 0.5
            pct = max(0.0, (cpu_total2 - cpu_total) / wall * 100.0)
            self.peak_cpu = max(self.peak_cpu, pct)
            self.peak_rss = max(self.peak_rss, rss)


async def batch(client: httpx.AsyncClient, n: int, concurrency: int) -> dict:
    sem = asyncio.Semaphore(concurrency)
    lat = []
    errors = 0

    async def one(_: int) -> None:
        nonlocal errors
        async with sem:
            t0 = time.perf_counter()
            try:
                resp = await client.get(URL)
                ok = resp.status_code == 200
            except httpx.HTTPError:
                ok = False
            dt = (time.perf_counter() - t0) * 1000.0
            lat.append(dt)
            if not ok:
                errors += 1

    t_start = time.perf_counter()
    await asyncio.gather(*(one(i) for i in range(n)))
    elapsed = time.perf_counter() - t_start

    lat.sort()
    def pct(p: float) -> float:
        if not lat:
            return 0.0
        return lat[min(int(p * len(lat)), len(lat) - 1)]

    return {
        "n": n,
        "concurrency": concurrency,
        "elapsed_s": elapsed,
        "rps": n / elapsed if elapsed else 0.0,
        "p50_ms": pct(0.50),
        "p95_ms": pct(0.95),
        "p99_ms": pct(0.99),
        "error_pct": 100.0 * errors / n if n else 0.0,
        "max_cpu_pct": 0.0,
        "peak_rss_kib": 0,
    }


async def main_async(args, pid: int, env: dict) -> int:
    limits = httpx.Limits(max_connections=args.concurrency * 4, max_keepalive_connections=args.concurrency * 4)
    async with httpx.AsyncClient(limits=limits, timeout=30.0) as client:
        # Warmup
        w = await batch(client, args.warmup, args.concurrency)
        print(f"warmup: {w['rps']:.0f} rps, {w['error_pct']:.2f}% errors")

        steady = []
        for run in range(3):
            sampler = Sampler(pid)
            sampler.start()
            res = await batch(client, args.requests, args.concurrency)
            sampler.stop()
            res["max_cpu_pct"] = sampler.peak_cpu
            res["peak_rss_kib"] = sampler.peak_rss
            steady.append(res)
            print(
                f"run {run + 1}: {res['rps']:.0f} rps | p50 {res['p50_ms']:.2f} ms "
                f"p95 {res['p95_ms']:.2f} ms p99 {res['p99_ms']:.2f} ms | "
                f"{res['error_pct']:.2f}% err | cpu {res['max_cpu_pct']:.0f}% rss {res['peak_rss_kib']} KiB"
            )

        # Saturation knee scan: double concurrency until error rate > 1%.
        saturation = []
        c = args.concurrency
        last_c = c
        scan_n = max(500, min(2000, args.requests))
        for _ in range(6):
            res = await batch(client, scan_n, c)
            saturation.append(res)
            last_c = c
            print(f"saturation c={c}: {res['rps']:.0f} rps p95 {res['p95_ms']:.2f} ms {res['error_pct']:.2f}% err")
            if res["error_pct"] > 1.0:
                break
            c *= 2
        knee = {
            "concurrency": last_c,
            "error_pct": saturation[-1]["error_pct"],
            "rps": saturation[-1]["rps"],
            "p95_ms": saturation[-1]["p95_ms"],
        }

        result = {
            "stack": args.stack,
            "endpoint": URL,
            "steady": steady,
            "saturation": saturation,
            "knee": knee,
            "note": "client = asyncio httpx keep-alive; error budget 1%",
        }
        out = Path(args.out)
        out.parent.mkdir(parents=True, exist_ok=True)
        out.write_text(json.dumps(result, indent=2))
        print(f"json: {out}")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--stack", choices=["rust", "python"], required=True)
    parser.add_argument("--out", required=True)
    parser.add_argument("--requests", type=int, default=2000)
    parser.add_argument("--concurrency", type=int, default=16)
    parser.add_argument("--warmup", type=int, default=1000)
    args = parser.parse_args()

    workdir = Path(tempfile.mkdtemp(prefix="netrail-bench-"))
    env = os.environ.copy()
    env.update(
        {
            "HOME": str(workdir),
            "XDG_CONFIG_HOME": str(workdir / "config"),
            "XDG_DATA_HOME": str(workdir / "data"),
            "XDG_CACHE_HOME": str(workdir / "cache"),
            "NETRAIL_DB_PATH": str(workdir / "netrail.db"),
            "NETRAIL_DB_KEY": base64.urlsafe_b64encode(os.urandom(32)).decode(),
            "NETRAIL_RATE_LIMIT": "0",
            "NETRAIL_AUTO_OPEN": "false",
        }
    )
    if args.stack == "rust":
        cmd = [str(ROOT / "src-tauri/target/release/netrail-api")]
    else:
        cmd = [sys.executable, "-m", "netrail"]

    server = subprocess.Popen(cmd, env=env, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    try:
        import urllib.request

        healthy = False
        for _ in range(100):
            try:
                with urllib.request.urlopen(URL, timeout=5) as resp:
                    if resp.status == 200:
                        healthy = True
                        break
            except OSError:
                pass
            time.sleep(0.2)
        if not healthy:
            print(f"ERROR: {args.stack} server never became healthy", file=sys.stderr)
            return 2
        return asyncio.run(main_async(args, server.pid, env))
    finally:
        server.send_signal(signal.SIGTERM)
        try:
            server.wait(timeout=10)
        except subprocess.TimeoutExpired:
            server.kill()
            server.wait()


if __name__ == "__main__":
    sys.exit(main())
