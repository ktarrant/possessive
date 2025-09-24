// use bevy::prelude::*;
// use bevy::render::camera::Projection;
// use bevy_ecs_tiled::prelude::*;
// use bevy_mod_aseprite::AsepritePlugin;
// mod hero;
// use hero::HeroPlugin;

// fn main() {
//     App::new()
//         .add_plugins(
//             DefaultPlugins.set(AssetPlugin {
//                 watch_for_changes_override: Some(true), // hot reload
//                 ..default()
//             }).set(ImagePlugin::default_nearest()),
//         )
//         .add_plugins(AsepritePlugin)
//         .add_plugins(HeroPlugin)
//         .add_plugins(TiledPlugin::default())
//         .add_systems(Startup, (setup_camera, spawn_map))
//         .run();
// }

// fn spawn_map(mut commands: Commands, assets: Res<AssetServer>) {
//     // assets/maps/demo.tmx (Orientation: Isometric, Tile size: 256x128)
//     let map: Handle<TiledMapAsset> = assets.load("maps/demo.tmx");
//     commands.spawn((
//         TiledMap(map),
//         TilemapAnchor::Center,      // center the diamond
//         Transform::default(),       // replace SpatialBundle
//         Visibility::Visible,
//         Name::new("TiledMap"),
//     ));
// }

// fn setup_camera(mut commands: Commands) {
//     commands.spawn((
//         Camera2d,
//         Camera::default(),
//         Projection::Orthographic(OrthographicProjection {
//             scale: 1.0,                          // start zoomed out for big iso tiles
//             ..OrthographicProjection::default_2d()
//         }),
//         Transform::from_xyz(0.0, 0.0, 1000.0),  // important: +Z
//         Visibility::Visible,
//         Name::new("Camera2D"),
//     ));
// }

mod terrain; // make the `terrain` folder visible to this crate
use terrain::{generate_map, MapTemplate};              // thanks to the re-exports
use terrain::debug_png::write_height_rgb;               // direct submodule import

fn main() {
    let tpl = MapTemplate::from_file("assets/maps/arabia.ron");
    let seed = 123456789;
    let g = generate_map(&tpl, seed);

    // Export a quick visualization
    std::fs::create_dir_all("out").ok();
    write_height_rgb("out/arabia_demo.png", &g.height, &g.passable, &g.water);
    println!("Players: {:?}", g.player_starts);
    println!("Gold nodes: {}", g.gold.len());
    println!("Berry tiles: {}", g.berries.len());
    println!("Forest points: {}", g.forest.len());
}

