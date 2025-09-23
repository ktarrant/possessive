// src/iso_map.rs
use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::*;

pub struct IsoMapPlugin;

impl Plugin for IsoMapPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_iso_map);
    }
}

fn setup_iso_map(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut atlases: ResMut<Assets<TextureAtlasLayout>>,
) {
    // --- Tilemap dimensions & tile size (classic iso = wide tile: 64x32) ---
    let map_size = TilemapSize { x: 10, y: 10 };
    let tile_size = TilemapTileSize { x: 64.0, y: 32.0 };

    // --- Tell the renderer this is an ISOMETRIC map ---
    let grid_size = TilemapGridSize { x: 64.0, y: 32.0 };
    let map_type = TilemapType::Isometric(IsoCoordSystem::Diamond);

    // --- Tileset (single image with a single tile for now) ---
    let texture_handle: Handle<Image> = asset_server.load("tiles.png");
    let atlas_layout = TextureAtlasLayout::from_grid(Vec2::new(64.0, 32.0), 1, 1, None, None);
    let atlas_layout = atlases.add(atlas_layout);

    // --- Create a layer entity ---
    let tilemap_entity = commands.spawn_empty().id();

    // --- Build the tile storage ---
    let mut storage = TileStorage::empty(map_size);
    for y in 0..map_size.y {
        for x in 0..map_size.x {
            let tile_pos = TilePos { x, y };
            let tile_entity = commands
                .spawn((
                    TileBundle {
                        position: tile_pos,
                        tilemap_id: TilemapId(tilemap_entity),
                        texture_index: TileTextureIndex(0),
                        ..Default::default()
                    },
                ))
                .id();
            storage.set(&tile_pos, tile_entity);
        }
    }

    // --- Spawn the map layer (like a sprite you can position/scale) ---
    commands.entity(tilemap_entity).insert((
        TilemapBundle {
            grid_size,
            size: map_size,
            storage,
            texture: TilemapTexture::Single(texture_handle),
            tile_size,
            map_type,
            transform: Transform::from_xyz(0.0, 0.0, 0.0), // tweak for camera centering
            texture_atlas: TilemapTextureAtlas::from(atlas_layout),
            ..Default::default()
        },
        Name::new("IsoLayer"),
    ));

    // Camera (orthographic) looking at the map
    commands.spawn(Camera2dBundle {
        projection: OrthographicProjection {
            scale: 0.75, // zoom in/out
            ..Default::default()
        },
        ..Default::default()
    });
}
