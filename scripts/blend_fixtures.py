"""Generate the deterministic blend-mode reference inputs (ADR 0004).

Writes `bottom.png` and `top.png` (512x512 straight RGBA8, no colour profile)
into crates/hoshi-core/tests/fixtures/blend/. Standard library only, so the
output is byte-identical on every machine. See docs/BLEND_REFERENCE.md.
"""

import colorsys
import pathlib
import struct
import sys
import zlib

SIZE = 512
OUT = pathlib.Path(__file__).resolve().parents[1] / "crates/hoshi-core/tests/fixtures/blend"
ALPHAS = [0, 1, 16, 32, 64, 96, 127, 128, 160, 192, 224, 240, 254, 255, 255, 255]
EDGE_COLORS = [
    (0, 0, 0), (255, 255, 255), (1, 1, 1), (254, 254, 254), (127, 127, 127),
    (128, 128, 128), (255, 0, 0), (0, 255, 0), (0, 0, 255), (255, 255, 0),
    (0, 255, 255), (255, 0, 255), (64, 32, 16), (16, 32, 64), (200, 100, 50),
    (50, 100, 200),
]


def hsv(h, s, v):
    r, g, b = colorsys.hsv_to_rgb(h % 1.0, s, v)
    return tuple(round(c * 255) for c in (r, g, b))


def lcg(seed):
    while True:
        seed = (seed * 1103515245 + 12345) & 0x7FFFFFFF
        yield seed >> 8


def pixel(layer, x, y, noise):
    """Return (r, g, b, a) for one layer; layer is 'bottom' or 'top'."""
    bottom = layer == "bottom"
    if x < 256 and y < 256:  # Q1: separable channel table, opaque gray ramps.
        v = x if bottom else y
        return (v, v, v, 255)
    if y < 256:  # Q2: alpha grid, 16x16 cells of 16x16 px.
        i, j = (x - 256) // 16, y // 16
        return (200, 120, 40, ALPHAS[j]) if bottom else (40, 90, 220, ALPHAS[i])
    if x < 256:  # Q3: non-separable colour (Hue/Saturation/Color/Luminosity).
        if bottom:
            return hsv(x / 256, 1 - (y - 256) / 320, 1.0) + (255,)
        i, j = x // 16, (y - 256) // 16
        sat = (1.0, 0.6, 0.3, 0.0)[j % 4]
        val = (1.0, 0.75, 0.5, 0.25)[j // 4]
        return hsv(i / 16 + 0.5, sat, val) + (255,)
    # Q4: edge cases. Top half of the quadrant: fixed colour pairs; bottom half:
    # seeded noise. Column 0 cells carry RGB under alpha 0 on the bottom layer.
    i, j = (x - 256) // 16, (y - 256) // 16
    if j < 8:
        if bottom and i == 0:  # non-black RGB hidden under alpha 0
            return EDGE_COLORS[(j + 6) % 16] + (0,)
        return EDGE_COLORS[(i + (0 if bottom else j * 3 + 5)) % 16] + (255,)
    return (next(noise) & 255, next(noise) & 255, next(noise) & 255, next(noise) & 255)


def png(width, height, rows):
    def chunk(kind, data):
        body = kind + data
        return struct.pack(">I", len(data)) + body + struct.pack(">I", zlib.crc32(body))

    raw = b"".join(b"\x00" + bytes(row) for row in rows)
    header = struct.pack(">IIBBBBB", width, height, 8, 6, 0, 0, 0)
    return (b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", header)
            + chunk(b"IDAT", zlib.compress(raw, 9)) + chunk(b"IEND", b""))


def render(layer):
    noise = lcg(1 if layer == "bottom" else 2)
    rows = []
    for y in range(SIZE):
        row = []
        for x in range(SIZE):
            row.extend(pixel(layer, x, y, noise))
        rows.append(row)
    return png(SIZE, SIZE, rows)


def main():
    OUT.mkdir(parents=True, exist_ok=True)
    for layer in ("bottom", "top"):
        (OUT / f"{layer}.png").write_bytes(render(layer))
    print(f"wrote {OUT}/bottom.png and top.png")
    return 0


if __name__ == "__main__":
    sys.exit(main())
