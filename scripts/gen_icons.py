#!/usr/bin/env python3
"""Generates the app icon set (PNG + ICO) for MC Server Manager using only the stdlib.

Usage: python3 scripts/gen_icons.py [out_dir]
"""
import os
import struct
import sys
import zlib

SIZE = 1024
OUT_DIR = sys.argv[1] if len(sys.argv) > 1 else "icons"
os.makedirs(OUT_DIR, exist_ok=True)

# palette
BG = (11, 18, 32, 255)          # #0b1220 deep navy
TOP = (111, 180, 93, 255)       # grass green
LEFT = (87, 146, 74, 255)
RIGHT = (65, 111, 56, 255)
EDGE = (7, 12, 20, 255)
DARK = (147, 202, 131, 255)     # highlight strip on top


def in_polygon(x, y, poly):
    inside = False
    n = len(poly)
    j = n - 1
    for i in range(n):
        xi, yi = poly[i]
        xj, yj = poly[j]
        if (yi > y) != (yj > y) and x < (xj - xi) * (y - yi) / (yj - yi + 1e-9) + xi:
            inside = not inside
        j = i
    return inside


def rounded_rect(x, y, x0, y0, x1, y1, r):
    if x < x0 or x > x1 or y < y0 or y > y1:
        return False
    if x0 + r <= x <= x1 - r or y0 + r <= y <= y1 - r:
        return True
    cx = x0 + r if x < x0 + r else (x1 - r if x > x1 - r else x)
    cy = y0 + r if y < y0 + r else (y1 - r if y > y1 - r else y)
    return (x - cx) ** 2 + (y - cy) ** 2 <= r * r


def cube_polys(cx, cy, r):
    h = r  # vertical height of sides
    top = [(cx, cy - r), (cx + 2 * r, cy), (cx, cy + r), (cx - 2 * r, cy)]
    left = [(cx - 2 * r, cy), (cx, cy + r), (cx, cy + r + h), (cx - 2 * r, cy + h)]
    right = [(cx + 2 * r, cy), (cx, cy + r), (cx, cy + r + h), (cx + 2 * r, cy + h)]
    return top, left, right


def raster():
    x0, y0, x1, y1 = 0, 0, SIZE, SIZE
    padding = SIZE * 0.06
    radius = int(SIZE * 0.16)
    cx, cy = SIZE / 2, SIZE / 2 + SIZE * 0.02
    r = SIZE * 0.24
    top, left, right = cube_polys(cx, cy, r)
    # highlight diamond on the top face
    hr = r * 0.42
    hi = [(cx, cy - hr), (cx + hr, cy), (cx, cy + hr), (cx - hr, cy)]

    px = bytearray(SIZE * SIZE * 4)
    for y in range(SIZE):
        row = y * SIZE * 4
        for x in range(SIZE):
            color = (0, 0, 0, 0)
            if rounded_rect(x, y, int(padding), int(padding), int(SIZE - padding), int(SIZE - padding), radius):
                color = BG
                if in_polygon(x, y, hi):
                    color = DARK
                elif in_polygon(x, y, top):
                    color = TOP
                elif in_polygon(x, y, left):
                    color = LEFT
                elif in_polygon(x, y, right):
                    color = RIGHT
                # cube edge outline
                for poly in (top, left, right):
                    if edge_dist(x, y, poly) < 6:
                        color = EDGE
                        break
            i = row + x * 4
            px[i] = color[0]
            px[i + 1] = color[1]
            px[i + 2] = color[2]
            px[i + 3] = color[3]
    return bytes(px)


def edge_dist(x, y, poly):
    n = len(poly)
    best = 1e18
    for i in range(n):
        x1, y1 = poly[i]
        x2, y2 = poly[(i + 1) % n]
        best = min(best, seg_dist(x, y, x1, y1, x2, y2))
    return best


def seg_dist(px, py, x1, y1, x2, y2):
    dx = x2 - x1
    dy = y2 - y1
    if dx == 0 and dy == 0:
        return ((px - x1) ** 2 + (py - y1) ** 2) ** 0.5
    t = max(0.0, min(1.0, ((px - x1) * dx + (py - y1) * dy) / (dx * dx + dy * dy)))
    qx = x1 + t * dx
    qy = y1 + t * dy
    return ((px - qx) ** 2 + (py - qy) ** 2) ** 0.5


def png(width, height, pixels):
    def chunk(tag, data):
        c = tag + data
        return struct.pack(">I", len(data)) + c + struct.pack(">I", zlib.crc32(c) & 0xFFFFFFFF)

    raw = bytearray()
    stride = width * 4
    for y in range(height):
        raw.append(0)  # filter: none
        raw += pixels[y * stride:(y + 1) * stride]
    ihdr = struct.pack(">IIBBBBB", width, height, 8, 6, 0, 0, 0)
    return (
        b"\x89PNG\r\n\x1a\n"
        + chunk(b"IHDR", ihdr)
        + chunk(b"IDAT", zlib.compress(bytes(raw), 9))
        + chunk(b"IEND", b"")
    )


def downscale(src, src_size, dst_size, row_size=4):
    out = bytearray(dst_size * dst_size * 4)
    scale = src_size / dst_size
    for y in range(dst_size):
        for x in range(dst_size):
            x0f = x * scale
            y0f = y * scale
            ax = int(x0f)
            ay = int(y0f)
            bx = min(ax + int(scale), src_size - 1)
            by = min(ay + int(scale), src_size - 1)
            rs = gs = bs = as_ = 0
            cnt = 0
            for yy in range(ay, by + 1):
                for xx in range(ax, bx + 1):
                    i = (yy * src_size + xx) * 4
                    rs += src[i]
                    gs += src[i + 1]
                    bs += src[i + 2]
                    as_ += src[i + 3]
                    cnt += 1
            cnt = max(cnt, 1)
            o = (y * dst_size + x) * 4
            out[o] = rs // cnt
            out[o + 1] = gs // cnt
            out[o + 2] = bs // cnt
            out[o + 3] = as_ // cnt
    return bytes(out)


def ico_from_pngs(pngs):
    """ICO file wrapping PNG-compressed images (Vista+)."""
    count = len(pngs)
    header = struct.pack("<HHH", 0, 1, count)
    entries = b""
    offset = 6 + 16 * count
    data = b""
    for (w, h, blob) in pngs:
        wb = 0 if w >= 256 else w
        hb = 0 if h >= 256 else h
        entries += struct.pack("<BBBBHHII", wb, hb, 0, 0, 1, 32, len(blob), offset)
        offset += len(blob)
        data += blob
    return header + entries + data


def main():
    full = raster()
    images = {}
    for size in (32, 128, 256, 512, 1024):
        if size == 1024:
            images[size] = full
        else:
            images[size] = downscale(full, 1024, size)
    for size, blob in images.items():
        with open(os.path.join(OUT_DIR, f"{size}x{size}.png"), "wb") as f:
            f.write(png(size, size, blob))
    with open(os.path.join(OUT_DIR, "icon.png"), "wb") as f:
        f.write(png(512, 512, images[512]))
    ico = ico_from_pngs([(32, 32, png(32, 32, images[32])),
                          (128, 128, png(128, 128, images[128])),
                          (256, 256, png(256, 256, images[256]))])
    with open(os.path.join(OUT_DIR, "icon.ico"), "wb") as f:
        f.write(ico)
    # tauri also expects 128x128@2x
    with open(os.path.join(OUT_DIR, "128x128@2x.png"), "wb") as f:
        f.write(png(256, 256, images[256]))
    print("icons written to", OUT_DIR)


if __name__ == "__main__":
    main()