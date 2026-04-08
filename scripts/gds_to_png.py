#!/usr/bin/env python3
"""Render a GDSII file to PNG using klayout.

Usage (inside `nix develop`):
    klayout -zz -r gds_to_png.py -rd gds=/path/to/in.gds -rd png=/path/to/out.png

klayout is a real GDS viewer, so this verifies the file opens and displays
correctly.
"""

import pya  # type: ignore

import os
import sys

# klayout -rd KEY=VALUE injects KEY into script globals; fall back to env.
try:
    gds_path = gds  # type: ignore[name-defined]
except NameError:
    gds_path = os.environ.get("gds")
try:
    png_path = png  # type: ignore[name-defined]
except NameError:
    png_path = os.environ.get("png")

if not gds_path or not png_path:
    print("usage: klayout -zz -r gds_to_png.py -rd gds=... -rd png=...", file=sys.stderr)
    raise SystemExit(1)

print(f"loading {gds_path}")
lv = pya.LayoutView()
cv_idx = lv.load_layout(gds_path, 0)
lv.select_cell(lv.active_cellview().layout().top_cell().cell_index(), cv_idx)

# Build a simple layer properties list.
lv.clear_layers()
palette = [
    ("#000000", "#ffffff"),
    ("#a0a0a0", "#303030"),
    ("#2080ff", "#001a33"),
    ("#ff8020", "#331a00"),
]
layers = list(lv.active_cellview().layout().layer_indexes())
for idx, li in enumerate(layers):
    info = lv.active_cellview().layout().get_info(li)
    lp = pya.LayerPropertiesNode()
    lp.source = f"{info.layer}/{info.datatype}@1"
    fill, frame = palette[idx % len(palette)]
    lp.fill_color = int(fill.lstrip("#"), 16)
    lp.frame_color = int(frame.lstrip("#"), 16)
    lp.dither_pattern = 1  # solid
    lp.visible = True
    lv.insert_layer(lv.end_layers(), lp)

lv.max_hier()
lv.zoom_fit()

width = 1600
height = 1000
lv.save_image(png_path, width, height)
print(f"wrote {png_path}")
