// src/sim/world.rs
use bevy::prelude::*;

pub const TILE_SIZE: f32 = 1.0; // 1 world unit per tile (you can change this)

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Terrain { Forest, Grassland, Mountain, Water }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TileObject { Tree, Bush, Rock, Ruin }

#[derive(Clone, Copy, Debug)]
pub struct Tile {
    pub terrain: Terrain,
    pub object: Option<TileObject>,
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

    pub fn cell_at_world(&self, pos: Vec2) -> IVec2 {
        IVec2::new((pos.x / TILE_SIZE).floor() as i32, (pos.y / TILE_SIZE).floor() as i32)
    }

    pub fn terrain_at_world(&self, pos: Vec2) -> Terrain {
        self.tile_at_cell(self.cell_at_world(pos))
            .map(|t| t.terrain)
            .unwrap_or(Terrain::Grassland) // default if out of bounds
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
