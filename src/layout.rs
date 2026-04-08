//! Hierarchical "one GDS cell per snippet" layout mode.
//!
//! In this mode each entry becomes its own `GdsStruct` (cell) sized to the
//! snippet's tight bounding box. The top cell contains a `GdsStructRef`
//! for each entry at its layout position.

use anyhow::Result;
use gds21::{
    GdsBoundary, GdsDateTimes, GdsElement, GdsLibrary, GdsPoint, GdsStruct, GdsStructRef, GdsUnits,
};
use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::config::{DesignRules, LayerConfig};
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

        // Build the cell with local-coordinate pixel boundaries. Flip Y
        // inside the cell so the text reads upright once this cell is
        // placed in the top cell.
        let mut cell = GdsStruct::new(cell_name.clone());
        for (x, y) in rotated.iter_on() {
            let gx = x as i32;
            let gy = h - 1 - y as i32;
            cell.elems.push(pixel_box(
                gx,
                gy,
                grid_nm,
                cfg.layers.text_layer,
                cfg.layers.text_datatype,
            ));
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
