//! GDSII output: text + fill on metal layer as boundaries.

use anyhow::Result;
use gds21::{GdsBoundary, GdsDateTimes, GdsElement, GdsLibrary, GdsPoint, GdsStruct, GdsUnits};
use std::path::Path;

use crate::config::ProjectConfig;
use crate::fill;
use crate::text_render::TextRenderer;

/// Collect (gx, gy) cells for every snippet after rasterization + rotation,
/// translated by snippet position.
pub fn collect_text_cells(cfg: &ProjectConfig, renderer: &mut TextRenderer) -> Vec<(i32, i32)> {
    let mut out = Vec::new();
    for snippet in &cfg.snippets {
        let Ok(bmp) = renderer.rasterize(snippet, &cfg.font_name) else {
            continue;
        };
        let rotated = bmp.rotate(snippet.rotation_deg);
        let ox = snippet.x.round() as i32;
        let oy = snippet.y.round() as i32;
        for (x, y) in rotated.iter_on() {
            out.push((ox + x as i32, oy + y as i32));
        }
    }
    out
}

pub fn build_library(cfg: &ProjectConfig, renderer: &mut TextRenderer) -> Result<GdsLibrary> {
    let grid_nm = cfg.grid_nm as i32;
    let units = GdsUnits::new(1e-3, 1e-9);

    let mut lib = GdsLibrary::new("GDS_TEXT");
    lib.units = units;
    lib.dates = GdsDateTimes::default();

    let mut top = GdsStruct::new("TOP");

    let text_cells = collect_text_cells(cfg, renderer);
    for &(gx, gy) in &text_cells {
        top.elems.push(pixel_box(
            gx,
            gy,
            grid_nm,
            cfg.layers.text_layer,
            cfg.layers.text_datatype,
        ));
    }

    let fills = fill::compute_fill_cells(cfg, &text_cells);
    for (gx, gy) in fills {
        top.elems.push(pixel_box(
            gx,
            gy,
            grid_nm,
            cfg.layers.fill_layer,
            cfg.layers.fill_datatype,
        ));
    }

    lib.structs.push(top);
    Ok(lib)
}

fn pixel_box(gx: i32, gy: i32, grid_nm: i32, layer: i16, datatype: i16) -> GdsElement {
    let x0 = gx * grid_nm;
    let y0 = gy * grid_nm;
    let x1 = x0 + grid_nm;
    let y1 = y0 + grid_nm;
    GdsElement::GdsBoundary(GdsBoundary {
        layer,
        datatype,
        xy: vec![
            GdsPoint::new(x0, y0),
            GdsPoint::new(x1, y0),
            GdsPoint::new(x1, y1),
            GdsPoint::new(x0, y1),
            GdsPoint::new(x0, y0),
        ],
        ..Default::default()
    })
}

pub fn write_gds(cfg: &ProjectConfig, renderer: &mut TextRenderer, path: &Path) -> Result<()> {
    let lib = build_library(cfg, renderer)?;
    lib.save(path)
        .map_err(|e| anyhow::anyhow!("gds21 save failed: {e}"))?;
    Ok(())
}
