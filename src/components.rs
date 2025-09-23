
use bevy::prelude::*;

#[derive(Component)]
pub struct Hero {
    pub mana: f32,
    pub max_mana: f32,
    pub regen: f32,
    pub class_: HeroClass,
    pub move_speed: f32,
    pub possess_cost: f32,
    pub raise_cost: f32,
    pub cast_range: f32,
}
#[derive(Clone, Copy)]
pub enum HeroClass { Necromancer, Possessor, Enchanter }

#[derive(Component)]
pub struct Creature { pub species: Species, pub alive: bool, pub possessed: bool, pub undead: bool }
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Species { Squirrel, Bear, Bird }

#[derive(Component)]
pub struct Worker { pub kind: HarvestKind, pub interval: f32, pub timer: f32 }
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum HarvestKind { Food, Wood }

#[derive(Component, Deref, DerefMut)] pub struct Vel(pub Vec2);
#[derive(Component)] pub struct Shrine { pub regen_bonus: f32, pub radius: f32 }
