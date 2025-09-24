use serde::Deserialize;

/// Only what Phase 1 needs. Keep it tiny.
#[derive(Deserialize, Clone)]
pub struct PlayerSpawns {
    /// Distance from map center to each base center.
    pub center_radius: i32,
    /// Flattened height value to use for each base disk.
    pub elevation: f32,
    /// Radius (tiles) of each base disk.
    pub base_radius: i32,
}

#[derive(Deserialize, Clone)]
pub struct TerrainWeights {
    pub grassland: f32,
    pub forest: f32,
    pub water: f32,
    pub mountain: f32,
}

#[derive(Deserialize, Clone)]
pub enum AreaSource { Center, Spawn }

#[derive(Deserialize, Clone)]
pub struct TerrainArea {
    pub source: AreaSource,
    pub radius: i32,
    pub weights: TerrainWeights,
    #[serde(default = "default_area_scale")]
    pub scale: f32,
}
fn default_area_scale() -> f32 { 1.0 }

// ---- NEW: patch size configuration with sensible defaults ----
#[derive(Deserialize, Clone)]
pub struct TerrainClumps {
    pub forest_patch: (i32, i32),   // min,max radius in tiles
    pub water_patch: (i32, i32),
    pub mountain_patch: (i32, i32),
}
fn default_clumps() -> TerrainClumps {
    TerrainClumps {
        forest_patch: (7, 16),
        water_patch: (6, 12),
        mountain_patch: (8, 18),
    }
}
fn default_shrine_grass_radius() -> i32 { 12 }

// Extend TerrainRules with shrine radius + clumps (both defaulted so your RON keeps working)
#[derive(Deserialize, Clone)]
pub struct TerrainRules {
    pub areas: Vec<TerrainArea>,
    #[serde(default = "default_shrine_grass_radius")]
    pub shrine_grass_radius: i32,
    #[serde(default = "default_clumps")]
    pub clumps: TerrainClumps,
}

// ---- Ley config (template) ----
#[derive(Deserialize, Clone)]
pub struct LeyConfig {
    #[serde(default)] pub shrines_per_base: usize,
    #[serde(default)] pub shrine_ring: i32,
    #[serde(default)] pub offset_deg: f32,
    #[serde(default)] pub connect_cycle: bool,
    #[serde(default)] pub connect_spokes: bool,
}
fn d_ley_shrines_per_base() -> usize { 1 }
fn d_ley_shrine_ring() -> i32 { 160 }
fn d_ley_offset_deg() -> f32 { 15.0 }
fn d_ley_connect_cycle() -> bool { true }
fn d_ley_connect_spokes() -> bool { true }

impl Default for LeyConfig {
    fn default() -> Self {
        Self {
            shrines_per_base: d_ley_shrines_per_base(),
            shrine_ring: d_ley_shrine_ring(),
            offset_deg: d_ley_offset_deg(),
            connect_cycle: d_ley_connect_cycle(),
            connect_spokes: d_ley_connect_spokes(),
        }
    }
}

// ---- Blend config (template) ----
#[derive(Deserialize, Clone)]
pub struct BlendConfig {
    #[serde(default = "d_blend_iterations")] pub iterations: usize,
    #[serde(default = "d_blend_radii")]      pub radii: (i32,i32,i32,i32), // (grass, forest, water, mountain)
    #[serde(default = "d_blend_inertia")]     pub inertia: f32,
    #[serde(default = "d_blend_boundary")]    pub boundary_only: bool,
}
fn d_blend_iterations() -> usize { 3 }
fn d_blend_radii() -> (i32,i32,i32,i32) { (2,3,2,3) }
fn d_blend_inertia() -> f32 { 0.25 }
fn d_blend_boundary() -> bool { true }

impl Default for BlendConfig {
    fn default() -> Self {
        Self { iterations: d_blend_iterations(), radii: d_blend_radii(), inertia: d_blend_inertia(), boundary_only: d_blend_boundary() }
    }
}

// ---- Fractal config (template) ----
#[derive(Deserialize, Clone)]
pub struct FractalConfig {
    #[serde(default = "d_fract_iterations")]  pub iterations: usize,
    #[serde(default = "d_fract_radii")]       pub radii: (i32,i32,i32,i32),
    #[serde(default = "d_fract_inertia")]     pub inertia: f32,
    #[serde(default = "d_fract_boundary")]    pub boundary_only: bool,
    #[serde(default = "d_warp_amp")]          pub warp_amp: f32,
    #[serde(default = "d_warp_freq")]         pub warp_freq: f32,
    #[serde(default = "d_warp_octaves")]      pub warp_octaves: u32,
    #[serde(default = "d_warp_gain")]         pub warp_gain: f32,
    #[serde(default = "d_warp_lacunarity")]   pub warp_lacunarity: f32,
    #[serde(default = "d_warp_seed")]         pub seed: u32,
}
fn d_fract_iterations() -> usize { 2 }
fn d_fract_radii() -> (i32,i32,i32,i32) { (2,3,2,3) }
fn d_fract_inertia() -> f32 { 0.2 }
fn d_fract_boundary() -> bool { true }
fn d_warp_amp() -> f32 { 5.0 }
fn d_warp_freq() -> f32 { 1.0 / 20.0 }
fn d_warp_octaves() -> u32 { 3 }
fn d_warp_gain() -> f32 { 0.55 }
fn d_warp_lacunarity() -> f32 { 2.2 }
fn d_warp_seed() -> u32 { 42 }

impl Default for FractalConfig {
    fn default() -> Self {
        Self {
            iterations: d_fract_iterations(),
            radii: d_fract_radii(),
            inertia: d_fract_inertia(),
            boundary_only: d_fract_boundary(),
            warp_amp: d_warp_amp(),
            warp_freq: d_warp_freq(),
            warp_octaves: d_warp_octaves(),
            warp_gain: d_warp_gain(),
            warp_lacunarity: d_warp_lacunarity(),
            seed: d_warp_seed(),
        }
    }
}

// add to your MapTemplate
#[derive(Deserialize, Clone)]
pub struct MapTemplate {
    pub size: (i32, i32),
    pub player_spawns: super::template::PlayerSpawns,
    pub terrain: TerrainRules, 

    #[serde(default)] pub ley: LeyConfig,
    #[serde(default)] pub blend: BlendConfig,
    #[serde(default)] pub fractal: FractalConfig,
}

impl MapTemplate {
    pub fn from_file(path: &str) -> Self {
        let s = std::fs::read_to_string(path).expect("read template RON");
        ron::from_str(&s).expect("parse template RON")
    }
}

