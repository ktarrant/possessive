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
mod units { pub mod world; pub mod creature; pub mod simview; }
use units::world::*;
use units::creature::*;
use units::simview::SimViewPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .insert_resource(make_demo_map(64, 64))
        .add_plugins(WildlifeSimPlugin) // decision → path → movement
        .add_plugins(SimViewPlugin)     // draw map + metrics UI
        .add_systems(Startup, spawn_demo_creatures)
        .add_systems(Update, plants_regrow_system) // keeps berries/nuts rising
        .run();
}

fn spawn_demo_creatures(mut commands: Commands) {
    // sprinkle a few of each
    let spawns = [
        (Species::Squirrel, Vec2::new(10.0, 10.0), 2.2),
        (Species::Squirrel, Vec2::new(14.0, 16.0), 2.2),
        (Species::Deer,     Vec2::new(20.0, 12.0), 2.0),
        (Species::Bird,     Vec2::new(12.0, 22.0), 2.6),
        (Species::Fox,      Vec2::new(30.0, 18.0), 2.4),
        (Species::Bear,     Vec2::new(40.0, 40.0), 1.8),
    ];
    for (sp, p, speed) in spawns {
        commands.spawn(CreatureBundle::new(sp, p, speed));
    }
}
