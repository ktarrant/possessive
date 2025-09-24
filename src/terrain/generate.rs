use std::path::Path;
use glam::IVec2;
use super::grid::Grid;
use super::template::{MapTemplate, LeyConfig, BlendConfig, FractalConfig};
use super::debug_png::{write_height_with_disks, write_height_with_overlays, write_terrain_classes, write_terrain_with_objects};
use super::spawns::{Phase1Bases, generate_phase1_bases};
use super::ley::{LeySettings, LeyNetwork, generate_ley};
use super::landscape::generate_phase3_terrain_clumps;
use super::blend::{blend_terrain, BlendSettings, blend_fractal, FractalSettings};
use super::objects::{generate_objects, PlacedObject};

// Converters from template configs -> runtime settings
fn to_blend_settings(c: &BlendConfig) -> BlendSettings {
    let (g, f, w, m) = c.radii;
    BlendSettings {
        iterations: c.iterations,
        radii: [g, f, w, m],
        inertia: c.inertia,
        boundary_only: c.boundary_only,
    }
}
fn to_fractal_settings(c: &FractalConfig) -> FractalSettings {
    let (g, f, w, m) = c.radii;
    FractalSettings {
        iterations: c.iterations,
        radii: [g, f, w, m],
        inertia: c.inertia,
        boundary_only: c.boundary_only,
        warp_amp: c.warp_amp,
        warp_freq: c.warp_freq,
        warp_octaves: c.warp_octaves,
        warp_gain: c.warp_gain,
        warp_lacunarity: c.warp_lacunarity,
        seed: c.seed,
    }
}
fn to_ley_settings(tpl: &MapTemplate, num_bases: usize) -> super::ley::LeySettings {
    let r: &LeyConfig = &tpl.ley;
    let spb = r.shrines_per_base;
    let total_shrines = spb.saturating_mul(num_bases);

    LeySettings {
        m_shrines: total_shrines,
        shrine_ring: r.shrine_ring,
        offset_deg: r.offset_deg,
        connect_cycle: r.connect_cycle,
        connect_spokes: r.connect_spokes,
    }
}

// Optional PNGs: pass Some("out") to save, or None to skip.
// Also allow passing settings; if None, we’ll read from template or fall back to defaults.
pub fn generate_all_phases(
    tpl: &MapTemplate,
    num_bases: usize,
    start_angle_deg: f32,
    ley_override: Option<LeySettings>,
    blend_override: Option<BlendSettings>,
    fractal_override: Option<FractalSettings>,
    terrain_seed: u32,
    out_dir: Option<&str>,
) -> (Phase1Bases, LeyNetwork, Grid<u8>, Vec<PlacedObject>) {
    // resolve configs (override > template > defaults)
    let ley_cfg = ley_override
        .unwrap_or_else(|| to_ley_settings(tpl, num_bases));
    let blend_cfg = blend_override
        .unwrap_or_else(|| to_blend_settings(&tpl.blend));
    let fractal_cfg = fractal_override
        .unwrap_or_else(|| to_fractal_settings(&tpl.fractal));

    let save = |name: &str, f: &dyn Fn(&Path)| {
        if let Some(dir) = out_dir {
            let dirp = Path::new(dir);
            let _ = std::fs::create_dir_all(dirp);
            f(&dirp.join(name));
        }
    };

    const PALETTE: [[u8;4];4] = [
        [110,180,110,255], // grassland
        [ 34,139, 34,255], // forest
        [ 64,120,255,255], // water
        [150,150,150,255], // mountain
    ];

    // Phase 1
    let p1 = generate_phase1_bases(tpl, num_bases, Some(start_angle_deg));
    save("phase1_bases.png", &|p| {
        let base_disks: Vec<_> = p1.base_centers.iter().map(|&c| (c, p1.base_radius, [255,64,64])).collect();
        write_height_with_disks(&p.to_string_lossy(), &p1.height, &base_disks);
    });

    // Phase 2
    let ley = generate_ley(
        tpl.size, num_bases, start_angle_deg,
        ley_cfg.m_shrines, ley_cfg.shrine_ring, ley_cfg.offset_deg,
        ley_cfg.connect_cycle, ley_cfg.connect_spokes
    );
    save("phase2_ley.png", &|p| {
        let base_disks_rgba: Vec<_> = p1.base_centers.iter().map(|&c| (c, p1.base_radius, [255,64,64,200])).collect();
        let shrine_points: Vec<_> = ley.shrines.iter().copied().map(|q| (q, [64,255,255,255])).collect();
        let center_px = IVec2::new(tpl.size.0/2, tpl.size.1/2);
        let ley_lines: Vec<_> = ley.lines.iter().map(|&(a,b)| {
            let is_spoke = a==center_px || b==center_px;
            let color = if is_spoke { [64,96,255,255] } else { [64,255,96,255] };
            (a,b,color)
        }).collect();
        write_height_with_overlays(&p.to_string_lossy(), &p1.height, &base_disks_rgba, &shrine_points, &ley_lines);
    });

    // Phase 3
    let classes = generate_phase3_terrain_clumps(tpl, &p1.base_centers, &ley.shrines, terrain_seed);
    save("phase3_terrain.png", &|p| {
        write_terrain_classes(&p.to_string_lossy(), &classes, &PALETTE);
    });

    // Phase 4A (blend)
    let blended = blend_terrain(
        tpl, &p1.base_centers, &ley.shrines, &classes, blend_cfg,
    );
    save("phase4a_blend.png", &|p| {
        write_terrain_classes(&p.to_string_lossy(), &blended, &PALETTE);
    });

    // Phase 4B (fractal)
    let final_classes = blend_fractal(
        tpl, &p1.base_centers, &ley.shrines, &blended, fractal_cfg,
    );
    save("phase4b_fractal.png", &|p| {
        write_terrain_classes(&p.to_string_lossy(), &final_classes, &PALETTE);
    });

    // Phase 5 (Populate with objects)
    let objs = generate_objects(
        &tpl,
        &final_classes,
        &p1.base_centers,   // from Phase 1
        &ley.shrines,       // from Phase 2
        0,                  // extra_seed or your own objects_seed
    );
    save("phase5_objects.png", &|p| {
        write_terrain_with_objects(&p.to_string_lossy(), &final_classes, &PALETTE, &objs, &tpl);
    });

    (p1, ley, final_classes, objs)
}
