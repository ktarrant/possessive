use bevy::prelude::*;
use bevy::ui::{UiRect, PositionType, BackgroundColor, BorderColor};

use super::world::{TileMap, TileObject, Terrain, food_totals, TILE_SIZE};
use super::creature::{Species, Position};


const VIS_TILE_PIXELS: f32 = 16.0;
const ANIMAL_DOT: f32 = 10.0;
const OBJECT_DOT: f32 = 8.0;

#[derive(Component)] struct TileSprite;
#[derive(Component)] struct ObjectSprite(IVec2);
#[derive(Component)] struct AnimalSprite;
#[derive(Component)] struct MetricsText;

#[derive(Resource)]
struct MetricsTimer(Timer);

pub struct SimViewPlugin;

impl Plugin for SimViewPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(MetricsTimer(Timer::from_seconds(0.25, TimerMode::Repeating)))
            .add_systems(Startup, (setup_camera, spawn_map_sprites, spawn_metrics_panel))
            .add_systems(Update, attach_animal_sprites)
            .add_systems(Update, (sync_animal_sprites, update_object_alpha, update_metrics).chain());
    }
}

fn setup_camera(mut commands: Commands, map: Res<TileMap>) {
    const Z: f32 = 1000.0;
    let cx = map.width as f32 * VIS_TILE_PIXELS * 0.5;
    let cy = map.height as f32 * VIS_TILE_PIXELS * 0.5;

    commands.spawn((
        Camera2d,                          // 2D camera component
        Projection::Orthographic(OrthographicProjection {
            scale: 2.0,                          // start zoomed out for big iso tiles
            ..OrthographicProjection::default_2d()
        }),
        Transform::from_xyz(cx, cy, Z),    // center on the map
        Visibility::default(),             // optional; Bevy will add required bits
    ));
}

// --- map + objects ---

fn terrain_color(t: Terrain) -> Color {
    match t {
        Terrain::Grassland => Color::srgb(0.56, 0.76, 0.45),
        Terrain::Forest    => Color::srgb(0.30, 0.55, 0.35),
        Terrain::Mountain  => Color::srgb(0.55, 0.52, 0.48),
        Terrain::Water     => Color::srgb(0.20, 0.45, 0.85),
    }
}

// user-preferred object colors:
fn object_color(obj: TileObject) -> Color {
    match obj {
        TileObject::Tree => Color::srgba_u8(255, 220,   0, 255), // bright yellow
        TileObject::Bush => Color::srgba_u8(255, 120,   0, 255), // vivid orange
    }
}

fn species_color(sp: Species) -> Color {
    match sp {
        Species::Squirrel => Color::srgb(0.72, 0.40, 0.10), // brown-ish
        Species::Deer     => Color::srgb(0.60, 0.45, 0.30),
        Species::Bird     => Color::srgb(0.15, 0.55, 0.95),
        Species::Fox      => Color::srgb(0.95, 0.40, 0.10),
        Species::Bear     => Color::srgb(0.25, 0.25, 0.30),
    }
}

fn tile_to_world(x: i32, y: i32) -> Vec3 {
    Vec3::new(
        (x as f32 + 0.5) * VIS_TILE_PIXELS,
        (y as f32 + 0.5) * VIS_TILE_PIXELS,
        0.0,
    )
}

fn spawn_map_sprites(mut commands: Commands, map: Res<TileMap>) {
    // background tiles
    for y in 0..map.height {
        for x in 0..map.width {
            let idx = (y * map.width + x) as usize;
            let t = &map.tiles[idx];

            commands.spawn((
                Sprite {
                    custom_size: Some(Vec2::splat(VIS_TILE_PIXELS)),
                    color: terrain_color(t.terrain),
                    ..Default::default()
                },
                Transform::from_translation(tile_to_world(x, y)),
                Visibility::default(),
                TileSprite,
            ));

            // overlay object dot (tree/bush)
            if let Some(obj) = t.object {
                let pct = match obj {
                    TileObject::Tree => if t.nuts_max > 0.0 { t.nuts / t.nuts_max } else { 0.0 },
                    TileObject::Bush => if t.berries_max > 0.0 { t.berries / t.berries_max } else { 0.0 },
                }.clamp(0.1, 1.0);

                commands.spawn((
                    Sprite {
                        custom_size: Some(Vec2::splat(OBJECT_DOT)),
                        color: object_color(obj).with_alpha(pct),
                        ..Default::default()
                    },
                    Transform::from_translation(tile_to_world(x, y) + Vec3::new(0.0, 0.0, 1.0)),
                    Visibility::default(),
                    ObjectSprite(IVec2::new(x, y)),
                ));
            }

        }
    }
}

// --- animals ---
fn attach_animal_sprites(
    mut commands: Commands,
    q: Query<(Entity, &super::creature::Species), Added<super::creature::Species>>
) {
    for (e, sp) in &q {
        commands.entity(e).insert((
            Sprite {
                custom_size: Some(Vec2::splat(ANIMAL_DOT)),
                color: species_color(*sp),
                ..Default::default()
            },
            Transform::from_translation(Vec3::new(0.0, 0.0, 2.0)),
            Visibility::default(),
            AnimalSprite,
        ));
    }
}


fn sync_animal_sprites(mut q: Query<(&Position, &mut Transform), With<AnimalSprite>>) {
    for (pos, mut tf) in &mut q {
        // sim positions are in TILE_SIZE units; scale to viz pixels
        let scale = VIS_TILE_PIXELS / TILE_SIZE;
        tf.translation.x = pos.p.x * scale;
        tf.translation.y = pos.p.y * scale;
        // z already set to 2.0
    }
}

fn update_object_alpha(map: Res<TileMap>, mut q: Query<(&ObjectSprite, &mut Sprite)>) {
    for (mark, mut sprite) in &mut q {
        if let Some(tile) = map.tile_at_cell(mark.0) {
            if let Some(obj) = tile.object {
                let pct = match obj {
                    TileObject::Tree => if tile.nuts_max > 0.0 { tile.nuts / tile.nuts_max } else { 0.0 },
                    TileObject::Bush => if tile.berries_max > 0.0 { tile.berries / tile.berries_max } else { 0.0 },
                }.clamp(0.1, 1.0);
                sprite.color = object_color(obj).with_alpha(pct);
            }
        }
    }
}

// --- metrics UI ---
fn spawn_metrics_panel(mut commands: Commands) {
    commands
        .spawn((
            // Panel node
            Node {
                position_type: PositionType::Absolute,
                right: Val::Px(12.0),
                top: Val::Px(12.0),
                padding: UiRect::all(Val::Px(10.0)),
                border: UiRect::all(Val::Px(1.0)),
                width: Val::Px(260.0),     // keep lines tidy
                ..default()
            },
            BackgroundColor(Color::srgb(0.10, 0.12, 0.14)),
            BorderColor(Color::srgb(0.25, 0.28, 0.32)),
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("metrics"),      // placeholder; we’ll overwrite this each tick
                TextFont {
                    font_size: 16.0,
                    ..default()
                },
                TextColor(Color::WHITE),
                MetricsText,
            ));
        });
}

fn update_metrics(
    time: Res<Time>,
    mut timer: ResMut<MetricsTimer>,
    map: Res<TileMap>,
    q_creatures: Query<&Species>,
    mut q_text: Query<&mut Text, With<MetricsText>>,
) {
    if !timer.0.tick(time.delta()).just_finished() { return; }

    // counts per species
    let (mut s, mut d, mut b, mut f, mut be) = (0, 0, 0, 0, 0);
    for sp in &q_creatures {
        match sp {
            Species::Squirrel => s += 1,
            Species::Deer     => d += 1,
            Species::Bird     => b += 1,
            Species::Fox      => f += 1,
            Species::Bear     => be += 1,
        }
    }
    let (nuts, berries) = food_totals(&map);

    if let Ok(mut text) = q_text.single_mut() {
        *text = Text::new(format!(
            "Wildlife Simulation\n\
            Map: {}×{}\n\n\
            Animals\n\
            Squirrel: {:>4}\n  Deer:     {:>4}\n  Bird:     {:>4}\n  Fox:      {:>4}\n  Bear:     {:>4}\n\n\
            Food (total available)\n\
            Nuts:    {:>6.2}\n  Berries: {:>6.2}\n",
            map.width, map.height, s, d, b, f, be, nuts, berries
        ));
    }
}
