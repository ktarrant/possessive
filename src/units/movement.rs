
use bevy::prelude::*;
use super::base::{Position, Velocity, Kinematics, BrainState, Brain};
use super::route::{Route};

// === Movement ===
pub fn movement_system(
    time: Res<Time>,
    map: Res<super::world::TileMap>,
    mut q: Query<(&mut Position, &mut Velocity, &Kinematics, &Route, &Brain)>,
) {
    let dt = time.delta_secs();
    let eps = 1e-3;
    let min = Vec2::splat(eps);
    let max = map.world_max() - Vec2::splat(eps);

    for (mut pos, mut vel, kin, route, brain) in &mut q {
        let desired = if brain.state == BrainState::Eating {
            Vec2::ZERO
        } else if let Some(goal) = route.current_target {
            let dir = (goal - pos.p).normalize_or_zero();
            let mult = map.speed_multiplier(pos.p);
            dir * (kin.base_speed * mult)
        } else {
            Vec2::ZERO
        };

        let accel = 10.0;
        let cur_v = vel.v;
        vel.v = cur_v + (desired - cur_v) * (accel * dt);

        let mut new_p = pos.p + vel.v * dt;

        // clamp to map edges (your earlier fix)
        if new_p.x < min.x { new_p.x = min.x; vel.v.x = 0.0; }
        if new_p.x > max.x { new_p.x = max.x; vel.v.x = 0.0; }
        if new_p.y < min.y { new_p.y = min.y; vel.v.y = 0.0; }
        if new_p.y > max.y { new_p.y = max.y; vel.v.y = 0.0; }

        pos.p = new_p;

        if let Some(goal) = route.current_target {
            if pos.p.distance_squared(goal) < 0.01 {
                pos.p = goal;
                vel.v = Vec2::ZERO;
            }
        }
    }
}
