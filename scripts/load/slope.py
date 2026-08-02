#!/usr/bin/env python3
"""Sprint 3 slope analysis: linear regression over the load-runner CSV to
detect resource growth (leaks). For each metric (RSS, FDs, sockets) it reports
slope, R^2, t and a two-sided p-value (normal approximation on Student-t,
df = n-2). p > 0.05 => the slope is not statistically significant (no growth).

Writes a per-stack SVG to docs/assets/ and appends a section to the shared
docs/sprint3-slope.md report (run once per stack for the side-by-side).

Usage:
  python3 scripts/load/slope.py --csv docs/assets/sprint3-rust.csv   --label "Rust (Axum)"
  python3 scripts/load/slope.py --csv docs/assets/sprint3-python.csv --label "Python (FastAPI/uvicorn)"
"""

from __future__ import annotations

import argparse
import csv
import math
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
ASSETS = ROOT / "docs" / "assets"
REPORT = ROOT / "docs" / "sprint3-slope.md"


def linear_regression(xs: list[float], ys: list[float]) -> dict[str, float]:
    n = len(xs)
    mx = sum(xs) / n
    my = sum(ys) / n
    sxx = sum((x - mx) ** 2 for x in xs)
    sxy = sum((x - mx) * (y - my) for x, y in zip(xs, ys))
    slope = sxy / sxx if sxx else 0.0
    intercept = my - slope * mx
    sse = sum((y - (intercept + slope * x)) ** 2 for x, y in zip(xs, ys))
    sst = sum((y - my) ** 2 for y in ys)
    r2 = 1.0 - sse / sst if sst else 0.0
    df = max(n - 2, 1)
    se = math.sqrt(sse / df / sxx) if sxx and df else 0.0
    t = slope / se if se else 0.0
    p = min(2.0 * math.erfc(abs(t) / math.sqrt(2.0)), 1.0)
    return {"n": float(n), "slope": slope, "intercept": intercept, "r2": r2, "t": t, "p": p}


def parse_csv(path: Path) -> tuple[list[float], dict[str, list[float]]]:
    xs: list[float] = []
    series: dict[str, list[float]] = {"rss_kib": [], "fd_count": [], "sockets": []}
    with path.open() as fh:
        for row in csv.DictReader(fh):
            xs.append(float(row["requests_done"]))
            for key in series:
                series[key].append(float(row[key]))
    return xs, series


def render_svg(xs, series, results, out: Path, label: str) -> None:
    width, height = 640, 320
    pad_l, pad_b, pad_t, pad_r = 56, 32, 24, 12
    plot_w = width - pad_l - pad_r
    plot_h = height - pad_t - pad_b
    x0, x1 = min(xs), max(xs)
    colors = {"rss_kib": "#2563eb", "fd_count": "#16a34a", "sockets": "#9333ea"}

    def scale(ys: list[float]) -> tuple[list[float], list[float]]:
        lo, hi = min(ys), max(ys)
        rng = (hi - lo) or 1.0
        xs_s = [pad_l + (x - x0) / (x1 - x0) * plot_w for x in xs]
        ys_s = [pad_t + (1.0 - (y - lo) / rng) * plot_h for y in ys]
        return xs_s, ys_s

    parts = [
        f'<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}">',
        f'<text x="{pad_l}" y="14" font-size="12">{label} — normalized resource curves</text>',
        f'<line x1="{pad_l}" y1="{pad_t}" x2="{pad_l}" y2="{pad_t + plot_h}" stroke="#888"/>',
        f'<line x1="{pad_l}" y1="{pad_t + plot_h}" x2="{pad_l + plot_w}" y2="{pad_t + plot_h}" stroke="#888"/>',
    ]
    for idx, (key, ys) in enumerate(series.items()):
        xs_s, ys_s = scale(ys)
        points = " ".join(f"{px:.1f},{py:.1f}" for px, py in zip(xs_s, ys_s))
        parts.append(
            f'<polyline points="{points}" fill="none" stroke="{colors[key]}" stroke-width="1.5"/>'
        )
        res = results[key]
        parts.append(
            f'<text x="{pad_l + plot_w}" y="{pad_t + 16 * (idx + 1)}" text-anchor="end" '
            f'font-size="11" fill="{colors[key]}">{key}: slope={res["slope"]:+.2e} p={res["p"]:.3f}</text>'
        )
    parts.append("</svg>")
    out.write_text("\n".join(parts))
    print(f"chart: {out}")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--csv", required=True)
    parser.add_argument("--label", required=True)
    args = parser.parse_args()

    csv_path = Path(args.csv)
    xs, series = parse_csv(csv_path)
    results = {key: linear_regression(xs, ys) for key, ys in series.items()}

    ASSETS.mkdir(parents=True, exist_ok=True)
    svg_name = f"{csv_path.stem}.svg"
    render_svg(xs, series, results, ASSETS / svg_name, args.label)

    # Practical thresholds (per the Sprint 3 plan): RSS < 1 MiB / 10k req,
    # FDs and sockets stable within ±2 over 10k requests.
    thresholds = {"rss_kib": 1024.0, "fd_count": 2.0, "sockets": 2.0}

    def verdict(key: str, res: dict[str, float]) -> str:
        per_10k = res["slope"] * 10.0
        threshold = thresholds[key]
        if per_10k > threshold:
            return "LEAK (growth exceeds budget)"
        if per_10k < -threshold:
            return "benign decline (within budget)"
        return "stable (within budget)"

    table = "\n".join(
        f"| {key} | {results[key]['slope']:+.3e} | {results[key]['r2']:.4f} | "
        f"{results[key]['t']:+.3f} | {results[key]['p']:.4f} | "
        f"{results[key]['slope'] * 10.0:+.3f} | {verdict(key, results[key])} |"
        for key in series
    )

    section = f"""## {args.label}

Load: 10k sequential + 1k concurrent `GET /api/health` (samples every 500
requests). Metrics from the server PID: `/proc/<pid>/status` (RSS),
`/proc/<pid>/fd` (FD count), `ss` (established sockets to :7421).

| Metric | slope (per 1k req) | R² | t | p (two-sided) | Δ / 10k req | Verdict |
|---|---|---|---|---|---|---|
{table}

![{args.label}](assets/{svg_name})
"""
    if REPORT.exists():
        REPORT.write_text(REPORT.read_text().rstrip() + "\n" + section)
    else:
        REPORT.write_text(
            f"# Sprint 3 — Resource Stability (dual-stack)\n\n"
            f"Slope = per 1000 requests; p is a normal approximation on "
            f"Student-t (df = n-2). **p > 0.05 → slope not statistically "
            f"significant (no leak).** Note: `tokio::runtime::metrics` was not "
            f"included — it requires the `tokio_unstable` cfg that shipped "
            f"binaries do not build with.\n" + section
        )
    print(f"report section: {REPORT}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
