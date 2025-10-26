pub mod util;
pub mod star;
pub mod planet;
pub mod pipeline;

pub use pipeline::vertex_shader;

use nalgebra_glm as glm;
use glm::Vec3;
use crate::color::Color;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlanetShader {
    StarCore, StarPlasma, StarCorona, StarHotspots, StarFilaments,
    EarthOcean, EarthLand, EarthClouds, EarthNight,
    MoonBase, MoonDetail, MoonRim, MoonThermal,
    RingBase, RingGlow,
    GasBase, GasEddies, GasHighClouds, GasAurora,
    RockLavaOcean, RockLavaLand, RockLavaClouds, RockLavaNight,
    RockVerdantOcean, RockVerdantLand, RockVerdantClouds, RockVerdantNight,
    MoonLavaBase, MoonLavaDetail, MoonLavaRim, MoonLavaThermal,
    MoonVerdantBase, MoonVerdantDetail, MoonVerdantRim, MoonVerdantThermal,
    GasGoldBase, GasGoldEddies, GasGoldHighClouds, GasGoldAurora,
    RingGoldBase, RingGoldGlow,
}

pub fn shade_planet(
    which: PlanetShader,
    pos_model: Vec3,
    n_view: Vec3,
    light_dir_view: Vec3,
    ambient: f32,
    unlit: bool
) -> Color {
    match which {
        PlanetShader::StarCore   => star::shade_core(pos_model, n_view, light_dir_view, ambient, unlit),
        PlanetShader::StarPlasma => star::shade_plasma(pos_model, n_view, light_dir_view, ambient, unlit),
        PlanetShader::StarCorona => star::shade_corona(pos_model, n_view, light_dir_view, ambient, unlit),
        PlanetShader::StarHotspots => star::shade_hotspots(pos_model, n_view, light_dir_view, ambient, unlit),
        PlanetShader::StarFilaments=> star::shade_filaments(pos_model, n_view, light_dir_view, ambient, unlit),
        PlanetShader::EarthOcean     => planet::shade_earth_ocean(pos_model, n_view, light_dir_view, ambient, unlit),
        PlanetShader::EarthLand      => planet::shade_earth_land(pos_model, n_view, light_dir_view, ambient, unlit),
        PlanetShader::EarthClouds    => planet::shade_earth_clouds(pos_model, n_view, light_dir_view, ambient, unlit),
        PlanetShader::EarthNight     => planet::shade_earth_night(pos_model, n_view, light_dir_view, ambient, unlit),
        // --- Lava rocky planet layers ---
        PlanetShader::RockLavaOcean   => planet::shade_rock_lava_ocean(pos_model, n_view, light_dir_view, ambient, unlit),
        PlanetShader::RockLavaLand    => planet::shade_rock_lava_land(pos_model, n_view, light_dir_view, ambient, unlit),
        PlanetShader::RockLavaClouds  => planet::shade_rock_lava_clouds(pos_model, n_view, light_dir_view, ambient, unlit),
        PlanetShader::RockLavaNight   => planet::shade_rock_lava_night(pos_model, n_view, light_dir_view, ambient, unlit),
        // --- Verdant rocky planet layers ---
        PlanetShader::RockVerdantOcean=> planet::shade_rock_verdant_ocean(pos_model, n_view, light_dir_view, ambient, unlit),
        PlanetShader::RockVerdantLand => planet::shade_rock_verdant_land(pos_model, n_view, light_dir_view, ambient, unlit),
        PlanetShader::RockVerdantClouds=> planet::shade_rock_verdant_clouds(pos_model, n_view, light_dir_view, ambient, unlit),
        PlanetShader::RockVerdantNight=> planet::shade_rock_verdant_night(pos_model, n_view, light_dir_view, ambient, unlit),
        // --- Moon ---
        PlanetShader::MoonBase      => planet::shade_moon_base(pos_model, n_view, light_dir_view, ambient, unlit),
        PlanetShader::MoonDetail    => planet::shade_moon_detail(pos_model, n_view, light_dir_view, ambient, unlit),
        PlanetShader::MoonRim       => planet::shade_moon_rim(pos_model, n_view, light_dir_view, ambient, unlit),
        PlanetShader::MoonThermal   => planet::shade_moon_thermal(pos_model, n_view, light_dir_view, ambient, unlit),
        // --- Lava moon ---
        PlanetShader::MoonLavaBase    => planet::shade_moon_lava_base(pos_model, n_view, light_dir_view, ambient, unlit),
        PlanetShader::MoonLavaDetail  => planet::shade_moon_lava_detail(pos_model, n_view, light_dir_view, ambient, unlit),
        PlanetShader::MoonLavaRim     => planet::shade_moon_lava_rim(pos_model, n_view, light_dir_view, ambient, unlit),
        PlanetShader::MoonLavaThermal => planet::shade_moon_lava_thermal(pos_model, n_view, light_dir_view, ambient, unlit),
        // --- Verdant moon ---
        PlanetShader::MoonVerdantBase    => planet::shade_moon_verdant_base(pos_model, n_view, light_dir_view, ambient, unlit),
        PlanetShader::MoonVerdantDetail  => planet::shade_moon_verdant_detail(pos_model, n_view, light_dir_view, ambient, unlit),
        PlanetShader::MoonVerdantRim     => planet::shade_moon_verdant_rim(pos_model, n_view, light_dir_view, ambient, unlit),
        PlanetShader::MoonVerdantThermal => planet::shade_moon_verdant_thermal(pos_model, n_view, light_dir_view, ambient, unlit),
        // --- Rings ---
        PlanetShader::RingBase      => planet::shade_ring_base(pos_model, n_view, light_dir_view, ambient, unlit),
        PlanetShader::RingGlow      => planet::shade_ring_glow(pos_model, n_view, light_dir_view, ambient, unlit),
        PlanetShader::RingGoldBase    => planet::shade_ring_gold_base(pos_model, n_view, light_dir_view, ambient, unlit),
        PlanetShader::RingGoldGlow    => planet::shade_ring_gold_glow(pos_model, n_view, light_dir_view, ambient, unlit),
        // --- Gas giants ---
        PlanetShader::GasBase => planet::shade_gas_base(pos_model, n_view, light_dir_view, ambient, unlit),
        PlanetShader::GasEddies => planet::shade_gas_eddies(pos_model, n_view, light_dir_view, ambient, unlit),
        PlanetShader::GasHighClouds => planet::shade_gas_highclouds(pos_model, n_view, light_dir_view, ambient, unlit),
        PlanetShader::GasAurora => planet::shade_gas_aurora(pos_model, n_view, light_dir_view, ambient, unlit),
        PlanetShader::GasGoldBase       => planet::shade_gas_gold_base(pos_model, n_view, light_dir_view, ambient, unlit),
        PlanetShader::GasGoldEddies     => planet::shade_gas_gold_eddies(pos_model, n_view, light_dir_view, ambient, unlit),
        PlanetShader::GasGoldHighClouds => planet::shade_gas_gold_highclouds(pos_model, n_view, light_dir_view, ambient, unlit),
        PlanetShader::GasGoldAurora     => planet::shade_gas_gold_aurora(pos_model, n_view, light_dir_view, ambient, unlit),
    }
}