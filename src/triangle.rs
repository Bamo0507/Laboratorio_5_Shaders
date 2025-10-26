use nalgebra_glm as glm;
use nalgebra_glm::{Vec3, Vec4};

use crate::fragment::Fragment;
use crate::vertex::Vertex;
use crate::shaders::{PlanetShader, shade_planet};
use crate::Uniforms;
use crate::color::Color;

const EPS: f32 = 1e-4; // bias to keep shared-edge pixels and avoid dark seams

// Signed area helper (edge function)
#[inline]
fn edge(ax: f32, ay: f32, bx: f32, by: f32, px: f32, py: f32) -> f32 {
    (px - ax) * (by - ay) - (py - ay) * (bx - ax)
}

#[inline]
fn is_top_left(ax: f32, ay: f32, bx: f32, by: f32) -> bool {
    let dy = by - ay;
    if dy > 0.0 {
        true
    } else if dy < 0.0 {
        false
    } else {
        (bx - ax) < 0.0
    }
}

pub fn triangle(
    v0: &Vertex,
    v1: &Vertex,
    v2: &Vertex,
    _light_dir: Vec3,   // unused: directional light (ignored in unlit mode)
    base: Color,        // base color for the ship
    _ambient: f32,      // unused
) -> Vec<Fragment> {
    let p0 = v0.transformed_position;
    let p1 = v1.transformed_position;
    let p2 = v2.transformed_position;

    // Degenerate? (area ~ 0) → nothing to draw
    let area = edge(p0.x, p0.y, p1.x, p1.y, p2.x, p2.y);
    if area.abs() < f32::EPSILON { return Vec::new(); }

    // Precompute top-left flags for each edge (p1->p2, p2->p0, p0->p1)
    let tl0 = is_top_left(p1.x, p1.y, p2.x, p2.y);
    let tl1 = is_top_left(p2.x, p2.y, p0.x, p0.y);
    let tl2 = is_top_left(p0.x, p0.y, p1.x, p1.y);

    // We'll make the area positive (so inside means w >= 0 on TL edges, > 0 otherwise)
    let sign = if area > 0.0 { 1.0 } else { -1.0 };

    // UNLIT: use the provided base color directly (no Lambert / rim / vertex tint)
    let shaded = base;

    // Bounding box (in screen space)
    let min_x = p0.x.min(p1.x).min(p2.x).floor() as i32;
    let max_x = p0.x.max(p1.x).max(p2.x).ceil() as i32;
    let min_y = p0.y.min(p1.y).min(p2.y).floor() as i32;
    let max_y = p0.y.max(p1.y).max(p2.y).ceil() as i32;

    let mut frags = Vec::new();

    for y in min_y..=max_y {
        for x in min_x..=max_x {
            let px = x as f32 + 0.5;
            let py = y as f32 + 0.5;

            // Edge function values, oriented so that inside is >= 0 on TL edges, > 0 otherwise
            let mut w0 = edge(p1.x, p1.y, p2.x, p2.y, px, py) * sign;
            let mut w1 = edge(p2.x, p2.y, p0.x, p0.y, px, py) * sign;
            let mut w2 = edge(p0.x, p0.y, p1.x, p1.y, px, py) * sign;

            // Top-left rule: include pixels on TL edges, exclude on others, with epsilon bias
            let e0 = if tl0 { w0 >= -EPS } else { w0 > -EPS };
            let e1 = if tl1 { w1 >= -EPS } else { w1 > -EPS };
            let e2 = if tl2 { w2 >= -EPS } else { w2 > -EPS };

            if e0 && e1 && e2 {
                // Interpolate z for depth test
                let inv_area = 1.0 / (area.abs());
                w0 *= inv_area; w1 *= inv_area; w2 *= inv_area;
                let z = w0 * p0.z + w1 * p1.z + w2 * p2.z;

                frags.push(Fragment::new(px, py, shaded, z));
            }
        }
    }

    frags
}

/// Triangle rasterizer that shades each fragment using a selected planet shader.
pub fn triangle_planet(
    v0: &Vertex,
    v1: &Vertex,
    v2: &Vertex,
    uniforms: &Uniforms,
    shader: PlanetShader,
    unlit: bool,
) -> Vec<Fragment> {
    let p0 = v0.transformed_position;
    let p1 = v1.transformed_position;
    let p2 = v2.transformed_position;

    let area = edge(p0.x, p0.y, p1.x, p1.y, p2.x, p2.y);
    if area.abs() < f32::EPSILON { return Vec::new(); }

    // Precompute top-left flags for each edge (p1->p2, p2->p0, p0->p1)
    let tl0 = is_top_left(p1.x, p1.y, p2.x, p2.y);
    let tl1 = is_top_left(p2.x, p2.y, p0.x, p0.y);
    let tl2 = is_top_left(p0.x, p0.y, p1.x, p1.y);

    // Normalize orientation
    let sign = if area > 0.0 { 1.0 } else { -1.0 };

    // Bounding box
    let min_x = p0.x.min(p1.x).min(p2.x).floor() as i32;
    let max_x = p0.x.max(p1.x).max(p2.x).ceil() as i32;
    let min_y = p0.y.min(p1.y).min(p2.y).floor() as i32;
    let max_y = p0.y.max(p1.y).max(p2.y).ceil() as i32;

    let mut frags = Vec::new();

    for y in min_y..=max_y {
        for x in min_x..=max_x {
            let px = x as f32 + 0.5;
            let py = y as f32 + 0.5;

            let mut w0 = edge(p1.x, p1.y, p2.x, p2.y, px, py) * sign;
            let mut w1 = edge(p2.x, p2.y, p0.x, p0.y, px, py) * sign;
            let mut w2 = edge(p0.x, p0.y, p1.x, p1.y, px, py) * sign;

            let e0 = if tl0 { w0 >= -EPS } else { w0 > -EPS };
            let e1 = if tl1 { w1 >= -EPS } else { w1 > -EPS };
            let e2 = if tl2 { w2 >= -EPS } else { w2 > -EPS };

            if !(e0 && e1 && e2) { continue; }

            // Barycentric weights normalized to area
            let inv_area = 1.0 / area.abs();
            w0 *= inv_area; w1 *= inv_area; w2 *= inv_area;

            // Interpolate z for depth test
            let z = w0 * p0.z + w1 * p1.z + w2 * p2.z;

            // Interpolate model-space position (for spherical patterns)
            let pos_model = v0.position * w0 + v1.position * w1 + v2.position * w2;

            // Interpolate view-space normal (then renormalize)
            let mut n = v0.transformed_normal * w0 + v1.transformed_normal * w1 + v2.transformed_normal * w2;
            if glm::length(&n) > 0.0 { n = glm::normalize(&n); }

            // Per-fragment lighting from star position (compute in WORLD space)
            let pw: Vec4 = uniforms.model_matrix * Vec4::new(pos_model.x, pos_model.y, pos_model.z, 1.0);
            let pos_world = Vec3::new(pw.x, pw.y, pw.z);
            // light_dir_view should be FROM light to scene; we pass (light->frag)
            let light_dir = glm::normalize(&(pos_world - uniforms.star_pos));
            let mut col = shade_planet(shader, pos_model, n, light_dir, uniforms.ambient, unlit);

            // --- Star light intensity boost with distance attenuation ---
            // Tune 400.0 to your scene scale; larger = slower falloff
            let d = glm::distance(&pos_world, &uniforms.star_pos);
            let atten = (1.0 / (1.0 + (d / 400.0).powf(2.0))).clamp(0.2, 1.0);
            let boost = 0.8 + 1.4 * atten; // near star up to ~2.2x

            let h = col.to_hex();
            let (mut r, mut g, mut b) = (
                ((h >> 16) & 0xFF) as f32 / 255.0,
                ((h >>  8) & 0xFF) as f32 / 255.0,
                ( h        & 0xFF) as f32 / 255.0,
            );
            r = (r * boost).clamp(0.0, 1.0);
            g = (g * boost).clamp(0.0, 1.0);
            b = (b * boost).clamp(0.0, 1.0);
            let col = Color::from_float(r, g, b);

            frags.push(Fragment::new(px, py, col, z));
        }
    }

    frags
}