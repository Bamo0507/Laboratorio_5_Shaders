#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Inspect {
    All,
    Star,
    Rocky,
    Gas,
    Lava,
    Verdant,
    GasGold,
}
use minifb::{Key, Window, WindowOptions};
use std::time::Duration;

mod framebuffer;
mod triangle;
mod vertex;
mod obj;
mod color;
mod skybox;
mod fragment;
mod shaders;
mod camera;
mod app;

use crate::app::App;

use nalgebra_glm::{Mat4, Vec3};
use framebuffer::Framebuffer;
use color::Color;

#[derive(Clone)]
pub struct Uniforms {
    pub model_matrix: Mat4,
    pub view_matrix: Mat4,
    pub proj_matrix: Mat4,
    pub screen_size: (f32, f32),

    pub light_dir: Vec3,
    pub base_color: Color,
    pub ambient: f32,
    pub star_pos: Vec3,
}

fn main() {
    // Window / framebuffer
    let window_width = 1280;
    let window_height = 720;
    let framebuffer_width = window_width;
    let framebuffer_height = window_height;
    let frame_delay = Duration::from_millis(16);

    let mut framebuffer = Framebuffer::new(framebuffer_width, framebuffer_height);
    let mut window = Window::new(
        "Solar System",
        window_width,
        window_height,
        WindowOptions::default(),
    ).unwrap();

    window.set_position(500, 500);
    framebuffer.set_background_color(0x333355);

    // App holds all scene state & rendering
    let mut app = App::new();

    while window.is_open() {
        if window.is_key_down(Key::Escape) { break; }

        framebuffer.clear();
        app.frame(&mut framebuffer, &window);

        window
            .update_with_buffer(&framebuffer.buffer, framebuffer_width, framebuffer_height)
            .unwrap();

        std::thread::sleep(frame_delay);
    }
}