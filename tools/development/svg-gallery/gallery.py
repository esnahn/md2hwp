# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (C) 2026 Euisoon Ahn
"""figures: save every gallery figure as SVG. convert: turn each SVG into EMF and PNG with svg2emf.

usage: gallery.py figures|convert <examples directory> <output directory> <svg2emf.exe>
Writes <stage>.json (item -> error text, empty on success) into the output directory.
"""
import json
import os
import subprocess
import sys
from concurrent.futures import ThreadPoolExecutor

stage, examples, out, tool = sys.argv[1], sys.argv[2], sys.argv[3], sys.argv[4]
here = os.path.dirname(os.path.abspath(__file__))
os.makedirs(out, exist_ok=True)


def run_example(path):
    relative = os.path.relpath(path, examples)
    name = relative[:-3].replace(os.sep, "__")
    try:
        done = subprocess.run(
            [sys.executable, "-I", os.path.join(here, "run_example.py"), path, out, name],
            capture_output=True, text=True, timeout=180, errors="replace",
        )
        error = "" if done.returncode == 0 else done.stderr.strip().splitlines()[-1:]
    except subprocess.TimeoutExpired:
        error = ["timeout"]
    return relative, error[0] if error else ""


def convert(svg):
    source = os.path.join(out, svg)
    done = subprocess.run([tool, source], capture_output=True, text=True, errors="replace")
    if done.returncode == 0:
        subprocess.run([tool, source, source[:-4] + ".png"], capture_output=True)
    return svg, done.stderr.strip()


with ThreadPoolExecutor(8) as pool:
    if stage == "figures":
        paths = sorted(
            os.path.join(folder, file)
            for folder, _, files in os.walk(examples)
            for file in files
            if file.endswith(".py")
        )
        results = dict(pool.map(run_example, paths))
    elif stage == "convert":
        results = dict(pool.map(convert, sorted(f for f in os.listdir(out) if f.endswith(".svg"))))
    else:
        sys.exit("stage must be figures or convert")
with open(os.path.join(out, f"{stage}.json"), "w", encoding="utf-8") as handle:
    json.dump(results, handle, ensure_ascii=False, indent=1)
print(len(results), "items;", sum(1 for value in results.values() if not value), "without error")