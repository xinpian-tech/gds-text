//! PDF preview output: renders text + fill as filled squares.

use anyhow::Result;
use printpdf::{Color, Line, Mm, PdfDocument, Point as PdfPoint, Rgb};
use std::fs::File;
use std::io::BufWriter;
use std::path::Path;

use crate::config::ProjectConfig;
use crate::fill;
use crate::gds_out;
use crate::text_render::TextRenderer;

pub fn write_pdf(cfg: &ProjectConfig, renderer: &mut TextRenderer, path: &Path) -> Result<()> {
    let grid = cfg.grid_nm as f64;
    let cell_mm = grid / 1_000_000.0;
    let canvas_w_mm = cfg.canvas_width_px as f64 * cell_mm;
    let canvas_h_mm = cfg.canvas_height_px as f64 * cell_mm;
    let margin = 10.0f64;
    let page_w = (canvas_w_mm + 2.0 * margin).max(100.0);
    let page_h = (canvas_h_mm + 2.0 * margin).max(100.0);

    let (doc, page, layer) =
        PdfDocument::new("gds-text preview", Mm(page_w), Mm(page_h), "layer1");
    let layer_ref = doc.get_page(page).get_layer(layer);

    // Canvas border.
    layer_ref.set_outline_color(Color::Rgb(Rgb::new(0.4, 0.4, 0.4, None)));
    layer_ref.set_outline_thickness(0.3);
    let border = Line {
        points: vec![
            (PdfPoint::new(Mm(margin), Mm(margin)), false),
            (PdfPoint::new(Mm(margin + canvas_w_mm), Mm(margin)), false),
            (
                PdfPoint::new(Mm(margin + canvas_w_mm), Mm(margin + canvas_h_mm)),
                false,
            ),
            (PdfPoint::new(Mm(margin), Mm(margin + canvas_h_mm)), false),
        ],
        is_closed: true,
    };
    layer_ref.add_line(border);

    let text_cells = gds_out::collect_text_cells(cfg, renderer);
    let fill_cells = fill::compute_fill_cells(cfg, &text_cells);

    draw_cells(
        &layer_ref,
        &text_cells,
        margin,
        canvas_h_mm,
        cell_mm,
        Color::Rgb(Rgb::new(0.0, 0.0, 0.0, None)),
    );
    draw_cells(
        &layer_ref,
        &fill_cells,
        margin,
        canvas_h_mm,
        cell_mm,
        Color::Rgb(Rgb::new(0.6, 0.6, 0.6, None)),
    );

    let file = File::create(path)?;
    let mut writer = BufWriter::new(file);
    doc.save(&mut writer)?;
    Ok(())
}

fn draw_cells(
    layer: &printpdf::PdfLayerReference,
    cells: &[(i32, i32)],
    margin: f64,
    canvas_h_mm: f64,
    cell_mm: f64,
    color: Color,
) {
    layer.set_fill_color(color);
    for &(gx, gy) in cells {
        let x_mm = margin + gx as f64 * cell_mm;
        let y_mm = margin + canvas_h_mm - (gy as f64 + 1.0) * cell_mm;
        let rect = Line {
            points: vec![
                (PdfPoint::new(Mm(x_mm), Mm(y_mm)), false),
                (PdfPoint::new(Mm(x_mm + cell_mm), Mm(y_mm)), false),
                (PdfPoint::new(Mm(x_mm + cell_mm), Mm(y_mm + cell_mm)), false),
                (PdfPoint::new(Mm(x_mm), Mm(y_mm + cell_mm)), false),
            ],
            is_closed: true,
        };
        layer.add_line(rect);
    }
}
