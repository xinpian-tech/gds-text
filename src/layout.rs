//! Hierarchical "one GDS cell per snippet" layout mode.
//!
//! In this mode each entry becomes its own `GdsStruct` (cell) sized to the
//! snippet's tight bounding box. The top cell contains a `GdsStructRef`
//! for each entry at its layout position.

use anyhow::Result;
use gds21::{GdsDateTimes, GdsElement, GdsLibrary, GdsPoint, GdsStruct, GdsStructRef, GdsUnits};
use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::bitmap::MergedRegion;
use crate::config::{DesignRules, LayerConfig};
use crate::gds_out;
use crate::text_render::TextRenderer;

/// One layout entry -- a single text snippet with its own local canvas.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutEntry {
    pub id: u64,
    pub text: String,
    /// Position of this cell inside the top cell, in grid cells.
    pub x: f32,
    pub y: f32,
    pub font_size: f32,
    #[serde(default)]
    pub rotation_deg: f32,
}

/// Hierarchical layout configuration consumed by [`write_layout_gds`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutConfig {
    pub grid_nm: u32,
    pub font_name: String,
    pub layers: LayerConfig,
    #[serde(default)]
    pub rules: DesignRules,
    /// Top cell dimensions in grid cells.
    pub canvas_width_px: u32,
    pub canvas_height_px: u32,
    pub entries: Vec<LayoutEntry>,
}

/// Emit a hierarchical GDS with one cell per entry.
pub fn write_layout_gds(
    cfg: &LayoutConfig,
    renderer: &mut TextRenderer,
    path: &Path,
) -> Result<()> {
    let grid_nm = cfg.grid_nm as i32;
    let units = GdsUnits::new(1e-3, 1e-9);

    let mut lib = GdsLibrary::new("GDS_TEXT");
    lib.units = units;
    lib.dates = GdsDateTimes::default();

    let top_h = cfg.canvas_height_px as i32;
    let mut top = GdsStruct::new("TOP");

    for entry in &cfg.entries {
        let cell_name = format!("C{:07}", entry.id);

        let snippet = crate::config::TextSnippet {
            id: entry.id,
            text: entry.text.clone(),
            x: 0.0,
            y: 0.0,
            font_size: entry.font_size,
            rotation_deg: 0.0,
        };
        let bmp = renderer.rasterize(&snippet, &cfg.font_name)?;
        let rotated = bmp.rotate(entry.rotation_deg);
        let h = rotated.height() as i32;
        let regions = rotated.to_merged_regions();

        // Build the cell with merged local-coordinate geometry. Flip Y
        // inside the cell so the text reads upright once placed.
        let mut cell = GdsStruct::new(cell_name.clone());
        for region in regions {
            match region {
                MergedRegion::Polygon(pts) => {
                    cell.elems.push(gds_out::polygon_boundary(
                        &pts,
                        0,
                        0,
                        h,
                        grid_nm,
                        cfg.layers.text_layer,
                        cfg.layers.text_datatype,
                    ));
                }
                MergedRegion::Rectangles(rects) => {
                    for r in &rects {
                        let gy = h - r.y as i32 - r.h as i32;
                        cell.elems.push(gds_out::rect_boundary(
                            r.x as i32,
                            gy,
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
        lib.structs.push(cell);

        // Top cell: reference the entry cell at its layout position. The
        // entry.y is measured from the canvas top; convert to GDS Y.
        let ref_x = (entry.x.round() as i32) * grid_nm;
        let ref_y = ((top_h - entry.y.round() as i32 - h) * grid_nm).max(0);
        top.elems.push(GdsElement::GdsStructRef(GdsStructRef {
            name: cell_name,
            xy: GdsPoint::new(ref_x, ref_y),
            ..Default::default()
        }));
    }

    lib.structs.push(top);
    lib.save(path)
        .map_err(|e| anyhow::anyhow!("gds21 save failed: {e}"))?;
    Ok(())
}
