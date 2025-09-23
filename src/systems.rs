
use bevy::prelude::*;
use crate::components::*;
use crate::resources::*;

pub fn move_hero(time: Res<Time>, input: Res<InputState>, mut q: Query<(&mut Transform, &Hero, &mut Vel)>) {
    let (mut tf, hero, mut vel) = q.single_mut();
    vel.0 = input.move_axis * hero.move_speed;
    tf.translation.x += vel.0.x * time.delta_seconds();
    tf.translation.y += vel.0.y * time.delta_seconds();
}

pub fn shrine_aura_and_regen(time: Res<Time>, mut hero_q: Query<(&Transform, &mut Hero)>, shrine_q: Query<(&Transform, &Shrine)>) {
    let (hero_tf, mut hero) = hero_q.single_mut();
    let mut bonus = 0.0;
    for (stf, shrine) in &shrine_q {
        let dist = hero_tf.translation.truncate().distance(stf.translation.truncate());
        if dist <= shrine.radius { bonus += shrine.regen_bonus; }
    }
    hero.mana = (hero.mana + (hero.regen + bonus) * time.delta_seconds()).clamp(0.0, hero.max_mana);
}

pub fn debug_kill_nearest(input: Res<InputState>, hero_q: Query<&Transform, With<Hero>>, mut q: Query<(&Transform, &mut Creature)>) {
    if !input.press_kill { return; }
    let hero_pos = hero_q.single().translation.truncate();
    let mut best: Option<(f32, Mut<Creature>)> = None;
    for (tf, c) in &mut q {
        if !c.alive { continue; }
        let d = hero_pos.distance(tf.translation.truncate());
        if d < 180.0 {
            match &mut best {
                None => best = Some((d, c)),
                Some((bd, _)) if d < *bd => best = Some((d, c)),
                _ => {}
            }
        }
    }
    if let Some((_d, mut c)) = best { c.alive = false; c.possessed = false; c.undead = false; }
}

pub fn possess_system(input: Res<InputState>, mut hero_q: Query<(&Transform, &mut Hero)>, mut q: Query<(&Transform, &mut Creature)>) {
    if !input.press_possess { return; }
    let (hero_tf, mut hero) = hero_q.single_mut();
    let mut best: Option<(f32, Mut<Creature>)> = None;
    for (tf, c) in &mut q {
        if c.alive && !c.possessed && !c.undead {
            let d = hero_tf.translation.truncate().distance(tf.translation.truncate());
            if d <= hero.cast_range {
                match &mut best {
                    None => best = Some((d, c)),
                    Some((bd, _)) if d < *bd => best = Some((d, c)),
                    _ => {}
                }
            }
        }
    }
    if let Some((_d, mut c)) = best {
        let mut cost = hero.possess_cost;
        if matches!(hero.class_, HeroClass::Possessor) { cost *= 0.7; }
        if hero.mana >= cost { hero.mana -= cost; c.possessed = true; }
    }
}

pub fn raise_dead_system(input: Res<InputState>, mut hero_q: Query<(&Transform, &mut Hero)>, mut q: Query<(&Transform, &mut Creature)>) {
    if !input.press_raise { return; }
    let (hero_tf, mut hero) = hero_q.single_mut();
    let mut best: Option<(f32, Mut<Creature>)> = None;
    for (tf, c) in &mut q {
        if !c.alive && !c.undead {
            let d = hero_tf.translation.truncate().distance(tf.translation.truncate());
            if d <= hero.cast_range {
                match &mut best {
                    None => best = Some((d, c)),
                    Some((bd, _)) if d < *bd => best = Some((d, c)),
                    _ => {}
                }
            }
        }
    }
    if let Some((_d, mut c)) = best {
        let mut cost = hero.raise_cost;
        if matches!(hero.class_, HeroClass::Necromancer) { cost *= 0.7; }
        if hero.mana >= cost { hero.mana -= cost; c.alive = true; c.undead = true; }
    }
}

pub fn harvest_tick_system(time: Res<Time>, mut stock: ResMut<Stockpiles>, mut q: Query<(&mut Worker, &Creature)>) {
    for (mut w, c) in &mut q {
        if !(c.possessed && c.alive) { continue; }
        w.timer -= time.delta_seconds();
        if w.timer <= 0.0 {
            w.timer = w.interval;
            match w.kind { HarvestKind::Food => stock.food += 1, HarvestKind::Wood => stock.wood += 1 }
        }
    }
}

pub fn hud_print_system(stock: Res<Stockpiles>, q: Query<&Hero>) {
    let hero = q.single();
    info!("Food={} Wood={} Mana={:.0}/{:.0}", stock.food, stock.wood, hero.mana, hero.max_mana);
}
