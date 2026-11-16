# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (C) 2026 Euisoon Ahn
"""Compares the GDI+ renderings of the EMFs with the resvg PNGs of the same SVGs.

usage: compare.py <output directory>
Reads <name>.gdiplus.png and <name>.png, writes compare.json and prints a summary.
"""
import json
import os
import sys

import numpy as np
from PIL import Image

Image.MAX_IMAGE_PIXELS = None  # the PNGs are our own renderings, up to 8000 pixels per side
out = sys.argv[1]
rows = []
for name in sorted(os.listdir(out)):
    if not name.endswith(".gdiplus.png"):
        continue
    stem = name[: -len(".gdiplus.png")]
    emf = np.asarray(Image.open(os.path.join(out, name)).convert("RGB"), dtype=np.int16)
    reference = Image.open(os.path.join(out, stem + ".png")).convert("RGBA")
    white = Image.new("RGBA", reference.size, (255, 255, 255, 255))
    reference = Image.alpha_composite(white, reference).convert("RGB")
    reference = reference.resize((emf.shape[1], emf.shape[0]), Image.LANCZOS)
    difference = np.abs(emf - np.asarray(reference, dtype=np.int16)).max(axis=2)
    rows.append((float(difference.mean()), float((difference > 64).mean()) * 100, stem))
if not rows:
    sys.exit("no <name>.gdiplus.png files to compare")
rows.sort(reverse=True)
with open(os.path.join(out, "compare.json"), "w", encoding="utf-8") as handle:
    json.dump([{"figure": s, "mean": m, "over64_percent": p} for m, p, s in rows], handle, indent=1)
means = np.array([row[0] for row in rows])
print(f"compared {len(rows)}; mean abs diff: median {np.median(means):.2f}, "
      f"p90 {np.percentile(means, 90):.2f}, max {means.max():.2f}")
print("worst 15 (mean diff /255, % pixels differing by more than 64):")
for mean, share, stem in rows[:15]:
    print(f"  {mean:6.2f}  {share:5.2f}%  {stem}")