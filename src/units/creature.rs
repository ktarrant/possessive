use bevy::prelude::*;
use std::collections::HashMap;
use super::base::{Position, Velocity, Kinematics, Species, BrainState, Brain};
use super::world::{TILE_SIZE};
use super::route::{Route, route_system};
use super::forage::{forage_system, cell_center, is_predator, is_prey_of};
use super::movement::{movement_system};

// Predation
pub const ATTACK_RANGE: f32         = 0.35 * TILE_SIZE;

pub const FLEE_SENSE_RANGE: f32 = 6.0;              // tiles
pub const FLEE_STEP: f32        = 6.0;              // tiles to dash when spooked

// Reproduction
pub const MATE_RANGE_TILES: f32 = 0.75;  // how close they must be (in tiles)
pub const REPRO_COOLDOWN: f32   = 30.0;  // seconds before parents can mate again
pub const OFFSPRING_JITTER: f32 = 0.15;  // in tiles, to avoid perfect overlap

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
pub struct Repro {
    pub timer: f32,     // seconds remaining on cooldown (<= 0.0 means ready)
}
impl Default for Repro {
    fn default() -> Self { Self { timer: 0.0 } }
}
impl Repro {
    #[inline] pub fn ready(&self) -> bool { self.timer <= 0.0 }
}

#[derive(Bundle, Debug)]
pub struct CreatureBundle {
    pub species: Species,
    pub pos: Position,
    pub vel: Velocity,
    pub kin: Kinematics,
    pub needs: Needs,
    pub brain: Brain,
    pub route: Route,
    pub repro: Repro,
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
            repro: Repro::default(),
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
            .add_systems(Update, repro_cooldown_system.before(SimSet::Decision))

            // DECISION set: prey flee first, then main decision, then hunt tracking
            .add_systems(Update, prey_flee_system.before(SimSet::Decision))
            .add_systems(Update, decision_system.in_set(SimSet::Decision))
            .add_systems(Update, forage_system.in_set(SimSet::Decision))

            // PATH & MOVEMENT as you already have
            .add_systems(Update, route_system.in_set(SimSet::Route))
            .add_systems(Update, movement_system.in_set(SimSet::Movement))

            // Resolve attacks after movement (positions are up-to-date)
            .add_systems(Update, mating_system.after(SimSet::Movement)) 
            .add_systems(Update, eat_system.after(SimSet::Movement))
            .add_systems(Update, attack_system.after(SimSet::Movement));
    }
}

// === Decision: forage if hungry; otherwise wander ===
fn decision_system(
    time: Res<Time>,
    map: Res<super::world::TileMap>,
    mut q: Query<(&Needs, &Position, &mut Brain)>
) {
    let dt = time.delta_secs();

    for (needs, pos, mut brain) in &mut q {
        brain.replan_cd -= dt;

        if brain.replan_cd > 0.0 { continue; }

        // Eating freezes decisions; let eat_system decide exit
        if brain.state == BrainState::Wander {
            if needs.is_hungry() {
                brain.replan_cd = 0.75;
                brain.state = BrainState::Forage;
            } else {
                // satiated → wander
                if brain.replan_cd > 0.0 && brain.desired_target.is_some() { continue; }
                let jitter = Vec2::new(fastrand::f32() - 0.5, fastrand::f32() - 0.5)
                    .normalize_or_zero() * 6.0;
                brain.desired_target = Some(map.clamp_target(pos.p + jitter));
                brain.replan_cd = 2.0 + fastrand::f32() * 2.0;
            }
            continue;
        }
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

// === Eat: drain tile stock, refill satiation ===
fn eat_system(
    time: Res<Time>,
    mut map: ResMut<super::world::TileMap>,
    mut q: Query<(&Species, &Position, &mut Route, &mut Needs, &mut Brain)>,
) {
    let dt = time.delta_secs();
    const EMPTY_EPS: f32    = 0.02;      // consider empty below this

    for (sp, pos, mut route, mut needs, mut brain) in &mut q {
        // arrival → start Eating (freeze movement)
        if brain.state == BrainState::Forage {
            if let Some(cell) = brain.target_cell {
                let center = cell_center(cell);
                if pos.p.distance_squared(center) < 0.05 {
                    brain.state = BrainState::Eating;
                    brain.desired_target = None;
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
            // brain.last_food_cell = Some(cell);
            // brain.last_food_cooldown = AVOID_SECONDS;
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
            // brain.last_food_cell = Some(cell);            // remember where we just ate
            // brain.last_food_cooldown = AVOID_SECONDS;     // avoid for a bit
            brain.state = BrainState::Wander;             // go relax
            brain.target_cell = None;
            brain.desired_target = None;                  // let decision pick a wander goal
        }
    }
}

// === Attack and Flee ===
fn attack_system(
    mut commands: Commands,
    mut predators: Query<(&Species, &Position, &mut Needs, &mut Brain)>,
    prey_q: Query<(Entity, &Species, &Position)>,
) {
    for (_pred_sp, ppos, mut needs, mut brain) in &mut predators {
        if brain.state != BrainState::Forage { continue; }
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
                    brain.desired_target = None;
                } else {
                    // relax
                    brain.state = BrainState::Wander;
                    brain.desired_target = None;
                }
            }
        } else {
            // lost target (already despawned)
            brain.state = BrainState::Wander;
            brain.target_entity = None;
            brain.desired_target = None;
        }
    }
}

fn prey_flee_system(
    map: Res<super::world::TileMap>,
    predators_q: Query<(&Species, &Position), With<Species>>,
    mut prey_q: Query<(&Species, &Position, &mut Brain), With<Species>>,
) {
    // collect *threatening* actors: we’ll filter per-prey by is_prey_of
    let preds: Vec<(Species, Vec2)> = predators_q
        .iter()
        .filter(|(sp, _)| is_predator(**sp))
        .map(|(sp, p)| (*sp, p.p))
        .collect();

    if preds.is_empty() { return; }

    // ranges are defined "in tiles"; convert to world units once
    let flee_r2 = (FLEE_SENSE_RANGE * TILE_SIZE).powi(2);
    let flee_step_world = FLEE_SENSE_RANGE.max(FLEE_STEP) * TILE_SIZE; // step at least as far as sense

    for (prey_sp, pos, mut brain) in &mut prey_q {
        // find nearest predator that actually hunts this species
        let mut best_d2 = f32::MAX;
        let mut away = Vec2::ZERO;

        for (pred_sp, ppos) in &preds {
            if !is_prey_of(*pred_sp, *prey_sp) { continue; } // <- key change: fox will flee bears
            let d2 = pos.p.distance_squared(*ppos);
            if d2 < best_d2 {
                best_d2 = d2;
                away = (pos.p - *ppos).normalize_or_zero();
            }
        }

        // no relevant predator nearby → do not force a state; let decision_system run
        if best_d2 == f32::MAX {
            // if we were fleeing last frame, just clear the target so decision can replan
            if brain.state == BrainState::Flee {
                brain.desired_target = None;
            }
            continue;
        }

        if best_d2 <= flee_r2 {
            // trigger/refresh flee
            brain.state = BrainState::Flee;
            brain.target_cell = None;
            brain.target_entity = None;

            // dash away and clamp (use world-units step)
            let flee_goal = map.clamp_target(pos.p + away * flee_step_world);
            brain.desired_target = Some(flee_goal);
            brain.replan_cd = 0.3; // keep updating while threatened
        } else if brain.state == BrainState::Flee {
            // threat far enough; stop steering here and let Decision choose next
            brain.state = BrainState::Wander;
            brain.desired_target = None;
        }
    }
}

// === Reproduction ===
fn repro_cooldown_system(time: Res<Time>, mut q: Query<&mut Repro>) {
    let dt = time.delta_secs();
    for mut r in &mut q {
        if r.timer > 0.0 {
            r.timer -= dt;
            if r.timer < 0.0 { r.timer = 0.0; }
        }
    }
}

fn mating_system(
    mut commands: Commands,
    map: Res<super::world::TileMap>,

    // ParamSet avoids B0001 by separating read & write phases
    mut ps: ParamSet<(
        // p0: read-only scan to collect candidates
        Query<(Entity, &Species, &Position, &Kinematics, &Needs, &Brain, &Repro)>,
        // p1: write parents when we commit a pair
        Query<(&mut Needs, &mut Brain, &mut Repro)>,
    )>,
) {
    let mate_r2 = (MATE_RANGE_TILES * TILE_SIZE).powi(2);
    let jitter_r = OFFSPRING_JITTER * TILE_SIZE;

    // -------- Phase A: collect eligible candidates & build bins --------
    // We store candidates in a Vec so bins keep small indices, not Entities.
    #[derive(Clone, Copy)]
    struct Cand { e: Entity, sp: Species, pos: Vec2, speed: f32, cell: IVec2 }

    let mut cands: Vec<Cand> = Vec::new();
    {
        let q = ps.p0();
        for (e, sp, pos, kin, needs, brain, repro) in q.iter() {
            if brain.state != BrainState::Wander { continue; }
            if needs.is_hungry() { continue; }
            if !repro.ready() { continue; }
            let cell = map.cell_at_world(pos.p);
            cands.push(Cand { e, sp: *sp, pos: pos.p, speed: kin.base_speed, cell });
        }
    }
    if cands.len() < 2 { return; }

    // Bin by cell; key as (i32,i32) to avoid Hash on IVec2
    let mut bins: HashMap<(i32,i32), Vec<usize>> = HashMap::with_capacity(cands.len());
    for (i, c) in cands.iter().enumerate() {
        bins.entry((c.cell.x, c.cell.y)).or_default().push(i);
    }

    // -------- Phase B: greedy pairing from bins; mutate via p1 --------
    let mut used = vec![false; cands.len()];
    let mut q_parents = ps.p1();

    // Neighbor offsets (own cell + 8 neighbors)
    const OFFS: [(i32,i32); 9] = [
        (-1,-1), (0,-1), (1,-1),
        (-1, 0), (0, 0), (1, 0),
        (-1, 1), (0, 1), (1, 1),
    ];

    for i in 0..cands.len() {
        if used[i] { continue; }
        let a = cands[i];

        // search nearest compatible partner in neighbor bins
        let mut best: Option<usize> = None;
        let mut best_d2 = f32::MAX;

        for (dx, dy) in OFFS {
            let key = (a.cell.x + dx, a.cell.y + dy);
            if let Some(list) = bins.get(&key) {
                for &j in list {
                    if j == i || used[j] { continue; }
                    let b = cands[j];
                    if b.sp != a.sp { continue; }
                    let d2 = a.pos.distance_squared(b.pos);
                    if d2 <= mate_r2 && d2 < best_d2 {
                        best_d2 = d2;
                        best = Some(j);
                    }
                }
            }
        }

        let Some(j) = best else { continue; };
        let b = cands[j];

        // Offspring placement & speed
        let mid = (a.pos + b.pos) * 0.5
            + Vec2::new(fastrand::f32() - 0.5, fastrand::f32() - 0.5)
                .normalize_or_zero() * jitter_r;
        let child_pos = map.clamp_target(mid);
        let child_speed = (a.speed + b.speed) * 0.5;

        // Mutate parents together (no aliasing)
        if let Ok([ (mut n1, mut br1, mut r1), (mut n2, mut br2, mut r2) ]) =
            q_parents.get_many_mut([a.e, b.e])
        {
            // Parents become hungry and chill
            n1.satiation = 0.0; n2.satiation = 0.0;

            br1.state = BrainState::Wander;
            br1.desired_target = None; br1.target_cell = None; br1.target_entity = None;

            br2.state = BrainState::Wander;
            br2.desired_target = None; br2.target_cell = None; br2.target_entity = None;

            r1.timer = REPRO_COOLDOWN;
            r2.timer = REPRO_COOLDOWN;

            // Spawn offspring (same species)
            commands.spawn(CreatureBundle::new(a.sp, child_pos, child_speed));

            used[i] = true;
            used[j] = true;
        }
    }
}
