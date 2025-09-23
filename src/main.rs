
use bevy::prelude::*;

mod components;
mod resources;
mod spawning;
mod input;
mod systems;

use resources::*;
use spawning::spawn_world;
use input::read_input;
use systems::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Possessive (Bevy Starter)".into(),
                resolution: (1280., 720.).into(),
                ..default()
            }),
            ..default()
        }))
        .insert_resource(Stockpiles::default())
        .insert_resource(InputState::default())
        .add_systems(Startup, spawn_world)
        .add_systems(Update, (
            read_input,
            move_hero,
            shrine_aura_and_regen,
            debug_kill_nearest,
            possess_system,
            raise_dead_system,
            harvest_tick_system,
            hud_print_system,
        ))
        .run();
}
