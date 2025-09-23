
use bevy::prelude::*;
use crate::resources::InputState;

pub fn read_input(keys: Res<ButtonInput<KeyCode>>, mut input: ResMut<InputState>) {
    let mut axis = Vec2::ZERO;
    if keys.pressed(KeyCode::KeyW) { axis.y += 1.0; }
    if keys.pressed(KeyCode::KeyS) { axis.y -= 1.0; }
    if keys.pressed(KeyCode::KeyA) { axis.x -= 1.0; }
    if keys.pressed(KeyCode::KeyD) { axis.x += 1.0; }
    input.move_axis = axis.clamp_length_max(1.0);
    input.press_possess = keys.just_pressed(KeyCode::KeyQ);
    input.press_raise = keys.just_pressed(KeyCode::KeyE);
    input.press_kill = keys.just_pressed(KeyCode::KeyK);
}
