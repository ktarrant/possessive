use glam::{IVec2, Vec2};

use super::grid::Grid;
use super::template::MapTemplate;
use super::ley::{LeyNetwork, generate_ley};
use super::debug_png::{write_height_with_disks, write_height_with_overlays};

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

/// Simple configuration for Phase 2 ley network.
pub struct LeyConfig {
    pub m_shrines: usize,   // how many shrines
    pub shrine_ring: i32,   // ring radius for shrines
    pub offset_deg: f32,    // angular offset vs. base ring start (e.g., 180/n for half-step)
    pub connect_cycle: bool,
    pub connect_spokes: bool,
}

/// Convenience orchestrator:
/// - runs Phase 1 (bases), saves `phase1_bases.png`
/// - runs Phase 2 (ley), saves `phase2_ley.png`
/// Returns (phase1, ley) so you can keep building later phases.
pub fn generate_step_by_step_and_save(
    tpl: &MapTemplate,
    num_bases: usize,
    start_angle_deg: f32,
    ley_cfg: LeyConfig,
    out_dir: &str,
) -> (Phase1Bases, LeyNetwork) {
    std::fs::create_dir_all(out_dir).ok();

    // ---- Phase 1
    let p1 = generate_phase1_bases(tpl, num_bases, Some(start_angle_deg));

    // Phase 1 debug image
    let phase1_path = format!("{}/phase1_bases.png", out_dir);
    let base_disks: Vec<_> = p1.base_centers
        .iter()
        .map(|&c| (c, p1.base_radius, [255, 64, 64])) // red fill
        .collect();
    write_height_with_disks(&phase1_path, &p1.height, &base_disks);

    // ---- Phase 2
    let ley = generate_ley(
        tpl.size,
        num_bases,
        start_angle_deg,
        ley_cfg.m_shrines,
        ley_cfg.shrine_ring,
        ley_cfg.offset_deg,
        ley_cfg.connect_cycle,
        ley_cfg.connect_spokes,
    );

    // Phase 2 debug image: height + base disks + shrine dots + colored ley lines
    let phase2_path = format!("{}/phase2_ley.png", out_dir);

    // Build overlays
    let base_disks_rgba: Vec<_> = p1.base_centers
        .iter()
        .map(|&c| (c, p1.base_radius, [255, 64, 64, 200]))
        .collect();

    let shrine_points: Vec<_> = ley.shrines
        .iter()
        .map(|&p| (p, [64, 255, 255, 255])) // cyan dots
        .collect();

    // Color spokes vs cycle edges
    let center_px = IVec2::new(tpl.size.0 / 2, tpl.size.1 / 2);
    let ley_lines: Vec<_> = ley.lines
        .iter()
        .map(|&(a,b)| {
            let is_spoke = a == center_px || b == center_px;
            let color = if is_spoke { [64, 96, 255, 255] } else { [64, 255, 96, 255] };
            (a, b, color)
        })
        .collect();

    write_height_with_overlays(
        &phase2_path,
        &p1.height,
        &base_disks_rgba,
        &shrine_points,
        &ley_lines,
    );

    (p1, ley)
}
