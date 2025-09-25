// src/sim/creature.rs
use bevy::prelude::*;
use super::world::TileMap;

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub enum Species { Squirrel, Deer, Bird, Fox, Bear }

#[derive(Component, Default, Debug)]
pub struct Position { pub p: Vec2 }

#[derive(Component, Default, Debug)]
pub struct Velocity { pub v: Vec2 }

#[derive(Component, Debug)]
pub struct Kinematics {
    /// max ground speed on neutral terrain (units/sec)
    pub base_speed: f32,
}

#[derive(Component, Debug, Default)]
pub struct Brain {
    /// Where this creature *wants* to go (set by decision step)
    pub desired_target: Option<Vec2>,
    /// Cooldown before re-planning a path (seconds)
    pub replan_cd: f32,
}

#[derive(Component, Debug, Default)]
pub struct Path {
    /// Minimal pathing: just head straight to target; later we'll store waypoints.
    pub current_target: Option<Vec2>,
}

#[derive(Bundle, Debug)]
pub struct CreatureBundle {
    pub species: Species,
    pub pos: Position,
    pub vel: Velocity,
    pub kin: Kinematics,
    pub brain: Brain,
    pub path: Path,
}

impl CreatureBundle {
    pub fn new(species: Species, pos: Vec2, base_speed: f32) -> Self {
        Self {
            species,
            pos: Position { p: pos },
            vel: Velocity::default(),
            kin: Kinematics { base_speed },
            brain: Brain::default(),
            path: Path::default(),
        }
    }
}

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub enum SimSet { Decision, Path, Movement }

pub struct WildlifeSimPlugin;

impl Plugin for WildlifeSimPlugin {
    fn build(&self, app: &mut App) {
        app.configure_sets(
                Update,
                (SimSet::Decision, SimSet::Path, SimSet::Movement).chain(),
            )
            .add_systems(Update, decision_system.in_set(SimSet::Decision))
            .add_systems(Update, path_system.in_set(SimSet::Path))
            .add_systems(Update, movement_system.in_set(SimSet::Movement));
    }
}

/// DECISION: super simple for now—wander toward a soft random goal.
/// Later: plug in your FSM (Forage/Hunt/SeekMate/etc).
fn decision_system(
    time: Res<Time>,
    mut q: Query<(&mut Brain, &Position), With<Species>>,
) {
    let dt = time.delta_secs();

    for (mut brain, pos) in &mut q {
        // replan every 2–4 seconds when idle/no target or cooldown elapsed
        brain.replan_cd -= dt;
        let needs_target = brain.desired_target.is_none() || brain.replan_cd <= 0.0;

        if needs_target {
            // Pick a small random offset around current position for a "wander" feel.
            // (Replace this block with: choose food/mate/prey/home-terrain cell)
            let jitter = Vec2::new(fastrand::f32() - 0.5, fastrand::f32() - 0.5).normalize_or_zero() * 6.0;
            brain.desired_target = Some(pos.p + jitter);
            brain.replan_cd = 2.0 + fastrand::f32() * 2.0;
        }
    }
}

/// PATH: trivial “path” = go directly to desired_target.
/// Later we’ll swap this to A* / flow-field with terrain costs.
fn path_system(
    mut q: Query<(&mut Path, &Brain, &Position), With<Species>>,
) {
    for (mut path, brain, pos) in &mut q {
        match (path.current_target, brain.desired_target) {
            (None, Some(goal)) => path.current_target = Some(goal),
            (Some(cur), Some(goal)) if cur.distance_squared(pos.p) < 0.25 => {
                // If we basically reached the old target, accept the new one.
                path.current_target = Some(goal);
            }
            (Some(_), Some(goal)) => {
                // Optional: if goal changed significantly, update early
                if let Some(cur) = path.current_target {
                    if cur.distance_squared(goal) > 9.0 {
                        path.current_target = Some(goal);
                    }
                }
            }
            (Some(cur), None) => {
                // keep heading to current target
                let _ = cur;
            }
            (None, None) => { /* no-op */ }
        }
    }
}

/// MOVEMENT: integrate velocity → position with terrain speed multiplier.
fn movement_system(
    time: Res<Time>,
    map: Res<TileMap>,
    mut q: Query<(&mut Position, &mut Velocity, &Kinematics, &Path), With<Species>>,
) {
    let dt = time.delta_secs();

    for (mut pos, mut vel, kin, path) in &mut q {
        let desired = if let Some(goal) = path.current_target {
            let dir = (goal - pos.p).normalize_or_zero();
            let terrain_mult = map.speed_multiplier(pos.p);
            dir * (kin.base_speed * terrain_mult)
        } else {
            Vec2::ZERO
        };

        // Simple critically-damped acceleration toward desired speed.
        let accel = 10.0;
        let cur_v = vel.v; // Vec2 is Copy; this is a cheap copy
        let delta_v = (desired - cur_v) * (accel * dt);
        vel.v = cur_v + delta_v;

        // Integrate
        pos.p += vel.v * dt;

        // Snap if close enough to target to avoid jitter
        if let Some(goal) = path.current_target {
            if pos.p.distance_squared(goal) < 0.01 {
                pos.p = goal;
                vel.v = Vec2::ZERO;
            }
        }
    }
}
