use nalgebra_glm as glm;
use glm::Vec3;
use crate::color::Color;
use super::util::*;



/* ───────────────────────────────────────────────────────────────────────────
   GAS GIANT — 4 LAYER SHADER SET (creative, iridescent)
   L1: shade_gas_base        (opaque)      — saturated domain‑warped bands
   L2: shade_gas_eddies      (additive)    — magenta/teal swirls crossing bands
   L3: shade_gas_highclouds  (additive)    — bright white/peach wisps (time‑animated)
   L4: shade_gas_aurora      (additive)    — polar teal↔violet curtains
   We pass time via `ambient` to the HighClouds layer.
   ─────────────────────────────────────────────────────────────────────────── */

#[inline] fn cg(hex: u32) -> Color { Color::from_hex(hex) }

pub fn shade_gas_base(pos_model: Vec3, n_view: Vec3, light_dir_view: Vec3, ambient: f32, unlit: bool) -> Color {
    let dir = if glm::length(&pos_model) > 0.0 { glm::normalize(&pos_model) } else { Vec3::new(0.0,1.0,0.0) };
    let (u, v) = spherical_uv(dir);
    let tau = std::f32::consts::TAU;
    let uu = u * tau;
    let lat = 1.0 - 2.0 * v; // [-1,1]

    // Creative palette (cool ↔ warm ↔ vivid)
    let cool_lo = cg(0x0E6BD8); // royal blue
    let cool_hi = cg(0x7FE9FF); // cyan
    let warm_lo = cg(0xF6B07A); // peach
    let warm_hi = cg(0xF39DD4); // pink
    let vivid_v = cg(0xA66BFF); // violet accent

    // Domain warp to keep band edges organic
    let w1 = fbm(Vec3::new(dir.y, dir.z, dir.x) * 3.2, 3) * 0.45;
    let w2 = fbm(Vec3::new(-dir.z, dir.x, dir.y) * 6.0, 2) * 0.25;
    let warp = w1 + w2 + 0.05 * (uu * 8.0).sin();

    // Base latitudinal bands, curved by warp
    let bands = (lat * 8.0 + 1.8 * (20.0 * dir.x).sin() + 3.0 * warp).sin() * 0.5 + 0.5;
    let mut c = mix(cool_lo, warm_lo, bands);
    c = mix(c, cool_hi, 0.35 * (0.5 + 0.5 * (uu * 1.3 + lat * 4.0).sin()));
    c = mix(c, warm_hi, 0.25 * (0.5 + 0.5 * (uu * 1.7 - lat * 3.2).cos()));
    c = mix(c, vivid_v, 0.10 * (0.5 + 0.5 * (uu * 3.4 + lat * 6.0).sin()));

    let shade = if unlit { 1.0 } else { lambert(n_view, light_dir_view, (ambient * 0.85).max(0.10)) };
    let (r,g,b) = color_to_f32(c);
    f32_to_color(r*shade, g*shade, b*shade)
}

pub fn shade_gas_eddies(pos_model: Vec3, n_view: Vec3, light_dir_view: Vec3, _ambient: f32, _unlit: bool) -> Color {
    let dir = if glm::length(&pos_model) > 0.0 { glm::normalize(&pos_model) } else { Vec3::new(0.0,1.0,0.0) };
    let (u, v) = spherical_uv(dir);
    let tau = std::f32::consts::TAU;
    let uu = u * tau;

    // Cross‑band swirl pattern
    let s = ((6.0 * uu + 18.0 * v).sin() * (12.0 * uu + 5.0 * v).sin()) * 0.5 + 0.5;
    let n = fbm(dir * 9.0, 3);
    let mask = smoothstep(0.50, 0.80, 0.6*s + 0.4*n);

    // Saturated eddy color that works as additive tint
    let ed_lo = cg(0x18F0FF); // teal
    let ed_hi = cg(0xED57FF); // magenta
    let c = mix(ed_lo, ed_hi, 0.5 + 0.5 * (uu * 0.7 + v * 4.0).sin());
    let (r,g,b) = color_to_f32(c);
    let k = 0.22 * mask * (0.35 + 0.65 * glm::dot(&n_view, &(-light_dir_view)).max(0.0));
    f32_to_color(r*k, g*k, b*k)
}

pub fn shade_gas_highclouds(pos_model: Vec3, n_view: Vec3, light_dir_view: Vec3, time: f32, _unlit: bool) -> Color {
    let dir = if glm::length(&pos_model) > 0.0 { glm::normalize(&pos_model) } else { Vec3::new(0.0,1.0,0.0) };
    let p = dir * 12.0 + Vec3::new(0.10*time, 0.07*time, -0.06*time);
    let n = fbm(p, 4);
    let mask = smoothstep(0.62, 0.74, n);

    let l = -light_dir_view;
    let sun = glm::dot(&n_view, &l).clamp(0.0, 1.0);

    let wh = cg(0xFFFFFF);
    let pe = cg(0xFFD4B2);
    let c = mix(wh, pe, 0.20 * (1.0 - sun));
    let (r,g,b) = color_to_f32(c);
    let intensity = 0.58 * mask * (0.45 + 0.55 * sun) + 0.15 * rim_term(n_view, 2.0) * mask;
    f32_to_color(r*intensity, g*intensity, b*intensity)
}

pub fn shade_gas_aurora(pos_model: Vec3, n_view: Vec3, light_dir_view: Vec3, _ambient: f32, _unlit: bool) -> Color {
    let dir = if glm::length(&pos_model) > 0.0 { glm::normalize(&pos_model) } else { Vec3::new(0.0,1.0,0.0) };
    let (_u, v) = spherical_uv(dir);
    let lat = 1.0 - 2.0 * v;

    // Polar mask
    let poles = smoothstep(0.65, 0.90, lat.abs());
    let wav   = (glm::dot(&dir, &glm::normalize(&Vec3::new(0.2,1.0,0.1))) * 24.0).sin().abs();
    let mask  = (0.6 * poles + 0.4 * wav.powf(6.0)) * 0.9;

    let a_lo = cg(0x35FFE6); // teal
    let a_hi = cg(0xB575FF); // violet
    let c = mix(a_lo, a_hi, 0.5 + 0.5 * (dir.x * 6.0 + dir.z * 5.0).sin());
    let (r,g,b) = color_to_f32(c);
    let k = 0.20 * mask * (0.25 + 0.75 * (1.0 - glm::dot(&n_view, &(-light_dir_view)).clamp(0.0,1.0)));
    f32_to_color(r*k, g*k, b*k)
}
/* ───────────────────────────────────────────────────────────────────────────
   EARTH‑LIKE PLANET — 4 LAYER SHADER SET
   L1: Ocean Base (opaque)
   L2: Land Albedo (additive tint mask)
   L3: Clouds (additive, time‑animated)
   L4: Night Lights (additive on night side)
   We pass time via `ambient` if needed (e.g., clouds).
   ─────────────────────────────────────────────────────────────────────────── */

#[inline] fn cc(hex: u32) -> Color { Color::from_hex(hex) }

// Palette
const OCEAN_DEEP:   u32 = 0x0B3D91;
const OCEAN_SHALLOW:u32 = 0x1E90FF;
const SEA_FOAM:     u32 = 0xB8E7FF;
const ICE_WHITE:    u32 = 0xE8F6FF;

const LAND_GREEN:   u32 = 0x2E8B57;
const LAND_TAN:     u32 = 0xC2B280;
const LAND_BROWN:   u32 = 0x7A5C43;
const SNOW_CAP:     u32 = 0xF4FAFF;

const CLOUD_WHITE:  u32 = 0xF3F8FF;
const CLOUD_SHADE:  u32 = 0xC8D9EA;


const CITY_LIGHT:   u32 = 0xFFC857;

// ── Helpers for Earth‑like continents (spherical blobs + noise warping)
#[inline] fn deg2rad(d: f32) -> f32 { d.to_radians() }
#[inline] fn sph_from_latlon(lat_deg: f32, lon_deg: f32) -> Vec3 {
    let lat = deg2rad(lat_deg);
    let lon = deg2rad(lon_deg);
    let x = lat.cos() * lon.cos();
    let y = lat.sin();
    let z = lat.cos() * lon.sin();
    glm::normalize(&Vec3::new(x, y, z))
}
#[inline] fn gauss_blob(dir: Vec3, lat: f32, lon: f32, width_deg: f32, weight: f32) -> f32 {
    let c = sph_from_latlon(lat, lon);
    let ang = glm::dot(&dir, &c).clamp(-1.0, 1.0).acos();
    let w = deg2rad(width_deg).max(1e-3);
    (-(ang / w).powf(2.0)).exp() * weight
}

/// Returns (land_mask, coast_band) both in 0..1.
/// land_mask is a fairly wide threshold; coast_band is a thin ring for shores.
fn continent_mask(dir: Vec3) -> (f32, f32) {
    // Coarse “continents” via a few anchored blobs
    let mut raw = 0.0;
    raw += gauss_blob(dir,  15.0,  -80.0, 45.0, 1.10); // Americas
    raw += gauss_blob(dir,  20.0,   20.0, 55.0, 1.05); // Africa/Europe
    raw += gauss_blob(dir,  45.0,   90.0, 60.0, 0.95); // Asia
    raw += gauss_blob(dir, -25.0,  135.0, 30.0, 0.70); // Australia
    raw += gauss_blob(dir,  72.0,  -42.0, 22.0, 0.55); // Greenland
    // Add some archipelago hints
    raw += gauss_blob(dir,  35.0,  140.0, 12.0, 0.35); // Japan
    raw += gauss_blob(dir,  14.0,  121.0, 16.0, 0.30); // Philippines/SEA
    raw += gauss_blob(dir, -40.0,   15.0, 18.0, 0.25); // Southern Africa

    // Noise warp to carve believable coastlines
    let warp = 0.25 * (fbm(dir * 8.0, 3) - 0.5)
             + 0.12 * (fbm(Vec3::new(dir.y, dir.z, dir.x) * 18.0, 2) - 0.5);
    let x = raw + warp - 0.35; // bias to balance ocean/land ratio

    let land = smoothstep(0.48, 0.52, x);
    // Thin coast band around the threshold (~couple of pixels wide from orbit)
    let edge = (x - 0.50).abs();
    let coast = (1.0 - (edge / 0.03).clamp(0.0, 1.0)).clamp(0.0, 1.0);
    (land, coast)
}

pub fn shade_earth_ocean(pos_model: Vec3, n_view: Vec3, light_dir_view: Vec3, ambient: f32, unlit: bool) -> Color {
    let dir = if glm::length(&pos_model) > 0.0 { glm::normalize(&pos_model) } else { Vec3::new(0.0,1.0,0.0) };
    let (_u, v) = spherical_uv(dir);
    let lat = 1.0 - 2.0 * v; // [-1,1]

    // Lighting terms
    let l = -light_dir_view;
    let sun = glm::dot(&n_view, &l).clamp(0.0, 1.0);

    // Base ocean color with subtle depth variation and polar ice tint
    let depth = (0.5 + 0.5 * fbm(dir * 2.5, 3)).clamp(0.0, 1.0);
    let mut ocean = mix(cc(OCEAN_DEEP), cc(OCEAN_SHALLOW), 0.20 + 0.70 * depth);
    let polar = smoothstep(0.55, 0.82, lat.abs());
    ocean = mix(ocean, cc(ICE_WHITE), 0.18 * polar);

    // Continents mask (and a thin coast ring)
    let (land_m, coast) = continent_mask(dir);

    // Land colors by latitude dryness + mountains & snow caps
    let elev = fbm(dir * 5.5, 3);
    let mountains = smoothstep(0.58, 0.78, elev);
    let dryness = smoothstep(0.20, 0.85, lat.abs());
    let base_land = mix(cc(LAND_GREEN), cc(LAND_TAN), 0.30 + 0.65 * dryness);
    let mut land_col = mix(base_land, cc(LAND_BROWN), 0.35 * mountains);
    let snow = smoothstep(0.65, 0.90, lat.abs()) * mountains;
    land_col = mix(land_col, cc(SNOW_CAP), 0.65 * snow);

    // Compose base: where land_m≈1 use land, else ocean
    let mut c = mix(ocean, land_col, land_m);

    // Shore foam/glint (helps read coastlines from orbit)
    let foam_int = 0.14 * coast * (0.40 + 0.60 * sun);
    c = mix(c, cc(SEA_FOAM), foam_int);

    let shade = if unlit { 1.0 } else { lambert(n_view, light_dir_view, (ambient * 0.45 + 0.15).min(0.40)) };
    let (r,g,b) = color_to_f32(c);
    f32_to_color(r*shade, g*shade, b*shade)
}

pub fn shade_earth_land(pos_model: Vec3, n_view: Vec3, light_dir_view: Vec3, _ambient: f32, _unlit: bool) -> Color {
    let dir = if glm::length(&pos_model) > 0.0 { glm::normalize(&pos_model) } else { Vec3::new(0.0,1.0,0.0) };
    let (_u, v) = spherical_uv(dir);
    let lat = 1.0 - 2.0 * v;
    let (land_m, coast) = continent_mask(dir);

    // Accent color (greener tropics, browner high latitudes)
    let dryness = smoothstep(0.25, 0.85, lat.abs());
    let accent = mix(cc(LAND_GREEN), cc(LAND_BROWN), dryness);

    // Sun factor so accents pop on day side; add extra at coasts
    let l = -light_dir_view;
    let sun = glm::dot(&n_view, &l).clamp(0.0, 1.0);
    let coast_boost = 0.35 * coast; // brighten beaches/shorelines

    let (r,g,b) = color_to_f32(accent);
    let k = (0.18 * land_m + coast_boost) * (0.35 + 0.65 * sun);
    f32_to_color(r*k, g*k, b*k)
}

pub fn shade_earth_clouds(pos_model: Vec3, n_view: Vec3, light_dir_view: Vec3, time: f32, _unlit: bool) -> Color {
    let dir = if glm::length(&pos_model) > 0.0 { glm::normalize(&pos_model) } else { Vec3::new(0.0,1.0,0.0) };
    // Sphere to UV (wrap-aware)
    let (mut u, mut v) = spherical_uv(dir);
    // Advect clouds slowly over time (wind)
    let wind_u = 0.025 * time;
    u = (u + wind_u).fract();

    // Sun / rim terms
    let rim = rim_term(n_view, 2.0);
    let l = -light_dir_view;
    let sun = glm::dot(&n_view, &l).clamp(0.0, 1.0);

    // Helper: tiny hash for jitter (deterministic)
    #[inline]
    fn hash21(p: (i32, i32)) -> f32 {
        let x = p.0 as f32;
        let y = p.1 as f32;
        let s = (x * 127.1 + y * 311.7).sin() * 43758.5453;
        s.fract()
    }

    // Distance on a torus (wrap-aware 0..1 domain)
    #[inline]
    fn wrap_dist(a: f32, b: f32) -> f32 {
        let d = (a - b).abs();
        d.min(1.0 - d)
    }

    // Build blobby clouds from a sum of soft discs at multiple scales.
    // We use a few grids of different resolutions, jittering disc centers and radii.
    let mut mask = 0.0f32;

    // Each entry: (cells_u, cells_v, base_radius, radius_jitter, opacity)
    let layers = [
        (8, 4,  0.075f32, 0.024f32, 0.55f32),  // large puffs (fewer/softer)
        (16, 8, 0.038f32, 0.015f32, 0.60f32),  // medium (smaller/softer)
        (32, 16,0.018f32, 0.008f32, 0.65f32),  // fine wisps (lighter)
    ];

    for (cu, cv, r0, rj, op) in layers {
        // Find the current UV's integer cell
        let iu = (u * cu as f32).floor() as i32;
        let iv = (v * cv as f32).floor() as i32;

        // Consider a 3x3 neighborhood of cells so discs from neighbors can cover this fragment
        for du in -1..=1 {
            for dv in -1..=1 {
                let cu_i = iu + du;
                let cv_i = iv + dv;

                // Jitter center inside the cell
                let j1 = hash21((cu_i * 9283 + 57, cv_i * 6899 + 101));
                let j2 = hash21((cu_i * 2311 + 73,  cv_i * 9157 + 19));
                let cx = (cu_i as f32 + 0.5 + (j1 - 0.5) * 0.7) / cu as f32;
                let cy = (cv_i as f32 + 0.5 + (j2 - 0.5) * 0.7) / cv as f32;

                // Radius jitter
                let rj_hash = hash21((cu_i * 4133 + 11, cv_i * 3769 + 29));
                let r = (r0 + rj * (rj_hash - 0.5)).max(0.005);

                // Wrap-aware distance
                let dx = wrap_dist(u, cx);
                let dy = wrap_dist(v, cy);
                let d = (dx*dx + dy*dy).sqrt();

                // Soft disc falloff (inner solid + soft edge)
                let inner = r * 0.65;
                let m = smoothstep(r, inner, d); // 0 at r .. 1 at inner

                // Slight height bias (make cloud tops brighter towards sub-solar)
                let tilt = 0.75 + 0.25 * sun;

                // Accumulate with capped add to keep things soft
                mask = (mask + m * op * tilt).min(1.0);
            }
        }
    }

    // Global coverage control — lower = fewer clouds (0..1)
    let density = 0.58f32;
    mask = (mask * density).min(1.0);

    // Add gentle erosion so silhouettes aren’t perfectly round
    // Use fbm to nibble at the mask edge
    let erosion = fbm(dir * 9.0 + Vec3::new(0.1*time, 0.07*time, -0.05*time), 3);
    let eroded = smoothstep(0.40, 0.60, mask - 0.22 * (erosion - 0.5));

    // Compose color: white with a cool shadow tint away from sun
    let c = mix(cc(CLOUD_WHITE), cc(CLOUD_SHADE), 0.30 * (1.0 - sun));
    let (r,g,b) = color_to_f32(c);

    // Final intensity: strong enough to be clearly visible, but still additive
    let intensity = 0.68 * eroded * (0.45 + 0.55 * sun) + 0.16 * rim * eroded;

    f32_to_color(r * intensity, g * intensity, b * intensity)
}

pub fn shade_earth_night(pos_model: Vec3, n_view: Vec3, light_dir_view: Vec3, time: f32, _unlit: bool) -> Color {
    let dir = if glm::length(&pos_model) > 0.0 { glm::normalize(&pos_model) } else { Vec3::new(0.0,1.0,0.0) };
    let (land_m, _coast) = continent_mask(dir);

    // Night mask from lighting
    let l = -light_dir_view;
    let day = glm::dot(&n_view, &l).clamp(0.0, 1.0);
    let night = (1.0 - day).powf(2.0);

    // Base density noise with slight twinkle
    let density = fbm(dir * 18.0 + Vec3::new(0.10*time, -0.07*time, 0.05*time), 3);
    let noise_cities = smoothstep(0.72, 0.86, density);

    // Add a few metro clusters for recognisable geography
    let mut clusters = 0.0;
    clusters += gauss_blob(dir,  40.0,  -74.0, 10.0, 1.0);  // NYC/Boston
    clusters += gauss_blob(dir,  51.5,   -0.1,  8.0, 1.0);  // London
    clusters += gauss_blob(dir,  48.8,    2.3,  8.5, 0.9);  // Paris
    clusters += gauss_blob(dir,  35.7,  139.7,  8.0, 1.0);  // Tokyo
    clusters += gauss_blob(dir,  31.2,  121.5, 10.0, 0.9);  // Shanghai
    clusters += gauss_blob(dir,  28.6,   77.2, 10.0, 0.9);  // Delhi
    clusters += gauss_blob(dir, -23.6,  -46.6, 10.0, 0.8);  // São Paulo
    clusters += gauss_blob(dir,   6.5,    3.4, 10.0, 0.8);  // Lagos
    let metro = (clusters * 0.85).clamp(0.0, 1.0);

    let cities = (noise_cities * 0.6 + metro).clamp(0.0, 1.0) * land_m;
    let c = cc(CITY_LIGHT);
    let (r,g,b) = color_to_f32(c);
    let intensity = 0.85 * cities * night;
    f32_to_color(r*intensity, g*intensity, b*intensity)
}

/* ───────────────────────────────────────────────────────────────────────────
   RING SHADERS
   - shade_ring_base (opaque): radial gradient + azimuthal banding + simple light
   - shade_ring_glow (additive): subtle glow near the outer rim, sun-facing
   Assumption: ring.obj lies in the XZ plane (Y ~ 0). If your OBJ is XY,
               change the `radial_r` line as noted below.
   ─────────────────────────────────────────────────────────────────────────── */

pub fn shade_ring_base(pos_model: Vec3, n_view: Vec3, light_dir_view: Vec3, _ambient: f32, unlit: bool) -> Color {
    // Radial coordinate in XZ plane (swap to XY if your ring OBJ uses XY)
    let radial_r = (pos_model.x * pos_model.x + pos_model.z * pos_model.z).sqrt();

    // Normalize radius 0..1 across the physical ring thickness
    let r_in  = 0.60;
    let r_out = 1.00;
    let rn = ((radial_r - r_in) / (r_out - r_in)).clamp(0.0, 1.0);

    // Azimuth for color drift and spoke effects
    let theta = pos_model.z.atan2(pos_model.x); // [-π, π]

    // ── Slate + Orange palette ─────────────────────────────────────────────
    // Slate family
    let slate_light  = Color::from_hex(0xB8C0CC); // pale slate
    let slate_mid    = Color::from_hex(0x768096); // mid slate
    let slate_deep   = Color::from_hex(0x3E4658); // deep slate
    let charcoal     = Color::from_hex(0x1B202B); // near-black slate
    let cassini_gap  = Color::from_hex(0x0E131C); // darkest gap
    // Orange/copper family
    let copper       = Color::from_hex(0xD3792E); // copper orange
    let amber        = Color::from_hex(0xFFB25B); // warm amber
    let rust         = Color::from_hex(0xA65020); // rust
    let peach        = Color::from_hex(0xFFD5AE); // pale peach (hairlines)

    // ── Large-scale ring segments (C, B, Cassini, A, outer fade) ──────────
    let c_ring    = smoothstep(0.62, 0.66, rn) * (1.0 - smoothstep(0.70, 0.73, rn));
    let b_ring    = smoothstep(0.66, 0.72, rn) * (1.0 - smoothstep(0.84, 0.88, rn));
    let cassini   = smoothstep(0.78, 0.80, rn) * (1.0 - smoothstep(0.82, 0.84, rn));
    let a_ring    = smoothstep(0.84, 0.90, rn) * (1.0 - smoothstep(0.98, 1.00, rn));
    let outerfade = smoothstep(0.96, 1.00, rn);

    // Azimuthal drift to keep colors lively
    let drift = 0.5 + 0.5 * (theta * 1.4 + rn * 6.0).sin();

    let mut c = Color::from_hex(0x000000);

    // C ring: slate-mid with copper/amber minerals
    let c_c0 = mix(slate_mid, copper, 0.35 + 0.40 * drift);
    let c_c  = mix(c_c0, amber, 0.15 * (0.5 + 0.5 * (theta * 1.8 - rn * 2.0).sin()));
    c = mix(c, c_c, c_ring);

    // Bright B ring: slate-light with amber opalescence
    let c_b0 = mix(slate_light, amber, 0.55 + 0.35 * (theta * 0.9 - rn * 3.8).sin().max(-1.0).min(1.0));
    let c_b  = mix(c_b0, copper, 0.16 * (0.5 + 0.5 * (theta * 1.2 + rn * 2.8).sin()));
    c = mix(c, c_b, b_ring);

    // Cassini Division
    c = mix(c, cassini_gap, 0.92 * cassini);

    // A ring: deeper oranges blended with slate hints
    let c_a0 = mix(rust, copper, 0.55 + 0.45 * (theta * 1.7 + rn * 3.0).sin());
    let c_a  = mix(c_a0, slate_light, 0.12 * (0.5 + 0.5 * (theta * 2.3 - rn * 1.4).sin()));
    c = mix(c, c_a, a_ring);

    // Outermost gentle fade (cool slate edge with a warm kiss)
    c = mix(c, mix(slate_light, copper, 0.25), outerfade * 0.55);

    // ── Medium and fine banding (slate + orange accents) ──────────────────
    // Medium bands (slight azimuthal warp)
    let warp_m = 0.035 * (fbm(Vec3::new(pos_model.y, pos_model.z, pos_model.x) * 6.0, 2) - 0.5)
               + 0.032 * (theta * 2.0).sin();
    let band_m = ((rn * 42.0 + 12.0 * warp_m).sin() * 0.5 + 0.5);
    let mid_tint = mix(copper, amber, 0.5 + 0.5 * (theta * 0.8 + rn * 5.0).sin());
    c = mix(c, mid_tint, 0.26 * band_m);

    // Fine hairline banding (dense dark/light lines)
    let warp_f = 0.020 * (fbm(Vec3::new(pos_model.z, pos_model.x, pos_model.y) * 22.0, 3) - 0.5)
               + 0.015 * (theta * 6.0).sin();
    let lines      = (rn * 340.0 + 50.0 * warp_f).sin().abs();
    let fine_dark  = lines.powf(6.5);
    let fine_light = (1.0 - lines).powf(6.5);
    c = mix(c, charcoal,  0.11 * fine_dark);
    let light_tint = mix(amber, peach, 0.60 + 0.40 * (theta * 1.4 + rn * 2.5).sin());
    c = mix(c, light_tint, 0.20 * fine_light);

    // Mineral flecks / icy sparkles (warm)
    let fleck = smoothstep(0.82, 0.96, fbm(Vec3::new(pos_model.x, pos_model.y, pos_model.z) * 60.0, 2));
    c = mix(c, copper, 0.08 * fleck);
    c = mix(c, amber,  0.06 * fleck * (0.5 + 0.5 * (theta * 3.0).sin()));

    // Radial "spokes" near inner B ring (still subtle)
    let spoke_zone = smoothstep(0.66, 0.72, rn) * (1.0 - smoothstep(0.76, 0.82, rn));
    let spokes = (theta * 24.0).sin().abs().powf(10.0);
    c = mix(c, charcoal, 0.06 * spokes * spoke_zone);

    // ── Gentle lighting (avoid neon look) ─────────────────────────────────
    let shade = if unlit { 1.0 } else { lambert(n_view, light_dir_view, 0.08) };
    let (r,g,b) = color_to_f32(c);
    f32_to_color(r * shade, g * shade, b * shade)
}

pub fn shade_ring_glow(pos_model: Vec3, n_view: Vec3, light_dir_view: Vec3, _ambient: f32, _unlit: bool) -> Color {
    let radial_r = (pos_model.x * pos_model.x + pos_model.z * pos_model.z).sqrt();
    let r_in  = 0.60;
    let r_out = 1.02; // glow extends slightly beyond geometry
    let rn  = ((radial_r - r_in) / (r_out - r_in)).clamp(0.0, 1.0);

    // Restrict glow to the outermost rim
    let rim = smoothstep(0.92, 1.00, rn);

    // Sun-facing term (subtle)
    let l = -light_dir_view;
    let sun = glm::dot(&n_view, &l).clamp(0.0, 1.0);

    // Azimuthal tint: amber ⇄ copper sweep with a hint of slate coolness
    let theta = pos_model.z.atan2(pos_model.x);
    let amber = Color::from_hex(0xFFB25B);
    let copper = Color::from_hex(0xD3792E);
    let slate  = Color::from_hex(0x8C96A8);
    let sweep = 0.5 + 0.5 * (theta * 1.3).sin();
    let base  = mix(amber, copper, sweep);
    let col   = mix(base, slate, 0.10 * (0.5 + 0.5 * (theta * 2.0).cos()));

    let (r,g,b) = color_to_f32(col);
    let glow = 0.038 * rim * (0.30 + 0.70 * sun);
    f32_to_color(r * glow, g * glow, b * glow)
}
/* ───────────────────────────────────────────────────────────────────────────
   MOON — 4 LAYER SHADERS
   L1: Base albedo (opaque): maria vs. highlands + crater rims/floors
   L2: Microdetail (additive): tiny highlights and speckle
   L3: Rim haze (additive): faint bluish-gray limb
   L4: Thermal (additive): very faint warm emission on night side
   ─────────────────────────────────────────────────────────────────────────── */

#[inline] fn mc(hex: u32) -> Color { Color::from_hex(hex) }
const MARE_DARK:   u32 = 0x3A3A40; // basaltic maria
const REGOLITH:    u32 = 0x8E8E94; // average gray
const HIGHLAND:    u32 = 0xB8B8BE; // brighter crust
const RIM_BRIGHT:  u32 = 0xD8D8DE; // bright crater rim dust
const THERMAL:     u32 = 0x6E4A2D; // faint warm night-side glow
const RIM_HAZE:    u32 = 0x9FB0C6; // cool limb tint

pub fn shade_moon_base(pos_model: Vec3, n_view: Vec3, light_dir_view: Vec3, ambient: f32, unlit: bool) -> Color {
    let dir = if glm::length(&pos_model) > 0.0 { glm::normalize(&pos_model) } else { Vec3::new(0.0,1.0,0.0) };

    // Broad maria vs highlands
    let w1 = fbm(dir * 1.8, 3);
    let w2 = fbm(Vec3::new(dir.y, dir.z, dir.x) * 2.4, 2);
    let macro_pat = (0.65*w1 + 0.35*w2).clamp(0.0,1.0);
    let maria_m   = smoothstep(0.52, 0.35, macro_pat); // darker basins

    // Crater field: ringy mask from high-frequency signals
    let hf  = fbm(dir * 14.0, 3);
    let hf2 = fbm(Vec3::new(dir.z, dir.x, dir.y) * 22.0, 2);
    let rings = (((hf * 28.0).fract()) - 0.5).abs(); // 0..0.5
    let rims  = smoothstep(0.16, 0.03, rings);
    let floors= smoothstep(0.78, 0.92, hf2 + 0.05);

    // Compose grayscale albedo
    let mut c = mix(mc(REGOLITH), mc(HIGHLAND), 0.40 + 0.40 * (1.0 - maria_m));
    c = mix(c, mc(MARE_DARK), maria_m * 0.85);         // maria darker
    c = mix(c, mc(RIM_BRIGHT), 0.35 * rims);           // bright rims
    c = mix(c, mc(MARE_DARK),  0.25 * floors);         // darker floors

    // Simple lambert; slightly higher ambient for readability
    let shade = if unlit { 1.0 } else { lambert(n_view, light_dir_view, (ambient*1.1 + 0.12).min(0.42)) };
    let (r,g,b) = color_to_f32(c);
    f32_to_color(r*shade, g*shade, b*shade)
}

pub fn shade_moon_detail(pos_model: Vec3, n_view: Vec3, light_dir_view: Vec3, _ambient: f32, _unlit: bool) -> Color {
    let dir = if glm::length(&pos_model) > 0.0 { glm::normalize(&pos_model) } else { Vec3::new(0.0,1.0,0.0) };
    let micro = fbm(dir * 36.0, 3).clamp(0.0,1.0);
    let l = -light_dir_view;
    let nl = glm::dot(&n_view, &l).max(0.0);
    let k = (nl.powf(3.5)) * smoothstep(0.55, 0.92, micro);
    let (r,g,b) = color_to_f32(mc(HIGHLAND));
    let i = 0.12 * k; // subtle
    f32_to_color(r*i, g*i, b*i)
}

pub fn shade_moon_rim(_pos_model: Vec3, n_view: Vec3, light_dir_view: Vec3, _ambient: f32, _unlit: bool) -> Color {
    let rim = rim_term(n_view, 2.4);
    let l = -light_dir_view;
    let sun = glm::dot(&n_view, &l).clamp(0.0, 1.0);
    let c = mc(RIM_HAZE);
    let (r,g,b) = color_to_f32(c);
    let i = 0.10 * rim * (0.35 + 0.65 * sun);
    f32_to_color(r*i, g*i, b*i)
}

pub fn shade_moon_thermal(pos_model: Vec3, n_view: Vec3, light_dir_view: Vec3, _ambient: f32, _unlit: bool) -> Color {
    let dir = if glm::length(&pos_model) > 0.0 { glm::normalize(&pos_model) } else { Vec3::new(0.0,1.0,0.0) };
    let l = -light_dir_view;
    let day = glm::dot(&n_view, &l).clamp(0.0, 1.0);
    let night = (1.0 - day);
    let night_mask = smoothstep(0.15, 0.65, night);
    // Faulty streaks
    let fault = (glm::dot(&dir, &glm::normalize(&Vec3::new(0.6,0.1,-0.7))) * 26.0).sin().abs();
    let faults_mask = smoothstep(0.86, 0.96, 1.0 - fault) * fbm(dir * 10.0, 2);
    let thermal = (0.55 * faults_mask + 0.45 * fbm(dir * 2.0, 2)) * night_mask;
    let (r,g,b) = color_to_f32(mc(THERMAL));
    let i = 0.18 * thermal;
    f32_to_color(r*i, g*i, b*i)
}

/* ───────────────────────────────────────────────────────────────────────────
   ROCKY — LAVA PALETTE (4 LAYERS reused from Earth structure)
   ─────────────────────────────────────────────────────────────────────────── */
#[inline] fn lc(hex: u32) -> Color { Color::from_hex(hex) }
const LAVA_OCEAN_DEEP:   u32 = 0x3B0B00; // dark molten
const LAVA_OCEAN_BRIGHT: u32 = 0xFF6A00; // bright lava
const LAVA_GLOW:         u32 = 0xFFA040; // glow foam
const LAVA_ASH:          u32 = 0x2A1E1A; // ash gray
const LAVA_BASALT:       u32 = 0x201818; // land base
const LAVA_RIDGE:        u32 = 0x3A2C2C; // ridges
const LAVA_EMBER:        u32 = 0xFF3B1A; // emissive veins
const LAVA_NIGHT:        u32 = 0xFF9A3A; // night emissive

pub fn shade_rock_lava_ocean(pos_model: Vec3, n_view: Vec3, light_dir_view: Vec3, ambient: f32, unlit: bool) -> Color {
    let dir = if glm::length(&pos_model) > 0.0 { glm::normalize(&pos_model) } else { Vec3::new(0.0,1.0,0.0) };
    let (_u, v) = spherical_uv(dir);
    let lat = 1.0 - 2.0 * v;
    let l = -light_dir_view;
    let sun = glm::dot(&n_view, &l).clamp(0.0, 1.0);

    // Lava seas depth + hot upwelling near equator
    let depth = (0.5 + 0.5 * fbm(dir * 2.0, 3)).clamp(0.0, 1.0);
    let mut ocean = mix(lc(LAVA_OCEAN_DEEP), lc(LAVA_OCEAN_BRIGHT), 0.25 + 0.70 * depth);
    let equ = smoothstep(0.20, 0.55, 1.0 - lat.abs());
    ocean = mix(ocean, lc(LAVA_GLOW), 0.20 * equ);

    // Basaltic continents (reuse continent mask)
    let (land_m, coast) = super::planet::continent_mask(dir);
    let elev = fbm(dir * 4.5, 3);
    let ridges = smoothstep(0.55, 0.78, elev);
    let mut land = mix(lc(LAVA_BASALT), lc(LAVA_RIDGE), ridges * 0.65);

    let mut c = mix(ocean, land, land_m);
    // Glowing shore where lava meets land
    let foam = 0.18 * coast * (0.35 + 0.65 * sun);
    c = mix(c, lc(LAVA_GLOW), foam);

    let shade = if unlit { 1.0 } else { lambert(n_view, light_dir_view, (ambient*0.45 + 0.12).min(0.38)) };
    let (r,g,b) = color_to_f32(c);
    f32_to_color(r*shade, g*shade, b*shade)
}

pub fn shade_rock_lava_land(pos_model: Vec3, n_view: Vec3, light_dir_view: Vec3, _ambient: f32, _unlit: bool) -> Color {
    let dir = if glm::length(&pos_model) > 0.0 { glm::normalize(&pos_model) } else { Vec3::new(0.0,1.0,0.0) };
    let (land_m, coast) = super::planet::continent_mask(dir);
    let l = -light_dir_view;
    let sun = glm::dot(&n_view, &l).clamp(0.0, 1.0);
    // Add hot ember veins as additive tint on land + along coasts
    let veins = smoothstep(0.70, 0.86, fbm(dir * 16.0, 3));
    let (r,g,b) = color_to_f32(lc(LAVA_EMBER));
    let k = (0.16 * land_m + 0.28 * coast) * (0.30 + 0.70 * sun) * veins;
    f32_to_color(r*k, g*k, b*k)
}

pub fn shade_rock_lava_clouds(pos_model: Vec3, n_view: Vec3, light_dir_view: Vec3, time: f32, _unlit: bool) -> Color {
    let dir = if glm::length(&pos_model) > 0.0 { glm::normalize(&pos_model) } else { Vec3::new(0.0,1.0,0.0) };
    // Sphere to UV (wrap-aware)
    let (mut u, mut v) = spherical_uv(dir);
    // Advect clouds slowly over time (wind)
    let wind_u = 0.025 * time;
    u = (u + wind_u).fract();

    // Sun / rim terms
    let rim = rim_term(n_view, 2.0);
    let l = -light_dir_view;
    let sun = glm::dot(&n_view, &l).clamp(0.0, 1.0);

    // Helper: tiny hash for jitter (deterministic)
    #[inline]
    fn hash21(p: (i32, i32)) -> f32 {
        let x = p.0 as f32;
        let y = p.1 as f32;
        let s = (x * 127.1 + y * 311.7).sin() * 43758.5453;
        s.fract()
    }

    // Distance on a torus (wrap-aware 0..1 domain)
    #[inline]
    fn wrap_dist(a: f32, b: f32) -> f32 {
        let d = (a - b).abs();
        d.min(1.0 - d)
    }

    // Build blobby clouds from a sum of soft discs at multiple scales.
    // We use a few grids of different resolutions, jittering disc centers and radii.
    let mut mask = 0.0f32;

    // Each entry: (cells_u, cells_v, base_radius, radius_jitter, opacity)
    let layers = [
        (8, 4,  0.075f32, 0.024f32, 0.70f32),  // large puffs (stronger)
        (16, 8, 0.038f32, 0.015f32, 0.75f32),  // medium (stronger)
        (32, 16,0.018f32, 0.008f32, 0.50f32),  // fine wisps (lighter so they don't wash out)
    ];

    for (cu, cv, r0, rj, op) in layers {
        // Find the current UV's integer cell
        let iu = (u * cu as f32).floor() as i32;
        let iv = (v * cv as f32).floor() as i32;

        // Consider a 3x3 neighborhood of cells so discs from neighbors can cover this fragment
        for du in -1..=1 {
            for dv in -1..=1 {
                let cu_i = iu + du;
                let cv_i = iv + dv;

                // Jitter center inside the cell
                let j1 = hash21((cu_i * 9283 + 57, cv_i * 6899 + 101));
                let j2 = hash21((cu_i * 2311 + 73,  cv_i * 9157 + 19));
                let cx = (cu_i as f32 + 0.5 + (j1 - 0.5) * 0.7) / cu as f32;
                let cy = (cv_i as f32 + 0.5 + (j2 - 0.5) * 0.7) / cv as f32;

                // Radius jitter
                let rj_hash = hash21((cu_i * 4133 + 11, cv_i * 3769 + 29));
                let r = (r0 + rj * (rj_hash - 0.5)).max(0.005);

                // Wrap-aware distance
                let dx = wrap_dist(u, cx);
                let dy = wrap_dist(v, cy);
                let d = (dx*dx + dy*dy).sqrt();

                // Soft disc falloff (inner solid + soft edge)
                let inner = r * 0.65;
                let m = smoothstep(r, inner, d); // 0 at r .. 1 at inner

                // Slight height bias (make cloud tops brighter towards sub-solar)
                let tilt = 0.75 + 0.25 * sun;

                // Accumulate with capped add to keep things soft
                mask = (mask + m * op * tilt).min(1.0);
            }
        }
    }

    // Global coverage control — lower = fewer clouds (0..1)
    let density = 0.52f32; // bump coverage so ash clouds are clearly visible
    mask = (mask * density).min(1.0);

    // Add gentle erosion so silhouettes aren’t perfectly round
    // Use fbm to nibble at the mask edge
    let erosion = fbm(dir * 9.0 + Vec3::new(0.1*time, 0.07*time, -0.05*time), 3);
    let eroded = smoothstep(0.40, 0.60, mask - 0.22 * (erosion - 0.5));

    // Compose color: ash gray (darker soot → mid‑gray highlight)
    let c_lo = Color::from_hex(0x232323); // deep ash
    let c_hi = Color::from_hex(0x5A5A5A); // medium ash gray
    // Slightly brighter toward the sun so shapes read, but stay muted
    let c = mix(c_lo, c_hi, 0.32 + 0.50 * sun);
    let (r,g,b) = color_to_f32(c);

    // Final intensity: strong enough to be clearly visible, but still additive
    let intensity = 0.85 * eroded * (0.45 + 0.55 * sun) + 0.22 * rim * eroded; // stronger so they show up

    f32_to_color(r * intensity, g * intensity, b * intensity)
}

pub fn shade_rock_lava_night(pos_model: Vec3, n_view: Vec3, light_dir_view: Vec3, time: f32, _unlit: bool) -> Color {
    let dir = if glm::length(&pos_model) > 0.0 { glm::normalize(&pos_model) } else { Vec3::new(0.0,1.0,0.0) };
    let (land_m, _coast) = super::planet::continent_mask(dir);
    let l = -light_dir_view; let day = glm::dot(&n_view, &l).clamp(0.0, 1.0); let night = (1.0 - day).powf(2.0);
    let flows = smoothstep(0.66, 0.80, fbm(dir * 12.0 + Vec3::new(0.08*time, -0.06*time, 0.05*time), 3));
    let emit = (flows * land_m).clamp(0.0, 1.0);
    let (r,g,b) = color_to_f32(lc(LAVA_NIGHT));
    let i = 0.95 * emit * night;
    f32_to_color(r*i, g*i, b*i)
}

/* ───────────────────────────────────────────────────────────────────────────
   ROCKY — VERDANT PALETTE (4 LAYERS)
   ─────────────────────────────────────────────────────────────────────────── */
#[inline] fn vc(hex: u32) -> Color { Color::from_hex(hex) }
const VER_OCEAN_DEEP:   u32 = 0x074A4F; // deep teal (not blue)
const VER_OCEAN_SHAL:   u32 = 0x14E0C7; // bright aqua shallows
const VER_FOAM:         u32 = 0xB6FFE4; // mint sea foam
const VER_LAND_LUSH:    u32 = 0x7AC943; // lime-green biomes
const VER_LAND_DRY:     u32 = 0x2E6E3B; // deep moss/fern
const VER_MOUNTAIN:     u32 = 0x2B2F34; // obsidian/graphite ridges
const VER_SNOW:         u32 = 0xE0FFF7; // icy-mint caps
const VER_CITY:         u32 = 0x72FFE8; // cool aqua city glow

pub fn shade_rock_verdant_ocean(pos_model: Vec3, n_view: Vec3, light_dir_view: Vec3, ambient: f32, unlit: bool) -> Color {
    let dir = if glm::length(&pos_model) > 0.0 { glm::normalize(&pos_model) } else { Vec3::new(0.0,1.0,0.0) };
    let (_u, v) = spherical_uv(dir);
    let lat = 1.0 - 2.0 * v;
    let l = -light_dir_view; let sun = glm::dot(&n_view, &l).clamp(0.0, 1.0);
    let depth = (0.5 + 0.5 * fbm(dir * 2.3, 3)).clamp(0.0, 1.0);
    let mut ocean = mix(vc(VER_OCEAN_DEEP), vc(VER_OCEAN_SHAL), 0.25 + 0.70*depth);
    let polar = smoothstep(0.55, 0.82, lat.abs());
    ocean = mix(ocean, vc(VER_SNOW), 0.16 * polar);

    let (land_m, coast) = super::planet::continent_mask(dir);
    let elev = fbm(dir * 5.2, 3);
    let mountains = smoothstep(0.58, 0.78, elev);
    let dryness = smoothstep(0.20, 0.85, lat.abs());
    let base_land = mix(vc(VER_LAND_LUSH), vc(VER_LAND_DRY), dryness);
    let mut land_col = mix(base_land, vc(VER_MOUNTAIN), 0.35 * mountains);
    let snow = smoothstep(0.65, 0.90, lat.abs()) * mountains;
    land_col = mix(land_col, vc(VER_SNOW), 0.55 * snow);

    let mut c = mix(ocean, land_col, land_m);
    c = mix(c, vc(VER_FOAM), 0.18 * coast * (0.35 + 0.65 * sun));

    let shade = if unlit { 1.0 } else { lambert(n_view, light_dir_view, (ambient*0.45 + 0.15).min(0.40)) };
    let (r,g,b) = color_to_f32(c);
    f32_to_color(r*shade, g*shade, b*shade)
}

pub fn shade_rock_verdant_land(pos_model: Vec3, n_view: Vec3, light_dir_view: Vec3, _ambient: f32, _unlit: bool) -> Color {
    let dir = if glm::length(&pos_model) > 0.0 { glm::normalize(&pos_model) } else { Vec3::new(0.0,1.0,0.0) };
    let (_u,_v)= spherical_uv(dir);
    let (land_m, coast) = super::planet::continent_mask(dir);
    let l = -light_dir_view; let sun = glm::dot(&n_view, &l).clamp(0.0,1.0);
    let accent = mix(vc(VER_LAND_LUSH), vc(VER_LAND_DRY), 0.20);
    let (r,g,b) = color_to_f32(accent);
    let k = (0.18 * land_m + 0.24 * coast) * (0.35 + 0.65 * sun);
    f32_to_color(r*k, g*k, b*k)
}

pub fn shade_rock_verdant_clouds(pos_model: Vec3, n_view: Vec3, light_dir_view: Vec3, time: f32, _unlit: bool) -> Color {
    let dir = if glm::length(&pos_model) > 0.0 { glm::normalize(&pos_model) } else { Vec3::new(0.0,1.0,0.0) };
    let p = dir * 10.0 + Vec3::new(0.10*time, 0.07*time, -0.06*time);
    let n = fbm(p, 4); let mask = smoothstep(0.58, 0.70, n);
    let l = -light_dir_view; let sun = glm::dot(&n_view, &l).clamp(0.0, 1.0);
    let c = mix(vc(0xECFFF7), vc(0xBDEDDC), 0.24 * (1.0 - sun));
    let (r,g,b) = color_to_f32(c);
    let intensity = 0.60 * mask * (0.45 + 0.55 * sun) + 0.16 * rim_term(n_view, 2.0) * mask;
    f32_to_color(r*intensity, g*intensity, b*intensity)
}

pub fn shade_rock_verdant_night(pos_model: Vec3, n_view: Vec3, light_dir_view: Vec3, time: f32, _unlit: bool) -> Color {
    let dir = if glm::length(&pos_model) > 0.0 { glm::normalize(&pos_model) } else { Vec3::new(0.0,1.0,0.0) };
    let (land_m, _coast) = super::planet::continent_mask(dir);
    let l = -light_dir_view; let day = glm::dot(&n_view, &l).clamp(0.0, 1.0); let night = (1.0 - day).powf(2.0);
    let density = fbm(dir * 18.0 + Vec3::new(0.10*time, -0.07*time, 0.05*time), 3);
    let cities = smoothstep(0.70, 0.86, density) * land_m;
    let (r,g,b) = color_to_f32(vc(VER_CITY));
    let intensity = 0.80 * cities * night;
    f32_to_color(r*intensity, g*intensity, b*intensity)
}

/* ───────────────────────────────────────────────────────────────────────────
   MOON VARIANTS — LAVA & VERDANT (color tweaks over base logic)
   ─────────────────────────────────────────────────────────────────────────── */
// LAVA MOON
pub fn shade_moon_lava_base(pos_model: Vec3, n_view: Vec3, light_dir_view: Vec3, ambient: f32, unlit: bool) -> Color {
    // reuse moon_base but bias colors warmer
    let mut c = shade_moon_base(pos_model, n_view, light_dir_view, ambient, unlit);
    // tint towards warm
    let warm = mc(0xB87444);
    c = mix(c, warm, 0.18);
    c
}
pub fn shade_moon_lava_detail(pos_model: Vec3, n_view: Vec3, light_dir_view: Vec3, a:f32, u:bool) -> Color { shade_moon_detail(pos_model, n_view, light_dir_view, a, u) }
pub fn shade_moon_lava_rim(pos_model: Vec3, n_view: Vec3, light_dir_view: Vec3, a:f32, u:bool) -> Color { shade_moon_rim(pos_model, n_view, light_dir_view, a, u) }
pub fn shade_moon_lava_thermal(pos_model: Vec3, n_view: Vec3, light_dir_view: Vec3, a:f32, u:bool) -> Color {
    let base = shade_moon_thermal(pos_model, n_view, light_dir_view, a, u);
    let glow = mc(0xFF7A2F); let (r,g,b)=color_to_f32(glow); let (br,bg,bb)=color_to_f32(base);
    f32_to_color((br*0.7 + r*0.3), (bg*0.7 + g*0.3), (bb*0.7 + b*0.3))
}

// VERDANT MOON
pub fn shade_moon_verdant_base(pos_model: Vec3, n_view: Vec3, light_dir_view: Vec3, ambient: f32, unlit: bool) -> Color {
    let mut c = shade_moon_base(pos_model, n_view, light_dir_view, ambient, unlit);
    let cool = mc(0x96BCA0);
    c = mix(c, cool, 0.16);
    c
}
pub fn shade_moon_verdant_detail(pos_model: Vec3, n_view: Vec3, light_dir_view: Vec3, a:f32, u:bool) -> Color { shade_moon_detail(pos_model, n_view, light_dir_view, a, u) }
pub fn shade_moon_verdant_rim(pos_model: Vec3, n_view: Vec3, light_dir_view: Vec3, a:f32, u:bool) -> Color { shade_moon_rim(pos_model, n_view, light_dir_view, a, u) }
pub fn shade_moon_verdant_thermal(pos_model: Vec3, n_view: Vec3, light_dir_view: Vec3, a:f32, u:bool) -> Color { shade_moon_thermal(pos_model, n_view, light_dir_view, a, u) }

/* ───────────────────────────────────────────────────────────────────────────
   GAS GIANT — GOLD/ORANGE PALETTE (4 LAYERS)
   ─────────────────────────────────────────────────────────────────────────── */
#[inline] fn gg(hex: u32) -> Color { Color::from_hex(hex) }
pub fn shade_gas_gold_base(pos_model: Vec3, n_view: Vec3, light_dir_view: Vec3, ambient: f32, unlit: bool) -> Color {
    let dir = if glm::length(&pos_model) > 0.0 { glm::normalize(&pos_model) } else { Vec3::new(0.0,1.0,0.0) };
    let (u, v) = spherical_uv(dir); let tau = std::f32::consts::TAU; let uu = u * tau; let lat = 1.0 - 2.0 * v;
    let gold_lo = gg(0xD8A100); let gold_hi = gg(0xFFE07A); let amber = gg(0xFFB25B); let rust = gg(0xA65E1E);
    let w1 = fbm(Vec3::new(dir.y, dir.z, dir.x) * 3.0, 3) * 0.45; let w2 = fbm(Vec3::new(-dir.z, dir.x, dir.y) * 6.0, 2) * 0.25; let warp = w1+w2+0.05*(uu*8.0).sin();
    let bands = (lat * 8.0 + 1.6 * (20.0 * dir.x).sin() + 3.0 * warp).sin() * 0.5 + 0.5;
    let mut c = mix(gold_lo, amber, bands); c = mix(c, gold_hi, 0.35 * (0.5 + 0.5 * (uu * 1.3 + lat * 4.0).sin())); c = mix(c, rust, 0.20 * (0.5 + 0.5 * (uu * 1.7 - lat * 3.2).cos()));
    let shade = if unlit { 1.0 } else { lambert(n_view, light_dir_view, (ambient * 0.85).max(0.10)) };
    let (r,g,b) = color_to_f32(c); f32_to_color(r*shade, g*shade, b*shade)
}
pub fn shade_gas_gold_eddies(pos_model: Vec3, n_view: Vec3, light_dir_view: Vec3, _a: f32, _u: bool) -> Color {
    let dir = if glm::length(&pos_model) > 0.0 { glm::normalize(&pos_model) } else { Vec3::new(0.0,1.0,0.0) };
    let (u, v) = spherical_uv(dir); let tau = std::f32::consts::TAU; let uu = u * tau;
    let s = ((6.0 * uu + 18.0 * v).sin() * (12.0 * uu + 5.0 * v).sin()) * 0.5 + 0.5; let n = fbm(dir * 9.0, 3); let mask = smoothstep(0.50, 0.80, 0.6*s + 0.4*n);
    let lo = gg(0xFFD088); let hi = gg(0xFF8C2E); let c = mix(lo, hi, 0.5 + 0.5 * (uu * 0.7 + v * 4.0).sin());
    let (r,g,b) = color_to_f32(c); let k = 0.22 * mask * (0.35 + 0.65 * glm::dot(&n_view, &(-light_dir_view)).max(0.0)); f32_to_color(r*k, g*k, b*k)
}
pub fn shade_gas_gold_highclouds(pos_model: Vec3, n_view: Vec3, light_dir_view: Vec3, time: f32, _u: bool) -> Color {
    let dir = if glm::length(&pos_model) > 0.0 { glm::normalize(&pos_model) } else { Vec3::new(0.0,1.0,0.0) };
    let p = dir * 12.0 + Vec3::new(0.10*time, 0.07*time, -0.06*time); let n = fbm(p, 4); let mask = smoothstep(0.62, 0.74, n);
    let l = -light_dir_view; let sun = glm::dot(&n_view, &l).clamp(0.0, 1.0);
    let wh = gg(0xFFF8EB); let pe = gg(0xFFD4A6); let c = mix(wh, pe, 0.22 * (1.0 - sun));
    let (r,g,b) = color_to_f32(c); let intensity = 0.58 * mask * (0.45 + 0.55 * sun) + 0.14 * rim_term(n_view, 2.0) * mask; f32_to_color(r*intensity, g*intensity, b*intensity)
}
pub fn shade_gas_gold_aurora(pos_model: Vec3, n_view: Vec3, light_dir_view: Vec3, _a: f32, _u: bool) -> Color {
    let dir = if glm::length(&pos_model) > 0.0 { glm::normalize(&pos_model) } else { Vec3::new(0.0,1.0,0.0) };
    let (_u, v) = spherical_uv(dir); let lat = 1.0 - 2.0 * v; let poles = smoothstep(0.65, 0.90, lat.abs());
    let wav = (glm::dot(&dir, &glm::normalize(&Vec3::new(0.2,1.0,0.1))) * 24.0).sin().abs(); let mask = (0.6 * poles + 0.4 * wav.powf(6.0)) * 0.9;
    let a_lo = gg(0xFFE38A); let a_hi = gg(0xFF9A3A); let c = mix(a_lo, a_hi, 0.5 + 0.5 * (dir.x * 6.0 + dir.z * 5.0).sin());
    let (r,g,b) = color_to_f32(c); let k = 0.18 * mask * (0.25 + 0.75 * (1.0 - glm::dot(&n_view, &(-light_dir_view)).clamp(0.0,1.0))); f32_to_color(r*k, g*k, b*k)
}

/* ───────────────────────────────────────────────────────────────────────────
   RING — GOLD PALETTE (base + glow)
   ─────────────────────────────────────────────────────────────────────────── */
pub fn shade_ring_gold_base(pos_model: Vec3, n_view: Vec3, light_dir_view: Vec3, _ambient: f32, unlit: bool) -> Color {
    let radial_r = (pos_model.x * pos_model.x + pos_model.z * pos_model.z).sqrt();
    let r_in  = 0.60; let r_out = 1.00; let rn = ((radial_r - r_in) / (r_out - r_in)).clamp(0.0, 1.0);
    let theta = pos_model.z.atan2(pos_model.x);
    let pale = gg(0xFFF0C7); let gold = gg(0xF2C66B); let bronze = gg(0xB8863B); let gap = gg(0x2A1C07);
    let c_ring = smoothstep(0.62, 0.66, rn) * (1.0 - smoothstep(0.70, 0.73, rn));
    let b_ring = smoothstep(0.66, 0.72, rn) * (1.0 - smoothstep(0.84, 0.88, rn));
    let cass   = smoothstep(0.78, 0.80, rn) * (1.0 - smoothstep(0.82, 0.84, rn));
    let a_ring = smoothstep(0.84, 0.90, rn) * (1.0 - smoothstep(0.98, 1.00, rn));
    let mut c = mix(pale, gold, 0.5 + 0.5 * (theta * 0.8 + rn * 1.4).sin());
    c = mix(c, gold, c_ring);
    c = mix(c, pale, 0.7 * b_ring);
    c = mix(c, gap,  0.9 * cass);
    c = mix(c, bronze, a_ring);
    // fine lines
    let lines = (rn * 320.0 + 40.0 * (theta * 1.2).sin()).sin().abs();
    let fine_dark = lines.powf(6.5); let fine_light = (1.0 - lines).powf(6.5);
    c = mix(c, bronze, 0.10 * fine_dark);
    c = mix(c, pale,   0.16 * fine_light);
    let shade = if unlit { 1.0 } else { lambert(n_view, light_dir_view, 0.08) };
    let (r,g,b) = color_to_f32(c); f32_to_color(r*shade, g*shade, b*shade)
}

pub fn shade_ring_gold_glow(pos_model: Vec3, n_view: Vec3, light_dir_view: Vec3, _ambient: f32, _unlit: bool) -> Color {
    let radial_r = (pos_model.x * pos_model.x + pos_model.z * pos_model.z).sqrt();
    let r_in  = 0.60; let r_out = 1.02; let rn = ((radial_r - r_in) / (r_out - r_in)).clamp(0.0, 1.0);
    let rim = smoothstep(0.92, 1.00, rn);
    let l = -light_dir_view; let sun = glm::dot(&n_view, &l).clamp(0.0, 1.0);
    let theta = pos_model.z.atan2(pos_model.x);
    let glowc = gg(0xFFE6A3);
    let (r,g,b) = color_to_f32(glowc);
    let glow = 0.040 * rim * (0.30 + 0.70 * sun) * (0.8 + 0.2 * (theta*1.3).sin().abs());
    f32_to_color(r*glow, g*glow, b*glow)
}