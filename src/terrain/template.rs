use serde::Deserialize;

#[derive(Deserialize, Clone)]
pub enum Symmetry { None, Rotational(u8) }

#[derive(Deserialize, Clone)]
pub struct ElevationLayer {
    pub freq: f32,
    pub amp: f32,
    pub octaves: u8,
    pub ridged: bool,
}

#[derive(Deserialize, Clone)]
pub struct Elevation {
    pub base_level: f32,
    pub falloff: f32,
    pub layers: Vec<ElevationLayer>,
}

#[derive(Deserialize, Clone)]
pub struct PlayerStartRules {
    pub symmetry: super::template::Symmetry,
    pub min_between_starts: i32,
    pub min_to_center: i32,
    pub plateau_radius: i32,
    pub jitter: i32,
}

#[derive(Deserialize, Clone)]
pub struct Fairness {
    pub primary_gold: u8,
    pub secondary_gold: u8,
    pub berries: u8,
    pub max_distance_jitter: i32,
}

#[derive(Deserialize, Clone)]
pub struct Densities {
    pub forest_density: f32,
    pub stragglers_per_player: u8,
}

#[derive(Deserialize, Clone)]
pub struct MapTemplate {
    pub name: String,
    pub size: (i32, i32),
    pub elevation: Elevation,
    pub player_start_rules: PlayerStartRules,
    pub fairness: Fairness,
    pub densities: Densities,
    // simple toggles for this starter module
    pub water: bool,
}

impl MapTemplate {
    pub fn from_file(path: &str) -> Self {
        let s = std::fs::read_to_string(path).expect("read template");
        ron::from_str(&s).expect("parse RON")
    }
}
