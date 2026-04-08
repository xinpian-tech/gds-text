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
    pub fn iter_on(&self) -> impl Iterator<Item = (u32, u32)> + '_ {
        (0..self.height).flat_map(move |y| {
            (0..self.width).filter_map(move |x| if self.get(x, y) { Some((x, y)) } else { None })
        })
    }
}
