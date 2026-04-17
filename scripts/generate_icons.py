"""
Generate the full set of Tauri bundle icons from scratch.

Design: a green peak-monitor circle with a subtle radial gradient and a
small inner dot, matching the tray icon aesthetic. All outputs are written
to src-tauri/icons/ and overwrite whatever is there (this is intentional —
we want to drop leftover icons from the template project).

Run from the project root:
    python scripts/generate_icons.py
"""

from __future__ import annotations

import os
import struct
from pathlib import Path
from typing import Iterable

from PIL import Image, ImageDraw, ImageFilter

# Brand colors, matching src-tauri/src/state.rs::PeakColor::rgba
PRIMARY = (34, 197, 94)      # #22c55e — green
PRIMARY_DARK = (21, 128, 61) # #15803d — darker green for gradient edge
ACCENT = (240, 253, 244)     # very light green for inner dot


def _draw_peak_circle(size: int) -> Image.Image:
    """Draw a green peak-monitor circle at the requested size (RGBA)."""
    # Oversample 4x and downscale at the end for smooth anti-aliased edges.
    scale = 4
    s = size * scale
    img = Image.new("RGBA", (s, s), (0, 0, 0, 0))
    draw = ImageDraw.Draw(img)

    center = s / 2
    radius = s / 2 - max(2, s // 48)  # leave a small padding

    # Radial gradient: iterate from outer radius inward, drawing rings.
    # This is the simplest portable gradient approach without numpy.
    steps = 60
    for i in range(steps, 0, -1):
        t = i / steps
        r_i = radius * t
        # Lerp from PRIMARY_DARK (outer) to PRIMARY (inner)
        inv = 1 - t
        r = int(PRIMARY[0] * inv + PRIMARY_DARK[0] * t * 0.6 + PRIMARY[0] * 0.4 * t)
        g = int(PRIMARY[1] * inv + PRIMARY_DARK[1] * t * 0.6 + PRIMARY[1] * 0.4 * t)
        b = int(PRIMARY[2] * inv + PRIMARY_DARK[2] * t * 0.6 + PRIMARY[2] * 0.4 * t)
        draw.ellipse(
            (center - r_i, center - r_i, center + r_i, center + r_i),
            fill=(r, g, b, 255),
        )

    # Specular highlight: soft white blob in the upper-left quadrant.
    highlight = Image.new("RGBA", (s, s), (0, 0, 0, 0))
    hdraw = ImageDraw.Draw(highlight)
    hr = radius * 0.55
    hc_x = center - radius * 0.22
    hc_y = center - radius * 0.30
    hdraw.ellipse(
        (hc_x - hr, hc_y - hr, hc_x + hr, hc_y + hr),
        fill=(255, 255, 255, 60),
    )
    highlight = highlight.filter(ImageFilter.GaussianBlur(radius=s / 40))
    img = Image.alpha_composite(img, highlight)

    # Inner accent dot — keeps the icon recognizable at small sizes.
    dot_r = radius * 0.22
    draw = ImageDraw.Draw(img)
    draw.ellipse(
        (center - dot_r, center - dot_r, center + dot_r, center + dot_r),
        fill=ACCENT + (230,),
    )

    # Downscale with LANCZOS for clean edges.
    return img.resize((size, size), Image.LANCZOS)


def _save_png(img: Image.Image, path: Path) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    img.save(path, "PNG", optimize=True)


def _save_ico(path: Path, sizes: Iterable[int]) -> None:
    """Build a multi-resolution Windows .ico from fresh renders."""
    images = [_draw_peak_circle(s) for s in sizes]
    # Pillow's ICO writer takes a base image + `sizes` list for extra frames.
    base = max(images, key=lambda im: im.size[0])
    base.save(
        path,
        format="ICO",
        sizes=[(im.size[0], im.size[1]) for im in images],
    )


def _save_icns_minimal(path: Path) -> None:
    """
    Write a minimal macOS .icns file containing a single 512x512 icon.
    We build the container by hand rather than pulling in an extra
    dependency. Format reference: https://en.wikipedia.org/wiki/Apple_Icon_Image_format
    Uses the `ic09` type (512x512 PNG-encoded).
    """
    img = _draw_peak_circle(512)
    from io import BytesIO

    png_buf = BytesIO()
    img.save(png_buf, "PNG", optimize=True)
    png_bytes = png_buf.getvalue()

    # Single icon chunk
    chunk_type = b"ic09"  # 512x512
    chunk = chunk_type + struct.pack(">I", len(png_bytes) + 8) + png_bytes

    # ICNS header: magic + total file length
    total_len = 8 + len(chunk)
    header = b"icns" + struct.pack(">I", total_len)

    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(header + chunk)


def main() -> None:
    root = Path(__file__).resolve().parent.parent
    icons_dir = root / "src-tauri" / "icons"

    # Wipe leftover template files so we don't ship stale assets.
    if icons_dir.exists():
        for old in icons_dir.iterdir():
            if old.is_file():
                old.unlink()
    icons_dir.mkdir(parents=True, exist_ok=True)

    # Core Tauri bundle icons
    _save_png(_draw_peak_circle(32), icons_dir / "32x32.png")
    _save_png(_draw_peak_circle(128), icons_dir / "128x128.png")
    _save_png(_draw_peak_circle(256), icons_dir / "128x128@2x.png")
    _save_png(_draw_peak_circle(512), icons_dir / "icon.png")

    # Multi-res Windows .ico (the one that shows in the taskbar / installer)
    _save_ico(icons_dir / "icon.ico", [16, 24, 32, 48, 64, 128, 256])

    # Minimal macOS .icns so `pnpm tauri build` on macOS is happy
    _save_icns_minimal(icons_dir / "icon.icns")

    # Windows Store (MSIX) tile logos — Tauri's bundler expects these when
    # targeting the Windows Store, and the template ships them, so we keep
    # the filenames identical but re-render them in our palette.
    store_tiles = {
        "StoreLogo.png": 50,
        "Square30x30Logo.png": 30,
        "Square44x44Logo.png": 44,
        "Square71x71Logo.png": 71,
        "Square89x89Logo.png": 89,
        "Square107x107Logo.png": 107,
        "Square142x142Logo.png": 142,
        "Square150x150Logo.png": 150,
        "Square284x284Logo.png": 284,
        "Square310x310Logo.png": 310,
    }
    for filename, size in store_tiles.items():
        _save_png(_draw_peak_circle(size), icons_dir / filename)

    print(f"Icons written to {icons_dir}")
    for p in sorted(icons_dir.iterdir()):
        print(f"  {p.name}  ({os.path.getsize(p)} bytes)")


if __name__ == "__main__":
    main()
