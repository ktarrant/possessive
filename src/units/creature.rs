use bevy::prelude::*;
use super::base::{Position, Velocity, Kinematics};
use super::world::{TILE_SIZE};
use super::route::{Route, route_system};

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub enum Species { Squirrel, Deer, Bird, Fox, Bear }
// near your other enums
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BrainState { Wander, Forage, Eating, Hunt, Flee }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FoodKind { Nuts, Berries }

// Shared plant hysteresis you already added:
pub const HYSTERESIS_RATIO: f32 = 0.45;

// Predation
pub const PREDATOR_SENSE_RANGE: f32 = 10.0;         // tiles
pub const ATTACK_RANGE: f32         = 0.35 * TILE_SIZE;

pub const FLEE_SENSE_RANGE: f32 = 6.0;              // tiles
pub const FLEE_STEP: f32        = 6.0;              // tiles to dash when spooked

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

#[derive(Component, Debug)]
pub struct Brain {
    pub state: BrainState,
    pub replan_cd: f32,

    // forage plant targeting
    pub target_cell: Option<IVec2>,
    pub last_food_cell: Option<IVec2>,
    pub last_food_cooldown: f32,

    // NEW: hunt targeting
    pub target_entity: Option<Entity>,
}

impl Default for Brain {
    fn default() -> Self {
        Self {
            state: BrainState::Wander,
            replan_cd: 0.0,
            target_cell: None,
            last_food_cell: None,
            last_food_cooldown: 0.0,
            target_entity: None,
        }
    }
}

#[derive(Bundle, Debug)]
pub struct CreatureBundle {
    pub species: Species,
    pub pos: Position,
    pub vel: Velocity,
    pub kin: Kinematics,
    pub needs: Needs,   // NEW
    pub brain: Brain,
    pub route: super::route::Route,
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
            route: Route::default(),
        }
    }
}

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub enum SimSet { Decision, Route, Movement }

pub struct WildlifeSimPlugin;

impl Plugin for WildlifeSimPlugin {
    fn build(&self, app: &mut App) {
        app.configure_sets(Update, (SimSet::Decision, SimSet::Route, SimSet::Movement).chain())
            .add_systems(Update, needs_tick_system.before(SimSet::Decision))

            // DECISION set: prey flee first, then main decision, then hunt tracking
            .add_systems(Update, prey_flee_system.before(SimSet::Decision))
            .add_systems(Update, decision_system.in_set(SimSet::Decision))
            .add_systems(Update, hunt_track_system.in_set(SimSet::Decision))

            // PATH & MOVEMENT as you already have
            .add_systems(Update, route_system.in_set(SimSet::Route))
            .add_systems(Update, movement_system.in_set(SimSet::Movement))

            // Resolve attacks after movement (positions are up-to-date)
            .add_systems(Update, eat_system.after(SimSet::Movement))
            .add_systems(Update, attack_system.after(SimSet::Movement));
    }
}

// === Decision: forage if hungry; otherwise wander ===
fn decision_system(
    time: Res<Time>,
    map: Res<super::world::TileMap>,
    mut q: Query<(&Species, &Needs, &Position, &mut Brain, &mut Route)>,
    prey_scan: Query<(Entity, &Species, &Position)>, // read-only; used by predators
) {
    let dt = time.delta_secs();

    for (sp, needs, pos, mut brain, mut route) in &mut q {
        // tick cooldowns
        if brain.last_food_cooldown > 0.0 {
            brain.last_food_cooldown = (brain.last_food_cooldown - dt).max(0.0);
        }
        brain.replan_cd -= dt;

        // Eating freezes decisions; let eat_system decide exit
        if brain.state == BrainState::Eating { continue; }

        // --- Predator branch: hunt when hungry ---
        if is_predator(*sp) && needs.is_hungry() {
            // find nearest valid prey in sense range
            let mut best: Option<(Entity, Vec2, f32, Species)> = None;
            for (e, prey_sp, ppos) in &prey_scan {
                if !is_prey_of(*sp, *prey_sp) { continue; }
                let d2 = pos.p.distance_squared(ppos.p);
                if d2 > PREDATOR_SENSE_RANGE * PREDATOR_SENSE_RANGE { continue; }
                match best {
                    None => best = Some((e, ppos.p, d2, *prey_sp)),
                    Some((_, _, bd2, _)) if d2 < bd2 => best = Some((e, ppos.p, d2, *prey_sp)),
                    _ => {}
                }
            }

            if let Some((e, target_pos, _d2, _prey_sp)) = best {
                brain.state = BrainState::Hunt;
                brain.target_entity = Some(e);
                brain.target_cell = None;
                brain.replan_cd = 0.15; // track frequently
                route.desired_target = Some(map.clamp_target(target_pos));
                continue;
            }

            // no prey seen → hungry wander
            if brain.replan_cd <= 0.0 || route.desired_target.is_none() {
                let jitter = Vec2::new(fastrand::f32() - 0.5, fastrand::f32() - 0.5)
                    .normalize_or_zero() * 5.0;
                brain.state = BrainState::Hunt; // "searching"
                brain.replan_cd = 0.6;
                route.desired_target = Some(map.clamp_target(pos.p + jitter));
            }
            continue;
        }

        // --- Herbivore/bird: hungry → forage plants ---
        if needs.is_hungry() {
            if brain.replan_cd > 0.0 && route.desired_target.is_some() { continue; }

            if let Some((cell, _kind)) = nearest_food_cell(
                &map, *sp, pos.p,
                brain.last_food_cell,
                brain.last_food_cooldown > 0.0,
                HYSTERESIS_RATIO,
            ) {
                brain.state = BrainState::Forage;
                brain.target_cell = Some(cell);
                route.desired_target = Some(map.clamp_target(cell_center(cell)));
                brain.replan_cd = 0.75;
            } else {
                // hungry wander step
                let jitter = Vec2::new(fastrand::f32() - 0.5, fastrand::f32() - 0.5)
                    .normalize_or_zero() * 4.0;
                brain.state = BrainState::Forage;
                brain.target_cell = None;
                route.desired_target = Some(map.clamp_target(pos.p + jitter));
                brain.replan_cd = 0.75;
            }
        } else {
            // satiated → wander
            if brain.replan_cd > 0.0 && route.desired_target.is_some() { continue; }
            let jitter = Vec2::new(fastrand::f32() - 0.5, fastrand::f32() - 0.5)
                .normalize_or_zero() * 6.0;
            brain.state = BrainState::Wander;
            brain.target_cell = None;
            brain.target_entity = None;
            route.desired_target = Some(map.clamp_target(pos.p + jitter));
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

fn is_predator(sp: Species) -> bool {
    matches!(sp, Species::Fox | Species::Bear)
}

fn is_prey_of(pred: Species, prey: Species) -> bool {
    match pred {
        Species::Fox  => matches!(prey, Species::Squirrel | Species::Bird),
        Species::Bear => matches!(prey, Species::Squirrel | Species::Deer | Species::Fox),
        _ => false,
    }
}

fn meat_gain(prey: Species) -> f32 {
    match prey {
        Species::Bird     => 1.2,
        Species::Squirrel => 1.5,
        Species::Fox      => 2.2,
        Species::Deer     => 3.0,
        _ => 0.0,
    }
}

// === Needs drain ===
fn needs_tick_system(time: Res<Time>, mut q: Query<&mut Needs>) {
    let dt = time.delta_secs();
    for mut needs in &mut q {
        needs.satiation = (needs.satiation - needs.hunger_rate * dt).max(0.0);
    }
}

// === Movement === (unchanged except borrow-safe accel)
fn movement_system(
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

// === Eat: drain tile stock, refill satiation ===
fn eat_system(
    time: Res<Time>,
    mut map: ResMut<super::world::TileMap>,
    mut q: Query<(&Species, &Position, &mut Route, &mut Needs, &mut Brain)>,
) {
    let dt = time.delta_secs();
    const AVOID_SECONDS: f32 = 15.0;     // cooldown before reusing the same tile
    const EMPTY_EPS: f32    = 0.02;      // consider empty below this

    for (sp, pos, mut route, mut needs, mut brain) in &mut q {
        // arrival → start Eating (freeze movement)
        if brain.state == BrainState::Forage {
            if let Some(cell) = brain.target_cell {
                let center = cell_center(cell);
                if pos.p.distance_squared(center) < 0.05 {
                    brain.state = BrainState::Eating;
                    route.desired_target = None;
                    route.current_target = None; // freeze
                }
            }
        }

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
            route.desired_target = Some(center);
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
            route.desired_target = None;
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
            route.desired_target = None;                  // let decision pick a wander goal
        }
    }
}

fn hunt_track_system(
    map: Res<super::world::TileMap>,
    mut predators: Query<(&Position, &mut Brain, &mut Route), (With<Species>,)>,
    prey_q: Query<(&Position, &Species), With<Species>>,
) {
    for (ppos, mut brain, mut route) in &mut predators {
        if brain.state != BrainState::Hunt { continue; }
        let Some(target) = brain.target_entity else { continue; };

        if let Ok((prey_pos, _prey_sp)) = prey_q.get(target) {
            // update pursuit target to the prey's current position
            route.desired_target = Some(map.clamp_target(prey_pos.p));
        } else {
            // target despawned / lost: fall back to wander
            brain.state = BrainState::Wander;
            brain.target_entity = None;
            route.desired_target = Some(map.clamp_target(ppos.p));
        }
    }
}

fn attack_system(
    mut commands: Commands,
    mut predators: Query<(&Species, &Position, &mut Needs, &mut Brain, &mut Route)>,
    prey_q: Query<(Entity, &Species, &Position)>,
) {
    for (_pred_sp, ppos, mut needs, mut brain, mut route) in &mut predators {
        if brain.state != BrainState::Hunt { continue; }
        let Some(target) = brain.target_entity else { continue; };

        if let Ok((prey_e, prey_sp, prey_pos)) = prey_q.get(target) {
            let dist2 = ppos.p.distance_squared(prey_pos.p);
            if dist2 <= ATTACK_RANGE * ATTACK_RANGE {
                // "kill" the prey
                commands.entity(prey_e).despawn();

                // eat gain
                needs.satiation = (needs.satiation + meat_gain(*prey_sp)).min(needs.cap);

                // done hunting this target
                brain.target_entity = None;
                if needs.is_hungry() {
                    // keep hunting; decision_system will pick a new target
                    route.desired_target = None;
                } else {
                    // relax
                    brain.state = BrainState::Wander;
                    route.desired_target = None;
                }
            }
        } else {
            // lost target (already despawned)
            brain.state = BrainState::Wander;
            brain.target_entity = None;
            route.desired_target = None;
        }
    }
}

fn prey_flee_system(
    map: Res<super::world::TileMap>,
    predators: Query<(&Species, &Position), With<Species>>,
    mut prey: Query<(&Species, &Position, &mut Brain, &mut Route), With<Species>>,
) {
    // collect predator positions
    let preds: Vec<Vec2> = predators
        .iter()
        .filter(|(sp, _)| is_predator(**sp))
        .map(|(_, p)| p.p).collect();

    if preds.is_empty() { return; }

    for (sp, pos, mut brain, mut route) in &mut prey {
        if is_predator(*sp) { continue; } // predators don't flee in this simple pass

        // nearest predator
        let mut close = false;
        let mut away = Vec2::ZERO;
        let mut best_d2 = f32::MAX;

        for pp in &preds {
            let d2 = pos.p.distance_squared(*pp);
            if d2 < best_d2 {
                best_d2 = d2;
                away = (pos.p - *pp).normalize_or_zero();
            }
        }

        if best_d2 <= FLEE_SENSE_RANGE * FLEE_SENSE_RANGE {
            close = true;
        }

        if close {
            brain.state = BrainState::Flee;
            brain.target_cell = None;
            brain.target_entity = None;

            // dash away and clamp
            let flee_goal = map.clamp_target(pos.p + away * FLEE_STEP);
            route.desired_target = Some(flee_goal);
            // small replan so they can keep running if still threatened
            brain.replan_cd = 0.3;
        }
    }
}
