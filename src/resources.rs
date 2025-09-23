
use bevy::prelude::*;

#[derive(Resource, Default)]
pub struct Stockpiles { pub food: i32, pub wood: i32 }

#[derive(Resource, Default)]
pub struct InputState { pub move_axis: Vec2, pub press_possess: bool, pub press_raise: bool, pub press_kill: bool }
