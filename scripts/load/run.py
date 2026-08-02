#!/usr/bin/env python3
"""Sprint 3 load generator + metric sampler (Resource Stability).

Starts a NetRail server (Rust `netrail-api` or Python `python -m netrail`),
runs N sequential + M concurrent GET /api/health requests, and samples the
server's RSS / open FDs / established sockets every `--every` requests.

Usage:
  python3 scripts/load/run.py --stack rust   --out results-rust.csv
  python3 scripts/load/run.py --stack python --out results-python.csv
"""

from __future__ import annotations

import argparse
import base64
import csv
import http.client
import os
import re
import signal
import subprocess
import sys
import tempfile
import threading
import time
from collections import Counter
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
BASE = "http://127.0.0.1:7421"
HEALTH = f"{BASE}/api/health"


def read_rss_kib(pid: int) -> int:
    try:
        status = Path(f"/proc/{pid}/status").read_text()
        match = re.search(r"^VmRSS:\s+(\d+) kB", status, re.M)
        return int(match.group(1)) if match else 0
    except OSError:
        return 0


def read_fd_count(pid: int) -> int:
    try:
        return len(os.listdir(f"/proc/{pid}/fd"))
    except OSError:
        return 0


def read_socket_count(port: int = 7421) -> int:
    try:
        proc = subprocess.run(
            [
                "ss",
                "-Htn",
                "state",
                "established",
                f"( sport = :{port} or dport = :{port} )",
            ],
            capture_output=True,
            text=True,
            timeout=3,
        )
        return len([l for l in proc.stdout.splitlines() if l.strip()])
    except (FileNotFoundError, subprocess.TimeoutExpired):
        return 0


def get_health() -> int:
    with urllib.request.urlopen(HEALTH, timeout=5) as resp:
        return resp.status


class Requester:
    """Reuses one keep-alive connection per thread; counts failures by cause."""

    def __init__(self) -> None:
        self.failures: Counter[str] = Counter()
        self._local = threading.local()

    def _conn(self):
        conn = getattr(self._local, "conn", None)
        if conn is None:
            conn = http.client.HTTPConnection("127.0.0.1", 7421, timeout=10)
            self._local.conn = conn
        return conn

    def ok(self) -> bool:
        try:
            conn = self._conn()
            conn.request("GET", "/api/health")
            resp = conn.getresponse()
            resp.read()
            return resp.status == 200
        except Exception as exc:  # noqa: BLE001
            self.failures[type(exc).__name__] += 1
            self._local.conn = None
            return False


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--stack", choices=["rust", "python"], required=True)
    parser.add_argument("--out", required=True, help="CSV output path")
    parser.add_argument("--sequential", type=int, default=10000)
    parser.add_argument("--concurrent", type=int, default=1000)
    parser.add_argument("--every", type=int, default=500)
    args = parser.parse_args()

    out_path = Path(args.out)
    out_path.parent.mkdir(parents=True, exist_ok=True)

    workdir = Path(tempfile.mkdtemp(prefix="netrail-load-"))
    env = os.environ.copy()
    env.update(
        {
            "HOME": str(workdir),
            "XDG_CONFIG_HOME": str(workdir / "config"),
            "XDG_DATA_HOME": str(workdir / "data"),
            "XDG_CACHE_HOME": str(workdir / "cache"),
            "NETRAIL_DB_PATH": str(workdir / "netrail.db"),
            "NETRAIL_DB_KEY": "X" * 44,  # placeholder; replaced below if needed
            "NETRAIL_RATE_LIMIT": "0",
        }
    )
    import base64

    env["NETRAIL_DB_KEY"] = base64.urlsafe_b64encode(os.urandom(32)).decode()

    if args.stack == "rust":
        cmd = [str(ROOT / "src-tauri/target/release/netrail-api")]
    else:
        cmd = [sys.executable, "-m", "netrail"]

    server = subprocess.Popen(cmd, env=env, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    pid = server.pid
    try:
        healthy = False
        probe = Requester()
        for _ in range(100):
            if probe.ok():
                healthy = True
                break
            time.sleep(0.2)
        if not healthy:
            print(f"ERROR: {args.stack} server never became healthy", file=sys.stderr)
            return 2

        with open(out_path, "w", newline="") as fh:
            writer = csv.writer(fh)
            writer.writerow(["sample", "requests_done", "rss_kib", "fd_count", "sockets"])

            sample_index = 0

            def sample(requests_done: int) -> None:
                nonlocal sample_index
                sample_index += 1
                writer.writerow(
                    [
                        sample_index,
                        requests_done,
                        read_rss_kib(pid),
                        read_fd_count(pid),
                        read_socket_count(),
                    ]
                )

            # Sequential phase
            req = Requester()
            ok = 0
            for i in range(args.sequential):
                if req.ok():
                    ok += 1
                if (i + 1) % args.every == 0:
                    sample(i + 1)
            print(f"sequential: {ok}/{args.sequential} ok")

            # Concurrent phase
            concurrency = 50
            per_worker = max(1, args.concurrent // concurrency)

            def worker(_: int) -> tuple[int, Counter[str]]:
                wreq = Requester()
                oks = sum(1 for _ in range(per_worker) if wreq.ok())
                return oks, wreq.failures

            with ThreadPoolExecutor(max_workers=concurrency) as pool:
                results = list(pool.map(worker, range(concurrency)))
            concurrent_ok = sum(ok for ok, _ in results)
            print(f"concurrent: {concurrent_ok}/{args.concurrent} ok")

            sample(args.sequential + args.concurrent)

        total = args.sequential + args.concurrent
        total_ok = ok + concurrent_ok
        completeness = 100.0 * total_ok / total if total else 0.0
        print(f"completeness: {completeness:.1f}%  csv={out_path}")
        all_failures: Counter[str] = Counter()
        all_failures.update(req.failures)
        for _, failures in results:
            all_failures.update(failures)
        if all_failures:
            print(f"failures by cause: {dict(all_failures)}")
        return 0 if total_ok == total else 1
    finally:
        server.send_signal(signal.SIGTERM)
        try:
            server.wait(timeout=10)
        except subprocess.TimeoutExpired:
            server.kill()
            server.wait()


if __name__ == "__main__":
    sys.exit(main())
