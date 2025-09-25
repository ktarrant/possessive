use bevy::prelude::*;

#[derive(Component, Default, Debug)]
pub struct Position { pub p: Vec2 }

#[derive(Component, Default, Debug)]
pub struct Velocity { pub v: Vec2 }

#[derive(Component, Debug)]
pub struct Kinematics { pub base_speed: f32 }

