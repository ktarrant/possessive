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
pub struct MapTemplate {
    /// Map dimensions in tiles (width, height).
    pub size: (i32, i32),
    /// Parameters for Phase 1 player base ring.
    pub player_spawns: PlayerSpawns,
}

impl MapTemplate {
    pub fn from_file(path: &str) -> Self {
        let s = std::fs::read_to_string(path).expect("read template RON");
        ron::from_str(&s).expect("parse template RON")
    }
}
