use bevy::prelude::*;
use super::world::{TILE_SIZE};

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]

pub enum Species { Squirrel, Deer, Bird, Fox, Bear }
// near your other enums
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BrainState { Wander, Forage, Eating }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FoodKind { Nuts, Berries }

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
// fn wants_nuts(sp: Species) -> bool {
//     matches!(sp, Species::Squirrel | Species::Bird)
// }
// fn wants_berries(sp: Species) -> bool {
//     matches!(sp, Species::Squirrel | Species::Bird | Species::Deer)
// }

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

#[derive(Component, Debug)]
pub struct Brain {
    pub state: BrainState,              // NEW
    pub desired_target: Option<Vec2>,
    pub replan_cd: f32,
    pub target_cell: Option<IVec2>,     // NEW: where we’re heading/eating
    pub last_food_cell: Option<IVec2>,  // NEW: hysteresis memory
    pub last_food_cooldown: f32,        // NEW: seconds to avoid last_food_cell
}

impl Default for Brain {
    fn default() -> Self {
        Self {
            state: BrainState::Wander,
            desired_target: None,
            replan_cd: 0.0,
            target_cell: None,
            last_food_cell: None,
            last_food_cooldown: 0.0,
        }
    }
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
            .add_systems(Update, needs_tick_system.before(SimSet::Decision))
            .add_systems(Update, decision_system.in_set(SimSet::Decision))
            .add_systems(Update, path_system.in_set(SimSet::Path))
            .add_systems(Update, movement_system.in_set(SimSet::Movement))
            .add_systems(Update, eat_system.after(SimSet::Movement));
    }
}

// === Decision: forage if hungry; otherwise wander ===
fn decision_system(
    time: Res<Time>,
    map: Res<super::world::TileMap>,
    mut q: Query<(&Species, &Needs, &Position, &mut Brain)>,
) {
    let dt = time.delta_secs();
    const HYSTERESIS_RATIO: f32 = 0.45;     // ~45% regrown before we consider the same tile again

    for (sp, needs, pos, mut brain) in &mut q {
        // cooldown tick
        if brain.last_food_cooldown > 0.0 { brain.last_food_cooldown = (brain.last_food_cooldown - dt).max(0.0); }
        brain.replan_cd -= dt;

        // If currently eating, do NOT replan here; `eat_system` decides when to leave Eating.
        if brain.state == BrainState::Eating {
            continue;
        }

        // Hungry → Forage; Satiated → Wander
        if needs.is_hungry() {
            // Replan only when needed / cooldown elapsed
            if brain.replan_cd > 0.0 && brain.desired_target.is_some() { continue; }

            if let Some((cell, _kind)) = nearest_food_cell(
                &map, *sp, pos.p,
                brain.last_food_cell,
                brain.last_food_cooldown > 0.0,
                HYSTERESIS_RATIO,
            ) {
                let goal = map.clamp_target(cell_center(cell));
                brain.state = BrainState::Forage;
                brain.target_cell = Some(cell);
                brain.desired_target = Some(goal);
                brain.replan_cd = 0.75; // snappier while hungry
                continue;
            }

            // No food found → small wander step (still hungry)
            let jitter = Vec2::new(fastrand::f32() - 0.5, fastrand::f32() - 0.5)
                .normalize_or_zero() * 4.0;
            brain.state = BrainState::Forage;
            brain.target_cell = None;
            brain.desired_target = Some(map.clamp_target(pos.p + jitter));
            brain.replan_cd = 0.75;
        } else {
            // Satiated: wander
            if brain.replan_cd > 0.0 && brain.desired_target.is_some() { continue; }
            let jitter = Vec2::new(fastrand::f32() - 0.5, fastrand::f32() - 0.5)
                .normalize_or_zero() * 6.0;
            brain.state = BrainState::Wander;
            brain.target_cell = None;
            brain.desired_target = Some(map.clamp_target(pos.p + jitter));
            brain.replan_cd = 2.0 + fastrand::f32() * 2.0;
        }
    }
}
fn tile_food_ratio_for_species(tile: &super::world::Tile, sp: Species) -> f32 {
    let nuts_r   = if tile.nuts_max    > 0.0 { tile.nuts    / tile.nuts_max    } else { 0.0 };
    let berries_r= if tile.berries_max > 0.0 { tile.berries / tile.berries_max } else { 0.0 };
    match sp {
        Species::Deer => berries_r,
        Species::Squirrel | Species::Bird => nuts_r.max(berries_r),
        _ => 0.0,
    }
}

fn cell_center(cell: IVec2) -> Vec2 {
    Vec2::new((cell.x as f32 + 0.5) * TILE_SIZE, (cell.y as f32 + 0.5) * TILE_SIZE)
}

fn nearest_food_cell(
    map: &super::world::TileMap,
    sp: Species,
    from: Vec2,
    avoid: Option<IVec2>,
    avoid_active: bool,
    hysteresis_ratio: f32,
) -> Option<(IVec2, FoodKind)> {
    let mut best: Option<(IVec2, f32, FoodKind)> = None;

    for y in 0..map.height {
        for x in 0..map.width {
            let cell = IVec2::new(x, y);
            let tile = &map.tiles[(y * map.width + x) as usize];

            // Hysteresis: skip the recently used tile until enough has regrown
            if avoid_active && Some(cell) == avoid {
                if tile_food_ratio_for_species(tile, sp) < hysteresis_ratio {
                    continue;
                }
            }

            // What can this species eat here?
            let mut kinds: [Option<FoodKind>; 2] = [None, None];
            let mut n = 0;
            if (matches!(sp, Species::Squirrel | Species::Bird)) && tile.nuts > 0.05 {
                kinds[n] = Some(FoodKind::Nuts); n += 1;
            }
            if (matches!(sp, Species::Squirrel | Species::Bird | Species::Deer)) && tile.berries > 0.05 {
                kinds[n] = Some(FoodKind::Berries); n += 1;
            }
            if n == 0 { continue; }

            // distance
            let c = cell_center(cell);
            let d2 = from.distance_squared(c);

            // choose primary kind (prefer the richer resource)
            let kind = match (kinds[0], kinds[1]) {
                (Some(FoodKind::Nuts), Some(FoodKind::Berries)) => {
                    if tile.nuts >= tile.berries { FoodKind::Nuts } else { FoodKind::Berries }
                }
                (Some(k), _) => k,
                _ => continue,
            };

            match best {
                None => best = Some((cell, d2, kind)),
                Some((_, bd2, _)) if d2 < bd2 => best = Some((cell, d2, kind)),
                _ => {}
            }
        }
    }

    best.map(|(c, _, k)| (c, k))
}

// === Needs drain ===
fn needs_tick_system(time: Res<Time>, mut q: Query<&mut Needs>) {
    let dt = time.delta_secs();
    for mut needs in &mut q {
        needs.satiation = (needs.satiation - needs.hunger_rate * dt).max(0.0);
    }
}

// === Path === (unchanged)
fn path_system(
    mut q: Query<(&Position, &mut Path, &mut Brain)>,
) {
    for (pos, mut path, mut brain) in &mut q {
        // read once into locals
        let desired = brain.desired_target;

        // keep your existing target sync logic
        match (path.current_target, desired) {
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

        // arrival → start Eating (freeze movement)
        if brain.state == BrainState::Forage {
            if let Some(cell) = brain.target_cell {
                let center = cell_center(cell);
                if pos.p.distance_squared(center) < 0.05 {
                    brain.state = BrainState::Eating;
                    brain.desired_target = None;
                    path.current_target = None; // freeze
                }
            }
        }
    }
}

// === Movement === (unchanged except borrow-safe accel)
fn movement_system(
    time: Res<Time>,
    map: Res<super::world::TileMap>,
    mut q: Query<(&mut Position, &mut Velocity, &Kinematics, &Path, &Brain)>,
) {
    let dt = time.delta_secs();
    let eps = 1e-3;
    let min = Vec2::splat(eps);
    let max = map.world_max() - Vec2::splat(eps);

    for (mut pos, mut vel, kin, path, brain) in &mut q {
        let desired = if brain.state == BrainState::Eating {
            Vec2::ZERO
        } else if let Some(goal) = path.current_target {
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
    mut map: ResMut<super::world::TileMap>,
    mut q: Query<(&Species, &Position, &mut Needs, &mut Brain)>,
) {
    let dt = time.delta_secs();
    const AVOID_SECONDS: f32 = 15.0;     // cooldown before reusing the same tile
    const EMPTY_EPS: f32    = 0.02;      // consider empty below this

    for (sp, pos, mut needs, mut brain) in &mut q {
        if brain.state != BrainState::Eating { continue; }

        // Must stand basically at the target food cell
        let Some(cell) = brain.target_cell else {
            // no cell? bail back to forage/wander next decision
            brain.state = BrainState::Wander;
            continue;
        };
        let center = cell_center(cell);
        if pos.p.distance_squared(center) > 0.05 {
            // drifted away: return to forage toward that cell
            brain.state = BrainState::Forage;
            brain.desired_target = Some(center);
            continue;
        }

        // Eat from the tile
        let Some(tile) = map.tile_at_cell_mut(cell) else {
            // cell vanished? go wander
            brain.state = BrainState::Wander;
            brain.target_cell = None;
            continue;
        };

        // total edible left for this species
        let mut edible = 0.0;
        if matches!(sp, Species::Squirrel | Species::Bird) { edible += tile.nuts.max(0.0); }
        if matches!(sp, Species::Squirrel | Species::Bird | Species::Deer) { edible += tile.berries.max(0.0); }

        if edible <= EMPTY_EPS {
            // Out of stock → remember this cell and avoid for a while
            brain.last_food_cell = Some(cell);
            brain.last_food_cooldown = AVOID_SECONDS;
            brain.state = BrainState::Forage;     // still hungry: find something else
            brain.desired_target = None;
            continue;
        }

        // Consume up to eat_rate*dt, preferring the richer resource
        let mut to_take = needs.eat_rate * dt;

        // helper to drain one resource
        let mut drain = |store: &mut f32, want: bool| -> f32 {
            if !want || *store <= 0.0 || to_take <= 0.0 { return 0.0; }
            let take = to_take.min(*store);
            *store -= take;
            to_take -= take;
            take
        };

        // choose order by which has more
        if matches!(sp, Species::Squirrel | Species::Bird) && tile.nuts >= tile.berries {
            let _ = drain(&mut tile.nuts, true);
            let _ = drain(&mut tile.berries, matches!(sp, Species::Squirrel | Species::Bird | Species::Deer));
        } else {
            let _ = drain(&mut tile.berries, matches!(sp, Species::Squirrel | Species::Bird | Species::Deer));
            let _ = drain(&mut tile.nuts, matches!(sp, Species::Squirrel | Species::Bird));
        }

        let gained = (needs.eat_rate * dt) - to_take;
        if gained > 0.0 {
            needs.satiation = (needs.satiation + gained).min(needs.cap);
        }

        // Exit Eating if full
        if (needs.satiation + 1e-4) >= needs.cap {
            brain.last_food_cell = Some(cell);            // remember where we just ate
            brain.last_food_cooldown = AVOID_SECONDS;     // avoid for a bit
            brain.state = BrainState::Wander;             // go relax
            brain.target_cell = None;
            brain.desired_target = None;                  // let decision pick a wander goal
        }
    }
}
