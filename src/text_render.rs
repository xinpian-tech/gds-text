//! Text rasterization using cosmic-text, following the ptouch-rs approach.

use anyhow::{Context, Result};
use cosmic_text::{Attrs, Buffer, Color, Family, FontSystem, Metrics, Shaping, SwashCache};

use crate::bitmap::Bitmap;
use crate::config::TextSnippet;

/// Shared cosmic-text renderer.
pub struct TextRenderer {
    font_system: FontSystem,
    swash_cache: SwashCache,
}

impl Default for TextRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl TextRenderer {
    pub fn new() -> Self {
        Self {
            font_system: FontSystem::new(),
            swash_cache: SwashCache::new(),
        }
    }

    /// List all available font family names.
    pub fn list_fonts(&self) -> Vec<String> {
        let mut names: Vec<String> = Vec::new();
        for face in self.font_system.db().faces() {
            for family in &face.families {
                names.push(family.0.clone());
            }
        }
        names.sort();
        names.dedup();
        names
    }

    /// Try to find a font by fuzzy name match.
    pub fn find_font(&self, name: &str) -> Option<String> {
        let needle = name.to_lowercase();
        for face in self.font_system.db().faces() {
            for family in &face.families {
                if family.0.to_lowercase().contains(&needle) {
                    return Some(family.0.clone());
                }
            }
        }
        None
    }

    /// Rasterize a snippet to a bitmap of grid cells. The font size is
    /// interpreted as grid cells (each cell = one pixel square in the bitmap).
    pub fn rasterize(&mut self, snippet: &TextSnippet, font_name: &str) -> Result<Bitmap> {
        if snippet.text.is_empty() {
            return Ok(Bitmap::new(1, 1));
        }

        let font_size = snippet.font_size.max(4.0);
        let line_height = (font_size * 1.2).ceil();
        let metrics = Metrics::new(font_size, line_height);

        // Prefer the requested family, falling back through a chain so CJK
        // always resolves to *something*.
        let family = if font_name.is_empty() {
            Family::Monospace
        } else {
            Family::Name(font_name)
        };
        let attrs = Attrs::new().family(family);

        let mut buffer = Buffer::new(&mut self.font_system, metrics);
        // Provide a generous layout area; we'll measure exactly after shaping.
        let layout_width = 16_384.0_f32;
        let layout_height = line_height * (snippet.text.lines().count().max(1) as f32) + 16.0;
        buffer.set_size(
            &mut self.font_system,
            Some(layout_width),
            Some(layout_height),
        );
        buffer.set_text(&mut self.font_system, &snippet.text, &attrs, Shaping::Advanced);
        buffer.shape_until_scroll(&mut self.font_system, true);

        // Measure tight extent of laid-out glyphs.
        let mut min_x: f32 = f32::MAX;
        let mut max_x: f32 = 0.0;
        let mut min_y: f32 = f32::MAX;
        let mut max_y: f32 = 0.0;
        let mut any = false;
        for run in buffer.layout_runs() {
            for g in run.glyphs.iter() {
                any = true;
                min_x = min_x.min(g.x);
                max_x = max_x.max(g.x + g.w);
                min_y = min_y.min(run.line_y - font_size);
                max_y = max_y.max(run.line_y + font_size * 0.3);
            }
        }
        if !any {
            return Ok(Bitmap::new(1, 1));
        }
        if min_x == f32::MAX {
            min_x = 0.0;
        }
        if min_y == f32::MAX {
            min_y = 0.0;
        }

        let w = ((max_x - min_x).ceil() as u32).max(1);
        let h = ((max_y - min_y).ceil() as u32 + (line_height as u32)).max(1);
        let x_offset = min_x.floor() as i32;
        let y_offset = min_y.floor() as i32;

        let mut bitmap = Bitmap::new(w + 2, h + 2);
        let color = Color::rgb(0, 0, 0);
        buffer.draw(
            &mut self.font_system,
            &mut self.swash_cache,
            color,
            |x, y, gw, gh, col| {
                if col.a() < 128 {
                    return;
                }
                let px = x - x_offset;
                let py = y - y_offset;
                for dy in 0..gh as i32 {
                    for dx in 0..gw as i32 {
                        let fx = px + dx;
                        let fy = py + dy;
                        if fx >= 0 && fy >= 0 {
                            bitmap.set(fx as u32, fy as u32, true);
                        }
                    }
                }
            },
        );

        Ok(trim(&bitmap))
    }
}

/// Trim empty rows/columns from all sides.
fn trim(b: &Bitmap) -> Bitmap {
    let (w, h) = (b.width(), b.height());
    let mut min_x = w;
    let mut min_y = h;
    let mut max_x = 0u32;
    let mut max_y = 0u32;
    let mut any = false;
    for y in 0..h {
        for x in 0..w {
            if b.get(x, y) {
                any = true;
                min_x = min_x.min(x);
                min_y = min_y.min(y);
                max_x = max_x.max(x);
                max_y = max_y.max(y);
            }
        }
    }
    if !any {
        return Bitmap::new(1, 1);
    }
    let nw = max_x - min_x + 1;
    let nh = max_y - min_y + 1;
    let mut out = Bitmap::new(nw, nh);
    for y in 0..nh {
        for x in 0..nw {
            if b.get(x + min_x, y + min_y) {
                out.set(x, y, true);
            }
        }
    }
    out
}

/// Error helper used by app to report load failures.
pub fn ensure_font_available(renderer: &TextRenderer, name: &str) -> Result<String> {
    renderer
        .find_font(name)
        .with_context(|| format!("font '{}' not installed", name))
}
