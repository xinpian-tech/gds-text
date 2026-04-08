//! 1-bit bitmap with rotation support, following ptouch-rs patterns.

#[derive(Debug, Clone)]
pub struct Bitmap {
    width: u32,
    height: u32,
    /// Row-major, one byte per pixel (0 or 1) for simplicity.
    data: Vec<u8>,
}

impl Bitmap {
    pub fn new(width: u32, height: u32) -> Self {
        let len = (width as usize) * (height as usize);
        Self {
            width,
            height,
            data: vec![0u8; len],
        }
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn set(&mut self, x: u32, y: u32, on: bool) {
        if x < self.width && y < self.height {
            self.data[(y * self.width + x) as usize] = on as u8;
        }
    }

    pub fn get(&self, x: u32, y: u32) -> bool {
        if x < self.width && y < self.height {
            self.data[(y * self.width + x) as usize] != 0
        } else {
            false
        }
    }

    /// Rotate around the center. For 0/90/180/270 exact rotations, uses
    /// lossless pixel remapping; otherwise nearest-neighbor on the rotated
    /// bounding box.
    pub fn rotate(&self, angle_deg: f32) -> Bitmap {
        let norm = ((angle_deg % 360.0) + 360.0) % 360.0;
        let eps = 0.5;
        if norm.abs() < eps || (norm - 360.0).abs() < eps {
            return self.clone();
        }
        if (norm - 90.0).abs() < eps {
            return self.rotate_cw_exact(1);
        }
        if (norm - 180.0).abs() < eps {
            return self.rotate_cw_exact(2);
        }
        if (norm - 270.0).abs() < eps {
            return self.rotate_cw_exact(3);
        }
        self.rotate_arbitrary(norm)
    }

    fn rotate_cw_exact(&self, steps: u8) -> Bitmap {
        match steps % 4 {
            0 => self.clone(),
            1 => {
                // 90 CW: (x, y) -> (H - 1 - y, x)
                let mut r = Bitmap::new(self.height, self.width);
                for y in 0..self.height {
                    for x in 0..self.width {
                        if self.get(x, y) {
                            r.set(self.height - 1 - y, x, true);
                        }
                    }
                }
                r
            }
            2 => {
                let mut r = Bitmap::new(self.width, self.height);
                for y in 0..self.height {
                    for x in 0..self.width {
                        if self.get(x, y) {
                            r.set(self.width - 1 - x, self.height - 1 - y, true);
                        }
                    }
                }
                r
            }
            3 => {
                // 270 CW: (x, y) -> (y, W - 1 - x)
                let mut r = Bitmap::new(self.height, self.width);
                for y in 0..self.height {
                    for x in 0..self.width {
                        if self.get(x, y) {
                            r.set(y, self.width - 1 - x, true);
                        }
                    }
                }
                r
            }
            _ => unreachable!(),
        }
    }

    fn rotate_arbitrary(&self, angle_deg: f32) -> Bitmap {
        let theta = angle_deg.to_radians();
        let (s, c) = (theta.sin(), theta.cos());
        let sw = self.width as f32;
        let sh = self.height as f32;
        let new_w = (sw * c.abs() + sh * s.abs()).ceil() as u32;
        let new_h = (sw * s.abs() + sh * c.abs()).ceil() as u32;
        let new_w = new_w.max(1);
        let new_h = new_h.max(1);

        let cx_src = (sw - 1.0) / 2.0;
        let cy_src = (sh - 1.0) / 2.0;
        let cx_dst = (new_w as f32 - 1.0) / 2.0;
        let cy_dst = (new_h as f32 - 1.0) / 2.0;

        let mut out = Bitmap::new(new_w, new_h);
        for ny in 0..new_h {
            for nx in 0..new_w {
                let rel_x = nx as f32 - cx_dst;
                let rel_y = ny as f32 - cy_dst;
                // Inverse rotation (screen-space CW rotation).
                let src_x = rel_x * c + rel_y * s + cx_src;
                let src_y = -rel_x * s + rel_y * c + cy_src;
                let sx = src_x.round() as i32;
                let sy = src_y.round() as i32;
                if sx >= 0
                    && sy >= 0
                    && (sx as u32) < self.width
                    && (sy as u32) < self.height
                    && self.get(sx as u32, sy as u32)
                {
                    out.set(nx, ny, true);
                }
            }
        }
        out
    }

    /// Iterate over all "on" pixels as (x, y) positions.
    #[allow(dead_code)]
    pub fn iter_on(&self) -> impl Iterator<Item = (u32, u32)> + '_ {
        (0..self.height).flat_map(move |y| {
            (0..self.width).filter_map(move |x| if self.get(x, y) { Some((x, y)) } else { None })
        })
    }

    /// Decompose the "on" cells into axis-aligned rectangles using a greedy
    /// row-run + column-extension algorithm.
    ///
    /// Each `Rect` covers a contiguous block of on-cells; together they form
    /// an exact tiling (no overlaps, no gaps). This is the cheap way to
    /// collapse per-pixel boundaries into larger polygons for GDSII output.
    pub fn to_rectangles(&self) -> Vec<Rect> {
        let w = self.width as usize;
        let h = self.height as usize;
        let mut covered = vec![false; w * h];
        let mut rects: Vec<Rect> = Vec::new();

        for y in 0..h {
            let mut x = 0;
            while x < w {
                let idx = y * w + x;
                if self.data[idx] == 0 || covered[idx] {
                    // Skip off-cells or already covered.
                    x += 1;
                    continue;
                }
                // Find maximum horizontal run starting at (x, y).
                let mut run_w = 1;
                while x + run_w < w {
                    let i = y * w + x + run_w;
                    if self.data[i] == 0 || covered[i] {
                        break;
                    }
                    run_w += 1;
                }
                // Extend vertically: each additional row must have all cells
                // in [x..x+run_w) on and not already covered.
                let mut run_h = 1;
                'outer: while y + run_h < h {
                    for xx in x..x + run_w {
                        let i = (y + run_h) * w + xx;
                        if self.data[i] == 0 || covered[i] {
                            break 'outer;
                        }
                    }
                    run_h += 1;
                }
                // Mark covered.
                for yy in y..y + run_h {
                    for xx in x..x + run_w {
                        covered[yy * w + xx] = true;
                    }
                }
                rects.push(Rect {
                    x: x as u32,
                    y: y as u32,
                    w: run_w as u32,
                    h: run_h as u32,
                });
                x += run_w;
            }
        }
        rects
    }
}

/// Axis-aligned rectangle of on-cells produced by [`Bitmap::to_rectangles`].
#[derive(Debug, Clone, Copy)]
pub struct Rect {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
}

/// One merged region of the on-cell set, produced by
/// [`Bitmap::to_merged_regions`]. Either a single polygon (for simple
/// connected components with no enclosed holes) or a list of rectangles
/// (for components with holes, where polygon + cut-and-slit would be
/// complex to encode correctly in GDSII).
#[derive(Debug, Clone)]
pub enum MergedRegion {
    /// A closed polygon in integer corner coordinates (CW). First and
    /// last vertex are the same for an explicit close.
    Polygon(Vec<(i32, i32)>),
    /// A union of axis-aligned rectangles. Used as a fallback when the
    /// connected component has interior holes.
    Rectangles(Vec<Rect>),
}

impl Bitmap {
    /// Merge the on-cells into the tightest boolean union representable
    /// without cut-and-slit: hole-less connected components become a
    /// single polygon, components with holes fall back to a rectangle
    /// decomposition of just that component's cells.
    pub fn to_merged_regions(&self) -> Vec<MergedRegion> {
        let components = self.connected_components();
        let mut out = Vec::with_capacity(components.len());
        for comp in &components {
            let loops = self.trace_component_boundary(comp);
            if loops.len() == 1 {
                out.push(MergedRegion::Polygon(loops.into_iter().next().unwrap()));
            } else {
                let rects = self.rectangles_for_cells(comp);
                out.push(MergedRegion::Rectangles(rects));
            }
        }
        out
    }

    fn connected_components(&self) -> Vec<Vec<(u32, u32)>> {
        let w = self.width as usize;
        let h = self.height as usize;
        let mut visited = vec![false; w * h];
        let mut out: Vec<Vec<(u32, u32)>> = Vec::new();
        for y in 0..h {
            for x in 0..w {
                let i = y * w + x;
                if visited[i] || self.data[i] == 0 {
                    continue;
                }
                let mut comp: Vec<(u32, u32)> = Vec::new();
                let mut stack: Vec<(usize, usize)> = vec![(x, y)];
                visited[i] = true;
                while let Some((cx, cy)) = stack.pop() {
                    comp.push((cx as u32, cy as u32));
                    let nbrs = [
                        (cx as i32 + 1, cy as i32),
                        (cx as i32 - 1, cy as i32),
                        (cx as i32, cy as i32 + 1),
                        (cx as i32, cy as i32 - 1),
                    ];
                    for (nx, ny) in nbrs {
                        if nx < 0 || ny < 0 || nx >= w as i32 || ny >= h as i32 {
                            continue;
                        }
                        let ni = (ny as usize) * w + (nx as usize);
                        if visited[ni] || self.data[ni] == 0 {
                            continue;
                        }
                        visited[ni] = true;
                        stack.push((nx as usize, ny as usize));
                    }
                }
                out.push(comp);
            }
        }
        out
    }

    fn trace_component_boundary(&self, cells: &[(u32, u32)]) -> Vec<Vec<(i32, i32)>> {
        use std::collections::{HashMap, HashSet};

        let cell_set: HashSet<(u32, u32)> = cells.iter().copied().collect();
        let has = |x: i32, y: i32| -> bool {
            x >= 0 && y >= 0 && cell_set.contains(&(x as u32, y as u32))
        };

        // Collect directed boundary edges. CW around on-cells (Y grows down).
        let mut edges: Vec<((i32, i32), (i32, i32))> = Vec::new();
        for &(ux, uy) in cells {
            let x = ux as i32;
            let y = uy as i32;
            if !has(x, y - 1) {
                edges.push(((x, y), (x + 1, y)));
            }
            if !has(x + 1, y) {
                edges.push(((x + 1, y), (x + 1, y + 1)));
            }
            if !has(x, y + 1) {
                edges.push(((x + 1, y + 1), (x, y + 1)));
            }
            if !has(x - 1, y) {
                edges.push(((x, y + 1), (x, y)));
            }
        }

        let mut by_start: HashMap<(i32, i32), Vec<usize>> = HashMap::new();
        for (i, &(from, _)) in edges.iter().enumerate() {
            by_start.entry(from).or_default().push(i);
        }

        let mut used = vec![false; edges.len()];
        let mut loops: Vec<Vec<(i32, i32)>> = Vec::new();
        for seed in 0..edges.len() {
            if used[seed] {
                continue;
            }
            let mut pts: Vec<(i32, i32)> = Vec::new();
            let mut cur = seed;
            loop {
                used[cur] = true;
                pts.push(edges[cur].0);
                let to = edges[cur].1;
                let candidates = by_start.get(&to).cloned().unwrap_or_default();
                let nxt = candidates.iter().find(|&&i| !used[i]).copied();
                match nxt {
                    Some(n) if n == seed => {
                        // Closed loop. Ensure explicit close.
                        pts.push(edges[seed].0);
                        break;
                    }
                    Some(n) => cur = n,
                    None => break,
                }
            }
            loops.push(pts);
        }
        // Compress collinear vertices so the polygon has as few points as
        // possible.
        loops
            .into_iter()
            .map(|mut p| {
                compress_collinear(&mut p);
                p
            })
            .collect()
    }

    fn rectangles_for_cells(&self, cells: &[(u32, u32)]) -> Vec<Rect> {
        // Build a sub-bitmap the size of the bounding box containing only
        // this component's cells, then run the standard rectangle
        // decomposition on it and translate the result back.
        let (mut min_x, mut min_y) = (u32::MAX, u32::MAX);
        let (mut max_x, mut max_y) = (0u32, 0u32);
        for &(x, y) in cells {
            min_x = min_x.min(x);
            min_y = min_y.min(y);
            max_x = max_x.max(x);
            max_y = max_y.max(y);
        }
        let w = max_x - min_x + 1;
        let h = max_y - min_y + 1;
        let mut sub = Bitmap::new(w, h);
        for &(x, y) in cells {
            sub.set(x - min_x, y - min_y, true);
        }
        sub.to_rectangles()
            .into_iter()
            .map(|r| Rect {
                x: r.x + min_x,
                y: r.y + min_y,
                w: r.w,
                h: r.h,
            })
            .collect()
    }
}

fn compress_collinear(pts: &mut Vec<(i32, i32)>) {
    if pts.len() < 3 {
        return;
    }
    // Drop the explicit close before compressing to avoid touching the
    // seam; add it back at the end.
    let closed = pts.first() == pts.last();
    if closed {
        pts.pop();
    }
    let mut i = 0;
    while i < pts.len() && pts.len() >= 3 {
        let prev = pts[(i + pts.len() - 1) % pts.len()];
        let cur = pts[i];
        let next = pts[(i + 1) % pts.len()];
        // Axis-aligned collinearity: all three share x or all share y.
        if (prev.0 == cur.0 && cur.0 == next.0) || (prev.1 == cur.1 && cur.1 == next.1) {
            pts.remove(i);
            // Don't advance; re-check the same index with the shifted
            // sequence.
            i = i.saturating_sub(1);
        } else {
            i += 1;
        }
    }
    if closed && !pts.is_empty() {
        let first = pts[0];
        pts.push(first);
    }
}
