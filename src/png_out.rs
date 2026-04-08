//! PNG rendering of the same layout we write to GDSII, and GDS-to-PNG round
//! trip verification.

use anyhow::Result;
use gds21::{GdsElement, GdsLibrary};
use std::path::Path;

use crate::config::{LayerConfig, ProjectConfig};
use crate::fill;
use crate::gds_out;
use crate::text_render::TextRenderer;

/// Write a PNG preview of the layout. Scale factor determines how many pixels
/// each grid cell becomes in the output image.
pub fn write_png(
    cfg: &ProjectConfig,
    renderer: &mut TextRenderer,
    path: &Path,
    scale: u32,
) -> Result<()> {
    let scale = scale.max(1);
    let w = cfg.canvas_width_px * scale;
    let h = cfg.canvas_height_px * scale;

    let mut img = image::RgbImage::new(w, h);
    for px in img.pixels_mut() {
        *px = image::Rgb([255, 255, 255]);
    }

    let text_cells = gds_out::collect_text_cells(cfg, renderer);
    let fill_cells = fill::compute_fill_cells(cfg, &text_cells);

    // Fill is gray, drawn first so text overlaps on top.
    for (gx, gy) in fill_cells {
        plot_cell(&mut img, gx, gy, scale, image::Rgb([160, 160, 160]));
    }
    for (gx, gy) in text_cells {
        plot_cell(&mut img, gx, gy, scale, image::Rgb([0, 0, 0]));
    }

    img.save(path)?;
    Ok(())
}

/// Read a GDSII file back and render every boundary element to a PNG.
/// This verifies end-to-end round-trip through the on-disk GDSII format.
pub fn render_gds(
    gds_path: &Path,
    png_path: &Path,
    layers: LayerConfig,
    scale: u32,
    padding_nm: i32,
) -> Result<()> {
    let lib = GdsLibrary::load(gds_path).map_err(|e| anyhow::anyhow!("gds21 load failed: {e}"))?;
    let grid_nm = (lib.units.db_unit() * 1e9).round() as i32;
    let grid_nm = grid_nm.max(1);

    // Collect boundary boxes with their layer info.
    struct Box2 {
        layer: i16,
        datatype: i16,
        x0: i32,
        y0: i32,
        x1: i32,
        y1: i32,
    }
    let mut boxes: Vec<Box2> = Vec::new();
    let mut min_x = i32::MAX;
    let mut min_y = i32::MAX;
    let mut max_x = i32::MIN;
    let mut max_y = i32::MIN;

    for s in &lib.structs {
        for el in &s.elems {
            if let GdsElement::GdsBoundary(b) = el {
                let xs: Vec<i32> = b.xy.iter().map(|p| p.x).collect();
                let ys: Vec<i32> = b.xy.iter().map(|p| p.y).collect();
                let x0 = *xs.iter().min().unwrap();
                let y0 = *ys.iter().min().unwrap();
                let x1 = *xs.iter().max().unwrap();
                let y1 = *ys.iter().max().unwrap();
                min_x = min_x.min(x0);
                min_y = min_y.min(y0);
                max_x = max_x.max(x1);
                max_y = max_y.max(y1);
                boxes.push(Box2 {
                    layer: b.layer,
                    datatype: b.datatype,
                    x0,
                    y0,
                    x1,
                    y1,
                });
            }
        }
    }

    if boxes.is_empty() {
        anyhow::bail!("no boundary elements in {}", gds_path.display());
    }

    // Add padding around the bounding box.
    min_x -= padding_nm;
    min_y -= padding_nm;
    max_x += padding_nm;
    max_y += padding_nm;

    let w_nm = (max_x - min_x).max(1);
    let h_nm = (max_y - min_y).max(1);
    let w_cells = (w_nm / grid_nm).max(1) as u32;
    let h_cells = (h_nm / grid_nm).max(1) as u32;
    let w_px = w_cells * scale;
    let h_px = h_cells * scale;

    let mut img = image::RgbImage::new(w_px, h_px);
    for px in img.pixels_mut() {
        *px = image::Rgb([255, 255, 255]);
    }

    for b in &boxes {
        let (r, g, bl) = if b.layer == layers.text_layer && b.datatype == layers.text_datatype {
            (0, 0, 0)
        } else if b.layer == layers.fill_layer && b.datatype == layers.fill_datatype {
            (160, 160, 160)
        } else {
            (40, 100, 200)
        };
        let color = image::Rgb([r, g, bl]);

        let gx0 = (b.x0 - min_x) / grid_nm;
        let gy0 = (b.y0 - min_y) / grid_nm;
        let gx1 = (b.x1 - min_x) / grid_nm;
        let gy1 = (b.y1 - min_y) / grid_nm;

        // GDSII has Y going up; our image has Y going down -> flip.
        let flipped_y0 = h_cells as i32 - gy1;
        let flipped_y1 = h_cells as i32 - gy0;

        let px0 = (gx0 * scale as i32).max(0) as u32;
        let py0 = flipped_y0.max(0) as u32 * scale;
        let px1 = ((gx1 * scale as i32).min(w_px as i32)).max(0) as u32;
        let py1 = (flipped_y1.max(0) as u32 * scale).min(h_px);

        for y in py0..py1 {
            for x in px0..px1 {
                img.put_pixel(x, y, color);
            }
        }
    }

    img.save(png_path)?;
    Ok(())
}

fn plot_cell(img: &mut image::RgbImage, gx: i32, gy: i32, scale: u32, color: image::Rgb<u8>) {
    if gx < 0 || gy < 0 {
        return;
    }
    let (w, h) = (img.width(), img.height());
    let x0 = (gx as u32) * scale;
    let y0 = (gy as u32) * scale;
    if x0 >= w || y0 >= h {
        return;
    }
    let x1 = (x0 + scale).min(w);
    let y1 = (y0 + scale).min(h);
    for y in y0..y1 {
        for x in x0..x1 {
            img.put_pixel(x, y, color);
        }
    }
}
