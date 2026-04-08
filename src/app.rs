//! egui application — main window, canvas, properties panel.

use eframe::egui;
use eframe::egui::epaint::text::{FontInsert, FontPriority, InsertFontFamily};

use crate::config::{MIN_GRID_NM, ProjectConfig, TextSnippet};
use crate::gds_out;
use crate::pdf_out;
use crate::text_render::TextRenderer;

pub struct GdsTextApp {
    cfg: ProjectConfig,
    renderer: TextRenderer,
    available_fonts: Vec<String>,
    selected: Option<usize>,
    // Canvas interaction state.
    dragging: Option<DragState>,
    // Status line.
    status: String,
    // Dirty flag for preview cache.
    preview_dirty: bool,
    preview_rgba: Option<(Vec<u8>, usize, usize)>,
    preview_texture: Option<egui::TextureHandle>,
}

struct DragState {
    idx: usize,
    grab_dx: f32,
    grab_dy: f32,
}

impl GdsTextApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        setup_fallback_fonts(&cc.egui_ctx);

        let renderer = TextRenderer::new();
        let available_fonts = renderer.list_fonts();

        let mut cfg = ProjectConfig::default();
        // Prime with a default snippet so the canvas isn't empty on first run.
        let id = cfg.alloc_id();
        cfg.snippets
            .push(TextSnippet::new(id, "GDS TEXT 中文", 20.0, 40.0));

        let status = if renderer.find_font(&cfg.font_name).is_some() {
            format!("font '{}' ok", cfg.font_name)
        } else {
            format!(
                "warning: font '{}' not found, using fallback",
                cfg.font_name
            )
        };

        Self {
            cfg,
            renderer,
            available_fonts,
            selected: Some(0),
            dragging: None,
            status,
            preview_dirty: true,
            preview_rgba: None,
            preview_texture: None,
        }
    }

    fn rebuild_preview(&mut self, ctx: &egui::Context) {
        let w = self.cfg.canvas_width_px as usize;
        let h = self.cfg.canvas_height_px as usize;
        if w == 0 || h == 0 {
            return;
        }

        let text_cells = gds_out::collect_text_cells(&self.cfg, &mut self.renderer);
        let fill_cells = crate::fill::compute_fill_cells(&self.cfg, &text_cells);

        let mut rgba = vec![255u8; w * h * 4];
        // Canvas background (white) already filled.

        // Draw fill (gray).
        for (gx, gy) in fill_cells {
            plot(&mut rgba, w, h, gx, gy, [180, 180, 180, 255]);
        }
        // Draw text (black).
        for (gx, gy) in text_cells {
            plot(&mut rgba, w, h, gx, gy, [0, 0, 0, 255]);
        }

        // Draw selection bounding boxes.
        for (i, snippet) in self.cfg.snippets.iter().enumerate() {
            let bb = snippet_bbox(&mut self.renderer, &self.cfg.font_name, snippet);
            let color = if Some(i) == self.selected {
                [0, 120, 255, 255]
            } else {
                [120, 180, 230, 255]
            };
            draw_rect(&mut rgba, w, h, bb.0, bb.1, bb.2, bb.3, color);
        }

        let color = egui::ColorImage::from_rgba_unmultiplied([w, h], &rgba);
        let handle = ctx.load_texture("canvas", color, egui::TextureOptions::NEAREST);
        self.preview_texture = Some(handle);
        self.preview_rgba = Some((rgba, w, h));
        self.preview_dirty = false;
    }

    fn mark_dirty(&mut self) {
        self.preview_dirty = true;
    }

    fn add_snippet(&mut self) {
        let id = self.cfg.alloc_id();
        let s = TextSnippet::new(id, "text", 10.0, 20.0);
        self.cfg.snippets.push(s);
        self.selected = Some(self.cfg.snippets.len() - 1);
        self.mark_dirty();
    }

    fn delete_selected(&mut self) {
        if let Some(idx) = self.selected {
            if idx < self.cfg.snippets.len() {
                self.cfg.snippets.remove(idx);
                if self.cfg.snippets.is_empty() {
                    self.selected = None;
                } else if idx >= self.cfg.snippets.len() {
                    self.selected = Some(self.cfg.snippets.len() - 1);
                }
                self.mark_dirty();
            }
        }
    }

    fn export_gds(&mut self) {
        let path = rfd::FileDialog::new()
            .add_filter("GDSII", &["gds"])
            .set_file_name("gds-text.gds")
            .save_file();
        if let Some(p) = path {
            match gds_out::write_gds(&self.cfg, &mut self.renderer, &p) {
                Ok(()) => self.status = format!("GDS written: {}", p.display()),
                Err(e) => self.status = format!("GDS export failed: {e}"),
            }
        }
    }

    fn export_pdf(&mut self) {
        let path = rfd::FileDialog::new()
            .add_filter("PDF", &["pdf"])
            .set_file_name("gds-text.pdf")
            .save_file();
        if let Some(p) = path {
            match pdf_out::write_pdf(&self.cfg, &mut self.renderer, &p) {
                Ok(()) => self.status = format!("PDF written: {}", p.display()),
                Err(e) => self.status = format!("PDF export failed: {e}"),
            }
        }
    }
}

impl eframe::App for GdsTextApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Top toolbar.
        egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("gds-text");
                ui.separator();
                if ui.button("+ Text").clicked() {
                    self.add_snippet();
                }
                if ui.button("Delete").clicked() {
                    self.delete_selected();
                }
                ui.separator();
                if ui.button("Export GDS").clicked() {
                    self.export_gds();
                }
                if ui.button("Export PDF").clicked() {
                    self.export_pdf();
                }
                ui.separator();
                ui.label(format!("snippets: {}", self.cfg.snippets.len()));
            });
        });

        // Status bar.
        egui::TopBottomPanel::bottom("status").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(&self.status);
            });
        });

        // Right properties panel.
        egui::SidePanel::right("properties")
            .default_width(280.0)
            .resizable(true)
            .show(ctx, |ui| {
                ui.heading("Project");
                ui.separator();

                let mut changed = false;

                ui.horizontal(|ui| {
                    ui.label("Grid (nm):");
                    let mut g = self.cfg.grid_nm;
                    if ui
                        .add(
                            egui::DragValue::new(&mut g)
                                .range(MIN_GRID_NM..=10_000)
                                .speed(10.0),
                        )
                        .changed()
                    {
                        self.cfg.grid_nm = g.max(MIN_GRID_NM);
                        changed = true;
                    }
                });

                ui.horizontal(|ui| {
                    ui.label("Fill density:");
                    if ui
                        .add(egui::Slider::new(&mut self.cfg.fill_density, 0.0..=0.8))
                        .changed()
                    {
                        changed = true;
                    }
                });

                ui.horizontal(|ui| {
                    ui.label("Canvas W:");
                    if ui
                        .add(egui::DragValue::new(&mut self.cfg.canvas_width_px).range(10..=5_000))
                        .changed()
                    {
                        changed = true;
                    }
                });
                ui.horizontal(|ui| {
                    ui.label("Canvas H:");
                    if ui
                        .add(egui::DragValue::new(&mut self.cfg.canvas_height_px).range(10..=5_000))
                        .changed()
                    {
                        changed = true;
                    }
                });

                ui.separator();
                ui.label("Font:");
                egui::ComboBox::from_id_salt("font_combo")
                    .selected_text(self.cfg.font_name.clone())
                    .show_ui(ui, |ui| {
                        for f in &self.available_fonts {
                            if ui.selectable_label(f == &self.cfg.font_name, f).clicked() {
                                self.cfg.font_name = f.clone();
                                changed = true;
                            }
                        }
                    });

                ui.separator();
                ui.collapsing("Design rules", |ui| {
                    ui.horizontal(|ui| {
                        ui.label("min width (nm):");
                        if ui
                            .add(
                                egui::DragValue::new(&mut self.cfg.rules.min_width_nm)
                                    .range(10..=10_000),
                            )
                            .changed()
                        {
                            changed = true;
                        }
                    });
                    ui.horizontal(|ui| {
                        ui.label("min spacing (nm):");
                        if ui
                            .add(
                                egui::DragValue::new(&mut self.cfg.rules.min_spacing_nm)
                                    .range(10..=10_000),
                            )
                            .changed()
                        {
                            changed = true;
                        }
                    });
                    ui.horizontal(|ui| {
                        ui.label("fill↔metal (nm):");
                        if ui
                            .add(
                                egui::DragValue::new(&mut self.cfg.rules.fill_to_metal_spacing_nm)
                                    .range(10..=20_000),
                            )
                            .changed()
                        {
                            changed = true;
                        }
                    });
                });

                ui.collapsing("Layers", |ui| {
                    ui.horizontal(|ui| {
                        ui.label("text layer:");
                        ui.add(egui::DragValue::new(&mut self.cfg.layers.text_layer));
                        ui.label("/");
                        ui.add(egui::DragValue::new(&mut self.cfg.layers.text_datatype));
                    });
                    ui.horizontal(|ui| {
                        ui.label("fill layer:");
                        ui.add(egui::DragValue::new(&mut self.cfg.layers.fill_layer));
                        ui.label("/");
                        ui.add(egui::DragValue::new(&mut self.cfg.layers.fill_datatype));
                    });
                });

                ui.separator();
                ui.heading("Snippets");
                let mut remove_idx: Option<usize> = None;
                for i in 0..self.cfg.snippets.len() {
                    ui.horizontal(|ui| {
                        let label = self.cfg.snippets[i].text.clone();
                        let resp = ui.selectable_label(
                            self.selected == Some(i),
                            if label.len() > 14 {
                                format!("#{i}: {}…", &label[..14])
                            } else {
                                format!("#{i}: {}", label)
                            },
                        );
                        if resp.clicked() {
                            self.selected = Some(i);
                            changed = true;
                        }
                        if ui.small_button("x").clicked() {
                            remove_idx = Some(i);
                        }
                    });
                }
                if let Some(i) = remove_idx {
                    self.cfg.snippets.remove(i);
                    if self.cfg.snippets.is_empty() {
                        self.selected = None;
                    } else if let Some(sel) = self.selected {
                        if sel >= self.cfg.snippets.len() {
                            self.selected = Some(self.cfg.snippets.len() - 1);
                        }
                    }
                    changed = true;
                }

                ui.separator();
                ui.heading("Selected");
                if let Some(idx) = self.selected {
                    if let Some(s) = self.cfg.snippets.get_mut(idx) {
                        if ui
                            .add(egui::TextEdit::multiline(&mut s.text).desired_rows(2))
                            .changed()
                        {
                            changed = true;
                        }
                        ui.horizontal(|ui| {
                            ui.label("size (cells):");
                            if ui
                                .add(egui::Slider::new(&mut s.font_size, 4.0..=200.0))
                                .changed()
                            {
                                changed = true;
                            }
                        });
                        ui.horizontal(|ui| {
                            ui.label("rotation:");
                            if ui
                                .add(egui::Slider::new(&mut s.rotation_deg, -180.0..=180.0))
                                .changed()
                            {
                                changed = true;
                            }
                        });
                        ui.horizontal(|ui| {
                            for a in [0.0, 45.0, 90.0, 135.0, 180.0, 270.0] {
                                if ui.small_button(format!("{a:.0}°")).clicked() {
                                    s.rotation_deg = a;
                                    changed = true;
                                }
                            }
                        });
                        ui.horizontal(|ui| {
                            ui.label("x:");
                            if ui.add(egui::DragValue::new(&mut s.x)).changed() {
                                changed = true;
                            }
                            ui.label("y:");
                            if ui.add(egui::DragValue::new(&mut s.y)).changed() {
                                changed = true;
                            }
                        });
                    }
                } else {
                    ui.label("(none)");
                }

                if changed {
                    self.mark_dirty();
                }
            });

        // Central canvas.
        egui::CentralPanel::default().show(ctx, |ui| {
            if self.preview_dirty || self.preview_texture.is_none() {
                self.rebuild_preview(ctx);
            }

            let canvas_size = egui::vec2(
                self.cfg.canvas_width_px as f32,
                self.cfg.canvas_height_px as f32,
            );
            let avail = ui.available_size();
            let scale = (avail.x / canvas_size.x)
                .min(avail.y / canvas_size.y)
                .clamp(0.1, 3.0);
            let display = canvas_size * scale;

            let (rect, response) = ui.allocate_exact_size(display, egui::Sense::click_and_drag());

            // Background.
            ui.painter()
                .rect_filled(rect, 0.0, egui::Color32::from_gray(230));
            if let Some(tex) = &self.preview_texture {
                ui.painter().image(
                    tex.id(),
                    rect,
                    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    egui::Color32::WHITE,
                );
            }

            // Hit testing + drag.
            let pointer = response.interact_pointer_pos();
            if response.drag_started() {
                if let Some(p) = pointer {
                    let local_x = (p.x - rect.min.x) / scale;
                    let local_y = (p.y - rect.min.y) / scale;
                    if let Some(hit) = hit_test(&mut self.renderer, &self.cfg, local_x, local_y) {
                        self.selected = Some(hit);
                        let s = &self.cfg.snippets[hit];
                        self.dragging = Some(DragState {
                            idx: hit,
                            grab_dx: local_x - s.x,
                            grab_dy: local_y - s.y,
                        });
                    }
                }
            }
            if response.dragged() {
                if let (Some(p), Some(drag)) = (pointer, self.dragging.as_ref()) {
                    let local_x = (p.x - rect.min.x) / scale;
                    let local_y = (p.y - rect.min.y) / scale;
                    if let Some(s) = self.cfg.snippets.get_mut(drag.idx) {
                        s.x = (local_x - drag.grab_dx).round();
                        s.y = (local_y - drag.grab_dy).round();
                        self.preview_dirty = true;
                    }
                }
            }
            if response.drag_stopped() {
                self.dragging = None;
            }

            if response.clicked() && self.dragging.is_none() {
                if let Some(p) = pointer {
                    let local_x = (p.x - rect.min.x) / scale;
                    let local_y = (p.y - rect.min.y) / scale;
                    self.selected = hit_test(&mut self.renderer, &self.cfg, local_x, local_y);
                    self.preview_dirty = true;
                }
            }
        });
    }
}

fn hit_test(renderer: &mut TextRenderer, cfg: &ProjectConfig, x: f32, y: f32) -> Option<usize> {
    for (i, snippet) in cfg.snippets.iter().enumerate().rev() {
        let (x0, y0, x1, y1) = snippet_bbox(renderer, &cfg.font_name, snippet);
        if (x as i32) >= x0 && (x as i32) < x1 && (y as i32) >= y0 && (y as i32) < y1 {
            return Some(i);
        }
    }
    None
}

fn snippet_bbox(
    renderer: &mut TextRenderer,
    font_name: &str,
    snippet: &TextSnippet,
) -> (i32, i32, i32, i32) {
    let Ok(bmp) = renderer.rasterize(snippet, font_name) else {
        return (0, 0, 0, 0);
    };
    let rotated = bmp.rotate(snippet.rotation_deg);
    let ox = snippet.x.round() as i32;
    let oy = snippet.y.round() as i32;
    (
        ox,
        oy,
        ox + rotated.width() as i32,
        oy + rotated.height() as i32,
    )
}

fn plot(rgba: &mut [u8], w: usize, h: usize, x: i32, y: i32, color: [u8; 4]) {
    if x < 0 || y < 0 || (x as usize) >= w || (y as usize) >= h {
        return;
    }
    let idx = ((y as usize) * w + (x as usize)) * 4;
    rgba[idx..idx + 4].copy_from_slice(&color);
}

#[allow(clippy::too_many_arguments)]
fn draw_rect(
    rgba: &mut [u8],
    w: usize,
    h: usize,
    x0: i32,
    y0: i32,
    x1: i32,
    y1: i32,
    color: [u8; 4],
) {
    for x in x0..x1 {
        plot(rgba, w, h, x, y0, color);
        plot(rgba, w, h, x, y1 - 1, color);
    }
    for y in y0..y1 {
        plot(rgba, w, h, x0, y, color);
        plot(rgba, w, h, x1 - 1, y, color);
    }
}

fn setup_fallback_fonts(ctx: &egui::Context) {
    let both = vec![
        InsertFontFamily {
            family: egui::FontFamily::Proportional,
            priority: FontPriority::Lowest,
        },
        InsertFontFamily {
            family: egui::FontFamily::Monospace,
            priority: FontPriority::Lowest,
        },
    ];
    ctx.add_font(FontInsert {
        name: "cjk_fallback".into(),
        data: egui::FontData::from_static(include_bytes!("../assets/fonts/DroidSansFallback.ttf")),
        families: both,
    });
}
