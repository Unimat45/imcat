#!/usr/bin/env python3
"""Regenerates the test fixture images in this directory.

The fixtures are committed, so this only needs to run if they change.
It uses Pillow (pip install pillow) on purpose: encoding with a different
library than the one imcat decodes with makes the format tests meaningful.
After regenerating, refresh the snapshots with `IMCAT_BLESS=1 cargo test`.
"""

import math
import struct
from pathlib import Path

from PIL import Image

HERE = Path(__file__).parent


def gradient(w, h, alpha):
    """Smooth colour ramps plus fine diagonal detail, optionally with varying alpha."""
    im = Image.new("RGBA", (w, h))
    px = im.load()
    for y in range(h):
        for x in range(w):
            a = int(255 * (0.5 + 0.5 * math.sin(x / 5.0 + y / 7.0))) if alpha else 255
            detail = 255 if (x + y) % 4 < 2 else 0
            px[x, y] = (x * 255 // (w - 1), y * 255 // (h - 1), detail, a)
    return im


def palette_image():
    """16x16, at most 256 colours, so even GIF stores it losslessly."""
    im = Image.new("RGB", (16, 16))
    px = im.load()
    for y in range(16):
        for x in range(16):
            px[x, y] = ((x * 17) & 0xF0, (y * 17) & 0xF0, ((x ^ y) * 16) & 0xF0)
    return im


def farbfeld(im, path):
    """Pillow can't write farbfeld; the format is simple enough to do by hand."""
    im = im.convert("RGBA")
    w, h = im.size
    pixels = im.load()
    with open(path, "wb") as f:
        f.write(b"farbfeld" + struct.pack(">II", w, h))
        for y in range(h):
            for x in range(w):
                f.write(struct.pack(">4H", *(c * 257 for c in pixels[x, y])))


def main():
    gradient(64, 40, alpha=False).save(HERE / "gradient.png")
    gradient(48, 31, alpha=True).save(HERE / "alpha.png")  # odd height on purpose

    formats = HERE / "formats"
    formats.mkdir(exist_ok=True)
    pal = palette_image()
    pal.save(formats / "reference.png")
    # Lossless: must render exactly like reference.png.
    pal.save(formats / "image.bmp")
    pal.save(formats / "image.gif")
    # ICO requires embedded PNGs to be RGBA (and the image crate enforces it).
    pal.convert("RGBA").save(formats / "image.ico", sizes=[(16, 16)])
    pal.save(formats / "image.ppm")
    pal.save(formats / "image.qoi")
    pal.save(formats / "image.tga")
    pal.save(formats / "image.tiff", compression="tiff_lzw")
    pal.save(formats / "image.webp", lossless=True)
    farbfeld(pal, formats / "image.ff")
    pal.save(formats / "no_extension", format="PNG")
    # Lossy: only checked for decoding at the right size.
    pal.save(formats / "image.jpg", quality=95)
    pal.save(formats / "image.dds", pixel_format="DXT1")

    # 40x20 pixels stored sideways: EXIF orientation 6 means "rotate 90° clockwise
    # to display", so imcat must show it 20 wide and 40 tall. The left half is red
    # and the right half blue, so after rotating, red is on top.
    sideways = Image.new("RGB", (40, 20), (255, 0, 0))
    sideways.paste((0, 0, 255), (20, 0, 40, 20))
    exif = Image.Exif()
    exif[0x0112] = 6
    sideways.save(HERE / "orientation6.jpg", quality=95, exif=exif)


if __name__ == "__main__":
    main()
