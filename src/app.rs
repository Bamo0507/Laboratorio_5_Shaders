

use nalgebra_glm as glm;
use glm::{Mat4, Vec3, Vec4};
use minifb::{Key, Window};
use std::time::Instant;

use rayon::prelude::*;

use crate::camera::Camera;
use crate::color::Color;
use crate::framebuffer::Framebuffer;
use crate::obj::Obj;
use crate::skybox::Skybox;
use crate::vertex::Vertex;
use crate::shaders::{self, PlanetShader};
use crate::{triangle, Uniforms};

#[inline]
fn create_model_matrix(translation: Vec3, scale: f32, rotation: Vec3) -> Mat4 {
    let (sin_x, cos_x) = rotation.x.sin_cos();
    let (sin_y, cos_y) = rotation.y.sin_cos();
    let (sin_z, cos_z) = rotation.z.sin_cos();

    let rotation_matrix_x = Mat4::new(
        1.0,  0.0,    0.0,   0.0,
        0.0,  cos_x, -sin_x, 0.0,
        0.0,  sin_x,  cos_x, 0.0,
        0.0,  0.0,    0.0,   1.0,
    );
    let rotation_matrix_y = Mat4::new(
        cos_y,  0.0,  sin_y, 0.0,
        0.0,    1.0,  0.0,   0.0,
        -sin_y, 0.0,  cos_y, 0.0,
        0.0,    0.0,  0.0,   1.0,
    );
    let rotation_matrix_z = Mat4::new(
        cos_z, -sin_z, 0.0, 0.0,
        sin_z,  cos_z, 0.0, 0.0,
        0.0,    0.0,  1.0, 0.0,
        0.0,    0.0,  0.0, 1.0,
    );
    let rotation_matrix = rotation_matrix_z * rotation_matrix_y * rotation_matrix_x;

    Mat4::new(
        scale, 0.0,   0.0,   translation.x,
        0.0,   scale, 0.0,   translation.y,
        0.0,   0.0,   scale, translation.z,
        0.0,   0.0,   0.0,   1.0,
    ) * rotation_matrix
}

/// Depth-aware 2D line (for trails)
#[inline]
fn draw_line_2d_depth(
    fb: &mut Framebuffer,
    mut x0: isize, mut y0: isize,
    x1: isize, y1: isize,
    color: u32,
    depth: f32,
) {
    let (w, h) = (fb.width as isize, fb.height as isize);
    let dx = (x1 - x0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let dy = -(y1 - y0).abs();
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;
    loop {
        if x0 >= 0 && y0 >= 0 && x0 < w && y0 < h {
            fb.plot(x0 as usize, y0 as usize, color, depth);
        }
        if x0 == x1 && y0 == y1 { break; }
        let e2 = 2 * err;
        if e2 >= dy { err += dy; x0 += sx; }
        if e2 <= dx { err += dx; y0 += sy; }
    }
}

/// Project a world position to screen (for trail points)
#[inline]
fn project_to_screen(
    view: &Mat4,
    proj: &Mat4,
    screen: (f32, f32),
    world_pos: Vec3,
) -> Option<(isize, isize)> {
    let p = glm::vec4(world_pos.x, world_pos.y, world_pos.z, 1.0);
    let v = view * p;
    let c = proj * v;
    if c.w.abs() < 1e-6 { return None; }
    let ndc = glm::vec3(c.x / c.w, c.y / c.w, c.z / c.w);
    if ndc.x < -1.0 || ndc.x > 1.0 || ndc.y < -1.0 || ndc.y > 1.0 || ndc.z < -1.0 || ndc.z > 1.0 {
        return None;
    }
    let (sw, sh) = screen;
    let sx = (ndc.x * 0.5 + 0.5) * sw;
    let sy = (1.0 - (ndc.y * 0.5 + 0.5)) * sh;
    Some((sx.round() as isize, sy.round() as isize))
}

/// Conservative view-frustum test for a bounding sphere in VIEW space.
#[inline]
fn is_sphere_visible(
    view: &Mat4,
    fov_y: f32,
    aspect: f32,
    near: f32,
    far: f32,
    world_center: Vec3,
    world_radius: f32,
) -> bool {
    let p = glm::vec4(world_center.x, world_center.y, world_center.z, 1.0);
    let v = view * p;
    let (xv, yv, zv) = (v.x, v.y, v.z);
    let depth = -zv;

    if depth + world_radius <= near { return false; }
    if depth - world_radius >= far  { return false; }

    let tan_half_y = (fov_y * 0.5).tan();
    let tan_half_x = tan_half_y * aspect;

    let max_x = depth * tan_half_x + world_radius;
    let min_x = -max_x;
    let max_y = depth * tan_half_y + world_radius;
    let min_y = -max_y;

    if xv < min_x { return false; }
    if xv > max_x { return false; }
    if yv < min_y { return false; }
    if yv > max_y { return false; }

    true
}

/* ==== App ============================================================== */

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Inspect { All, Star, Rocky, Gas, Lava, Verdant, GasGold }

pub struct App {
    // camera & view
    camera: Camera,
    pub fov_y: f32,
    near: f32,
    far: f32,

    // assets
    skybox: Skybox,
    sphere_arrays: Vec<Vertex>,
    ring_arrays: Vec<Vertex>,

    // state
    start_time: Instant,
    inspect: Inspect,
    prev_inspect: Inspect,

    // trails
    rocky_trail: Vec<Vec3>,
    gas_trail: Vec<Vec3>,
    lava_trail: Vec<Vec3>,
    verdant_trail: Vec<Vec3>,
    gas_gold_trail: Vec<Vec3>,
    max_trail_len: usize,

    // layout constants
    star_scale: f32,

    rocky_orbit: f32,
    rocky_scale: f32,
    rocky_orbit_speed: f32,
    rocky_spin_speed: f32,

    gas_orbit_rel: f32,
    gas_scale: f32,
    gas_orbit_speed: f32,
    gas_spin_speed: f32,

    lava_orbit_rel: f32,
    lava_scale: f32,
    lava_orbit_speed: f32,
    lava_spin_speed: f32,

    verdant_orbit_rel: f32,
    verdant_scale: f32,
    verdant_orbit_speed: f32,
    verdant_spin_speed: f32,

    gas_gold_orbit_rel: f32,
    gas_gold_scale: f32,
    gas_gold_orbit_speed: f32,
    gas_gold_spin_speed: f32,

    ambient_planet: f32,
}

impl App {
    pub fn new() -> Self {
        let skybox = Skybox::load("assets/skybox");
        let sphere = Obj::load("assets/models/sphere.obj").expect("Failed to load sphere");
        let ring   = Obj::load("assets/models/ring.obj").expect("Failed to load ring.obj");

        Self {
            camera: Camera::new(3600.0),
            fov_y: 60.0_f32.to_radians(),
            near: 50.0,
            far: 8000.0,

            skybox,
            sphere_arrays: sphere.get_vertex_array(),
            ring_arrays: ring.get_vertex_array(),

            start_time: Instant::now(),
            inspect: Inspect::All,
            prev_inspect: Inspect::All,

            rocky_trail: Vec::new(),
            gas_trail: Vec::new(),
            lava_trail: Vec::new(),
            verdant_trail: Vec::new(),
            gas_gold_trail: Vec::new(),
            max_trail_len: 500,

            star_scale: 110.0,

            rocky_orbit: 380.0,
            rocky_scale: 52.0,
            rocky_orbit_speed: 0.55,
            rocky_spin_speed: 0.9,

            gas_orbit_rel: 700.0,
            gas_scale: 140.0,
            gas_orbit_speed: 0.32,
            gas_spin_speed: 0.5,

            lava_orbit_rel: 180.0,
            lava_scale: 44.0,
            lava_orbit_speed: 0.68,
            lava_spin_speed: 1.1,

            verdant_orbit_rel: 320.0,
            verdant_scale: 48.0,
            verdant_orbit_speed: 0.47,
            verdant_spin_speed: 0.8,

            gas_gold_orbit_rel: 1280.0,
            gas_gold_scale: 120.0,
            gas_gold_orbit_speed: 0.27,
            gas_gold_spin_speed: 0.45,

            ambient_planet: 0.38,
        }
    }

    #[inline] fn center_on_circle(&self, radius: f32, theta: f32) -> Vec3 {
        Vec3::new(radius * theta.cos(), 0.0, -radius * theta.sin())
    }

    pub fn handle_hotkeys(&mut self, window: &Window) {
        if window.is_key_down(Key::Key0) { self.inspect = Inspect::All; }
        if window.is_key_down(Key::Key1) { self.inspect = Inspect::Star; }
        if window.is_key_down(Key::Key2) { self.inspect = Inspect::Rocky; }
        if window.is_key_down(Key::Key3) { self.inspect = Inspect::Gas; }
        if window.is_key_down(Key::Key4) { self.inspect = Inspect::Lava; }
        if window.is_key_down(Key::Key5) { self.inspect = Inspect::Verdant; }
        if window.is_key_down(Key::Key6) { self.inspect = Inspect::GasGold; }

        if self.inspect != self.prev_inspect {
            let focus_radius = match self.inspect {
                Inspect::Star     => self.star_scale * 1.30,
                Inspect::Rocky    => self.rocky_scale * 1.20,
                Inspect::Gas      => self.gas_scale * 1.20,
                Inspect::Lava     => self.lava_scale * 1.20,
                Inspect::Verdant  => self.verdant_scale * 1.20,
                Inspect::GasGold  => self.gas_gold_scale * 1.20,
                Inspect::All      => 1000.0,
            };
            let inspect_zoom = if self.inspect == Inspect::All { 1.0 } else { 1.35 }; // a little farther when inspecting
            self.camera.frame_radius(focus_radius * inspect_zoom);
            self.prev_inspect = self.inspect;
        }
    }

    pub fn frame(&mut self, framebuffer: &mut Framebuffer, window: &Window) {
        // input
        self.camera.update_input(window);
        self.handle_hotkeys(window);

        // time: keep ORBITS continuous always; optionally freeze only spin/animated layers when inspecting
        let t_real = self.start_time.elapsed().as_secs_f32();
        let frozen = !matches!(self.inspect, Inspect::All);
        let t_orbit = t_real;            // do NOT freeze orbit when inspecting
        let t_spin  = if frozen { 0.0 } else { t_real }; // can freeze spins/clouds if desired

        // matrices
        let aspect = framebuffer.width as f32 / framebuffer.height as f32;
        let target = {
            let rocky_center  = self.center_on_circle(self.rocky_orbit,                             self.rocky_orbit_speed * t_orbit);
            let lava_center   = self.center_on_circle(self.rocky_orbit + self.lava_orbit_rel,       self.lava_orbit_speed * t_orbit);
            let verdant_center= self.center_on_circle(self.rocky_orbit + self.verdant_orbit_rel,    self.verdant_orbit_speed * t_orbit);
            let gas_center    = self.center_on_circle(self.rocky_orbit + self.gas_orbit_rel,        self.gas_orbit_speed * t_orbit);
            let gas_gold_cent = self.center_on_circle(self.rocky_orbit + self.gas_gold_orbit_rel,   self.gas_gold_orbit_speed * t_orbit);
            match self.inspect {
                Inspect::All     => Vec3::new(0.0,0.0,0.0),
                Inspect::Star    => Vec3::new(0.0,0.0,0.0),
                Inspect::Rocky   => rocky_center,
                Inspect::Gas     => gas_center,
                Inspect::Lava    => lava_center,
                Inspect::Verdant => verdant_center,
                Inspect::GasGold => gas_gold_cent,
            }
        };
        let view = self.camera.view_matrix(target);
        let proj = glm::perspective(aspect, self.fov_y, self.near, self.far);
        let screen_size = (framebuffer.width as f32, framebuffer.height as f32);

        // ambient boost when inspecting a single body
        let ambient_for_planets = if matches!(self.inspect, Inspect::All) { self.ambient_planet } else { 1.0 };

        // skybox
        let (yaw, pitch) = self.camera.angles();
        self.skybox.draw(framebuffer, self.fov_y, yaw, pitch);

        // render the whole system
        self.draw_system(framebuffer, view, proj, screen_size, t_real, t_orbit, t_spin, ambient_for_planets, frozen, aspect);
    }

    fn draw_system(
        &mut self,
        framebuffer: &mut Framebuffer,
        view: Mat4,
        proj: Mat4,
        screen_size: (f32, f32),
        t_real: f32, t_orbit: f32, t_spin: f32,
        ambient_for_planets: f32,
        frozen: bool,
        aspect: f32,
    ) {
        // ——— helpers that mirror your current render functions ———
        let mut render_planet = |fb: &mut Framebuffer, uniforms: &Uniforms, verts: &[Vertex], shader: PlanetShader, unlit: bool| {
            let transformed: Vec<_> = verts.par_iter().map(|v| shaders::vertex_shader(v, uniforms)).collect();
            let tris: Vec<_> = transformed.chunks_exact(3).map(|c| [c[0].clone(), c[1].clone(), c[2].clone()]).collect();
            let frags: Vec<_> = tris.par_iter().flat_map(|tri| triangle::triangle_planet(&tri[0], &tri[1], &tri[2], uniforms, shader, unlit)).collect();
            for f in frags {
                let x = f.position.x as isize; let y = f.position.y as isize;
                if x >= 0 && y >= 0 && (x as usize) < fb.width && (y as usize) < fb.height {
                    fb.plot(x as usize, y as usize, f.color.to_hex(), f.depth);
                }
            }
        };
        let mut render_planet_add = |fb: &mut Framebuffer, uniforms: &Uniforms, verts: &[Vertex], shader: PlanetShader, unlit: bool| {
            let transformed: Vec<_> = verts.par_iter().map(|v| shaders::vertex_shader(v, uniforms)).collect();
            let tris: Vec<_> = transformed.chunks_exact(3).map(|c| [c[0].clone(), c[1].clone(), c[2].clone()]).collect();
            let frags: Vec<_> = tris.par_iter().flat_map(|tri| triangle::triangle_planet(&tri[0], &tri[1], &tri[2], uniforms, shader, unlit)).collect();
            for f in frags {
                let x = f.position.x as isize; let y = f.position.y as isize;
                if x >= 0 && y >= 0 && (x as usize) < fb.width && (y as usize) < fb.height {
                    fb.blend_add(x as usize, y as usize, f.color.to_hex());
                }
            }
        };

        let is_visible = |center: Vec3, radius: f32| -> bool {
            is_sphere_visible(&view, self.fov_y, aspect, self.near, self.far, center, radius)
        };

        // ======= BEGIN: ported rendering from previous main.rs =======

        // Time/centers
        let star_pos = Vec3::new(0.0, 0.0, 0.0);
        let star_rot = Vec3::new(0.0, 0.0, 0.0);

        let rocky_center = self.center_on_circle(self.rocky_orbit, self.rocky_orbit_speed * t_orbit);
        let lava_orbit = self.rocky_orbit + self.lava_orbit_rel;
        let lava_center = self.center_on_circle(lava_orbit, self.lava_orbit_speed * t_orbit);
        let verdant_orbit = self.rocky_orbit + self.verdant_orbit_rel;
        let verdant_center = self.center_on_circle(verdant_orbit, self.verdant_orbit_speed * t_orbit);
        let gas_orbit = self.rocky_orbit + self.gas_orbit_rel;
        let gas_center = self.center_on_circle(gas_orbit, self.gas_orbit_speed * t_orbit);
        let gas_gold_orbit = self.rocky_orbit + self.gas_gold_orbit_rel;
        let gas_gold_center = self.center_on_circle(gas_gold_orbit, self.gas_gold_orbit_speed * t_orbit);

        // Inspect toggles
        let allow_star     = matches!(self.inspect, Inspect::All | Inspect::Star);
        let allow_rocky    = matches!(self.inspect, Inspect::All | Inspect::Rocky);
        let allow_gas      = matches!(self.inspect, Inspect::All | Inspect::Gas);
        let allow_lava     = matches!(self.inspect, Inspect::All | Inspect::Lava);
        let allow_verdant  = matches!(self.inspect, Inspect::All | Inspect::Verdant);
        let allow_gas_gold = matches!(self.inspect, Inspect::All | Inspect::GasGold);

        // ===== STAR =====
        let fov_y = self.fov_y; let near = self.near; let far = self.far; let aspect = aspect;
        let star_visible = is_visible(star_pos, self.star_scale * 1.30);
        if allow_star && star_visible {
            // Core
            let mm_core = create_model_matrix(star_pos, self.star_scale, star_rot);
            let u_core = Uniforms { model_matrix: mm_core, view_matrix: view, proj_matrix: proj, screen_size, light_dir: Vec3::new(0.0,-1.0,0.0), base_color: Color::from_hex(0x0A1E4A), ambient: 0.0, star_pos };
            render_planet(framebuffer, &u_core, &self.sphere_arrays, PlanetShader::StarCore, true);
            // Plasma
            let mm_plasma = create_model_matrix(star_pos, self.star_scale * 1.08, star_rot);
            let u_plasma = Uniforms { model_matrix: mm_plasma, ..u_core };
            render_planet_add(framebuffer, &u_plasma, &self.sphere_arrays, PlanetShader::StarPlasma, true);
            // Filaments
            let mm_fil = create_model_matrix(star_pos, self.star_scale * 1.06, star_rot);
            let u_fil = Uniforms { model_matrix: mm_fil, ..u_core };
            render_planet_add(framebuffer, &u_fil, &self.sphere_arrays, PlanetShader::StarFilaments, true);
            // Hotspots (opaque overlay)
            let mm_hot = create_model_matrix(star_pos, self.star_scale * 1.002, star_rot);
            let u_hot = Uniforms { model_matrix: mm_hot, ambient: if frozen { 0.0 } else { t_real }, ..u_core };
            render_planet(framebuffer, &u_hot, &self.sphere_arrays, PlanetShader::StarHotspots, true);
            // Corona
            let mm_corona = create_model_matrix(star_pos, self.star_scale * 1.30, star_rot);
            let u_corona = Uniforms { model_matrix: mm_corona, ..u_core };
            render_planet_add(framebuffer, &u_corona, &self.sphere_arrays, PlanetShader::StarCorona, true);
        }

        // ===== ROCKY (EARTH-LIKE) + MOON =====
        self.rocky_trail.push(rocky_center);
        if self.rocky_trail.len() > self.max_trail_len { self.rocky_trail.remove(0); }
        let rocky_visible = allow_rocky && is_visible(rocky_center, self.rocky_scale * 1.02);
        if rocky_visible {
            let rocky_rot = Vec3::new(0.0, -self.rocky_spin_speed * t_spin, 0.0);
            let mm_rocky = create_model_matrix(rocky_center, self.rocky_scale, rocky_rot);
            let rocky_light_dir = glm::normalize(&(rocky_center - star_pos));
            let u_earth = Uniforms { model_matrix: mm_rocky, view_matrix: view, proj_matrix: proj, screen_size, light_dir: rocky_light_dir, base_color: Color::from_hex(0x000000), ambient: ambient_for_planets, star_pos };
            render_planet(framebuffer, &u_earth, &self.sphere_arrays, PlanetShader::EarthOcean, false);
            render_planet_add(framebuffer, &u_earth, &self.sphere_arrays, PlanetShader::EarthLand, false);
            let mm_clouds = create_model_matrix(rocky_center, self.rocky_scale * 1.020, rocky_rot);
            let u_clouds = Uniforms { model_matrix: mm_clouds, ambient: if frozen { 0.0 } else { t_real }, ..u_earth };
            render_planet_add(framebuffer, &u_clouds, &self.sphere_arrays, PlanetShader::EarthClouds, true);
            let u_night = Uniforms { ambient: if frozen { 0.0 } else { t_real }, ..u_earth };
            render_planet_add(framebuffer, &u_night, &self.sphere_arrays, PlanetShader::EarthNight, true);

            // Moon
            let moon_scale: f32 = self.rocky_scale * 0.32;
            let moon_center = rocky_center + Vec3::new(self.rocky_scale * 1.8, self.rocky_scale * 0.9, 18.0);
            let moon_rot = Vec3::new(0.0, -0.6 * t_spin, 0.0);
            let mm_moon = create_model_matrix(moon_center, moon_scale, moon_rot);
            let moon_ld = glm::normalize(&(moon_center - star_pos));
            let u_moon = Uniforms { model_matrix: mm_moon, light_dir: moon_ld, ..u_earth };
            render_planet(framebuffer, &u_moon, &self.sphere_arrays, PlanetShader::MoonBase, false);
            render_planet_add(framebuffer, &u_moon, &self.sphere_arrays, PlanetShader::MoonDetail, false);
            render_planet_add(framebuffer, &u_moon, &self.sphere_arrays, PlanetShader::MoonRim, true);
            render_planet_add(framebuffer, &u_moon, &self.sphere_arrays, PlanetShader::MoonThermal, true);
        }

        // ===== LAVA ROCKY + MOON =====
        self.lava_trail.push(lava_center);
        if self.lava_trail.len() > self.max_trail_len { self.lava_trail.remove(0); }
        let lava_visible = allow_lava && is_visible(lava_center, self.lava_scale * 1.02);
        if lava_visible {
            let lava_rot = Vec3::new(0.0, -self.lava_spin_speed * t_spin, 0.0);
            let mm_lava = create_model_matrix(lava_center, self.lava_scale, lava_rot);
            let lava_ld = glm::normalize(&(lava_center - star_pos));
            let u_lava = Uniforms { model_matrix: mm_lava, view_matrix: view, proj_matrix: proj, screen_size, light_dir: lava_ld, base_color: Color::from_hex(0x000000), ambient: ambient_for_planets, star_pos };
            render_planet(framebuffer, &u_lava, &self.sphere_arrays, PlanetShader::RockLavaOcean, false);
            render_planet_add(framebuffer, &u_lava, &self.sphere_arrays, PlanetShader::RockLavaLand, false);
            let mm_lava_clouds = create_model_matrix(lava_center, self.lava_scale * 1.018, lava_rot);
            let u_lava_clouds = Uniforms { model_matrix: mm_lava_clouds, ambient: if frozen { 0.0 } else { t_real }, ..u_lava };
            render_planet_add(framebuffer, &u_lava_clouds, &self.sphere_arrays, PlanetShader::RockLavaClouds, true);
            let u_lava_night = Uniforms { ambient: if frozen { 0.0 } else { t_real }, ..u_lava };
            render_planet_add(framebuffer, &u_lava_night, &self.sphere_arrays, PlanetShader::RockLavaNight, true);

            let lava_moon_scale: f32 = self.lava_scale * 0.28;
            let lava_moon_center = lava_center + Vec3::new(self.lava_scale * 1.6, self.lava_scale * 0.7, 14.0);
            let lava_moon_rot = Vec3::new(0.0, -0.7 * t_spin, 0.0);
            let mm_lava_moon = create_model_matrix(lava_moon_center, lava_moon_scale, lava_moon_rot);
            let lava_moon_ld = glm::normalize(&(lava_moon_center - star_pos));
            let u_lava_moon = Uniforms { model_matrix: mm_lava_moon, light_dir: lava_moon_ld, ..u_lava };
            render_planet(framebuffer, &u_lava_moon, &self.sphere_arrays, PlanetShader::MoonLavaBase, false);
            render_planet_add(framebuffer, &u_lava_moon, &self.sphere_arrays, PlanetShader::MoonLavaDetail, false);
            render_planet_add(framebuffer, &u_lava_moon, &self.sphere_arrays, PlanetShader::MoonLavaRim, true);
            render_planet_add(framebuffer, &u_lava_moon, &self.sphere_arrays, PlanetShader::MoonLavaThermal, true);
        }

        // ===== VERDANT ROCKY + MOON =====
        self.verdant_trail.push(verdant_center);
        if self.verdant_trail.len() > self.max_trail_len { self.verdant_trail.remove(0); }
        let verdant_visible = allow_verdant && is_visible(verdant_center, self.verdant_scale * 1.02);
        if verdant_visible {
            let verdant_rot = Vec3::new(0.0, -self.verdant_spin_speed * t_spin, 0.0);
            let mm_verdant = create_model_matrix(verdant_center, self.verdant_scale, verdant_rot);
            let verdant_ld = glm::normalize(&(verdant_center - star_pos));
            let u_verdant = Uniforms { model_matrix: mm_verdant, view_matrix: view, proj_matrix: proj, screen_size, light_dir: verdant_ld, base_color: Color::from_hex(0x000000), ambient: ambient_for_planets, star_pos };
            render_planet(framebuffer, &u_verdant, &self.sphere_arrays, PlanetShader::RockVerdantOcean, false);
            render_planet_add(framebuffer, &u_verdant, &self.sphere_arrays, PlanetShader::RockVerdantLand, false);
            let mm_verdant_clouds = create_model_matrix(verdant_center, self.verdant_scale * 1.02, verdant_rot);
            let u_verdant_clouds = Uniforms { model_matrix: mm_verdant_clouds, ambient: if frozen { 0.0 } else { t_real }, ..u_verdant };
            render_planet_add(framebuffer, &u_verdant_clouds, &self.sphere_arrays, PlanetShader::RockVerdantClouds, true);
            let u_verdant_night = Uniforms { ambient: if frozen { 0.0 } else { t_real }, ..u_verdant };
            render_planet_add(framebuffer, &u_verdant_night, &self.sphere_arrays, PlanetShader::RockVerdantNight, true);

            let ver_moon_scale: f32 = self.verdant_scale * 0.26;
            let ver_moon_center = verdant_center + Vec3::new(self.verdant_scale * 1.7, self.verdant_scale * 0.8, 16.0);
            let ver_moon_rot = Vec3::new(0.0, -0.55 * t_spin, 0.0);
            let mm_ver_moon = create_model_matrix(ver_moon_center, ver_moon_scale, ver_moon_rot);
            let ver_moon_ld = glm::normalize(&(ver_moon_center - star_pos));
            let u_ver_moon = Uniforms { model_matrix: mm_ver_moon, light_dir: ver_moon_ld, ..u_verdant };
            render_planet(framebuffer, &u_ver_moon, &self.sphere_arrays, PlanetShader::MoonVerdantBase, false);
            render_planet_add(framebuffer, &u_ver_moon, &self.sphere_arrays, PlanetShader::MoonVerdantDetail, false);
            render_planet_add(framebuffer, &u_ver_moon, &self.sphere_arrays, PlanetShader::MoonVerdantRim, true);
            render_planet_add(framebuffer, &u_ver_moon, &self.sphere_arrays, PlanetShader::MoonVerdantThermal, true);
        }

        // ===== GAS GIANT + RING =====
        self.gas_trail.push(gas_center);
        if self.gas_trail.len() > self.max_trail_len { self.gas_trail.remove(0); }
        let ring_outer = self.gas_scale * 1.06;
        let gas_visible = allow_gas && is_visible(gas_center, ring_outer);
        if gas_visible {
            let gas_rot = Vec3::new(0.0, -self.gas_spin_speed * t_spin, 0.0);
            let mm_gas = create_model_matrix(gas_center, self.gas_scale, gas_rot);
            let gas_ld = glm::normalize(&(gas_center - star_pos));

            // Ring (base + glow)
            let tilt_x = 28.0_f32.to_radians();
            let tilt_z = 12.0_f32.to_radians();
            let ring_spin = -0.15 * t_spin;
            let ring_scale_factor: f32 = 1.0;
            let ring_glow_expand: f32 = 1.06;
            let ring_scale = self.gas_scale * ring_scale_factor;
            let ring_rot   = Vec3::new(tilt_x, ring_spin, tilt_z);

            let mm_ring = create_model_matrix(gas_center, ring_scale, ring_rot);
            let u_ring_base = Uniforms { model_matrix: mm_ring, view_matrix: view, proj_matrix: proj, screen_size, light_dir: gas_ld, base_color: Color::from_hex(0x000000), ambient: 0.0, star_pos };
            render_planet(framebuffer, &u_ring_base, &self.ring_arrays, PlanetShader::RingBase, true);
            let mm_ring_glow = create_model_matrix(gas_center, ring_scale * ring_glow_expand, ring_rot);
            let u_ring_glow = Uniforms { model_matrix: mm_ring_glow, ..u_ring_base };
            render_planet_add(framebuffer, &u_ring_glow, &self.ring_arrays, PlanetShader::RingGlow, true);

            // Sphere layers
            let u_gas_base = Uniforms { model_matrix: mm_gas, view_matrix: view, proj_matrix: proj, screen_size, light_dir: gas_ld, base_color: Color::from_hex(0x000000), ambient: (ambient_for_planets*0.85).max(0.10), star_pos };
            render_planet(framebuffer, &u_gas_base, &self.sphere_arrays, PlanetShader::GasBase, false);
            render_planet_add(framebuffer, &u_gas_base, &self.sphere_arrays, PlanetShader::GasEddies, false);
            render_planet_add(framebuffer, &Uniforms{ ambient: if frozen { 0.0 } else { t_real }, ..u_gas_base.clone() }, &self.sphere_arrays, PlanetShader::GasHighClouds, true);
            render_planet_add(framebuffer, &u_gas_base, &self.sphere_arrays, PlanetShader::GasAurora, true);
        }

        // ===== GOLD GAS GIANT + RING =====
        self.gas_gold_trail.push(gas_gold_center);
        if self.gas_gold_trail.len() > self.max_trail_len { self.gas_gold_trail.remove(0); }
        let ring2_outer = self.gas_gold_scale * 1.05;
        let gas_gold_visible = allow_gas_gold && is_visible(gas_gold_center, ring2_outer);
        if gas_gold_visible {
            let gas_gold_rot = Vec3::new(0.0, -self.gas_gold_spin_speed * t_spin, 0.0);
            let mm_gas_gold = create_model_matrix(gas_gold_center, self.gas_gold_scale, gas_gold_rot);
            let gas_gold_ld = glm::normalize(&(gas_gold_center - star_pos));

            let tilt_x2 = 21.0_f32.to_radians();
            let tilt_z2 = -8.0_f32.to_radians();
            let ring_spin2 = 0.12 * t_spin;
            let ring2_scale = self.gas_gold_scale * 1.35;
            let ring2_rot = Vec3::new(tilt_x2, ring_spin2, tilt_z2);
            let mm_ring2 = create_model_matrix(gas_gold_center, ring2_scale, ring2_rot);
            let u_ring2 = Uniforms{ model_matrix: mm_ring2, view_matrix: view, proj_matrix: proj, screen_size, light_dir: gas_gold_ld, base_color: Color::from_hex(0x000000), ambient: 0.0, star_pos };
            render_planet(framebuffer, &u_ring2, &self.ring_arrays, PlanetShader::RingGoldBase, true);
            let mm_ring2_glow = create_model_matrix(gas_gold_center, ring2_scale * 1.05, ring2_rot);
            let u_ring2g = Uniforms { model_matrix: mm_ring2_glow, ..u_ring2 };
            render_planet_add(framebuffer, &u_ring2g, &self.ring_arrays, PlanetShader::RingGoldGlow, true);

            let u_gas_gold = Uniforms { model_matrix: mm_gas_gold, view_matrix: view, proj_matrix: proj, screen_size, light_dir: gas_gold_ld, base_color: Color::from_hex(0x000000), ambient: (ambient_for_planets*0.85).max(0.10), star_pos };
            render_planet(framebuffer, &u_gas_gold, &self.sphere_arrays, PlanetShader::GasGoldBase, false);
            render_planet_add(framebuffer, &u_gas_gold, &self.sphere_arrays, PlanetShader::GasGoldEddies, false);
            render_planet_add(framebuffer, &Uniforms{ ambient: if frozen { 0.0 } else { t_real }, ..u_gas_gold }, &self.sphere_arrays, PlanetShader::GasGoldHighClouds, true);
            render_planet_add(framebuffer, &u_gas_gold, &self.sphere_arrays, PlanetShader::GasGoldAurora, true);
        }

        // ===== TRAILS (only in All) =====
        if matches!(self.inspect, Inspect::All) {
            if rocky_visible    { self.draw_trail(framebuffer, &view, &proj, screen_size, &self.rocky_trail,    0x44AAFF); }
            if gas_visible      { self.draw_trail(framebuffer, &view, &proj, screen_size, &self.gas_trail,      0xFFAA44); }
            if lava_visible     { self.draw_trail(framebuffer, &view, &proj, screen_size, &self.lava_trail,     0xFF5A22); }
            if verdant_visible  { self.draw_trail(framebuffer, &view, &proj, screen_size, &self.verdant_trail,  0x4CFF7A); }
            if gas_gold_visible { self.draw_trail(framebuffer, &view, &proj, screen_size, &self.gas_gold_trail, 0xE8B84A); }
        }
        // ======= END: ported rendering =======
    }

    fn draw_trail(&self, fb: &mut Framebuffer, view: &Mat4, proj: &Mat4, screen: (f32,f32), pts: &Vec<Vec3>, color: u32) {
        if pts.len() < 2 { return; }
        const TRAIL_DEPTH: f32 = 0.999_999;
        let projected: Vec<Option<(isize,isize)>> = pts.par_iter().map(|wp| project_to_screen(view, proj, screen, *wp)).collect();
        let mut last: Option<(isize, isize)> = None;
        for pix_opt in projected {
            if let Some(pix) = pix_opt {
                if let Some((px, py)) = last {
                    draw_line_2d_depth(fb, px, py, pix.0, pix.1, color, TRAIL_DEPTH);
                }
                last = Some(pix);
            } else { last = None; }
        }
    }
}