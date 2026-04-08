//! gds-text — render text snippets to GDSII + PDF with Calibre-style dummy fill.

mod app;
mod bitmap;
mod config;
mod fill;
mod gds_out;
mod pdf_out;
mod text_render;

use eframe::egui;

fn main() -> eframe::Result {
    env_logger::init();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 800.0])
            .with_min_inner_size([900.0, 600.0])
            .with_title("gds-text"),
        ..Default::default()
    };

    eframe::run_native(
        "gds-text",
        options,
        Box::new(|cc| Ok(Box::new(app::GdsTextApp::new(cc)))),
    )
}
