use super::grid::Grid;
use super::template::{MapTemplate, TerrainWeights, TerrainRules, TerrainArea, AreaSource};
use super::debug_png::{write_height_with_disks, write_height_with_overlays, write_terrain_classes};
use super::ley::{LeyNetwork, generate_ley};
use glam::{IVec2, Vec2};

/// Output of Phase 1: base disks flattened at identical elevation.
pub struct Phase1Bases {
    pub height: Grid<f32>,
    pub base_centers: Vec<IVec2>,
    pub base_radius: i32,
}

/// Phase 1 only (no file I/O): place `num_bases` evenly on a ring and flatten disks.
pub fn generate_phase1_bases(
    tpl: &MapTemplate,
    num_bases: usize,
    start_angle_deg: Option<f32>,
) -> Phase1Bases {
    assert!(num_bases >= 1, "num_bases must be >= 1");

    let size = IVec2::new(tpl.size.0, tpl.size.1);
    let mut height = Grid::<f32>::new(size.x, size.y);

    let center = size.as_vec2() / 2.0;
    let r_disk = tpl.player_spawns.base_radius.max(1);
    // Prevent disks from clipping the border.
    let max_ring = ((size.x.min(size.y) / 2) - r_disk - 1).max(0);
    let ring = tpl.player_spawns.center_radius.min(max_ring);
    let base_elev = tpl.player_spawns.elevation;

    let start_angle = start_angle_deg.unwrap_or(0.0).to_radians();

    let mut centers = Vec::with_capacity(num_bases);
    for i in 0..num_bases {
        let ang = start_angle + (i as f32) * std::f32::consts::TAU / (num_bases as f32);
        let p = center + Vec2::new(ang.cos(), ang.sin()) * (ring as f32);
        let c = IVec2::new(p.x.round() as i32, p.y.round() as i32);
        centers.push(c);

        // Fill a solid disk at uniform elevation.
        for y in (c.y - r_disk)..=(c.y + r_disk) {
            if y < 0 || y >= size.y { continue; }
            for x in (c.x - r_disk)..=(c.x + r_disk) {
                if x < 0 || x >= size.x { continue; }
                let dx = x - c.x;
                let dy = y - c.y;
                if dx*dx + dy*dy <= r_disk*r_disk {
                    height.set(x, y, base_elev);
                }
            }
        }
    }

    Phase1Bases { height, base_centers: centers, base_radius: r_disk }
}
