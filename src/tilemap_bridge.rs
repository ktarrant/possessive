use bevy::prelude::*; // brings IVec2/Vec2 types
use crate::terrain::landscape::{
    TERRAIN_GRASSLAND, TERRAIN_FOREST, TERRAIN_WATER, TERRAIN_MOUNTAIN,
};
use crate::terrain::objects::PlacedObject;
use crate::terrain::template::MapTemplate;
use crate::terrain::grid::Grid;

use crate::units::world::{TileMap, Tile, Terrain, TileObject};

#[inline]
fn tile_from_class(class: u8) -> Tile {
    let terrain = match class {
        TERRAIN_GRASSLAND => Terrain::Grassland,
        TERRAIN_FOREST    => Terrain::Forest,
        TERRAIN_WATER     => Terrain::Water,
        TERRAIN_MOUNTAIN  => Terrain::Mountain,
        _                 => Terrain::Grassland,
    };
    Tile {
        terrain,
        object: None,
        nuts: 0.0,
        berries: 0.0,
        nuts_max: 0.0,
        berries_max: 0.0,
    }
}

/// Convert a class grid into a TileMap (terrain only).
pub fn classes_to_tilemap(classes: &Grid<u8>) -> TileMap {
    let w = classes.w;
    let h = classes.h;
    // Fill with grass; replace each tile below.
    let mut map = TileMap::new(w, h, tile_from_class(TERRAIN_GRASSLAND));
    for y in 0..h {
        for x in 0..w {
            let i = (y * w + x) as usize;
            let class = *classes.get(x, y);
            map.tiles[i] = tile_from_class(class);
        }
    }
    map
}

/// Apply generated objects (trees/bushes/caves) into the TileMap tiles.
/// Caves are applied if your TileObject has a Cave variant; otherwise they’re skipped.
pub fn apply_objects_to_tilemap(
    map: &mut TileMap,
    tpl: &MapTemplate,
    objects: &[PlacedObject],
) {
    for o in objects {
        if let Some(i) = map.idx(o.pos) {
            let tdef = &tpl.objects.types[o.kind as usize];
            let lname = tdef.name.to_lowercase();

            // Decide which TileObject to set
            let obj = if lname.contains("tree") {
                Some(TileObject::Tree)
            } else if lname.contains("bush") {
                Some(TileObject::Bush)
            } else if lname.contains("cave") {
                // comment this line out if you didn't add TileObject::Cave
                Some(TileObject::Cave)
            } else {
                None
            };

            if let Some(objk) = obj {
                let tile = &mut map.tiles[i];
                tile.object = Some(objk);

                if objk == TileObject::Tree {
                    tile.nuts_max = 8.0;
                    tile.nuts = tile.nuts_max;
                } else if objk == TileObject::Bush {
                    tile.berries_max = 6.0;
                    tile.berries = tile.berries_max;
                }
            }
        }
    }
}
