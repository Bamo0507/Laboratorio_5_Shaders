use nalgebra_glm as glm;
use glm::Vec3;
use crate::framebuffer::Framebuffer;

pub struct ImageFace {
    pub w: usize,
    pub h: usize,
    pub data: Vec<u32>, // 0xRRGGBB
}

pub struct Skybox {
    // order: 0=+X(px), 1=-X(nx), 2=+Y(py), 3=-Y(ny), 4=+Z(pz), 5=-Z(nz)
    faces: [ImageFace; 6],
}

impl Skybox {
    pub fn load(dir: &str) -> Self {
        fn load_face(path: &str) -> ImageFace {
            let img = image::io::Reader::open(path)
                .expect("open skybox face")
                .decode()
                .expect("decode skybox face")
                .to_rgba8();
            let (w, h) = img.dimensions();
            let mut data = Vec::with_capacity((w * h) as usize);
            for p in img.pixels() {
                let r = p[0] as u32;
                let g = p[1] as u32;
                let b = p[2] as u32;
                data.push((r << 16) | (g << 8) | b);
            }
            ImageFace { w: w as usize, h: h as usize, data }
        }

        let px = load_face(&format!("{}/px.png", dir));
        let nx = load_face(&format!("{}/nx.png", dir));
        let py = load_face(&format!("{}/py.png", dir));
        let ny = load_face(&format!("{}/ny.png", dir));
        let pz = load_face(&format!("{}/pz.png", dir));
        let nz = load_face(&format!("{}/nz.png", dir));

        Skybox { faces: [px, nx, py, ny, pz, nz] }
    }

    #[inline]
    fn sample(&self, dir: Vec3) -> u32 {
        // Map direction → cube face + (u,v) in [-1,1], then to [0,1]
        let x = dir.x; let y = dir.y; let z = dir.z;
        let ax = x.abs(); let ay = y.abs(); let az = z.abs();

        let (idx, u, v) = if ax >= ay && ax >= az {
            if x > 0.0 { (0, -z/ax, -y/ax) } else { (1,  z/ax, -y/ax) } // ±X
        } else if ay >= az {
            if y > 0.0 { (2,  x/ay,  z/ay) } else { (3,  x/ay, -z/ay) } // ±Y
        } else {
            if z > 0.0 { (4,  x/az, -y/az) } else { (5, -x/az, -y/az) } // ±Z
        };

        let s = ((u + 1.0) * 0.5).clamp(0.0, 1.0);
        let t = ((v + 1.0) * 0.5).clamp(0.0, 1.0);

        let face = &self.faces[idx];
        let ix = (s * (face.w as f32 - 1.0)).round() as usize;
        let iy = (t * (face.h as f32 - 1.0)).round() as usize;
        face.data[iy * face.w + ix]
    }

    /// Draw a skybox for a pinhole camera at the origin with yaw/pitch.
    /// `fov_y` in radians. Positive yaw turns right; positive pitch looks up.
    pub fn draw(&self, fb: &mut Framebuffer, fov_y: f32, yaw: f32, pitch: f32) {
        let aspect = fb.width as f32 / fb.height as f32;
        let tan_half = (fov_y * 0.5).tan();
        let (sy, cy) = yaw.sin_cos();
        let (sp, cp) = pitch.sin_cos();

        for y in 0..fb.height {
            let vv = 1.0 - ((y as f32 + 0.5) / fb.height as f32) * 2.0; // NDC y in [-1,1]
            for x in 0..fb.width {
                let uu = ((x as f32 + 0.5) / fb.width as f32) * 2.0 - 1.0; // NDC x

                // Ray direction in camera space (z looking forward negative)
                let mut dir = Vec3::new(uu * aspect * tan_half, vv * tan_half, -1.0);
                dir = glm::normalize(&dir);

                // Apply yaw (Y axis), then pitch (X axis)
                // yaw
                let dx =  cy * dir.x + sy * dir.z;
                let dz = -sy * dir.x + cy * dir.z;
                dir.x = dx; dir.z = dz;
                // pitch
                let dy =  cp * dir.y - sp * dir.z;
                let dz2 = sp * dir.y + cp * dir.z;
                dir.y = dy; dir.z = dz2;

                let color = self.sample(dir);
                let idx = y * fb.width + x;
                fb.buffer[idx] = color;
                fb.zbuffer[idx] = f32::INFINITY; // sky is always the farthest
            }
        }
    }
}