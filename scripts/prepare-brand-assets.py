#!/usr/bin/env python3
"""Crop Gemini watermarks and export NetRail brand assets."""

from __future__ import annotations

import shutil
from pathlib import Path

from PIL import Image, ImageChops, ImageDraw, ImageFilter

ROOT = Path(__file__).resolve().parents[1]
BRANDING = ROOT / "branding"
SOURCES = BRANDING / "sources"
STATIC = ROOT / "netrail" / "static"

# Fanout bars — best contrast at dock/tray sizes
APP_ICON_SRC = SOURCES / "app-icon-source.png"
# Railroad tracks — splash / README hero
SPLASH_SRC = SOURCES / "splash-source.png"


def crop_watermark(img: Image.Image, trim_ratio: float = 0.07) -> Image.Image:
    """Remove bottom-right Gemini diamond watermark and center-crop square."""
    w, h = img.size
    trimmed = img.crop((0, 0, int(w * (1 - trim_ratio)), int(h * (1 - trim_ratio))))
    tw, th = trimmed.size
    side = min(tw, th)
    left = (tw - side) // 2
    top = (th - side) // 2
    return trimmed.crop((left, top, left + side, top + side))


def rounded_square_mask(size: int, radius: int) -> Image.Image:
    mask = Image.new("L", (size, size), 0)
    draw = ImageDraw.Draw(mask)
    draw.rounded_rectangle((0, 0, size, size), radius=radius, fill=255)
    return mask


def export_app_icon(img: Image.Image, out: Path, size: int = 1024) -> None:
    icon = img.resize((size, size), Image.Resampling.LANCZOS)
    mask = rounded_square_mask(size, int(size * 0.18))
    icon.putalpha(mask)
    icon.save(out, optimize=True)


def export_tray_icon(img: Image.Image, out: Path, size: int = 64) -> None:
    """White silhouette on transparent for Linux/macOS tray templates."""
    rgba = img.convert("RGBA").resize((size, size), Image.Resampling.LANCZOS)
    pixels = rgba.load()
    for y in range(size):
        for x in range(size):
            r, g, b, _ = pixels[x, y]
            # Drop dark rounded-square background; keep yellow/white symbol.
            if r < 95 and g < 95 and b < 95:
                pixels[x, y] = (0, 0, 0, 0)
            else:
                pixels[x, y] = (255, 255, 255, 255)
    rgba.save(out, optimize=True)


def export_web_mark(img: Image.Image, out: Path, size: int = 64) -> None:
    icon = img.resize((size, size), Image.Resampling.LANCZOS)
    mask = rounded_square_mask(size, int(size * 0.22))
    icon.putalpha(mask)
    icon.save(out, optimize=True)


def export_splash(img: Image.Image, out: Path) -> None:
    """Splash card: symbol centered on dark canvas."""
    canvas = Image.new("RGBA", (960, 540), (11, 15, 20, 255))
    symbol = crop_watermark(img, trim_ratio=0.07)
    symbol = symbol.resize((280, 280), Image.Resampling.LANCZOS)
    mask = rounded_square_mask(280, 50)
    symbol.putalpha(mask)
    x = (960 - 280) // 2
    y = (540 - 280) // 2 - 20
    canvas.paste(symbol, (x, y), symbol)
    canvas.save(out, optimize=True)


def main() -> None:
    BRANDING.mkdir(exist_ok=True)
    SOURCES.mkdir(exist_ok=True)
    STATIC.mkdir(parents=True, exist_ok=True)

    source_names = (
        "Gemini_Generated_Image_8ioq1t8ioq1t8ioq.png",
        "Gemini_Generated_Image_nvg0janvg0janvg0.png",
        "Gemini_Generated_Image_k8zx5tk8zx5tk8zx.png",
    )

    def resolve_source(name: str) -> Path:
        root_path = ROOT / name
        if root_path.exists():
            return root_path
        archived = SOURCES / name
        if archived.exists():
            return archived
        raise FileNotFoundError(f"Missing source image: {name}")

    gemini_app = resolve_source(source_names[0])
    gemini_splash = resolve_source(source_names[1])
    gemini_alt = resolve_source(source_names[2])

    for src in (gemini_app, gemini_splash, gemini_alt):
        dest = SOURCES / src.name
        if src.resolve() != dest.resolve():
            shutil.copy2(src, dest)

    app_raw = Image.open(gemini_app).convert("RGBA")
    splash_raw = Image.open(gemini_splash).convert("RGBA")

    app_cropped = crop_watermark(app_raw)
    splash_cropped = crop_watermark(splash_raw)

    export_app_icon(app_cropped, BRANDING / "app-icon-1024.png")
    export_tray_icon(app_cropped, BRANDING / "tray-icon.png")
    export_web_mark(app_cropped, STATIC / "icon.png", size=64)
    export_web_mark(app_cropped, STATIC / "icon-128.png", size=128)
    export_splash(splash_raw, STATIC / "splash.png")

    app_cropped.save(APP_ICON_SRC)
    splash_cropped.save(SPLASH_SRC)

    print(f"Wrote {BRANDING / 'app-icon-1024.png'}")
    print(f"Wrote {BRANDING / 'tray-icon.png'}")
    print(f"Wrote {STATIC / 'icon.png'}")
    print(f"Wrote {STATIC / 'splash.png'}")


if __name__ == "__main__":
    main()