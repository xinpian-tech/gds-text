//! Calibre-style dummy fill respecting sky130-inspired design rules.

use std::collections::HashSet;

use crate::config::ProjectConfig;

/// Compute dummy fill cells.
///
/// Algorithm:
/// 1. Exclusion set = grow text cells by `fill_to_metal_spacing` cells.
/// 2. Walk a regular grid over the canvas with step = spacing + cell.
/// 3. Deterministic pseudo-shuffle to spread fills, then truncate to the
///    target count implied by `fill_density`.
pub fn compute_fill_cells(cfg: &ProjectConfig, used: &[(i32, i32)]) -> Vec<(i32, i32)> {
    if cfg.fill_density <= 0.0 {
        return Vec::new();
    }

    let grid = cfg.grid_nm as i32;
    let excl_cells = (cfg.rules.fill_to_metal_spacing_nm as i32).div_ceil(grid);
    let spacing_cells = (cfg.rules.min_spacing_nm as i32).div_ceil(grid).max(1);
    let step = spacing_cells + 1;

    let mut excluded: HashSet<(i32, i32)> = HashSet::new();
    for &(ux, uy) in used {
        for dy in -excl_cells..=excl_cells {
            for dx in -excl_cells..=excl_cells {
                excluded.insert((ux + dx, uy + dy));
            }
        }
    }

    let w = cfg.canvas_width_px as i32;
    let h = cfg.canvas_height_px as i32;
    let mut candidates: Vec<(i32, i32)> = Vec::new();
    let mut y = 0;
    while y < h {
        let mut x = 0;
        while x < w {
            if !excluded.contains(&(x, y)) {
                candidates.push((x, y));
            }
            x += step;
        }
        y += step;
    }

    // Density cap achievable on this stepped grid.
    let max_density = 1.0 / ((step * step) as f32).max(1.0);
    let effective_density = cfg.fill_density.min(max_density);
    let target = ((w * h) as f32 * effective_density).round() as usize;

    // Deterministic Fisher-Yates (fixed seed).
    let mut shuffled = candidates;
    let mut state: u64 = 0x9E37_79B9_7F4A_7C15;
    for i in (1..shuffled.len()).rev() {
        state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        let j = (state >> 33) as usize % (i + 1);
        shuffled.swap(i, j);
    }
    shuffled.truncate(target);
    shuffled.sort();
    shuffled
}
