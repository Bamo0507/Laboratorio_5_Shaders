use nalgebra_glm as glm;
use glm::Vec3;
use crate::color::Color;

#[inline]
pub fn color_to_f32(c: Color) -> (f32, f32, f32) {
    let h = c.to_hex();
    (
        ((h >> 16) & 0xFF) as f32 / 255.0,
        ((h >> 8)  & 0xFF) as f32 / 255.0,
        ( h        & 0xFF) as f32 / 255.0,
    )
}
#[inline]
pub fn f32_to_color(r: f32, g: f32, b: f32) -> Color {
    Color::from_float(r.clamp(0.0,1.0), g.clamp(0.0,1.0), b.clamp(0.0,1.0))
}
#[inline]
pub fn mix(c1: Color, c2: Color, t: f32) -> Color {
    let (r1,g1,b1) = color_to_f32(c1);
    let (r2,g2,b2) = color_to_f32(c2);
    f32_to_color(
        r1 + (r2 - r1) * t,
        g1 + (g2 - g1) * t,
        b1 + (b2 - b1) * t
    )
}
#[inline]
pub fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}
#[inline]
pub fn snoise(p: Vec3) -> f32 {
    let s1 = (glm::dot(&p, &Vec3::new(12.9898, 78.233, 37.719)).sin() * 0.5 + 0.5);
    let s2 = (glm::dot(&p, &Vec3::new( 4.8980,  7.230,  3.456)).cos() * 0.5 + 0.5);
    let s3 = (glm::dot(&p, &Vec3::new( 9.2330, 12.110,  5.678)).sin() * 0.5 + 0.5);
    s1 * 0.5 + s2 * 0.35 + s3 * 0.15
}
#[inline]
pub fn fbm(mut p: Vec3, octaves: i32) -> f32 {
    let mut acc = 0.0;
    let mut amp = 0.5;
    let mut freq = 1.0;
    for _ in 0..octaves {
        acc += amp * snoise(p * freq);
        freq *= 2.0;
        amp *= 0.5;
    }
    acc
}
#[inline]
pub fn spherical_uv(n: Vec3) -> (f32, f32) {
    let theta = n.z.atan2(n.x);                   // [-pi, pi]
    let phi   = n.y.acos();                       // [0, pi]
    let u = 0.5 + theta / std::f32::consts::TAU;  // [0,1]
    let v =       phi   / std::f32::consts::PI;   // [0,1]
    (u, v)
}
#[inline]
pub fn lambert(n_view: Vec3, light_dir_view: Vec3, ambient: f32) -> f32 {
    let l = -light_dir_view;
    (ambient + glm::dot(&n_view, &l).max(0.0)).clamp(0.0, 1.0)
}
#[inline]
pub fn rim_term(n_view: Vec3, power: f32) -> f32 {
    (1.0 - n_view.z.abs().clamp(0.0, 1.0)).powf(power).clamp(0.0, 1.0)
}