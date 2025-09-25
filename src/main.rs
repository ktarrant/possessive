// mod terrain;

// use terrain::generate::{generate_all_phases};

// fn main() {
//     let tpl = terrain::template::MapTemplate::from_file("assets/maps/mt_breyer.ron");
//     // let tpl = terrain::template::MapTemplate::from_file("assets/maps/haunted_woods.ron");
//     let num_bases = 6usize;
//     let start_angle_deg = 0.0;

//     // choose any seed to stabilize the texture of terrain
//     let terrain_seed = 123456;

//     let (_p1, _ley, _final, _objects) = generate_all_phases(
//         &tpl,
//         num_bases,    // num_bases
//         start_angle_deg, // start_angle_deg
//         None,         // ley: use template (or defaults)
//         None,         // blend: use template (or defaults)
//         None,         // fractal: use template (or defaults)
//         terrain_seed, // terrain_seed
//         Some("out"),  // PNG dir or None
//     );
// }

// src/main.rs (example wiring)
mod units;

use bevy::prelude::*;
use units::base::*;
use units::world::{make_demo_map, TileMap, Terrain, TILE_SIZE, plants_regrow_system};
use units::creature::{CreatureBundle, WildlifeSimPlugin};
use units::simview::SimViewPlugin;

/// Big numbers? Tune here.
const N_SQUIRREL: usize = 150;
const N_DEER:     usize = 30;
const N_BIRD:     usize = 42;
const N_FOX:      usize = 12;
const N_BEAR:     usize = 4;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .insert_resource(make_demo_map(64, 64))
        .add_plugins(WildlifeSimPlugin) // decision → path → movement
        .add_plugins(SimViewPlugin)     // draw map + metrics UI
        .add_systems(Startup, spawn_load_test)
        .add_systems(Update, plants_regrow_system) // keeps berries/nuts rising
        .run();
}

/// Matching your demo speeds (avg tile/sec in your sim units)
fn default_speed(sp: Species) -> f32 {
    match sp {
        Species::Squirrel => 2.2,
        Species::Deer     => 2.0,
        Species::Bird     => 2.6,
        Species::Fox      => 2.4,
        Species::Bear     => 1.8,
    }
}

/// Species → allowed terrains
fn allowed_terrain(sp: Species, t: Terrain) -> bool {
    match sp {
        Species::Squirrel => matches!(t, Terrain::Forest),
        Species::Deer     => matches!(t, Terrain::Forest | Terrain::Grassland),
        Species::Bird     => matches!(t, Terrain::Forest | Terrain::Grassland),
        Species::Fox      => matches!(t, Terrain::Grassland),
        Species::Bear     => matches!(t, Terrain::Mountain),
    }
}

/// Pick a random cell that fits the species' home terrain (fallback to any).
fn random_cell_for_species(map: &TileMap, sp: Species) -> IVec2 {
    // Try up to a few hundred cells that match preferred terrain.
    for _ in 0..400 {
        let x = fastrand::i32(0..map.width);
        let y = fastrand::i32(0..map.height);
        let idx = (y * map.width + x) as usize;
        let t = map.tiles[idx].terrain;
        if allowed_terrain(sp, t) {
            return IVec2::new(x, y);
        }
    }
    // Fallback: truly any cell.
    IVec2::new(
        fastrand::i32(0..map.width),
        fastrand::i32(0..map.height),
    )
}

/// Center of a tile in world units, plus a small random jitter so things don’t overlap perfectly.
fn random_pos_in_cell(cell: IVec2) -> Vec2 {
    let center = Vec2::new(
        (cell.x as f32 + 0.5) * TILE_SIZE,
        (cell.y as f32 + 0.5) * TILE_SIZE,
    );
    let jitter = (Vec2::new(fastrand::f32() - 0.5, fastrand::f32() - 0.5)
        .normalize_or_zero()) * (0.35 * TILE_SIZE);
    center + jitter
}

pub fn spawn_load_test(mut commands: Commands, map: Res<TileMap>) {
    // helper to spawn many of one species
    let mut spawn_many = |count: usize, sp: Species| {
        let speed = default_speed(sp);
        for _ in 0..count {
            let cell = random_cell_for_species(&map, sp);
            let pos  = random_pos_in_cell(cell);
            commands.spawn(CreatureBundle::new(sp, pos, speed));
        }
    };

    spawn_many(N_SQUIRREL, Species::Squirrel);
    spawn_many(N_DEER,     Species::Deer);
    spawn_many(N_BIRD,     Species::Bird);
    spawn_many(N_FOX,      Species::Fox);
    spawn_many(N_BEAR,     Species::Bear);
}
