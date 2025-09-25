use bevy::prelude::*;
use super::base::{Species, Position, BrainState, Brain, FoodKind};
use super::world::{TILE_SIZE};

// Plant foraging hysteresis
pub const HYSTERESIS_RATIO: f32 = 0.45;
// Predation
pub const PREDATOR_SENSE_RANGE: f32 = 10.0;         // tiles

pub fn forage_system(
    time: Res<Time>,
    map: Res<super::world::TileMap>,
    mut q: Query<(&Species, &Position, &mut Brain)>,
    prey_scan: Query<(Entity, &Species, &Position)>, // read-only; used by predators
) {
    let dt = time.delta_secs();

    for (sp, pos, mut brain) in &mut q {
        // // tick cooldowns
        // if brain.last_food_cooldown > 0.0 {
        //     brain.last_food_cooldown = (brain.last_food_cooldown - dt).max(0.0);
        // }
        brain.replan_cd -= dt;

        // Only make foraging decisions in foraging mode.
        if brain.state != BrainState::Forage { continue; }

        // --- Predator branch: hunt when hungry ---
        if is_predator(*sp) {
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
                brain.target_entity = Some(e);
                brain.target_cell = None;
                brain.replan_cd = 0.15; // track frequently
                brain.desired_target = Some(map.clamp_target(target_pos));
                continue;
            }

            // no prey seen → hungry wander
            if brain.replan_cd <= 0.0 || brain.desired_target.is_none() {
                let jitter = Vec2::new(fastrand::f32() - 0.5, fastrand::f32() - 0.5)
                    .normalize_or_zero() * 5.0;
                brain.replan_cd = 0.6;
                brain.desired_target = Some(map.clamp_target(pos.p + jitter));
            }
            continue;
        } else {
            // --- Herbivore/bird: hungry → forage plants ---
            if brain.replan_cd > 0.0 && brain.desired_target.is_some() { continue; }

            if let Some((cell, _kind)) = nearest_food_cell(
                &map, *sp, pos.p,
                HYSTERESIS_RATIO,
            ) {
                brain.state = BrainState::Forage;
                brain.target_cell = Some(cell);
                brain.desired_target = Some(map.clamp_target(cell_center(cell)));
                brain.replan_cd = 0.75;
            } else {
                // hungry wander step
                let jitter = Vec2::new(fastrand::f32() - 0.5, fastrand::f32() - 0.5)
                    .normalize_or_zero() * 4.0;
                brain.state = BrainState::Forage;
                brain.target_cell = None;
                brain.desired_target = Some(map.clamp_target(pos.p + jitter));
                brain.replan_cd = 0.75;
            }
        }
    }
}


pub fn cell_center(cell: IVec2) -> Vec2 {
    Vec2::new((cell.x as f32 + 0.5) * TILE_SIZE, (cell.y as f32 + 0.5) * TILE_SIZE)
}

fn nearest_food_cell(
    map: &super::world::TileMap,
    sp: Species,
    from: Vec2,
    hysteresis_ratio: f32,
) -> Option<(IVec2, FoodKind)> {
    let mut best: Option<(IVec2, f32, FoodKind)> = None;

    for y in 0..map.height {
        for x in 0..map.width {
            let cell = IVec2::new(x, y);
            let tile = &map.tiles[(y * map.width + x) as usize];
            
            if tile_food_ratio_for_species(tile, sp) < hysteresis_ratio {
                continue;
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

fn tile_food_ratio_for_species(tile: &super::world::Tile, sp: Species) -> f32 {
    let nuts_r   = if tile.nuts_max    > 0.0 { tile.nuts    / tile.nuts_max    } else { 0.0 };
    let berries_r= if tile.berries_max > 0.0 { tile.berries / tile.berries_max } else { 0.0 };
    match sp {
        Species::Deer => berries_r,
        Species::Squirrel | Species::Bird => nuts_r.max(berries_r),
        _ => 0.0,
    }
}

pub fn is_predator(sp: Species) -> bool {
    matches!(sp, Species::Fox | Species::Bear)
}

pub fn is_prey_of(pred: Species, prey: Species) -> bool {
    match pred {
        Species::Fox  => matches!(prey, Species::Squirrel | Species::Bird),
        Species::Bear => matches!(prey, Species::Squirrel | Species::Deer | Species::Fox),
        _ => false,
    }
}
