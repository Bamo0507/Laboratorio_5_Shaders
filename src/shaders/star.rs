use nalgebra_glm as glm;
use glm::Vec3;
use crate::color::Color;
use super::util::*;

// ═══════════════════════════════════════════════════════════════════════════
// FIRST SHADER — CORE (opaque, light blue luminous base)
/// BLUE STAR — Core (single‑pass): light blue luminous base with subtle granulation
pub fn shade_core(pos_model: Vec3, n_view: Vec3, light_dir_view: Vec3, _ambient: f32, unlit: bool) -> Color {
    let dir = if glm::length(&pos_model) > 0.0 { glm::normalize(&pos_model) } else { Vec3::new(0.0, 1.0, 0.0) };

    // Palette: light luminous blues
    let c_base  = Color::from_hex(0x75C9FF); // light sky blue
    let c_glow = Color::from_hex(0xBEE9FF); 

    // Subtle cellular granulation (very low contrast)
    let w = fbm(dir * 2.0, 3);
    let g = fbm(Vec3::new(dir.y, dir.z, dir.x) * 3.1 + Vec3::new(w, w, w), 2);
    let t = (0.65 * w + 0.35 * g).clamp(0.0, 1.0);
    let mut c = mix(c_base, c_glow, 0.20 + 0.25 * t); // very gentle variation

    // Rim/center shaping to feel emissive
    let shade = if unlit { 1.0 } else { lambert(n_view, light_dir_view, 0.18) };
    let rim = rim_term(n_view, 2.2);
    c = mix(c, c_glow, 0.03 * rim);

    let (r, g, b) = color_to_f32(c);
    f32_to_color(r * shade, g * shade, b * shade)
}

// ═══════════════════════════════════════════════════════════════════════════
// SECOND SHADER — PLASMA BANDS (darker warped rings)
/// BLUE STAR — Darker, deformed circular bands around the sphere (additive but darker mix)
/// Reuses the former `plasma` slot to implement noisy warped rings that wrap the sphere.
pub fn shade_plasma(pos_model: Vec3, n_view: Vec3, light_dir_view: Vec3, _ambient: f32, unlit: bool) -> Color {
    let dir = if glm::length(&pos_model) > 0.0 { glm::normalize(&pos_model) } else { Vec3::new(0.0, 1.0, 0.0) };
    let (u, v) = spherical_uv(dir); // v ~ latitude (0..1), u ~ longitude (0..1)
    let tau = std::f32::consts::TAU;
    let uu = u * tau;

    // Darker blues for the bands
    let c_dark  = Color::from_hex(0x072A4F); // darker deep blue
    let c_deep  = Color::from_hex(0x01122A); // very dark navy

    // Warp the latitude coordinate so bands become deformed circles
    // Combine a few FBM signals and a longitunal sine warp to bend rings.
    let warp = 0.12 * (fbm(dir * 5.0, 3) - 0.5) + 0.08 * (uu * 3.0).sin();
    let vv = (v + warp).fract();

    // Build multiple ring families at different frequencies, then combine
    let ring1 = ((vv * 18.0).fract() - 0.5).abs(); // 18 bands
    let ring2 = (((vv * 9.0 + 0.25).fract()) - 0.5).abs(); // offset set
    let ring3 = (((vv * 5.0 - 0.15).fract()) - 0.5).abs();

    // Convert to soft masks (thickness varies with noise)
    let th1 = 0.18 + 0.10 * fbm(Vec3::new(dir.z, dir.x, dir.y) * 6.0, 2);
    let th2 = 0.22 + 0.08 * fbm(dir * 7.5, 2);
    let th3 = 0.26 + 0.06 * fbm(Vec3::new(-dir.y, dir.z, dir.x) * 9.0, 2);

    let m1 = smoothstep(th1, th1 * 0.5, ring1);
    let m2 = smoothstep(th2, th2 * 0.5, ring2);
    let m3 = smoothstep(th3, th3 * 0.5, ring3);

    let band_mix = (0.55 * m1 + 0.30 * m2 + 0.15 * m3).clamp(0.0, 1.0);
    let mut c = mix(c_dark, c_deep, 0.35 + 0.50 * band_mix);

    // Slight illumination shaping so it sits under highlights
    let shade = if unlit { 1.0 } else { lambert(n_view, light_dir_view, 0.15) };
    let (r, g, b) = color_to_f32(c);
    // Premultiply results lightly (acts like a darkening overlay)
    f32_to_color(r * shade * 0.65, g * shade * 0.65, b * shade * 0.75)
}

// ═══════════════════════════════════════════════════════════════════════════
// THIRD SHADER — CORONA (additive rim/streamers)
/// BLUE STAR — Corona shell (additive): long streamers and rim glow
pub fn shade_corona(pos_model: Vec3, n_view: Vec3, light_dir_view: Vec3, _ambient: f32, unlit: bool) -> Color {
    let dir = if glm::length(&pos_model) > 0.0 { glm::normalize(&pos_model) } else { Vec3::new(0.0, 1.0, 0.0) };
    let (u, v) = spherical_uv(dir);
    let tau = std::f32::consts::TAU;
    let uu = u * tau;

    let base = Color::from_hex(0x274B7A);   // deeper navy-azure base
    let hi   = Color::from_hex(0x76A7D8);   // dimmer light blue

    let spokes  = ((64.0 * uu).sin().abs()).powf(1.4);
    let height  = (1.0 - v).clamp(0.0, 1.0);
    let flicker = fbm(dir * 7.0, 3);
    let t = (0.30 * (0.6 * flicker + 0.4 * spokes) * height).clamp(0.0, 1.0);
    let c = mix(base, hi, t);

    let shade = if unlit { 1.0 } else { lambert(n_view, light_dir_view, 0.20) };
    let rim_geom = rim_term(n_view, 2.8);
    let (r, g, b) = color_to_f32(c);
    let intensity = 0.05 * rim_geom;  // softer corona
    f32_to_color(r * shade * intensity, g * shade * intensity, b * shade * intensity)
}

// ═══════════════════════════════════════════════════════════════════════════
// FOURTH SHADER — FILAMENTS (additive thin cyan arcs)
/// BLUE STAR — Thin cyan filaments/arcs (additive): meandering lines that hug the rim
pub fn shade_filaments(pos_model: Vec3, n_view: Vec3, light_dir_view: Vec3, _ambient: f32, unlit: bool) -> Color {
    let dir = if glm::length(&pos_model) > 0.0 { glm::normalize(&pos_model) } else { Vec3::new(0.0, 1.0, 0.0) };
    let c_lo = Color::from_hex(0x1E4E86);   // deeper filament low
    let c_hi = Color::from_hex(0x5B8BB6);   // darker medium high

    // Flow direction from gradient
    let e = 0.2;
    let dx = snoise(dir * 8.0 + Vec3::new(e, 0.0, 0.0)) - snoise(dir * 8.0 - Vec3::new(e, 0.0, 0.0));
    let dy = snoise(dir * 8.0 + Vec3::new(0.0, e, 0.0)) - snoise(dir * 8.0 - Vec3::new(0.0, e, 0.0));
    let dz = snoise(dir * 8.0 + Vec3::new(0.0, 0.0, e)) - snoise(dir * 8.0 - Vec3::new(0.0, 0.0, e));
    let grad = Vec3::new(dx, dy, dz);
    let mut tang = grad - dir * glm::dot(&grad, &dir);
    if glm::length(&tang) < 1e-4 { tang = glm::cross(&dir, &Vec3::new(0.0,1.0,0.0)); }
    tang = glm::normalize(&tang);

    let phase = glm::dot(&(dir * 5.5), &tang) + fbm(dir * 10.0, 2) * 1.5;
    let stripe = (phase.sin().abs()).powf(7.0); // very thin filaments
    let rim = rim_term(n_view, 2.2);

    let c = mix(c_lo, c_hi, 0.6);
    let (r,g,b) = color_to_f32(c);
    let intensity = 0.12 * rim * stripe; // slightly dimmer so hotspots pop
    f32_to_color(r * intensity, g * intensity, b * intensity)
}

// ═══════════════════════════════════════════════════════════════════════════
// FOURTH SHADER — Hotspots/speckles (opaque overlay, high-contrast, rim-gated)
// Small dark blue blobs overlay the core, clearly visible even over the bright center.
pub fn shade_hotspots(pos_model: Vec3, n_view: Vec3, _light_dir_view: Vec3, _time: f32, _unlit: bool) -> Color {
    let dir = if glm::length(&pos_model) > 0.0 { glm::normalize(&pos_model) } else { Vec3::new(0.0,1.0,0.0) };

    // Rebuild the core look (non‑hotspot pixels must match the core exactly)
    let c_base  = Color::from_hex(0x75C9FF);
    let c_glow  = Color::from_hex(0xBEE9FF);
    let w = fbm(dir * 2.0, 3);
    let g = fbm(Vec3::new(dir.y, dir.z, dir.x) * 3.1 + Vec3::new(w, w, w), 2);
    let t = (0.65 * w + 0.35 * g).clamp(0.0, 1.0);
    let core = mix(c_base, c_glow, 0.20 + 0.25 * t);

    // Much darker blue to mix toward (not black, keeps star hue)
    let c_dark = Color::from_hex(0x00132A); // deeper blue target for strong contrast

    // Large blobs + some detail; lower frequencies for readability
    let n_lo = fbm(dir * 1.9, 3);  // larger, clearer blobs
    let n_hi = fbm(dir * 6.5, 3);  // keep some detail

    // Wider thresholds so areas are clearly visible
    let m_lo = smoothstep(0.42, 0.68, n_lo); // widen coverage
    let m_hi = smoothstep(0.64, 0.82, n_hi);

    // Favor the rim slightly but never hide at the center
    let rim = rim_term(n_view, 2.0);
    let rim_gate = 0.30 + 0.70 * rim;     // minimum 30% visibility at center

    let mask = (0.70 * m_lo + 0.30 * m_hi) * rim_gate;

    // Strong darkening so it reads over a very bright core
    let amount = 0.92;                    // stronger darkening
    let k = (amount * mask).clamp(0.0, 1.0);

    mix(core, c_dark, k)
}