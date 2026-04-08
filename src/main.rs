//! gds-text -- render text snippets to GDSII + PDF with Calibre-style dummy fill.

mod app;
mod bitmap;
mod config;
mod fill;
mod gds_out;
mod layout;
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
    if args.len() >= 2 && args[1] == "export-layout" {
        return match run_export_layout_cli(&args[2..]) {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("export-layout failed: {e}");
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
    println!("  gds-text                          launch GUI");
    println!("  gds-text export [OPTIONS]         export a flat layout (one canvas)");
    println!("  gds-text export-layout [OPTIONS]  export a hierarchical layout");
    println!("                                    (one cell per user, top cell refs them)");
    println!();
    println!("EXPORT OPTIONS:");
    println!("  --gds <path>        write GDSII to <path>");
    println!("  --config <path>     load full ProjectConfig JSON (overrides other opts)");
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

    let mut cfg_path: Option<PathBuf> = None;
    let mut gds_path: Option<PathBuf> = None;
    let mut cfg = ProjectConfig::default();
    let mut text = "GDS TEXT 中文".to_string();
    let mut font_size: f32 = 18.0;
    let mut rotation: f32 = 0.0;
    let mut x: f32 = 20.0;
    let mut y: f32 = 40.0;
    let mut single_text_overridden = false;

    let mut i = 0;
    while i < args.len() {
        let a = args[i].as_str();
        let next = |i: usize| -> anyhow::Result<&String> {
            args.get(i + 1)
                .with_context(|| format!("missing value for {a}", a = args[i]))
        };
        match a {
            "--config" => {
                cfg_path = Some(PathBuf::from(next(i)?));
                i += 2;
            }
            "--gds" => {
                gds_path = Some(PathBuf::from(next(i)?));
                i += 2;
            }
            "--text" => {
                text = next(i)?.clone();
                single_text_overridden = true;
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

    let Some(gds_path) = gds_path else {
        bail!("must specify --gds <path>");
    };

    // If a config file is given, it supplies the entire ProjectConfig and any
    // command line overrides for single-snippet fields are ignored.
    if let Some(p) = cfg_path {
        let json =
            std::fs::read_to_string(&p).with_context(|| format!("reading {}", p.display()))?;
        cfg = serde_json::from_str::<ProjectConfig>(&json)
            .with_context(|| format!("parsing {} as ProjectConfig JSON", p.display()))?;
    } else {
        let id = cfg.alloc_id();
        let mut snippet = TextSnippet::new(
            id,
            if single_text_overridden {
                text
            } else {
                "GDS TEXT 中文".to_string()
            },
            x,
            y,
        );
        snippet.font_size = font_size;
        snippet.rotation_deg = rotation;
        cfg.snippets.push(snippet);
    }

    println!(
        "loaded config: {} snippets on {}x{} cells ({}nm grid)",
        cfg.snippets.len(),
        cfg.canvas_width_px,
        cfg.canvas_height_px,
        cfg.grid_nm
    );

    let mut renderer = TextRenderer::new();
    gds_out::write_gds(&cfg, &mut renderer, &gds_path)?;
    println!("wrote {}", gds_path.display());
    Ok(())
}

fn run_export_layout_cli(args: &[String]) -> anyhow::Result<()> {
    use anyhow::{Context, bail};

    let mut cfg_path: Option<PathBuf> = None;
    let mut gds_path: Option<PathBuf> = None;

    let mut i = 0;
    while i < args.len() {
        let a = args[i].as_str();
        let next = |i: usize| -> anyhow::Result<&String> {
            args.get(i + 1)
                .with_context(|| format!("missing value for {a}", a = args[i]))
        };
        match a {
            "--config" => {
                cfg_path = Some(PathBuf::from(next(i)?));
                i += 2;
            }
            "--gds" => {
                gds_path = Some(PathBuf::from(next(i)?));
                i += 2;
            }
            other => bail!("unknown option: {other}"),
        }
    }

    let cfg_path = cfg_path.ok_or_else(|| anyhow::anyhow!("must specify --config <path>"))?;
    let gds_path = gds_path.ok_or_else(|| anyhow::anyhow!("must specify --gds <path>"))?;

    let json = std::fs::read_to_string(&cfg_path)
        .with_context(|| format!("reading {}", cfg_path.display()))?;
    let cfg = serde_json::from_str::<layout::LayoutConfig>(&json)
        .with_context(|| format!("parsing {} as LayoutConfig JSON", cfg_path.display()))?;

    println!(
        "loaded layout: {} cells on {}x{} top canvas ({}nm grid)",
        cfg.entries.len(),
        cfg.canvas_width_px,
        cfg.canvas_height_px,
        cfg.grid_nm
    );

    let mut renderer = crate::text_render::TextRenderer::new();
    layout::write_layout_gds(&cfg, &mut renderer, &gds_path)?;
    println!("wrote {}", gds_path.display());
    Ok(())
}
