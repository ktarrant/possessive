use super::grid::Grid;
use super::template::MapTemplate;
use super::debug_png::{write_height_with_disks, write_height_with_overlays, write_terrain_classes};
use glam::IVec2;
use super::spawns::{Phase1Bases, generate_phase1_bases};
use super::ley::{LeyConfig, LeyNetwork, generate_ley};
use super::landscape::{generate_phase3_terrain_clumps};
use super::blend::{blend_terrain_and_save, BlendSettings};
use super::blend::{blend_fractal_and_save, FractalSettings};

// Save per phase, now using the new Phase 3:
pub fn generate_all_phases_and_save(
    tpl: &MapTemplate,
    num_bases: usize,
    start_angle_deg: f32,
    ley_cfg: LeyConfig,
    terrain_seed: u32,
    out_dir: &str,
) -> (Phase1Bases, LeyNetwork, Grid<u8>) {
    std::fs::create_dir_all(out_dir).ok();

    // Phase 1
    let p1 = generate_phase1_bases(tpl, num_bases, Some(start_angle_deg));
    let p1_path = format!("{}/phase1_bases.png", out_dir);
    let base_disks: Vec<_> = p1.base_centers.iter().map(|&c| (c, p1.base_radius, [255,64,64])).collect();
    write_height_with_disks(&p1_path, &p1.height, &base_disks);

    // Phase 2
    let ley = generate_ley(
        tpl.size, num_bases, start_angle_deg,
        ley_cfg.m_shrines, ley_cfg.shrine_ring, ley_cfg.offset_deg,
        ley_cfg.connect_cycle, ley_cfg.connect_spokes
    );
    let p2_path = format!("{}/phase2_ley.png", out_dir);
    let base_disks_rgba: Vec<_> = p1.base_centers.iter().map(|&c| (c, p1.base_radius, [255,64,64,200])).collect();
    let shrine_points: Vec<_> = ley.shrines.iter().copied().map(|p| (p, [64,255,255,255])).collect();
    let center_px = IVec2::new(tpl.size.0/2, tpl.size.1/2);
    let ley_lines: Vec<_> = ley.lines.iter().map(|&(a,b)| {
        let is_spoke = a==center_px || b==center_px;
        let color = if is_spoke { [64,96,255,255] } else { [64,255,96,255] };
        (a,b,color)
    }).collect();
    write_height_with_overlays(&p2_path, &p1.height, &base_disks_rgba, &shrine_points, &ley_lines);

    // Phase 3 (NEW clumpy terrain + hard buffers)
    let classes = generate_phase3_terrain_clumps(tpl, &p1.base_centers, &ley.shrines, terrain_seed);
    let p3_path = format!("{}/phase3_terrain.png", out_dir);
    const PALETTE: [[u8;4];4] = [
        [110,180,110,255], // grassland
        [ 34,139, 34,255], // forest
        [ 64,120,255,255], // water
        [150,150,150,255], // mountain
    ];
    write_terrain_classes(&p3_path, &classes, &PALETTE);

    let blended = blend_terrain_and_save(
        &tpl,
        &p1.base_centers,
        &ley.shrines,
        &classes,                    // from Phase 3
        BlendSettings::default(),    // tweak as you like
        "out/phase4_blended.png",
    );

    let fractal = blend_fractal_and_save(
        &tpl,
        &p1.base_centers,
        &ley.shrines,
        &classes,
        FractalSettings {
            iterations: 2,          // try 2–3
            radii: [2,3,2,3],
            inertia: 0.2,           // lower = freer to change at edges
            boundary_only: true,
            warp_amp: 5.0,          // stronger jaggies
            warp_freq: 1.0/20.0,    // coarser patterns
            warp_octaves: 3,
            warp_gain: 0.55,
            warp_lacunarity: 2.2,
            seed: 42,
        },
        "out/phase4b_fractal.png",
    );

    (p1, ley, classes)
}
