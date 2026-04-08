//! gds-text -- render text snippets to GDSII + PDF with Calibre-style dummy fill.

mod app;
mod bitmap;
mod config;
mod fill;
mod gds_out;
mod pdf_out;
mod png_out;
mod text_render;

use eframe::egui;
use std::path::PathBuf;
use std::process::ExitCode;

use crate::config::{ProjectConfig, TextSnippet};
use crate::text_render::TextRenderer;

fn main() -> ExitCode {
    env_logger::init();

    let args: Vec<String> = std::env::args().collect();
    if args.len() >= 2 && args[1] == "--help" {
        print_help();
        return ExitCode::SUCCESS;
    }
    if args.len() >= 2 && args[1] == "export" {
        return match run_export_cli(&args[2..]) {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("export failed: {e}");
                ExitCode::FAILURE
            }
        };
    }
    if args.len() >= 2 && args[1] == "render-gds" {
        return match run_render_gds_cli(&args[2..]) {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("render-gds failed: {e}");
                ExitCode::FAILURE
            }
        };
    }

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 800.0])
            .with_min_inner_size([900.0, 600.0])
            .with_title("gds-text"),
        ..Default::default()
    };

    match eframe::run_native(
        "gds-text",
        options,
        Box::new(|cc| Ok(Box::new(app::GdsTextApp::new(cc)))),
    ) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("gui error: {e}");
            ExitCode::FAILURE
        }
    }
}

fn print_help() {
    println!("gds-text -- render text snippets to GDSII + PDF");
    println!();
    println!("USAGE:");
    println!("  gds-text                         launch GUI");
    println!("  gds-text export [OPTIONS]        export a preset layout without GUI");
    println!("  gds-text render-gds <in> <out>   render existing GDSII to a PNG");
    println!();
    println!("EXPORT OPTIONS:");
    println!("  --gds <path>        write GDSII to <path>");
    println!("  --pdf <path>        write PDF preview to <path>");
    println!("  --png <path>        write PNG preview to <path>");
    println!("  --png-scale <u32>   PNG pixels per grid cell (default: 4)");
    println!("  --text <string>     text to render (default: 'GDS TEXT 中文')");
    println!("  --font <name>       font family (default: 'Sarasa Mono SC')");
    println!("  --font-size <f32>   font size in grid cells (default: 18)");
    println!("  --rotation <f32>    rotation in degrees (default: 0)");
    println!("  --grid-nm <u32>     grid precision in nm, >= 100 (default: 150)");
    println!("  --fill <f32>        fill density 0..0.8 (default: 0.35)");
    println!("  --canvas-w <u32>    canvas width in cells (default: 800)");
    println!("  --canvas-h <u32>    canvas height in cells (default: 500)");
}

fn run_export_cli(args: &[String]) -> anyhow::Result<()> {
    use anyhow::{Context, bail};

    let mut cfg = ProjectConfig::default();
    let mut gds_path: Option<PathBuf> = None;
    let mut pdf_path: Option<PathBuf> = None;
    let mut png_path: Option<PathBuf> = None;
    let mut png_scale: u32 = 4;
    let mut text = "GDS TEXT 中文".to_string();
    let mut font_size: f32 = 18.0;
    let mut rotation: f32 = 0.0;
    let mut x: f32 = 20.0;
    let mut y: f32 = 40.0;

    let mut i = 0;
    while i < args.len() {
        let a = args[i].as_str();
        let next = |i: usize| -> anyhow::Result<&String> {
            args.get(i + 1)
                .with_context(|| format!("missing value for {a}", a = args[i]))
        };
        match a {
            "--gds" => {
                gds_path = Some(PathBuf::from(next(i)?));
                i += 2;
            }
            "--pdf" => {
                pdf_path = Some(PathBuf::from(next(i)?));
                i += 2;
            }
            "--png" => {
                png_path = Some(PathBuf::from(next(i)?));
                i += 2;
            }
            "--png-scale" => {
                png_scale = next(i)?.parse()?;
                i += 2;
            }
            "--text" => {
                text = next(i)?.clone();
                i += 2;
            }
            "--font" => {
                cfg.font_name = next(i)?.clone();
                i += 2;
            }
            "--font-size" => {
                font_size = next(i)?.parse()?;
                i += 2;
            }
            "--rotation" => {
                rotation = next(i)?.parse()?;
                i += 2;
            }
            "--grid-nm" => {
                cfg.grid_nm = next(i)?.parse()?;
                i += 2;
            }
            "--fill" => {
                cfg.fill_density = next(i)?.parse()?;
                i += 2;
            }
            "--canvas-w" => {
                cfg.canvas_width_px = next(i)?.parse()?;
                i += 2;
            }
            "--canvas-h" => {
                cfg.canvas_height_px = next(i)?.parse()?;
                i += 2;
            }
            "--x" => {
                x = next(i)?.parse()?;
                i += 2;
            }
            "--y" => {
                y = next(i)?.parse()?;
                i += 2;
            }
            other => bail!("unknown option: {other}"),
        }
    }

    if gds_path.is_none() && pdf_path.is_none() && png_path.is_none() {
        bail!("must specify at least one of --gds, --pdf, --png");
    }

    let id = cfg.alloc_id();
    let mut snippet = TextSnippet::new(id, text, x, y);
    snippet.font_size = font_size;
    snippet.rotation_deg = rotation;
    cfg.snippets.push(snippet);

    let mut renderer = TextRenderer::new();
    if let Some(p) = &gds_path {
        gds_out::write_gds(&cfg, &mut renderer, p)?;
        println!("wrote {}", p.display());
    }
    if let Some(p) = &pdf_path {
        pdf_out::write_pdf(&cfg, &mut renderer, p)?;
        println!("wrote {}", p.display());
    }
    if let Some(p) = &png_path {
        png_out::write_png(&cfg, &mut renderer, p, png_scale)?;
        println!("wrote {}", p.display());
    }
    Ok(())
}

fn run_render_gds_cli(args: &[String]) -> anyhow::Result<()> {
    use anyhow::bail;
    if args.len() < 2 {
        bail!("usage: gds-text render-gds <in.gds> <out.png> [--scale N] [--pad-nm N]");
    }
    let in_path = PathBuf::from(&args[0]);
    let out_path = PathBuf::from(&args[1]);
    let mut scale: u32 = 4;
    let mut padding_nm: i32 = 500;
    let layers = config::LayerConfig::default();

    let mut i = 2;
    while i < args.len() {
        match args[i].as_str() {
            "--scale" => {
                scale = args
                    .get(i + 1)
                    .ok_or_else(|| anyhow::anyhow!("missing value for --scale"))?
                    .parse()?;
                i += 2;
            }
            "--pad-nm" => {
                padding_nm = args
                    .get(i + 1)
                    .ok_or_else(|| anyhow::anyhow!("missing value for --pad-nm"))?
                    .parse()?;
                i += 2;
            }
            other => bail!("unknown option: {other}"),
        }
    }

    png_out::render_gds(&in_path, &out_path, layers, scale, padding_nm)?;
    println!("wrote {}", out_path.display());
    Ok(())
}
