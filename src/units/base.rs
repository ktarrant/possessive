use bevy::prelude::*;

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub enum Species { Squirrel, Deer, Bird, Fox, Bear }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FoodKind { Nuts, Berries }

#[derive(Component, Default, Debug)]
pub struct Position { pub p: Vec2 }

#[derive(Component, Default, Debug)]
pub struct Velocity { pub v: Vec2 }

#[derive(Component, Debug)]
pub struct Kinematics { pub base_speed: f32 }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BrainState { Wander, Forage, Eating, Hunt, Flee }

#[derive(Component, Debug)]
pub struct Brain {
    pub state: BrainState,
    pub replan_cd: f32,

    // forage plant targeting
    pub target_cell: Option<IVec2>,
    // path goal
    pub desired_target: Option<Vec2>,
    // hunting
    pub target_entity: Option<Entity>,
}

impl Default for Brain {
    fn default() -> Self {
        Self {
            state: BrainState::Wander,
            replan_cd: 0.0,
            target_cell: None,
            desired_target: None,
            target_entity: None,
        }
    }
}
