//! GDSII output: text + fill on metal layer as merged boundaries.

use anyhow::Result;
use gds21::{GdsBoundary, GdsDateTimes, GdsElement, GdsLibrary, GdsPoint, GdsStruct, GdsUnits};
use std::path::Path;

use crate::bitmap::{MergedRegion, Rect};
use crate::config::ProjectConfig;
use crate::fill;
use crate::text_render::TextRenderer;

/// Per-snippet merged rectangles in canvas coordinates (Y-down).
pub struct SnippetRect {
    /// Rectangle in canvas cell coordinates.
    pub rect: Rect,
}

/// Rasterize every snippet, rotate it, merge the pixels into rectangles,
/// and translate each rectangle into the canvas coordinate frame.
pub fn collect_text_rects(cfg: &ProjectConfig, renderer: &mut TextRenderer) -> Vec<SnippetRect> {
    let mut out = Vec::new();
    for snippet in &cfg.snippets {
        let Ok(bmp) = renderer.rasterize(snippet, &cfg.font_name) else {
            continue;
        };
        let rotated = bmp.rotate(snippet.rotation_deg);
        let rects = rotated.to_rectangles();
        let ox = snippet.x.round() as i32;
        let oy = snippet.y.round() as i32;
        for r in rects {
            out.push(SnippetRect {
                rect: Rect {
                    x: (ox + r.x as i32).max(0) as u32,
                    y: (oy + r.y as i32).max(0) as u32,
                    w: r.w,
                    h: r.h,
                },
            });
        }
    }
    out
}

/// Expand the merged rectangles back into a dense list of (gx, gy) cells.
/// Used by the dummy-fill exclusion calculation.
fn rects_to_cells(rects: &[SnippetRect]) -> Vec<(i32, i32)> {
    let mut cells = Vec::new();
    for sr in rects {
        let r = &sr.rect;
        for dy in 0..r.h {
            for dx in 0..r.w {
                cells.push(((r.x + dx) as i32, (r.y + dy) as i32));
            }
        }
    }
    cells
}

pub fn build_library(cfg: &ProjectConfig, renderer: &mut TextRenderer) -> Result<GdsLibrary> {
    let grid_nm = cfg.grid_nm as i32;
    let units = GdsUnits::new(1e-3, 1e-9);

    let mut lib = GdsLibrary::new("GDS_TEXT");
    lib.units = units;
    lib.dates = GdsDateTimes::default();

    let mut top = GdsStruct::new("TOP");

    // Flip canvas Y to GDS Y (bottom-left origin) so a GDS viewer shows
    // the text upright.
    let canvas_h = cfg.canvas_height_px as i32;

    let mut used_cells: Vec<(i32, i32)> = Vec::new();
    for snippet in &cfg.snippets {
        let Ok(bmp) = renderer.rasterize(snippet, &cfg.font_name) else {
            continue;
        };
        let rotated = bmp.rotate(snippet.rotation_deg);
        let ox = snippet.x.round() as i32;
        let oy = snippet.y.round() as i32;

        // Grow the used-cell set (used only by the dummy-fill exclusion).
        for (x, y) in rotated.iter_on() {
            used_cells.push((ox + x as i32, oy + y as i32));
        }

        for region in rotated.to_merged_regions() {
            match region {
                MergedRegion::Polygon(pts) => {
                    top.elems.push(polygon_boundary(
                        &pts,
                        ox,
                        oy,
                        canvas_h,
                        grid_nm,
                        cfg.layers.text_layer,
                        cfg.layers.text_datatype,
                    ));
                }
                MergedRegion::Rectangles(rects) => {
                    for r in rects {
                        let gx = ox + r.x as i32;
                        let gy_top = oy + r.y as i32;
                        let gy_bottom = canvas_h - gy_top - r.h as i32;
                        top.elems.push(rect_boundary(
                            gx,
                            gy_bottom,
                            r.w as i32,
                            r.h as i32,
                            grid_nm,
                            cfg.layers.text_layer,
                            cfg.layers.text_datatype,
                        ));
                    }
                }
            }
        }
    }

    // Fill cells are single 1x1 squares by design.
    let fills = fill::compute_fill_cells(cfg, &used_cells);
    for (gx, gy) in fills {
        let gy_flipped = canvas_h - 1 - gy;
        top.elems.push(rect_boundary(
            gx,
            gy_flipped,
            1,
            1,
            grid_nm,
            cfg.layers.fill_layer,
            cfg.layers.fill_datatype,
        ));
    }

    lib.structs.push(top);
    Ok(lib)
}

/// Build a GdsBoundary from a closed polygon in canvas cell coordinates.
/// The polygon's Y is flipped to match the GDS bottom-left convention, and
/// the result is translated by (ox, oy) in cells.
pub fn polygon_boundary(
    pts: &[(i32, i32)],
    ox: i32,
    oy: i32,
    canvas_h: i32,
    grid_nm: i32,
    layer: i16,
    datatype: i16,
) -> GdsElement {
    let mut xy: Vec<GdsPoint> = pts
        .iter()
        .map(|&(x, y)| {
            let cx = (ox + x) * grid_nm;
            let cy = (canvas_h - (oy + y)) * grid_nm;
            GdsPoint::new(cx, cy)
        })
        .collect();
    if xy.first() != xy.last()
        && let Some(first) = xy.first().cloned()
    {
        xy.push(first);
    }
    GdsElement::GdsBoundary(GdsBoundary {
        layer,
        datatype,
        xy,
        ..Default::default()
    })
}

/// Build a rectangular GdsBoundary at grid-cell origin (gx, gy) spanning
/// `w_cells` x `h_cells` cells, each cell being `grid_nm` nanometres.
pub fn rect_boundary(
    gx: i32,
    gy: i32,
    w_cells: i32,
    h_cells: i32,
    grid_nm: i32,
    layer: i16,
    datatype: i16,
) -> GdsElement {
    let x0 = gx * grid_nm;
    let y0 = gy * grid_nm;
    let x1 = x0 + w_cells * grid_nm;
    let y1 = y0 + h_cells * grid_nm;
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

/// Backwards-compatible cell listing used by the GUI preview pipeline.
pub fn collect_text_cells(cfg: &ProjectConfig, renderer: &mut TextRenderer) -> Vec<(i32, i32)> {
    rects_to_cells(&collect_text_rects(cfg, renderer))
}
