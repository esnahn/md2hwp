# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (C) 2026 Euisoon Ahn
"""Runs one gallery example without a GUI and saves every figure it leaves open as SVG."""
import os
import runpy
import sys

import matplotlib

matplotlib.use("Agg")
import matplotlib.pyplot as plt  # noqa: E402

path, out, name = sys.argv[1], sys.argv[2], sys.argv[3]
plt.show = lambda *args, **kwargs: None
os.chdir(os.path.dirname(path))
sys.path.append(os.path.dirname(path))  # sibling helper modules, never ahead of the stdlib
try:
    runpy.run_path(path, run_name="__main__")
except SystemExit:
    pass
for index, number in enumerate(plt.get_fignums()):
    plt.figure(number).savefig(os.path.join(out, f"{name}_{index}.svg"))