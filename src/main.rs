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
use bevy::prelude::*;
mod units { pub mod world; pub mod creature; }
use units::world::*;
use units::creature::*;

fn main() {
    App::new()
        .add_plugins(MinimalPlugins) // swap to DefaultPlugins in your game
        .insert_resource(make_demo_map())
        .add_plugins(WildlifeSimPlugin)
        .add_systems(Startup, spawn_demo)
        .run();
}

fn make_demo_map() -> TileMap {
    // 64x64 map with a watery stripe
    let mut map = TileMap::new(64, 64, Tile { terrain: Terrain::Grassland, object: None });
    for y in 0..map.height {
        for x in 0..map.width {
            let idx = (y * map.width + x) as usize;
            let terrain = if x % 13 == 0 { Terrain::Water }
            else if y % 17 == 0 { Terrain::Mountain }
            else if (x + y) % 7 == 0 { Terrain::Forest }
            else { Terrain::Grassland };
            map.tiles[idx].terrain = terrain;
        }
    }
    map
}

fn spawn_demo(mut commands: Commands) {
    // Drop in a few creatures
    commands.spawn(CreatureBundle::new(Species::Squirrel, Vec2::new(10.0, 10.0), 2.2));
    commands.spawn(CreatureBundle::new(Species::Deer,     Vec2::new(20.0, 12.0), 2.0));
    commands.spawn(CreatureBundle::new(Species::Bird,     Vec2::new(12.0, 22.0), 2.6));
    commands.spawn(CreatureBundle::new(Species::Fox,      Vec2::new(30.0, 18.0), 2.4));
    commands.spawn(CreatureBundle::new(Species::Bear,     Vec2::new(40.0, 40.0), 1.8));
}
