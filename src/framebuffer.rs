pub struct Framebuffer {
    pub width: usize,
    pub height: usize,
    pub buffer: Vec<u32>,
    pub zbuffer: Vec<f32>,
    background_color: u32,
    current_color: u32,
}

impl Framebuffer {
    pub fn new(width: usize, height: usize) -> Self {
        Framebuffer {
            width,
            height,
            buffer: vec![0; width * height],
            zbuffer: vec![f32::INFINITY; width * height],
            background_color: 0x000000,
            current_color: 0xFFFFFF,
        }
    }

    pub fn clear(&mut self) {
        for px in self.buffer.iter_mut() {
            *px = self.background_color;
        }
        for z in self.zbuffer.iter_mut() {
            *z = f32::INFINITY;
        }
    }

    // legacy point API (no depth test) - still available if you need it
    pub fn point(&mut self, x: usize, y: usize) {
        if x < self.width && y < self.height {
            self.buffer[y * self.width + x] = self.current_color;
        }
    }

    // NEW: depth-tested plot with explicit color and depth
    pub fn plot(&mut self, x: usize, y: usize, color: u32, depth: f32) {
        if x < self.width && y < self.height {
            let idx = y * self.width + x;
            if depth < self.zbuffer[idx] {
                self.zbuffer[idx] = depth;
                self.buffer[idx] = color;
            }
        }
    }

    pub fn set_background_color(&mut self, color: u32) {
        self.background_color = color;
    }

    pub fn set_current_color(&mut self, color: u32) {
        self.current_color = color;
    }

    pub fn blend_add(&mut self, x: usize, y: usize, color: u32) {
        if x < self.width && y < self.height {
            let idx = y * self.width + x;

            let dst = self.buffer[idx];
            let dr = ((dst >> 16) & 0xFF) as u16;
            let dg = ((dst >> 8)  & 0xFF) as u16;
            let db = ( dst        & 0xFF) as u16;

            let sr = ((color >> 16) & 0xFF) as u16;
            let sg = ((color >> 8)  & 0xFF) as u16;
            let sb = ( color        & 0xFF) as u16;

            let nr = (dr + sr).min(255) as u32;
            let ng = (dg + sg).min(255) as u32;
            let nb = (db + sb).min(255) as u32;

            self.buffer[idx] = (nr << 16) | (ng << 8) | nb;
            // NOTE: depth is NOT updated on additive blends
        }
    }
}