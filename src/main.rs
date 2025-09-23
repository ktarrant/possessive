use bevy::prelude::*;
use bevy::render::camera::Projection;
use bevy_ecs_tiled::prelude::*; // TiledPlugin, TiledMapAsset, TiledMap, TilemapAnchor

fn main() {
    App::new()
        .add_plugins(
            DefaultPlugins.set(AssetPlugin {
                watch_for_changes_override: Some(true), // hot reload
                ..default()
            })
        )
        .add_plugins(TiledPlugin::default())
        .add_systems(Startup, (setup_camera, spawn_map, spawn_debug_square))
        .run();
}

fn spawn_map(mut commands: Commands, assets: Res<AssetServer>) {
    // assets/maps/demo.tmx (Orientation: Isometric, Tile size: 256x128)
    let map: Handle<TiledMapAsset> = assets.load("maps/demo.tmx");
    commands.spawn((
        TiledMap(map),
        TilemapAnchor::Center,      // center the diamond
        Transform::default(),       // replace SpatialBundle
        Visibility::Visible,
        Name::new("TiledMap"),
    ));
}

fn spawn_debug_square(mut commands: Commands) {
    commands.spawn(Sprite {
        color: Color::srgba(1.0, 0.0, 0.0, 1.0),
        rect: Some(Rect::from_center_size(Vec2::ZERO, Vec2::splat(100.0))),
        ..default()
    });
}

fn setup_camera(mut commands: Commands) {
    commands.spawn((
        Camera2d,
        Camera::default(),
        Projection::Orthographic(OrthographicProjection {
            scale: 6.0,                          // start zoomed out for big iso tiles
            ..OrthographicProjection::default_2d()
        }),
        Transform::from_xyz(0.0, 0.0, 1000.0),  // important: +Z
        Visibility::Visible,
        Name::new("Camera2D"),
    ));
}
