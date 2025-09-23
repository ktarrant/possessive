
use bevy::prelude::*;
use crate::components::*;

pub fn spawn_world(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());

    // Hero
    commands.spawn((
        SpriteBundle {
            sprite: Sprite { color: Color::srgb_u8(75,230,120), custom_size: Some(Vec2::splat(24.0)), ..default() },
            transform: Transform::from_xyz(0.0, 0.0, 1.0),
            ..default()
        },
        Hero {
            mana: 120.0, max_mana: 120.0, regen: 6.0, class_: HeroClass::Necromancer,
            move_speed: 220.0, possess_cost: 30.0, raise_cost: 28.0, cast_range: 160.0,
        },
        Vel(Vec2::ZERO),
        Name::new("Hero"),
    ));

    // Squirrel
    commands.spawn((
        SpriteBundle {
            sprite: Sprite { color: Color::srgb_u8(204,153,102), custom_size: Some(Vec2::splat(16.0)), ..default() },
            transform: Transform::from_xyz(150.0, 0.0, 0.0),
            ..default()
        },
        Creature { species: Species::Squirrel, alive: true, possessed: false, undead: false },
        Worker { kind: HarvestKind::Food, interval: 2.5, timer: 2.5 },
        Name::new("Squirrel"),
    ));

    // Bear
    commands.spawn((
        SpriteBundle {
            sprite: Sprite { color: Color::srgb_u8(102,76,51), custom_size: Some(Vec2::splat(20.0)), ..default() },
            transform: Transform::from_xyz(250.0, 0.0, 0.0),
            ..default()
        },
        Creature { species: Species::Bear, alive: true, possessed: false, undead: false },
        Worker { kind: HarvestKind::Wood, interval: 3.0, timer: 3.0 },
        Name::new("Bear"),
    ));

    // Bird
    commands.spawn((
        SpriteBundle {
            sprite: Sprite { color: Color::srgb_u8(180,180,255), custom_size: Some(Vec2::splat(16.0)), ..default() },
            transform: Transform::from_xyz(320.0, 80.0, 0.0),
            ..default()
        },
        Creature { species: Species::Bird, alive: true, possessed: false, undead: false },
        Name::new("Bird"),
    ));

    // Shrine
    commands.spawn((
        SpriteBundle {
            sprite: Sprite { color: Color::srgb(0.6,0.4,0.9), custom_size: Some(Vec2::splat(20.0)), ..default() },
            transform: Transform::from_xyz(420.0, 0.0, 0.0),
            ..default()
        },
        Shrine { regen_bonus: 6.0, radius: 64.0 },
        Name::new("Shrine"),
    ));
}
