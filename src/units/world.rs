use bevy::prelude::*;

pub const TILE_SIZE: f32 = 1.0; // sim unit per tile

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Terrain { Forest, Grassland, Mountain, Water }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TileObject { Tree, Bush }

#[derive(Clone, Copy, Debug)]
pub struct Tile {
    pub terrain: Terrain,
    pub object: Option<TileObject>,
    // simple food stocks (only meaningful if object is Tree/Bush)
    pub nuts: f32,
    pub berries: f32,
    pub nuts_max: f32,
    pub berries_max: f32,
}

#[derive(Resource)]
pub struct TileMap {
    pub width: i32,
    pub height: i32,
    pub tiles: Vec<Tile>,
}

impl TileMap {
    pub fn new(width: i32, height: i32, fill: Tile) -> Self {
        let len = (width * height) as usize;
        Self { width, height, tiles: vec![fill; len] }
    }

    #[inline]
    pub fn idx(&self, cell: IVec2) -> Option<usize> {
        if cell.x >= 0 && cell.y >= 0 && cell.x < self.width && cell.y < self.height {
            Some((cell.y * self.width + cell.x) as usize)
        } else { None }
    }

    pub fn tile_at_cell(&self, cell: IVec2) -> Option<&Tile> {
        self.idx(cell).map(|i| &self.tiles[i])
    }

    pub fn tile_at_cell_mut(&mut self, cell: IVec2) -> Option<&mut Tile> {
        if let Some(i) = self.idx(cell) { Some(&mut self.tiles[i]) } else { None }
    }

    pub fn cell_at_world(&self, pos: Vec2) -> IVec2 {
        IVec2::new((pos.x / TILE_SIZE).floor() as i32, (pos.y / TILE_SIZE).floor() as i32)
    }

    pub fn terrain_at_world(&self, pos: Vec2) -> Terrain {
        self.tile_at_cell(self.cell_at_world(pos))
            .map(|t| t.terrain)
            .unwrap_or(Terrain::Grassland)
    }

    /// Movement multiplier (<= 1.0 slows you down)
    pub fn speed_multiplier(&self, pos: Vec2) -> f32 {
        match self.terrain_at_world(pos) {
            Terrain::Water     => 0.5,
            Terrain::Mountain  => 0.8,
            Terrain::Forest    => 1.0,
            Terrain::Grassland => 1.0,
        }
    }
}

// --- plant regen ---

const TREE_NUTS_MAX: f32 = 8.0;
const TREE_NUTS_REGEN_PER_SEC: f32 = 0.03;

const BUSH_BERRIES_MAX: f32 = 6.0;
const BERRIES_REGEN_PER_SEC: f32 = 0.04;

pub fn plants_regrow_system(mut map: ResMut<TileMap>, time: Res<Time>) {
    let dt = time.delta_secs();
    for t in &mut map.tiles {
        match t.object {
            Some(TileObject::Tree) => {
                t.nuts = (t.nuts + TREE_NUTS_REGEN_PER_SEC * dt).min(t.nuts_max);
            }
            Some(TileObject::Bush) => {
                t.berries = (t.berries + BERRIES_REGEN_PER_SEC * dt).min(t.berries_max);
            }
            _ => {}
        }
    }
}

// --- demo map helpers ---

fn empty_tile(terrain: Terrain) -> Tile {
    Tile {
        terrain,
        object: None,
        nuts: 0.0,
        berries: 0.0,
        nuts_max: 0.0,
        berries_max: 0.0,
    }
}

pub fn make_demo_map(width: i32, height: i32) -> TileMap {
    let mut map = TileMap::new(width, height, empty_tile(Terrain::Grassland));
    // simple terrain pattern
    for y in 0..map.height {
        for x in 0..map.width {
            let idx = (y * map.width + x) as usize;
            let terrain = if x % 13 == 0 { Terrain::Water }
            else if y % 17 == 0 { Terrain::Mountain }
            else if (x + y) % 7 == 0 { Terrain::Forest }
            else { Terrain::Grassland };
            map.tiles[idx] = empty_tile(terrain);
        }
    }
    // sprinkle trees/bushes on suitable terrain
    for y in 0..map.height {
        for x in 0..map.width {
            let idx = (y * map.width + x) as usize;
            let terrain = map.tiles[idx].terrain;
            let roll = fastrand::f32();
            match terrain {
                Terrain::Forest => {
                    if roll < 0.10 {
                        map.tiles[idx].object = Some(TileObject::Tree);
                        map.tiles[idx].nuts_max = TREE_NUTS_MAX;
                        map.tiles[idx].nuts = TREE_NUTS_MAX;
                    } else if roll < 0.18 {
                        map.tiles[idx].object = Some(TileObject::Bush);
                        map.tiles[idx].berries_max = BUSH_BERRIES_MAX;
                        map.tiles[idx].berries = BUSH_BERRIES_MAX;
                    }
                }
                Terrain::Grassland => {
                    if roll < 0.06 {
                        map.tiles[idx].object = Some(TileObject::Bush);
                        map.tiles[idx].berries_max = BUSH_BERRIES_MAX;
                        map.tiles[idx].berries = BUSH_BERRIES_MAX;
                    }
                }
                _ => {}
            }
        }
    }
    map
}

/// Sum of nuts and berries currently available on the map.
pub fn food_totals(map: &TileMap) -> (f32, f32) {
    map.tiles.iter().fold((0.0, 0.0), |(n, b), t| (n + t.nuts, b + t.berries))
}
