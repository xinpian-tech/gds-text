# gds-text

Render text snippets to GDSII and PDF with Calibre-style dummy fill.

- Font: Sarasa Mono SC (CJK monospace)
- Output: GDSII (via `gds21`) and PDF (via `printpdf`)
- Rust edition 2024, managed with Nix flake
- Dot-matrix raster aligned to an integer nm grid (>= 100 nm)
- Calibre-style dummy fill with configurable density
- Sky130-inspired design rule minimums
- egui-based GUI with draggable, rotatable text snippets

## Build

```
nix develop
cargo run --release
```

## Usage

Add text snippets from the toolbar, drag them on the canvas, set their text,
font size, rotation and position in the properties panel. Export to GDSII or
PDF from the toolbar.

Design rules, grid precision, fill density, and layer numbers are all
configurable in the properties panel.
