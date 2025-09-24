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
    pub default: TerrainWeights,
    pub areas: Vec<TerrainArea>,
    #[serde(default = "default_shrine_grass_radius")]
    pub shrine_grass_radius: i32,
    #[serde(default = "default_clumps")]
    pub clumps: TerrainClumps,
}

// add to your MapTemplate
#[derive(Deserialize, Clone)]
pub struct MapTemplate {
    pub size: (i32, i32),
    pub player_spawns: super::template::PlayerSpawns,
    pub terrain: TerrainRules,                // <-- NEW
    // (keep anything else you already had)
}

impl MapTemplate {
    pub fn from_file(path: &str) -> Self {
        let s = std::fs::read_to_string(path).expect("read template RON");
        ron::from_str(&s).expect("parse template RON")
    }
}

