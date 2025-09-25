use bevy::prelude::*;
use super::base::{Position};

#[derive(Component, Debug, Default)]
pub struct Route {
    pub current_target: Option<Vec2>,
    pub desired_target: Option<Vec2>,
}

// === Route === (unchanged)
pub fn route_system(
    mut q: Query<(&Position, &mut Route)>,
) {
    for (pos, mut route) in &mut q {
        // keep your existing target sync logic
        match (route.current_target, route.desired_target) {
            (None, Some(goal)) => route.current_target = Some(goal),
            (Some(cur), Some(goal)) if cur.distance_squared(pos.p) < 0.25 => {
                route.current_target = Some(goal);
            }
            (Some(cur), Some(goal)) => {
                if cur.distance_squared(goal) > 9.0 {
                    route.current_target = Some(goal);
                }
            }
            _ => {}
        }
    }
}