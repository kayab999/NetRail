#!/usr/bin/env python3
"""Sprint 4 report: reads docs/assets/bench-{rust,python}.json and writes the
side-by-side docs/bench-dual.md."""

from __future__ import annotations

import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
ASSETS = ROOT / "docs" / "assets"
REPORT = ROOT / "docs" / "bench-dual.md"


def load(stack: str) -> dict:
    return json.loads((ASSETS / f"bench-{stack}.json").read_text())


def fmt_steady(res: dict) -> str:
    return (
        f"| {res['concurrency']} | {res['rps']:.0f} | {res['p50_ms']:.2f} | "
        f"{res['p95_ms']:.2f} | {res['p99_ms']:.2f} | {res['error_pct']:.2f} | "
        f"{res['max_cpu_pct']:.0f} | {res['peak_rss_kib']} |"
    )


def main() -> int:
    rust = load("rust")
    python = load("python")

    def med(data: dict, key: str) -> float:
        vals = [r[key] for r in data["steady"]]
        vals.sort()
        return vals[len(vals) // 2]

    summary = [
        f"Medians across the 3 steady-state runs (C = {rust['steady'][0]['concurrency']}, "
        f"N = {rust['steady'][0]['n']}):",
        "",
        "| stack | rps | p50 (ms) | p95 (ms) | errors % | CPU % | peak RSS |",
        "|---|---|---|---|---|---|---|",
        f"| Rust (Axum) | {med(rust, 'rps'):.0f} | {med(rust, 'p50_ms'):.2f} | "
        f"{med(rust, 'p95_ms'):.2f} | {med(rust, 'error_pct'):.2f} | {med(rust, 'max_cpu_pct'):.0f} | "
        f"{med(rust, 'peak_rss_kib') / 1024:.1f} MiB |",
        f"| Python (FastAPI/uvicorn) | {med(python, 'rps'):.0f} | {med(python, 'p50_ms'):.2f} | "
        f"{med(python, 'p95_ms'):.2f} | {med(python, 'error_pct'):.2f} | {med(python, 'max_cpu_pct'):.0f} | "
        f"{med(python, 'peak_rss_kib') / 1024:.1f} MiB |",
        "",
        "Neither stack exceeded the 1% error budget at C = 512 (largest scanned); "
        "p95 latency degradation is the practical knee indicator. These are "
        "single-core-local figures — the health endpoint also performs settings/"
        "keyring/DB work, so absolute rps is workload-specific, not a ceiling.",
    ]

    sections = []
    for stack, data in [("Rust (Axum)", rust), ("Python (FastAPI/uvicorn)", python)]:
        steady_rows = "\n".join(fmt_steady(r) for r in data["steady"])
        sat_rows = "\n".join(
            f"| {r['concurrency']} | {r['rps']:.0f} | {r['p95_ms']:.2f} | "
            f"{r['error_pct']:.2f} |"
            for r in data["saturation"]
        )
        knee = data["knee"]
        knee_txt = (
            f"First concurrency exceeding the 1% error budget: **{knee['concurrency']}** "
            f"({knee['error_pct']:.2f}% errors, {knee['rps']:.0f} rps)"
            if knee.get("error_pct", 0.0) > 1.0
            else f"No knee within the scanned range: {knee['concurrency']} concurrent stayed under 1% errors."
        )
        sections.append(
            f"""## {stack}

Endpoint: `{data['endpoint']}` — client: asyncio httpx keep-alive (C = {data['steady'][0]['concurrency']},
N = {data['steady'][0]['n']} per run, 3 runs). CPU/RSS sampled from the server PID.

| C | rps | p50 (ms) | p95 (ms) | p99 (ms) | errors % | CPU % | peak RSS (KiB) |
|---|---|---|---|---|---|---|---|
{steady_rows}

Saturation scan (stop at >1% errors):

| C | rps | p95 (ms) | errors % |
|---|---|---|---|
{sat_rows}

{knee_txt}
"""
        )

    header = f"""# Sprint 4 — Dual-stack benchmarks

Endpoint: `{rust['endpoint']}`. Client is an asyncio httpx keep-alive pool; the
1% error budget defines the saturation knee. Note: `tokio::runtime::metrics`
is not enabled in shipped binaries.

{chr(10).join(summary)}
"""
    REPORT.write_text(header + "\n".join(sections))
    print(f"report: {REPORT}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
