use bevy::prelude::*;
use bevy_mod_aseprite::{Aseprite, AsepriteAnimation, AsepriteAsset};

pub struct HeroPlugin;

const HERO_SCALE: f32 = 3.0; // try 2.0–4.0 depending on taste

// fn debug_tags(assets: Res<Assets<AsepriteAsset>>, hh: Res<HeroHandles>) {
//     for handle_opt in [&hh.idle, &hh.walk, &hh.run] {
//         if let Some(handle) = handle_opt {
//             if let Some(asset) = assets.get(handle) {
//                 let info = asset.info();

//                 // Tags (keyed by name)
//                 for (name, _tag) in &info.tags {
//                     info!("Aseprite tag: {}", name);
//                 }

//                 // Slices (also keyed by name)
//                 for (name, _slice) in &info.slices {
//                     info!("Aseprite slice: {}", name);
//                 }
//             }
//         }
//     }
// }

impl Plugin for HeroPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<HeroHandles>()
            .add_systems(Startup, load_hero_aseprites)
            .add_systems(Update, (spawn_hero_once_loaded, drive_hero));
            // .add_systems(Update, debug_tags);
    }
}

#[derive(Resource, Default)]
struct HeroHandles {
    // idle: Option<Handle<AsepriteAsset>>,
    walk: Option<Handle<AsepriteAsset>>,
    run:  Option<Handle<AsepriteAsset>>,
    spawned: bool,
}

#[derive(Component)]
pub struct Hero;

#[derive(Component, Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Facing { #[default] S, SE, E, NE, N, NW, W, SW }

#[derive(Component, Default)]
struct MoveState { velocity: Vec2 }

#[derive(Component)]
struct HeroAnimSet {
    // idle: Handle<AsepriteAsset>,
    walk: Handle<AsepriteAsset>,
    run:  Handle<AsepriteAsset>,
}

#[derive(Component)]
struct ActiveTag(String); // keep track of current tag string

fn load_hero_aseprites(mut commands: Commands, assets: Res<AssetServer>) {
    commands.insert_resource(HeroHandles {
        // idle: Some(assets.load("hero/idle.aseprite")),
        walk: Some(assets.load("hero/walk.aseprite")),
        run:  Some(assets.load("hero/run.aseprite")),
        spawned: false,
    });
}
fn spawn_hero_once_loaded(
    mut commands: Commands,
    mut hero_handles: ResMut<HeroHandles>,
    ase_assets: Res<Assets<AsepriteAsset>>,
) {
    if hero_handles.spawned { return; }
    let (Some(walk_h), Some(run_h)) = (&hero_handles.walk, &hero_handles.run) else { return; };

    if ase_assets.get(walk_h).is_none() || ase_assets.get(run_h).is_none() {
        return;
    }

    // Start on Walk_Down (we'll freeze it when idle)
    let walk_asset = ase_assets.get(walk_h).unwrap();
    let start_tag = "Walk_Down";
    let anim = AsepriteAnimation::new(walk_asset.info(), start_tag);
    let mut tf = Transform::from_xyz(0.0, 0.0, 10.0);
    tf.scale = Vec3::splat(HERO_SCALE);

    commands.spawn((
        Hero,
        Name::new("Hero"),
        Sprite {
            image: walk_asset.texture().clone_weak(),
            texture_atlas: Some(TextureAtlas {
                index: anim.current_frame(),
                layout: walk_asset.layout().clone_weak(),
            }),
            ..default()
        },
        tf, // <-- use the scaled transform
        Aseprite { asset: walk_h.clone_weak(), anim },
        Facing::S,
        MoveState::default(),
        HeroAnimSet { walk: walk_h.clone_weak(), run: run_h.clone_weak() },
        ActiveTag(start_tag.to_string()),
    ));

    hero_handles.spawned = true;
}

fn drive_hero(
    time: Res<Time>,
    kb: Res<ButtonInput<KeyCode>>,
    mut q: Query<(
        &mut Aseprite,
        &mut Sprite,
        &mut Transform,
        &mut MoveState,
        &HeroAnimSet,
        &mut Facing,
        &mut ActiveTag,
    ), With<Hero>>,
    ase_assets: Res<Assets<AsepriteAsset>>,
) {
    let Ok((mut ase, mut sprite, mut tf, mut mv, animset, mut facing, mut active_tag)) = q.single_mut() else { return; };

    // input → velocity
    let mut v = Vec2::ZERO;
    if kb.pressed(KeyCode::KeyW) { v.y += 1.0; }
    if kb.pressed(KeyCode::KeyS) { v.y -= 1.0; }
    if kb.pressed(KeyCode::KeyA) { v.x -= 1.0; }
    if kb.pressed(KeyCode::KeyD) { v.x += 1.0; }
    if v != Vec2::ZERO { v = v.normalize(); }

    let walk_speed = 120.0;
    let run_speed  = 240.0;
    let target_speed = if kb.pressed(KeyCode::ShiftLeft) { run_speed } else { walk_speed };

    mv.velocity = v * target_speed;
    tf.translation += Vec3::new(mv.velocity.x, mv.velocity.y, 0.0) * time.delta_secs();

    let moving = v.length_squared() > 0.0;
    let new_facing = if moving { dir_to_facing(v) } else { *facing };
    let group = facing_to_group(new_facing);
    sprite.flip_x = matches!(new_facing, Facing::E | Facing::NE | Facing::SE);

    // Choose sheet + tag
    let (asset_handle, tag_prefix) = if !moving {
        (&animset.walk, "Walk")     // <- fake idle with Walk_* tag
    } else if kb.pressed(KeyCode::ShiftLeft) {
        (&animset.run, "Run")
    } else {
        (&animset.walk, "Walk")
    };

    let Some(asset) = ase_assets.get(asset_handle) else { return; };
    let desired_tag = group_tag(tag_prefix, group);

    // Switch sheet/tag if needed, otherwise leave animation to plugin
    if ase.asset != *asset_handle {
        ase.asset = asset_handle.clone_weak();
        sprite.image = asset.texture().clone_weak();
        if let Some(atlas) = sprite.texture_atlas.as_mut() {
            atlas.layout = asset.layout().clone_weak();
        }
        ase.anim = AsepriteAnimation::new(asset.info(), desired_tag);
        active_tag.0 = desired_tag.to_string();
    } else if active_tag.0 != desired_tag {
        ase.anim = AsepriteAnimation::new(asset.info(), desired_tag);
        active_tag.0 = desired_tag.to_string();
    }

    // If not moving, freeze the pose by resetting to frame 0 every frame.
    if !moving {
        // Recreate ensures current_frame() is 0; set atlas index explicitly.
        ase.anim = AsepriteAnimation::new(asset.info(), desired_tag);
        if let Some(atlas) = sprite.texture_atlas.as_mut() {
            atlas.index = ase.anim.current_frame(); // usually 0
        }
    } else {
        // Optional: immediate sync after switching; plugin continues animating.
        if let Some(atlas) = sprite.texture_atlas.as_mut() {
            atlas.index = ase.anim.current_frame();
        }
    }

    *facing = new_facing;
}


fn dir_to_facing(v: Vec2) -> Facing {
    if v.length_squared() < 1e-6 { return Facing::S; }
    let d = wrap_deg(v.to_angle().to_degrees());
    use Facing::*;
    match true {
        _ if d > 157.5 || d <= -157.5 => W,
        _ if d > 112.5 => NW,
        _ if d > 67.5  => N,
        _ if d > 22.5  => NE,
        _ if d > -22.5 => E,
        _ if d > -67.5 => SE,
        _ if d > -112.5 => S,
        _ => SW,
    }
}

fn wrap_deg(x: f32) -> f32 {
    if x <= -180.0 { x + 360.0 } else if x > 180.0 { x - 360.0 } else { x }
}

#[derive(Clone, Copy, Debug)]
enum DirGroup { Down, DownSide, Side, UpSide, Up }

fn facing_to_group(f: Facing) -> DirGroup {
    use Facing::*;
    match f {
        S        => DirGroup::Down,
        SE | SW  => DirGroup::DownSide,
        E | W    => DirGroup::Side,
        NE | NW  => DirGroup::UpSide,
        N        => DirGroup::Up,
    }
}

fn group_tag(prefix: &str, g: DirGroup) -> &'static str {
    match (prefix, g) {
        ("Walk", DirGroup::Down)     => "Walk_Down",
        ("Walk", DirGroup::DownSide) => "Walk_Down_Side",
        ("Walk", DirGroup::Side)     => "Walk_Side",
        ("Walk", DirGroup::UpSide)   => "Walk_Up_Side",
        ("Walk", DirGroup::Up)       => "Walk_Up",

        ("Run", DirGroup::Down)      => "Run_Down",
        ("Run", DirGroup::DownSide)  => "Run_Down_Side",
        ("Run", DirGroup::Side)      => "Run_Side",
        ("Run", DirGroup::UpSide)    => "Run_Up_Side",
        ("Run", DirGroup::Up)        => "Run_Up",

        // If you later add Idle_* tags, add mappings here:
        ("Idle", DirGroup::Down)     => "Idle_Down",
        ("Idle", DirGroup::DownSide) => "Idle_Down_Side",
        ("Idle", DirGroup::Side)     => "Idle_Side",
        ("Idle", DirGroup::UpSide)   => "Idle_Up_Side",
        ("Idle", DirGroup::Up)       => "Idle_Up",

        _ => "Walk_Down", // safe fallback
    }
}
