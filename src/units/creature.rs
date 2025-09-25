use bevy::prelude::*;
use super::world::{TileMap, TILE_SIZE};

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub enum Species { Squirrel, Deer, Bird, Fox, Bear }

// NEW: basic needs/satiation
#[derive(Component, Debug)]
pub struct Needs {
    pub satiation: f32,        // 0..cap
    pub cap: f32,              // full tank
    pub hungry_threshold: f32, // below this = hungry
    pub hunger_rate: f32,      // drain per second
    pub eat_rate: f32,         // gain per second when eating
}

impl Needs {
    pub fn is_hungry(&self) -> bool { self.satiation < self.hungry_threshold }
}

// species diet flags (for now only plants; predators still wander)
fn wants_nuts(sp: Species) -> bool {
    matches!(sp, Species::Squirrel | Species::Bird)
}
fn wants_berries(sp: Species) -> bool {
    matches!(sp, Species::Squirrel | Species::Bird | Species::Deer)
}

// species presets
fn default_needs(sp: Species) -> Needs {
    match sp {
        Species::Squirrel => Needs { satiation: 2.5, cap: 4.0, hungry_threshold: 2.0, hunger_rate: 0.05, eat_rate: 0.8 },
        Species::Deer     => Needs { satiation: 3.0, cap: 6.0, hungry_threshold: 2.5, hunger_rate: 0.07, eat_rate: 1.0 },
        Species::Bird     => Needs { satiation: 2.0, cap: 3.5, hungry_threshold: 1.8, hunger_rate: 0.04, eat_rate: 0.6 },
        Species::Fox      => Needs { satiation: 2.5, cap: 5.0, hungry_threshold: 2.2, hunger_rate: 0.06, eat_rate: 0.9 },
        Species::Bear     => Needs { satiation: 3.5, cap: 8.0, hungry_threshold: 3.0, hunger_rate: 0.08, eat_rate: 1.2 },
    }
}

#[derive(Component, Default, Debug)] pub struct Position { pub p: Vec2 }
#[derive(Component, Default, Debug)] pub struct Velocity { pub v: Vec2 }
#[derive(Component, Debug)] pub struct Kinematics { pub base_speed: f32 }

#[derive(Component, Debug, Default)]
pub struct Brain {
    pub desired_target: Option<Vec2>,
    pub replan_cd: f32,
}

#[derive(Component, Debug, Default)]
pub struct Path { pub current_target: Option<Vec2> }

#[derive(Bundle, Debug)]
pub struct CreatureBundle {
    pub species: Species,
    pub pos: Position,
    pub vel: Velocity,
    pub kin: Kinematics,
    pub needs: Needs,   // NEW
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
            needs: default_needs(species), // NEW
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
        app.configure_sets(Update, (SimSet::Decision, SimSet::Path, SimSet::Movement).chain())
            // NEW: drain satiation before decisions
            .add_systems(Update, needs_tick_system.before(SimSet::Decision))
            .add_systems(Update, decision_system.in_set(SimSet::Decision))
            .add_systems(Update, path_system.in_set(SimSet::Path))
            .add_systems(Update, movement_system.in_set(SimSet::Movement))
            // NEW: eating happens after we move (so we can arrive then eat)
            .add_systems(Update, eat_system.after(SimSet::Movement));
    }
}

// === Decision: forage if hungry; otherwise wander ===

fn decision_system(
    time: Res<Time>,
    map: Res<TileMap>,
    mut q: Query<(&Species, &Needs, &Position, &mut Brain)>,
) {
    let dt = time.delta_secs();
    for (sp, needs, pos, mut brain) in &mut q {
        brain.replan_cd -= dt;
        if brain.replan_cd > 0.0 && brain.desired_target.is_some() { continue; }

        if needs.is_hungry() {
            // find nearest viable food tile; fallback to short wander if none
            if let Some(cell) = nearest_food_cell(&map, *sp, pos.p) {
                brain.desired_target = Some(cell_center(cell));
                brain.replan_cd = 1.0; // quick replan cadence while hungry
                continue;
            }
        }

        // Wander target (small jitter around current pos)
        let jitter = Vec2::new(fastrand::f32() - 0.5, fastrand::f32() - 0.5)
            .normalize_or_zero() * 6.0;
        brain.desired_target = Some(pos.p + jitter);
        brain.replan_cd = 2.0 + fastrand::f32() * 2.0;
    }
}

// find the nearest tile that provides something this species eats and has stock
fn nearest_food_cell(map: &TileMap, sp: Species, from: Vec2) -> Option<IVec2> {
    let mut best: Option<(IVec2, f32)> = None;
    for y in 0..map.height {
        for x in 0..map.width {
            let cell = IVec2::new(x, y);
            let tile = &map.tiles[(y * map.width + x) as usize];
            // check resource
            let mut ok = false;
            if wants_nuts(sp) && tile.nuts > 0.1 { ok = true; }
            if wants_berries(sp) && tile.berries > 0.1 { ok = true; }
            if !ok { continue; }
            let c = cell_center(cell);
            let d2 = from.distance_squared(c);
            match best {
                None => best = Some((cell, d2)),
                Some((_, bd2)) if d2 < bd2 => best = Some((cell, d2)),
                _ => {}
            }
        }
    }
    best.map(|(cell, _)| cell)
}

fn cell_center(cell: IVec2) -> Vec2 {
    Vec2::new((cell.x as f32 + 0.5) * TILE_SIZE, (cell.y as f32 + 0.5) * TILE_SIZE)
}

// === Needs drain ===
fn needs_tick_system(time: Res<Time>, mut q: Query<&mut Needs>) {
    let dt = time.delta_secs();
    for mut needs in &mut q {
        needs.satiation = (needs.satiation - needs.hunger_rate * dt).max(0.0);
    }
}

// === Path === (unchanged)
fn path_system(mut q: Query<(&mut Path, &Brain, &Position)>) {
    for (mut path, brain, pos) in &mut q {
        match (path.current_target, brain.desired_target) {
            (None, Some(goal)) => path.current_target = Some(goal),
            (Some(cur), Some(goal)) if cur.distance_squared(pos.p) < 0.25 => {
                path.current_target = Some(goal);
            }
            (Some(cur), Some(goal)) => {
                if cur.distance_squared(goal) > 9.0 {
                    path.current_target = Some(goal);
                }
            }
            _ => {}
        }
    }
}

// === Movement === (unchanged except borrow-safe accel)
fn movement_system(
    time: Res<Time>,
    map: Res<TileMap>,
    mut q: Query<(&mut Position, &mut Velocity, &Kinematics, &Path)>,
) {
    let dt = time.delta_secs();
    for (mut pos, mut vel, kin, path) in &mut q {
        let desired = if let Some(goal) = path.current_target {
            let dir = (goal - pos.p).normalize_or_zero();
            let mult = map.speed_multiplier(pos.p);
            dir * (kin.base_speed * mult)
        } else { Vec2::ZERO };

        let accel = 10.0;
        let cur_v = vel.v;
        vel.v = cur_v + (desired - cur_v) * (accel * dt);
        pos.p += vel.v * dt;

        if let Some(goal) = path.current_target {
            if pos.p.distance_squared(goal) < 0.01 {
                pos.p = goal;
                vel.v = Vec2::ZERO;
            }
        }
    }
}

// === Eat: drain tile stock, refill satiation ===
fn eat_system(
    time: Res<Time>,
    mut map: ResMut<TileMap>,
    mut q: Query<(&Species, &Position, &mut Needs)>,
) {
    let dt = time.delta_secs();
    for (sp, pos, mut needs) in &mut q {
        if !needs.is_hungry() { continue; }

        // only eat if standing very close to the center of a food tile
        let cell = map.cell_at_world(pos.p);
        if let Some(tile) = map.tile_at_cell_mut(cell) {
            let mut ate = 0.0;
            if wants_nuts(*sp) && tile.nuts > 0.0 {
                let take = (needs.eat_rate * dt).min(tile.nuts);
                tile.nuts -= take;
                ate += take;
            }
            if wants_berries(*sp) && tile.berries > 0.0 {
                let take = (needs.eat_rate * dt).min(tile.berries);
                tile.berries -= take;
                ate += take;
            }
            if ate > 0.0 {
                needs.satiation = (needs.satiation + ate).min(needs.cap);
            }
        }
    }
}
